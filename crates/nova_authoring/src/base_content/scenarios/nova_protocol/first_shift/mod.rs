//! "First Shift" - the chapter New Game opens on.
//!
//! The cutter takes a crew out of the industrial carrier Meridian for an
//! ordinary day on the rock plate, and comes back to a wreck. The whole first
//! half is that working shift, and it teaches the helm by needing it: burn to a
//! mark, learn the thrusters in open space, work three crates out of the plate,
//! lock a transit mark, hand the leg to the computer. Nothing shoots at the
//! player and the cutter carries no gun, because the chapter's ending is not a
//! fight they can lose - it is one they can only watch.
//!
//! The orbit is NOT assigned work. It is a detour the crew talks the captain
//! into on the way to the third crate, and the Meridian's answer to it - get
//! back on the plate, we lift within the hour - is what sends them across the
//! belt and then home again with a clean manifest, which is the last thing
//! anybody on either ship says about an ordinary day.
//!
//! The attack is a REAL set piece, not damage off screen. The last thing the
//! shift asks for is the run home, and the cutter parks itself on an outer mark
//! three kilometres off the Meridian - which is where the whole thing is
//! composed from. The warship is a `SpaceshipController::None` actor the
//! scenario flies and fires by name: it comes out from behind the large
//! planetoid in two legs, turns its whole hull onto the carrier, walks six
//! siege torpedoes out of its bays, puts two railgun slugs into the Meridian,
//! and burns away outbound. Every step of that hangs off the PREVIOUS one's
//! completion event rather than a guessed delay, so it stages identically at
//! any frame rate.
//!
//! The camera is four anchored shots and no more: the cutter as the warship
//! comes out from cover, the warship as its tubes open, the Meridian for the
//! guns, and the cutter again for the kill. It is handed back twice - once
//! across the long middle leg, so the approach is the player's own view, and
//! once before the aftermath - and it never touches the helm, so the player can
//! fly out of their own set piece at any point in it.
//!
//! Script shape follows the mainline convention: one `beat` counter gates every
//! handler, and an objective posts a beat LATER than the line that introduces
//! it (see `pacing`). The lines live in `story` and the map in `marks`, so a
//! dialogue pass and a pacing pass are two separate edits.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

mod marks;
mod story;
#[cfg(test)]
mod tests;

use marks::*;

use super::{
    cast::{BEACON, CARRIER_NAME, CONTROL, COPILOT, CUTTER_NAME, DECK_CHIEF, ENGINEER, PLAYER},
    pacing::{self, INSTRUCTION_GAP, MID_GAP, REVEAL_GAP},
    second_shift::SECOND_SHIFT_SCENARIO_ID,
    ships, stage, SCENARIO_ELAPSED_VAR,
};
use crate::scenario_helpers::prelude::*;

/// The scenario id, shared with nova_menu's New Game entry.
pub const FIRST_SHIFT_SCENARIO_ID: &str = "first_shift";

/// Where the chapter leaves the player: the outer mark off the Meridian's
/// quarter that the set piece is composed from and the cutter is still sitting
/// on when the credits line lands.
///
/// Exported for chapter two, which opens on the same coordinates. It is the ONE
/// number the two chapters share; everything else about this chapter's staging
/// stays inside it.
pub(super) const HOME_HOLD_POS: Meters3 = HOME_MARK.position;

// --- objectives --------------------------------------------------------------
//
// One gesture, or one errand, each.

const OBJ_BURN: &str = "burn";
const OBJ_STOP: &str = "stop";
const OBJ_TRIM_LATERAL: &str = "trim_lateral";
const OBJ_TRIM_VERTICAL: &str = "trim_vertical";
const OBJ_CRATE_FIRST: &str = "crate_first";
const OBJ_CRATE_SECOND: &str = "crate_second";
const OBJ_LOCK: &str = "lock";
const OBJ_GOTO: &str = "goto";
const OBJ_TRANSIT: &str = "transit";
const OBJ_DETOUR: &str = "detour";
const OBJ_ORBIT: &str = "orbit";
const OBJ_RETURN: &str = "return";
const OBJ_SEARCH: &str = "search";
const OBJ_HOME: &str = "home";
const OBJ_WITNESS: &str = "witness";
const OBJ_SILENCE: &str = "silence";
const OBJ_DONE: &str = "done";

// --- beats -------------------------------------------------------------------
//
// The one counter every handler is gated on. Each value is the state the shift
// is IN, named for what the player is being asked to do while it holds.

const VAR_BEAT: &str = "beat";

/// Hand-fly to the work mark.
const BEAT_LAUNCH: f64 = 1.0;
/// Come to a real stop before the RCS briefing.
const BEAT_STOP: f64 = 1.5;
/// First RCS translation, across.
const BEAT_TRIM_LATERAL: f64 = 2.0;
/// Second RCS translation, up.
const BEAT_TRIM_VERTICAL: f64 = 3.0;
/// The first crate, on the plate's open edge.
const BEAT_CRATE_FIRST: f64 = 4.0;
/// The second, well inside the rocks.
const BEAT_CRATE_SECOND: f64 = 5.0;
/// Hold the radar on the first transit mark.
const BEAT_LOCK: f64 = 6.0;
/// Hand that leg to the computer.
const BEAT_GOTO: f64 = 7.0;
/// The same gesture again, with nothing said over it.
const BEAT_TRANSIT: f64 = 8.0;
/// The crew's detour: fly to the survey body.
const BEAT_DETOUR: f64 = 9.0;
/// Hold the ring.
const BEAT_ORBIT: f64 = 10.0;
/// Back to the plate, on the Meridian's orders.
const BEAT_RETURN: f64 = 11.0;
/// Working the last crate.
const BEAT_SEARCH: f64 = 12.0;
/// The run home with the crates aboard. The attack opens on ARRIVAL, not on
/// the last crate: the set piece is composed from the hold, so the shift has to
/// put the cutter there first.
const BEAT_VANTAGE: f64 = 13.0;
/// The warship is out and moving. Every stage of the set piece runs here.
const BEAT_ATTACK: f64 = 14.0;
/// The warship is a plume on the horizon and the channel is dead.
const BEAT_DISTRESS: f64 = 15.0;
/// The win is locked but the Victory overlay has not landed. Every defeat gate
/// sits below it, so dying during the epilogue declares nothing.
const BEAT_OUTRO: f64 = 16.0;
const BEAT_WON: f64 = 17.0;

// --- timing ------------------------------------------------------------------

/// The opening's first line, and the gap between the two that follow it. Three
/// short lines at this gap put at most two cards on the panel at once, and the
/// speed cap makes the drift diegetic: the cutter idles out of the bay while
/// the chief runs the board.
const OPEN_FIRST_AT: f64 = 2.0;
const OPEN_GAP: f64 = 5.0;

/// How long the ring must hold before the detour counts as flown. Long enough
/// that the orbit is a thing the crew DID rather than a box that ticked, and
/// long enough to say three lines over while the workload is nothing.
const ORBIT_HOLD_SECS: f64 = 13.0;
/// The crew's lines during the hold, from the moment the ring goes stable.
const ORBIT_TALK_FIRST_AT: f64 = 3.0;
const ORBIT_TALK_GAP: f64 = 4.5;

/// The warship's approach, timed against a MEASURED run rather than an
/// estimate. On this stage, with this hull, the three legs take:
///
/// - cover to the emergence point, 3.40 km: about 34 s
/// - emergence to the firing mark, 4.03 km: about 33 s
/// - the turn onto the carrier: about 6 s
///
/// So the approach is a minute and a quarter of a ship getting closer, and it
/// carries five lines: one on each leg boundary, which the engine fires off the
/// real arrival, and one in the middle of each move leg, which is what the two
/// numbers below are. Only these two assume a flight time. If the warship's
/// thrust or mass changes, they are the only values to re-measure.
const APPROACH_SILENT_AT: f64 = 20.0;
const APPROACH_CHALLENGE_AT: f64 = 18.0;

/// The salvo's own cadence, also timed against the measured run.
///
/// The order is the SHOT list, not the gun list: six bays walked open under a
/// tight shot on the warship, one line from the cockpit, then the camera cuts
/// to the Meridian for the two lances and the torpedoes that arrive after
/// them, and back to the cutter for the kill. Nothing is said over a launch or
/// an impact.
///
/// The measured ordnance decides where the two cuts go. A Breaker leaves its
/// tube at 80 m/s and runs at 700, and across this 6.6 km the first one arrives
/// about 13 s after the first tube opens and the last about 23 s, with the
/// carrier gone a half second behind it. So the camera is on the Meridian at
/// 7.5 s - five clear seconds before anything reaches it - and off again at
/// 20 s, three seconds before the hull it is anchored to stops existing.
const SALVO_BAY_GAP: f64 = 1.0;
const SALVO_TUBES_CALL_AT: f64 = 1.5;
const SALVO_CUT_TO_CARRIER_AT: f64 = 1.0;
const SALVO_FIRST_LANCE_AT: f64 = 0.5;
const SALVO_SECOND_LANCE_AT: f64 = 1.5;
const SALVO_LAST_WORDS_AT: f64 = 2.0;
const SALVO_POD_CALL_AT: f64 = 4.5;
const SALVO_CUT_TO_CUTTER_AT: f64 = 4.0;
/// The camera comes home here - after the last torpedo has arrived, and before
/// a word of the aftermath. What follows is the player's own view again.
const SALVO_RELEASE_AT: f64 = 8.0;
const SALVO_EXIT_AT: f64 = 4.0;
const SALVO_SECOND_CALL_AT: f64 = 7.0;
const SALVO_SILENCE_AT: f64 = 6.0;
const SALVO_DISTRESS_AT: f64 = 14.0;

// --- keys --------------------------------------------------------------------

const SEQ_OPENING: &str = "opening";
const SEQ_ORBIT_TALK: &str = "orbit_talk";
const SEQ_PLUME: &str = "plume";
const SEQ_EMERGING: &str = "emerging";
const SEQ_CLOSING: &str = "closing";
const SEQ_SALVO: &str = "salvo";
const TIMER_ORBIT_HOLD: &str = "orbit_hold";
const ORDER_EMERGE: &str = "warship_emerge";
const ORDER_APPROACH: &str = "warship_approach";
const ORDER_ALIGN: &str = "warship_align";
const ORDER_EXIT: &str = "warship_exit";

// --- the cast on the map -----------------------------------------------------

/// The player's cutter: the block-built workboat, unarmed, with the helm verbs
/// withheld and handed back one lesson at a time.
///
/// The gates are spawn MODIFICATIONS aimed at the shared hull's flight computer
/// rather than being baked into the catalog ship, so they apply from the
/// instant the controller is built and only to this spawn.
fn cutter() -> ScenarioObjectConfig {
    let controller_gate = vec![
        SectionModification::DisableVerb(FlightVerb::Rcs),
        SectionModification::DisableVerb(FlightVerb::Lock),
        SectionModification::DisableVerb(FlightVerb::Goto),
        SectionModification::DisableVerb(FlightVerb::Orbit),
    ];
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_CUTTER.to_string(),
            name: CUTTER_NAME.to_string(),
            position: CUTTER_START_POS,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            // No mount, so no input mapping: the cutter cannot shoot anything,
            // and the chapter is authored around that.
            controller: SpaceshipController::Player(PlayerControllerConfig {
                speed_cap: Some(CUTTER_SPEED_CAP),
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

/// The stolen warship, spawned already pointing at its emergence mark.
///
/// `SpaceshipController::None`: it is visibly Enemy on the HUD and yet it will
/// never acquire, never chase and never fire at anything the script does not
/// name. That separation is the point - the chapter's threat is a thing that
/// happens TO the player, and a bot deciding for itself could not be trusted to
/// leave an unarmed cutter alone.
///
/// Its two spinal guns are the catalog's SIEGE lance (`ships::block`), not the
/// standard one every other ship in the fleet carries: the beat this scene has
/// to sell is one shot opening a carrier, and the standard lance is priced to
/// bore a corridor through a corvette. That is a prototype the warship hull
/// mounts, not a number this scenario writes - so the gun is visible in the
/// catalog and no other lance in the game changed.
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

/// The detour's arrival gate, spawned with the detour so its trigger cannot
/// fire before there is a leg to fly.
fn approach_ring() -> EventActionConfig {
    EventActionConfig::CreateScenarioArea(ScenarioAreaConfig {
        id: ID_APPROACH_RING.to_string(),
        name: "Approach Ring".to_string(),
        position: stage::INSPECTION_POS,
        rotation: Quat::IDENTITY,
        radius: APPROACH_RING_RADIUS,
    })
}

// --- small builders ----------------------------------------------------------

/// OnEnter/OnExit of `area` by the cutter.
fn cutter_enters(area: &str) -> EventFilterConfig {
    entity_pair(area, ID_CUTTER)
}

/// A player GOTO completion at `target` by the cutter.
fn cutter_completes_goto(target: &str) -> EventFilterConfig {
    entity_pair(target, ID_CUTTER)
}

/// One line of the opening conversation, `after` seconds behind the previous.
fn open_line(after: f64, speaker: &str, line: &str) -> SequenceStepConfig {
    step(after, vec![story_message(speaker, line)])
}

/// The sequence key a beat's delayed half runs on. One per beat, and the beat
/// numbers are unique, so the chains can never collide.
fn beat_key(beat: f64) -> String {
    format!("beat_{beat}")
}

/// A beat's world - its objective, its marks, its hint emphasis - landing
/// `delay` after the transition that played its line, so the introducing line
/// is finished before the panel changes.
fn beat_setup(beat: f64, delay: f64, actions: Vec<EventActionConfig>) -> EventActionConfig {
    pacing::beat_later(&beat_key(beat), delay, actions)
}

/// The same, with one more line in between: the transition's line lands and
/// fades, a coaching line follows, and the panel changes an instruction gap
/// after that.
///
/// What a lesson needs when the control it teaches is not the one just used -
/// the reveal and the instruction are two different thoughts, and stacking them
/// on one card is how the old script asked a player to read during a maneuver.
fn coached_beat_setup(
    beat: f64,
    speaker: &str,
    line: &str,
    actions: Vec<EventActionConfig>,
) -> EventActionConfig {
    sequence(
        &beat_key(beat),
        vec![
            step(REVEAL_GAP, vec![story_message(speaker, line)]),
            step(INSTRUCTION_GAP, actions),
        ],
    )
}

/// A handler that answers one of the warship's helm orders. The whole set piece
/// is a chain of these: no step guesses how long the one before it takes.
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
            number_equals(VAR_BEAT, BEAT_ATTACK),
        ],
        actions,
    }
}

fn move_warship(order: &str, position: Meters3) -> EventActionConfig {
    EventActionConfig::MoveShipTo(MoveShipToActionConfig {
        order: order.to_string(),
        ship: ID_WARSHIP.to_string(),
        position,
        arrival_standoff: Some(WARSHIP_APPROACH_STANDOFF),
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
        id: ID_CUTTER.to_string(),
        verb,
        enabled: true,
    })
}

/// One shot: hang the camera off `anchor` and point it at `look_at`.
///
/// Camera authority only. The beat explicitly suspends control before the
/// first shot and restores it when [`release_camera`] hands the view back.
fn film(anchor: &str, offset: Meters3, look_at: CameraLookAtConfig) -> EventActionConfig {
    EventActionConfig::SetCameraAnchor(SetCameraAnchorActionConfig {
        anchor: anchor.to_string(),
        offset,
        // World axes: two of the three anchors are free to turn, and a
        // hull-local offset would compose the shot differently depending on
        // which way the ship happens to be pointing when the guns come out.
        frame: CameraOffsetFrame::World,
        look_at,
    })
}

/// Look at `id`.
fn at(id: &str) -> CameraLookAtConfig {
    CameraLookAtConfig::Object(id.to_string())
}

/// Give the camera back to the cutter's own chase rig.
fn release_camera() -> EventActionConfig {
    EventActionConfig::ReleaseCamera(ReleaseCameraActionConfig)
}

fn suspend_player_control() -> EventActionConfig {
    EventActionConfig::SuspendPlayerControl(SuspendPlayerControlActionConfig)
}

fn resume_player_control() -> EventActionConfig {
    EventActionConfig::ResumePlayerControl(ResumePlayerControlActionConfig)
}

// --- the shift ---------------------------------------------------------------

pub(crate) fn first_shift(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut start_spawns = vec![cutter(), carrier()];
    start_spawns.extend(stage::belt(&asteroid_texture));
    // The belt lights itself: there is no engine light in this game.
    start_spawns.extend(
        ThreePointRig::around("first_shift", Meters3::new(0.0, 0.0, -2_000.0), 25.0).objects(),
    );

    let events = vec![
        // The world, the counter, and the three lines that start the shift. No
        // objective while the chief talks: the panel stays empty until she
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
                    set_variable(VAR_BEAT, number(BEAT_LAUNCH)),
                    sequence(
                        SEQ_OPENING,
                        vec![
                            open_line(OPEN_FIRST_AT, DECK_CHIEF, story::OPEN_CHIEF_CLEAR),
                            open_line(OPEN_GAP, PLAYER, story::OPEN_PLAYER_GREEN),
                            open_line(OPEN_GAP, DECK_CHIEF, story::OPEN_CHIEF_BURN),
                            step(
                                INSTRUCTION_GAP,
                                [post_objective(OBJ_BURN, story::OBJ_TEXT_BURN)]
                                    .into_iter()
                                    .chain(WORK_MARK.raise())
                                    .collect(),
                            ),
                        ],
                    ),
                ])
                .collect(),
        },
        // The mark is made. STOP must finish before the thrusters are taught
        // HERE, in open space, where a mistake costs nothing - not on the
        // plate, which is where the old script first asked for them. The 150
        // m/s manual governor stays for the whole shift.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![WORK_MARK.entered(), number_equals(VAR_BEAT, BEAT_LAUNCH)],
            actions: [
                set_variable(VAR_BEAT, number(BEAT_STOP)),
                complete_objective(OBJ_BURN),
                story_message(COPILOT, story::TRIM_COPILOT_STOP),
            ]
            .into_iter()
            .chain(WORK_MARK.clear())
            .chain([beat_setup(
                BEAT_STOP,
                INSTRUCTION_GAP,
                vec![
                    post_objective(OBJ_STOP, story::OBJ_TEXT_STOP),
                    show_hint_emphasis("STOP"),
                ],
            )])
            .collect(),
        },
        // STOP is a physical gate, not an elapsed dialogue gap. Only once the
        // autopilot reports rest does the RCS lesson open.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStopComplete,
            once: true,
            filters: vec![entity(ID_CUTTER), number_equals(VAR_BEAT, BEAT_STOP)],
            actions: vec![
                set_variable(VAR_BEAT, number(BEAT_TRIM_LATERAL)),
                complete_objective(OBJ_STOP),
                clear_hint_emphasis("STOP"),
                story_message(COPILOT, story::TRIM_COPILOT_TEACH),
                beat_setup(
                    BEAT_TRIM_LATERAL,
                    INSTRUCTION_GAP,
                    [
                        grant(FlightVerb::Rcs),
                        post_objective(OBJ_TRIM_LATERAL, story::OBJ_TEXT_TRIM_LATERAL),
                        show_hint_emphasis("RCS"),
                    ]
                    .into_iter()
                    .chain(TRIM_LATERAL.raise())
                    .collect(),
                ),
            ],
        },
        // The second axis. Same gesture, almost nothing said over it.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![
                TRIM_LATERAL.entered(),
                number_equals(VAR_BEAT, BEAT_TRIM_LATERAL),
            ],
            actions: [
                set_variable(VAR_BEAT, number(BEAT_TRIM_VERTICAL)),
                complete_objective(OBJ_TRIM_LATERAL),
                story_message(COPILOT, story::TRIM_COPILOT_SECOND_AXIS),
            ]
            .into_iter()
            .chain(TRIM_LATERAL.clear())
            .chain([beat_setup(
                BEAT_TRIM_VERTICAL,
                INSTRUCTION_GAP,
                [post_objective(
                    OBJ_TRIM_VERTICAL,
                    story::OBJ_TEXT_TRIM_VERTICAL,
                )]
                .into_iter()
                .chain(TRIM_VERTICAL.raise())
                .collect(),
            )])
            .collect(),
        },
        // Only now does the plate open, with the first crate on its outer edge
        // where the rocks are sparse. The leg out to it is the quiet time
        // before the field starts asking for things.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![
                TRIM_VERTICAL.entered(),
                number_equals(VAR_BEAT, BEAT_TRIM_VERTICAL),
            ],
            actions: [
                set_variable(VAR_BEAT, number(BEAT_CRATE_FIRST)),
                complete_objective(OBJ_TRIM_VERTICAL),
                story_message(DECK_CHIEF, story::CRATE_CHIEF_FIRST),
            ]
            .into_iter()
            .chain(TRIM_VERTICAL.clear())
            .chain([beat_setup(
                BEAT_CRATE_FIRST,
                INSTRUCTION_GAP,
                reveal_crate(1, OBJ_CRATE_FIRST, story::OBJ_TEXT_CRATE_FIRST),
            )])
            .collect(),
        },
        // One crate at a time, each revealed only when the one before it is
        // aboard. Three chips at once turns a lesson in flying the plate into a
        // shopping list.
        crate_pickup(
            1,
            BEAT_CRATE_FIRST,
            OBJ_CRATE_FIRST,
            BEAT_CRATE_SECOND,
            ENGINEER,
            story::CRATE_ENGINEER_SECOND,
            reveal_crate(2, OBJ_CRATE_SECOND, story::OBJ_TEXT_CRATE_SECOND),
        ),
        // Two aboard, and the third is out of the plate entirely - which is the
        // errand that needs the targeting computer.
        crate_pickup(
            2,
            BEAT_CRATE_SECOND,
            OBJ_CRATE_SECOND,
            BEAT_LOCK,
            DECK_CHIEF,
            story::LOCK_CHIEF,
            [
                grant(FlightVerb::Lock),
                clear_hint_emphasis("RCS"),
                post_objective(OBJ_LOCK, story::OBJ_TEXT_LOCK),
                show_hint_emphasis("RADAR"),
            ]
            .into_iter()
            .chain(TRANSIT_ONE.raise())
            .collect(),
        ),
        // The lock landed. GOTO is its own lesson, on its own card, once there
        // is a lock for it to fly.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTravelLockStart,
            once: true,
            filters: vec![TRANSIT_ONE.entered(), number_equals(VAR_BEAT, BEAT_LOCK)],
            actions: vec![
                set_variable(VAR_BEAT, number(BEAT_GOTO)),
                complete_objective(OBJ_LOCK),
                clear_hint_emphasis("RADAR"),
                story_message(DECK_CHIEF, story::GOTO_CHIEF),
                beat_setup(
                    BEAT_GOTO,
                    INSTRUCTION_GAP,
                    vec![
                        grant(FlightVerb::Goto),
                        post_objective(OBJ_GOTO, story::OBJ_TEXT_GOTO),
                        show_hint_emphasis("GOTO"),
                    ],
                ),
            ],
        },
        // The repeat: the same two keys with four words over them, so the
        // gesture is practised once before it matters. Completion waits for
        // the autopilot to settle before the target is retired.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnGotoComplete,
            once: true,
            filters: vec![
                cutter_completes_goto(TRANSIT_ONE.id),
                number_equals(VAR_BEAT, BEAT_GOTO),
            ],
            actions: [
                set_variable(VAR_BEAT, number(BEAT_TRANSIT)),
                complete_objective(OBJ_GOTO),
                clear_hint_emphasis("GOTO"),
                story_message(DECK_CHIEF, story::TRANSIT_CHIEF_AGAIN),
            ]
            .into_iter()
            .chain(TRANSIT_ONE.clear())
            .chain([beat_setup(
                BEAT_TRANSIT,
                INSTRUCTION_GAP,
                [post_objective(OBJ_TRANSIT, story::OBJ_TEXT_TRANSIT)]
                    .into_iter()
                    .chain(TRANSIT_TWO.raise())
                    .collect(),
            )])
            .collect(),
        },
        // The detour. It is the CREW's idea, out here where the survey body is
        // close and the third crate is still a long way off - not an assignment,
        // which is what makes the Meridian's answer to it land.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnGotoComplete,
            once: true,
            filters: vec![
                cutter_completes_goto(TRANSIT_TWO.id),
                number_equals(VAR_BEAT, BEAT_TRANSIT),
            ],
            actions: [
                set_variable(VAR_BEAT, number(BEAT_DETOUR)),
                complete_objective(OBJ_TRANSIT),
                story_message(COPILOT, story::DETOUR_COPILOT),
            ]
            .into_iter()
            .chain(TRANSIT_TWO.clear())
            .chain([coached_beat_setup(
                BEAT_DETOUR,
                ENGINEER,
                story::DETOUR_ENGINEER,
                vec![
                    grant(FlightVerb::Orbit),
                    approach_ring(),
                    post_objective(OBJ_DETOUR, story::OBJ_TEXT_DETOUR),
                    attach_objective_marker(stage::ID_INSPECTION, "SURVEY BODY"),
                ],
            )])
            .collect(),
        },
        // Arrival, deep in the body's pull.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![
                cutter_enters(ID_APPROACH_RING),
                number_equals(VAR_BEAT, BEAT_DETOUR),
            ],
            actions: vec![
                set_variable(VAR_BEAT, number(BEAT_ORBIT)),
                complete_objective(OBJ_DETOUR),
                story_message(COPILOT, story::ORBIT_COPILOT),
                beat_setup(
                    BEAT_ORBIT,
                    MID_GAP,
                    vec![
                        post_objective(OBJ_ORBIT, story::OBJ_TEXT_ORBIT),
                        show_hint_emphasis("ORBIT"),
                    ],
                ),
            ],
        },
        // The hold: stable station-keeping starts the clock and the crew's one
        // unguarded conversation; losing the ring or ending the orbit cancels
        // the clock, so only one continuous hold finishes the detour.
        orbit_watch(EventConfig::OnOrbitStable, true),
        orbit_watch(EventConfig::OnOrbitUnstable, false),
        orbit_watch(EventConfig::OnOrbitEnd, false),
        // And the Meridian, which has been watching the whole thing, sends them
        // back to the job they left.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: true,
            filters: vec![timer(TIMER_ORBIT_HOLD), number_equals(VAR_BEAT, BEAT_ORBIT)],
            actions: vec![
                set_variable(VAR_BEAT, number(BEAT_RETURN)),
                complete_objective(OBJ_ORBIT),
                clear_hint_emphasis("ORBIT"),
                detach_objective_marker(stage::ID_INSPECTION),
                despawn_object(ID_APPROACH_RING),
                story_message(CONTROL, story::RETURN_CONTROL),
                coached_beat_setup(
                    BEAT_RETURN,
                    DECK_CHIEF,
                    story::RETURN_CHIEF,
                    [post_objective(OBJ_RETURN, story::OBJ_TEXT_RETURN)]
                        .into_iter()
                        .chain(WORK_SITE.raise())
                        .collect(),
                ),
            ],
        },
        // Back on the plate, parked at the work site: the return completes on
        // GOTO's physical settle edge, not on crossing the mark or leaving the
        // body it was flown from.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnGotoComplete,
            once: true,
            filters: vec![
                cutter_completes_goto(WORK_SITE.id),
                number_equals(VAR_BEAT, BEAT_RETURN),
            ],
            actions: [
                set_variable(VAR_BEAT, number(BEAT_SEARCH)),
                complete_objective(OBJ_RETURN),
                story_message(COPILOT, story::SEARCH_COPILOT),
            ]
            .into_iter()
            .chain(WORK_SITE.clear())
            .chain([beat_setup(
                BEAT_SEARCH,
                INSTRUCTION_GAP,
                reveal_crate(3, OBJ_SEARCH, story::OBJ_TEXT_SEARCH),
            )])
            .collect(),
        },
        // The last crate comes aboard, the sheet is clean, and the chief calls
        // them in. The mark is lit on radar and sized for the autopilot, so the
        // leg home is a GOTO with the crew talking over it - which is how the
        // shift gets the cutter parked exactly where the set piece is composed
        // from without ever taking the stick off the player.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![
                cutter_enters(&crate_id(3)),
                number_equals(VAR_BEAT, BEAT_SEARCH),
            ],
            actions: vec![
                set_variable(VAR_BEAT, number(BEAT_VANTAGE)),
                complete_objective(OBJ_SEARCH),
                detach_objective_marker(&crate_id(3)),
                despawn_object(&crate_id(3)),
                story_message(DECK_CHIEF, story::HOME_CHIEF),
                beat_setup(
                    BEAT_VANTAGE,
                    INSTRUCTION_GAP,
                    [post_objective(OBJ_HOME, story::OBJ_TEXT_HOME)]
                        .into_iter()
                        .chain(HOME_MARK.raise())
                        .collect(),
                ),
            ],
        },
        // GOTO has come to rest at the outer mark, three kilometres off the Meridian, with the
        // whole belt on the other side of the canopy. THAT is what the attack
        // waits for: the player is where the shot is, holding station, and not
        // about to hit anything.
        //
        // The camera comes on in the SAME frame the warship starts moving,
        // because the entrance is the shot and the first leg is thirty-four
        // measured seconds long: the player's own hull in the near ground, a
        // plume nobody can identify yet down the middle of the frame, and one
        // line from Control over it. Waiting for the ship to ARRIVE before
        // filming it would spend that whole leg on the chase rig.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnGotoComplete,
            once: true,
            filters: vec![
                cutter_completes_goto(HOME_MARK.id),
                number_equals(VAR_BEAT, BEAT_VANTAGE),
            ],
            actions: [
                set_variable(VAR_BEAT, number(BEAT_ATTACK)),
                complete_objective(OBJ_HOME),
            ]
            .into_iter()
            .chain(HOME_MARK.clear())
            .chain([
                spawn_object(warship()),
                move_warship(ORDER_EMERGE, WARSHIP_EMERGE_POS),
                suspend_player_control(),
                film(ID_CUTTER, CINEMA_ENTRY_OFFSET, at(ID_WARSHIP)),
                story_message(CONTROL, story::ATTACK_CONTROL_PLUME),
                pacing::beat_later(
                    SEQ_PLUME,
                    REVEAL_GAP,
                    vec![
                        post_objective(OBJ_WITNESS, story::OBJ_TEXT_WITNESS),
                        attach_objective_marker(ID_WARSHIP, "UNKNOWN"),
                    ],
                ),
                // Halfway through the first leg, with nothing to do but watch
                // it grow.
                sequence(
                    SEQ_EMERGING,
                    vec![step(
                        APPROACH_SILENT_AT,
                        vec![story_message(COPILOT, story::ATTACK_COPILOT_SILENT)],
                    )],
                ),
            ])
            .collect(),
        },
        // It is out from behind the body and close enough to read a hull off.
        // The entry shot ends here, on the identification: the camera goes back
        // to the player for the second leg, so the half minute in which the
        // thing crosses the belt is theirs to look around in.
        on_order(
            ORDER_EMERGE,
            vec![
                release_camera(),
                resume_player_control(),
                story_message(PLAYER, story::ATTACK_PLAYER_MILITARY),
                move_warship(ORDER_APPROACH, WARSHIP_FIRING_POS),
                // Halfway through the second leg. The Meridian tries talking to
                // it, which is the last thing anybody tries.
                sequence(
                    SEQ_CLOSING,
                    vec![step(
                        APPROACH_CHALLENGE_AT,
                        vec![story_message(CONTROL, story::ATTACK_CONTROL_CHALLENGE)],
                    )],
                ),
            ],
        ),
        // On its firing mark. Now it turns, and the turn takes six seconds that
        // the deck chief spends narrating it.
        on_order(
            ORDER_APPROACH,
            vec![
                story_message(DECK_CHIEF, story::ATTACK_CHIEF_TURNING),
                EventActionConfig::ForceAlign(ForceAlignActionConfig {
                    order: ORDER_ALIGN.to_string(),
                    ship: ID_WARSHIP.to_string(),
                    look_at: stage::CARRIER_POS,
                    tolerance_degrees: WARSHIP_ALIGN_TOLERANCE,
                }),
            ],
        ),
        // The bore is on the Meridian and the alignment HOLDS it there, so
        // every gun below fires down the same line. Nothing is said into the
        // gap: the next voice is the cockpit, over open tubes.
        on_order(ORDER_ALIGN, vec![salvo()]),
        // The distress act: the warship is a plume on the horizon and the wreck
        // starts talking on its own.
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
                    spawn_object(stage::beacon(ID_DISTRESS, CARRIER_NAME, stage::CARRIER_POS)),
                    attach_objective_marker(ID_DISTRESS, CARRIER_NAME),
                    story_message(PLAYER, story::AFTER_PLAYER_CARRIER_SIGNAL),
                ],
            ),
        },
        defeat(story::DEFEAT_DESTROYED, EventConfig::OnDestroyed),
        defeat(story::DEFEAT_NEUTRALIZED, EventConfig::OnNeutralized),
    ];

    ScenarioConfig {
        description: "A routine shift on the rock plate, out of the carrier Meridian.".to_string(),
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

/// Put one crate on the map and point the panel at it. The crate is spawned
/// with its objective, not at OnStart: a crate that exists before it is asked
/// for is a crate the player can collect against nothing.
fn reveal_crate(nth: usize, objective: &str, text: &str) -> Vec<EventActionConfig> {
    vec![
        spawn_object(crate_object(nth)),
        post_objective(objective, text),
        attach_objective_marker(&crate_id(nth), "CRATE"),
    ]
}

/// One crate coming aboard, and the beat it opens.
///
/// The pickup owns the despawn (the crate goes, not just its chip) and carries
/// the whole transition, so the reveal of the NEXT crate is one delayed step
/// behind the line that asks for it.
#[expect(
    clippy::too_many_arguments,
    reason = "one call per crate, and every argument is the beat's own text or id: \
              folding them into a struct would move the same list one line up"
)]
fn crate_pickup(
    nth: usize,
    beat: f64,
    objective: &str,
    next_beat: f64,
    speaker: &str,
    line: &str,
    setup: Vec<EventActionConfig>,
) -> ScenarioEventConfig {
    let id = crate_id(nth);
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnEnter,
        once: true,
        filters: vec![cutter_enters(&id), number_equals(VAR_BEAT, beat)],
        actions: vec![
            set_variable(VAR_BEAT, number(next_beat)),
            complete_objective(objective),
            detach_objective_marker(&id),
            despawn_object(&id),
            story_message(speaker, line),
            beat_setup(next_beat, INSTRUCTION_GAP, setup),
        ],
    }
}

/// The orbit hold's clock, and the crew's conversation over it: `start` arms
/// both, the other two cancel the clock.
fn orbit_watch(event: EventConfig, start: bool) -> ScenarioEventConfig {
    let arm = vec![
        EventActionConfig::TimerStart(TimerStartActionConfig {
            key: TIMER_ORBIT_HOLD.to_string(),
            seconds: number(ORBIT_HOLD_SECS),
        }),
        sequence(
            SEQ_ORBIT_TALK,
            vec![
                step(
                    ORBIT_TALK_FIRST_AT,
                    vec![story_message(ENGINEER, story::ORBIT_ENGINEER_VIEW)],
                ),
                step(
                    ORBIT_TALK_GAP,
                    vec![story_message(COPILOT, story::ORBIT_COPILOT_LOG)],
                ),
                step(
                    ORBIT_TALK_GAP,
                    vec![story_message(PLAYER, story::ORBIT_PLAYER_LOG)],
                ),
            ],
        ),
    ];
    ScenarioEventConfig {
        label: None,
        name: event,
        once: false,
        filters: vec![
            cutter_enters(stage::ID_INSPECTION),
            number_equals(VAR_BEAT, BEAT_ORBIT),
        ],
        actions: if start {
            arm
        } else {
            vec![EventActionConfig::TimerCancel(TimerCancelActionConfig {
                key: TIMER_ORBIT_HOLD.to_string(),
            })]
        },
    }
}

/// The salvo: the whole attack, from the first tube to the empty channel.
///
/// It is a single chain because it is a single continuous event, and because
/// the cadence IS the writing. The shot list, in order:
///
/// 1. Tight on the WARSHIP, down its own firing line, while six bays walk open
///    and six torpedoes leave straight away from the camera.
/// 2. One line from the cockpit, which is the only thing said over the guns.
/// 3. Cut to the MERIDIAN. The two lances land first (a slug crosses 6.6 km in
///    under half a second), then the torpedoes arrive out of the same axis over
///    the following seconds. Nothing is said over an impact.
/// 4. Cut back to the CUTTER, three kilometres off, for the last torpedoes and
///    the kill - which is where the player watches it from, and which is not a
///    shot the engine can take once the carrier is gone.
/// 5. The camera comes home before a word of the aftermath.
///
/// The warship is left firing off screen on purpose. The beat is not a ship
/// shooting; it is a carrier coming apart with its crew still on the channel.
fn salvo() -> EventActionConfig {
    let bays = ships::BLOCK_WARSHIP_BAY_IDS;
    let mut steps = vec![step(
        0.0,
        vec![
            suspend_player_control(),
            film(ID_WARSHIP, CINEMA_TUBES_OFFSET, at(ID_CARRIER)),
            fire_bay(bays[0]),
        ],
    )];
    steps.extend(
        bays[1..]
            .iter()
            .map(|bay| step(SALVO_BAY_GAP, vec![fire_bay(bay)])),
    );
    steps.extend([
        step(
            SALVO_TUBES_CALL_AT,
            vec![story_message(PLAYER, story::ATTACK_PLAYER_TUBES)],
        ),
        step(
            SALVO_CUT_TO_CARRIER_AT,
            vec![film(ID_CARRIER, CINEMA_IMPACT_OFFSET, at(ID_WARSHIP))],
        ),
        step(
            SALVO_FIRST_LANCE_AT,
            vec![fire_railgun(ships::BLOCK_WARSHIP_RAILGUN_IDS[0])],
        ),
        step(
            SALVO_SECOND_LANCE_AT,
            vec![fire_railgun(ships::BLOCK_WARSHIP_RAILGUN_IDS[1])],
        ),
        step(
            SALVO_LAST_WORDS_AT,
            vec![story_message(DECK_CHIEF, story::ATTACK_CHIEF_LAST)],
        ),
        step(
            SALVO_POD_CALL_AT,
            vec![story_message(PLAYER, story::ATTACK_PLAYER_POD)],
        ),
        // Off the carrier before it dies, and back onto the cutter for the last
        // impacts. A camera anchored to a hull loses its anchor when that hull
        // is destroyed, so holding this shot to the end would hand the camera
        // back BY ITSELF on the one frame that matters.
        step(
            SALVO_CUT_TO_CUTTER_AT,
            vec![film(ID_CUTTER, CINEMA_DEATH_OFFSET, at(ID_CARRIER))],
        ),
        // The wreck has stopped moving by now. The camera goes back to the
        // cutter's own rig - the aftermath is the player's view, not a shot.
        step(
            SALVO_RELEASE_AT,
            vec![release_camera(), resume_player_control()],
        ),
        step(
            SALVO_EXIT_AT,
            vec![
                // Nothing waits on this arrival. The order exists so the ship
                // leaves under its own thrust, taking its time, entirely
                // unbothered - and OUTBOUND, clear of the large body it came
                // from, because a move order flies a straight line and will
                // fly it through a planetoid.
                EventActionConfig::MoveShipTo(MoveShipToActionConfig {
                    order: ORDER_EXIT.to_string(),
                    ship: ID_WARSHIP.to_string(),
                    position: WARSHIP_EXIT_POS,
                    arrival_standoff: None,
                }),
                story_message(PLAYER, story::AFTER_PLAYER_SAY_AGAIN),
            ],
        ),
        step(
            SALVO_SECOND_CALL_AT,
            vec![story_message(PLAYER, story::AFTER_PLAYER_ANYONE)],
        ),
        step(
            SALVO_SILENCE_AT,
            vec![
                complete_objective(OBJ_WITNESS),
                detach_objective_marker(ID_WARSHIP),
                post_objective(OBJ_SILENCE, story::OBJ_TEXT_SILENCE),
            ],
        ),
        step(
            SALVO_DISTRESS_AT,
            vec![set_variable(VAR_BEAT, number(BEAT_DISTRESS))],
        ),
    ]);
    sequence(SEQ_SALVO, steps)
}

/// The epilogue: the tease line, then the banner and the hand-off to chapter
/// two.
fn outro() -> EventActionConfig {
    pacing::outro_sequence(
        VAR_BEAT,
        BEAT_WON,
        BEACON,
        story::OUTRO_BEACON,
        story::OUTRO_BANNER,
        vec![post_objective(OBJ_DONE, story::OBJ_TEXT_DONE)],
        Some(SECOND_SHIFT_SCENARIO_ID.to_string()),
    )
}

/// The Defeat pair. The cutter is unarmed and nothing hunts it, so the only way
/// to lose this chapter is to fly into something - which the rock plate makes
/// entirely possible.
fn defeat(message: &str, event: EventConfig) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: event,
        once: true,
        filters: vec![entity(ID_CUTTER), number_less_than(VAR_BEAT, BEAT_OUTRO)],
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
