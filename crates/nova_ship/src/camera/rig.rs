//! The camera-controller entity: the rigs it spawns (normal, free-look,
//! turret), the markers that tag them, the player input actions bound onto
//! it, and the live look ray gameplay reads.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_gameplay::prelude::*;

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
        parent.spawn((
            Name::new("Player Input Controller"),
            PlayerInputMarker,
            actions!(
                PlayerInputMarker[
                    (
                        Name::new("Input: Camera Rotate"),
                        Action::<CameraInputRotate>::new(),
                        Bindings::spawn((
                            // Bevy requires single entities to be wrapped in
                            // `Spawn`. You can attach modifiers to individual
                            // bindings as well.
                            Spawn((Binding::mouse_motion(), Scale::splat(0.001), Negate::all())),
                            Axial::right_stick().with((Scale::splat(2.0), Negate::none())),
                        )),
                    ),
                    (
                        Name::new("Input: Free Look Mode"),
                        Action::<FreeLookInput>::new(),
                        bindings![KeyCode::AltLeft, GamepadButton::LeftTrigger],
                    ),
                    (
                        Name::new("Input: Combat Mode"),
                        Action::<CombatInput>::new(),
                        bindings![MouseButton::Right, GamepadButton::LeftTrigger2],
                    ),
                ]
            ),
        ));
    });
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
