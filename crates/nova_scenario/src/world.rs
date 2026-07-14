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
    /// Logging-only: the last variable snapshot we debug-logged. `state_to_world_system`
    /// runs every frame, so it logs the variables only when they DIFFER from this, to
    /// avoid per-frame spam.
    last_logged_variables: HashMap<String, VariableLiteral>,
}

impl EventWorld for NovaEventWorld {
    fn world_to_state_system(_world: &mut World) {}

    fn state_to_world_system(world: &mut World) {
        // Copy the objectives to the bevy world, mapping nova's scenario-action config to the
        // generic bevy_common_systems Objective the HUD renders. Write-on-diff, not a blind
        // clear+extend: this system now runs every frame (the OnUpdate pulse keeps the event
        // queue warm), and an unconditional write would flag GameObjectives changed every
        // frame - making the objectives panel despawn and respawn its text lines per frame
        // for the whole session (review R2.1 of task 20260711-180506).
        let objectives = world.resource::<Self>().objectives.clone();
        let differs = {
            let current = &world.resource::<GameObjectives>().objectives;
            current.len() != objectives.len()
                || current
                    .iter()
                    .zip(objectives.iter())
                    .any(|(have, want)| have.id != want.id || have.message != want.message)
        };
        if differs {
            world.resource_mut::<GameObjectives>().objectives = objectives
                .iter()
                .map(|objective| Objective::new(&objective.id, &objective.message))
                .collect();
        }

        // Log variables ONLY when they change since the last log - this system runs
        // every frame (the OnUpdate pulse keeps the event queue warm), so an
        // unconditional log spams the debug stream. Clone the snapshot only on a change.
        let changed_snapshot = {
            let this = world.resource::<Self>();
            if this.variables != this.last_logged_variables {
                debug!("# Current Variables:");
                for (key, value) in &this.variables {
                    debug!("Variable: {} = {:?}", key, value);
                }
                Some(this.variables.clone())
            } else {
                None
            }
        };
        if let Some(snapshot) = changed_snapshot {
            world.resource_mut::<Self>().last_logged_variables = snapshot;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The objectives sync is write-on-diff (review R2.1 of task
    /// 20260711-180506): with the OnUpdate pulse keeping the event queue
    /// warm, state_to_world runs every frame, and a blind clear+extend
    /// would flag GameObjectives changed per frame - the objectives panel
    /// (gated on resource_changed) would tear down and rebuild its text
    /// lines every frame. Count actual change-detections across repeated
    /// syncs: one per real change, not one per run.
    #[test]
    fn unchanged_objectives_do_not_flag_the_resource() {
        #[derive(Resource, Default)]
        struct Rebuilds(usize);

        let mut app = App::new();
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<Rebuilds>();
        app.add_systems(
            Update,
            (
                NovaEventWorld::state_to_world_system,
                (|mut rebuilds: ResMut<Rebuilds>| rebuilds.0 += 1)
                    .run_if(resource_changed::<GameObjectives>),
            )
                .chain(),
        );

        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .push_objective(ObjectiveActionConfig::new("b1", "Burn for Beacon 1"));

        for _ in 0..5 {
            app.update();
        }
        // One rebuild for the initial resource insert/change, not five.
        // (resource_changed also fires on the very first frame after
        // init, so allow the init tick plus the real change.)
        let after_first = app.world().resource::<Rebuilds>().0;
        assert!(
            after_first <= 2,
            "unchanged objectives must not re-flag the resource, got {} rebuilds",
            after_first
        );

        // A REAL change still lands exactly once more.
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .remove_objective("b1");
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .push_objective(ObjectiveActionConfig::new("b1", "Recovered: 1/3"));
        for _ in 0..5 {
            app.update();
        }
        let after_second = app.world().resource::<Rebuilds>().0;
        assert_eq!(
            after_second,
            after_first + 1,
            "a real objective change lands exactly one rebuild"
        );
    }
}
