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

// Re-export the underlying harness plugins so examples can name/extend them
// (e.g. build a bespoke timeline) without reaching into bevy_common_systems.
use bevy::prelude::*;
pub use bevy_common_systems::debug::harness::prelude::{AutopilotPlugin, ScreenshotPlugin};
use nova_gameplay::GameStates;
use nova_scenario::prelude::{ScenarioId, ScenarioLoaded};

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

/// Smoke-test assertion preset: fail a headless run if scenario init is broken.
///
/// A scenario-loading example passes `autopilot: cycle complete, no panic` even
/// if the scenario silently came up empty. This preset closes that gap: it
/// observes the [`ScenarioLoaded`] init-status payload and panics (which fails
/// the `BCS_AUTOPILOT` run with a non-zero exit) when init is trivial -- the
/// wrong scenario id, zero event handlers, or zero objects -- and, via a `fired`
/// flag checked on entering `Playing`, when the event never fires at all.
///
/// Add it under the `debug` feature next to [`nova_autopilot`], passing the id
/// the example expects to load:
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use nova_debug::harness::assert_scenario_loaded;
/// # fn add(app: &mut App) {
/// app.add_plugins(assert_scenario_loaded("asteroid_field"));
/// # }
/// ```
///
/// The assertion is an invariant every scenario-loading example already holds,
/// so it is harmless (a single observer) in a normal `cargo run` too. It expects
/// exactly one scenario to load, which fits the examples that load once and do
/// not switch scenarios within the autopilot window.
pub fn assert_scenario_loaded(expected_id: impl Into<ScenarioId>) -> ScenarioLoadedAssertPlugin {
    ScenarioLoadedAssertPlugin {
        expected_id: expected_id.into(),
    }
}

/// Plugin returned by [`assert_scenario_loaded`]. Construct it through that
/// preset rather than directly.
pub struct ScenarioLoadedAssertPlugin {
    expected_id: ScenarioId,
}

impl Plugin for ScenarioLoadedAssertPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ScenarioLoadAssertion {
            expected_id: self.expected_id.clone(),
            fired: false,
        });
        app.add_observer(assert_scenario_loaded_payload);
        app.add_systems(OnEnter(GameStates::Playing), assert_scenario_loaded_fired);
    }
}

/// Backs [`ScenarioLoadedAssertPlugin`]: the id the smoke run expects to load and
/// whether [`ScenarioLoaded`] has fired for it yet.
#[derive(Resource)]
struct ScenarioLoadAssertion {
    expected_id: ScenarioId,
    fired: bool,
}

/// Assert the [`ScenarioLoaded`] payload is non-trivial, right where the data is
/// known good. A panic here fails the smoke run, so a regression that loads the
/// wrong scenario or spawns nothing is caught instead of passing on
/// `autopilot: cycle complete` alone.
fn assert_scenario_loaded_payload(
    loaded: On<ScenarioLoaded>,
    mut assertion: ResMut<ScenarioLoadAssertion>,
) {
    info!(
        "smoke: ScenarioLoaded id={:?} handlers={} objects={}",
        loaded.scenario_id, loaded.handler_count, loaded.object_count
    );

    assert_eq!(
        loaded.scenario_id, assertion.expected_id,
        "smoke: ScenarioLoaded reported scenario id {:?}, expected {:?}",
        loaded.scenario_id, assertion.expected_id
    );
    assert!(
        loaded.handler_count > 0,
        "smoke: ScenarioLoaded for {:?} reported zero event handlers -- scenario init registered no handlers",
        loaded.scenario_id
    );
    assert!(
        loaded.object_count > 0,
        "smoke: ScenarioLoaded for {:?} reported zero objects -- scenario init spawned nothing",
        loaded.scenario_id
    );

    assertion.fired = true;
}

/// By the time gameplay starts, the scenario must have loaded. If [`ScenarioLoaded`]
/// never fired, the payload assertion never ran, so guard the silent-empty case
/// here: reaching `Playing` with no load is itself a failure.
fn assert_scenario_loaded_fired(assertion: Res<ScenarioLoadAssertion>) {
    assert!(
        assertion.fired,
        "smoke: reached Playing but ScenarioLoaded for {:?} never fired -- scenario init silently failed",
        assertion.expected_id
    );
}
