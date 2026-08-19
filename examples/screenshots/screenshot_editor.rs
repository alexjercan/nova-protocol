//! screenshot_editor: the sandbox editor with a ship BUILT in it
//! (`feature-editor.png`), driven through the shipped app (`editor_app`).
//!
//! The ship is built by real pointer gestures over the real picking pipeline, so
//! a build that stopped placing sections fails the run instead of shooting an
//! empty editor.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - build the ship, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNG (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_editor --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_editor --features debug
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
use ui_walk::{count_sections, pose_editor_camera, Gestures, STEP_DEADLINE_SECS};

#[derive(Parser)]
#[command(name = "screenshot_editor")]
#[command(version = "1.0.0")]
#[command(about = "Capture the sandbox editor with a ship built in it. Autopilot-only: a scripted pointer build over the real editor", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same app the game/binary runs (main menu over the ambience backdrop).
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PERF_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            // Turn command errors (despawned-entity targets on the menu/editor
            // teardown) into panics so the run fails loudly on them.
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // Clean frames at a known 16:9 size: force the window resolution and drop
        // the dev overlays. The HUD chrome is re-hidden right before the capture
        // (entering the editor re-raises it).
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(editor_script());
    }

    app.run()
}

/// The driven walk: menu -> editor -> build a ship -> shoot it.
#[cfg(feature = "debug")]
fn editor_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    // The HUD is re-raised by entering the editor, so it is dropped again right
    // before the shot rather than once at `Startup`. `shoot` itself is the
    // capture gate: unarmed, this whole walk runs and writes nothing.
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
        // The editor: reached the way a player reaches it.
        .click("leave for the editor", "Sandbox Button")
        .step("reach the editor")
        .until(and(state_is(GameStates::Playing), frames(SETTLE_FRAMES)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("pose the editor camera off the axis")
        .on_enter(pose_editor_camera)
        .until(frames(SETTLE_FRAMES))
        .add()
        .click("create the ship", "Create New Spaceship Button V2")
        .step("settle the new ship and its colliders")
        .until(frames(SETTLE_FRAMES))
        .add()
        // Build it: arm a part through the gallery, then click the ship itself
        // through the real picking pipeline. A placed section mates the socket
        // nearest the pointer, so each beat below names the section it grows
        // from and the face it grows out of.
        .arm("arm the hull", "reinforced_hull_section")
        .place("hull ahead of the controller", Vec3::ZERO, Vec3::X)
        .place("hull ahead of that", Vec3::new(1.0, 0.0, 0.0), Vec3::X)
        .arm("arm the thruster", "basic_thruster_section")
        .place("thruster on the tail", Vec3::new(2.0, 0.0, 0.0), Vec3::X)
        .arm("arm the turret", "pdc_kinetic_turret_section")
        .place("turret on the spine", Vec3::new(1.0, 0.0, 0.0), Vec3::Y)
        .step("the ship was built")
        .on_enter(|world: &mut World| {
            // The controller the New Ship button spawns, plus the four sections
            // the gestures placed. A short count means a click missed its face
            // and the shot shows a thinner ship than the figure claims.
            let sections = count_sections(world);
            assert_eq!(
                sections, 5,
                "the build gestures must leave a controller, two hulls, a \
                 thruster and a turret on the preview ship"
            );
            info!("editor build: {sections} sections on the preview ship");
        })
        .until(frames(1))
        .add()
        // Put the part down before the shot: a builder holding one sees every
        // free socket drawn on the ship, which is the right answer while
        // building and pure clutter in a figure of the finished thing.
        .click("put the part down", "Select Section Button")
        // Park the pointer clear of the ship: hovering a section raises the
        // placement GHOST, and a translucent extra section is exactly the thing
        // a reader would mistake for part of the built ship.
        .step("park the pointer clear of the ship")
        .on_enter(|world: &mut World| move_cursor(Vec2::new(1720.0, 960.0))(world))
        .until(frames(SETTLE_FRAMES))
        .add()
        // The last step holds until the PNG is on disk, so the driver cannot
        // report done out from under a pending write.
        .step("capture the editor with the built ship")
        .on_enter(shot("feature-editor.png"))
        .until(shot_written("feature-editor.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
