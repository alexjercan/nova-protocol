//! Headless smoke-test harness for nova examples.
//!
//! Thin nova-specific presets over the `bevy_common_systems` env-gated harness
//! plugins ([`AutopilotPlugin`] / [`ScreenshotPlugin`]), pinned to nova's
//! [`GameStates`] lifecycle. Both are inert unless their env var is set
//! (`BCS_AUTOPILOT` / `BCS_SHOT`), so an example adds them permanently and pays
//! nothing in a normal run.
//!
//! ## Why the autopilot does not force `Playing`
//!
//! Nova's `Loading -> Playing` transition is *asset-gated*: the loader flips it
//! in `OnEnter(GameAssetsStates::Loaded)`, not on any input. If the autopilot
//! force-set `Playing` on its own timeline it would either fire before the
//! `GameAssets` resource exists (panicking scene setup that reads it) or re-enter
//! `Playing` after the loader already did (double-running `OnEnter(Playing)`
//! setup). So [`nova_autopilot`] holds `Loading` on a single generous step
//! instead of forcing anything: the loader reaches `Playing` on its own within
//! that window, the run exercises gameplay (and any
//! [`input`](AutopilotPlugin::input) closure) there, and the autopilot exits
//! cleanly with `AppExit::Success` when the step ends. The
//! `nova harness: reached Playing` line (emitted by [`DebugPlugin`](crate::DebugPlugin)
//! under the autopilot) confirms the loader got there before the exit, so a run
//! that silently never leaves `Loading` fails the smoke test instead of passing.
//!
//! ## Usage
//!
//! Add the preset under the `debug` feature (the harness lives there); it is a
//! no-op unless `BCS_AUTOPILOT` is set, so leaving it in costs nothing:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use nova_debug::harness::nova_autopilot;
//! # fn add(app: &mut App) {
//! // No input needed -- just drive to Playing and exit without panic:
//! app.add_plugins(nova_autopilot());
//!
//! // Poke fire/thrust while in Playing (gate input to the gameplay state so it
//! // does not run during Loading):
//! app.add_plugins(nova_autopilot().input(|world, _elapsed| {
//!     use nova_gameplay::GameStates;
//!     if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
//!         return;
//!     }
//!     world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Space);
//! }));
//! # }
//! ```
//!
//! Run it headless:
//!
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 03_scenario --features debug
//! # look for: `nova harness: reached Playing`
//! #           `autopilot: cycle complete, no panic`
//! ```

use nova_gameplay::GameStates;

// Re-export the underlying harness plugins so examples can name/extend them
// (e.g. build a bespoke timeline) without reaching into bevy_common_systems.
pub use bevy_common_systems::debug::harness::prelude::{AutopilotPlugin, ScreenshotPlugin};

/// Seconds the [`nova_autopilot`] preset holds `Loading` before exiting. Must
/// comfortably outlast asset loading (the loader drives `Loading -> Playing` on
/// its own) so the run spends real time in `Playing` before the clean exit.
pub const NOVA_AUTOPILOT_SECS: f32 = 6.0;

/// Settle frames the [`nova_screenshot`] preset waits after `Playing` is
/// reached, so the scene and UI have a few frames to render before the capture.
pub const NOVA_SCREENSHOT_SETTLE_FRAMES: u32 = 30;

/// Env-gated autopilot preset for nova examples.
///
/// Holds `Loading` for [`NOVA_AUTOPILOT_SECS`] (the asset loader reaches
/// `Playing` within that window on its own -- see the module docs on why this
/// does not force the transition), then exits with `AppExit::Success`. Chain
/// [`input`](AutopilotPlugin::input) to poke fire/thrust while in `Playing`.
/// Inert unless `BCS_AUTOPILOT` is set.
pub fn nova_autopilot() -> AutopilotPlugin<GameStates> {
    AutopilotPlugin::new().hold(GameStates::Loading, NOVA_AUTOPILOT_SECS)
}

/// Env-gated screenshot preset for nova examples: advance to `Playing`, settle
/// [`NOVA_SCREENSHOT_SETTLE_FRAMES`] frames, capture the primary window to a
/// PNG, and exit. Inert unless `BCS_SHOT` is set (a `WxH` value also overrides
/// the window resolution). See [`ScreenshotPlugin`].
///
/// Unlike [`nova_autopilot`], this force-advances to `Playing` on the first
/// frame, so it is best used with examples that set their scene up in
/// `OnEnter(GameAssetsStates::Loaded)` (the nova scenario convention, e.g.
/// `03_scenario`) rather than `OnEnter(GameStates::Playing)`, which the early
/// forced transition would run before `GameAssets` is ready.
pub fn nova_screenshot() -> ScreenshotPlugin<GameStates> {
    ScreenshotPlugin::new(GameStates::Playing).settle_frames(NOVA_SCREENSHOT_SETTLE_FRAMES)
}
