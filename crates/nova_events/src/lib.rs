//! `nova_events` is the event vocabulary shared between gameplay and the
//! scenario engine. It defines the game-event kinds a scenario reacts to -
//! `OnStartEvent`, `OnUpdateEvent`, `OnDestroyedEvent`, the area
//! `OnEnterEvent`/`OnExitEvent`, `OnOrbitEvent`, `OnTravelLockEvent`,
//! `OnCombatLockEvent` - and the entity-identity components that tag scenario
//! objects so filters can find them (`EntityId`, `EntityTypeName`). It is
//! engine-light glue: `nova_gameplay` emits these events and `nova_scenario`
//! filters and dispatches on them.
#![warn(missing_docs)]

use bevy::prelude::*;
use bevy_common_systems::prelude::*;

/// Glob-import surface: `use nova_events::prelude::*` brings the entity-identity
/// components, every `On*Event`/`On*EventInfo` pair, and the reflect-field name
/// constants into scope.
pub mod prelude {
    pub use super::{
        EntityId, EntityTypeName, OnCombatLockEvent, OnCombatLockEventInfo, OnDestroyedEvent,
        OnDestroyedEventInfo, OnEnterEvent, OnEnterEventInfo, OnExitEvent, OnExitEventInfo,
        OnOrbitEvent, OnOrbitEventInfo, OnStartEvent, OnStartEventInfo, OnTravelLockEvent,
        OnTravelLockEventInfo, OnUpdateEvent, OnUpdateEventInfo, ENTITY_ID_COMPONENT_NAME,
        ENTITY_OTHER_ID_COMPONENT_NAME, ENTITY_OTHER_TYPE_NAME_COMPONENT_NAME,
        ENTITY_TYPE_NAME_COMPONENT_NAME,
    };
}

/// Component tagging a scenario object with its scenario id, so event filters can
/// find it by name. Inserted by `nova_gameplay`/`nova_scenario` when spawning
/// scenario objects.
#[derive(Component, Debug, Clone, Default, Deref, DerefMut, Reflect)]
pub struct EntityId(pub String);

impl EntityId {
    /// Build an [`EntityId`] from anything convertible to a `String`.
    pub fn new<S: Into<String>>(s: S) -> Self {
        EntityId(s.into())
    }
}

/// Reflect field name for the acting entity's id (the `id` key in event info).
pub const ENTITY_ID_COMPONENT_NAME: &str = "id";
/// Reflect field name for the acting entity's type name (`type_name`).
pub const ENTITY_TYPE_NAME_COMPONENT_NAME: &str = "type_name";
/// Reflect field name for the other entity's id (`other_id`) in pair events.
pub const ENTITY_OTHER_ID_COMPONENT_NAME: &str = "other_id";
/// Reflect field name for the other entity's type name (`other_type_name`).
pub const ENTITY_OTHER_TYPE_NAME_COMPONENT_NAME: &str = "other_type_name";

/// Component tagging a scenario object with its type name, so event filters can
/// match on kind. Inserted alongside [`EntityId`] when spawning scenario objects.
#[derive(Component, Debug, Clone, Default, Deref, DerefMut, Reflect)]
pub struct EntityTypeName(pub String);

impl EntityTypeName {
    /// Build an [`EntityTypeName`] from anything convertible to a `String`.
    pub fn new<S: Into<String>>(s: S) -> Self {
        EntityTypeName(s.into())
    }
}

/// Event kind fired once when the scenario starts (`onstart`); carries
/// [`OnStartEventInfo`]. `nova_scenario` uses it to run start triggers.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onstart")]
#[event_info(OnStartEventInfo)]
pub struct OnStartEvent;

/// Payload for [`OnStartEvent`] - empty (the start event carries no operands).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnStartEventInfo;

/// Event kind fired when a scenario object is destroyed (`ondestroyed`); carries
/// [`OnDestroyedEventInfo`] naming the destroyed entity.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("ondestroyed")]
#[event_info(OnDestroyedEventInfo)]
pub struct OnDestroyedEvent;

/// Payload for [`OnDestroyedEvent`]: the destroyed entity's scenario id and type
/// name (RON keys `id` / `type_name`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnDestroyedEventInfo {
    /// Scenario id of the destroyed entity.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of the destroyed entity.
    #[serde(rename = "type_name")]
    pub type_name: String,
}

/// Event kind fired when one entity enters another's area (`onenter`); carries
/// [`OnEnterEventInfo`].
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onenter")]
#[event_info(OnEnterEventInfo)]
pub struct OnEnterEvent;

/// Payload for [`OnEnterEvent`]: the area entity (`id`) and the entering entity
/// (`other_id` / `other_type_name`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnEnterEventInfo {
    /// Scenario id of the area entity.
    #[serde(rename = "id")]
    pub id: String,
    /// Scenario id of the entering entity.
    #[serde(rename = "other_id")]
    pub other_id: String,
    /// Type name of the entering entity.
    #[serde(rename = "other_type_name")]
    pub other_type_name: String,
}

/// Event kind fired when one entity leaves another's area (`onexit`); carries
/// [`OnExitEventInfo`], the same shape as [`OnEnterEvent`].
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onexit")]
#[event_info(OnExitEventInfo)]
pub struct OnExitEvent;

/// Payload for [`OnExitEvent`]: the area entity (`id`) and the leaving entity
/// (`other_id` / `other_type_name`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnExitEventInfo {
    /// Scenario id of the area entity.
    #[serde(rename = "id")]
    pub id: String,
    /// Scenario id of the leaving entity.
    #[serde(rename = "other_id")]
    pub other_id: String,
    /// Type name of the leaving entity.
    #[serde(rename = "other_type_name")]
    pub other_type_name: String,
}

/// A ship has HELD a stable autopilot orbit around a well for the hold
/// window (nova_scenario's orbit-hold tracker fires it). `id` is the
/// well's scenario id, `other` the orbiting ship - the same shape as
/// [`OnEnterEvent`], so scenario filters compose identically.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onorbit")]
#[event_info(OnOrbitEventInfo)]
pub struct OnOrbitEvent;

/// Payload for [`OnOrbitEvent`]: the orbited well (`id`) and the orbiting ship
/// (`other_id` / `other_type_name`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnOrbitEventInfo {
    /// Scenario id of the orbited well.
    #[serde(rename = "id")]
    pub id: String,
    /// Scenario id of the orbiting ship.
    #[serde(rename = "other_id")]
    pub other_id: String,
    /// Type name of the orbiting ship.
    #[serde(rename = "other_type_name")]
    pub other_type_name: String,
}

/// The player's TRAVEL lock (white, nav) landed on a scenario object
/// (nova_scenario's lock bridge fires it, once per acquisition). `id` is
/// the locked target's scenario id, `other` the locking ship - the same
/// shape as [`OnEnterEvent`], so scenario filters compose identically.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("ontravellock")]
#[event_info(OnTravelLockEventInfo)]
pub struct OnTravelLockEvent;

/// Payload for [`OnTravelLockEvent`]: the locked target (`id`) and the locking
/// ship (`other_id` / `other_type_name`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnTravelLockEventInfo {
    /// Scenario id of the locked target.
    #[serde(rename = "id")]
    pub id: String,
    /// Scenario id of the locking ship.
    #[serde(rename = "other_id")]
    pub other_id: String,
    /// Type name of the locking ship.
    #[serde(rename = "other_type_name")]
    pub other_type_name: String,
}

/// The player's COMBAT lock (red) landed on a scenario object. Same
/// contract as [`OnTravelLockEvent`]; a separate event (not a field) so
/// the entity filters keep working unchanged.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("oncombatlock")]
#[event_info(OnCombatLockEventInfo)]
pub struct OnCombatLockEvent;

/// Payload for [`OnCombatLockEvent`]: the locked target (`id`) and the locking
/// ship (`other_id` / `other_type_name`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnCombatLockEventInfo {
    /// Scenario id of the locked target.
    #[serde(rename = "id")]
    pub id: String,
    /// Scenario id of the locking ship.
    #[serde(rename = "other_id")]
    pub other_id: String,
    /// Type name of the locking ship.
    #[serde(rename = "other_type_name")]
    pub other_type_name: String,
}

/// Event kind fired every scenario tick (`onupdate`); carries
/// [`OnUpdateEventInfo`]. `nova_scenario` uses it to run per-frame triggers.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onupdate")]
#[event_info(OnUpdateEventInfo)]
pub struct OnUpdateEvent;

/// Payload for [`OnUpdateEvent`] - empty (the tick event carries no operands).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnUpdateEventInfo;
