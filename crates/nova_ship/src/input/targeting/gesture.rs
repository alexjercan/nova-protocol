//! The CTRL gesture that drives the radar: hold to search and commit into a
//! slot, tap to clear staged (combat first, then travel).

#[cfg(test)]
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::{Cancel as ActionCancel, *};
use nova_gameplay::prelude::*;

use super::contacts::ship_grants_lock;
#[cfg(test)]
use super::radar::{lock_dwell_secs, update_radar_search};
use crate::prelude::*;

/// One radar gesture threshold (seconds), shared by the `Hold` (radar/commit)
/// and `Tap` (clear) conditions on CTRL - deriving both from one constant is
/// what keeps the boundary frame from falling in a gap between them.
pub const RADAR_TAP_SECS: f32 = 0.25;

/// Hold CTRL: radar on (live retarget); release past the threshold commits.
#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct RadarHoldInput;

/// Tap CTRL: staged lock clear.
#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct RadarClearInput;

/// Start of the radar hold: open the search. The destination slot is NOT
/// decided here - it latches at the hold threshold, in the live search
/// (Q1a). Gated on the computer's Lock capability and, like every intent
/// observer, on the pause overlay.
#[expect(
    clippy::type_complexity,
    reason = "the controller query filters on two markers at once"
)]
pub(super) fn on_radar_start(
    _: On<Start<RadarHoldInput>>,
    mut commands: Commands,
    pause: Res<State<nova_gameplay::PauseStates>>,
    mut denied: MessageWriter<RadarDenied>,
    q_controllers: Query<
        (&ChildOf, Option<&WithheldVerbs>),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    q_ship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up. Releases stay ungated so held keys clear cleanly
    // during a pause.
    if pause.get().is_frozen() {
        return;
    }
    for ship in &q_ship {
        if !ship_grants_lock(ship, &q_controllers) {
            // No Lock capability on this computer: the radar does not come
            // on - and says so (deny buzz + adornment flash, F7/Q8a).
            denied.write(RadarDenied);
            continue;
        }
        commands.entity(ship).insert(RadarState::default());
    }
}

/// Radar teardown, shared by both release paths: with the lock written live
/// under the sweep (strand A1), a release has nothing to commit - it only
/// closes the search. Deliberately not pause-gated: state cleanup must
/// always run, like the release observers elsewhere.
fn close_radar_search(
    commands: &mut Commands,
    q_ship: &Query<
        Entity,
        (
            With<SpaceshipRootMarker>,
            With<PlayerSpaceshipMarker>,
            With<RadarState>,
        ),
    >,
) {
    for ship in q_ship {
        commands.entity(ship).remove::<RadarState>();
    }
}

/// Radar release past the hold threshold (`Complete`): the lock already
/// holds whatever the sweep last resolved - it sticks; just close the
/// search.
pub(super) fn on_radar_commit(
    _: On<Complete<RadarHoldInput>>,
    mut commands: Commands,
    q_ship: Query<
        Entity,
        (
            With<SpaceshipRootMarker>,
            With<PlayerSpaceshipMarker>,
            With<RadarState>,
        ),
    >,
) {
    close_radar_search(&mut commands, &q_ship);
}

/// Radar release BEFORE the hold threshold (`Cancel`): nothing was latched
/// or written yet; close the search - the same physical release fires the
/// Tap clear separately.
pub(super) fn on_radar_cancel(
    _: On<ActionCancel<RadarHoldInput>>,
    mut commands: Commands,
    q_ship: Query<
        Entity,
        (
            With<SpaceshipRootMarker>,
            With<PlayerSpaceshipMarker>,
            With<RadarState>,
        ),
    >,
) {
    close_radar_search(&mut commands, &q_ship);
}

/// The tap clear (staged): lowered - the combat lock first,
/// else the travel lock (disengaging an engaged GOTO with it); raised - only
/// ever the combat lock. Emits [`LockClearedToast`] so the mode-scoped
/// gesture is legible on the HUD.
#[expect(
    clippy::type_complexity,
    reason = "one query term per lock the tap clears"
)]
pub(super) fn on_lock_clear_tap(
    _: On<Fire<RadarClearInput>>,
    mut commands: Commands,
    pause: Res<State<nova_gameplay::PauseStates>>,
    mut toasts: MessageWriter<LockClearedToast>,
    mut q_ship: Query<
        (
            Entity,
            Option<&WeaponsRaised>,
            &mut TravelLock,
            &mut CombatLock,
            Option<&Autopilot>,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    if pause.get().is_frozen() {
        return;
    }
    for (ship, raised, mut travel, mut combat, autopilot) in &mut q_ship {
        let raised = raised.is_some_and(|raised| raised.0);
        if combat.0.is_some() {
            let target = combat.0.take();
            toasts.write(LockClearedToast {
                combat: true,
                target,
            });
        } else if !raised && travel.0.is_some() {
            let target = travel.0.take();
            toasts.write(LockClearedToast {
                combat: false,
                target,
            });
            // Clearing the designation disengages an engaged GOTO (decision
            // from the recap Q&A); other maneuvers (STOP/ORBIT) are not
            // lock-bound and keep flying.
            if autopilot
                .is_some_and(|autopilot| matches!(autopilot.action, AutopilotAction::Goto { .. }))
            {
                commands.entity(ship).remove::<Autopilot>();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    /// Count of [`RadarLockAcquired`] cues seen, drained by a test-local
    /// reader system so the once-per-gesture contract is observable across
    /// frames (Messages double-buffer per update).
    #[derive(Resource, Default)]
    struct AcquiredCueCount(usize);

    /// Count of [`RadarRetargeted`] ticks seen, same test-local reader
    /// pattern.
    #[derive(Resource, Default)]
    struct RetargetCueCount(usize);

    /// Real input -> real `Hold`/`Tap` conditions -> the REAL radar search:
    /// the full gesture e2e on the production flight rig, with
    /// `TimeUpdateStrategy::ManualDuration` driving the clock the conditions
    /// tick on (50 ms steps: 5 updates = the exact 250 ms threshold) and the
    /// production split camera rig (ACTIVE normal rig, dormant turret decoy
    /// 90 degrees off - reading the wrong rig fails loudly) feeding
    /// `update_radar_search`, which under the live-lock model owns the
    /// threshold latch and the slot writes. Nothing is stuffed: bodies are
    /// picked off the real look ray.
    fn gesture_app() -> (App, Entity) {
        use bevy::input::InputPlugin;

        use crate::input::player::{flight_input_rig, FlightInputMarker};

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<nova_gameplay::PauseStates>();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(50),
        ));
        app.init_resource::<TargetingSettings>();
        // These gesture tests exercise the LATCH / keep-last / tap-clear
        // axis, which is orthogonal to the acquisition dwell. Neutralize the
        // dwell to zero here so a candidate commits the moment it settles
        // (the pre-dwell semantics these tests were written for); the dwell
        // GATE has its own focused tests (`dwell_*`) that set a real
        // duration.
        {
            let mut settings = app.world_mut().resource_mut::<TargetingSettings>();
            settings.lock_dwell_base = 0.0;
            settings.lock_dwell_range_factor = 0.0;
            settings.lock_dwell_min = 0.0;
        }
        app.init_resource::<AcquiredCueCount>();
        app.add_input_context::<FlightInputMarker>();
        app.add_observer(on_radar_start);
        app.add_observer(on_radar_commit);
        app.add_observer(on_radar_cancel);
        app.add_observer(on_lock_clear_tap);
        app.add_message::<LockClearedToast>();
        app.add_message::<RadarLockAcquired>();
        app.add_message::<RadarRetargeted>();
        app.add_message::<RadarDenied>();
        // The live search runs inside the pause-gated input set, exactly as
        // the production plugin wires it.
        app.add_systems(
            Update,
            update_radar_search.in_set(crate::input::SpaceshipInputSystems),
        );
        crate::configure_pause_gating(&mut app);
        app.add_systems(
            Update,
            |mut cues: MessageReader<RadarLockAcquired>, mut count: ResMut<AcquiredCueCount>| {
                count.0 += cues.read().count();
            },
        );

        // The production split camera rig: the ACTIVE normal rig looks down
        // -Z; the dormant turret decoy points 90 degrees off so a system
        // reading the wrong rig picks the wrong bodies and fails loudly.
        app.world_mut().spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraNormalInputMarker,
            SpaceshipRotationInputActiveMarker,
            PointRotationOutput(Quat::IDENTITY),
        ));
        app.world_mut().spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotationOutput(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        ));

        // A player ship whose computer grants Lock (the default loadout).
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
                targeting_state(),
            ))
            .id();
        // No WithheldVerbs: an absent component grants every verb (Lock).
        app.world_mut()
            .spawn((ControllerSectionMarker, ChildOf(ship)));

        // The context registry finalizes in App::finish; run the lifecycle
        // before spawning the rig, like the production app does.
        app.finish();
        app.cleanup();
        app.update();
        app.world_mut().spawn(flight_input_rig());
        app.update();
        (app, ship)
    }

    /// A radar-pickable ship body at `position` (ships are intrinsic-class:
    /// full scanner range).
    fn spawn_ship(app: &mut App, position: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                SpaceshipRootMarker,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(position),
            ))
            .id()
    }

    /// Point the ACTIVE normal rig's look ray (identity = -Z).
    fn aim_look(app: &mut App, rotation: Quat) {
        let rig = app
            .world_mut()
            .query_filtered::<Entity, With<SpaceshipRotationInputActiveMarker>>()
            .iter(app.world())
            .next()
            .expect("active rig");
        app.world_mut()
            .entity_mut(rig)
            .insert(PointRotationOutput(rotation));
    }

    fn press_ctrl(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ControlLeft);
    }
    fn release_ctrl(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ControlLeft);
    }
    fn travel_of(app: &App, ship: Entity) -> Option<Entity> {
        app.world().get::<TravelLock>(ship).unwrap().0
    }
    fn combat_of(app: &App, ship: Entity) -> Option<Entity> {
        app.world().get::<CombatLock>(ship).unwrap().0
    }

    #[test]
    fn radar_locks_at_the_threshold_and_retargets_while_held() {
        let (mut app, ship) = gesture_app();
        let ahead = spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));
        // A second body 90 degrees left (identity-look rotated +PI/2 about Y
        // maps -Z onto -X).
        let left = spawn_ship(&mut app, Vec3::new(-100.0, 0.0, 0.0));

        // Tap window: the search is open and the picker may already see the
        // body, but NOTHING latches or writes before the hold threshold.
        press_ctrl(&mut app);
        for _ in 0..3 {
            app.update();
        }
        let radar = *app.world().get::<RadarState>(ship).expect("search open");
        assert_eq!(radar.engaged, None, "nothing latches inside the tap window");
        assert_eq!(
            radar.candidate,
            Some(ahead),
            "the picker sees the body dead ahead (guard: the write test below is live)"
        );
        assert_eq!(
            travel_of(&app, ship),
            None,
            "no write inside the tap window"
        );

        // Cross the threshold: the slot latches (lowered = travel) and the
        // lock is written WHILE STILL HELD - no release needed.
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            app.world().get::<RadarState>(ship).unwrap().engaged,
            Some(RadarSlot::Travel),
            "the threshold latches the slot"
        );
        assert_eq!(
            travel_of(&app, ship),
            Some(ahead),
            "the lock is live under the sweep"
        );

        // Sweep to the second body: the lock retargets instantly, still held.
        aim_look(&mut app, Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        app.update();
        assert_eq!(
            travel_of(&app, ship),
            Some(left),
            "the live lock follows the sweep"
        );

        // Release: the lock sticks, the search closes.
        release_ctrl(&mut app);
        app.update();
        assert_eq!(travel_of(&app, ship), Some(left), "release = it sticks");
        assert!(
            app.world().get::<RadarState>(ship).is_none(),
            "the search closes on release"
        );
        assert_eq!(combat_of(&app, ship), None, "the engaged slot only");
    }

    #[test]
    fn raising_inside_the_tap_window_latches_the_combat_slot() {
        // Q1a (threshold latch): CTRL pressed lowered, RMB raised 100 ms
        // later - by the threshold the stance is combat, so the COMBAT slot
        // engages. Under the retired press-time latch this gesture wrote the
        // TRAVEL lock (the recorded same-frame RMB+CTRL sharp edge); this
        // test fails against that model by construction.
        let (mut app, ship) = gesture_app();
        let enemy = spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));

        press_ctrl(&mut app);
        app.update();
        app.update();
        app.world_mut().entity_mut(ship).insert(WeaponsRaised(true));
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world().get::<RadarState>(ship).unwrap().engaged,
            Some(RadarSlot::Combat),
            "the threshold latches the CURRENT stance (Q1a)"
        );
        assert_eq!(combat_of(&app, ship), Some(enemy));
        assert_eq!(
            travel_of(&app, ship),
            None,
            "press-time latching would have routed this to travel"
        );
        release_ctrl(&mut app);
        app.update();
        assert_eq!(combat_of(&app, ship), Some(enemy), "sticks");
    }

    #[test]
    fn an_empty_sweep_keeps_the_last_target() {
        // Q2a (keep-last): once acquired, sweeping over empty space never
        // drops the lock - tap is the only clear.
        let (mut app, ship) = gesture_app();
        let ahead = spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));

        press_ctrl(&mut app);
        for _ in 0..6 {
            app.update();
        }
        assert_eq!(travel_of(&app, ship), Some(ahead), "acquired (guard)");

        // Sweep to empty space (90 degrees right: nothing there).
        aim_look(
            &mut app,
            Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        );
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            app.world().get::<RadarState>(ship).unwrap().candidate,
            None,
            "the picker sees empty space (guard: keep-last is exercised)"
        );
        assert_eq!(
            travel_of(&app, ship),
            Some(ahead),
            "keep-last: the empty sweep does not drop the lock"
        );
        release_ctrl(&mut app);
        app.update();
        assert_eq!(travel_of(&app, ship), Some(ahead));
    }

    #[test]
    fn a_hold_that_never_resolves_leaves_the_locks_alone() {
        // D1 rephrased for the live model: a hold over empty space latches a
        // slot but never writes it - pre-existing locks survive.
        let (mut app, ship) = gesture_app();
        let target = spawn_ship(&mut app, Vec3::new(0.0, 0.0, 100.0));
        let enemy = spawn_ship(&mut app, Vec3::new(0.0, 100.0, 0.0));
        app.world_mut()
            .entity_mut(ship)
            .insert((TravelLock(Some(target)), CombatLock(Some(enemy))));

        press_ctrl(&mut app);
        for _ in 0..7 {
            app.update();
        }
        release_ctrl(&mut app);
        app.update();
        assert_eq!(travel_of(&app, ship), Some(target), "travel survives");
        assert_eq!(combat_of(&app, ship), Some(enemy), "combat survives");
    }

    #[test]
    fn quick_taps_clear_staged_and_disengage_goto() {
        let (mut app, ship) = gesture_app();
        let target = spawn_ship(&mut app, Vec3::new(0.0, 0.0, 100.0));
        let enemy = spawn_ship(&mut app, Vec3::new(0.0, 100.0, 0.0));
        app.world_mut()
            .entity_mut(ship)
            .insert((TravelLock(Some(target)), CombatLock(Some(enemy))));

        // A quick tap (2 frames = 100 ms < threshold): staged clear - the
        // combat lock first, travel survives...
        press_ctrl(&mut app);
        app.update();
        release_ctrl(&mut app);
        app.update();
        assert_eq!(
            combat_of(&app, ship),
            None,
            "first tap clears the combat lock"
        );
        assert_eq!(travel_of(&app, ship), Some(target));

        //...the second tap clears the travel lock and disengages a GOTO.
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Goto { target }));
        press_ctrl(&mut app);
        app.update();
        release_ctrl(&mut app);
        app.update();
        assert_eq!(
            travel_of(&app, ship),
            None,
            "second tap clears the travel lock"
        );
        assert!(
            app.world().get::<Autopilot>(ship).is_none(),
            "clearing the designation disengages the GOTO"
        );
    }

    #[test]
    fn raised_tap_clears_combat_only_and_the_boundary_release_sticks() {
        let (mut app, ship) = gesture_app();
        let target = spawn_ship(&mut app, Vec3::new(0.0, 0.0, 100.0));
        let enemy = spawn_ship(&mut app, Vec3::new(0.0, 100.0, 0.0));
        app.world_mut().entity_mut(ship).insert((
            TravelLock(Some(target)),
            CombatLock(Some(enemy)),
            WeaponsRaised(true),
        ));

        // Raised tap: combat only, never travel.
        press_ctrl(&mut app);
        app.update();
        release_ctrl(&mut app);
        app.update();
        assert_eq!(combat_of(&app, ship), None);
        assert_eq!(
            travel_of(&app, ship),
            Some(target),
            "a raised tap never touches the travel lock"
        );

        // Boundary frame: at EXACTLY the shared threshold (5 x 50 ms) the
        // Hold has fired - the lock is ALREADY live - and the Tap has
        // expired, so the release sticks the lock and clears nothing.
        let enemy2 = spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));
        press_ctrl(&mut app);
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            combat_of(&app, ship),
            Some(enemy2),
            "the lock is live at the boundary frame"
        );
        release_ctrl(&mut app);
        app.update();
        assert_eq!(
            combat_of(&app, ship),
            Some(enemy2),
            "the boundary release sticks (one shared threshold, no gap)"
        );
        assert_eq!(travel_of(&app, ship), Some(target), "and does not clear");
    }

    #[test]
    fn an_engaged_combat_sweep_holds_the_decay_at_zero() {
        // F12: an engaged combat sweep IS combat activity - a long sweep
        // must not cross the 30 s decay boundary mid-gesture.
        let (mut app, ship) = gesture_app();
        spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));
        app.world_mut().entity_mut(ship).insert(WeaponsRaised(true));

        press_ctrl(&mut app);
        for _ in 0..6 {
            app.update();
        }
        app.world_mut()
            .entity_mut(ship)
            .insert(CombatDecay(COMBAT_DECAY_SECS - 0.01));
        app.update();
        assert_eq!(
            app.world().get::<CombatDecay>(ship).unwrap().0,
            0.0,
            "the engaged combat sweep resets the decay every frame"
        );
    }

    /// THE TAP/HOLD BOUNDARY, measured. Candidate mechanism 5 was "a CTRL
    /// hold the player meant as a hold lands under `RADAR_TAP_SECS` and
    /// clears the lock instead". The rig steps at 50 ms, so this sweeps 1..=8
    /// held frames (50..400 ms) across the 250 ms threshold and records where
    /// the gesture flips from clear to commit.
    #[test]
    fn the_tap_hold_boundary_sits_exactly_at_the_shared_threshold() {
        const STEP_MS: u64 = 50;
        let frames_to_threshold = (RADAR_TAP_SECS * 1000.0) as u64 / STEP_MS;
        let mut outcomes = Vec::new();
        for held in 1..=8u64 {
            let (mut app, ship) = gesture_app();
            let ahead = spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));
            let enemy = spawn_ship(&mut app, Vec3::new(0.0, 200.0, 0.0));
            app.world_mut()
                .entity_mut(ship)
                .insert((CombatLock(Some(enemy)), WeaponsRaised(true)));

            press_ctrl(&mut app);
            for _ in 0..held {
                app.update();
            }
            release_ctrl(&mut app);
            app.update();
            outcomes.push((held * STEP_MS, combat_of(&app, ship) == Some(ahead)));
        }

        for &(ms, committed) in &outcomes {
            let expected = ms >= (RADAR_TAP_SECS * 1000.0) as u64;
            assert_eq!(
                committed,
                expected,
                "a {ms} ms hold should {} - full curve: {outcomes:?}",
                if expected {
                    "COMMIT a lock"
                } else {
                    "tap-clear"
                }
            );
        }
        assert_eq!(
            frames_to_threshold, 5,
            "guard: the sweep really does straddle the threshold"
        );
    }

    #[test]
    fn the_acquired_cue_fires_once_per_gesture() {
        // Q3a (acquire-only): one cue at the first write, silence across the
        // live retargets; a new gesture earns a new cue.
        let (mut app, ship) = gesture_app();
        spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));
        let left = spawn_ship(&mut app, Vec3::new(-100.0, 0.0, 0.0));

        press_ctrl(&mut app);
        for _ in 0..6 {
            app.update();
        }
        assert_eq!(app.world().resource::<AcquiredCueCount>().0, 1, "acquired");

        // Retarget: no new cue.
        aim_look(&mut app, Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            travel_of(&app, ship),
            Some(left),
            "guard: the retarget happened"
        );
        assert_eq!(
            app.world().resource::<AcquiredCueCount>().0,
            1,
            "retargets are silent (Q3a)"
        );

        // Release, re-hold: the next gesture cues again.
        release_ctrl(&mut app);
        app.update();
        press_ctrl(&mut app);
        for _ in 0..6 {
            app.update();
        }
        assert_eq!(app.world().resource::<AcquiredCueCount>().0, 2);
    }

    #[test]
    fn a_retarget_within_a_held_gesture_ticks_but_the_acquire_does_not() {
        // Q3a keeps the ACQUIRE cue once-per-gesture adds the subtle retarget
        // tick for the re-designations that follow. The acquire must NOT
        // count as a retarget, and the first sweep to a new body must tick.
        let (mut app, ship) = gesture_app();
        app.init_resource::<RetargetCueCount>();
        app.add_systems(
            Update,
            |mut cues: MessageReader<RadarRetargeted>, mut count: ResMut<RetargetCueCount>| {
                count.0 += cues.read().count();
            },
        );
        spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));
        let left = spawn_ship(&mut app, Vec3::new(-100.0, 0.0, 0.0));

        press_ctrl(&mut app);
        for _ in 0..6 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<AcquiredCueCount>().0,
            1,
            "acquired once"
        );
        assert_eq!(
            app.world().resource::<RetargetCueCount>().0,
            0,
            "the acquire is not a retarget - no tick on the first resolve"
        );

        // Sweep to the second body: the lock re-designates while still held.
        aim_look(&mut app, Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            travel_of(&app, ship),
            Some(left),
            "guard: the retarget actually happened"
        );
        assert_eq!(
            app.world().resource::<AcquiredCueCount>().0,
            1,
            "still just the one acquire"
        );
        assert!(
            app.world().resource::<RetargetCueCount>().0 >= 1,
            "the re-designation ticked at least once"
        );
    }

    #[test]
    fn a_lock_less_computer_cannot_radar() {
        let (mut app, ship) = gesture_app();
        // Withhold the Lock capability.
        let controller = app
            .world_mut()
            .query_filtered::<Entity, With<ControllerSectionMarker>>()
            .iter(app.world())
            .next()
            .unwrap();
        app.world_mut()
            .entity_mut(controller)
            .insert(WithheldVerbs([FlightVerb::Lock].into_iter().collect()));

        press_ctrl(&mut app);
        app.update();
        assert!(
            app.world().get::<RadarState>(ship).is_none(),
            "no Lock capability: the radar does not come on"
        );

        // Delivery guard: granting it back opens the search on the next press.
        release_ctrl(&mut app);
        app.update();
        app.world_mut()
            .entity_mut(controller)
            .insert(WithheldVerbs::default());
        press_ctrl(&mut app);
        app.update();
        assert!(app.world().get::<RadarState>(ship).is_some());
    }

    #[test]
    fn a_pause_freezes_the_live_lock_but_the_search_still_closes() {
        // Live-lock pause semantics: what was acquired BEFORE the pause is a
        // completed acquisition and sticks, with no pending commit to drop;
        // while paused the gated search neither latches nor retargets; a
        // release during the pause still tears the search down.
        let (mut app, ship) = gesture_app();
        let ahead = spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));
        let left = spawn_ship(&mut app, Vec3::new(-100.0, 0.0, 0.0));

        press_ctrl(&mut app);
        for _ in 0..6 {
            app.update();
        }
        assert_eq!(travel_of(&app, ship), Some(ahead), "acquired pre-pause");

        app.world_mut()
            .resource_mut::<NextState<nova_gameplay::PauseStates>>()
            .set(nova_gameplay::PauseStates::Paused);
        app.update();

        // A sweep during the pause retargets nothing (delivery guard: the
        // same sweep DID retarget in the live test above).
        aim_look(&mut app, Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            travel_of(&app, ship),
            Some(ahead),
            "the paused radar does not retarget"
        );
        let _ = left;

        release_ctrl(&mut app);
        app.update();
        assert!(
            app.world().get::<RadarState>(ship).is_none(),
            "the search still closes during a pause"
        );
        assert_eq!(
            travel_of(&app, ship),
            Some(ahead),
            "the pre-pause acquisition sticks"
        );
    }

    /// Give a gesture app a deterministic dwell: `base` seconds, distance-flat
    /// (range_factor 0) so the frame count is exact, and a wide clamp so the
    /// floor/ceiling never interfere. With `TimeUpdateStrategy` at 50 ms,
    /// `base` seconds is `base / 0.05` held frames past the hold threshold.
    fn set_dwell(app: &mut App, base: f32) {
        let mut settings = app.world_mut().resource_mut::<TargetingSettings>();
        settings.lock_dwell_base = base;
        settings.lock_dwell_range_factor = 0.0;
        settings.lock_dwell_min = 0.0;
        settings.lock_dwell_max = 5.0;
    }

    fn radar_of(app: &App, ship: Entity) -> RadarState {
        *app.world().get::<RadarState>(ship).expect("search open")
    }

    #[test]
    fn lock_dwell_scales_with_distance_and_clamps() {
        let s = TargetingSettings::default();
        let near = lock_dwell_secs(0.0, 1.0, &s);
        let mid = lock_dwell_secs(1000.0, 1.0, &s);
        let far = lock_dwell_secs(4000.0, 1.0, &s);
        assert!(near < mid && mid < far, "dwell grows with distance");
        assert!(
            (near - s.lock_dwell_base).abs() < 1e-6,
            "point-blank dwell is the base"
        );
        let at_ref = lock_dwell_secs(s.lock_dwell_reference_range, 1.0, &s);
        assert!(
            (at_ref - s.lock_dwell_base * (1.0 + s.lock_dwell_range_factor)).abs() < 1e-4,
            "at the reference range the distance term is full strength"
        );
        assert!(
            (far - at_ref).abs() < 1e-6,
            "the distance term saturates beyond the reference range"
        );
        let single = lock_dwell_secs(500.0, 1.0, &s);
        let doubled = lock_dwell_secs(500.0, 2.0, &s);
        assert!(
            (doubled - 2.0 * single).abs() < 1e-4,
            "the modifier is a linear multiplier (the stealth/aspect seam)"
        );
        assert!(
            (lock_dwell_secs(0.0, 100.0, &s) - s.lock_dwell_max).abs() < 1e-6,
            "a huge modifier clamps to the ceiling"
        );
        assert!(
            (lock_dwell_secs(0.0, 0.0, &s) - s.lock_dwell_min).abs() < 1e-6,
            "a zero modifier clamps to the floor"
        );

        // Misordered knobs (min > max) must not panic (f32::clamp would); the
        // floor wins.
        let mut bad = s.clone();
        bad.lock_dwell_min = 3.0;
        bad.lock_dwell_max = 1.0;
        assert!(
            (lock_dwell_secs(0.0, 1.0, &bad) - 3.0).abs() < 1e-6,
            "an out-of-order min/max clamps to the floor without panicking"
        );
    }

    #[test]
    fn the_dwell_gates_the_slot_commit_and_the_ring_fills() {
        // A 1.0 s dwell (20 held frames): after the threshold the candidate is
        // PENDING (ring filling) and nothing is written; only once the dwell
        // completes does the slot commit and the acquire cue fire.
        let (mut app, ship) = gesture_app();
        set_dwell(&mut app, 1.0);
        let ahead = spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));

        // Threshold (5) + a few dwell frames: still charging.
        press_ctrl(&mut app);
        for _ in 0..10 {
            app.update();
        }
        let radar = radar_of(&app, ship);
        assert_eq!(radar.candidate, Some(ahead), "the picker settled");
        assert_eq!(
            radar.dwell_target,
            Some(ahead),
            "the dwell is charging on it"
        );
        assert!(
            (radar.dwell_needed - 1.0).abs() < 1e-6,
            "the needed dwell is cached for the ring HUD"
        );
        assert!(radar.is_dwelling(), "an active dwell reads as dwelling");
        let fraction = radar.dwell_fill();
        assert!(
            fraction > 0.0 && fraction < 1.0,
            "the ring is partway (fraction {fraction})"
        );
        assert_eq!(
            travel_of(&app, ship),
            None,
            "nothing commits while the dwell is incomplete"
        );
        assert_eq!(
            app.world().resource::<AcquiredCueCount>().0,
            0,
            "no acquire cue before the snap"
        );

        // Hold through the rest of the dwell: it commits and cues once.
        for _ in 0..20 {
            app.update();
        }
        assert_eq!(
            travel_of(&app, ship),
            Some(ahead),
            "the completed dwell hard-commits the slot"
        );
        assert_eq!(
            app.world().resource::<AcquiredCueCount>().0,
            1,
            "the acquire cue fires at the snap, once"
        );
        assert!(
            !radar_of(&app, ship).is_dwelling(),
            "a completed dwell no longer reads as dwelling (the ring hides)"
        );
    }

    #[test]
    fn sweeping_off_before_the_ring_fills_cancels_and_keeps_last() {
        // A prior travel lock plus a 1.0 s dwell: charge partway on a new
        // body, then sweep to empty space - the dwell resets (cancel) and the
        // prior lock is kept (keep-last), nothing new commits.
        let (mut app, ship) = gesture_app();
        set_dwell(&mut app, 1.0);
        let prior = spawn_ship(&mut app, Vec3::new(0.0, 0.0, 100.0)); // behind
        let ahead = spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));
        app.world_mut()
            .entity_mut(ship)
            .insert(TravelLock(Some(prior)));

        press_ctrl(&mut app);
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(
            radar_of(&app, ship).dwell_target,
            Some(ahead),
            "charging on the new body"
        );
        assert_eq!(
            travel_of(&app, ship),
            Some(prior),
            "the prior lock still holds mid-dwell (keep-last)"
        );

        // Sweep 90 degrees right onto empty space before the dwell completes.
        aim_look(
            &mut app,
            Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        );
        for _ in 0..3 {
            app.update();
        }
        let radar = radar_of(&app, ship);
        assert_eq!(radar.candidate, None, "the picker sees empty space");
        assert_eq!(radar.dwell_target, None, "the dwell was cancelled");
        assert_eq!(radar.dwell_secs, 0.0, "and reset to zero");
        assert_eq!(
            travel_of(&app, ship),
            Some(prior),
            "keep-last: the cancelled acquisition never touched the lock"
        );
    }

    #[test]
    fn a_re_designation_earns_its_own_dwell() {
        // Commit `ahead`, then sweep to `left`: the committed lock keeps-last
        // through the SECOND dwell and only moves once it completes, firing the
        // retarget tick (not a fresh acquire).
        let (mut app, ship) = gesture_app();
        set_dwell(&mut app, 1.0);
        app.init_resource::<RetargetCueCount>();
        app.add_systems(
            Update,
            |mut cues: MessageReader<RadarRetargeted>, mut count: ResMut<RetargetCueCount>| {
                count.0 += cues.read().count();
            },
        );
        let ahead = spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));
        let left = spawn_ship(&mut app, Vec3::new(-100.0, 0.0, 0.0));

        press_ctrl(&mut app);
        for _ in 0..30 {
            app.update();
        }
        assert_eq!(travel_of(&app, ship), Some(ahead), "first lock committed");
        assert_eq!(app.world().resource::<AcquiredCueCount>().0, 1);

        // Sweep to the left body: a fresh dwell starts, the old lock holds.
        aim_look(&mut app, Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(
            radar_of(&app, ship).dwell_target,
            Some(left),
            "the re-designation is charging its own dwell"
        );
        assert_eq!(
            travel_of(&app, ship),
            Some(ahead),
            "keep-last: the lock stays on the first target until the new dwell completes"
        );
        assert_eq!(
            app.world().resource::<RetargetCueCount>().0,
            0,
            "no retarget tick before the second dwell completes"
        );

        for _ in 0..20 {
            app.update();
        }
        assert_eq!(
            travel_of(&app, ship),
            Some(left),
            "the second dwell moves the lock"
        );
        assert_eq!(
            app.world().resource::<AcquiredCueCount>().0,
            1,
            "a re-designation is not a fresh acquire"
        );
        assert!(
            app.world().resource::<RetargetCueCount>().0 >= 1,
            "it ticks the retarget cue at the snap"
        );
    }

    #[test]
    fn the_combat_slot_is_dwell_gated_too() {
        // The dwell gates BOTH slots, not combat only: a raised (combat)
        // gesture is pending mid-dwell and commits at completion.
        let (mut app, ship) = gesture_app();
        set_dwell(&mut app, 1.0);
        app.world_mut().entity_mut(ship).insert(WeaponsRaised(true));
        let enemy = spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));

        press_ctrl(&mut app);
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(
            radar_of(&app, ship).engaged,
            Some(RadarSlot::Combat),
            "the combat slot latched at the threshold"
        );
        assert_eq!(
            combat_of(&app, ship),
            None,
            "but the combat lock is pending through the dwell"
        );

        for _ in 0..20 {
            app.update();
        }
        assert_eq!(
            combat_of(&app, ship),
            Some(enemy),
            "the completed dwell commits the combat lock"
        );
    }
}
