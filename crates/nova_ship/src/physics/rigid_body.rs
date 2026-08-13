//! The point-velocity relation for a rigidly attached point on a moving body.

use bevy::prelude::*;

/// The world-space velocity of a point rigidly attached to a moving body.
///
/// This is the standard rigid-body relation `v_point = v_linear + omega x (p - com)`: a point
/// offset from the center of mass of a spinning body moves faster than the body's linear
/// velocity alone, by the tangential contribution of its rotation.
///
/// `center_of_mass` and `point` must be in the same (world) frame. avian's
/// `ComputedCenterOfMass` is body-*local*, so convert it with the body's global transform
/// before calling: `body_transform.transform_point(*center_of_mass)`.
///
/// The canonical use is projectile muzzle velocity: a shot inherits the full motion of its
/// muzzle - including the swing from the body's rotation - not just the body's linear
/// velocity.
///
/// ```
/// # use bevy::prelude::*;
/// # use nova_ship::prelude::rigid_body_point_velocity;
/// // A point 3 units along +X of a body spinning at 2 rad/s about +Y (stationary centre)
/// // swings along -Z at 6 units/s.
/// let v = rigid_body_point_velocity(Vec3::ZERO, Vec3::Y * 2.0, Vec3::ZERO, Vec3::X * 3.0);
/// assert!((v - Vec3::new(0.0, 0.0, -6.0)).length() < 1e-5);
/// ```
pub fn rigid_body_point_velocity(
    linear_velocity: Vec3,
    angular_velocity: Vec3,
    center_of_mass: Vec3,
    point: Vec3,
) -> Vec3 {
    linear_velocity + angular_velocity.cross(point - center_of_mass)
}
