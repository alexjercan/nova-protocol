//! `nova_events` is the event vocabulary shared between gameplay and the
//! scenario engine. It defines the game-event kinds a scenario reacts to -
//! `OnStartEvent`, `OnUpdateEvent`, `OnDestroyedEvent`, the area
//! `OnEnterEvent`/`OnExitEvent`, `OnOrbitEvent`, `OnTravelLockEvent`,
//! `OnCombatLockEvent` - and the entity-identity components that tag scenario
//! objects so filters can find them (`EntityId`, `EntityTypeName`). It is
//! engine-light glue: `nova_gameplay` emits these events and `nova_scenario`
//! filters and dispatches on them.

use bevy::prelude::*;
use bevy_common_systems::prelude::*;

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

#[derive(Component, Debug, Clone, Default, Deref, DerefMut, Reflect)]
pub struct EntityId(pub String);

impl EntityId {
    pub fn new<S: Into<String>>(s: S) -> Self {
        EntityId(s.into())
    }
}

pub const ENTITY_ID_COMPONENT_NAME: &str = "id";
pub const ENTITY_TYPE_NAME_COMPONENT_NAME: &str = "type_name";
pub const ENTITY_OTHER_ID_COMPONENT_NAME: &str = "other_id";
pub const ENTITY_OTHER_TYPE_NAME_COMPONENT_NAME: &str = "other_type_name";

#[derive(Component, Debug, Clone, Default, Deref, DerefMut, Reflect)]
pub struct EntityTypeName(pub String);

impl EntityTypeName {
    pub fn new<S: Into<String>>(s: S) -> Self {
        EntityTypeName(s.into())
    }
}

#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onstart")]
#[event_info(OnStartEventInfo)]
pub struct OnStartEvent;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnStartEventInfo;

#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("ondestroyed")]
#[event_info(OnDestroyedEventInfo)]
pub struct OnDestroyedEvent;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnDestroyedEventInfo {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "type_name")]
    pub type_name: String,
}

#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onenter")]
#[event_info(OnEnterEventInfo)]
pub struct OnEnterEvent;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnEnterEventInfo {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "other_id")]
    pub other_id: String,
    #[serde(rename = "other_type_name")]
    pub other_type_name: String,
}

#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onexit")]
#[event_info(OnExitEventInfo)]
pub struct OnExitEvent;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnExitEventInfo {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "other_id")]
    pub other_id: String,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnOrbitEventInfo {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "other_id")]
    pub other_id: String,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnTravelLockEventInfo {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "other_id")]
    pub other_id: String,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnCombatLockEventInfo {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "other_id")]
    pub other_id: String,
    #[serde(rename = "other_type_name")]
    pub other_type_name: String,
}

#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onupdate")]
#[event_info(OnUpdateEventInfo)]
pub struct OnUpdateEvent;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnUpdateEventInfo;
