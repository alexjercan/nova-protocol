//! screenshot_nova_os_apps: the NOVA OS ship app filling the tube
//! (`news-090-nova-os-apps.png`).
//!
//! It boots the one-ship range from `shared/computer.rs`, opens the computer with
//! Tab and launches the map app and then the ship app through the real keyboard
//! path.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - launch both apps, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNG (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_nova_os_apps --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_nova_os_apps --features debug
//! # look for: `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

// The range and the keyboard path, shared with the other ship-computer walk.
#[path = "shared/computer.rs"]
mod computer;
use computer::nova_os_range;
#[cfg(feature = "debug")]
use computer::{press_enter, press_escape, press_tab, type_word};

#[derive(Parser)]
#[command(name = "screenshot_nova_os_apps")]
#[command(version = "1.0.0")]
#[command(about = "Capture the NOVA OS ship app filling the tube. Autopilot-only: a scripted command walk", long_about = None)]
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
                // Wait for the ship to EXIST: the computer keys off the player
                // ship root, so a beat fired before it spawned would open
                // nothing.
                .step("load the nova_os range")
                .enter(GameStates::Loading)
                .until(player_ship_present())
                .deadline(30.0)
                .add()
                .step("open the computer")
                .on_enter(press_tab)
                .until(frames(SETTLE_FRAMES))
                .add()
                .step("type the map command")
                .on_enter(|world| type_word(world, "map"))
                .until(frames(6))
                .add()
                // The map app writes no file of its own, and the beat stays
                // anyway: it exercises the map app and the RTT/wgsl schematic
                // path, so a render panic in either fails this run.
                .step("launch the map app")
                .on_enter(press_enter)
                .until(frames(SETTLE_FRAMES))
                .add()
                // Leave the map app back to the prompt, then type `ship`.
                .step("type the ship command")
                .on_enter(|world| {
                    press_escape(world);
                    type_word(world, "ship");
                })
                .until(frames(6))
                .add()
                // Launch the ship schematic app and let its RTT scene
                // build/settle. This exercises the real render path (a
                // wgsl/render panic would fail the run).
                .step("launch the ship app")
                .on_enter(press_enter)
                .until(frames(SETTLE_FRAMES))
                .add()
                // The last step holds until the PNG is on disk, so the driver
                // cannot report done out from under a pending write.
                .step("capture the ship app")
                .on_enter(move |world| shoot(world, "news-090-nova-os-apps.png"))
                .until(shot_written("news-090-nova-os-apps.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add(),
        );
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(nova_os_range(&game_assets, &sections)));
}
