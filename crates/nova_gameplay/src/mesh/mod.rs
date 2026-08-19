//! Procedural mesh construction: [`builder`] grows a triangle soup
//! (octahedron, cone, noise displacement) into a `Mesh`, and [`field`] holds a
//! solid as a signed grid that can be carved and remeshed.
//!
//! Nothing here takes a finished mesh APART. A body that dies detaches whole
//! and keeps the art it had (`crate::integrity::explode`); the only thing that
//! ever cuts geometry is a carve, and a carve works on a field.
//!
//! A grid is only ever ANALYTIC now - an asteroid's own noise. Reading one out
//! of authored glTF art was how ship sections carved, and that came out costing
//! more than it drew: see the epic's Phase 4e.
//!
//! Nova owns these because the asteroids, the thruster exhausts and the velocity
//! indicator are all built here - their subdivision depths are art decisions,
//! not engine ones. The builder shares [`crate::math`]'s `slerp` with
//! `nova_ship`'s camera rigs and [`crate::transform`].

pub mod builder;
pub mod field;

/// The `TriangleMeshBuilder` and the `SignedField`.
pub mod prelude {
    pub use super::{builder::TriangleMeshBuilder, field::SignedField};
}
