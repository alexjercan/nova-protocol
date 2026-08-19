//! screenshot_orbit: the ORBIT verb holding a ring around a real well.
//!
//! Loads "The ring" (`shared/ring.rs`), engages the flight computer's ORBIT
//! verb with an explicit plan, waits for the insertion burn to circularize, and
//! shoots the tutorial figure once the hull is steady on the ring.
//!
//! Ships one manifest image: `tutorial-orbit` - the planetoid, the holo ring
//! across it, the radius spoke, and the ship on the far end of that spoke.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the script, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture the shot (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_orbit --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_orbit --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/ring.rs"]
mod ring;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_orbit")]
#[command(version = "1.0.0")]
#[command(about = "The ORBIT verb holding a ring around a real well. Autopilot-only: the flight computer flies it and the camera is posed", long_about = None)]
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
                // The insertion burn: the ship is at rest on the ring radius, so
                // the verb has a real circularization to fly - full drive, the
                // hull swinging onto the plane, the holo ring and the radius
                // spoke already up. Waiting on the PHASE, not on a stopwatch.
                .step("engage the orbit computer")
                .on_enter(ring::engage_orbit)
                .until(ring::orbit_burning())
                .deadline(20.0)
                .add()
                // Circularized: the verb reports Hold once the velocity error is
                // inside the hold tolerance. A long deadline, because the
                // insertion is a real burn against a real well and its duration
                // is the ship's thrust-to-mass, not a number this file picks.
                .step("settle onto the ring")
                .until(and(
                    ring::orbit_holding(),
                    scenario_variable_is("orbit_stable", 1.0),
                ))
                .deadline(120.0)
                .add()
                .step("let the ring steady")
                .until(elapsed(ring::STEADY_SECS))
                .add()
                // The shipped image. The follow camera looks down the TRACK, and
                // on a ring the body is 90 degrees off it - the game's own view
                // of an orbit has the thing being orbited out of frame, which is
                // a fine thing to fly and a useless thing to teach from. So the
                // tutorial figure is posed: outboard of the ship and slightly
                // above it, aimed most of the way at the body, so the whole
                // planetoid, the holo ring across it, the radius spoke and the
                // ship on the far end of that spoke are one picture. HUD ON - the
                // ring, the spoke and the AP chip are the subject.
                //
                // The offsets are small next to the aim distance ON PURPOSE. Ship
                // and body are ~250 units apart and the camera sits ~50 from the
                // ship: every unit of lateral offset swings the two apart in
                // frame, and at `Y * 30, -track * 20` the pair spanned more than
                // the lens had and the body was cropped off the top edge.
                .step("frame the orbit shot")
                .on_enter(|world| {
                    ring::hud_instrument(world);
                    let ship = ring::ship_position(world);
                    let out = ship.normalize_or_zero();
                    let track = ring::ship_heading(world);
                    ring::pin(
                        world,
                        ship + out * 45.0 + Vec3::Y * 15.0 - track * 10.0,
                        // Well short of the ship, most of the way to the body:
                        // the planetoid takes the middle of the frame and the
                        // ship falls out to the low corner on the end of its
                        // spoke, which is the relationship the figure teaches.
                        ship - out * 120.0,
                    );
                })
                .until(elapsed(0.4))
                .add()
                .step("capture the orbit shot")
                .on_enter(move |world| shoot(world, "tutorial-orbit.png"))
                .until(shot_written("tutorial-orbit.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add(),
        );
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(ring::the_ring(&game_assets, &ships)));
}
