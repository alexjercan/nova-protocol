//! screenshot_hero_ship: the drydock's hero racer, close.
//!
//! Loads "Drydock drift" (`shared/drydock.rs`) and poses one three-quarter
//! beauty pass on the hero hull, near enough that its sections read.
//!
//! Ships one manifest image: `wiki-sections`.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - walk the framing, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture the shot (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_hero_ship --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_hero_ship --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/drydock.rs"]
mod drydock;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_hero_ship")]
#[command(version = "1.0.0")]
#[command(about = "The drydock's hero racer, close enough for its sections to read. Autopilot-only: a posed set behind a scripted camera", long_about = None)]
struct Cli;

/// Seconds a step may sit before it is called a stall. Sized with headroom for
/// a slow software-rendered CI GPU (llvmpipe). An expiry is an error exit
/// naming the step, so a run that never loads the scene fails loudly instead of
/// producing an unframed shot.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 30.0;

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
        // Clean frames at a known 16:9: force the window size, drop the dev
        // overlays and the HUD chrome (this set carries no player HUD, so the
        // fps/version bar is just clutter).
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        // The scene is posed, so it must not drift under the framing - but only
        // on a capture run, so a plain `cargo run` keeps its physics and the
        // yard really does drift.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                // The scene is live once the loader has spawned its camera;
                // posing before that poses nothing.
                .step("wait for the drydock scene")
                .enter(GameStates::Loading)
                .until(and(
                    state_is(GameStates::Playing),
                    scenario_camera_present(),
                ))
                .deadline(STEP_DEADLINE_SECS)
                .add()
                // The hero, close: a three-quarter beauty pass where the hull's
                // sections read.
                .step("frame wiki-sections.png")
                .on_enter(|world: &mut World| {
                    pose_camera(world, Vec3::new(7.0, 2.5, 9.0), Vec3::new(0.0, 0.2, 0.0));
                })
                .until(frames(SETTLE_FRAMES))
                .add()
                // The shot step holds until the PNG is on disk, so nothing can
                // move the camera out from under a pending write.
                .step("shoot wiki-sections.png")
                .on_enter(|world: &mut World| shoot(world, "wiki-sections.png"))
                .until(shot_written("wiki-sections.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add(),
        );
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(drydock::drydock_drift(&game_assets, &ships)));
}
