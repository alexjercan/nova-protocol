//! [`NovaEventWorld`]: the bridge an authored handler acts through.
//!
//! Handlers run outside the ECS, so they cannot touch `World` directly. They
//! queue intent here - commands, story lines, objectives, HUD readouts,
//! variables - and a pair of sync systems drains it into and out of the real
//! world each frame.
//!
//! Touch this module when adding a new thing an authored action can do.

/// Glob-import surface: `use nova_scenario::world::prelude::*` re-exports the
/// public API of this module.
pub mod prelude {
    pub use super::NovaEventWorld;
}

use std::collections::VecDeque;

use bevy::{ecs::world::CommandQueue, platform::collections::HashMap, prelude::*};
use nova_events::prelude::EventWorld;
use nova_gameplay::prelude::*;
use nova_hud::prelude::*;

use crate::prelude::*;

/// The event world for the live scenario: the game-specific [`EventWorld`]
/// carrying scenario state that scenario actions read and write - objectives,
/// story log, variables (including the reserved scenario clock), a deferred
/// command queue, and a queued next-scenario switch. Inserted as a resource by
/// the scenario stack and cleared at teardown; its `state_to_world_system`
/// mirrors this state into the bevy world (HUD, scenario switches) each frame.
#[derive(Resource, Default)]
pub struct NovaEventWorld {
    queued_commands: VecDeque<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
    objectives: Vec<ObjectiveActionConfig>,
    /// The scenario's story-message log, in delivery order. Append-only within
    /// a scenario; cleared at teardown with the rest of the event world.
    story_messages: Vec<StoryMessageActionConfig>,
    /// The scenario's active HUD readouts, in authored order. Upserted/cleared
    /// by slot via the `HudReadout` action; the sync copies each one's CURRENT
    /// bound-variable value into the HUD's [`HudReadouts`] resource every
    /// frame. Cleared at teardown with the rest of the event world.
    hud_readouts: Vec<HudReadoutActionConfig>,
    variables: HashMap<String, VariableLiteral>,
    /// Keyed timer deadlines on the pause-frozen scenario clock.
    timers: HashMap<String, f64>,
    /// Every position a `ScatterObjects` action has placed this scenario, in
    /// placement order. Separation is a property of the FIELD, not of one
    /// action: a belt is authored as sibling scatters whose regions abut, and a
    /// per-action set would let two of them put rocks inside each other - the
    /// exact overlap `min_separation` exists to prevent. Cleared at teardown
    /// with the rest of the event world.
    scatter_placements: Vec<Vec3>,
    /// The queued scenario switch, if a `NextScenario` action has requested one.
    pub next_scenario: Option<NextScenarioActionConfig>,
    /// The delayed non-lingering cut's clock: armed by the NextScenario action,
    /// ticked by `state_to_world_system` on the world's (pause-frozen) time;
    /// the switch executes at expiry.
    pub next_scenario_delay: Option<Timer>,
    /// Logging-only: the last variable snapshot we debug-logged. `state_to_world_system`
    /// runs every frame, so it logs the variables only when they DIFFER from this, to
    /// avoid per-frame spam.
    last_logged_variables: HashMap<String, VariableLiteral>,
}

impl EventWorld for NovaEventWorld {
    fn world_to_state_system(_world: &mut World) {
        // Nothing to carry from the bevy world into the event world right now.
        // The graphics-budget carry lived here (to thin scatter on the lower
        // tiers) but scatter is no longer a preset lever. Kept as a required
        // `EventWorld` method so the plumbing exists if a future action needs
        // live world state pulled in before the queue processes.
    }

    fn state_to_world_system(world: &mut World) {
        // Copy the objectives to the bevy world, mapping nova's scenario-action
        // config to the nova_gameplay Objective the HUD renders.
        // Write-on-diff, not a blind clear+extend: this system now runs every
        // frame (the OnUpdate pulse keeps the event queue warm), and an
        // unconditional write would flag GameObjectives changed every frame -
        // making the objectives panel despawn and respawn its text lines per
        // frame for the whole session.
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
                        dwell: m.dwell,
                        icon: m.icon.clone(),
                    })
                    .collect();
            }
        }

        // Copy the active HUD readouts into the HUD's HudReadouts resource
        // (nova_gameplay), each with its bound variable's CURRENT value read
        // off the event world THIS frame. Unlike the story log this is NOT
        // append-only: the value tracks a live variable (the scenario clock
        // ticks while the scenario is live), so the set is rebuilt each frame
        // the sync runs rather than write-on-diff. Under the outcome/ pause
        // freeze the clock stops advancing and the sync stops re-running, so
        // the last value latches and the row PERSISTS at the final time (this
        // is how "final time on Victory" holds). The render side updates its
        // rows' text in place, so the resource write costs no entity churn.
        // Guarded on the resource existing so event-world rigs without the HUD
        // half keep working. An empty set (teardown or all readouts cleared)
        // drops every row - the same leak pin as the comms feed.
        let readouts: Vec<(HudReadoutActionConfig, f64)> = {
            let this = world.resource::<Self>();
            this.hud_readouts
                .iter()
                .map(|readout| {
                    let value = match this.variables.get(&readout.variable) {
                        Some(VariableLiteral::Number(n)) => *n,
                        // An undefined or non-numeric variable reads as 0.0,
                        // the same fail-closed default as scenario_elapsed
                        // before its first tick - a readout shown before its
                        // variable exists prints a zero, not garbage.
                        _ => 0.0,
                    };
                    (readout.clone(), value)
                })
                .collect()
        };
        if let Some(mut hud_readouts) = world.get_resource_mut::<HudReadouts>() {
            let next: Vec<HudReadoutEntry> = readouts
                .into_iter()
                .map(|(readout, value)| HudReadoutEntry {
                    slot: readout.slot,
                    label: readout.label,
                    format: readout.format.into(),
                    value,
                })
                .collect();
            if hud_readouts.0 != next {
                hud_readouts.0 = next;
            }
        }

        // Log variables ONLY when they change since the last log - this system
        // runs every frame (the OnUpdate pulse keeps the event queue warm), so
        // an unconditional log spams the debug stream. Clone the snapshot only
        // on a change. The engine clock is EXCLUDED from the diff: it advances
        // every live frame by design, and letting it count as "changed" would
        // defeat this guard.
        let changed_snapshot = {
            let this = world.resource::<Self>();
            let differs_ignoring_clock =
                |a: &HashMap<String, VariableLiteral>, b: &HashMap<String, VariableLiteral>| {
                    let clock = crate::loader::SCENARIO_ELAPSED_VAR;
                    a.iter().any(|(k, v)| k != clock && b.get(k) != Some(v))
                        || b.keys().any(|k| k != clock && !a.contains_key(k))
                };
            if differs_ignoring_clock(&this.variables, &this.last_logged_variables) {
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
            // The delayed cut: while the delay runs, the world keeps playing -
            // tick on the world's virtual clock (a paused game holds the cut)
            // and only switch at expiry.
            let delta = world.resource::<Time>().delta();
            let still_waiting = {
                let mut event_world = world.resource_mut::<Self>();
                match event_world.next_scenario_delay.as_mut() {
                    Some(timer) => !timer.tick(delta).is_finished(),
                    None => false,
                }
            };
            // NOTE: no early return while waiting - the command-queue flush below
            // must keep running through the delay window, or every queued
            // spawn/effect starves until the cut.
            if !still_waiting {
                // Consume the request up front so the switch fires exactly once, rather than
                // relying on the subsequent LoadScenario/UnloadScenario to clear the world.
                world.resource_mut::<Self>().next_scenario = None;
                world.resource_mut::<Self>().next_scenario_delay = None;

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
        }

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
    /// Reset all scenario state (variables, timers, objectives, story log,
    /// queued commands, and any pending switch) at scenario teardown.
    pub fn clear(&mut self) {
        // Undrained commands die with the scenario. Legitimate on teardown, but
        // it is also how an `Outcome` composed with an INSTANT switch (`linger:
        // false`) gets swallowed before it can show - leave a trace for the
        // scenario author.
        if !self.queued_commands.is_empty() {
            debug!(
                "NovaEventWorld::clear: discarding {} undrained command(s) at teardown",
                self.queued_commands.len()
            );
        }
        self.queued_commands.clear();
        self.objectives.clear();
        self.story_messages.clear();
        self.hud_readouts.clear();
        self.variables.clear();
        self.timers.clear();
        self.scatter_placements.clear();
        self.next_scenario = None;
        self.next_scenario_delay = None;
    }

    /// Every position scattered so far this scenario, across ALL
    /// `ScatterObjects` actions. A scatter rejects a candidate that crowds any
    /// of them, so sibling fields with overlapping regions still spawn clear of
    /// each other.
    pub fn scatter_placements(&self) -> &[Vec3] {
        &self.scatter_placements
    }

    /// Record a position a scatter placed, so later scatters keep clear of it.
    pub fn push_scatter_placement(&mut self, position: Vec3) {
        self.scatter_placements.push(position);
    }

    /// Defer a closure that needs real Bevy world access; it runs at frame end
    /// when `state_to_world_system` drains the queue, in push order.
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

    /// Show, update, or clear a HUD readout by slot (see
    /// `HudReadoutActionConfig`). A `visible: true` config upserts the slot
    /// (keeping authored order for a fresh slot, replacing config in place for
    /// an existing one); a `visible: false` config removes it. The sync then
    /// mirrors the active set - with each readout's live variable value - into
    /// the HUD's `HudReadouts` resource.
    pub fn set_hud_readout(&mut self, config: HudReadoutActionConfig) {
        match self
            .hud_readouts
            .iter_mut()
            .find(|existing| existing.slot == config.slot)
        {
            Some(existing) if config.visible => *existing = config,
            Some(_) => self.hud_readouts.retain(|r| r.slot != config.slot),
            None if config.visible => self.hud_readouts.push(config),
            None => {
                debug!(
                    "set_hud_readout: clear of unknown slot '{}' ignored",
                    config.slot
                );
            }
        }
    }

    /// Add a HUD objective; warns if one with the same id is already active.
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

    /// Remove (complete) the HUD objective with the given id; warns if none is
    /// active under that id.
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
                // A release also skips any pending delayed cut: Enter during a
                // delay window jumps the beat, and a cross-handler overlay's
                // Continue is never a dead button (the frozen delay clock would
                // otherwise hold the switch under the pause forever).
                self.next_scenario_delay = None;
                true
            }
            None => false,
        }
    }

    /// Start or restart a timer. Returns false when the computed deadline is
    /// not finite.
    pub(crate) fn start_timer(&mut self, key: String, seconds: f64) -> bool {
        if key.trim().is_empty() || !seconds.is_finite() || seconds <= 0.0 {
            return false;
        }
        let now = match self.variables.get(crate::loader::SCENARIO_ELAPSED_VAR) {
            Some(VariableLiteral::Number(value)) => *value,
            _ => 0.0,
        };
        let deadline = now + seconds;
        if !deadline.is_finite() {
            return false;
        }
        self.timers.insert(key, deadline);
        true
    }

    /// Cancel a timer. A missing key is a no-op.
    pub(crate) fn cancel_timer(&mut self, key: &str) {
        self.timers.remove(key);
    }

    /// Remove and return keys whose deadline has passed, in deterministic key
    /// order. Removing before dispatch lets an end handler restart the key.
    pub(crate) fn drain_ended_timers(&mut self, now: f64) -> Vec<String> {
        let mut ended: Vec<String> = self
            .timers
            .iter()
            .filter(|(_, deadline)| now >= **deadline)
            .map(|(key, _)| key.clone())
            .collect();
        ended.sort_unstable();
        for key in &ended {
            self.timers.remove(key);
        }
        ended
    }

    /// True when a timer is currently running. Intended for diagnostics and
    /// tests; authored content observes completion through `OnTimerEnd`.
    pub fn timer_is_running(&self, key: &str) -> bool {
        self.timers.contains_key(key)
    }

    /// Set a scenario variable, overwriting any existing value under `key`.
    pub fn insert_variable(&mut self, key: String, value: VariableLiteral) {
        self.variables.insert(key, value);
    }

    /// Read a scenario variable by key, or `None` if it is unset.
    pub fn get_variable(&self, key: &str) -> Option<&VariableLiteral> {
        self.variables.get(key)
    }

    /// Read-only iteration over all scenario variables (unordered). For
    /// observers that snapshot the variable state - the run-timeline recorder
    /// (nova_probe) diffs successive snapshots to log changes - without
    /// exposing the map for mutation.
    pub fn variables(&self) -> impl Iterator<Item = (&String, &VariableLiteral)> {
        self.variables.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The delayed non-lingering cut: the switch holds for the authored delay
    /// while the world keeps running, then fires. The fail-first is the first
    /// assert - today's instant cut would have switched on the first update.
    #[test]
    fn a_delayed_cut_holds_then_switches() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        #[derive(Resource, Default)]
        struct Loads(usize);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.5,
        )));
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<nova_gameplay::prelude::GameObjectives>();
        app.init_resource::<Loads>();
        let mut scenarios = GameScenarios::default();
        scenarios.insert(
            "next_up".to_string(),
            crate::loader::ScenarioConfig {
                id: "next_up".to_string(),
                name: "Next".to_string(),
                description: String::new(),
                cubemap: "self://sky.png".into(),
                thumbnail: None,
                hidden: true,
                menu_backdrop: false,
                events: vec![],
            },
        );
        app.insert_resource(scenarios);
        app.add_observer(|_: On<LoadScenario>, mut loads: ResMut<Loads>| loads.0 += 1);
        app.add_systems(Update, NovaEventWorld::state_to_world_system);

        {
            let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
            let action = NextScenarioActionConfig {
                scenario_id: "next_up".to_string(),
                linger: false,
                delay: Some(2.0),
            };
            use nova_events::prelude::EventAction;
            action.action(&mut world, &default());
        }

        // ~1s in (0.25s measured/update): still holding.
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<Loads>().0,
            0,
            "the delayed cut must NOT switch inside its window (the \
             pre-change instant cut fails here)"
        );
        // Past 2s: the cut fires exactly once.
        for _ in 0..8 {
            app.update();
        }
        assert_eq!(app.world().resource::<Loads>().0, 1, "the cut fired once");
        assert!(
            app.world()
                .resource::<NovaEventWorld>()
                .next_scenario
                .is_none(),
            "the request was consumed"
        );
    }

    /// the command flush must keep RUNNING through the delay window (the tick
    /// must not early-return past it) - a command queued mid-window applies
    /// before the cut. Mutation-proven: an early return while waiting fails
    /// this test.
    #[test]
    fn the_command_flush_runs_through_the_delay_window() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        #[derive(Resource, Default)]
        struct Applied(bool);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.5,
        )));
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<nova_gameplay::prelude::GameObjectives>();
        app.init_resource::<Applied>();
        app.insert_resource(GameScenarios::default());
        app.add_systems(Update, NovaEventWorld::state_to_world_system);

        {
            let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
            use nova_events::prelude::EventAction;
            NextScenarioActionConfig {
                scenario_id: "never_loads".to_string(),
                linger: false,
                delay: Some(30.0),
            }
            .action(&mut world, &default());
            world.push_command(|commands| {
                commands.queue(|world: &mut World| {
                    world.resource_mut::<Applied>().0 = true;
                });
            });
        }
        app.update();
        assert!(
            app.world().resource::<Applied>().0,
            "a command queued mid-window must apply long before the cut \
             (an early return while waiting starves the flush)"
        );
    }

    /// an authored absurd delay must not panic Timer construction - the apply
    /// finite-checks and caps it.
    #[test]
    fn absurd_delays_are_capped_not_panics() {
        use nova_events::prelude::EventAction;
        let mut world = NovaEventWorld::default();
        NextScenarioActionConfig {
            scenario_id: "x".to_string(),
            linger: false,
            delay: Some(1e30),
        }
        .action(&mut world, &default());
        let timer = world.next_scenario_delay.as_ref().expect("armed, capped");
        assert!(timer.duration().as_secs_f32() <= 300.0);

        NextScenarioActionConfig {
            scenario_id: "x".to_string(),
            linger: false,
            delay: Some(f32::INFINITY),
        }
        .action(&mut world, &default());
        assert!(
            world.next_scenario_delay.is_none(),
            "non-finite delays arm nothing"
        );
    }

    /// releasing skips a pending delayed cut - Enter during the window jumps
    /// the beat instead of a silent no-op.
    #[test]
    fn release_skips_the_pending_delay() {
        use nova_events::prelude::EventAction;
        let mut world = NovaEventWorld::default();
        NextScenarioActionConfig {
            scenario_id: "x".to_string(),
            linger: false,
            delay: Some(30.0),
        }
        .action(&mut world, &default());
        assert!(world.next_scenario_delay.is_some());
        assert!(world.release_lingering_next());
        assert!(
            world.next_scenario_delay.is_none(),
            "release clears the delay so the next sync switches at once"
        );
    }

    /// clear() (teardown) drops a pending delayed cut with the request.
    #[test]
    fn clear_drops_the_pending_delayed_cut() {
        let mut world = NovaEventWorld::default();
        use nova_events::prelude::EventAction;
        NextScenarioActionConfig {
            scenario_id: "x".to_string(),
            linger: false,
            delay: Some(9.0),
        }
        .action(&mut world, &default());
        assert!(world.next_scenario_delay.is_some(), "armed");
        world.clear();
        assert!(world.next_scenario.is_none());
        assert!(
            world.next_scenario_delay.is_none(),
            "teardown drops the clock"
        );
    }

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
                dwell: None,
                icon: None,
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
                dwell: None,
                icon: None,
            });
        bare.update();
    }

    /// The authored per-line dwell rides the sync into the HUD line.
    #[test]
    fn story_sync_carries_the_authored_dwell() {
        let mut app = App::new();
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<StoryFeed>();
        app.add_systems(Update, NovaEventWorld::state_to_world_system);
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .push_story_message(StoryMessageActionConfig {
                speaker: "Okono".to_string(),
                text: "Read this slowly.".to_string(),
                dwell: Some(12.0),
                icon: None,
            });
        for _ in 0..5 {
            app.update();
        }
        let feed = app.world().resource::<StoryFeed>();
        assert_eq!(feed.0.len(), 1);
        assert_eq!(
            feed.0[0].dwell,
            Some(12.0),
            "the sync must carry the authored hold to the panel"
        );
    }

    /// The authored comms icon rides the sync into the HUD line, using the same
    /// `AssetRef<Image>` path object the scenario action parsed.
    #[test]
    fn story_sync_carries_the_authored_icon() {
        let mut app = App::new();
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<StoryFeed>();
        app.add_systems(Update, NovaEventWorld::state_to_world_system);
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .push_story_message(StoryMessageActionConfig {
                speaker: "Okono".to_string(),
                text: "Look at me.".to_string(),
                dwell: None,
                icon: Some(AssetRef::from("self://icons/okono.png")),
            });
        for _ in 0..5 {
            app.update();
        }
        let feed = app.world().resource::<StoryFeed>();
        assert_eq!(feed.0.len(), 1);
        assert_eq!(
            feed.0[0].icon.as_ref().and_then(|icon| icon.path()),
            Some("self://icons/okono.png"),
            "the sync must carry the authored icon to the panel"
        );
    }

    /// The objectives sync is write-on-diff: with the OnUpdate pulse keeping
    /// the event queue warm, state_to_world runs every frame, and a blind
    /// clear+extend would flag GameObjectives changed per frame - the
    /// objectives panel (gated on resource_changed) would tear down and rebuild
    /// its text lines every frame. Count actual change-detections across
    /// repeated syncs: one per real change, not one per run.
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
