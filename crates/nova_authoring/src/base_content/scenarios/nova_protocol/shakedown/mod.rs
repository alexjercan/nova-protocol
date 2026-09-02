//! "Shakedown Run" - the starter scenario New Game drops the player into (beat
//! sheet in).
//!
//! Five beats, each introducing one verb where it is the natural tool:
//! burn to a beacon (W), freelook to find the next one (Alt), weave a
//! debris cluster collecting crates (X earns its keep), hand the ship to
//! the computer (G GOTO, O ORBIT), and drive off a single gentle pirate
//! that snuck into the debris field (RMB/LMB/combat). Objectives carry the
//! key names in brackets, matching the hint-cluster labels; beacons and
//! crates self-advertise (blink, glow, HUD chips) - the layer-0/1
//! conveyance of the spike.
//!
//! Script shape: one `beat` counter variable gates every handler, so a
//! stray re-entry cannot re-fire a finished beat. Count milestones (the
//! crate tally) advance on `OnUpdate` handlers keyed on the count value
//! rather than piggybacking the pickup event - handler execution order
//! within one event is query-iteration order, and the update-gated form
//! does not depend on it.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use super::{
    cast::{CAPTAIN_HALLORAN, PLAYER},
    pacing::{self, INSTRUCTION_GAP, MID_GAP, REVEAL_GAP},
    ships, SCENARIO_ELAPSED_VAR,
};
use crate::scenario_helpers::prelude::*;

/// The scenario id, shared with nova_menu's New Game entry.
pub const SHAKEDOWN_SCENARIO_ID: &str = "shakedown_run";

// Layout. Distances are deliberately short (a few kilometres between
// objectives): "close enough to see" is the cheapest objective marker. The
// planetoid numbers are authored against the RUNTIME geometry, not the nominal
// radius (the authored-vs-derived lesson): a nominal-200 m asteroid's noise
// mesh reaches ASTEROID_GEOMETRIC_FACTOR_MIN..MAX times its nominal radius
// (3.5-6.0, pinned by nova_scenario's seed sweep; observed [3.70, 5.64] over
// 256 seeds), so the geometric body radius runs 700-1 200 m, the SOI (8x)
// 5.6-9.6 km, and the ORBIT ring (1.5 * (body_radius + 10 m)) 1.06-1.82 km.
// The config-shape tests below pin the layout against the WHOLE range - caught
// the first cut assuming a single observed seed band (4.0-4.55), under which a
// high-factor seed parked the orbit ring OUTSIDE the old 1.6 km gate and
// soft-locked beat 4.
const PLAYER_SPAWN: Meters3 = Meters3::ZERO;
/// Beat 1: dead ahead of the spawn heading (-Z).
const BEACON_1_POS: Meters3 = Meters3::new(0.0, 0.0, -3_500.0);
/// Beat 2: ~120 degrees off the beacon-1 boresight, so freelook (or a
/// deliberate turn) is genuinely how you find it.
const BEACON_2_POS: Meters3 = Meters3::new(2_600.0, 200.0, -2_000.0);
/// Beat 3: a loose debris cluster past beacon 2 - pushed out so no crate
/// sensor overlaps the (now standoff-sized) beacon trigger.
const DEBRIS_CENTER: Meters3 = Meters3::new(3_500.0, 200.0, -1_600.0);
/// The three salvage crates, strung ALONG the cluster rather than bunched. The
/// old scatter sat ~290-370 m apart, so with the 80 m pickup radius (160 m
/// sensor diameter) a fast pass could sweep two sensors almost at once and they
/// read as a single pickup. These are spread to at least 530 m center-to-center
/// (a ~370 m gap between sensor surfaces), so each pickup registers as its own
/// moment - reinforced by the per-crate pickup cue. The spread is pinned by
/// `crates_are_spaced_for_distinct_pickups` and stays clear of beacon 2's
/// trigger and the planetoid SOI (the geometry tests below).
const CRATE_POSITIONS: [Meters3; 3] = [
    Meters3::new(3_450.0, 300.0, -1_900.0),
    Meters3::new(3_600.0, 50.0, -1_450.0),
    Meters3::new(3_950.0, 350.0, -1_100.0),
];
/// The stage dressing and late-run destination: a planetoid with a real
/// gravity well. Pulled in to ~7.6 km of the spawn so it is a LANDMARK the
/// early beats fly against and the belt below can bend around it, instead of a
/// speck on the horizon. Playtest round 2 finding 1 (the player fighting
/// gravity while weaving crates) is now held by the MASS, not by distance: the
/// mass below buys a 3.29 km SOI, which still falls short of the debris
/// cluster. The SOI edge is crossed on the waypoint leg.
const PLANETOID_POS: Meters3 = Meters3::new(5_000.0, -400.0, -5_600.0);
const PLANETOID_NOMINAL_RADIUS: Meters = Meters(200.0);
/// The planetoid's mass parameter (mu, u^3/s^2) - the only authored gravity
/// number, setting both the pull and the reach. Tune it by the SOI the layout
/// wants: `mu = soi_cutoff_accel * soi^2`. At the engine default this is a
/// 3.29 km SOI on every mesh seed, which is what beat 4 below is authored
/// against; the pull at the geometric surface (700-1 200 m) runs 19-55 m/s^2,
/// under the escapability cap on every seed. Sized DOWN with the move above:
/// the well must still stop short of the crate beat now that the body itself
/// is close.
const PLANETOID_MASS: f32 = 27_000.0;
/// The FIRST radar-lock target (beat sheet v2): a comfortable GOTO leg from the
/// debris cluster, OUTSIDE the planetoid SOI so the hands-off ride is
/// gravity-free, and inside the default beacon lock range (6 km) from the
/// cluster.
const BEACON_3_POS: Meters3 = Meters3::new(6_000.0, 900.0, 1_200.0);
/// The waypoint-run target: 2.4 km out from the planetoid - inside the SOI (so
/// the ORBIT hint lights on arrival) with its trigger clear of both the widest
/// orbit ring and the coast ring (the already-inside-when-armed trap; pinned
/// below). The beacon-3 -> beacon-4 leg (~5.4 km) is beyond the DEFAULT beacon
/// lock range, so beacon 4 authors the signature its leg needs (pinned below).
const BEACON_4_POS: Meters3 = Meters3::new(6_800.0, 100.0, -4_100.0);
const BEACON_4_LOCK_SIGNATURE: Meters = Meters(300.0);
/// The gravity-coast ring: a planetoid-centered invisible trigger sphere.
/// Entering it (drifting in from the beacon-4 park) is the coast beat; LEAVING
/// it after the held orbit is the break-away beat. Outside the widest orbit
/// ring, inside the SOI, and just inside the nominal beacon-4 park so
/// the coast is SHORT (playtest 2026-07-13: the 2.1 km ring made the drift read
/// as dead air) - all pinned below. A player somehow already inside when the
/// ring spawns still advances: a spawned area fires OnEnter for bodies it lands
/// on (pinned in nova_scenario's area tests).
const COAST_RING_RADIUS: Meters = Meters(2_400.0);
/// The live-fire rehearsal target: an inert three-section hulk drifting near
/// the old salvage field, outside the SOI so it stays where the lesson put it.
const DERELICT_POS: Meters3 = Meters3::new(3_000.0, -400.0, 400.0);
/// The pirate spawns back at the debris cluster once the rehearsal is
/// done, and patrols it.
const PIRATE_SPAWN: Meters3 = Meters3::new(3_800.0, 400.0, -1_000.0);
const PIRATE_PATROL: [Meters3; 3] = [
    Meters3::new(3_000.0, 200.0, -1_700.0),
    Meters3::new(3_600.0, 250.0, -1_100.0),
    Meters3::new(3_300.0, 600.0, -1_400.0),
];
/// Beacon trigger radius. MUST contain the GOTO park point: the autopilot
/// stops arrival_standoff (500 m, FlightSettings) from an unsized target,
/// and a trigger smaller than that leaves the ship parked 100 m OUTSIDE its
/// own objective (playtest 2026-07-12 finding 2). Pinned by a config test
/// against FlightSettings::default().
const BEACON_AREA_RADIUS: Meters = Meters(700.0);
/// Crate pickup radius: tight enough to require flying AT the crate.
const CRATE_AREA_RADIUS: Meters = Meters(80.0);

/// One knot of the slalom belt: a world-space box of seeded rocks.
struct BeltKnot {
    id_prefix: &'static str,
    center: Meters3,
    /// Half-extents of the box, per axis.
    half_extent: Meters3,
    seed: u64,
    count: u32,
}

/// The slalom belt: five near knots strung into a banana that leaves the spawn
/// corridor, alternates sides of the beacon-1 -> beacon-2 line, and bends its
/// tail around the planetoid. The scenario used to read as empty black sky with
/// 9 rocks in it; these give the early legs something to fly THROUGH.
///
/// Every knot obeys the pocket rule - its BOX SURFACE, plus a rock's own
/// collider, plus 200 m, clears the player spawn, every beacon trigger, the
/// debris cluster, the derelict and the planetoid's widest orbit ring - so no
/// beat loses the air it needs. The same rule holds against every AUTOPILOT leg
/// (the GOTO to beacon 3, the waypoint run to beacon 4, the run in to the
/// orbit): the player is hands-off there and cannot dodge, so those corridors
/// carry no rock. Both pinned by `belt_knots_keep_every_beat_pocket_clear`.
///
/// The knot boxes MAY abut: separation is shared across scatters, so the seam
/// between two knots is as safe as the inside of one.
///
/// Counts are owner round-2 numbers: the round-1 26-per-knot belt read as
/// cluttered.
// The seeds read `<date>_<index>`, not a magnitude. Thousands separators would
// destroy that, so the grouping stays as authored.
#[expect(
    clippy::inconsistent_digit_grouping,
    reason = "seeds are date_index, not magnitudes"
)]
const BELT_KNOTS: [BeltKnot; 5] = [
    BeltKnot {
        id_prefix: "belt_k1_",
        center: Meters3::new(550.0, -200.0, -1_700.0),
        half_extent: Meters3::new(700.0, 350.0, 700.0),
        seed: 20260805_001,
        count: 12,
    },
    BeltKnot {
        id_prefix: "belt_k2_",
        center: Meters3::new(1_700.0, 350.0, -4_600.0),
        half_extent: Meters3::new(700.0, 350.0, 700.0),
        seed: 20260805_002,
        count: 12,
    },
    BeltKnot {
        id_prefix: "belt_k3_",
        center: Meters3::new(2_450.0, 450.0, -3_800.0),
        half_extent: Meters3::new(700.0, 350.0, 700.0),
        seed: 20260805_003,
        count: 12,
    },
    // 1 700 on x, not the sketched 2 000: at 2 000 this knot's box runs into
    // the planetoid's widest orbit ring.
    BeltKnot {
        id_prefix: "belt_k4_",
        center: Meters3::new(1_700.0, 200.0, -5_600.0),
        half_extent: Meters3::new(850.0, 400.0, 850.0),
        seed: 20260805_004,
        count: 12,
    },
    BeltKnot {
        id_prefix: "belt_k5_",
        center: Meters3::new(200.0, 150.0, -5_450.0),
        half_extent: Meters3::new(850.0, 400.0, 850.0),
        seed: 20260805_005,
        count: 12,
    },
];
/// Near-knot NOMINAL rock radius range. Read it against the noise mesh, not as
/// a size: the collider reaches `radius * ASTEROID_GEOMETRIC_FACTOR_MAX`, so
/// 20 m nominal is a 120 m body. Round 1 authored 34 m (a 200 m body) and the
/// knots spawned inside each other.
const BELT_ROCK_RADIUS: (Meters, Meters) = (Meters(8.0), Meters(20.0));
/// Two widest near rocks side by side (2 * 20 m * 6.0) plus a gap. Below this
/// the scatter places rocks that intersect on spawn, and dynamic bodies born
/// overlapping shove each other apart hard enough to destroy each other.
const BELT_ROCK_SEPARATION: Meters = Meters(320.0);
/// The far parallax layer: one planetoid-centred ring of bigger rocks, giving
/// the belt depth behind the near knots (the reference screenshot's two-scale
/// split). Its hole CONTAINS the whole playable volume (the farthest beat point
/// is the spawn, ~7.52 km from the planetoid) - a seeded ring cannot be aimed
/// rock-by-rock, so the only way to keep it out of a beat is to keep it out of
/// every beat. Pinned.
const BELT_FAR_PREFIX: &str = "belt_far_";
const BELT_FAR_COUNT: u32 = 18;
#[expect(
    clippy::inconsistent_digit_grouping,
    reason = "seeds are date_index, not magnitudes"
)]
const BELT_FAR_SEED: u64 = 20260805_100;
const BELT_FAR_RING: (Meters, Meters) = (Meters(10_500.0), Meters(14_500.0));
const BELT_FAR_Y_SPREAD: Meters = Meters(2_000.0);
/// DO NOT raise past `GravitySettings::min_well_radius` (50 m). At or above it
/// every far rock would get the default well (mu 4 000, ~1.26 km SOI each) and
/// spray gravity across the legs authored to be gravity-free. Pinned.
const BELT_FAR_RADIUS: (Meters, Meters) = (Meters(40.0), Meters(49.0));
/// Two widest far rocks side by side (2 * 49 m * 6.0) plus a gap.
const BELT_FAR_SEPARATION: Meters = Meters(800.0);

const BEACON_COLOR: Color = Color::srgb(0.3, 0.9, 1.0);

/// The scavenger's territorial tether around its patrol centroid: combat
/// breaks off beyond it, keeping the beat-5 fight at the debris field
/// (playtest round 3 finding 3).
const PIRATE_LEASH_RADIUS: Meters = Meters(1_500.0);

/// Soft manual-speed cap on the starter ship: at 250 m/s a 3.5 km leg
/// still takes a quarter minute and a missed brake does not send a new
/// pilot sailing out of the play area (playtest 2026-07-12 finding 1).
const PLAYER_SPEED_CAP: MetersPerSecond = MetersPerSecond(250.0);

// Scenario entity ids (strings are the script's wiring; the config-shape
// test cross-checks every reference against the spawn set).
const ID_PLAYER: &str = "player_spaceship";
const ID_BEACON_1: &str = "beacon_1";
const ID_BEACON_2: &str = "beacon_2";
const ID_BEACON_3: &str = "beacon_3";
const ID_BEACON_4: &str = "beacon_4";
const ID_COAST_RING: &str = "coast_ring";
const ID_DERELICT: &str = "derelict";
const ID_PLANETOID: &str = "planetoid";
const ID_PIRATE: &str = "pirate";

// Objective ids (beat sheet v2: one gesture per objective).
const OBJ_B1: &str = "b1_burn";
const OBJ_B2: &str = "b2_look";
const OBJ_B3: &str = "b3_salvage";
const OBJ_B4: &str = "b4_lock";
const OBJ_B5: &str = "b5_autopilot";
const OBJ_B6: &str = "b6_waypoint";
const OBJ_B7: &str = "b7_coast";
const OBJ_B8: &str = "b8_orbit";
const OBJ_B9: &str = "b9_break";
const OBJ_B10: &str = "b10_paint";
const OBJ_B11: &str = "b11_fire";
const OBJ_B12: &str = "b12_contact";
const OBJ_DONE: &str = "done";

// Script variables.
const VAR_BEAT: &str = "beat";
/// The outro beat: the scavenger is down and the win is locked, but the
/// Victory overlay has not landed yet. The defeat handlers gate BELOW it, so
/// clipping a rock during the outro cannot overwrite the win.
const BEAT_OUTRO: f64 = 13.0;
/// The won beat, set with the banner.
const BEAT_WON: f64 = 14.0;
const VAR_CRATES: &str = "crates_recovered";
/// The highest beat whose setup has fired. Not a latch - `beat_setup` no longer
/// needs one - but a SIGNAL: the salvage pickups wait on it to know their
/// objective has posted.
const VAR_SETUP_LAST: &str = "setup_last";
const TIMER_ORBIT_HOLD: &str = "orbit_hold";
const ORBIT_HOLD_SECS: f64 = 5.0;

/// The opening conversation's sequence key.
const SEQ_OPENING: &str = "opening";
/// The scavenger fight is a threat reveal: the warning line lands with the
/// spawn, and the objective posts a beat later - the same gap the story
/// scenarios use, so no comms line shares a frame with an objective anywhere in
/// the mainline.
///
/// A TIMER rather than a `Sequence` step, because this beat must be ABANDONED
/// if the world moves on: a fast kill sets beat 13 inside the gap, and the
/// objective would then post under the Victory overlay with nothing left to
/// complete it. A step runs when its delay elapses; only a handler can still
/// ask whether the beat is current.
const TIMER_SCAV_GATE: &str = "scav_gate";

// The opening conversation runs on the scenario clock (seconds). The 250 m/s
// speed cap makes the ~40s drift diegetic: the ship idles out of the dock while
// Capt. Halloran talks, and objective 1 posts only when she sends you off.
/// The opening conversation's first line, and the gap between the four that
/// follow it. Steps of one sequence now, so each is a gap from the PREVIOUS
/// line rather than an absolute clock mark.
const OPEN_1_AT: f64 = 2.0;
const OPEN_GAP: f64 = 3.0;
// The gap between a beat transition and the objective it introduces, in seconds
// of play time. The transition completes the previous objective and plays the
// beat's comms line; the next objective (and its beacon) posts a gap LATER, not
// the same frame (owner playtest). The gap is chosen PER BEAT by the line's
// relationship to the objective (pacing): INSTRUCTION_GAP when the objective
// echoes a coaching line (most nav beats - the objective lands mid-read),
// MID_GAP for a reveal-then-instruct line, REVEAL_GAP for a threat the player
// absorbs first (the scavenger). Each transition's `stamp_gate` call names its
// category.

/// OnEnter of `area` by the player ship.
fn player_enters(area: &str) -> EventFilterConfig {
    entity_pair(area, ID_PLAYER)
}

/// One line of the opening conversation, `after` seconds behind the previous
/// one. The lines are steps of a single sequence, so the ordering is the
/// cursor's - the five lines used to be five handlers sequenced by a counter
/// variable each of them read and wrote.
fn open_line(after: f64, speaker: &str, line: &str) -> SequenceStepConfig {
    step(after, vec![story_message(speaker, line)])
}

/// Post a beat's world - its objective, its beacon, its markers and any hint
/// emphasis - a beat AFTER the transition that plays its comms line, so the
/// introducing line finishes before the new objective appears (owner playtest:
/// "wait at least for the dialogue to finish before we add a new objective").
///
/// The transition runs this as its LAST action. `delay` is the transition's
/// pacing category (INSTRUCTION_GAP / MID_GAP / REVEAL_GAP), chosen by how its
/// line relates to the objective.
///
/// It used to be a handler of its own, waiting on a `beat == N` filter and a
/// deadline the transition stamped into a shared gate variable. The cursor
/// makes both structural: the chain belongs to the transition that started it.
/// `setup_last` STAYS, because it is also a signal - mid-beat handlers (the
/// salvage pickups) wait on it to know their objective has posted.
fn beat_setup(beat: f64, delay: f64, actions: Vec<EventActionConfig>) -> EventActionConfig {
    let mut all = vec![set_variable(VAR_SETUP_LAST, number(beat))];
    all.extend(actions);
    pacing::beat_later(&format!("beat_{beat}"), delay, all)
}

fn beacon(id: &str, label: &str, position: Meters3) -> ScenarioObjectConfig {
    beacon_with_signature(id, label, position, None)
}

/// A beacon whose radar signature is authored for a longer-than-default
/// GOTO leg (beacon 4's waypoint run; the leg-vs-range pin lives in the
/// geometry test).
fn beacon_with_signature(
    id: &str,
    label: &str,
    position: Meters3,
    lock_signature: Option<Meters>,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: label.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: label.to_string(),
            radius: Meters(20.0),
            color: BEACON_COLOR,
            area_radius: Some(BEACON_AREA_RADIUS),
            lock_signature,
        }),
    }
}

/// The live-fire rehearsal target: three connected light hull cells, with no
/// pilot, systems or weapons. It teaches the ship damage model the next beat
/// asks the player to use instead of making a rock impersonate a wreck.
fn derelict() -> ScenarioObjectConfig {
    let section = |id: &str, z: f32| SpaceshipSectionConfig {
        id: id.to_string(),
        position: Vec3::new(0.0, 0.0, z),
        rotation: Quat::IDENTITY,
        source: SectionSource::Prototype("light_hull_section".to_string()),
        modifications: vec![],
    };
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_DERELICT.to_string(),
            name: "Derelict Hulk".to_string(),
            position: DERELICT_POS,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Neutral),
            hull: ships::inline_hull(vec![
                section("hulk_fore", -1.0),
                section("hulk_mid", 0.0),
                section("hulk_aft", 1.0),
            ]),
            ..Default::default()
        }),
    }
}

/// The invisible gravity-coast trigger sphere around the planetoid.
fn coast_ring() -> EventActionConfig {
    EventActionConfig::CreateScenarioArea(ScenarioAreaConfig {
        id: ID_COAST_RING.to_string(),
        name: "Coast Ring".to_string(),
        position: PLANETOID_POS,
        rotation: Quat::IDENTITY,
        radius: COAST_RING_RADIUS,
    })
}

fn crate_object(index: usize, position: Meters3) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: format!("crate_{}", index),
            name: format!("Supply Crate {}", index),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::SalvageCrate(SalvageCrateConfig {
            size: Meters(15.0),
            area_radius: CRATE_AREA_RADIUS,
            pickup_sound: Some(AssetRef::from("self://sounds/salvage_pickup.wav")),
        }),
    }
}

/// The shakedown ship: deliberately minimal - controller, one hull, one
/// thruster, ONE turret (no torpedo bay). One of everything keeps the
/// component-cycle lesson trivially readable.
fn player_ship() -> ScenarioObjectConfig {
    // The player flies the cargoa corvette - the armed-hauler hull whose pod
    // shoulders carry the two PDC turrets. Both fire on LMB / right trigger.
    //
    // GOTO/LOCK/ORBIT start WITHHELD on the corvette's controller cube: the pilot
    // has not flown a controlled leg and the targeting computer is offline. The
    // beat handlers grant them one at a time via SetControllerVerb (GOTO after
    // beat 1, LOCK at the radar beat, ORBIT when the coast objective asks).
    // Authored as DisableVerb MODIFICATIONS aimed at the shared hull's flight
    // computer (not baked into the catalog ship) so they apply from the instant
    // the controller is built, and only to THIS spawn.
    let controller_gate = vec![
        SectionModification::DisableVerb(FlightVerb::Goto),
        SectionModification::DisableVerb(FlightVerb::Lock),
        SectionModification::DisableVerb(FlightVerb::Orbit),
        // RCS is off in the mainline campaign until the rework - unlike the
        // three above, no beat re-grants it, so it stays disabled for the whole
        // run.
        SectionModification::DisableVerb(FlightVerb::Rcs),
    ];
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_PLAYER.to_string(),
            name: "Player Spaceship".to_string(),
            position: PLAYER_SPAWN,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::Player(PlayerControllerConfig {
                // Both corvette turret cubes fire on LMB / right trigger.
                input_mapping: ships::CARGOA_TURRET_IDS
                    .iter()
                    .map(|id| {
                        (
                            id.to_string(),
                            vec![
                                MouseButton::Left.into(),
                                GamepadButton::RightTrigger2.into(),
                            ],
                        )
                    })
                    .collect(),
                speed_cap: Some(PLAYER_SPEED_CAP),
                // Finite ammo: the weapons auto-reload, so a spent magazine
                // recovers on its own; the player sees the ammo readout and
                // reload cadence from the first scenario.
            }),
            hull: ships::hull(ships::CARGOA_SHIP_ID),
            modifications: vec![ships::on_section(
                ships::FUSELAGE_SECTION_ID,
                controller_gate,
            )],
        }),
    }
}

/// The scavenger: the player ship's silhouette in scavenger grade - light
/// hull, light turret - passive (patrolling the debris cluster) until the
/// player closes inside AI engage range or shoots first.
fn pirate_ship() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_PIRATE.to_string(),
            name: "Scavenger".to_string(),
            position: PIRATE_SPAWN,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::AI(AIControllerConfig {
                patrol: PIRATE_PATROL.to_vec(),
                // Territorial: the scavenger fights AT the debris field
                // and breaks off if the duel drifts away (playtest round
                // 3 finding 3) - the leash comfortably covers the patrol
                // loop and the crate scatter.
                leash: Some(PIRATE_LEASH_RADIUS),
                // Arrival grace (beat-sheet pass): the tutorial's one fight
                // announces itself - the scavenger prowls readably before its
                // guns come up.
                engage_delay: Some(5.0),
                ..Default::default()
            }),
            // A scavenger-grade corvette: weaker turrets, squishier hull.
            hull: ships::hull(ships::CARGOA_RAIDER_SHIP_ID),
            ..Default::default()
        }),
    }
}

/// The belt's rock template: the debris rock, so a belt rock is the same
/// forgiving hazard the crate beat already teaches - collidable, destructible,
/// no well of its own.
fn belt_rock(
    id_prefix: &str,
    radius: Meters,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id_prefix.to_string(),
            name: "Rock".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            material: None,
            destroy_sound: Some(AssetRef::from("self://sounds/destroy_rock.wav")),
            radius,
            texture: asteroid_texture,
            mass: None,
            invulnerable: false,
            seed: None,
            lock_signature: None,
        }),
    }
}

/// The belt's scatter actions: the five near knots as world-space boxes, then
/// the far parallax ring around the planetoid.
fn belt_scatters(asteroid_texture: &AssetRef<Image>) -> Vec<EventActionConfig> {
    let mut actions: Vec<EventActionConfig> = BELT_KNOTS
        .iter()
        .map(|knot| {
            EventActionConfig::ScatterObjects(ScatterObjectsConfig {
                id_prefix: knot.id_prefix.to_string(),
                count: knot.count,
                seed: knot.seed,
                region: ScatterRegion::Box {
                    min: knot.center - knot.half_extent,
                    max: knot.center + knot.half_extent,
                },
                template: belt_rock(knot.id_prefix, BELT_ROCK_RADIUS.0, asteroid_texture.clone()),
                asteroid_radius: Some(BELT_ROCK_RADIUS),
                min_separation: Some(BELT_ROCK_SEPARATION),
            })
        })
        .collect();
    actions.push(EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: BELT_FAR_PREFIX.to_string(),
        count: BELT_FAR_COUNT,
        seed: BELT_FAR_SEED,
        region: ScatterRegion::Ring {
            center: PLANETOID_POS,
            inner: BELT_FAR_RING.0,
            outer: BELT_FAR_RING.1,
            y_min: -BELT_FAR_Y_SPREAD,
            y_max: BELT_FAR_Y_SPREAD,
        },
        template: belt_rock(BELT_FAR_PREFIX, BELT_FAR_RADIUS.0, asteroid_texture.clone()),
        asteroid_radius: Some(BELT_FAR_RADIUS),
        min_separation: Some(BELT_FAR_SEPARATION),
    }));
    actions
}

/// The outro chain: the fight's own line lands in the win handler, and this
/// carries the hand-off to chapter two and the banner. Both win variants start
/// the same cursor - only one of them can ever fire.
fn outro() -> EventActionConfig {
    pacing::outro_sequence(
        VAR_BEAT,
        BEAT_WON,
        CAPTAIN_HALLORAN,
        "It was flying scout, though. A distress call is already crackling \
         from the deep field - and it is not one of ours.",
        "Shakedown complete. The belt is yours - and something out in the \
         deep field is already calling for help.",
        vec![post_objective(
            OBJ_DONE,
            "Shakedown complete. Tap [CTRL] to stand down your locks - the belt is yours.",
        )],
        Some(super::broadside::BROADSIDE_SCENARIO_ID.to_string()),
    )
}

pub(crate) fn shakedown_run(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    // The debris cluster: fixed offsets, not rng - the layout is content,
    // and determinism keeps the config-shape tests honest.
    const ROCK_OFFSETS: [Meters3; 9] = [
        Meters3::new(-350.0, 50.0, 200.0),
        Meters3::new(-150.0, -100.0, -250.0),
        Meters3::new(100.0, 250.0, 150.0),
        Meters3::new(300.0, -50.0, -200.0),
        Meters3::new(450.0, 150.0, 100.0),
        Meters3::new(-250.0, 300.0, -100.0),
        Meters3::new(50.0, -200.0, 300.0),
        Meters3::new(250.0, 400.0, -350.0),
        Meters3::new(-450.0, -150.0, -50.0),
    ];
    const ROCK_RADII: [Meters; 9] = [
        Meters(25.0),
        Meters(15.0),
        Meters(30.0),
        Meters(20.0),
        Meters(10.0),
        Meters(25.0),
        Meters(15.0),
        Meters(20.0),
        Meters(30.0),
    ];

    let mut start_spawns: Vec<ScenarioObjectConfig> = Vec::new();
    start_spawns.push(player_ship());
    // Beacon 1 spawns LAZILY when the opening conversation hands off to
    // objective 1, like beacons 2-4: during the ~40s captain briefing there is
    // nothing to fly to yet, so a burn cannot skip it.
    start_spawns.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_PLANETOID.to_string(),
            name: "Planetoid".to_string(),
            position: PLANETOID_POS,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            material: None,
            destroy_sound: Some(AssetRef::from("self://sounds/destroy_rock.wav")),
            radius: PLANETOID_NOMINAL_RADIUS,
            texture: asteroid_texture.clone(),
            mass: Some(PLANETOID_MASS),
            invulnerable: true,
            seed: None,
            lock_signature: None,
        }),
    });
    for (i, (offset, radius)) in ROCK_OFFSETS.iter().zip(ROCK_RADII).enumerate() {
        start_spawns.push(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: format!("debris_{}", i),
                name: format!("Debris {}", i),
                position: DEBRIS_CENTER + *offset,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                material: None,
                destroy_sound: Some(AssetRef::from("self://sounds/destroy_rock.wav")),
                radius,
                texture: asteroid_texture.clone(),
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        });
    }
    for (i, position) in CRATE_POSITIONS.iter().enumerate() {
        start_spawns.push(crate_object(i + 1, *position));
    }
    // The run lights itself: there is no engine light.
    start_spawns.extend(ThreePointRig::around("shakedown", Meters3::ZERO, 10.0).objects());

    let events = vec![
        // Beat 1 setup: the world and the variables. The opening conversation
        // (below) runs on the scenario clock before objective 1 posts; beacon 1
        // and beacons 2-4 and the pirate all spawn LAZILY with their beats, so a
        // new chip appearing on the HUD always means "this is next".
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: start_spawns
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .chain(belt_scatters(&asteroid_texture))
                .chain([
                    set_variable(VAR_BEAT, number(1.0)),
                    set_variable(VAR_CRATES, number(0.0)),
                    set_variable(VAR_SETUP_LAST, number(0.0)),
                    // The opening conversation: a five-line back-and-forth with
                    // the captain over ~40s (owner pacing pass), then the
                    // hand-off that posts objective one. The speed cap makes
                    // the drift diegetic - you idle out while she briefs you.
                    // This is the base campaign's FIRST player voice ("You");
                    // terse and professional, the belt register.
                    //
                    // No objective while she talks (owner pacing pass): the
                    // panel stays empty and the first objective posts only on
                    // the hand-off step. The conversation carries the voice;
                    // the panel waits for it.
                    sequence(
                        SEQ_OPENING,
                        vec![
                            open_line(
                                OPEN_1_AT,
                                CAPTAIN_HALLORAN,
                                "Shakedown's your own now - fresh hull, cold guns. Ease her out, \
                                 nice and slow.",
                            ),
                            open_line(
                                OPEN_GAP,
                                PLAYER,
                                "Copy, Halloran. Board's green, lines are cold.",
                            ),
                            open_line(
                                OPEN_GAP,
                                CAPTAIN_HALLORAN,
                                "Belt's quiet today. Good day to learn her helm before it isn't.",
                            ),
                            open_line(OPEN_GAP, PLAYER, "Understood. Where do you want me?"),
                            open_line(
                                OPEN_GAP,
                                CAPTAIN_HALLORAN,
                                "Salvage beacon's lit dead ahead. Burn for it when you're set - \
                                 and mind your brakes.",
                            ),
                            // Conversation over: post objective 1, spawn and
                            // mark beacon 1. The gold marker rides the current
                            // leg's target (conveyance layer 2); its beacon
                            // chip yields while marked, so each beacon shows
                            // exactly one chip.
                            step(
                                0.0,
                                vec![
                                    spawn_object(beacon(ID_BEACON_1, "BEACON 1", BEACON_1_POS)),
                                    post_objective(OBJ_B1, "Burn to Beacon 1."),
                                    attach_objective_marker(ID_BEACON_1, "BEACON 1"),
                                ],
                            ),
                        ],
                    ),
                ])
                .collect(),
        },
        // Beat 1 -> 2: reach beacon 1. Complete the leg, release the governor
        // and grant GOTO (clearing beat 1 earns it), and call the next mark.
        // The objective and beacon 2 post a beat later (beat_setup below), once
        // the captain's line lands - never the same frame.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![player_enters(ID_BEACON_1), number_equals(VAR_BEAT, 1.0)],
            actions: vec![
                set_variable(VAR_BEAT, number(2.0)),
                complete_objective(OBJ_B1),
                // The training governor releases once the pilot has proven
                // a controlled leg (playtest round 2 finding 3).
                EventActionConfig::SetSpeedCap(SetSpeedCapActionConfig {
                    id: ID_PLAYER.to_string(),
                    cap: None,
                }),
                // GOTO unlocks with the first objective: the ship starts with
                // it withheld (player_ship's controller config) and clearing
                // beat 1 grants it (spike).
                EventActionConfig::SetControllerVerb(SetControllerVerbActionConfig {
                    id: ID_PLAYER.to_string(),
                    verb: FlightVerb::Goto,
                    enabled: true,
                }),
                story_message(
                    CAPTAIN_HALLORAN,
                    "Good burn. Next one's off your beam - swing your look \
                     around and find it.",
                ),
                // Beat 2 posts off the beam a beat after the captain's call.
                beat_setup(
                    2.0,
                    INSTRUCTION_GAP,
                    vec![
                        spawn_object(beacon(ID_BEACON_2, "BEACON 2", BEACON_2_POS)),
                        post_objective(OBJ_B2, "Find Beacon 2 - hold [Alt] to look around."),
                        // Marker hand-off: attach runs after the spawn above
                        // (action list order), so the fresh beacon is findable.
                        detach_objective_marker(ID_BEACON_1),
                        attach_objective_marker(ID_BEACON_2, "BEACON 2"),
                    ],
                ),
            ],
        },
        // Beat 2 -> 3: reach beacon 2; the debris cluster is right there. The
        // pilot calls the sweep; the salvage objective and the crate markers
        // post a beat later (beat_setup below), once the line lands.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![player_enters(ID_BEACON_2), number_equals(VAR_BEAT, 2.0)],
            actions: vec![
                set_variable(VAR_BEAT, number(3.0)),
                complete_objective(OBJ_B2),
                story_message(
                    PLAYER,
                    "Salvage beacons. I'll sweep the cluster and pull them in.",
                ),
                // Beat 3 posts the sweep a beat after the call. The crate
                // markers post here too, so a pickup cannot land before the
                // objective (the pickup handlers below wait on
                // `setup_last == 3`).
                beat_setup(
                    3.0,
                    INSTRUCTION_GAP,
                    vec![
                        post_objective(OBJ_B3, "Recover the 3 supply crates."),
                        // All three crates carry the marker at once; each dies
                        // with its crate, so the survivors answer "which is left".
                        detach_objective_marker(ID_BEACON_2),
                        attach_objective_marker("crate_1", "SALVAGE"),
                        attach_objective_marker("crate_2", "SALVAGE"),
                        attach_objective_marker("crate_3", "SALVAGE"),
                    ],
                ),
            ],
        },
        // Beat 3 pickups: one handler per crate (the despawn action needs the
        // concrete id). Counting is a variable; the tally text and the beat
        // advance are OnUpdate handlers below, so nothing depends on handler
        // order within the pickup event. The pickups wait on beat 3's setup
        // (`setup_last == 3`): the crates exist from OnStart, so without this
        // guard a pickup during the intro line would count against an objective
        // that has not posted yet, and beat_setup would then overwrite the
        // tally text.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![
                player_enters("crate_1"),
                number_equals(VAR_BEAT, 3.0),
                number_equals(VAR_SETUP_LAST, 3.0),
            ],
            actions: vec![despawn_object("crate_1"), increment_variable(VAR_CRATES)],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![
                player_enters("crate_2"),
                number_equals(VAR_BEAT, 3.0),
                number_equals(VAR_SETUP_LAST, 3.0),
            ],
            actions: vec![despawn_object("crate_2"), increment_variable(VAR_CRATES)],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![
                player_enters("crate_3"),
                number_equals(VAR_BEAT, 3.0),
                number_equals(VAR_SETUP_LAST, 3.0),
            ],
            actions: vec![despawn_object("crate_3"), increment_variable(VAR_CRATES)],
        },
        // Tally text (1/3, 2/3): complete + re-add rebuilds the panel line in
        // the same frame (no flicker; verified in).
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![number_equals(VAR_BEAT, 3.0), number_equals(VAR_CRATES, 1.0)],
            actions: vec![
                complete_objective(OBJ_B3),
                post_objective(OBJ_B3, "Crates recovered: 1/3."),
            ],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![number_equals(VAR_BEAT, 3.0), number_equals(VAR_CRATES, 2.0)],
            actions: vec![
                complete_objective(OBJ_B3),
                post_objective(OBJ_B3, "Crates recovered: 2/3."),
            ],
        },
        // Beat 3 -> 4: all crates aboard - the targeting computer comes online
        // (the capability beat: until this grant a CTRL hold answered with the
        // deny buzz) and the first radar lesson begins. One gesture: the lock
        // (beat sheet v2). Beacon 3 sits OUTSIDE the SOI, within default beacon
        // lock range of the cluster.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![number_equals(VAR_BEAT, 3.0), number_equals(VAR_CRATES, 3.0)],
            actions: vec![
                set_variable(VAR_BEAT, number(4.0)),
                complete_objective(OBJ_B3),
                story_message(
                    CAPTAIN_HALLORAN,
                    "Targeting computer's warmed up. Hold your radar on it \
                     till the lock sets.",
                ),
                // Beat 4 brings the targeting computer online WITH its lesson
                // (the capability beat): the beacon, the objective, the LOCK
                // grant and the RADAR emphasis all post a beat after the line.
                beat_setup(
                    4.0,
                    INSTRUCTION_GAP,
                    vec![
                        EventActionConfig::SetControllerVerb(SetControllerVerbActionConfig {
                            id: ID_PLAYER.to_string(),
                            verb: FlightVerb::Lock,
                            enabled: true,
                        }),
                        spawn_object(beacon(ID_BEACON_3, "BEACON 3", BEACON_3_POS)),
                        post_objective(OBJ_B4, "Lock onto Beacon 3 - hold [CTRL]."),
                        attach_objective_marker(ID_BEACON_3, "BEACON 3"),
                        show_hint_emphasis("RADAR"),
                    ],
                ),
            ],
        },
        // Beat 4 -> 5: the white lock LANDED (OnTravelLockStart - the lesson
        // ticks the instant the radar rewards it). One gesture: [G].
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTravelLockStart,
            once: true,
            filters: vec![player_enters(ID_BEACON_3), number_equals(VAR_BEAT, 4.0)],
            actions: vec![
                set_variable(VAR_BEAT, number(5.0)),
                complete_objective(OBJ_B4),
                // The RADAR lesson is done the instant the lock lands.
                clear_hint_emphasis("RADAR"),
                story_message(
                    CAPTAIN_HALLORAN,
                    "Now hand her to the computer - it flies the leg while you \
                     watch the belt.",
                ),
                // Beat 5 hands off to the autopilot a beat after the line.
                beat_setup(
                    5.0,
                    INSTRUCTION_GAP,
                    vec![
                        post_objective(OBJ_B5, "Locked. Press [G] to let the computer fly."),
                        show_hint_emphasis("GOTO"),
                    ],
                ),
            ],
        },
        // Beat 5 -> 6: arrival at beacon 3. The waypoint run: beacon 4
        // appears (long leg, signature authored for it) - re-designating
        // and re-pressing [G] teaches that GOTO captures the lock at the
        // press (the re-designation semantics, previously untaught).
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![player_enters(ID_BEACON_3), number_equals(VAR_BEAT, 5.0)],
            actions: vec![
                set_variable(VAR_BEAT, number(6.0)),
                complete_objective(OBJ_B5),
                story_message(
                    PLAYER,
                    "Long leg to the next mark. Re-locking and handing off \
                     again.",
                ),
                // Beat 6 lays the next waypoint a beat after the call.
                beat_setup(
                    6.0,
                    INSTRUCTION_GAP,
                    vec![
                        spawn_object(beacon_with_signature(
                            ID_BEACON_4,
                            "BEACON 4",
                            BEACON_4_POS,
                            Some(BEACON_4_LOCK_SIGNATURE),
                        )),
                        post_objective(OBJ_B6, "New waypoint: Beacon 4. Lock it, press [G] again."),
                        detach_objective_marker(ID_BEACON_3),
                        attach_objective_marker(ID_BEACON_4, "BEACON 4"),
                    ],
                ),
            ],
        },
        // Beat 6 -> 7: arrival at beacon 4, deep in the planetoid's grip.
        // The gravity coast: zero keys, the well does the flying. The ring
        // spawns HERE (not at start), so its OnEnter cannot fire early.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![player_enters(ID_BEACON_4), number_equals(VAR_BEAT, 6.0)],
            actions: vec![
                set_variable(VAR_BEAT, number(7.0)),
                complete_objective(OBJ_B6),
                // The autopilot leg is over; its hint clears now.
                clear_hint_emphasis("GOTO"),
                story_message(
                    CAPTAIN_HALLORAN,
                    "That's the planetoid's pull. Ease off the drive and let \
                     the well carry you.",
                ),
                // Beat 7 opens the coast a beat after the line: the ring spawns
                // HERE (not at start), so its OnEnter cannot fire early.
                // Reveal-then-instruct ("that's the planetoid's pull - ease off
                // the drive"): a mid gap.
                beat_setup(
                    7.0,
                    MID_GAP,
                    vec![
                        coast_ring(),
                        post_objective(OBJ_B7, "Cut the burn and coast in."),
                        detach_objective_marker(ID_BEACON_4),
                        attach_objective_marker(ID_PLANETOID, "PLANETOID"),
                    ],
                ),
            ],
        },
        // Beat 7 -> 8: the drift crossed the coast ring. One gesture: [O]
        // (orbit lifecycle is autopilot state - a position gate is unwinnable
        // because the ORBIT verb rings at max(band, engage radius);
        // playtest finding 5).
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![player_enters(ID_COAST_RING), number_equals(VAR_BEAT, 7.0)],
            actions: vec![
                set_variable(VAR_BEAT, number(8.0)),
                complete_objective(OBJ_B7),
                story_message(
                    CAPTAIN_HALLORAN,
                    "Ride it around - the computer will hold your orbit for \
                     you.",
                ),
                // Beat 8 brings the orbit computer online WITH its lesson a
                // beat after the line (the same capability choreography as GOTO
                // and LOCK): the contextual [O] row lights the moment the text
                // asks.
                beat_setup(
                    8.0,
                    INSTRUCTION_GAP,
                    vec![
                        EventActionConfig::SetControllerVerb(SetControllerVerbActionConfig {
                            id: ID_PLAYER.to_string(),
                            verb: FlightVerb::Orbit,
                            enabled: true,
                        }),
                        post_objective(OBJ_B8, "Press [O] to hold an orbit."),
                    ],
                ),
            ],
        },
        // Stable station-keeping starts the authored hold. Losing stability or
        // ending ORBIT cancels it, so only one continuous five-second hold
        // completes the lesson.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnOrbitStable,
            once: false,
            filters: vec![player_enters(ID_PLANETOID), number_equals(VAR_BEAT, 8.0)],
            actions: vec![EventActionConfig::TimerStart(TimerStartActionConfig {
                key: TIMER_ORBIT_HOLD.to_string(),
                seconds: number(ORBIT_HOLD_SECS),
            })],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnOrbitUnstable,
            once: false,
            filters: vec![player_enters(ID_PLANETOID), number_equals(VAR_BEAT, 8.0)],
            actions: vec![EventActionConfig::TimerCancel(TimerCancelActionConfig {
                key: TIMER_ORBIT_HOLD.to_string(),
            })],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnOrbitEnd,
            once: false,
            filters: vec![player_enters(ID_PLANETOID), number_equals(VAR_BEAT, 8.0)],
            actions: vec![EventActionConfig::TimerCancel(TimerCancelActionConfig {
                key: TIMER_ORBIT_HOLD.to_string(),
            })],
        },
        // Beat 8 -> 9: orbit held. Break away (teaches [Z] with a real
        // completion: leaving the coast ring). The derelict spawns now,
        // back by the salvage field - outside the SOI, so it stays put.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: true,
            filters: vec![
                EventFilterConfig::Timer(TimerFilterConfig {
                    key: TIMER_ORBIT_HOLD.to_string(),
                }),
                number_equals(VAR_BEAT, 8.0),
            ],
            actions: vec![
                set_variable(VAR_BEAT, number(9.0)),
                complete_objective(OBJ_B8),
                // The derelict spawns and the marker hands off at the
                // TRANSITION, not in beat_setup: [Z] (STOP) is granted from the
                // start, so a fast break-away could exit the coast ring (beat 9
                // -> 10) before the delayed setup runs. If the hulk did not yet
                // exist beat 10 would soft-lock with nothing to paint, and a
                // skipped setup would strand the marker on the planetoid. It
                // spawns back by the salvage field, outside the SOI. Only the
                // break-away objective text waits for the line.
                spawn_object(derelict()),
                detach_objective_marker(ID_PLANETOID),
                attach_objective_marker(ID_DERELICT, "DERELICT"),
                story_message(
                    CAPTAIN_HALLORAN,
                    "Good. Break the orbit and burn clear when you're ready.",
                ),
                // Beat 9's break-away objective posts a beat after the line
                // (the hulk and its marker are already up, above).
                beat_setup(
                    9.0,
                    INSTRUCTION_GAP,
                    vec![post_objective(
                        OBJ_B9,
                        "Break away - press [Z] and burn clear.",
                    )],
                ),
            ],
        },
        // Beat 9 -> 10: left the ring. The live-fire rehearsal begins: the
        // combat lock in calm - this is where the viewfinder inset, the
        // fine-lock and guided torpedoes become discoverable.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnExit,
            once: true,
            filters: vec![player_enters(ID_COAST_RING), number_equals(VAR_BEAT, 9.0)],
            actions: vec![
                set_variable(VAR_BEAT, number(10.0)),
                complete_objective(OBJ_B9),
                story_message(
                    CAPTAIN_HALLORAN,
                    "Dead hulk off your old salvage field. Blood the guns on \
                     it - lock it up and watch your viewfinder.",
                ),
                // Beat 10 calls the paint a beat after the line: the objective
                // posts and the RADAR hint lights for the combat lock.
                // Reveal-then-instruct ("dead hulk off your old field - blood
                // the guns on it"): a mid gap lets the new target register
                // before the paint task.
                beat_setup(
                    10.0,
                    MID_GAP,
                    vec![
                        post_objective(OBJ_B10, "Paint the derelict - hold [RMB] and [CTRL]."),
                        show_hint_emphasis("RADAR"),
                    ],
                ),
            ],
        },
        // Beat 10 -> 11: the RED lock landed on the hulk. One gesture:
        // fire.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnCombatLockStart,
            once: true,
            filters: vec![player_enters(ID_DERELICT), number_equals(VAR_BEAT, 10.0)],
            actions: vec![
                set_variable(VAR_BEAT, number(11.0)),
                complete_objective(OBJ_B10),
                post_objective(OBJ_B11, "Locked on. Open fire - [LMB]."),
                clear_hint_emphasis("RADAR"),
            ],
        },
        // The hulk is dust -> the fight, from ANY rehearsal beat (lt 12,
        // not eq 11): the derelict is destructible the moment it spawns
        // (beat 9), and a player who shoots it before locking it must SKIP
        // ahead, not soft-lock on a consumed one-shot (playtest
        // 2026-07-13: got stuck exactly there). Completing objectives that
        // never posted is a no-op removal; clearing an unset emphasis
        // likewise. Every gesture was rehearsed (or skipped by
        // demonstration), so the fight is the exam: ONE line.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![entity(ID_DERELICT), number_less_than(VAR_BEAT, 12.0)],
            actions: vec![
                set_variable(VAR_BEAT, number(12.0)),
                complete_objective(OBJ_B9),
                complete_objective(OBJ_B10),
                complete_objective(OBJ_B11),
                clear_hint_emphasis("RADAR"),
                spawn_object(pirate_ship()),
                // The one fight announces itself (beat-sheet telegraph): a
                // warning line, a spawn back at the debris field, and the
                // scavenger's own engage_delay grace before its guns come up.
                // Pacing pass: the objective posts a beat after this warning,
                // not the same frame.
                story_message(
                    CAPTAIN_HALLORAN,
                    "Contact - scavenger picking through your debris field. \
                     Drive it off.",
                ),
                // Threat reveal: the scavenger telegraph is a beat to absorb -
                // full gap. The scavenger's own engage_delay covers it.
                start_timer(TIMER_SCAV_GATE, REVEAL_GAP),
                // Defensive detach (the destroyed hulk takes its marker
                // with it; do not depend on despawn timing), then the
                // marker jumps to the intruder (attach after its spawn).
                detach_objective_marker(ID_DERELICT),
                attach_objective_marker(ID_PIRATE, "SCAVENGER"),
            ],
        },
        // The scavenger objective, a beat after the warning line. Still gated
        // on beat 12: a fast kill (the win sets beat 13) must not post a stale
        // objective under the Victory overlay.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: true,
            filters: vec![timer(TIMER_SCAV_GATE), number_equals(VAR_BEAT, 12.0)],
            actions: vec![post_objective(OBJ_B12, "Drive off the scavenger.")],
        },
        // Beat 12 end: pirate destroyed - the chapter is won. The Victory
        // overlay chains into Broadside (chapter two) via the lingering switch:
        // Continue (or Enter) answers the call, Main Menu keeps the win. The
        // stand-down lesson line stays in the objective under the overlay -
        // input still works behind it, and the gesture recurs naturally in the
        // next chapter's fights.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![entity(ID_PIRATE), number_equals(VAR_BEAT, 12.0)],
            actions: pacing::open_outro(
                VAR_BEAT,
                BEAT_OUTRO,
                outro(),
                vec![
                    complete_objective(OBJ_B12),
                    detach_objective_marker(ID_PIRATE),
                    story_message(CAPTAIN_HALLORAN, "The scavenger is scrap. Good shooting."),
                ],
            ),
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnNeutralized,
            once: true,
            filters: vec![entity(ID_PIRATE), number_equals(VAR_BEAT, 12.0)],
            actions: pacing::open_outro(
                VAR_BEAT,
                BEAT_OUTRO,
                outro(),
                vec![
                    complete_objective(OBJ_B12),
                    detach_objective_marker(ID_PIRATE),
                    story_message(
                        CAPTAIN_HALLORAN,
                        "The scavenger drifts dead - guns cold, engines dark. \
                     Good shooting.",
                    ),
                ],
            ),
        },
        // Player death: the Defeat overlay offers Retry (the lingering restart)
        // and Main Menu, so a death never silently queues a restart the player
        // has to know to press Enter for.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![entity(ID_PLAYER), number_less_than(VAR_BEAT, BEAT_OUTRO)],
            actions: vec![
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "Your ship broke apart in the belt.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: SHAKEDOWN_SCENARIO_ID.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnNeutralized,
            once: true,
            filters: vec![entity(ID_PLAYER), number_less_than(VAR_BEAT, BEAT_OUTRO)],
            actions: vec![
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "Nothing left to fight with - you drift derelict in the belt.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: SHAKEDOWN_SCENARIO_ID.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
        // The between-beat comms lines now play AT each transition (owner
        // playtest): the line lands as the previous objective completes, and
        // the next objective posts a beat LATER via beat_setup, once the line
        // has finished - never the same frame. The combat exam (beats 11-12)
        // stays tight by design (the fight is the exam) and announces itself
        // with the scavenger telegraph above.
    ];

    ScenarioConfig {
        description: "First flight: beacons, salvage, orbit - and one scavenger.".to_string(),
        // The main-story entry point: listed in the Scenarios picker.
        // Generated placeholder art (scripts/gen-scenario-thumbnails.py);
        // real art overwrites this same path with no code change.
        thumbnail: Some(AssetRef::from("self://thumbnails/shakedown_run.png")),
        // Chapter one of the Nova Protocol campaign; membership + order now
        // live in the `nova_protocol` campaign mapping.
        watches: vec![scenario_elapsed_watch(SCENARIO_ELAPSED_VAR)],
        events,
        ..ScenarioConfig::new(
            SHAKEDOWN_SCENARIO_ID.to_string(),
            "Shakedown Run".to_string(),
            cubemap,
        )
    }
}

#[cfg(test)]
mod tests;
