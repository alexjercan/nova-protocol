//! screenshot_interface: the TAB interface, Map, Ship, then Inventory panes
//! (`wiki-interface-map.png`, `wiki-interface-ship.png`, and
//! `wiki-interface-inventory.png`).
//!
//! It boots the range with traffic from `shared/computer.rs`, opens the
//! interface with Tab (it opens on the Map pane) and switches to the Ship pane
//! with M, then switches to Inventory with M again, through the real keyboard path.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - open all three panes, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNGs (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_interface --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_interface --features debug
//! # look for: `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_protocol::nova_interface::pane::InterfacePaneType;
use nova_protocol::prelude::*;

// The range and the keyboard path, shared with the command walks.
#[path = "shared/computer.rs"]
mod computer;
use computer::interface_plot_range;
#[cfg(feature = "debug")]
use computer::press_tab;

#[derive(Parser)]
#[command(name = "screenshot_interface")]
#[command(version = "1.0.0")]
#[command(about = "Capture the TAB interface Map, Ship, and Inventory panes. Autopilot-only: a scripted command walk", long_about = None)]
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
                // Wait for the ship to EXIST: the interface needs a player
                // ship, so a beat fired before it spawned would open nothing.
                .step("load the interface plot range")
                .enter(GameStates::Loading)
                .until(player_ship_present())
                .deadline(30.0)
                .add()
                .step("open the interface")
                .on_enter(press_tab)
                .until(the_pane_shows(InterfacePaneType::Map))
                .deadline(STEP_DEADLINE_SECS)
                .add()
                // A shown pane is not a STILL one: its offscreen scene builds
                // and settles over frames, which is render work and stays a
                // frame count.
                .step("settle the map pane for the shot")
                .until(frames(SETTLE_FRAMES))
                .add()
                .step("capture the map pane")
                .on_enter(|world| shoot(world, "wiki-interface-map.png"))
                .until(shot_written("wiki-interface-map.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                // Exercises the real render path for the Ship pane's schematic
                // scene (a wgsl/render panic would fail the run).
                .step("switch to the ship pane")
                .on_enter(press_key(KeyCode::KeyM))
                .until(the_pane_shows(InterfacePaneType::Ship))
                .deadline(STEP_DEADLINE_SECS)
                .add()
                .step("settle the ship pane for the shot")
                .on_enter(release_key(KeyCode::KeyM))
                .until(frames(SETTLE_FRAMES))
                .add()
                .step("capture the ship pane")
                .on_enter(|world| shoot(world, "wiki-interface-ship.png"))
                .until(shot_written("wiki-interface-ship.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                .step("switch to the inventory pane")
                .on_enter(press_key(KeyCode::KeyM))
                .until(the_pane_shows(InterfacePaneType::Inventory))
                .deadline(STEP_DEADLINE_SECS)
                .add()
                .step("settle the inventory pane for the shot")
                .until(frames(SETTLE_FRAMES))
                .add()
                // Wait until the PNG is on disk before the driver exits.
                .step("capture the inventory pane")
                .on_enter(|world| shoot(world, "wiki-interface-inventory.png"))
                .until(shot_written("wiki-interface-inventory.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add(),
        );
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

/// Advance once the interface is open on `pane`.
#[cfg(feature = "debug")]
fn the_pane_shows(
    pane: InterfacePaneType,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            && world
                .get_resource::<InterfacePaneType>()
                .is_some_and(|shown| *shown == pane)
    })
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(interface_plot_range(&game_assets, &sections)));
}
