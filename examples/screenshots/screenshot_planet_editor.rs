//! screenshot_planet_editor: a PLANET placed and re-typed in the real editor.
//!
//! One subject: that a creator can reach a planet from the object palette and
//! see the type and seed they picked. The editor stage draws the real planet
//! surface rather than a swatch, so a type change has to change the body on
//! screen - and a walk that only proved a node existed would miss exactly the
//! bug this exists to catch.
//!
//! The gestures are the shipped ones: the Add menu, the palette row, and the
//! inspector's own choice chip. Nothing here reaches into the document.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - place the planet, re-type it,
//!   exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNGs (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_planet_editor --features debug
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

// The pointer gestures, shared with the other editor walks.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use ui_walk::{pose_editor_camera, the_build_camera_is_posed, Gestures};

#[derive(Parser)]
#[command(name = "screenshot_planet_editor")]
#[command(version = "1.0.0")]
#[command(about = "Place a planet in the editor and change its type. Autopilot-only: a scripted pointer walk over the real editor", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(planet_editor_script());
    }

    app.run()
}

/// The driven walk: menu -> editor -> place a planet -> re-type it.
#[cfg(feature = "debug")]
fn planet_editor_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
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
        .click("leave for the editor", "Sandbox Button")
        .step("reach the editor")
        .until(state_is(GameStates::Playing))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("pose the editor camera off the axis")
        .on_enter(pose_editor_camera)
        .until(the_build_camera_is_posed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // The palette row is named for the choice's own label, so this is the
        // same click a creator makes.
        .click("open the Add menu", "Add Menu Button")
        .click("place a planet", "Add Planet")
        .step("let the placed world mesh and settle")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the placed planet")
        .on_enter(shot("feature-editor-planet-dust.png"))
        .until(shot_written("feature-editor-planet-dust.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The type, through the inspector's own chip. If the body on screen
        // does not change with it, the preview is a swatch and this walk is
        // the thing that says so.
        .click("open the type chip", "Inspector Choice Planet Type")
        .click(
            "make it a temperate world",
            "Inspector Choice Planet Type Temperate",
        )
        .step("let the re-typed world mesh and settle")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the re-typed planet")
        .on_enter(shot("feature-editor-planet-temperate.png"))
        .until(shot_written("feature-editor-planet-temperate.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
