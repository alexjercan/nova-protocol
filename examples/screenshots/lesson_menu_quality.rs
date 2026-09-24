//! lesson_menu_quality: the three demonstrations that are pictures of a
//! SURFACE the main menu opens over itself - `novaos_shell` (the `:` command
//! shell), `advanced_audio` (the four-slider mix) and `advanced_graphics` (the
//! one preset that trades richness for framerate).
//!
//! Its own producer rather than three more frames on `lesson_menu_advanced`,
//! because that one is already at the three-frame cap the example catalog sets
//! (`Cargo.toml`). The split falls on a seam rather than in the middle of one:
//! that walk photographs three DIFFERENT SCREENS a menu BUTTON opens, and this
//! one photographs what opens over the menu without one.
//!
//! The two Settings frames are a POINTER walk over the real menu
//! (`shared/ui_walk.rs`): a tab that would not open for a player does not open
//! here either, and each shot is asserted against a row the tab owns before it
//! is taken. The shell frame is a KEYBOARD walk over the real keyboard path
//! (`shared/computer.rs`), for the same reason - a `:` that stopped opening the
//! shell fails the run rather than reprinting the last shot.
//!
//! ## The order is what each surface costs to reach
//!
//! The shell FIRST, because its lesson says it opens "over the menu" and the
//! menu is what the walk is already standing on - a shell shot taken after
//! Settings would be a shell over a modal. It is then closed and PROVEN closed
//! (`the_shell_is_closed`) before a pointer gesture is attempted: the CRT
//! blocks the buttons underneath it, so a click sent while the raster is still
//! sliding off would land on nothing.
//!
//! Audio next, and not for the handbook's order. Audio is the tab Settings
//! OPENS on (`SettingsTabKind::default`), so shooting it first costs no
//! gesture, and the Graphics shot then proves the tab bar actually moved.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - walk all three surfaces, exit
//!   clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNGs (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_menu_quality --features debug
//! ```

// The real keyboard path into the terminal, shared with the NOVA OS walks.
#[path = "shared/computer.rs"]
mod computer;

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use computer::{press_escape, run_command, the_shell_answered, the_shell_is_closed, type_char};
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
#[command(about = "Record the handbook's shell, audio and graphics screen demonstrations", long_about = None)]
struct Cli;

/// `novaos_shell`: the `:` command shell open over the menu, with a run in it.
#[cfg(feature = "debug")]
const SHELL_SHOT: &str = "novaos_shell.png";
/// What the shell is asked, because it is the one command that answers with
/// the RUN rather than with a list: what is loaded and what is enabled.
#[cfg(feature = "debug")]
const SHELL_COMMAND: &str = "status";
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

/// Menu -> the command shell -> Settings -> the audio tab, then graphics.
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
        // NOVA OS: "The command shell". `:` is read as a TYPED CHARACTER
        // rather than as a key code (`nova_menu::pause::open_command_shell`),
        // so a layout that prints `:` somewhere else still opens it - and the
        // walk has to send it the same way.
        .step("open the command shell over the menu")
        .on_enter(|world: &mut World| type_char(world, ":"))
        .until(nova_os_raster_open())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("run the status command")
        .on_enter(|world: &mut World| run_command(world, SHELL_COMMAND))
        .until(the_shell_answered())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("settle the scrollback")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the command shell")
        .on_enter(shot(SHELL_SHOT))
        .until(shot_written(SHELL_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // Closed and PROVEN closed before the first pointer gesture: the CRT
        // blocks what is under it, so a click sent while the raster is still
        // on screen would land on the monitor rather than on Settings.
        .step("close the shell and let the raster slide off")
        .on_enter(press_escape)
        .until(the_shell_is_closed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("settle the menu again")
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
        // Out through the menu's own front door. A harnessed example owes the
        // smoke contract either way: a run that ends in a settings modal cannot
        // be told from an app that died while still loading, which is what
        // `reached_playing` is there to catch.
        .click("close Settings", "Settings Back Button")
        .click("start a new game", "New Game Button")
        .click("create the world", "Create World Button")
        .step("reach the first flight")
        .until(state_is(GameStates::Playing))
        .deadline(STEP_DEADLINE_SECS)
        .add()
}
