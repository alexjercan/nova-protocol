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

/// A run's whole rail: what to say about it, and every row in frame order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Film {
    /// The header strip's content.
    pub head: Head,
    /// Every row, by ascending frame.
    pub cues: Vec<Cue>,
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

/// Build the rail from a run's events.
pub fn film(events: &[BenchEvent]) -> Film {
    let mut head = Head::default();
    let mut tick = 1;
    let mut turn = 0;
    let mut rows: Vec<(u64, u64, Row)> = Vec::new();
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
                tick: at, errors, ..
            } => {
                tick = *at;
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
            BenchEvent::RunEnd { reason, .. } => rows.push((
                tick,
                turn,
                Row {
                    lane: Lane::Act,
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
        cues: place(rows),
    }
}

/// Spread the rows over frames: a row lands on its own tick's frame, or
/// [`MIN_GAP`] after the row before it, whichever is later.
fn place(rows: Vec<(u64, u64, Row)>) -> Vec<Cue> {
    let mut placed: Vec<Cue> = Vec::with_capacity(rows.len());
    for (tick, turn, row) in rows {
        let own = tick.saturating_sub(1);
        let frame = placed
            .last()
            .map_or(own, |previous| own.max(previous.frame + MIN_GAP));
        placed.push(Cue {
            frame,
            tick,
            turn,
            row,
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

/// One gesture as a row. The shell lines carry their own lane so a cheat
/// cannot pass for an ordinary command, and drop the `command` prefix: the
/// lane already said it.
fn gesture_row(gesture: &Gesture) -> Row {
    match gesture {
        Gesture::Command(line) => Row {
            lane: if line.split_whitespace().next() == Some("cheats") {
                Lane::Cheat
            } else {
                Lane::Nova
            },
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

/// One line, single spaced: the rail has no room for the model's paragraphs.
fn flatten(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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
                text: "I will aim 35 degrees starboard.".into(),
            },
            BenchEvent::AgentRequest {
                request: json!({ "act": { "gestures": [{ "press": "flight.main_drive" }], "ticks": 180 } }),
            },
            BenchEvent::ChannelIn {
                tick: 313,
                observation: json!({}),
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
        let film = film(&events());
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
                (Lane::Error, "tick 4 is in the past (line 9)", 312, 1),
                (Lane::Cheat, "cheats arm", 333, 2),
            ]
        );
    }

    #[test]
    fn the_view_shows_the_newest_rows_that_fit_and_the_turn_beside_them() {
        let film = film(&events());
        assert!(film.view(0, 60, 6).is_empty() || film.view(0, 60, 6).len() == 1);
        let late = film.view(400, 60, 3);
        assert_eq!(late.len(), 3);
        assert_eq!(late[2].row.lane, Lane::Cheat);
        assert_eq!(film.turn(400), 2);
        assert_eq!(film.turn(0), 0);
        assert_eq!(Film::tick(0), 1);
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
