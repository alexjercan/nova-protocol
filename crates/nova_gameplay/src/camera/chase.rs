//! A Bevy plugin that implements a smooth chase camera with configurable offset,
//! smoothing, and look-ahead behavior.
//!
//! ## Overview
//!
//! The `ChaseCameraPlugin` provides functionality for attaching a chase-style
//! third-person camera to any entity. The camera follows the target with a
//! configurable positional offset, smoothly interpolates toward its desired
//! location, and automatically rotates to look ahead of the target using a
//! separate focus offset.
//!
//! The system is split into three conceptual parts:
//!
//! 1. **`ChaseCamera`** - the main configuration component describing how the
//!    camera should behave (offset, look-ahead offset, smoothing).
//! 2. **`ChaseCameraInput`** - a driver component that external systems can
//!    update each frame to tell the camera *where the target is* and *which
//!    rotation to orbit around*.
//! 3. **`ChaseCameraState`** - internal state used for smooth interpolation.
//!
//! This separation allows your game logic or player controller to update
//! `ChaseCameraInput` freely without worrying about camera smoothing logic.
//!
//! ## Usage
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use nova_gameplay::prelude::*;
//! # fn setup(mut commands: Commands) {
//! commands.spawn((
//!     Camera3d::default(),
//!     ChaseCamera {
//!         offset: Vec3::new(0.0, 5.0, -20.0),
//!         focus_offset: Vec3::new(0.0, 0.0, 20.0),
//!         smoothing: 0.1,
//!     },
//! ));
//! # }
//!
//! // In your player movement system:
//! # fn player_movement(input: &mut ChaseCameraInput, player_position: Vec3, player_rotation: Quat) {
//! input.anchor_pos = player_position;
//! input.anchor_rot = player_rotation;
//! # }
//! ```
//!
//! After inserting `ChaseCamera`, the plugin automatically initializes required
//! components and ensures the camera updates after gameplay logic.

use bevy::prelude::*;

use crate::math::LerpSnap;

/// Glob-import surface for the chase camera rig.
pub mod prelude {
    pub use super::{ChaseCamera, ChaseCameraInput, ChaseCameraPlugin, ChaseCameraSystems};
}

/// A component describing how a chase camera should behave.
///
/// The `ChaseCamera` causes a camera to follow a target with a configurable
/// positional offset and smooth interpolation. It can also look ahead of the
/// target using a separate focus offset for improved visual clarity.
///
/// The final camera transform is computed by combining:
/// - `ChaseCameraInput.anchor_pos`  - where the target currently is
/// - `ChaseCameraInput.anchor_rot`  - the rotation the camera should orbit around
/// - `offset`                       - relative position behind or above the target
/// - `focus_offset`                 - point in front of the target to look toward
///
/// The motion is smoothed using a time-based lerp controlled by `smoothing`.
#[derive(Component, Debug, Reflect)]
#[require(Transform)]
pub struct ChaseCamera {
    /// The position offset relative to the target's anchor frame.
    /// Positive Z moves **behind** the target when using the typical -Z forward direction.
    pub offset: Vec3,

    /// A second offset used to determine the camera's look-at target.
    /// Positive Z looks **ahead** of the target.
    pub focus_offset: Vec3,

    /// Smoothing factor for camera movement.
    ///
    /// `0.0`  -> no smoothing (camera snaps to target)
    /// `1.0`  -> extremely smooth (slowly interpolates)
    pub smoothing: f32,
}

impl Default for ChaseCamera {
    fn default() -> Self {
        Self {
            offset: Vec3::new(0.0, 5.0, -20.0),
            focus_offset: Vec3::new(0.0, 0.0, 20.0),
            smoothing: 0.0,
        }
    }
}

/// Input component used to drive the chase camera.
///
/// This component defines the "anchor frame" that the camera uses each frame.
/// External systems (such as player controllers or AI controllers) should write
/// to this component to tell the camera **where the target is** and **how the
/// camera's offset frame should be oriented**.
#[derive(Component, Default, Debug, Reflect)]
pub struct ChaseCameraInput {
    /// The rotation representing the target's forward/up frame.
    /// The camera offset and look-at computations are performed in this space.
    pub anchor_rot: Quat,

    /// The target position the camera should follow.
    pub anchor_pos: Vec3,
}

/// Internal state used for smoothing camera motion.
///
/// Not intended to be modified manually.
#[derive(Component, Default, Debug, Reflect)]
struct ChaseCameraState {
    /// The smoothed anchor position used to compute the final camera transform.
    anchor_pos: Vec3,
}

/// The system set used by the chase camera plugin.
///
/// Only contains camera sync systems.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChaseCameraSystems {
    /// Systems that update camera smoothing state and apply transforms.
    Sync,
}

/// Plugin that manages chase camera components.
///
/// Automatically attaches required components, handles cleanup, and manages
/// system execution order so camera updates occur after gameplay input.
pub struct ChaseCameraPlugin;

impl Plugin for ChaseCameraPlugin {
    fn build(&self, app: &mut App) {
        debug!("ChaseCameraPlugin: build");

        app.add_observer(initialize_chase_camera);
        app.add_observer(destroy_chase_camera);

        // NOTE: PostUpdate, so camera motion is computed after game logic and input
        // systems -- in Update it would lag them by a frame and jitter.
        app.add_systems(
            PostUpdate,
            (
                chase_camera_update_state_system,
                chase_camera_sync_transform_system,
            )
                .chain()
                .in_set(ChaseCameraSystems::Sync),
        );
    }
}

/// Initializes the associated input and state for any newly-added `ChaseCamera`.
fn initialize_chase_camera(
    insert: On<Insert, ChaseCamera>,
    mut commands: Commands,
    q_state: Query<Has<ChaseCameraState>, With<ChaseCamera>>,
) {
    let entity = insert.entity;
    trace!("initialize_chase_camera: entity {:?}", entity);

    let Ok(has_state) = q_state.get(entity) else {
        error!(
            "initialize_chase_camera: entity {:?} not found in q_state",
            entity
        );
        return;
    };

    commands.entity(entity).insert(ChaseCameraInput::default());

    if !has_state {
        commands.entity(entity).insert(ChaseCameraState::default());
    }
}

/// Removes related internal components when the chase camera is removed.
fn destroy_chase_camera(remove: On<Remove, ChaseCamera>, mut commands: Commands) {
    let entity = remove.entity;
    trace!("destroy_chase_camera: entity {:?}", entity);

    // NOTE: `try_remove`, since the entity may already be despawned.
    commands
        .entity(entity)
        .try_remove::<(ChaseCameraInput, ChaseCameraState)>();
}

/// Updates the smoothed camera state based on target movement.
fn chase_camera_update_state_system(
    time: Res<Time>,
    mut q_camera: Query<
        (&ChaseCamera, &ChaseCameraInput, &mut ChaseCameraState),
        With<ChaseCamera>,
    >,
) {
    let dt = time.delta_secs();

    for (chase, input, mut state) in q_camera.iter_mut() {
        let target_pos = input.anchor_pos;

        let desired_pos = target_pos
            + input.anchor_rot * Vec3::NEG_Z * chase.offset.z
            + input.anchor_rot * Vec3::Y * chase.offset.y
            + input.anchor_rot * Vec3::X * chase.offset.x;

        state.anchor_pos = state
            .anchor_pos
            .lerp_and_snap(desired_pos, chase.smoothing, dt);
    }
}

/// Syncs the actual camera transform with the smoothed state and look-at point.
fn chase_camera_sync_transform_system(
    mut q_camera: Query<
        (
            &ChaseCamera,
            &ChaseCameraInput,
            &ChaseCameraState,
            &mut Transform,
        ),
        With<ChaseCamera>,
    >,
) {
    for (chase, input, state, mut transform) in q_camera.iter_mut() {
        transform.translation = state.anchor_pos;

        let focus = input.anchor_pos
            + input.anchor_rot * Vec3::NEG_Z * chase.focus_offset.z
            + input.anchor_rot * Vec3::Y * chase.focus_offset.y
            + input.anchor_rot * Vec3::X * chase.focus_offset.x;

        transform.look_at(focus, input.anchor_rot * Vec3::Y);
    }
}
