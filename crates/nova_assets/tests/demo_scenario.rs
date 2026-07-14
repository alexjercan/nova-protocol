//! End-to-end proof of the RON modding pipeline: load the real
//! `assets/scenarios/demo.scenario.ron` through the production
//! `nova_modding` asset loader on a headless asset server, then run the real
//! `register_scenario` system (through `GameAssets`) and assert the resulting
//! `GameScenarios` resource carries the RON-authored `"demo"` scenario
//! ALONGSIDE the four code-built built-ins.
//!
//! This drives the exact loader and register wiring the game ships: the RON
//! decode into a `ScenarioAsset` via `ScenarioAssetLoader`, the
//! `Assets<ScenarioAsset>` lookup in `register_scenario`, and the additive
//! insert into `GameScenarios`. The asset IO reads the real workspace
//! `assets/` dir (tests run with the crate root as cwd).

use std::time::{Duration, Instant};

use bevy::{
    asset::{AssetPlugin, LoadState},
    ecs::system::RunSystemOnce,
    prelude::*,
};
use nova_assets::prelude::*;
use nova_modding::prelude::{NovaModdingPlugin, ScenarioAsset};
use nova_scenario::prelude::GameScenarios;

#[test]
fn demo_scenario_ron_loads_into_game_scenarios() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            // Tests run with the crate root as cwd; the asset folder lives at
            // the workspace root.
            file_path: "../../assets".to_string(),
            ..default()
        },
    ));
    // The production modding plugin: registers ScenarioAsset + the
    // `*.scenario.ron` loader.
    app.add_plugins(NovaModdingPlugin);

    // Load every scenario RON through the real asset server + loader: the demo
    // plus the four built-ins (which are now data files too - task
    // 20260525-133028 follow-up). register_scenario looks each up by handle.
    let asset_server = app.world().resource::<AssetServer>().clone();
    let handle: Handle<ScenarioAsset> = asset_server.load("scenarios/demo.scenario.ron");
    let asteroid_field: Handle<ScenarioAsset> =
        asset_server.load("scenarios/asteroid_field.scenario.ron");
    let asteroid_next: Handle<ScenarioAsset> =
        asset_server.load("scenarios/asteroid_next.scenario.ron");
    let menu_ambience: Handle<ScenarioAsset> =
        asset_server.load("scenarios/menu_ambience.scenario.ron");
    let shakedown: Handle<ScenarioAsset> =
        asset_server.load("scenarios/shakedown_run.scenario.ron");
    let all_handles = [
        &handle,
        &asteroid_field,
        &asteroid_next,
        &menu_ambience,
        &shakedown,
    ];

    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        app.update();
        let mut all_loaded = true;
        for h in all_handles {
            match asset_server.load_state(h) {
                LoadState::Loaded => {}
                LoadState::Failed(err) => panic!("a scenario failed to load: {err}"),
                _ => all_loaded = false,
            }
        }
        if all_loaded {
            break;
        }
        assert!(Instant::now() < deadline, "timed out loading the scenarios");
        std::thread::sleep(Duration::from_millis(5));
    }

    // The loaded config decodes to the authored scenario.
    {
        let scenarios = app.world().resource::<Assets<ScenarioAsset>>();
        let demo = scenarios.get(&handle).expect("loaded demo asset present");
        assert_eq!(demo.0.id, "demo");
        // OnStart event with the debug message, objective, three asteroids and
        // a beacon.
        assert_eq!(demo.0.events.len(), 1);
        assert_eq!(demo.0.events[0].actions.len(), 6);
    }

    // Build the GameAssets the register system reads: default handles for the
    // built-ins' assets (register_scenario never resolves them - AssetRef
    // stays a path), and the REAL demo handle we just loaded.
    let game_assets = GameAssets {
        cubemap: Handle::default(),
        asteroid_texture: Handle::default(),
        hull_01: Handle::default(),
        turret_yaw_01: Handle::default(),
        turret_pitch_01: Handle::default(),
        turret_barrel_01: Handle::default(),
        torpedo_bay_01: Handle::default(),
        fps_icon: Handle::default(),
        target_sprite: Handle::default(),
        demo_scenario: handle.clone(),
        asteroid_field_scenario: asteroid_field.clone(),
        asteroid_next_scenario: asteroid_next.clone(),
        menu_ambience_scenario: menu_ambience.clone(),
        shakedown_scenario: shakedown.clone(),
    };
    app.world_mut().insert_resource(game_assets);

    // The real section registry (the built-in ship scenarios reference it).
    app.world_mut()
        .run_system_once(nova_assets::register_sections_for_test)
        .expect("register sections");

    // Run the production register_scenario system.
    app.world_mut()
        .run_system_once(nova_assets::register_scenario_for_test)
        .expect("register scenario");

    let scenarios = app.world().resource::<GameScenarios>();
    // The RON-authored demo is present ALONGSIDE the four built-ins.
    assert!(
        scenarios.contains_key("demo"),
        "the RON-authored demo scenario must be registered"
    );
    for built_in in [
        "asteroid_field",
        "asteroid_next",
        "menu_ambience",
        "shakedown_run",
    ] {
        assert!(
            scenarios.contains_key(built_in),
            "built-in scenario '{built_in}' must still be registered"
        );
    }
    assert_eq!(
        scenarios.get("demo").map(|s| s.id.as_str()),
        Some("demo"),
        "the demo entry is keyed by and carries the authored id"
    );
}
