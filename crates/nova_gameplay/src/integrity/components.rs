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

/// Glob-import surface: `use nova_gameplay::integrity::components::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{
        ConnectedTo, IntegrityDestroyMarker, IntegrityDisabledMarker, IntegrityLeafMarker,
        IntegrityRoot,
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
/// A disabled INTERIOR node is merely deactivated (a dead section still holds
/// the ship together); a disabled LEAF is destroyed.
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
