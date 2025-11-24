use bevy::{platform::collections::HashMap, prelude::*};

pub mod prelude {
    pub use super::{IntegrityDisabledMarker, IntegrityGraph, IntegrityLeafMarker};
}

/// A graph representing how the collider+health entities are connected to each other.
#[derive(Component, Debug, Default, Deref, DerefMut, Reflect)]
pub struct IntegrityGraph(pub HashMap<Entity, Vec<Entity>>);

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

