//! loop_player_flight: a built hull accelerates across the rock hollow under
//! the production player camera.
//!
//! The script holds the normal flight-burn key. It does not pose, move, or
//! replace the camera.

#[path = "shared/hollow.rs"]
mod hollow;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_player_flight")]
#[command(version = "1.0.0")]
#[command(about = "Capture a flight burn through the normal player camera")]
struct Cli;

#[cfg(feature = "debug")]
const LOOP_NAME: &str = "landing-player-flight";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin);
        app.add_plugins(flight_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(hollow::ordnance_hollow(&game_assets, &ships)));
}

#[cfg(feature = "debug")]
fn flight_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the player ship")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle the production chase camera")
        .on_enter(hide_hud)
        .until(elapsed(0.8))
        .add()
        .step("open the player-flight loop")
        .on_enter(|world| loop_start(world, LOOP_NAME))
        .until(frames(1))
        .add()
        .step("hold the built hull")
        .until(elapsed(0.5))
        .add()
        .step("start the flight burn")
        .on_enter(press_action("main_drive"))
        .until(elapsed(3.0))
        .add()
        .step("release the flight burn")
        .on_enter(release_action("main_drive"))
        .until(elapsed(1.0))
        .add()
        .step("close the player-flight loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
}
