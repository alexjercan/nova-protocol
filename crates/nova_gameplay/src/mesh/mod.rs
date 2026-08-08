//! Procedural mesh construction and destruction: [`builder`] grows a triangle
//! soup (octahedron, cone, noise displacement, plane slicing) into a `Mesh`,
//! [`explode`] cuts a finished mesh into debris fragments with an outward
//! direction each, and the private `slice` module is the triangle-vs-plane
//! kernel both of them run on.
//!
//! Nova owns these because the asteroids, the thruster exhausts, the velocity
//! indicator and the destruction debris are all built here - their subdivision
//! depths and fragment counts are art decisions, not engine ones. The builder
//! shares [`crate::math`]'s `slerp` with `nova_ship`'s camera rigs and
//! [`crate::transform`].

pub mod builder;
pub mod explode;
mod slice;

/// The `TriangleMeshBuilder` and the explode submodule's prelude.
pub mod prelude {
    pub use super::{builder::TriangleMeshBuilder, explode::prelude::*};
}
