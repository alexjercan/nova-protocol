//! The baseline: a scripted hunter, in process.
//! Close, raise, aim, lock, fire. It speaks the referee protocol like any other
//! client, so its audit reads the same as pi's.

use serde_json::{json, Value};

use crate::{game::GameChannel, referee::Referee};

/// Beyond this range the hunter burns the main drive toward its target.
const CLOSE_TO_M: f64 = 2500.0;
/// Ticks per act: half a second of world between decisions.
const ACT_TICKS: u64 = 30;
/// Aim delta per degree of turn, the figure the manual gives for
/// `camera.camera_rotate`.
const AIM_DELTA_PER_DEG: f64 = 27.0;
/// Share of the screen error one act corrects. Under 1, so a gain that is off
/// for the raised camera still converges instead of swinging past.
const AIM_GAIN: f64 = 0.7;
/// Ticks one act's correction is spread over.
const AIM_TICKS: u64 = 5;
/// Screen error, in degrees, the hunter leaves alone: well inside the
/// radar's pick tolerance of about one degree.
const AIM_DEADBAND_DEG: f64 = 0.3;

/// Play until the referee ends the run, or until no hostile is left and the
/// hunter calls `finish`. Returns the stop reason for the front.
pub fn run<G: GameChannel>(referee: &mut Referee<G>) -> String {
    let mut seen_hostile = false;
    loop {
        if referee.is_over() {
            return "agent_exit".into();
        }
        let view = referee.observe(&[]);
        seen_hostile |= hostiles(&view).next().is_some();
        let request = match decide(&view, seen_hostile) {
            Decision::Act(gestures) => {
                json!({ "act": { "gestures": gestures, "ticks": ACT_TICKS } })
            }
            Decision::Finish(report) => json!({ "finish": { "status": "done", "report": report } }),
        };
        referee.handle(&request);
    }
}

enum Decision {
    Act(Vec<Value>),
    Finish(String),
}

/// The hostile contacts still standing.
fn hostiles(view: &Value) -> impl Iterator<Item = &Value> {
    view["contacts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|contact| contact["allegiance"] == "Enemy" && contact["defeated"] != true)
}

/// The policy: one act's worth of gestures from one view. A world with no
/// hostile yet is waited on (a scenario spawns over its first ticks); a
/// world whose hostiles are all down is finished.
fn decide(view: &Value, seen_hostile: bool) -> Decision {
    let me = &view["me"];
    let target = hostiles(view).min_by(|a, b| {
        a["distance_m"]
            .as_f64()
            .unwrap_or(f64::MAX)
            .total_cmp(&b["distance_m"].as_f64().unwrap_or(f64::MAX))
    });
    let Some(target) = target else {
        return if seen_hostile {
            Decision::Finish("no hostile contact remains".into())
        } else {
            Decision::Act(vec![])
        };
    };
    let live: Vec<&str> = view["inputs"]["live"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let held: Vec<&str> = view["inputs"]["held"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if !live.contains(&"targeting.radar_hold") {
        return Decision::Act(vec![]);
    }
    let distance = target["distance_m"].as_f64().unwrap_or(0.0);
    let locked = me["combat_lock"].as_str().is_some();
    let mut gestures = Vec::new();
    let mut want = |wire: &str, condition: bool| {
        let down = held.contains(&wire);
        if condition && !down {
            gestures.push(json!({ "press": wire }));
        } else if !condition && down {
            gestures.push(json!({ "release": wire }));
        }
    };
    want("flight.main_drive", distance > CLOSE_TO_M);
    // Stance up before the sweep: the radar commits a COMBAT lock only while
    // weapons are raised.
    want("targeting.combat_stance", true);
    want("targeting.radar_hold", !locked);
    for section in me["sections"].as_array().into_iter().flatten() {
        let kind = section["weapon"]["kind"].as_str();
        let Some(id) = section["id"].as_str() else {
            continue;
        };
        if matches!(kind, Some("turret" | "railgun")) && section["alive"] != false {
            want(&format!("section.{id}"), locked);
        }
    }
    // The radar locks what is under the screen centre, so the hunter puts
    // the target there for the dwell to charge on.
    if let Some(aim) = aim_at(target).filter(|_| !locked) {
        gestures.push(aim);
    }
    Decision::Act(gestures)
}

/// The `camera.camera_rotate` aim that walks `target`'s `screen_deg` toward
/// `[0, 0]`, or `None` when it is already there or the view carries no screen
/// bearing. Positive delta turns right and pitches down.
fn aim_at(target: &Value) -> Option<Value> {
    let screen = target["screen_deg"].as_array()?;
    let (azimuth, elevation) = (screen.first()?.as_f64()?, screen.get(1)?.as_f64()?);
    if azimuth.hypot(elevation) <= AIM_DEADBAND_DEG {
        return None;
    }
    let per_tick = AIM_GAIN * AIM_DELTA_PER_DEG / AIM_TICKS as f64;
    Some(json!({
        "aim": "camera.camera_rotate",
        "delta": [azimuth * per_tick, -elevation * per_tick],
        "ticks": AIM_TICKS,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(distance: f64, lock: Option<&str>, held: &[&str]) -> Value {
        json!({
            "me": {
                "combat_lock": lock,
                "sections": [
                    { "id": "turret_port", "alive": true, "weapon": { "kind": "turret" } },
                    { "id": "hull", "alive": true }
                ]
            },
            "contacts": [{ "id": "raider_1", "allegiance": "Enemy", "defeated": false, "distance_m": distance }],
            "inputs": { "live": ["targeting.radar_hold"], "held": held }
        })
    }

    #[test]
    fn the_hunter_closes_raises_locks_then_fires() {
        let Decision::Act(gestures) = decide(&view(2800.0, None, &[]), true) else {
            panic!("acts")
        };
        assert_eq!(
            gestures,
            vec![
                json!({ "press": "flight.main_drive" }),
                json!({ "press": "targeting.combat_stance" }),
                json!({ "press": "targeting.radar_hold" }),
            ]
        );
        let Decision::Act(gestures) = decide(
            &view(
                2000.0,
                Some("raider_1"),
                &[
                    "flight.main_drive",
                    "targeting.combat_stance",
                    "targeting.radar_hold",
                ],
            ),
            true,
        ) else {
            panic!("acts")
        };
        assert_eq!(
            gestures,
            vec![
                json!({ "release": "flight.main_drive" }),
                json!({ "release": "targeting.radar_hold" }),
                json!({ "press": "section.turret_port" }),
            ]
        );
    }

    #[test]
    fn the_hunter_turns_the_screen_centre_onto_the_target_until_it_locks() {
        let mut off_centre = view(2000.0, None, &[]);
        off_centre["contacts"][0]["screen_deg"] = json!([3.0, 2.0]);
        let Decision::Act(gestures) = decide(&off_centre, true) else {
            panic!("acts")
        };
        let aim = gestures
            .iter()
            .find(|gesture| gesture["aim"] == "camera.camera_rotate")
            .expect("an unlocked target off the screen centre is aimed at");
        assert!(
            aim["delta"][0].as_f64().unwrap() > 0.0,
            "right toward +azimuth"
        );
        assert!(
            aim["delta"][1].as_f64().unwrap() < 0.0,
            "up toward +elevation"
        );

        let mut centred = off_centre.clone();
        centred["contacts"][0]["screen_deg"] = json!([0.1, -0.1]);
        let mut locked = off_centre.clone();
        locked["me"]["combat_lock"] = json!("raider_1");
        for view in [centred, locked] {
            let Decision::Act(gestures) = decide(&view, true) else {
                panic!("acts")
            };
            assert!(gestures.iter().all(|gesture| gesture.get("aim").is_none()));
        }
    }

    #[test]
    fn with_no_hostile_left_the_hunter_finishes_but_an_empty_world_is_waited_on() {
        let mut done = view(100.0, None, &[]);
        done["contacts"][0]["defeated"] = json!(true);
        assert!(matches!(decide(&done, true), Decision::Finish(_)));
        assert!(matches!(decide(&done, false), Decision::Act(gestures) if gestures.is_empty()));
        assert!(hostiles(&done).next().is_none());
        assert!(hostiles(&view(100.0, None, &[])).next().is_some());
    }
}
