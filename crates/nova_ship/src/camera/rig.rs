//! The camera-controller entity: the rigs it spawns (normal, free-look,
//! turret), the markers that tag them, the player input actions bound onto
//! it, and the live look ray gameplay reads.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_gameplay::prelude::*;
use nova_input::prelude::*;

use super::{chase::ChaseCamera, handback::CameraHandbackBlend};

/// Marker component to identify the camera controller for the player's
/// spaceship.
///
/// This should be added to an entity that also has a `ChaseCamera` component.
#[derive(Component, Debug, Clone, Reflect)]
#[require(ChaseCamera)]
pub struct SpaceshipCameraController;

/// The live look ray: the [`PointRotationOutput`] of whichever camera rig
/// currently holds [`SpaceshipRotationInputActiveMarker`] - Normal, FreeLook
/// or Turret. Consumers that need "where is the player looking RIGHT NOW"
/// (the targeting picker, the radar) read this instead of pinning a specific
/// rig, whose output freezes the moment its mode is left (the frozen-ray
/// bug).
///
/// Press-frame property: on the frame a mode transition begins, the active
/// marker still sits on the OUTGOING rig (marker moves are command-flushed
/// after the sync system), so this accessor is the live look at press time.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ActiveLookRay<'w, 's> {
    query: Query<
        'w,
        's,
        &'static PointRotationOutput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipRotationInputActiveMarker>,
        ),
    >,
}

impl ActiveLookRay<'_, '_> {
    /// The active rig's rotation, or `None` when no rig exists (menu states,
    /// headless tests without a camera).
    pub fn rotation(&self) -> Option<Quat> {
        self.query.iter().next().map(|output| **output)
    }

    /// The active look direction (unit vector), if a rig exists.
    pub fn direction(&self) -> Option<Vec3> {
        self.rotation()
            .map(|rotation| (rotation * Vec3::NEG_Z).normalize())
    }
}

/// General Marker for the rotation input of the spaceship camera.
#[derive(Component, Debug, Clone)]
pub struct SpaceshipCameraInputMarker;

/// Marker for the rotation input of the spaceship camera in normal mode.
#[derive(Component, Debug, Clone)]
pub struct SpaceshipCameraNormalInputMarker;

/// Marker for the rotation input of the spaceship camera in free look mode.
#[derive(Component, Debug, Clone)]
pub struct SpaceshipCameraFreeLookInputMarker;

/// Marker for the rotation input of the spaceship camera in turret mode.
#[derive(Component, Debug, Clone)]
pub struct SpaceshipCameraTurretInputMarker;

/// Tags the one camera rig whose look ray is currently live (moves with the
/// active [`SpaceshipCameraControlMode`](super::mode::SpaceshipCameraControlMode));
/// [`ActiveLookRay`] reads through it.
#[derive(Component, Debug, Clone)]
pub struct SpaceshipRotationInputActiveMarker;

pub(super) fn insert_camera_controller(
    add: On<Add, SpaceshipCameraController>,
    mut commands: Commands,
    q_camera: Query<Entity, With<SpaceshipCameraController>>,
) {
    let entity = add.entity;
    trace!("insert_camera_controller: entity {:?}", entity);

    let Ok(camera) = q_camera.get(entity) else {
        error!(
            "insert_camera_controller: entity {:?} not found in q_camera",
            add.entity
        );
        return;
    };

    commands
        .entity(camera)
        .insert(ChaseCamera::default())
        // A fresh controller starts blend-free: a stale handback blend
        // surviving a death/respawn path that skipped the teardown would
        // play a wrong 0.45s swing on the first frame of the new life.
        .remove::<CameraHandbackBlend>()
        .with_children(|parent| {
            parent.spawn((
                SpaceshipCameraInputMarker,
                SpaceshipCameraNormalInputMarker,
                SpaceshipRotationInputActiveMarker,
                PointRotation::default(),
            ));
        });
}

pub(super) fn insert_camera_freelook(
    add: On<Add, SpaceshipCameraController>,
    mut commands: Commands,
    q_camera: Query<Entity, (With<ChaseCamera>, With<SpaceshipCameraController>)>,
) {
    let entity = add.entity;
    trace!("insert_camera_controller: entity {:?}", entity);

    let Ok(camera) = q_camera.get(entity) else {
        error!(
            "insert_camera_controller: entity {:?} not found in q_camera",
            entity
        );
        return;
    };

    commands.entity(camera).with_children(|parent| {
        parent.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraFreeLookInputMarker,
            PointRotation::default(),
        ));
    });
}

pub(super) fn insert_camera_turret(
    add: On<Add, SpaceshipCameraController>,
    mut commands: Commands,
    q_camera: Query<Entity, (With<ChaseCamera>, With<SpaceshipCameraController>)>,
) {
    let entity = add.entity;
    trace!("insert_camera_turret: entity {:?}", entity);

    let Ok(camera) = q_camera.get(entity) else {
        error!(
            "insert_camera_turret: entity {:?} not found in q_camera",
            entity
        );
        return;
    };

    commands.entity(camera).with_children(|parent| {
        parent.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotation::default(),
        ));
    });
}

pub(super) fn insert_player_input(
    add: On<Add, SpaceshipCameraController>,
    mut commands: Commands,
    q_camera: Query<Entity, (With<ChaseCamera>, With<SpaceshipCameraController>)>,
    bindings: Res<InputBindings>,
) {
    let entity = add.entity;
    trace!("insert_camera_turret: entity {:?}", entity);

    let Ok(camera) = q_camera.get(entity) else {
        error!(
            "insert_player_input: entity {:?} not found in q_camera",
            entity
        );
        return;
    };

    // Spawn a player input controller entity to hold the input from the player
    commands.entity(camera).with_children(|parent| {
        parent.spawn(player_input_rig(&bindings));
    });
}

/// The camera rig bundle, bound from the registry. A named fn so the rebind
/// rebuild below spawns the SAME rig the observer does, rather than a second
/// copy that can drift from it.
fn player_input_rig(bindings: &InputBindings) -> impl Bundle {
    (
        Name::new("Player Input Controller"),
        PlayerInputMarker,
        actions!(
            PlayerInputMarker[
                (
                    Name::new("Input: Camera Rotate"),
                    Action::<CameraInputRotate>::new(),
                    // The two look axes carry modifiers per binding, so
                    // they are spawned beside whatever the registry holds.
                    bindings.bundle_with(
                        "camera_rotate",
                        (
                            Spawn((
                                Binding::mouse_motion(),
                                mouse_sensitivity(MousePath::Look),
                                Negate::all(),
                            )),
                            Axial::right_stick().with((Scale::splat(2.0), Negate::none())),
                        ),
                    ),
                ),
                (
                    Name::new("Input: Free Look Mode"),
                    Action::<FreeLookInput>::new(),
                    bindings.bundle("free_look"),
                ),
                (
                    Name::new("Input: Combat Mode"),
                    Action::<CombatInput>::new(),
                    bindings.bundle("combat_stance"),
                ),
            ]
        ),
    )
}

/// Rebuild the camera rig when the table moves, for the same reason the flight
/// rig is rebuilt: the rig snapshots the registry when the camera appears, and
/// the pause overlay rebinds while both already exist.
pub(super) fn rebuild_player_input_on_rebind(
    mut commands: Commands,
    bindings: Res<InputBindings>,
    q_rig: Query<(Entity, &ChildOf), With<PlayerInputMarker>>,
) {
    let cameras: Vec<Entity> = q_rig
        .iter()
        .map(|(rig, child_of)| {
            commands.entity(rig).try_despawn();
            child_of.parent()
        })
        .collect();
    for camera in cameras {
        commands.entity(camera).with_children(|parent| {
            parent.spawn(player_input_rig(&bindings));
        });
    }
}

pub(super) fn destroy_camera_controller(
    remove: On<Remove, SpaceshipCameraController>,
    mut commands: Commands,
    q_camera: Query<&Children, With<ChaseCamera>>,
) {
    let entity = remove.entity;
    trace!("destroy_camera_controller: entity {:?}", entity);

    let Ok(children) = q_camera.get(entity) else {
        error!(
            "destroy_camera_controller: entity {:?} not found in q_camera",
            entity
        );
        return;
    };

    for child in children.iter() {
        commands.entity(child).try_despawn();
    }

    commands
        .entity(entity)
        .try_remove::<(ChaseCamera, SpaceshipCameraController, CameraHandbackBlend)>();
}

#[derive(Component, Debug, Clone)]
pub(super) struct PlayerInputMarker;

#[derive(InputAction)]
#[action_output(Vec2)]
pub(super) struct CameraInputRotate;

#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct FreeLookInput;

#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct CombatInput;

#[cfg(test)]
mod tests {
    use bevy::input::{mouse::MouseMotion, InputPlugin};

    use super::*;
    use crate::{camera::mode::on_rotation_input, input::bindings::camera_bindings};

    /// The look sensitivity drives the REAL camera rig, through the same
    /// `camera_rotate` action that normal steering, free look and turret aim
    /// all read - and it reaches a rig that already exists, which is what a
    /// slider moved from the pause overlay depends on.
    ///
    /// The right stick is bound to the same action with a gain of its own, and
    /// must not move when the setting does.
    #[test]
    fn the_look_sensitivity_scales_mouse_look_and_never_the_stick() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.add_plugins(MouseSensitivityPlugin);
        app.init_state::<nova_gameplay::PauseStates>();
        app.add_input_context::<PlayerInputMarker>();
        app.add_observer(on_rotation_input);

        let look = app
            .world_mut()
            .spawn((
                SpaceshipCameraInputMarker,
                SpaceshipRotationInputActiveMarker,
                PointRotationInput::default(),
            ))
            .id();

        app.finish();
        app.cleanup();
        app.update();
        app.world_mut()
            .spawn(player_input_rig(&InputBindings::from_actions(
                camera_bindings(),
            )));
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        app.update();

        let sweep = |app: &mut App| {
            app.world_mut().write_message(MouseMotion {
                delta: Vec2::new(30.0, 0.0),
            });
            app.update();
            app.world().get::<PointRotationInput>(look).unwrap().0.x
        };

        let at_default = sweep(&mut app);
        assert!(
            (at_default + 30.0 * MousePath::Look.default_raw()).abs() < 1e-6,
            "the rig starts on the look default (negated, got {at_default})"
        );

        // The slider goes to its top while the rig is already live.
        app.world_mut()
            .resource_mut::<MouseSensitivity>()
            .set_percent(MousePath::Look, 300.0);
        let at_top = sweep(&mut app);
        assert!(
            (at_top + 30.0 * MousePath::Look.range().raw(300.0)).abs() < 1e-6,
            "the top of the range reaches a rig that already existed \
             (got {at_top}, was {at_default})"
        );

        // The other two paths are not this one.
        let mut sensitivity = app.world_mut().resource_mut::<MouseSensitivity>();
        sensitivity.set_percent(MousePath::Rcs, 500.0);
        sensitivity.set_percent(MousePath::FreeCamera, 300.0);
        assert!(
            (sweep(&mut app) - at_top).abs() < 1e-9,
            "the RCS and free-camera sliders leave mouse look alone"
        );

        // The pad reads the same action through its own fixed gain. Full
        // right-stick deflection has to answer to none of the three settings.
        let mut gamepad = app.world_mut().get_mut::<Gamepad>(pad).unwrap();
        gamepad.analog_mut().set(GamepadAxis::RightStickX, 1.0);
        app.update();
        let stick_at_top = app.world().get::<PointRotationInput>(look).unwrap().0.x;

        app.world_mut()
            .resource_mut::<MouseSensitivity>()
            .set_percent(MousePath::Look, 100.0);
        app.update();
        assert!(
            (app.world().get::<PointRotationInput>(look).unwrap().0.x - stick_at_top).abs() < 1e-9,
            "the stick keeps its own gain whatever the mouse sliders say"
        );
    }
}
