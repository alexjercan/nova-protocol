//! The handbook's practice ranges: four focused scenarios a training lesson
//! hands off to, and nothing else.
//!
//! A drill is NOT a chapter. It declares [`ScenarioRole::Lesson`], so the
//! Scenarios picker renders no row for it and a campaign may not list it: the
//! only way in is the Practice button on the lesson it belongs to. That is
//! what the role exists to make checkable - a lesson may only practise in a
//! scenario built for practising.
//!
//! Each one is Basic Training's range (`super::tutorial::range`) with
//! everything the lesson is not about taken away: no briefing, no cast, no
//! card to work through. What is left is a short flight under four rules that
//! make it a drill rather than a place to fly:
//!
//! - THE HELM IS THE LESSON'S. A drill hands over the verbs its lessons teach
//!   and withholds the rest, so the verb that would fly the drill FOR the
//!   player is not there to press. A withheld verb draws a dark chip in the
//!   keybind dock, which is the HUD already saying "not on this range".
//! - THE BOARD IS ONE SHORT LINE. The objective says what to do in a handful
//!   of words. The reason goes on the comms channel, where a line is read once
//!   and leaves - rather than sitting on the HUD for the whole drill, being
//!   scrolled past. The voice there is a plain `Training` label with no
//!   portrait: a practice flight teaches, it does not act.
//! - THE VERB IS LIT. The keybind chip for whatever the live beat is about
//!   pulses until that beat is done, so "what do I press" is answered on the
//!   HUD, in the player's own bindings, and never spelled in authored text.
//! - IT ENDS. The last beat wins the drill: a closing line, then the VICTORY
//!   banner a beat later with nothing queued behind it, so the
//!   overlay offers Main Menu alone and the player lands back where the
//!   Practice button was. Losing the trainer ends it too - DEFEAT, whose
//!   Retry puts them straight back on the range.
//!
//! Winning a drill is still FEEDBACK, not proof. Nothing here marks a lesson
//! completed: flying to a beacon says the player went somewhere, not that they
//! learned anything, and the handbook's `Completed` state is reserved for an
//! outcome that actually asserts the skill.

use bevy::prelude::*;
use nova_gameplay::prelude::AssetRef;
use nova_scenario::prelude::*;
use nova_ship::prelude::ShipCapabilities;

use super::{
    pacing::beat_later,
    tutorial::range::{
        belts, lights, planetoid, raise_range_boundary, target_hulk, target_id, target_label,
        trainer_at, trainer_with, Mark, HINT_GOTO, HINT_ORBIT, HINT_RADAR, HINT_RCS, HINT_STOP,
        ID_PLANETOID, ID_TRAINER, MARK_ALPHA, MARK_BRAVO, MARK_CHARLIE, PLANETOID_LABEL,
        TARGET_COUNT,
    },
    BaseContentAssets,
};
use crate::scenario_helpers::prelude::*;

#[cfg(test)]
mod tests;

/// Who the comms card is from. A label, not a character: a practice flight
/// explains itself in its own voice, and Basic Training's cast
/// (`super::tutorial`) stays where it belongs.
const TRAINING: &str = "Training";

/// What the player's ship is called here. Basic Training's cadet picket has a
/// callsign; on a practice flight it is just your ship.
const PLAYER_SHIP: &str = "Your ship";

/// Momentum practice: fly out, release the drive, and keep the speed.
pub(crate) const DRILL_MOMENTUM_ID: &str = "drill_momentum";
/// STOP practice: build speed, then hand the ship to the flight computer.
pub(crate) const DRILL_STOP_ID: &str = "drill_stop";
/// Autopilot practice: a planetoid to fly out to, orbit, and fly back from.
pub(crate) const DRILL_AUTOPILOT_ID: &str = "drill_autopilot";
/// Turret practice: a line of unarmed targets and a gun.
pub(crate) const DRILL_GUNNERY_ID: &str = "drill_gunnery";

// --- the shape of a drill ----------------------------------------------------

/// Which beat is live. Every handler is guarded by it, so a drill cannot be
/// flown out of order - and, past [`BEAT_DONE`], cannot be re-entered at all.
const VAR_BEAT: &str = "beat";

/// The beat a WON drill parks on. Past every guard: the outro cannot fire
/// twice, and a trainer lost while the banner is coming up is not a defeat on
/// a drill the player already flew.
const BEAT_DONE: f64 = 90.0;

/// The gap between the line that says the drill landed and the banner that
/// ends it. Long enough to read the line against the range one last time,
/// short enough that the player is not waiting on a screen they have finished
/// with.
const OUTRO_AFTER: f64 = 3.0;

/// The outro's sequence key. One per drill, and a drill has one outro.
const OUTRO: &str = "drill_outro";

fn advance(beat: f64) -> EventActionConfig {
    set_variable(VAR_BEAT, number(beat))
}

fn in_beat(beat: f64) -> EventFilterConfig {
    number_equals(VAR_BEAT, beat)
}

/// A handler that fires once, for the beat it belongs to.
fn once(
    event: EventConfig,
    filters: Vec<EventFilterConfig>,
    actions: Vec<EventActionConfig>,
) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: event,
        once: true,
        filters,
        actions,
    }
}

fn on_start(actions: Vec<EventActionConfig>) -> ScenarioEventConfig {
    once(EventConfig::OnStart, vec![], actions)
}

/// The handler that fires once when the trainer reaches `mark` on `beat`.
fn on_arrival(mark: &Mark, beat: f64, actions: Vec<EventActionConfig>) -> ScenarioEventConfig {
    once(
        EventConfig::OnEnter,
        vec![mark.gate_entered(), in_beat(beat)],
        actions,
    )
}

/// Put a mark up with its arrival gate, the way Basic Training does.
fn raise(mark: &Mark) -> Vec<EventActionConfig> {
    let mut actions = mark.raise();
    actions.push(mark.raise_gate());
    actions
}

/// What ENDS a drill: the objective it just paid off, the line that says so,
/// and the VICTORY banner a beat later.
///
/// Nothing is queued behind the banner. A chapter chains; a drill is over, and
/// the overlay with no queued switch offers Main Menu alone - which is where
/// the handbook is, one click from the Practice button that sent the player
/// here.
fn win(objective: &str, line: &str, banner: &str) -> Vec<EventActionConfig> {
    vec![
        complete_objective(objective),
        advance(BEAT_DONE),
        comms(TRAINING, line),
        beat_later(
            OUTRO,
            OUTRO_AFTER,
            vec![EventActionConfig::Outcome(OutcomeActionConfig::new(
                ScenarioOutcomeKind::Victory,
                banner,
            ))],
        ),
    ]
}

/// The other ending: the trainer is gone. DEFEAT queues the drill itself, so
/// the overlay's Retry is another run at the same range rather than a trip
/// back through the menu.
fn lose(id: &str, message: &str, event: EventConfig) -> ScenarioEventConfig {
    once(
        event,
        vec![entity(ID_TRAINER), number_less_than(VAR_BEAT, BEAT_DONE)],
        vec![
            EventActionConfig::Outcome(OutcomeActionConfig::new(
                ScenarioOutcomeKind::Defeat,
                message,
            )),
            EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: id.to_string(),
                linger: true,
                delay: None,
            }),
        ],
    )
}

/// A drill's shell: the id, the name the pause menu shows, the one line the
/// handbook does not repeat, the role that keeps it out of the picker, and
/// both ways it can end.
fn drill(
    id: &str,
    name: &str,
    description: &str,
    cubemap: AssetRef<Image>,
    lost: &str,
    mut events: Vec<ScenarioEventConfig>,
) -> ScenarioConfig {
    events.push(lose(id, lost, EventConfig::OnDestroyed));
    events.push(lose(id, lost, EventConfig::OnNeutralized));
    ScenarioConfig {
        description: description.to_string(),
        role: ScenarioRole::Lesson,
        events,
        ..ScenarioConfig::new(id, name, cubemap)
    }
}

/// Everything every drill puts on the range before its own furniture: the
/// player's ship on the helm its lessons teach, the lights, the belts, the
/// range edge, and the first beat.
fn range_floor(
    key: &str,
    capabilities: ShipCapabilities,
    asteroid_texture: &AssetRef<Image>,
) -> Vec<EventActionConfig> {
    let mut ship = trainer_with(capabilities);
    ship.base.name = PLAYER_SHIP.to_string();
    let mut actions: Vec<EventActionConfig> = std::iter::once(ship)
        .chain(lights(key))
        .map(EventActionConfig::SpawnScenarioObject)
        .collect();
    actions.extend(belts(asteroid_texture));
    actions.push(raise_range_boundary());
    actions.push(advance(1.0));
    actions
}

/// The helm a drill hands over, written as the verbs it KEEPS. Point defence and
/// docking are not among them: the trainer carries neither a point-defense mount
/// nor a docking port, so both flags are left where Basic Training leaves them.
const fn helm(stop: bool, rcs: bool, lock: bool, goto: bool, orbit: bool) -> ShipCapabilities {
    ShipCapabilities {
        stop_enabled: stop,
        rcs_enabled: rcs,
        lock_enabled: lock,
        goto_enabled: goto,
        orbit_enabled: orbit,
        point_defense_enabled: true,
        dock_enabled: true,
    }
}

// --- the ranges --------------------------------------------------------------

/// "Turn, then thrust" and "You keep your speed": two marks, and no flight
/// computer at all.
///
/// STOP is withheld ON PURPOSE and it is the whole drill: with the computer
/// able to cancel the burn, a player never has to turn the hull around and
/// spend the speed back, which is the only way momentum is learned. GOTO and
/// ORBIT would fly the leg outright, and there is nothing here to lock, so the
/// player is left with the nose and the throttle.
fn momentum(assets: &BaseContentAssets) -> ScenarioConfig {
    const OBJ_ALPHA: &str = "alpha";
    const OBJ_BRAVO: &str = "bravo";

    let mut start = range_floor(
        DRILL_MOMENTUM_ID,
        helm(false, true, false, false, false),
        &assets.asteroid_texture.clone(),
    );
    start.extend(raise(&MARK_ALPHA));
    start.push(post_objective(OBJ_ALPHA, "Fly out to mark ALPHA."));
    start.push(comms(
        TRAINING,
        "The flight computer is switched off for this flight. You have the nose and the \
         main drive only. Fly out to mark ALPHA.",
    ));

    let mut on_alpha = vec![complete_objective(OBJ_ALPHA), advance(2.0)];
    on_alpha.extend(MARK_ALPHA.clear());
    on_alpha.extend(raise(&MARK_BRAVO));
    on_alpha.push(post_objective(OBJ_BRAVO, "Fly to mark BRAVO."));
    on_alpha.push(comms(
        TRAINING,
        "Mark ALPHA reached. BRAVO is off to your right. Turn to face it first, then \
         thrust: the drive only pushes where the nose points.",
    ));

    let mut on_bravo = MARK_BRAVO.clear();
    on_bravo.extend(win(
        OBJ_BRAVO,
        "Both marks reached. With no flight computer, every change of speed came from your \
         own thrust.",
        "Momentum practice complete.",
    ));

    drill(
        DRILL_MOMENTUM_ID,
        "Momentum practice",
        "Fly to two marks with the flight computer switched off.",
        assets.cubemap.clone(),
        "Your ship was destroyed. Starting again.",
        vec![
            on_start(start),
            on_arrival(&MARK_ALPHA, 1.0, on_alpha),
            on_arrival(&MARK_BRAVO, 2.0, on_bravo),
        ],
    )
}

/// "The STOP order" and "Using the RCS thrusters": one mark to stop ON, and a
/// second one to move sideways onto.
///
/// Arriving and stopping are deliberately two beats. The mark's volume is wide
/// enough to reach at speed, so a player who only flies at it passes through
/// it - and is told, at the moment it happens, that arriving is not the same
/// as being at rest.
fn stop(assets: &BaseContentAssets) -> ScenarioConfig {
    const OBJ_REST: &str = "rest";
    const OBJ_SLIDE: &str = "slide";

    let mut start = range_floor(
        DRILL_STOP_ID,
        helm(true, true, false, false, false),
        &assets.asteroid_texture.clone(),
    );
    start.extend(raise(&MARK_ALPHA));
    start.push(post_objective(OBJ_REST, "Stop on mark ALPHA."));
    start.push(show_hint_emphasis(HINT_STOP));
    start.push(comms(
        TRAINING,
        "Fly out to mark ALPHA and stop on it. STOP hands the ship to the flight computer, \
         which turns the ship around and thrusts until your speed reads zero.",
    ));

    let on_alpha = vec![
        advance(2.0),
        comms(
            TRAINING,
            "You are on the mark but still moving. Use STOP and let the flight computer \
             take your speed down to zero.",
        ),
    ];

    let mut at_rest = vec![
        complete_objective(OBJ_REST),
        advance(3.0),
        clear_hint_emphasis(HINT_STOP),
    ];
    at_rest.extend(MARK_ALPHA.clear());
    at_rest.extend(raise(&MARK_BRAVO));
    at_rest.push(post_objective(
        OBJ_SLIDE,
        "Use RCS to move onto mark BRAVO.",
    ));
    at_rest.push(show_hint_emphasis(HINT_RCS));
    at_rest.push(comms(
        TRAINING,
        "Stopped. Now use the RCS thrusters: they move the ship sideways without turning \
         it. Move onto mark BRAVO.",
    ));

    let mut on_bravo = vec![clear_hint_emphasis(HINT_RCS)];
    on_bravo.extend(MARK_BRAVO.clear());
    on_bravo.extend(win(
        OBJ_SLIDE,
        "Both marks done. You stopped the ship with STOP and placed it with RCS.",
        "STOP practice complete.",
    ));

    drill(
        DRILL_STOP_ID,
        "STOP practice",
        "Stop on a mark, then move sideways onto a second one.",
        assets.cubemap.clone(),
        "Your ship was destroyed. Starting again.",
        vec![
            on_start(start),
            on_arrival(&MARK_ALPHA, 1.0, on_alpha),
            // The stop itself, and only after the mark: a player who parks at
            // the start line has stopped, but not on anything.
            once(
                EventConfig::OnStopComplete,
                vec![entity(ID_TRAINER), in_beat(2.0)],
                at_rest,
            ),
            on_arrival(&MARK_BRAVO, 3.0, on_bravo),
        ],
    )
}

/// "GOTO a mark" and "ORBIT a mark": the planetoid, and a mark to fly back
/// to.
///
/// The same rock Basic Training flies out to, for the same reason - it is the
/// only body on the range with enough pull for an orbit to mean anything -
/// with the leg home marked so the drill ends somewhere rather than trailing
/// off in deep space. STOP is withheld: the whole drill is letting the
/// computer fly, and a player who brakes by hand at the rock has flown around
/// the lesson rather than through it.
fn autopilot(assets: &BaseContentAssets) -> ScenarioConfig {
    const OBJ_OUT: &str = "out";
    const OBJ_ORBIT: &str = "orbit";
    const OBJ_HOME: &str = "home";

    let mut start = range_floor(
        DRILL_AUTOPILOT_ID,
        helm(false, true, true, true, true),
        &assets.asteroid_texture.clone(),
    );
    start.push(EventActionConfig::SpawnScenarioObject(planetoid()));
    start.push(attach_objective_marker(ID_PLANETOID, PLANETOID_LABEL));
    start.push(post_objective(OBJ_OUT, "Use GOTO to reach the planetoid."));
    start.push(show_hint_emphasis(HINT_RADAR));
    start.push(comms(
        TRAINING,
        "Keep your weapons lowered for this flight. Hold radar on the planetoid until the \
         white travel lock takes. The flight computer only flies to a white lock.",
    ));

    drill(
        DRILL_AUTOPILOT_ID,
        "Autopilot practice",
        "Use GOTO and ORBIT to fly out to a planetoid and back.",
        assets.cubemap.clone(),
        "Your ship was destroyed. Starting again.",
        vec![
            on_start(start),
            // The cadet who sweeps with weapons raised: the red lock is the
            // gun's, and the computer will not fly it.
            once(
                EventConfig::OnCombatLockStart,
                vec![trainer_at(ID_PLANETOID), in_beat(1.0)],
                vec![comms(
                    TRAINING,
                    "That red lock is the weapon lock. Lower your weapons and take the lock \
                     again. The flight computer will not fly to a red lock.",
                )],
            ),
            once(
                EventConfig::OnTravelLockStart,
                vec![trainer_at(ID_PLANETOID), in_beat(1.0)],
                vec![
                    advance(2.0),
                    clear_hint_emphasis(HINT_RADAR),
                    show_hint_emphasis(HINT_GOTO),
                    comms(
                        TRAINING,
                        "Travel lock set. Give the GOTO order and the flight computer \
                         flies the whole leg for you.",
                    ),
                ],
            ),
            once(
                EventConfig::OnGotoComplete,
                vec![trainer_at(ID_PLANETOID), in_beat(2.0)],
                vec![
                    complete_objective(OBJ_OUT),
                    advance(3.0),
                    clear_hint_emphasis(HINT_GOTO),
                    post_objective(OBJ_ORBIT, "Use ORBIT to circle the planetoid."),
                    show_hint_emphasis(HINT_ORBIT),
                    comms(
                        TRAINING,
                        "You have arrived, and you are inside the planetoid's gravity. \
                         ORBIT settles the ship into a circle and holds it.",
                    ),
                ],
            ),
            // A HELD orbit is what puts the leg home on the board - not the
            // arrival, and not a mark raised at the start, either of which
            // would end the drill before the autopilot had flown anything.
            once(
                EventConfig::OnOrbitStable,
                vec![trainer_at(ID_PLANETOID), in_beat(3.0)],
                {
                    let mut actions = vec![
                        complete_objective(OBJ_ORBIT),
                        advance(4.0),
                        clear_hint_emphasis(HINT_ORBIT),
                        detach_objective_marker(ID_PLANETOID),
                    ];
                    actions.extend(raise(&MARK_CHARLIE));
                    actions.push(post_objective(OBJ_HOME, "Use GOTO to reach mark CHARLIE."));
                    actions.push(show_hint_emphasis(HINT_GOTO));
                    actions.push(comms(
                        TRAINING,
                        "The orbit is stable. Mark CHARLIE next, then use GOTO to fly back \
                         to it.",
                    ));
                    actions
                },
            ),
            on_arrival(&MARK_CHARLIE, 4.0, {
                let mut actions = vec![clear_hint_emphasis(HINT_GOTO)];
                actions.extend(MARK_CHARLIE.clear());
                actions.extend(win(
                    OBJ_HOME,
                    "You are on mark CHARLIE. The flight computer flew the whole route, out \
                     and back.",
                    "Autopilot practice complete.",
                ));
                actions
            }),
        ],
    )
}

/// "Using the radar", "Raising weapons" and "Lock a component": the firing
/// line, and nothing that shoots back.
///
/// The hulks are the tutorial's, unarmed and stationary, because the lessons
/// this range serves are about the lock and the stance rather than about a
/// fight. A drone that answered would teach the player to hurry. The drill is
/// ONE of them - Target 1, marked, named on the board - and the other four
/// stand there as the rest of the line.
fn gunnery(assets: &BaseContentAssets) -> ScenarioConfig {
    const OBJ_TARGET: &str = "target";

    let mut start = range_floor(
        DRILL_GUNNERY_ID,
        helm(true, true, true, false, false),
        &assets.asteroid_texture.clone(),
    );
    start.extend(
        (1..=TARGET_COUNT)
            .map(target_hulk)
            .map(EventActionConfig::SpawnScenarioObject),
    );
    start.push(attach_objective_marker(target_id(1), target_label(1)));
    start.push(post_objective(
        OBJ_TARGET,
        format!("Destroy {}.", target_label(1)),
    ));
    start.push(show_hint_emphasis(HINT_RADAR));
    start.push(comms(
        TRAINING,
        "Five unarmed targets are on the range, and none of them shoot back. Find Target 1 \
         with the radar, raise your weapons, and destroy it.",
    ));

    drill(
        DRILL_GUNNERY_ID,
        "Turret practice",
        "Five unarmed target ships to practise shooting.",
        assets.cubemap.clone(),
        "Your ship was destroyed. Starting again.",
        vec![
            on_start(start),
            // The cadet who sweeps with weapons lowered: the white lock is
            // real, and it feeds no gun.
            once(
                EventConfig::OnTravelLockStart,
                vec![trainer_at(target_id(1)), in_beat(1.0)],
                vec![comms(
                    TRAINING,
                    "That white lock is the travel lock and it does not feed the guns. Raise \
                     your weapons and take the lock again.",
                )],
            ),
            once(
                EventConfig::OnCombatLockStart,
                vec![trainer_at(target_id(1)), in_beat(1.0)],
                vec![
                    advance(2.0),
                    clear_hint_emphasis(HINT_RADAR),
                    comms(
                        TRAINING,
                        "That red lock is the weapon lock. Scroll the wheel to step the lock \
                         onto one section, such as a drive or a turret.",
                    ),
                ],
            ),
            // Filtered on Target 1 rather than left open: the range is full of
            // belt rocks, and an unfiltered handler would call the drill done
            // the first time the player clipped one.
            once(
                EventConfig::OnDestroyed,
                vec![entity(target_id(1)), number_less_than(VAR_BEAT, BEAT_DONE)],
                win(
                    OBJ_TARGET,
                    "Target 1 is destroyed. The other four stay on the range, so you can come \
                     back and practise.",
                    "Turret practice complete.",
                ),
            ),
        ],
    )
}

/// Every practice range, in generated-content order.
pub(crate) fn catalog(assets: &BaseContentAssets) -> Vec<ScenarioConfig> {
    vec![
        momentum(assets),
        stop(assets),
        autopilot(assets),
        gunnery(assets),
    ]
}
