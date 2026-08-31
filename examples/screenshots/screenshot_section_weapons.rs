//! screenshot_section_weapons: the wiki closeups of the weapon sections -
//! `wiki-section-turret.png`, `wiki-section-turret-twin.png` and
//! `wiki-section-torpedo-bay.png`.
//!
//! The showcase ship and the turntable are `shared/showcase.rs`: the camera
//! holds one bearing inside the photo rig's good wedge and the SHIP yaws to
//! bring each section round to it, most of a revolution across the three
//! shots.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - turn the ship through both
//!   closeups, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write each PNG (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_section_weapons --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_section_weapons --features debug
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
#[command(name = "screenshot_section_weapons")]
#[command(version = "1.0.0")]
#[command(about = "Capture the wiki closeups of the turret and torpedo-bay sections. Autopilot-only: posed closeups on a scripted turntable", long_about = None)]
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
        app.add_plugins(weapons_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_ship);
}

fn setup_ship(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(section_ship(&game_assets, &sections)));
}

/// The three weapons: the gatling and the bay on the flanks, the twin on the
/// front hull's roof. Mount points match the ship layout in `section_ship`.
#[cfg(feature = "debug")]
fn section_shots() -> [SectionShot; 3] {
    [
        // Turret: the flank it is mounted on, turned enough that the barrel
        // rakes across the frame instead of foreshortening into a dot.
        SectionShot {
            mount: Vec3::new(1.0, 0.0, 0.0),
            faces: Vec3::new(1.0, 0.0, 0.55),
            distance: 3.6,
            path: "wiki-section-turret.png",
        },
        // Twin turret: on the roof, turned so both tubes read side by side
        // instead of eclipsing each other.
        SectionShot {
            mount: Vec3::new(0.0, 0.75, -1.0),
            faces: Vec3::new(0.55, 0.0, -1.0),
            distance: 3.6,
            path: "wiki-section-turret-twin.png",
        },
        // Torpedo bay: the opposite flank, so the ship turns most of a
        // revolution. Angled to keep the open muzzle and a flank of the
        // two-cell tube in one read.
        SectionShot {
            mount: Vec3::new(-1.0, 0.0, 0.5),
            faces: Vec3::new(-1.0, 0.0, -0.5),
            distance: 4.5,
            path: "wiki-section-torpedo-bay.png",
        },
    ]
}

/// The driven walk: wait for the showcase, then present-settle-shoot each
/// weapon in turn.
#[cfg(feature = "debug")]
fn weapons_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
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
