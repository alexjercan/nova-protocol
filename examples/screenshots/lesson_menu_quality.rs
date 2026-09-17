//! lesson_menu_quality: the two ADVANCED demonstrations that are pictures of
//! the Settings modal's own tabs - `advanced_audio` (the four-slider mix) and
//! `advanced_graphics` (the one preset that trades richness for framerate).
//!
//! Its own producer rather than two more frames on `lesson_menu_advanced`,
//! because that one is already at the three-frame cap the example catalog sets
//! (`Cargo.toml`). The split falls on a seam rather than in the middle of one:
//! that walk photographs three DIFFERENT SCREENS a menu button opens, and this
//! one photographs two tabs of a single modal, so it opens Settings once and
//! never leaves it.
//!
//! The walk is a POINTER walk over the real menu (`shared/ui_walk.rs`): a tab
//! that would not open for a player does not open here either, and each shot is
//! asserted against a row the tab owns before it is taken.
//!
//! Audio first, and not for the handbook's order. Audio is the tab Settings
//! OPENS on (`SettingsTabKind::default`), so shooting it first costs no
//! gesture, and the Graphics shot then proves the tab bar actually moved.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - walk both tabs, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNGs (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_menu_quality --features debug
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

// The pointer gestures, shared with the other menu walks.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use ui_walk::{hide_menu_version, Gestures};

#[derive(Parser)]
#[command(name = "lesson_menu_quality")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's audio and graphics screen demonstrations", long_about = None)]
struct Cli;

/// `advanced_audio`: Settings on the tab that holds the four mixer sliders.
#[cfg(feature = "debug")]
const AUDIO_SHOT: &str = "advanced_audio.png";
/// `advanced_graphics`: Settings on the tab that holds the quality preset.
#[cfg(feature = "debug")]
const GRAPHICS_SHOT: &str = "advanced_graphics.png";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same app the game binary runs: the main menu over its live backdrop.
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(menu_quality_script());
    }

    app.run()
}

/// Menu -> Settings -> the audio tab, then the graphics tab.
#[cfg(feature = "debug")]
fn menu_quality_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    // The chrome is dropped right before each shot rather than once at Startup,
    // because other states re-raise it. `shoot` is the capture gate: unarmed,
    // this whole walk runs and writes nothing.
    let shot = |path: &'static str| {
        move |world: &mut World| {
            hide_hud(world);
            hide_menu_version(world);
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
        .click("open Settings", "Settings Button")
        .step("settle the settings modal")
        .until(frames(SETTLE_FRAMES))
        .add()
        // ADVANCED: "Audio mix" - the tab Settings opens on, so the whole mix
        // is in shot without touching the bar.
        .step("the four mixer rows are on the tab")
        .on_enter(|world: &mut World| {
            assert_named_visible("Master Volume Row")(world);
            assert_named_visible("Interface Volume Row")(world);
            assert_named_visible("World Volume Row")(world);
            assert_named_visible("Music Volume Row")(world);
        })
        .add()
        .step("capture the audio tab")
        .on_enter(shot(AUDIO_SHOT))
        .until(shot_written(AUDIO_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // ADVANCED: "Graphics presets" - the same modal, one tab over.
        .click("open the Graphics tab", "Settings Tab: Graphics")
        .step("settle the graphics tab")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("the preset row is on the tab")
        .on_enter(|world: &mut World| {
            assert_named_visible("Graphics Row")(world);
            assert_named_visible("Window Mode Row")(world);
        })
        .add()
        .step("capture the graphics tab")
        .on_enter(shot(GRAPHICS_SHOT))
        .until(shot_written(GRAPHICS_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
