//! The completion protocol: how a driver reports that its run finished, and how
//! a host app observes that and shuts down.
//!
//! Before this module, every harness actor wrote [`AppExit`] on its own
//! clock - the autopilot after its wall-second timeline, a frame capture
//! after its frame-count window - and whoever finished first ended the app,
//! discarding everyone else's data. The races were resolved per game by
//! folklore (conditionally adding one plugin or the other), which is how an
//! 11-frames-short capture silently lost 229 samples downstream.
//!
//! The protocol: every armed collector REGISTERS itself at plugin build and
//! reports DONE when its own clock completes. A watcher writes
//! [`AppExit::Success`] only when the pending set is EMPTY. A deadline
//! backstop ([`DEADLINE_ENV`], default [`DEFAULT_DEADLINE_SECS`]) exits
//! with [`AppExit::error`] NAMING the laggards, so a supervisor sees
//! "capture never completed" in the log instead of a silent hang or kill.
//!
//! Two rules keep it honest:
//! - SUCCESS exits negotiate: no registered collector may write
//!   `AppExit::Success` itself - it reports done and the watcher decides.
//! - ERROR exits abort: a collector that FAILS (a loop the encoder could not
//!   write, an expired self-completing script) writes `AppExit::error`
//!   directly - an abort is not a completion and must not wait for anyone.
//!
//! Registration is env-gated with the collectors themselves: an unarmed
//! harness registers nothing, the resource never exists, and the watcher is
//! never added - a normal run pays nothing.
//!
//! ## Joining the protocol
//!
//! A caller-owned collector joins in two moves: [`register`] from
//! `Plugin::build`, behind its own armed check, and
//! [`HarnessCompletion::done`] when its work finishes. It never writes
//! `AppExit::Success` itself.
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use nova_autopilot::completion::{self, HarnessCompletion};
//!
//! /// This collector's name in the pending set. Must outlive the app.
//! const FRAME_LOG: &str = "frame_log";
//!
//! struct FrameLogPlugin;
//!
//! impl Plugin for FrameLogPlugin {
//!     fn build(&self, app: &mut App) {
//!         // Unarmed: add nothing and register nothing. Registering here
//!         // would hold the exit open forever.
//!         if std::env::var("MY_FRAME_LOG").is_err() {
//!             return;
//!         }
//!         completion::register(app, FRAME_LOG);
//!         app.add_systems(Update, log_frames);
//!     }
//! }
//!
//! fn log_frames(mut frames: Local<u32>, mut completion: ResMut<HarnessCompletion>) {
//!     *frames += 1;
//!     if *frames == 600 {
//!         // Report done and let the watcher decide the exit, so a slower
//!         // collector still gets its frames.
//!         completion.done(FRAME_LOG);
//!     }
//! }
//! ```

use bevy::prelude::*;

/// The completion protocol's own systems in `Last`.
///
/// Named so that anything READING [`AppExit`] can order itself after the
/// writer. Bevy exits after the frame in which `AppExit` is written - there is
/// no next frame to catch up in - so an unordered reader in the same schedule
/// misses the exit entirely on an unfavourable ordering.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AutopilotCompletionSystems {
    /// Writes [`AppExit`] once the pending set empties or the deadline
    /// expires.
    Watch,
}

/// Collector name the scripted autopilot driver registers under.
pub const AUTOPILOT: &str = "autopilot";

/// Environment variable overriding the completion deadline, in seconds.
pub const DEADLINE_ENV: &str = "NOVA_AUTOPILOT_DEADLINE";

/// Default seconds before the watcher gives up on pending collectors and
/// error-exits naming them. Deliberately generous (collectors own their own
/// pacing) but meant to resolve BELOW any outer supervisor timeout, so the
/// named-laggards log line wins over a SIGKILL.
pub const DEFAULT_DEADLINE_SECS: f32 = 120.0;

/// The pending-collector set. Exists only once something registers.
#[derive(Resource, Debug)]
pub struct HarnessCompletion {
    pending: Vec<&'static str>,
    deadline_secs: f32,
    elapsed: f32,
    exited: bool,
}

impl Default for HarnessCompletion {
    fn default() -> Self {
        Self {
            pending: Vec::new(),
            deadline_secs: std::env::var(DEADLINE_ENV)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_DEADLINE_SECS),
            elapsed: 0.0,
            exited: false,
        }
    }
}

impl HarnessCompletion {
    /// Report `name`'s work complete. Unknown or already-done names warn
    /// (a protocol bug worth seeing) instead of panicking a live run.
    pub fn done(&mut self, name: &str) {
        match self.pending.iter().position(|p| *p == name) {
            Some(index) => {
                self.pending.remove(index);
                debug!(
                    "harness completion: {name} done ({} still pending)",
                    self.pending.len()
                );
            }
            None => warn!("harness completion: done({name}) but it is not pending"),
        }
    }

    /// Whether `name` has registered and not yet reported done.
    pub fn is_pending(&self, name: &str) -> bool {
        self.pending.contains(&name)
    }

    /// Whether any collector OTHER than `name` is still pending - the loop
    /// condition for a collector that can repeat its work (an autopilot
    /// cycling its scene) while slower collectors (a frame capture) finish.
    pub fn others_pending(&self, name: &str) -> bool {
        self.pending.iter().any(|p| *p != name)
    }
}

/// Register `name` as a pending collector and make sure the watcher runs.
/// Call from `Plugin::build` AFTER the collector's own armed check - an
/// unarmed collector must not register (it would hold the exit forever).
pub fn register(app: &mut App, name: &'static str) {
    let world = app.world_mut();
    // The resource's absence is what marks the FIRST registration. Adding the
    // watcher once per registrant would run it N times per frame, and its
    // `elapsed` accumulation would burn the deadline N times too fast.
    let first = world.get_resource::<HarnessCompletion>().is_none();
    let mut completion = world.get_resource_or_insert_with(HarnessCompletion::default);
    if completion.pending.contains(&name) {
        warn!("harness completion: {name} registered twice; ignoring the second");
        return;
    }
    completion.pending.push(name);
    if first {
        app.add_systems(
            Last,
            completion_watch.in_set(AutopilotCompletionSystems::Watch),
        );
    }
}

/// The exit decision, once per frame in `Last`: all done -> `AppExit::
/// Success`; deadline expired -> `AppExit::error` naming the laggards.
///
/// On `Time<Real>` and not the virtual clone the schedule's default `Time`
/// resolves to. This is a BACKSTOP against a run that is not progressing, and
/// a paused clock is one of the ways a run stops progressing: the pause overlay
/// and the ship computer both hold `Time<Virtual>`, so a script that opens
/// either and then waits on an `until` that never holds would have burned no
/// deadline at all and hung until a supervisor killed it, naming nothing.
/// (Inside a loop capture `Time<Real>` is itself pinned to the profile's frame
/// step, so the backstop counts rendered frames there; see the loops module.)
fn completion_watch(
    time: Res<Time<Real>>,
    completion: Option<ResMut<HarnessCompletion>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut completion) = completion else {
        return;
    };
    if completion.exited {
        return;
    }
    if completion.pending.is_empty() {
        info!("harness completion: all collectors done, exiting");
        exit.write(AppExit::Success);
        completion.exited = true;
        return;
    }
    completion.elapsed += time.delta_secs();
    if completion.elapsed >= completion.deadline_secs {
        error!(
            "harness completion: deadline ({}s) expired with collectors still \
             pending: {:?}",
            completion.deadline_secs, completion.pending
        );
        exit.write(AppExit::error());
        completion.exited = true;
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::schedule::SingleThreadedExecutor;

    use super::*;
    use crate::log_capture::capturing_logs;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app
    }

    fn exits(app: &mut App) -> Vec<AppExit> {
        app.world_mut()
            .resource_mut::<Messages<AppExit>>()
            .drain()
            .collect()
    }

    #[test]
    fn exits_success_only_when_every_collector_is_done() {
        let mut app = app();
        register(&mut app, "a");
        register(&mut app, "b");
        app.update();
        app.update();
        assert!(exits(&mut app).is_empty(), "two pending: no exit");

        app.world_mut()
            .resource_mut::<HarnessCompletion>()
            .done("a");
        app.update();
        assert!(exits(&mut app).is_empty(), "one pending: still no exit");

        app.world_mut()
            .resource_mut::<HarnessCompletion>()
            .done("b");
        app.update();
        assert_eq!(
            exits(&mut app),
            vec![AppExit::Success],
            "empty pending set: negotiated success"
        );
        app.update();
        assert!(exits(&mut app).is_empty(), "exit fires exactly once");
    }

    #[test]
    fn single_collector_parity_with_the_old_direct_exit() {
        let mut app = app();
        register(&mut app, AUTOPILOT);
        app.world_mut()
            .resource_mut::<HarnessCompletion>()
            .done(AUTOPILOT);
        app.update();
        assert_eq!(exits(&mut app), vec![AppExit::Success]);
    }

    #[test]
    fn deadline_error_exits_naming_the_laggards() {
        let (observed, logs) = capturing_logs(|| {
            let mut app = app();
            register(&mut app, crate::loops::LOOP_CAPTURE);
            // The capture sink is thread-local, so the watcher has to run on
            // THIS thread rather than on a task-pool worker.
            app.edit_schedule(Last, |schedule| {
                schedule.set_executor(SingleThreadedExecutor::new());
            });
            app.world_mut()
                .resource_mut::<HarnessCompletion>()
                .deadline_secs = 0.0;
            app.update();
            app.update();
            exits(&mut app)
        });
        assert_eq!(observed.len(), 1);
        assert_ne!(
            observed[0],
            AppExit::Success,
            "an expired deadline is an ERROR exit"
        );
        assert!(
            logs.contains(crate::loops::LOOP_CAPTURE),
            "the deadline error must NAME the laggards; logged: {logs}"
        );
    }

    /// The backstop keeps counting while the game clock is held.
    ///
    /// A pause overlay and the ship computer both pause `Time<Virtual>`. A
    /// script that opens one and waits on an `until` that never holds is
    /// exactly the run this deadline exists to end, so it must not be the run
    /// the deadline sleeps through.
    #[test]
    fn a_paused_game_clock_does_not_stop_the_deadline() {
        let mut app = app();
        register(&mut app, AUTOPILOT);
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        app.world_mut()
            .resource_mut::<HarnessCompletion>()
            .deadline_secs = f32::MAX;
        for _ in 0..16 {
            app.update();
        }

        assert_eq!(
            app.world().resource::<Time<Virtual>>().delta_secs(),
            0.0,
            "the fixture must actually be holding the game clock"
        );
        assert!(
            app.world().resource::<HarnessCompletion>().elapsed > 0.0,
            "the backstop burned no time while the game clock was held"
        );
    }

    #[test]
    fn the_deadline_clock_tracks_wall_time_whatever_the_collector_count() {
        let mut app = app();
        register(&mut app, AUTOPILOT);
        register(&mut app, crate::loops::LOOP_CAPTURE);
        app.world_mut()
            .resource_mut::<HarnessCompletion>()
            .deadline_secs = f32::MAX;
        for _ in 0..64 {
            app.update();
        }
        let counted = app.world().resource::<HarnessCompletion>().elapsed;
        let wall = app.world().resource::<Time<Real>>().elapsed_secs();
        assert!(
            counted <= wall * 1.01,
            "the deadline clock ran at {}x wall time: one watcher per \
             registrant would expire the backstop N times early",
            counted / wall
        );
    }

    #[test]
    fn unknown_done_warns_but_does_not_poison_the_run() {
        let mut app = app();
        register(&mut app, "a");
        app.world_mut()
            .resource_mut::<HarnessCompletion>()
            .done("typo");
        assert!(app.world().resource::<HarnessCompletion>().is_pending("a"));
    }

    #[test]
    fn duplicate_registration_is_ignored() {
        let mut app = app();
        register(&mut app, "a");
        register(&mut app, "a");
        app.world_mut()
            .resource_mut::<HarnessCompletion>()
            .done("a");
        app.update();
        assert_eq!(
            exits(&mut app),
            vec![AppExit::Success],
            "one done clears a doubly-registered name"
        );
    }
}

/// `HarnessCompletion` and its `register` entry point, `AutopilotCompletionSystems`,
/// and the harness names and deadline knobs.
pub mod prelude {
    pub use super::{
        register, AutopilotCompletionSystems, HarnessCompletion, AUTOPILOT, DEADLINE_ENV,
        DEFAULT_DEADLINE_SECS,
    };
}
