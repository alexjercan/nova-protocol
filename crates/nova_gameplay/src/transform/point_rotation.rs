//! A Bevy plugin that enables point rotation based on input deltas.
//!
//! PointRotation allows an entity to be rotated based on input deltas, typically from mouse
//! movement. The rotation is applied around the entity's local axes, allowing for intuitive
//! control of the entity's orientation.

use bevy::prelude::*;

/// Glob-import surface for the point-rotation rig.
pub mod prelude {
    pub use super::{
        PointRollInput, PointRotation, PointRotationInput, PointRotationOutput, PointRotationPlugin,
    };
}

/// Component that marks an entity as a point that can be rotated.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct PointRotation {
    /// Initial rotation of the point - used to compute the initial facing direction.
    ///
    /// The default, `Quat::IDENTITY`, means the point faces down the -Z axis with
    /// up on +Y and right on +X.
    pub initial_rotation: Quat,
}

impl Default for PointRotation {
    fn default() -> Self {
        Self {
            initial_rotation: Quat::IDENTITY,
        }
    }
}

/// The delta by how much to rotate the point
#[derive(Component, Default, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
pub struct PointRotationInput(pub Vec2);

/// The roll delta for this frame, in radians about the point's forward axis.
///
/// Positive turns the point clockwise as seen looking down its forward axis:
/// the right side drops. A sibling of [`PointRotationInput`] so the look
/// writers and the roll writer never share one component.
///
/// The integrator consumes it: it applies the delta once and sets it to zero.
/// A frame on which the writer does not run (a scenario load gates the camera
/// set, but not this integrator) therefore rolls nothing.
#[derive(Component, Default, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
pub struct PointRollInput(pub f32);

/// The target rotation state of the point, which is computed from the State and the Input.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect)]
pub struct PointRotationOutput(pub Quat);

/// A plugin that will enable the PointRotation system.
///
/// PointRotation allows an entity to be rotated based on input deltas, typically from mouse
/// movement. The rotation is applied around the entity's local axes, allowing for intuitive
/// control of the entity's orientation.
pub struct PointRotationPlugin;

impl Plugin for PointRotationPlugin {
    fn build(&self, app: &mut App) {
        trace!("PointRotationPlugin: build");

        app.add_observer(initialize_point_rotation_system);

        app.add_systems(PostUpdate, point_rotation_update_system);
    }
}

fn initialize_point_rotation_system(
    insert: On<Insert, PointRotation>,
    mut commands: Commands,
    q_point: Query<&PointRotation, With<PointRotation>>,
) {
    let entity = insert.entity;
    trace!("initialize_point_rotation_system: entity {:?}", entity);

    let Ok(point) = q_point.get(entity) else {
        error!(
            "initialize_point_rotation_system: entity {:?} not found in q_point",
            entity
        );
        return;
    };

    commands
        .entity(entity)
        .insert(PointRotationInput(Vec2::ZERO))
        .insert(PointRollInput(0.0))
        .insert(PointRotationOutput(point.initial_rotation));
}

fn point_rotation_update_system(
    mut q_point: Query<
        (
            &PointRotationInput,
            &mut PointRollInput,
            &mut PointRotationOutput,
        ),
        With<PointRotation>,
    >,
) {
    for (input, mut roll, mut out) in &mut q_point {
        let mut state = PointRotationState::from(**out);
        state = compute_point_rotation(&state, input, **roll);
        **out = point_rotation_quat(state);
        if **roll != 0.0 {
            **roll = 0.0;
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PointRotationState {
    forward: Vec3,
    right: Vec3,
}

impl From<Quat> for PointRotationState {
    fn from(rotation: Quat) -> Self {
        let forward = rotation * Vec3::NEG_Z;
        let right = rotation * Vec3::X;
        Self { forward, right }
    }
}

fn compute_point_rotation(
    state: &PointRotationState,
    input: &PointRotationInput,
    roll: f32,
) -> PointRotationState {
    let mut new_state = *state;

    let delta_x = input.x;
    let delta_y = input.y;

    if delta_x != 0.0 {
        let up = new_state.right.cross(new_state.forward).normalize();
        let yaw_quat = Quat::from_axis_angle(up, delta_x);
        new_state.forward = (yaw_quat * new_state.forward).normalize();
        new_state.right = (yaw_quat * new_state.right).normalize();
    }

    if delta_y != 0.0 {
        let right = new_state.right.normalize();
        let pitch_quat = Quat::from_axis_angle(right, delta_y);
        new_state.forward = (pitch_quat * new_state.forward).normalize();
        new_state.right = (pitch_quat * new_state.right).normalize();
    }

    if roll != 0.0 {
        let roll_quat = Quat::from_axis_angle(new_state.forward, roll);
        // Project out any forward component left by the normalizations above:
        // a skewed basis makes `from_mat3` return garbage.
        let right = (roll_quat * new_state.right).reject_from_normalized(new_state.forward);
        new_state.right = right.normalize();
    }

    new_state
}

fn point_rotation_quat(state: PointRotationState) -> Quat {
    let forward = state.forward.normalize();
    let right = state.right.normalize();
    let up = state.right.cross(state.forward).normalize();

    // The local basis (right, up, -forward) must stay right-handed -- `from_mat3` on an
    // improper (det -1) basis returns garbage.
    let mat3 = Mat3::from_cols(right, up, -forward);
    Quat::from_mat3(&mat3)
}

#[cfg(test)]
mod test {
    use super::*;

    /// Yawing 90 degrees to the right from the -Z default.
    #[test]
    fn test_compute_point_rotation_1() {
        let initial_state = PointRotationState {
            forward: Vec3::NEG_Z,
            right: Vec3::X,
        };

        let input = PointRotationInput(Vec2::new(-std::f32::consts::FRAC_PI_2, 0.0));
        let new_state = compute_point_rotation(&initial_state, &input, 0.0);
        assert!(new_state.forward.abs_diff_eq(Vec3::X, 1e-6));
        assert!(new_state.right.abs_diff_eq(Vec3::Z, 1e-6));
    }

    /// Pitching 90 degrees up from the -Z default; right is unchanged.
    #[test]
    fn test_compute_point_rotation_2() {
        let initial_state = PointRotationState {
            forward: Vec3::NEG_Z,
            right: Vec3::X,
        };

        let input = PointRotationInput(Vec2::new(0.0, std::f32::consts::FRAC_PI_2));
        let new_state = compute_point_rotation(&initial_state, &input, 0.0);
        assert!(new_state.forward.abs_diff_eq(Vec3::Y, 1e-6));
        assert!(new_state.right.abs_diff_eq(Vec3::X, 1e-6));
    }

    /// Combined 45 degrees right and 45 degrees up: yaw and pitch compose.
    #[test]
    fn test_compute_point_rotation_3() {
        let initial_state = PointRotationState {
            forward: Vec3::NEG_Z,
            right: Vec3::X,
        };

        let input = PointRotationInput(Vec2::new(
            -std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_4,
        ));
        let new_state = compute_point_rotation(&initial_state, &input, 0.0);
        let expected_forward = Vec3::new(0.5, 0.70710677, -0.5).normalize();
        let expected_right = Vec3::new(0.70710677, 0.0, 0.70710677).normalize();
        assert!(new_state.forward.abs_diff_eq(expected_forward, 1e-6));
        assert!(new_state.right.abs_diff_eq(expected_right, 1e-6));
    }

    /// Yawing 90 degrees to the left: the mirror of test 1.
    #[test]
    fn test_compute_point_rotation_4() {
        let initial_state = PointRotationState {
            forward: Vec3::NEG_Z,
            right: Vec3::X,
        };

        let input = PointRotationInput(Vec2::new(std::f32::consts::FRAC_PI_2, 0.0));
        let new_state = compute_point_rotation(&initial_state, &input, 0.0);
        assert!(new_state.forward.abs_diff_eq(Vec3::NEG_X, 1e-6));
        assert!(new_state.right.abs_diff_eq(Vec3::NEG_Z, 1e-6));
    }

    /// Pitching 90 degrees down: the mirror of test 2.
    #[test]
    fn test_compute_point_rotation_5() {
        let initial_state = PointRotationState {
            forward: Vec3::NEG_Z,
            right: Vec3::X,
        };

        let input = PointRotationInput(Vec2::new(0.0, -std::f32::consts::FRAC_PI_2));
        let new_state = compute_point_rotation(&initial_state, &input, 0.0);
        assert!(new_state.forward.abs_diff_eq(Vec3::NEG_Y, 1e-6));
        assert!(new_state.right.abs_diff_eq(Vec3::X, 1e-6));
    }
    /// A positive roll turns the point about its own forward axis: the
    /// forward ray does not move, the right side drops, and the output quat
    /// carries exactly that basis.
    #[test]
    fn a_positive_roll_keeps_forward_and_drops_the_right_side() {
        let yawed = Quat::from_rotation_y(0.7) * Quat::from_rotation_x(0.3);
        let state = PointRotationState::from(yawed);
        let angle = 0.4;

        let rolled = compute_point_rotation(&state, &PointRotationInput::default(), angle);
        assert!(rolled.forward.abs_diff_eq(state.forward, 1e-6));
        let up = state.right.cross(state.forward).normalize();
        let expected_right = state.right * angle.cos() - up * angle.sin();
        assert!(
            rolled.right.abs_diff_eq(expected_right, 1e-5),
            "right {:?}, expected {expected_right:?}",
            rolled.right
        );

        let output = point_rotation_quat(rolled);
        assert!((output * Vec3::NEG_Z).abs_diff_eq(state.forward, 1e-5));
        assert!((output * Vec3::X).abs_diff_eq(expected_right, 1e-5));
        assert!(
            output.angle_between(Quat::from_axis_angle(state.forward, angle) * yawed) < 1e-4,
            "the roll is a rotation about the forward axis"
        );

        let back = compute_point_rotation(&rolled, &PointRotationInput::default(), -angle);
        assert!(
            back.right.abs_diff_eq(state.right, 1e-5),
            "the opposite roll undoes it"
        );
    }
    /// One written roll delta turns the point once. The integrator consumes
    /// it, so the frames after it - where a gated or paused writer did not
    /// run - roll nothing more.
    #[test]
    fn a_roll_delta_applies_once_and_does_not_repeat() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(PointRotationPlugin);
        let point = app.world_mut().spawn(PointRotation::default()).id();
        app.update();

        let angle = 0.25;
        app.world_mut().get_mut::<PointRollInput>(point).unwrap().0 = angle;
        app.update();
        let rolled = **app.world().get::<PointRotationOutput>(point).unwrap();
        assert!(
            rolled.angle_between(Quat::from_axis_angle(Vec3::NEG_Z, angle)) < 1e-5,
            "the written delta rolls the point once, got {rolled:?}"
        );
        assert_eq!(**app.world().get::<PointRollInput>(point).unwrap(), 0.0);

        for _ in 0..10 {
            app.update();
        }
        let held = **app.world().get::<PointRotationOutput>(point).unwrap();
        assert!(
            held.angle_between(rolled) < 1e-6,
            "frames without a write roll nothing, drifted {}",
            held.angle_between(rolled)
        );
    }
}
