//! The referee: owns the clock, the held set, the score and the verdict.
//! Every agent front - the in-process baseline, the socket server the
//! external agents talk to - drives this one struct.
//!
//! The referee is generic over [`GameChannel`], so a test seats it on a
//! scripted fake and never spawns a process.

use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};

use serde_json::{json, Map, Value};

use crate::{
    audit::{BenchEvent, Bus},
    cli::BudgetArgs,
    game::{GameChannel, BOOT_TIMEOUT, STEP_TIMEOUT},
    gesture::{check_shared_keys, expand, parse_gestures, shared_keys, Gesture},
    observation::condense,
    score::{end_state, Scorer},
};

/// Ticks per simulated second: the channel's fixed step.
pub const TICKS_PER_SECOND: u64 = 60;
/// The step an `act` takes when the agent names none: half a second.
pub const DEFAULT_ACT_TICKS: u64 = 30;

/// The three budgets a run ends on, whichever comes first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// Game ticks.
    pub ticks: u64,
    /// `act` calls.
    pub turns: u64,
    /// Wall clock for the whole run.
    pub deadline: Duration,
}

impl From<&BudgetArgs> for Budget {
    fn from(args: &BudgetArgs) -> Self {
        Self {
            ticks: args.ticks,
            turns: args.turns,
            deadline: args.deadline(),
        }
    }
}

impl Budget {
    /// The budget as the audit records it.
    pub fn to_json(self) -> Value {
        json!({ "ticks": self.ticks, "turns": self.turns, "deadline": self.deadline.as_secs() })
    }
}

/// The referee over one game.
pub struct Referee<G: GameChannel> {
    game: G,
    bus: Bus,
    budget: Budget,
    audit_raw: bool,
    tick: u64,
    held: BTreeSet<String>,
    scorer: Scorer,
    last: Option<Value>,
    last_errors: Vec<Value>,
    over: Option<String>,
    started: Instant,
}

impl<G: GameChannel> Referee<G> {
    /// Seat a referee on a game that has been spawned but not yet stepped.
    pub fn new(game: G, bus: Bus, budget: Budget, audit_raw: bool) -> Self {
        Self {
            game,
            bus,
            budget,
            audit_raw,
            tick: 0,
            held: BTreeSet::new(),
            scorer: Scorer::default(),
            last: None,
            last_errors: Vec::new(),
            over: None,
            started: Instant::now(),
        }
    }

    /// Step the world to tick 1 and take the first observation. The boot
    /// happens here: the channel prints nothing before its first step.
    pub fn start(&mut self) -> Result<Value, String> {
        self.started = Instant::now();
        let line = json!({ "tick": 1 });
        self.send(&line)?;
        let answer = match self.game.read_answer(BOOT_TIMEOUT) {
            Ok(answer) => answer,
            Err(error) => {
                let reason = format!("game_error: {error}");
                self.end(&reason);
                return Err(error);
            }
        };
        self.tick = 1;
        let no_player = !answer.snapshot["ships"]
            .as_array()
            .is_some_and(|ships| ships.iter().any(|ship| ship["controller"] == "Player"));
        self.absorb(answer.snapshot, answer.errors);
        if no_player {
            self.bus.emit(BenchEvent::Note {
                text: "no player ship on the first tick; a scenario that refused to start says why in game.log".into(),
            });
        }
        self.check_end();
        Ok(self.observe(&[]))
    }

    /// Whether the run has ended.
    pub fn is_over(&self) -> bool {
        self.over.is_some()
    }

    /// Why the run ended, once it has.
    pub fn reason(&self) -> Option<&str> {
        self.over.as_deref()
    }

    /// The scorer, for the front that feeds it usage.
    pub fn scorer_mut(&mut self) -> &mut Scorer {
        &mut self.scorer
    }

    /// The score so far.
    pub fn score_json(&self) -> Value {
        self.scorer.to_json()
    }

    /// The score as the log prints it.
    pub fn score_table(&self) -> String {
        self.scorer.table()
    }

    /// The bus this referee reports on.
    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    /// The current tick.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// The pilot's view now: the last snapshot condensed, plus the clock, the
    /// budget left and whether the run is over. Free - the clock stands.
    ///
    /// `expand` names the body groups to open in full. An expansion is a
    /// second read of the snapshot already in hand, so it costs tokens and
    /// never game time - which is why it lives on `observe` rather than on a
    /// tool of its own.
    pub fn observe(&self, expand: &[String]) -> Value {
        let view = self
            .last
            .as_ref()
            .map(|snapshot| condense(snapshot, &self.held, expand))
            .unwrap_or_else(|| json!({}));
        let mut ordered = Map::new();
        ordered.insert("tick".into(), json!(self.tick));
        ordered.insert("game_seconds".into(), json!(self.game_seconds()));
        ordered.insert("over".into(), json!(self.over.is_some()));
        ordered.insert("ended_by".into(), json!(self.over));
        ordered.insert(
            "budget_left".into(),
            json!({
                "ticks": self.budget.ticks.saturating_sub(self.tick),
                "turns": self.budget.turns.saturating_sub(self.scorer.score.turns),
                "seconds": self.budget.deadline.saturating_sub(self.started.elapsed()).as_secs(),
            }),
        );
        if !self.last_errors.is_empty() {
            ordered.insert(
                "game_errors".into(),
                json!(self
                    .last_errors
                    .iter()
                    .map(|error| error["error"].clone())
                    .collect::<Vec<_>>()),
            );
        }
        if let Value::Object(fields) = view {
            ordered.extend(fields);
        }
        Value::Object(ordered)
    }

    /// Apply gestures, run the clock `ticks` further, observe. The one way
    /// time passes. Once the run is over this answers the last observation
    /// with `over: true` and moves nothing.
    pub fn act(&mut self, gestures: &[Gesture], ticks: u64) -> Value {
        if self.check_deadline() {
            return self.observe(&[]);
        }
        if self.scorer.score.turns >= self.budget.turns {
            self.end("turns");
            return self.observe(&[]);
        }
        let remaining = self.budget.ticks.saturating_sub(self.tick);
        if remaining == 0 {
            self.end("ticks");
            return self.observe(&[]);
        }
        let expansion = expand(gestures, self.tick, ticks.min(remaining), &mut self.held);
        self.scorer.score.turns += 1;
        self.scorer.score.gestures += gestures.len() as u64;
        for line in &expansion.lines {
            if let Err(error) = self.send(line) {
                self.end(&format!("game_error: {error}"));
                return self.observe(&[]);
            }
        }
        match self.game.read_answer(STEP_TIMEOUT) {
            Ok(answer) => {
                self.tick = expansion.end_tick;
                self.absorb(answer.snapshot, answer.errors);
                self.check_end();
            }
            Err(error) => self.end(&format!("game_error: {error}")),
        }
        self.observe(&[])
    }

    /// The agent declares itself finished. The status and report are
    /// recorded as data; the score comes from the world.
    pub fn finish(&mut self, status: &str, report: &str) {
        self.scorer.score.agent_status = Some(status.to_string());
        self.scorer.score.agent_report = Some(report.to_string());
        self.end("finish");
    }

    /// One referee-protocol request, answered. Malformed requests get an
    /// `error` reply and the run continues.
    pub fn handle(&mut self, request: &Value) -> Value {
        self.bus.emit(BenchEvent::AgentRequest {
            request: request.clone(),
        });
        let reply = self.answer(request);
        let recorded = match reply.get("ok").filter(|ok| ok.get("tick").is_some()) {
            Some(ok) => json!({ "ok": format!("observation@{}", ok["tick"]) }),
            None => reply.clone(),
        };
        self.bus.emit(BenchEvent::AgentReply { reply: recorded });
        reply
    }

    fn answer(&mut self, request: &Value) -> Value {
        let Some(object) = request.as_object() else {
            return json!({ "error": "a request is one object: {\"observe\": {}}, {\"act\": {...}}, {\"page\": {...}} or {\"finish\": {...}}" });
        };
        if let Some(observe) = object.get("observe") {
            let expand = match parse_expand(observe.get("expand")) {
                Ok(expand) => expand,
                Err(error) => return json!({ "error": error }),
            };
            return json!({ "ok": self.observe(&expand) });
        }
        if let Some(act) = object.get("act") {
            let gestures = match parse_gestures(act.get("gestures").unwrap_or(&json!([]))) {
                Ok(gestures) => gestures,
                Err(error) => return json!({ "error": error }),
            };
            // The world says which actions share a physical key, so a game
            // that declares a new shadow is guarded here without an edit.
            let shared = self.last.as_ref().map(shared_keys).unwrap_or_default();
            if let Err(error) = check_shared_keys(&gestures, &shared) {
                return json!({ "error": error });
            }
            let ticks = match act.get("ticks") {
                None | Some(Value::Null) => DEFAULT_ACT_TICKS,
                Some(value) => match value.as_u64() {
                    Some(ticks) => ticks,
                    None => {
                        return json!({ "error": "`ticks` is a whole number of game ticks (60 per second)" })
                    }
                },
            };
            return json!({ "ok": self.act(&gestures, ticks) });
        }
        if let Some(request) = object.get("page") {
            let Some(name) = request.get("name").and_then(Value::as_str) else {
                return json!({
                    "error": format!("`page` takes a page name: {}", crate::manual::page_names())
                });
            };
            return match crate::manual::page(name) {
                Some(text) => json!({ "ok": { "page": name, "text": text } }),
                None => json!({
                    "error": format!("no page `{name}`; the pages are {}", crate::manual::page_names())
                }),
            };
        }
        if let Some(finish) = object.get("finish") {
            let status = finish["status"].as_str().unwrap_or("done");
            let report = finish["report"].as_str().unwrap_or("");
            self.finish(status, report);
            return json!({ "ok": { "over": true, "ended_by": self.over } });
        }
        json!({ "error": "unknown request; send observe, act, page or finish" })
    }

    /// End the run for `reason` if the wall clock is spent. Returns whether
    /// the run is over, for whatever reason.
    pub fn check_deadline(&mut self) -> bool {
        if self.over.is_none() && self.started.elapsed() >= self.budget.deadline {
            self.end("deadline");
        }
        self.over.is_some()
    }

    /// End the run: record why, score it, let the game exit. Idempotent.
    pub fn end(&mut self, reason: &str) {
        if self.over.is_some() {
            return;
        }
        self.over = Some(reason.to_string());
        self.scorer.score.ended_by = reason.to_string();
        self.stamp_clock();
        if let Some(last) = &self.last {
            // The end state grades an open goal, so it reads every body, not
            // the pilot's summary of them.
            self.scorer.score.end = end_state(&condense(last, &self.held, &["all".to_string()]));
        }
        self.bus.emit(BenchEvent::RunEnd {
            reason: reason.to_string(),
            score: self.scorer.to_json(),
        });
        self.game.close();
    }

    fn send(&mut self, line: &Value) -> Result<(), String> {
        self.bus.emit(BenchEvent::ChannelOut { line: line.clone() });
        self.game.send(line)
    }

    fn absorb(&mut self, snapshot: Value, errors: Vec<Value>) {
        self.scorer.observe(&snapshot);
        self.scorer.bad_lines(errors.len() as u64);
        self.stamp_clock();
        for error in &errors {
            self.bus.emit(BenchEvent::Refusal {
                detail: json!({ "game": error["error"], "line": error["line"] }),
            });
        }
        let observation = condense(&snapshot, &self.held, &[]);
        self.bus.emit(BenchEvent::ChannelIn {
            tick: self.tick,
            observation,
            raw: self.audit_raw.then(|| snapshot.clone()),
            errors: errors.clone(),
        });
        self.last_errors = errors;
        self.last = Some(snapshot);
    }

    fn check_end(&mut self) {
        if self.scorer.score.outcome != "none" {
            self.end("outcome");
        } else if self.tick >= self.budget.ticks {
            self.end("ticks");
        } else {
            self.check_deadline();
        }
    }

    fn stamp_clock(&mut self) {
        self.scorer.score.ticks = self.tick;
        self.scorer.score.game_seconds = self.game_seconds();
        self.scorer.score.wall_seconds =
            (self.started.elapsed().as_secs_f64() * 10.0).round() / 10.0;
    }

    fn game_seconds(&self) -> f64 {
        (self.tick as f64 / TICKS_PER_SECOND as f64 * 100.0).round() / 100.0
    }
}

/// The `expand` list of an `observe` request: the body-group keys to open in
/// full. Absent is the summary view; `all` opens every group.
fn parse_expand(value: Option<&Value>) -> Result<Vec<String>, String> {
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(keys)) => keys
            .iter()
            .map(|key| {
                key.as_str()
                    .map(ToString::to_string)
                    .ok_or_else(|| format!("`expand` takes group keys as strings, not {key}"))
            })
            .collect(),
        Some(other) => Err(format!(
            "`expand` is an array of group keys, like [\"5-10km.astern\"], not {other}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Answer;

    /// A game that answers every step from a script, tracking what it was sent.
    struct Scripted {
        sent: Vec<Value>,
        answers: Vec<Value>,
        closed: bool,
    }

    impl Scripted {
        fn new(answers: Vec<Value>) -> Self {
            Self {
                sent: Vec::new(),
                answers,
                closed: false,
            }
        }
    }

    impl GameChannel for Scripted {
        fn send(&mut self, line: &Value) -> Result<(), String> {
            self.sent.push(line.clone());
            Ok(())
        }

        fn read_answer(&mut self, _: Duration) -> Result<Answer, String> {
            if self.answers.is_empty() {
                return Err("script exhausted".into());
            }
            Ok(Answer {
                snapshot: self.answers.remove(0),
                errors: vec![],
            })
        }

        fn close(&mut self) {
            self.closed = true;
        }
    }

    fn world(raider_health: f64, outcome: Option<&str>) -> Value {
        json!({
            "mission": {
                "objectives": [{ "id": "kill", "message": "Destroy the raider." }],
                "outcome": outcome.map(|kind| json!({ "kind": kind, "message": "" })),
                "comms": [],
            },
            "ships": [
                { "id": "player", "controller": "Player", "allegiance": "Player", "position": [0, 0, 0], "rotation": [0, 0, 0, 1],
                  "linear_velocity": [0, 0, 0], "angular_velocity": [0, 0, 0], "health": { "current": 800, "max": 800 }, "sections": [] },
                { "id": "raider_1", "controller": "AI", "allegiance": "Enemy", "position": [0, 0, -280], "rotation": [0, 0, 0, 1],
                  "linear_velocity": [0, 0, 0], "health": { "current": raider_health, "max": 600 }, "defeated": raider_health <= 0.0 }
            ],
            "applied": [],
            "input": { "live": [], "contexts": [] },
            "beacons": [],
            "ordnance": []
        })
    }

    fn budget(ticks: u64, turns: u64) -> Budget {
        Budget {
            ticks,
            turns,
            deadline: Duration::from_secs(60),
        }
    }

    #[test]
    fn the_referee_stamps_ticks_scores_the_world_and_ends_on_the_outcome() {
        let game = Scripted::new(vec![
            world(600.0, None),
            world(300.0, None),
            world(0.0, Some("Victory")),
        ]);
        let mut referee = Referee::new(game, Bus::quiet(), budget(18_000, 300), false);
        let first = referee.start().unwrap();
        assert_eq!(first["tick"], 1);
        assert_eq!(first["over"], false);
        assert_eq!(first["contacts"][0]["distance_m"], 2800.0);

        let reply = referee.handle(
            &json!({ "act": { "gestures": [{ "press": "flight.main_drive" }], "ticks": 30 } }),
        );
        assert_eq!(reply["ok"]["tick"], 31);
        assert_eq!(reply["ok"]["inputs"]["held"], json!(["flight.main_drive"]));
        assert_eq!(reply["ok"]["budget_left"]["turns"], 299);

        let reply = referee.handle(&json!({ "act": { "gestures": [] } }));
        assert_eq!(reply["ok"]["tick"], 61);
        assert_eq!(reply["ok"]["over"], true);
        assert_eq!(reply["ok"]["ended_by"], "outcome");
        assert!(referee.is_over());
        let score = referee.score_json();
        assert_eq!(score["outcome"], "victory");
        assert_eq!(score["kills"], 1);
        assert_eq!(score["turns"], 2);
        assert_eq!(score["gestures"], 1);
        assert_eq!(score["ticks"], 61);
        assert_eq!(score["ended_by"], "outcome");

        // Over: another act moves nothing and answers the same view.
        let reply =
            referee.handle(&json!({ "act": { "gestures": [{ "tap": "x" }], "ticks": 30 } }));
        assert_eq!(reply["ok"]["tick"], 61);
        assert_eq!(referee.score_json()["turns"], 2);
    }

    #[test]
    fn a_malformed_request_is_refused_and_the_run_continues() {
        let game = Scripted::new(vec![world(600.0, None), world(600.0, None)]);
        let mut referee = Referee::new(game, Bus::quiet(), budget(18_000, 300), false);
        referee.start().unwrap();
        assert!(referee.handle(&json!([1, 2])).get("error").is_some());
        assert!(referee.handle(&json!({ "warp": 9 })).get("error").is_some());
        assert!(referee
            .handle(&json!({ "act": { "gestures": [{ "zap": "x" }] } }))
            .get("error")
            .is_some());
        assert!(referee
            .handle(&json!({ "act": { "gestures": [], "ticks": "soon" } }))
            .get("error")
            .is_some());
        assert!(!referee.is_over());
        assert_eq!(referee.handle(&json!({ "observe": {} }))["ok"]["tick"], 1);
        assert_eq!(referee.handle(&json!({ "act": {} }))["ok"]["tick"], 31);
    }

    #[test]
    fn the_budgets_end_the_run_and_finish_records_the_agents_word() {
        let answers: Vec<Value> = (0..5).map(|_| world(600.0, None)).collect();
        let mut referee =
            Referee::new(Scripted::new(answers), Bus::quiet(), budget(50, 300), false);
        referee.start().unwrap();
        let reply = referee.act(&[], 30);
        assert_eq!(reply["tick"], 31);
        assert_eq!(reply["over"], false);
        let reply = referee.act(&[], 30);
        assert_eq!(reply["tick"], 50);
        assert_eq!(reply["ended_by"], "ticks");

        let answers: Vec<Value> = (0..3).map(|_| world(600.0, None)).collect();
        let mut referee = Referee::new(
            Scripted::new(answers),
            Bus::quiet(),
            budget(18_000, 1),
            false,
        );
        referee.start().unwrap();
        referee.act(&[], 30);
        assert_eq!(referee.act(&[], 30)["ended_by"], "turns");

        let mut referee = Referee::new(
            Scripted::new(vec![world(600.0, None)]),
            Bus::quiet(),
            budget(18_000, 300),
            false,
        );
        referee.start().unwrap();
        let reply = referee
            .handle(&json!({ "finish": { "status": "gave_up", "report": "Cannot find it." } }));
        assert_eq!(reply["ok"]["ended_by"], "finish");
        assert_eq!(referee.score_json()["agent_status"], "gave_up");
        assert_eq!(referee.score_json()["agent_report"], "Cannot find it.");
    }

    /// The two free reads: a manual page, and one body group opened up. What
    /// the agent chose to look at lands in the audit either way.
    #[test]
    fn a_page_and_an_expansion_are_free_reads_that_move_no_clock() {
        let game = Scripted::new(vec![world(600.0, None), world(600.0, None)]);
        let mut referee = Referee::new(game, Bus::quiet(), budget(18_000, 300), false);
        referee.start().unwrap();

        let reply = referee.handle(&json!({ "page": { "name": "targeting" } }));
        assert!(reply["ok"]["text"].as_str().unwrap().contains("dwell"));
        let reply = referee.handle(&json!({ "page": { "name": "warp-drive" } }));
        assert!(reply["error"].as_str().unwrap().contains("the pages are"));

        let reply = referee.handle(&json!({ "observe": { "expand": ["5-10km.bow"] } }));
        assert_eq!(reply["ok"]["tick"], 1);
        let reply = referee.handle(&json!({ "observe": { "expand": "everything" } }));
        assert!(reply["error"]
            .as_str()
            .unwrap()
            .contains("array of group keys"));
        assert_eq!(referee.tick(), 1, "neither read moved the clock");
    }

    #[test]
    fn a_game_that_stops_answering_ends_the_run_as_a_game_error() {
        let mut referee = Referee::new(
            Scripted::new(vec![world(600.0, None)]),
            Bus::quiet(),
            budget(18_000, 300),
            false,
        );
        referee.start().unwrap();
        let reply = referee.act(&[Gesture::Tap("x".into())], 30);
        assert_eq!(reply["over"], true);
        assert!(reply["ended_by"]
            .as_str()
            .unwrap()
            .starts_with("game_error"));
    }
}
