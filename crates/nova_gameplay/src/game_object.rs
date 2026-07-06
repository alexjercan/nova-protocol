//! Shared building blocks for destructible game objects.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;

use crate::prelude::ExplodableEntity;

pub mod prelude {
    pub use super::destructible_body;
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
