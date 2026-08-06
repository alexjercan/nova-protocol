//! A simple WASD and mouse look camera implementation for Bevy.
//!
//! This plugin provides a first person style camera controller that supports:
//! - WASD movement
//! - Vertical movement (for example, space and shift keys)
//! - Mouse look for yaw and pitch
//!
//! The camera logic is split into three parts:
//! 1. `WASDCamera` stores configuration like sensitivity values.
//! 2. `WASDCameraInput` stores user input for the current frame. Your input
//!    systems should write to this component.
//! 3. Internal target and state components track camera motion and apply it
//!    to the Transform each frame.
//!
//! To use the WASD camera:
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use nova_gameplay::prelude::*;
//! # fn setup(mut commands: Commands) {
//! commands.spawn((
//!     Camera3d::default(),
//!     WASDCamera::default(),
//! ));
//! # }
//!
//! // In your input system:
//! # fn input_system(input: &mut WASDCameraInput, mouse_delta: Vec2, movement_axis: Vec2, vertical_axis: f32) {
//! input.pan = mouse_delta;
//! input.wasd = movement_axis;
//! input.vertical = vertical_axis;
//! # }
//! ```
//!
//! The plugin will handle smoothing and transform updates automatically.

use bevy::prelude::*;

/// Glob-import surface for the WASD free-fly camera rig.
pub mod prelude {
    pub use super::{WASDCamera, WASDCameraInput, WASDCameraPlugin, WASDCameraSystems};
}

/// The main WASD camera configuration component.
///
/// This component defines how sensitive the camera is to mouse movements
/// and keyboard movement inputs. Insert this component on a camera entity
/// to enable WASD movement and mouse look.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[require(Transform)]
pub struct WASDCamera {
    /// Mouse look sensitivity.
    pub look_sensitivity: f32,

    /// Movement sensitivity for WASD and vertical movement.
    pub wasd_sensitivity: f32,
}

impl Default for WASDCamera {
    fn default() -> Self {
        Self {
            look_sensitivity: 0.1,
            wasd_sensitivity: 0.5,
        }
    }
}

/// Input component for the WASD camera.
///
/// Your input system should update these values every frame.
/// The plugin reads this component to determine how to move
/// and rotate the camera.
///
/// - `pan` contains the mouse delta for yaw and pitch.
/// - `wasd` contains horizontal and forward movement values.
/// - `vertical` is upward or downward movement.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct WASDCameraInput {
    /// Mouse delta driving yaw and pitch.
    pub pan: Vec2,
    /// Horizontal and forward movement.
    pub wasd: Vec2,
    /// Upward or downward movement.
    pub vertical: f32,
}

/// Internal target state. This is where the camera *wants* to be.
///
/// The target is updated based on user input.
#[derive(Component, Clone, Copy, Debug, Reflect)]
struct WASDCameraTarget {
    position: Vec3,
    yaw: f32,
    pitch: f32,
}

/// Internal smoothed state used to update the Transform.
///
/// This mirrors the target but allows for interpolation if needed.
#[derive(Component, Clone, Copy, Debug, Reflect)]
struct WASDCameraState {
    position: Vec3,
    yaw: f32,
    pitch: f32,
}

/// System set for the WASD camera plugin.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum WASDCameraSystems {
    /// Writes the camera `Transform` from the smoothed target state.
    Sync,
}

/// Plugin that manages the WASD camera components and systems.
///
/// This plugin initializes camera state when a `WASDCamera` is added,
/// updates the target and internal state based on input, and applies
/// the resulting transform each frame.
pub struct WASDCameraPlugin;

impl Plugin for WASDCameraPlugin {
    fn build(&self, app: &mut App) {
        debug!("WASDCameraPlugin: build");

        app.add_observer(initialize_wasd_camera);
        app.add_observer(destroy_wasd_camera);

        // NOTE: PostUpdate, so every input system has already run this frame.
        app.add_systems(
            PostUpdate,
            (update_target, update_state, sync_transform)
                .chain()
                .in_set(WASDCameraSystems::Sync)
                .before(TransformSystems::Propagate),
        );
    }
}

/// Initialize the WASD camera input, target, and state components.
fn initialize_wasd_camera(
    insert: On<Insert, WASDCamera>,
    mut commands: Commands,
    q_transform: Query<&Transform, With<WASDCamera>>,
) {
    let entity = insert.entity;
    trace!("initialize_wasd_camera: entity {:?}", entity);

    let Ok(transform) = q_transform.get(entity) else {
        error!(
            "initialize_wasd_camera: entity {:?} not found in q_transform",
            entity
        );
        return;
    };

    let translation = transform.translation;
    let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);

    commands.entity(entity).insert((
        WASDCameraInput::default(),
        WASDCameraTarget {
            position: translation,
            yaw,
            pitch,
        },
        WASDCameraState {
            position: translation,
            yaw,
            pitch,
        },
    ));
}

/// Clean up camera components when the WASD camera is removed.
fn destroy_wasd_camera(remove: On<Remove, WASDCamera>, mut commands: Commands) {
    let entity = remove.entity;
    trace!("destroy_wasd_camera: entity {:?}", entity);

    commands
        .entity(entity)
        .try_remove::<(WASDCameraInput, WASDCameraTarget, WASDCameraState)>();
}

/// Update the target state based on user input.
fn update_target(mut q_camera: Query<(&WASDCamera, &WASDCameraInput, &mut WASDCameraTarget)>) {
    for (camera, input, mut target) in q_camera.iter_mut() {
        target.yaw -= input.pan.x * camera.look_sensitivity;
        target.pitch -= input.pan.y * camera.look_sensitivity;

        let rotation = Quat::from_euler(EulerRot::YXZ, target.yaw, target.pitch, 0.0);

        let forward = rotation * Vec3::NEG_Z;
        let right = Quat::from_rotation_y(target.yaw) * Vec3::X;

        target.position += forward * input.wasd.y * camera.wasd_sensitivity
            + right * input.wasd.x * camera.wasd_sensitivity;

        target.position += Vec3::Y * input.vertical * camera.wasd_sensitivity;
    }
}

/// Copy the target values into the state.
/// This allows for smoothing in the future if needed.
fn update_state(mut q_camera: Query<(&mut WASDCameraState, &WASDCameraTarget)>) {
    for (mut state, target) in q_camera.iter_mut() {
        state.position = target.position;
        state.yaw = target.yaw;
        state.pitch = target.pitch;
    }
}

/// Apply the current state to the camera transform.
fn sync_transform(
    mut q_camera: Query<(&mut Transform, &WASDCameraState), Changed<WASDCameraState>>,
) {
    for (mut transform, state) in q_camera.iter_mut() {
        let rotation = Quat::from_euler(EulerRot::YXZ, state.yaw, state.pitch, 0.0);
        *transform = Transform {
            translation: state.position,
            rotation,
            ..Default::default()
        };
    }
}
