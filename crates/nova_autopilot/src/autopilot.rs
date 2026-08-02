//! Scripted state-driver: force-advance a game's state machine.
//!
//! [`AutopilotPlugin`] is an env-gated dev tool that drives a game through its
//! [`States`] machine on a fixed timeline for headless verification. It owns
//! the state clock, the transition logging and its report to the
//! [`completion`] protocol; the game supplies the timeline
//! (a list of `(state, seconds)` steps) and an optional per-frame input
//! closure that pokes whatever it wants to drive gameplay.
//!
//! It is inert unless the [`AUTOPILOT_ENV`] environment variable is set, so a
//! game adds it unconditionally and pays nothing in a normal run:
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use nova_autopilot::autopilot::AutopilotPlugin;
//!
//! #[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
//! enum GameState {
//!     #[default]
//!     Menu,
//!     Playing,
//!     GameOver,
//! }
//!
//! # fn build(app: &mut App) {
//! app.add_plugins(
//!     AutopilotPlugin::new()
//!         .hold(GameState::Menu, 0.5)
//!         .hold(GameState::Playing, 3.0)
//!         .hold(GameState::GameOver, 0.5)
//!         .input(|world, elapsed| {
//!             // Runs every frame with full world access and total elapsed
//!             // seconds; here, thrust continuously once in Playing.
//!             if elapsed > 0.5 {
//!                 world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Space);
//!             }
//!         }),
//! );
//! # }
//! ```

use std::sync::Arc;

use bevy::{input::InputSystems, prelude::*, state::state::FreelyMutableState};

use crate::completion;

/// Environment variable that arms the scripted autopilot. Unset, the plugin
/// adds nothing at all.
pub const AUTOPILOT_ENV: &str = "NOVA_AUTOPILOT";

/// Per-frame input hook: full world access plus total elapsed seconds since the
/// autopilot started driving.
type InputFn = dyn Fn(&mut World, f32) + Send + Sync;

/// Message written each time a
/// [`loop_while_pending`](AutopilotPlugin::loop_while_pending) autopilot
/// restarts its cycle because other completion collectors are still pending.
/// The game observes it to reset its scene/script state (re-trigger a
/// scenario load, zero its script resource) so the repeated cycle measures
/// ACTIVITY, not an idle tail.
#[derive(Message)]
pub struct AutopilotLoop;

/// Env-gated plugin that force-drives a [`States`] machine along a scripted
/// timeline for headless verification.
///
/// Build the timeline with [`hold`](Self::hold) and, optionally, attach a
/// per-frame input closure with [`input`](Self::input). When [`AUTOPILOT_ENV`]
/// is set the plugin sets the first state, holds each step for its duration
/// while advancing `NextState`, logs every transition, and reports completion
/// to the [`completion`] protocol after the last step (the
/// app exits when EVERY registered collector - a frame capture, a screenshot -
/// is done, not when the first one finishes). When the env var is unset it
/// adds nothing.
pub struct AutopilotPlugin<S: States + FreelyMutableState> {
    schedule: Vec<(S, f32)>,
    input: Option<Arc<InputFn>>,
    self_completing: bool,
    loop_while_pending: bool,
}

impl<S: States + FreelyMutableState> Default for AutopilotPlugin<S> {
    fn default() -> Self {
        Self {
            schedule: Vec::new(),
            input: None,
            self_completing: false,
            loop_while_pending: false,
        }
    }
}

impl<S: States + FreelyMutableState> AutopilotPlugin<S> {
    /// Create an empty autopilot. Add steps with [`hold`](Self::hold).
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a step: enter `state` and hold it for `seconds` before advancing
    /// to the next step. The first `hold` call names the starting state.
    pub fn hold(mut self, state: S, seconds: f32) -> Self {
        self.schedule.push((state, seconds));
        self
    }

    /// Set the per-frame input closure. It runs every frame in `PreUpdate`,
    /// after Bevy has collected input for the frame (`InputSystems`) but before
    /// the game's `Update` input systems read it, with `&mut World` and the
    /// total elapsed seconds. So it can poke input resources (`ButtonInput`,
    /// `Touches`, ...) or the game's own input components and the game will see
    /// the poke this frame -- including a fresh `just_pressed`, which the input
    /// collection would otherwise have cleared.
    ///
    /// The closure runs in every state, so if it presses keys a menu reacts to
    /// (an "any key to start" screen), gate it to the gameplay state to avoid
    /// tripping those transitions early:
    /// `if *world.resource::<State<GameState>>().get() != GameState::Playing { return; }`.
    pub fn input(mut self, f: impl Fn(&mut World, f32) + Send + Sync + 'static) -> Self {
        self.input = Some(Arc::new(f));
        self
    }

    /// Mark completion as SCRIPT-OWNED: the timeline is a runway, not the
    /// finish line. The input closure's staged script reports done itself
    /// (`world.resource_mut::<HarnessCompletion>().done(completion::AUTOPILOT)`)
    /// when its final stage lands; if the TIMELINE expires first the script
    /// stalled, and the run exits [`AppExit::error`] naming it - an abort,
    /// not a completion, so a stalled script can never pass as a finished
    /// cycle.
    pub fn self_completing(mut self) -> Self {
        self.self_completing = true;
        self
    }

    /// Repeat the cycle while OTHER completion collectors are still
    /// pending: at the timeline's end, instead of reporting done, write an
    /// [`AutopilotLoop`] message (the game resets its scene/script on it),
    /// zero the cycle clock, and keep driving - a frame capture then
    /// measures repeated ACTIVITY instead of an idle tail. Reports done
    /// normally once nothing else is pending. Ignored (with a warning) when
    /// combined with [`self_completing`](Self::self_completing) - a
    /// script-owned run decides its own repetition.
    pub fn loop_while_pending(mut self) -> Self {
        self.loop_while_pending = true;
        self
    }
}

/// Internal driver state; kept out of the prelude per the crate conventions.
#[derive(Resource)]
struct AutopilotState<S: States + FreelyMutableState> {
    schedule: Vec<(S, f32)>,
    input: Option<Arc<InputFn>>,
    index: usize,
    elapsed: f32,
    state_elapsed: f32,
    started: bool,
    done: bool,
    self_completing: bool,
    loop_while_pending: bool,
    loops: u32,
}

impl<S: States + FreelyMutableState> Plugin for AutopilotPlugin<S> {
    fn build(&self, app: &mut App) {
        if std::env::var(AUTOPILOT_ENV).is_err() {
            return;
        }
        if self.schedule.is_empty() {
            warn!("AutopilotPlugin: {AUTOPILOT_ENV} set but the schedule is empty; doing nothing");
            return;
        }

        debug!(
            "AutopilotPlugin: build ({AUTOPILOT_ENV} active, {} steps)",
            self.schedule.len()
        );

        let loop_while_pending = if self.loop_while_pending && self.self_completing {
            warn!(
                "AutopilotPlugin: loop_while_pending is ignored with self_completing \
                 (a script-owned run decides its own repetition)"
            );
            false
        } else {
            self.loop_while_pending
        };
        app.insert_resource(AutopilotState::<S> {
            schedule: self.schedule.clone(),
            input: self.input.clone(),
            index: 0,
            elapsed: 0.0,
            state_elapsed: 0.0,
            started: false,
            done: false,
            self_completing: self.self_completing,
            loop_while_pending,
            loops: 0,
        });
        app.add_message::<AutopilotLoop>();
        completion::register(app, completion::AUTOPILOT);
        // NOTE: must run after `InputSystems`, which clears `just_pressed`
        // every frame - only then does the input closure's fresh press
        // survive into the game's Update systems.
        app.add_systems(PreUpdate, autopilot_drive::<S>.after(InputSystems));
    }
}

/// Exclusive driver: sets the initial state, advances the timeline, runs the
/// input closure, and reports done after the last step.
///
/// Exclusive because the input closure takes `&mut World`; the driver state is
/// removed for the duration so the closure has unencumbered world access.
fn autopilot_drive<S: States + FreelyMutableState>(world: &mut World) {
    let mut st = world
        .remove_resource::<AutopilotState<S>>()
        .expect("AutopilotState is inserted by AutopilotPlugin::build");

    // NOTE: stay inert once finished - do NOT index past the end of the
    // schedule while the completion watcher waits on other collectors.
    if st.done
        || (st.self_completing
            && !world
                .resource::<completion::HarnessCompletion>()
                .is_pending(completion::AUTOPILOT))
    {
        world.insert_resource(st);
        return;
    }

    // NOTE: skip the set when already in the first state, or the run opens
    // with a spurious OnExit/OnEnter of the default state; the clock starts
    // a frame later, once the transition has applied.
    if !st.started {
        let first = st.schedule[0].0.clone();
        if *world.resource::<State<S>>().get() != first {
            world.resource_mut::<NextState<S>>().set(first.clone());
        }
        trace!("autopilot: start in {first:?}");
        st.started = true;
        world.insert_resource(st);
        return;
    }

    let dt = world.resource::<Time>().delta_secs();
    st.elapsed += dt;
    st.state_elapsed += dt;

    if let Some(input) = st.input.clone() {
        input(world, st.elapsed);
    }

    // NOTE: while looping, finish the moment other collectors are done
    // rather than at the cycle's end - a slow cycle otherwise wastes up to
    // its full length and can straddle the deadline into a false laggard.
    if st.loops > 0
        && !world
            .resource::<completion::HarnessCompletion>()
            .others_pending(completion::AUTOPILOT)
    {
        info!(
            "autopilot: collectors done after {} loop(s); cycle complete, no panic (t={:.1}s)",
            st.loops, st.elapsed
        );
        world
            .resource_mut::<completion::HarnessCompletion>()
            .done(completion::AUTOPILOT);
        st.done = true;
        world.insert_resource(st);
        return;
    }

    let hold = st.schedule[st.index].1;
    if st.state_elapsed >= hold {
        st.index += 1;
        if st.index >= st.schedule.len() {
            if st.loop_while_pending
                && world
                    .resource::<completion::HarnessCompletion>()
                    .others_pending(completion::AUTOPILOT)
            {
                st.loops += 1;
                info!(
                    "autopilot: cycle {} restarting - other collectors still pending",
                    st.loops
                );
                world.write_message(AutopilotLoop);
                // NOTE: hold the final step (no further state transitions)
                // and zero both clocks, so the input script sees a fresh
                // cycle rather than a monotonic elapsed.
                st.index = st.schedule.len() - 1;
                st.elapsed = 0.0;
                st.state_elapsed = 0.0;
                world.insert_resource(st);
                return;
            }
            if st.self_completing {
                // NOTE: an expired runway is an ABORT, not a completion -
                // error exits do not negotiate with other collectors.
                error!(
                    "autopilot: timeline expired but the self-completing \
                     script never reported done (t={:.1}s)",
                    st.elapsed
                );
                world.write_message(AppExit::error());
            } else {
                info!("autopilot: cycle complete, no panic (t={:.1}s)", st.elapsed);
                world
                    .resource_mut::<completion::HarnessCompletion>()
                    .done(completion::AUTOPILOT);
            }
            st.done = true;
            world.insert_resource(st);
            return;
        }
        let next = st.schedule[st.index].0.clone();
        info!("autopilot: -> {next:?} (t={:.1}s)", st.elapsed);
        world.resource_mut::<NextState<S>>().set(next);
        st.state_elapsed = 0.0;
    }

    world.insert_resource(st);
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex, Once},
        time::Duration,
    };

    use bevy::{input::InputPlugin, state::app::StatesPlugin, time::TimeUpdateStrategy};

    use super::*;

    #[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
    enum TestState {
        #[default]
        Boot,
        Playing,
        Over,
    }

    /// Arm the plugin for the whole test binary. Set once, never removed and
    /// never with a second value, so the parallel test threads cannot observe
    /// disagreeing environments.
    fn arm() {
        static ARM: Once = Once::new();
        ARM.call_once(|| std::env::set_var(AUTOPILOT_ENV, "1"));
    }

    /// Frames are 1/60s of MANUAL time: `Time` is rewritten from `Time<Real>`
    /// every frame, so a hand-advanced clock would be stomped and the timeline
    /// would otherwise depend on how fast the test host runs.
    const FRAME: Duration = Duration::from_nanos(16_666_667);

    /// What game systems saw, so the assertions read the driver's effects from
    /// OUTSIDE the driver.
    #[derive(Resource, Default)]
    struct Seen {
        states: Vec<TestState>,
        space_just_pressed: bool,
        loops: usize,
        boot_enters: usize,
    }

    fn record_state(state: Res<State<TestState>>, mut seen: ResMut<Seen>) {
        if seen.states.last() != Some(state.get()) {
            seen.states.push(state.get().clone());
        }
    }

    fn record_input(input: Res<ButtonInput<KeyCode>>, mut seen: ResMut<Seen>) {
        seen.space_just_pressed |= input.just_pressed(KeyCode::Space);
    }

    fn record_loops(mut reader: MessageReader<AutopilotLoop>, mut seen: ResMut<Seen>) {
        seen.loops += reader.read().count();
    }

    /// Headless rig: minimal app, real state machine, REAL input collection
    /// (without `InputPlugin` the `InputSystems` set is empty and the driver's
    /// ordering would be untested), and a deterministic clock.
    fn app(plugin: AutopilotPlugin<TestState>) -> App {
        arm();
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, InputPlugin));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));
        app.init_state::<TestState>();
        app.init_resource::<Seen>();
        app.add_systems(Update, (record_state, record_input, record_loops));
        app.add_plugins(plugin);
        app
    }

    fn exits(app: &mut App) -> Vec<AppExit> {
        app.world_mut()
            .resource_mut::<Messages<AppExit>>()
            .drain()
            .collect()
    }

    /// Run `frames` frames, collecting every [`AppExit`] written along the
    /// way. Drained per frame: `Messages` is double-buffered, so an exit
    /// written mid-run is gone by the time a later frame reads it.
    fn run(app: &mut App, frames: usize) -> Vec<AppExit> {
        let mut observed = Vec::new();
        for _ in 0..frames {
            app.update();
            observed.extend(exits(app));
        }
        observed
    }

    #[test]
    fn autopilot_drives_the_timeline_and_reports_done() {
        let mut app = app(AutopilotPlugin::new()
            .hold(TestState::Playing, 0.1)
            .hold(TestState::Over, 0.1));

        // Two warm-up frames, then 6 frames per 0.1s step, plus slack.
        let observed = run(&mut app, 20);

        assert_eq!(
            app.world().resource::<Seen>().states,
            // No `Boot`: the driver's first frame sets the first step and
            // `StateTransition` applies it before `Update` ever reads the
            // state, which is what keeps the run from opening on a spurious
            // OnExit/OnEnter of the default state.
            vec![TestState::Playing, TestState::Over],
            "the timeline advances the state machine in order"
        );
        assert!(
            !app.world()
                .resource::<completion::HarnessCompletion>()
                .is_pending(completion::AUTOPILOT),
            "the driver reports done to the completion protocol"
        );
        assert_eq!(
            observed,
            vec![AppExit::Success],
            "the watcher exits once the only collector is done"
        );
        assert!(
            run(&mut app, 5).is_empty(),
            "a finished driver stays inert instead of re-reporting"
        );
    }

    #[test]
    fn a_timeline_starting_in_the_current_state_does_not_re_enter_it() {
        let mut app = app(AutopilotPlugin::new()
            .hold(TestState::Boot, 0.1)
            .hold(TestState::Playing, 0.1));
        app.add_systems(OnEnter(TestState::Boot), |mut seen: ResMut<Seen>| {
            seen.boot_enters += 1;
        });

        run(&mut app, 20);

        assert_eq!(
            app.world().resource::<Seen>().boot_enters,
            1,
            "only `init_state`'s own entry: setting NextState to the state the \
             run is already in would open it with a spurious OnExit/OnEnter"
        );
        assert_eq!(
            app.world().resource::<Seen>().states,
            vec![TestState::Boot, TestState::Playing],
            "the timeline still advances off the first step"
        );
    }

    #[test]
    fn input_closure_press_survives_input_collection() {
        let mut app =
            app(AutopilotPlugin::new()
                .hold(TestState::Playing, 10.0)
                .input(|world, _| {
                    world
                        .resource_mut::<ButtonInput<KeyCode>>()
                        .press(KeyCode::Space);
                }));

        run(&mut app, 4);

        assert!(
            app.world().resource::<Seen>().space_just_pressed,
            "the closure runs after InputSystems, so its press is still \
             just_pressed when Update reads it"
        );
    }

    #[test]
    fn expired_self_completing_runway_error_exits() {
        let mut app = app(AutopilotPlugin::new()
            .hold(TestState::Playing, 0.1)
            .self_completing());

        let observed = run(&mut app, 15);

        assert_eq!(observed.len(), 1, "the expired runway exits exactly once");
        assert_ne!(
            observed[0],
            AppExit::Success,
            "a script that never reported done ABORTS; it must not pass"
        );
        assert!(
            app.world()
                .resource::<completion::HarnessCompletion>()
                .is_pending(completion::AUTOPILOT),
            "the abort does not fake a completion report"
        );
    }

    #[test]
    fn loop_while_pending_resets_and_finishes_early() {
        let elapsed = Arc::new(Mutex::new(Vec::<f32>::new()));
        let recorder = Arc::clone(&elapsed);
        let mut app = app(AutopilotPlugin::new()
            .hold(TestState::Playing, 0.1)
            .loop_while_pending()
            .input(move |_, t| recorder.lock().unwrap().push(t)));
        // A second, slower collector: what the autopilot loops FOR.
        completion::register(&mut app, completion::SCREENSHOT);

        let observed = run(&mut app, 20);

        assert!(
            app.world().resource::<Seen>().loops >= 2,
            "the cycle repeats while the other collector is pending"
        );
        let seen = elapsed.lock().unwrap().clone();
        assert!(
            seen.windows(2).any(|w| w[1] < w[0]),
            "the cycle clock zeroes on restart, so the script sees a fresh \
             cycle rather than a monotonic elapsed: {seen:?}"
        );
        assert!(
            app.world()
                .resource::<completion::HarnessCompletion>()
                .is_pending(completion::AUTOPILOT),
            "the autopilot holds its own completion open while looping"
        );
        assert!(observed.is_empty(), "no exit while looping");

        app.world_mut()
            .resource_mut::<completion::HarnessCompletion>()
            .done(completion::SCREENSHOT);
        // One frame for the driver to notice, one for the watcher's decision -
        // NOT the rest of the cycle.
        assert_eq!(
            run(&mut app, 2),
            vec![AppExit::Success],
            "clearing the other collector finishes the cycle immediately"
        );
    }
}
