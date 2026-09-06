//! The event bus: every event of a run, in order, to the audit file and to
//! the renderer. The referee emits and never knows who listens.
//!
//! `audit.jsonl` is one object per event with the wall time `t` in seconds
//! since the bus opened, flushed per line so a crash loses nothing. The
//! `channel_out` events alone replay the run.

use std::{
    fmt::Write as _,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    sync::{Arc, Mutex},
    time::Instant,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One thing that happened.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum BenchEvent {
    /// The run's setup: what plays what, under which budgets.
    RunStart {
        /// Scenario label.
        scenario: String,
        /// Agent label.
        agent: String,
        /// The goal the agent is prompted with.
        goal: String,
        /// The seed, when fixed.
        seed: Option<u64>,
        /// The budgets, as `{ticks, turns, deadline}`.
        budget: Value,
        /// The run directory.
        run_dir: String,
    },
    /// The run's end: why, and the score.
    RunEnd {
        /// The reason the run ended.
        reason: String,
        /// The score.
        score: Value,
    },
    /// One wire line written to the game.
    ChannelOut {
        /// The line.
        line: Value,
    },
    /// One snapshot read from the game, condensed. `raw` holds the full
    /// snapshot when `--audit-raw` is set.
    ChannelIn {
        /// The tick the world stands at.
        tick: u64,
        /// The observation.
        observation: Value,
        /// The full snapshot, when kept.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raw: Option<Value>,
        /// The game's error lines since the previous snapshot.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        errors: Vec<Value>,
    },
    /// A referee-protocol request, verbatim.
    AgentRequest {
        /// The request.
        request: Value,
    },
    /// A referee-protocol reply, verbatim.
    AgentReply {
        /// The reply.
        reply: Value,
    },
    /// Assistant prose (pi).
    AgentText {
        /// The text.
        text: String,
    },
    /// Assistant thinking (pi).
    AgentThinking {
        /// The text.
        text: String,
    },
    /// A tool call as pi saw it start or end.
    AgentTool {
        /// The tool name.
        name: String,
        /// The arguments.
        args: Value,
        /// The result, when this is the end of the call.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result: Option<String>,
        /// Whether the tool reported an error.
        #[serde(default)]
        is_error: bool,
    },
    /// One assistant message's token usage (pi).
    AgentUsage {
        /// The usage block, verbatim.
        usage: Value,
    },
    /// Something the referee refused or the game rejected.
    Refusal {
        /// What and why.
        detail: Value,
    },
    /// A note from the bench itself: a nudge sent, a process spawned.
    Note {
        /// The note.
        text: String,
    },
}

/// The renderer attached to the bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ui {
    /// One line per event on stderr.
    Log,
    /// Nothing on stderr.
    Quiet,
}

struct Inner {
    sink: Option<BufWriter<File>>,
    ui: Ui,
    started: Instant,
}

/// The bus: cloneable, shared between the referee and the agent front.
#[derive(Clone)]
pub struct Bus {
    inner: Arc<Mutex<Inner>>,
}

impl Bus {
    /// Open the audit file and attach the renderer.
    pub fn open(audit_path: &Path, ui: Ui) -> Result<Self, String> {
        let file = File::create(audit_path)
            .map_err(|error| format!("could not create {}: {error}", audit_path.display()))?;
        Ok(Self::new(Some(BufWriter::new(file)), ui))
    }

    /// A bus with no file: tests, and the in-memory replay check.
    pub fn quiet() -> Self {
        Self::new(None, Ui::Quiet)
    }

    fn new(sink: Option<BufWriter<File>>, ui: Ui) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                sink,
                ui,
                started: Instant::now(),
            })),
        }
    }

    /// Seconds since the bus opened.
    pub fn elapsed(&self) -> f64 {
        self.inner
            .lock()
            .map(|inner| inner.started.elapsed().as_secs_f64())
            .unwrap_or(0.0)
    }

    /// Record one event: to the audit, then to the renderer.
    pub fn emit(&self, event: BenchEvent) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let t = inner.started.elapsed().as_secs_f64();
        if let Some(sink) = inner.sink.as_mut() {
            let mut record = serde_json::to_value(&event).unwrap_or(Value::Null);
            if let Some(object) = record.as_object_mut() {
                let mut ordered = serde_json::Map::new();
                ordered.insert("t".into(), Value::from((t * 1000.0).round() / 1000.0));
                ordered.append(object);
                record = Value::Object(ordered);
            }
            let _ = writeln!(sink, "{record}").and_then(|()| sink.flush());
        }
        if inner.ui == Ui::Log {
            if let Some(line) = render(&event) {
                eprintln!("[{t:8.1}s] {line}");
            }
        }
    }
}

/// The log line for an event, or `None` for the events the log skips (the
/// audit still has them).
pub fn render(event: &BenchEvent) -> Option<String> {
    Some(match event {
        BenchEvent::RunStart {
            scenario,
            agent,
            goal,
            seed,
            budget,
            run_dir,
        } => {
            let seed = seed.map_or("os".to_string(), |seed| seed.to_string());
            format!(
                "run   {scenario} / {agent}  seed {seed}  budget {} ticks, {} turns, {} s\n            goal: {goal}\n            dir:  {run_dir}",
                budget["ticks"], budget["turns"], budget["deadline"]
            )
        }
        BenchEvent::RunEnd { reason, score } => {
            format!("end   {reason}\n{}", indent(&score_lines(score)))
        }
        BenchEvent::ChannelIn {
            tick,
            observation,
            errors,
            ..
        } => {
            let mut line = format!("tick  {tick:>6}  {}", summary(observation));
            for error in errors {
                let _ = write!(
                    line,
                    "\n            game refused line {}: {}",
                    error["line"], error["error"]
                );
            }
            line
        }
        BenchEvent::AgentRequest { request } => {
            if let Some(act) = request.get("act") {
                let gestures: Vec<String> = act["gestures"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(gesture_label)
                    .collect();
                format!("act   +{} ticks  {}", act["ticks"], gestures.join(", "))
            } else if let Some(finish) = request.get("finish") {
                format!(
                    "finish {}  {}",
                    finish["status"].as_str().unwrap_or("?"),
                    finish["report"].as_str().unwrap_or("")
                )
            } else if request.get("observe").is_some() {
                return None;
            } else {
                format!("agent sent {request}")
            }
        }
        BenchEvent::AgentReply { reply } => {
            let error = reply.get("error")?;
            format!("refused {error}")
        }
        BenchEvent::AgentText { text } => format!("agent {}", clip(text, 400)),
        BenchEvent::AgentThinking { text } => format!("think {}", clip(text, 200)),
        BenchEvent::AgentTool {
            name,
            args,
            result,
            is_error,
        } => match result {
            None => return None,
            Some(result) if *is_error => {
                format!("tool  {name} {args} -> error {}", clip(result, 200))
            }
            Some(_) => return None,
        },
        BenchEvent::AgentUsage { .. } | BenchEvent::ChannelOut { .. } => return None,
        BenchEvent::Refusal { detail } => format!("refused {detail}"),
        BenchEvent::Note { text } => format!("note  {text}"),
    })
}

/// One line on the pilot's state, from an observation.
fn summary(observation: &Value) -> String {
    let me = &observation["me"];
    if me.is_null() {
        return format!("no player ship  state {}", observation["game_state"]);
    }
    let mut line = format!(
        "{} hp {}/{} spd {} m/s",
        me["id"].as_str().unwrap_or("?"),
        me["health"]["current"],
        me["health"]["max"],
        me["speed_mps"]
    );
    if let Some(lock) = me["combat_lock"].as_str() {
        let _ = write!(line, " lock {lock}");
    }
    for contact in observation["contacts"]
        .as_array()
        .into_iter()
        .flatten()
        .take(3)
    {
        let _ = write!(
            line,
            " | {} {} m brg {} hp {}/{}{}",
            contact["id"].as_str().unwrap_or("?"),
            contact["distance_m"],
            contact["bearing_deg"],
            contact["health"]["current"],
            contact["health"]["max"],
            if contact["defeated"] == true {
                " DEFEATED"
            } else {
                ""
            }
        );
    }
    if let Some(outcome) = observation["outcome"].as_object() {
        let _ = write!(line, " | {} {}", outcome["kind"], outcome["message"]);
    }
    line
}

fn gesture_label(gesture: &Value) -> String {
    let Some(object) = gesture.as_object() else {
        return gesture.to_string();
    };
    let Some((verb, value)) = object
        .iter()
        .find(|(key, _)| !matches!(key.as_str(), "delta" | "ticks"))
    else {
        return gesture.to_string();
    };
    match verb.as_str() {
        "aim" => format!(
            "aim {} {} x{}",
            value.as_str().unwrap_or("?"),
            object
                .get("delta")
                .map_or("?".to_string(), ToString::to_string),
            object
                .get("ticks")
                .map_or("1".to_string(), ToString::to_string)
        ),
        _ => format!(
            "{verb} {}",
            value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_string)
        ),
    }
}

fn score_lines(score: &Value) -> String {
    let Some(object) = score.as_object() else {
        return score.to_string();
    };
    object
        .iter()
        .filter(|(_, value)| !value.is_null())
        .map(|(key, value)| match value {
            Value::String(text) => format!("{key:<22}{text}"),
            Value::Array(items) => format!("{key:<22}{}", items.len()),
            other => format!("{key:<22}{other}"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn indent(text: &str) -> String {
    text.lines()
        .map(|line| format!("            {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn clip(text: &str, max: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        flat
    } else {
        let head: String = flat.chars().take(max).collect();
        format!("{head}...")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn events_round_trip_through_json_with_a_tag() {
        let event = BenchEvent::ChannelIn {
            tick: 30,
            observation: json!({ "me": null }),
            raw: None,
            errors: vec![],
        };
        let text = serde_json::to_string(&event).unwrap();
        assert!(text.starts_with(r#"{"event":"channel_in""#));
        assert!(!text.contains("raw"));
        let back: BenchEvent = serde_json::from_str(&text).unwrap();
        assert_eq!(back, event);
    }

    #[test]
    fn the_log_shows_acts_and_ticks_and_skips_the_wire() {
        let act = BenchEvent::AgentRequest {
            request: json!({ "act": { "gestures": [{ "press": "flight.main_drive" }, { "aim": "camera.camera_rotate", "delta": [5, 0], "ticks": 3 }], "ticks": 30 } }),
        };
        assert_eq!(
            render(&act).unwrap(),
            "act   +30 ticks  press flight.main_drive, aim camera.camera_rotate [5,0] x3"
        );
        assert!(render(&BenchEvent::ChannelOut {
            line: json!({ "tick": 1 })
        })
        .is_none());
        assert!(render(&BenchEvent::AgentRequest {
            request: json!({ "observe": {} })
        })
        .is_none());
        let tick = BenchEvent::ChannelIn {
            tick: 30,
            observation: json!({
                "me": { "id": "player", "health": { "current": 800, "max": 800 }, "speed_mps": 12.0, "combat_lock": "raider_1" },
                "contacts": [{ "id": "raider_1", "distance_m": 2800.0, "bearing_deg": [-3.0, 0.5], "health": { "current": 600, "max": 600 }, "defeated": false }],
                "outcome": null
            }),
            raw: None,
            errors: vec![json!({ "error": "tick must increase", "line": 4 })],
        };
        let line = render(&tick).unwrap();
        assert!(line.starts_with("tick      30  player hp 800/800 spd 12.0 m/s lock raider_1 | raider_1 2800.0 m brg [-3.0,0.5] hp 600/600"));
        assert!(line.contains("game refused line 4: \"tick must increase\""));
    }

    #[test]
    fn a_quiet_bus_swallows_events_and_keeps_time() {
        let bus = Bus::quiet();
        bus.emit(BenchEvent::Note {
            text: "hello".into(),
        });
        assert!(bus.elapsed() >= 0.0);
    }
}
