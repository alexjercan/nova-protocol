use std::collections::VecDeque;

use bevy::{ecs::world::CommandQueue, platform::collections::HashMap, prelude::*};
use bevy_common_systems::prelude::EventWorld;
use nova_gameplay::prelude::*;

use crate::prelude::*;

#[derive(Resource, Default)]
pub struct NovaEventWorld {
    queued_commands: VecDeque<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
    objectives: Vec<ObjectiveActionConfig>,
    /// The scenario's story-message log, in delivery order (task
    /// 20260716-183220). Append-only within a scenario; cleared at teardown
    /// with the rest of the event world.
    story_messages: Vec<StoryMessageActionConfig>,
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

        // Copy the story log to the HUD's StoryFeed (nova_gameplay), the same
        // write-on-diff discipline as the objectives above. Length compare is
        // sufficient: the log is append-only within a scenario and emptied at
        // teardown. Guarded on the resource existing so event-world rigs
        // without the HUD half (unit tests, headless tools) keep working.
        let story = world.resource::<Self>().story_messages.clone();
        if let Some(mut feed) = world.get_resource_mut::<StoryFeed>() {
            if feed.0.len() != story.len() {
                feed.0 = story
                    .iter()
                    .map(|m| StoryLine {
                        speaker: m.speaker.clone(),
                        text: m.text.clone(),
                    })
                    .collect();
            }
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
        // Undrained commands die with the scenario. Legitimate on teardown,
        // but it is also how an `Outcome` composed with an INSTANT switch
        // (`linger: false`) gets swallowed before it can show - leave a
        // trace for the scenario author (outcome review R1.2).
        if !self.queued_commands.is_empty() {
            debug!(
                "NovaEventWorld::clear: discarding {} undrained command(s) at teardown",
                self.queued_commands.len()
            );
        }
        self.queued_commands.clear();
        self.objectives.clear();
        self.story_messages.clear();
        self.variables.clear();
        self.next_scenario = None;
    }

    pub fn push_command<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Commands) + Send + Sync + 'static,
    {
        self.queued_commands.push_back(Box::new(f));
    }

    /// Append a story line for the comms panel (see `StoryMessageActionConfig`).
    pub fn push_story_message(&mut self, message: StoryMessageActionConfig) {
        self.story_messages.push(message);
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

    /// Release a lingering `NextScenario` request so the switch fires on the
    /// next state sync. Returns false when nothing is queued. The one
    /// mechanism behind both the scenario-advance input (Enter/DPadDown) and
    /// the outcome overlay's Continue/Retry button.
    pub fn release_lingering_next(&mut self) -> bool {
        match self.next_scenario.as_mut() {
            Some(request) => {
                request.linger = false;
                true
            }
            None => false,
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

    /// The story log syncs into the HUD's StoryFeed with the same
    /// write-on-diff discipline as objectives, clears with the event world
    /// (the teardown reset class), and the sync is a no-op - not a panic -
    /// when the rig has no StoryFeed at all (headless event-world rigs).
    #[test]
    fn story_messages_sync_clear_and_tolerate_a_missing_feed() {
        #[derive(Resource, Default)]
        struct FeedChanges(usize);

        let mut app = App::new();
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<StoryFeed>();
        app.init_resource::<FeedChanges>();
        app.add_systems(
            Update,
            (
                NovaEventWorld::state_to_world_system,
                (|mut changes: ResMut<FeedChanges>| changes.0 += 1)
                    .run_if(resource_changed::<StoryFeed>),
            )
                .chain(),
        );

        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .push_story_message(StoryMessageActionConfig {
                speaker: "Okono".to_string(),
                text: "Strip it clean.".to_string(),
            });
        for _ in 0..5 {
            app.update();
        }
        {
            let feed = app.world().resource::<StoryFeed>();
            assert_eq!(feed.0.len(), 1, "the pushed line synced into the feed");
            assert_eq!(feed.0[0].speaker, "Okono");
            assert_eq!(feed.0[0].text, "Strip it clean.");
        }
        let after_first = app.world().resource::<FeedChanges>().0;
        assert!(
            after_first <= 2,
            "an unchanged log must not re-flag the feed, got {after_first} changes"
        );

        // Teardown: the event-world clear empties the feed on the next sync.
        app.world_mut().resource_mut::<NovaEventWorld>().clear();
        app.update();
        assert!(
            app.world().resource::<StoryFeed>().0.is_empty(),
            "clearing the event world must empty the feed (no leaked lines)"
        );

        // A rig WITHOUT the feed: the sync must skip, not panic.
        let mut bare = App::new();
        bare.init_resource::<NovaEventWorld>();
        bare.init_resource::<GameObjectives>();
        bare.add_systems(Update, NovaEventWorld::state_to_world_system);
        bare.world_mut()
            .resource_mut::<NovaEventWorld>()
            .push_story_message(StoryMessageActionConfig {
                speaker: "Okono".to_string(),
                text: "No HUD here.".to_string(),
            });
        bare.update();
    }

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
