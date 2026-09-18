//! lesson_start_welcome: the handbook's demonstration for "How training works"
//! (`assets/base/training/start_welcome.webp`).
//!
//! A picture of the thing the reader is looking at: the Lessons screen, open on
//! a lesson that carries every part of one - a demonstration, the text, a
//! binding chip and a Practice button. The lesson it opens is itself a real
//! recording, so the picture of a lesson shows a lesson, not a placeholder.
//!
//! The walk is a POINTER walk over the real menu (`shared/ui_walk.rs`), and the
//! shot is taken only after the SCREEN's own record says the click landed - a
//! missed row leaves the pane on the previous lesson and the frame would still
//! look plausible.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - open the handbook, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNG (staged under
//!   `NOVA_CAPTURE_DIR`). `scripts/capture-lesson-media.sh` runs this and
//!   encodes the result as the WebP the handbook ships.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_start_welcome --features debug
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

// The pointer gestures, shared with the other menu walks.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use nova_ui::widget::Selected;
#[cfg(feature = "debug")]
use ui_walk::{hide_menu_version, Gestures};

#[derive(Parser)]
#[command(name = "lesson_start_welcome")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's own demonstration", long_about = None)]
struct Cli;

/// The lesson this shoots for, and so the name of the still.
#[cfg(feature = "debug")]
const SHOT: &str = "start_welcome.png";

/// The lesson the handbook is opened ON.
///
/// A LOOP tip that also carries a binding chip and a Practice button, so the
/// one picture of "each lesson is one screen" holds every part of a lesson
/// screen at once.
#[cfg(feature = "debug")]
const WELCOME_ROW: &str = "Lesson Row: flight_aim";

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
        app.add_plugins(welcome_script());
    }

    app.run()
}

/// Menu -> the handbook -> a lesson with a demonstration -> shoot it.
#[cfg(feature = "debug")]
fn welcome_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .enter(GameStates::Loading)
        .until(state_is(GameStates::MainMenu))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("settle the menu and its ambience backdrop")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click("open the handbook", "Lessons Button")
        .step("settle the handbook")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click("select a lesson with a demonstration", WELCOME_ROW)
        .step("settle the lesson's details pane")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("the handbook is open on that lesson")
        .on_enter(|world: &mut World| {
            // The screen's OWN record of which click landed, not a restatement
            // of what the beat intended.
            let landed = world
                .query_filtered::<&Name, With<Selected>>()
                .iter(world)
                .any(|selected| selected.as_str() == WELCOME_ROW);
            assert!(
                landed,
                "the click on `{WELCOME_ROW}` never landed: the list has not \
                 marked that row Selected, so the details pane shows the \
                 PREVIOUS lesson"
            );
            assert_named_visible("Lesson Media Frame")(world);
        })
        .add()
        .step("capture the handbook")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            hide_menu_version(world);
            shoot(world, SHOT);
        })
        .until(shot_written(SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // Out through the handbook's OWN hand-off, since the row this walk
        // selected is the one that offers it. A harnessed example owes the
        // smoke contract either way: a run that ends in the menu cannot be told
        // from an app that died while still loading (`reached_playing`).
        .click("fly the lesson's practice range", "Lesson Practice Button")
        .step("reach the practice range")
        .until(state_is(GameStates::Playing))
        .deadline(STEP_DEADLINE_SECS)
        .add()
}
