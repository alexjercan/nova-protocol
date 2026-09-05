//! `Sequence`: an ordered list of steps whose cursor the ENGINE holds.
//!
//! A linear beat chain is the common shape in shipped content, and without
//! this it is spelled as sibling `OnUpdate` handlers coordinated by a counter
//! variable the author increments by hand. The cursor cannot live in the
//! action - `EventAction::action` takes `&self`, actions are `Arc`-shared, and
//! `EventHandlerIndex` stores CLONES of handlers - so it lives in
//! [`NovaEventWorld`] under an authored literal key, exactly as a timer does.

use std::sync::Arc;

use bevy::prelude::*;
use nova_events::prelude::*;

use crate::prelude::*;

/// What a step waits for before it runs.
///
/// Both gates may sit on one step, and a step with neither runs the moment it
/// becomes current. The semantics are WAIT, never SKIP: a step whose gate is
/// shut blocks the rest of the sequence rather than being passed over.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SequenceStepConfig {
    /// Seconds on the pause-frozen scenario clock from when this step became
    /// current. Replaces the `mark_clock` write plus the
    /// `GreaterThan(scenario_elapsed, gate)` filter that reads it.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub after: Option<f64>,
    /// The event this step waits for, with the filters that qualify it. Only
    /// events that arrive AFTER the step became current count.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub until: Option<SequenceGateConfig>,
    /// How long the step may wait before the run is declared stuck. Required
    /// on a step with an `until` gate (the lint says so): whether a gate can
    /// ever open is a runtime question, and a silent soft-lock is the worst
    /// thing a mod can ship.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub deadline: Option<f64>,
    /// What the step does once its gates open.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub actions: Vec<EventActionConfig>,
}

impl SequenceStepConfig {
    /// Whether this step runs in the SAME frame as the step before it.
    ///
    /// [`advance_scenario_sequences`] drains ready steps in a `while` loop at
    /// one timestamp, and handing a step back stamps `since` with that same
    /// timestamp - so a step that waits for nothing is already ready on the
    /// next turn of the loop. Only a real delay, or a gate that has to wait
    /// for an event, puts a frame between two steps.
    ///
    /// Read by [`ScenarioEventConfig::action_groups`](crate::prelude::ScenarioEventConfig::action_groups),
    /// which is what every "in the same frame" lint and pacing rule counts.
    pub fn runs_with_the_step_before(&self) -> bool {
        self.until.is_none() && self.after.is_none_or(|after| after <= 0.0)
    }
}

/// The event a step waits for: the same event + filter vocabulary a handler
/// uses, moved inside the step.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SequenceGateConfig {
    /// The event kind to wait for.
    pub name: EventConfig,
    /// Filters that must all pass for the event to open this step.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub filters: Vec<EventFilterConfig>,
}

/// Start a keyed sequence: an ordered beat chain the engine walks.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SequenceActionConfig {
    /// Scenario-local key the cursor is filed under. A literal, so the lint
    /// can pair a sequence with its gates and a save could carry the cursor.
    pub key: String,
    /// The steps, in order.
    pub steps: Vec<SequenceStepConfig>,
}

impl EventAction<NovaEventWorld> for SequenceActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        if self.key.trim().is_empty() {
            error!("SequenceActionConfig: sequence key must not be empty");
            return;
        }
        if self.steps.is_empty() {
            error!("SequenceActionConfig: sequence '{}' has no steps", self.key);
            return;
        }
        world.start_sequence(self.key.clone(), Arc::new(self.steps.clone()));
    }
}

/// The engine's own handler action behind a step's `until` gate.
///
/// Deliberately NOT an [`EventActionConfig`] arm: it is the wake mechanism, not
/// authored vocabulary, and the closed action vocabulary is what `content lint`
/// walks. The loader spawns one handler per gated step; the action is inert
/// unless the cursor is standing on that exact step, so a gate that fires early
/// or late does nothing.
#[derive(Clone, Debug)]
struct SequenceGateAction {
    /// The sequence whose cursor this gate belongs to.
    key: String,
    /// The step index this gate opens.
    step: usize,
}

impl EventAction<NovaEventWorld> for SequenceGateAction {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        world.open_sequence_gate(&self.key, self.step);
    }
}

/// The wake handlers for every gated step of every `Sequence` an action list
/// can start, nested sequences included.
///
/// The loader spawns these beside the scenario's own handlers, once, at load.
/// A gate handler is deliberately NOT `once`: its step may not be current yet,
/// so it has to survive every earlier firing of its event to still be there
/// when the cursor arrives.
pub fn sequence_gate_handlers(actions: &[EventActionConfig]) -> Vec<EventHandler<NovaEventWorld>> {
    let mut handlers = Vec::new();
    for action in actions {
        action.walk(&mut |action| {
            let EventActionConfig::Sequence(config) = action else {
                return;
            };
            for (index, step) in config.steps.iter().enumerate() {
                let Some(gate) = step.until.as_ref() else {
                    continue;
                };
                let mut handler = EventHandler::<NovaEventWorld>::from(gate.name);
                for filter in &gate.filters {
                    handler.add_filter(filter.clone());
                }
                handler.add_action(SequenceGateAction {
                    key: config.key.clone(),
                    step: index,
                });
                handlers.push(handler);
            }
        });
    }
    handlers
}

/// Walk every running sequence's cursor on the pause-frozen scenario clock.
///
/// Chained into the scenario pulse, so it is frozen under the pause menu and
/// held while queued spawns are still landing, like every other scenario tick.
/// A step's actions run with a DEFAULT `GameEventInfo`: no action reads the
/// event payload, and a step is a scenario beat rather than a reaction to one
/// entity.
pub fn advance_scenario_sequences(mut world: ResMut<NovaEventWorld>) {
    let now = world.scenario_elapsed();
    while let Some(actions) = world.take_ready_sequence_step(now) {
        for action in &actions {
            action.action(&mut world, &GameEventInfo::default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variables::{VariableFactorNode, VariableLiteral, VariableTermNode};

    /// A step whose only job is to say it ran: it writes its own ordinal into
    /// `at`, so the variable reads as the last beat the cursor delivered.
    fn beat(ordinal: f64) -> EventActionConfig {
        EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "at".to_string(),
            expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Number(ordinal)),
            )),
        })
    }

    fn timed(after: f64, ordinal: f64) -> SequenceStepConfig {
        SequenceStepConfig {
            after: Some(after),
            actions: vec![beat(ordinal)],
            ..default()
        }
    }

    fn gated(deadline: f64, ordinal: f64) -> SequenceStepConfig {
        SequenceStepConfig {
            until: Some(SequenceGateConfig {
                name: EventConfig::OnEnter,
                filters: vec![],
            }),
            deadline: Some(deadline),
            actions: vec![beat(ordinal)],
            ..default()
        }
    }

    /// Run whatever the cursor is ready to deliver at `now`, as the driver does.
    fn pump(world: &mut NovaEventWorld, now: f64) {
        while let Some(actions) = world.take_ready_sequence_step(now) {
            for action in &actions {
                action.action(world, &default());
            }
        }
    }

    fn at(world: &NovaEventWorld) -> Option<f64> {
        match world.get_variable("at") {
            Some(VariableLiteral::Number(n)) => Some(*n),
            _ => None,
        }
    }

    fn start(world: &mut NovaEventWorld, key: &str, steps: Vec<SequenceStepConfig>) {
        SequenceActionConfig {
            key: key.to_string(),
            steps,
        }
        .action(world, &default());
    }

    /// The whole point: each step's delay is measured from when THAT step
    /// became current, and nothing in the scenario holds the cursor.
    #[test]
    fn each_step_waits_its_own_delay_from_the_previous_one() {
        let mut world = NovaEventWorld::default();
        start(
            &mut world,
            "opening",
            vec![timed(2.0, 1.0), timed(3.0, 2.0)],
        );

        pump(&mut world, 1.9);
        assert_eq!(at(&world), None, "the first step has not come due");

        pump(&mut world, 2.0);
        assert_eq!(at(&world), Some(1.0));

        pump(&mut world, 4.9);
        assert_eq!(
            at(&world),
            Some(1.0),
            "step two counts from step one, not from the start"
        );

        pump(&mut world, 5.0);
        assert_eq!(at(&world), Some(2.0));
        assert_eq!(world.sequence_step("opening"), None, "the run is spent");
    }

    /// Zero-delay steps chain inside ONE frame; the driver does not spend a
    /// frame per beat.
    #[test]
    fn steps_with_no_delay_all_land_in_one_pass() {
        let mut world = NovaEventWorld::default();
        start(
            &mut world,
            "opening",
            vec![timed(0.0, 1.0), timed(0.0, 2.0), timed(0.0, 3.0)],
        );
        pump(&mut world, 0.0);
        assert_eq!(at(&world), Some(3.0));
    }

    /// A gated step BLOCKS: the steps behind it wait rather than running past.
    #[test]
    fn a_shut_gate_holds_the_steps_behind_it() {
        let mut world = NovaEventWorld::default();
        start(&mut world, "run", vec![gated(60.0, 1.0), timed(0.0, 2.0)]);

        pump(&mut world, 30.0);
        assert_eq!(at(&world), None, "the gate is shut");

        world.open_sequence_gate("run", 0);
        pump(&mut world, 30.0);
        assert_eq!(at(&world), Some(2.0), "the gate released the whole tail");
    }

    /// A gate that fires while the cursor is elsewhere must not arm a step the
    /// run has not reached - the ordering guarantee is the whole win.
    #[test]
    fn a_gate_for_another_step_does_not_arm_this_one() {
        let mut world = NovaEventWorld::default();
        start(&mut world, "run", vec![timed(5.0, 1.0), gated(60.0, 2.0)]);

        world.open_sequence_gate("run", 1);
        pump(&mut world, 5.0);
        assert_eq!(
            at(&world),
            Some(1.0),
            "the early gate did not carry over to step two"
        );
    }

    /// A gate that can never open is a soft-lock, which is the worst thing a
    /// mod can ship. The deadline turns it into a loud stop.
    #[test]
    fn a_step_past_its_deadline_stops_the_sequence() {
        let mut world = NovaEventWorld::default();
        start(&mut world, "run", vec![gated(10.0, 1.0), timed(0.0, 2.0)]);

        pump(&mut world, 9.0);
        assert_eq!(world.sequence_step("run"), Some(0));

        pump(&mut world, 11.0);
        world.open_sequence_gate("run", 0);
        pump(&mut world, 12.0);
        assert_eq!(at(&world), None, "the deadline stopped the run for good");
    }

    /// Restarting a live sequence would replay beats the player already heard.
    #[test]
    fn starting_a_running_sequence_again_is_refused() {
        let mut world = NovaEventWorld::default();
        start(&mut world, "opening", vec![timed(1.0, 1.0)]);
        start(&mut world, "opening", vec![timed(1.0, 9.0)]);

        pump(&mut world, 1.0);
        assert_eq!(at(&world), Some(1.0), "the first run is the one that plays");
    }

    /// Two sequences run side by side, each on its own cursor.
    #[test]
    fn separate_keys_hold_separate_cursors() {
        let mut world = NovaEventWorld::default();
        start(&mut world, "left", vec![timed(1.0, 1.0)]);
        start(&mut world, "right", vec![timed(5.0, 2.0)]);

        pump(&mut world, 1.0);
        assert_eq!(world.sequence_step("left"), None, "left finished");
        assert_eq!(world.sequence_step("right"), Some(0), "right is still due");
    }

    #[test]
    fn teardown_clears_the_cursor() {
        let mut world = NovaEventWorld::default();
        start(&mut world, "opening", vec![timed(1.0, 1.0)]);
        world.clear();
        pump(&mut world, 100.0);
        assert_eq!(at(&world), None);
    }

    /// One gate handler per gated step, and none for an ungated one.
    #[test]
    fn the_loader_gets_one_gate_handler_per_gated_step() {
        let actions = vec![EventActionConfig::Sequence(SequenceActionConfig {
            key: "run".to_string(),
            steps: vec![timed(1.0, 1.0), gated(60.0, 2.0), gated(60.0, 3.0)],
        })];
        assert_eq!(sequence_gate_handlers(&actions).len(), 2);
    }

    /// A sequence started from inside another sequence's step is reached by the
    /// walkers too - the gate would otherwise never be registered.
    #[test]
    fn a_nested_sequences_gates_are_registered_as_well() {
        let actions = vec![EventActionConfig::Sequence(SequenceActionConfig {
            key: "outer".to_string(),
            steps: vec![SequenceStepConfig {
                actions: vec![EventActionConfig::Sequence(SequenceActionConfig {
                    key: "inner".to_string(),
                    steps: vec![gated(60.0, 1.0)],
                })],
                ..default()
            }],
        })];
        assert_eq!(sequence_gate_handlers(&actions).len(), 1);
    }

    #[test]
    fn sequence_configs_round_trip_through_ron() {
        let action = EventActionConfig::Sequence(SequenceActionConfig {
            key: "opening".to_string(),
            steps: vec![
                timed(2.0, 1.0),
                SequenceStepConfig {
                    until: Some(SequenceGateConfig {
                        name: EventConfig::OnEnter,
                        filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                            id: Some("beacon_1".to_string()),
                            ..default()
                        })],
                    }),
                    deadline: Some(120.0),
                    ..default()
                },
            ],
        });
        let ron = ron::ser::to_string(&action).expect("serialize sequence");
        let parsed: EventActionConfig = ron::de::from_str(&ron).expect("parse sequence");
        let EventActionConfig::Sequence(config) = parsed else {
            panic!("not a sequence");
        };
        assert_eq!(config.key, "opening");
        assert_eq!(config.steps.len(), 2);
        assert_eq!(config.steps[0].after, Some(2.0));
        assert_eq!(config.steps[1].deadline, Some(120.0));
    }
}
