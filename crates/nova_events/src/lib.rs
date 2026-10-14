//! `nova_events` is the event vocabulary shared between gameplay and the
//! scenario engine. It defines the game-event kinds a scenario reacts to -
//! `OnStartEvent`, `OnUpdateEvent`, `OnDefeatedEvent`, `OnDestroyedEvent`,
//! `OnNeutralizedEvent`, area, orbit-lifecycle, lock, docking-lifecycle,
//! ship-order-completion, and timer events - and identity components that
//! tag scenario objects so filters can find them (`EntityId`, `EntityTypeName`). It is
//! engine-light glue: `nova_gameplay` emits these events and `nova_scenario`
//! filters and dispatches on them. It also owns the [`engine`] that queues and
//! dispatches those events.
//!
//! It also owns [`units`], the physical quantities Nova authors and reasons in,
//! [`scale`], the structural load limit, and [`generated`], the world grid
//! cell and the typed id of a body a generator placed in one. Those are not
//! events; they live here because this is the deepest crate BOTH the physics
//! side (`nova_ship`) and the display side (`nova_ui`) can reach, the scale
//! has to have exactly one definition in the workspace, and a generated id is
//! minted by `nova_world` above `nova_scenario`, which resolves it.
#![warn(missing_docs)]

use bevy::prelude::*;
use nova_events_macros::EventKind;

use crate::engine::*;

pub mod engine;
pub mod generated;
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
            EventKind, EventWorld, GameEvent, GameEventInfo, GameEventsPlugin, HandlerRetired,
        },
        generated::{GeneratedBodyId, NotAGeneratedBodyId, SectorCoord},
        scale::LOAD_LIMIT,
        units::prelude::*,
        AddedEventInfo, CargoEventInfo, CinematicEventInfo, CreditChangeCauseType,
        CreditsChangedEventInfo, DockingEventInfo, EntityId, EntityTypeName, EscortHiredEventInfo,
        ItemTradeType, LockEventInfo, MinedEventInfo, OnAddedEvent, OnCinematicFinishedEvent,
        OnCinematicSkippedEvent, OnCombatLockEndEvent, OnCombatLockStartEvent,
        OnCreditsChangedEvent, OnDefeatedEvent, OnDefeatedEventInfo, OnDestroyedEvent,
        OnDestroyedEventInfo, OnDockedEvent, OnEnterEvent, OnEnterEventInfo, OnEscortHiredEvent,
        OnExitEvent, OnExitEventInfo, OnGotoCompleteEvent, OnGotoCompleteEventInfo,
        OnJettisonedEvent, OnMinedEvent, OnNeutralizedEvent, OnNeutralizedEventInfo,
        OnOrbitEndEvent, OnOrbitLapEvent, OnOrbitStableEvent, OnOrbitStartEvent,
        OnOrbitUnstableEvent, OnPickedUpEvent, OnRebuiltEvent, OnRepairedEvent, OnResumeEvent,
        OnResumeEventInfo, OnShipBoughtEvent, OnShipOrderCanceledEvent,
        OnShipOrderCanceledEventInfo, OnShipOrderCompleteEvent, OnShipOrderCompleteEventInfo,
        OnShipOrderFailedEvent, OnShipOrderFailedEventInfo, OnShipOrderInterruptedEvent,
        OnShipOrderInterruptedEventInfo, OnShipOrderResumedEvent, OnShipOrderResumedEventInfo,
        OnStartEvent, OnStartEventInfo, OnStationBoughtEvent, OnStopCompleteEvent,
        OnStopCompleteEventInfo, OnStoredEvent, OnStrippedEvent, OnSwappedEvent, OnTimerEndEvent,
        OnTimerEndEventInfo, OnTradedEvent, OnTravelLockEndEvent, OnTravelLockStartEvent,
        OnUndockedEvent, OnUpdateEvent, OnUpdateEventInfo, OnWithdrawnEvent, OrbitEventInfo,
        RebuiltEventInfo, RepairedEventInfo, ScenarioAddressableMarker, ShipBoughtEventInfo,
        ShipOrderKind, StationBoughtEventInfo, StorageEventInfo, StrippedEventInfo,
        SwappedEventInfo, TradedEventInfo, ANCHOR_TYPE_NAME, ASTEROID_TYPE_NAME, BEACON_TYPE_NAME,
        CANISTER_FIELD_NAME, CINEMATIC_KEY_FIELD_NAME, CREDIT_CAUSE_FIELD_NAME,
        CURRENT_CREDITS_FIELD_NAME, DELTA_CREDITS_FIELD_NAME, ENTITY_ID_COMPONENT_NAME,
        ENTITY_OTHER_ID_COMPONENT_NAME, ENTITY_OTHER_TYPE_NAME_COMPONENT_NAME,
        ENTITY_TYPE_NAME_COMPONENT_NAME, ITEM_COUNT_FIELD_NAME, ITEM_FIELD_NAME, LIGHT_TYPE_NAME,
        PLANET_TYPE_NAME, PREVIOUS_CREDITS_FIELD_NAME, PRICE_FIELD_NAME, SECTION_FIELD_NAME,
        SHIP_ORDER_FIELD_NAME, SHIP_ORDER_KIND_FIELD_NAME, SPACESHIP_TYPE_NAME,
        TIMER_KEY_FIELD_NAME, TRADE_FIELD_NAME,
    };
}

/// Component tagging a scenario object with its scenario id, so event filters can
/// find it by name. Inserted by `nova_gameplay`/`nova_scenario` when spawning
/// scenario objects.
#[derive(
    Component, Debug, Clone, Default, Deref, DerefMut, Reflect, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct EntityId(pub String);

impl EntityId {
    /// Build an [`EntityId`] from anything convertible to a `String`.
    pub fn new<S: Into<String>>(s: S) -> Self {
        EntityId(s.into())
    }
}

/// Marker that an entity's [`EntityId`] is an AUTHORED id, so a scenario
/// action or an authored directive may address it by name.
///
/// A capability, not an exemption: an entity is addressable only when a
/// scenario author named it, so a spawner that forgets this marker loses a
/// lookup loudly instead of silently joining the authored namespace. Being
/// scoped to the session is not enough. A streamed sector and the bodies in
/// it are scoped - unloading must take them - and they carry generated ids
/// like `sector_0_0_0_body_0`, which nobody wrote.
///
/// It lives here, beside [`EntityId`], because the rule has to be readable
/// from every crate that resolves one. `nova_scenario` filters its own
/// actions on it, and so does `nova_ship`, which is BELOW `nova_scenario`
/// and resolves an authored well target of an orbit directive or an Orbit
/// helm order. A live-world surface that discovers entities rather than being
/// told their names - the `cmd>` shell's completion, a nearest-well orbit
/// target - deliberately does not filter on it.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ScenarioAddressableMarker;

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
/// Field name for a cinematic event's scenario-local key.
///
/// Named for the SUBJECT, like `order` and `kind`, and deliberately not `key`:
/// a payload field is what a filter reads, so two events spelling two different
/// identities the same way let a `Timer` filter answer a scene's ending. The
/// creator reference documents this field as `cinematic`.
pub const CINEMATIC_KEY_FIELD_NAME: &str = "cinematic";
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
// a contract between the spawner and every reader, and a reader (`nova_interface`'s
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
/// [`EntityTypeName`] value for an authored planet.
pub const PLANET_TYPE_NAME: &str = "planet";
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

/// Event kind fired once when a scenario cinematic ends for ANY reason
/// (`oncinematicfinished`): its last beat ran, the player skipped it, a
/// `CancelCinematic` stopped it, or a step blew its deadline.
///
/// The scene's post-state - camera released, control returned, the objective
/// the scene was hiding - is authored against this one event, so it lands on
/// every path out of the scene rather than only the patient one.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("oncinematicfinished")]
#[event_info(CinematicEventInfo)]
pub struct OnCinematicFinishedEvent;

/// Event kind fired when the player skips a cinematic (`oncinematicskipped`),
/// immediately before its [`OnCinematicFinishedEvent`].
///
/// Carries only what the skip has to CATCH UP: the world the unplayed beats
/// would have left behind. Everything both paths share belongs on the finish.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("oncinematicskipped")]
#[event_info(CinematicEventInfo)]
pub struct OnCinematicSkippedEvent;

/// Payload for [`OnCinematicFinishedEvent`] and [`OnCinematicSkippedEvent`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct CinematicEventInfo {
    /// Scenario-local key of the cinematic that ended, carried under
    /// [`CINEMATIC_KEY_FIELD_NAME`].
    #[serde(rename = "cinematic")]
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

/// Event kind fired once when a saved world's scenario is loaded again
/// (`onresume`); carries [`OnResumeEventInfo`]. It fires INSTEAD of
/// [`OnStartEvent`] on a resumed load: the saved state (objectives,
/// variables, timers, story, readouts, marks) is already restored when it
/// fires, so its handler spawns what the save does not carry, such as the
/// player and the lights, and never re-seeds scenario state.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onresume")]
#[event_info(OnResumeEventInfo)]
pub struct OnResumeEvent;

/// Payload for [`OnResumeEvent`] - empty (the resume event carries no
/// operands).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct OnResumeEventInfo;

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

/// A ship completed one net revolution of its ORBIT ring around a well.
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

/// Two hulls were clamped together by `DOCK` (`ondocked`).
///
/// The capture edge, not the sleeve art: the joint that holds the pair exists
/// when this fires, and the tubes are still reaching across the gap.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("ondocked")]
#[event_info(DockingEventInfo)]
pub struct OnDockedEvent;

/// A docking connection was let go (`onundocked`), for ANY reason: either
/// pilot pressed `DOCK` again, or the pair lost a port or a ship.
///
/// One event for every ending, the way [`OnCinematicFinishedEvent`] is one
/// event for every way out of a scene: what a beat owes the pair back - the
/// objective, the next order, the clearance - is the same whichever way the
/// clamp came off. The pair is named in the roles it DOCKED in, so the two
/// events filter identically.
///
/// A pair that dies WHOLE - both hulls gone in the same breath - reports only
/// [`OnDestroyedEvent`], the same silence area exit and orbit end keep for a
/// despawned body. One hull dying is a release like any other: the survivor is
/// free, and it is told so.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onundocked")]
#[event_info(DockingEventInfo)]
pub struct OnUndockedEvent;

/// Shared payload for the docking lifecycle: the ship that was docked WITH
/// (`id`) and the ship that asked for the dock (`other_id` /
/// `other_type_name`).
///
/// The same passive-then-active shape as the area, orbit and lock payloads,
/// so one `Entity` filter addresses all of them. `DOCK` is a player verb, so
/// the asking ship is the player's and the other side is whatever it flew up
/// to; both sides are hulls, which is why there is no `type_name` for the
/// first - it would read `spaceship` on every dock in the game.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct DockingEventInfo {
    /// Scenario id of the ship that was docked with.
    #[serde(rename = "id")]
    pub id: String,
    /// Scenario id of the ship that asked for the dock.
    #[serde(rename = "other_id")]
    pub other_id: String,
    /// Type name of the ship that asked for the dock.
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

/// Field name for the item design id an inventory event moved.
pub const ITEM_FIELD_NAME: &str = "item";
/// Field name for the unit count an inventory event moved.
pub const ITEM_COUNT_FIELD_NAME: &str = "count";
/// Field name for the canister runtime id a pickup or jettison names.
pub const CANISTER_FIELD_NAME: &str = "canister";
/// Field name for the [`ItemTradeType`] of a completed trade.
pub const TRADE_FIELD_NAME: &str = "trade";
/// Field name for the credits a trade moved.
pub const PRICE_FIELD_NAME: &str = "price_cr";
/// Field name for a ship's balance before a credits change.
pub const PREVIOUS_CREDITS_FIELD_NAME: &str = "previous_cr";
/// Field name for a ship's balance after a credits change.
pub const CURRENT_CREDITS_FIELD_NAME: &str = "current_cr";
/// Field name for the signed credits change.
pub const DELTA_CREDITS_FIELD_NAME: &str = "delta_cr";
/// Field name for the [`CreditChangeCauseType`] of a credits change.
pub const CREDIT_CAUSE_FIELD_NAME: &str = "cause";
/// Field name for the pane section code a repair named.
pub const SECTION_FIELD_NAME: &str = "section";

/// Which way a trade with a docked trader moves items and credits.
///
/// Authored in an `OnTraded` handler's expression and carried in the trade
/// payload, so it lives here beside the payload rather than in the gameplay
/// rule that plans the trade or the pane that applies it.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Reflect,
)]
pub enum ItemTradeType {
    /// Items from the trader into the player ship, paid at the item's ask.
    Buy,
    /// Items from the player ship into the trader, paid at the item's bid.
    Sell,
}

impl ItemTradeType {
    /// The name this direction carries in a serialized event payload; held to
    /// what serde writes by a test, for the same reason as
    /// [`ShipOrderKind::as_str`].
    pub fn as_str(self) -> &'static str {
        match self {
            ItemTradeType::Buy => "Buy",
            ItemTradeType::Sell => "Sell",
        }
    }
}

/// Why a ship's credits balance moved.
///
/// Every writer of a balance after spawn names its cause here, so a handler
/// can tell a sale from a looted strongbox without reading the sign.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Reflect,
)]
pub enum CreditChangeCauseType {
    /// A Buy or Sell with a docked trader moved the credits.
    Trade,
    /// A Take emptied a docked derelict's strongbox.
    Take,
    /// A rebuild at a docked station paid for a section's recipe.
    Rebuild,
    /// A swap at a docked station paid for the offered section's recipe.
    Swap,
    /// An add at a docked station paid for the new section's recipe.
    Add,
    /// A whole ship bought at a docked station: the new hull's price less
    /// the old hull's trade-in.
    ShipBuy,
    /// An escort hired at a docked station: its price.
    EscortHire,
    /// A station bought from its civilization while docked: its price.
    StationBuy,
    /// An armed `credits give` or `credits set` command wrote the balance.
    Cheat,
}

impl CreditChangeCauseType {
    /// The name this cause carries in a serialized event payload.
    pub fn as_str(self) -> &'static str {
        match self {
            CreditChangeCauseType::Trade => "Trade",
            CreditChangeCauseType::Take => "Take",
            CreditChangeCauseType::Rebuild => "Rebuild",
            CreditChangeCauseType::Swap => "Swap",
            CreditChangeCauseType::Add => "Add",
            CreditChangeCauseType::ShipBuy => "ShipBuy",
            CreditChangeCauseType::EscortHire => "EscortHire",
            CreditChangeCauseType::StationBuy => "StationBuy",
            CreditChangeCauseType::Cheat => "Cheat",
        }
    }
}

/// A mining beam pulse carved ore off an asteroid (`onmined`).
///
/// One event per pulse that flips at least one corner: the pulse is the
/// irrevocable mutation, and the canister it later ejects is a drop that
/// `onpickedup` already reports. A pulse that only warms the beam or hits a
/// barren rock fires nothing.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onmined")]
#[event_info(MinedEventInfo)]
pub struct OnMinedEvent;

/// Payload for [`OnMinedEvent`]: the asteroid that lost ore (`id`) and the
/// ship whose beam took it (`other_id` / `other_type_name`), with the ore
/// item id and the units carved.
///
/// Passive subject first, the way the docking payload names the ship docked
/// WITH first: a handler that watches one rock filters on `id`, one that
/// watches the miner filters on `other_id`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct MinedEventInfo {
    /// Scenario id of the asteroid that lost ore.
    #[serde(rename = "id")]
    pub id: String,
    /// Scenario id of the mining ship.
    #[serde(rename = "other_id")]
    pub other_id: String,
    /// Type name of the mining ship.
    #[serde(rename = "other_type_name")]
    pub other_type_name: String,
    /// Item design id of the ore the pulse carved.
    #[serde(rename = "item")]
    pub item: String,
    /// Ore units the pulse carved: one per flipped corner.
    #[serde(rename = "count")]
    pub count: u32,
}

/// A cargo intake took a canister into a ship's hold (`onpickedup`).
///
/// One event per item stack the canister carried, in the tick the intake
/// despawns it; a canister of two stacks reports twice with the same
/// `canister` id.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onpickedup")]
#[event_info(CargoEventInfo)]
pub struct OnPickedUpEvent;

/// A ship dropped a canister out of its hold (`onjettisoned`).
///
/// Fires from the intake that ejects the canister, not from the pane that
/// asked for it, so a queued jettison that never leaves the hull (the intake
/// is disabled or destroyed) reports nothing. One event per stack, like
/// [`OnPickedUpEvent`].
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onjettisoned")]
#[event_info(CargoEventInfo)]
pub struct OnJettisonedEvent;

/// Shared payload for pickup and jettison: the ship whose hold changed (`id`
/// / `type_name`), the canister's runtime id, and one item stack.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct CargoEventInfo {
    /// Scenario id of the ship whose hold changed.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of that ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
    /// Runtime id of the canister that was taken or dropped.
    #[serde(rename = "canister")]
    pub canister: u64,
    /// Item design id of the stack.
    #[serde(rename = "item")]
    pub item: String,
    /// Units in the stack.
    #[serde(rename = "count")]
    pub count: u32,
}

/// A Buy or Sell with a docked trader moved items and credits (`ontraded`).
///
/// One event per confirmed trade. The two balance moves it causes each raise
/// their own [`OnCreditsChangedEvent`].
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("ontraded")]
#[event_info(TradedEventInfo)]
pub struct OnTradedEvent;

/// Payload for [`OnTradedEvent`]: the trader (`id`) and the ship that traded
/// with it (`other_id` / `other_type_name`), the direction, the item, the
/// count and the credits that changed hands.
///
/// The trader is the passive subject, as the docked-with ship is in
/// [`DockingEventInfo`]: a trade is a player verb against the hull it docked
/// to, so a handler that watches one shop filters on `id`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Reflect)]
pub struct TradedEventInfo {
    /// Scenario id of the trader.
    #[serde(rename = "id")]
    pub id: String,
    /// Scenario id of the ship that traded.
    #[serde(rename = "other_id")]
    pub other_id: String,
    /// Type name of the ship that traded.
    #[serde(rename = "other_type_name")]
    pub other_type_name: String,
    /// Which way the items moved, seen from the trading ship.
    #[serde(rename = "trade")]
    pub trade: ItemTradeType,
    /// Item design id that moved.
    #[serde(rename = "item")]
    pub item: String,
    /// Units that moved.
    #[serde(rename = "count")]
    pub count: u32,
    /// Credits the buyer paid the seller.
    #[serde(rename = "price_cr")]
    pub price_cr: u32,
}

impl Default for TradedEventInfo {
    fn default() -> Self {
        Self {
            id: String::new(),
            other_id: String::new(),
            other_type_name: String::new(),
            trade: ItemTradeType::Buy,
            item: String::new(),
            count: 0,
            price_cr: 0,
        }
    }
}

/// A ship's credits balance moved (`oncreditschanged`).
///
/// One event per ship whose balance changed, from the one writer every
/// balance change goes through; a trade therefore raises two, one per party,
/// and a handler picks its side with an `Entity` filter on `id`. A write that
/// leaves the balance where it was raises nothing.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("oncreditschanged")]
#[event_info(CreditsChangedEventInfo)]
pub struct OnCreditsChangedEvent;

/// Payload for [`OnCreditsChangedEvent`]: the ship (`id` / `type_name`), its
/// balance before and after, the signed difference, and the cause.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Reflect)]
pub struct CreditsChangedEventInfo {
    /// Scenario id of the ship whose balance moved.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of that ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
    /// Balance before the change.
    #[serde(rename = "previous_cr")]
    pub previous_cr: u32,
    /// Balance after the change.
    #[serde(rename = "current_cr")]
    pub current_cr: u32,
    /// `current_cr - previous_cr`, negative when the ship paid.
    #[serde(rename = "delta_cr")]
    pub delta_cr: i64,
    /// Why the balance moved.
    #[serde(rename = "cause")]
    pub cause: CreditChangeCauseType,
}

impl CreditsChangedEventInfo {
    /// The signed difference a balance move carries; widened so a full-range
    /// drop from `u32::MAX` to zero still fits.
    pub fn delta(previous_cr: u32, current_cr: u32) -> i64 {
        i64::from(current_cr) - i64::from(previous_cr)
    }
}

impl Default for CreditsChangedEventInfo {
    fn default() -> Self {
        Self {
            id: String::new(),
            type_name: String::new(),
            previous_cr: 0,
            current_cr: 0,
            delta_cr: 0,
            cause: CreditChangeCauseType::Trade,
        }
    }
}

/// A Ship pane repair spent hull plates on a section (`onrepaired`).
///
/// One event per confirmed repair that spends at least one plate; a repair
/// the rule refuses (no plates, a full section) fires nothing.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onrepaired")]
#[event_info(RepairedEventInfo)]
pub struct OnRepairedEvent;

/// Payload for [`OnRepairedEvent`]: the ship (`id` / `type_name`), the pane
/// code of the repaired section, the plate item and the plates spent.
///
/// The section is named by its pane code (`HULL-3`) rather than its design
/// id: the code is what the player read when they ordered the repair, and a
/// hull carries the same design id on several sections.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct RepairedEventInfo {
    /// Scenario id of the repaired ship.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of that ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
    /// Pane section code of the repaired section.
    #[serde(rename = "section")]
    pub section: String,
    /// Item design id of the plate the repair spent.
    #[serde(rename = "item")]
    pub item: String,
    /// Plates the repair spent.
    #[serde(rename = "count")]
    pub count: u32,
}

/// A Ship pane rebuild at a docked station put a lost section back
/// (`onrebuilt`).
///
/// One event per rebuild the rule allows; a refused rebuild fires nothing.
/// The credits it paid also raise one `OnCreditsChanged` per party.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onrebuilt")]
#[event_info(RebuiltEventInfo)]
pub struct OnRebuiltEvent;

/// Payload for [`OnRebuiltEvent`]: the ship (`id` / `type_name`), the pane
/// code of the rebuilt section, every part the recipe spent and the credits
/// the ship paid the station.
///
/// The section is named by its pane code (`PDC-1`), as a repair names it:
/// the code derives from the design, so it is the code the section had
/// before it was lost.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct RebuiltEventInfo {
    /// Scenario id of the rebuilt ship.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of that ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
    /// Pane section code of the rebuilt section.
    #[serde(rename = "section")]
    pub section: String,
    /// Every part the recipe spent, as a count per item design id, from the
    /// hold and bought together.
    #[serde(rename = "parts")]
    pub parts: std::collections::BTreeMap<String, u32>,
    /// Credits the ship paid: the recipe's work plus the bought parts at ask.
    #[serde(rename = "price_cr")]
    pub price_cr: u32,
}

/// A Ship pane swap at a docked station replaced a live section with a part
/// of the same family and footprint (`onswapped`).
///
/// One event per swap the rule allows; a refused swap fires nothing. The
/// credits it paid also raise one `OnCreditsChanged` per party.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onswapped")]
#[event_info(SwappedEventInfo)]
pub struct OnSwappedEvent;

/// Payload for [`OnSwappedEvent`]: the ship (`id` / `type_name`), the pane
/// code of the swapped section, the section prototype it was and is now,
/// every part the recipe spent and the credits the ship paid the station.
///
/// A swap keeps the section's id and family, so its pane code holds.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct SwappedEventInfo {
    /// Scenario id of the ship.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of that ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
    /// Pane section code of the swapped section.
    #[serde(rename = "section")]
    pub section: String,
    /// The section prototype id the section was.
    #[serde(rename = "from")]
    pub from: String,
    /// The section prototype id it is now.
    #[serde(rename = "to")]
    pub to: String,
    /// Every part the offered part's recipe spent, as a count per item design
    /// id, from the hold and bought together.
    #[serde(rename = "parts")]
    pub parts: std::collections::BTreeMap<String, u32>,
    /// Credits the ship paid: the recipe's work plus the bought parts at ask.
    #[serde(rename = "price_cr")]
    pub price_cr: u32,
}

/// A Ship pane add at a docked station mounted a new section at a free
/// socket of a live section (`onadded`).
///
/// One event per add the rule allows; a refused add fires nothing. The
/// credits it paid also raise one `OnCreditsChanged` per party.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onadded")]
#[event_info(AddedEventInfo)]
pub struct OnAddedEvent;

/// Payload for [`OnAddedEvent`]: the ship (`id` / `type_name`), the pane
/// code the new section takes, the section prototype added, the pane code
/// and socket id it was mounted at, every part the recipe spent and the
/// credits the ship paid the station.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct AddedEventInfo {
    /// Scenario id of the ship.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of that ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
    /// Pane section code the new section takes.
    #[serde(rename = "section")]
    pub section: String,
    /// The section prototype id added.
    #[serde(rename = "part")]
    pub part: String,
    /// Pane section code of the section the part was mounted on.
    #[serde(rename = "at")]
    pub at: String,
    /// The socket id of that section the part mates.
    #[serde(rename = "point")]
    pub point: String,
    /// Every part the recipe spent, as a count per item design id, from the
    /// hold and bought together.
    #[serde(rename = "parts")]
    pub parts: std::collections::BTreeMap<String, u32>,
    /// Credits the ship paid: the recipe's work plus the bought parts at ask.
    #[serde(rename = "price_cr")]
    pub price_cr: u32,
}

/// A Ship pane buy at a docked station replaced the ship's whole hull with
/// one from the station's catalogue (`onshipbought`).
///
/// One event per buy the rule allows; a refused buy fires nothing. The
/// credits it moved also raise one `OnCreditsChanged` per party, and each
/// stack the old hold moved into the station's storage raises `OnStored`.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onshipbought")]
#[event_info(ShipBoughtEventInfo)]
pub struct OnShipBoughtEvent;

/// Payload for [`OnShipBoughtEvent`]: the ship (`id` / `type_name`), the
/// catalogue name of the hull it bought, the hull's price and what the
/// station paid for the old hull.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct ShipBoughtEventInfo {
    /// Scenario id of the ship.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of that ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
    /// The catalogue name of the bought hull.
    #[serde(rename = "hull")]
    pub hull: String,
    /// The new hull's price: every section's recipe at ask.
    #[serde(rename = "price_cr")]
    pub price_cr: u32,
    /// What the station paid for the old hull: every live section's strip
    /// value at bid, scaled by its health.
    #[serde(rename = "trade_in_cr")]
    pub trade_in_cr: u32,
}

/// A Ship pane hire at a docked station added an escort from the station's
/// catalogue to the player's fleet (`onescorthired`).
///
/// One event per hire the rule allows; a refused hire fires nothing. The
/// credits it moved also raise one `OnCreditsChanged` per party.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onescorthired")]
#[event_info(EscortHiredEventInfo)]
pub struct OnEscortHiredEvent;

/// Payload for [`OnEscortHiredEvent`]: the hiring ship (`id` / `type_name`),
/// the escort's scenario id, the catalogue name of its hull and its price.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct EscortHiredEventInfo {
    /// Scenario id of the hiring ship.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of that ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
    /// Scenario id of the hired escort.
    #[serde(rename = "escort")]
    pub escort: String,
    /// The catalogue name of the escort's hull.
    #[serde(rename = "hull")]
    pub hull: String,
    /// The hire price: every section's recipe at ask, times the hire factor.
    #[serde(rename = "price_cr")]
    pub price_cr: u32,
}

/// A Ship pane buy at a docked station made the station the player's
/// (`onstationbought`).
///
/// One event per buy the rule allows; a refused buy fires nothing. The
/// credits it moved also raise one `OnCreditsChanged` on the buying ship;
/// the station's balance does not move, since the price goes to its
/// civilization.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onstationbought")]
#[event_info(StationBoughtEventInfo)]
pub struct OnStationBoughtEvent;

/// Payload for [`OnStationBoughtEvent`]: the buying ship (`id` /
/// `type_name`), the station's scenario id and the price.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct StationBoughtEventInfo {
    /// Scenario id of the buying ship.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of that ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
    /// Scenario id of the bought station.
    #[serde(rename = "station")]
    pub station: String,
    /// The price: the station's worth at ask times the ownership factor.
    #[serde(rename = "price_cr")]
    pub price_cr: u32,
}

/// A Ship pane strip took one section off the docked lootable partner and
/// put its parts in the ship's hold (`onstripped`).
///
/// One event per strip that finished; a refused or cancelled strip fires
/// nothing. No credits move.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onstripped")]
#[event_info(StrippedEventInfo)]
pub struct OnStrippedEvent;

/// Payload for [`OnStrippedEvent`]: the ship that stripped (`id` /
/// `type_name`), the stripped hull's scenario id, the pane code of the
/// section it lost and every part the strip put in the hold.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct StrippedEventInfo {
    /// Scenario id of the ship that stripped.
    #[serde(rename = "id")]
    pub id: String,
    /// Type name of that ship.
    #[serde(rename = "type_name")]
    pub type_name: String,
    /// Scenario id of the stripped hull.
    #[serde(rename = "hull")]
    pub hull: String,
    /// Pane section code of the stripped section on that hull.
    #[serde(rename = "section")]
    pub section: String,
    /// Every part the strip put in the hold, as a count per item design id.
    #[serde(rename = "parts")]
    pub parts: std::collections::BTreeMap<String, u32>,
}

/// A Store at a docked station moved items from the ship's hold into the
/// station's storage (`onstored`).
///
/// One event per confirmed Store; a refused one fires nothing. No credits
/// move, so no [`OnCreditsChangedEvent`] follows.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onstored")]
#[event_info(StorageEventInfo)]
pub struct OnStoredEvent;

/// A Withdraw at a docked station moved items from the station's storage
/// into the ship's hold (`onwithdrawn`).
///
/// One event per confirmed Withdraw; a refused one fires nothing. No credits
/// move.
#[derive(Debug, Clone, EventKind, Reflect)]
#[event_name("onwithdrawn")]
#[event_info(StorageEventInfo)]
pub struct OnWithdrawnEvent;

/// Payload for [`OnStoredEvent`] and [`OnWithdrawnEvent`]: the station
/// (`id`), the ship that stored or withdrew (`other_id` /
/// `other_type_name`), the item and the count.
///
/// The station is the passive subject, as the trader is in
/// [`TradedEventInfo`], so a handler that watches one station filters on
/// `id`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Reflect)]
pub struct StorageEventInfo {
    /// Scenario id of the station.
    #[serde(rename = "id")]
    pub id: String,
    /// Scenario id of the ship that stored or withdrew.
    #[serde(rename = "other_id")]
    pub other_id: String,
    /// Type name of that ship.
    #[serde(rename = "other_type_name")]
    pub other_type_name: String,
    /// Item design id that moved.
    #[serde(rename = "item")]
    pub item: String,
    /// Units that moved.
    #[serde(rename = "count")]
    pub count: u32,
}

#[cfg(test)]
mod inventory_event_tests {
    use super::*;

    /// A filter compares against the serialized payload, so the two `as_str`
    /// names must be exactly what serde writes.
    #[test]
    fn the_trade_and_cause_names_are_the_ones_serde_writes() {
        for trade in [ItemTradeType::Buy, ItemTradeType::Sell] {
            let serialized = serde_json::to_value(trade).expect("a unit variant serializes");
            assert_eq!(serialized.as_str(), Some(trade.as_str()));
        }
        for cause in [
            CreditChangeCauseType::Trade,
            CreditChangeCauseType::Take,
            CreditChangeCauseType::Rebuild,
            CreditChangeCauseType::Swap,
            CreditChangeCauseType::Add,
            CreditChangeCauseType::ShipBuy,
            CreditChangeCauseType::EscortHire,
            CreditChangeCauseType::StationBuy,
            CreditChangeCauseType::Cheat,
        ] {
            let serialized = serde_json::to_value(cause).expect("a unit variant serializes");
            assert_eq!(serialized.as_str(), Some(cause.as_str()));
        }
    }

    /// The delta is signed and wide enough for the whole balance range: a
    /// looted strongbox and a full-price purchase both report exactly.
    #[test]
    fn the_credits_delta_is_signed_over_the_whole_range() {
        assert_eq!(CreditsChangedEventInfo::delta(50, 80), 30);
        assert_eq!(CreditsChangedEventInfo::delta(80, 50), -30);
        assert_eq!(
            CreditsChangedEventInfo::delta(u32::MAX, 0),
            -i64::from(u32::MAX)
        );
        assert_eq!(
            CreditsChangedEventInfo::delta(0, u32::MAX),
            i64::from(u32::MAX)
        );
    }

    /// The trade payload keeps the trader under the passive `id` slot and the
    /// trading ship under `other_id`, so one `Entity` filter shape serves
    /// docking and trading alike.
    #[test]
    fn the_trade_payload_names_the_trader_as_the_passive_subject() {
        let info = TradedEventInfo {
            id: "trader".to_string(),
            other_id: "miner".to_string(),
            other_type_name: SPACESHIP_TYPE_NAME.to_string(),
            trade: ItemTradeType::Sell,
            item: "StoneOre".to_string(),
            count: 4,
            price_cr: 12,
        };
        let value = serde_json::to_value(&info).expect("the payload serializes");
        assert_eq!(
            value.get(ENTITY_ID_COMPONENT_NAME).and_then(|v| v.as_str()),
            Some("trader")
        );
        assert_eq!(
            value
                .get(ENTITY_OTHER_ID_COMPONENT_NAME)
                .and_then(|v| v.as_str()),
            Some("miner")
        );
        assert_eq!(
            value.get(TRADE_FIELD_NAME).and_then(|v| v.as_str()),
            Some("Sell")
        );
        assert_eq!(
            value.get(PRICE_FIELD_NAME).and_then(|v| v.as_u64()),
            Some(12)
        );
    }
}
