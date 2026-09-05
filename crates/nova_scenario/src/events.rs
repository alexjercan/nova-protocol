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

/// Declare the closed trigger vocabulary.
///
/// One row names the authored RON tag, the `nova_events` event type it
/// dispatches on, and the label an authoring menu lists it under. The enum, the
/// lowering, the menu list and the labels all come off it, so an event added
/// here is an event the editor can already offer.
macro_rules! scenario_events {
    (
        $(
            $(#[$meta:meta])*
            $variant:ident => $event:ty { label: $label:literal }
        ),* $(,)?
    ) => {
        /// The event a handler reacts to: the RON `name` of a scenario handler,
        /// mapped to the concrete `nova_events` event type it dispatches on.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub enum EventConfig {
            $(
                $(#[$meta])*
                $variant,
            )*
        }

        impl From<EventConfig> for EventHandler<NovaEventWorld> {
            fn from(value: EventConfig) -> Self {
                match value {
                    $( EventConfig::$variant => EventHandler::new::<$event>(), )*
                }
            }
        }

        impl EventConfig {
            /// How many triggers there are.
            pub const COUNT: usize = [$( stringify!($variant), )*].len();

            /// Every trigger a handler can listen for, in menu order.
            pub const ALL: [EventConfig; Self::COUNT] = [
                $( EventConfig::$variant, )*
            ];

            /// The variant name, which is also the RON tag.
            pub fn name(self) -> &'static str {
                match self {
                    $( EventConfig::$variant => stringify!($variant), )*
                }
            }

            /// The row label an authoring menu lists this trigger under.
            pub fn label(self) -> &'static str {
                match self {
                    $( EventConfig::$variant => $label, )*
                }
            }
        }
    };
}

scenario_events! {
    /// Fires once, right after a scenario loads.
    OnStart => OnStartEvent { label: "On Start" },
    /// Fires once when a ship is neutralized or directly destroyed.
    OnDefeated => OnDefeatedEvent { label: "On Defeated" },
    /// Fires when an entity is physically destroyed.
    OnDestroyed => OnDestroyedEvent { label: "On Destroyed" },
    /// Fires when a ship is NEUTRALIZED - an armed combatant that is disarmed
    /// (no working weapon) OR brain-dead (had a flight computer, none working),
    /// so it is out of the fight even with hull intact and still present in
    /// the world. Distinct from `OnDestroyed`; filters by ship id/type_name
    /// the same way.
    OnNeutralized => OnNeutralizedEvent { label: "On Neutralized" },
    /// Fires every frame while a scenario is live and unpaused.
    OnUpdate => OnUpdateEvent { label: "On Update" },
    /// Fires once when a keyed scenario timer ends.
    OnTimerEnd => OnTimerEndEvent { label: "On Timer End" },
    /// Fires when a cinematic ends for ANY reason: its last beat ran, the
    /// player skipped it, a `CancelCinematic` stopped it, or a step blew its
    /// deadline. What the scene owes the player back - the camera, the
    /// controls, the objective it was covering - is authored here, once.
    OnCinematicFinished => OnCinematicFinishedEvent { label: "On Cinematic Finished" },
    /// Fires when the player skips a cinematic, immediately BEFORE its
    /// `OnCinematicFinished`. Carries only the catch-up: what the beats that
    /// never played would have left behind.
    OnCinematicSkipped => OnCinematicSkippedEvent { label: "On Cinematic Skipped" },
    /// Fires when a body enters an area/zone (`id` = the area, other = the body).
    OnEnter => OnEnterEvent { label: "On Enter" },
    /// Fires when a body leaves an area/zone (`id` = the area, other = the body).
    OnExit => OnExitEvent { label: "On Exit" },
    /// The player's GOTO maneuver reached its target and came to rest.
    OnGotoComplete => OnGotoCompleteEvent { label: "On GOTO Complete" },
    /// The player's STOP maneuver brought the ship to rest.
    OnStopComplete => OnStopCompleteEvent { label: "On STOP Complete" },
    /// An ORBIT maneuver engaged for a well.
    OnOrbitStart => OnOrbitStartEvent { label: "On Orbit Start" },
    /// An ORBIT maneuver entered stable station-keeping.
    OnOrbitStable => OnOrbitStableEvent { label: "On Orbit Stable" },
    /// One net revolution of the ORBIT ring completed. Corrections on the
    /// ring keep counting; only a sustained departure restarts the lap.
    OnOrbitLap => OnOrbitLapEvent { label: "On Orbit Lap" },
    /// Stable station-keeping was lost while ORBIT remained engaged.
    OnOrbitUnstable => OnOrbitUnstableEvent { label: "On Orbit Unstable" },
    /// A surviving ship ended ORBIT or switched wells.
    OnOrbitEnd => OnOrbitEndEvent { label: "On Orbit End" },
    /// The player's TRAVEL lock landed on a scenario object.
    OnTravelLockStart => OnTravelLockStartEvent { label: "On Travel Lock" },
    /// The player's TRAVEL lock left a scenario object.
    OnTravelLockEnd => OnTravelLockEndEvent { label: "On Travel Unlock" },
    /// The player's COMBAT lock landed on a scenario object.
    OnCombatLockStart => OnCombatLockStartEvent { label: "On Combat Lock" },
    /// The player's COMBAT lock left a scenario object.
    OnCombatLockEnd => OnCombatLockEndEvent { label: "On Combat Unlock" },
    /// A ship reached a keyed HELM order's condition - it arrived, settled on
    /// a bearing, came to rest, closed its patrol loop, or established its
    /// orbit. Cancellation, interruption and failure each have their own
    /// event, so a beat chained off a completion never runs for an order that
    /// did not finish.
    OnShipOrderComplete => OnShipOrderCompleteEvent { label: "On Ship Order Complete" },
    /// Autonomous AI took the helm back from an installed order. TRANSIENT -
    /// the order stays installed and resumes, so a beat here must not retire
    /// what it is waiting for.
    OnShipOrderInterrupted => OnShipOrderInterruptedEvent { label: "On Ship Order Interrupted" },
    /// An interrupted order got its helm back and picked its directive up
    /// where it left off.
    OnShipOrderResumed => OnShipOrderResumedEvent { label: "On Ship Order Resumed" },
    /// A ship order was retired on purpose and for good - `ClearShipOrder`, or
    /// a replacement order taking the helm. Terminal; an order that had
    /// already completed or failed does not also cancel.
    OnShipOrderCanceled => OnShipOrderCanceledEvent { label: "On Ship Order Canceled" },
    /// An accepted ship order became impossible to continue: the well an orbit
    /// needed went away, or the hull lost the computer or engines the maneuver
    /// runs on. Terminal. An order REFUSED at issue time never reaches this -
    /// that is a lint error, not a scenario event.
    OnShipOrderFailed => OnShipOrderFailedEvent { label: "On Ship Order Failed" },
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

#[cfg(test)]
mod table_tests {
    use super::*;

    /// The trigger table is the editor's whole list of triggers, so two rows
    /// sharing a name or a label would hide one event behind another.
    #[test]
    fn every_trigger_has_its_own_name_and_label() {
        for field in [EventConfig::name, EventConfig::label] {
            let mut values: Vec<&str> = EventConfig::ALL.iter().map(|name| field(*name)).collect();
            values.sort_unstable();
            values.dedup();
            assert_eq!(values.len(), EventConfig::COUNT);
            assert!(values.iter().all(|value| !value.trim().is_empty()));
        }
    }

    /// A handler's `name` in RON is the variant, so the table's name and the
    /// tag it deserializes from are one string.
    #[cfg(feature = "serde")]
    #[test]
    fn a_trigger_deserializes_from_the_name_the_table_reports() {
        for expected in EventConfig::ALL {
            let parsed: EventConfig =
                ron::from_str(expected.name()).expect("the table's name is the RON tag");
            assert_eq!(parsed, expected);
        }
    }
}
