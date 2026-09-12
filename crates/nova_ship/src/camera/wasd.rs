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
//! # use nova_ship::prelude::{WASDCamera, WASDCameraInput};
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
//!
//! MOVEMENT IS A SPEED, not a per-frame step: the rig reads the frame's own
//! delta, so the same held key crosses the same distance on a 30 Hz machine and
//! on a 240 Hz one. The clock is the VIRTUAL one, so every surface that stops
//! the world - the pause overlay, the CRT, a scenario load - stops the free-fly
//! camera with it.

use bevy::{camera::Projection, prelude::*};
use nova_events::units::prelude::*;

/// Glob-import surface for the WASD free-fly camera rig.
pub mod prelude {
    pub use super::{
        WASDCamera, WASDCameraInput, WASDCameraPlugin, WASDCameraProfile, WASDCameraSystems,
        WASD_ACCELERATION_MAX, WASD_ACCELERATION_SECS, WASD_BASE_SPEED, WASD_FOV_EASE_SECS,
        WASD_FOV_FEEDBACK,
    };
}

/// How fast a free-fly camera flies with the ramp at rest.
///
/// A walk-the-hull speed: 60 m/s crosses a 30-cell carrier in five seconds,
/// which is the register a builder inspects a plate in. Everything longer than
/// a hull is what the ramp below is for.
pub const WASD_BASE_SPEED: MetersPerSecond = MetersPerSecond(60.0);

/// How long a held translation takes to reach [`WASD_ACCELERATION_MAX`].
pub const WASD_ACCELERATION_SECS: f32 = 2.0;

/// The most the ramp multiplies [`WASD_BASE_SPEED`] by.
///
/// 32x is 1,920 m/s, which crosses the tutorial's 8 km range in about four
/// seconds - the other end of the same journey the base speed starts. QUADRATIC
/// in the held time rather than linear: the first half-second stays in the
/// inspection register a nudge is aimed at, and the distance register arrives
/// only once the key is genuinely held.
pub const WASD_ACCELERATION_MAX: f32 = 32.0;

/// How much wider the lens goes at full ramp, in degrees.
///
/// Feedback, not framing: the widening is what makes speed READ as speed, the
/// way a chase camera's does. Small enough that a builder framing a part never
/// sees the framing move under a nudge.
pub const WASD_FOV_FEEDBACK: f32 = 8.0;

/// How long the lens takes to ease back once the ramp lets go.
pub const WASD_FOV_EASE_SECS: f32 = 0.25;

/// The main WASD camera configuration component.
///
/// This component defines how sensitive the camera is to mouse movements
/// and how fast the rig flies. Insert this component on a camera entity
/// to enable WASD movement and mouse look.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[require(Transform)]
pub struct WASDCamera {
    /// Mouse look sensitivity.
    pub look_sensitivity: f32,

    /// How fast the rig flies with the ramp at rest.
    pub speed: MetersPerSecond,

    /// Whether a held translation ramps the speed up to
    /// [`WASD_ACCELERATION_MAX`]. Off is a constant-speed rig, which is what a
    /// measured walk and a screenshot example want.
    pub accelerate: bool,

    /// Whether the ramp widens the lens by up to [`WASD_FOV_FEEDBACK`]
    /// degrees. Independent of [`Self::accelerate`]: a camera may ramp without
    /// the lens moving, which is what a capture whose framing is the
    /// measurement needs.
    pub fov_feedback: bool,
}

impl Default for WASDCamera {
    fn default() -> Self {
        Self {
            look_sensitivity: 0.1,
            speed: WASD_BASE_SPEED,
            accelerate: true,
            fov_feedback: true,
        }
    }
}

/// The settings a camera keeps across losing and regaining its controller.
///
/// [`WASDCamera`] itself cannot carry them: the rig is REMOVED and re-inserted
/// every time something poses the camera by hand (the editor's framing, the
/// gallery's park, a scripted scenario pose), because its re-insert is what
/// makes the private target state re-read the transform. The profile survives
/// that, so a camera that was flown with the ramp off comes back with the ramp
/// still off.
///
/// Absent means "the stock profile": a camera that never asked for anything
/// gets [`WASDCamera::default`].
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect)]
pub struct WASDCameraProfile(pub WASDCamera);

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

impl WASDCameraInput {
    /// The translation the three axes ask for, in the camera's own frame, with
    /// no axis worth more than a full press.
    ///
    /// CLAMPED rather than normalized: a stick half over still means half
    /// speed, and only the corner case - two keys held, which asks for a
    /// diagonal of length `sqrt(2)` - is brought back to one.
    fn direction(self) -> Vec3 {
        Vec3::new(self.wasd.x, self.vertical, self.wasd.y).clamp_length_max(1.0)
    }
}

/// The ramp and the lens it drives, for one camera.
///
/// Held here rather than on [`WASDCamera`] because it is STATE, not a setting:
/// a profile survives the rig coming off and going back on, and the ramp must
/// not - a camera that was flying at 32x when the gallery parked it has to come
/// back at rest.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
struct WASDCameraDrive {
    /// How long translation has been held, seconds, capped at the ramp length.
    held: f32,
    /// The degrees currently added to the lens.
    fov_extra: f32,
    /// The lens the camera wore before the rig touched it, radians. `None`
    /// until the rig has widened it once, so a camera whose lens never moved is
    /// never written back to.
    base_fov: Option<f32>,
}

impl WASDCameraDrive {
    /// How far along the ramp a held translation is, 0 at rest and 1 at the
    /// top.
    fn ramp(self) -> f32 {
        (self.held / WASD_ACCELERATION_SECS).clamp(0.0, 1.0)
    }

    /// What the ramp multiplies the base speed by: quadratic in the held time,
    /// 1 at rest and [`WASD_ACCELERATION_MAX`] at the top.
    fn speed_factor(self) -> f32 {
        let ramp = self.ramp();
        1.0 + (WASD_ACCELERATION_MAX - 1.0) * ramp * ramp
    }

    /// The degrees the lens wants right now, from the same quadratic the speed
    /// reads - so the widening tracks the speed and not the key.
    fn fov_target(self) -> f32 {
        let ramp = self.ramp();
        WASD_FOV_FEEDBACK * ramp * ramp
    }
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
        trace!("WASDCameraPlugin: build");

        app.add_observer(initialize_wasd_camera);
        app.add_observer(destroy_wasd_camera);

        // PostUpdate, so every input system has already run this frame.
        app.add_systems(
            PostUpdate,
            (update_target, update_lens, update_state, sync_transform)
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
        WASDCameraDrive::default(),
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

/// Clean up camera components when the WASD camera is removed, and give the
/// lens back exactly as it was found.
///
/// The restore is the whole reason this observer reads the projection: a camera
/// whose rig was taken off mid-ramp would otherwise keep the widened lens for
/// as long as it lived, and every surface that poses the camera by hand takes
/// the rig off.
fn destroy_wasd_camera(
    remove: On<Remove, WASDCamera>,
    mut commands: Commands,
    q_drive: Query<&WASDCameraDrive>,
    mut q_projection: Query<&mut Projection>,
) {
    let entity = remove.entity;
    trace!("destroy_wasd_camera: entity {:?}", entity);

    if let Ok(drive) = q_drive.get(entity) {
        if let Ok(mut projection) = q_projection.get_mut(entity) {
            restore_fov(&mut projection, *drive);
        }
    }

    commands.entity(entity).try_remove::<(
        WASDCameraInput,
        WASDCameraDrive,
        WASDCameraTarget,
        WASDCameraState,
    )>();
}

/// Update the target state based on user input.
///
/// `Time` is the VIRTUAL clock: a camera that kept flying behind a pause
/// overlay, a CRT or a loading panel would move the world the player is about
/// to be handed back.
fn update_target(
    time: Res<Time>,
    mut q_camera: Query<(
        &WASDCamera,
        &WASDCameraInput,
        &mut WASDCameraDrive,
        &mut WASDCameraTarget,
    )>,
) {
    let delta = time.delta_secs();
    for (camera, input, mut drive, mut target) in q_camera.iter_mut() {
        target.yaw -= input.pan.x * camera.look_sensitivity;
        target.pitch -= input.pan.y * camera.look_sensitivity;

        let direction = input.direction();
        // The RAMP is on translation alone: turning the camera, or letting go
        // of one of two held keys, is the same gesture continuing. Releasing
        // everything is what says the journey is over.
        if direction == Vec3::ZERO {
            drive.held = 0.0;
        } else if camera.accelerate {
            drive.held = (drive.held + delta).min(WASD_ACCELERATION_SECS);
        }

        let rotation = Quat::from_euler(EulerRot::YXZ, target.yaw, target.pitch, 0.0);
        let forward = rotation * Vec3::NEG_Z;
        let right = Quat::from_rotation_y(target.yaw) * Vec3::X;

        // Engine boundary: the rig flies a Bevy `Transform`, which counts world
        // units, and the speed it is authored with is meters per second.
        let step = camera.speed.to_engine() * drive.speed_factor() * delta;
        target.position +=
            (forward * direction.z + right * direction.x + Vec3::Y * direction.y) * step;
    }
}

/// Widen the lens with the ramp and ease it back when the ramp lets go.
fn update_lens(
    time: Res<Time>,
    mut q_camera: Query<(&WASDCamera, &mut WASDCameraDrive, &mut Projection)>,
) {
    let delta = time.delta_secs();
    for (camera, mut drive, mut projection) in q_camera.iter_mut() {
        let target = if camera.fov_feedback {
            drive.fov_target()
        } else {
            0.0
        };
        if target >= drive.fov_extra {
            drive.fov_extra = target;
        } else {
            // A fixed RATE rather than a fraction per frame: the lens takes
            // the same quarter of a second to come home from the top however
            // the frame times fall.
            let ease = WASD_FOV_FEEDBACK / WASD_FOV_EASE_SECS * delta;
            drive.fov_extra = (drive.fov_extra - ease).max(target);
        }
        let Projection::Perspective(perspective) = projection.as_mut() else {
            continue;
        };
        // The base is taken the first time the rig moves the lens, so a camera
        // authored at a wider or narrower default keeps it.
        let base = *drive.base_fov.get_or_insert(perspective.fov);
        let wanted = base + drive.fov_extra.to_radians();
        if (perspective.fov - wanted).abs() > f32::EPSILON {
            perspective.fov = wanted;
        }
        if drive.fov_extra <= 0.0 {
            drive.base_fov = None;
        }
    }
}

/// Put the lens back where the rig found it.
fn restore_fov(projection: &mut Projection, drive: WASDCameraDrive) {
    let (Projection::Perspective(perspective), Some(base)) = (projection, drive.base_fov) else {
        return;
    };
    perspective.fov = base;
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

#[cfg(test)]
mod tests;
