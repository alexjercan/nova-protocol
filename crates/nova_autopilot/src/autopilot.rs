//! Scripted state-driver: walk a game through a list of named steps.
//!
//! [`AutopilotPlugin`] is an env-gated dev tool that drives a game for headless
//! verification. A script is a list of STEPS, and a step is four optional
//! things plus one required one:
//!
//! | Part | Meaning |
//! | --- | --- |
//! | `name` | what a log line and a stall message call this beat |
//! | `enter` | the [`States`] value to set on entry |
//! | `on_enter` | a world action run once, on entry (an [`input`](crate::input) gesture, a scenario poke) |
//! | `each` | a world action run every frame, with the IN-STEP elapsed seconds |
//! | `until` | the [`Predicate`] that advances the step |
//! | `deadline` | in-step seconds after which an unsatisfied `until` ABORTS the run, naming the step |
//!
//! The step advances the first frame `until` holds. Elapsed time is one
//! predicate among many ([`elapsed`](crate::predicate::elapsed)), so a timed
//! beat - enter a state, wait N seconds - is the same machinery as every other
//! wait rather than a second mechanism.
//!
//! It is inert unless the [`AUTOPILOT_ENV`] environment variable is set, so a
//! game adds it unconditionally and pays nothing in a normal run:
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use nova_autopilot::prelude::*;
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
//!         // Wait for what actually has to happen, not for a guessed duration:
//!         // a slow load delays the beats instead of eating them.
//!         .step("load")
//!         .enter(GameState::Menu)
//!         .until(state_is(GameState::Playing))
//!         .deadline(30.0)
//!         .add()
//!         .step("thrust")
//!         .on_enter(press_key(KeyCode::Space))
//!         .until(elapsed(2.0))
//!         .add()
//!         .step("game over")
//!         .enter(GameState::GameOver)
//!         .until(elapsed(0.5))
//!         .add(),
//! );
//! # }
//! ```
//!
//! ## Deadlines
//!
//! A step's `deadline` is IN-STEP seconds and is unset by default: the
//! run-level watcher ([`DEADLINE_ENV`](crate::completion::DEADLINE_ENV)) stays
//! the backstop for a script that hangs somewhere without one. Set a deadline
//! where a stall is worth NAMING, and keep the sum of a script's deadlines
//! under the run-level value - otherwise the generic hang detector wins the
//! race and the named-step diagnostic is lost. That ordering is documented, not
//! enforced: the run-level value comes from the harness that launches the
//! process, which this crate cannot see.

use std::sync::Arc;

use bevy::{input::InputSystems, prelude::*, state::state::FreelyMutableState};

use crate::{completion, predicate::prelude::Predicate};

/// Environment variable that arms the scripted autopilot. Unset, the plugin
/// adds nothing at all.
pub const AUTOPILOT_ENV: &str = "NOVA_AUTOPILOT";

/// A step's entry action: full world access, run once when the step begins.
type EnterFn = dyn Fn(&mut World) + Send + Sync;

/// A step's per-frame action: full world access, the elapsed seconds IN THIS
/// STEP (not since the run started - that clock is the one every script used to
/// correct away by hand), and the count of driven frames this step has had,
/// starting at 1 on its first driven frame.
///
/// The frame index is there because a gesture that has to act on PARTICULAR
/// frames within one step - a double click, whose two releases must land two
/// frames apart - would otherwise have to smuggle a counter of its own through
/// the closures. The driver is already holding that number.
type EachFn = dyn Fn(&mut World, f32, u32) + Send + Sync;

/// Message written each time a [`loop_from`](AutopilotPlugin::loop_from)
/// autopilot restarts its cycle because other completion collectors are still
/// pending. The game observes it to reset its scene/script state (re-trigger a
/// scenario load, zero its script resource) so the repeated cycle measures
/// ACTIVITY, not an idle tail.
///
/// [`on_loop`](AutopilotPlugin::on_loop) is the in-driver twin, for a reset
/// that wants `&mut World` instead of a system.
#[derive(Message)]
pub struct AutopilotLoop;

/// A step's stall diagnosis: read the world at the moment the deadline expired
/// and say what the wait was actually looking at. Run ONCE, on the abort path,
/// so it may be as expensive as the answer needs.
type DiagnoseFn = dyn Fn(&World) -> String + Send + Sync;

/// One beat of a script. Built through [`AutopilotPlugin::step`]; the driver
/// stores the list and walks it.
struct Step<S: States + FreelyMutableState> {
    name: String,
    enter: Option<S>,
    on_enter: Vec<Arc<EnterFn>>,
    each: Option<Arc<EachFn>>,
    until: Arc<Predicate>,
    deadline: Option<f32>,
    diagnose: Option<Arc<DiagnoseFn>>,
}

// Hand-written rather than derived: `#[derive(Clone)]` would add an `S: Clone`
// bound, and a `States` type is not required to be `Clone` at all. The `Arc`
// callbacks clone by refcount, so nothing here needs it.
impl<S: States + FreelyMutableState> Clone for Step<S> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            enter: self.enter.clone(),
            on_enter: self.on_enter.clone(),
            each: self.each.clone(),
            until: Arc::clone(&self.until),
            deadline: self.deadline,
            diagnose: self.diagnose.clone(),
        }
    }
}

/// The advance condition of a step that declares none: advance on the next
/// driven frame. A step is then a pure action - enter a state, poke the world,
/// move on - which is what a beat list of instant gestures wants.
fn immediately() -> Arc<Predicate> {
    Arc::new(|_: &World| true)
}

/// Env-gated plugin that walks a [`States`] machine through a scripted list of
/// predicate-advanced steps for headless verification.
///
/// Build the script with [`step`](Self::step) (or its
/// [`hold`](Self::hold) shorthand). When [`AUTOPILOT_ENV`] is set the plugin
/// enters each step in turn - setting `NextState`, running the entry action -
/// advances it the frame its predicate holds, logs every transition, and
/// reports completion to the [`completion`] protocol after the last step (the
/// app exits when EVERY registered collector - a frame capture, a loop
/// recorder - is done, not when the first one finishes). When the env var is
/// unset it adds nothing.
pub struct AutopilotPlugin<S: States + FreelyMutableState> {
    steps: Vec<Step<S>>,
    input: Option<Arc<EachFn>>,
    loop_from: Option<String>,
    on_loop: Option<Arc<EnterFn>>,
}

// Hand-written rather than derived: `#[derive(Default)]` would add an
// `S: Default` bound the empty plugin does not need - no `S` value is built.
impl<S: States + FreelyMutableState> Default for AutopilotPlugin<S> {
    fn default() -> Self {
        Self {
            steps: Vec::new(),
            input: None,
            loop_from: None,
            on_loop: None,
        }
    }
}

impl<S: States + FreelyMutableState> AutopilotPlugin<S> {
    /// Create an empty autopilot. Add beats with [`step`](Self::step).
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin a step called `name`. Chain the parts it needs and close it with
    /// [`add`](StepBuilder::add), which hands the plugin back:
    ///
    /// ```rust,no_run
    /// # use bevy::prelude::*;
    /// # use nova_autopilot::prelude::*;
    /// # #[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)] enum S { #[default] Boot }
    /// AutopilotPlugin::<S>::new()
    ///     .step("fire").on_enter(press_mouse(MouseButton::Left)).until(elapsed(1.0)).add();
    /// ```
    ///
    /// The name is what the run log and a stall message call the beat, so name
    /// it after what it is WAITING FOR ("lock the prey") rather than what it
    /// pokes.
    pub fn step(self, name: impl Into<String>) -> StepBuilder<S> {
        StepBuilder {
            step: Step {
                name: name.into(),
                enter: None,
                on_enter: Vec::new(),
                each: None,
                until: immediately(),
                deadline: None,
                diagnose: None,
            },
            plugin: self,
        }
    }

    /// Append the three beats of a CLICK on the UI node called `name`: wait for
    /// it to lay out, press it, and release it - the last beat holding until
    /// `landed`, which is what the click was FOR.
    ///
    /// The one spelling of the fleet's most-used gesture. Three beats because
    /// that is the real shape: `click_named` warns and CONTINUES when the name
    /// resolves to nothing, so a press fired at a panel that has not laid out
    /// yet is a beat silently lost; and widgets act on `Activate`, which fires
    /// on RELEASE over the same node, so the release is the beat that carries
    /// the effect.
    ///
    /// `landed` is the point of the parameter. A gesture that ends on
    /// [`pointer_released`](crate::predicate::pointer_released) has only proved
    /// that the pointer let go - the walk then asserts the button's effect on
    /// whatever frame it happens to reach. Passing the effect itself
    /// (`state_is(Playing)`, `ui_node_present("Inspector Field Name")`,
    /// `and(editor_field_focused(), ..)`) makes a click that MISSED fail here,
    /// named, instead of three beats later on a symptom. Pass
    /// `pointer_released()` where the effect is genuinely the caller's next
    /// beat to wait on.
    ///
    /// The layout beat carries [`ui_node_diagnosis`](crate::input::ui_node_diagnosis),
    /// so a stall says which of "no such node", "hidden", and "not laid out
    /// yet" it hit.
    pub fn click_named(
        self,
        label: &str,
        name: &str,
        landed: Arc<Predicate>,
        deadline: f32,
    ) -> Self {
        let aim = name.to_string();
        let re_aim = name.to_string();
        self.step(format!("{label}: the widget is up"))
            .until(crate::predicate::ui_node_present(name.to_string()))
            .diagnose(crate::input::ui_node_diagnosis(name.to_string()))
            .deadline(deadline)
            .add()
            // Aimed, then WAITED on. A resolvable rect is not a clickable
            // widget: an overlay on its way out takes every pick, and a reflow
            // moves the widget out from under an aim already sent. Re-aimed
            // every frame, so the beat recovers from the reflow instead of
            // holding a stale coordinate until the deadline.
            .step(format!("{label}: aim"))
            .on_enter(crate::input::hover_named(aim))
            .each(move |world: &mut World, _, _| {
                crate::input::hover_named(re_aim.clone())(world);
            })
            .until(crate::predicate::pointer_over_node(name.to_string()))
            .diagnose(crate::predicate::pointer_hover_diagnosis(name.to_string()))
            .deadline(deadline)
            .add()
            .step(format!("{label}: press"))
            .on_enter(crate::input::press_mouse(MouseButton::Left))
            .until(crate::predicate::pointer_pressed())
            .deadline(deadline)
            .add()
            .step(format!("{label}: release"))
            .on_enter(crate::input::release_mouse(MouseButton::Left))
            .until(landed)
            .deadline(deadline)
            .add()
    }

    /// Append the two beats of a DOUBLE CLICK on the UI node called `name`:
    /// wait for it to lay out, then press-release-press-release it.
    ///
    /// The gesture is ONE step, not a beat per press, because the window it has
    /// to land inside is wall-clock. A widget that reads a double click times
    /// the two clicks on `Time<Real>` - the editor's scene tree allows 0.5s -
    /// while a beat costs several frames of harness bookkeeping: entry, the
    /// frame its ack arrives, and the frame that advances it. Under a software
    /// renderer that is most of a second per beat, so four beats put the two
    /// clicks over a second apart and the widget reads two singles.
    ///
    /// Inside one step the presses are driven a frame at a time, which puts the
    /// two RELEASES - what a widget acts on - exactly two frames apart at any
    /// frame rate. Each is still its own frame: a press and a release in the
    /// SAME frame activate nothing, because the press is a message the picking
    /// backend does not read until the next `PreUpdate`.
    ///
    /// `deadline` is the stall bound for each of the two beats.
    pub fn double_click_named(self, label: &str, name: &str, deadline: f32) -> Self {
        let second = name.to_string();
        let first = name.to_string();
        self.step(format!("{label}: the widget is up"))
            .until(crate::predicate::ui_node_present(name.to_string()))
            .diagnose(crate::input::ui_node_diagnosis(name.to_string()))
            .deadline(deadline)
            .add()
            .step(format!("{label}: double click"))
            .on_enter(crate::input::click_named(first))
            // The step's own frame index, which the driver already keeps: frame
            // 1 is the one after entry, so the gesture reads press (on entry),
            // release, press, release on four consecutive frames.
            .each(move |world: &mut World, _, frame| match frame {
                1 | 3 => crate::input::release_mouse(MouseButton::Left)(world),
                2 => crate::input::click_named(second.clone())(world),
                _ => {}
            })
            .until(crate::predicate::and(
                crate::predicate::frames(4),
                crate::predicate::pointer_released(),
            ))
            .deadline(deadline)
            .add()
    }

    /// Attach `f` as the per-frame action of every step that does not define
    /// its own [`each`](StepBuilder::each).
    ///
    /// The whole-script shorthand for a single driving closure. It runs every
    /// frame in `PreUpdate`, after Bevy has collected input for the frame
    /// (`InputSystems`) but before the game's `Update` input systems read it,
    /// with `&mut World` and the elapsed seconds IN THE CURRENT STEP. So it can
    /// poke input resources (`ButtonInput`, `Touches`, ...) or the game's own
    /// components and the game will see the poke this frame -- including a
    /// fresh `just_pressed`, which the input collection would otherwise have
    /// cleared.
    ///
    /// The closure runs in every state, so if it presses keys a menu reacts to
    /// (an "any key to start" screen), gate it to the gameplay state to avoid
    /// tripping those transitions early:
    /// `if *world.resource::<State<GameState>>().get() != GameState::Playing { return; }`.
    pub fn input(mut self, f: impl Fn(&mut World, f32, u32) + Send + Sync + 'static) -> Self {
        self.input = Some(Arc::new(f));
        self
    }

    /// Repeat the script from the step called `name` while OTHER completion
    /// collectors are still pending: at the last step, instead of reporting
    /// done, write an [`AutopilotLoop`] message, run the
    /// [`on_loop`](Self::on_loop) hook, jump the cursor back and zero the
    /// clocks - a frame capture then measures repeated ACTIVITY instead of an
    /// idle tail. Reports done normally once nothing else is pending, and
    /// finishes the moment that happens rather than at the cycle's end.
    ///
    /// Panics at build time when no step carries `name`: a typo would silently
    /// turn a looping run into a one-shot one.
    pub fn loop_from(mut self, name: impl Into<String>) -> Self {
        self.loop_from = Some(name.into());
        self
    }

    /// Run `f` on every [`loop_from`](Self::loop_from) restart, before the
    /// script resumes: the in-driver reset hook (reload the scene, zero the
    /// script's own resource). The [`AutopilotLoop`] message is written too,
    /// for resets that would rather be a system.
    pub fn on_loop(mut self, f: impl Fn(&mut World) + Send + Sync + 'static) -> Self {
        self.on_loop = Some(Arc::new(f));
        self
    }
}

/// One step, mid-construction. Created by [`AutopilotPlugin::step`] and closed
/// by [`add`](Self::add), which returns the plugin so steps chain.
pub struct StepBuilder<S: States + FreelyMutableState> {
    plugin: AutopilotPlugin<S>,
    step: Step<S>,
}

impl<S: States + FreelyMutableState> StepBuilder<S> {
    /// Set the app's state on entry. Skipped when the run is already in that
    /// state, so a step never opens with a spurious `OnExit`/`OnEnter` pair.
    pub fn enter(mut self, state: S) -> Self {
        self.step.enter = Some(state);
        self
    }

    /// Append `f` to the actions run once on entry, after the state has been
    /// set. Actions run in call order. The [`input`](crate::input) actions are
    /// built for this slot.
    pub fn on_enter(mut self, f: impl Fn(&mut World) + Send + Sync + 'static) -> Self {
        self.step.on_enter.push(Arc::new(f));
        self
    }

    /// Run `f` every frame this step is current, with the elapsed seconds IN
    /// THIS STEP and the count of driven frames it has had (1 on the first).
    /// Takes precedence over a plugin-wide [`input`](AutopilotPlugin::input)
    /// closure.
    ///
    /// The frame index is for a gesture that must act on PARTICULAR frames -
    /// see [`double_click_named`](AutopilotPlugin::double_click_named), whose
    /// two releases have to land two frames apart at any frame rate. Ignore it
    /// (`|world, _, _|`) for a closure that just drives every frame.
    pub fn each(mut self, f: impl Fn(&mut World, f32, u32) + Send + Sync + 'static) -> Self {
        self.step.each = Some(Arc::new(f));
        self
    }

    /// Explain a stall: run `f` when this step's [`deadline`](Self::deadline)
    /// expires and append what it returns to the abort message.
    ///
    /// A predicate answers only true or false, so a step that timed out says
    /// how long it waited and nothing about WHY. Where the wait collapses
    /// several distinct failures into one `false` - a node that does not exist,
    /// one that exists and is hidden, one that exists and has not laid out -
    /// this is where they are told apart. Run once, on the abort path only, so
    /// it may cost whatever the answer costs.
    pub fn diagnose(mut self, f: impl Fn(&World) -> String + Send + Sync + 'static) -> Self {
        self.step.diagnose = Some(Arc::new(f));
        self
    }

    /// Advance the step the first frame `predicate` holds. Unset, the step
    /// advances on the next driven frame.
    pub fn until(mut self, predicate: Arc<Predicate>) -> Self {
        self.step.until = predicate;
        self
    }

    /// Abort the run if the step is still current `seconds` after it began.
    /// The expiry is an ERROR exit naming the step, never a completion - see
    /// the module docs on sizing it under the run-level deadline.
    ///
    /// REAL seconds, not the app's. A host that pauses its own clock - Nova
    /// stops `Time<Virtual>` behind the pause overlay and the ship computer -
    /// would otherwise freeze every deadline set from there on, and a stalled
    /// beat would hold the run open until something outside it gave up.
    pub fn deadline(mut self, seconds: f32) -> Self {
        self.step.deadline = Some(seconds);
        self
    }

    /// Close the step and hand the plugin back for the next one.
    pub fn add(mut self) -> AutopilotPlugin<S> {
        self.plugin.steps.push(self.step);
        self.plugin
    }
}

/// The driver's clocks, kept in the world (unlike [`AutopilotState`], which is
/// removed for the duration of the frame's actions) so the time predicates can
/// read them.
#[derive(Resource, Default)]
pub(crate) struct AutopilotClock {
    /// Seconds since the run started, or since the last loop restart.
    pub(crate) elapsed: f32,
    /// Seconds since the current step was entered.
    pub(crate) step_elapsed: f32,
    /// Driven frames since the current step was entered.
    pub(crate) step_frames: u32,
    /// REAL seconds since the current step was entered - the clock a deadline
    /// is measured on.
    ///
    /// Not [`step_elapsed`](Self::step_elapsed): that runs on the app's own
    /// `Time`, which Nova PAUSES with the game. A walk that opens the pause
    /// overlay or the ship computer freezes it, and every deadline set past
    /// that point stops counting - a stalled beat then holds the run open
    /// until the outer harness kills it, with nothing naming the step.
    pub(crate) step_real: f32,
}

/// Internal driver state; kept out of the prelude per the crate conventions.
#[derive(Resource)]
struct AutopilotState<S: States + FreelyMutableState> {
    steps: Vec<Step<S>>,
    index: usize,
    /// Whether the current step's entry (state set + `on_enter`) has run.
    entered: bool,
    done: bool,
    loop_from: Option<usize>,
    on_loop: Option<Arc<EnterFn>>,
    loops: u32,
}

impl<S: States + FreelyMutableState> Plugin for AutopilotPlugin<S> {
    fn build(&self, app: &mut App) {
        if std::env::var(AUTOPILOT_ENV).is_err() {
            return;
        }
        if self.steps.is_empty() {
            warn!("AutopilotPlugin: {AUTOPILOT_ENV} set but the script is empty; doing nothing");
            return;
        }

        debug!(
            "AutopilotPlugin: build ({AUTOPILOT_ENV} active, {} steps)",
            self.steps.len()
        );

        let loop_from = self.loop_from.as_ref().map(|name| {
            self.steps
                .iter()
                .position(|step| step.name == *name)
                .unwrap_or_else(|| {
                    panic!(
                        "AutopilotPlugin: loop_from(\"{name}\") names no step; the script has {:?}",
                        self.steps.iter().map(|step| &step.name).collect::<Vec<_>>()
                    )
                })
        });

        // The plugin-wide input closure is the `each` of every step that did
        // not declare one, so `hold(...).input(...)` keeps meaning what it did.
        let steps = self
            .steps
            .iter()
            .map(|step| Step {
                each: step.each.clone().or_else(|| self.input.clone()),
                ..step.clone()
            })
            .collect();

        app.insert_resource(AutopilotState::<S> {
            steps,
            index: 0,
            entered: false,
            done: false,
            loop_from,
            on_loop: self.on_loop.clone(),
            loops: 0,
        });
        app.init_resource::<AutopilotClock>();
        app.add_message::<AutopilotLoop>();
        completion::register(app, completion::AUTOPILOT);
        // Must run after `InputSystems`, which clears `just_pressed`
        // every frame - only then does a synthesized press survive into the
        // game's Update systems.
        app.add_systems(PreUpdate, autopilot_drive::<S>.after(InputSystems));
        // A driven run owns the pointer: a real cursor event landing between a
        // press beat and its release beat would cancel the click silently
        // (`crate::input`).
        crate::input::register_pointer_pin(app);
    }
}

/// Exclusive driver: enters each step, runs its actions, advances it when its
/// predicate holds, aborts it when its deadline expires, and reports done after
/// the last one.
///
/// Exclusive because the step actions take `&mut World`; the driver state is
/// removed for the duration so they have unencumbered world access.
fn autopilot_drive<S: States + FreelyMutableState>(world: &mut World) {
    let mut st = world
        .remove_resource::<AutopilotState<S>>()
        .expect("AutopilotState is inserted by AutopilotPlugin::build");

    // Stay inert once finished - do NOT index past the end of the script
    // while the completion watcher waits on other collectors.
    if st.done {
        world.insert_resource(st);
        return;
    }

    // Entry gets its own frame, so the state transition has applied (and
    // the entry action's effects are visible) before any predicate is polled.
    if !st.entered {
        let step = st.steps[st.index].clone();
        if let Some(state) = &step.enter {
            // Skip the set when already in that state, or the step opens
            // with a spurious OnExit/OnEnter of it.
            if *world.resource::<State<S>>().get() != *state {
                world.resource_mut::<NextState<S>>().set(state.clone());
            }
        }
        info!("autopilot: step `{}` begins", step.name);
        for on_enter in &step.on_enter {
            on_enter(world);
        }
        st.entered = true;
        let mut clock = world.resource_mut::<AutopilotClock>();
        clock.step_elapsed = 0.0;
        clock.step_frames = 0;
        clock.step_real = 0.0;
        world.insert_resource(st);
        return;
    }

    let dt = world.resource::<Time>().delta_secs();
    let real = world.resource::<Time<Real>>().delta_secs();
    let (elapsed, step_elapsed, step_frames, step_real) = {
        let mut clock = world.resource_mut::<AutopilotClock>();
        clock.elapsed += dt;
        clock.step_elapsed += dt;
        clock.step_frames += 1;
        clock.step_real += real;
        (
            clock.elapsed,
            clock.step_elapsed,
            clock.step_frames,
            clock.step_real,
        )
    };

    let step = st.steps[st.index].clone();
    if let Some(each) = &step.each {
        each(world, step_elapsed, step_frames);
    }

    // While looping, finish the moment other collectors are done rather
    // than at the cycle's end - a slow cycle otherwise wastes up to its full
    // length and can straddle the deadline into a false laggard.
    if st.loops > 0
        && !world
            .resource::<completion::HarnessCompletion>()
            .others_pending(completion::AUTOPILOT)
    {
        info!(
            "autopilot: collectors done after {} loop(s); cycle complete, no panic (t={elapsed:.1}s)",
            st.loops
        );
        world
            .resource_mut::<completion::HarnessCompletion>()
            .done(completion::AUTOPILOT);
        st.done = true;
        world.insert_resource(st);
        return;
    }

    if !(step.until)(world) {
        if step.deadline.is_some_and(|deadline| step_real >= deadline) {
            let state = world.resource::<State<S>>().get().clone();
            let why = step
                .diagnose
                .as_ref()
                .map(|diagnose| format!(" - {}", diagnose(world)))
                .unwrap_or_default();
            // An expired step is an ABORT, not a completion - error exits
            // do not negotiate with the other collectors, and the driver must
            // never report done for a script that stalled.
            error!(
                "autopilot: step `{}` stalled after {step_real:.1}s \
                 (run {elapsed:.1}s, state {state:?}){why}",
                step.name
            );
            world.write_message(AppExit::error());
            st.done = true;
        }
        world.insert_resource(st);
        return;
    }

    st.index += 1;
    if st.index < st.steps.len() {
        st.entered = false;
        world.insert_resource(st);
        return;
    }

    let others_pending = world
        .resource::<completion::HarnessCompletion>()
        .others_pending(completion::AUTOPILOT);
    match st.loop_from {
        Some(from) if others_pending => {
            st.loops += 1;
            info!(
                "autopilot: cycle {} restarting from step `{}` - other collectors still pending",
                st.loops, st.steps[from].name
            );
            world.write_message(AutopilotLoop);
            if let Some(on_loop) = st.on_loop.clone() {
                on_loop(world);
            }
            st.index = from;
            st.entered = false;
            let mut clock = world.resource_mut::<AutopilotClock>();
            clock.elapsed = 0.0;
            clock.step_elapsed = 0.0;
            clock.step_frames = 0;
        }
        _ => {
            info!("autopilot: cycle complete, no panic (t={elapsed:.1}s)");
            world
                .resource_mut::<completion::HarnessCompletion>()
                .done(completion::AUTOPILOT);
            st.done = true;
        }
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
    use crate::{
        input::press_key,
        predicate::{elapsed, resource_where, state_is},
    };

    #[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
    enum TestState {
        #[default]
        Boot,
        Playing,
        Over,
    }

    /// The timed beat, spelled once for the tests that need several of them.
    ///
    /// It used to be public API (`AutopilotPlugin::hold`) and had no caller
    /// outside this module: the fleet writes `.step(..).enter(..)
    /// .until(elapsed(..))` out, because a beat worth naming is worth naming.
    impl AutopilotPlugin<TestState> {
        fn hold(self, state: TestState, seconds: f32) -> Self {
            let name = format!("hold:{state:?}");
            self.step(name).enter(state).until(elapsed(seconds)).add()
        }
    }

    /// Arm the plugin for the whole test binary. Set once, never removed and
    /// never with a second value, so the parallel test threads cannot observe
    /// disagreeing environments.
    fn arm() {
        static ARM: Once = Once::new();
        ARM.call_once(|| std::env::set_var(AUTOPILOT_ENV, "1"));
    }

    /// Frames are 1/60s of MANUAL time: `Time` is rewritten from `Time<Real>`
    /// every frame, so a hand-advanced clock would be stomped and the script
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

    /// The world fact a predicate step waits on, flipped by the test rather
    /// than by the clock.
    #[derive(Resource, Default)]
    struct Gate(bool);

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
        app.init_resource::<Gate>();
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

    /// The whole point of the rewrite: the advance condition is an OBSERVATION,
    /// and the clock is only one observation among them. With time running and
    /// the fact false the driver stays put; with time stopped and the fact true
    /// it advances anyway.
    #[test]
    fn step_advances_only_when_its_predicate_holds() {
        let mut app = app(AutopilotPlugin::new()
            .step("wait for the gate")
            .enter(TestState::Playing)
            .until(resource_where::<Gate>(|gate| gate.0))
            .add()
            .step("after")
            .enter(TestState::Over)
            .until(elapsed(0.05))
            .add());

        // Seconds of clock, none of them the condition.
        run(&mut app, 60);
        assert_eq!(
            app.world().resource::<Seen>().states,
            vec![TestState::Playing],
            "the clock ran for a second and the step did not advance"
        );

        // Stop time dead, then satisfy the predicate: it advances anyway.
        app.world_mut()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.world_mut().resource_mut::<Gate>().0 = true;
        run(&mut app, 4);
        assert_eq!(
            app.world().resource::<Seen>().states,
            vec![TestState::Playing, TestState::Over],
            "a satisfied predicate advances the step with the clock stopped"
        );
    }

    #[test]
    fn a_timed_step_holds_its_state_for_the_elapsed_seconds() {
        let mut app = app(AutopilotPlugin::new()
            .hold(TestState::Playing, 0.1)
            .hold(TestState::Over, 0.1));

        // Entry frames, then 6 frames per 0.1s step, plus slack.
        let observed = run(&mut app, 20);

        assert_eq!(
            app.world().resource::<Seen>().states,
            // No `Boot`: the driver's first frame sets the first step's state
            // and `StateTransition` applies it before `Update` ever reads the
            // state, which is what keeps the run from opening on a spurious
            // OnExit/OnEnter of the default state.
            vec![TestState::Playing, TestState::Over],
            "the timed steps advance the state machine in order"
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
    fn a_step_starting_in_the_current_state_does_not_re_enter_it() {
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
            "the script still advances off the first step"
        );
    }

    /// Both action hooks are scheduled after `InputSystems`, so a press either
    /// one synthesizes is still `just_pressed` when `Update` reads it. Ordered
    /// the other way, input collection would clear the press in the same frame
    /// and every synthesized key would be invisible to gameplay.
    #[test]
    fn step_actions_run_after_input_collection() {
        // The per-frame hook, via the `input(f)` sugar that attaches it to
        // every step.
        let mut each_app =
            app(AutopilotPlugin::new()
                .hold(TestState::Playing, 10.0)
                .input(|world, _, _| {
                    world
                        .resource_mut::<ButtonInput<KeyCode>>()
                        .press(KeyCode::Space);
                }));

        run(&mut each_app, 4);

        assert!(
            each_app.world().resource::<Seen>().space_just_pressed,
            "the `each` closure runs after InputSystems, so its press is still \
             just_pressed when Update reads it"
        );

        // The entry hook, which fires on the step's first frame only and so
        // has exactly one frame in which to be seen.
        let mut on_enter_app = app(AutopilotPlugin::new()
            .step("press on entry")
            .enter(TestState::Playing)
            .on_enter(press_key(KeyCode::Space))
            .until(elapsed(10.0))
            .add());

        run(&mut on_enter_app, 4);

        assert!(
            on_enter_app.world().resource::<Seen>().space_just_pressed,
            "the `on_enter` action runs after InputSystems too, so its press \
             is just_pressed on the entry frame rather than swallowed by it"
        );
    }

    #[test]
    fn multiple_on_enter_actions_run_in_call_order() {
        let actions = Arc::new(Mutex::new(Vec::new()));
        let first = Arc::clone(&actions);
        let second = Arc::clone(&actions);
        let mut app = app(AutopilotPlugin::new()
            .step("compose entry actions")
            .on_enter(move |_| first.lock().unwrap().push("first"))
            .on_enter(move |_| second.lock().unwrap().push("second"))
            .add());

        run(&mut app, 4);

        assert_eq!(*actions.lock().unwrap(), ["first", "second"]);
    }

    /// The per-step `each` clock is STEP-relative, which is what the scripts'
    /// hand-rolled `playing_since` offsets existed to compute.
    #[test]
    fn the_each_clock_is_step_relative() {
        let seen = Arc::new(Mutex::new(Vec::<f32>::new()));
        let recorder = Arc::clone(&seen);
        let mut app = app(AutopilotPlugin::new()
            .hold(TestState::Playing, 0.1)
            .step("second")
            .enter(TestState::Over)
            .each(move |_, t, _| recorder.lock().unwrap().push(t))
            .until(elapsed(0.1))
            .add());

        run(&mut app, 20);

        let observed = seen.lock().unwrap().clone();
        assert!(
            !observed.is_empty() && observed[0] < 0.05,
            "the second step's clock starts near zero, not at the run's \
             elapsed: {observed:?}"
        );
    }

    /// A step whose predicate never holds must ABORT with its name, not idle
    /// out and pass.
    #[test]
    fn stalled_step_aborts_naming_the_step() {
        let mut app = app(AutopilotPlugin::new()
            .step("wait for the gate")
            .enter(TestState::Playing)
            .until(resource_where::<Gate>(|gate| gate.0))
            .deadline(0.1)
            .add());

        let observed = run(&mut app, 15);

        assert_eq!(observed.len(), 1, "the stalled step exits exactly once");
        assert_ne!(
            observed[0],
            AppExit::Success,
            "a step that never satisfied its predicate ABORTS; it must not pass"
        );
        assert!(
            app.world()
                .resource::<completion::HarnessCompletion>()
                .is_pending(completion::AUTOPILOT),
            "the abort does not fake a completion report"
        );
    }

    /// A deadline is REAL seconds. A host that pauses its own clock - Nova
    /// stops `Time<Virtual>` behind the pause overlay and the ship computer -
    /// must not be able to freeze the backstop: a walk that stalls in a paused
    /// screen would otherwise hold the run open forever with nothing naming
    /// the step (`system_headless_drag` did, for 180 seconds, on a beat whose
    /// widget had been renamed).
    #[test]
    fn a_deadline_still_expires_while_the_app_clock_is_paused() {
        let mut app = app(AutopilotPlugin::new()
            .step("wait for something that never comes")
            .until(resource_where::<Gate>(|gate| gate.0))
            .deadline(0.1)
            .add());
        app.world_mut().resource_mut::<Time<Virtual>>().pause();

        let observed = run(&mut app, 20);

        assert_eq!(
            observed,
            vec![AppExit::error()],
            "the deadline runs on the wall, not on the clock the app paused"
        );
    }

    /// A deadline is a backstop, not a timer: a step that satisfies its
    /// predicate in time advances normally and the run passes.
    #[test]
    fn a_deadline_that_is_not_reached_changes_nothing() {
        let mut app = app(AutopilotPlugin::new()
            .step("reach playing")
            .enter(TestState::Playing)
            .until(state_is(TestState::Playing))
            .deadline(5.0)
            .add());

        let observed = run(&mut app, 10);

        assert_eq!(observed, vec![AppExit::Success]);
        assert!(!app
            .world()
            .resource::<completion::HarnessCompletion>()
            .is_pending(completion::AUTOPILOT));
    }

    #[test]
    fn loop_point_restarts_at_the_labeled_step_and_resets() {
        let elapsed_seen = Arc::new(Mutex::new(Vec::<f32>::new()));
        let recorder = Arc::clone(&elapsed_seen);
        let resets = Arc::new(Mutex::new(0usize));
        let counter = Arc::clone(&resets);
        let mut app = app(AutopilotPlugin::new()
            .hold(TestState::Playing, 0.1)
            .loop_from("hold:Playing")
            .on_loop(move |_| *counter.lock().unwrap() += 1)
            .input(move |_, t, _| recorder.lock().unwrap().push(t)));
        // A second, slower collector: what the autopilot loops FOR.
        completion::register(&mut app, crate::loops::LOOP_CAPTURE);

        let observed = run(&mut app, 30);

        assert!(
            app.world().resource::<Seen>().loops >= 2,
            "the cycle repeats while the other collector is pending"
        );
        assert_eq!(
            *resets.lock().unwrap(),
            app.world().resource::<Seen>().loops,
            "the on_loop hook runs once per restart, alongside the message"
        );
        let seen = elapsed_seen.lock().unwrap().clone();
        assert!(
            seen.windows(2).any(|w| w[1] < w[0]),
            "the step clock zeroes on restart, so the script sees a fresh \
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
            .done(crate::loops::LOOP_CAPTURE);
        // Two frames for the driver to notice and the watcher to decide -
        // NOT the rest of the cycle.
        assert_eq!(
            run(&mut app, 3),
            vec![AppExit::Success],
            "clearing the other collector finishes the cycle immediately"
        );
    }

    /// A `loop_from` naming no step is a script bug, and a silent one-shot run
    /// would hide it behind a passing capture.
    #[test]
    #[should_panic(expected = "loop_from(\"nope\") names no step")]
    fn loop_from_an_unknown_step_fails_the_build() {
        let _ = app(AutopilotPlugin::new()
            .hold(TestState::Playing, 0.1)
            .loop_from("nope"));
    }
}

/// The `AutopilotPlugin` driver, its `StepBuilder` vocabulary, `AutopilotLoop`
/// and `AUTOPILOT_ENV`.
pub mod prelude {
    pub use super::{AutopilotLoop, AutopilotPlugin, StepBuilder, AUTOPILOT_ENV};
}
