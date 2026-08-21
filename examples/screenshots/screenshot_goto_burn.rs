//! screenshot_goto_burn: the GOTO verb's departure burn, chased out of the well.
//!
//! Loads "The ring" (`shared/ring.rs`) and engages the travel computer on the
//! survey beacon over the pole FROM REST, rather than out of a held orbit: the
//! shot is a chase of a ship under burn climbing out of a real well, and the
//! insertion the orbit set flies adds nothing to it. Skipping it is what keeps
//! this run short and repeatable.
//!
//! Ships one manifest image: `feature-autopilot` - `AP GOTO - BURN`, the
//! trajectory ribbon and the destination readout, with the drive lit at the lens.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the script, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture the shot (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_goto_burn --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_goto_burn --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/ring.rs"]
mod ring;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_goto_burn")]
#[command(version = "1.0.0")]
#[command(about = "The GOTO verb's departure burn out of a real well. Autopilot-only: the camera flies the leg with the ship", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PERF_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                // Wait for the ship to EXIST rather than for a guessed load
                // duration: a slow load delays the beats instead of eating them.
                .step("load the ring")
                .enter(GameStates::Loading)
                .until(player_ship_present())
                .deadline(30.0)
                .add()
                .step("settle at the start")
                .on_enter(ring::hud_instrument)
                .until(elapsed(1.0))
                .add()
                // The same [`Autopilot`] component the G keybind inserts, so the
                // leg is the flight computer's and not a scripted animation.
                // Waiting on the telemetry closing, not on a stopwatch.
                .step("engage the travel computer")
                .on_enter(ring::engage_goto)
                .until(ring::player_burning())
                .deadline(30.0)
                .add()
                // The burn, close and from behind: the drive is lit and the plume
                // is pointed at the lens, with the ribbon running up out of frame
                // to the beacon. HUD ON - this one is FOR the chrome (`AP GOTO -
                // BURN`, the destination readout, the ribbon).
                .step("frame the departure burn")
                .on_enter(|world| {
                    ring::hud_instrument(world);
                    ring::chase(world, 26.0, 20.0, 4.0, 10.0);
                })
                .until(elapsed(0.3))
                .add()
                .step("capture the departure burn")
                .on_enter(move |world| shoot(world, "feature-autopilot.png"))
                .until(shot_written("feature-autopilot.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add(),
        );
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        // The leg camera re-solves against the ship every frame; it is inert
        // until a beat installs a [`ring::LegCamera`].
        app.add_systems(Update, ring::drive_leg_camera);
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(ring::the_ring(&game_assets, &ships)));
}
