//! "Shakedown Run" - the starter scenario New Game drops the player into
//! (task 20260711-180506, beat sheet in
//! docs/spikes/20260712-092926-starter-scenario.md).
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

use bevy::{platform::collections::HashMap, prelude::*};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

/// The scenario id, shared with nova_menu's New Game entry.
pub const SHAKEDOWN_SCENARIO_ID: &str = "shakedown_run";

// Layout. Distances are deliberately short (a few hundred units between
// objectives): "close enough to see" is the cheapest objective marker.
// The planetoid numbers are authored against the RUNTIME geometry, not the
// nominal radius (the authored-vs-derived lesson, 20260711-180455): a
// nominal-20u asteroid's noise mesh reaches ASTEROID_GEOMETRIC_FACTOR_MIN
// ..MAX times its nominal radius (3.5-6.0, pinned by nova_scenario's seed
// sweep; observed [3.70, 5.64] over 256 seeds), so the geometric body
// radius runs 70-120u, the SOI (8x) 560-960u, and the ORBIT ring
// (1.5 * (body_radius + 1)) 106-182u. The config-shape tests below pin
// the layout against the WHOLE range - review R2.2 caught the first cut
// assuming a single observed seed band (4.0-4.55), under which a
// high-factor seed parked the orbit ring OUTSIDE the old 160u gate and
// soft-locked beat 4.
const PLAYER_SPAWN: Vec3 = Vec3::ZERO;
/// Beat 1: dead ahead of the spawn heading (-Z).
const BEACON_1_POS: Vec3 = Vec3::new(0.0, 0.0, -350.0);
/// Beat 2: ~120 degrees off the beacon-1 boresight, so freelook (or a
/// deliberate turn) is genuinely how you find it.
const BEACON_2_POS: Vec3 = Vec3::new(260.0, 20.0, -200.0);
/// Beat 3: a loose debris cluster past beacon 2 - pushed out so no crate
/// sensor overlaps the (now standoff-sized) beacon trigger.
const DEBRIS_CENTER: Vec3 = Vec3::new(350.0, 20.0, -160.0);
const CRATE_POSITIONS: [Vec3; 3] = [
    Vec3::new(340.0, 15.0, -175.0),
    Vec3::new(360.0, 30.0, -150.0),
    Vec3::new(375.0, 10.0, -165.0),
];
/// The stage dressing and late-run destination: a planetoid with a real
/// gravity well, far enough that even the WORST-seed SOI (960u) falls
/// short of the debris cluster - playtest round 2 finding 1: at the old
/// ~650u separation the player was fighting gravity while weaving
/// crates. The SOI edge is crossed on the waypoint leg on every seed.
const PLANETOID_POS: Vec3 = Vec3::new(1240.0, -105.0, -700.0);
const PLANETOID_NOMINAL_RADIUS: f32 = 20.0;
/// The FIRST radar-lock target (beat sheet v2, spike 20260713-140742):
/// a comfortable GOTO leg from the debris cluster, OUTSIDE even the
/// worst-seed SOI so the hands-off ride is gravity-free, and inside the
/// default beacon lock range (600u) from the cluster.
const BEACON_3_POS: Vec3 = Vec3::new(600.0, 90.0, 120.0);
/// The waypoint-run target: the old beacon-3 spot scaled out to 300u from
/// the planetoid - inside the smallest-seed SOI (so the ORBIT hint lights
/// on arrival) with its trigger clear of the coast ring (the
/// already-inside-when-armed trap; pinned below). The beacon-3 -> beacon-4
/// leg (~800u) is beyond the DEFAULT beacon lock range, so beacon 4
/// authors the signature its leg needs (pinned below).
const BEACON_4_POS: Vec3 = Vec3::new(985.0, -69.0, -545.0);
const BEACON_4_LOCK_SIGNATURE: f32 = 30.0;
/// The gravity-coast ring: a planetoid-centered invisible trigger sphere.
/// Entering it (drifting in from the beacon-4 park) is the coast beat;
/// LEAVING it after the held orbit is the break-away beat. Outside the
/// widest orbit ring, inside the smallest SOI, clear of beacon 4's
/// trigger - all pinned below.
const COAST_RING_RADIUS: f32 = 210.0;
/// The live-fire rehearsal target: an inert hulk drifting near the old
/// salvage field, OUTSIDE the worst-seed SOI (a dynamic body inside it
/// would fall into the planetoid). Its combat-lock range is short
/// (signature = radius, ~75u) - the marker walks the player in.
const DERELICT_POS: Vec3 = Vec3::new(300.0, -40.0, 40.0);
const DERELICT_RADIUS: f32 = 2.5;
const DERELICT_HEALTH: f32 = 150.0;
/// The pirate spawns back at the debris cluster once the rehearsal is
/// done, and patrols it.
const PIRATE_SPAWN: Vec3 = Vec3::new(380.0, 40.0, -100.0);
const PIRATE_PATROL: [Vec3; 3] = [
    Vec3::new(300.0, 20.0, -170.0),
    Vec3::new(360.0, 25.0, -110.0),
    Vec3::new(330.0, 60.0, -140.0),
];
/// Beacon trigger radius. MUST contain the GOTO park point: the autopilot
/// stops arrival_standoff (50u, FlightSettings) from an unsized target,
/// and a trigger smaller than that leaves the ship parked 10u OUTSIDE its
/// own objective (playtest 2026-07-12 finding 2). Pinned by a config test
/// against FlightSettings::default().
const BEACON_AREA_RADIUS: f32 = 70.0;
/// Crate pickup radius: tight enough to require flying AT the crate.
const CRATE_AREA_RADIUS: f32 = 8.0;

const BEACON_COLOR: Color = Color::srgb(0.3, 0.9, 1.0);

/// The scavenger's territorial tether (world units around its patrol
/// centroid): combat breaks off beyond it, keeping the beat-5 fight at
/// the debris field (playtest round 3 finding 3).
const PIRATE_LEASH_RADIUS: f32 = 150.0;

/// Soft manual-speed cap (u/s) on the starter ship: at 25 u/s a 350u leg
/// still takes a quarter minute and a missed brake does not send a new
/// pilot sailing out of the play area (playtest 2026-07-12 finding 1).
const PLAYER_SPEED_CAP: f32 = 25.0;

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
const VAR_CRATES: &str = "crates_recovered";
const VAR_TALLY_SHOWN: &str = "tally_shown";

// Expression / action shorthands - the raw node constructors are too
// verbose to keep a 14-handler script readable.

fn num(value: f64) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(
        VariableFactorNode::new_literal(VariableLiteral::Number(value)),
    ))
}

fn var(name: &str) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(VariableFactorNode::new_name(
        name.to_string(),
    )))
}

fn set(key: &str, expression: VariableExpressionNode) -> EventActionConfig {
    EventActionConfig::VariableSet(VariableSetActionConfig {
        key: key.to_string(),
        expression,
    })
}

fn add_one(key: &str) -> EventActionConfig {
    set(
        key,
        VariableExpressionNode::new_add(
            VariableTermNode::new_factor(VariableFactorNode::new_name(key.to_string())),
            num(1.0),
        ),
    )
}

fn eq_num(name: &str, value: f64) -> EventFilterConfig {
    EventFilterConfig::Expression(ExpressionFilterConfig(VariableConditionNode::new_equals(
        var(name),
        num(value),
    )))
}

fn lt_num(name: &str, value: f64) -> EventFilterConfig {
    EventFilterConfig::Expression(ExpressionFilterConfig(
        VariableConditionNode::new_less_than(var(name), num(value)),
    ))
}

/// OnEnter of `area` by the player ship.
fn player_enters(area: &str) -> EventFilterConfig {
    EventFilterConfig::Entity(EntityFilterConfig {
        id: Some(area.to_string()),
        other_id: Some(ID_PLAYER.to_string()),
        ..default()
    })
}

fn destroyed(id: &str) -> EventFilterConfig {
    EventFilterConfig::Entity(EntityFilterConfig {
        id: Some(id.to_string()),
        ..default()
    })
}

fn objective(id: &str, message: &str) -> EventActionConfig {
    EventActionConfig::Objective(ObjectiveActionConfig::new(id, message))
}

fn complete(id: &str) -> EventActionConfig {
    EventActionConfig::ObjectiveComplete(ObjectiveCompleteActionConfig { id: id.to_string() })
}

fn despawn(id: &str) -> EventActionConfig {
    EventActionConfig::DespawnScenarioObject(DespawnScenarioObjectActionConfig::new(id))
}

/// Attach the gold objective marker to a scenario entity (task
/// 20260712-093831). Ordered AFTER the target's spawn action when both sit
/// in one handler - actions queue in list order.
fn mark(target_id: &str, label: &str) -> EventActionConfig {
    EventActionConfig::ObjectiveMarkerAttach(ObjectiveMarkerAttachActionConfig::new(
        target_id, label,
    ))
}

fn unmark(target_id: &str) -> EventActionConfig {
    EventActionConfig::ObjectiveMarkerDetach(ObjectiveMarkerDetachActionConfig::new(target_id))
}

fn emphasize(verb: &str) -> EventActionConfig {
    EventActionConfig::HintEmphasisSet(HintEmphasisSetActionConfig::new(verb))
}

fn deemphasize(verb: &str) -> EventActionConfig {
    EventActionConfig::HintEmphasisClear(HintEmphasisClearActionConfig::new(verb))
}

fn spawn(object: ScenarioObjectConfig) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(object)
}

fn beacon(id: &str, label: &str, position: Vec3) -> ScenarioObjectConfig {
    beacon_with_signature(id, label, position, None)
}

/// A beacon whose radar signature is authored for a longer-than-default
/// GOTO leg (beacon 4's waypoint run; the leg-vs-range pin lives in the
/// geometry test).
fn beacon_with_signature(
    id: &str,
    label: &str,
    position: Vec3,
    lock_signature: Option<f32>,
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
            radius: 2.0,
            color: BEACON_COLOR,
            area_radius: Some(BEACON_AREA_RADIUS),
            lock_signature,
        }),
    }
}

/// The live-fire rehearsal target: an inert asteroid-kind hulk - zero new
/// spawn paths (asteroids lock, zoom in the viewfinder, and die); the
/// inert-SHIP silhouette is recorded future polish (spike 20260713-140742).
fn derelict(game_assets: &crate::GameAssets) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_DERELICT.to_string(),
            name: "Derelict Hulk".to_string(),
            position: DERELICT_POS,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: DERELICT_RADIUS,
            texture: game_assets.asteroid_texture.clone(),
            health: DERELICT_HEALTH,
            surface_gravity: None,
            invulnerable: false,
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

fn crate_object(index: usize, position: Vec3) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: format!("crate_{}", index),
            name: format!("Supply Crate {}", index),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::SalvageCrate(SalvageCrateConfig {
            size: 1.5,
            area_radius: CRATE_AREA_RADIUS,
        }),
    }
}

fn section(
    sections: &GameSections,
    id: &str,
    section_id: &str,
    position: Vec3,
) -> SpaceshipSectionConfig {
    SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation: Quat::IDENTITY,
        config: sections.get_section(section_id).unwrap().clone(),
    }
}

/// The shakedown ship: deliberately minimal - controller, one hull, one
/// thruster, ONE turret (no torpedo bay). One of everything keeps the
/// component-cycle lesson trivially readable.
fn player_ship(sections: &GameSections) -> ScenarioObjectConfig {
    let turret = SpaceshipSectionConfig {
        id: "turret".to_string(),
        // Directly behind the controller: the minimal ships have no
        // hull_back, and a turret at -2 left a one-section hole in the
        // silhouette (playtest 2026-07-12 finding 7).
        position: Vec3::new(0.0, 0.0, -1.0),
        rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        config: sections
            .get_section("better_turret_section")
            .unwrap()
            .clone(),
    };
    // GOTO and LOCK start WITHHELD on the player's controller: the pilot has
    // not yet flown a controlled leg, and the targeting computer is not
    // online yet. The Beat 1 -> 2 handler's SetControllerVerb grants GOTO
    // after the first objective (spike
    // docs/spikes/20260712-143551-controller-provided-verb-flags.md); the
    // Beat 3 -> 4 handler grants LOCK - the radar-era capability beat (task
    // 20260713-090653): until then a CTRL hold answers with the deny buzz +
    // flash, diegetically "no targeting computer". Authored in config, not
    // an OnStart action, so both are off from the instant the controller
    // section is built - no spawn-vs-action ordering window - and the shared
    // basic_controller_section catalog entry (the pirate reuses it) stays
    // untouched because we clone-and-override here.
    let controller = {
        let mut config = sections
            .get_section("basic_controller_section")
            .unwrap()
            .clone();
        if let SectionKind::Controller(ref mut controller_config) = config.kind {
            controller_config.verbs.goto = false;
            controller_config.verbs.lock = false;
        }
        SpaceshipSectionConfig {
            id: "controller".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            config,
        }
    };
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_PLAYER.to_string(),
            name: "Player Spaceship".to_string(),
            position: PLAYER_SPAWN,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: HashMap::from([(
                    "turret".to_string(),
                    vec![
                        MouseButton::Left.into(),
                        GamepadButton::RightTrigger2.into(),
                    ],
                )]),

                speed_cap: Some(PLAYER_SPEED_CAP),
                // The first/New Game scenario: unlimited ammo so the intro is
                // not gated on running dry before a reload mechanic exists
                // (task 20260712-140250).
                infinite_ammo: true,
            }),
            sections: vec![
                controller,
                section(sections, "hull_front", "reinforced_hull_section", Vec3::Z),
                section(
                    sections,
                    "thruster",
                    "basic_thruster_section",
                    Vec3::Z * 2.0,
                ),
                turret,
            ],
        }),
    }
}

/// The scavenger: the player ship's silhouette in scavenger grade - light
/// hull, light turret - passive (patrolling the debris cluster) until the
/// player closes inside AI engage range or shoots first.
fn pirate_ship(sections: &GameSections) -> ScenarioObjectConfig {
    let turret = SpaceshipSectionConfig {
        id: "turret".to_string(),
        // Directly behind the controller: the minimal ships have no
        // hull_back, and a turret at -2 left a one-section hole in the
        // silhouette (playtest 2026-07-12 finding 7).
        position: Vec3::new(0.0, 0.0, -1.0),
        rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        config: sections
            .get_section("light_turret_section")
            .unwrap()
            .clone(),
    };
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_PIRATE.to_string(),
            name: "Scavenger".to_string(),
            position: PIRATE_SPAWN,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig {
                patrol: PIRATE_PATROL.to_vec(),
                // Territorial: the scavenger fights AT the debris field
                // and breaks off if the duel drifts away (playtest round
                // 3 finding 3) - the leash comfortably covers the patrol
                // loop and the crate scatter.
                leash: Some(PIRATE_LEASH_RADIUS),
                ..Default::default()
            }),
            sections: vec![
                section(
                    sections,
                    "controller",
                    "basic_controller_section",
                    Vec3::ZERO,
                ),
                section(sections, "hull_front", "light_hull_section", Vec3::Z),
                section(
                    sections,
                    "thruster",
                    "basic_thruster_section",
                    Vec3::Z * 2.0,
                ),
                turret,
            ],
        }),
    }
}

pub fn shakedown_run(game_assets: &crate::GameAssets, sections: &GameSections) -> ScenarioConfig {
    // The debris cluster: fixed offsets, not rng - the layout is content,
    // and determinism keeps the config-shape tests honest.
    const ROCK_OFFSETS: [Vec3; 9] = [
        Vec3::new(-35.0, 5.0, 20.0),
        Vec3::new(-15.0, -10.0, -25.0),
        Vec3::new(10.0, 25.0, 15.0),
        Vec3::new(30.0, -5.0, -20.0),
        Vec3::new(45.0, 15.0, 10.0),
        Vec3::new(-25.0, 30.0, -10.0),
        Vec3::new(5.0, -20.0, 30.0),
        Vec3::new(25.0, 40.0, -35.0),
        Vec3::new(-45.0, -15.0, -5.0),
    ];
    const ROCK_RADII: [f32; 9] = [2.5, 1.5, 3.0, 2.0, 1.0, 2.5, 1.5, 2.0, 3.0];

    let mut start_spawns: Vec<ScenarioObjectConfig> = Vec::new();
    start_spawns.push(player_ship(sections));
    start_spawns.push(beacon(ID_BEACON_1, "BEACON 1", BEACON_1_POS));
    start_spawns.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_PLANETOID.to_string(),
            name: "Planetoid".to_string(),
            position: PLANETOID_POS,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: PLANETOID_NOMINAL_RADIUS,
            texture: game_assets.asteroid_texture.clone(),
            health: 2000.0,
            surface_gravity: Some(6.0),
            invulnerable: true,
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
                radius,
                texture: game_assets.asteroid_texture.clone(),
                health: 100.0,
                surface_gravity: None,
                invulnerable: false,
            }),
        });
    }
    for (i, position) in CRATE_POSITIONS.iter().enumerate() {
        start_spawns.push(crate_object(i + 1, *position));
    }

    let events = vec![
        // Beat 1 setup: the world, the variables, the first objective.
        // Beacons 2/3 and the pirate spawn LAZILY with their beats, so a
        // new chip appearing on the HUD always means "this is next".
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: start_spawns
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .chain([
                    set(VAR_BEAT, num(1.0)),
                    set(VAR_CRATES, num(0.0)),
                    set(VAR_TALLY_SHOWN, num(0.0)),
                    objective(
                        OBJ_B1,
                        "Hold [W] and burn for BEACON 1. Tap [X] to stop.",
                    ),
                    // The gold marker rides the current leg's target
                    // (conveyance layer 2, task 20260712-093831); its
                    // beacon chip yields while marked, so each beacon
                    // shows exactly one chip.
                    mark(ID_BEACON_1, "BEACON 1"),
                ])
                .collect(),
        },
        // Beat 1 -> 2: reach beacon 1; beacon 2 appears off the beam.
        ScenarioEventConfig {
            name: EventConfig::OnEnter,
            filters: vec![player_enters(ID_BEACON_1), eq_num(VAR_BEAT, 1.0)],
            actions: vec![
                set(VAR_BEAT, num(2.0)),
                complete(OBJ_B1),
                // The training governor releases once the pilot has proven
                // a controlled leg (playtest round 2 finding 3).
                EventActionConfig::SetSpeedCap(SetSpeedCapActionConfig {
                    id: ID_PLAYER.to_string(),
                    cap: None,
                }),
                // GOTO unlocks with the first objective: the ship starts with
                // it withheld (player_ship's controller config) and clearing
                // beat 1 grants it (spike
                // docs/spikes/20260712-143551-controller-provided-verb-flags.md).
                EventActionConfig::SetControllerVerb(SetControllerVerbActionConfig {
                    id: ID_PLAYER.to_string(),
                    verb: FlightVerb::Goto,
                    enabled: true,
                }),
                spawn(beacon(ID_BEACON_2, "BEACON 2", BEACON_2_POS)),
                objective(
                    OBJ_B2,
                    "Governor released. BEACON 2 is somewhere off your beam. Hold [Alt] to look around and find it.",
                ),
                // Marker hand-off: attach runs after the spawn above
                // (action list order), so the fresh beacon is findable.
                unmark(ID_BEACON_1),
                mark(ID_BEACON_2, "BEACON 2"),
            ],
        },
        // Beat 2 -> 3: reach beacon 2; the debris cluster is right there.
        ScenarioEventConfig {
            name: EventConfig::OnEnter,
            filters: vec![player_enters(ID_BEACON_2), eq_num(VAR_BEAT, 2.0)],
            actions: vec![
                set(VAR_BEAT, num(3.0)),
                complete(OBJ_B2),
                objective(
                    OBJ_B3,
                    "Recover 3 supply crates from the debris cluster.",
                ),
                // All three crates carry the marker at once; each dies
                // with its crate, so the survivors answer "which is left".
                unmark(ID_BEACON_2),
                mark("crate_1", "SALVAGE"),
                mark("crate_2", "SALVAGE"),
                mark("crate_3", "SALVAGE"),
            ],
        },
        // Beat 3 pickups: one handler per crate (the despawn action needs
        // the concrete id). Counting is a variable; the tally text and the
        // beat advance are OnUpdate handlers below, so nothing depends on
        // handler order within the pickup event.
        ScenarioEventConfig {
            name: EventConfig::OnEnter,
            filters: vec![player_enters("crate_1"), eq_num(VAR_BEAT, 3.0)],
            actions: vec![despawn("crate_1"), add_one(VAR_CRATES)],
        },
        ScenarioEventConfig {
            name: EventConfig::OnEnter,
            filters: vec![player_enters("crate_2"), eq_num(VAR_BEAT, 3.0)],
            actions: vec![despawn("crate_2"), add_one(VAR_CRATES)],
        },
        ScenarioEventConfig {
            name: EventConfig::OnEnter,
            filters: vec![player_enters("crate_3"), eq_num(VAR_BEAT, 3.0)],
            actions: vec![despawn("crate_3"), add_one(VAR_CRATES)],
        },
        // Tally text (1/3, 2/3): complete + re-add rebuilds the panel line
        // in the same frame (no flicker; verified in 20260712-093044).
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![
                eq_num(VAR_BEAT, 3.0),
                eq_num(VAR_CRATES, 1.0),
                lt_num(VAR_TALLY_SHOWN, 1.0),
            ],
            actions: vec![
                set(VAR_TALLY_SHOWN, num(1.0)),
                complete(OBJ_B3),
                objective(OBJ_B3, "Supply crates recovered: 1/3."),
            ],
        },
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![
                eq_num(VAR_BEAT, 3.0),
                eq_num(VAR_CRATES, 2.0),
                lt_num(VAR_TALLY_SHOWN, 2.0),
            ],
            actions: vec![
                set(VAR_TALLY_SHOWN, num(2.0)),
                complete(OBJ_B3),
                objective(OBJ_B3, "Supply crates recovered: 2/3."),
            ],
        },
        // Beat 3 -> 4: all crates aboard - the targeting computer comes
        // online (the capability beat, task 20260713-090653: until this
        // grant a CTRL hold answered with the deny buzz) and the first
        // radar lesson begins. One gesture: the lock (beat sheet v2, spike
        // 20260713-140742). Beacon 3 sits OUTSIDE the SOI, within default
        // beacon lock range of the cluster.
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![eq_num(VAR_BEAT, 3.0), eq_num(VAR_CRATES, 3.0)],
            actions: vec![
                set(VAR_BEAT, num(4.0)),
                complete(OBJ_B3),
                EventActionConfig::SetControllerVerb(SetControllerVerbActionConfig {
                    id: ID_PLAYER.to_string(),
                    verb: FlightVerb::Lock,
                    enabled: true,
                }),
                spawn(beacon(ID_BEACON_3, "BEACON 3", BEACON_3_POS)),
                objective(
                    OBJ_B4,
                    "Targeting computer online. Hold [CTRL] on BEACON 3 until the lock sticks.",
                ),
                mark(ID_BEACON_3, "BEACON 3"),
                emphasize("RADAR"),
            ],
        },
        // Beat 4 -> 5: the white lock LANDED (OnTravelLock - the lesson
        // ticks the instant the radar rewards it). One gesture: [G].
        ScenarioEventConfig {
            name: EventConfig::OnTravelLock,
            filters: vec![player_enters(ID_BEACON_3), eq_num(VAR_BEAT, 4.0)],
            actions: vec![
                set(VAR_BEAT, num(5.0)),
                complete(OBJ_B4),
                objective(OBJ_B5, "Locked. Press [G] - let the computer fly."),
                deemphasize("RADAR"),
                emphasize("GOTO"),
            ],
        },
        // Beat 5 -> 6: arrival at beacon 3. The waypoint run: beacon 4
        // appears (long leg, signature authored for it) - re-designating
        // and re-pressing [G] teaches that GOTO captures the lock at the
        // press (the re-designation semantics, previously untaught).
        ScenarioEventConfig {
            name: EventConfig::OnEnter,
            filters: vec![player_enters(ID_BEACON_3), eq_num(VAR_BEAT, 5.0)],
            actions: vec![
                set(VAR_BEAT, num(6.0)),
                complete(OBJ_B5),
                spawn(beacon_with_signature(
                    ID_BEACON_4,
                    "BEACON 4",
                    BEACON_4_POS,
                    Some(BEACON_4_LOCK_SIGNATURE),
                )),
                objective(OBJ_B6, "New waypoint: BEACON 4. Lock it, press [G] again."),
                unmark(ID_BEACON_3),
                mark(ID_BEACON_4, "BEACON 4"),
            ],
        },
        // Beat 6 -> 7: arrival at beacon 4, deep in the planetoid's grip.
        // The gravity coast: zero keys, the well does the flying. The ring
        // spawns HERE (not at start), so its OnEnter cannot fire early.
        ScenarioEventConfig {
            name: EventConfig::OnEnter,
            filters: vec![player_enters(ID_BEACON_4), eq_num(VAR_BEAT, 6.0)],
            actions: vec![
                set(VAR_BEAT, num(7.0)),
                complete(OBJ_B6),
                coast_ring(),
                objective(
                    OBJ_B7,
                    "You are in the planetoid's pull. Cut the burn and coast.",
                ),
                unmark(ID_BEACON_4),
                mark(ID_PLANETOID, "PLANETOID"),
                deemphasize("GOTO"),
            ],
        },
        // Beat 7 -> 8: the drift crossed the coast ring. One gesture: [O]
        // (OnOrbit is autopilot state - a position gate is unwinnable
        // because the ORBIT verb rings at max(band, engage radius);
        // playtest finding 5).
        ScenarioEventConfig {
            name: EventConfig::OnEnter,
            filters: vec![player_enters(ID_COAST_RING), eq_num(VAR_BEAT, 7.0)],
            actions: vec![
                set(VAR_BEAT, num(8.0)),
                complete(OBJ_B7),
                objective(OBJ_B8, "Press [O] and hold the orbit."),
            ],
        },
        // Beat 8 -> 9: orbit held. Break away (teaches [Z] with a real
        // completion: leaving the coast ring). The derelict spawns now,
        // back by the salvage field - outside the SOI, so it stays put.
        ScenarioEventConfig {
            name: EventConfig::OnOrbit,
            filters: vec![player_enters(ID_PLANETOID), eq_num(VAR_BEAT, 8.0)],
            actions: vec![
                set(VAR_BEAT, num(9.0)),
                complete(OBJ_B8),
                spawn(derelict(game_assets)),
                objective(OBJ_B9, "Orbit held. Press [Z] to break away and burn clear."),
                unmark(ID_PLANETOID),
                mark(ID_DERELICT, "DERELICT"),
            ],
        },
        // Beat 9 -> 10: left the ring. The live-fire rehearsal begins: the
        // combat lock in calm - this is where the viewfinder inset, the
        // fine-lock and guided torpedoes become discoverable.
        ScenarioEventConfig {
            name: EventConfig::OnExit,
            filters: vec![player_enters(ID_COAST_RING), eq_num(VAR_BEAT, 9.0)],
            actions: vec![
                set(VAR_BEAT, num(10.0)),
                complete(OBJ_B9),
                objective(
                    OBJ_B10,
                    "A derelict hulk drifts by your old salvage field. Hold [RMB], keep [CTRL] on it - watch the viewfinder.",
                ),
                emphasize("RADAR"),
            ],
        },
        // Beat 10 -> 11: the RED lock landed on the hulk. One gesture:
        // fire.
        ScenarioEventConfig {
            name: EventConfig::OnCombatLock,
            filters: vec![player_enters(ID_DERELICT), eq_num(VAR_BEAT, 10.0)],
            actions: vec![
                set(VAR_BEAT, num(11.0)),
                complete(OBJ_B10),
                objective(OBJ_B11, "Locked on. Open fire - [LMB]."),
                deemphasize("RADAR"),
            ],
        },
        // Beat 11 -> 12: the hulk is dust. NOW the scavenger slips into
        // the debris field - every gesture was rehearsed, so the fight is
        // the exam, not the lesson: ONE line.
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_DERELICT), eq_num(VAR_BEAT, 11.0)],
            actions: vec![
                set(VAR_BEAT, num(12.0)),
                complete(OBJ_B11),
                spawn(pirate_ship(sections)),
                objective(
                    OBJ_B12,
                    "A scavenger is picking through your debris field. Drive it off.",
                ),
                // Defensive detach (the destroyed hulk takes its marker
                // with it; do not depend on despawn timing), then the
                // marker jumps to the intruder (attach after its spawn).
                unmark(ID_DERELICT),
                mark(ID_PIRATE, "SCAVENGER"),
            ],
        },
        // Beat 12 end: pirate destroyed.
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_PIRATE), eq_num(VAR_BEAT, 12.0)],
            actions: vec![
                set(VAR_BEAT, num(13.0)),
                complete(OBJ_B12),
                // The stand-down closes the loop: the tap-clear pops the
                // unlatch ghosts and the safety click answers - the last
                // radar gesture the tutorial has not shown.
                objective(
                    OBJ_DONE,
                    "Shakedown complete. Tap [CTRL] to stand down your locks - the belt is yours.",
                ),
                unmark(ID_PIRATE),
            ],
        },
        // Player death: back to the top (Enter confirms - linger).
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_PLAYER)],
            actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: SHAKEDOWN_SCENARIO_ID.to_string(),
                linger: true,
            })],
        },
    ];

    ScenarioConfig {
        id: SHAKEDOWN_SCENARIO_ID.to_string(),
        name: "Shakedown Run".to_string(),
        description: "First flight: beacons, salvage, orbit - and one scavenger.".to_string(),
        cubemap: game_assets.cubemap.clone(),
        events,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario() -> ScenarioConfig {
        let assets = crate::scenario::tests::dummy_assets();
        shakedown_run(&assets, &crate::scenario::tests::real_sections())
    }

    /// Every action across all handlers, flattened.
    fn all_actions(config: &ScenarioConfig) -> impl Iterator<Item = &EventActionConfig> {
        config.events.iter().flat_map(|event| event.actions.iter())
    }

    /// The whole script is wired with id STRINGS; a typo fails silently at
    /// runtime (a handler that never fires). Cross-check: every id any
    /// filter matches on or any despawn targets is spawned by some action
    /// (object, area, or the two lazily spawned beacons and pirate).
    #[test]
    fn every_referenced_id_is_spawned() {
        let config = scenario();

        let mut spawned: Vec<String> = Vec::new();
        for action in all_actions(&config) {
            match action {
                EventActionConfig::SpawnScenarioObject(object) => {
                    spawned.push(object.base.id.clone());
                }
                EventActionConfig::CreateScenarioArea(area) => {
                    spawned.push(area.id.clone());
                }
                _ => {}
            }
        }

        let mut referenced: Vec<String> = Vec::new();
        for event in &config.events {
            for filter in &event.filters {
                if let EventFilterConfig::Entity(entity) = filter {
                    referenced.extend(entity.id.clone());
                    referenced.extend(entity.other_id.clone());
                }
            }
        }
        for action in all_actions(&config) {
            match action {
                EventActionConfig::DespawnScenarioObject(despawn) => {
                    referenced.push(despawn.id.clone());
                }
                // Marker targets are id strings too - a typo'd attach is a
                // silently missing marker.
                EventActionConfig::ObjectiveMarkerAttach(attach) => {
                    referenced.push(attach.target_id.clone());
                }
                EventActionConfig::ObjectiveMarkerDetach(detach) => {
                    referenced.push(detach.target_id.clone());
                }
                _ => {}
            }
        }

        for id in &referenced {
            assert!(
                spawned.contains(id),
                "id '{}' is referenced by the script but never spawned; spawned: {:?}",
                id,
                spawned
            );
        }
    }

    /// The conveyance choreography (task 20260712-093831), pinned at the
    /// config level: every leg's target is marked, hand-offs detach the
    /// previous marker, an attach that shares a handler with its target's
    /// spawn comes AFTER the spawn (actions queue in list order - an
    /// attach before the spawn resolves nothing), and the beat-4 GOTO
    /// emphasis is cleared by the orbit handler.
    #[test]
    fn the_marker_rides_every_leg_and_hands_off() {
        let config = scenario();

        // Handler index -> (attach targets, detach targets) in order.
        let marker_ops = |event: &ScenarioEventConfig| {
            let mut attaches = Vec::new();
            let mut detaches = Vec::new();
            for action in &event.actions {
                match action {
                    EventActionConfig::ObjectiveMarkerAttach(attach) => {
                        attaches.push(attach.target_id.clone());
                    }
                    EventActionConfig::ObjectiveMarkerDetach(detach) => {
                        detaches.push(detach.target_id.clone());
                    }
                    _ => {}
                }
            }
            (attaches, detaches)
        };

        // OnStart marks beacon 1.
        let on_start = config
            .events
            .iter()
            .find(|event| matches!(event.name, EventConfig::OnStart))
            .unwrap();
        assert_eq!(marker_ops(on_start).0, vec![ID_BEACON_1.to_string()]);

        // Attach-after-spawn ordering: in every handler that both spawns
        // an object and attaches a marker to it, the spawn comes first.
        for event in &config.events {
            let mut spawned_so_far: Vec<&str> = Vec::new();
            let spawned_by_this_handler: Vec<String> = {
                // Ids spawned by OTHER handlers before this one can run are
                // not checkable statically; restrict the ordering assert to
                // ids this same handler spawns.
                event
                    .actions
                    .iter()
                    .filter_map(|action| match action {
                        EventActionConfig::SpawnScenarioObject(object) => {
                            Some(object.base.id.clone())
                        }
                        _ => None,
                    })
                    .collect()
            };
            for action in &event.actions {
                match action {
                    EventActionConfig::SpawnScenarioObject(object) => {
                        spawned_so_far.push(object.base.id.as_str());
                    }
                    EventActionConfig::ObjectiveMarkerAttach(attach)
                        if spawned_by_this_handler.contains(&attach.target_id) =>
                    {
                        assert!(
                            spawned_so_far.contains(&attach.target_id.as_str()),
                            "attach to '{}' precedes its spawn in the same handler",
                            attach.target_id
                        );
                    }
                    _ => {}
                }
            }
        }

        // Hand-offs down the v2 leg chain: beacon 1 -> beacon 2 -> crates
        // -> beacon 3 -> beacon 4 -> planetoid -> derelict -> pirate ->
        // done (each attach handler detaches the previous leg's marker;
        // the crate markers die with their crates).
        let handler_with_attach = |target: &str| {
            config
                .events
                .iter()
                .find(|event| marker_ops(event).0.iter().any(|id| id == target))
                .unwrap_or_else(|| panic!("some handler attaches to '{}'", target))
        };
        assert_eq!(
            marker_ops(handler_with_attach(ID_BEACON_2)).1,
            vec![ID_BEACON_1.to_string()]
        );
        let crates_handler = handler_with_attach("crate_1");
        assert_eq!(marker_ops(crates_handler).1, vec![ID_BEACON_2.to_string()]);
        assert_eq!(
            marker_ops(crates_handler).0,
            vec!["crate_1", "crate_2", "crate_3"]
        );
        assert_eq!(
            marker_ops(handler_with_attach(ID_BEACON_3)).1,
            Vec::<String>::new()
        );
        assert_eq!(
            marker_ops(handler_with_attach(ID_BEACON_4)).1,
            vec![ID_BEACON_3.to_string()]
        );
        assert_eq!(
            marker_ops(handler_with_attach(ID_PLANETOID)).1,
            vec![ID_BEACON_4.to_string()]
        );
        assert_eq!(
            marker_ops(handler_with_attach(ID_DERELICT)).1,
            vec![ID_PLANETOID.to_string()]
        );
        assert_eq!(
            marker_ops(handler_with_attach(ID_PIRATE)).1,
            vec![ID_DERELICT.to_string()]
        );
        let done_handler = config
            .events
            .iter()
            .find(|event| {
                event.actions.iter().any(|action| {
                    matches!(action, EventActionConfig::Objective(objective) if objective.id == OBJ_DONE)
                })
            })
            .unwrap();
        assert_eq!(marker_ops(done_handler).1, vec![ID_PIRATE.to_string()]);

        // Emphasis pairing: every emphasized verb is cleared downstream
        // (teardown covers death, but the happy path must not rely on it).
        // v2 sequences: RADAR for the first lock (cleared when it lands),
        // GOTO for the autopilot legs (cleared at the coast), RADAR again
        // for the combat rehearsal (cleared when the red lock lands).
        let mut set_verbs = Vec::new();
        let mut cleared_verbs = Vec::new();
        for action in all_actions(&config) {
            match action {
                EventActionConfig::HintEmphasisSet(set) => set_verbs.push(set.verb.clone()),
                EventActionConfig::HintEmphasisClear(clear) => {
                    cleared_verbs.push(clear.verb.clone())
                }
                _ => {}
            }
        }
        assert_eq!(
            set_verbs,
            vec!["RADAR".to_string(), "GOTO".to_string(), "RADAR".to_string()]
        );
        assert_eq!(
            cleared_verbs,
            vec!["RADAR".to_string(), "GOTO".to_string(), "RADAR".to_string()]
        );
    }

    /// The ambush choreography: the pirate is NOT part of the opening
    /// spawn set - it enters in exactly one later handler (the salvage
    /// completion), patrolling the debris cluster, passive by
    /// construction (patrol AI engages only inside AI_ENGAGE_RANGE or
    /// when damaged).
    #[test]
    fn pirate_spawns_late_at_the_debris_cluster() {
        let config = scenario();

        let on_start_spawns: Vec<&ScenarioObjectConfig> = config
            .events
            .iter()
            .filter(|event| matches!(event.name, EventConfig::OnStart))
            .flat_map(|event| event.actions.iter())
            .filter_map(|action| match action {
                EventActionConfig::SpawnScenarioObject(object) => Some(object),
                _ => None,
            })
            .collect();
        assert!(
            on_start_spawns
                .iter()
                .all(|object| object.base.id != ID_PIRATE),
            "the pirate must not be in the opening spawn set"
        );

        let pirate_spawns: Vec<&ScenarioObjectConfig> = all_actions(&config)
            .filter_map(|action| match action {
                EventActionConfig::SpawnScenarioObject(object) if object.base.id == ID_PIRATE => {
                    Some(object)
                }
                _ => None,
            })
            .collect();
        assert_eq!(pirate_spawns.len(), 1, "exactly one pirate spawn action");

        let pirate = pirate_spawns[0];
        let ScenarioObjectKind::Spaceship(ship) = &pirate.kind else {
            panic!("the pirate is a spaceship");
        };
        let SpaceshipController::AI(ai) = &ship.controller else {
            panic!("the pirate is AI-controlled");
        };
        assert!(!ai.patrol.is_empty(), "the pirate patrols");
        for waypoint in &ai.patrol {
            assert!(
                waypoint.distance(DEBRIS_CENTER) < 100.0,
                "patrol waypoint {:?} is over the debris cluster",
                waypoint
            );
        }
    }

    /// Ship minimalism (user direction 2026-07-12): one turret each, no
    /// torpedo bays; the pirate's turret is the light one and its hull the
    /// light one - "gentle" is data, not behavior tweaks.
    #[test]
    fn ships_are_minimal_and_the_pirate_is_scavenger_grade() {
        let config = scenario();

        let ships: Vec<(&str, &SpaceshipConfig)> = all_actions(&config)
            .filter_map(|action| match action {
                EventActionConfig::SpawnScenarioObject(object) => match &object.kind {
                    ScenarioObjectKind::Spaceship(ship) => Some((object.base.id.as_str(), ship)),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(ships.len(), 2, "player and pirate only");

        for (id, ship) in &ships {
            let turrets: Vec<_> = ship
                .sections
                .iter()
                .filter(|section| matches!(section.config.kind, SectionKind::Turret(_)))
                .collect();
            assert_eq!(turrets.len(), 1, "'{}' carries exactly one turret", id);
            assert!(
                !ship
                    .sections
                    .iter()
                    .any(|section| matches!(section.config.kind, SectionKind::Torpedo(_))),
                "'{}' has no torpedo bay",
                id
            );

            let expected_turret = if *id == ID_PIRATE {
                "light_turret_section"
            } else {
                "better_turret_section"
            };
            assert_eq!(
                turrets[0].config.base.id, expected_turret,
                "'{}' turret grade",
                id
            );
        }

        // No holes in the silhouette (playtest finding 7): every section
        // sits within one unit of another section.
        for (id, ship) in &ships {
            for section in &ship.sections {
                let adjacent = ship.sections.iter().any(|other| {
                    other.id != section.id
                        && other.position.distance(section.position) <= 1.0 + 1e-3
                });
                assert!(
                    adjacent,
                    "'{}' section '{}' at {:?} has no adjacent neighbor",
                    id, section.id, section.position
                );
            }
        }

        let pirate = ships.iter().find(|(id, _)| *id == ID_PIRATE).unwrap().1;
        let SpaceshipController::AI(pirate_ai) = &pirate.controller else {
            panic!("the pirate is AI-controlled");
        };
        assert_eq!(
            pirate_ai.leash,
            Some(PIRATE_LEASH_RADIUS),
            "the scavenger is leashed to the debris field (playtest round 3)"
        );
        assert!(
            pirate
                .sections
                .iter()
                .any(|section| section.config.base.id == "light_hull_section"),
            "the pirate's hull is the light one"
        );
    }

    /// The layout is authored against the planetoid's RUNTIME geometry
    /// (authored-vs-derived lesson): the noise mesh reaches
    /// ASTEROID_GEOMETRIC_FACTOR_MIN..MAX times the nominal radius - the
    /// bounds are exported by nova_scenario and pinned there by a 256-seed
    /// sweep (review R2.2: the first cut hardcoded one observed band,
    /// 4.0-4.55, and real seeds reach 5.64, which parked the orbit ring
    /// OUTSIDE the old 160u gate: a silent beat-4 softlock). The SOI is
    /// soi_factor(8) times the geometric radius
    /// (GravityWell::from_surface_gravity), and the ORBIT ring parks at
    /// orbit_clearance_factor(1.5) * (body_radius + surface_margin(1))
    /// (flight.rs). Pin the beat-4 geometry against the WORST seed in the
    /// exported range: beacon 3 inside the smallest SOI, outside the gate;
    /// the gate outside the widest orbit ring.
    #[test]
    fn beat4_geometry_holds_across_the_derived_radius_range() {
        const SOI_FACTOR: f32 = 8.0;
        const ORBIT_CLEARANCE: f32 = 1.5;
        const SURFACE_MARGIN: f32 = 1.0;

        let smallest_soi = SOI_FACTOR * PLANETOID_NOMINAL_RADIUS * ASTEROID_GEOMETRIC_FACTOR_MIN;
        let largest_soi = SOI_FACTOR * PLANETOID_NOMINAL_RADIUS * ASTEROID_GEOMETRIC_FACTOR_MAX;
        let widest_ring = ORBIT_CLEARANCE
            * (PLANETOID_NOMINAL_RADIUS * ASTEROID_GEOMETRIC_FACTOR_MAX + SURFACE_MARGIN);

        // Beacon 3 (the FIRST lock target, beat sheet v2): its GOTO leg is
        // the gravity-free rehearsal, so it must clear even the worst-seed
        // SOI - and stay within the DEFAULT beacon lock range of the
        // debris cluster, where the lesson is taught (BEACON_LOCK_SIGNATURE
        // 20 * signature_range_per_unit 30 = 600u; both cited constants).
        let beacon_3_planetoid = BEACON_3_POS.distance(PLANETOID_POS);
        assert!(
            beacon_3_planetoid > largest_soi + 40.0,
            "beacon 3 ({beacon_3_planetoid:.0}u from the planetoid) must clear the \
             largest plausible SOI ({largest_soi:.0}u)"
        );
        let default_lock_range = 20.0 * 30.0;
        let cluster_to_beacon_3 = DEBRIS_CENTER.distance(BEACON_3_POS);
        assert!(
            cluster_to_beacon_3 < default_lock_range - 100.0,
            "beacon 3 ({cluster_to_beacon_3:.0}u from the cluster) must be well inside \
             the default beacon lock range ({default_lock_range:.0}u)"
        );

        // Beacon 4 (the waypoint target): inside the smallest SOI with
        // margin so the ORBIT hint lights on arrival, outside the widest
        // orbit ring, and its 70u trigger must stay CLEAR of the coast
        // ring - a player still inside a trigger when its OnEnter beat
        // arms misses the CollisionStart (the already-inside trap, same
        // rule as the crate sensors below).
        let beacon_4_distance = BEACON_4_POS.distance(PLANETOID_POS);
        assert!(
            beacon_4_distance < smallest_soi * 0.75,
            "beacon 4 ({beacon_4_distance:.0}u) sits inside the smallest plausible SOI \
             ({smallest_soi:.0}u) with margin, so the ORBIT hint lights on arrival"
        );
        assert!(
            beacon_4_distance > widest_ring + 30.0,
            "beacon 4 ({beacon_4_distance:.0}u) clears the widest orbit ring \
             ({widest_ring:.0}u)"
        );
        assert!(
            beacon_4_distance - BEACON_AREA_RADIUS > COAST_RING_RADIUS + 10.0,
            "beacon 4's trigger inner edge ({:.0}u) stays outside the coast ring \
             ({COAST_RING_RADIUS}u) - the already-inside trap",
            beacon_4_distance - BEACON_AREA_RADIUS
        );
        // The waypoint LEG must be lockable: beacon 4 authors its own
        // signature (BEACON_4_LOCK_SIGNATURE * 30u/unit, the range model).
        let waypoint_leg = BEACON_3_POS.distance(BEACON_4_POS);
        assert!(
            waypoint_leg < BEACON_4_LOCK_SIGNATURE * 30.0 - 50.0,
            "the waypoint leg ({waypoint_leg:.0}u) fits beacon 4's authored lock range \
             ({:.0}u) with margin",
            BEACON_4_LOCK_SIGNATURE * 30.0
        );

        // The coast ring: outside the widest orbit ring (the held orbit
        // must stay INSIDE the ring, or breaking away could not be
        // detected by OnExit - and a swing outside during capture would
        // fire it early, though the beat guard eats that), inside the
        // smallest SOI (the coast is FELT on every seed).
        assert!(
            COAST_RING_RADIUS > widest_ring + 20.0,
            "the coast ring ({COAST_RING_RADIUS}u) clears the widest orbit ring \
             ({widest_ring:.0}u)"
        );
        assert!(
            COAST_RING_RADIUS < smallest_soi - 50.0,
            "the coast ring ({COAST_RING_RADIUS}u) sits well inside the smallest SOI \
             ({smallest_soi:.0}u)"
        );

        // The derelict: a DYNAMIC body - inside the SOI it would fall into
        // the planetoid; it must hold still by the old salvage field.
        let derelict_distance = DERELICT_POS.distance(PLANETOID_POS);
        assert!(
            derelict_distance > largest_soi + 40.0,
            "the derelict ({derelict_distance:.0}u from the planetoid) must clear the \
             largest plausible SOI ({largest_soi:.0}u)"
        );

        // Playtest round 2 finding 1: the debris cluster (and every crate
        // in it) must sit OUTSIDE the worst-seed SOI - the salvage beat is
        // flown by hand, and fighting gravity while weaving crates reads
        // as a bug, not a challenge.
        let cluster_distance = DEBRIS_CENTER.distance(PLANETOID_POS);
        assert!(
            cluster_distance > largest_soi + 40.0,
            "the debris cluster ({cluster_distance:.0}u from the planetoid) must clear \
             the largest plausible SOI ({largest_soi:.0}u)"
        );
        for (i, crate_pos) in CRATE_POSITIONS.iter().enumerate() {
            let distance = crate_pos.distance(PLANETOID_POS);
            assert!(
                distance > largest_soi + 40.0,
                "crate_{} ({distance:.0}u) sits outside the largest plausible SOI \
                 with margin",
                i + 1
            );
        }

        // Review R2.3 (adapted): the beacon triggers must CONTAIN the
        // GOTO park point (playtest finding 2) - the autopilot stops
        // arrival_standoff from an unsized target, and a smaller trigger
        // parks the ship outside its own objective.
        let standoff = nova_gameplay::prelude::FlightSettings::default().arrival_standoff;
        assert!(
            BEACON_AREA_RADIUS > standoff + 10.0,
            "beacon trigger ({BEACON_AREA_RADIUS}u) must contain the GOTO park point \
             (standoff {standoff}u) with margin"
        );
        // No crate sensor reachable from inside beacon 2's trigger:
        // the beat 2->3 flip happens inside beacon 2's area, and a player
        // already parked inside a crate sensor when the pickups arm would
        // miss its CollisionStart.
        for (i, crate_pos) in CRATE_POSITIONS.iter().enumerate() {
            let distance = crate_pos.distance(BEACON_2_POS);
            assert!(
                distance > BEACON_AREA_RADIUS + CRATE_AREA_RADIUS,
                "crate_{} ({distance:.0}u from beacon 2) must not overlap beacon 2's \
                 trigger volume",
                i + 1
            );
        }
    }

    /// Player death restarts the run (linger: Enter confirms), matching
    /// the asteroid_field pattern.
    #[test]
    fn player_death_routes_back_to_shakedown() {
        let config = scenario();

        let death_routes: Vec<&NextScenarioActionConfig> = config
            .events
            .iter()
            .filter(|event| {
                matches!(event.name, EventConfig::OnDestroyed)
                    && event.filters.iter().any(|filter| {
                        matches!(
                            filter,
                            EventFilterConfig::Entity(entity)
                                if entity.id.as_deref() == Some(ID_PLAYER)
                        )
                    })
            })
            .flat_map(|event| event.actions.iter())
            .filter_map(|action| match action {
                EventActionConfig::NextScenario(next) => Some(next),
                _ => None,
            })
            .collect();

        assert_eq!(death_routes.len(), 1);
        assert_eq!(death_routes[0].scenario_id, SHAKEDOWN_SCENARIO_ID);
        assert!(death_routes[0].linger, "Enter confirms the restart");
    }

    /// A headless app running the real event pipeline with the scenario's
    /// handlers registered exactly as on_load_scenario registers them -
    /// the shared rig for the beat-walk tests (review R2.4).
    fn scripted_app() -> App {
        use avian3d::prelude::PhysicsPlugins;
        use bevy_rand::prelude::{EntropyPlugin, WyRand};

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            PhysicsPlugins::default(),
            EntropyPlugin::<WyRand>::default(),
        ));
        app.init_asset::<StandardMaterial>();
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        // Production inits this in the HUD plugin; the emphasis actions
        // write it through the same drain the walk exercises.
        app.init_resource::<HintEmphasis>();
        app.add_plugins(ScenarioObjectsPlugin { render: false });
        app.finish();

        let config = scenario();
        for event in &config.events {
            let mut handler = EventHandler::<NovaEventWorld>::from(event.name);
            for filter in &event.filters {
                handler.add_filter(filter.clone());
            }
            for action in &event.actions {
                handler.add_action(action.clone());
            }
            app.world_mut().spawn(handler);
        }
        app
    }

    /// Fire the OnStart the loader fires after registration, plus one
    /// OnUpdate pulse (the loader's fire_on_update equivalent).
    fn boot(app: &mut App) {
        use nova_events::prelude::*;
        app.world_mut()
            .commands()
            .fire::<OnStartEvent>(OnStartEventInfo);
        pulse(app);
    }

    /// One OnUpdate pulse + settle, the way the loader's fire_on_update
    /// emits it while a scenario is live.
    fn pulse(app: &mut App) {
        use nova_events::prelude::*;
        app.world_mut()
            .commands()
            .fire::<OnUpdateEvent>(OnUpdateEventInfo);
        app.update();
        app.update();
    }

    /// The player ship enters `area` (the physics half of this event is
    /// proven by the salvage pipeline test in 20260712-093044).
    fn enter(app: &mut App, area: &str) {
        use nova_events::prelude::*;
        app.world_mut()
            .commands()
            .fire::<OnEnterEvent>(OnEnterEventInfo {
                id: area.to_string(),
                other_id: ID_PLAYER.to_string(),
                other_type_name: "spaceship".to_string(),
            });
        app.update();
        app.update();
    }

    /// The player has held an orbit around `well` (the orbit-hold
    /// tracker's event; the tracker itself is tested in nova_scenario's
    /// loader tests - here the script consumes the event).
    fn orbit(app: &mut App, well: &str) {
        use nova_events::prelude::*;
        app.world_mut()
            .commands()
            .fire::<OnOrbitEvent>(OnOrbitEventInfo {
                id: well.to_string(),
                other_id: ID_PLAYER.to_string(),
                other_type_name: "spaceship".to_string(),
            });
        app.update();
        app.update();
    }

    fn destroy(app: &mut App, id: &str) {
        use nova_events::prelude::*;
        app.world_mut()
            .commands()
            .fire::<OnDestroyedEvent>(OnDestroyedEventInfo {
                id: id.to_string(),
                type_name: "spaceship".to_string(),
            });
        app.update();
        app.update();
    }

    /// The player left `area` (the area plugin's exit half).
    fn exit(app: &mut App, area: &str) {
        use nova_events::prelude::*;
        app.world_mut()
            .commands()
            .fire::<OnExitEvent>(OnExitEventInfo {
                id: area.to_string(),
                other_id: ID_PLAYER.to_string(),
                other_type_name: "spaceship".to_string(),
            });
        app.update();
        app.update();
    }

    /// The player's TRAVEL lock landed on `id` (the loader's lock bridge -
    /// tested in nova_scenario; here the script consumes the event). The
    /// bridge ECHOES a held lock every few seconds, so firing this twice
    /// for the same id models a stale held lock.
    fn travel_lock(app: &mut App, id: &str) {
        use nova_events::prelude::*;
        app.world_mut()
            .commands()
            .fire::<OnTravelLockEvent>(OnTravelLockEventInfo {
                id: id.to_string(),
                other_id: ID_PLAYER.to_string(),
                other_type_name: "spaceship".to_string(),
            });
        app.update();
        app.update();
    }

    /// The player's COMBAT lock landed on `id`.
    fn combat_lock(app: &mut App, id: &str) {
        use nova_events::prelude::*;
        app.world_mut()
            .commands()
            .fire::<OnCombatLockEvent>(OnCombatLockEventInfo {
                id: id.to_string(),
                other_id: ID_PLAYER.to_string(),
                other_type_name: "spaceship".to_string(),
            });
        app.update();
        app.update();
    }

    /// Walk ALL FIVE BEATS through the real event pipeline: the actual
    /// handlers registered exactly as the loader registers them, real
    /// spawn/despawn commands applied to a real World, beat transitions
    /// driven by the same OnEnter/OnDestroyed/OnUpdate events production
    /// fires. This test owns the SCRIPT: gating, counting, lazy spawns,
    /// tally text, the main ending.
    #[test]
    fn the_five_beats_walk_end_to_end() {
        use nova_events::prelude::*;

        let mut app = scripted_app();

        let beat = |app: &App| -> f64 {
            match app
                .world()
                .resource::<NovaEventWorld>()
                .get_variable(VAR_BEAT)
            {
                Some(VariableLiteral::Number(n)) => *n,
                other => panic!("beat variable missing or non-numeric: {:?}", other),
            }
        };
        let has_objective = |app: &App, id: &str| -> bool {
            app.world()
                .resource::<GameObjectives>()
                .objectives
                .iter()
                .any(|objective| objective.id == id)
        };
        let objective_message = |app: &App, id: &str| -> String {
            app.world()
                .resource::<GameObjectives>()
                .objectives
                .iter()
                .find(|objective| objective.id == id)
                .map(|objective| objective.message.clone())
                .unwrap_or_default()
        };
        let entity_with_id = |app: &mut App, id: &str| -> Option<Entity> {
            let mut query = app.world_mut().query::<(Entity, &EntityId)>();
            query
                .iter(app.world())
                .find(|(_, entity_id)| entity_id.0 == id)
                .map(|(entity, _)| entity)
        };
        let marker_label = |app: &mut App, id: &str| -> Option<String> {
            let entity = entity_with_id(app, id)?;
            app.world()
                .get::<ObjectiveMarkerTarget>(entity)
                .map(|marker| marker.label.clone())
        };
        let goto_emphasized =
            |app: &App| -> bool { app.world().resource::<HintEmphasis>().contains("GOTO") };
        let radar_emphasized =
            |app: &App| -> bool { app.world().resource::<HintEmphasis>().contains("RADAR") };
        // The Lock capability on the player's REAL controller section (the
        // capability beat, task 20260713-090653 - same pin shape as the
        // training governor).
        let lock_granted = |app: &mut App, player: Entity| -> bool {
            let mut q_controllers = app
                .world_mut()
                .query_filtered::<(&ChildOf, &ControllerVerbs), With<ControllerSectionMarker>>();
            q_controllers
                .iter(app.world())
                .find(|(ChildOf(parent), _)| *parent == player)
                .map(|(_, verbs)| verbs.granted(FlightVerb::Lock))
                .expect("the player ship has a controller section")
        };

        // Boot: OnStart is what the loader fires after registration.
        boot(&mut app);

        assert_eq!(beat(&app), 1.0);
        assert_eq!(
            marker_label(&mut app, ID_BEACON_1).as_deref(),
            Some("BEACON 1"),
            "the gold marker rides beacon 1 from the start"
        );
        assert!(has_objective(&app, OBJ_B1), "beat 1 objective is up");
        assert!(
            entity_with_id(&mut app, ID_PLAYER).is_some(),
            "the player ship spawned"
        );
        assert!(entity_with_id(&mut app, ID_BEACON_1).is_some());
        assert!(
            entity_with_id(&mut app, ID_BEACON_2).is_none(),
            "beacon 2 spawns lazily with its beat"
        );
        assert!(entity_with_id(&mut app, ID_PLANETOID).is_some());
        assert!(entity_with_id(&mut app, "crate_1").is_some());
        // The training governor is aboard for beat 1 (delivery guard for
        // the release assert below: the cap must exist to be removed).
        let player = entity_with_id(&mut app, ID_PLAYER).unwrap();
        assert!(
            app.world().get::<FlightSpeedCap>(player).is_some(),
            "the training governor caps the fresh ship"
        );
        assert!(
            !lock_granted(&mut app, player),
            "the targeting computer starts OFFLINE (lock withheld; CTRL answers with the deny cue)"
        );

        // Beat 1 -> 2.
        enter(&mut app, ID_BEACON_1);
        assert_eq!(beat(&app), 2.0);
        assert!(!has_objective(&app, OBJ_B1), "beat 1 objective completed");
        assert!(has_objective(&app, OBJ_B2));
        assert!(entity_with_id(&mut app, ID_BEACON_2).is_some());
        // The governor releases with the beat (playtest round 2 finding 3).
        assert!(
            app.world().get::<FlightSpeedCap>(player).is_none(),
            "reaching beacon 1 releases the training governor"
        );
        // Marker hand-off: beacon 1 yields, the fresh beacon 2 carries it.
        assert_eq!(marker_label(&mut app, ID_BEACON_1), None);
        assert_eq!(
            marker_label(&mut app, ID_BEACON_2).as_deref(),
            Some("BEACON 2")
        );

        // A stray re-entry into beacon 1 must not re-fire the beat.
        enter(&mut app, ID_BEACON_1);
        assert_eq!(beat(&app), 2.0, "finished beats do not re-fire");

        // Beat 2 -> 3.
        enter(&mut app, ID_BEACON_2);
        assert_eq!(beat(&app), 3.0);
        assert!(has_objective(&app, OBJ_B3));
        // All three crates carry the marker at once.
        assert_eq!(marker_label(&mut app, ID_BEACON_2), None);
        for crate_id in ["crate_1", "crate_2", "crate_3"] {
            assert_eq!(
                marker_label(&mut app, crate_id).as_deref(),
                Some("SALVAGE"),
                "{crate_id} is marked for the sweep"
            );
        }

        // Beat 3: the salvage sweep. Tally text follows the count via the
        // OnUpdate milestones; crates despawn on pickup.
        enter(&mut app, "crate_1");
        pulse(&mut app);
        assert!(
            entity_with_id(&mut app, "crate_1").is_none(),
            "picked-up crate despawns"
        );
        assert!(
            objective_message(&app, OBJ_B3).contains("1/3"),
            "tally shows 1/3, got: {}",
            objective_message(&app, OBJ_B3)
        );

        enter(&mut app, "crate_2");
        pulse(&mut app);
        assert!(objective_message(&app, OBJ_B3).contains("2/3"));

        enter(&mut app, "crate_3");
        pulse(&mut app);
        assert_eq!(beat(&app), 4.0, "all crates aboard advances the beat");
        assert!(has_objective(&app, OBJ_B4));
        assert!(
            entity_with_id(&mut app, ID_BEACON_3).is_some(),
            "beacon 3 appears with beat 4"
        );
        assert!(
            entity_with_id(&mut app, ID_PIRATE).is_none(),
            "beat 4 is pirate-free (playtest finding 4)"
        );
        // Beat 4 conveyance: the marker rides the lock target, RADAR (and
        // only RADAR) pulses, and the targeting computer is now online.
        assert_eq!(
            marker_label(&mut app, ID_BEACON_3).as_deref(),
            Some("BEACON 3")
        );
        assert!(radar_emphasized(&app), "beat 4 emphasizes RADAR");
        assert!(!goto_emphasized(&app), "GOTO waits for its own beat");
        assert!(
            lock_granted(&mut app, player),
            "beat 4 brings the targeting computer ONLINE (delivery guard: withheld at boot)"
        );

        // Beat 4 -> 5: the white lock lands (the OnTravelLock bridge).
        travel_lock(&mut app, ID_BEACON_3);
        assert_eq!(beat(&app), 5.0, "the lock lesson ticks on the lock");
        assert!(has_objective(&app, OBJ_B5));
        assert!(!radar_emphasized(&app), "RADAR retires with its lesson");
        assert!(goto_emphasized(&app), "beat 5 emphasizes GOTO");

        // The bridge ECHOES held locks every few seconds: a stale re-fire
        // for beacon 3 during beat 5 must be a no-op (beat guards own
        // ordering; the echo exists so a lock HELD across a beat advance
        // can still complete a lesson, not to skip ones already done).
        travel_lock(&mut app, ID_BEACON_3);
        assert_eq!(
            beat(&app),
            5.0,
            "a stale lock echo does not re-fire the beat"
        );

        // Beat 5 -> 6: arrival at beacon 3; the waypoint run opens.
        enter(&mut app, ID_BEACON_3);
        assert_eq!(beat(&app), 6.0);
        assert!(has_objective(&app, OBJ_B6));
        assert!(
            entity_with_id(&mut app, ID_BEACON_4).is_some(),
            "beacon 4 spawns lazily with its beat"
        );
        assert_eq!(marker_label(&mut app, ID_BEACON_3), None);
        assert_eq!(
            marker_label(&mut app, ID_BEACON_4).as_deref(),
            Some("BEACON 4")
        );

        // Beat 6 -> 7: arrival at beacon 4; the coast ring appears and the
        // hands-off lesson retires.
        enter(&mut app, ID_BEACON_4);
        assert_eq!(beat(&app), 7.0);
        assert!(has_objective(&app, OBJ_B7));
        assert!(
            entity_with_id(&mut app, ID_COAST_RING).is_some(),
            "the coast ring spawns with its beat (never early - the \
             already-inside trap)"
        );
        assert!(!goto_emphasized(&app), "GOTO retires at the coast");
        assert_eq!(
            marker_label(&mut app, ID_PLANETOID).as_deref(),
            Some("PLANETOID")
        );

        // Beat 7 -> 8: the drift crosses the ring.
        enter(&mut app, ID_COAST_RING);
        assert_eq!(beat(&app), 8.0);
        assert!(has_objective(&app, OBJ_B8));

        // Beat 8 -> 9: orbit held; the derelict appears by the old field.
        orbit(&mut app, ID_PLANETOID);
        assert_eq!(beat(&app), 9.0);
        assert!(has_objective(&app, OBJ_B9));
        assert!(
            entity_with_id(&mut app, ID_DERELICT).is_some(),
            "the derelict spawns with the rehearsal"
        );
        assert!(
            entity_with_id(&mut app, ID_PIRATE).is_none(),
            "still no scavenger - the rehearsal comes first"
        );
        assert_eq!(
            marker_label(&mut app, ID_DERELICT).as_deref(),
            Some("DERELICT")
        );

        // Beat 9 -> 10: breaking away exits the ring; the combat-lock
        // lesson begins.
        exit(&mut app, ID_COAST_RING);
        assert_eq!(beat(&app), 10.0);
        assert!(has_objective(&app, OBJ_B10));
        assert!(radar_emphasized(&app), "the rehearsal re-emphasizes RADAR");

        // An early COMBAT lock on the derelict during beat 9 would have
        // been a no-op; the echo covers the held lock once beat 10 arms -
        // modeled here by the beat-10 fire.
        combat_lock(&mut app, ID_DERELICT);
        assert_eq!(beat(&app), 11.0, "the red lock ticks the lesson");
        assert!(has_objective(&app, OBJ_B11));
        assert!(!radar_emphasized(&app), "RADAR retires with the red lock");

        // Beat 11 -> 12: the hulk dies; NOW the scavenger appears.
        destroy(&mut app, ID_DERELICT);
        assert_eq!(beat(&app), 12.0);
        assert!(has_objective(&app, OBJ_B12));
        assert!(
            entity_with_id(&mut app, ID_PIRATE).is_some(),
            "the scavenger spawns with the beat-12 reveal"
        );
        assert_eq!(
            marker_label(&mut app, ID_PIRATE).as_deref(),
            Some("SCAVENGER")
        );

        // Beat 12 -> done: the scavenger driven off.
        destroy(&mut app, ID_PIRATE);
        assert_eq!(beat(&app), 13.0);
        assert!(!has_objective(&app, OBJ_B12));
        assert!(has_objective(&app, OBJ_DONE), "the run completes");
        // Free flight is marker-free: the done handler's defensive detach
        // (the rig's destroy event does not despawn the wreck, so the
        // detach action is what clears it here).
        assert_eq!(marker_label(&mut app, ID_PIRATE), None);
    }

    /// The pirate exists only from the beat-12 reveal on (playtest finding
    /// 4 lineage), so an "early kill" is no longer reachable: a stray
    /// OnDestroyed(pirate) DURING the rehearsal (e.g. a scenario edit
    /// re-introducing an early spawn) must be a no-op, not a skipped
    /// fight - the beat-12 guard on the kill handler owns that.
    #[test]
    fn pirate_destruction_only_counts_during_the_final_beat() {
        let mut app = scripted_app();
        boot(&mut app);
        enter(&mut app, ID_BEACON_1);
        enter(&mut app, ID_BEACON_2);
        for crate_id in ["crate_1", "crate_2", "crate_3"] {
            enter(&mut app, crate_id);
            pulse(&mut app);
        }
        travel_lock(&mut app, ID_BEACON_3);
        enter(&mut app, ID_BEACON_3);
        enter(&mut app, ID_BEACON_4);
        enter(&mut app, ID_COAST_RING);
        orbit(&mut app, ID_PLANETOID);
        exit(&mut app, ID_COAST_RING);

        // Beat 10 (the rehearsal): a pirate death event is out-of-script;
        // nothing moves.
        destroy(&mut app, ID_PIRATE);
        let objectives = &app.world().resource::<GameObjectives>().objectives;
        assert!(
            !objectives.iter().any(|objective| objective.id == OBJ_DONE),
            "a stray pirate death during the rehearsal must not complete the run"
        );

        // The real path still works: red lock, hulk down, scavenger down.
        combat_lock(&mut app, ID_DERELICT);
        destroy(&mut app, ID_DERELICT);
        destroy(&mut app, ID_PIRATE);
        let objectives = &app.world().resource::<GameObjectives>().objectives;
        assert!(
            objectives.iter().any(|objective| objective.id == OBJ_DONE),
            "the beat-12 kill completes the run, got: {:?}",
            objectives
        );
    }

    /// The beat variable gates every non-setup handler: a stray re-entry
    /// into an old area cannot re-fire a finished beat, and the tally
    /// milestones advance on OnUpdate (order-independent of the pickup
    /// event's handler iteration).
    #[test]
    fn every_gameplay_handler_is_beat_gated() {
        let config = scenario();

        for event in &config.events {
            if matches!(event.name, EventConfig::OnStart) {
                continue;
            }
            // The death handler is deliberately beat-free (dying is always
            // fatal).
            let is_death_handler = event.filters.iter().any(|filter| {
                matches!(
                    filter,
                    EventFilterConfig::Entity(entity)
                        if entity.id.as_deref() == Some(ID_PLAYER)
                )
            });
            if is_death_handler {
                continue;
            }
            assert!(
                event
                    .filters
                    .iter()
                    .any(|filter| matches!(filter, EventFilterConfig::Expression(_))),
                "handler {:?} with entity filters {:?} lacks a beat/variable guard",
                event.name,
                event.filters.len()
            );
        }
    }

    /// The first/New Game scenario must not gate the player on ammo
    /// (task 20260712-140250): guard that the player ship is actually built
    /// with `infinite_ammo` ON, so it cannot be silently turned off. Fails if
    /// the flag is dropped or flipped - the mechanism test in nova_scenario
    /// would still pass, so this is the one that pins the user-facing behavior.
    #[test]
    fn the_new_game_player_has_infinite_ammo() {
        let sections = crate::scenario::tests::real_sections();
        let player = player_ship(&sections);
        let ScenarioObjectKind::Spaceship(config) = player.kind else {
            panic!("the player object must be a spaceship");
        };
        let SpaceshipController::Player(controller) = config.controller else {
            panic!("the player ship must be player-controlled");
        };
        assert!(
            controller.infinite_ammo,
            "the New Game player must have infinite ammo"
        );
    }

    /// The player's controller section is authored with GOTO withheld (STOP
    /// and ORBIT stay granted), so the verb is off from the instant the
    /// section is built - no OnStart-action ordering window.
    #[test]
    fn the_new_game_player_starts_with_goto_withheld() {
        let sections = crate::scenario::tests::real_sections();
        let player = player_ship(&sections);
        let ScenarioObjectKind::Spaceship(config) = player.kind else {
            panic!("the player object must be a spaceship");
        };
        let controller = config
            .sections
            .iter()
            .find(|section| matches!(section.config.kind, SectionKind::Controller(_)))
            .expect("the player ship has a controller section");
        let SectionKind::Controller(ref controller_config) = controller.config.kind else {
            unreachable!("filtered to Controller above");
        };
        assert!(
            !controller_config.verbs.goto,
            "GOTO starts withheld on the fresh player controller"
        );
        assert!(
            controller_config.verbs.stop && controller_config.verbs.orbit,
            "STOP and ORBIT are granted from the start"
        );
    }

    /// End-to-end: GOTO is withheld on the live player controller after boot
    /// and is granted when the first objective (beat 1) completes. Withheld
    /// initially and granted after - deleting either the config off-state or
    /// the beat-1 SetControllerVerb would flip one of these asserts.
    #[test]
    fn goto_unlocks_at_the_first_objective() {
        use nova_events::prelude::*;

        let mut app = scripted_app();

        let controller_goto = |app: &mut App| -> bool {
            let player = {
                let mut q = app.world_mut().query::<(Entity, &EntityId)>();
                q.iter(app.world())
                    .find(|(_, id)| id.0 == ID_PLAYER)
                    .map(|(e, _)| e)
                    .expect("player ship spawned")
            };
            let mut q = app
                .world_mut()
                .query_filtered::<(&ChildOf, &ControllerVerbs), With<ControllerSectionMarker>>();
            q.iter(app.world())
                .find(|(&ChildOf(parent), _)| parent == player)
                .map(|(_, verbs)| verbs.goto)
                .expect("player has a controller section carrying ControllerVerbs")
        };

        boot(&mut app);
        assert!(
            !controller_goto(&mut app),
            "GOTO is withheld on the fresh ship"
        );

        // Clearing beat 1 (the first objective) grants GOTO.
        enter(&mut app, ID_BEACON_1);
        assert!(
            controller_goto(&mut app),
            "reaching beacon 1 (first objective) unlocks GOTO"
        );
    }
}
