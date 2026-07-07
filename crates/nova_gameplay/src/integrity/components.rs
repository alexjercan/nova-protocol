use bevy::prelude::*;

pub mod prelude {
    pub use super::{
        ConnectedTo, IntegrityDestroyMarker, IntegrityDisabledMarker, IntegrityLeafMarker,
        IntegrityRoot,
    };
}

/// Marks the body that owns an integrity structure (a ship root, or a lone body like an
/// asteroid). Its integrity nodes are its descendants that carry [`ConnectedTo`].
///
/// This replaces the old central `IntegrityGraph(HashMap<Entity, Vec<Entity>>)` component:
/// instead of one map on the root describing every node's neighbors, each node now stores
/// its own neighbor list in [`ConnectedTo`], and the root just needs to be identifiable
/// (for aggregate health and whole-body destruction).
#[derive(Component, Debug, Default, Reflect)]
pub struct IntegrityRoot;

/// The integrity neighbors of a node, i.e. the adjacent nodes it is structurally connected
/// to. Lives on the node itself (a ship section, or an asteroid's collider node) rather than
/// in a central graph.
///
/// A node with one or zero neighbors is a leaf ([`IntegrityLeafMarker`]); removing a node
/// prunes it from its neighbors' lists, which can turn them into leaves and drive the
/// destruction chain reaction.
#[derive(Component, Debug, Default, Deref, DerefMut, Reflect)]
pub struct ConnectedTo(pub Vec<Entity>);

/// Marker component for leaf nodes in the integrity graph.
///
/// When an entity that is disabled becomes a leaf, it should be "destroyed" (chain reaction).
#[derive(Component, Debug, Default, Reflect)]
pub struct IntegrityLeafMarker;

/// Marker component for entities that are disabled due to having zero health.
///
/// When an entity that is leaf becomes disabled, it should be "destroyed".
#[derive(Component, Debug, Default, Reflect)]
pub struct IntegrityDisabledMarker;

#[derive(Component, Debug, Default, Reflect)]
pub struct IntegrityDestroyMarker;
