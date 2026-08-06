//! Rigs that drive an entity's [`Transform`](bevy::prelude::Transform) as an
//! *output*: you write an `*Input` or `*Target` component each frame, the rig
//! smooths it, and the resulting pose lands on the entity plus a mirrored
//! `*Output` component gameplay can read. [`point_rotation`] turns mouse deltas
//! into an orientation, [`smooth_look_rotation`] eases toward a target angle
//! around an axis, and the three sphere orbits place an entity on a sphere's
//! surface from spherical coordinates.
//!
//! Nova owns these because the ship, the turrets and the camera all steer
//! through them, so their smoothing constants and system ordering are gameplay
//! decisions, not engine ones. They share [`crate::math`]'s spherical
//! conversions with [`crate::camera`].

pub mod directional_sphere_orbit;
pub mod point_rotation;
pub mod random_sphere_orbit;
pub mod smooth_look_rotation;
pub mod sphere_orbit;

/// Glob-import surface: `use nova_gameplay::transform::prelude::*` re-exports
/// the public API of all five rigs.
pub mod prelude {
    pub use super::{
        directional_sphere_orbit::prelude::*, point_rotation::prelude::*,
        random_sphere_orbit::prelude::*, smooth_look_rotation::prelude::*,
        sphere_orbit::prelude::*,
    };
}
