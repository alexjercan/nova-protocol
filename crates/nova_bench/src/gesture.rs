//! The agent's vocabulary: gestures without ticks, expanded onto the wire
//! with ticks the referee stamps. Pure - a test expands and reads the lines.
//!
//! The channel needs a tick on every line and refuses a past one. An agent
//! that had to manage that counter would divert tokens to bookkeeping and,
//! worse, could stall the clock by mistake. So the agent speaks relative
//! gestures and the referee owns the schedule.

use std::collections::BTreeSet;

use serde_json::{json, Value};

/// The longest an `aim` may be held, in ticks.
///
/// A minute of game time at `TICKS_PER_SECOND`, which is already far longer
/// than any gesture a run has a use for. The bound exists because `ticks`
/// arrives straight off the model through a schema that types it as `Any`, and
/// `expand` writes ONE WIRE LINE PER TICK: a plausible unit slip - ticks given
/// as milliseconds - turns a single aim into millions of lines written to the
/// game's stdin and mirrored to the audit log, with no deadline check in the
/// send loop to interrupt it. Refused rather than clamped, so the model is told
/// what it got wrong instead of silently aiming for a different span than it
/// asked for.
pub const MAX_AIM_TICKS: u64 = 60 * 60;

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

    /// The wire name this gesture drives, for the gestures that name one.
    fn wire(&self) -> Option<&str> {
        match self {
            Self::Press(wire) | Self::Release(wire) | Self::Tap(wire) => Some(wire),
            Self::Aim { wire, .. } => Some(wire),
            Self::Command(_) | Self::Text(_) | Self::Key(_) | Self::Pointer(_) => None,
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

/// Wire names that are two readings of ONE physical key. The game binds
/// `radar_clear` as a tap and `radar_hold` as a hold on the same key and
/// threshold, by design: a short press clears, a long one searches.
///
/// One act stamps its instant gestures onto one tick, so an act carrying both
/// names hands the key two contradictory readings and the tap never fires.
/// Nothing downstream can report that - a press has no verdict to give - so
/// the expander refuses the pair here, where the agent still gets a message it
/// can act on.
/// Parse the `gestures` array of an `act` request. Every element is one
/// object with exactly one verb key; the error names the element and the
/// reason so an agent can correct itself.
///
/// This is a SHAPE check only. Whether two of the gestures fight over one
/// physical key depends on the bindings the game is running, so it is
/// [`check_shared_keys`], which the referee calls with the pairs the world
/// reported.
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

/// Refuse an act that drives both readings of one key.
///
/// `shared` is the world's own `inputs.shared`: each pair is an action and the
/// shadow that follows it onto the same physical key - a short press read
/// against a long one. An act driving both hands the rig contradictory input
/// and neither reading fires cleanly. The pairs are NOT a table here, because
/// the game declares the relation and a driver copy would go stale the day
/// someone declares a second one.
pub fn check_shared_keys(gestures: &[Gesture], shared: &[[String; 2]]) -> Result<(), String> {
    let driven: Vec<&str> = gestures.iter().filter_map(Gesture::wire).collect();
    for [first, second] in shared {
        if driven.contains(&first.as_str()) && driven.contains(&second.as_str()) {
            return Err(format!(
                "`{first}` and `{second}` are two readings of one key: an act that drives both \
                 hands it contradictory input and neither reading lands. Put the second one in \
                 the next act."
            ));
        }
    }
    Ok(())
}

/// The `input.shared` pairs of a raw snapshot, as [`check_shared_keys`] takes
/// them. A snapshot without the key reports no pairs, which is what a world
/// with no shadow action reports too.
pub fn shared_keys(snapshot: &Value) -> Vec<[String; 2]> {
    snapshot["input"]["shared"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|pair| {
            let first = pair.get(0)?.as_str()?.to_string();
            let second = pair.get(1)?.as_str()?.to_string();
            Some([first, second])
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
                .filter(|ticks| (1..=MAX_AIM_TICKS).contains(ticks))
                .ok_or(format!(
                    "`aim` takes `ticks` as a whole number from 1 to {MAX_AIM_TICKS}"
                ))?;
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
                last = last.max(first.saturating_add(*ticks).saturating_sub(1));
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

    /// Both refusals of the first tutorial victory were this pair, and the
    /// silent trigger that used to hint at it is gone: the act is refused
    /// here instead, with a message that says what to do next.
    #[test]
    fn one_act_cannot_drive_both_readings_of_a_shared_key() {
        let world = json!({
            "input": { "shared": [["targeting.radar_hold", "targeting.radar_clear"]] }
        });
        let pairs = shared_keys(&world);
        let both = parse_gestures(&json!([
            { "release": "targeting.radar_hold" },
            { "tap": "targeting.radar_clear" },
        ]))
        .unwrap();
        let error = check_shared_keys(&both, &pairs).unwrap_err();
        assert!(error.contains("two readings of one key"), "{error}");
        assert!(error.contains("next act"), "{error}");

        // Either one alone is ordinary.
        let alone = parse_gestures(&json!([{ "tap": "targeting.radar_clear" }])).unwrap();
        assert!(check_shared_keys(&alone, &pairs).is_ok());
        let unrelated = parse_gestures(&json!([
            { "release": "targeting.radar_hold" },
            { "press": "flight.main_drive" },
        ]))
        .unwrap();
        assert!(check_shared_keys(&unrelated, &pairs).is_ok());
    }

    /// The pairs come from the world, so a game that declares a second shadow
    /// is guarded without the bench being taught about it.
    #[test]
    fn a_pair_the_world_declares_is_guarded_without_a_table_here() {
        let world = json!({ "input": { "shared": [["flight.stop", "flight.stop_hard"]] } });
        let gestures = parse_gestures(&json!([
            { "tap": "flight.stop" },
            { "tap": "flight.stop_hard" },
        ]))
        .unwrap();
        let error = check_shared_keys(&gestures, &shared_keys(&world)).unwrap_err();
        assert!(error.contains("flight.stop_hard"), "{error}");

        // And a world that declares none guards nothing.
        assert!(check_shared_keys(&gestures, &shared_keys(&json!({}))).is_ok());
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

    /// `ticks` arrives untyped off the model, and every tick is a wire line.
    #[test]
    fn an_aim_held_longer_than_the_cap_is_refused_with_the_cap_in_the_message() {
        let slip = json!({ "aim": "look", "delta": [1.0, 0.0], "ticks": 2_000_000 });
        let error = parse_gesture(&slip).expect_err("a unit slip must not expand");
        assert!(
            error.contains(&MAX_AIM_TICKS.to_string()),
            "the model cannot correct itself from `{error}`"
        );

        let held = json!({ "aim": "look", "delta": [1.0, 0.0], "ticks": MAX_AIM_TICKS });
        assert!(
            parse_gesture(&held).is_ok(),
            "the cap itself is a legal aim"
        );
    }
}
