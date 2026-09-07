//! screenshot_gravity: the drydock hero with the yard's planetoid behind it.
//!
//! Loads "Drydock drift" (`shared/drydock.rs`) and poses one framing: the hero
//! gunship in the near field with the well's planetoid behind it.
//!
//! Ships one manifest image: `feature-gravity`.
//!
//! The wiki's gravity figure is NOT shot here. A body on black teaches nothing
//! about a well; `screenshot_orbit` flies a real ring around a real one, with
//! the holo ring, the radius spoke and the ship on the end of it in one frame,
//! and the wiki page takes that.
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
                // The gravity feature: hero left of centre, the planetoid
                // right of it and 5.9 km back, belt rocks between the two for
                // depth. Both subjects sit on the horizontal centreline and
                // 29 degrees apart, which is what decides where the lens
                // stands: the hero is 195 m off, so the ONLY freedom left is
                // which way the camera faces out of it, and a pose that put
                // the hero dead centre put the body behind the hero.
                .step("frame feature-gravity.png")
                .on_enter(|world: &mut World| {
                    pose_camera(
                        world,
                        Meters3::new(41.0, 32.0, 193.0),
                        Meters3::new(43.0, 0.0, -4.0),
                    );
                })
                .until(frames(SETTLE_FRAMES))
                .add()
                .step("shoot feature-gravity.png")
                .on_enter(|world: &mut World| shoot(world, "feature-gravity.png"))
                .until(shot_written("feature-gravity.png"))
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
