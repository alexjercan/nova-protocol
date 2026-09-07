//! The score: raw metrics diffed out of the snapshot stream. No composite -
//! a single number is policy, and the report shows the table instead.
//!
//! The score never reads the agent's own report. It sees what the game
//! said, snapshot by snapshot, so an agent cannot claim a result it did
//! not get.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::observation::bodies_of;

/// Token usage and cost, from pi's `message_end` events. Absent for agents
/// that do not report usage.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Llm {
    /// Assistant messages counted.
    pub messages: u64,
    /// Prompt tokens, cache hits included.
    pub input: u64,
    /// Completion tokens.
    pub output: u64,
    /// Prompt tokens served from cache.
    pub cache_read: u64,
    /// Prompt tokens written to cache.
    pub cache_write: u64,
    /// Total cost in the provider's currency, as pi computes it.
    pub cost: f64,
}

/// The run's metrics.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Score {
    /// `victory`, `defeat` or `none`, from `mission.outcome` at the end.
    pub outcome: String,
    /// Objective ids that appeared at any point: the live list, plus every
    /// card the flight log says was posted. A card posted and completed
    /// between two snapshots never reaches the live list.
    pub objectives_seen: BTreeSet<String>,
    /// Objective ids the flight log says completed, plus ids that appeared in
    /// the live list and then vanished before any defeat.
    pub objectives_completed: BTreeSet<String>,
    /// Ticks the run used.
    pub ticks: u64,
    /// The same, in simulated seconds.
    pub game_seconds: f64,
    /// Wall-clock seconds from the first step to the end.
    pub wall_seconds: f64,
    /// `act` calls the agent made.
    pub turns: u64,
    /// Gestures those acts carried.
    pub gestures: u64,
    /// Health the player's hull lost, summed over decreases.
    pub damage_taken: f64,
    /// Player sections that stopped being alive.
    pub sections_lost: u64,
    /// Rounds the player's mounts spent, summed over decreases.
    pub ammo_spent: u64,
    /// Hostile ships that turned `defeated`, or left the world altogether: a
    /// hull that breaks up despawns without ever reading `defeated`.
    pub kills: u64,
    /// Wire lines the game refused outright: an unknown name, an axis driven
    /// as a button, a malformed line. The DRIVER writing nonsense - never the
    /// game deciding not to do something, which an input ack cannot see and
    /// no longer claims to (see `nova_channel::apply::AppliedEntry`).
    pub bad_lines: u64,
    /// Whether the run carries NOVA OS's cheat mark. A benchmark that cannot
    /// see a cheat is not a benchmark: arming cheats marks the attempt for
    /// good, and the mark is copied here from the snapshot.
    pub cheated: bool,
    /// Token usage, when the agent reports it.
    pub llm: Option<Llm>,
    /// Why the run ended: `outcome`, `ticks`, `turns`, `deadline`, `finish`,
    /// `agent_exit`, `game_error`.
    pub ended_by: String,
    /// What the agent said when it called `finish`, if it did.
    pub agent_status: Option<String>,
    /// The agent's own report, verbatim; data, never a score input.
    pub agent_report: Option<String>,
    /// Where things stood when the run ended, for a goal the scenario does
    /// not score itself: the autopilot, the well, and the range to every
    /// contact, beacon and body. See [`end_state`].
    pub end: Value,
}

/// The end state as one table line: the helm, then the nearest body and
/// beacon. `None` when the run recorded no view.
fn end_row(end: &Value) -> Option<String> {
    if end.is_null() {
        return None;
    }
    let helm = match end["autopilot"].as_object() {
        Some(engaged) => format!(
            "{} {} ({})",
            engaged["action"].as_str().unwrap_or("?"),
            engaged["target"].as_str().unwrap_or("-"),
            engaged["phase"].as_str().unwrap_or("?")
        ),
        None => "manual".into(),
    };
    let nearest = |list: &Value, key: &str| -> Option<String> {
        list.as_array()?
            .iter()
            .filter_map(|item| Some((item["id"].as_str()?, item[key].as_f64()?)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, range)| format!("{id} {range:.0} m"))
    };
    let mut parts = vec![format!("helm {helm}")];
    if let Some(well) = end["gravity_well"].as_str() {
        parts.push(format!("well {well}"));
    }
    if let Some(body) = nearest(&end["bodies"], "surface_m") {
        parts.push(format!("body {body}"));
    }
    if let Some(beacon) = nearest(&end["beacons"], "distance_m") {
        parts.push(format!("beacon {beacon}"));
    }
    if let Some(contact) = nearest(&end["contacts"], "distance_m") {
        parts.push(format!("contact {contact}"));
    }
    Some(parts.join(", "))
}

/// One of the view's plain lists, as a slice the range cut can walk.
fn list(value: &Value) -> Vec<Value> {
    value.as_array().cloned().unwrap_or_default()
}

/// The end state a reader grades an open goal against, cut from the last
/// pilot's view: the helm (autopilot engaged and completed, the dominant
/// well, speed) and the range to every contact, beacon and body.
///
/// Pass a view condensed with `expand: ["all"]`: a summary view carries only
/// the bodies the pilot was acting on, and an end state is read by a person
/// grading a goal, not by the pilot.
pub fn end_state(view: &Value) -> Value {
    let me = &view["me"];
    let ranges = |list: &[Value], keys: &[&str]| -> Vec<Value> {
        list.iter()
            .map(|item| {
                let mut record = json!({ "id": item["id"] });
                for key in keys {
                    if !item[*key].is_null() {
                        record[*key] = item[*key].clone();
                    }
                }
                record
            })
            .collect()
    };
    json!({
        "autopilot": me["autopilot"]["engaged"],
        "autopilot_completed": me["autopilot"]["completed"],
        "gravity_well": me["gravity_well"],
        "speed_mps": me["speed_mps"],
        "travel_lock": me["travel_lock"],
        "contacts": ranges(&list(&view["contacts"]), &["distance_m", "defeated"]),
        "beacons": ranges(&list(&view["beacons"]), &["distance_m"]),
        "bodies": ranges(&bodies_of(view), &["surface_m", "radius_m"]),
    })
}

/// What the previous snapshot said, so the next one can be diffed.
#[derive(Debug, Default)]
struct Previous {
    health: Option<f64>,
    alive: BTreeMap<String, bool>,
    rounds: BTreeMap<String, u64>,
    defeated: BTreeSet<String>,
    hostiles: BTreeSet<String>,
}

/// The scorer: a [`Score`] and the memory to diff against.
#[derive(Debug, Default)]
pub struct Scorer {
    /// The running score.
    pub score: Score,
    previous: Previous,
    defeated_already: bool,
}

impl Scorer {
    /// Fold one snapshot in. Order matters: the first call sets the baseline
    /// nothing is diffed against.
    pub fn observe(&mut self, snapshot: &Value) {
        let mission = &snapshot["mission"];
        let outcome = mission["outcome"]["kind"].as_str().map(str::to_lowercase);
        if outcome.as_deref() == Some("defeat") {
            self.defeated_already = true;
        }
        self.score.outcome = outcome.unwrap_or_else(|| "none".into());

        let current: BTreeSet<String> = mission["objectives"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|objective| objective["id"].as_str().map(str::to_string))
            .collect();
        if !self.defeated_already {
            for id in self.score.objectives_seen.iter() {
                if !current.contains(id) {
                    self.score.objectives_completed.insert(id.clone());
                }
            }
        }
        self.score.objectives_seen.extend(current);

        // The live list is SAMPLED, one reading per act, so a card posted and
        // completed inside one act is invisible to it. The flight log is the
        // record NOVA OS prints and it keeps both halves, so it counts a card
        // the sampling missed - and a logged completion is an event, not the
        // disappearance the defeat guard above exists to distrust.
        for entry in mission["log"].as_array().into_iter().flatten() {
            let Some(id) = entry["id"].as_str() else {
                continue;
            };
            self.score.objectives_seen.insert(id.to_string());
            if entry["kind"] == "completed" {
                self.score.objectives_completed.insert(id.to_string());
            }
        }

        let ships = snapshot["ships"].as_array().cloned().unwrap_or_default();
        if let Some(me) = ships.iter().find(|ship| ship["controller"] == "Player") {
            self.observe_me(me);
        }
        let mut present = BTreeSet::new();
        for ship in ships.iter().filter(|ship| ship["allegiance"] == "Enemy") {
            let Some(id) = ship["id"].as_str() else {
                continue;
            };
            present.insert(id.to_string());
            self.previous.hostiles.insert(id.to_string());
            if ship["defeated"] == true && self.previous.defeated.insert(id.to_string()) {
                self.score.kills += 1;
            }
        }
        for id in self.previous.hostiles.difference(&present) {
            if self.previous.defeated.insert(id.clone()) {
                self.score.kills += 1;
            }
        }
        if mission["cheats"]["marked"] == true {
            self.score.cheated = true;
        }
    }

    fn observe_me(&mut self, me: &Value) {
        if let Some(health) = me["health"]["current"].as_f64() {
            if let Some(previous) = self.previous.health {
                if health < previous {
                    self.score.damage_taken += previous - health;
                }
            }
            self.previous.health = Some(health);
        }
        for section in me["sections"].as_array().into_iter().flatten() {
            let Some(id) = section["id"].as_str() else {
                continue;
            };
            let alive = section["alive"] != false;
            if let Some(true) = self.previous.alive.insert(id.to_string(), alive) {
                if !alive {
                    self.score.sections_lost += 1;
                }
            }
            if let Some(rounds) = section["weapon"]["ammo"]["rounds"].as_u64() {
                if let Some(previous) = self.previous.rounds.insert(id.to_string(), rounds) {
                    if rounds < previous {
                        self.score.ammo_spent += previous - rounds;
                    }
                }
            }
        }
    }

    /// Count the game's own error lines (a wire line it refused to parse).
    pub fn bad_lines(&mut self, count: u64) {
        self.score.bad_lines += count;
    }

    /// Fold one assistant message's usage in.
    pub fn usage(&mut self, usage: &Value) {
        let llm = self.score.llm.get_or_insert_with(Llm::default);
        llm.messages += 1;
        llm.input += usage["input"].as_u64().unwrap_or(0);
        llm.output += usage["output"].as_u64().unwrap_or(0);
        llm.cache_read += usage["cacheRead"].as_u64().unwrap_or(0);
        llm.cache_write += usage["cacheWrite"].as_u64().unwrap_or(0);
        llm.cost += usage["cost"]["total"].as_f64().unwrap_or(0.0);
    }

    /// The score as it is written to `score.json` and the audit.
    pub fn to_json(&self) -> Value {
        serde_json::to_value(&self.score).unwrap_or_else(|_| json!({}))
    }

    /// The score as the log prints it: one line per metric that moved.
    pub fn table(&self) -> String {
        let score = &self.score;
        let mut rows = vec![
            format!("outcome              {}", score.outcome),
            format!("ended_by             {}", score.ended_by),
            format!(
                "objectives           {}/{}",
                score.objectives_completed.len(),
                score.objectives_seen.len()
            ),
            format!(
                "ticks                {} ({:.1} s game, {:.1} s wall)",
                score.ticks, score.game_seconds, score.wall_seconds
            ),
            format!("turns / gestures     {} / {}", score.turns, score.gestures),
            format!("damage_taken         {:.1}", score.damage_taken),
            format!("sections_lost        {}", score.sections_lost),
            format!("ammo_spent           {}", score.ammo_spent),
            format!("kills                {}", score.kills),
            format!("bad_lines            {}", score.bad_lines),
        ];
        if score.cheated {
            rows.push("cheats               ARMED: this run is marked".into());
        }
        if let Some(llm) = &score.llm {
            rows.push(format!(
                "llm                  {} messages, {} in ({} cached), {} out, cost {:.4}",
                llm.messages, llm.input, llm.cache_read, llm.output, llm.cost
            ));
        }
        if let Some(status) = &score.agent_status {
            rows.push(format!("agent_status         {status}"));
        }
        if let Some(end) = end_row(&score.end) {
            rows.push(format!("end                  {end}"));
        }
        rows.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hostile_that_leaves_the_world_counts_as_a_kill() {
        let mut scorer = Scorer::default();
        scorer.observe(&world(100.0, 10, false, &[]));
        assert_eq!(scorer.score.kills, 0);
        let mut gone = world(100.0, 10, false, &[]);
        gone["ships"]
            .as_array_mut()
            .unwrap()
            .retain(|ship| ship["id"] != "raider_1");
        scorer.observe(&gone);
        assert_eq!(scorer.score.kills, 1);
        scorer.observe(&gone);
        assert_eq!(scorer.score.kills, 1, "a hull only breaks up once");
        scorer.observe(&world(100.0, 10, true, &[]));
        assert_eq!(
            scorer.score.kills, 1,
            "a defeated hull that returns is the same kill"
        );
    }

    #[test]
    fn the_end_row_names_the_helm_and_the_nearest_things() {
        let view = json!({
            "me": {
                "autopilot": { "engaged": { "action": "Orbit", "target": "planetoid", "phase": "Hold" }, "completed": null },
                "gravity_well": "planetoid", "speed_mps": 41.2, "travel_lock": null
            },
            "contacts": [{ "id": "derelict", "distance_m": 2400.0, "defeated": false, "bearing_deg": [1, 2] }],
            "beacons": [{ "id": "work_mark", "distance_m": 5000.5 }],
            "bodies": {
                "near": [{ "id": "planetoid", "surface_m": 812.0, "radius_m": 633.0 }],
                "in_the_way": [],
                "groups": [{
                    "key": "5-10km.bow", "count": 1,
                    "bodies": [{ "id": "rock_small", "surface_m": 6000.0, "radius_m": 60.0 }]
                }]
            }
        });
        let end = end_state(&view);
        assert_eq!(end["autopilot"]["action"], "Orbit");
        assert!(end["contacts"][0].get("bearing_deg").is_none());
        assert_eq!(
            end_row(&end).unwrap(),
            "helm Orbit planetoid (Hold), well planetoid, body planetoid 812 m, beacon work_mark 5000 m, contact derelict 2400 m"
        );
        assert_eq!(end_row(&Value::Null), None);
        let manual = end_state(&json!({ "me": { "autopilot": { "engaged": null } } }));
        assert_eq!(end_row(&manual).unwrap(), "helm manual");
    }

    fn world(health: f64, rounds: u64, raider_defeated: bool, objectives: &[&str]) -> Value {
        json!({
            "mission": {
                "objectives": objectives.iter().map(|id| json!({ "id": id, "message": "" })).collect::<Vec<_>>(),
                "outcome": null,
                "comms": [],
            },
            "ships": [
                {
                    "id": "player", "controller": "Player", "allegiance": "Player",
                    "health": { "current": health, "max": 800 },
                    "sections": [
                        { "id": "turret_port", "alive": true, "weapon": { "ammo": { "rounds": rounds, "capacity": 40 } } },
                        { "id": "fin", "alive": health > 500.0 }
                    ]
                },
                { "id": "raider_1", "controller": "AI", "allegiance": "Enemy", "defeated": raider_defeated }
            ],
            "applied": []
        })
    }

    #[test]
    fn the_score_diffs_damage_ammo_kills_and_completed_objectives() {
        let mut scorer = Scorer::default();
        scorer.observe(&world(800.0, 40, false, &["close", "kill"]));
        scorer.observe(&world(700.0, 30, false, &["kill"]));
        scorer.observe(&world(750.0, 40, false, &["kill"]));
        scorer.observe(&world(400.0, 25, true, &[]));
        let score = &scorer.score;
        assert_eq!(score.damage_taken, 450.0);
        assert_eq!(score.ammo_spent, 25);
        assert_eq!(score.kills, 1);
        assert_eq!(score.sections_lost, 1);
        assert_eq!(
            score.objectives_completed,
            BTreeSet::from(["close".to_string(), "kill".to_string()])
        );
        assert_eq!(score.outcome, "none");
    }

    /// The live objective list is sampled once per act. A card that is posted
    /// and completed inside one act never appears in it, and only the flight
    /// log remembers it happened.
    #[test]
    fn a_card_posted_and_completed_between_two_acts_still_counts() {
        let mut scorer = Scorer::default();
        scorer.observe(&world(800.0, 40, false, &["kill"]));
        let mut later = world(800.0, 40, false, &["kill"]);
        later["mission"]["log"] = json!([
            { "kind": "posted", "id": "scan", "message": "Scan the wreck." },
            { "kind": "completed", "id": "scan", "message": "Scan the wreck." },
            { "kind": "posted", "id": "kill", "message": "Kill the raider." },
        ]);
        scorer.observe(&later);
        assert!(scorer.score.objectives_seen.contains("scan"));
        assert!(scorer.score.objectives_completed.contains("scan"));
        assert!(
            !scorer.score.objectives_completed.contains("kill"),
            "a card the log only posted is not complete"
        );
    }

    /// The mark is copied off the world, not asked of the agent, and it
    /// sticks: the run that armed cheats never scores clean again.
    #[test]
    fn an_armed_cheat_marks_the_score_for_good() {
        let mut scorer = Scorer::default();
        scorer.observe(&world(800.0, 40, false, &[]));
        assert!(!scorer.score.cheated);
        let mut armed = world(800.0, 40, false, &[]);
        armed["mission"]["cheats"] = json!({ "armed": true, "marked": true });
        scorer.observe(&armed);
        assert!(scorer.score.cheated);
        assert!(scorer.table().contains("cheats               ARMED"));
        scorer.observe(&world(800.0, 40, false, &[]));
        assert!(scorer.score.cheated, "the mark outlives the command");
    }

    #[test]
    fn a_defeat_stops_counting_vanished_objectives_as_completed() {
        let mut scorer = Scorer::default();
        scorer.observe(&world(800.0, 40, false, &["survive"]));
        let mut lost = world(0.0, 40, false, &[]);
        lost["mission"]["outcome"] = json!({ "kind": "Defeat", "message": "Hull lost." });
        scorer.observe(&lost);
        assert_eq!(scorer.score.outcome, "defeat");
        assert!(scorer.score.objectives_completed.is_empty());
        assert_eq!(scorer.score.objectives_seen.len(), 1);
    }

    #[test]
    fn usage_accumulates_and_the_table_prints_it() {
        let mut scorer = Scorer::default();
        scorer.usage(
            &json!({ "input": 100, "output": 20, "cacheRead": 50, "cost": { "total": 0.01 } }),
        );
        scorer.usage(
            &json!({ "input": 120, "output": 30, "cacheRead": 90, "cost": { "total": 0.02 } }),
        );
        let llm = scorer.score.llm.as_ref().unwrap();
        assert_eq!(
            (llm.messages, llm.input, llm.output, llm.cache_read),
            (2, 220, 50, 140)
        );
        assert!((llm.cost - 0.03).abs() < 1e-9);
        assert!(scorer.table().contains("llm                  2 messages"));
    }
}
