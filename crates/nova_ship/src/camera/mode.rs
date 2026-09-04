//! Camera mode and stance: derive [`SpaceshipCameraControlMode`] and
//! [`WeaponsRaised`] from the held inputs each frame, move the active-rig
//! marker to match, and route the look input onto whichever rig is live.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_gameplay::{prelude::*, transform::prelude::PointRotationInput};

use super::rig::{
    CameraInputRotate, CombatInput, FreeLookInput, SpaceshipCameraFreeLookInputMarker,
    SpaceshipCameraInputMarker, SpaceshipCameraNormalInputMarker, SpaceshipCameraTurretInputMarker,
    SpaceshipRotationInputActiveMarker,
};
use crate::prelude::*;

/// The mode that the camera is currently in for controlling the spaceship.
///
/// Derived each frame from the HELD state of the mode inputs (Turret while
/// RMB/CombatInput is held, else FreeLook while Alt/FreeLookInput is held, else
/// Normal). Memoryless by design: any press/release order in any nesting lands
/// on the right mode, which last-writer-wins observers cannot guarantee - an
/// Alt-release while RMB is held stomps the mode back to Normal. `PartialEq` +
/// `set_if_neq` keep `is_changed()` meaningful for the rig-sync system.
#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub enum SpaceshipCameraControlMode {
    /// Default flight framing; look input steers the ship.
    #[default]
    Normal,
    /// Look around freely without steering the ship (Alt held).
    FreeLook,
    /// Aim mode: look input drives manual turret aim (RMB held).
    Turret,
}

/// Weapons-raised: the gameplay-facing flag for "the player is holding the
/// combat stance" (RMB/CombatInput held), derived each frame onto the PLAYER
/// ship root alongside the camera mode. Gameplay consumers (the radar slot
/// latch, the weapons safety, manual turret aim) read THIS component, never the
/// camera enum: the enum is a camera concern, and routing gameplay off it is a
/// known bug class. Living on the ship root means a respawn starts lowered for
/// free.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
pub struct WeaponsRaised(pub bool);

pub(super) fn sync_spaceship_control_mode(
    mut commands: Commands,
    mode: Res<SpaceshipCameraControlMode>,
    _spaceship: Single<&Transform, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    // The OUTGOING rig: the marker still sits on the rig being left this
    // frame (marker moves below are command-flushed), so its output is the
    // live look at transition time - the seed for the incoming rig. Seeding
    // unconditionally from the NORMAL rig instead snaps a raise out of
    // FreeLook to wherever that rig last pointed rather than to the flanker
    // being looked at.
    active_output: Query<
        &PointRotationOutput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipRotationInputActiveMarker>,
        ),
    >,
    spaceship_input_rotation: Single<Entity, With<SpaceshipCameraNormalInputMarker>>,
    spaceship_input_free_look: Single<Entity, With<SpaceshipCameraFreeLookInputMarker>>,
    spaceship_input_turret: Single<Entity, With<SpaceshipCameraTurretInputMarker>>,
) {
    if !mode.is_changed() {
        return;
    }

    let seed = active_output
        .iter()
        .next()
        .map(|output| **output)
        .unwrap_or_default();
    let spaceship_input_rotation = spaceship_input_rotation.into_inner();
    let spaceship_input_free_look = spaceship_input_free_look.into_inner();
    let spaceship_input_combat = spaceship_input_turret.into_inner();

    match *mode {
        // The NORMAL rig is deliberately never re-seeded on return: it drives
        // the SHIP's PD rotation, and seeding it from a free-look/turret
        // direction would steer the hull to wherever the player was looking.
        SpaceshipCameraControlMode::Normal => {
            commands
                .entity(spaceship_input_rotation)
                .insert(SpaceshipRotationInputActiveMarker);
            commands
                .entity(spaceship_input_free_look)
                .remove::<SpaceshipRotationInputActiveMarker>();
            commands
                .entity(spaceship_input_combat)
                .remove::<SpaceshipRotationInputActiveMarker>();
        }
        SpaceshipCameraControlMode::FreeLook => {
            commands
                .entity(spaceship_input_rotation)
                .remove::<SpaceshipRotationInputActiveMarker>();
            commands
                .entity(spaceship_input_free_look)
                .insert(PointRotation {
                    initial_rotation: seed,
                })
                .insert(SpaceshipRotationInputActiveMarker);
            commands
                .entity(spaceship_input_combat)
                .remove::<SpaceshipRotationInputActiveMarker>();
        }
        SpaceshipCameraControlMode::Turret => {
            commands
                .entity(spaceship_input_rotation)
                .remove::<SpaceshipRotationInputActiveMarker>();
            commands
                .entity(spaceship_input_free_look)
                .remove::<SpaceshipRotationInputActiveMarker>();
            commands
                .entity(spaceship_input_combat)
                .insert(PointRotation {
                    initial_rotation: seed,
                })
                .insert(SpaceshipRotationInputActiveMarker);
        }
    }
    // The ChaseCamera fields themselves (offset/focus/smoothing) are owned by
    // `update_camera_rig`, chained after this system - never re-inserted (an
    // insert would fire the chase rig's observer and reset the anchor to the origin for
    // a frame, the visible snap this system's history fixed), and never
    // written only-on-change (a respawned camera would lose them).
}

pub(super) fn on_rotation_input(
    fire: On<Fire<CameraInputRotate>>,
    mut q_input: Query<
        &mut PointRotationInput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipRotationInputActiveMarker>,
        ),
    >,
    q_rcs: Query<(), (With<PlayerSpaceshipMarker>, With<RcsActive>)>,
    pause: Res<State<nova_gameplay::PauseStates>>,
    control: Option<Res<PlayerControlSuspended>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up. Releases stay ungated so held keys
    // clear cleanly during a pause.
    if pause.get().is_frozen()
        || crate::input::player::control::player_control_is_suspended(control)
    {
        return;
    }

    // While RCS fine-adjust is held the mouse is repurposed to translation to
    // translation, so it must not orbit the camera either. ZERO the rig rate
    // rather than merely skipping the write: `point_rotation_update_system`
    // integrates the rate every frame, so a stale nonzero value left over from
    // a mouse that was moving at the moment SHIFT was pressed would keep
    // drifting the view. Held at zero, the rig quat stays at the frozen
    // heading, so the helm resumes on exit without a snap (no re-seed, unlike
    // the autopilot).
    if !q_rcs.is_empty() {
        for mut input in &mut q_input {
            **input = Vec2::ZERO;
        }
        return;
    }

    for mut input in &mut q_input {
        **input = fire.value;
    }
}

pub(super) fn on_rotation_input_completed(
    _: On<Complete<CameraInputRotate>>,
    mut q_input: Query<&mut PointRotationInput, With<SpaceshipCameraInputMarker>>,
) {
    for mut input in &mut q_input {
        **input = Vec2::ZERO;
    }
}

/// Whether a held bool action currently fires, read from its action entity's
/// state (the `cycle_modifier_held` pattern - a plain Down-conditioned action
/// reports `Fired` while its key is held).
fn action_held<A: InputAction>(q: &Query<&TriggerState, With<Action<A>>>) -> bool {
    q.iter().any(|&state| state == TriggerState::Fired)
}

/// Derive the camera control mode AND the weapons-raised flag from the HELD
/// state of the mode inputs, each frame: Turret while CombatInput is held
/// (priority), else FreeLook while FreeLookInput is held, else Normal. Replaces
/// the four last-writer-wins observers: memoryless, so nested holds (Alt during
/// RMB, either release order) always land on the right mode, and a
/// press+release entirely inside a pause leaves no trace - the state after
/// unpause is a function of what is held NOW. Deliberately not pause-gated,
/// like the camera chain it heads: the mode is a camera concern, and every
/// gameplay consumer of [`WeaponsRaised`] is pause-gated itself.
pub(super) fn derive_control_mode_and_raised(
    mut commands: Commands,
    mut mode: ResMut<SpaceshipCameraControlMode>,
    q_combat: Query<&TriggerState, With<Action<CombatInput>>>,
    q_free_look: Query<&TriggerState, With<Action<FreeLookInput>>>,
    control: Option<Res<PlayerControlSuspended>>,
    mut q_ship: Query<
        (Entity, Option<&mut WeaponsRaised>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let suspended = crate::input::player::control::player_control_is_suspended(control);
    let combat_held = !suspended && action_held(&q_combat);
    let next = if combat_held {
        SpaceshipCameraControlMode::Turret
    } else if !suspended && action_held(&q_free_look) {
        SpaceshipCameraControlMode::FreeLook
    } else {
        SpaceshipCameraControlMode::Normal
    };
    mode.set_if_neq(next);

    // The raised flag mirrors the combat hold onto the player ship root
    // (self-healing insert: a fresh ship starts lowered and gains the flag on
    // its first frame).
    for (ship, raised) in &mut q_ship {
        match raised {
            Some(mut raised) => {
                raised.set_if_neq(WeaponsRaised(combat_held));
            }
            None => {
                commands.entity(ship).insert(WeaponsRaised(combat_held));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::input::InputPlugin;

    use super::*;
    use crate::camera::{
        framing::{update_camera_rig, CAMERA_SMOOTHING},
        rig::PlayerInputMarker,
        SpaceshipCameraController,
    };

    /// Switching camera mode must retune the chase offsets without resetting
    /// the anchor to the origin. Re-inserting `ChaseCamera` (the previous
    /// approach) fired the chase rig's insert observer, which reset `ChaseCameraInput` to
    /// the origin for a frame - the visible one-frame snap.
    #[test]
    fn switching_camera_mode_keeps_the_anchor_off_origin() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(ChaseCameraPlugin);
        app.init_resource::<SpaceshipCameraControlMode>();
        app.add_systems(
            Update,
            (sync_spaceship_control_mode, update_camera_rig).chain(),
        );

        // A player ship far from the origin, plus the input rig
        // `sync_spaceship_control_mode` drives (one active-marked normal input,
        // a free-look input, a turret input).
        let anchor = Vec3::new(100.0, 20.0, -50.0);
        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(anchor),
        ));
        app.world_mut().spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraNormalInputMarker,
            SpaceshipRotationInputActiveMarker,
            PointRotationOutput::default(),
        ));
        app.world_mut().spawn(SpaceshipCameraFreeLookInputMarker);
        app.world_mut().spawn(SpaceshipCameraTurretInputMarker);
        let camera = app.world_mut().spawn(SpaceshipCameraController).id();

        // First frame initializes `ChaseCameraInput`; set the anchor as the
        // per-frame input system (`update_chase_camera_input`) would.
        app.update();
        app.world_mut()
            .get_mut::<ChaseCameraInput>(camera)
            .expect("ChaseCameraInput should be initialized by the chase plugin")
            .anchor_pos = anchor;

        // Switch to FreeLook.
        *app.world_mut().resource_mut::<SpaceshipCameraControlMode>() =
            SpaceshipCameraControlMode::FreeLook;
        app.update();

        // The switch applied the mode rig's weight-giving smoothing.
        assert_eq!(
            app.world().get::<ChaseCamera>(camera).unwrap().smoothing,
            CAMERA_SMOOTHING,
            "mode switches must (re)apply the gameplay camera smoothing"
        );

        // The anchor survives the switch, rather than resetting to the origin
        // for a frame....
        assert_eq!(
            app.world()
                .get::<ChaseCameraInput>(camera)
                .unwrap()
                .anchor_pos,
            anchor,
            "switching camera mode must not reset the chase anchor to the origin"
        );
        // ...and the offsets now reflect FreeLook.
        assert_eq!(
            app.world().get::<ChaseCamera>(camera).unwrap().offset,
            Vec3::new(0.0, 10.0, -30.0)
        );
    }

    /// Distinct per-rig rotations so a seed from the WRONG rig fails loudly.
    fn rot(deg: f32) -> Quat {
        Quat::from_rotation_y(deg.to_radians())
    }

    /// A mode-derivation app with the REAL input stack (InputPlugin +
    /// EnhancedInput + the production action bindings) and FAITHFUL SPLIT
    /// RIGS - one entity per mode, only one holding the active marker, each
    /// with its own distinct PointRotationOutput (a single both-marker rig
    /// masks exactly the frozen-ray/seeding bug class this task fixes).
    /// Returns (app, normal, freelook, turret, ship).
    fn mode_app() -> (App, Entity, Entity, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<nova_gameplay::PauseStates>();
        app.init_resource::<SpaceshipCameraControlMode>();
        app.add_input_context::<PlayerInputMarker>();
        app.add_systems(
            Update,
            (derive_control_mode_and_raised, sync_spaceship_control_mode).chain(),
        );

        let normal = app
            .world_mut()
            .spawn((
                SpaceshipCameraInputMarker,
                SpaceshipCameraNormalInputMarker,
                SpaceshipRotationInputActiveMarker,
                PointRotation::default(),
                PointRotationOutput(rot(0.0)),
            ))
            .id();
        let freelook = app
            .world_mut()
            .spawn((
                SpaceshipCameraInputMarker,
                SpaceshipCameraFreeLookInputMarker,
                PointRotationOutput(rot(45.0)),
            ))
            .id();
        let turret = app
            .world_mut()
            .spawn((
                SpaceshipCameraInputMarker,
                SpaceshipCameraTurretInputMarker,
                PointRotationOutput(rot(90.0)),
            ))
            .id();
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
            ))
            .id();
        // The context registry finalizes in App::finish, so run the plugin
        // lifecycle BEFORE spawning the action rig, like the production app
        // does (same sequencing as the wheel-routing e2e test).
        app.finish();
        app.cleanup();
        app.update();
        // The production action rig (insert_player_input's shape), so the
        // derivation reads REAL TriggerStates driven by device input.
        app.world_mut().spawn((
            PlayerInputMarker,
            actions!(
                PlayerInputMarker[
                    (
                        Action::<FreeLookInput>::new(),
                        bindings![KeyCode::AltLeft, GamepadButton::LeftTrigger]
                    ),
                    (
                        Action::<CombatInput>::new(),
                        bindings![MouseButton::Right, GamepadButton::LeftTrigger2]
                    ),
                ]
            ),
        ));
        app.update();
        (app, normal, freelook, turret, ship)
    }

    fn mode_of(app: &App) -> SpaceshipCameraControlMode {
        app.world().resource::<SpaceshipCameraControlMode>().clone()
    }

    fn active_rig(app: &mut App) -> Entity {
        let mut rigs: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<SpaceshipRotationInputActiveMarker>>()
            .iter(app.world())
            .collect();
        assert_eq!(rigs.len(), 1, "exactly one rig holds the active marker");
        rigs.pop().unwrap()
    }

    fn raised(app: &App, ship: Entity) -> bool {
        app.world()
            .get::<WeaponsRaised>(ship)
            .map(|raised| raised.0)
            .unwrap_or(false)
    }

    fn seed_of(app: &App, rig: Entity) -> Quat {
        app.world()
            .get::<PointRotation>(rig)
            .expect("rig has a PointRotation")
            .initial_rotation
    }

    fn press_rmb(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
    }
    fn release_rmb(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Right);
    }
    fn press_alt(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::AltLeft);
    }
    fn release_alt(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::AltLeft);
    }

    /// The full nested-hold transition matrix: every press/release order
    /// lands on the derived mode, the marker follows, and the raised flag
    /// mirrors the combat hold. Last-writer-wins observers fail the "release
    /// Alt while RMB held" step - mode stomped to Normal while raised, which
    /// is manual aim on a frozen ray.
    #[test]
    fn nested_holds_always_land_on_the_derived_mode() {
        let (mut app, normal, freelook, turret, ship) = mode_app();
        assert!(matches!(mode_of(&app), SpaceshipCameraControlMode::Normal));
        assert_eq!(active_rig(&mut app), normal);
        assert!(!raised(&app, ship), "spawn state is lowered");

        // RMB -> Turret, raised.
        press_rmb(&mut app);
        app.update();
        assert!(matches!(mode_of(&app), SpaceshipCameraControlMode::Turret));
        assert_eq!(active_rig(&mut app), turret);
        assert!(raised(&app, ship));

        // Alt pressed WHILE RMB held: Turret has priority; nothing moves.
        press_alt(&mut app);
        app.update();
        assert!(matches!(mode_of(&app), SpaceshipCameraControlMode::Turret));
        assert_eq!(active_rig(&mut app), turret);
        assert!(raised(&app, ship));

        // RMB released while Alt held: FreeLook, not Normal, and lowered.
        release_rmb(&mut app);
        app.update();
        assert!(matches!(
            mode_of(&app),
            SpaceshipCameraControlMode::FreeLook
        ));
        assert_eq!(active_rig(&mut app), freelook);
        assert!(!raised(&app, ship));

        // Alt released: back to Normal.
        release_alt(&mut app);
        app.update();
        assert!(matches!(mode_of(&app), SpaceshipCameraControlMode::Normal));
        assert_eq!(active_rig(&mut app), normal);

        // The other release order: Alt first, then RMB joins, then Alt
        // releases - Turret must SURVIVE the Alt release (old bug: Normal).
        press_alt(&mut app);
        app.update();
        assert!(matches!(
            mode_of(&app),
            SpaceshipCameraControlMode::FreeLook
        ));
        press_rmb(&mut app);
        app.update();
        assert!(matches!(mode_of(&app), SpaceshipCameraControlMode::Turret));
        release_alt(&mut app);
        app.update();
        assert!(
            matches!(mode_of(&app), SpaceshipCameraControlMode::Turret),
            "releasing Alt while RMB is held must keep Turret"
        );
        assert_eq!(active_rig(&mut app), turret);
        assert!(raised(&app, ship));
        release_rmb(&mut app);
        app.update();
        assert!(matches!(mode_of(&app), SpaceshipCameraControlMode::Normal));
    }

    /// Transition seeding takes the OUTGOING rig's live output: raising out
    /// of FreeLook aims where the free look pointed (45 deg here), NOT where
    /// the normal rig last pointed (0 deg - the pre-fix source; distinct
    /// rotations make the wrong source fail). Returning to Normal never
    /// re-seeds the normal rig (it steers the SHIP).
    #[test]
    fn transitions_seed_from_the_outgoing_rig() {
        let (mut app, normal, freelook, turret, _ship) = mode_app();
        let normal_seed_before = seed_of(&app, normal);

        // Normal -> FreeLook: seeded from the normal rig's output (0 deg).
        press_alt(&mut app);
        app.update();
        assert!(seed_of(&app, freelook).angle_between(rot(0.0)) < 1e-4);

        // Simulate free-looking at a flanker: the freelook rig's LIVE output
        // moves to 45 deg (already its spawn value; make it explicit).
        app.world_mut()
            .entity_mut(freelook)
            .insert(PointRotationOutput(rot(45.0)));

        // FreeLook -> Turret (raise while free-looking): the turret rig must
        // seed from the FREELOOK output (45 deg), not the normal rig (0 deg).
        press_rmb(&mut app);
        app.update();
        assert!(
            seed_of(&app, turret).angle_between(rot(45.0)) < 1e-4,
            "raising out of FreeLook must aim at the flanker being looked at"
        );

        // Back to Normal: the normal rig is deliberately NOT re-seeded.
        release_rmb(&mut app);
        release_alt(&mut app);
        app.update();
        assert_eq!(
            seed_of(&app, normal),
            normal_seed_before,
            "the ship-steering rig must never be seeded from a look direction"
        );
    }

    /// A press+release entirely inside a pause leaves NO trace after
    /// unpause (memoryless derivation - the state is a function of what is
    /// held NOW), and a press HELD through the unpause is honored. The
    /// delivery guard is the held case: the same gesture demonstrably CAN
    /// raise, so the no-trace assertion is not vacuous.
    #[test]
    fn pause_gestures_leave_no_trace_after_unpause() {
        let (mut app, _normal, _freelook, _turret, ship) = mode_app();
        app.world_mut()
            .resource_mut::<NextState<nova_gameplay::PauseStates>>()
            .set(nova_gameplay::PauseStates::Paused);
        app.update();

        // Press AND release inside the pause.
        press_rmb(&mut app);
        app.update();
        release_rmb(&mut app);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<nova_gameplay::PauseStates>>()
            .set(nova_gameplay::PauseStates::Unpaused);
        app.update();
        assert!(matches!(mode_of(&app), SpaceshipCameraControlMode::Normal));
        assert!(
            !raised(&app, ship),
            "a paused press+release leaves no trace"
        );

        // Press inside the pause, HELD through unpause: honored.
        app.world_mut()
            .resource_mut::<NextState<nova_gameplay::PauseStates>>()
            .set(nova_gameplay::PauseStates::Paused);
        app.update();
        press_rmb(&mut app);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<nova_gameplay::PauseStates>>()
            .set(nova_gameplay::PauseStates::Unpaused);
        app.update();
        assert!(
            raised(&app, ship),
            "a hold surviving the pause reflects real current intent"
        );
    }

    /// While RCS fine-adjust is held, the mouse is repurposed to translation,
    /// so `on_rotation_input` must ZERO the rig rate - not merely skip -
    /// because the bcs integrator applies the rate every frame and a stale
    /// value (mouse moving at the instant SHIFT was pressed) would drift the
    /// view. Revert the fix (write `fire.value`, or early-return leaving the
    /// stale rate) and the rate stays non-zero and this fails.
    #[test]
    fn rcs_zeroes_the_rig_rate_so_the_view_does_not_drift() {
        use bevy::input::mouse::MouseMotion;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<nova_gameplay::PauseStates>();
        app.add_input_context::<PlayerInputMarker>();
        app.add_observer(on_rotation_input);

        // The active normal rig, seeded with a NON-ZERO rate - as if the mouse
        // were moving at the moment RCS was entered.
        let rig = app
            .world_mut()
            .spawn((
                SpaceshipCameraInputMarker,
                SpaceshipCameraNormalInputMarker,
                SpaceshipRotationInputActiveMarker,
                PointRotation::default(),
                PointRotationInput(Vec2::new(0.3, -0.2)),
                PointRotationOutput(rot(0.0)),
            ))
            .id();
        // A player ship already holding RCS.
        app.world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker, RcsActive));

        app.finish();
        app.cleanup();
        app.update();
        // The camera rotate action, bound to mouse motion like production.
        app.world_mut().spawn((
            PlayerInputMarker,
            actions!(PlayerInputMarker[
                (
                    Action::<CameraInputRotate>::new(),
                    Bindings::spawn(Spawn((Binding::mouse_motion(), Scale::splat(1.0)))),
                ),
            ]),
        ));
        app.update();

        // Mouse moves while RCS is held.
        app.world_mut().write_message(MouseMotion {
            delta: Vec2::new(15.0, 8.0),
        });
        app.update();

        assert_eq!(
            app.world().get::<PointRotationInput>(rig).unwrap().0,
            Vec2::ZERO,
            "RCS holds the rig rate at zero so the view does not drift"
        );
    }
}
