//! Shared building blocks for destructible game objects.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;

use crate::prelude::ExplodableEntity;

pub mod prelude {
    pub use super::{destructible_body, rigid_body_point_velocity};
}

/// The world-space velocity of a point rigidly attached to a moving body.
///
/// This is the standard rigid-body relation `v_point = v_linear + omega x (p - com)`: a point
/// offset from the center of mass of a spinning body moves faster than the body's linear
/// velocity alone, by the tangential contribution of its rotation.
///
/// `center_of_mass` and `point` must be in the same (world) frame. avian's
/// `ComputedCenterOfMass` is body-*local*, so convert it with the body's global transform
/// before calling: `ship_transform.transform_point(*center_of_mass)`.
///
/// Used for projectile muzzle velocity so a shot inherits the full motion of its muzzle -
/// including the swing from the ship's rotation - not just the ship's linear velocity.
pub fn rigid_body_point_velocity(
    linear_velocity: Vec3,
    angular_velocity: Vec3,
    center_of_mass: Vec3,
    point: Vec3,
) -> Vec3 {
    linear_velocity + angular_velocity.cross(point - center_of_mass)
}

/// The common makeup of a destructible game object: a health pool, physics density, and
/// the ability to explode into fragments when destroyed.
///
/// Pair it with a [`Collider`] (whose shape varies per object) on an entity that is
/// parented to a [`RigidBody`]. This captures the boilerplate shared by ship sections and
/// scenario objects such as asteroids, so each spawn site only spells out what is
/// actually different (its name/markers and its collider shape).
pub fn destructible_body(health: f32, density: f32) -> impl Bundle {
    (
        Health::new(health),
        ColliderDensity(density),
        ExplodableEntity,
        Visibility::Inherited,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_on_a_purely_translating_body_moves_with_the_body() {
        // No rotation: every point moves at the body's linear velocity, regardless of offset.
        let v = rigid_body_point_velocity(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::ZERO,
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
        );
        assert_eq!(v, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn a_point_at_the_center_of_mass_ignores_rotation() {
        // omega x 0 = 0, so a muzzle exactly on the COM inherits only the linear velocity.
        let v = rigid_body_point_velocity(
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(0.0, 7.0, 0.0),
            Vec3::new(2.0, 2.0, 2.0),
            Vec3::new(2.0, 2.0, 2.0),
        );
        assert_eq!(v, Vec3::new(5.0, 0.0, 0.0));
    }

    #[test]
    fn rotation_adds_tangential_velocity_at_an_offset() {
        // Spin about +Y at 2 rad/s; a point 3 units along +X of a stationary COM swings along
        // -Z: omega x r = (0,2,0) x (3,0,0) = (2*0-0*0, 0*3-0*0, 0*0-2*3) = (0,0,-6).
        let v = rigid_body_point_velocity(
            Vec3::ZERO,
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::ZERO,
            Vec3::new(3.0, 0.0, 0.0),
        );
        assert_eq!(v, Vec3::new(0.0, 0.0, -6.0));
    }

    #[test]
    fn linear_and_rotational_contributions_add() {
        let v = rigid_body_point_velocity(
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::ZERO,
            Vec3::new(3.0, 0.0, 0.0),
        );
        assert_eq!(v, Vec3::new(1.0, 0.0, -6.0));
    }
}
