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
    pub use super::{BlockCategoryType, EventConfig};
}

/// The group an authoring palette files a block under.
///
/// One closed set for both tables, because a palette lists triggers and actions
/// side by side and a builder looks for a subject, not for a Rust type. The
/// engine never reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
pub enum BlockCategoryType {
    /// What the run itself does: starting, ending, timing, chaining beats.
    Flow,
    /// What the player is told: objectives, comms and the HUD.
    Mission,
    /// What stands on the board, and what happens to the board.
    World,
    /// The camera, the player's controls and scripted scenes.
    Camera,
    /// What a ship can do and is told to do.
    Ships,
    /// Sides, weapons and what a fight reports.
    Combat,
    /// Cargo, credits, yards and what they report.
    Trade,
    /// Scenario variables, values and authoring aids.
    Data,
}

impl BlockCategoryType {
    /// Every group, in the order a palette lists them.
    pub const ALL: [BlockCategoryType; 8] = [
        BlockCategoryType::Flow,
        BlockCategoryType::Mission,
        BlockCategoryType::World,
        BlockCategoryType::Camera,
        BlockCategoryType::Ships,
        BlockCategoryType::Combat,
        BlockCategoryType::Trade,
        BlockCategoryType::Data,
    ];

    /// The word a palette shows.
    pub fn label(self) -> &'static str {
        match self {
            BlockCategoryType::Flow => "Flow",
            BlockCategoryType::Mission => "Mission",
            BlockCategoryType::World => "World",
            BlockCategoryType::Camera => "Camera",
            BlockCategoryType::Ships => "Ships",
            BlockCategoryType::Combat => "Combat",
            BlockCategoryType::Trade => "Trade",
            BlockCategoryType::Data => "Data",
        }
    }
}

/// Declare the closed trigger vocabulary.
///
/// One row names the authored RON tag, the `nova_events` event type it
/// dispatches on, the label an authoring menu lists it under, and the palette
/// group it is filed in. The enum, the lowering, the menu list and the labels
/// all come off it, so an event added here is an event the editor can already
/// offer, and a row with no category does not compile.
macro_rules! scenario_events {
    (
        $(
            $(#[$meta:meta])*
            $variant:ident => $event:ty { label: $label:literal, category: $category:ident }
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

            /// The group an authoring palette files this trigger under.
            pub fn category(self) -> BlockCategoryType {
                match self {
                    $( EventConfig::$variant => BlockCategoryType::$category, )*
                }
            }

            /// The payload fields an expression can read with `event.<field>`
            /// under this trigger, sorted: every text, number and boolean
            /// field of the payload type the row names.
            ///
            /// Read off a default payload rather than listed by hand, so a
            /// field added to a payload struct is a field the lint accepts,
            /// and one removed is one it refuses.
            pub fn payload_fields(self) -> Vec<String> {
                self.default_payload()
                    .field_names()
                    .into_iter()
                    .map(str::to_string)
                    .collect()
            }

            /// The payload this trigger's event type carries, with every field
            /// at its default.
            fn default_payload(self) -> GameEventInfo {
                match self {
                    $(
                        EventConfig::$variant => GameEventInfo::from_data(
                            <<$event as EventKind>::Info>::default(),
                        ),
                    )*
                }
            }
        }
    };
}

scenario_events! {
    /// Fires once, right after a scenario loads.
    OnStart => OnStartEvent { label: "On Start", category: Flow },
    /// Fires once, instead of `OnStart`, when a saved world's scenario loads
    /// again: the saved objectives, variables, timers, story, readouts and
    /// marks are already restored, so the handler spawns only what the save
    /// does not carry.
    OnResume => OnResumeEvent { label: "On Resume", category: Flow },
    /// Fires once when a ship is neutralized or directly destroyed.
    OnDefeated => OnDefeatedEvent { label: "On Defeated", category: Combat },
    /// Fires when an entity is physically destroyed.
    OnDestroyed => OnDestroyedEvent { label: "On Destroyed", category: Combat },
    /// Fires when a ship is NEUTRALIZED - an armed combatant that is disarmed
    /// (no working weapon) OR brain-dead (had a flight computer, none working),
    /// so it is out of the fight even with hull intact and still present in
    /// the world. Distinct from `OnDestroyed`; filters by ship id/type_name
    /// the same way.
    OnNeutralized => OnNeutralizedEvent { label: "On Neutralized", category: Combat },
    /// Fires every frame while a scenario is live and unpaused.
    OnUpdate => OnUpdateEvent { label: "On Update", category: Flow },
    /// Fires once when a keyed scenario timer ends.
    OnTimerEnd => OnTimerEndEvent { label: "On Timer End", category: Flow },
    /// An action refused to run at run time and changed nothing (`action` =
    /// its RON name, `id` = the id it targeted, `reason` = a stable word
    /// such as `insufficient_credits`). Once per refused action.
    OnActionRefused => OnActionRefusedEvent { label: "On Action Refused", category: Flow },
    /// Fires when a cinematic ends for ANY reason: its last beat ran, the
    /// player skipped it, a `CancelCinematic` stopped it, or a step blew its
    /// deadline. What the scene owes the player back - the camera, the
    /// controls, the objective it was covering - is authored here, once.
    OnCinematicFinished => OnCinematicFinishedEvent { label: "On Cinematic Finished", category: Camera },
    /// Fires when the player skips a cinematic, immediately BEFORE its
    /// `OnCinematicFinished`. Carries only the catch-up: what the beats that
    /// never played would have left behind.
    OnCinematicSkipped => OnCinematicSkippedEvent { label: "On Cinematic Skipped", category: Camera },
    /// Fires when a body enters an area/zone (`id` = the area, other = the body).
    OnEnter => OnEnterEvent { label: "On Enter", category: World },
    /// Fires when a body leaves an area/zone (`id` = the area, other = the body).
    OnExit => OnExitEvent { label: "On Exit", category: World },
    /// The player's GOTO maneuver reached its target and came to rest.
    OnGotoComplete => OnGotoCompleteEvent { label: "On GOTO Complete", category: Ships },
    /// The player's STOP maneuver brought the ship to rest.
    OnStopComplete => OnStopCompleteEvent { label: "On STOP Complete", category: Ships },
    /// An ORBIT maneuver engaged for a well.
    OnOrbitStart => OnOrbitStartEvent { label: "On Orbit Start", category: Ships },
    /// An ORBIT maneuver entered stable station-keeping.
    OnOrbitStable => OnOrbitStableEvent { label: "On Orbit Stable", category: Ships },
    /// One net revolution of the ORBIT ring completed. Corrections on the
    /// ring keep counting; only a sustained departure restarts the lap.
    OnOrbitLap => OnOrbitLapEvent { label: "On Orbit Lap", category: Ships },
    /// Stable station-keeping was lost while ORBIT remained engaged.
    OnOrbitUnstable => OnOrbitUnstableEvent { label: "On Orbit Unstable", category: Ships },
    /// A surviving ship ended ORBIT or switched wells.
    OnOrbitEnd => OnOrbitEndEvent { label: "On Orbit End", category: Ships },
    /// The player's TRAVEL lock landed on a scenario object.
    OnTravelLockStart => OnTravelLockStartEvent { label: "On Travel Lock", category: Ships },
    /// The player's TRAVEL lock left a scenario object.
    OnTravelLockEnd => OnTravelLockEndEvent { label: "On Travel Unlock", category: Ships },
    /// The player's COMBAT lock landed on a scenario object.
    OnCombatLockStart => OnCombatLockStartEvent { label: "On Combat Lock", category: Combat },
    /// The player's COMBAT lock left a scenario object.
    OnCombatLockEnd => OnCombatLockEndEvent { label: "On Combat Unlock", category: Combat },
    /// Two hulls were clamped together by `DOCK` (`id` = the ship docked
    /// with, other = the ship that asked).
    OnDocked => OnDockedEvent { label: "On Docked", category: Ships },
    /// A dock was let go, by either pilot's `DOCK`, or a lost port or ship.
    /// One event for every ending, and the pair keeps the roles it docked in.
    /// A ship despawned while docked reports only `OnDestroyed`.
    OnUndocked => OnUndockedEvent { label: "On Undocked", category: Ships },
    /// A ship reached a keyed HELM order's condition - it arrived, settled on
    /// a bearing, came to rest, closed its patrol loop, or established its
    /// orbit. Cancellation, interruption and failure each have their own
    /// event, so a beat chained off a completion never runs for an order that
    /// did not finish.
    OnShipOrderComplete => OnShipOrderCompleteEvent { label: "On Ship Order Complete", category: Ships },
    /// Autonomous AI took the helm back from an installed order. TRANSIENT -
    /// the order stays installed and resumes, so a beat here must not retire
    /// what it is waiting for.
    OnShipOrderInterrupted => OnShipOrderInterruptedEvent { label: "On Ship Order Interrupted", category: Ships },
    /// An interrupted order got its helm back and picked its directive up
    /// where it left off.
    OnShipOrderResumed => OnShipOrderResumedEvent { label: "On Ship Order Resumed", category: Ships },
    /// A ship order was retired on purpose and for good - `ClearShipOrder`, or
    /// a replacement order taking the helm. Terminal; an order that had
    /// already completed or failed does not also cancel.
    OnShipOrderCanceled => OnShipOrderCanceledEvent { label: "On Ship Order Canceled", category: Ships },
    /// An accepted ship order became impossible to continue: the well an orbit
    /// needed went away, or the hull lost the computer or engines the maneuver
    /// runs on. Terminal. An order REFUSED at issue time never reaches this -
    /// that is a lint error, not a scenario event.
    OnShipOrderFailed => OnShipOrderFailedEvent { label: "On Ship Order Failed", category: Ships },
    /// A mining pulse carved ore off an asteroid (`id` = the asteroid, other
    /// = the mining ship; `item`, `count` = the ore and the units). One event
    /// per pulse that flips at least one corner; a barren rock fires nothing.
    OnMined => OnMinedEvent { label: "On Mined", category: Trade },
    /// A cargo intake took a canister into a hold (`id` = the ship; `canister`,
    /// `item`, `count`). One event per stack the canister carried.
    OnPickedUp => OnPickedUpEvent { label: "On Picked Up", category: Trade },
    /// An intake dropped a canister out of a hold (`id` = the ship; `canister`,
    /// `item`, `count`). Fires when the canister leaves the hull, not when the
    /// pane queued it. One event per stack.
    OnJettisoned => OnJettisonedEvent { label: "On Jettisoned", category: Trade },
    /// A Buy or Sell with a docked trader went through (`id` = the trader,
    /// other = the ship that traded; `trade`, `item`, `count`, `price_cr`).
    /// Once per confirmed trade.
    OnTraded => OnTradedEvent { label: "On Traded", category: Trade },
    /// A ship's credits balance moved (`id` = the ship; `previous_cr`,
    /// `current_cr`, `delta_cr`, `cause` = `Trade`, `Take`, `Rebuild`,
    /// `Swap`, `Add`, `ShipBuy`, `EscortHire`, `StationBuy`, `JobReward` or
    /// `Refine`).
    /// Once per ship per change, so a trade raises one for each party.
    OnCreditsChanged => OnCreditsChangedEvent { label: "On Credits Changed", category: Trade },
    /// A Ship pane repair spent hull plates (`id` = the ship; `section` = the
    /// pane code such as `HULL-3`, `item`, `count` = plates spent). Once per
    /// repair that spends at least one plate.
    OnRepaired => OnRepairedEvent { label: "On Repaired", category: Trade },
    /// A Ship pane rebuild at a docked station put a lost section back (`id`
    /// = the ship; `section` = the pane code such as `PDC-1`, `parts` = every
    /// part spent, `price_cr` = credits paid). Once per rebuild.
    OnRebuilt => OnRebuiltEvent { label: "On Rebuilt", category: Trade },
    /// A Ship pane swap at a docked station replaced a live section with a
    /// part of the same family and footprint (`id` = the ship; `section` =
    /// the pane code such as `PDC-1`, `from` and `to` = the section prototype
    /// ids, `parts` = every part spent, `price_cr` = credits paid). Once per
    /// swap.
    OnSwapped => OnSwappedEvent { label: "On Swapped", category: Trade },
    /// A Ship pane add at a docked station mounted a new section at a free
    /// socket (`id` = the ship; `section` = the new pane code such as
    /// `PDC-3`, `part` = the section prototype id, `at` and `point` = the
    /// pane code and socket id it was mounted on, `parts` = every part
    /// spent, `price_cr` = credits paid). Once per add.
    OnAdded => OnAddedEvent { label: "On Added", category: Trade },
    /// A Ship pane buy at a docked station replaced the ship's hull with one
    /// from the station's catalogue (`id` = the ship; `hull` = the catalogue
    /// name, `price_cr` = the new hull's price). Once per buy; a free board
    /// of a ship the player owns fires none.
    OnShipBought => OnShipBoughtEvent { label: "On Ship Bought", category: Trade },
    /// A Ship pane hire at a docked station added an escort to the player's
    /// fleet (`id` = the hiring ship; `escort` = the escort's scenario id,
    /// `hull` = the catalogue name, `price_cr` = the hire price). Once per
    /// hire.
    OnEscortHired => OnEscortHiredEvent { label: "On Escort Hired", category: Trade },
    /// The Jobs pane accepted a board offer (`id` = the job's id, `kind`,
    /// `station` = the poster, `reward_cr`, `post_key`). Once per accept; a refused
    /// accept fires none, and a Load fires none for a job it restores.
    /// Generated worlds only.
    OnJobAccepted => OnJobAcceptedEvent { label: "On Job Accepted", category: Trade },
    /// A held job's work in the world is finished and it waits for its
    /// hand-in (`id` = the job's id, `kind`, `station`, `reward_cr`, `post_key`).
    /// Once per job; never for a Procure, which is done by the goods at the
    /// hand-in. Generated worlds only.
    OnJobDone => OnJobDoneEvent { label: "On Job Done", category: Trade },
    /// The Jobs pane handed a job in at its posting station and the reward
    /// was paid (`id` = the job's id, `kind`, `station`, `reward_cr`, `post_key`).
    /// Once per hand-in; a refused hand-in fires none. Generated worlds only.
    OnJobHandedIn => OnJobHandedInEvent { label: "On Job Handed In", category: Trade },
    /// A held job ended with no hand-in (`id` = the job's id, `kind`,
    /// `station`, `reward_cr`, `post_key`; `cause` = `Lost` when the world took
    /// it, `Abandoned` for the pane's Drop). Once per job. Generated worlds
    /// only.
    OnJobDropped => OnJobDroppedEvent { label: "On Job Dropped", category: Trade },
    /// A civilization's stance toward the player moved one step
    /// (`civilization` = its id text, `from` and `to` = `Ally`, `Neutral` or
    /// `Enemy`). Once per step: an offence's, each neighbour's it reached, a
    /// merit's, the decay's and a paid fine's; a provocation fires none.
    /// Generated worlds only.
    OnStanceChanged => OnStanceChangedEvent { label: "On Stance Changed", category: World },
    /// The player crossed into another sector of a generated world (`sector`
    /// = the entered sector's id text, `x`, `y`, `z` = its indices, `from` =
    /// the sector left). Once per crossing; the sector a New Game or a Load
    /// starts in fires none. Generated worlds only.
    OnSectorEntered => OnSectorEnteredEvent { label: "On Sector Entered", category: World },
    /// A Ship pane buy at a docked station made the station the player's
    /// (`id` = the buying ship; `station` = the station's scenario id,
    /// `price_cr` = the price). Once per buy.
    OnStationBought => OnStationBoughtEvent { label: "On Station Bought", category: Trade },
    /// An Inventory pane strip took one section off the docked lootable partner
    /// (`id` = the ship that stripped; `hull` = the stripped hull's scenario
    /// id, `section` = its pane code such as `PDC-1`, `parts` = every part
    /// put in the hold). Once per section as a whole-hull strip takes it; the
    /// last one names the berth port.
    OnStripped => OnStrippedEvent { label: "On Stripped", category: Trade },
    /// A Store at a docked station moved items into its storage (`id` = the
    /// station, other = the ship that stored; `item`, `count`). Once per
    /// confirmed Store.
    OnStored => OnStoredEvent { label: "On Stored", category: Trade },
    /// A Withdraw at a docked station moved items out of its storage (`id` =
    /// the station, other = the ship that withdrew; `item`, `count`). Once
    /// per confirmed Withdraw.
    OnWithdrawn => OnWithdrawnEvent { label: "On Withdrawn", category: Trade },
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

    /// The lint reads a trigger's readable fields off a DEFAULT payload, so a
    /// field that serializes as nothing there (an `Option` left `None`) would
    /// be refused in an expression although the live event carries it.
    #[test]
    fn no_default_payload_hides_a_field_behind_a_null() {
        for trigger in EventConfig::ALL {
            match trigger.default_payload().data {
                Some(serde_json::Value::Object(fields)) => {
                    for (name, value) in &fields {
                        assert!(!value.is_null(), "{trigger:?} field '{name}'");
                    }
                }
                // A payload with no operands at all.
                Some(serde_json::Value::Null) => {}
                other => panic!("{trigger:?} payload is not a struct: {other:?}"),
            }
        }
        assert_eq!(
            EventConfig::OnTraded.payload_fields(),
            [
                "count",
                "id",
                "item",
                "other_id",
                "other_type_name",
                "price_cr",
                "trade"
            ]
        );
        assert!(
            !EventConfig::OnRebuilt
                .payload_fields()
                .contains(&"parts".to_string()),
            "a map is not a value an expression has"
        );
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
