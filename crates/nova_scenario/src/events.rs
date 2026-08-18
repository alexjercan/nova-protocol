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
    /// Fires once when a ship is neutralized or directly destroyed.
    OnDefeated,
    /// Fires when an entity is physically destroyed.
    OnDestroyed,
    /// Fires when a ship is NEUTRALIZED - an armed combatant that is disarmed
    /// (no working weapon) OR brain-dead (had a flight computer, none working),
    /// so it is out of the fight even with hull intact and still present in
    /// the world. Distinct from `OnDestroyed`; filters by ship id/type_name
    /// the same way.
    OnNeutralized,
    /// Fires every frame while a scenario is live and unpaused.
    OnUpdate,
    /// Fires once when a keyed scenario timer ends.
    OnTimerEnd,
    /// Fires when a body enters an area/zone (`id` = the area, other = the body).
    OnEnter,
    /// Fires when a body leaves an area/zone (`id` = the area, other = the body).
    OnExit,
    /// An ORBIT maneuver engaged for a well.
    OnOrbitStart,
    /// An ORBIT maneuver entered stable station-keeping.
    OnOrbitStable,
    /// Stable station-keeping was lost while ORBIT remained engaged.
    OnOrbitUnstable,
    /// A surviving ship ended ORBIT or switched wells.
    OnOrbitEnd,
    /// The player's TRAVEL lock landed on a scenario object.
    OnTravelLockStart,
    /// The player's TRAVEL lock left a scenario object.
    OnTravelLockEnd,
    /// The player's COMBAT lock landed on a scenario object.
    OnCombatLockStart,
    /// The player's COMBAT lock left a scenario object.
    OnCombatLockEnd,
}

impl From<EventConfig> for EventHandler<NovaEventWorld> {
    fn from(value: EventConfig) -> Self {
        match value {
            EventConfig::OnStart => EventHandler::new::<OnStartEvent>(),
            EventConfig::OnDefeated => EventHandler::new::<OnDefeatedEvent>(),
            EventConfig::OnDestroyed => EventHandler::new::<OnDestroyedEvent>(),
            EventConfig::OnNeutralized => EventHandler::new::<OnNeutralizedEvent>(),
            EventConfig::OnUpdate => EventHandler::new::<OnUpdateEvent>(),
            EventConfig::OnTimerEnd => EventHandler::new::<OnTimerEndEvent>(),
            EventConfig::OnEnter => EventHandler::new::<OnEnterEvent>(),
            EventConfig::OnExit => EventHandler::new::<OnExitEvent>(),
            EventConfig::OnOrbitStart => EventHandler::new::<OnOrbitStartEvent>(),
            EventConfig::OnOrbitStable => EventHandler::new::<OnOrbitStableEvent>(),
            EventConfig::OnOrbitUnstable => EventHandler::new::<OnOrbitUnstableEvent>(),
            EventConfig::OnOrbitEnd => EventHandler::new::<OnOrbitEndEvent>(),
            EventConfig::OnTravelLockStart => EventHandler::new::<OnTravelLockStartEvent>(),
            EventConfig::OnTravelLockEnd => EventHandler::new::<OnTravelLockEndEvent>(),
            EventConfig::OnCombatLockStart => EventHandler::new::<OnCombatLockStartEvent>(),
            EventConfig::OnCombatLockEnd => EventHandler::new::<OnCombatLockEndEvent>(),
        }
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;

    #[test]
    fn on_defeated_round_trips_through_authored_ron() {
        let event: EventConfig = ron::from_str("OnDefeated").unwrap();
        assert!(matches!(event, EventConfig::OnDefeated));
        assert_eq!(ron::to_string(&event).unwrap(), "OnDefeated");
    }
}
