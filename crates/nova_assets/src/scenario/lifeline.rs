//! Lifeline - chapter three, part one: the convoy defense (task
//! 20260721-160957, spike tasks/20260721-155249/SPIKE.md).
//!
//! The Rust Tally was the gang's muscle, not its head. Breaking it provokes
//! the Tallyman: his raiders hit the belt's supply convoy in revenge, and
//! the player screens it until the relief wing arrives. A NEW encounter
//! shape on every axis: the objective is PROTECT (not kill-all), the
//! composition is light waves, and the pressure is a clock - a relief
//! countdown on the HUD (the HudReadout surface's first campaign use).
//!
//! The convoy is the ch3-mechanisms discovery in shipped form (task
//! 20260721-160906): STALLED haulers - `controller: None` (station-keeping,
//! `SpaceshipRootMarker` still applies) with `allegiance: Some(Player)`, so
//! enemy AI genuinely targets them over the relation model while they
//! cannot chase anyone. Raiders spawning nearer to the convoy than to the
//! player draw fire onto the convoy (nearest-hostile rule, pinned by
//! `ally_relation_tests`), which is the whole mission.
//!
//! Waves stage on the scenario clock AND the previous wave's kill flags, so
//! a slow player is never buried under stacked waves (the schedule
//! self-balances: late clears push later waves toward the relief bell). Win:
//! the relief timer expires with at least one hauler alive (the raiders
//! scatter), or the last wave dies early. Lose: the player dies, or BOTH
//! haulers die. Every raider spawn is telegraphed per the beat sheet - a
//! warning line, a far spawn (outside the light turret's threat envelope of
//! every friendly anchor), and an `engage_delay` grace.
//!
//! Victory chains (lingering) into the finale: the relief wing traced the
//! raiders' burn to the claim, and `final_tally` (task 20260721-161020)
//! waits behind Continue.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::{
    cast::{BELT_RELAY, CAPTAIN_HALLORAN, TALLYMAN},
    craft::{self, ShipGrade},
    shakedown::{
        complete, destroyed, eq_num, gt_num, lt_num, mark, num, objective, set, spawn, story,
        unmark, var,
    },
    SCATTER_SEED,
};

pub(crate) const LIFELINE_SCENARIO_ID: &str = "lifeline";

const ID_PLAYER: &str = "player_spaceship";
const ID_QUEEN: &str = "hauler_queen";
const ID_MERIDIAN: &str = "hauler_meridian";

const OBJ_SCREEN: &str = "screen_convoy";

/// Story act: 1 = the defense is live, 2 = won, 3 = lost. Terminal acts are
/// distinct so the win gate (`act == 1`) can never fire after the
/// both-haulers loss (which sets 3), and vice versa.
const VAR_ACT: &str = "act";
/// Per-hauler death flags (0/1), raised by the beacon-dark beats. Both up =
/// the loss; either up = the win banner's half-convoy variant.
const VAR_QUEEN_DOWN: &str = "queen_down";
const VAR_MERIDIAN_DOWN: &str = "meridian_down";
/// One-shot flags: each wave has spawned (0/1). The spawn gates check the
/// clock AND the previous wave's kill flags AND this flag, so a gate fires
/// exactly once (the standard one-shot idiom).
const VAR_W1_UP: &str = "w1_up";
const VAR_W2_UP: &str = "w2_up";
const VAR_W3_UP: &str = "w3_up";
/// Per-raider kill flags (0/1) - the broadside pattern: independent flags,
/// no counter arithmetic, so a double OnDestroyed cannot overshoot a gate.
const VAR_R1A_DOWN: &str = "r1a_down";
const VAR_R1B_DOWN: &str = "r1b_down";
const VAR_R2A_DOWN: &str = "r2a_down";
const VAR_R2B_DOWN: &str = "r2b_down";
const VAR_R2C_DOWN: &str = "r2c_down";
const VAR_R3A_DOWN: &str = "r3a_down";
const VAR_R3B_DOWN: &str = "r3b_down";
/// One-shot flags for the paced comms beats (the greeting and the breathe
/// lines), so the recurring OnUpdate gates fire their line exactly once.
const VAR_HELLO_SAID: &str = "hello_said";
const VAR_W1_CLEAR_SAID: &str = "w1_clear_said";
const VAR_W2_CLEAR_SAID: &str = "w2_clear_said";
/// The HUD countdown: `RELIEF_SECS - scenario_elapsed`, recomputed every
/// frame while the act is live, displayed by the `relief` readout in Time
/// format. Only writing `scenario_elapsed` itself is linted; a DERIVED
/// countdown is the documented pattern.
const VAR_RELIEF_REMAINING: &str = "relief_remaining";

/// The relief bell: the defense's fixed length. The wave schedule leaves
/// the last wave at least ~50s of life even on a slow clear, and the win
/// fires at the bell regardless of live raiders (they scatter).
const RELIEF_SECS: f64 = 240.0;
/// Wave clock gates (seconds of scenario_elapsed). Each ALSO requires the
/// previous wave cleared, so these are "no earlier than" marks.
const W1_AT: f64 = 25.0;
const W2_AT: f64 = 95.0;
const W3_AT: f64 = 165.0;
/// The greeting line's clock gate: one breath after the opening dispatch.
const HELLO_AT: f64 = 9.0;

/// Player spawn, looking down the lane toward the stalled convoy.
const PLAYER_SPAWN: Vec3 = Vec3::new(0.0, 0.0, 40.0);
/// The stalled convoy, mid-lane at the transfer stop.
const QUEEN_POS: Vec3 = Vec3::new(0.0, 5.0, -420.0);
const MERIDIAN_POS: Vec3 = Vec3::new(70.0, -12.0, -520.0);
/// Raider spawn points: deep field past the convoy, all >= 700u from the
/// player spawn AND both haulers - outside the light turret's threat
/// envelope of every friendly anchor, so the balance audit stays clean by
/// construction (the corvette envelope is the larger one; W3 spawns
/// deepest). Pinned by `lifeline_convoy.rs`.
const W1_SPAWNS: [Vec3; 2] = [
    Vec3::new(150.0, 25.0, -1250.0),
    Vec3::new(90.0, -15.0, -1310.0),
];
const W2_SPAWNS: [Vec3; 3] = [
    Vec3::new(-210.0, 30.0, -1300.0),
    Vec3::new(-270.0, -25.0, -1360.0),
    Vec3::new(250.0, 45.0, -1340.0),
];
const W3_SPAWNS: [Vec3; 2] = [
    Vec3::new(0.0, 35.0, -1400.0),
    Vec3::new(80.0, -20.0, -1450.0),
];

/// Ships spawn -Z forward; raiders come from deep -Z toward the convoy, so
/// they are authored with the same about-face as broadside's combatants.
fn facing_the_lane() -> Quat {
    Quat::from_rotation_y(std::f32::consts::PI)
}

/// The player's chapter-three ship: unchanged from Broadside (the racer
/// with the better turrets, finite ammo, no torpedo bay, RCS gated).
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

/// A stalled convoy hauler: the cargoa hull, NO controller (drives are
/// cold - it station-keeps and cannot chase), PLAYER allegiance so raider
/// AI genuinely hunts it. The defend mission in two fields.
fn convoy_hauler(id: &str, name: &str, position: Vec3, yaw: f32) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::from_rotation_y(yaw),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Player),
            sections: craft::cargoa_sections(),
        }),
    }
}

/// A raider: the scavenger-grade racer, leashed to the convoy fight,
/// telegraphed with an arrival grace. `grade` lifts W3's corvette to full
/// player-grade turrets - the "real guns" the Tallyman promises.
fn raider(id: &str, spawn_pos: Vec3, grade: ShipGrade, engage_delay: f32) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: match grade {
                ShipGrade::Player => "Raider Corvette".to_string(),
                ShipGrade::Enemy => "Tally Raider".to_string(),
            },
            position: spawn_pos,
            rotation: facing_the_lane(),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig {
                // The run-in: patrol from the spawn to the convoy's lane.
                patrol: vec![spawn_pos, QUEEN_POS + Vec3::new(0.0, 30.0, 80.0)],
                leash: Some(520.0),
                engage_delay: Some(engage_delay),
                ..Default::default()
            }),
            allegiance: None,
            sections: craft::racer_sections(grade, vec![]),
        }),
    }
}

/// A wave-spawn beat: the warning line, the ships, their markers. One
/// comms line per beat (the beat sheet); every ship telegraphed.
fn wave_beat(
    up_flag: &str,
    line_speaker: &str,
    line: &str,
    ships: Vec<(ScenarioObjectConfig, &str)>,
) -> Vec<EventActionConfig> {
    let mut actions = vec![set(up_flag, num(1.0)), story(line_speaker, line)];
    for (ship, label) in ships {
        let id = ship.base.id.clone();
        actions.push(spawn(ship));
        actions.push(mark(&id, label));
    }
    actions
}

/// A raider-kill beat: raise the flag, drop the marker.
fn kill_flag(id: &str, flag: &str) -> ScenarioEventConfig {
    ScenarioEventConfig {
        name: EventConfig::OnDestroyed,
        filters: vec![destroyed(id)],
        actions: vec![set(flag, num(1.0)), unmark(id)],
    }
}

/// The lane's hard cover: invulnerable boulders staggered along the convoy
/// stretch - cover exists near the fight but does not enclose it (a lane,
/// not the Broadside bowl). Same two-tier scheme as Broadside.
fn lane_boulders(asteroid_texture: &AssetRef<Image>) -> Vec<ScenarioObjectConfig> {
    let boulder = |id: &str, position: Vec3, radius: f32| ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: "Lane Boulder".to_string(),
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
            lock_signature: None,
        }),
    };
    vec![
        boulder("lane_boulder_1", Vec3::new(90.0, 18.0, -360.0), 4.0),
        boulder("lane_boulder_2", Vec3::new(-95.0, -12.0, -470.0), 4.5),
        boulder("lane_boulder_3", Vec3::new(35.0, 28.0, -580.0), 5.0),
        boulder("lane_boulder_4", Vec3::new(-70.0, 22.0, -300.0), 3.5),
    ]
}

/// Light destructible chaff along the lane - sparser than Broadside's bowl
/// (the lane reads open), same deterministic seed discipline.
fn lane_chaff(asteroid_texture: &AssetRef<Image>) -> EventActionConfig {
    EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "lane_rock_".to_string(),
        count: 14,
        seed: SCATTER_SEED,
        region: ScatterRegion::Box {
            min: Vec3::new(-190.0, -40.0, -560.0),
            max: Vec3::new(190.0, 40.0, -160.0),
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "lane_rock_".to_string(),
                name: "Lane Rock".to_string(),
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

/// A transfer-stop beacon framing the lane.
fn lane_beacon(id: &str, label: &str, position: Vec3) -> ScenarioObjectConfig {
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
            color: Color::srgb(1.0, 0.75, 0.3),
            area_radius: None,
            lock_signature: None,
        }),
    }
}

/// A one-shot clock-gated comms beat: fires its line once when the clock
/// passes `at` and the act is still live.
fn paced_line(
    said_flag: &str,
    at: f64,
    speaker: &str,
    line: &str,
    extra_filters: Vec<EventFilterConfig>,
) -> ScenarioEventConfig {
    let mut filters = vec![
        eq_num(VAR_ACT, 1.0),
        eq_num(said_flag, 0.0),
        gt_num(SCENARIO_ELAPSED_VAR, at),
    ];
    filters.extend(extra_filters);
    ScenarioEventConfig {
        name: EventConfig::OnUpdate,
        filters,
        actions: vec![set(said_flag, num(1.0)), story(speaker, line)],
    }
}

/// A Victory beat: complete the objective, set the terminal act, show the
/// banner - and chain (lingering) into the finale: the relief wing traced
/// the raiders' burn, and Continue follows it (task 20260721-161020).
fn victory(message: &str, extra_filters: Vec<EventFilterConfig>) -> ScenarioEventConfig {
    let mut filters = vec![eq_num(VAR_ACT, 1.0)];
    filters.extend(extra_filters);
    ScenarioEventConfig {
        name: EventConfig::OnUpdate,
        filters,
        actions: vec![
            set(VAR_ACT, num(2.0)),
            complete(OBJ_SCREEN),
            EventActionConfig::Outcome(OutcomeActionConfig::new(
                ScenarioOutcomeKind::Victory,
                message,
            )),
            EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: super::final_tally::FINAL_TALLY_SCENARIO_ID.to_string(),
                linger: true,
                delay: None,
            }),
        ],
    }
}

pub(crate) fn lifeline(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    // --- OnStart: the stage, the state, the countdown, the dispatch. ---
    let mut opening = vec![
        set(VAR_ACT, num(1.0)),
        set(VAR_QUEEN_DOWN, num(0.0)),
        set(VAR_MERIDIAN_DOWN, num(0.0)),
        set(VAR_W1_UP, num(0.0)),
        set(VAR_W2_UP, num(0.0)),
        set(VAR_W3_UP, num(0.0)),
        set(VAR_R1A_DOWN, num(0.0)),
        set(VAR_R1B_DOWN, num(0.0)),
        set(VAR_R2A_DOWN, num(0.0)),
        set(VAR_R2B_DOWN, num(0.0)),
        set(VAR_R2C_DOWN, num(0.0)),
        set(VAR_R3A_DOWN, num(0.0)),
        set(VAR_R3B_DOWN, num(0.0)),
        set(VAR_HELLO_SAID, num(0.0)),
        set(VAR_W1_CLEAR_SAID, num(0.0)),
        set(VAR_W2_CLEAR_SAID, num(0.0)),
        set(VAR_RELIEF_REMAINING, num(RELIEF_SECS)),
        spawn(player_ship()),
        spawn(convoy_hauler(
            ID_QUEEN,
            "Hauler Ceres Queen",
            QUEEN_POS,
            0.5,
        )),
        spawn(convoy_hauler(
            ID_MERIDIAN,
            "Hauler Long Meridian",
            MERIDIAN_POS,
            -0.4,
        )),
        spawn(lane_beacon(
            "beacon_transfer",
            "TRANSFER STOP",
            Vec3::new(35.0, -2.0, -470.0),
        )),
        spawn(lane_beacon(
            "beacon_lane",
            "LANE MARKER",
            Vec3::new(-10.0, 12.0, -140.0),
        )),
        lane_chaff(&asteroid_texture),
    ];
    opening.extend(lane_boulders(&asteroid_texture).into_iter().map(spawn));
    opening.extend([
        story(
            BELT_RELAY,
            "Relief wing is spooled and burning your way - four minutes \
             out. The convoy holds the lane until they arrive.",
        ),
        objective(
            OBJ_SCREEN,
            "Keep the convoy alive until the relief wing arrives.",
        ),
        mark(ID_QUEEN, "CERES QUEEN"),
        mark(ID_MERIDIAN, "LONG MERIDIAN"),
        EventActionConfig::HudReadout(HudReadoutActionConfig {
            slot: "relief".to_string(),
            variable: VAR_RELIEF_REMAINING.to_string(),
            format: HudReadoutFormat::Time,
            label: Some("RELIEF".to_string()),
            visible: true,
        }),
    ]);

    let events = vec![
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: opening,
        },
        // The countdown, recomputed every live frame: RELIEF_SECS - clock.
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![eq_num(VAR_ACT, 1.0)],
            actions: vec![set(
                VAR_RELIEF_REMAINING,
                VariableExpressionNode::new_subtract(
                    VariableTermNode::Factor(VariableFactorNode::Literal(VariableLiteral::Number(
                        RELIEF_SECS,
                    ))),
                    var(SCENARIO_ELAPSED_VAR),
                ),
            )],
        },
        // Halloran's greeting, one breath after the dispatch line.
        paced_line(
            VAR_HELLO_SAID,
            HELLO_AT,
            CAPTAIN_HALLORAN,
            "Halloran here - the Queen's guild runs this line. Drives are \
             cold on a transfer fault; we could not run if we wanted to.",
            vec![],
        ),
        // --- Wave one: two raiders, one vector. ---
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_W1_UP, 0.0),
                gt_num(SCENARIO_ELAPSED_VAR, W1_AT),
            ],
            actions: wave_beat(
                VAR_W1_UP,
                BELT_RELAY,
                "Two contacts off the shelf, one vector, coming down the lane.",
                vec![
                    (
                        raider("raider_1a", W1_SPAWNS[0], ShipGrade::Enemy, 8.0),
                        "RAIDER",
                    ),
                    (
                        raider("raider_1b", W1_SPAWNS[1], ShipGrade::Enemy, 8.0),
                        "RAIDER",
                    ),
                ],
            ),
        },
        kill_flag("raider_1a", VAR_R1A_DOWN),
        kill_flag("raider_1b", VAR_R1B_DOWN),
        // Breathe: wave one cleared, before wave two shows.
        paced_line(
            VAR_W1_CLEAR_SAID,
            0.0,
            CAPTAIN_HALLORAN,
            "Clean shooting. Watch the dark - the Tallyman does not send \
             twice the same way.",
            vec![
                eq_num(VAR_R1A_DOWN, 1.0),
                eq_num(VAR_R1B_DOWN, 1.0),
                eq_num(VAR_W2_UP, 0.0),
            ],
        ),
        // --- Wave two: three raiders, split vectors (one flanker). ---
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_W2_UP, 0.0),
                gt_num(SCENARIO_ELAPSED_VAR, W2_AT),
                eq_num(VAR_R1A_DOWN, 1.0),
                eq_num(VAR_R1B_DOWN, 1.0),
            ],
            actions: wave_beat(
                VAR_W2_UP,
                BELT_RELAY,
                "Three more - they split the lane, one swinging wide onto \
                 your flank.",
                vec![
                    (
                        raider("raider_2a", W2_SPAWNS[0], ShipGrade::Enemy, 8.0),
                        "RAIDER",
                    ),
                    (
                        raider("raider_2b", W2_SPAWNS[1], ShipGrade::Enemy, 8.0),
                        "RAIDER",
                    ),
                    (
                        raider("raider_2c", W2_SPAWNS[2], ShipGrade::Enemy, 8.0),
                        "RAIDER",
                    ),
                ],
            ),
        },
        kill_flag("raider_2a", VAR_R2A_DOWN),
        kill_flag("raider_2b", VAR_R2B_DOWN),
        kill_flag("raider_2c", VAR_R2C_DOWN),
        // Breathe: the Tallyman speaks for himself.
        paced_line(
            VAR_W2_CLEAR_SAID,
            0.0,
            TALLYMAN,
            "You are burning my margins, pilot. The next crew brings real \
             guns.",
            vec![
                eq_num(VAR_R2A_DOWN, 1.0),
                eq_num(VAR_R2B_DOWN, 1.0),
                eq_num(VAR_R2C_DOWN, 1.0),
                eq_num(VAR_W3_UP, 0.0),
            ],
        ),
        // --- Wave three: the full-gun corvette and its escort. ---
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_W3_UP, 0.0),
                gt_num(SCENARIO_ELAPSED_VAR, W3_AT),
                eq_num(VAR_R2A_DOWN, 1.0),
                eq_num(VAR_R2B_DOWN, 1.0),
                eq_num(VAR_R2C_DOWN, 1.0),
            ],
            actions: wave_beat(
                VAR_W3_UP,
                BELT_RELAY,
                "Last push: a full-gun corvette with an escort. Hold them off.",
                vec![
                    (
                        raider("raider_3a", W3_SPAWNS[0], ShipGrade::Player, 8.0),
                        "CORVETTE",
                    ),
                    (
                        raider("raider_3b", W3_SPAWNS[1], ShipGrade::Enemy, 8.0),
                        "RAIDER",
                    ),
                ],
            ),
        },
        kill_flag("raider_3a", VAR_R3A_DOWN),
        kill_flag("raider_3b", VAR_R3B_DOWN),
        // --- The convoy's fate. Each hauler death raises its flag and gets
        // its beacon-dark line; BOTH down is the loss (act 3 closes the
        // win gate before the defeat shows).
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_QUEEN), lt_num(VAR_ACT, 2.0)],
            actions: vec![
                set(VAR_QUEEN_DOWN, num(1.0)),
                unmark(ID_QUEEN),
                story(BELT_RELAY, "The Ceres Queen's beacon just went dark."),
            ],
        },
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_MERIDIAN), lt_num(VAR_ACT, 2.0)],
            actions: vec![
                set(VAR_MERIDIAN_DOWN, num(1.0)),
                unmark(ID_MERIDIAN),
                story(BELT_RELAY, "The Long Meridian's beacon just went dark."),
            ],
        },
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![
                eq_num(VAR_ACT, 1.0),
                eq_num(VAR_QUEEN_DOWN, 1.0),
                eq_num(VAR_MERIDIAN_DOWN, 1.0),
            ],
            actions: vec![
                set(VAR_ACT, num(3.0)),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "Both beacons dark. The lane belongs to the Tallyman now.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: LIFELINE_SCENARIO_ID.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
        // --- The wins. Four gated variants: (relief bell | early clear) x
        // (convoy whole | half lost). All mutually exclusive: act==1 plus
        // the bell/early and fate filters; the first to fire sets act=2.
        victory(
            "The relief wing drops out of the burn, guns hot - and the \
             raiders scatter. The convoy is whole - and the wing traced \
             the raiders' burn back to a claim deep on the shelf.",
            vec![
                gt_num(SCENARIO_ELAPSED_VAR, RELIEF_SECS),
                eq_num(VAR_QUEEN_DOWN, 0.0),
                eq_num(VAR_MERIDIAN_DOWN, 0.0),
            ],
        ),
        victory(
            "The relief wing drops out of the burn, guns hot - and the \
             raiders scatter. Half the convoy made it - and the wing \
             traced the raiders' burn back to a claim deep on the shelf. \
             The Tallyman will answer for the other half.",
            vec![
                gt_num(SCENARIO_ELAPSED_VAR, RELIEF_SECS),
                EventFilterConfig::Conditional(ConditionalFilterConfig::Or(
                    Box::new(eq_num(VAR_QUEEN_DOWN, 1.0)),
                    Box::new(eq_num(VAR_MERIDIAN_DOWN, 1.0)),
                )),
            ],
        ),
        victory(
            "The last raider breaks apart before the relief wing even \
             arrives. The convoy is whole - and the last burst off the \
             raider traced back to a claim deep on the shelf.",
            vec![
                eq_num(VAR_W3_UP, 1.0),
                eq_num(VAR_R3A_DOWN, 1.0),
                eq_num(VAR_R3B_DOWN, 1.0),
                eq_num(VAR_QUEEN_DOWN, 0.0),
                eq_num(VAR_MERIDIAN_DOWN, 0.0),
            ],
        ),
        victory(
            "The last raider breaks apart before the relief wing even \
             arrives. Half the convoy made it - and the last burst off the \
             raider traced back to a claim deep on the shelf. The Tallyman \
             will answer for the rest.",
            vec![
                eq_num(VAR_W3_UP, 1.0),
                eq_num(VAR_R3A_DOWN, 1.0),
                eq_num(VAR_R3B_DOWN, 1.0),
                EventFilterConfig::Conditional(ConditionalFilterConfig::Or(
                    Box::new(eq_num(VAR_QUEEN_DOWN, 1.0)),
                    Box::new(eq_num(VAR_MERIDIAN_DOWN, 1.0)),
                )),
            ],
        ),
        // --- Lose: the player dies on a live act; retry THIS scenario.
        // Terminal act FIRST (review R1.1): CurrentOutcome is
        // last-write-wins, and the bell Victory's clock gate is true every
        // pulse - without act 3 here, a mutual-destruction trade (the
        // player's blast killing the last raider just after the player
        // dies) could overwrite this Defeat with a Victory over the queued
        // retry. Act 3 closes every win gate and stops the countdown.
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![destroyed(ID_PLAYER), eq_num(VAR_ACT, 1.0)],
            actions: vec![
                set(VAR_ACT, num(3.0)),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "The convoy watches your wreck drift down the lane the \
                     raiders now own.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: LIFELINE_SCENARIO_ID.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
    ];

    ScenarioConfig {
        id: LIFELINE_SCENARIO_ID.to_string(),
        name: "Lifeline".to_string(),
        description: "The Tallyman hits back where it hurts: screen a \
                      stalled hauler convoy against raider waves until the \
                      relief wing arrives. Chapter three of the base \
                      storyline, part one."
            .to_string(),
        cubemap,
        // The chapter head: picker-visible with the placeholder thumbnail
        // (real per-scenario art: task 20260715-220011), like Broadside.
        thumbnail: Some(AssetRef::from("self://banner.png")),
        hidden: false,
        menu_backdrop: false,
        events,
    }
}
