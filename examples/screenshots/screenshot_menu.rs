//! screenshot_menu: the main menu over the ambience backdrop
//! (`tutorial-menu.png`) and the Settings panel open over it
//! (`wiki-settings.png`), driven through the shipped app (`editor_app`).
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - walk the menu, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write each PNG (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_menu --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_menu --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

// The pointer gestures, shared with the other menu walks. Script-only, so the
// whole module sits behind one gate here.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use ui_walk::{Gestures, STEP_DEADLINE_SECS};

#[derive(Parser)]
#[command(name = "screenshot_menu")]
#[command(version = "1.0.0")]
#[command(about = "Capture the main menu and its Settings panel. Autopilot-only: a scripted pointer walk over the real menu", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same app the game/binary runs (main menu over the ambience backdrop).
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            // Turn command errors (despawned-entity targets on the menu
            // teardown) into panics so the run fails loudly on them.
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // Clean frames at a known 16:9 size: force the window resolution and drop
        // the dev overlays.
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(menu_script());
    }

    app.run()
}

/// The driven walk: menu -> Settings, one shot per state.
#[cfg(feature = "debug")]
fn menu_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    // The HUD chrome is dropped right before every shot rather than once at
    // `Startup`, because other states re-raise it. `shoot` itself is the capture
    // gate: unarmed, this whole walk runs and writes nothing.
    let shot = |path: &'static str| {
        move |world: &mut World| {
            hide_hud(world);
            shoot(world, path);
        }
    };

    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .enter(GameStates::Loading)
        .until(state_is(GameStates::MainMenu))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("settle the menu and its ambience backdrop")
        .until(frames(SETTLE_FRAMES))
        .add()
        // Hide the HUD first, and let the PNG land BEFORE navigating away:
        // clicking on in the same frame captured a black mid-teardown frame.
        .step("capture the main menu")
        .on_enter(shot("tutorial-menu.png"))
        .until(shot_written("tutorial-menu.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The Settings panel: an overlay over the menu, not its own state.
        .click("open Settings", "Settings Button")
        .step("settle the settings panel")
        .until(frames(SETTLE_FRAMES))
        .add()
        // The panel is toggled by Visibility alone, so this has to assert
        // VISIBLE, not merely laid out: the shot is otherwise the main menu
        // again under a different name.
        .step("the settings panel is up")
        .on_enter(assert_named_visible("Settings Panel"))
        .until(frames(1))
        .add()
        .step("capture the settings panel")
        .on_enter(shot("wiki-settings.png"))
        .until(shot_written("wiki-settings.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
