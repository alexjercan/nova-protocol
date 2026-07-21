use bevy::prelude::*;
use bevy_common_systems::prelude::*;
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
            EventConfig::OnUpdate => EventHandler::new::<OnUpdateEvent>(),
            EventConfig::OnEnter => EventHandler::new::<OnEnterEvent>(),
            EventConfig::OnExit => EventHandler::new::<OnExitEvent>(),
            EventConfig::OnOrbit => EventHandler::new::<OnOrbitEvent>(),
            EventConfig::OnTravelLock => EventHandler::new::<OnTravelLockEvent>(),
            EventConfig::OnCombatLock => EventHandler::new::<OnCombatLockEvent>(),
        }
    }
}
