//! The wire's envelope: one JSON object per line, `tick` plus at most one
//! payload key, parsed into a typed [`Lane`] or refused with the message the
//! design record fixed.
//!
//! Parsing is pure and worldless - whether a NAME resolves (an action, a
//! section, a UI target) is the applier's question, answered against the live
//! world in `apply`. This module only answers "is this line the wire's shape".

use bevy::prelude::*;
use nova_input::prelude::InputPhase;

/// The keys the parent design page reserved for the console vocabulary. A line
/// carrying one is refused with the follow-up task id, so a driver written
/// against the future schema fails loudly instead of half-working.
const RESERVED: [&str; 2] = ["action", "command"];

/// One parsed line: an optional schedule tick and the lane payload. A line
/// with no payload is the bare-tick step instruction.
#[derive(Debug, Clone, PartialEq)]
pub struct Envelope {
    /// The tick this line schedules itself for. Required in step mode.
    pub tick: Option<u64>,
    /// The lane payload; `None` is a bare tick.
    pub lane: Option<Lane>,
}

/// The five input lanes.
#[derive(Debug, Clone, PartialEq)]
pub enum Lane {
    /// A named verb or a `section.<id>`, pressed or released.
    Input {
        /// The addressed `<group>.<name>` wire name.
        wire: String,
        /// Which half of the press.
        phase: InputPhase,
    },
    /// A per-frame delta on a named axis action.
    Aim {
        /// The addressed `<group>.<name>` wire name.
        wire: String,
        /// How far, this frame. The registry decides which way.
        delta: Vec2,
    },
    /// Characters into whatever has focus.
    Text(String),
    /// One literal editing key, as a tap.
    Key(String),
    /// A pointer gesture.
    Pointer(PointerCmd),
}

/// The pointer lane's gestures.
#[derive(Debug, Clone, PartialEq)]
pub enum PointerCmd {
    /// Move to a named widget or to raw logical pixels.
    To(PointerTarget),
    /// Push a button down where the pointer is.
    Press(MouseButton),
    /// Let it back up.
    Release(MouseButton),
    /// Turn the wheel by this many lines.
    Wheel(f32),
}

/// Where a pointer move lands.
#[derive(Debug, Clone, PartialEq)]
pub enum PointerTarget {
    /// A UI `Name`, resolved through the same census the snapshot prints.
    Name(String),
    /// Raw logical pixels - the fallback, and the only address the CRT glass
    /// has (its census speaks window px, not names).
    Px(Vec2),
}

/// Parse one wire line. `Err` carries the error message the runner echoes on
/// stdout with the line number.
pub fn parse_line(raw: &str) -> Result<Envelope, String> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|error| format!("not a JSON object: {error}"))?;
    let Some(object) = value.as_object() else {
        return Err(format!("not a JSON object: {value}"));
    };

    for reserved in RESERVED {
        if object.contains_key(reserved) {
            return Err(format!(
                "`{reserved}` is reserved; the console vocabulary is task 20260827-120347"
            ));
        }
    }

    let tick = match object.get("tick") {
        None => None,
        Some(tick) => Some(
            tick.as_u64()
                .ok_or_else(|| format!("`tick` is not a whole number: {tick}"))?,
        ),
    };

    // `phase` rides beside `input`; everything else is one payload key.
    let payload: Vec<(&String, &serde_json::Value)> = object
        .iter()
        .filter(|(key, _)| *key != "tick" && *key != "phase")
        .collect();
    if payload.len() > 1 {
        return Err("one payload key per line".to_string());
    }
    let Some((key, value)) = payload.first() else {
        return Ok(Envelope { tick, lane: None });
    };

    let lane = match key.as_str() {
        "input" => Lane::Input {
            wire: as_string(value, "input")?,
            phase: parse_phase(object.get("phase"))?,
        },
        "aim" => parse_aim(value)?,
        "text" => Lane::Text(as_string(value, "text")?),
        "key" => Lane::Key(as_string(value, "key")?),
        "pointer" => Lane::Pointer(parse_pointer(value)?),
        _ => return Err(format!("unknown lane [{key:?}]")),
    };
    Ok(Envelope {
        tick,
        lane: Some(lane),
    })
}

fn as_string(value: &serde_json::Value, lane: &str) -> Result<String, String> {
    value
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| format!("`{lane}` takes a string, not {value}"))
}

fn parse_phase(phase: Option<&serde_json::Value>) -> Result<InputPhase, String> {
    match phase.and_then(serde_json::Value::as_str) {
        None | Some("start") => Ok(InputPhase::Press),
        Some("stop") => Ok(InputPhase::Release),
        Some(other) => Err(format!("phase must be `start` or `stop`, not `{other}`")),
    }
}

fn parse_aim(value: &serde_json::Value) -> Result<Lane, String> {
    let (Some(name), Some(delta)) = (
        value.get("name").and_then(serde_json::Value::as_str),
        value.get("delta").and_then(parse_vec2),
    ) else {
        return Err("aim payload needs name and a [x, y] delta".to_string());
    };
    Ok(Lane::Aim {
        wire: name.to_string(),
        delta,
    })
}

fn parse_pointer(value: &serde_json::Value) -> Result<PointerCmd, String> {
    let Some(object) = value.as_object() else {
        return Err("pointer payload needs to/press/release/wheel".to_string());
    };
    if let Some(target) = object.get("to") {
        let target = match target {
            serde_json::Value::String(name) => PointerTarget::Name(name.clone()),
            _ => PointerTarget::Px(
                parse_vec2(target)
                    .ok_or("pointer `to` takes a Name or [x, y] logical pixels".to_string())?,
            ),
        };
        return Ok(PointerCmd::To(target));
    }
    if let Some(button) = object.get("press") {
        return Ok(PointerCmd::Press(parse_button(button)?));
    }
    if let Some(button) = object.get("release") {
        return Ok(PointerCmd::Release(parse_button(button)?));
    }
    if let Some(lines) = object.get("wheel") {
        let lines = lines
            .as_f64()
            .ok_or_else(|| format!("pointer `wheel` takes a number, not {lines}"))?;
        #[expect(clippy::cast_possible_truncation, reason = "wheel lines are small")]
        return Ok(PointerCmd::Wheel(lines as f32));
    }
    Err("pointer payload needs to/press/release/wheel".to_string())
}

fn parse_button(value: &serde_json::Value) -> Result<MouseButton, String> {
    match value.as_str() {
        Some("left") => Ok(MouseButton::Left),
        Some("right") => Ok(MouseButton::Right),
        Some("middle") => Ok(MouseButton::Middle),
        _ => Err(format!("no mouse button named {value}")),
    }
}

fn parse_vec2(value: &serde_json::Value) -> Option<Vec2> {
    let pair = value.as_array()?;
    let [x, y] = pair.as_slice() else {
        return None;
    };
    #[expect(clippy::cast_possible_truncation, reason = "wire floats are f32-sized")]
    Some(Vec2::new(x.as_f64()? as f32, y.as_f64()? as f32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_named_input_line_parses_with_its_phase() {
        let line = parse_line(r#"{"tick":120,"input":"targeting.radar_hold","phase":"start"}"#)
            .expect("the schema line from the design page parses");
        assert_eq!(line.tick, Some(120));
        assert_eq!(
            line.lane,
            Some(Lane::Input {
                wire: "targeting.radar_hold".to_string(),
                phase: InputPhase::Press,
            })
        );

        let stop = parse_line(r#"{"tick":180,"input":"targeting.radar_hold","phase":"stop"}"#)
            .expect("the stop half parses");
        assert!(matches!(
            stop.lane,
            Some(Lane::Input {
                phase: InputPhase::Release,
                ..
            })
        ));
    }

    #[test]
    fn a_bare_tick_is_the_step_instruction() {
        let line = parse_line(r#"{"tick":500}"#).expect("a bare tick parses");
        assert_eq!(line.tick, Some(500));
        assert_eq!(line.lane, None);
    }

    #[test]
    fn the_reserved_lanes_are_refused_with_the_follow_up_task() {
        let error = parse_line(r#"{"tick":1,"action":"radar"}"#).unwrap_err();
        assert_eq!(
            error,
            "`action` is reserved; the console vocabulary is task 20260827-120347"
        );
        assert!(parse_line(r#"{"tick":1,"command":"status"}"#).is_err());
    }

    #[test]
    fn two_payload_keys_on_one_line_are_refused() {
        let error = parse_line(r#"{"tick":1,"input":"a.b","text":"hi"}"#).unwrap_err();
        assert_eq!(error, "one payload key per line");
    }

    #[test]
    fn the_pointer_lane_takes_a_name_or_raw_pixels() {
        assert_eq!(
            parse_line(r#"{"tick":400,"pointer":{"to":"Resume"}}"#)
                .unwrap()
                .lane,
            Some(Lane::Pointer(PointerCmd::To(PointerTarget::Name(
                "Resume".to_string()
            ))))
        );
        assert_eq!(
            parse_line(r#"{"tick":410,"pointer":{"to":[640,360]}}"#)
                .unwrap()
                .lane,
            Some(Lane::Pointer(PointerCmd::To(PointerTarget::Px(Vec2::new(
                640.0, 360.0
            )))))
        );
        assert_eq!(
            parse_line(r#"{"tick":411,"pointer":{"wheel":-2.0}}"#)
                .unwrap()
                .lane,
            Some(Lane::Pointer(PointerCmd::Wheel(-2.0)))
        );
        assert!(parse_line(r#"{"tick":1,"pointer":{"poke":1}}"#).is_err());
    }

    #[test]
    fn an_unknown_lane_and_a_non_object_line_are_refused() {
        assert_eq!(
            parse_line(r#"{"tick":1,"warp":9}"#).unwrap_err(),
            "unknown lane [\"warp\"]"
        );
        assert!(parse_line("[1,2]").unwrap_err().starts_with("not a JSON"));
        assert!(parse_line("garbage").unwrap_err().starts_with("not a JSON"));
    }
}
