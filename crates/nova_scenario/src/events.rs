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
    /// The player's GOTO maneuver reached its target and came to rest.
    OnGotoComplete,
    /// The player's STOP maneuver brought the ship to rest.
    OnStopComplete,
    /// An ORBIT maneuver engaged for a well.
    OnOrbitStart,
    /// An ORBIT maneuver entered stable station-keeping.
    OnOrbitStable,
    /// One net revolution completed while ORBIT remained stable.
    OnOrbitLap,
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
    /// A ship reached a keyed HELM order's condition - it arrived, settled on
    /// a bearing, came to rest, closed its patrol loop, or established its
    /// orbit. Cancellation, interruption and failure each have their own
    /// event, so a beat chained off a completion never runs for an order that
    /// did not finish.
    OnShipOrderComplete,
    /// Autonomous AI took the helm back from an installed order. TRANSIENT -
    /// the order stays installed and resumes, so a beat here must not retire
    /// what it is waiting for.
    OnShipOrderInterrupted,
    /// An interrupted order got its helm back and picked its directive up
    /// where it left off.
    OnShipOrderResumed,
    /// A ship order was retired on purpose and for good - `ClearShipOrder`, or
    /// a replacement order taking the helm. Terminal; an order that had
    /// already completed or failed does not also cancel.
    OnShipOrderCanceled,
    /// An accepted ship order became impossible to continue: the well an orbit
    /// needed went away, or the hull lost the computer or engines the maneuver
    /// runs on. Terminal. An order REFUSED at issue time never reaches this -
    /// that is a lint error, not a scenario event.
    OnShipOrderFailed,
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
            EventConfig::OnGotoComplete => EventHandler::new::<OnGotoCompleteEvent>(),
            EventConfig::OnStopComplete => EventHandler::new::<OnStopCompleteEvent>(),
            EventConfig::OnOrbitStart => EventHandler::new::<OnOrbitStartEvent>(),
            EventConfig::OnOrbitStable => EventHandler::new::<OnOrbitStableEvent>(),
            EventConfig::OnOrbitLap => EventHandler::new::<OnOrbitLapEvent>(),
            EventConfig::OnOrbitUnstable => EventHandler::new::<OnOrbitUnstableEvent>(),
            EventConfig::OnOrbitEnd => EventHandler::new::<OnOrbitEndEvent>(),
            EventConfig::OnTravelLockStart => EventHandler::new::<OnTravelLockStartEvent>(),
            EventConfig::OnTravelLockEnd => EventHandler::new::<OnTravelLockEndEvent>(),
            EventConfig::OnCombatLockStart => EventHandler::new::<OnCombatLockStartEvent>(),
            EventConfig::OnCombatLockEnd => EventHandler::new::<OnCombatLockEndEvent>(),
            EventConfig::OnShipOrderComplete => EventHandler::new::<OnShipOrderCompleteEvent>(),
            EventConfig::OnShipOrderInterrupted => {
                EventHandler::new::<OnShipOrderInterruptedEvent>()
            }
            EventConfig::OnShipOrderResumed => EventHandler::new::<OnShipOrderResumedEvent>(),
            EventConfig::OnShipOrderCanceled => EventHandler::new::<OnShipOrderCanceledEvent>(),
            EventConfig::OnShipOrderFailed => EventHandler::new::<OnShipOrderFailedEvent>(),
        }
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;

    #[test]
    fn maneuver_completion_events_round_trip_through_authored_ron() {
        for event in [
            EventConfig::OnGotoComplete,
            EventConfig::OnStopComplete,
            EventConfig::OnOrbitLap,
        ] {
            let ron = ron::to_string(&event).expect("the event serializes");
            let back: EventConfig = ron::from_str(&ron).expect("the event deserializes");
            assert_eq!(format!("{back:?}"), format!("{event:?}"));
        }
    }

    #[test]
    fn on_defeated_round_trips_through_authored_ron() {
        let event: EventConfig = ron::from_str("OnDefeated").unwrap();
        assert!(matches!(event, EventConfig::OnDefeated));
        assert_eq!(ron::to_string(&event).unwrap(), "OnDefeated");
    }
}
