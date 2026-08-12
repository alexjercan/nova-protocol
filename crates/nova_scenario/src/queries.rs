//! Typed read-only scenario queries and continuously sampled watches.

/// Glob-import surface for scenario query authoring types.
pub mod prelude {
    pub use super::{
        EntityProperty, EntityQuery, EntityQueryFilter, QueryConfig, ScenarioProperty,
        ScenarioQuery, WatchConfig,
    };
}

/// A typed read-only query supported by scenario expressions and watches.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QueryConfig {
    /// Read a property of the running scenario.
    Scenario(ScenarioQuery),
    /// Read a property from exactly one matching entity.
    Entity(EntityQuery),
}

/// A query against the running scenario itself.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScenarioQuery {
    /// The scenario property to return.
    pub property: ScenarioProperty,
}

/// Properties available on the running scenario.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ScenarioProperty {
    /// Live, unpaused seconds since this scenario started.
    Elapsed,
}

/// A strict single-entity query.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityQuery {
    /// The selector that must match exactly one entity.
    pub filter: EntityQueryFilter,
    /// The entity property to return.
    pub property: EntityProperty,
}

/// Filters supported by a strict single-entity query.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityQueryFilter {
    /// Exact authored entity id.
    pub id: String,
}

/// Properties available on a single entity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EntityProperty {
    /// Magnitude of the entity's linear velocity in units per second.
    Speed,
}

/// A query sampled each live update and published under a variable name.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WatchConfig {
    /// Read-only variable name receiving the sampled value.
    pub variable: String,
    /// Typed query sampled for this watch.
    pub query: QueryConfig,
}
