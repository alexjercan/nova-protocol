//! The scenario state a world save carries, and how a resumed load puts it
//! back.
//!
//! A saved world names the scenario it was running and what that scenario had
//! done: the clock, the mutable variables, the objectives on the HUD, the
//! story log, the HUD readouts, the keyed timers still running, the `once`
//! handlers already spent and the objective marks on generated bodies. A Load
//! inserts it as [`ResumedScenario`]; the loader restores it before it fires
//! `OnResume` instead of `OnStart`.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use nova_events::prelude::{GeneratedBodyId, HandlerRetired};

use crate::prelude::*;

/// Glob-import surface: `use nova_scenario::resume::prelude::*` brings the
/// saved scenario state and the resources around it into scope.
pub mod prelude {
    pub use super::{ResumedScenario, SavedScenario, ScenarioHandlerIndex, SpentHandlers};
}

/// The scenario state one world save carries.
///
/// Sequences and cinematics are NOT carried: a save waits for a running one
/// to finish, so a save never holds a mid-beat script.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SavedScenario {
    /// The scenario that was running. A Load loads this id, and refuses a
    /// save whose id the loaded content lacks.
    pub scenario_id: ScenarioId,
    /// The pause-frozen scenario clock, in seconds.
    pub elapsed: f64,
    /// Every mutable scenario variable. Watched variables are derived from
    /// the world and are not saved.
    pub variables: BTreeMap<String, VariableLiteral>,
    /// The objectives on the HUD, in posting order.
    pub objectives: Vec<ObjectiveActionConfig>,
    /// The story log, in delivery order.
    pub story: Vec<NarrativeCueActionConfig>,
    /// The HUD readouts, in authored order.
    pub readouts: Vec<HudReadoutActionConfig>,
    /// Every running keyed timer and the seconds it had LEFT. A resumed load
    /// rebases each one on the restored clock.
    pub timers: BTreeMap<String, f64>,
    /// The `once` handlers that fired, as indexes into the scenario's
    /// `events`. A resumed load does not rebuild them. An `OnResume` handler
    /// is never listed: it fires once per load.
    pub spent_handlers: Vec<usize>,
    /// The objective marks on generated bodies, by body id.
    pub marks: Vec<(GeneratedBodyId, String)>,
}

impl SavedScenario {
    /// Capture what `scenario` has done so far.
    pub fn capture(
        scenario: &ScenarioConfig,
        world: &NovaEventWorld,
        spent: &SpentHandlers,
        marks: &GeneratedMarks,
    ) -> Self {
        Self {
            scenario_id: scenario.id.clone(),
            elapsed: world.scenario_elapsed(),
            variables: world
                .mutable_variables()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            objectives: world.objectives().to_vec(),
            story: world.story_messages().to_vec(),
            readouts: world.hud_readouts().to_vec(),
            timers: world.timers_remaining().collect(),
            spent_handlers: spent
                .0
                .iter()
                .copied()
                .filter(|index| {
                    scenario
                        .events
                        .get(*index)
                        .is_some_and(|event| event.name != EventConfig::OnResume)
                })
                .collect(),
            marks: marks
                .iter()
                .map(|(id, label)| (id.clone(), label.clone()))
                .collect(),
        }
    }
}

/// The saved scenario state a Load brought in, waiting for its scenario to
/// load. The loader takes it on the load whose id matches and restores it
/// before `OnResume` fires.
#[derive(Resource, Debug, Clone)]
pub struct ResumedScenario(pub SavedScenario);

/// The index of an authored handler in the loaded scenario's `events`, worn
/// by the handler entity so a retired `once` handler can be named in the
/// save. The engine's own gate handlers wear none.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenarioHandlerIndex(pub usize);

/// Which `once` handlers of the loaded scenario have fired, by index into
/// its `events`. Seeded from the save on a resumed load, so a handler skipped
/// at the rebuild is still listed on the next save. Cleared at teardown.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct SpentHandlers(pub BTreeSet<usize>);

/// Record a retired `once` handler. Runs before the engine's despawn applies,
/// so the index is still on the entity.
pub(crate) fn record_spent_handler(
    retired: On<HandlerRetired>,
    handlers: Query<&ScenarioHandlerIndex>,
    mut spent: ResMut<SpentHandlers>,
) {
    if let Ok(index) = handlers.get(retired.handler) {
        spent.0.insert(index.0);
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use nova_events::prelude::SectorCoord;

    use super::*;

    /// A saved scenario with every field set survives the RON the world save
    /// is written in: the typed body id, the literal variables and the timer
    /// remainders all read back as written.
    #[test]
    fn a_saved_scenario_round_trips_through_ron() {
        let saved = SavedScenario {
            scenario_id: "open_world".to_string(),
            elapsed: 61.5,
            variables: BTreeMap::from([
                ("count".to_string(), VariableLiteral::Number(3.0)),
                ("done".to_string(), VariableLiteral::Boolean(false)),
                ("tag".to_string(), VariableLiteral::String("a".to_string())),
            ]),
            objectives: vec![ObjectiveActionConfig::new("find", "Find the wreck")],
            story: vec![NarrativeCueActionConfig {
                speaker: "Control".to_string(),
                text: "Go.".to_string(),
                dwell: Some(4.0),
                icon: None,
                accent: Color::WHITE,
            }],
            readouts: vec![HudReadoutActionConfig::new("left", "count")],
            timers: BTreeMap::from([("patrol".to_string(), 12.25)]),
            spent_handlers: vec![0, 3],
            marks: vec![(
                GeneratedBodyId::new(SectorCoord::new(-1, 0, 2), "background_rock", 4),
                "WRECK".to_string(),
            )],
        };
        let text = ron::ser::to_string(&saved).unwrap();
        let back: SavedScenario = ron::from_str(&text).unwrap();
        assert_eq!(back, saved);
    }

    /// The capture lists a spent handler by its index, but never an
    /// `OnResume` handler: that one fires on every load, and listing it would
    /// skip the player spawn on the second resume.
    #[test]
    fn the_capture_never_lists_a_spent_on_resume_handler() {
        let handler = |name: EventConfig| ScenarioEventConfig {
            label: None,
            name,
            once: true,
            filters: vec![],
            actions: vec![],
        };
        let scenario = ScenarioConfig {
            events: vec![
                handler(EventConfig::OnStart),
                handler(EventConfig::OnResume),
                handler(EventConfig::OnTimerEnd),
            ],
            ..ScenarioConfig::new(
                "saving",
                "Saving",
                nova_gameplay::prelude::AssetRef::default(),
            )
        };
        let spent = SpentHandlers(BTreeSet::from([0, 1, 2]));
        let saved = SavedScenario::capture(
            &scenario,
            &NovaEventWorld::default(),
            &spent,
            &GeneratedMarks::default(),
        );
        assert_eq!(saved.spent_handlers, vec![0, 2]);
    }
}
