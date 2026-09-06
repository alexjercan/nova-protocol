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
    pub use super::{scenario_has_settled, NovaEventWorld};
}

use core::time::Duration;
use std::{collections::VecDeque, sync::Arc};

// Bevy's platform Instant, not std's - `std::time::Instant::now` panics
// on wasm32-unknown-unknown, which this crate ships to.
use bevy::{
    ecs::world::CommandQueue,
    platform::collections::{HashMap, HashSet},
    platform::time::Instant,
    prelude::*,
};
use nova_events::{prelude::EventWorld, units::prelude::*};
use nova_gameplay::prelude::*;
use nova_hud::prelude::*;

use crate::{loader::WakeProfile, prelude::*};

/// How long ONE frame may spend applying queued scenario commands.
///
/// A shipped chapter's `OnStart` queues every object it spawns, and applying
/// that whole queue in one go cost a ~300 ms frame - a frame nothing can be
/// drawn on, so the loading panel froze on the exact frames it exists to cover.
/// The drain is chunked under this wall-clock budget instead: a slower machine
/// takes MORE FRAMES, and a frame never gives the drain more than a fifth of
/// itself.
///
/// A time budget rather than a command count because the commands are wildly
/// uneven - one clad ship is worth hundreds of rocks. One command is ALWAYS
/// applied per run, so an object costing more than the whole budget still lands
/// (overrunning by its own cost) instead of deadlocking the drain.
///
/// 3 ms is about a fifth of a 60 Hz frame, which leaves the rest of the
/// schedule its budget while still emptying a chapter's queue in a handful of
/// frames. That fifth is the rule, and this is its floor: a frame that is
/// already slow gets the same share of ITSELF (see [`SPAWN_DRAIN_FRAME_SHARE`]),
/// or a machine drawing a frame a second would land one rock per second and
/// keep a chapter's ships waiting a wall-clock minute behind its scatter.
const SPAWN_DRAIN_BUDGET: Duration = Duration::from_millis(3);

/// The fraction of the previous frame's wall time the drain may take when
/// that is more than [`SPAWN_DRAIN_BUDGET`].
const SPAWN_DRAIN_FRAME_SHARE: u32 = 5;

/// Run condition: the live scenario has finished spawning AND its art is in
/// memory.
///
/// The scenario SCRIPT is gated on this - the clock, the `OnUpdate` pulse, the
/// keyed timers and the event dispatch all stand down while queued spawns are
/// still landing. An `OnUpdate` predicate that counts objects therefore never
/// reads a half-built world: the world is not yet LIVE, rather than briefly
/// inconsistent.
///
/// [`ScenarioPreload`] extends that to the scenario's glTF, so no mission time
/// passes behind a loading panel that is still up. Optional, because a rig can
/// register the clock without the loader plugin that owns the resource; a rig
/// with no warm-up settles on the spawn queue alone.
pub fn scenario_has_settled(
    world: Res<NovaEventWorld>,
    preload: Option<Res<ScenarioPreload>>,
) -> bool {
    !world.is_settling() && !preload.is_some_and(|preload| preload.is_pending())
}

/// One running [`SequenceActionConfig`]: the engine's cursor into an authored
/// step list.
///
/// The cursor CANNOT live in the action - `EventAction::action` takes `&self`,
/// actions are `Arc`-shared, and the handler index stores clones - so it lives
/// here under the authored literal key, exactly as a timer deadline does.
struct SequenceRun {
    /// The authored literal the sequence and its gates are filed under.
    key: String,
    /// The authored steps. Shared rather than owned so restarting or cloning
    /// the config does not copy the whole beat chain.
    steps: Arc<Vec<SequenceStepConfig>>,
    /// The step waiting to run. `steps.len()` once the run is finished.
    step: usize,
    /// Scenario clock when `step` became current - what `after` and `deadline`
    /// are both measured from.
    since: f64,
    /// Whether this step's `until` gate has fired since the step became
    /// current. Cleared on every advance, so an early gate does not carry.
    gate_open: bool,
    /// Set when a deadline expired. A stopped run never advances again and
    /// keeps its key, so the failure cannot be papered over by a restart.
    stopped: bool,
    /// Set when this run is a SCENE rather than scenario logic: how it may be
    /// left, and whether it has reported its ending yet. `None` for a plain
    /// `Sequence`.
    cinematic: Option<CinematicRun>,
}

/// The part of a running [`SequenceRun`] that makes it a cinematic.
struct CinematicRun {
    /// Whether the skip binding may end this scene.
    skippable: bool,
    /// Scenario clock when the scene started, for the skip hold-off.
    armed_at: f64,
    /// Whether the ending has been reported. A scene reports exactly one, so
    /// a cancel that races the last beat cannot fire the events twice.
    ended: bool,
}

/// A cinematic that ended this frame, waiting for the driver to announce it.
pub(crate) struct CinematicEnding {
    /// The scene's authored key.
    pub key: String,
    /// Whether the player asked to leave. A skip announces itself BEFORE the
    /// finish, so the catch-up handler runs before the restore.
    pub skipped: bool,
}

impl SequenceRun {
    /// Whether the skip binding may end this run right now.
    fn is_skippable_at(&self, now: f64) -> bool {
        !self.stopped
            && self.step < self.steps.len()
            && self.cinematic.as_ref().is_some_and(|scene| {
                scene.skippable
                    && !scene.ended
                    && now - scene.armed_at >= CINEMATIC_SKIP_ARM_SECONDS
            })
    }

    /// Claim this run's one ending, or `None` if it is not a scene or has
    /// already reported.
    fn end_cinematic(&mut self, skipped: bool) -> Option<CinematicEnding> {
        let scene = self.cinematic.as_mut()?;
        if scene.ended {
            return None;
        }
        scene.ended = true;
        Some(CinematicEnding {
            key: self.key.clone(),
            skipped,
        })
    }
}

/// What a stuck step was waiting for, for the deadline log line.
fn describe_step_gate(step: &SequenceStepConfig) -> String {
    match (&step.until, step.after) {
        (Some(gate), _) => format!("event {:?}", gate.name),
        (None, Some(after)) => format!("its {after}s delay"),
        (None, None) => "nothing".to_string(),
    }
}

/// The event world for the live scenario: the game-specific [`EventWorld`]
/// carrying scenario state that scenario actions read and write - objectives,
/// story log, mutable and watched variables, typed query snapshots, a deferred
/// command queue, and a queued next-scenario switch. Inserted as a resource by
/// the scenario stack and cleared at teardown; its `state_to_world_system`
/// mirrors this state into the bevy world (HUD, scenario switches) each frame.
#[derive(Resource, Default)]
pub struct NovaEventWorld {
    queued_commands: VecDeque<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
    objectives: Vec<ObjectiveActionConfig>,
    /// The scenario's story-message log, in delivery order. Append-only within
    /// a scenario; cleared at teardown with the rest of the event world.
    story_messages: Vec<NarrativeCueActionConfig>,
    /// The scenario's active HUD readouts, in authored order. Upserted/cleared
    /// by slot via the `HudReadout` action; the sync copies each one's CURRENT
    /// bound-variable value into the HUD's [`HudReadouts`] resource every
    /// frame. Cleared at teardown with the rest of the event world.
    hud_readouts: Vec<HudReadoutActionConfig>,
    variables: HashMap<String, VariableLiteral>,
    watched_values: HashMap<String, VariableLiteral>,
    watches: Vec<WatchConfig>,
    /// Whether anything in the loaded scenario can read an entity query, so the
    /// per-frame sampler that feeds them is worth running at all. Set from
    /// `ScenarioConfig::reads_an_entity_query` at load; false at teardown.
    reads_entity_queries: bool,
    query_values: HashMap<QueryConfig, VariableLiteral>,
    scenario_elapsed: f64,
    /// Keyed timer deadlines on the pause-frozen scenario clock.
    timers: HashMap<String, f64>,
    /// Every `Sequence` the scenario has started, in start order. A `Vec`
    /// rather than a map because the driver walks them, and a walk over a hash
    /// map is not deterministic - two sequences whose beats land on one frame
    /// must land in the same order on every run.
    sequences: Vec<SequenceRun>,
    /// Cinematic endings the driver has not announced yet. Held rather than
    /// fired on the spot because the world has no `Commands`, and because a
    /// scene can end from three places (the last beat, the skip binding, a
    /// cancel action) that must all announce it the same way.
    cinematic_endings: Vec<CinematicEnding>,
    /// The title card on screen, and the scenario time it was posted. One at a
    /// time: a second card replaces the first rather than stacking, because two
    /// cards are two answers to "where am I". Dropped when its hold runs out,
    /// so no handler has to take it down.
    cinematic_title: Option<(CinematicTitleActionConfig, f64)>,
    /// Every position a `ScatterObjects` action has placed this scenario, in
    /// placement order. Separation is a property of the FIELD, not of one
    /// action: a belt is authored as sibling scatters whose regions abut, and a
    /// per-action set would let two of them put rocks inside each other - the
    /// exact overlap `min_separation` exists to prevent. Cleared at teardown
    /// with the rest of the event world.
    scatter_placements: Vec<Meters3>,
    /// The queued scenario switch, if a `NextScenario` action has requested one.
    pub next_scenario: Option<NextScenarioActionConfig>,
    /// The delayed non-lingering cut's clock: armed by the NextScenario action,
    /// ticked by `state_to_world_system` on the world's (pause-frozen) time;
    /// the switch executes at expiry.
    pub next_scenario_delay: Option<Timer>,
    /// What the loaded scenario needs to be woken FOR, derived from its
    /// `OnUpdate` filters at load. See `loader::wake`.
    wake: WakeProfile,
    /// Variable names written since the last [`Self::take_wake`]. The pulse
    /// reads it to decide whether an `OnUpdate` event is worth queueing at all.
    dirty: HashSet<String>,
    /// The scenario clock at the last [`Self::take_wake`], so a scheduled wake
    /// time is tested as a CROSSING rather than a level - a level test would
    /// fire the pulse every frame after the threshold instead of once.
    wake_checked_at: f64,
    /// Logging-only: the last variable snapshot we debug-logged. `state_to_world_system`
    /// runs every frame, so it logs the variables only when they DIFFER from this, to
    /// avoid per-frame spam.
    last_logged_variables: HashMap<String, VariableLiteral>,
}

impl EventWorld for NovaEventWorld {
    fn world_to_state_system(_world: &mut World) {
        // Nothing to carry from the bevy world into the event world: scatter is
        // not a preset lever, and nothing else needs live world state. Kept as a
        // required `EventWorld` method so the plumbing exists for an action that
        // does.
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
        // Resolve each cue's channel id against the merged catalog HERE, once,
        // so the panel is handed a drawn channel and never a lookup of its own.
        let channels = world
            .get_resource::<GameChannels>()
            .cloned()
            .unwrap_or_default();
        if let Some(mut feed) = world.get_resource_mut::<StoryFeed>() {
            if feed.0.len() != story.len() {
                feed.0 = story
                    .iter()
                    .map(|m| StoryLine {
                        speaker: m.speaker.clone(),
                        text: m.text.clone(),
                        dwell: m.dwell,
                        icon: m.icon.clone(),
                        channel: channels.resolve(&m.channel),
                    })
                    .collect();
            }
        }

        // Tell the HUD whether a scene the player may leave is playing, and
        // which action leaves it. Write-on-diff for the same reason the
        // objectives are: this runs every frame.
        let skippable = world
            .resource::<Self>()
            .skippable_cinematic()
            .map(|_| CINEMATIC_SKIP_ACTION.to_string());
        if let Some(mut prompt) = world.get_resource_mut::<CinematicPrompt>() {
            if prompt.skip_action != skippable {
                prompt.skip_action = skippable;
            }
        }

        // Hand the HUD the live title card, with its age on the scenario's own
        // pause-frozen clock: the card holds through a pause rather than
        // bleeding away behind the menu. Write-on-diff like the objectives, but
        // the age changes every frame, so in practice this writes while a card
        // is up and stops when it expires.
        let card = world
            .resource_mut::<Self>()
            .cinematic_title()
            .map(|(config, age)| TitleCard {
                corner: config.corner.into(),
                location: config.location.clone(),
                date: config.date.clone(),
                note: config.note.clone(),
                age,
                seconds: config.seconds,
            });
        if let Some(mut title) = world.get_resource_mut::<CinematicTitle>() {
            if title.card != card {
                title.card = card;
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
                    let value = match this.get_variable(&readout.variable) {
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
            if this.variables != this.last_logged_variables {
                // ONE line naming what actually moved, because that is the
                // question a reader watching a scenario has. Dumping the whole
                // table said "something changed" in N lines and made the reader
                // diff them by eye; the full table stays one trace away.
                let mut changed: Vec<String> = this
                    .variables
                    .iter()
                    .filter(|(key, value)| this.last_logged_variables.get(*key) != Some(*value))
                    .map(|(key, value)| format!("{key}={value:?}"))
                    .collect();
                // Teardown CLEARS the table, so a diff that only looked at what
                // is present now would report "0 changed" on the one transition
                // that dropped everything.
                changed.extend(
                    this.last_logged_variables
                        .keys()
                        .filter(|key| !this.variables.contains_key(*key))
                        .map(|key| format!("{key}=<removed>")),
                );
                // HashMap order is arbitrary, so an unsorted line would reorder
                // between two otherwise identical runs and defeat a log diff.
                changed.sort();
                debug!(
                    "scenario variables: {} changed of {} live ({})",
                    changed.len(),
                    this.variables.len(),
                    changed.join(", ")
                );
                for (key, value) in &this.variables {
                    trace!("Variable: {} = {:?}", key, value);
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
            // No early return while waiting - the command-queue flush below
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

        // CHUNKED, not drained (see [`SPAWN_DRAIN_BUDGET`]). One command is
        // popped and applied at a time, which is what keeps each object
        // ATOMIC: a ship's sections all land inside one `apply`, so the
        // `Added<SectionLinkPoints>` batch the integrity graph and the derived
        // skin key off is still complete the first time they see it.
        let last_frame = world
            .get_resource::<Time<Real>>()
            .map(Time::delta)
            .unwrap_or_default();
        let budget = SPAWN_DRAIN_BUDGET.max(last_frame / SPAWN_DRAIN_FRAME_SHARE);
        let started = Instant::now();
        while let Some(command) = world
            .resource_mut::<NovaEventWorld>()
            .queued_commands
            .pop_front()
        {
            let mut queue = CommandQueue::default();
            let mut commands = Commands::new(&mut queue, world);
            command(&mut commands);
            queue.apply(world);

            if started.elapsed() >= budget {
                break;
            }
        }
    }

    fn is_settling(&self) -> bool {
        !self.queued_commands.is_empty()
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
        self.watched_values.clear();
        self.watches.clear();
        self.wake = WakeProfile::EveryFrame;
        self.dirty.clear();
        self.wake_checked_at = 0.0;
        self.reads_entity_queries = false;
        self.query_values.clear();
        self.scenario_elapsed = 0.0;
        self.timers.clear();
        self.sequences.clear();
        self.cinematic_endings.clear();
        self.cinematic_title = None;
        self.scatter_placements.clear();
        self.next_scenario = None;
        self.next_scenario_delay = None;
    }

    /// Every position scattered so far this scenario, across ALL
    /// `ScatterObjects` actions. A scatter rejects a candidate that crowds any
    /// of them, so sibling fields with overlapping regions still spawn clear of
    /// each other.
    pub fn scatter_placements(&self) -> &[Meters3] {
        &self.scatter_placements
    }

    /// Record a position a scatter placed, so later scatters keep clear of it.
    pub fn push_scatter_placement(&mut self, position: Meters3) {
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

    /// Append a narrative cue for the comms panel (see
    /// `NarrativeCueActionConfig`).
    pub fn push_narrative_cue(&mut self, message: NarrativeCueActionConfig) {
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
        let deadline = self.scenario_elapsed + seconds;
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

    /// Start a keyed sequence at its first step.
    ///
    /// Restarting a LIVE key is refused: the cursor is the sequence's only
    /// state, so a restart would replay beats the player has already seen. A
    /// finished run is pruned, so the same key may be started again later.
    pub(crate) fn start_sequence(&mut self, key: String, steps: Arc<Vec<SequenceStepConfig>>) {
        self.start_run(key, steps, None);
    }

    /// Start a keyed cinematic: the same cursor, plus the ending it owes.
    pub(crate) fn start_cinematic(
        &mut self,
        key: String,
        steps: Arc<Vec<SequenceStepConfig>>,
        skippable: bool,
    ) {
        let armed_at = self.scenario_elapsed;
        self.start_run(
            key,
            steps,
            Some(CinematicRun {
                skippable,
                armed_at,
                ended: false,
            }),
        );
    }

    fn start_run(
        &mut self,
        key: String,
        steps: Arc<Vec<SequenceStepConfig>>,
        cinematic: Option<CinematicRun>,
    ) {
        self.prune_finished_sequences();
        if self.sequences.iter().any(|run| run.key == key) {
            error!("sequence '{key}' is already running; ignoring the restart");
            return;
        }
        self.sequences.push(SequenceRun {
            key,
            steps,
            step: 0,
            since: self.scenario_elapsed,
            gate_open: false,
            stopped: false,
            cinematic,
        });
    }

    /// End the live skippable scene, as the player asked. `false` when no
    /// scene is asking to be left - a stray press, or one inside the hold-off.
    pub(crate) fn skip_cinematic(&mut self) -> bool {
        let now = self.scenario_elapsed;
        let Some(run) = self
            .sequences
            .iter_mut()
            .find(|run| run.is_skippable_at(now))
        else {
            return false;
        };
        run.step = run.steps.len();
        let ending = run.end_cinematic(true);
        let skipped = ending.is_some();
        self.cinematic_endings.extend(ending);
        skipped
    }

    /// End a named scene from the scenario. Announces a finish, never a skip.
    pub(crate) fn cancel_cinematic(&mut self, key: &str) {
        let Some(run) = self
            .sequences
            .iter_mut()
            .find(|run| run.key == key && run.cinematic.is_some())
        else {
            error!("CancelCinematic: no cinematic '{key}' is running");
            return;
        };
        run.step = run.steps.len();
        let ending = run.end_cinematic(false);
        self.cinematic_endings.extend(ending);
    }

    /// Show a title card, replacing whatever card is up (see
    /// [`CinematicTitleActionConfig`]).
    pub fn post_cinematic_title(&mut self, config: CinematicTitleActionConfig) {
        let now = self.scenario_elapsed;
        self.cinematic_title = Some((config, now));
    }

    /// The card on screen right now with its age, or `None` once its hold has
    /// run out. Reading is what expires it: the card is presentation, and a
    /// scenario that is not being synced has no screen to be on.
    pub fn cinematic_title(&mut self) -> Option<(&CinematicTitleActionConfig, f32)> {
        let now = self.scenario_elapsed;
        let (config, posted_at) = self.cinematic_title.as_ref()?;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "an age in seconds, for a fade"
        )]
        let age = (now - posted_at) as f32;
        if age >= config.seconds {
            self.cinematic_title = None;
            return None;
        }
        self.cinematic_title
            .as_ref()
            .map(|(config, _)| (config, age))
    }

    /// The key of the scene the player may leave right now, for the HUD prompt.
    pub fn skippable_cinematic(&self) -> Option<&str> {
        let now = self.scenario_elapsed;
        self.sequences
            .iter()
            .find(|run| run.is_skippable_at(now))
            .map(|run| run.key.as_str())
    }

    /// Take the endings the driver has yet to announce.
    pub(crate) fn drain_cinematic_endings(&mut self) -> Vec<CinematicEnding> {
        std::mem::take(&mut self.cinematic_endings)
    }

    /// Open the `until` gate of one step, from the handler the loader spawned
    /// for it.
    ///
    /// The step index is checked against the cursor, so a gate event that
    /// arrives while the cursor is somewhere else does nothing. That is the
    /// ordering guarantee: a step is armed by the run reaching it, never by the
    /// world happening to satisfy it early.
    pub(crate) fn open_sequence_gate(&mut self, key: &str, step: usize) {
        let Some(run) = self.sequences.iter_mut().find(|run| run.key == key) else {
            return;
        };
        if run.stopped || run.step != step {
            return;
        }
        run.gate_open = true;
    }

    /// Advance the first sequence whose current step is ready, and return that
    /// step's actions.
    ///
    /// The cursor moves BEFORE the actions are handed back, so a step that
    /// starts another sequence - or writes anything else back into this world -
    /// cannot see a cursor that is still standing on itself. The driver calls
    /// this in a loop, so a run of zero-delay steps resolves within one frame.
    pub(crate) fn take_ready_sequence_step(&mut self, now: f64) -> Option<Vec<EventActionConfig>> {
        self.prune_finished_sequences();
        let mut endings = Vec::new();
        let mut ready = None;
        for run in &mut self.sequences {
            if run.stopped {
                continue;
            }
            let Some(step) = run.steps.get(run.step) else {
                continue;
            };
            let waited = now - run.since;
            let after_done = step.after.is_none_or(|after| waited >= after);
            let gate_done = step.until.is_none() || run.gate_open;
            if after_done && gate_done {
                let actions = step.actions.clone();
                run.step += 1;
                run.since = now;
                run.gate_open = false;
                if run.step >= run.steps.len() {
                    endings.extend(run.end_cinematic(false));
                }
                ready = Some(actions);
                break;
            }
            // A step that can never finish is a soft-lock, which is the worst
            // thing a scenario can ship. Stop the run and SAY SO rather than
            // skipping ahead, which would hide the authoring bug behind a
            // scenario that still limps to its end.
            if step.deadline.is_some_and(|deadline| waited >= deadline) {
                error!(
                    "sequence '{}' step {} passed its {}s deadline waiting for {}; the sequence stops here",
                    run.key,
                    run.step,
                    step.deadline.unwrap_or_default(),
                    describe_step_gate(step),
                );
                run.stopped = true;
                // A stuck scene is the author's bug, but the camera and the
                // controls it took are the PLAYER's. The ending still fires.
                endings.extend(run.end_cinematic(false));
            }
        }
        self.cinematic_endings.append(&mut endings);
        ready
    }

    /// Drop runs that reached the end of their step list, freeing the key. A
    /// run STOPPED by a deadline is kept: its key stays occupied so a restart
    /// cannot paper over the failure.
    fn prune_finished_sequences(&mut self) {
        self.sequences
            .retain(|run| run.stopped || run.step < run.steps.len());
    }

    /// Which step a sequence is standing on, or `None` when no run holds that
    /// key. Diagnostics and tests; authored content never reads the cursor.
    pub fn sequence_step(&self, key: &str) -> Option<usize> {
        self.sequences
            .iter()
            .find(|run| run.key == key)
            .map(|run| run.step)
    }

    /// Set a mutable scenario variable, unless a watch owns the name.
    pub fn insert_variable(&mut self, key: String, value: VariableLiteral) {
        if self.watches.iter().any(|watch| watch.variable == key) {
            error!("cannot write watched scenario variable '{}'", key);
            return;
        }
        self.dirty.insert(key.clone());
        self.variables.insert(key, value);
    }

    /// Read a mutable or watched scenario variable by key.
    pub fn get_variable(&self, key: &str) -> Option<&VariableLiteral> {
        self.watched_values
            .get(key)
            .or_else(|| self.variables.get(key))
    }

    /// Configure the watches owned by the newly loaded scenario.
    ///
    /// `reads_entity_queries` covers the whole scenario, not just `watches`: an
    /// entity query is equally legal as an inline expression factor, and the
    /// sampler that answers one has to be running before the action does. Pass
    /// `ScenarioConfig::reads_an_entity_query`.
    pub(crate) fn set_watches(&mut self, watches: Vec<WatchConfig>, reads_entity_queries: bool) {
        self.watches = watches;
        self.reads_entity_queries = reads_entity_queries;
        self.query_values.insert(
            QueryConfig::Scenario(ScenarioQuery {
                property: ScenarioProperty::Elapsed,
            }),
            VariableLiteral::Number(self.scenario_elapsed),
        );
        self.watched_values.clear();
        self.publish_watches();
    }

    /// Install the loaded scenario's wake profile. Set from `loader::wake`
    /// beside [`Self::set_watches`]; `EveryFrame` again at teardown.
    pub(crate) fn set_wake(&mut self, wake: WakeProfile) {
        self.wake = wake;
        self.dirty.clear();
        self.wake_checked_at = self.scenario_elapsed;
    }

    /// Whether the `OnUpdate` pulse is worth firing this frame.
    ///
    /// Two reasons to wake, and neither is a poll: a variable some `OnUpdate`
    /// filter reads was written, or the clock crossed a threshold one compares
    /// against. `EveryFrame` is the fail-safe answer for a scenario the
    /// analyser could not prove - the cost of a miss is today's behaviour.
    ///
    /// Read-only, because a Bevy run condition must be. The pulse itself calls
    /// [`Self::consume_wake`] when it fires, so a frame that does NOT fire
    /// leaves the reasons standing rather than swallowing them.
    pub(crate) fn is_wake_due(&self) -> bool {
        match &self.wake {
            WakeProfile::EveryFrame => true,
            WakeProfile::OnChange { vars, times } => {
                let crossed = times
                    .iter()
                    .any(|at| self.wake_checked_at < *at && *at <= self.scenario_elapsed);
                crossed || !self.dirty.is_disjoint(vars)
            }
        }
    }

    /// Retire the reasons the pulse just fired for. A scheduled time is tested
    /// as a CROSSING of this mark, so a threshold fires once and not on every
    /// frame after it.
    pub(crate) fn consume_wake(&mut self) {
        self.dirty.clear();
        self.wake_checked_at = self.scenario_elapsed;
    }

    /// Whether the loaded scenario can read an entity query - the gate on the
    /// per-frame entity sampler.
    pub(crate) fn reads_entity_queries(&self) -> bool {
        self.reads_entity_queries
    }

    fn publish_watches(&mut self) {
        self.watched_values.clear();
        for watch in &self.watches {
            if let Some(value) = self.query_values.get(&watch.query) {
                self.watched_values
                    .insert(watch.variable.clone(), value.clone());
            }
        }
    }

    /// Current pause-frozen scenario time.
    pub(crate) fn scenario_elapsed(&self) -> f64 {
        self.scenario_elapsed
    }

    /// Advance the pause-frozen scenario time.
    pub(crate) fn advance_scenario_elapsed(&mut self, delta: f64) {
        self.scenario_elapsed += delta;
        self.query_values.insert(
            QueryConfig::Scenario(ScenarioQuery {
                property: ScenarioProperty::Elapsed,
            }),
            VariableLiteral::Number(self.scenario_elapsed),
        );
        self.publish_watches();
    }

    /// Replace sampled entity query values, then publish all watches.
    pub(crate) fn sample_entity_speeds(&mut self, speeds: HashMap<String, Option<f64>>) {
        self.query_values
            .retain(|query, _| matches!(query, QueryConfig::Scenario(_)));
        for (id, speed) in speeds {
            if let Some(speed) = speed {
                self.query_values.insert(
                    QueryConfig::Entity(EntityQuery {
                        filter: EntityQueryFilter { id },
                        property: EntityProperty::Speed,
                    }),
                    VariableLiteral::Number(speed),
                );
            }
        }
        self.publish_watches();
    }

    /// Read a typed query from the current coherent world snapshot.
    pub fn query_value(&self, query: &QueryConfig) -> Option<&VariableLiteral> {
        self.query_values.get(query)
    }

    /// Read-only iteration over all scenario variables (unordered). For
    /// observers that snapshot the variable state - the run-timeline recorder
    /// (nova_probe) diffs successive snapshots to log changes - without
    /// exposing the map for mutation.
    pub fn variables(&self) -> impl Iterator<Item = (&String, &VariableLiteral)> {
        self.variables.iter().chain(self.watched_values.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The queued-command flush is CHUNKED, not drained. Five commands costing
    /// 2 ms each cannot fit in the 3 ms budget, so the batch takes several
    /// runs - which is the point: the old single `queue.apply` paid a shipped
    /// chapter's whole spawn burst as ONE ~300 ms frame. While commands remain
    /// the event world reports itself SETTLING, and every command still lands
    /// exactly once, in push order.
    ///
    /// Fail-first: the pre-change flush applies all five in the first run, so
    /// both the "not all applied yet" and the "settling" asserts fail.
    #[test]
    fn the_spawn_flush_is_chunked_across_runs() {
        use std::sync::{Arc, Mutex};

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let applied: Arc<Mutex<Vec<usize>>> = Arc::default();
        {
            let mut event_world = world.resource_mut::<NovaEventWorld>();
            for index in 0..5 {
                let applied = applied.clone();
                event_world.push_command(move |commands| {
                    commands.queue(move |_: &mut World| {
                        // Each object costs a little under the frame budget, so
                        // no single run can take the whole batch.
                        std::thread::sleep(Duration::from_millis(2));
                        applied.lock().unwrap().push(index);
                    });
                });
            }
        }
        assert!(
            world.resource::<NovaEventWorld>().is_settling(),
            "a queued batch reports the world as settling"
        );

        NovaEventWorld::state_to_world_system(&mut world);
        assert!(
            applied.lock().unwrap().len() < 5,
            "one run must not swallow the whole batch (got {:?})",
            applied.lock().unwrap()
        );
        assert!(
            world.resource::<NovaEventWorld>().is_settling(),
            "the world is still settling with commands left"
        );

        let mut runs = 1;
        while world.resource::<NovaEventWorld>().is_settling() {
            NovaEventWorld::state_to_world_system(&mut world);
            runs += 1;
            assert!(runs < 100, "the drain must terminate");
        }

        assert!(runs >= 2, "the batch spanned {runs} run(s)");
        assert_eq!(
            *applied.lock().unwrap(),
            vec![0, 1, 2, 3, 4],
            "every command lands exactly once, in push order"
        );
        assert!(
            !world.resource::<NovaEventWorld>().is_settling(),
            "an empty queue means the world is live"
        );
    }

    /// A command costing MORE than the whole budget still lands: the drain
    /// always applies at least one per run, so an expensive object overruns by
    /// its own cost instead of deadlocking the queue behind it.
    #[test]
    fn an_over_budget_command_still_lands() {
        use std::sync::{Arc, Mutex};

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let applied: Arc<Mutex<bool>> = Arc::default();
        {
            let flag = applied.clone();
            world
                .resource_mut::<NovaEventWorld>()
                .push_command(move |commands| {
                    commands.queue(move |_: &mut World| {
                        std::thread::sleep(SPAWN_DRAIN_BUDGET * 2);
                        *flag.lock().unwrap() = true;
                    });
                });
        }

        NovaEventWorld::state_to_world_system(&mut world);
        assert!(*applied.lock().unwrap(), "the over-budget command applied");
        assert!(!world.resource::<NovaEventWorld>().is_settling());
    }

    /// A slow frame widens the budget to a share of itself: behind a
    /// one-second frame, a queue that would take several fast frames lands in
    /// one run.
    #[test]
    fn a_slow_frame_drains_a_share_of_itself() {
        use std::sync::{Arc, Mutex};

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();
        let mut clock = Time::<Real>::default();
        clock.advance_by(Duration::from_secs(1));
        world.insert_resource(clock);

        let landed: Arc<Mutex<u32>> = Arc::default();
        for _ in 0..10 {
            let count = landed.clone();
            world
                .resource_mut::<NovaEventWorld>()
                .push_command(move |commands| {
                    commands.queue(move |_: &mut World| {
                        std::thread::sleep(SPAWN_DRAIN_BUDGET);
                        *count.lock().unwrap() += 1;
                    });
                });
        }

        NovaEventWorld::state_to_world_system(&mut world);
        assert_eq!(
            *landed.lock().unwrap(),
            10,
            "a fifth of a one-second frame lands ten budget-sized commands in one run"
        );
        assert!(!world.resource::<NovaEventWorld>().is_settling());
    }

    #[test]
    fn watched_names_are_read_only_and_share_normal_lookup() {
        let mut world = NovaEventWorld::default();
        world.set_watches(
            vec![WatchConfig {
                variable: "elapsed".to_string(),
                query: QueryConfig::Scenario(ScenarioQuery {
                    property: ScenarioProperty::Elapsed,
                }),
            }],
            false,
        );
        world.advance_scenario_elapsed(3.0);
        world.sample_entity_speeds(HashMap::new());
        assert_eq!(
            world.get_variable("elapsed"),
            Some(&VariableLiteral::Number(3.0))
        );

        world.insert_variable("elapsed".to_string(), VariableLiteral::Number(99.0));
        assert_eq!(
            world.get_variable("elapsed"),
            Some(&VariableLiteral::Number(3.0)),
            "mutable writes cannot replace a watched value"
        );
    }

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
                skybox_brightness: nova_ship::prelude::DEFAULT_SKYBOX_BRIGHTNESS,
                watches: vec![],
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
            .push_narrative_cue(NarrativeCueActionConfig {
                channel: CHANNEL_COMMS.to_string(),
                speaker: "Alpha".to_string(),
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
            assert_eq!(feed.0[0].speaker, "Alpha");
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
            .push_narrative_cue(NarrativeCueActionConfig {
                channel: CHANNEL_COMMS.to_string(),
                speaker: "Alpha".to_string(),
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
            .push_narrative_cue(NarrativeCueActionConfig {
                channel: CHANNEL_COMMS.to_string(),
                speaker: "Alpha".to_string(),
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
            .push_narrative_cue(NarrativeCueActionConfig {
                channel: CHANNEL_COMMS.to_string(),
                speaker: "Alpha".to_string(),
                text: "Look at me.".to_string(),
                dwell: None,
                icon: Some(AssetRef::from("self://icons/alpha.png")),
            });
        for _ in 0..5 {
            app.update();
        }
        let feed = app.world().resource::<StoryFeed>();
        assert_eq!(feed.0.len(), 1);
        assert_eq!(
            feed.0[0].icon.as_ref().and_then(|icon| icon.path()),
            Some("self://icons/alpha.png"),
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

    /// A cue names a channel by id; the sync is where that id becomes the
    /// channel the panel draws. It is tone, tag and signal strength - so a line
    /// that reached the HUD holding only its id is a line in the wrong voice.
    #[test]
    fn story_sync_resolves_each_cue_id_against_the_authored_catalog() {
        let mut app = App::new();
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<StoryFeed>();
        app.insert_resource(GameChannels(vec![
            NarrativeChannelConfig::new(CHANNEL_COMMS, ChipTone::Comms),
            NarrativeChannelConfig::new(CHANNEL_CREW, ChipTone::Phosphor),
            NarrativeChannelConfig::new(CHANNEL_GUARD, ChipTone::Amber)
                .with_tag("GUARD")
                .with_signal_strength(0.55),
        ]));
        app.add_systems(Update, NovaEventWorld::state_to_world_system);

        let authored = [CHANNEL_COMMS, CHANNEL_CREW, CHANNEL_GUARD];
        for id in authored {
            app.world_mut()
                .resource_mut::<NovaEventWorld>()
                .push_narrative_cue(NarrativeCueActionConfig {
                    channel: id.to_string(),
                    speaker: "Alpha".to_string(),
                    text: "Say again.".to_string(),
                    dwell: None,
                    icon: None,
                });
        }
        for _ in 0..5 {
            app.update();
        }

        let catalog = app.world().resource::<GameChannels>().clone();
        let feed = app.world().resource::<StoryFeed>();
        assert_eq!(feed.0.len(), authored.len());
        for (line, id) in feed.0.iter().zip(authored) {
            assert_eq!(
                Some(&line.channel),
                catalog.get_channel(id),
                "the line kept its id instead of the channel it names"
            );
        }
    }

    /// The HUD is told the skip is live only while a scene is offering it, and
    /// which action leaves the scene. A prompt left up after the scene ended
    /// offers the player a key that does nothing.
    #[test]
    fn the_skip_prompt_follows_the_scene_that_offers_it() {
        let mut app = App::new();
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<CinematicPrompt>();
        app.add_systems(Update, NovaEventWorld::state_to_world_system);

        {
            let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
            world.start_cinematic(
                "strike".to_string(),
                Arc::new(vec![SequenceStepConfig {
                    after: Some(30.0),
                    ..default()
                }]),
                true,
            );
        }
        app.update();
        assert!(
            app.world()
                .resource::<CinematicPrompt>()
                .skip_action
                .is_none(),
            "the hold-off has not passed, so nothing is offered yet"
        );

        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .advance_scenario_elapsed(CINEMATIC_SKIP_ARM_SECONDS);
        app.update();
        assert_eq!(
            app.world()
                .resource::<CinematicPrompt>()
                .skip_action
                .as_deref(),
            Some(CINEMATIC_SKIP_ACTION),
            "the prompt names the action, so the HUD can print the live binding"
        );

        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .skip_cinematic();
        app.update();
        assert!(
            app.world()
                .resource::<CinematicPrompt>()
                .skip_action
                .is_none(),
            "the offer goes with the scene"
        );
    }
}
