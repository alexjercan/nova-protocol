//! Season 1, chapter one scenario configuration.
//!
//! The event flow combines a low-speed obstacle lane, a course change, a
//! docking action, and a timed transfer. The player ship has no weapons. The
//! layout tests lateral hull placement and a controlled docking approach.
//!
//! Script shape follows the mainline convention: one `beat` counter gates
//! every handler, and an objective posts a beat LATER than the line that
//! introduces it (see `pacing`). The lines live in `script` and the map in
//! `stage`, so a dialogue pass and a layout pass are two separate edits.
//!
//! The transfer is written as gated timers rather than a sequence. A sequence
//! runs to its end after the clamp is released. Each timer checks the docking
//! state, so an early release pauses progress and re-docking resumes it.
//!
//! Portraits are attached in one pass over finished events because the speaker
//! key selects the image through `script::portrait`. Accent selection remains
//! independent: `crew` uses instrument green and `comms` uses radio blue.

use bevy::prelude::Image;
use nova_events::prelude::Meters3;
use nova_gameplay::prelude::AssetRef;
use nova_scenario::prelude::*;

mod script;
pub(crate) mod stage;
#[cfg(test)]
mod tests;

use stage::*;

use super::{pacing, HINT_DOCK, HINT_RADAR, HINT_RCS};
use crate::scenario_helpers::prelude::*;

/// The chapter's scenario id: the campaign's first member, and the id the
/// retry hands back to.
pub const CHAPTER_ONE_SCENARIO_ID: &str = "season_one_chapter_one";

// --- objectives --------------------------------------------------------------

const OBJ_LANE: &str = "lane";
const OBJ_REACH: &str = "reach";
const OBJ_DOCK: &str = "dock";
const OBJ_HOLD: &str = "hold";
const OBJ_RELEASE: &str = "release";

// --- beats -------------------------------------------------------------------

/// The one counter every handler is gated on.
const VAR_BEAT: &str = "beat";

/// The opening scene is running.
const BEAT_OPEN: f64 = 1.0;
/// One beat per mark of the lane. A mark's gate is raised by the beat before
/// it, so a player who flies through a volume early finds nothing there.
const LANE_BEATS: [f64; 4] = [2.0, 3.0, 4.0, 5.0];
/// The lane is complete; the call scene and its branch run.
const BEAT_CALL: f64 = 6.0;
/// The intercept: come about and close on Gantry.
const BEAT_REACH: f64 = 7.0;
/// Standing off Gantry, with the collar to find. The docking card is up for
/// the whole of this beat: every path INTO it posts the card in the same frame
/// it sets the beat, which is what makes completing that card on the clamp
/// safe.
const BEAT_DOCK: f64 = 8.0;
/// Let go of the collar before the transfer is done, and the chapter stands
/// here until the card comes back: the release has fired, its lines have run,
/// and the ask is still a beat away.
///
/// A beat of its own rather than an early return to [`BEAT_DOCK`], because a
/// player who lets go and clamps again in the same second would otherwise
/// complete a card that was never posted and then be handed one that nothing
/// can take down.
const BEAT_REGRIP: f64 = 8.5;
/// Clamped, before the timed transfer's first beat.
const BEAT_HOLD: f64 = 9.0;
/// Clamped, with people crossing and the hold objective posted.
const BEAT_TRANSFER: f64 = 10.0;
/// All three aboard: let go and go home.
const BEAT_RELEASE: f64 = 11.0;
/// The epilogue runs.
const BEAT_OUTRO: f64 = 12.0;
/// The banner is up.
const BEAT_WON: f64 = 13.0;

// --- timings -----------------------------------------------------------------
//
// Authored timings, not physics: nudge them after playtest.
//
// The gaps below are all at least six seconds, and they are gaps between
// ARRIVALS rather than holds. The comms
// panel keeps a card for eight seconds and shows three at once, so it was
// never the panel rushing the reader: a four-second gap simply put a new card
// on the stack while the two before it were still worth reading, and the
// newest one shunted them upward as it landed. At six or seven the card being
// read is usually the only one moving.

/// The opening scene, and how long its card stays up.
const SCENE_OPEN: &str = "open";
const OPEN_CARD_SECONDS: f32 = 9.0;
/// Opening view: off Kaveri's starboard quarter and above, looking back along
/// the hull at the lane it is about to fly.
const OPEN_OFFSET: Meters3 = Meters3::new(120.0, 45.0, 150.0);
/// First line of the scene, after the card has been on screen a beat.
const OPEN_FIRST_AT: f64 = 2.5;
/// Between two lines with the same visual accent.
const OPEN_GAP: f64 = 6.5;
/// Around an answer or a second card from the same speaker. This gap is shorter
/// because the card sequence continues.
const OPEN_REPLY_GAP: f64 = 5.0;

/// A mark's arrival line -> the next mark. Short: the ship is still moving,
/// and the lane is one continuous run rather than seven separate lessons.
const LANE_GAP: f64 = 3.0;

/// The call scene and its line gaps. It is longer than the lane sequence to
/// keep successive cards readable.
///
/// A SCENE takes control, stops the player ship, and holds one camera framing.
/// The target hull's damage must remain legible from this shot.
const SCENE_CALL: &str = "call";
/// The shot: off Gantry's starboard BOW and above, looking aft down the length
/// of the hull.
///
/// From here the hull reads intact at the near end and open at the far one -
/// the stack gone, the arch cut in half, the transom empty where the drive
/// was - with its own plating drifting past the stern behind it. Standing off
/// the stern instead would put that plating between the lens and the ship.
/// Kaveri arrives on the opposite flank, so the player never flies this
/// angle.
const CALL_OFFSET: Meters3 = Meters3::new(150.0, 45.0, -120.0);
/// LANE-4 -> the traffic. Long enough that the lane feels finished first.
const CALL_OPEN_AT: f64 = 6.0;
/// Between two lines of the call.
const CALL_GAP: f64 = 6.5;
/// Around a short answer or the second half of a split line.
const CALL_REPLY_GAP: f64 = 5.0;
/// The order line -> the course change on the HUD. Long enough that the line
/// is still on screen, and read, when the objective chip pops under it.
const CALL_ORDER_GAP: f64 = 4.5;

/// The approach chain: the port check, and the commitment. It runs from the
/// INNER ring (`STANDOFF`), not from the arrival gate - see that mark.
const SEQ_APPROACH: &str = "approach";
const NEAR_FIRST_AT: f64 = 2.0;
const NEAR_GAP: f64 = 6.0;
/// "We come to you" -> the docking card. The line commits, the card names the
/// goal, and the DOCK chip under it carries the key.
const NEAR_DOCK_GAP: f64 = 5.0;

/// The timer the docking card is posted on, rather than the last step of the
/// approach sequence itself.
///
/// The card has to be able to NOT arrive. A player already on the collar when
/// this lands has moved the beat past `BEAT_REACH`, and a pending
/// timer whose beat has moved fires into a handler that no longer matches - the
/// same timer mechanism as the transfer. A sequence step would have run
/// regardless and posted a card for a dock that had already happened.
const TIMER_DOCK: &str = "dock_card";

/// The timed transfer beats, in order, as timer keys. Each one is armed by the
/// beat before it and gated on the clamp still being on.
const TRANSFER_KEYS: [&str; 5] = [
    "transfer_start",
    "transfer_owen",
    "transfer_aboard",
    "transfer_all",
    "transfer_done",
];
/// The clamp -> the hold card.
const TRANSFER_CARD_AFTER: f64 = 4.0;
/// Between two beats of people crossing. The helm is dead while the clamp is
/// on, so this is the one stretch where reading costs nothing - but it is read
/// against a static picture, which is its own kind of slow.
const TRANSFER_GAP: f64 = 6.0;
/// The last head count -> the release card.
const TRANSFER_DONE_AFTER: f64 = 4.0;

/// An early release -> the card asking for the collar again. A timer rather
/// than a sequence, so the beat it lands in is checked when it lands: a player
/// back on the collar by then is not asked to dock again.
const TIMER_REGRIP: &str = "regrip";
const REGRIP_GAP: f64 = 2.5;

// --- helpers -----------------------------------------------------------------

fn advance(beat: f64) -> EventActionConfig {
    set_variable(VAR_BEAT, number(beat))
}

fn in_beat(beat: f64) -> EventFilterConfig {
    number_equals(VAR_BEAT, beat)
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

/// Give every cue the face of whoever speaks it.
///
/// One pass over the finished graph, the tutorial's rule: a line that authored
/// its own icon keeps it, and a speaker with no portrait draws the panel's
/// fallback tile instead of a wrong face.
fn apply_portraits(events: &mut [ScenarioEventConfig]) {
    for event in events {
        for action in &mut event.actions {
            action.walk_mut(&mut |action| {
                let EventActionConfig::NarrativeCue(cue) = action else {
                    return;
                };
                if cue.icon.is_none() {
                    cue.icon = script::portrait(&cue.speaker)
                        .map(|name| AssetRef::from(format!("self://portraits/{name}.png")));
                }
            });
        }
    }
}

/// A handler that may fire again: the dock, the release, and every beat of the
/// transfer. The player may let go and come back, so the chapter has to be
/// able to run the same beats a second time.
fn repeatable(
    name: EventConfig,
    filters: Vec<EventFilterConfig>,
    actions: Vec<EventActionConfig>,
) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name,
        once: false,
        filters,
        actions,
    }
}

/// The clamp, by the pair that made it: Kaveri asked, Gantry answered.
fn clamp() -> EventFilterConfig {
    entity_pair(ID_GANTRY, ID_KAVERI)
}

fn crew_line(after: f64, speaker: &str, line: &str) -> SequenceStepConfig {
    step(after, vec![crew(speaker, line)])
}

fn radio_line(after: f64, speaker: &str, line: &str) -> SequenceStepConfig {
    step(after, vec![comms(speaker, line)])
}

/// One timed-transfer beat: its line and the next timer key.
///
/// Gated on `beat` as well as on its own timer, which is the whole point of
/// writing this chain on timers: the release handler moves the beat back, and
/// a pending timer whose beat has moved fires into a handler that no longer
/// matches. The chain stops where it stands instead of narrating a transfer
/// that is no longer happening.
fn transfer_beat(
    key: &str,
    beat: f64,
    actions: Vec<EventActionConfig>,
    next: Option<(&str, f64)>,
) -> ScenarioEventConfig {
    let mut actions = actions;
    if let Some((next_key, after)) = next {
        actions.push(start_timer(next_key, after));
    }
    repeatable(
        EventConfig::OnTimerEnd,
        vec![timer(key), in_beat(beat)],
        actions,
    )
}

/// Let go of the collar before the transfer is done: the chapter parks on
/// [`BEAT_REGRIP`] until the card comes back.
///
/// `posted` carries the hold objective when there is one to take down - the
/// release can land before the card does, and completing an objective that was
/// never posted is a warning the chapter should not print.
fn early_release(beat: f64, posted: Vec<EventActionConfig>) -> ScenarioEventConfig {
    let mut actions = posted;
    actions.extend([
        crew(script::RINA, script::HOLD_EARLY_RELEASE),
        advance(BEAT_REGRIP),
        start_timer(TIMER_REGRIP, REGRIP_GAP),
    ]);
    repeatable(
        EventConfig::OnUndocked,
        vec![clamp(), in_beat(beat)],
        actions,
    )
}

/// The Defeat pair. Nothing in the chapter shoots, so the only way to lose is
/// to fly the workship into something: a rock, or Gantry.
fn defeat(id: &str, message: &str) -> ScenarioEventConfig {
    once(
        EventConfig::OnDestroyed,
        vec![entity(id), number_less_than(VAR_BEAT, BEAT_OUTRO)],
        vec![
            EventActionConfig::Outcome(OutcomeActionConfig::new(
                ScenarioOutcomeKind::Defeat,
                message,
            )),
            EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: CHAPTER_ONE_SCENARIO_ID.to_string(),
                linger: true,
                delay: None,
            }),
        ],
    )
}

// --- the chapter -------------------------------------------------------------

/// Build chapter one.
pub(crate) fn chapter_one(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut spawns = vec![kaveri(), gantry()];
    spawns.extend(wreckage());
    spawns.extend(moons());
    spawns.extend(lights(CHAPTER_ONE_SCENARIO_ID));

    let mut start = spawns
        .into_iter()
        .map(EventActionConfig::SpawnScenarioObject)
        .collect::<Vec<_>>();
    start.extend(rock(&asteroid_texture));
    start.extend([
        advance(BEAT_OPEN),
        EventActionConfig::SuspendPlayerControl(SuspendPlayerControlActionConfig),
        EventActionConfig::SetCameraAnchor(SetCameraAnchorActionConfig {
            blend: None,
            anchor: ID_KAVERI.to_string(),
            offset: OPEN_OFFSET,
            frame: CameraOffsetFrame::World,
            look_at: CameraLookAtConfig::Object(ID_KAVERI.to_string()),
        }),
        // The scene may be walked out of: a player who has read the book, or
        // flown the chapter once, does not owe it a second viewing. What it
        // leaves behind is authored on the finish handler, which a skip fires
        // too.
        cinematic(
            SCENE_OPEN,
            true,
            vec![
                step(
                    0.0,
                    vec![title(
                        ScreenCornerConfig::TopLeft,
                        script::OPEN_CARD_PLACE,
                        script::OPEN_CARD_WHEN,
                        script::OPEN_CARD_NOTE,
                        OPEN_CARD_SECONDS,
                    )],
                ),
                crew_line(OPEN_FIRST_AT, script::RINA, script::OPEN_LOAD),
                crew_line(OPEN_GAP, script::TOMAS, script::OPEN_LANE),
                crew_line(OPEN_REPLY_GAP, script::TOMAS, script::OPEN_CHOICE),
                crew_line(OPEN_GAP, script::LEILA, script::OPEN_COST),
                crew_line(OPEN_REPLY_GAP, script::JONAH, script::OPEN_ORDER),
                crew_line(OPEN_GAP, script::TOMAS, script::OPEN_MARKS),
                crew_line(OPEN_REPLY_GAP, script::TOMAS, script::OPEN_HANDOVER),
            ],
        ),
    ]);

    let mut events = vec![
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: true,
            filters: vec![],
            actions: start,
        },
        // Every way out of the opening scene ends here: the camera and the
        // helm come back, and the lane's first mark is up.
        once(
            EventConfig::OnCinematicFinished,
            vec![scene(SCENE_OPEN)],
            [
                EventActionConfig::ReleaseCamera(ReleaseCameraActionConfig),
                EventActionConfig::ResumePlayerControl(ResumePlayerControlActionConfig),
                advance(LANE_BEATS[0]),
                post_objective(OBJ_LANE, script::OBJ_TEXT_LANE),
            ]
            .into_iter()
            .chain(LANE[0].raise())
            .chain([LANE[0].raise_gate(), show_hint_emphasis(HINT_RCS)])
            .collect(),
        ),
    ];

    // The lane. Each mark takes itself down, says one line, and puts the next
    // one up a beat later - which is also what arms the handler for it.
    let lane_lines = [
        script::LANE_ONE,
        script::LANE_TWO,
        script::LANE_THREE,
        script::LANE_FOUR,
    ];
    let lane_voices = [script::TOMAS, script::LEILA, script::RINA, script::TOMAS];
    for index in 0..LANE.len() - 1 {
        let next = index + 1;
        events.push(once(
            EventConfig::OnEnter,
            vec![
                LANE[index].entered_by(ID_KAVERI),
                in_beat(LANE_BEATS[index]),
            ],
            LANE[index]
                .clear()
                .into_iter()
                .chain([
                    crew(lane_voices[index], lane_lines[index]),
                    pacing::beat_later(
                        &format!("beat_{}", LANE_BEATS[next]),
                        LANE_GAP,
                        [advance(LANE_BEATS[next])]
                            .into_iter()
                            .chain(LANE[next].raise())
                            .chain([LANE[next].raise_gate()])
                            .collect(),
                    ),
                ])
                .collect(),
        ));
    }

    let last = LANE.len() - 1;
    events.extend([
        // LANE-4: the lane is behind them, and the call comes in. No card
        // competes with it - the next thing the player is asked for is the
        // course change at the end of the chain.
        once(
            EventConfig::OnEnter,
            vec![LANE[last].entered_by(ID_KAVERI), in_beat(LANE_BEATS[last])],
            LANE[last]
                .clear()
                .into_iter()
                .chain([
                    clear_hint_emphasis(HINT_RCS),
                    complete_objective(OBJ_LANE),
                    advance(BEAT_CALL),
                    crew(script::TOMAS, script::LANE_FOUR),
                    // The helm, the ship's speed and the camera, in that
                    // order and all in the same frame. The cut is what makes
                    // the stop free: the player never watches a loaded
                    // workship drop to a dead stop in one tick, they are
                    // already looking at Gantry when it happens.
                    EventActionConfig::SuspendPlayerControl(SuspendPlayerControlActionConfig),
                    EventActionConfig::ZeroShipMotion(ZeroShipMotionActionConfig {
                        id: ID_KAVERI.to_string(),
                    }),
                    EventActionConfig::SetCameraAnchor(SetCameraAnchorActionConfig {
                        blend: None,
                        anchor: ID_GANTRY.to_string(),
                        offset: CALL_OFFSET,
                        frame: CameraOffsetFrame::World,
                        look_at: CameraLookAtConfig::Object(ID_GANTRY.to_string()),
                    }),
                    cinematic(
                        SCENE_CALL,
                        true,
                        vec![
                            crew_line(CALL_OPEN_AT, script::SAMIR, script::CALL_TRAFFIC),
                            radio_line(CALL_GAP, script::NADIA, script::CALL_MAYDAY),
                            crew_line(CALL_REPLY_GAP, script::SAMIR, script::CALL_KNOWN),
                            crew_line(CALL_REPLY_GAP, script::JONAH, script::CALL_ANSWER),
                            radio_line(CALL_GAP, script::NADIA, script::CALL_THREE),
                            crew_line(CALL_GAP, script::TOMAS, script::CALL_COST),
                            crew_line(CALL_REPLY_GAP, script::TOMAS, script::CALL_PRICE),
                            crew_line(CALL_GAP, script::RINA, script::CALL_RINA),
                            radio_line(CALL_GAP, script::ELENA, script::CALL_REFUSAL),
                            radio_line(CALL_REPLY_GAP, script::ELENA, script::CALL_REED),
                            radio_line(CALL_GAP, script::ELENA, script::CALL_BACKING),
                            crew_line(CALL_REPLY_GAP, script::JONAH, script::CALL_DECISION),
                            // The hold: the order line lands and the shot
                            // stays on Gantry. Nothing to run - the beat IS
                            // the action, and it keeps the course change off
                            // the same frame as the line that caused it.
                            step(CALL_ORDER_GAP, vec![]),
                        ],
                    ),
                ])
                .collect(),
        ),
        // Every way out of the call ends here, a skip included: the camera and
        // the helm come back, and the course change is on the HUD. Kaveri is
        // at rest when it lands, which is the whole reason the scene took the
        // ship's speed off: handing the helm back mid-lane at the full manual
        // cap would leave the player braking instead of turning.
        once(
            EventConfig::OnCinematicFinished,
            vec![scene(SCENE_CALL)],
            vec![
                EventActionConfig::ReleaseCamera(ReleaseCameraActionConfig),
                EventActionConfig::ResumePlayerControl(ResumePlayerControlActionConfig),
                advance(BEAT_REACH),
                post_objective(OBJ_REACH, script::OBJ_TEXT_REACH),
                attach_objective_marker(ID_GANTRY, GANTRY_NAME),
                APPROACH.raise_gate(),
                show_hint_emphasis(HINT_RADAR),
            ],
        ),
        // Arriving: the card comes down, the marker comes off the hull, and
        // one line runs. The collar dialogue waits for the ring inside this
        // one, where the ship is slow enough to read four cards.
        once(
            EventConfig::OnEnter,
            vec![APPROACH.entered_by(ID_KAVERI), in_beat(BEAT_REACH)],
            vec![
                complete_objective(OBJ_REACH),
                despawn_object(APPROACH.gate_id()),
                crew(script::TOMAS, script::NEAR_SIGHT),
                STANDOFF.raise_gate(),
            ],
        ),
        // Inside the standoff ring: the port check, the answer, and the order.
        // The card itself is not the last STEP of this chain but a timer the
        // last step arms - see `TIMER_DOCK`.
        once(
            EventConfig::OnEnter,
            vec![STANDOFF.entered_by(ID_KAVERI), in_beat(BEAT_REACH)],
            vec![
                despawn_object(STANDOFF.gate_id()),
                sequence(
                    SEQ_APPROACH,
                    vec![
                        crew_line(NEAR_FIRST_AT, script::LEILA, script::NEAR_PORT_CHECK),
                        radio_line(NEAR_GAP, script::NADIA, script::NEAR_PORT_SOUND),
                        step(
                            NEAR_GAP,
                            vec![
                                crew(script::JONAH, script::NEAR_COMMIT),
                                start_timer(TIMER_DOCK, NEAR_DOCK_GAP),
                            ],
                        ),
                    ],
                ),
            ],
        ),
        // The docking card, a breath behind the order that commits to it, and
        // only while the collar is still empty.
        once(
            EventConfig::OnTimerEnd,
            vec![timer(TIMER_DOCK), in_beat(BEAT_REACH)],
            vec![
                advance(BEAT_DOCK),
                post_objective(OBJ_DOCK, script::OBJ_TEXT_DOCK),
                show_hint_emphasis(HINT_DOCK),
                show_hint_emphasis(HINT_RCS),
            ],
        ),
        // The player is already on the collar when the card would arrive.
        // DOCK is in the player's hands from the first frame of the chapter, so
        // this is reachable - and without it the clamp lands in a beat nothing
        // listens to, the card posts behind it, and the chapter has no way
        // forward but letting go again. The pending `TIMER_DOCK` fires into a
        // beat that has moved and says nothing, which is the whole reason the
        // card is a timer.
        once(
            EventConfig::OnDocked,
            vec![clamp(), in_beat(BEAT_REACH)],
            vec![
                clear_hint_emphasis(HINT_RADAR),
                detach_objective_marker(ID_GANTRY),
                advance(BEAT_HOLD),
                comms(script::LEILA, script::DOCK_SEAL),
                start_timer(TRANSFER_KEYS[0], TRANSFER_CARD_AFTER),
            ],
        ),
        // The clamp. Repeatable on purpose: a player who lets go and comes
        // back restarts the transfer at its first beat.
        repeatable(
            EventConfig::OnDocked,
            vec![clamp(), in_beat(BEAT_DOCK)],
            vec![
                clear_hint_emphasis(HINT_RADAR),
                clear_hint_emphasis(HINT_DOCK),
                clear_hint_emphasis(HINT_RCS),
                complete_objective(OBJ_DOCK),
                detach_objective_marker(ID_GANTRY),
                advance(BEAT_HOLD),
                comms(script::LEILA, script::DOCK_SEAL),
                start_timer(TRANSFER_KEYS[0], TRANSFER_CARD_AFTER),
            ],
        ),
    ]);

    // The timed transfer. One handler per beat, each gated on the clamp still
    // being on, and the card itself is the first of them - so the release
    // handler below always knows whether there is an objective to take down.
    events.extend([
        transfer_beat(
            TRANSFER_KEYS[0],
            BEAT_HOLD,
            vec![
                advance(BEAT_TRANSFER),
                post_objective(OBJ_HOLD, script::OBJ_TEXT_HOLD),
            ],
            Some((TRANSFER_KEYS[1], TRANSFER_GAP)),
        ),
        transfer_beat(
            TRANSFER_KEYS[1],
            BEAT_TRANSFER,
            vec![crew(script::RINA, script::HOLD_OWEN)],
            Some((TRANSFER_KEYS[2], TRANSFER_GAP)),
        ),
        transfer_beat(
            TRANSFER_KEYS[2],
            BEAT_TRANSFER,
            vec![crew(script::SAMIR, script::HOLD_ABOARD)],
            Some((TRANSFER_KEYS[3], TRANSFER_GAP)),
        ),
        transfer_beat(
            TRANSFER_KEYS[3],
            BEAT_TRANSFER,
            vec![comms(script::NADIA, script::HOLD_ALL_THREE)],
            Some((TRANSFER_KEYS[4], TRANSFER_DONE_AFTER)),
        ),
        transfer_beat(
            TRANSFER_KEYS[4],
            BEAT_TRANSFER,
            vec![
                advance(BEAT_RELEASE),
                complete_objective(OBJ_HOLD),
                post_objective(OBJ_RELEASE, script::OBJ_TEXT_RELEASE),
                show_hint_emphasis(HINT_DOCK),
            ],
            None,
        ),
        // Letting go too early, with and without the card up.
        early_release(BEAT_HOLD, vec![]),
        early_release(BEAT_TRANSFER, vec![complete_objective(OBJ_HOLD)]),
        // The ask, a beat after the release - and only if the collar is still
        // empty. The card and the beat move together, which is what lets the
        // clamp above complete a card it can be sure is up.
        repeatable(
            EventConfig::OnTimerEnd,
            vec![timer(TIMER_REGRIP), in_beat(BEAT_REGRIP)],
            vec![
                advance(BEAT_DOCK),
                post_objective(OBJ_DOCK, script::OBJ_TEXT_DOCK_AGAIN),
                attach_objective_marker(ID_GANTRY, GANTRY_NAME),
                show_hint_emphasis(HINT_DOCK),
            ],
        ),
        // Back on the collar before the ask arrived: no card was posted, so
        // none is taken down, and the timed transfer starts again.
        repeatable(
            EventConfig::OnDocked,
            vec![clamp(), in_beat(BEAT_REGRIP)],
            vec![
                advance(BEAT_HOLD),
                comms(script::LEILA, script::DOCK_SEAL),
                start_timer(TRANSFER_KEYS[0], TRANSFER_CARD_AFTER),
            ],
        ),
        // Letting go with all three aboard: the chapter is won, and the run
        // home is the epilogue's to tell.
        once(
            EventConfig::OnUndocked,
            vec![clamp(), in_beat(BEAT_RELEASE)],
            vec![
                clear_hint_emphasis(HINT_DOCK),
                complete_objective(OBJ_RELEASE),
                advance(BEAT_OUTRO),
                comms(script::JONAH, script::WON_LINE),
                pacing::outro_sequence(
                    VAR_BEAT,
                    BEAT_WON,
                    script::ELENA,
                    script::OUTRO_TEASE,
                    script::OUTRO_BANNER,
                    vec![],
                    None,
                ),
            ],
        ),
        defeat(ID_KAVERI, script::DEFEAT_KAVERI),
        defeat(ID_GANTRY, script::DEFEAT_GANTRY),
    ]);

    apply_portraits(&mut events);

    ScenarioConfig {
        description: "Season 1, chapter one. Kaveri is running home to Baikal with a \
                      replacement pump assembly when a stranded ship calls for help. Thread \
                      the working lane, come about, and bring three people off Gantry. No \
                      weapons."
            .to_string(),
        thumbnail: Some(AssetRef::from(
            "self://thumbnails/season_one_chapter_one.png",
        )),
        events,
        ..ScenarioConfig::new(
            CHAPTER_ONE_SCENARIO_ID.to_string(),
            "A Useful Job".to_string(),
            cubemap,
        )
    }
}
