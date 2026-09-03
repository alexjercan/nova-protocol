//! "First Shift" - the chapter New Game opens on.
//!
//! An engineer takes a maintenance cutter out of the industrial carrier
//! Meridian for a routine inspection round, and comes back to a wreck. The
//! whole first half is a working shift that happens to teach the helm: burn to
//! a mark, thruster your way through a rock plate for three crates, lock the
//! planetoid, hand the leg to the computer, hold an orbit. Nothing shoots at
//! the player and the cutter carries no gun, because the chapter's ending is
//! not a fight the player can lose - it is one they can only watch.
//!
//! The attack is a REAL set piece, not damage off screen. The warship is a
//! `SpaceshipController::None` actor the scenario flies and fires by name: it
//! moves out from behind the large planetoid under its own thrust, turns its
//! whole hull onto the carrier, puts two railgun slugs into it, walks six
//! siege torpedoes across it, and burns away. Every step of that hangs off the
//! PREVIOUS one's completion event rather than a guessed delay, so it stages
//! identically at any frame rate.
//!
//! Script shape follows the mainline convention: one `beat` counter gates
//! every handler, and an objective posts a beat LATER than the line that
//! introduces it (see `pacing`).

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use super::{
    cast::{BEACON, CARRIER_NAME, CONTROL, DECK_CHIEF, PLAYER},
    pacing::{self, INSTRUCTION_GAP, MID_GAP, REVEAL_GAP},
    second_shift::SECOND_SHIFT_SCENARIO_ID,
    ships, stage, SCENARIO_ELAPSED_VAR,
};
use crate::scenario_helpers::prelude::*;

/// The scenario id, shared with nova_menu's New Game entry.
pub const FIRST_SHIFT_SCENARIO_ID: &str = "first_shift";

// --- layout ------------------------------------------------------------------
//
// Reviewed in `examples/playable/first_shift_map.rs`. The fixed belt - both
// planetoids, the rock plate, the far dressing - lives in `stage`; these are
// the marks this chapter adds to it.

/// The cutter undocks off the carrier's port side, close enough that the hull
/// fills the mirror.
const PLAYER_START_POS: Meters3 = Meters3::new(-1_100.0, 0.0, 2_500.0);
/// The first mark: a straight 3.6 km burn away from home, so the opening leg
/// is nothing but stick and throttle.
const FLIGHT_BEACON_POS: Meters3 = Meters3::new(0.0, 100.0, -900.0);
/// The three crates, deep in the rock plate where only the cutter fits. Spread
/// far enough apart that each pickup is its own moment.
const CRATE_POSITIONS: [Meters3; 3] = [
    Meters3::new(2_800.0, 20.0, -3_800.0),
    Meters3::new(2_300.0, 20.0, -4_250.0),
    Meters3::new(1_700.0, 20.0, -4_400.0),
];
/// Crate pickup radius: tight enough to require flying AT the crate, which is
/// the whole reason the thrusters are introduced here.
const CRATE_AREA_RADIUS: Meters = Meters(80.0);

/// The inspection round's arrival gate: an invisible sphere on the small
/// planetoid. GOTO parks 500 m outside the geometric body (700-1 200 m), so
/// the ring contains every park point on every mesh seed; the widest orbit
/// ring (1.82 km) is inside it too, so holding the orbit cannot fall out of
/// the gate. LEAVING it afterwards is what opens the attack.
const APPROACH_RING_RADIUS: Meters = Meters(2_400.0);
/// The orbit the round is signed off with, in seconds of continuous stable
/// station-keeping.
const ORBIT_HOLD_SECS: f64 = 5.0;

/// Where the warship waits: tucked behind the large planetoid, 3.4 km off its
/// centre on the side away from the whole inspection route. Nothing on the
/// route has a line to it.
const WARSHIP_HIDE_POS: Meters3 = Meters3::new(7_900.0, 250.0, -6_500.0);
/// Where it comes out to shoot from: 4.3 km clear of the planetoid's widest
/// possible body, 6.6 km off the carrier, and broadside to a cutter flying
/// home. The player watches the whole thing from abeam.
const WARSHIP_FIRING_POS: Meters3 = Meters3::new(3_700.0, 150.0, -2_200.0);
/// Where it goes afterwards. Nothing waits on this arrival - the order exists
/// to make the ship leave under thrust rather than blink out.
const WARSHIP_EXIT_POS: Meters3 = Meters3::new(11_000.0, 1_200.0, -12_000.0);
/// How close the approach parks. The default 500 m is fine for gameplay and
/// far too loose for staging: the firing mark is chosen for its sight lines.
const WARSHIP_APPROACH_STANDOFF: Meters = Meters(200.0);
/// How square the bore must be on the carrier before the guns are allowed to
/// speak. Two degrees at 6.6 km is a 230 m error - inside a hull this size.
const WARSHIP_ALIGN_TOLERANCE: f32 = 2.0;

/// Soft manual-speed cap while the chief is still talking: a missed brake in
/// the first minute should not send a new pilot out of the belt.
const PLAYER_SPEED_CAP: MetersPerSecond = MetersPerSecond(250.0);

// Scenario entity ids.
const ID_PLAYER: &str = "player_spaceship";
const ID_CARRIER: &str = "carrier";
const ID_WARSHIP: &str = "warship";
const ID_FLIGHT_BEACON: &str = "flight_beacon";
const ID_APPROACH_RING: &str = "approach_ring";
const ID_DISTRESS: &str = "distress_beacon";

// Objective ids: one gesture, or one errand, each.
const OBJ_BURN: &str = "burn";
const OBJ_SALVAGE: &str = "salvage";
const OBJ_LOCK: &str = "lock";
const OBJ_APPROACH: &str = "approach";
const OBJ_ORBIT: &str = "orbit";
const OBJ_RETURN: &str = "return";
const OBJ_WITNESS: &str = "witness";
const OBJ_SILENCE: &str = "silence";
const OBJ_DONE: &str = "done";

// Script variables.
const VAR_BEAT: &str = "beat";
const VAR_CRATES: &str = "crates_recovered";
/// The highest beat whose delayed setup has fired. The crate pickups wait on
/// it to know their objective has posted.
const VAR_SETUP_LAST: &str = "setup_last";
/// The distress act: the warship is gone, the channel is dead, and the beacon
/// is about to start. Reached from the salvo chain's last step.
const BEAT_DISTRESS: f64 = 9.0;
/// The outro act: the win is locked but the Victory overlay has not landed.
/// Every defeat gate sits below it, so dying during the epilogue declares
/// nothing.
const BEAT_OUTRO: f64 = 10.0;
const BEAT_WON: f64 = 11.0;

// Sequence, timer and order keys.
const SEQ_OPENING: &str = "opening";
const SEQ_SALVO: &str = "salvo";
const TIMER_ORBIT_HOLD: &str = "orbit_hold";
const ORDER_APPROACH: &str = "warship_approach";
const ORDER_ALIGN: &str = "warship_align";
const ORDER_EXIT: &str = "warship_exit";

/// The opening conversation's first line, and the gap between the ones that
/// follow it. The speed cap makes the drift diegetic: the cutter idles out of
/// the bay while the chief runs the board.
const OPEN_1_AT: f64 = 2.0;
const OPEN_GAP: f64 = 3.5;

/// OnEnter of `area` by the player ship.
fn player_enters(area: &str) -> EventFilterConfig {
    entity_pair(area, ID_PLAYER)
}

/// One line of the opening conversation, `after` seconds behind the previous.
fn open_line(after: f64, speaker: &str, line: &str) -> SequenceStepConfig {
    step(after, vec![story_message(speaker, line)])
}

/// Post a beat's world - its objective, its marks, its hint emphasis - a beat
/// AFTER the transition that played its comms line, so the introducing line
/// finishes before the new objective appears.
fn beat_setup(beat: f64, delay: f64, actions: Vec<EventActionConfig>) -> EventActionConfig {
    let mut all = vec![set_variable(VAR_SETUP_LAST, number(beat))];
    all.extend(actions);
    pacing::beat_later(&format!("beat_{beat}"), delay, all)
}

/// A completion handler for one of the warship's helm orders. The whole set
/// piece is a chain of these: no step guesses how long the one before it takes.
fn on_order(order: &str, actions: Vec<EventActionConfig>) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnShipOrderComplete,
        once: true,
        filters: vec![
            EventFilterConfig::ShipOrder(ShipOrderFilterConfig {
                order: Some(order.to_string()),
                ship: Some(ID_WARSHIP.to_string()),
                kind: None,
            }),
            number_equals(VAR_BEAT, 7.0),
        ],
        actions,
    }
}

/// The player's cutter: the block-built workboat, unarmed, with the helm
/// verbs withheld and handed back one lesson at a time.
///
/// The gates are spawn MODIFICATIONS aimed at the shared hull's flight
/// computer rather than being baked into the catalog ship, so they apply from
/// the instant the controller is built and only to this spawn.
fn player_ship() -> ScenarioObjectConfig {
    let controller_gate = vec![
        SectionModification::DisableVerb(FlightVerb::Rcs),
        SectionModification::DisableVerb(FlightVerb::Lock),
        SectionModification::DisableVerb(FlightVerb::Goto),
        SectionModification::DisableVerb(FlightVerb::Orbit),
    ];
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_PLAYER.to_string(),
            name: "Maintenance Cutter".to_string(),
            position: PLAYER_START_POS,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            // No mount, so no input mapping: the cutter cannot shoot
            // anything, and the chapter is authored around that.
            controller: SpaceshipController::Player(PlayerControllerConfig {
                speed_cap: Some(PLAYER_SPEED_CAP),
                ..Default::default()
            }),
            hull: ships::hull(ships::BLOCK_CUTTER_SHIP_ID),
            modifications: vec![ships::on_section(
                ships::BLOCK_BRIDGE_SECTION_ID,
                controller_gate,
            )],
        }),
    }
}

/// The Meridian: the largest hull the base game ships, parked and unarmed.
/// Neutral, so nothing in the belt has any reason to shoot it - which makes
/// what happens to it entirely the warship's doing.
fn carrier() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_CARRIER.to_string(),
            name: format!("ICV {CARRIER_NAME}"),
            position: stage::CARRIER_POS,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Neutral),
            hull: ships::hull(ships::BLOCK_CARRIER_SHIP_ID),
            ..Default::default()
        }),
    }
}

/// The stolen warship, spawned already pointing at its firing mark.
///
/// `SpaceshipController::None`: it is visibly Enemy on the HUD and yet it will
/// never acquire, never chase and never fire at anything the script does not
/// name. That separation is the point - the chapter's threat is a thing that
/// happens TO the player, and a bot deciding for itself could not be trusted
/// to leave an unarmed cutter alone.
fn warship() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_WARSHIP.to_string(),
            name: "Unidentified Warship".to_string(),
            position: WARSHIP_HIDE_POS,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Enemy),
            hull: ships::hull(ships::BLOCK_WARSHIP_SHIP_ID),
            ..Default::default()
        }),
    }
}

fn crate_object(index: usize, position: Meters3) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: format!("crate_{index}"),
            name: format!("Maintenance Crate {index}"),
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

/// The inspection round's arrival gate, spawned with the GOTO lesson so its
/// trigger cannot fire before there is a leg to fly.
fn approach_ring() -> EventActionConfig {
    EventActionConfig::CreateScenarioArea(ScenarioAreaConfig {
        id: ID_APPROACH_RING.to_string(),
        name: "Approach Ring".to_string(),
        position: stage::INSPECTION_POS,
        rotation: Quat::IDENTITY,
        radius: APPROACH_RING_RADIUS,
    })
}

fn move_warship(order: &str, position: Meters3, standoff: Option<Meters>) -> EventActionConfig {
    EventActionConfig::MoveShipTo(MoveShipToActionConfig {
        order: order.to_string(),
        ship: ID_WARSHIP.to_string(),
        position,
        arrival_standoff: standoff,
    })
}

fn fire_railgun(section: &str) -> EventActionConfig {
    EventActionConfig::ForceRailgunFire(ForceRailgunFireActionConfig {
        ship: ID_WARSHIP.to_string(),
        section: section.to_string(),
    })
}

fn fire_bay(section: &str) -> EventActionConfig {
    EventActionConfig::ForceTorpedoFire(ForceTorpedoFireActionConfig {
        ship: ID_WARSHIP.to_string(),
        section: section.to_string(),
        target: ID_CARRIER.to_string(),
    })
}

fn grant(verb: FlightVerb) -> EventActionConfig {
    EventActionConfig::SetControllerVerb(SetControllerVerbActionConfig {
        id: ID_PLAYER.to_string(),
        verb,
        enabled: true,
    })
}

/// The salvo: the whole attack, from the first slug to the empty channel.
///
/// It is a single chain because it is a single continuous event, and because
/// the cadence IS the writing - a deliberate pause between the two lances, six
/// bays walked across the hull rather than dumped at once, then the long quiet
/// while the ordnance crosses six kilometres of nothing.
///
/// Every torpedo is in the air before the first one arrives (the last leaves
/// at +6 s, the flight is about ten), so the whole salvo launches whatever the
/// hull ahead of it is doing by then.
fn salvo() -> EventActionConfig {
    let bays = ships::BLOCK_WARSHIP_BAY_IDS;
    sequence(
        SEQ_SALVO,
        vec![
            step(0.0, vec![fire_railgun(ships::BLOCK_WARSHIP_RAILGUN_IDS[0])]),
            step(2.5, vec![fire_railgun(ships::BLOCK_WARSHIP_RAILGUN_IDS[1])]),
            step(
                3.0,
                vec![story_message(
                    DECK_CHIEF,
                    "Drive's gone. We have no power to the ring and no way to \
                     answer them. Stay where you are.",
                )],
            ),
            step(3.0, vec![fire_bay(bays[0])]),
            step(1.2, vec![fire_bay(bays[1])]),
            step(1.2, vec![fire_bay(bays[2])]),
            step(1.2, vec![fire_bay(bays[3])]),
            step(1.2, vec![fire_bay(bays[4])]),
            step(1.2, vec![fire_bay(bays[5])]),
            step(
                2.0,
                vec![story_message(PLAYER, "Chief. Chief, get to a pod. Chief -")],
            ),
            // The ordnance crosses, and the channel stops.
            step(12.0, vec![set_variable(VAR_BEAT, number(8.0))]),
            step(
                4.0,
                vec![
                    // Nothing waits on this arrival. The order exists so the
                    // ship leaves the way it came - under its own thrust,
                    // taking its time, entirely unbothered.
                    move_warship(ORDER_EXIT, WARSHIP_EXIT_POS, None),
                    story_message(PLAYER, "Meridian Control, cutter one. Say again."),
                ],
            ),
            step(
                7.0,
                vec![story_message(PLAYER, "Meridian. Anyone on this channel.")],
            ),
            step(
                6.0,
                vec![post_objective(
                    OBJ_SILENCE,
                    "Hold position and keep the channel open.",
                )],
            ),
            step(14.0, vec![set_variable(VAR_BEAT, number(BEAT_DISTRESS))]),
        ],
    )
}

/// The epilogue: the tease line, then the banner and the hand-off to chapter
/// two.
fn outro() -> EventActionConfig {
    pacing::outro_sequence(
        VAR_BEAT,
        BEAT_WON,
        BEACON,
        "ANY VESSEL. ANY VESSEL. THIS IS MERIDIAN. HULL BREACH ALL DECKS. \
         SURVIVORS UNKNOWN.",
        "The Meridian is gone. Something in the wreck is still transmitting.",
        vec![post_objective(
            OBJ_DONE,
            "First shift complete. The beacon is still running.",
        )],
        Some(SECOND_SHIFT_SCENARIO_ID.to_string()),
    )
}

/// The Defeat pair. The cutter is unarmed and nothing hunts it, so the only
/// way to lose this chapter is to fly into something - which the rock plate
/// makes entirely possible.
fn defeat(message: &str, event: EventConfig) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: event,
        once: true,
        filters: vec![entity(ID_PLAYER), number_less_than(VAR_BEAT, BEAT_OUTRO)],
        actions: vec![
            EventActionConfig::Outcome(OutcomeActionConfig::new(
                ScenarioOutcomeKind::Defeat,
                message,
            )),
            EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: FIRST_SHIFT_SCENARIO_ID.to_string(),
                linger: true,
                delay: None,
            }),
        ],
    }
}

pub(crate) fn first_shift(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut start_spawns = vec![player_ship(), carrier()];
    start_spawns.extend(stage::belt(&asteroid_texture));
    for (index, position) in CRATE_POSITIONS.into_iter().enumerate() {
        start_spawns.push(crate_object(index + 1, position));
    }
    // The belt lights itself: there is no engine light in this game.
    start_spawns.extend(
        ThreePointRig::around("first_shift", Meters3::new(0.0, 0.0, -2_000.0), 25.0).objects(),
    );

    let events = vec![
        // The world, the counters, and the conversation that starts the shift.
        // No objective while the chief talks: the panel stays empty until she
        // sends the cutter off.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: start_spawns
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .chain([
                    set_variable(VAR_BEAT, number(1.0)),
                    set_variable(VAR_CRATES, number(0.0)),
                    set_variable(VAR_SETUP_LAST, number(0.0)),
                    sequence(
                        SEQ_OPENING,
                        vec![
                            open_line(
                                OPEN_1_AT,
                                DECK_CHIEF,
                                "Cutter one, you are clear of the bay. Take her out slow - she \
                                 is a work boat, not a racer.",
                            ),
                            open_line(OPEN_GAP, PLAYER, "Clear of the bay. Board is green."),
                            open_line(
                                OPEN_GAP,
                                DECK_CHIEF,
                                "Inspection round today. Ring survey, and the plate dropped three \
                                 crates on the last shift.",
                            ),
                            open_line(OPEN_GAP, PLAYER, "Understood. Where do you want me first?"),
                            open_line(
                                OPEN_GAP,
                                DECK_CHIEF,
                                "Work mark is lit ahead of you. Burn for it, and mind your \
                                 brakes - the Meridian is a big thing to reverse into.",
                            ),
                            step(
                                0.0,
                                vec![
                                    spawn_object(stage::beacon(
                                        ID_FLIGHT_BEACON,
                                        "WORK MARK",
                                        FLIGHT_BEACON_POS,
                                    )),
                                    post_objective(OBJ_BURN, "Burn to the work mark."),
                                    attach_objective_marker(ID_FLIGHT_BEACON, "WORK MARK"),
                                ],
                            ),
                        ],
                    ),
                ])
                .collect(),
        },
        // Beat 1 -> 2: the mark is made. The governor comes off, the thrusters
        // come on, and the errand becomes a real one.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![
                player_enters(ID_FLIGHT_BEACON),
                number_equals(VAR_BEAT, 1.0),
            ],
            actions: vec![
                set_variable(VAR_BEAT, number(2.0)),
                complete_objective(OBJ_BURN),
                // The training governor releases once a controlled leg is
                // proven.
                EventActionConfig::SetSpeedCap(SetSpeedCapActionConfig {
                    id: ID_PLAYER.to_string(),
                    cap: None,
                }),
                story_message(
                    DECK_CHIEF,
                    "Crates are in the rock plate south of you. Nothing our size fits in \
                     there - use your thrusters and take it a metre at a time.",
                ),
                beat_setup(
                    2.0,
                    INSTRUCTION_GAP,
                    vec![
                        grant(FlightVerb::Rcs),
                        post_objective(
                            OBJ_SALVAGE,
                            "Recover the 3 maintenance crates - hold [SHIFT] to thruster sideways.",
                        ),
                        detach_objective_marker(ID_FLIGHT_BEACON),
                        attach_objective_marker("crate_1", "CRATE"),
                        attach_objective_marker("crate_2", "CRATE"),
                        attach_objective_marker("crate_3", "CRATE"),
                        show_hint_emphasis("RCS"),
                    ],
                ),
            ],
        },
        // The pickups. One handler per crate (the despawn needs the concrete
        // id); the tally and the beat advance are update-gated below, so
        // nothing depends on handler order within one event. All three wait on
        // beat 2's setup, because the crates exist from OnStart and a pickup
        // during the opening would count against an objective that has not
        // posted.
        crate_pickup("crate_1"),
        crate_pickup("crate_2"),
        crate_pickup("crate_3"),
        crate_tally(1.0),
        crate_tally(2.0),
        // Beat 2 -> 3: the errand is done and the round begins. The targeting
        // computer comes online with the lesson that needs it.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![number_equals(VAR_BEAT, 2.0), number_equals(VAR_CRATES, 3.0)],
            actions: vec![
                set_variable(VAR_BEAT, number(3.0)),
                complete_objective(OBJ_SALVAGE),
                clear_hint_emphasis("RCS"),
                story_message(
                    DECK_CHIEF,
                    "Good. Ring survey next - the small body out west. Warm the targeting \
                     computer and hold your radar on it.",
                ),
                beat_setup(
                    3.0,
                    INSTRUCTION_GAP,
                    vec![
                        grant(FlightVerb::Lock),
                        post_objective(OBJ_LOCK, "Lock the inspection planetoid - hold [CTRL]."),
                        attach_objective_marker(stage::ID_INSPECTION, "SURVEY"),
                        show_hint_emphasis("RADAR"),
                    ],
                ),
            ],
        },
        // Beat 3 -> 4: the lock landed. Hand the leg to the computer.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTravelLockStart,
            once: true,
            filters: vec![
                player_enters(stage::ID_INSPECTION),
                number_equals(VAR_BEAT, 3.0),
            ],
            actions: vec![
                set_variable(VAR_BEAT, number(4.0)),
                complete_objective(OBJ_LOCK),
                clear_hint_emphasis("RADAR"),
                // The gate spawns at the TRANSITION, not in the delayed setup:
                // GOTO is granted a beat later, but a hand-flown run at the
                // planetoid must still find a ring to arrive in.
                approach_ring(),
                story_message(
                    DECK_CHIEF,
                    "Now hand her to the computer. It flies the leg; you watch the belt.",
                ),
                beat_setup(
                    4.0,
                    INSTRUCTION_GAP,
                    vec![
                        grant(FlightVerb::Goto),
                        post_objective(OBJ_APPROACH, "Locked. Press [G] to let the computer fly."),
                        show_hint_emphasis("GOTO"),
                    ],
                ),
            ],
        },
        // Beat 4 -> 5: arrival, deep in the planetoid's pull. The round is
        // signed off with a held orbit.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![
                player_enters(ID_APPROACH_RING),
                number_equals(VAR_BEAT, 4.0),
            ],
            actions: vec![
                set_variable(VAR_BEAT, number(5.0)),
                complete_objective(OBJ_APPROACH),
                clear_hint_emphasis("GOTO"),
                story_message(
                    DECK_CHIEF,
                    "That is the body's pull you are feeling. Ride it round - the computer \
                     will hold the ring for you while the survey runs.",
                ),
                beat_setup(
                    5.0,
                    MID_GAP,
                    vec![
                        grant(FlightVerb::Orbit),
                        post_objective(OBJ_ORBIT, "Press [O] and hold the orbit."),
                    ],
                ),
            ],
        },
        // The hold: stable station-keeping starts the clock, and losing it or
        // ending the orbit cancels it, so only one continuous five seconds
        // finishes the survey.
        orbit_watch(EventConfig::OnOrbitStable, true),
        orbit_watch(EventConfig::OnOrbitUnstable, false),
        orbit_watch(EventConfig::OnOrbitEnd, false),
        // Beat 5 -> 6: survey logged, shift over, come home.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: true,
            filters: vec![timer(TIMER_ORBIT_HOLD), number_equals(VAR_BEAT, 5.0)],
            actions: vec![
                set_variable(VAR_BEAT, number(6.0)),
                complete_objective(OBJ_ORBIT),
                story_message(
                    CONTROL,
                    "Cutter one, Meridian Control. Survey received. Break your orbit and come \
                     home - the bay is holding a slot for you.",
                ),
                beat_setup(
                    6.0,
                    INSTRUCTION_GAP,
                    vec![
                        post_objective(OBJ_RETURN, format!("Return to the {CARRIER_NAME}.")),
                        detach_objective_marker(stage::ID_INSPECTION),
                        attach_objective_marker(ID_CARRIER, CARRIER_NAME),
                    ],
                ),
            ],
        },
        // Beat 6 -> 7: the cutter breaks away for home, and something the belt
        // has no name for comes out from behind the large body.
        //
        // The warship spawns HERE rather than at OnStart. The player is at the
        // small planetoid with nine kilometres and a second planetoid between
        // them, so the spawn is unobservable - and a ship that does not exist
        // during the survey cannot be stumbled into during it.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnExit,
            once: true,
            filters: vec![
                player_enters(ID_APPROACH_RING),
                number_equals(VAR_BEAT, 6.0),
            ],
            actions: vec![
                set_variable(VAR_BEAT, number(7.0)),
                complete_objective(OBJ_RETURN),
                spawn_object(warship()),
                move_warship(
                    ORDER_APPROACH,
                    WARSHIP_FIRING_POS,
                    Some(WARSHIP_APPROACH_STANDOFF),
                ),
                story_message(
                    CONTROL,
                    "Cutter one, hold. We have a drive plume off the large body and no \
                     transponder on it.",
                ),
                // A threat reveal, and one nothing can be done about: the full
                // gap, so the line lands before the panel changes.
                beat_setup(
                    7.0,
                    REVEAL_GAP,
                    vec![
                        post_objective(OBJ_WITNESS, "Keep your distance. Do not close."),
                        detach_objective_marker(ID_CARRIER),
                        attach_objective_marker(ID_WARSHIP, "UNKNOWN"),
                    ],
                ),
            ],
        },
        // The set piece, one completion at a time. It is out; now it turns.
        on_order(
            ORDER_APPROACH,
            vec![
                story_message(
                    PLAYER,
                    "Control, that is a fleet hull. Earth military. It is turning on you.",
                ),
                EventActionConfig::ForceAlign(ForceAlignActionConfig {
                    order: ORDER_ALIGN.to_string(),
                    ship: ID_WARSHIP.to_string(),
                    look_at: stage::CARRIER_POS,
                    tolerance_degrees: WARSHIP_ALIGN_TOLERANCE,
                }),
            ],
        ),
        // The bore is on the Meridian and the alignment HOLDS it there, so
        // every gun below fires down the same line.
        on_order(
            ORDER_ALIGN,
            vec![
                story_message(
                    CONTROL,
                    "Unidentified vessel, this is a civil industrial hull. We are unarmed. \
                     Respond.",
                ),
                salvo(),
            ],
        ),
        // The distress act: the warship is a plume on the horizon and the
        // wreck starts talking on its own.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![number_equals(VAR_BEAT, BEAT_DISTRESS)],
            actions: pacing::open_outro(
                VAR_BEAT,
                BEAT_OUTRO,
                outro(),
                vec![
                    complete_objective(OBJ_SILENCE),
                    detach_objective_marker(ID_WARSHIP),
                    spawn_object(stage::beacon(ID_DISTRESS, "MERIDIAN", stage::CARRIER_POS)),
                    attach_objective_marker(ID_DISTRESS, "MERIDIAN"),
                    story_message(
                        PLAYER,
                        "There is a carrier signal in there. Weak, but it is running.",
                    ),
                ],
            ),
        },
        defeat(
            "Your cutter broke apart in the belt.",
            EventConfig::OnDestroyed,
        ),
        defeat(
            "Nothing left to fly with - you drift derelict in the belt.",
            EventConfig::OnNeutralized,
        ),
    ];

    ScenarioConfig {
        description: "A routine inspection round out of the carrier Meridian.".to_string(),
        thumbnail: Some(AssetRef::from("self://thumbnails/first_shift.png")),
        watches: vec![scenario_elapsed_watch(SCENARIO_ELAPSED_VAR)],
        events,
        ..ScenarioConfig::new(
            FIRST_SHIFT_SCENARIO_ID.to_string(),
            "First Shift".to_string(),
            cubemap,
        )
    }
}

/// One crate's pickup: despawn it and count it.
fn crate_pickup(id: &str) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnEnter,
        once: true,
        filters: vec![
            player_enters(id),
            number_equals(VAR_BEAT, 2.0),
            number_equals(VAR_SETUP_LAST, 2.0),
        ],
        actions: vec![despawn_object(id), increment_variable(VAR_CRATES)],
    }
}

/// The running tally. Completing and re-posting rebuilds the panel line in the
/// same frame, with no flicker.
fn crate_tally(count: f64) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnUpdate,
        once: true,
        filters: vec![
            number_equals(VAR_BEAT, 2.0),
            number_equals(VAR_CRATES, count),
        ],
        actions: vec![
            complete_objective(OBJ_SALVAGE),
            post_objective(OBJ_SALVAGE, format!("Crates recovered: {count}/3.")),
        ],
    }
}

/// The orbit hold's clock: `start` arms it, the other two cancel it.
fn orbit_watch(event: EventConfig, start: bool) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: event,
        once: false,
        filters: vec![
            player_enters(stage::ID_INSPECTION),
            number_equals(VAR_BEAT, 5.0),
        ],
        actions: vec![if start {
            EventActionConfig::TimerStart(TimerStartActionConfig {
                key: TIMER_ORBIT_HOLD.to_string(),
                seconds: number(ORBIT_HOLD_SECS),
            })
        } else {
            EventActionConfig::TimerCancel(TimerCancelActionConfig {
                key: TIMER_ORBIT_HOLD.to_string(),
            })
        }],
    }
}

#[cfg(test)]
mod tests {
    use nova_scenario::prelude::ASTEROID_GEOMETRIC_FACTOR_MAX;

    use super::*;
    use crate::base_content::sections::ordnance;

    fn config() -> ScenarioConfig {
        first_shift(AssetRef::default(), AssetRef::default())
    }

    /// Every action of every handler and every chain beat, flattened.
    fn all_actions(config: &ScenarioConfig) -> Vec<EventActionConfig> {
        let mut found = Vec::new();
        for action in config.events.iter().flat_map(|event| event.actions.iter()) {
            action.walk(&mut |action| found.push(action.clone()));
        }
        found
    }

    /// The handler that answers one of the warship's completions.
    fn completion_handler(config: &ScenarioConfig, order: &str) -> ScenarioEventConfig {
        config
            .events
            .iter()
            .find(|event| {
                matches!(event.name, EventConfig::OnShipOrderComplete)
                    && event.filters.iter().any(|filter| {
                        matches!(filter, EventFilterConfig::ShipOrder(ship_order)
                            if ship_order.order.as_deref() == Some(order))
                    })
            })
            .unwrap_or_else(|| panic!("no handler waits on order '{order}'"))
            .clone()
    }

    /// The whole set piece is a CHAIN OF COMPLETIONS, not a chain of guesses.
    /// A rewrite that replaced any link with a timer would stage differently
    /// on every machine: a heavy hull's approach and turn take as long as they
    /// take.
    #[test]
    fn every_beat_of_the_attack_waits_on_the_one_before_it() {
        let config = config();
        // The break for home issues the approach.
        let issues_approach = config.events.iter().any(|event| {
            matches!(event.name, EventConfig::OnExit)
                && event.actions.iter().any(|action| {
                    matches!(action, EventActionConfig::MoveShipTo(move_to)
                        if move_to.order == ORDER_APPROACH && move_to.ship == ID_WARSHIP)
                })
        });
        assert!(
            issues_approach,
            "nothing sends the warship out from behind the planetoid"
        );

        // Arriving turns it onto the carrier...
        let aligns = completion_handler(&config, ORDER_APPROACH)
            .actions
            .iter()
            .any(|action| {
                matches!(action, EventActionConfig::ForceAlign(align)
                    if align.order == ORDER_ALIGN && align.look_at == stage::CARRIER_POS)
            });
        assert!(aligns, "arriving does not put the bore on the Meridian");

        // ...and the settled bore is what opens fire, so no gun can speak
        // while the hull is still swinging.
        let opens_fire = completion_handler(&config, ORDER_ALIGN).actions.iter().any(
            |action| matches!(action, EventActionConfig::Sequence(chain) if chain.key == SEQ_SALVO),
        );
        assert!(opens_fire, "the alignment does not open the salvo");
    }

    /// Both lances and all six bays fire, each exactly once and each by name.
    /// The old broad all-bays action could not stage this at all; the failure
    /// this guards is quieter - a copy-paste that fires one flank twice and
    /// leaves three tubes loaded.
    #[test]
    fn the_salvo_fires_every_gun_the_warship_carries_exactly_once() {
        let actions = all_actions(&config());
        for lance in ships::BLOCK_WARSHIP_RAILGUN_IDS {
            let shots = actions
                .iter()
                .filter(|action| {
                    matches!(action, EventActionConfig::ForceRailgunFire(fire)
                        if fire.ship == ID_WARSHIP && fire.section == lance)
                })
                .count();
            assert_eq!(shots, 1, "lance '{lance}' fires {shots} times, not once");
        }
        for bay in ships::BLOCK_WARSHIP_BAY_IDS {
            let launches = actions
                .iter()
                .filter(|action| {
                    matches!(action, EventActionConfig::ForceTorpedoFire(fire)
                        if fire.ship == ID_WARSHIP
                            && fire.section == bay
                            && fire.target == ID_CARRIER)
                })
                .count();
            assert_eq!(
                launches, 1,
                "bay '{bay}' launches {launches} times at the Meridian, not once"
            );
        }
    }

    /// Every torpedo is away before the first one arrives.
    ///
    /// The salvo is authored as a walk across the hull, and a walk only reads
    /// as one if the whole rack is in the air while it happens. Stretch the
    /// launch cadence past the crossing time and the later bays fire into a
    /// ship that is already coming apart - or, if it came apart entirely, at a
    /// target that no longer exists and skip.
    #[test]
    fn the_whole_rack_is_away_before_the_first_one_lands() {
        let crossing =
            (WARSHIP_FIRING_POS - stage::CARRIER_POS).length().0 / ordnance::breaker().max_speed.0;

        let EventActionConfig::Sequence(chain) = salvo() else {
            unreachable!("the salvo is a sequence");
        };
        let mut elapsed = 0.0_f32;
        let mut first_launch = None;
        let mut last_launch = 0.0_f32;
        for step in &chain.steps {
            elapsed += step.after.unwrap_or(0.0) as f32;
            if step
                .actions
                .iter()
                .any(|action| matches!(action, EventActionConfig::ForceTorpedoFire(_)))
            {
                first_launch.get_or_insert(elapsed);
                last_launch = elapsed;
            }
        }
        let spread = last_launch - first_launch.expect("the salvo launches something");
        assert!(
            spread < crossing,
            "the salvo takes {spread:.1}s to launch but the first torpedo arrives \
             after {crossing:.1}s"
        );
    }

    /// The warship's marks clear the planetoid it hides behind, on EVERY mesh
    /// seed. The noise mesh reaches six times the nominal radius, so the body
    /// this ship parks beside can be three kilometres across - and a capital
    /// hull spawned inside one would be shoved out of it hard enough to come
    /// apart before the chapter's first line.
    #[test]
    fn neither_warship_mark_can_land_inside_the_large_planetoid() {
        let worst_case = stage::CONCEALMENT_RADIUS.0 * ASTEROID_GEOMETRIC_FACTOR_MAX;
        for (name, mark) in [
            ("the hide mark", WARSHIP_HIDE_POS),
            ("the firing mark", WARSHIP_FIRING_POS),
        ] {
            let clearance = (mark - stage::CONCEALMENT_POS).length().0;
            assert!(
                clearance > worst_case,
                "{name} sits {clearance:.0} m out, inside the body's worst-case \
                 {worst_case:.0} m surface"
            );
        }
    }

    /// The arrival gate contains every place the autopilot can park and every
    /// ring the orbit can hold, on every seed. If a high-factor seed put the
    /// park point or the orbit ring OUTSIDE the gate, the approach objective
    /// would never complete and the chapter would soft-lock at its midpoint -
    /// which is exactly how the shakedown's equivalent beat broke once.
    #[test]
    fn the_approach_ring_contains_every_park_point_and_every_orbit() {
        // Both are functions of the geometric body radius, which the seed
        // decides; the widest one is what has to fit.
        let widest_body = stage::INSPECTION_RADIUS.0 * ASTEROID_GEOMETRIC_FACTOR_MAX;
        // GOTO stops the ship its arrival standoff outside the surface.
        let park = widest_body + Meters::from_engine(FlightSettings::default().arrival_standoff).0;
        // The stable band the ORBIT verb circularizes into.
        let orbit_ring = 1.5 * (widest_body + 10.0);
        let gate = APPROACH_RING_RADIUS.0;
        assert!(
            park < gate,
            "a GOTO parks {park:.0} m out, outside the {gate:.0} m arrival gate"
        );
        assert!(
            orbit_ring < gate,
            "the orbit rides {orbit_ring:.0} m out, outside the {gate:.0} m gate"
        );
    }

    /// No crate is buried in a rock. The plate is authored tight enough that
    /// only the cutter fits, which is the same thing as saying a crate placed
    /// carelessly ends up inside a boulder no one can reach.
    #[test]
    fn every_crate_sits_clear_of_every_rock() {
        for crate_position in CRATE_POSITIONS {
            for (rock, radius) in stage::SALVAGE_ROCKS {
                let separation = (crate_position - rock).length().0;
                let required = radius.0 * ASTEROID_GEOMETRIC_FACTOR_MAX + CRATE_AREA_RADIUS.0;
                assert!(
                    separation > required,
                    "a crate at {crate_position:?} is inside the worst-case rock at \
                     {rock:?} ({separation:.0} m against {required:.0} m)"
                );
            }
        }
    }

    /// The cutter's helm is handed back one verb at a time, and every verb it
    /// spawns without is granted by some beat. A withheld verb no beat returns
    /// is a soft-lock waiting for the objective that asks for it.
    #[test]
    fn every_withheld_verb_is_granted_back_by_a_later_beat() {
        let ScenarioObjectKind::Spaceship(cutter) = player_ship().kind else {
            unreachable!("the player object is a ship");
        };
        let withheld: Vec<FlightVerb> = cutter
            .modifications
            .iter()
            .flat_map(|modification| modification.modifications.iter())
            .filter_map(|modification| match modification {
                SectionModification::DisableVerb(verb) => Some(*verb),
                _ => None,
            })
            .collect();
        assert!(!withheld.is_empty(), "the cutter starts with a gated helm");

        let actions = all_actions(&config());
        for verb in withheld {
            let granted = actions.iter().any(|action| {
                matches!(action, EventActionConfig::SetControllerVerb(set)
                    if set.id == ID_PLAYER && set.verb == verb && set.enabled)
            });
            assert!(granted, "{verb:?} is withheld and never handed back");
        }
    }
}
