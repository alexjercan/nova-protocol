//! screenshot_interface: the TAB interface, Map, Ship, then Inventory panes
//! (`wiki-interface-map.png`, `wiki-interface-ship.png`, and
//! `wiki-interface-inventory.png`), and the 0.15.0 news loop of the same walk
//! (`news-0150-tab-interface.webm`).
//!
//! It boots the range with traffic from `shared/computer.rs`. The loop opens
//! over live flight, Tab opens the interface on the Map pane, and M steps to
//! the Ship pane and to Inventory, all through the real keyboard path. Once
//! the loop is written, M wraps back to Map and the walk steps through the
//! three panes again for the stills: no still is shot while the loop records.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - open all three panes twice,
//!   exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the loop and the PNGs
//!   (staged under `NOVA_CAPTURE_DIR`).
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

/// The news loop: Tab over live flight, then Map, Ship and Inventory.
#[cfg(feature = "debug")]
const TAB_LOOP: &str = "news-0150-tab-interface";

/// Frames the loop holds live flight before Tab, and each pane after it shows.
/// Two seconds at the default 30 fps loop clock.
#[cfg(feature = "debug")]
const LOOP_HOLD_FRAMES: u32 = 60;

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
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(interface_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

/// Press M, wait for `pane`, and let M up, so the next press is a fresh edge.
#[cfg(feature = "debug")]
fn next_pane(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    pane: InterfacePaneType,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    script
        .step(format!("switch to the {pane:?} pane"))
        .on_enter(press_key(KeyCode::KeyM))
        .until(the_pane_shows(pane))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step(format!("let M up on the {pane:?} pane"))
        .on_enter(release_key(KeyCode::KeyM))
        .until(frames(1))
        .add()
}

/// Settle the shown pane and write its still. A shown pane is not a STILL
/// one: its offscreen scene builds and settles over frames, which is render
/// work and stays a frame count.
#[cfg(feature = "debug")]
fn shoot_pane(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    path: &'static str,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    script
        .step(format!("settle the pane for {path}"))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step(format!("capture {path}"))
        .on_enter(move |world: &mut World| shoot(world, path))
        .until(shot_written(path))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

/// The walk: record the loop through the three panes, then step through them
/// again for the stills.
#[cfg(feature = "debug")]
fn interface_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        // Wait for the ship to EXIST: the interface needs a player ship, so a
        // beat fired before it spawned would open nothing.
        .step("load the interface plot range")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("open the loop over live flight")
        .on_enter(|world: &mut World| {
            hide_status_bar(world);
            loop_start(world, TAB_LOOP);
        })
        .until(frames(LOOP_HOLD_FRAMES))
        .add()
        .step("open the interface")
        .on_enter(press_tab)
        .until(the_pane_shows(InterfacePaneType::Map))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("hold the Map pane in the loop")
        .until(frames(LOOP_HOLD_FRAMES))
        .add();
    let script = next_pane(script, InterfacePaneType::Ship)
        .step("hold the Ship pane in the loop")
        .until(frames(LOOP_HOLD_FRAMES))
        .add();
    let script = next_pane(script, InterfacePaneType::Inventory)
        .step("hold the Inventory pane in the loop")
        .until(frames(LOOP_HOLD_FRAMES))
        .add()
        .step("close the loop")
        .on_enter(|world: &mut World| loop_end(world, TAB_LOOP))
        .until(loop_written(TAB_LOOP))
        .deadline(60.0)
        .add();

    // M wraps Inventory -> Map, so the stills walk the same three panes.
    let script = shoot_pane(
        next_pane(script, InterfacePaneType::Map),
        "wiki-interface-map.png",
    );
    // Exercises the real render path for the Ship pane's schematic scene (a
    // wgsl/render panic would fail the run).
    let script = shoot_pane(
        next_pane(script, InterfacePaneType::Ship),
        "wiki-interface-ship.png",
    );
    shoot_pane(
        next_pane(script, InterfacePaneType::Inventory),
        "wiki-interface-inventory.png",
    )
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
