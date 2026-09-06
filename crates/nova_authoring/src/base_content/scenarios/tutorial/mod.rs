//! "Basic Training" - the range New Game opens on.
//!
//! A Fleet cadet on a gunnery range, talked through a qualification card by
//! Range Control: burn to a mark, STOP, slide across on the thrusters, travel-
//! lock the planetoid and let GOTO fly the leg, ORBIT it, GOTO home, then the
//! gun: lock a target, shoot it apart, clear the line, and beat two drones
//! that shoot back. No story. The range is the editor's stock world - rocks,
//! five hulks, two dormant pickets, a planetoid - so what a cadet learns here
//! is what a builder's range asks for.
//!
//! Script shape follows the mainline convention: one `beat` counter gates
//! every handler, and an objective posts a beat LATER than the line that
//! introduces it (see `pacing`). A lesson's verb is granted in that same
//! deferred step, beside its card and its beat number, so no handler is armed
//! before the card that names it exists. The gun has no verb to withhold, so
//! the two fire lessons are written to survive a cadet who shoots early: a
//! target that dies ahead of its lesson moves the card on instead of stalling
//! it. The lines live in `script` and the map in `range`, so a dialogue pass
//! and a layout pass are two separate edits.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

mod range;
mod script;
#[cfg(test)]
mod tests;

use range::*;
use script::{PLAYER, RANGE_CONTROL};

use super::pacing::{self, INSTRUCTION_GAP, MID_GAP, REVEAL_GAP};
use crate::scenario_helpers::prelude::*;

/// The scenario id New Game starts, and the id the retry hands back to.
pub const TUTORIAL_SCENARIO_ID: &str = "tutorial";

// --- objectives --------------------------------------------------------------

const OBJ_BURN: &str = "burn";
const OBJ_STOP: &str = "stop";
const OBJ_RCS: &str = "rcs";
const OBJ_NAV: &str = "nav_lock";
const OBJ_GOTO: &str = "goto";
const OBJ_ORBIT: &str = "orbit";
const OBJ_RETURN: &str = "return";
const OBJ_LOCK: &str = "lock";
const OBJ_FIRE: &str = "fire";
const OBJ_LINE: &str = "line";
const OBJ_LIVE: &str = "live";

// --- beats -------------------------------------------------------------------

/// The one counter every handler is gated on.
const VAR_BEAT: &str = "beat";
/// Targets destroyed, whichever beat they died in.
const VAR_SCRAPPED: &str = "targets_scrapped";
/// Whether Target 1 is gone: the two fire lessons read it to move on.
const VAR_TARGET_1_DOWN: &str = "target_1_down";
/// Drones defeated, whichever beat they fell in.
const VAR_DRONES_DOWN: &str = "drones_down";

/// The briefing, over the establishing shot.
const BEAT_BRIEF: f64 = 1.0;
/// Burn to ALPHA done; STOP granted.
const BEAT_STOP: f64 = 2.0;
/// At rest; RCS granted, BRAVO up.
const BEAT_RCS: f64 = 3.0;
/// On BRAVO; LOCK granted, the planetoid marked for a travel lock.
const BEAT_NAV: f64 = 4.0;
/// Travel-locked; GOTO granted, the leg out.
const BEAT_GOTO: f64 = 5.0;
/// Parked off the planetoid; ORBIT granted.
const BEAT_ORBIT: f64 = 6.0;
/// The orbit holds; CHARLIE up, the leg home.
const BEAT_RETURN: f64 = 7.0;
/// On CHARLIE; the gun's lock, Target 1 marked.
const BEAT_LOCK: f64 = 8.0;
/// Locked; shoot Target 1 apart.
const BEAT_FIRE: f64 = 9.0;
/// Target 1 gone; clear the line.
const BEAT_LINE: f64 = 10.0;
/// The line clear; the drones live.
const BEAT_LIVE: f64 = 11.0;
/// Both drones down; the epilogue runs.
const BEAT_OUTRO: f64 = 12.0;
/// The banner is up.
const BEAT_WON: f64 = 13.0;

// --- timings -----------------------------------------------------------------
//
// Authored timings, not physics: nudge them after playtest.

const SEQ_BRIEFING: &str = "briefing";
/// First line of the briefing, after the card has been on screen a beat.
const BRIEF_FIRST_AT: f64 = 2.0;
/// Between two Range Control lines.
const BRIEF_GAP: f64 = 4.5;
/// Before and after the cadet's acknowledgement.
const BRIEF_REPLY_GAP: f64 = 2.5;
/// How long the opening card stays up.
const OPEN_CARD_SECONDS: f32 = 8.0;
/// Opening view: off the trainer's port quarter, above, looking back at the
/// hull with the range behind it.
const OPEN_OFFSET: Meters3 = Meters3::new(-140.0, 50.0, 160.0);

/// The keybind-dock chips a lesson pulses.
const HINT_STOP: &str = "STOP";
const HINT_RCS: &str = "RCS";
const HINT_RADAR: &str = "RADAR";
const HINT_GOTO: &str = "GOTO";
const HINT_ORBIT: &str = "ORBIT";

// --- helpers -----------------------------------------------------------------

fn advance(beat: f64) -> EventActionConfig {
    set_variable(VAR_BEAT, number(beat))
}

fn in_beat(beat: f64) -> EventFilterConfig {
    number_equals(VAR_BEAT, beat)
}

/// The trainer paired with `id`: the shape a lock, a GOTO arrival, an orbit
/// and an area event all report, the target first.
fn trainer_at(id: impl Into<String>) -> EventFilterConfig {
    entity_pair(id, ID_TRAINER)
}

/// A lesson's world - its beat number, its verb, its card, its marks, its
/// hint - landing `delay` after the line that introduced it. The sequence key
/// is the beat's own, and the beats are unique, so the chains never collide.
fn lesson_later(beat: f64, delay: f64, actions: Vec<EventActionConfig>) -> EventActionConfig {
    pacing::beat_later(&format!("beat_{beat}"), delay, actions)
}

fn grant(verb: FlightVerb) -> EventActionConfig {
    EventActionConfig::SetControllerVerb(SetControllerVerbActionConfig {
        id: ID_TRAINER.to_string(),
        verb,
        enabled: true,
    })
}

fn range_line(after: f64, line: &str) -> SequenceStepConfig {
    step(after, vec![comms(RANGE_CONTROL, line)])
}

fn player_line(after: f64, line: &str) -> SequenceStepConfig {
    step(after, vec![comms(PLAYER, line)])
}

/// Hang the camera off the trainer for the briefing. Camera authority only:
/// the beat suspends control before the shot and restores it when the view
/// is handed back.
fn film_trainer() -> EventActionConfig {
    EventActionConfig::SetCameraAnchor(SetCameraAnchorActionConfig {
        blend: None,
        anchor: ID_TRAINER.to_string(),
        offset: OPEN_OFFSET,
        frame: CameraOffsetFrame::World,
        look_at: CameraLookAtConfig::Object(ID_TRAINER.to_string()),
    })
}

fn release_camera() -> EventActionConfig {
    EventActionConfig::ReleaseCamera(ReleaseCameraActionConfig)
}

fn suspend_player_control() -> EventActionConfig {
    EventActionConfig::SuspendPlayerControl(SuspendPlayerControlActionConfig)
}

fn resume_player_control() -> EventActionConfig {
    EventActionConfig::ResumePlayerControl(ResumePlayerControlActionConfig)
}

fn once(
    name: EventConfig,
    filters: Vec<EventFilterConfig>,
    actions: Vec<EventActionConfig>,
) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name,
        once: true,
        filters,
        actions,
    }
}

/// The line's card: every target still standing gets a marker, and the beat
/// moves on. Shared by both fire lessons, whichever one Target 1 died in.
fn line_setup() -> Vec<EventActionConfig> {
    let mut actions = vec![
        advance(BEAT_LINE),
        complete_objective(OBJ_FIRE),
        post_objective(OBJ_LINE, script::OBJ_TEXT_LINE),
    ];
    actions.extend(
        (2..=TARGET_COUNT).map(|nth| attach_objective_marker(target_id(nth), target_label(nth))),
    );
    actions
}

/// The Defeat pair. The card can be failed by flying into a rock, or by
/// losing the gun to a drone.
fn defeat(message: &str, event: EventConfig) -> ScenarioEventConfig {
    once(
        event,
        vec![entity(ID_TRAINER), number_less_than(VAR_BEAT, BEAT_OUTRO)],
        vec![
            EventActionConfig::Outcome(OutcomeActionConfig::new(
                ScenarioOutcomeKind::Defeat,
                message,
            )),
            EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: TUTORIAL_SCENARIO_ID.to_string(),
                linger: true,
                delay: None,
            }),
        ],
    )
}

/// The epilogue: the tease line, then the banner. Nothing is handed off to:
/// the card ends, and the Scenarios board is where the cadet goes next.
fn outro() -> EventActionConfig {
    pacing::outro_sequence(
        VAR_BEAT,
        BEAT_WON,
        RANGE_CONTROL,
        script::OUTRO_TEASE,
        script::OUTRO_BANNER,
        vec![],
        None,
    )
}

/// The two faces on the channel. Both are the base game's own art.
fn portrait(speaker: &str) -> Option<AssetRef<Image>> {
    let name = match speaker {
        RANGE_CONTROL => "range-control",
        PLAYER => "player",
        _ => return None,
    };
    Some(AssetRef::from(format!("self://portraits/{name}.png")))
}

fn apply_portraits(events: &mut [ScenarioEventConfig]) {
    for event in events {
        for action in &mut event.actions {
            action.walk_mut(&mut |action| {
                if let EventActionConfig::NarrativeCue(cue) = action {
                    if cue.icon.is_none() {
                        cue.icon = portrait(&cue.speaker);
                    }
                }
            });
        }
    }
}

// --- the card ----------------------------------------------------------------

/// Build the training range.
pub(crate) fn tutorial(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut spawns = vec![trainer(), planetoid()];
    spawns.extend((1..=TARGET_COUNT).map(target_hulk));
    spawns.extend(
        DRONES
            .iter()
            .map(|(id, name, position)| drone(id, name, *position)),
    );
    spawns.extend(lights());

    let mut start = spawns
        .into_iter()
        .map(EventActionConfig::SpawnScenarioObject)
        .collect::<Vec<_>>();
    start.extend(belts(&asteroid_texture));
    start.push(raise_range_boundary());
    start.extend([
        advance(BEAT_BRIEF),
        set_number(VAR_SCRAPPED, 0.0),
        set_number(VAR_TARGET_1_DOWN, 0.0),
        set_number(VAR_DRONES_DOWN, 0.0),
    ]);
    start.extend(
        DRONES
            .iter()
            .map(|(id, _, _)| set_number(drone_down_var(id), 0.0)),
    );
    start.extend([
        suspend_player_control(),
        film_trainer(),
        title(
            ScreenCornerConfig::TopLeft,
            script::OPEN_CARD_PLACE,
            script::OPEN_CARD_WHEN,
            script::OPEN_CARD_NOTE,
            OPEN_CARD_SECONDS,
        ),
        sequence(
            SEQ_BRIEFING,
            vec![
                range_line(BRIEF_FIRST_AT, script::BRIEF_HELLO),
                range_line(BRIEF_GAP, script::BRIEF_CARD),
                player_line(BRIEF_REPLY_GAP, script::BRIEF_READY),
                range_line(BRIEF_REPLY_GAP, script::BRIEF_HELM),
                // The helm and the first card land together, a beat after the
                // line that hands them over.
                step(
                    INSTRUCTION_GAP,
                    [
                        release_camera(),
                        resume_player_control(),
                        post_objective(OBJ_BURN, script::OBJ_TEXT_BURN),
                    ]
                    .into_iter()
                    .chain(MARK_ALPHA.raise())
                    .chain([MARK_ALPHA.raise_gate()])
                    .collect(),
                ),
            ],
        ),
    ]);

    let mut events = vec![
        // The range, the counters, and the briefing. No objective competes
        // with the conversation; the trainer holds the establishing frame
        // until Range Control hands the helm over.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: true,
            filters: vec![],
            actions: start,
        },
        // ALPHA: the burn is done, and STOP is the next thing on the card.
        once(
            EventConfig::OnEnter,
            vec![MARK_ALPHA.gate_entered(), in_beat(BEAT_BRIEF)],
            [complete_objective(OBJ_BURN)]
                .into_iter()
                .chain(MARK_ALPHA.clear())
                .chain([
                    comms(RANGE_CONTROL, script::STOP_LINE),
                    lesson_later(
                        BEAT_STOP,
                        INSTRUCTION_GAP,
                        vec![
                            advance(BEAT_STOP),
                            grant(FlightVerb::Stop),
                            post_objective(OBJ_STOP, script::OBJ_TEXT_STOP),
                            show_hint_emphasis(HINT_STOP),
                        ],
                    ),
                ])
                .collect(),
        ),
        // At rest: the thrusters, across to BRAVO.
        once(
            EventConfig::OnStopComplete,
            vec![entity(ID_TRAINER), in_beat(BEAT_STOP)],
            vec![
                clear_hint_emphasis(HINT_STOP),
                complete_objective(OBJ_STOP),
                comms(RANGE_CONTROL, script::RCS_LINE),
                lesson_later(
                    BEAT_RCS,
                    INSTRUCTION_GAP,
                    [
                        advance(BEAT_RCS),
                        grant(FlightVerb::Rcs),
                        post_objective(OBJ_RCS, script::OBJ_TEXT_RCS),
                    ]
                    .into_iter()
                    .chain(MARK_BRAVO.raise())
                    .chain([MARK_BRAVO.raise_gate(), show_hint_emphasis(HINT_RCS)])
                    .collect(),
                ),
            ],
        ),
        // BRAVO: the pattern is flown, and the flight computer is next. First
        // its designation: the white travel lock, on the planetoid.
        once(
            EventConfig::OnEnter,
            vec![MARK_BRAVO.gate_entered(), in_beat(BEAT_RCS)],
            [clear_hint_emphasis(HINT_RCS), complete_objective(OBJ_RCS)]
                .into_iter()
                .chain(MARK_BRAVO.clear())
                .chain([
                    comms(RANGE_CONTROL, script::NAV_LINE),
                    lesson_later(
                        BEAT_NAV,
                        INSTRUCTION_GAP,
                        vec![
                            advance(BEAT_NAV),
                            grant(FlightVerb::Lock),
                            post_objective(OBJ_NAV, script::OBJ_TEXT_NAV),
                            attach_objective_marker(ID_PLANETOID, PLANETOID_LABEL),
                            show_hint_emphasis(HINT_RADAR),
                        ],
                    ),
                ])
                .collect(),
        ),
        // A red lock on the planetoid: the sweep landed with weapons raised.
        once(
            EventConfig::OnCombatLockStart,
            vec![trainer_at(ID_PLANETOID), in_beat(BEAT_NAV)],
            vec![comms(RANGE_CONTROL, script::NAV_COMBAT_NUDGE)],
        ),
        // Travel-locked: GOTO has a place to fly to.
        once(
            EventConfig::OnTravelLockStart,
            vec![trainer_at(ID_PLANETOID), in_beat(BEAT_NAV)],
            vec![
                clear_hint_emphasis(HINT_RADAR),
                complete_objective(OBJ_NAV),
                comms(RANGE_CONTROL, script::GOTO_LINE),
                lesson_later(
                    BEAT_GOTO,
                    INSTRUCTION_GAP,
                    vec![
                        advance(BEAT_GOTO),
                        grant(FlightVerb::Goto),
                        post_objective(OBJ_GOTO, script::OBJ_TEXT_GOTO),
                        show_hint_emphasis(HINT_GOTO),
                    ],
                ),
            ],
        ),
        // Parked off the planetoid, inside its pull: ORBIT.
        once(
            EventConfig::OnGotoComplete,
            vec![trainer_at(ID_PLANETOID), in_beat(BEAT_GOTO)],
            vec![
                clear_hint_emphasis(HINT_GOTO),
                complete_objective(OBJ_GOTO),
                comms(RANGE_CONTROL, script::ORBIT_LINE),
                lesson_later(
                    BEAT_ORBIT,
                    INSTRUCTION_GAP,
                    vec![
                        advance(BEAT_ORBIT),
                        grant(FlightVerb::Orbit),
                        post_objective(OBJ_ORBIT, script::OBJ_TEXT_ORBIT),
                        show_hint_emphasis(HINT_ORBIT),
                    ],
                ),
            ],
        ),
        // The orbit holds: the leg home, the same two keys on a mark. The
        // gate, not the arrival, closes it: a cadet who flies home by hand is
        // home all the same.
        once(
            EventConfig::OnOrbitStable,
            vec![trainer_at(ID_PLANETOID), in_beat(BEAT_ORBIT)],
            vec![
                clear_hint_emphasis(HINT_ORBIT),
                complete_objective(OBJ_ORBIT),
                detach_objective_marker(ID_PLANETOID),
                comms(RANGE_CONTROL, script::RETURN_LINE),
                lesson_later(
                    BEAT_RETURN,
                    INSTRUCTION_GAP,
                    [
                        advance(BEAT_RETURN),
                        post_objective(OBJ_RETURN, script::OBJ_TEXT_RETURN),
                    ]
                    .into_iter()
                    .chain(MARK_CHARLIE.raise())
                    .chain([MARK_CHARLIE.raise_gate(), show_hint_emphasis(HINT_GOTO)])
                    .collect(),
                ),
            ],
        ),
        // A red lock on CHARLIE from orbit: the computer flies only the white one.
        once(
            EventConfig::OnCombatLockStart,
            vec![trainer_at(MARK_CHARLIE.id), in_beat(BEAT_RETURN)],
            vec![comms(RANGE_CONTROL, script::RETURN_COMBAT_NUDGE)],
        ),
        // CHARLIE: back on the line, and the gun's lock is next.
        once(
            EventConfig::OnEnter,
            vec![MARK_CHARLIE.gate_entered(), in_beat(BEAT_RETURN)],
            [
                clear_hint_emphasis(HINT_GOTO),
                complete_objective(OBJ_RETURN),
            ]
            .into_iter()
            .chain(MARK_CHARLIE.clear())
            .chain([
                comms(RANGE_CONTROL, script::LOCK_LINE),
                lesson_later(
                    BEAT_LOCK,
                    INSTRUCTION_GAP,
                    vec![
                        advance(BEAT_LOCK),
                        post_objective(OBJ_LOCK, script::OBJ_TEXT_LOCK),
                        attach_objective_marker(target_id(1), target_label(1)),
                        show_hint_emphasis(HINT_RADAR),
                    ],
                ),
            ])
            .collect(),
        ),
        // A travel lock on Target 1: the sweep landed with weapons lowered.
        once(
            EventConfig::OnTravelLockStart,
            vec![trainer_at(target_id(1)), in_beat(BEAT_LOCK)],
            vec![comms(RANGE_CONTROL, script::LOCK_TRAVEL_NUDGE)],
        ),
        // Combat-locked: the range goes hot.
        once(
            EventConfig::OnCombatLockStart,
            vec![trainer_at(target_id(1)), in_beat(BEAT_LOCK)],
            vec![
                clear_hint_emphasis(HINT_RADAR),
                complete_objective(OBJ_LOCK),
                comms(RANGE_CONTROL, script::FIRE_LINE),
                lesson_later(
                    BEAT_FIRE,
                    INSTRUCTION_GAP,
                    vec![
                        advance(BEAT_FIRE),
                        post_objective(OBJ_FIRE, script::OBJ_TEXT_FIRE),
                    ],
                ),
            ],
        ),
        // Target 1 shot apart before the lock landed: the lock lesson is not
        // repeated on a target that no longer exists, and the card moves on.
        once(
            EventConfig::OnUpdate,
            vec![
                in_beat(BEAT_LOCK),
                number_greater_than(VAR_TARGET_1_DOWN, 0.5),
            ],
            vec![
                clear_hint_emphasis(HINT_RADAR),
                complete_objective(OBJ_LOCK),
                comms(RANGE_CONTROL, script::SCRAP_EARLY_LINE),
                lesson_later(BEAT_LINE, MID_GAP, line_setup()),
            ],
        ),
        // Target 1 down: the rest of the line.
        once(
            EventConfig::OnUpdate,
            vec![
                in_beat(BEAT_FIRE),
                number_greater_than(VAR_TARGET_1_DOWN, 0.5),
            ],
            vec![
                complete_objective(OBJ_FIRE),
                comms(RANGE_CONTROL, script::SCRAP_LINE),
                lesson_later(BEAT_LINE, MID_GAP, line_setup()),
            ],
        ),
        // The line clear: the drones go live a reveal later.
        once(
            EventConfig::OnUpdate,
            vec![
                in_beat(BEAT_LINE),
                number_greater_than(VAR_SCRAPPED, TARGET_COUNT as f64 - 0.5),
            ],
            vec![
                complete_objective(OBJ_LINE),
                comms(RANGE_CONTROL, script::LIVE_LINE),
                lesson_later(
                    BEAT_LIVE,
                    REVEAL_GAP,
                    [advance(BEAT_LIVE)]
                        .into_iter()
                        .chain(DRONES.iter().map(|(id, _, _)| wake_drone(id)))
                        .chain([post_objective(OBJ_LIVE, script::OBJ_TEXT_LIVE)])
                        .chain(
                            DRONES
                                .iter()
                                .map(|(id, name, _)| attach_objective_marker(*id, *name)),
                        )
                        .collect(),
                ),
            ],
        ),
        // Both drones down: the card is passed.
        once(
            EventConfig::OnUpdate,
            vec![
                in_beat(BEAT_LIVE),
                number_greater_than(VAR_DRONES_DOWN, DRONES.len() as f64 - 0.5),
            ],
            vec![
                advance(BEAT_OUTRO),
                complete_objective(OBJ_LIVE),
                comms(RANGE_CONTROL, script::WON_LINE),
                outro(),
            ],
        ),
    ];

    // The tallies. Ungated: a target counts whenever it dies, so a cadet who
    // works ahead of the card is never asked to do it again.
    for nth in 1..=TARGET_COUNT {
        let mut actions = vec![
            increment_variable(VAR_SCRAPPED),
            detach_objective_marker(target_id(nth)),
        ];
        if nth == 1 {
            actions.push(set_number(VAR_TARGET_1_DOWN, 1.0));
        }
        events.push(once(
            EventConfig::OnDestroyed,
            vec![entity(target_id(nth))],
            actions,
        ));
    }
    // A drone counts once, whichever way it goes: defeated where the cadet
    // can see it, or crippled and coasting off the range. Its flag is what
    // keeps a drone that is disarmed and then drifts out from counting twice,
    // and what keeps the boundary's line off a drone that blew up inside it.
    for (id, _, _) in DRONES {
        let down = drone_down_var(id);
        let tally = |extra: Vec<EventActionConfig>| -> Vec<EventActionConfig> {
            [
                set_number(&down, 1.0),
                increment_variable(VAR_DRONES_DOWN),
                detach_objective_marker(id),
            ]
            .into_iter()
            .chain(extra)
            .collect()
        };
        events.push(once(
            EventConfig::OnDefeated,
            vec![entity(id), number_equals(&down, 0.0)],
            tally(vec![]),
        ));
        events.push(once(
            EventConfig::OnExit,
            vec![
                drone_left_range(id),
                in_beat(BEAT_LIVE),
                number_equals(&down, 0.0),
            ],
            tally(vec![
                comms(RANGE_CONTROL, script::DRONE_ADRIFT_LINE),
                despawn_object(id),
            ]),
        ));
    }
    events.push(defeat(script::DEFEAT_DESTROYED, EventConfig::OnDestroyed));
    events.push(defeat(
        script::DEFEAT_NEUTRALIZED,
        EventConfig::OnNeutralized,
    ));
    apply_portraits(&mut events);

    ScenarioConfig {
        description: "Fleet gunnery qualification: fly the pattern, take the flight computer \
                      out to the planetoid and back, clear the line of target hulks, then \
                      beat two live drones. Range Control talks you through it."
            .to_string(),
        thumbnail: Some(AssetRef::from("self://thumbnails/tutorial.png")),
        events,
        ..ScenarioConfig::new(
            TUTORIAL_SCENARIO_ID.to_string(),
            "Basic Training".to_string(),
            cubemap,
        )
    }
}
