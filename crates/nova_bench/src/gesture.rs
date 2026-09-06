//! The agent's vocabulary: gestures without ticks, expanded onto the wire
//! with ticks the referee stamps. Pure - a test expands and reads the lines.
//!
//! The channel needs a tick on every line and refuses a past one. An agent
//! that had to manage that counter would divert tokens to bookkeeping and,
//! worse, could stall the clock by mistake. So the agent speaks relative
//! gestures and the referee owns the schedule.

use std::collections::BTreeSet;

use serde_json::{json, Value};

/// One thing the agent does. The wire name conventions are the channel's:
/// `<group>.<name>` for a registered action, `section.<id>` for a mount.
#[derive(Debug, Clone, PartialEq)]
pub enum Gesture {
    /// Start a named input and keep it held until released.
    Press(String),
    /// Stop a held input.
    Release(String),
    /// Start on one tick, stop on the next.
    Tap(String),
    /// Feed an axis action `delta` per tick, for `ticks` ticks.
    Aim {
        /// The axis action's wire name.
        wire: String,
        /// The per-tick delta, in the axis's own units (mouse pixels).
        delta: [f64; 2],
        /// How many consecutive ticks to feed it.
        ticks: u64,
    },
    /// One Command-shell line, as typed at the CRT prompt.
    Command(String),
    /// Characters into whatever has focus.
    Text(String),
    /// One literal editing key, tapped.
    Key(String),
    /// A pointer gesture, passed through to the channel as is.
    Pointer(Value),
}

impl Gesture {
    /// The wire payload (no tick) this gesture writes. `Tap` and `Aim` write
    /// several lines; this is the first.
    fn payload(&self) -> Value {
        match self {
            Self::Press(wire) => json!({ "input": wire, "phase": "start" }),
            Self::Release(wire) => json!({ "input": wire, "phase": "stop" }),
            Self::Tap(wire) => json!({ "input": wire, "phase": "start" }),
            Self::Aim { wire, delta, .. } => json!({ "aim": { "name": wire, "delta": delta } }),
            Self::Command(text) => json!({ "command": text }),
            Self::Text(text) => json!({ "text": text }),
            Self::Key(key) => json!({ "key": key }),
            Self::Pointer(pointer) => json!({ "pointer": pointer }),
        }
    }

    /// A one-line label for the audit and the log.
    pub fn label(&self) -> String {
        match self {
            Self::Press(wire) => format!("press {wire}"),
            Self::Release(wire) => format!("release {wire}"),
            Self::Tap(wire) => format!("tap {wire}"),
            Self::Aim { wire, delta, ticks } => {
                format!("aim {wire} [{}, {}] x{ticks}", delta[0], delta[1])
            }
            Self::Command(text) => format!("command {text:?}"),
            Self::Text(text) => format!("text {text:?}"),
            Self::Key(key) => format!("key {key}"),
            Self::Pointer(pointer) => format!("pointer {pointer}"),
        }
    }
}

/// Parse the `gestures` array of an `act` request. Every element is one
/// object with exactly one verb key; the error names the element and the
/// reason so an agent can correct itself.
pub fn parse_gestures(value: &Value) -> Result<Vec<Gesture>, String> {
    let Some(list) = value.as_array() else {
        return Err("`gestures` must be an array of gesture objects".into());
    };
    list.iter()
        .enumerate()
        .map(|(index, item)| {
            parse_gesture(item).map_err(|reason| format!("gesture {index}: {reason}"))
        })
        .collect()
}

fn parse_gesture(item: &Value) -> Result<Gesture, String> {
    let Some(object) = item.as_object() else {
        return Err(format!("not an object: {item}"));
    };
    let verbs: Vec<&String> = object
        .keys()
        .filter(|key| !matches!(key.as_str(), "delta" | "ticks"))
        .collect();
    let [verb] = verbs.as_slice() else {
        return Err(
            "one verb per gesture (press, release, tap, aim, command, text, key, pointer)".into(),
        );
    };
    let value = &object[verb.as_str()];
    let text = |what: &str| -> Result<String, String> {
        value
            .as_str()
            .map(ToString::to_string)
            .ok_or_else(|| format!("`{what}` takes a string, not {value}"))
    };
    match verb.as_str() {
        "press" => Ok(Gesture::Press(text("press")?)),
        "release" => Ok(Gesture::Release(text("release")?)),
        "tap" => Ok(Gesture::Tap(text("tap")?)),
        "aim" => {
            let wire = text("aim")?;
            let delta = object
                .get("delta")
                .and_then(Value::as_array)
                .filter(|pair| pair.len() == 2)
                .and_then(|pair| Some([pair[0].as_f64()?, pair[1].as_f64()?]))
                .ok_or("`aim` needs `delta: [x, y]`")?;
            let ticks = object
                .get("ticks")
                .map_or(Some(1), Value::as_u64)
                .filter(|ticks| *ticks >= 1)
                .ok_or("`aim` takes `ticks` as a whole number of at least 1")?;
            Ok(Gesture::Aim { wire, delta, ticks })
        }
        "command" => Ok(Gesture::Command(text("command")?)),
        "text" => Ok(Gesture::Text(text("text")?)),
        "key" => Ok(Gesture::Key(text("key")?)),
        "pointer" => Ok(Gesture::Pointer(value.clone())),
        other => Err(format!("unknown verb `{other}`")),
    }
}

/// The wire lines one `act` writes, each with its tick, ending in the bare
/// step instruction.
#[derive(Debug, Clone, PartialEq)]
pub struct Expansion {
    /// Every line, in the order it is written. The last is the bare tick.
    pub lines: Vec<Value>,
    /// The tick the world stands at once the step completes.
    pub end_tick: u64,
}

/// Stamp gestures onto the wire from `start` (the current tick) and run the
/// clock `ticks` further. Instant gestures land on the next tick, a tap's
/// release the one after, an aim on `ticks` consecutive ticks. The step runs
/// to `start + ticks` or to the last gesture's tick, whichever is later, so a
/// long aim is never cut short. `held` tracks what is down after this act.
pub fn expand(
    gestures: &[Gesture],
    start: u64,
    ticks: u64,
    held: &mut BTreeSet<String>,
) -> Expansion {
    let first = start + 1;
    let mut lines = Vec::new();
    let mut last = first;
    for gesture in gestures {
        match gesture {
            Gesture::Press(wire) => {
                held.insert(wire.clone());
                lines.push(stamp(first, gesture.payload()));
            }
            Gesture::Release(wire) => {
                held.remove(wire);
                lines.push(stamp(first, gesture.payload()));
            }
            Gesture::Tap(wire) => {
                lines.push(stamp(first, gesture.payload()));
                lines.push(stamp(first + 1, json!({ "input": wire, "phase": "stop" })));
                last = last.max(first + 1);
            }
            Gesture::Aim { ticks, .. } => {
                for offset in 0..*ticks {
                    lines.push(stamp(first + offset, gesture.payload()));
                }
                last = last.max(first + ticks - 1);
            }
            _ => lines.push(stamp(first, gesture.payload())),
        }
    }
    let end_tick = (start + ticks.max(1)).max(last);
    lines.push(json!({ "tick": end_tick }));
    Expansion { lines, end_tick }
}

fn stamp(tick: u64, mut payload: Value) -> Value {
    payload["tick"] = json!(tick);
    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gestures_parse_one_verb_each_and_name_the_bad_one() {
        let parsed = parse_gestures(&json!([
            { "press": "flight.main_drive" },
            { "tap": "targeting.radar_hold" },
            { "aim": "camera.camera_rotate", "delta": [40, 0], "ticks": 3 },
            { "command": "map" },
            { "pointer": { "to": "Resume" } },
        ]))
        .unwrap();
        assert_eq!(parsed[0], Gesture::Press("flight.main_drive".into()));
        assert_eq!(parsed[1], Gesture::Tap("targeting.radar_hold".into()));
        assert_eq!(
            parsed[2],
            Gesture::Aim {
                wire: "camera.camera_rotate".into(),
                delta: [40.0, 0.0],
                ticks: 3
            }
        );
        assert_eq!(parsed[3], Gesture::Command("map".into()));
        assert!(matches!(parsed[4], Gesture::Pointer(_)));

        let error = parse_gestures(&json!([{ "press": "a" }, { "warp": 9 }])).unwrap_err();
        assert_eq!(error, "gesture 1: unknown verb `warp`");
        assert!(parse_gestures(&json!([{ "press": "a", "tap": "b" }])).is_err());
        assert!(parse_gestures(&json!([{ "aim": "x" }])).is_err());
        assert!(parse_gestures(&json!({})).is_err());
    }

    #[test]
    fn an_act_stamps_next_tick_then_steps_and_tracks_what_is_held() {
        let mut held = BTreeSet::new();
        let expansion = expand(
            &[
                Gesture::Press("flight.main_drive".into()),
                Gesture::Tap("targeting.radar_hold".into()),
            ],
            100,
            30,
            &mut held,
        );
        assert_eq!(
            expansion.lines,
            vec![
                json!({ "tick": 101, "input": "flight.main_drive", "phase": "start" }),
                json!({ "tick": 101, "input": "targeting.radar_hold", "phase": "start" }),
                json!({ "tick": 102, "input": "targeting.radar_hold", "phase": "stop" }),
                json!({ "tick": 130 }),
            ]
        );
        assert_eq!(expansion.end_tick, 130);
        assert_eq!(held, BTreeSet::from(["flight.main_drive".to_string()]));

        let expansion = expand(
            &[Gesture::Release("flight.main_drive".into())],
            130,
            1,
            &mut held,
        );
        assert_eq!(expansion.end_tick, 131);
        assert!(held.is_empty());
    }

    #[test]
    fn a_long_aim_extends_the_step_and_a_zero_step_still_moves_one_tick() {
        let mut held = BTreeSet::new();
        let expansion = expand(
            &[Gesture::Aim {
                wire: "camera.camera_rotate".into(),
                delta: [10.0, 0.0],
                ticks: 5,
            }],
            0,
            2,
            &mut held,
        );
        assert_eq!(expansion.lines.len(), 6);
        assert_eq!(expansion.lines[4]["tick"], 5);
        assert_eq!(expansion.end_tick, 5);
        assert_eq!(expand(&[], 7, 0, &mut held).end_tick, 8);
    }
}
