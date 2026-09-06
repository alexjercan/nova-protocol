//! The pilot's view: the raw probe snapshot condensed to what the agent is
//! told. Meters and meters per second throughout (the snapshot speaks
//! engine units, ten meters each); bearings are azimuth and elevation from
//! the nose, starboard and up positive, so "turn left three degrees" is a
//! sentence an agent can act on.
//!
//! Pure: a snapshot value in, an observation value out. The referee adds the
//! clock, the budget and the over flag on top.

use std::collections::BTreeSet;

use serde_json::{json, Value};

/// Meters per engine unit, the project's world-unit rule.
const METERS_PER_UNIT: f64 = 10.0;
/// How many comms lines the agent is shown, newest last.
const COMMS_SHOWN: usize = 12;

/// Condense one snapshot. `held` is what the referee knows is down.
pub fn condense(snapshot: &Value, held: &BTreeSet<String>) -> Value {
    let ships = snapshot["ships"].as_array().cloned().unwrap_or_default();
    let me = ships
        .iter()
        .find(|ship| ship["controller"] == "Player")
        .or_else(|| ships.first());
    let frame = me.map(Frame::of);
    let mission = &snapshot["mission"];

    let contacts: Vec<Value> = ships
        .iter()
        .filter(|ship| me.is_none_or(|me| me["id"] != ship["id"]))
        .map(|ship| contact(ship, frame.as_ref()))
        .collect();
    let beacons: Vec<Value> = snapshot["beacons"]
        .as_array()
        .map(|beacons| {
            beacons
                .iter()
                .map(|beacon| {
                    let position = vec3(&beacon["position"]);
                    let mut record = json!({ "id": beacon["id"], "label": beacon["label"] });
                    range_and_bearing(&mut record, frame.as_ref(), position);
                    record
                })
                .collect()
        })
        .unwrap_or_default();
    let bodies: Vec<Value> = snapshot["bodies"]
        .as_array()
        .map(|bodies| {
            bodies
                .iter()
                .map(|body| {
                    let position = vec3(&body["position"]);
                    let radius = body["radius"].as_f64().unwrap_or(0.0) * METERS_PER_UNIT;
                    let mut record = json!({
                        "id": body["id"],
                        "name": body["name"],
                        "kind": body["kind"],
                        "radius_m": round1(radius),
                        "invulnerable": body["invulnerable"],
                    });
                    range_and_bearing(&mut record, frame.as_ref(), position);
                    if let Some(distance) = record["distance_m"].as_f64() {
                        record["surface_m"] = json!(round1((distance - radius).max(0.0)));
                    }
                    record
                })
                .collect()
        })
        .unwrap_or_default();
    let me_id = me.map(|me| me["id"].clone()).unwrap_or(Value::Null);
    let (inbound, outbound) = snapshot["ordnance"]
        .as_array()
        .map(|ordnance| {
            ordnance
                .iter()
                .fold((0u64, 0u64), |(inbound, outbound), round| {
                    if round["owner"] == me_id {
                        (inbound, outbound + 1)
                    } else {
                        (inbound + 1, outbound)
                    }
                })
        })
        .unwrap_or_default();
    let comms: Vec<Value> = mission["comms"]
        .as_array()
        .map(|lines| {
            lines
                .iter()
                .rev()
                .take(COMMS_SHOWN)
                .rev()
                .map(|line| json!({ "speaker": line["speaker"], "text": line["text"] }))
                .collect()
        })
        .unwrap_or_default();
    let (refused, commands) = acks(&snapshot["applied"]);
    let mut live: Vec<Value> = snapshot["input"]["live"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    live.sort_by_key(ToString::to_string);

    json!({
        "game_state": snapshot["game_state"],
        "ui": {
            "pause": snapshot["ui"]["pause"],
            "computer": snapshot["ui"]["computer"]["mode"],
        },
        "objectives": mission["objectives"].as_array().cloned().unwrap_or_default(),
        "outcome": mission["outcome"],
        "comms": comms,
        "me": me.map(|me| self_record(me, frame.as_ref())),
        "contacts": contacts,
        "beacons": beacons,
        "bodies": bodies,
        "ordnance": { "inbound": inbound, "outbound": outbound },
        "inputs": { "live": live, "held": held.iter().collect::<Vec<_>>() },
        "commands": commands,
        "refused": refused,
    })
}

/// The player's own ship.
fn self_record(me: &Value, frame: Option<&Frame>) -> Value {
    let velocity = vec3(&me["linear_velocity"]);
    let speed = length(velocity) * METERS_PER_UNIT;
    let velocity_bearing = frame
        .filter(|_| speed > 0.05)
        .map(|frame| bearing(frame, velocity));
    let angular = vec3(&me["angular_velocity"]);
    let (sections, plates) = sections_and_plates(&me["sections"]);
    json!({
        "id": me["id"],
        "name": me["name"],
        "position_m": scale(vec3(&me["position"])),
        "speed_mps": round1(speed),
        "velocity_bearing_deg": velocity_bearing,
        "turn_rate_dps": round1(length(angular).to_degrees()),
        "health": me["health"],
        "weapons_hot": me["weapons_hot"],
        "combat_lock": me["combat_lock"],
        "travel_lock": me["travel_lock"],
        "autopilot": me["autopilot"],
        "gravity_well": me["gravity_well"],
        "collapsing": me["collapsing"],
        "defeated": me["defeated"],
        "sections": sections,
        "hull_plates": plates,
    })
}

/// The sections a pilot reads by name (mounts, bridge, drives) and the
/// armour plates folded into one count: a hull carries dozens of plates
/// and naming each would bury the guns.
fn sections_and_plates(sections: &Value) -> (Vec<Value>, Value) {
    let mut named = Vec::new();
    let (mut total, mut damaged, mut lost) = (0u64, 0u64, 0u64);
    for section in sections.as_array().into_iter().flatten() {
        if section["class"] != "Hull" {
            named.push(self::section(section));
            continue;
        }
        total += 1;
        if section["alive"] == false {
            lost += 1;
        } else if section["health"]["current"].as_f64() < section["health"]["max"].as_f64() {
            damaged += 1;
        }
    }
    (
        named,
        json!({ "total": total, "damaged": damaged, "lost": lost }),
    )
}

/// One section as the pilot's damage board and gun status show it.
fn section(section: &Value) -> Value {
    let mut record = json!({
        "id": section["id"],
        "class": section["class"],
        "health": section["health"],
        "alive": section["alive"],
    });
    if section["disabled"] == true {
        record["disabled"] = json!(true);
    }
    let weapon = &section["weapon"];
    if weapon.is_object() {
        let mut gun = json!({
            "kind": weapon["kind"],
            "firing": weapon["firing"],
            "on_target": weapon["on_target"],
            "ammo": weapon["ammo"],
        });
        if !weapon["charge"].is_null() {
            gun["charging_secs"] = weapon["charge"].clone();
        }
        if !weapon["reload"].is_null() {
            gun["reloading"] = json!(true);
        }
        record["weapon"] = gun;
    }
    record
}

/// Another ship, as radar shows it: range, bearing, closing speed, state.
fn contact(ship: &Value, frame: Option<&Frame>) -> Value {
    let position = vec3(&ship["position"]);
    let velocity = vec3(&ship["linear_velocity"]);
    let mut record = json!({
        "id": ship["id"],
        "name": ship["name"],
        "allegiance": ship["allegiance"],
        "controller": ship["controller"],
        "speed_mps": round1(length(velocity) * METERS_PER_UNIT),
        "health": ship["health"],
        "weapons_hot": ship["weapons_hot"],
        "ai_target": ship["ai_target"],
        "combat_lock": ship["combat_lock"],
        "collapsing": ship["collapsing"],
        "defeated": ship["defeated"],
        "neutralized": ship["neutralized"],
    });
    range_and_bearing(&mut record, frame, position);
    if let Some(frame) = frame {
        let to = sub(position, frame.position);
        let range = length(to);
        if range > 1e-6 {
            let unit = [to[0] / range, to[1] / range, to[2] / range];
            let relative = sub(velocity, frame.velocity);
            let closing = -(relative[0] * unit[0] + relative[1] * unit[1] + relative[2] * unit[2]);
            record["closing_mps"] = json!(round1(closing * METERS_PER_UNIT));
        }
    }
    record
}

/// Add `distance_m` and `bearing_deg` for a world position, when there is a
/// frame to measure from.
fn range_and_bearing(record: &mut Value, frame: Option<&Frame>, position: [f64; 3]) {
    let Some(frame) = frame else { return };
    let to = sub(position, frame.position);
    record["distance_m"] = json!(round1(length(to) * METERS_PER_UNIT));
    record["bearing_deg"] = json!(bearing(frame, to));
}

/// The player's frame: where the nose points and how fast the hull moves.
struct Frame {
    position: [f64; 3],
    velocity: [f64; 3],
    /// The conjugate of the ship's rotation: world into local.
    inverse: [f64; 4],
}

impl Frame {
    fn of(ship: &Value) -> Self {
        let q = ship["rotation"]
            .as_array()
            .filter(|q| q.len() == 4)
            .map(|q| {
                [
                    q[0].as_f64().unwrap_or(0.0),
                    q[1].as_f64().unwrap_or(0.0),
                    q[2].as_f64().unwrap_or(0.0),
                    q[3].as_f64().unwrap_or(1.0),
                ]
            })
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);
        Self {
            position: vec3(&ship["position"]),
            velocity: vec3(&ship["linear_velocity"]),
            inverse: [-q[0], -q[1], -q[2], q[3]],
        }
    }
}

/// Azimuth and elevation of a world-frame direction from the nose (-Z
/// local): azimuth positive to starboard, elevation positive up.
fn bearing(frame: &Frame, direction: [f64; 3]) -> [f64; 2] {
    let local = rotate(frame.inverse, direction);
    let flat = (local[0] * local[0] + local[2] * local[2]).sqrt();
    // Straight up or down has no azimuth; atan2 on a negative zero says 180.
    let azimuth = if flat < 1e-9 {
        0.0
    } else {
        local[0].atan2(-local[2]).to_degrees()
    };
    let elevation = local[1].atan2(flat).to_degrees();
    [round1(azimuth), round1(elevation)]
}

/// Rotate `v` by the quaternion `q` (x, y, z, w).
fn rotate(q: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    let axis = [q[0], q[1], q[2]];
    let t = scale2(cross(axis, v));
    let wt = [t[0] * q[3], t[1] * q[3], t[2] * q[3]];
    let ct = cross(axis, t);
    [
        v[0] + wt[0] + ct[0],
        v[1] + wt[1] + ct[1],
        v[2] + wt[2] + ct[2],
    ]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn scale2(v: [f64; 3]) -> [f64; 3] {
    [v[0] * 2.0, v[1] * 2.0, v[2] * 2.0]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn length(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn vec3(value: &Value) -> [f64; 3] {
    value
        .as_array()
        .filter(|v| v.len() == 3)
        .map(|v| {
            [
                v[0].as_f64().unwrap_or(0.0),
                v[1].as_f64().unwrap_or(0.0),
                v[2].as_f64().unwrap_or(0.0),
            ]
        })
        .unwrap_or([0.0; 3])
}

/// Engine units to meters, one decimal.
fn scale(v: [f64; 3]) -> [f64; 3] {
    [
        round1(v[0] * METERS_PER_UNIT),
        round1(v[1] * METERS_PER_UNIT),
        round1(v[2] * METERS_PER_UNIT),
    ]
}

fn round1(value: f64) -> f64 {
    let rounded = (value * 10.0).round() / 10.0;
    if rounded == 0.0 {
        0.0
    } else {
        rounded
    }
}

/// Sort the channel's acks: inputs that did not take, and command answers.
/// A `start` that reads `None` after the frame is an input the game did not
/// accept - the context was not up, or the hull cannot do it now.
fn acks(applied: &Value) -> (Vec<Value>, Vec<Value>) {
    let mut refused = Vec::new();
    let mut commands = Vec::new();
    for ack in applied.as_array().into_iter().flatten() {
        if ack.get("command").is_some() {
            commands.push(json!({
                "command": ack["command"],
                "state": ack["state"],
                "detail": ack["detail"],
                "rows": ack["rows"],
            }));
            continue;
        }
        let state = ack["state"].as_str().unwrap_or_default();
        let start = ack["phase"] == "start";
        let refusal = matches!(state, "refused" | "error") || (start && state == "None");
        if refusal {
            refused.push(json!({
                "input": ack["input"],
                "phase": ack["phase"],
                "state": ack["state"],
            }));
        }
    }
    (refused, commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Value {
        json!({
            "game_state": "Playing",
            "ui": { "pause": false, "computer": null },
            "mission": {
                "objectives": [{ "id": "kill", "message": "Destroy the raider." }],
                "outcome": null,
                "comms": [{ "speaker": "Ops", "text": "Go.", "channel": "comms" }],
            },
            "ships": [
                {
                    "id": "player", "name": "Cutter", "controller": "Player", "allegiance": "Player",
                    "position": [0, 0, 0], "rotation": [0, 0, 0, 1],
                    "linear_velocity": [0, 0, -2], "angular_velocity": [0, 0, 0],
                    "health": { "current": 800, "max": 800 }, "weapons_hot": true,
                    "combat_lock": null, "travel_lock": null, "collapsing": false, "defeated": false,
                    "autopilot": { "engaged": { "action": "Goto", "target": "mark", "phase": "Burn" }, "completed": null },
                    "gravity_well": "planetoid",
                    "sections": [
                        { "id": "turret_port", "class": "Turret", "health": { "current": 50, "max": 50 }, "alive": true, "disabled": false,
                          "weapon": { "kind": "turret", "firing": false, "on_target": false, "ammo": { "rounds": 40, "capacity": 40 }, "charge": null, "reload": null } },
                        { "id": "hull", "class": "Hull", "health": { "current": 100, "max": 100 }, "alive": true, "disabled": false, "weapon": null }
                    ]
                },
                {
                    "id": "raider_1", "name": "Raider", "controller": "AI", "allegiance": "Hostile",
                    "position": [10, 0, -280], "rotation": [0, 0, 0, 1],
                    "linear_velocity": [0, 0, 1], "angular_velocity": [0, 0, 0],
                    "health": { "current": 600, "max": 600 }, "weapons_hot": false,
                    "combat_lock": null, "ai_target": "player", "collapsing": false, "defeated": false, "neutralized": false,
                    "sections": []
                }
            ],
            "beacons": [{ "id": "mark", "label": "WORK MARK", "position": [0, 100, 0] }],
            "bodies": [
                { "id": "planetoid", "name": "Kestrel", "kind": "Planet", "position": [0, 0, -700], "radius": 60, "invulnerable": true },
                { "id": "rock", "name": "Rock", "kind": "Asteroid", "position": [-100, 0, 0], "radius": 1.5, "invulnerable": false }
            ],
            "ordnance": [
                { "kind": "bullet", "owner": "player" },
                { "kind": "torpedo", "owner": "raider_1" },
                { "kind": "bullet", "owner": "player" }
            ],
            "applied": [
                { "line": 3, "input": "flight.main_drive", "phase": "start", "state": "Ongoing" },
                { "line": 4, "input": "section.turret_port", "phase": "start", "state": "None" },
                { "line": 5, "command": "map", "state": "ok", "detail": "1 row", "rows": ["raider_1 2800m"] }
            ],
            "input": { "live": ["flight.main_drive", "camera.camera_rotate"], "contexts": ["always", "flight"] }
        })
    }

    #[test]
    fn the_pilot_sees_meters_bearings_and_closing_speed() {
        let held = BTreeSet::from(["flight.main_drive".to_string()]);
        let view = condense(&snapshot(), &held);
        assert_eq!(view["me"]["id"], "player");
        assert_eq!(view["me"]["speed_mps"], 20.0);
        assert_eq!(view["me"]["velocity_bearing_deg"], json!([0.0, 0.0]));
        assert_eq!(view["me"]["sections"].as_array().unwrap().len(), 1);
        assert_eq!(view["me"]["sections"][0]["weapon"]["ammo"]["rounds"], 40);
        assert_eq!(
            view["me"]["hull_plates"],
            json!({ "total": 1, "damaged": 0, "lost": 0 })
        );

        let raider = &view["contacts"][0];
        assert_eq!(raider["id"], "raider_1");
        assert_eq!(raider["distance_m"], 2801.8);
        // Ahead and a touch to starboard: +X is the pilot's right.
        assert_eq!(raider["bearing_deg"], json!([2.0, 0.0]));
        // We run at it at 2 units/s and it runs at us at 1: 3 units/s = 30 m/s.
        assert_eq!(raider["closing_mps"], 30.0);

        assert_eq!(view["beacons"][0]["bearing_deg"], json!([0.0, 90.0]));
        assert_eq!(view["beacons"][0]["distance_m"], 1000.0);
        // A body's range is to its centre; `surface_m` is what the hull can
        // get to. The planetoid is dead ahead, the rock hard to port.
        let planetoid = &view["bodies"][0];
        assert_eq!(planetoid["kind"], "Planet");
        assert_eq!(planetoid["distance_m"], 7000.0);
        assert_eq!(planetoid["radius_m"], 600.0);
        assert_eq!(planetoid["surface_m"], 6400.0);
        assert_eq!(planetoid["bearing_deg"], json!([0.0, 0.0]));
        assert_eq!(view["bodies"][1]["bearing_deg"], json!([-90.0, 0.0]));
        assert_eq!(view["bodies"][1]["surface_m"], 985.0);
        assert_eq!(view["me"]["autopilot"]["engaged"]["action"], "Goto");
        assert_eq!(view["me"]["gravity_well"], "planetoid");
        assert_eq!(view["ordnance"], json!({ "inbound": 1, "outbound": 2 }));
        assert_eq!(view["inputs"]["held"], json!(["flight.main_drive"]));
        assert_eq!(view["objectives"][0]["id"], "kill");
        assert_eq!(view["comms"][0]["speaker"], "Ops");
    }

    #[test]
    fn a_start_that_reads_none_is_a_refusal_and_commands_keep_their_rows() {
        let view = condense(&snapshot(), &BTreeSet::new());
        assert_eq!(view["refused"].as_array().unwrap().len(), 1);
        assert_eq!(view["refused"][0]["input"], "section.turret_port");
        assert_eq!(view["commands"][0]["rows"], json!(["raider_1 2800m"]));
    }

    #[test]
    fn a_turned_ship_measures_bearings_from_its_own_nose() {
        // Yawed 90 degrees left (about +Y): the nose points down -X, so a
        // contact on -X is dead ahead and one on -Z is off the starboard beam.
        let half = std::f64::consts::FRAC_1_SQRT_2;
        let frame = Frame {
            position: [0.0; 3],
            velocity: [0.0; 3],
            inverse: [0.0, -half, 0.0, half],
        };
        assert_eq!(bearing(&frame, [-10.0, 0.0, 0.0]), [0.0, 0.0]);
        assert_eq!(bearing(&frame, [0.0, 0.0, -10.0]), [90.0, 0.0]);
        assert_eq!(bearing(&frame, [10.0, 0.0, 0.0])[0].abs(), 180.0);
    }

    #[test]
    fn an_empty_world_condenses_to_nothing_rather_than_a_panic() {
        let view = condense(&json!({}), &BTreeSet::new());
        assert!(view["me"].is_null());
        assert_eq!(view["contacts"], json!([]));
        assert_eq!(view["ordnance"], json!({ "inbound": 0, "outbound": 0 }));
    }
}
