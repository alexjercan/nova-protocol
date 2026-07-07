use std::collections::VecDeque;

use bevy::{ecs::world::CommandQueue, platform::collections::HashMap, prelude::*};
use bevy_common_systems::prelude::EventWorld;
use nova_gameplay::prelude::*;

use crate::prelude::*;

#[derive(Resource, Default)]
pub struct NovaEventWorld {
    queued_commands: VecDeque<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
    objectives: Vec<ObjectiveActionConfig>,
    variables: HashMap<String, VariableLiteral>,
    pub next_scenario: Option<NextScenarioActionConfig>,
}

impl EventWorld for NovaEventWorld {
    fn world_to_state_system(_world: &mut World) {}

    fn state_to_world_system(world: &mut World) {
        // Copy the objectives to the bevy world
        let objectives = &world.resource::<Self>().objectives.clone();
        world.resource_mut::<GameObjectivesHud>().objectives.clear();
        world
            .resource_mut::<GameObjectivesHud>()
            .objectives
            .extend(objectives.iter().cloned());

        // Log variables
        debug!("# Current Variables:");
        for (key, value) in &world.resource::<Self>().variables {
            debug!("Variable: {} = {:?}", key, value);
        }

        // If a next scenario is queued (and not lingering), switch to it. `linger` keeps
        // the request pending without switching, so a scenario can stay on screen after a
        // NextScenario action until something clears the flag.
        let request = world.resource::<Self>().next_scenario.clone();
        if let Some(request) = request.filter(|r| !r.linger) {
            // Consume the request up front so the switch fires exactly once, rather than
            // relying on the subsequent LoadScenario/UnloadScenario to clear the world.
            world.resource_mut::<Self>().next_scenario = None;

            match world
                .resource::<GameScenarios>()
                .get(&request.scenario_id)
                .cloned()
            {
                Some(config) => {
                    debug!(
                        "state_to_world: switching to next scenario '{}'",
                        request.scenario_id
                    );
                    world.trigger(LoadScenario(config));
                }
                None => {
                    error!(
                        "state_to_world: next scenario id '{}' not found in GameScenarios; unloading",
                        request.scenario_id
                    );
                    world.trigger(UnloadScenario);
                }
            }
        }

        // Apply all the commands in the queue
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        if !event_world.queued_commands.is_empty() {
            let queued_commands = std::mem::take(&mut event_world.queued_commands);

            let mut queue = CommandQueue::default();
            let mut commands = Commands::new(&mut queue, world);

            for cmd in queued_commands.into_iter() {
                cmd(&mut commands);
            }

            queue.apply(world);
        }
    }
}

impl NovaEventWorld {
    pub fn clear(&mut self) {
        self.queued_commands.clear();
        self.objectives.clear();
        self.variables.clear();
        self.next_scenario = None;
    }

    pub fn push_command<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Commands) + Send + Sync + 'static,
    {
        self.queued_commands.push_back(Box::new(f));
    }

    pub fn push_objective(&mut self, objective: ObjectiveActionConfig) {
        if self.objectives.iter().any(|obj| obj.id == objective.id) {
            warn!(
                "push_objective: objective id '{}' is already active; the scenario is \
                 adding a duplicate",
                objective.id
            );
        }
        debug!("push_objective: added objective '{}'", objective.id);
        self.objectives.push(objective);
    }

    pub fn remove_objective(&mut self, id: &str) {
        let before = self.objectives.len();
        self.objectives.retain(|obj| obj.id != id);
        if self.objectives.len() == before {
            warn!(
                "remove_objective: no active objective with id '{}' to complete; check the \
                 scenario for a typo or a missing Objective action that should create it",
                id
            );
        } else {
            debug!("remove_objective: completed objective '{}'", id);
        }
    }

    pub fn insert_variable(&mut self, key: String, value: VariableLiteral) {
        self.variables.insert(key, value);
    }

    pub fn get_variable(&self, key: &str) -> Option<&VariableLiteral> {
        self.variables.get(key)
    }
}
