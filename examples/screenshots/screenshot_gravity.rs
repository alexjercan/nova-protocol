//! screenshot_gravity: the drydock's planetoid, from the yard and from close in.
//!
//! Loads "Drydock drift" (`shared/drydock.rs`) and poses two framings of the
//! same body: the hero with the well behind it, then the well as the subject.
//!
//! Ships two manifest images: `feature-gravity` and `wiki-gravity`.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - walk both framings, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture the shots (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_gravity --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_gravity --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/drydock.rs"]
mod drydock;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_gravity")]
#[command(version = "1.0.0")]
#[command(about = "The drydock's planetoid, framed from the yard and from close in. Autopilot-only: a posed set behind a scripted camera", long_about = None)]
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
        // The scene is posed, so it must not drift between framings - but only
        // on a capture run, so a plain `cargo run` keeps its physics and the
        // yard really does drift.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
        // Every capture gets its OWN step, holding until the PNG is on disk:
        // Bevy services one primary-window capture per frame, and a framing
        // that moved the camera out from under a pending write would shoot the
        // wrong picture.
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
                // The gravity feature: hero in the near left, the planetoid
                // behind it down-right, belt rocks between the two for depth.
                .step("frame feature-gravity.png")
                .on_enter(|world: &mut World| {
                    pose_camera(world, Vec3::new(-6.5, 2.6, 9.5), Vec3::new(0.0, 0.0, -2.0));
                })
                .until(frames(SETTLE_FRAMES))
                .add()
                .step("shoot feature-gravity.png")
                .on_enter(|world: &mut World| shoot(world, "feature-gravity.png"))
                .until(shot_written("feature-gravity.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                // The planetoid as the subject: closer in, the body filling the
                // lower half with the yard's rocks passing in front of it.
                .step("frame wiki-gravity.png")
                .on_enter(|world: &mut World| {
                    pose_camera(
                        world,
                        Vec3::new(40.0, -6.0, -110.0),
                        drydock::PLANETOID_POSITION,
                    );
                })
                .until(frames(SETTLE_FRAMES))
                .add()
                .step("shoot wiki-gravity.png")
                .on_enter(|world: &mut World| shoot(world, "wiki-gravity.png"))
                .until(shot_written("wiki-gravity.png"))
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
