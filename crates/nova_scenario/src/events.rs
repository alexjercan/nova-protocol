//! [`EventConfig`], the authored form of a scenario event: the name a handler
//! listens for, lowered into the `nova_events` engine's own event kind.
//!
//! Touch this module when adding an event a scenario can react to.

use bevy::prelude::*;
use nova_events::prelude::*;

use crate::prelude::*;

/// Glob-import surface: `use nova_scenario::events::prelude::*` brings the
/// [`EventConfig`] handler-trigger enum into scope.
pub mod prelude {
    pub use super::EventConfig;
}

/// The event a handler reacts to: the RON `name` of a scenario handler, mapped
/// to the concrete `nova_events` event type it dispatches on.
#[derive(Debug, Clone, Copy, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EventConfig {
    /// Fires once, right after a scenario loads.
    OnStart,
    /// Fires when an entity is destroyed.
    OnDestroyed,
    /// Fires when a ship is NEUTRALIZED - an armed combatant that has lost all
    /// working weapons AND all working thrusters, so it is out of the fight
    /// even with hull intact and still present in the world. Distinct from
    /// `OnDestroyed`; filters by ship id/type_name the same way.
    OnNeutralized,
    /// Fires every frame while a scenario is live and unpaused.
    OnUpdate,
    /// Fires when a body enters an area/zone (`id` = the area, other = the body).
    OnEnter,
    /// Fires when a body leaves an area/zone (`id` = the area, other = the body).
    OnExit,
    /// A ship has held an autopilot orbit around a well for the hold
    /// window (the orbit-hold tracker in loader.rs fires it once per
    /// engagement). Filters like OnEnter: id = the well, other = the ship.
    OnOrbit,
    /// The player's TRAVEL lock landed on a scenario object (the lock
    /// bridge in loader.rs fires it once per acquisition). Filters like
    /// OnEnter: id = the locked target, other = the player ship.
    OnTravelLock,
    /// The player's COMBAT lock landed on a scenario object. Same contract
    /// as OnTravelLock.
    OnCombatLock,
}

impl From<EventConfig> for EventHandler<NovaEventWorld> {
    fn from(value: EventConfig) -> Self {
        match value {
            EventConfig::OnStart => EventHandler::new::<OnStartEvent>(),
            EventConfig::OnDestroyed => EventHandler::new::<OnDestroyedEvent>(),
            EventConfig::OnNeutralized => EventHandler::new::<OnNeutralizedEvent>(),
            EventConfig::OnUpdate => EventHandler::new::<OnUpdateEvent>(),
            EventConfig::OnEnter => EventHandler::new::<OnEnterEvent>(),
            EventConfig::OnExit => EventHandler::new::<OnExitEvent>(),
            EventConfig::OnOrbit => EventHandler::new::<OnOrbitEvent>(),
            EventConfig::OnTravelLock => EventHandler::new::<OnTravelLockEvent>(),
            EventConfig::OnCombatLock => EventHandler::new::<OnCombatLockEvent>(),
        }
    }
}
