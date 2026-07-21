//! `nova_debug` is the debug-only tooling plugin, compiled only under the
//! `debug` feature so it costs nothing in a shipped build. `DebugPlugin` adds
//! the world inspector and dev overlays (gravity, section wireframes); the
//! `harness` module provides the headless-run presets the examples and the
//! `nova_probe` run-harness drive - `nova_autopilot` (scripted play) and
//! `nova_screenshot` (settled-frame capture). Import the presets from the
//! prelude; the raw plugin types stay reachable under `nova_debug::harness::`.

#![warn(missing_docs)]

use bevy::prelude::*;
use bevy_common_systems::{debug::harness::AUTOPILOT_ENV, prelude::*};
use nova_gameplay::GameStates;

pub mod gravity;
pub mod harness;
pub mod screenshot;
pub mod sections;

/// Glob-import surface: `use nova_debug::prelude::*` brings the harness presets
/// ([`nova_autopilot`](harness::nova_autopilot),
/// [`nova_screenshot`](harness::nova_screenshot), the reel plugin) and
/// [`DebugPlugin`] into scope; the raw plugin types stay under
/// `nova_debug::harness::` to avoid clashing with Bevy's own `ScreenshotPlugin`.
pub mod prelude {
    // Only the presets are the intended entry point. The raw `AutopilotPlugin` /
    // `ScreenshotPlugin` types stay reachable via `nova_debug::harness::` for the
    // rare bespoke-timeline case, so glob-importing this prelude does not clash
    // with Bevy's own `bevy::render::view::screenshot::ScreenshotPlugin`.
    pub use super::{
        debugdump,
        harness::{
            assert_scenario_loaded, capture_window, hide_dev_overlays, nova_autopilot,
            nova_screenshot, reel_pose_camera, ReelBeat, ReelCamera, ScreenshotReelPlugin,
        },
        screenshot::{ScreenshotHotkeyPlugin, SCREENSHOT_KEYCODE},
        DebugPlugin,
    };
}

/// The keycode to toggle debug mode.
pub const DEBUG_TOGGLE_KEYCODE: KeyCode = KeyCode::F11;

/// Resource with debug toggle state.
#[derive(Resource, Default, Clone, Debug, Deref, DerefMut, PartialEq, Eq, Hash)]
pub struct DebugEnabled(pub bool);

/// [`SystemSet`] gating the debug overlays; [`DebugPlugin`] configures it in
/// `Update` and `PostUpdate` to run only while [`DebugEnabled`] is `true`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DebugSystems;

/// A plugin that adds various debugging tools.
///
/// Adds the world inspector, wireframe/section/gravity overlays and the
/// screenshot hotkey as sub-plugins, inserts [`DebugEnabled`], and runs
/// `toggle_debug_mode` in `Update`; the overlay sub-plugins run under the
/// [`DebugSystems`] set gated on [`DebugEnabled`].
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugEnabled(true));

        app.add_plugins(InspectorDebugPlugin);
        app.add_plugins(WireframeDebugPlugin);
        app.add_plugins(sections::SectionsDebugPlugin);
        app.add_plugins(gravity::GravityDebugPlugin);
        app.add_plugins(screenshot::ScreenshotHotkeyPlugin);

        app.add_systems(Update, toggle_debug_mode);

        // Under the headless autopilot (`nova_debug::harness`), confirm the
        // asset loader actually reached gameplay before the clean exit, so a run
        // that silently stalls in `Loading` fails the smoke test instead of
        // passing on `autopilot: cycle complete, no panic` alone.
        if std::env::var(AUTOPILOT_ENV).is_ok() {
            app.add_systems(OnEnter(GameStates::Playing), || {
                info!("nova harness: reached Playing")
            });
        }

        app.configure_sets(
            Update,
            DebugSystems.run_if(resource_equals(DebugEnabled(true))),
        );
        app.configure_sets(
            PostUpdate,
            DebugSystems.run_if(resource_equals(DebugEnabled(true))),
        );
    }
}

fn toggle_debug_mode(mut debug: ResMut<DebugEnabled>, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(DEBUG_TOGGLE_KEYCODE) {
        **debug = !**debug;
    }
}

/// Print the `Update` schedule's system graph (via `bevy_mod_debugdump`) for
/// inspecting system ordering; a dev-only diagnostic, not wired into a schedule.
pub fn debugdump(app: &mut App) {
    bevy_mod_debugdump::print_schedule_graph(app, Update);
    // bevy_mod_debugdump::print_schedule_graph(app, PostUpdate);
    // bevy_mod_debugdump::print_schedule_graph(app, FixedUpdate);
}
