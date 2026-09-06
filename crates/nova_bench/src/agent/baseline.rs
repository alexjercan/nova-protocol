//! The baseline: the channel task's `agent_loop.py` hunter, in process.
//! Close, raise, lock, fire. It speaks the referee protocol like any other
//! client, so its audit reads the same as pi's.

use serde_json::{json, Value};

use crate::{game::GameChannel, referee::Referee};

/// Beyond this range the hunter burns the main drive toward its target.
const CLOSE_TO_M: f64 = 2500.0;
/// Ticks per act: half a second of world between decisions.
const ACT_TICKS: u64 = 30;

/// Play until the referee ends the run, or until no hostile is left and the
/// hunter calls `finish`. Returns the stop reason for the front.
pub fn run<G: GameChannel>(referee: &mut Referee<G>) -> String {
    let mut seen_hostile = false;
    loop {
        if referee.is_over() {
            return "agent_exit".into();
        }
        let view = referee.observe();
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
    Decision::Act(gestures)
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
    fn with_no_hostile_left_the_hunter_finishes_but_an_empty_world_is_waited_on() {
        let mut done = view(100.0, None, &[]);
        done["contacts"][0]["defeated"] = json!(true);
        assert!(matches!(decide(&done, true), Decision::Finish(_)));
        assert!(matches!(decide(&done, false), Decision::Act(gestures) if gestures.is_empty()));
        assert!(hostiles(&done).next().is_none());
        assert!(hostiles(&view(100.0, None, &[])).next().is_some());
    }
}
