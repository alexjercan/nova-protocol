//! `nova_events` is the event vocabulary shared between gameplay and the
//! scenario engine. It defines the game-event kinds a scenario reacts to -
//! `OnStartEvent`, `OnUpdateEvent`, `OnDefeatedEvent`, `OnDestroyedEvent`,
//! `OnNeutralizedEvent`, area, orbit-lifecycle, lock, ship-order-completion,
//! and timer events - and identity components that
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
        OnEnterEventInfo, OnExitEvent, OnExitEventInfo, OnGotoCompleteEvent,
        OnGotoCompleteEventInfo, OnNeutralizedEvent, OnNeutralizedEventInfo, OnOrbitEndEvent,
        OnOrbitLapEvent, OnOrbitStableEvent, OnOrbitStartEvent, OnOrbitUnstableEvent,
        OnShipOrderCanceledEvent, OnShipOrderCanceledEventInfo, OnShipOrderCompleteEvent,
        OnShipOrderCompleteEventInfo, OnShipOrderFailedEvent, OnShipOrderFailedEventInfo,
        OnShipOrderInterruptedEvent, OnShipOrderInterruptedEventInfo, OnShipOrderResumedEvent,
        OnShipOrderResumedEventInfo, OnStartEvent, OnStartEventInfo, OnStopCompleteEvent,
        OnStopCompleteEventInfo, OnTimerEndEvent, OnTimerEndEventInfo, OnTravelLockEndEvent,
        OnTravelLockStartEvent, OnUpdateEvent, OnUpdateEventInfo, OrbitEventInfo, ShipOrderKind,
        ANCHOR_TYPE_NAME, ASTEROID_TYPE_NAME, BEACON_TYPE_NAME, ENTITY_ID_COMPONENT_NAME,
        ENTITY_OTHER_ID_COMPONENT_NAME, ENTITY_OTHER_TYPE_NAME_COMPONENT_NAME,
        ENTITY_TYPE_NAME_COMPONENT_NAME, LIGHT_TYPE_NAME, SALVAGE_CRATE_TYPE_NAME,
        SHIP_ORDER_FIELD_NAME, SHIP_ORDER_KIND_FIELD_NAME, SPACESHIP_TYPE_NAME,
        TIMER_KEY_FIELD_NAME,
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
/// Field name for a completed ship order's authored key.
pub const SHIP_ORDER_FIELD_NAME: &str = "order";
/// Field name for a completed ship order's [`ShipOrderKind`].
pub const SHIP_ORDER_KIND_FIELD_NAME: &str = "kind";

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

/// The player's GOTO maneuver reached its target and came to rest.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("ongotocomplete")]
#[event_info(OnGotoCompleteEventInfo)]
pub struct OnGotoCompleteEvent;

/// Payload for [`OnGotoCompleteEvent`]: the destination (`id`) and player ship
/// (`other_id` / `other_type_name`). This matches lock and area pair events so
/// the ordinary entity-pair filter can address both sides.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnGotoCompleteEventInfo {
    /// Scenario id of the reached GOTO target.
    #[serde(rename = "id")]
    pub id: String,
    /// Scenario id of the player ship that arrived.
    #[serde(rename = "other_id")]
    pub other_id: String,
    /// Type name of the player ship.
    #[serde(rename = "other_type_name")]
    pub other_type_name: String,
}

/// The player's STOP maneuver brought the ship to rest.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onstopcomplete")]
#[event_info(OnStopCompleteEventInfo)]
pub struct OnStopCompleteEvent;

/// Payload for [`OnStopCompleteEvent`], naming the player ship that stopped.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnStopCompleteEventInfo {
    /// Scenario id of the player ship.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of the player ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
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

/// A ship completed one net revolution in stable ORBIT around a well.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onorbitlap")]
#[event_info(OrbitEventInfo)]
pub struct OnOrbitLapEvent;

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

/// Which HELM order a ship was given.
///
/// The helm actions are one mutually exclusive family, so the kind is part of
/// an order's identity: a handler that waits for "the warship finished
/// turning" must not be woken by the move that preceded it. Authored in a
/// `ShipOrder` filter and carried in every ship-order payload, so it lives
/// here beside them rather than in either crate that reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Reflect)]
pub enum ShipOrderKind {
    /// A `MoveShipTo` order: the ship flew to its mark and came to rest.
    Move,
    /// A `ForceAlign` order: the ship turned onto its bearing and settled.
    Align,
    /// A `StopShip` order: the ship killed its velocity.
    Stop,
    /// A `PatrolShip` order: the ship flew one loop of its authored route.
    Patrol,
    /// An `OrbitShip` order: the ship established a station-keeping orbit.
    Orbit,
}

impl ShipOrderKind {
    /// The name this kind carries in a serialized event payload.
    ///
    /// A filter reads the payload as JSON, so it needs the variant's SERIALIZED
    /// name, and deriving it by hand at the read site is how the two drift
    /// apart. One function, and a test that holds it to what serde emits.
    pub fn as_str(self) -> &'static str {
        match self {
            ShipOrderKind::Move => "Move",
            ShipOrderKind::Align => "Align",
            ShipOrderKind::Stop => "Stop",
            ShipOrderKind::Patrol => "Patrol",
            ShipOrderKind::Orbit => "Orbit",
        }
    }
}

/// Event kind fired once when a ship completes a keyed HELM order
/// (`onshipordercomplete`) - arrival, alignment, stop, patrol loop or orbit
/// insertion.
///
/// The completion a scenario SEQUENCES on. A cinematic that fires a railgun
/// once the bore is on its target cannot use a guessed delay: the turn takes
/// as long as the hull's rotation authority says it takes.
///
/// Completion reports that the physical CONDITION was reached, not that the
/// behavior stopped: an alignment holds its bearing afterwards and an orbit
/// keeps station-keeping. Cancellation, replacement and failure each have
/// their own event, so a handler waiting on a completion never runs for an
/// order that did not finish.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onshipordercomplete")]
#[event_info(OnShipOrderCompleteEventInfo)]
pub struct OnShipOrderCompleteEvent;

/// Payload for [`OnShipOrderCompleteEvent`]: the completed order's authored
/// key (`order`) and kind (`kind`), plus the ship that completed it.
///
/// The ship keeps the well-known `id` / `type_name` names, so the ordinary
/// entity filter still matches on it. The order's own identity gets its own
/// two fields instead of riding on the entity id or the timer key: those name
/// other things, and a handler that confused them would fire on the wrong beat.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Reflect)]
pub struct OnShipOrderCompleteEventInfo {
    /// Authored key of the completed order.
    #[serde(rename = "order")]
    pub order: String,
    /// Scenario id of the ship that completed it.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of that ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
    /// Which helm order completed.
    #[serde(rename = "kind")]
    pub kind: ShipOrderKind,
}

impl Default for OnShipOrderCompleteEventInfo {
    fn default() -> Self {
        Self {
            order: String::new(),
            id: String::new(),
            type_name: String::new(),
            kind: ShipOrderKind::Move,
        }
    }
}

/// Event kind fired when autonomous AI takes the helm back from an installed
/// ship order (`onshiporderinterrupted`).
///
/// TRANSIENT, and the only ship-order event that is: the durable order stays
/// installed and its directive is kept whole, so the AI flies its own routine
/// and the order resumes from where it was. An interruption can happen many
/// times over one order's life, which is exactly why it is not cancellation -
/// a beat that retires its objective here would retire it for a fight the
/// ship is about to come back from.
///
/// Only an AI ship can produce this, and only one whose profile authorizes an
/// interruption. Without that authorization an order owns the helm until a
/// terminal outcome.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onshiporderinterrupted")]
#[event_info(OnShipOrderInterruptedEventInfo)]
pub struct OnShipOrderInterruptedEvent;

/// Payload for [`OnShipOrderInterruptedEvent`]. Same four fields, under the
/// same names, as every other ship-order payload: one `ShipOrder` filter
/// matches whichever of them a handler listens for.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Reflect)]
pub struct OnShipOrderInterruptedEventInfo {
    /// Authored key of the interrupted order.
    #[serde(rename = "order")]
    pub order: String,
    /// Scenario id of the ship that was flying it.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of that ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
    /// Which helm order was interrupted.
    #[serde(rename = "kind")]
    pub kind: ShipOrderKind,
}

impl Default for OnShipOrderInterruptedEventInfo {
    fn default() -> Self {
        Self {
            order: String::new(),
            id: String::new(),
            type_name: String::new(),
            kind: ShipOrderKind::Move,
        }
    }
}

/// Event kind fired when an interrupted ship order gets its helm back
/// (`onshiporderresumed`).
///
/// The other half of [`OnShipOrderInterruptedEvent`], and the reason the
/// directive is kept durable rather than living only in the autopilot: the
/// execution is rebuilt from what the order still says, so a half-flown move
/// carries on to the same mark instead of restarting somewhere else.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onshiporderresumed")]
#[event_info(OnShipOrderResumedEventInfo)]
pub struct OnShipOrderResumedEvent;

/// Payload for [`OnShipOrderResumedEvent`]; the same four fields as every
/// other ship-order payload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Reflect)]
pub struct OnShipOrderResumedEventInfo {
    /// Authored key of the resumed order.
    #[serde(rename = "order")]
    pub order: String,
    /// Scenario id of the ship flying it again.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of that ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
    /// Which helm order resumed.
    #[serde(rename = "kind")]
    pub kind: ShipOrderKind,
}

impl Default for OnShipOrderResumedEventInfo {
    fn default() -> Self {
        Self {
            order: String::new(),
            id: String::new(),
            type_name: String::new(),
            kind: ShipOrderKind::Move,
        }
    }
}

/// Event kind fired when a ship order is retired on purpose and for good
/// (`onshipordercanceled`) - `ClearShipOrder`, or a replacement order taking
/// the helm.
///
/// TERMINAL, and the counterpart to interruption: nothing resumes. A beat
/// holding an objective open "until the tug finishes docking" retires it
/// here, because the tug never will.
///
/// An order that already reached a terminal outcome does not cancel a second
/// time - clearing a completed alignment reports nothing, because the
/// alignment did complete.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onshipordercanceled")]
#[event_info(OnShipOrderCanceledEventInfo)]
pub struct OnShipOrderCanceledEvent;

/// Payload for [`OnShipOrderCanceledEvent`]; the same four fields as every
/// other ship-order payload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Reflect)]
pub struct OnShipOrderCanceledEventInfo {
    /// Authored key of the canceled order.
    #[serde(rename = "order")]
    pub order: String,
    /// Scenario id of the ship it was installed on.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of that ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
    /// Which helm order was canceled.
    #[serde(rename = "kind")]
    pub kind: ShipOrderKind,
}

impl Default for OnShipOrderCanceledEventInfo {
    fn default() -> Self {
        Self {
            order: String::new(),
            id: String::new(),
            type_name: String::new(),
            kind: ShipOrderKind::Move,
        }
    }
}

/// Event kind fired when an ACCEPTED ship order becomes impossible to
/// continue (`onshiporderfailed`).
///
/// TERMINAL. The order was installed and being flown, and then the world took
/// something away it needed: the well an orbit was inserting into stopped
/// existing, or the hull lost the flight computer or the engines the maneuver
/// runs on. The difference from cancellation is who decided - a scenario
/// cancels, the world fails.
///
/// An order REFUSED at issue time (no such ship, a player-driven one, an
/// empty patrol route) was never accepted and reports nothing here; that is a
/// lint error and a runtime log, not a scenario event. There is deliberately
/// no reason code yet: the executor logs the detailed cause, and a scenario
/// that must branch on which failure happened would need reasons this engine
/// cannot yet enumerate honestly.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onshiporderfailed")]
#[event_info(OnShipOrderFailedEventInfo)]
pub struct OnShipOrderFailedEvent;

/// Payload for [`OnShipOrderFailedEvent`]; the same four fields as every
/// other ship-order payload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Reflect)]
pub struct OnShipOrderFailedEventInfo {
    /// Authored key of the failed order.
    #[serde(rename = "order")]
    pub order: String,
    /// Scenario id of the ship that could not finish it.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of that ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
    /// Which helm order failed.
    #[serde(rename = "kind")]
    pub kind: ShipOrderKind,
}

impl Default for OnShipOrderFailedEventInfo {
    fn default() -> Self {
        Self {
            order: String::new(),
            id: String::new(),
            type_name: String::new(),
            kind: ShipOrderKind::Move,
        }
    }
}

#[cfg(test)]
mod ship_order_tests {
    use super::*;

    /// [`ShipOrderKind::as_str`] is what a scenario filter compares against a
    /// serialized payload, so it must be exactly what serde writes there. A
    /// silent divergence would make every `kind`-filtered handler stop
    /// matching, with nothing logged.
    #[test]
    fn the_order_kind_name_is_the_one_serde_writes() {
        for kind in [
            ShipOrderKind::Move,
            ShipOrderKind::Align,
            ShipOrderKind::Stop,
            ShipOrderKind::Patrol,
            ShipOrderKind::Orbit,
        ] {
            let serialized = serde_json::to_value(kind).expect("a unit variant serializes");
            assert_eq!(
                serialized.as_str(),
                Some(kind.as_str()),
                "as_str must match the serialized form of {kind:?}"
            );
        }
    }

    /// The completion payload names the ship with the WELL-KNOWN entity keys
    /// and the order with its own two, so the ordinary entity filter still
    /// matches the ship while order identity never rides on an entity id.
    #[test]
    fn the_completion_payload_keeps_order_identity_off_the_entity_keys() {
        let info = OnShipOrderCompleteEventInfo {
            order: "warship_bore".to_string(),
            id: "warship".to_string(),
            type_name: SPACESHIP_TYPE_NAME.to_string(),
            kind: ShipOrderKind::Align,
        };
        let value = serde_json::to_value(&info).expect("the payload serializes");

        assert_eq!(
            value.get(ENTITY_ID_COMPONENT_NAME).and_then(|v| v.as_str()),
            Some("warship"),
            "the ship is reachable through the ordinary entity filter"
        );
        assert_eq!(
            value.get(SHIP_ORDER_FIELD_NAME).and_then(|v| v.as_str()),
            Some("warship_bore"),
            "the order key has its own field"
        );
        assert_eq!(
            value
                .get(SHIP_ORDER_KIND_FIELD_NAME)
                .and_then(|v| v.as_str()),
            Some("Align"),
            "and so does the kind"
        );
        assert!(
            value.get(TIMER_KEY_FIELD_NAME).is_none(),
            "the order key does not masquerade as a timer key"
        );
    }
}
