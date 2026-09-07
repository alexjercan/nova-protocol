//! Replay: feed a recorded audit's wire lines to a fresh game and check
//! that the world ends where the audit says it did. This is the channel's
//! replay guarantee applied to a whole run; with `--record` it is the
//! agent's run as a movie.
//!
//! Two plays of one seed are not byte-identical once rounds fly (how many
//! rounds land drifts between processes), so the verdict has a `close`
//! grade: same tick, same outcome, same ships down, positions and the
//! health of the ships still standing within tolerance. A defeated ship's
//! leftover health is not compared; it only says how hard the last volley
//! hit.

use std::{path::Path, process::ExitCode};

use serde_json::{json, Value};

use crate::{
    audit::{BenchEvent, Bus},
    cli::ReplayOptions,
    game::{
        game_exe, GameChannel, GameConfig, GameProcess, ScenarioTarget, BOOT_TIMEOUT, STEP_TIMEOUT,
    },
    movie,
    observation::condense,
    paths,
};

/// What the audit holds that a replay needs.
#[derive(Debug, Clone, PartialEq)]
pub struct Recording {
    /// The scenario token as the run was launched.
    pub scenario: String,
    /// The seed, when the run fixed one.
    pub seed: Option<u64>,
    /// Every wire line, in order.
    pub lines: Vec<Value>,
    /// The last observation the run recorded.
    pub last: Option<Value>,
}

/// Read a recording out of an audit file's text.
pub fn recording(text: &str) -> Result<Recording, String> {
    let mut scenario = None;
    let mut seed = None;
    let mut lines = Vec::new();
    let mut last = None;
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let event: BenchEvent = serde_json::from_str(line)
            .map_err(|error| format!("audit line {}: {error}", index + 1))?;
        match event {
            BenchEvent::RunStart {
                scenario: token,
                seed: run_seed,
                ..
            } => {
                scenario = Some(token);
                seed = run_seed;
            }
            BenchEvent::ChannelOut { line } => lines.push(line),
            BenchEvent::ChannelIn {
                tick,
                mut observation,
                ..
            } => {
                observation["tick"] = json!(tick);
                last = Some(observation);
            }
            _ => {}
        }
    }
    Ok(Recording {
        scenario: scenario.ok_or("the audit has no run_start event")?,
        seed,
        lines,
        last,
    })
}

/// The fields a replay compares: where the ships ended and how they fared.
pub fn fingerprint(observation: &Value) -> Value {
    let contacts: Vec<Value> = observation["contacts"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|contact| {
            json!({
                "id": contact["id"],
                "distance_m": contact["distance_m"],
                "health": contact["health"],
                "defeated": contact["defeated"],
            })
        })
        .collect();
    json!({
        "tick": observation["tick"],
        "me": {
            "position_m": observation["me"]["position_m"],
            "health": observation["me"]["health"],
        },
        "contacts": contacts,
        "outcome": observation["outcome"],
    })
}

/// How a replay's end compares with the recording's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Byte for byte.
    Match,
    /// Same tick, outcome and ships down; positions and the health of the
    /// ships still standing within tolerance.
    Close,
    /// Something else happened.
    Mismatch,
}

impl Verdict {
    /// The word the run ends on.
    pub fn label(self) -> &'static str {
        match self {
            Self::Match => "replay_match",
            Self::Close => "replay_close",
            Self::Mismatch => "replay_mismatch",
        }
    }
}

/// Relative tolerance on a number for [`Verdict::Close`].
const CLOSE_RELATIVE: f64 = 0.05;
/// Absolute tolerance in meters, so a ship at rest may still jitter.
const CLOSE_ABSOLUTE: f64 = 10.0;

/// Grade two fingerprints.
pub fn compare(expected: &Value, actual: &Value) -> Verdict {
    if expected == actual {
        return Verdict::Match;
    }
    if expected["tick"] != actual["tick"] || expected["outcome"] != actual["outcome"] {
        return Verdict::Mismatch;
    }
    let close = |a: &Value, b: &Value| -> bool {
        match (a.as_f64(), b.as_f64()) {
            (Some(a), Some(b)) => {
                (a - b).abs() <= CLOSE_ABSOLUTE
                    || (a - b).abs() <= CLOSE_RELATIVE * a.abs().max(b.abs())
            }
            _ => a == b,
        }
    };
    let me_close = (0..3).all(|axis| {
        close(
            &expected["me"]["position_m"][axis],
            &actual["me"]["position_m"][axis],
        )
    }) && close(
        &expected["me"]["health"]["current"],
        &actual["me"]["health"]["current"],
    );
    if !me_close {
        return Verdict::Mismatch;
    }
    let expected_contacts = expected["contacts"].as_array().cloned().unwrap_or_default();
    let actual_contacts = actual["contacts"].as_array().cloned().unwrap_or_default();
    if expected_contacts.len() != actual_contacts.len() {
        return Verdict::Mismatch;
    }
    let contacts_close = expected_contacts
        .iter()
        .zip(&actual_contacts)
        .all(|(a, b)| {
            let standing = a["defeated"] != json!(true);
            a["id"] == b["id"]
                && a["defeated"] == b["defeated"]
                && close(&a["distance_m"], &b["distance_m"])
                && (!standing || close(&a["health"]["current"], &b["health"]["current"]))
        });
    if contacts_close {
        Verdict::Close
    } else {
        Verdict::Mismatch
    }
}

/// Drive the recording against a fresh game and compare the end.
pub fn replay(options: &ReplayOptions) -> Result<ExitCode, String> {
    let text = std::fs::read_to_string(&options.audit)
        .map_err(|error| format!("could not read {}: {error}", options.audit.display()))?;
    let recording = recording(&text)?;
    let root = paths::repo_root();
    let out = options.out.clone().unwrap_or_else(|| {
        options
            .audit
            .parent()
            .unwrap_or(Path::new("."))
            .join("replay")
    });
    let profile_dir = out.join("profile");
    std::fs::create_dir_all(&profile_dir)
        .map_err(|error| format!("could not create {}: {error}", out.display()))?;
    let bus = Bus::open(&out.join("audit.jsonl"), options.ui)?;
    bus.emit(BenchEvent::RunStart {
        scenario: recording.scenario.clone(),
        agent: "replay".into(),
        goal: format!("replay {}", options.audit.display()),
        seed: recording.seed,
        budget: json!({ "ticks": null, "turns": null, "deadline": null }),
        run_dir: out.display().to_string(),
    });
    let config = GameConfig {
        exe: game_exe()?,
        root,
        scenario: ScenarioTarget::parse(&recording.scenario),
        seed: recording.seed,
        record: options.record.clone(),
        profile_dir,
        log_path: out.join("game.log"),
    };
    let mut game = GameProcess::spawn(&config)?;
    let held = std::collections::BTreeSet::new();
    let mut last = None;
    let mut first = true;
    for line in &recording.lines {
        bus.emit(BenchEvent::ChannelOut { line: line.clone() });
        game.send(line)?;
        let bare = line
            .as_object()
            .is_some_and(|object| object.len() == 1 && object.contains_key("tick"));
        if !bare {
            continue;
        }
        let timeout = if first { BOOT_TIMEOUT } else { STEP_TIMEOUT };
        first = false;
        let answer = game.read_answer(timeout)?;
        let tick = line["tick"].as_u64().unwrap_or(0);
        let mut observation = condense(&answer.snapshot, &held, &[]);
        observation["tick"] = json!(tick);
        bus.emit(BenchEvent::ChannelIn {
            tick,
            observation: observation.clone(),
            raw: None,
            errors: answer.errors,
        });
        last = Some(observation);
    }
    game.close();
    let movie = options
        .record
        .as_deref()
        .map(|frames| movie::report(&bus, frames));

    let expected = recording
        .last
        .as_ref()
        .map(fingerprint)
        .unwrap_or(Value::Null);
    let actual = last.as_ref().map(fingerprint).unwrap_or(Value::Null);
    let verdict = compare(&expected, &actual);
    let reason = verdict.label();
    bus.emit(BenchEvent::RunEnd {
        reason: reason.into(),
        score: json!({ "verdict": reason, "expected": expected, "actual": actual }),
    });
    println!("{}\n{reason}", out.display());
    if verdict != Verdict::Match {
        println!("expected {expected}\nactual   {actual}");
    }
    if let Some(line) = movie {
        println!("{line}");
    }
    Ok(if verdict == Verdict::Mismatch {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recording_keeps_the_scenario_the_wire_and_the_last_view() {
        let text = [
            r#"{"t":0.0,"event":"run_start","scenario":"tutorial","agent":"baseline","goal":"g","seed":7,"budget":{},"run_dir":"/r"}"#,
            r#"{"t":0.1,"event":"channel_out","line":{"tick":1}}"#,
            r#"{"t":0.2,"event":"channel_in","tick":1,"observation":{"tick":1,"me":null}}"#,
            r#"{"t":0.3,"event":"agent_text","text":"hi"}"#,
            r#"{"t":0.4,"event":"channel_out","line":{"tick":2,"input":"x","phase":"start"}}"#,
            r#"{"t":0.5,"event":"channel_out","line":{"tick":31}}"#,
            r#"{"t":0.6,"event":"channel_in","tick":31,"observation":{"me":{"position_m":[1,2,3]}}}"#,
        ]
        .join("\n");
        let recording = recording(&text).unwrap();
        assert_eq!(recording.scenario, "tutorial");
        assert_eq!(recording.seed, Some(7));
        assert_eq!(recording.lines.len(), 3);
        let last = recording.last.unwrap();
        assert_eq!(last["me"]["position_m"], json!([1, 2, 3]));
        assert_eq!(last["tick"], 31);
        assert!(super::recording("").is_err());
    }

    #[test]
    fn a_replay_that_drifts_a_little_is_close_and_a_different_end_is_a_mismatch() {
        let end = |hp: f64, defeated: bool, tick: u64| {
            json!({
                "tick": tick,
                "me": { "position_m": [0.0, 5.3, -1506.6], "health": { "current": 10260.0, "max": 10260.0 } },
                "contacts": [{ "id": "raider", "distance_m": 1111.8, "health": { "current": hp, "max": 6240.0 }, "defeated": defeated }],
                "outcome": { "kind": "Victory", "message": "" }
            })
        };
        assert_eq!(
            compare(&end(3176.5, true, 661), &end(3176.5, true, 661)),
            Verdict::Match
        );
        assert_eq!(
            compare(&end(3176.5, true, 661), &end(3283.6, true, 661)),
            Verdict::Close
        );
        assert_eq!(
            compare(&end(3176.5, true, 661), &end(3176.5, false, 661)),
            Verdict::Mismatch
        );
        assert_eq!(
            compare(&end(3176.5, true, 661), &end(3176.5, true, 691)),
            Verdict::Mismatch
        );
        assert_eq!(
            compare(&end(3176.5, true, 661), &end(1000.0, true, 661)),
            Verdict::Close,
            "a defeated ship's leftover health is noise"
        );
        assert_eq!(
            compare(&end(3176.5, false, 661), &end(1000.0, false, 661)),
            Verdict::Mismatch,
            "a standing ship's health counts"
        );
    }

    #[test]
    fn the_fingerprint_keeps_positions_health_and_the_outcome_only() {
        let print = fingerprint(&json!({
            "tick": 5,
            "me": { "position_m": [1, 2, 3], "health": { "current": 1, "max": 2 }, "speed_mps": 9 },
            "contacts": [{ "id": "r", "distance_m": 10, "health": {}, "defeated": false, "bearing_deg": [1, 2] }],
            "outcome": null
        }));
        assert_eq!(print["me"]["position_m"], json!([1, 2, 3]));
        assert!(print["me"].get("speed_mps").is_none());
        assert!(print["contacts"][0].get("bearing_deg").is_none());
    }
}
