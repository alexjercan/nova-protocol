//! The pilot's view: the raw probe snapshot condensed to what the agent is
//! told. Meters and meters per second throughout (the snapshot speaks
//! engine units, ten meters each); bearings are azimuth and elevation from
//! the nose, starboard and up positive, so "turn left three degrees" is a
//! sentence an agent can act on.
//!
//! The view is a PILOT's view, not a census. A scenario can field a hundred
//! rocks and enumerating them costs more tokens than the rest of the world put
//! together, so [`bodies_view`] carries in full only what a pilot acts on -
//! what is close, and what is across a line the pilot has committed to - and
//! summarises the rest by distance and bearing. `expand` opens one summary
//! group back up; the referee answers that from the snapshot it already holds,
//! so looking costs no game time.
//!
//! Pure: a snapshot value in, an observation value out. The referee adds the
//! clock, the budget and the over flag on top.

use std::collections::BTreeSet;

use serde_json::{json, Map, Value};

/// Meters per engine unit, the project's world-unit rule.
const METERS_PER_UNIT: f64 = 10.0;
/// How many comms lines the agent is shown, newest last.
const COMMS_SHOWN: usize = 12;
/// A body this close to the hull is carried in full, whatever it is.
const NEAR_SURFACE_M: f64 = 2000.0;
/// A body the current drift brings this close, this soon, is carried in full
/// too: it is not scenery, it is what the ship is about to hit.
const NEAR_APPROACH_M: f64 = 500.0;
/// How far ahead the closest-approach test looks.
const NEAR_APPROACH_SECS: f64 = 60.0;
/// The clearance an engaged GOTO wants along its line.
const GOTO_CORRIDOR_M: f64 = 300.0;
/// The distance ladder a summarised body is filed on, by surface range. Fixed
/// edges on purpose: a rock changes group only when it crosses one, so two
/// observations of an unmoved world read the same.
const BANDS: &[(f64, &str)] = &[
    (1000.0, "<1km"),
    (2000.0, "1-2km"),
    (5000.0, "2-5km"),
    (10000.0, "5-10km"),
    (25000.0, "10-25km"),
];
/// The band past the end of [`BANDS`].
const FAR_BAND: &str = ">25km";

/// The one summarised tier of the bodies block. Named because [`bodies_of`]
/// has to tell a group from a tier, and a second speller would drift.
const GROUPS: &str = "groups";

/// Whether an `expand` request opens `key` of `block`.
///
/// `all` opens everything and a block's own name opens that whole block, so a
/// new expandable block asks this rather than re-deciding it - forgetting the
/// `all` case is how the score's end state, which asks for `all`, would come
/// back silently truncated.
fn expands(expand: &[String], block: &str, key: &str) -> bool {
    expand
        .iter()
        .any(|asked| asked == "all" || asked == block || asked == key)
}

/// Condense one snapshot. `held` is what the referee knows is down; `expand`
/// names the body groups to open in full (`all` opens every one).
pub fn condense(snapshot: &Value, held: &BTreeSet<String>, expand: &[String]) -> Value {
    let ships = snapshot["ships"].as_array().cloned().unwrap_or_default();
    // The player's hull or nobody. A destroyed integrity root is despawned in
    // the frame its marker lands, so the last snapshot of a lost run carries no
    // Player ship at all - and falling back to the first ship there reframed
    // the whole observation onto the ENEMY: its health reported as `me.health`,
    // its nose as the origin every bearing is measured from, and the player's
    // own round in flight re-attributed as inbound. `Scorer::observe_me`
    // already requires the same thing, so this is also what keeps the score and
    // the view agreeing about who the player is. `me: None` degrades honestly.
    let me = ships.iter().find(|ship| ship["controller"] == "Player");
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
    let bodies = bodies_view(
        &snapshot["bodies"],
        frame.as_ref(),
        &focuses(snapshot, me),
        expand,
    );
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
    let commands = command_acks(&snapshot["applied"]);
    let mut live: Vec<Value> = snapshot["input"]["live"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    live.sort_by_key(ToString::to_string);

    let mut view = json!({
        "game_state": snapshot["game_state"],
        "ui": {
            "pause": snapshot["ui"]["pause"],
            "computer": snapshot["ui"]["computer"]["mode"],
        },
        "objectives": mission["objectives"].as_array().cloned().unwrap_or_default(),
        "objective_log": mission["log"].as_array().cloned().unwrap_or_default(),
        "outcome": mission["outcome"],
        "comms": comms,
        "me": me.map(|me| self_record(me, frame.as_ref())),
        "contacts": contacts,
        "beacons": beacons,
        "bodies": bodies,
        "ordnance": { "inbound": inbound, "outbound": outbound },
        "inputs": {
            "live": live,
            "held": held.iter().collect::<Vec<_>>(),
            // Pairs of wire names that share one physical key. Driving both in
            // one act is refused, and the pairs come from the game rather than
            // a table here, so this list is the whole rule.
            "shared": snapshot["input"]["shared"].as_array().cloned().unwrap_or_default(),
        },
        "commands": commands,
    });
    // Both of these are absent far more often than they are present, and an
    // absent key is the cheapest way to say "no".
    if !mission["cinematic"]["playing"].is_null() {
        view["cinematic"] = mission["cinematic"].clone();
    }
    if mission["cheats"]["marked"] == true {
        view["cheats_marked"] = json!(true);
    }
    view
}

/// Every body a view carries, whatever tier it landed in. A view condensed
/// with `expand: ["all"]` carries them all.
///
/// This walks the block's STRUCTURE rather than naming the tiers, so a tier
/// added to [`bodies_view`] reaches the score without an edit here. Naming
/// them would fail the quiet way: the score's end state would be computed
/// from a partial world, missing exactly the bodies a new tier promoted for
/// being worth promoting.
pub fn bodies_of(view: &Value) -> Vec<Value> {
    let Some(bodies) = view["bodies"].as_object() else {
        return Vec::new();
    };
    let mut all: Vec<Value> = Vec::new();
    for (tier, records) in bodies {
        let records = records.as_array().into_iter().flatten();
        if tier == GROUPS {
            for group in records {
                all.extend(group["bodies"].as_array().cloned().unwrap_or_default());
            }
        } else {
            // Only the records, so a key carrying anything else - `expanded`
            // names its groups as strings - is not mistaken for a tier.
            all.extend(records.filter(|record| record.is_object()).cloned());
        }
    }
    all
}

/// A place the pilot has committed to looking at or flying through. A body
/// across one of these lines is not scenery, whatever its range.
struct Focus {
    /// How the view explains the obstruction, in the pilot's words.
    reason: String,
    /// Where the line ends, in engine units.
    position: [f64; 3],
    /// Clearance the line wants beside the body's own radius, engine units.
    margin: f64,
}

/// The lines this view has to keep clear: the radar or weapons lock, and an
/// engaged GOTO's run to its target.
fn focuses(snapshot: &Value, me: Option<&Value>) -> Vec<Focus> {
    let Some(me) = me else { return Vec::new() };
    let mut focuses = Vec::new();
    for slot in ["combat_lock", "travel_lock"] {
        if let Some(id) = me[slot].as_str() {
            if let Some(position) = position_of(snapshot, id) {
                focuses.push(Focus {
                    reason: format!("occludes the {slot} on {id}"),
                    position,
                    margin: 0.0,
                });
            }
        }
    }
    let engaged = &me["autopilot"]["engaged"];
    if engaged["action"] == "Goto" {
        if let Some(id) = engaged["target"].as_str() {
            if let Some(position) = position_of(snapshot, id) {
                focuses.push(Focus {
                    reason: format!("in the GOTO path to {id}"),
                    position,
                    margin: GOTO_CORRIDOR_M / METERS_PER_UNIT,
                });
            }
        }
    }
    focuses
}

/// Where the thing called `id` stands, wherever in the world it is filed.
fn position_of(snapshot: &Value, id: &str) -> Option<[f64; 3]> {
    for list in ["ships", "beacons", "bodies"] {
        let found = snapshot[list]
            .as_array()
            .into_iter()
            .flatten()
            .find(|item| item["id"] == id);
        if let Some(item) = found {
            return Some(vec3(&item["position"]));
        }
    }
    None
}

/// The rocks and planets, tiered. `near` and `in_the_way` are carried in full
/// because a pilot acts on them; everything else is summarised on the fixed
/// ladder of [`BANDS`] crossed with a bearing sector, and `expand` opens one
/// group back up by its key.
///
/// Every body is classified ALONE, as a pure function of that body and the
/// ship's frame: no k, no seed, no iteration, no memory of the last view. Two
/// observations of one unmoved world therefore group identically, which is
/// what lets an agent reason about a group across turns.
fn bodies_view(
    bodies: &Value,
    frame: Option<&Frame>,
    focuses: &[Focus],
    expand: &[String],
) -> Value {
    let everything = expands(expand, "bodies", "");
    let mut near = Vec::new();
    let mut in_the_way = Vec::new();
    let mut grouped: std::collections::BTreeMap<String, Vec<Value>> =
        std::collections::BTreeMap::new();

    for body in bodies.as_array().into_iter().flatten() {
        let record = body_record(body, frame);
        let Some(frame) = frame else {
            // No player ship: nothing can be measured, so nothing is scenery.
            near.push(record);
            continue;
        };
        let position = vec3(&body["position"]);
        let radius = body["radius"].as_f64().unwrap_or(0.0);
        if let Some(focus) = focuses
            .iter()
            .find(|focus| blocks(frame, position, radius, focus))
        {
            let mut record = record;
            record["why"] = json!(focus.reason);
            in_the_way.push(record);
            continue;
        }
        if is_near(frame, position, radius, &record) {
            near.push(record);
            continue;
        }
        let key = format!(
            "{}.{}",
            band(record["surface_m"].as_f64().unwrap_or(0.0)),
            sector(record["bearing_deg"][0].as_f64().unwrap_or(0.0))
        );
        grouped.entry(key).or_default().push(record);
    }

    let mut groups: Vec<Value> = grouped
        .into_iter()
        .map(|(key, records)| {
            let opened = everything || expands(expand, "bodies", &key);
            group_record(&key, records, opened)
        })
        .collect();
    groups.sort_by(|a, b| {
        a["nearest_surface_m"]
            .as_f64()
            .unwrap_or(0.0)
            .total_cmp(&b["nearest_surface_m"].as_f64().unwrap_or(0.0))
    });
    // What the request actually opened. An expansion key can go stale between
    // observations - the ship moves and a group re-bins - and without this the
    // reply to a key that matched nothing is byte-identical to the reply to no
    // request at all, so a typo, a stale key and an emptied group look alike.
    let opened: Vec<Value> = groups
        .iter()
        .filter(|group| group["bodies"].is_array())
        .map(|group| group["key"].clone())
        .collect();
    let mut block = Map::new();
    block.insert("near".into(), json!(near));
    block.insert("in_the_way".into(), json!(in_the_way));
    block.insert(GROUPS.into(), json!(groups));
    block.insert("expanded".into(), json!(opened));
    Value::Object(block)
}

/// One summarised group: what a pilot would say about a patch of rock -
/// how many, how close the nearest one gets, how big the biggest is.
fn group_record(key: &str, records: Vec<Value>, expanded: bool) -> Value {
    let smallest = |field: &str| {
        records
            .iter()
            .filter_map(|record| record[field].as_f64())
            .min_by(f64::total_cmp)
    };
    let largest = |field: &str| {
        records
            .iter()
            .filter_map(|record| record[field].as_f64())
            .max_by(f64::total_cmp)
    };
    let mut group = json!({
        "key": key,
        "count": records.len(),
        "nearest_surface_m": smallest("surface_m"),
        "farthest_surface_m": largest("surface_m"),
        "largest_radius_m": largest("radius_m"),
    });
    if expanded {
        group["bodies"] = json!(records);
    }
    group
}

/// Whether a body is close enough, or closing fast enough, to be carried in
/// full: inside [`NEAR_SURFACE_M`] of the hull, or on a course that brings it
/// within [`NEAR_APPROACH_M`] inside [`NEAR_APPROACH_SECS`].
fn is_near(frame: &Frame, position: [f64; 3], radius: f64, record: &Value) -> bool {
    if record["surface_m"].as_f64().unwrap_or(f64::MAX) <= NEAR_SURFACE_M {
        return true;
    }
    let speed = length(frame.velocity);
    if speed < 1e-6 {
        return false;
    }
    let to = sub(position, frame.position);
    let along = dot(to, frame.velocity) / (speed * speed);
    if along <= 0.0 || along > NEAR_APPROACH_SECS {
        return false;
    }
    let closest = [
        to[0] - frame.velocity[0] * along,
        to[1] - frame.velocity[1] * along,
        to[2] - frame.velocity[2] * along,
    ];
    (length(closest) - radius) * METERS_PER_UNIT <= NEAR_APPROACH_M
}

/// Whether a body sits between the ship and a [`Focus`], close enough to the
/// line of sight to block it. Behind the ship or past the target is clear.
fn blocks(frame: &Frame, position: [f64; 3], radius: f64, focus: &Focus) -> bool {
    let to_target = sub(focus.position, frame.position);
    let range = length(to_target);
    if range < 1e-6 {
        return false;
    }
    let unit = [
        to_target[0] / range,
        to_target[1] / range,
        to_target[2] / range,
    ];
    let to_body = sub(position, frame.position);
    let along = dot(to_body, unit);
    if along <= 0.0 || along >= range {
        return false;
    }
    let offset = [
        to_body[0] - unit[0] * along,
        to_body[1] - unit[1] * along,
        to_body[2] - unit[2] * along,
    ];
    length(offset) < radius + focus.margin
}

/// The band of [`BANDS`] a surface range falls in.
fn band(surface_m: f64) -> &'static str {
    BANDS
        .iter()
        .find(|(edge, _)| surface_m < *edge)
        .map_or(FAR_BAND, |(_, name)| *name)
}

/// The quarter of the sky an azimuth points into, in the words a pilot uses.
fn sector(azimuth: f64) -> &'static str {
    match azimuth.abs() {
        magnitude if magnitude <= 45.0 => "bow",
        magnitude if magnitude <= 135.0 => {
            if azimuth > 0.0 {
                "starboard"
            } else {
                "port"
            }
        }
        _ => "astern",
    }
}

/// One asteroid or planet in full: what it is, how big, where, and how fast
/// the range to it is shrinking.
fn body_record(body: &Value, frame: Option<&Frame>) -> Value {
    let position = vec3(&body["position"]);
    let radius = body["radius"].as_f64().unwrap_or(0.0) * METERS_PER_UNIT;
    let mut record = json!({
        "id": body["id"],
        "name": body["name"],
        "kind": body["kind"],
        "radius_m": round1(radius),
        "invulnerable": body["invulnerable"],
    });
    range_and_bearing(&mut record, frame, position);
    if let Some(distance) = record["distance_m"].as_f64() {
        record["surface_m"] = json!(round1((distance - radius).max(0.0)));
    }
    if let Some(frame) = frame {
        // A body does not move, so the closing rate is the ship's own run at
        // it - positive while the range shrinks, as for a contact.
        let to = sub(position, frame.position);
        let range = length(to);
        if range > 1e-6 {
            let unit = [to[0] / range, to[1] / range, to[2] / range];
            record["closing_mps"] = json!(round1(dot(frame.velocity, unit) * METERS_PER_UNIT));
        }
    }
    record
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
        "withheld_verbs": withheld_verbs(&me["sections"]),
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
        "radar": me["radar"],
        "gravity_well": me["gravity_well"],
        "collapsing": me["collapsing"],
        "defeated": me["defeated"],
        "sections": sections,
        "hull_plates": plates,
    })
}

/// The flight verbs the ship is not allowed to use right now, named as the
/// scenario withheld them: `Goto`, `Lock`, `Orbit`, `Rcs`, `Stop`.
///
/// A tutorial hands the verbs over one lesson at a time, and a player sees
/// that on the hint strip. Without it the pilot cannot tell "I pressed it
/// wrong" from "this ship cannot do that yet" - the radar acquires nothing at
/// all while `Lock` is withheld, whatever the gesture looks like.
fn withheld_verbs(sections: &Value) -> Vec<Value> {
    let mut verbs: Vec<Value> = sections
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|section| section["modifications"].as_array().into_iter().flatten())
        .filter_map(|modification| modification.get("DisableVerb").cloned())
        .collect();
    verbs.sort_by_key(ToString::to_string);
    verbs.dedup();
    verbs
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

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
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

/// The command answers out of the channel's acks.
///
/// The input acks are dropped on the floor here, and that is the point: an
/// input ack is an ECHO of a line the frame consumed, and the agent already
/// knows what it sent. A COMMAND is different - it is a request the shell
/// answers - so its status, its one-line detail and the rows a player would
/// have read all come through.
fn command_acks(applied: &Value) -> Vec<Value> {
    applied
        .as_array()
        .into_iter()
        .flatten()
        .filter(|ack| ack.get("command").is_some())
        .map(|ack| {
            json!({
                "command": ack["command"],
                "state": ack["state"],
                "detail": ack["detail"],
                "rows": ack["rows"],
            })
        })
        .collect()
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
                "log": [
                    { "kind": "posted", "id": "kill", "message": "Destroy the raider." },
                    { "kind": "posted", "id": "close", "message": "Close to 2 km." },
                    { "kind": "completed", "id": "close", "message": "Close to 2 km." }
                ],
                "outcome": null,
                "comms": [{ "speaker": "Ops", "text": "Go.", "channel": "comms" }],
                "cinematic": { "playing": null, "skippable": null },
                "cheats": { "armed": false, "marked": false },
            },
            "ships": [
                {
                    "id": "player", "name": "Cutter", "controller": "Player", "allegiance": "Player",
                    "position": [0, 0, 0], "rotation": [0, 0, 0, 1],
                    "linear_velocity": [0, 0, -2], "angular_velocity": [0, 0, 0],
                    "health": { "current": 800, "max": 800 }, "weapons_hot": true,
                    "combat_lock": null, "travel_lock": null, "collapsing": false, "defeated": false,
                    "autopilot": { "engaged": { "action": "Goto", "target": "mark", "phase": "Burn" }, "completed": null },
                    "radar": null,
                    "gravity_well": "planetoid",
                    "sections": [
                        { "id": "turret_port", "class": "Turret", "health": { "current": 50, "max": 50 }, "alive": true, "disabled": false,
                          "weapon": { "kind": "turret", "firing": false, "on_target": false, "ammo": { "rounds": 40, "capacity": 40 }, "charge": null, "reload": null } },
                        { "id": "hull", "class": "Hull", "health": { "current": 100, "max": 100 }, "alive": true, "disabled": false, "weapon": null },
                        { "id": "bridge", "class": "Controller", "health": { "current": 300, "max": 300 }, "alive": true, "disabled": false, "weapon": null,
                          "modifications": [{ "SetHealth": 300.0 }, { "DisableVerb": "Orbit" }, { "DisableVerb": "Lock" }] }
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
                { "line": 3, "input": "flight.main_drive", "phase": "start", "tick": 61 },
                { "line": 4, "input": "section.turret_port", "phase": "start", "tick": 61 },
                { "line": 5, "command": "map", "state": "ok", "detail": "1 row", "rows": ["raider_1 2800m"], "tick": 61 }
            ],
            "input": {
                "live": ["flight.main_drive", "camera.camera_rotate"],
                "contexts": ["always", "flight"],
                "shared": [["targeting.radar_hold", "targeting.radar_clear"]]
            }
        })
    }

    fn view() -> Value {
        condense(&snapshot(), &BTreeSet::new(), &[])
    }

    #[test]
    fn the_pilot_sees_meters_bearings_and_closing_speed() {
        let held = BTreeSet::from(["flight.main_drive".to_string()]);
        let view = condense(&snapshot(), &held, &[]);
        assert_eq!(view["me"]["id"], "player");
        assert_eq!(view["me"]["speed_mps"], 20.0);
        assert_eq!(view["me"]["velocity_bearing_deg"], json!([0.0, 0.0]));
        assert_eq!(view["me"]["sections"].as_array().unwrap().len(), 2);
        assert_eq!(view["me"]["withheld_verbs"], json!(["Lock", "Orbit"]));
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
        assert_eq!(view["me"]["autopilot"]["engaged"]["action"], "Goto");
        assert_eq!(view["me"]["gravity_well"], "planetoid");
        assert_eq!(view["ordnance"], json!({ "inbound": 1, "outbound": 2 }));
        assert_eq!(view["inputs"]["held"], json!(["flight.main_drive"]));
        // Read off the snapshot's own `input` block, not the view's `inputs`.
        assert_eq!(
            view["inputs"]["shared"],
            json!([["targeting.radar_hold", "targeting.radar_clear"]])
        );
        assert_eq!(view["objectives"][0]["id"], "kill");
        assert_eq!(view["comms"][0]["speaker"], "Ops");
    }

    /// A card that went up and came down between two reads leaves no trace in
    /// `objectives`; the log is where the agent and the scorer find it.
    #[test]
    fn the_objective_log_carries_cards_the_live_list_has_dropped() {
        let view = view();
        assert_eq!(view["objective_log"].as_array().unwrap().len(), 3);
        assert_eq!(view["objective_log"][2]["kind"], "completed");
        assert_eq!(view["objective_log"][2]["id"], "close");
    }

    /// A body's range is to its centre; `surface_m` is what the hull can get
    /// to. The rock is a kilometre off and carried in full; the planetoid is
    /// seven and summarised until it is asked for.
    #[test]
    fn a_close_rock_is_carried_in_full_and_the_rest_is_summarised() {
        let view = view();
        let near = view["bodies"]["near"].as_array().unwrap();
        assert_eq!(near.len(), 1);
        assert_eq!(near[0]["id"], "rock");
        assert_eq!(near[0]["surface_m"], 985.0);
        assert_eq!(near[0]["bearing_deg"], json!([-90.0, 0.0]));
        assert_eq!(view["bodies"]["in_the_way"], json!([]));

        let groups = view["bodies"]["groups"].as_array().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["key"], "5-10km.bow");
        assert_eq!(groups[0]["count"], 1);
        assert_eq!(groups[0]["nearest_surface_m"], 6400.0);
        assert_eq!(groups[0]["largest_radius_m"], 600.0);
        assert!(
            groups[0].get("bodies").is_none(),
            "a summary carries no records until it is expanded"
        );

        let opened = condense(&snapshot(), &BTreeSet::new(), &["5-10km.bow".to_string()]);
        let group = &opened["bodies"]["groups"][0];
        assert_eq!(group["bodies"][0]["id"], "planetoid");
        assert_eq!(group["bodies"][0]["distance_m"], 7000.0);
        assert_eq!(group["bodies"][0]["radius_m"], 600.0);
        assert_eq!(group["bodies"][0]["closing_mps"], 20.0);
        assert_eq!(bodies_of(&opened).len(), 2);
    }

    /// The same world twice groups the same way: the tier is a pure function
    /// of one body and the frame, so an agent can reason across turns.
    #[test]
    fn two_views_of_one_world_group_identically() {
        assert_eq!(view()["bodies"], view()["bodies"]);
    }

    /// The safety rule: a rock the current drift runs into is carried in full
    /// however far away it is, because expansion is for detail, never safety.
    #[test]
    fn a_rock_on_a_collision_course_is_never_summarised() {
        let mut snapshot = snapshot();
        // 20 km dead ahead, and 400 m/s of closing speed: 50 seconds out.
        snapshot["bodies"] = json!([
            { "id": "wall", "name": "Wall", "kind": "Asteroid", "position": [0, 0, -2000], "radius": 5, "invulnerable": false }
        ]);
        snapshot["ships"][0]["linear_velocity"] = json!([0, 0, -40]);
        let view = condense(&snapshot, &BTreeSet::new(), &[]);
        assert_eq!(view["bodies"]["near"][0]["id"], "wall");
        assert_eq!(view["bodies"]["groups"], json!([]));
    }

    /// A rock across the lock is not scenery, and the view says why.
    #[test]
    fn a_rock_across_the_lock_is_called_out_with_its_reason() {
        let mut snapshot = snapshot();
        snapshot["ships"][0]["combat_lock"] = json!("raider_1");
        // Half way to the raider at 2.8 km, dead on the line of sight.
        snapshot["bodies"] = json!([
            { "id": "screen", "name": "Screen", "kind": "Asteroid", "position": [5, 0, -140], "radius": 8, "invulnerable": false }
        ]);
        let view = condense(&snapshot, &BTreeSet::new(), &[]);
        let blocking = &view["bodies"]["in_the_way"][0];
        assert_eq!(blocking["id"], "screen");
        assert_eq!(blocking["why"], "occludes the combat_lock on raider_1");
    }

    #[test]
    fn a_command_keeps_its_rows_and_an_input_ack_is_not_echoed_back() {
        let view = view();
        assert!(
            view.get("refused").is_none(),
            "an input has no verdict to report"
        );
        assert_eq!(view["commands"].as_array().unwrap().len(), 1);
        assert_eq!(view["commands"][0]["rows"], json!(["raider_1 2800m"]));
    }

    /// Two facts the pilot's HUD paints that the view used to drop: the lock
    /// dwell charging, and a scene playing that the skip key cannot end.
    #[test]
    fn the_view_carries_the_lock_dwell_and_the_cinematic() {
        let mut snapshot = snapshot();
        snapshot["ships"][0]["radar"] = json!({
            "slot": "Combat", "candidate": "raider_1", "dwell_target": "raider_1",
            "dwell_secs": 0.4, "dwell_needed": 1.0, "dwell_fill": 0.4
        });
        snapshot["mission"]["cinematic"] = json!({ "playing": "opening", "skippable": null });
        snapshot["mission"]["cheats"] = json!({ "armed": true, "marked": true });
        let view = condense(&snapshot, &BTreeSet::new(), &[]);
        assert_eq!(view["me"]["radar"]["dwell_fill"], 0.4);
        assert_eq!(view["me"]["radar"]["dwell_target"], "raider_1");
        assert_eq!(view["cinematic"]["playing"], "opening");
        assert_eq!(view["cinematic"]["skippable"], Value::Null);
        assert_eq!(view["cheats_marked"], true);
    }

    /// A quiet world says none of it: an absent key is the cheapest "no".
    #[test]
    fn a_clean_run_with_no_scene_carries_neither_key() {
        let view = view();
        assert!(view.get("cinematic").is_none());
        assert!(view.get("cheats_marked").is_none());
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
    fn the_ladder_and_the_sectors_read_as_a_pilot_would_say_them() {
        assert_eq!(band(0.0), "<1km");
        assert_eq!(band(999.9), "<1km");
        assert_eq!(band(1000.0), "1-2km");
        assert_eq!(band(24_999.0), "10-25km");
        assert_eq!(band(25_000.0), ">25km");
        assert_eq!(sector(0.0), "bow");
        assert_eq!(sector(-45.0), "bow");
        assert_eq!(sector(90.0), "starboard");
        assert_eq!(sector(-90.0), "port");
        assert_eq!(sector(179.0), "astern");
    }

    #[test]
    fn an_empty_world_condenses_to_nothing_rather_than_a_panic() {
        let view = condense(&json!({}), &BTreeSet::new(), &[]);
        assert!(view["me"].is_null());
        assert_eq!(view["contacts"], json!([]));
        assert_eq!(
            view["bodies"],
            json!({ "near": [], "in_the_way": [], "groups": [], "expanded": [] })
        );
        assert_eq!(view["ordnance"], json!({ "inbound": 0, "outbound": 0 }));
        assert!(bodies_of(&view).is_empty());
    }

    /// The last snapshot of a lost run: the player's root was despawned in the
    /// frame it died, so the only ship left is the one that killed it.
    #[test]
    fn a_run_that_lost_its_hull_reports_no_me_rather_than_the_enemys() {
        let mut snapshot = snapshot();
        let ships = snapshot["ships"].as_array_mut().expect("ships");
        ships.retain(|ship| ship["controller"] != "Player");
        let view = condense(&snapshot, &BTreeSet::new(), &[]);

        assert!(
            view["me"].is_null(),
            "the enemy was reported as the player's own hull: {}",
            view["me"]
        );
        assert_eq!(
            view["contacts"].as_array().map(Vec::len),
            Some(1),
            "the surviving raider is a contact, not the point of view"
        );
    }
}
