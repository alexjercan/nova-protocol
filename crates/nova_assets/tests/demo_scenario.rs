//! End-to-end proof of the folder-bundle modding pipeline: load the real
//! `assets/base/base.bundle.ron` through the production `nova_modding` bundle loader
//! on a headless asset server. The bundle loader recursively loads every content
//! file the manifest lists, so waiting for the bundle's RECURSIVE load state
//! reaching `Loaded` waits for all of its content. Then run the real
//! `register_bundles` system (through `GameAssets`) and assert the resulting
//! `GameScenarios` carries the RON-authored `"demo"` scenario ALONGSIDE the four
//! built-ins AND `GameSections` is populated from the base section content.
//!
//! This drives the bundle decode + route wiring: the `base.bundle.ron` decode
//! into a `BundleAsset` (with its content handles) via `BundleAssetLoader`, the
//! recursive load of each `ContentAsset`, and the `register_bundles` route into
//! `GameSections` / `GameScenarios`. The asset IO reads the real workspace
//! `assets/` dir (tests run with the crate root as cwd).
//!
//! NOTE: this test loads the bundle with a TYPED `Handle<BundleAsset>`, which
//! resolves the loader by asset type. The game loads it UNTYPED through
//! bevy_asset_loader (extension-only resolution) - see `bundle_untyped_load`
//! for the guard that pins that path.

use std::time::{Duration, Instant};

use bevy::{
    asset::{AssetPlugin, RecursiveDependencyLoadState},
    ecs::system::RunSystemOnce,
    prelude::*,
};
use nova_assets::prelude::*;
use nova_gameplay::prelude::GameSections;
use nova_modding::prelude::{BundleAsset, Content, ContentAsset, NovaModdingPlugin};
use nova_scenario::prelude::GameScenarios;

#[test]
fn base_bundle_loads_into_game_registries() {
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
    // The production modding plugin: registers ContentAsset + BundleAsset and
    // their `*.content.ron` / `*.bundle.ron` loaders.
    app.add_plugins(NovaModdingPlugin);

    // Load the base bundle through the real asset server + loader. The bundle
    // loader `load_context.load`s each content file the manifest lists, so the
    // bundle's RECURSIVE dependency load state waits for all of them.
    let asset_server = app.world().resource::<AssetServer>().clone();
    let base_bundle: Handle<BundleAsset> = asset_server.load("base/base.bundle.ron");

    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        app.update();
        match asset_server.recursive_dependency_load_state(&base_bundle) {
            RecursiveDependencyLoadState::Loaded => break,
            RecursiveDependencyLoadState::Failed(err) => {
                panic!("the base bundle (or its content) failed to load: {err}")
            }
            _ => {}
        }
        assert!(
            Instant::now() < deadline,
            "timed out loading the base bundle + its content"
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    // The bundle carries a content handle per manifest entry (six files: the
    // section catalog + five scenarios).
    let demo = {
        let bundles = app.world().resource::<Assets<BundleAsset>>();
        let bundle = bundles.get(&base_bundle).expect("base bundle present");
        assert_eq!(
            bundle.content.len(),
            6,
            "the manifest lists six content files"
        );

        // Find the demo content among the bundle's content handles and assert it
        // decoded to the authored scenario: a single `Content::Scenario` item
        // with the OnStart event's six actions.
        let contents = app.world().resource::<Assets<ContentAsset>>();
        let mut demo_handle = None;
        for handle in &bundle.content {
            let content = contents.get(handle).expect("bundle content loaded");
            for item in &content.0 {
                if let Content::Scenario(scenario) = item {
                    if scenario.id == "demo" {
                        assert_eq!(scenario.events.len(), 1);
                        assert_eq!(scenario.events[0].actions.len(), 6);
                        demo_handle = Some(handle.clone());
                    }
                }
            }
        }
        demo_handle.expect("the demo scenario is present in the bundle's content")
    };
    let _ = demo;

    // Build the GameAssets the register system reads: default handles for the
    // raw assets (register_bundles never resolves them - AssetRef stays a path),
    // and the REAL base bundle handle we just loaded.
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
        base_bundle: base_bundle.clone(),
    };
    app.world_mut().insert_resource(game_assets);

    // Run the production register_bundles system: routes Section items into
    // GameSections and Scenario items into GameScenarios.
    app.world_mut()
        .run_system_once(nova_assets::register_bundles_for_test)
        .expect("register bundles");

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

/// Regression guard for the in-game load path (task 20260714-163342).
///
/// bevy_asset_loader kicks off every `GameAssets` field with an UNTYPED
/// `asset_server.load_untyped(path)`, which resolves the loader by EXTENSION
/// ONLY - there is no asset type to fall back on. Bevy's full extension is
/// everything after the FIRST dot in the file name, so a manifest named
/// `bundle.ron` resolves to the bare `ron` extension (no loader) and the base
/// bundle silently fails to load in the running game, leaving the section /
/// scenario registries empty. The `<pack>.bundle.ron` stem makes the full
/// extension `bundle.ron`, which `BundleAssetLoader` registers.
///
/// This test loads the base bundle exactly as the game does - UNTYPED - and
/// asserts it resolves and reaches `Loaded` (never `Failed`). It fails under the
/// old `bundle.ron` name; the typed `base_bundle_loads_into_game_registries`
/// test above cannot catch this because the type gives it a by-type fallback.
#[test]
fn bundle_untyped_load_resolves_the_loader() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: "../../assets".to_string(),
            ..default()
        },
    ));
    app.add_plugins(NovaModdingPlugin);

    let asset_server = app.world().resource::<AssetServer>().clone();
    // UNTYPED, mirroring bevy_asset_loader's collection kickoff.
    let handle = asset_server.load_untyped("base/base.bundle.ron");

    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        app.update();
        match asset_server.recursive_dependency_load_state(&handle) {
            RecursiveDependencyLoadState::Loaded => break,
            RecursiveDependencyLoadState::Failed(err) => panic!(
                "untyped load of base/base.bundle.ron failed - the loader did not \
                 resolve by extension (this is the in-game failure mode): {err}"
            ),
            _ => {}
        }
        assert!(
            Instant::now() < deadline,
            "timed out on the untyped base bundle load"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
