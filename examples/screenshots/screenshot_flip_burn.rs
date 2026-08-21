//! screenshot_flip_burn: the flip-and-burn at the far end of a GOTO leg.
//!
//! Loads "The ring" (`shared/ring.rs`), engages the travel computer on the
//! survey beacon FROM REST (no orbit insertion first - the leg is what this
//! shot is about), then rides it out: burn, coast, FLIP, retro burn. The camera
//! flies with the ship and the shot is taken at the END of the swing.
//!
//! Ships one manifest image: `wiki-flight`.
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
//!   cargo run --example screenshot_flip_burn --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_flip_burn --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/ring.rs"]
mod ring;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_flip_burn")]
#[command(version = "1.0.0")]
#[command(about = "The flip-and-burn at the far end of a GOTO leg. Autopilot-only: the camera flies the leg with the ship", long_about = None)]
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
                .step("engage the travel computer")
                .on_enter(ring::engage_goto)
                .until(ring::player_burning())
                .deadline(30.0)
                .add()
                // Coast, then the flip: the computer swings the ship end-for-end
                // and lights the drive back down the path. The wait is on the
                // BRAKING transition (the telemetry drops its flip point once the
                // brake is planned), not on a stopwatch. The camera is handed
                // back for the coast - a pinned pose would spend it watching a
                // ship shrink.
                .step("coast to the flip")
                .on_enter(|world| {
                    ring::hud_instrument(world);
                    ring::unpose(world);
                })
                .until(ring::player_braking())
                .deadline(150.0)
                .add()
                // The money frame of a flip-and-burn is the END of the swing: the
                // hull is round, the drive is lit back down the path and the plume
                // points where the ship is going. Waiting for the phase, not for a
                // fraction of a rotation nobody can time.
                .step("flip and burn")
                .until(ring::player_retro_burning())
                .deadline(25.0)
                .add()
                // So the camera is AHEAD of the ship for this one, not behind it:
                // braking, the drive fires down the track, and the plume that was
                // at the lens during the departure is now on the far side of the
                // hull. It also fixes the frame the first cut of this beat got
                // wrong - the swing puts the hull's shadow side to a fixed-
                // direction rig, and `ring::lit_side` is what keeps the lens on
                // the key.
                .step("frame the flip")
                .on_enter(|world| {
                    ring::hud_instrument(world);
                    ring::lead(world, 21.0, 11.0, 6.0);
                })
                .until(elapsed(0.3))
                .add()
                .step("capture the flip")
                .on_enter(move |world| shoot(world, "wiki-flight.png"))
                .until(shot_written("wiki-flight.png"))
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
