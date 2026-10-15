//! `nova_debug` is the debug-only tooling plugin, compiled only under the
//! `debug` feature so it costs nothing in a shipped build. `DebugPlugin` adds
//! the Nova Debug window (F11) and the dev overlays it switches; the
//! `harness` module is the Nova adapter over the `nova_autopilot` driver,
//! providing the headless-run presets the examples and the `nova_probe`
//! run-harness drive - `nova_autopilot` (scripted play) and `nova_screenshot`
//! (the settled-frame capture beat it appends) - plus the one capture idiom the
//! screenshot examples script their shots with (`shoot`). Import the presets
//! from the prelude; the raw driver type stays reachable under
//! `nova_debug::harness::`.

#![warn(missing_docs)]

use bevy::prelude::*;
use nova_autopilot::autopilot::AUTOPILOT_ENV;
use nova_gameplay::GameStates;

use crate::{inspector::InspectorDebugPlugin, wireframe::WireframeDebugPlugin};

pub mod diagnostics;
pub mod gravity;
pub mod harness;
pub mod hud;
pub mod inspector;
// Host-only: the hotkey stamps the file with the wall clock and writes it
// through `std::fs`, and wasm32-unknown-unknown has neither - `SystemTime::now`
// is a panicking stub there, and there is no Downloads directory to write to.
// A browser saves an image from the canvas, which is a different feature.
#[cfg(not(target_arch = "wasm32"))]
pub mod screenshot;
pub mod sections;
#[cfg(not(target_arch = "wasm32"))]
mod sfx_sidecar;
pub mod wireframe;

/// The `bevy_egui` the Nova Debug window draws with, for a debug example that
/// draws its own egui window on the same context.
pub use bevy_inspector_egui::bevy_egui;

/// Glob-import surface: `use nova_debug::prelude::*` brings the harness presets
/// ([`nova_autopilot`](harness::nova_autopilot),
/// [`nova_screenshot`](harness::nova_screenshot)), the capture idiom
/// ([`shoot`](harness::shoot) and its scene dressing) and [`DebugPlugin`] into
/// scope; the raw driver type stays under `nova_debug::harness::`.
pub mod prelude {
    // Only the presets are the intended entry point. The raw `AutopilotPlugin`
    // type stays reachable via `nova_debug::harness::` for the rare
    // bespoke-timeline case.
    // The generic half of the script vocabulary, so an example writes its beats
    // from ONE import instead of reaching into `nova_autopilot` alongside the
    // Nova-typed predicates it is composing them with. `not` is deliberately
    // absent: it collides with `bevy::prelude::not`, which every example globs,
    // so a caller that wants it names `nova_autopilot::predicate::not`.
    // The phase `drive_action` takes. A range that holds an action every frame
    // names it; the `press_action` / `release_action` beats do not.
    pub use nova_autopilot::{
        input::{
            assert_named_visible, click_at, click_named, hover_named, keep_hovering_named,
            move_cursor, press_edit_key, press_key, press_mouse, release_key, release_mouse,
            scroll_lines, scroll_pixels, type_text, ui_node_centre, ui_node_diagnosis,
            ui_node_rect,
        },
        predicate::{
            and, any_entity, elapsed, frames, loop_written, or, pointer_at, pointer_at_node,
            pointer_hover_diagnosis, pointer_over_node, pointer_pressed, pointer_released,
            resource_where, sheet_written, shot_written, state_is, ui_node_present,
            window_scale_factor_is, window_size_is,
        },
        // The logical key `press_edit_key` takes. Bevy's prelude carries
        // `KeyCode` - the PHYSICAL key - but not this one.
        prelude::Key,
    };
    pub use nova_input::prelude::InputPhase;

    #[cfg(not(target_arch = "wasm32"))]
    pub use super::screenshot::{ScreenshotHotkeyPlugin, SCREENSHOT_KEYCODE};
    pub use super::{
        debugdump,
        diagnostics::{
            DiagnosticsGizmos, DiagnosticsHintPlugin, DiagnosticsPanel, DiagnosticsPlugin,
            DiagnosticsSystems,
        },
        harness::{
            assert_scenario_loaded, capture_window, capturing, drive_action, editor_field_focused,
            editor_filter_focused, editor_gallery_closed, editor_gallery_open,
            editor_gallery_selected, editor_gizmo_on, editor_inspector_reads, editor_part_armed,
            editor_placement_clear, editor_placement_refused, editor_placement_solved,
            editor_tool_is, force_capture_resolution, freeze_bodies, hide_dev_overlays, hide_hud,
            hide_status_bar, loop_end, loop_start, nova_autopilot, nova_os_command_line_reads,
            nova_os_raster_open, nova_screenshot, player_ship_present, pose_camera, press_action,
            release_action, scenario_camera_present, scenario_is_built, scenario_variable_is,
            script_reports_done, section_gone, sheet_start, shoot, CaptureLog, LoopCapturePlugin,
            LoopProfile, SheetGrid, BEAT_DEADLINE_SECS, NOVA_AUTOPILOT_STEP, SETTLE_FRAMES,
            SHOT_DEADLINE_SECS, STEP_DEADLINE_SECS,
        },
        hud::{overlays::DebugOverlays, DebugHud, DebugHudPlugin, DebugHudSystems},
        DebugPlugin,
    };
}

/// The keycode that opens and closes the Nova Debug window.
pub const DEBUG_TOGGLE_KEYCODE: KeyCode = KeyCode::F11;

/// A plugin that adds various debugging tools.
///
/// Adds the Nova Debug window, the wireframe/section/gravity overlays and the
/// F3 diagnostics layer ([`DiagnosticsPlugin`](diagnostics::DiagnosticsPlugin))
/// as sub-plugins, plus, on host targets, the screenshot hotkey and the loop
/// capture SFX sidecar. Every overlay boots off and has its own flag in
/// [`DebugOverlays`](hud::overlays::DebugOverlays); F11 opens the Nova Debug
/// window ([`DebugHudPlugin`](hud::DebugHudPlugin)).
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        // Egui first: the debug window draws only when it is there.
        app.add_plugins(InspectorDebugPlugin);
        app.add_plugins(hud::DebugHudPlugin);
        app.add_plugins(WireframeDebugPlugin);
        app.add_plugins(sections::SectionsDebugPlugin);
        app.add_plugins(gravity::GravityDebugPlugin);
        app.add_plugins(diagnostics::DiagnosticsPlugin);
        #[cfg(not(target_arch = "wasm32"))]
        app.add_plugins(screenshot::ScreenshotHotkeyPlugin);
        // Every WebM loop gets its SFX sidecar and Opus track without a second
        // pair of step calls. Host-only: the render runs ffmpeg from a thread.
        #[cfg(not(target_arch = "wasm32"))]
        app.add_plugins(sfx_sidecar::SfxSidecarPlugin);

        hud::overlays::build(app);

        // Under the headless autopilot (the `harness` presets), confirm the
        // asset loader actually reached gameplay before the clean exit, so a run
        // that silently stalls in `Loading` fails probe's `reached_playing`
        // check instead of passing on `cycle complete, no panic` alone.
        if std::env::var(AUTOPILOT_ENV).is_ok() {
            app.add_systems(OnEnter(GameStates::Playing), || {
                info!("{}", crate::harness::REACHED_PLAYING)
            });
        }
    }
}

/// Print the `Update` schedule's system graph (via `bevy_mod_debugdump`) for
/// inspecting system ordering; a dev-only diagnostic, not wired into a schedule.
pub fn debugdump(app: &mut App) {
    bevy_mod_debugdump::print_schedule_graph(app, Update);
    // bevy_mod_debugdump::print_schedule_graph(app, PostUpdate);
    // bevy_mod_debugdump::print_schedule_graph(app, FixedUpdate);
}
