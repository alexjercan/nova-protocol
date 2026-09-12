//! The graph a destructible structure is described by: nodes that know their
//! structural neighbours, all under one body.
//!
//! A structure is a set of node entities, each carrying [`ConnectedTo`], all
//! descending from a single body marked [`IntegrityRoot`]. [`super::core`] drives
//! the lifecycle over them: health depletion inserts [`IntegrityDisabledMarker`],
//! a disabled leaf (or a disabled root) inserts [`IntegrityDestroyMarker`], and
//! destroying a node prunes it from its neighbours, which can turn them into
//! leaves and cascade.
//!
//! The graph is node-local (each node owns its own neighbour list) rather than a
//! central adjacency map on the root, so the section grid, an asteroid's lone
//! collider, and a hand-built test rig all describe themselves the same way
//! without this module knowing the layout.

use bevy::prelude::*;

/// The structural graph components: `IntegrityRoot`, `IntegrityEnvelope`,
/// `ConnectedTo` and the leaf, disabled and destroy markers.
pub mod prelude {
    pub use super::{
        ConnectedTo, IntegrityDestroyMarker, IntegrityDisabledMarker, IntegrityEnvelope,
        IntegrityLeafMarker, IntegrityRoot,
    };
}

/// Marks the body that owns an integrity structure - a ship root, or a lone body
/// such as an asteroid. Its integrity nodes are the descendants carrying
/// [`ConnectedTo`].
///
/// The root only needs to be identifiable so the pipeline can find it for
/// whole-body concerns: a disabled root destroys the entire structure, leaf or
/// not. It goes on the `RigidBody` entity whose colliders are the nodes.
#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct IntegrityRoot;

/// How far an [`IntegrityRoot`]'s own structure reaches from its centre of
/// mass, world units.
///
/// Published by the layer that OWNS the body, because only that layer knows
/// what the body is made of: a ship writes its live hull envelope, a rock its
/// radius. This layer never derives it - a pass over a capital's sections for
/// every section that dies in its collapse is quadratic in the worst frame
/// there is.
///
/// The destruction layer is the reader. A piece is born INSIDE the structure it
/// came off, so how deep it is standing decides how hard it has to be thrown
/// and how long it stays a ghost ([`explode`](super::explode)). A body that
/// publishes nothing is read as reaching no further than the piece itself,
/// which is what a lone collider or a drifting wreck actually is.
///
/// Engine units: it is compared against avian positions and spent on avian
/// velocities.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct IntegrityEnvelope(pub f32);

/// The integrity neighbours of a node: the adjacent nodes it is structurally
/// connected to. Lives on the node itself (a section collider, a structural
/// cell), not in a central graph.
///
/// A node with one or zero neighbours is a leaf ([`IntegrityLeafMarker`]).
/// Removing a node prunes it from its neighbours' lists, which can turn them
/// into leaves and drive the chain reaction. A lone node gets an empty list, so
/// it is a leaf and is destroyed as soon as it is disabled.
#[derive(Component, Debug, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct ConnectedTo(pub Vec<Entity>);

/// Marks a leaf node in the integrity graph (one or zero neighbours).
///
/// Derived from [`ConnectedTo`] by `derive_integrity_leaves`. A node that is
/// disabled and BECOMES a leaf is destroyed - that is the chain reaction.
#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct IntegrityLeafMarker;

/// Marks a node disabled by reaching zero health.
///
/// Inserted when a node gains [`HealthZeroMarker`](super::health::HealthZeroMarker).
/// The generic core destroys a disabled leaf. Structure adapters can destroy
/// disabled interior nodes immediately; ships do this for direct depletion.
#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct IntegrityDisabledMarker;

/// Marks a node the frame it is destroyed - the public seam of the pipeline.
///
/// The core only inserts this marker and prunes the graph; it never decides what
/// "destroyed" looks like. Nova's reaction lives in
/// [`explode`](super::explode): slice the mesh, spawn debris, fire
/// `OnDestroyedEvent`.
#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct IntegrityDestroyMarker;
