//! Final Tally - chapter three, part two: the finale at the gang's claim
//! (task 20260721-161020, spike tasks/20260721-155249/SPIKE.md).
//!
//! The intercepted burn from Lifeline traces to a dead claim: a cracked
//! megahauler anchorage berthed deep in a planetoid's gravity well - the
//! base chain's FIRST combat gravity well, ringed by a scattered belt (the
//! Ring region's first combat use). The player coasts into the pull (the
//! tutorial's gravity-coast beat, now with stakes), SURVEYS the anchorage
//! by holding a travel lock on it (the lock verb reused narratively), breaks
//! the two-ship orbital picket - guards on rails, the orbit directive's
//! first combat use (pinned by ai.rs orbit_directive_tests) - and then the
//! Final Tally itself casts off with an escort: the campaign's only
//! simultaneous capital + escort fight, and its peak.
//!
//! The ending is a proper close, not an omission: the flagship kill opens a
//! clock-gated epilogue (confirm line, then the guild's close, then the
//! Victory overlay) and the campaign completes with NOTHING queued - by
//! design this time, stated in the banner.
//!
//! Structure notes: the planetoid sits at WORLD ORIGIN because ScatterObjects'
//! Ring region is origin-centred (sample() replaces the template position);
//! everything else is authored around it. Gates are FLAG-based (surveyed,
//! picket kills), not act-sequenced, so killing the picket before surveying
//! cannot deadlock the cast-off. Terminal acts close every outcome gate the
//! moment any outcome is declared (LESSONS:
//! outcome-is-last-write-wins-close-the-act): act 1 live, 4 epilogue (the
//! win is locked - a post-kill death declares nothing), 2 won, 3 lost.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::{
    cast::{BELT_RELAY, CAPTAIN_HALLORAN, TALLYMAN},
    craft::{self, ShipGrade},
    pacing::{self, clock_past, mark_clock, open_gate, MID_GAP, REVEAL_GAP},
    shakedown::{
        complete, destroyed, eq_num, gt_num, mark, num, objective, set, spawn, story, unmark, var,
    },
    SCATTER_SEED,
};

pub(crate) const FINAL_TALLY_SCENARIO_ID: &str = "final_tally";

const ID_PLAYER: &str = "player_spaceship";
/// The gravity well the claim hides in. The id doubles as the pickets'
/// orbit-directive target.
const ID_ANCHOR: &str = "claim_anchor";
/// The cracked megahauler's two hull sections - invulnerable set dressing,
/// hard cover, and the SURVEY target (the bow carries the long-range lock
/// signature).
const ID_WRECK_BOW: &str = "anchorage_bow";
const ID_WRECK_STERN: &str = "anchorage_stern";
const ID_PICKET_A: &str = "picket_a";
const ID_PICKET_B: &str = "picket_b";
const ID_FLAGSHIP: &str = "flagship";
const ID_ESCORT: &str = "escort";

const OBJ_SURVEY: &str = "survey";
const OBJ_PICKET: &str = "picket";
const OBJ_BREAK: &str = "break_flagship";

/// Story act: 1 = live (approach, survey, both fights), 4 = the epilogue
/// (flagship dead, the win locked - no outcome can overwrite it), 2 = won,
/// 3 = lost. Terminal acts per the ledger lesson.
const VAR_ACT: &str = "act";
/// One-shot: the anchorage has been surveyed (travel lock held on the bow).
const VAR_SURVEYED: &str = "surveyed";
/// Per-picket kill flags (the broadside pattern: flags, not counters).
const VAR_PICKET_A_DOWN: &str = "picket_a_down";
const VAR_PICKET_B_DOWN: &str = "picket_b_down";
/// One-shot: the flagship has cast off.
const VAR_CAST_OFF: &str = "cast_off";
/// The clock mark (seconds) the cast-off waits for: pickets-down + a
/// breathe. Written by the pickets-down beat as `scenario_elapsed + 6`.
const VAR_CAST_AT: &str = "cast_at";
/// The clock mark the epilogue's beats ride: written by the flagship-kill
/// beat as the kill-time; the close line fires at +4, the banner at +9.
const VAR_EPILOGUE_AT: &str = "epilogue_at";
/// One-shots for the paced lines.
const VAR_HELLO_SAID: &str = "hello_said";
const VAR_TAUNT_SAID: &str = "taunt_said";
const VAR_CLOSE_SAID: &str = "close_said";
/// Pacing (task 20260722-092421): objectives post a beat after the comms line
/// that introduces them. The survey objective follows the opening dispatch; the
/// picket objective follows the survey-confirmed line. Each gate holds a
/// `mark_clock` deadline; the `_posted` flag latches the one-shot.
const VAR_SURVEY_GATE: &str = "survey_gate";
const VAR_SURVEY_POSTED: &str = "survey_posted";
const VAR_PICKET_GATE: &str = "picket_gate";
const VAR_PICKET_POSTED: &str = "picket_posted";
const VAR_BREAK_GATE: &str = "break_gate";
const VAR_BREAK_POSTED: &str = "break_posted";

/// The greeting line's clock gate.
const HELLO_AT: f64 = 9.0;
/// Breathe between pickets-down and the cast-off.
const CAST_OFF_DELAY: f64 = 6.0;
/// Epilogue pacing: the close line and the banner, after the kill.
const CLOSE_LINE_AFTER: f64 = 4.0;
const BANNER_AFTER: f64 = 9.0;

/// The planetoid: nominal 20u, surface gravity 6 - the shakedown
/// planetoid's proven numbers (geometric body 70-120u, SOI 560-960u, from
/// the measured ASTEROID_GEOMETRIC_FACTOR range; the harness pins the
/// derived clearances).
const ANCHOR_POS: Vec3 = Vec3::new(0.0, -20.0, 0.0);
const ANCHOR_RADIUS: f32 = 20.0;
/// Player spawn: outside even the worst-seed SOI (960u from the well), so
/// the approach COASTS into the pull - the tutorial callback.
const PLAYER_SPAWN: Vec3 = Vec3::new(0.0, 20.0, 1150.0);
/// The anchorage: two big invulnerable hull-section rocks off the
/// planetoid's shoulder, clear of its worst-case body.
const WRECK_BOW_POS: Vec3 = Vec3::new(200.0, 20.0, 140.0);
const WRECK_STERN_POS: Vec3 = Vec3::new(-90.0, -40.0, 230.0);
/// Picket spawns: on the well's wire, opposite shoulders, both outside the
/// raider design floor (700u) of the player spawn.
const PICKET_A_SPAWN: Vec3 = Vec3::new(300.0, 0.0, 100.0);
const PICKET_B_SPAWN: Vec3 = Vec3::new(-280.0, 40.0, -120.0);
/// The cast-off berth: the flagship and its escort emerge from BEHIND the
/// anchorage bow (triggered spawns, kept outside the flagship's own 1000u
/// torpedo envelope of the player SPAWN - 952u tripped the balance WARN at
/// z=210, so the berth sits deeper; the audit stays clean with zero acks).
const FLAGSHIP_SPAWN: Vec3 = Vec3::new(150.0, -10.0, 90.0);
const ESCORT_SPAWN: Vec3 = Vec3::new(60.0, 30.0, 280.0);
/// The long-range survey signature on the anchorage bow: lockable from the
/// coast-in (default beacon signature 20 reads ~600u; the bow reads ~1350u).
const WRECK_SURVEY_SIGNATURE: f32 = 45.0;

fn facing_the_approach() -> Quat {
    Quat::from_rotation_y(std::f32::consts::PI)
}

/// The player's finale ship: unchanged from Lifeline/Broadside.
fn player_ship() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_PLAYER.to_string(),
            name: "Player Spaceship".to_string(),
            position: PLAYER_SPAWN,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: craft::RACER_TURRET_IDS
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
                speed_cap: None,
                infinite_ammo: false,
                lock_refire_secs: None,
            }),
            allegiance: None,
            sections: craft::racer_sections(
                ShipGrade::Player,
                vec![SectionModification::DisableVerb(FlightVerb::Rcs)],
            ),
        }),
    }
}

/// The claim's planetoid: invulnerable, gravity-authored - the well the
/// whole finale is staged in, and the pickets' orbit target.
fn claim_anchor(asteroid_texture: &AssetRef<Image>) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_ANCHOR.to_string(),
            name: "The Claim".to_string(),
            position: ANCHOR_POS,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
            destroy_sound: None,
            radius: ANCHOR_RADIUS,
            texture: asteroid_texture.clone(),
            health: 2000.0,
            surface_gravity: Some(6.0),
            invulnerable: true,
            lock_signature: None,
        }),
    }
}

/// An anchorage hull section: a big invulnerable rock (the Ledger's
/// Ceres-Matron set-dressing trick) - hard cover in the well, and the bow
/// carries the survey signature.
fn anchorage_wreck(
    id: &str,
    name: &str,
    position: Vec3,
    radius: f32,
    lock_signature: Option<f32>,
    asteroid_texture: &AssetRef<Image>,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
            destroy_sound: None,
            radius,
            texture: asteroid_texture.clone(),
            health: 1000.0,
            surface_gravity: None,
            invulnerable: true,
            lock_signature,
        }),
    }
}

/// A picket guard: scavenger-grade racer holding an ORBIT directive around
/// the claim - a guard on rails (combat pulls it off the orbit, calm
/// returns it; ai.rs orbit_directive_tests). Graced like every telegraphed
/// hostile; leashed to the well so the fight stays in the pull.
fn picket(id: &str, spawn_pos: Vec3) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: "Tally Picket".to_string(),
            position: spawn_pos,
            rotation: facing_the_approach(),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig {
                orbit: Some(ID_ANCHOR.to_string()),
                leash: Some(600.0),
                engage_delay: Some(8.0),
                ..Default::default()
            }),
            allegiance: None,
            sections: craft::racer_sections(ShipGrade::Enemy, vec![]),
        }),
    }
}

/// The Final Tally: the gang's flagship - the cargob capital at full grade
/// (two PDC turrets, two torpedo tubes), no leash: it casts off to end it.
fn flagship() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_FLAGSHIP.to_string(),
            name: "Flagship Final Tally".to_string(),
            position: FLAGSHIP_SPAWN,
            rotation: facing_the_approach(),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig::default()),
            allegiance: None,
            sections: craft::cargob_sections(),
        }),
    }
}

/// The flagship's escort: a scavenger-grade racer screening the capital
/// (first-pass grade; the playtest tunable is one word).
fn escort() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_ESCORT.to_string(),
            name: "Tally Escort".to_string(),
            position: ESCORT_SPAWN,
            rotation: facing_the_approach(),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig {
                patrol: vec![ESCORT_SPAWN, FLAGSHIP_SPAWN + Vec3::new(0.0, 40.0, 80.0)],
                leash: Some(700.0),
                engage_delay: Some(4.0),
                ..Default::default()
            }),
            allegiance: None,
            sections: craft::racer_sections(ShipGrade::Enemy, vec![]),
        }),
    }
}

/// The belt ring around the claim: the Ring region's first combat use -
/// destructible chaff orbiting the well's plane.
fn claim_belt(asteroid_texture: &AssetRef<Image>) -> EventActionConfig {
    EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "belt_rock_".to_string(),
        count: 16,
        seed: SCATTER_SEED,
        region: ScatterRegion::Ring {
            inner: 260.0,
            outer: 420.0,
            y_min: -70.0,
            y_max: -10.0,
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "belt_rock_".to_string(),
                name: "Claim Belt Rock".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
                destroy_sound: Some(AssetRef::from("self://sounds/explosion.wav")),
                radius: 1.0,
                texture: asteroid_texture.clone(),
                health: 100.0,
                surface_gravity: None,
                invulnerable: false,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((1.5, 3.5)),
    })
}

/// Filter: the player's travel lock landed on `target` (OnTravelLock:
/// id = the locked object, other = the locking ship).
fn player_travel_locks(target: &str) -> EventFilterConfig {
    EventFilterConfig::Entity(EntityFilterConfig {
        id: Some(target.to_string()),
        other_id: Some(ID_PLAYER.to_string()),
        ..default()
    })
}

pub(crate) fn final_tally(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let opening = vec![
        set(VAR_ACT, num(1.0)),
        set(VAR_SURVEYED, num(0.0)),
        set(VAR_PICKET_A_DOWN, num(0.0)),
        set(VAR_PICKET_B_DOWN, num(0.0)),
        set(VAR_CAST_OFF, num(0.0)),
        set(VAR_CAST_AT, num(0.0)),
        set(VAR_EPILOGUE_AT, num(0.0)),
        set(VAR_HELLO_SAID, num(0.0)),
        set(VAR_TAUNT_SAID, num(0.0)),
        set(VAR_CLOSE_SAID, num(0.0)),
        set(VAR_SURVEY_POSTED, num(0.0)),
        set(VAR_PICKET_POSTED, num(0.0)),
        set(VAR_BREAK_POSTED, num(0.0)),
        // Seed the transition gates so their gated_once filters read a defined 0
        // before the survey / cast-off stamp them, not an undefined var (bug
        // 20260722-114541). The survey gate is seeded by its open_gate below.
        set(VAR_PICKET_GATE, num(0.0)),
        set(VAR_BREAK_GATE, num(0.0)),
        spawn(player_ship()),
        spawn(claim_anchor(&asteroid_texture)),
        spawn(anchorage_wreck(
            ID_WRECK_BOW,
            "Anchorage Bow",
            WRECK_BOW_POS,
            8.0,
            Some(WRECK_SURVEY_SIGNATURE),
            &asteroid_texture,
        )),
        spawn(anchorage_wreck(
            ID_WRECK_STERN,
            "Anchorage Stern",
            WRECK_STERN_POS,
            6.5,
            None,
            &asteroid_texture,
        )),
        spawn(picket(ID_PICKET_A, PICKET_A_SPAWN)),
        spawn(picket(ID_PICKET_B, PICKET_B_SPAWN)),
        claim_belt(&asteroid_texture),
        // Pacing pass (task 20260722-092421): the survey objective posts a beat
        // after this dispatch (the gated_once handler below), not the same frame.
        story(
            BELT_RELAY,
            "The raiders' burn traces to a dead claim: a cracked megahauler \
             berthed deep in a planetoid's pull. Confirm what's hiding there.",
        ),
        // Reveal-then-instruct: "confirm what's hiding there" sets up, the
        // objective explains the travel-lock mechanic - a mid gap (review
        // 20260722-163718). The anchorage marker is already up (below).
        open_gate(VAR_SURVEY_GATE, MID_GAP),
        mark(ID_WRECK_BOW, "ANCHORAGE"),
    ];

    let events = vec![
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: opening,
        },
        // The survey objective posts a beat after the opening dispatch (pacing
        // pass), while the approach is live.
        pacing::gated_once(
            VAR_SURVEY_POSTED,
            VAR_SURVEY_GATE,
            vec![eq_num(VAR_ACT, 1.0)],
            vec![objective(
                OBJ_SURVEY,
                "Survey the anchorage - hold a travel lock on the wreck's bow.",
            )],
        ),
        // Halloran's sendoff, one breath after the dispatch.
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_HELLO_SAID, 0.0),
                gt_num(SCENARIO_ELAPSED_VAR, HELLO_AT),
            ],
            actions: vec![
                set(VAR_HELLO_SAID, num(1.0)),
                story(
                    CAPTAIN_HALLORAN,
                    "Whatever is berthed in that pull, pilot - the guild \
                     settles its debts. So does he.",
                ),
            ],
        },
        // The SURVEY: the travel lock lands on the bow. OnTravelLock
        // recurs every 5s while held, so the one-shot flag gates it. TWO
        // fate variants (review R1.1, the lifeline banner pattern): the
        // pickets may already be drift when the survey lands - that path
        // must not post a picket objective nothing will ever complete, nor
        // mark two dead ships.
        ScenarioEventConfig {
            name: EventConfig::OnTravelLock,
            filters: vec![
                player_travel_locks(ID_WRECK_BOW),
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_SURVEYED, 0.0),
                EventFilterConfig::Conditional(ConditionalFilterConfig::Or(
                    Box::new(eq_num(VAR_PICKET_A_DOWN, 0.0)),
                    Box::new(eq_num(VAR_PICKET_B_DOWN, 0.0)),
                )),
            ],
            actions: vec![
                set(VAR_SURVEYED, num(1.0)),
                complete(OBJ_SURVEY),
                unmark(ID_WRECK_BOW),
                story(
                    BELT_RELAY,
                    "Confirmed: the Final Tally, berthed hot behind the \
                     wreck. Two pickets riding the well.",
                ),
                // The confirm line reveals the pickets (already on-screen
                // orbiting), so the reveal is short - a mid gap lands "break the
                // picket" snappier without stepping on the line (review
                // 20260722-163718).
                mark_clock(VAR_PICKET_GATE, MID_GAP),
            ],
        },
        // The picket objective, a beat after the survey confirm. Guarded on at
        // least one picket still live: if BOTH die inside the beat, the objective
        // never posts (the pickets-down beat below drives on), so nothing is left
        // pointing at dead ships. The gate is only stamped on the pickets-live
        // survey path, so this cannot fire on the already-drift variant.
        pacing::gated_once(
            VAR_PICKET_POSTED,
            VAR_PICKET_GATE,
            vec![EventFilterConfig::Conditional(ConditionalFilterConfig::Or(
                Box::new(eq_num(VAR_PICKET_A_DOWN, 0.0)),
                Box::new(eq_num(VAR_PICKET_B_DOWN, 0.0)),
            ))],
            vec![
                objective(OBJ_PICKET, "Break the orbital picket."),
                mark(ID_PICKET_A, "PICKET"),
                mark(ID_PICKET_B, "PICKET"),
            ],
        ),
        ScenarioEventConfig {
            name: EventConfig::OnTravelLock,
            filters: vec![
                player_travel_locks(ID_WRECK_BOW),
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_SURVEYED, 0.0),
                eq_num(VAR_PICKET_A_DOWN, 1.0),
                eq_num(VAR_PICKET_B_DOWN, 1.0),
            ],
            actions: vec![
                set(VAR_SURVEYED, num(1.0)),
                complete(OBJ_SURVEY),
                unmark(ID_WRECK_BOW),
                story(
                    BELT_RELAY,
                    "Confirmed: the Final Tally, berthed hot behind the \
                     wreck - and its pickets are already drift.",
                ),
            ],
        },
        // Picket kill flags (unconditional, one handler each).
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_PICKET_A)],
            actions: vec![set(VAR_PICKET_A_DOWN, num(1.0)), unmark(ID_PICKET_A)],
        },
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_PICKET_B)],
            actions: vec![set(VAR_PICKET_B_DOWN, num(1.0)), unmark(ID_PICKET_B)],
        },
        // Both pickets down: the Tallyman's last taunt, and the cast-off
        // clock starts (pickets-down + a breathe). Flag-gated, not
        // act-sequenced, so a pre-survey picket kill cannot deadlock -
        // the cast-off below still waits for the survey.
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_TAUNT_SAID, 0.0),
                eq_num(VAR_PICKET_A_DOWN, 1.0),
                eq_num(VAR_PICKET_B_DOWN, 1.0),
            ],
            actions: vec![
                set(VAR_TAUNT_SAID, num(1.0)),
                mark_clock(VAR_CAST_AT, CAST_OFF_DELAY),
                complete(OBJ_PICKET),
                story(
                    TALLYMAN,
                    "You counted my pickets, pilot. Now count the tubes on \
                     my flagship.",
                ),
            ],
        },
        // The CAST-OFF: survey done, pickets down, the breathe elapsed -
        // the Final Tally and its escort emerge from behind the anchorage.
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_CAST_OFF, 0.0),
                eq_num(VAR_SURVEYED, 1.0),
                eq_num(VAR_TAUNT_SAID, 1.0),
                clock_past(VAR_CAST_AT),
            ],
            actions: vec![
                set(VAR_CAST_OFF, num(1.0)),
                // Pacing pass (task 20260722-092421): the flagship and its gold
                // marker appear with this reveal line; the break objective posts
                // a beat later (the gated_once below), not the same frame.
                story(
                    BELT_RELAY,
                    "Capital burn off the anchorage - tubes open. That's \
                     the flagship.",
                ),
                spawn(flagship()),
                spawn(escort()),
                // Threat reveal (the capital ship emerges): full absorb beat -
                // the flagship's approach IS the peak-fight framing (review
                // 20260722-163718). The marker is set with the reveal (below).
                mark_clock(VAR_BREAK_GATE, REVEAL_GAP),
                mark(ID_FLAGSHIP, "FINAL TALLY"),
            ],
        },
        // The break objective, a beat after the cast-off reveal. Gated on the
        // live act so a fast kill (which locks act 4) cannot post a stale
        // objective under the epilogue.
        pacing::gated_once(
            VAR_BREAK_POSTED,
            VAR_BREAK_GATE,
            vec![eq_num(VAR_ACT, 1.0)],
            vec![objective(OBJ_BREAK, "Break the Final Tally.")],
        ),
        // The KILL: the epilogue opens. Act 4 locks the win (a post-kill
        // player death declares nothing; the escort's fate is its own -
        // it runs, narratively). The confirm line fires now; the close
        // and the banner ride the epilogue clock.
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_FLAGSHIP), eq_num(VAR_ACT, 1.0)],
            actions: vec![
                set(VAR_ACT, num(4.0)),
                mark_clock(VAR_EPILOGUE_AT, 0.0),
                complete(OBJ_BREAK),
                unmark(ID_FLAGSHIP),
                story(
                    BELT_RELAY,
                    "The Final Tally is breaking up. The claim is going dark.",
                ),
            ],
        },
        // Epilogue close line, +4s.
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![
                eq_num(VAR_ACT, 4.0),
                eq_num(VAR_CLOSE_SAID, 0.0),
                EventFilterConfig::Expression(ExpressionFilterConfig(
                    VariableConditionNode::new_greater_than(
                        var(SCENARIO_ELAPSED_VAR),
                        VariableExpressionNode::new_add(
                            VariableTermNode::Factor(VariableFactorNode::new_name(VAR_EPILOGUE_AT)),
                            VariableExpressionNode::new_term(VariableTermNode::Factor(
                                VariableFactorNode::Literal(VariableLiteral::Number(
                                    CLOSE_LINE_AFTER,
                                )),
                            )),
                        ),
                    ),
                )),
            ],
            actions: vec![
                set(VAR_CLOSE_SAID, num(1.0)),
                story(
                    CAPTAIN_HALLORAN,
                    "Quota's settled, pilot. The guild will not forget \
                     whose guns held the line.",
                ),
            ],
        },
        // The banner, +9s: the campaign completes - by design, with
        // nothing queued (the banner says so).
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![
                eq_num(VAR_ACT, 4.0),
                EventFilterConfig::Expression(ExpressionFilterConfig(
                    VariableConditionNode::new_greater_than(
                        var(SCENARIO_ELAPSED_VAR),
                        VariableExpressionNode::new_add(
                            VariableTermNode::Factor(VariableFactorNode::new_name(VAR_EPILOGUE_AT)),
                            VariableExpressionNode::new_term(VariableTermNode::Factor(
                                VariableFactorNode::Literal(VariableLiteral::Number(BANNER_AFTER)),
                            )),
                        ),
                    ),
                )),
            ],
            actions: vec![
                set(VAR_ACT, num(2.0)),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Victory,
                    "The claim is quiet. The Tallyman's ledger is closed, \
                     his flagship is drift, and the belt's lanes are open. \
                     End of the base campaign - for now.",
                )),
            ],
        },
        // Lose: the player dies while the fight is LIVE (act 1 only - the
        // epilogue's act 4 locks the win; terminal act 3 closes every gate
        // per the ledger lesson). Retry THIS scenario.
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_PLAYER), eq_num(VAR_ACT, 1.0)],
            actions: vec![
                set(VAR_ACT, num(3.0)),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "The claim keeps its secret, and the Tallyman keeps \
                     the belt.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: FINAL_TALLY_SCENARIO_ID.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
    ];

    ScenarioConfig {
        id: FINAL_TALLY_SCENARIO_ID.to_string(),
        name: "Final Tally".to_string(),
        description: "The trace ends at the gang's claim: survey the \
                      anchorage, break the orbital picket, and finish the \
                      Final Tally in its own gravity well. Chapter three of \
                      the base storyline, part two."
            .to_string(),
        cubemap,
        // A mid-story continuation reached from Lifeline's victory chain
        // (the Broadside-gunship precedent), with the placeholder thumbnail
        // (real art: task 20260715-220011).
        thumbnail: Some(AssetRef::from("self://banner.png")),
        hidden: true,
        menu_backdrop: false,
        events,
    }
}
