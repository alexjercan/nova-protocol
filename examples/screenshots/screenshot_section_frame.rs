//! screenshot_section_frame: the wiki closeups of the flight-frame sections -
//! `wiki-section-controller.png`, `wiki-section-hull.png`,
//! `wiki-section-thruster.png` - and the two hull-variant cards,
//! `wiki-section-hull-cargo.png` and `wiki-section-hull-tank.png`.
//!
//! The showcase ship and the turntable are `shared/showcase.rs`: the camera
//! holds one bearing inside the photo rig's good wedge and the SHIP yaws to
//! bring each section round to it.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - turn the ship through every
//!   closeup, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write each PNG (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_section_frame --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_section_frame --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

// The showcase ship and its turntable, shared with the other section closeups.
#[path = "shared/showcase.rs"]
mod showcase;
use showcase::section_ship;
#[cfg(feature = "debug")]
use showcase::{present_section, SectionShot, STEP_DEADLINE_SECS};

#[derive(Parser)]
#[command(name = "screenshot_section_frame")]
#[command(version = "1.0.0")]
#[command(about = "Capture the wiki closeups of the controller, hull and thruster sections plus the cargo/tank hull cards. Autopilot-only: posed closeups on a scripted turntable", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        // Clean frames at a known 16:9: force the window size, drop the dev
        // overlays and the HUD chrome (the showcase carries no player HUD, so
        // the fps/version bar is just clutter).
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        // The turntable sets the ship's rotation per shot, so nothing may push
        // it back - but only on a capture run, so a plain `cargo run` keeps its
        // physics.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
        app.add_plugins(frame_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_ship);
}

fn setup_ship(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(section_ship(&game_assets, &sections)));
}

/// The flight frame: the parts a ship needs to be a ship. Mount points match the
/// ship layout in `section_ship`.
#[cfg(feature = "debug")]
fn section_shots() -> [SectionShot; 5] {
    [
        // Controller: the bridge, read across the spine from the front quarter.
        SectionShot {
            mount: Vec3::ZERO,
            faces: Vec3::NEG_Z,
            distance: 5.0,
            path: "wiki-section-controller.png",
        },
        // Front hull: plating and frame, taken off the nose quarter. Not the
        // broadside - that puts the turret barrel straight across the subject.
        SectionShot {
            mount: Vec3::new(0.0, 0.0, -1.0),
            faces: Vec3::new(-0.35, 0.0, -1.0),
            distance: 4.0,
            path: "wiki-section-hull.png",
        },
        // Thruster: off the nozzle's axis, not down it. Dead astern points the
        // plume at the lens and the bloom eats the bell that is the subject.
        SectionShot {
            mount: Vec3::new(0.0, 0.0, 2.0),
            faces: Vec3::new(0.55, 0.0, 1.0),
            distance: 4.4,
            path: "wiki-section-thruster.png",
        },
        // Cargo hull: the starboard-aft cell, read off its own flank so the
        // caged freight faces carry the frame. Tighter than the other cards:
        // at 3.6 the tank vessel on the roof cell outdraws the crates.
        SectionShot {
            mount: Vec3::new(1.0, 0.0, 1.0),
            faces: Vec3::new(1.0, 0.0, 0.55),
            distance: 3.0,
            path: "wiki-section-hull-cargo.png",
        },
        // Tank hull: the roof cell, turned so the pressure vessel shows
        // through the frame rails instead of a bare end plate. Tight, so the
        // cargo deck and the gatlings below crop away.
        SectionShot {
            mount: Vec3::new(0.0, 1.0, 1.0),
            faces: Vec3::new(1.0, 0.0, 0.55),
            distance: 3.0,
            path: "wiki-section-hull-tank.png",
        },
    ]
}

/// The driven walk: wait for the showcase, then present-settle-shoot each
/// section in turn.
#[cfg(feature = "debug")]
fn frame_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        // The showcase is live once the loader has spawned its camera; posing
        // before that poses nothing.
        .step("wait for the section showcase")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add();

    for shot in section_shots() {
        let path = shot.path;
        script = script
            .step(format!("present {path}"))
            .on_enter(move |world: &mut World| present_section(world, &shot))
            .until(frames(SETTLE_FRAMES))
            .add()
            // The shot step holds until the PNG is on disk, so the next turn
            // cannot swing the ship out from under a pending write.
            .step(format!("shoot {path}"))
            .on_enter(move |world: &mut World| shoot(world, path))
            .until(shot_written(path))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
    }
    script
}
