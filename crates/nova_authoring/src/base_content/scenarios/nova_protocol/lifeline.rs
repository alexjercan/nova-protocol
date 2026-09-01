//! Lifeline - chapter three, part one: the convoy defense (spike).
//!
//! The Rust Tally was the gang's muscle, not its head. Breaking it provokes
//! the Tallyman: his raiders hit the belt's supply convoy in revenge, and
//! the player screens it until the relief wing arrives. A NEW encounter
//! shape on every axis: the objective is PROTECT (not kill-all), the
//! composition is light waves, and the pressure is a clock - a relief
//! countdown on the HUD (the HudReadout surface's first campaign use).
//!
//! The convoy is the ch3-mechanisms discovery in shipped form: LOITERING
//! couriers on the unarmed racer hull (hull cubes, two thrusters, a
//! controller, NO weapon) flying a slow patrol loop through the belt, tagged `AINonCombatant`
//! at spawn so they never chase or shoot. Their `allegiance: Some(Player)`
//! keeps enemy AI targeting them over the relation model, so raiders spawning
//! nearer to the convoy than to the player draw fire onto it (nearest-hostile
//! rule, pinned by `ally_relation_tests`), which is the whole mission - the
//! player screens a convoy that flies around and holds the belt instead of
//! drifting off.
//!
//! Waves stage on the scenario clock AND the previous wave's kill flags, so
//! a slow player is never buried under stacked waves (the schedule
//! self-balances: late clears push later waves toward the relief bell). Win:
//! the relief timer expires with at least one convoy ship alive (the raiders
//! scatter), or the last wave dies early. Lose: the player dies, or BOTH
//! convoy ships die. Every raider spawn is telegraphed per the beat sheet - a
//! warning line, a far spawn (outside the light turret's threat envelope of
//! every friendly anchor), and an `engage_delay` grace.
//!
//! Victory chains (lingering) into the finale: the relief wing traced the
//! raiders' burn to the claim, and `final_tally` waits behind Continue.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use super::{
    cast::{BELT_RELAY, CAPTAIN_HALLORAN, TALLYMAN},
    pacing::{self, REVEAL_GAP},
    ships, SCATTER_SEED, SCENARIO_ELAPSED_VAR,
};
use crate::scenario_helpers::prelude::*;

pub(crate) const LIFELINE_SCENARIO_ID: &str = "lifeline";

const ID_PLAYER: &str = "player_spaceship";
const ID_QUEEN: &str = "hauler_queen";
const ID_MERIDIAN: &str = "hauler_meridian";

const OBJ_SCREEN: &str = "screen_convoy";

/// Story act: 1 = the defense is live, 2 = won, 3 = lost. Terminal acts are
/// distinct so the win gate (`act == 1`) can never fire after the
/// both-ships loss (which sets 3), and vice versa.
const VAR_ACT: &str = "act";
/// The outro act: the defense is decided and the win locked, but the banner
/// has not landed. It sits OUTSIDE the defeat gates (`act == 1`), so a death
/// during the outro beats cannot overwrite the win.
const ACT_OUTRO: f64 = 4.0;
const ACT_WON: f64 = 2.0;
/// Per-ship death flags (0/1), raised by the beacon-dark beats. Both up =
/// the loss; either up = the win banner's half-convoy variant.
const VAR_QUEEN_DOWN: &str = "queen_down";
const VAR_MERIDIAN_DOWN: &str = "meridian_down";
/// Signals: this wave is on the board (0/1). The breathe line before a wave
/// waits on its flag still being 0, and the early-clear wins wait on wave
/// three being up. The spawn gates themselves retire on their own.
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
/// The opening chain's sequence key: the dispatch line, then the objective a
/// beat later (never the same frame), then Halloran's greeting. Three beats the
/// engine walks, where each used to be a handler of its own carrying a clock
/// mark and an act guard.
const SEQ_OPENING: &str = "opening";
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
/// The convoy's holding stations, mid-lane at the transfer stop. The convoy
/// ships LOITER around these: unarmed non-combatant AI ships flying a slow loop
/// through the belt so they read as alive and hold their ground under fire
/// instead of drifting off when a raider shoves them. They never shoot or chase
/// (unarmed => AINonCombatant), but stay Player-aligned so the raiders still
/// hunt them and the player must screen them.
const QUEEN_POS: Vec3 = Vec3::new(0.0, 5.0, -420.0);
const MERIDIAN_POS: Vec3 = Vec3::new(70.0, -12.0, -520.0);
/// Loiter loops: legs > the ~75u patrol arrival radius (arrival_standoff 50 +
/// waypoint slack 25) so the convoy ships actually FLY the loop instead of parking
/// at the cluster, and centred on the holding stations so they stay in the belt
/// near where the player expects to defend them.
const QUEEN_LOITER: [Vec3; 3] = [
    Vec3::new(60.0, 20.0, -370.0),
    Vec3::new(-70.0, 0.0, -410.0),
    Vec3::new(10.0, -10.0, -480.0),
];
const MERIDIAN_LOITER: [Vec3; 3] = [
    Vec3::new(100.0, 0.0, -480.0),
    Vec3::new(30.0, -25.0, -540.0),
    Vec3::new(90.0, -5.0, -590.0),
];
/// Raider spawn points: deep field past the convoy, all >= 700u from the
/// player spawn AND both convoy ships - outside the light turret's threat
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

/// The player's chapter-three ship: unchanged from Broadside (the cargoa
/// corvette with the better turrets, finite ammo, no torpedo bay, RCS gated).
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
                speed_cap: None,
                infinite_ammo: false,
            }),
            allegiance: None,
            hull: ships::hull(ships::CARGOA_SHIP_ID),
            modifications: vec![ships::on_section(
                ships::FUSELAGE_SECTION_ID,
                vec![SectionModification::DisableVerb(FlightVerb::Rcs)],
            )],
        }),
    }
}

/// A loitering convoy ship: the racer hull (unarmed - hull cubes, two rear
/// thrusters, a controller), an AI driver flying `patrol` so it slow-loops the
/// belt and holds its ground under fire instead of drifting off, PLAYER
/// allegiance so raider AI genuinely hunts it. Unarmed, so nova_scenario tags
/// it `AINonCombatant` at spawn: it never targets, chases, or shoots - it just
/// flies its loop and gets defended.
fn convoy_ship(
    id: &str,
    name: &str,
    position: Vec3,
    yaw: f32,
    patrol: Vec<Vec3>,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::from_rotation_y(yaw),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig {
                patrol,
                ..Default::default()
            }),
            allegiance: Some(Allegiance::Player),
            hull: ships::hull(ships::RACER_SHIP_ID),
            ..Default::default()
        }),
    }
}

/// A raider: a cargoa corvette, leashed to the convoy fight, telegraphed with
/// an arrival grace. `ship` picks the grade: W3's corvette is the FULL
/// player-grade hull - the "real guns" the Tallyman promises - where the
/// earlier waves fly the scavenger-grade raider.
fn raider(id: &str, spawn_pos: Vec3, ship: &str, engage_delay: f32) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: match ship {
                ships::CARGOA_SHIP_ID => "Raider Corvette".to_string(),
                _ => "Tally Raider".to_string(),
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
            hull: ships::hull(ship),
            ..Default::default()
        }),
    }
}

/// A wave-spawn beat: the warning line, the ships, their markers. One
/// comms line per beat (the beat sheet); every ship telegraphed. `up_flag`
/// is the signal LATER beats read to ask whether this wave is on the board -
/// wave one has no such reader, so it raises nothing.
fn wave_beat(
    up_flag: Option<&str>,
    line_speaker: &str,
    line: &str,
    ships: Vec<(ScenarioObjectConfig, &str)>,
) -> Vec<EventActionConfig> {
    let mut actions = Vec::new();
    if let Some(flag) = up_flag {
        actions.push(set_variable(flag, number(1.0)));
    }
    actions.push(story_message(line_speaker, line));
    for (ship, label) in ships {
        let id = ship.base.id.clone();
        actions.push(spawn_object(ship));
        actions.push(attach_objective_marker(&id, label));
    }
    actions
}

/// A raider defeat beat: raise the flag, drop the marker once whether the ship
/// was neutralized or directly destroyed.
fn defeat_flag(id: &str, flag: &str) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnDefeated,
        once: true,
        filters: vec![entity(id)],
        actions: vec![set_variable(flag, number(1.0)), detach_objective_marker(id)],
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
            impact_sound: Some(AssetRef::from("self://sounds/impact_rock.wav")),
            destroy_sound: None,
            radius,
            texture: asteroid_texture.clone(),
            mass: None,
            invulnerable: true,
            seed: None,
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
                impact_sound: Some(AssetRef::from("self://sounds/impact_rock.wav")),
                destroy_sound: Some(AssetRef::from("self://sounds/destroy_rock.wav")),
                radius: 1.0,
                texture: asteroid_texture.clone(),
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((1.5, 3.5)),
        min_separation: None,
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

/// A one-shot comms beat: fires its line once the act is live, the clock has
/// passed `at` and `extra_filters` agree, then retires.
///
/// `at: 0.0` means NO clock gate, and emits none. `scenario_elapsed > 0.0` is
/// true from the first tick, so authoring it would be a filter that reads the
/// clock every frame to answer yes - which is exactly the ceremony the pacing
/// primitives exist to delete, and it hides the state gate that is doing the
/// real work behind a clock the reader has to check.
fn paced_line(
    at: f64,
    speaker: &str,
    line: &str,
    extra_filters: Vec<EventFilterConfig>,
) -> ScenarioEventConfig {
    let mut filters = vec![number_equals(VAR_ACT, 1.0)];
    if at > 0.0 {
        filters.push(number_greater_than(SCENARIO_ELAPSED_VAR, at));
    }
    filters.extend(extra_filters);
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnUpdate,
        once: true,
        filters,
        actions: vec![story_message(speaker, line)],
    }
}

/// A Victory beat: complete the objective, lock the win, and say how THIS
/// variant's defense ended. The shared outro tail (the trace into the finale,
/// then the banner) follows a few seconds later.
///
/// Note the bell variants can fire with raiders still on the board - the
/// relief wing's arrival is narrated, not simulated - so the outro plays over
/// a lane that may still be live. `ACT_OUTRO` is what makes that safe: it sits
/// outside the defeat gate, so a death during those seconds cannot overwrite
/// the win the player already earned.
fn victory(message: &str, extra_filters: Vec<EventFilterConfig>) -> ScenarioEventConfig {
    let mut filters = vec![number_equals(VAR_ACT, 1.0)];
    filters.extend(extra_filters);
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnUpdate,
        once: true,
        filters,
        actions: pacing::open_outro(
            VAR_ACT,
            ACT_OUTRO,
            outro(),
            vec![
                complete_objective(OBJ_SCREEN),
                story_message(BELT_RELAY, message),
            ],
        ),
    }
}

/// The outro chain: the trace into the finale, then the banner. All four win
/// variants start the same cursor - only one of them can ever fire.
fn outro() -> EventActionConfig {
    pacing::outro_sequence(
        VAR_ACT,
        ACT_WON,
        BELT_RELAY,
        "The wing traced the raiders' burn back to a claim deep on the \
         shelf. That is where the Tallyman counts his take.",
        "The convoy is through and the lane is open - and the raiders' burn \
         leads somewhere worth following.",
        vec![],
        Some(super::final_tally::FINAL_TALLY_SCENARIO_ID.to_string()),
    )
}

pub(crate) fn lifeline(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    // --- OnStart: the stage, the state, the countdown, the dispatch. ---
    let mut opening = vec![
        set_variable(VAR_ACT, number(1.0)),
        set_variable(VAR_QUEEN_DOWN, number(0.0)),
        set_variable(VAR_MERIDIAN_DOWN, number(0.0)),
        set_variable(VAR_W2_UP, number(0.0)),
        set_variable(VAR_W3_UP, number(0.0)),
        set_variable(VAR_R1A_DOWN, number(0.0)),
        set_variable(VAR_R1B_DOWN, number(0.0)),
        set_variable(VAR_R2A_DOWN, number(0.0)),
        set_variable(VAR_R2B_DOWN, number(0.0)),
        set_variable(VAR_R2C_DOWN, number(0.0)),
        set_variable(VAR_R3A_DOWN, number(0.0)),
        set_variable(VAR_R3B_DOWN, number(0.0)),
        set_variable(VAR_RELIEF_REMAINING, number(RELIEF_SECS)),
        spawn_object(player_ship()),
        spawn_object(convoy_ship(
            ID_QUEEN,
            "Yacht Ceres Queen",
            QUEEN_POS,
            0.5,
            QUEEN_LOITER.to_vec(),
        )),
        spawn_object(convoy_ship(
            ID_MERIDIAN,
            "Courier Long Meridian",
            MERIDIAN_POS,
            -0.4,
            MERIDIAN_LOITER.to_vec(),
        )),
        spawn_object(lane_beacon(
            "beacon_transfer",
            "TRANSFER STOP",
            Vec3::new(35.0, -2.0, -470.0),
        )),
        spawn_object(lane_beacon(
            "beacon_lane",
            "LANE MARKER",
            Vec3::new(-10.0, 12.0, -140.0),
        )),
        lane_chaff(&asteroid_texture),
    ];
    opening.extend(
        lane_boulders(&asteroid_texture)
            .into_iter()
            .map(spawn_object),
    );
    opening.extend([
        story_message(
            BELT_RELAY,
            "Relief wing is spooled and burning your way - four minutes \
             out. The convoy holds the lane until they arrive.",
        ),
        // The opening chain. Reveal beat: "the convoy holds the lane" is a
        // situation to absorb, so the screen objective waits the full gap
        // (pacing) rather than sharing a frame with the dispatch line. The
        // greeting follows a breath later.
        sequence(
            SEQ_OPENING,
            vec![
                step(
                    REVEAL_GAP,
                    vec![post_objective(
                        OBJ_SCREEN,
                        "Keep the convoy alive until the relief wing arrives.",
                    )],
                ),
                step(
                    HELLO_AT - REVEAL_GAP,
                    vec![story_message(
                        CAPTAIN_HALLORAN,
                        "Halloran here - the Queen's guild runs this line. Drives are \
                         cold on a transfer fault; we could not run if we wanted to.",
                    )],
                ),
            ],
        ),
        attach_objective_marker(ID_QUEEN, "CERES QUEEN"),
        attach_objective_marker(ID_MERIDIAN, "LONG MERIDIAN"),
        EventActionConfig::HudReadout(HudReadoutActionConfig {
            slot: "relief".to_string(),
            variable: VAR_RELIEF_REMAINING.to_string(),
            format: HudReadoutFormatConfig::Time,
            label: Some("RELIEF".to_string()),
            visible: true,
        }),
    ]);
    opening.extend(ThreePointRig::around("lifeline", Vec3::ZERO, 10.0).actions());

    let events = vec![
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: opening,
        },
        // The countdown, recomputed every live frame: RELIEF_SECS - clock.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: false,
            filters: vec![number_equals(VAR_ACT, 1.0)],
            actions: vec![set_variable(
                VAR_RELIEF_REMAINING,
                VariableExpressionNode::new_subtract(
                    VariableTermNode::Factor(VariableFactorNode::Literal(VariableLiteral::Number(
                        RELIEF_SECS,
                    ))),
                    variable(SCENARIO_ELAPSED_VAR),
                ),
            )],
        },
        // --- Wave one: two raiders, one vector. ---
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![
                number_equals(VAR_ACT, 1.0),
                number_greater_than(SCENARIO_ELAPSED_VAR, W1_AT),
            ],
            actions: wave_beat(
                None,
                BELT_RELAY,
                "Two contacts off the shelf, one vector, coming down the lane.",
                vec![
                    (
                        raider("raider_1a", W1_SPAWNS[0], ships::CARGOA_RAIDER_SHIP_ID, 8.0),
                        "RAIDER",
                    ),
                    (
                        raider("raider_1b", W1_SPAWNS[1], ships::CARGOA_RAIDER_SHIP_ID, 8.0),
                        "RAIDER",
                    ),
                ],
            ),
        },
        defeat_flag("raider_1a", VAR_R1A_DOWN),
        defeat_flag("raider_1b", VAR_R1B_DOWN),
        // Breathe: wave one cleared, before wave two shows.
        paced_line(
            0.0,
            CAPTAIN_HALLORAN,
            "Clean shooting. Watch the dark - the Tallyman does not send \
             twice the same way.",
            vec![
                number_equals(VAR_R1A_DOWN, 1.0),
                number_equals(VAR_R1B_DOWN, 1.0),
                number_equals(VAR_W2_UP, 0.0),
            ],
        ),
        // --- Wave two: three raiders, split vectors (one flanker). ---
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![
                number_equals(VAR_ACT, 1.0),
                number_greater_than(SCENARIO_ELAPSED_VAR, W2_AT),
                number_equals(VAR_R1A_DOWN, 1.0),
                number_equals(VAR_R1B_DOWN, 1.0),
            ],
            actions: wave_beat(
                Some(VAR_W2_UP),
                BELT_RELAY,
                "Three more - they split the lane, one swinging wide onto \
                 your flank.",
                vec![
                    (
                        raider("raider_2a", W2_SPAWNS[0], ships::CARGOA_RAIDER_SHIP_ID, 8.0),
                        "RAIDER",
                    ),
                    (
                        raider("raider_2b", W2_SPAWNS[1], ships::CARGOA_RAIDER_SHIP_ID, 8.0),
                        "RAIDER",
                    ),
                    (
                        raider("raider_2c", W2_SPAWNS[2], ships::CARGOA_RAIDER_SHIP_ID, 8.0),
                        "RAIDER",
                    ),
                ],
            ),
        },
        defeat_flag("raider_2a", VAR_R2A_DOWN),
        defeat_flag("raider_2b", VAR_R2B_DOWN),
        defeat_flag("raider_2c", VAR_R2C_DOWN),
        // Breathe: the Tallyman speaks for himself.
        paced_line(
            0.0,
            TALLYMAN,
            "You are burning my margins, pilot. The next crew brings real \
             guns.",
            vec![
                number_equals(VAR_R2A_DOWN, 1.0),
                number_equals(VAR_R2B_DOWN, 1.0),
                number_equals(VAR_R2C_DOWN, 1.0),
                number_equals(VAR_W3_UP, 0.0),
            ],
        ),
        // --- Wave three: the full-gun corvette and its escort. ---
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![
                number_equals(VAR_ACT, 1.0),
                number_greater_than(SCENARIO_ELAPSED_VAR, W3_AT),
                number_equals(VAR_R2A_DOWN, 1.0),
                number_equals(VAR_R2B_DOWN, 1.0),
                number_equals(VAR_R2C_DOWN, 1.0),
            ],
            actions: wave_beat(
                Some(VAR_W3_UP),
                BELT_RELAY,
                "Last push: a full-gun corvette with an escort. Hold them off.",
                vec![
                    (
                        raider("raider_3a", W3_SPAWNS[0], ships::CARGOA_SHIP_ID, 8.0),
                        "CORVETTE",
                    ),
                    (
                        raider("raider_3b", W3_SPAWNS[1], ships::CARGOA_RAIDER_SHIP_ID, 8.0),
                        "RAIDER",
                    ),
                ],
            ),
        },
        defeat_flag("raider_3a", VAR_R3A_DOWN),
        defeat_flag("raider_3b", VAR_R3B_DOWN),
        // --- The convoy's fate. Each convoy-ship death raises its flag and gets
        // its beacon-dark line; BOTH down is the loss (act 3 closes the
        // win gate before the defeat shows).
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![entity(ID_QUEEN), number_less_than(VAR_ACT, 2.0)],
            actions: vec![
                set_variable(VAR_QUEEN_DOWN, number(1.0)),
                detach_objective_marker(ID_QUEEN),
                story_message(BELT_RELAY, "The Ceres Queen's beacon just went dark."),
            ],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![entity(ID_MERIDIAN), number_less_than(VAR_ACT, 2.0)],
            actions: vec![
                set_variable(VAR_MERIDIAN_DOWN, number(1.0)),
                detach_objective_marker(ID_MERIDIAN),
                story_message(BELT_RELAY, "The Long Meridian's beacon just went dark."),
            ],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![
                number_equals(VAR_ACT, 1.0),
                number_equals(VAR_QUEEN_DOWN, 1.0),
                number_equals(VAR_MERIDIAN_DOWN, 1.0),
            ],
            actions: vec![
                set_variable(VAR_ACT, number(3.0)),
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
             raiders scatter. The convoy is whole.",
            vec![
                number_greater_than(SCENARIO_ELAPSED_VAR, RELIEF_SECS),
                number_equals(VAR_QUEEN_DOWN, 0.0),
                number_equals(VAR_MERIDIAN_DOWN, 0.0),
            ],
        ),
        victory(
            "The relief wing drops out of the burn, guns hot - and the \
             raiders scatter. Half the convoy made it. The Tallyman will \
             answer for the other half.",
            vec![
                number_greater_than(SCENARIO_ELAPSED_VAR, RELIEF_SECS),
                EventFilterConfig::Conditional(ConditionalFilterConfig::Or(
                    Box::new(number_equals(VAR_QUEEN_DOWN, 1.0)),
                    Box::new(number_equals(VAR_MERIDIAN_DOWN, 1.0)),
                )),
            ],
        ),
        victory(
            "The last raider breaks apart before the relief wing even \
             arrives. The convoy is whole.",
            vec![
                number_equals(VAR_W3_UP, 1.0),
                number_equals(VAR_R3A_DOWN, 1.0),
                number_equals(VAR_R3B_DOWN, 1.0),
                number_equals(VAR_QUEEN_DOWN, 0.0),
                number_equals(VAR_MERIDIAN_DOWN, 0.0),
            ],
        ),
        victory(
            "The last raider breaks apart before the relief wing even \
             arrives. Half the convoy made it. The Tallyman will answer for \
             the rest.",
            vec![
                number_equals(VAR_W3_UP, 1.0),
                number_equals(VAR_R3A_DOWN, 1.0),
                number_equals(VAR_R3B_DOWN, 1.0),
                EventFilterConfig::Conditional(ConditionalFilterConfig::Or(
                    Box::new(number_equals(VAR_QUEEN_DOWN, 1.0)),
                    Box::new(number_equals(VAR_MERIDIAN_DOWN, 1.0)),
                )),
            ],
        ),
        // --- Lose: the player dies on a live act; retry THIS scenario.
        // Terminal act FIRST: CurrentOutcome is last-write-wins, and the bell
        // Victory's clock gate is true every pulse - without act 3 here, a
        // mutual-destruction trade (the player's blast killing the last raider
        // just after the player dies) could overwrite this Defeat with a
        // Victory over the queued retry. Act 3 closes every win gate and stops
        // the countdown.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![entity(ID_PLAYER), number_equals(VAR_ACT, 1.0)],
            actions: vec![
                set_variable(VAR_ACT, number(3.0)),
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
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnNeutralized,
            once: true,
            filters: vec![entity(ID_PLAYER), number_equals(VAR_ACT, 1.0)],
            actions: vec![
                set_variable(VAR_ACT, number(3.0)),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "Nothing left to fight with - you drift down the lane the raiders now own.",
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
                      stalled convoy against raider waves until the \
                      relief wing arrives. Chapter three of the base \
                      storyline, part one."
            .to_string(),
        cubemap,
        // The chapter head: picker-visible, like Broadside.
        // Generated placeholder art (scripts/gen-scenario-thumbnails.py);
        // real art overwrites this same path with no code change.
        thumbnail: Some(AssetRef::from("self://thumbnails/lifeline.png")),
        hidden: false,
        menu_backdrop: false,
        skybox_brightness: DEFAULT_SKYBOX_BRIGHTNESS,
        watches: vec![scenario_elapsed_watch(SCENARIO_ELAPSED_VAR)],
        // Chapter three of the Nova Protocol campaign. Membership + order now
        // live in the `nova_protocol` campaign mapping, which also lists the
        // hidden finale (`final_tally`) for replay.
        events,
    }
}
