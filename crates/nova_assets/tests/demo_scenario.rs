//! End-to-end proof of the RON modding pipeline: load the real
//! `assets/**/*.content.ron` files through the production `nova_modding` asset
//! loader on a headless asset server, then run the real `register_content`
//! system (through `GameAssets`) and assert the resulting `GameScenarios`
//! carries the RON-authored `"demo"` scenario ALONGSIDE the four built-ins AND
//! `GameSections` is populated from the base section content.
//!
//! This drives the exact loader and register wiring the game ships: the RON
//! decode into a `ContentAsset` via `ContentAssetLoader`, the
//! `Assets<ContentAsset>` lookup in `register_content`, and the by-variant route
//! into `GameSections` / `GameScenarios`. The asset IO reads the real workspace
//! `assets/` dir (tests run with the crate root as cwd).

use std::time::{Duration, Instant};

use bevy::{
    asset::{AssetPlugin, LoadState},
    ecs::system::RunSystemOnce,
    prelude::*,
};
use nova_assets::prelude::*;
use nova_gameplay::prelude::GameSections;
use nova_modding::prelude::{Content, ContentAsset, NovaModdingPlugin};
use nova_scenario::prelude::GameScenarios;

#[test]
fn demo_content_ron_loads_into_game_registries() {
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
    // The production modding plugin: registers ContentAsset + the
    // `*.content.ron` loader.
    app.add_plugins(NovaModdingPlugin);

    // Load every content RON through the real asset server + loader: the base
    // section catalog, the demo and the four built-in scenarios. register_content
    // looks each up by handle and routes its items by variant.
    let asset_server = app.world().resource::<AssetServer>().clone();
    let section_content: Handle<ContentAsset> = asset_server.load("sections/base.content.ron");
    let demo: Handle<ContentAsset> = asset_server.load("scenarios/demo.content.ron");
    let asteroid_field: Handle<ContentAsset> =
        asset_server.load("scenarios/asteroid_field.content.ron");
    let asteroid_next: Handle<ContentAsset> =
        asset_server.load("scenarios/asteroid_next.content.ron");
    let menu_ambience: Handle<ContentAsset> =
        asset_server.load("scenarios/menu_ambience.content.ron");
    let shakedown: Handle<ContentAsset> = asset_server.load("scenarios/shakedown_run.content.ron");
    let all_handles = [
        section_content.clone().untyped(),
        demo.clone().untyped(),
        asteroid_field.clone().untyped(),
        asteroid_next.clone().untyped(),
        menu_ambience.clone().untyped(),
        shakedown.clone().untyped(),
    ];

    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        app.update();
        let mut all_loaded = true;
        for h in &all_handles {
            match asset_server.load_state(h.id()) {
                LoadState::Loaded => {}
                LoadState::Failed(err) => panic!("an asset failed to load: {err}"),
                _ => all_loaded = false,
            }
        }
        if all_loaded {
            break;
        }
        assert!(Instant::now() < deadline, "timed out loading the assets");
        std::thread::sleep(Duration::from_millis(5));
    }

    // The loaded demo content decodes to the authored scenario: a single
    // `Content::Scenario` item, with the OnStart event's six actions.
    {
        let contents = app.world().resource::<Assets<ContentAsset>>();
        let demo_content = contents.get(&demo).expect("loaded demo content present");
        assert_eq!(demo_content.0.len(), 1);
        let Content::Scenario(scenario) = &demo_content.0[0] else {
            panic!("the demo content is a single Scenario item");
        };
        assert_eq!(scenario.id, "demo");
        assert_eq!(scenario.events.len(), 1);
        assert_eq!(scenario.events[0].actions.len(), 6);
    }

    // Build the GameAssets the register system reads: default handles for the
    // raw assets (register_content never resolves them - AssetRef stays a path),
    // and the REAL content handles we just loaded.
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
        section_content: section_content.clone(),
        demo_content: demo.clone(),
        asteroid_field_content: asteroid_field.clone(),
        asteroid_next_content: asteroid_next.clone(),
        menu_ambience_content: menu_ambience.clone(),
        shakedown_content: shakedown.clone(),
    };
    app.world_mut().insert_resource(game_assets);

    // Run the production register_content system: routes Section items into
    // GameSections and Scenario items into GameScenarios.
    app.world_mut()
        .run_system_once(nova_assets::register_content_for_test)
        .expect("register content");

    // GameSections is populated from the base section content: the named
    // prototypes the editor palette and ship scenarios reference are present.
    {
        let sections = app.world().resource::<GameSections>();
        assert!(
            !sections.is_empty(),
            "the section catalog loaded from RON into GameSections"
        );
        assert!(
            sections.get_section("basic_controller_section").is_some(),
            "a known catalog prototype id resolves"
        );
    }

    // The RON-authored demo is present in GameScenarios ALONGSIDE the four
    // built-ins.
    let scenarios = app.world().resource::<GameScenarios>();
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
