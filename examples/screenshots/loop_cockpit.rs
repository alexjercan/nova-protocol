//! loop_cockpit: a quiet flight view gains live GOTO instruments, then opens
//! NOVA OS and its map on the same ship and target.
//!
//! Every transition uses the production input path. The script only presses
//! the controls, records the result, and exits.

#[path = "shared/computer.rs"]
mod computer;
#[path = "shared/ring.rs"]
mod ring;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use computer::{press_enter, press_tab, type_word};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_cockpit")]
#[command(version = "1.0.0")]
#[command(about = "Capture the contextual cockpit opening NOVA OS on its live GOTO target")]
struct Cli;

#[cfg(feature = "debug")]
const LOOP_NAME: &str = "landing-cockpit";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(cockpit_script());
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

#[cfg(feature = "debug")]
fn cockpit_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the quiet cockpit")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle without a maneuver")
        .on_enter(ring::hud_instrument)
        .until(elapsed(0.8))
        .add()
        .step("open the cockpit loop")
        .on_enter(|world| loop_start(world, LOOP_NAME))
        .add()
        .step("hold the quiet view")
        .until(elapsed(0.7))
        .add()
        .step("engage GOTO")
        .on_enter(ring::engage_goto)
        .until(ring::player_burning())
        .deadline(30.0)
        .add()
        .step("read the live maneuver")
        .until(elapsed(1.0))
        .add()
        .step("open NOVA OS")
        .on_enter(press_tab)
        .until(elapsed(0.8))
        .add()
        .step("type the map command")
        .on_enter(|world| type_word(world, "map"))
        .until(frames(6))
        .add()
        .step("launch the map")
        .on_enter(press_enter)
        .until(elapsed(1.5))
        .add()
        .step("close the cockpit loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
}
