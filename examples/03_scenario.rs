use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "03_scenario")]
#[command(version = "1.0.0")]
#[command(about = "A simple example showing how to create a basic scenario in nova_protocol", long_about = None)]
struct Cli;

/// The scenario this example loads. Shared between the loader and the smoke-test
/// assertion so both agree on what "loaded" means.
const SCENARIO_ID: &str = "asteroid_field";

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // Headless smoke-test harness: inert in a normal run, drives Loading ->
    // Playing and exits without panic under `BCS_AUTOPILOT`, or captures a PNG
    // under `BCS_SHOT`. Behind `debug` because the harness lives there.
    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_autopilot());
        app.add_plugins(nova_screenshot());
    }

    app.run();
}

fn custom_plugin(app: &mut App) {
    app.add_systems(
        OnEnter(GameAssetsStates::Loaded),
        |mut commands: Commands, scenarios: Res<GameScenarios>| {
            let scenario = scenarios
                .get(SCENARIO_ID)
                .expect("Scenario 'asteroid_field' not found");
            commands.trigger(LoadScenario(scenario.clone()));
        },
    );

    // Smoke-test assertion (debug-gated, like the harness itself): observe the
    // `ScenarioLoaded` init-status payload and fail the headless run if scenario
    // init is broken -- either the event carried a trivial/empty payload, or it
    // never fired at all and the scene silently came up empty.
    #[cfg(feature = "debug")]
    {
        app.init_resource::<ScenarioLoadProbe>();
        app.add_observer(assert_scenario_loaded_payload);
        app.add_systems(OnEnter(GameStates::Playing), assert_scenario_loaded_fired);
    }
}

/// Records whether `ScenarioLoaded` fired, so `assert_scenario_loaded_fired` can
/// catch the case where scenario init silently produced nothing.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct ScenarioLoadProbe {
    fired: bool,
}

/// Assert the `ScenarioLoaded` payload is non-trivial, right where the data is
/// known good. A panic here fails the `BCS_AUTOPILOT` smoke run (non-zero exit),
/// so a regression that loads the wrong scenario or spawns nothing is caught
/// instead of passing on `autopilot: cycle complete` alone.
#[cfg(feature = "debug")]
fn assert_scenario_loaded_payload(
    loaded: On<ScenarioLoaded>,
    mut probe: ResMut<ScenarioLoadProbe>,
) {
    info!(
        "smoke: ScenarioLoaded id={:?} handlers={} objects={}",
        loaded.scenario_id, loaded.handler_count, loaded.object_count
    );

    assert_eq!(
        loaded.scenario_id, SCENARIO_ID,
        "smoke: ScenarioLoaded reported scenario id {:?}, expected {:?}",
        loaded.scenario_id, SCENARIO_ID
    );
    assert!(
        loaded.handler_count > 0,
        "smoke: ScenarioLoaded reported zero event handlers -- scenario init registered no handlers"
    );
    assert!(
        loaded.object_count > 0,
        "smoke: ScenarioLoaded reported zero objects -- scenario init spawned nothing"
    );

    probe.fired = true;
}

/// By the time gameplay starts, the scenario must have loaded. If `ScenarioLoaded`
/// never fired, the payload assertion above never ran, so guard the silent-empty
/// case here: reaching `Playing` with no load is itself a failure.
#[cfg(feature = "debug")]
fn assert_scenario_loaded_fired(probe: Res<ScenarioLoadProbe>) {
    assert!(
        probe.fired,
        "smoke: reached Playing but ScenarioLoaded never fired -- scenario init silently failed"
    );
}
