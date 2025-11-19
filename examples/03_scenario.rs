use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "03_scenario")]
#[command(version = "1.0.0")]
#[command(about = "A simple example showing how to create a basic scenario in nova_protocol", long_about = None)]
struct Cli;

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    app.run();
}

fn custom_plugin(app: &mut App) {
    app.add_systems(
        OnEnter(GameAssetsStates::Loaded),
        |mut commands: Commands, scenarios: Res<GameScenarios>| {
            let scenario = scenarios
                .get("asteroid_field")
                .expect("Scenario 'asteroid_field' not found");
            commands.trigger(LoadScenario(scenario.clone()));
        },
    );
}
