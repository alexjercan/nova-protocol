//! `nova_events` is the event vocabulary shared between gameplay and the
//! scenario engine. It defines the game-event kinds a scenario reacts to -
//! `OnStartEvent`, `OnUpdateEvent`, `OnDefeatedEvent`, `OnDestroyedEvent`,
//! `OnNeutralizedEvent`,
//! area, orbit-lifecycle, lock, and timer events - and identity components that
//! tag scenario objects so filters can find them (`EntityId`, `EntityTypeName`). It is
//! engine-light glue: `nova_gameplay` emits these events and `nova_scenario`
//! filters and dispatches on them. It also owns the [`engine`] that queues and
//! dispatches those events.
//!
//! It also owns [`units`], the physical quantities Nova authors and reasons in,
//! and [`scale`], the structural load limit. Those are not events; they live
//! here because this is the deepest crate BOTH the physics side (`nova_ship`)
//! and the display side (`nova_ui`) can reach, and the scale has to have
//! exactly one definition in the workspace.
#![warn(missing_docs)]

use bevy::prelude::*;
use nova_events_macros::EventKind;

use crate::engine::*;

pub mod engine;
pub mod scale;
pub mod units;

/// Glob-import surface: `use nova_events::prelude::*` brings the entity-identity
/// components, every `On*Event`/`On*EventInfo` pair, the reflect-field name and
/// well-known type-name constants, the physical quantity types and the
/// world-scale constants, and the event engine into scope.
pub mod prelude {
    // The derive expands to `impl EventKind for #name`, so the TRAIT must
    // be in scope wherever the derive is used. Keep the two exported together.
    pub use nova_events_macros::EventKind;

    pub use super::{
        engine::{
            CommandsGameEventExt, EventAction, EventFilter, EventHandler, EventHandlerIndex,
            EventKind, EventWorld, GameEvent, GameEventInfo, GameEventsPlugin,
        },
        scale::LOAD_LIMIT,
        units::prelude::*,
        EntityId, EntityTypeName, LockEventInfo, OnCombatLockEndEvent, OnCombatLockStartEvent,
        OnDefeatedEvent, OnDefeatedEventInfo, OnDestroyedEvent, OnDestroyedEventInfo, OnEnterEvent,
        OnEnterEventInfo, OnExitEvent, OnExitEventInfo, OnNeutralizedEvent, OnNeutralizedEventInfo,
        OnOrbitEndEvent, OnOrbitStableEvent, OnOrbitStartEvent, OnOrbitUnstableEvent, OnStartEvent,
        OnStartEventInfo, OnTimerEndEvent, OnTimerEndEventInfo, OnTravelLockEndEvent,
        OnTravelLockStartEvent, OnUpdateEvent, OnUpdateEventInfo, OrbitEventInfo, ANCHOR_TYPE_NAME,
        ASTEROID_TYPE_NAME, BEACON_TYPE_NAME, ENTITY_ID_COMPONENT_NAME,
        ENTITY_OTHER_ID_COMPONENT_NAME, ENTITY_OTHER_TYPE_NAME_COMPONENT_NAME,
        ENTITY_TYPE_NAME_COMPONENT_NAME, LIGHT_TYPE_NAME, SALVAGE_CRATE_TYPE_NAME,
        SPACESHIP_TYPE_NAME, TIMER_KEY_FIELD_NAME,
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
/// Field name for a timer event's scenario-local key.
pub const TIMER_KEY_FIELD_NAME: &str = "key";

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

// The well-known type names live HERE, beside the component that carries them,
// rather than beside the `nova_scenario` object that spawns each one. They are
// a contract between the spawner and every reader, and a reader (`nova_os_ui`'s
// map, `nova_gameplay`'s integrity tests) must not depend on `nova_scenario` to
// name one. This crate is the floor both sides already stand on.
/// [`EntityTypeName`] value for an authored anchor.
pub const ANCHOR_TYPE_NAME: &str = "anchor";
/// [`EntityTypeName`] value for an authored asteroid.
pub const ASTEROID_TYPE_NAME: &str = "asteroid";
/// [`EntityTypeName`] value for an authored beacon.
pub const BEACON_TYPE_NAME: &str = "beacon";
/// [`EntityTypeName`] value for an authored light.
pub const LIGHT_TYPE_NAME: &str = "light";
/// [`EntityTypeName`] value for an authored salvage crate.
pub const SALVAGE_CRATE_TYPE_NAME: &str = "salvage_crate";
/// [`EntityTypeName`] value for an authored spaceship.
pub const SPACESHIP_TYPE_NAME: &str = "spaceship";

/// Event kind fired once when a keyed scenario timer ends (`ontimerend`).
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("ontimerend")]
#[event_info(OnTimerEndEventInfo)]
pub struct OnTimerEndEvent;

/// Payload for [`OnTimerEndEvent`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnTimerEndEventInfo {
    /// Scenario-local key of the timer that ended.
    pub key: String,
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

/// Event kind fired once when a ship is defeated (`ondefeated`) by either
/// neutralization or direct physical destruction. Later destruction of an
/// already-neutralized wreck does not fire it again.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("ondefeated")]
#[event_info(OnDefeatedEventInfo)]
pub struct OnDefeatedEvent;

/// Payload for [`OnDefeatedEvent`]: the defeated ship's scenario id and type
/// name (RON keys `id` / `type_name`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnDefeatedEventInfo {
    /// Scenario id of the defeated ship.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of the defeated ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
}

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

/// Event kind fired when a ship is NEUTRALIZED (`onneutralized`) - it was an
/// armed combatant that is now disarmed (no working weapon) OR brain-dead (had
/// a flight computer, none working), so it is out of the fight even though its
/// hull may be intact and the ship is still present in the world. Distinct
/// from [`OnDestroyedEvent`]: a neutralized ship is not despawned. Carries
/// [`OnNeutralizedEventInfo`] naming the ship.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onneutralized")]
#[event_info(OnNeutralizedEventInfo)]
pub struct OnNeutralizedEvent;

/// Payload for [`OnNeutralizedEvent`]: the neutralized ship's scenario id and
/// type name (RON keys `id` / `type_name`), mirroring [`OnDestroyedEventInfo`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnNeutralizedEventInfo {
    /// Scenario id of the neutralized ship.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of the neutralized ship.
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

/// An ORBIT maneuver started around a well.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onorbitstart")]
#[event_info(OrbitEventInfo)]
pub struct OnOrbitStartEvent;

/// An ORBIT maneuver entered its stable station-keeping phase.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onorbitstable")]
#[event_info(OrbitEventInfo)]
pub struct OnOrbitStableEvent;

/// A stable ORBIT became unstable while its maneuver remained engaged.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onorbitunstable")]
#[event_info(OrbitEventInfo)]
pub struct OnOrbitUnstableEvent;

/// A surviving ship ended or switched its ORBIT maneuver.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onorbitend")]
#[event_info(OrbitEventInfo)]
pub struct OnOrbitEndEvent;

/// Shared payload for orbit lifecycle events: the well (`id`) and ship
/// (`other_id` / `other_type_name`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OrbitEventInfo {
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

/// The player's TRAVEL lock (white, nav) landed on a scenario object.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("ontravellockstart")]
#[event_info(LockEventInfo)]
pub struct OnTravelLockStartEvent;

/// The player's TRAVEL lock left a scenario object.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("ontravellockend")]
#[event_info(LockEventInfo)]
pub struct OnTravelLockEndEvent;

/// The player's COMBAT lock (red) landed on a scenario object.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("oncombatlockstart")]
#[event_info(LockEventInfo)]
pub struct OnCombatLockStartEvent;

/// The player's COMBAT lock left a scenario object.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("oncombatlockend")]
#[event_info(LockEventInfo)]
pub struct OnCombatLockEndEvent;

/// Shared payload for lock lifecycle events: the locked target (`id`) and the
/// locking ship (`other_id` / `other_type_name`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct LockEventInfo {
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
