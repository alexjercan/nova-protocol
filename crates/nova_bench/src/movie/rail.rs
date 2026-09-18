//! The action rail: `audit.jsonl` read back as the lines the movie draws.
//!
//! One audit event becomes zero or more [`Row`]s, and a row is placed on the
//! frame it first appears on. The mapping is exact rather than guessed: the
//! channel's recorder captures one frame per stepped tick and numbers from
//! zero (`nova_channel::record`), so **frame = tick - 1**.
//!
//! Agent prose carries no tick of its own - the world is frozen while the
//! model thinks - so it lands on the tick of the snapshot the model was
//! answering. Rows sharing a tick are spread by [`MIN_GAP`] frames, or a
//! whole opening turn lands on frame zero as a wall of text.

use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use serde_json::Value;

use crate::{
    audit::BenchEvent,
    gesture::{parse_gestures, Gesture},
};

/// Frames between two rows that would otherwise share one frame: 0.35 s.
pub const MIN_GAP: u64 = 21;

/// Frames the last row stays on screen after it appears: 2 s to read it.
pub const HOLD: u64 = 120;

/// The most held frames a movie may grow by so its rail can finish: 30 s. A
/// command-heavy run writes more rows than its footage has seconds - the NOVA
/// OS shell spends almost no ticks - and truncating the end of the log is the
/// one thing the rail must not do.
pub const MAX_TAIL: u64 = 1800;

/// Which lane a row belongs to. The lane picks the label and the colour, and
/// is the whole reason the rail is not a JSON dump: a refusal must not read
/// like a plan, and an armed cheat must not read like an ordinary command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    /// The goal the run was prompted with, once, at the head of the rail.
    Goal,
    /// Assistant thinking, one row per header line.
    Think,
    /// Assistant prose.
    Say,
    /// A look at the world that spends no tick: `observe`, `page`.
    Look,
    /// A gesture the agent asked for, or the wait between them.
    Act,
    /// One NOVA OS command shell line.
    Nova,
    /// A command shell line that touches cheats.
    Cheat,
    /// The game refused a line, or the referee refused the agent.
    Error,
}

impl Lane {
    /// The label printed after the prompt.
    pub fn label(self) -> &'static str {
        match self {
            Self::Goal => "goal",
            Self::Think => "think",
            Self::Say => "say",
            Self::Look => "look",
            Self::Act => "act",
            Self::Nova => "nova",
            Self::Cheat => "cheat",
            Self::Error => "err",
        }
    }

    /// The lane's colour as `[r, g, b]`, on the movie's dark panel.
    pub fn color(self) -> [u8; 3] {
        match self {
            Self::Goal => [0xD8, 0xC7, 0x8F],
            Self::Think => [0x9A, 0xA0, 0xAA],
            Self::Say => [0xE6, 0xE9, 0xEF],
            Self::Look => [0x8F, 0xB8, 0xD8],
            Self::Act => [0x7C, 0xFF, 0x9B],
            Self::Nova => [0x6F, 0xE3, 0xE1],
            Self::Cheat => [0xFF, 0xC6, 0x5C],
            Self::Error => [0xFF, 0x6B, 0x6B],
        }
    }
}

/// One line of the rail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Which lane, which colour.
    pub lane: Lane,
    /// The text after the lane label, already flattened to one line.
    pub text: String,
}

/// A row placed on the timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cue {
    /// The first frame the row is on screen.
    pub frame: u64,
    /// The tick the world stood at when the row happened.
    pub tick: u64,
    /// The `act` the row belongs to, counting from one; zero before the first.
    pub turn: u64,
    /// The row.
    pub row: Row,
}

/// What the header strip says about the run: the same identity a reader needs
/// to reproduce it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Head {
    /// The scenario, short: `slingshot`, not the whole fixture path.
    pub scenario: String,
    /// The agent label, as the run recorded it.
    pub agent: String,
    /// The seed, when the run pinned one.
    pub seed: Option<u64>,
}

impl Head {
    /// The header strip's fixed left half: what ran, under which seed.
    pub fn identity(&self) -> String {
        let seed = self
            .seed
            .map_or_else(|| "seed os".to_string(), |seed| format!("seed {seed}"));
        format!("{}   {}   {seed}", self.scenario, self.agent)
    }
}

/// Where the movie stands. The two differ only in the tail a short, chatty
/// run needs to finish scrolling: there the footage has run out, so the rail
/// goes on while the clock stays at the last tick the game drew.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct At {
    /// The frame the rail is at.
    pub rail: u64,
    /// The frame the header's clock and tick read from.
    pub clock: u64,
}

impl At {
    /// Inside the footage, the rail and the clock are the same frame.
    pub fn live(frame: u64) -> Self {
        Self {
            rail: frame,
            clock: frame,
        }
    }
}

/// A run's whole rail: what to say about it, and every row in frame order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Film {
    /// The header strip's content.
    pub head: Head,
    /// Every row, by ascending frame.
    pub cues: Vec<Cue>,
    /// The frame from which the run carries NOVA OS's cheat mark, when it
    /// ever does. The header says so from there on, so no clip of a marked
    /// run can be lifted out looking clean.
    pub cheated_from: Option<u64>,
}

impl Film {
    /// The rows on screen at `frame`, oldest first, bounded by `lines` display
    /// lines once wrapped to `cols` columns. Newest rows win the budget.
    pub fn view(&self, frame: u64, cols: usize, lines: usize) -> Vec<&Cue> {
        let end = self.cues.partition_point(|cue| cue.frame <= frame);
        let mut budget = lines;
        let mut first = end;
        for index in (0..end).rev() {
            let height = wrap(&self.cues[index].row.text, cols).len();
            if height > budget {
                break;
            }
            budget -= height;
            first = index;
        }
        self.cues[first..end].iter().collect()
    }

    /// The `act` count at `frame`: the turn number the header shows.
    pub fn turn(&self, frame: u64) -> u64 {
        self.cues
            .partition_point(|cue| cue.frame <= frame)
            .checked_sub(1)
            .map_or(0, |last| self.cues[last].turn)
    }

    /// The tick the world stands at on `frame`. The frames are the clock, so
    /// this is arithmetic, not a lookup.
    pub fn tick(frame: u64) -> u64 {
        frame + 1
    }

    /// Whether the run was already marked at `frame`.
    pub fn cheated(&self, frame: u64) -> bool {
        self.cheated_from.is_some_and(|from| frame >= from)
    }

    /// Held frames to add after `frames` of footage so the last row appears
    /// and stays readable. Zero for a run whose rail drained long ago.
    pub fn tail(&self, frames: u64) -> u64 {
        self.cues
            .last()
            .map_or(0, |last| last.frame + HOLD)
            .saturating_sub(frames.saturating_sub(1))
            .min(MAX_TAIL)
    }
}

/// Read an audit file into its events. A truncated final line - a crash mid
/// flush - is skipped, not an error: the frames before it are still a movie.
pub fn read(path: &Path) -> Result<Vec<BenchEvent>, String> {
    let file =
        File::open(path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
    Ok(BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str(&line).ok())
        .collect())
}

/// Build the rail from a run's events, over a recording `frames` long.
pub fn film(events: &[BenchEvent], frames: u64) -> Film {
    let mut head = Head::default();
    let mut tick = 1;
    let mut turn = 0;
    let mut rows: Vec<(u64, u64, Row)> = Vec::new();
    let mut marked: Option<u64> = None;
    for event in events {
        match event {
            BenchEvent::RunStart {
                scenario,
                agent,
                goal,
                seed,
                ..
            } => {
                head = Head {
                    scenario: short_scenario(scenario),
                    agent: agent.clone(),
                    seed: *seed,
                };
                rows.push((
                    tick,
                    turn,
                    Row {
                        lane: Lane::Goal,
                        text: flatten(goal),
                    },
                ));
            }
            BenchEvent::ChannelIn {
                tick: at,
                observation,
                errors,
                ..
            } => {
                tick = *at;
                // The world declares the mark, not the shape of a command:
                // `ammo infinite player_spaceship on` is an ordinary shell
                // line until the game says the run is marked.
                if marked.is_none() && observation["cheats_marked"] == true {
                    marked = Some(tick);
                    rows.push((
                        tick,
                        turn,
                        Row {
                            lane: Lane::Cheat,
                            text: "CHEATS ARMED: this run is marked".into(),
                        },
                    ));
                }
                for error in errors {
                    rows.push((tick, turn, refusal(error)));
                }
            }
            BenchEvent::AgentThinking { text } => {
                for header in headers(text) {
                    rows.push((
                        tick,
                        turn,
                        Row {
                            lane: Lane::Think,
                            text: header,
                        },
                    ));
                }
            }
            BenchEvent::AgentText { text } => rows.push((
                tick,
                turn,
                Row {
                    lane: Lane::Say,
                    text: flatten(text),
                },
            )),
            BenchEvent::AgentRequest { request } => {
                if request.get("act").is_some() {
                    turn += 1;
                }
                for row in request_rows(request) {
                    rows.push((tick, turn, row));
                }
            }
            BenchEvent::Refusal { detail } => rows.push((tick, turn, refusal(detail))),
            // The goal lane bookends the run: it opens with what was asked
            // and closes with how it ended.
            BenchEvent::RunEnd { reason, .. } => rows.push((
                tick,
                turn,
                Row {
                    lane: Lane::Goal,
                    text: format!("run ended: {reason}"),
                },
            )),
            BenchEvent::AgentReply { .. }
            | BenchEvent::AgentTool { .. }
            | BenchEvent::AgentUsage { .. }
            | BenchEvent::ChannelOut { .. }
            | BenchEvent::Note { .. } => {}
        }
    }
    Film {
        head,
        cues: place(&rows, frames),
        // The header marks the frame the game declared it, not the frame the
        // rail scrolls the row into view: the transcript may lag the world,
        // the mark may not.
        cheated_from: marked.map(|tick| tick.saturating_sub(1)),
    }
}

/// Spread the rows over the movie at the widest gap that still fits.
///
/// A row lands on its own tick's frame, or `gap` frames after the row before
/// it, whichever is later. [`MIN_GAP`] is what a reader wants; a rail that
/// would then outlive the footage by more than [`MAX_TAIL`] tightens until it
/// fits, because losing the end of the log is worse than reading it quickly.
fn place(rows: &[(u64, u64, Row)], frames: u64) -> Vec<Cue> {
    let room = frames.saturating_sub(1) + MAX_TAIL;
    (1..=MIN_GAP)
        .rev()
        .map(|gap| spread(rows, gap))
        .find(|placed| placed.last().is_none_or(|last| last.frame + HOLD <= room))
        .unwrap_or_else(|| spread(rows, 1))
}

/// Place every row at one gap.
fn spread(rows: &[(u64, u64, Row)], gap: u64) -> Vec<Cue> {
    let mut placed: Vec<Cue> = Vec::with_capacity(rows.len());
    for (tick, turn, row) in rows {
        let own = tick.saturating_sub(1);
        let frame = placed
            .last()
            .map_or(own, |previous| own.max(previous.frame + gap));
        placed.push(Cue {
            frame,
            tick: *tick,
            turn: *turn,
            row: row.clone(),
        });
    }
    placed
}

/// The rows one referee-protocol request produces.
fn request_rows(request: &Value) -> Vec<Row> {
    if let Some(act) = request.get("act") {
        let Ok(gestures) = parse_gestures(&act["gestures"]) else {
            return vec![Row {
                lane: Lane::Act,
                text: act["gestures"].to_string(),
            }];
        };
        if gestures.is_empty() {
            return vec![Row {
                lane: Lane::Act,
                text: format!("wait {} ticks", act["ticks"]),
            }];
        }
        return gestures.iter().map(gesture_row).collect();
    }
    if let Some(page) = request.get("page") {
        return vec![Row {
            lane: Lane::Look,
            text: format!("page {}", page["name"].as_str().unwrap_or("?")),
        }];
    }
    if let Some(observe) = request.get("observe") {
        let expand: Vec<&str> = observe["expand"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        return vec![Row {
            lane: Lane::Look,
            text: if expand.is_empty() {
                "observe".to_string()
            } else {
                format!("observe {}", expand.join(" "))
            },
        }];
    }
    if let Some(finish) = request.get("finish") {
        return vec![Row {
            lane: Lane::Act,
            text: format!("finish {}", finish["status"].as_str().unwrap_or("?")),
        }];
    }
    Vec::new()
}

/// One gesture as a row. A shell line gets its own lane, apart from the helm,
/// and drops the `command` prefix: the lane already said it. No command name
/// is read for cheats here - only the game's own mark colours a run, so a
/// cheat the game does not admit cannot dress itself in the cheat lane.
fn gesture_row(gesture: &Gesture) -> Row {
    match gesture {
        Gesture::Command(line) => Row {
            lane: Lane::Nova,
            text: line.clone(),
        },
        other => Row {
            lane: Lane::Act,
            text: other.label(),
        },
    }
}

/// A refused line, from the game's own error object or the referee's.
fn refusal(detail: &Value) -> Row {
    let reason = detail["error"]
        .as_str()
        .map_or_else(|| detail.to_string(), flatten);
    let text = match detail["line"].as_u64() {
        Some(line) => format!("{reason} (line {line})"),
        None => reason,
    };
    Row {
        lane: Lane::Error,
        text,
    }
}

/// The scenario as the header says it: the fixture's own name, not its path.
fn short_scenario(raw: &str) -> String {
    let name = raw.rsplit(['/', '\\']).next().unwrap_or(raw);
    name.strip_suffix(".content.ron")
        .unwrap_or(name)
        .to_string()
}

/// Thinking arrives as markdown headers joined by blank lines. Each header is
/// its own row, so a three-header thought reads as three beats.
fn headers(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| line.trim().trim_matches('*').trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

/// One line, single spaced, with markdown bold dropped: the model writes
/// prose for a chat window, and `**a name**` on the rail is two literal
/// asterisks. The rail has no room for its paragraphs either.
fn flatten(text: &str) -> String {
    text.replace("**", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Wrap to `cols` columns on word boundaries. The face is monospaced, so a
/// column is a character and the wrap is exact.
pub fn wrap(text: &str, cols: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let width = line.chars().count();
        if width > 0 && width + 1 + word.chars().count() > cols {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
        while line.chars().count() > cols {
            let head: String = line.chars().take(cols).collect();
            line = line.chars().skip(cols).collect();
            lines.push(head);
        }
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn events() -> Vec<BenchEvent> {
        vec![
            BenchEvent::RunStart {
                scenario: "crates/nova_bench/scenarios/slingshot.content.ron".into(),
                agent: "pi-gpt-5.6-sol-medium".into(),
                goal: "Reach EXIT\nwithout firing.".into(),
                seed: Some(7),
                budget: json!({}),
                run_dir: "/runs/1".into(),
            },
            BenchEvent::AgentThinking {
                text: "**Planning the assist**\n\n**Picking the offset**".into(),
            },
            BenchEvent::AgentText {
                text: "I will aim **35 degrees** starboard.".into(),
            },
            BenchEvent::AgentRequest {
                request: json!({ "act": { "gestures": [{ "press": "flight.main_drive" }], "ticks": 180 } }),
            },
            BenchEvent::ChannelIn {
                tick: 313,
                observation: json!({ "cheats_marked": true }),
                raw: None,
                errors: vec![json!({ "error": "tick 4 is in the past", "line": 9 })],
            },
            BenchEvent::AgentRequest {
                request: json!({ "act": { "gestures": [{ "command": "cheats arm" }], "ticks": 1 } }),
            },
        ]
    }

    #[test]
    fn every_lane_of_a_run_lands_on_the_frame_its_tick_drew() {
        let film = film(&events(), 1000);
        assert_eq!(film.head.scenario, "slingshot");
        assert_eq!(
            film.head.identity(),
            "slingshot   pi-gpt-5.6-sol-medium   seed 7"
        );
        let rows: Vec<(Lane, &str, u64, u64)> = film
            .cues
            .iter()
            .map(|cue| (cue.row.lane, cue.row.text.as_str(), cue.frame, cue.turn))
            .collect();
        assert_eq!(
            rows,
            vec![
                (Lane::Goal, "Reach EXIT without firing.", 0, 0),
                (Lane::Think, "Planning the assist", MIN_GAP, 0),
                (Lane::Think, "Picking the offset", MIN_GAP * 2, 0),
                (
                    Lane::Say,
                    "I will aim 35 degrees starboard.",
                    MIN_GAP * 3,
                    0
                ),
                (Lane::Act, "press flight.main_drive", MIN_GAP * 4, 1),
                (Lane::Cheat, "CHEATS ARMED: this run is marked", 312, 1),
                (Lane::Error, "tick 4 is in the past (line 9)", 333, 1),
                (Lane::Nova, "cheats arm", 354, 2),
            ]
        );
        assert_eq!(film.cheated_from, Some(312));
        assert!(!film.cheated(311) && film.cheated(312));
    }

    #[test]
    fn the_view_shows_the_newest_rows_that_fit_and_the_turn_beside_them() {
        let film = film(&events(), 1000);
        let opening = film.view(0, 60, 6);
        assert_eq!(opening.len(), 1);
        assert_eq!(opening[0].row.lane, Lane::Goal);
        let late = film.view(400, 60, 3);
        assert_eq!(late.len(), 3);
        assert_eq!(late[2].row.lane, Lane::Nova);
        assert_eq!(film.turn(400), 2);
        assert_eq!(film.turn(0), 0);
        assert_eq!(Film::tick(0), 1);
    }

    #[test]
    fn a_rail_longer_than_its_footage_tightens_instead_of_losing_its_end() {
        let mut long = events();
        long.extend((0..200).map(|beat| BenchEvent::AgentThinking {
            text: format!("beat {beat}"),
        }));
        let roomy = film(&events(), 1000);
        assert_eq!(roomy.cues[1].frame, MIN_GAP);
        assert_eq!(roomy.tail(1000), 0);

        let tight = film(&long, 60);
        assert_eq!(tight.cues.len(), roomy.cues.len() + 200);
        assert_eq!(roomy.cheated_from, Some(312));
        let last = tight.cues.last().expect("rows").frame;
        assert!(
            last + HOLD <= 59 + MAX_TAIL,
            "the whole rail fits the footage plus its held tail"
        );
        assert!(
            tight.cues[1].frame < MIN_GAP,
            "the gap tightened rather than dropping the end of the log"
        );
        assert_eq!(tight.tail(60), last + HOLD - 59);
    }

    #[test]
    fn a_waiting_act_and_a_look_read_as_themselves() {
        let rows = request_rows(&json!({ "act": { "gestures": [], "ticks": 240 } }));
        assert_eq!(rows[0].text, "wait 240 ticks");
        let rows = request_rows(&json!({ "observe": { "expand": ["5-10km.bow"] } }));
        assert_eq!(
            rows[0],
            Row {
                lane: Lane::Look,
                text: "observe 5-10km.bow".into()
            }
        );
        let rows = request_rows(&json!({ "page": { "name": "travel" } }));
        assert_eq!(rows[0].text, "page travel");
        let rows = request_rows(&json!({ "finish": { "status": "done" } }));
        assert_eq!(rows[0].text, "finish done");
        assert!(request_rows(&json!({ "observe": {} }))[0].text == "observe");
    }

    #[test]
    fn wrapping_breaks_on_words_and_still_cuts_a_word_longer_than_the_line() {
        assert_eq!(wrap("one two three", 7), vec!["one two", "three"]);
        assert_eq!(wrap("", 7), vec![""]);
        assert_eq!(wrap("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
    }
}
