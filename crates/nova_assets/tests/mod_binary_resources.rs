//! Mods ship their own binary resources (task 20260716-123544). Two proofs on a
//! headless asset server reading the real workspace `assets/`:
//!
//! 1. DOGFOOD: enabling the shipped `variety` mod merges a scenario whose skybox
//!    and asteroid texture are the mod's OWN files - its `self://` refs resolve
//!    to `mods/variety/...` (the shipped folder), not to base `assets/`.
//! 2. GATE: a `self://` ref that names no declared `resources` member is
//!    recorded as an Error content issue, so the runtime gate refuses it.

use std::time::{Duration, Instant};

use bevy::{
    asset::{AssetPlugin, RecursiveDependencyLoadState, UntypedAssetId},
    ecs::system::RunSystemOnce,
    prelude::*,
};
use nova_assets::prelude::*;
use nova_modding::prelude::{
    BundleAsset, CatalogEntry, Content, ContentAsset, InstalledCatalog, ModEntry, ModMeta,
    NovaModdingPlugin,
};
use nova_scenario::prelude::{ContentIssues, GameScenarios, ScenarioConfig};

/// A headless app: asset server on the workspace `assets/`, modding loaders, and
/// an empty downloaded set (production always inits it; register_bundles reads it).
fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: "../../assets".to_string(),
            ..default()
        },
    ));
    app.add_plugins(NovaModdingPlugin);
    app.init_resource::<DownloadedMods>();
    app
}

/// Pump updates until `handle` is recursively loaded, panicking on failure/timeout.
fn wait_recursive_loaded(app: &mut App, server: &AssetServer, handle: UntypedAssetId, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        app.update();
        match server.get_recursive_dependency_load_state(handle) {
            Some(RecursiveDependencyLoadState::Loaded) => break,
            Some(RecursiveDependencyLoadState::Failed(err)) => panic!("{what} failed: {err}"),
            _ => {}
        }
        assert!(Instant::now() < deadline, "timed out loading {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// A `GameAssets` with default raw handles (register_bundles keeps AssetRefs as
/// paths, never resolving these) and the given catalog handle.
fn game_assets_with_catalog(catalog: Handle<InstalledCatalog>) -> GameAssets {
    GameAssets {
        cubemap: Handle::default(),
        asteroid_texture: Handle::default(),
        hull_01: Handle::default(),
        turret_yaw_01: Handle::default(),
        turret_pitch_01: Handle::default(),
        turret_barrel_01: Handle::default(),
        torpedo_bay_01: Handle::default(),
        fps_icon: Handle::default(),
        target_sprite: Handle::default(),
        catalog,
    }
}

/// Load the real catalog, enable `enabled`, run `register_bundles` once.
fn merge_with_enabled(enabled: &[&str]) -> App {
    let mut app = headless_app();
    let server = app.world().resource::<AssetServer>().clone();
    let catalog: Handle<InstalledCatalog> = server.load("mods.catalog.ron");
    wait_recursive_loaded(
        &mut app,
        &server,
        catalog.id().untyped(),
        "the mods catalog",
    );

    app.world_mut()
        .insert_resource(game_assets_with_catalog(catalog));
    app.world_mut()
        .insert_resource(EnabledMods(enabled.iter().map(|s| s.to_string()).collect()));
    app.world_mut()
        .run_system_once(nova_assets::register_bundles_for_test)
        .expect("register bundles");
    app
}

#[test]
fn variety_mod_self_refs_resolve_against_its_own_folder() {
    let app = merge_with_enabled(&["base", "variety"]);
    let scenarios = app.world().resource::<GameScenarios>();
    let scenario = scenarios
        .get("variety_pack_showcase")
        .expect("the variety scenario merged");

    // The skybox is a top-level AssetRef: it must point at the mod's own folder.
    assert_eq!(
        scenario.cubemap.path(),
        Some("mods/variety/textures/nebula.png"),
        "the scenario skybox self:// ref resolves against mods/variety",
    );

    // The asteroid texture is nested in a spawn action; assert on the serialized
    // tree so the deep AssetRef is covered without hand-walking the action enum.
    let json = serde_json::to_string(scenario).expect("scenario serializes");
    assert!(
        json.contains("mods/variety/textures/rock.png"),
        "the nested asteroid texture self:// ref is rewritten too: {json}",
    );
    assert!(
        !json.contains("self://"),
        "no self:// sentinel survives the merge: {json}",
    );

    // And it must merge issue-free (all self:// refs are declared resources).
    assert!(
        app.world()
            .resource::<ContentIssues>()
            .errors("variety_pack_showcase")
            .is_empty(),
        "the variety scenario has no content-gate errors",
    );

    // The rewritten skybox path must point at a real shipped file (asset root is
    // `../../assets` for these tests), and its `.meta` sidecar - the cube
    // reinterpret - must ship alongside it. Proves the dogfood art is actually
    // where the resolved refs point, without standing up an image loader.
    let skybox = std::path::Path::new("../../assets/mods/variety/textures/nebula.png");
    assert!(
        skybox.exists(),
        "the shipped skybox file exists at the rewritten path"
    );
    assert!(
        skybox.with_extension("png.meta").exists(),
        "the skybox's RowCount .meta sidecar ships next to it",
    );
    assert!(
        std::path::Path::new("../../assets/mods/variety/textures/rock.png").exists(),
        "the shipped asteroid texture exists at the rewritten path",
    );
}

#[test]
fn an_undeclared_self_ref_is_an_error_content_issue() {
    // A synthetic mod whose scenario references a resource it does NOT declare.
    let mut app = headless_app();
    let scenario = ScenarioConfig {
        id: "greedy_scenario".to_string(),
        name: "Greedy".to_string(),
        description: "references a resource it never shipped".to_string(),
        cubemap: nova_gameplay::prelude::AssetRef::from("self://textures/missing.png".to_string()),
        ..Default::default()
    };
    let content = app
        .world_mut()
        .resource_mut::<Assets<ContentAsset>>()
        .add(ContentAsset(vec![Content::Scenario(scenario)]));
    let bundle = app
        .world_mut()
        .resource_mut::<Assets<BundleAsset>>()
        .add(BundleAsset {
            content: vec![content],
            meta: ModMeta::default(),
            new_game_scenario: None,
            // Declares NO resources, so the self:// ref above is undeclared.
            resources: vec![],
            resource_base: "mods/greedy".to_string(),
        });
    let synthetic = InstalledCatalog {
        entries: vec![CatalogEntry {
            decl: ModEntry {
                id: "base".to_string(),
                bundle: "base/base.bundle.ron".to_string(),
                base: true,
                hidden: false,
            },
            bundle,
        }],
    };
    let handle = app
        .world_mut()
        .resource_mut::<Assets<InstalledCatalog>>()
        .add(synthetic);
    app.world_mut()
        .insert_resource(game_assets_with_catalog(handle));
    app.world_mut()
        .insert_resource(EnabledMods(["base".to_string()].into_iter().collect()));
    app.world_mut()
        .run_system_once(nova_assets::register_bundles_for_test)
        .expect("register bundles");

    let issues = app.world().resource::<ContentIssues>();
    let errors = issues.errors("greedy_scenario");
    assert_eq!(
        errors.len(),
        1,
        "one undeclared-resource error: {:?}",
        issues.0
    );
    assert!(
        errors[0].message.contains("self://textures/missing.png"),
        "the error names the undeclared resource: {}",
        errors[0].message,
    );
}
