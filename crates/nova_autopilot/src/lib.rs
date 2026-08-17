//! `nova_autopilot` is the standalone home for Nova Protocol's automation
//! drivers and the completion protocol they share. It depends on `bevy` only:
//! no `nova_*` crate, no `avian3d`, no game code at all.
//!
//! ## Ownership boundary
//!
//! This crate owns the drivers (scripted autopilot, settled-frame screenshot),
//! the [`capture`] primitive their scripts shoot with, and the completion
//! protocol that reports when a run has finished. It does not own anything
//! Nova-specific: the adapters - scenario
//! presets, camera posing, rigid-body freezing, overlay hiding - stay in
//! `nova_debug` and reach in through caller hooks. The same line runs through
//! the script vocabulary: generic predicates ([`predicate`]) and generic
//! pointer/key synthesis ([`input`]) live here; a predicate that names a Nova
//! type is built on them in `nova_debug::harness`.
//!
//! ## The step model
//!
//! A script is a list of NAMED STEPS, and a step advances the frame its
//! [`Predicate`](predicate::Predicate) holds - "the ship spawned", "the
//! scenario set this variable", "two seconds passed". Elapsed time is one
//! predicate among many, which is why
//! [`hold`](autopilot::AutopilotPlugin::hold) is sugar over the step model
//! rather than a second mechanism, and why a stall names the beat that stalled
//! instead of dumping a tuple of booleans. See [`autopilot`] for the parts of a
//! step and for how per-step deadlines relate to the run-level one.
//!
//! Nova-SHAPED choices (env var names, defaults, API vocabulary) do live here.
//! Nova TYPES do not. The drivers are generic over the app's state type,
//! `S: States + FreelyMutableState`, and that generic is what keeps
//! `nova_gameplay::GameStates` - and with it the whole game dependency tree -
//! out of this crate.
//!
//! ## The environment contract
//!
//! Every driver is inert unless its own variable is set, so a host app adds
//! the plugins unconditionally and a normal run pays nothing. Setting a
//! variable is what ARMS the driver; the value only matters where noted.
//!
//! | Variable | Arms | Read by | Value |
//! | --- | --- | --- | --- |
//! | `NOVA_AUTOPILOT` ([`AUTOPILOT_ENV`](autopilot::AUTOPILOT_ENV)) | the scripted state driver | [`AutopilotPlugin`](autopilot::AutopilotPlugin) | any (presence only) |
//! | `NOVA_SHOT` ([`SCREENSHOT_ENV`](screenshot::SCREENSHOT_ENV)) | the single settled-frame capture, UNLESS `NOVA_AUTOPILOT` is also set - both drivers write `NextState`, so the autopilot wins and [`ScreenshotPlugin`](screenshot::ScreenshotPlugin) stands down with a warning | [`ScreenshotPlugin`](screenshot::ScreenshotPlugin) | `WxH` overrides the window size; anything else is a plain toggle |
//! | `NOVA_CAPTURE` ([`CAPTURE_ENV`](capture::CAPTURE_ENV)) | the CAPTURE path of a script that has one - it takes its shots instead of driving straight through | [`capturing`](capture::capturing), which the script reads while building its steps | any (presence only) |
//! | `NOVA_SHOT_DIR` ([`SHOT_DIR_ENV`](capture::SHOT_DIR_ENV)) | nothing on its own | [`capture_window`](capture::capture_window) | directory RELATIVE capture paths resolve under; absolute paths ignore it |
//! | `NOVA_AUTOPILOT_DEADLINE` ([`DEADLINE_ENV`](completion::DEADLINE_ENV)) | nothing on its own | the [`completion`] watcher | seconds before the run error-exits naming the laggards (default [`DEFAULT_DEADLINE_SECS`](completion::DEFAULT_DEADLINE_SECS)); the RUN-level backstop under a script's own per-step [`deadline`](autopilot::StepBuilder::deadline)s |
//!
//! `NOVA_SHOT` and `NOVA_CAPTURE` are deliberately distinct: a scripted
//! capture run and a one-off snapshot must never fight over the same window.
//!
//! ## The completion protocol
//!
//! One run can carry several collectors (a scripted autopilot, a frame
//! capture), each finishing on its own clock. Whoever exits first used
//! to discard everyone else's data, so the exit is negotiated instead - see
//! [`completion`]. Two rules:
//!
//! 1. **Register before the run starts.** A collector calls
//!    [`completion::register`] from `Plugin::build`, behind its own armed
//!    check. Nothing may join later, and an unarmed collector must not join at
//!    all (it would hold the exit open until the deadline).
//! 2. **The app exits only when every registrant reports done.** A collector
//!    reports [`HarnessCompletion::done`](completion::HarnessCompletion::done)
//!    and never writes `AppExit::Success` itself; the watcher writes it once
//!    the pending set empties. An ERROR exit is the exception - an abort is
//!    not a completion and waits for no one.
//!
//! ## Reading it end to end
//!
//! `examples/driven_app.rs` is the whole crate in one file: a real
//! `DefaultPlugins` app with its own state machine, driven through named
//! predicate steps by the autopilot and exited by the completion protocol,
//! importing no `nova_*` crate but this one. Run it with
//! `NOVA_AUTOPILOT=1 cargo run -p nova_autopilot --example driven_app`;
//! `tests/autopilot_example.rs` runs the same thing headless and asserts on
//! the exit status and the log lines.

#![warn(missing_docs)]

// No outer docs here: every module below carries its own `//!` docs, and an
// outer `///` would concatenate ahead of them and re-resolve their intra-doc
// links (`AppExit`, `AUTOPILOT_ENV`, `SCREENSHOT_ENV`) in THIS module's scope,
// where they do not exist. See 20260802-183340 REVIEW.md R1.3.
pub mod autopilot;
pub mod capture;
pub mod completion;
pub mod exit;
pub mod input;
#[cfg(test)]
mod log_capture;
pub mod predicate;
pub mod screenshot;

/// Glob-import surface: `use nova_autopilot::prelude::*`.
///
/// Every public item of the six modules is re-exported here verbatim, so a
/// caller never needs a module path.
///
/// [`capture_window`](capture::capture_window) is deliberately in here next to
/// the step vocabulary: shooting is a step's business, not a driver's.
///
/// Names are re-exported unaliased. Two share a name with something outside
/// `bevy::prelude` or inside it:
///
/// - [`ScreenshotPlugin`](screenshot::ScreenshotPlugin) shares its name with
///   Bevy's render-side plugin; neither is in `bevy::prelude`, so a glob import
///   of both preludes does not collide.
/// - [`not`](predicate::not) shares its name with Bevy's run-condition
///   combinator, which IS in `bevy::prelude`. A file globbing both must name
///   whichever it wants explicitly. (The predicate that would have been
///   `in_state` is called [`state_is`](predicate::state_is) for the same
///   reason - that clash is used everywhere, so it is spelled apart instead.)
pub mod prelude {
    pub use crate::{
        autopilot::{AutopilotLoop, AutopilotPlugin, StepBuilder, AUTOPILOT_ENV},
        capture::{
            capture_window, capturing, CaptureLog, CAPTURE_ENV, CAPTURE_RESOLUTION, SHOT_DIR_ENV,
        },
        completion::{
            register, HarnessCompletion, AUTOPILOT, DEADLINE_ENV, DEFAULT_DEADLINE_SECS, SCREENSHOT,
        },
        input::{
            assert_named_visible, click_at, click_named, hover_named, move_cursor, press_key,
            press_mouse, release_key, release_mouse, scroll_lines, scroll_pixels, type_text,
            ui_node_centre, ui_node_rect,
        },
        predicate::{
            and, any_entity, elapsed, frames, not, resource_where, shot_written, state_is,
            Predicate,
        },
        screenshot::{ScreenshotPlugin, MAX_WAIT_FRAMES, SCREENSHOT_ENV},
    };
}
