//! Mods ship their own binary resources (task 20260716-123544) and reference a
//! declared dependency's resources (task 20260716-215423). Proofs on a headless
//! asset server:
//!
//! 1. DOGFOOD: enabling the shipped `example` mod merges a scenario whose skybox
//!    and asteroid texture are the mod's OWN files - its `self://` refs resolve
//!    to `mods/example/...` (the shipped folder), not to base `assets/`.
//! 2. GATE: a `self://` ref that names no declared `resources` member is
//!    recorded as an Error content issue, so the runtime gate refuses it.
//! 3. CROSS-MOD: a `dep://<id>/` ref in a consumer resolves against dependency
//!    `<id>`'s folder; a ref to a non-declared dependency or an undeclared
//!    resource of a declared dependency is an Error content issue. `dep://base`
//!    resolves against base's folder without declaring base (base is implicit).

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
fn example_mod_self_refs_resolve_against_its_own_folder() {
    let app = merge_with_enabled(&["base", "example"]);
    let scenarios = app.world().resource::<GameScenarios>();
    let scenario = scenarios
        .get("example_arena")
        .expect("the example scenario merged");

    // The skybox is a top-level AssetRef: it must point at the mod's own folder.
    assert_eq!(
        scenario.cubemap.path(),
        Some("mods/example/textures/nebula.png"),
        "the scenario skybox self:// ref resolves against mods/example",
    );

    // The asteroid texture is nested in a spawn action; assert on the serialized
    // tree so the deep AssetRef is covered without hand-walking the action enum.
    let json = serde_json::to_string(scenario).expect("scenario serializes");
    assert!(
        json.contains("mods/example/textures/rock.png"),
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
            .errors("example_arena")
            .is_empty(),
        "the example scenario has no content-gate errors",
    );

    // The rewritten skybox path must point at a real shipped file (asset root is
    // `../../assets` for these tests), and its `.meta` sidecar - the cube
    // reinterpret - must ship alongside it. Proves the dogfood art is actually
    // where the resolved refs point, without standing up an image loader.
    let skybox = std::path::Path::new("../../assets/mods/example/textures/nebula.png");
    assert!(
        skybox.exists(),
        "the shipped skybox file exists at the rewritten path"
    );
    assert!(
        skybox.with_extension("png.meta").exists(),
        "the skybox's RowCount .meta sidecar ships next to it",
    );
    assert!(
        std::path::Path::new("../../assets/mods/example/textures/rock.png").exists(),
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

/// Merge a synthetic two-mod set through `register_bundles`: an `art` mod that
/// ships `art_resources` (no content), and a `consumer` mod whose scenario's
/// skybox is `reference`, declaring `consumer_deps`. Both are enabled. The
/// consumer merges AFTER art (it lists art as a dependency in the topo cases),
/// so art's `resource_base`/`resources` are known when the consumer is flattened.
fn merge_cross_mod(reference: &str, consumer_deps: &[&str], art_resources: &[&str]) -> App {
    let mut app = headless_app();

    // `art`: ships resources at `mods/art`, no content of its own.
    let art_bundle = app
        .world_mut()
        .resource_mut::<Assets<BundleAsset>>()
        .add(BundleAsset {
            content: vec![],
            meta: ModMeta::default(),
            new_game_scenario: None,
            resources: art_resources.iter().map(|s| s.to_string()).collect(),
            resource_base: "mods/art".to_string(),
        });

    // `consumer`: one scenario whose skybox is the ref under test.
    let scenario = ScenarioConfig {
        id: "consumer_scenario".to_string(),
        name: "Consumer".to_string(),
        description: "references a dependency's resource".to_string(),
        cubemap: nova_gameplay::prelude::AssetRef::from(reference.to_string()),
        ..Default::default()
    };
    let content = app
        .world_mut()
        .resource_mut::<Assets<ContentAsset>>()
        .add(ContentAsset(vec![Content::Scenario(scenario)]));
    let consumer_bundle = app
        .world_mut()
        .resource_mut::<Assets<BundleAsset>>()
        .add(BundleAsset {
            content: vec![content],
            meta: ModMeta {
                dependencies: consumer_deps.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            },
            new_game_scenario: None,
            resources: vec![],
            resource_base: "mods/consumer".to_string(),
        });

    let entry = |id: &str, bundle| CatalogEntry {
        decl: ModEntry {
            id: id.to_string(),
            bundle: format!("mods/{id}/{id}.bundle.ron"),
            base: false,
            hidden: false,
        },
        bundle,
    };
    let catalog = InstalledCatalog {
        entries: vec![entry("art", art_bundle), entry("consumer", consumer_bundle)],
    };
    let handle = app
        .world_mut()
        .resource_mut::<Assets<InstalledCatalog>>()
        .add(catalog);
    app.world_mut()
        .insert_resource(game_assets_with_catalog(handle));
    app.world_mut().insert_resource(EnabledMods(
        ["art".to_string(), "consumer".to_string()]
            .into_iter()
            .collect(),
    ));
    app.world_mut()
        .run_system_once(nova_assets::register_bundles_for_test)
        .expect("register bundles");
    app
}

#[test]
fn a_dep_ref_resolves_against_the_dependency_folder() {
    let app = merge_cross_mod(
        "dep://art/textures/sky.png",
        &["art"],
        &["textures/sky.png"],
    );
    let scenarios = app.world().resource::<GameScenarios>();
    let scenario = scenarios
        .get("consumer_scenario")
        .expect("the consumer scenario merged");
    assert_eq!(
        scenario.cubemap.path(),
        Some("mods/art/textures/sky.png"),
        "the dep:// ref resolves against the DEPENDENCY's folder, not the consumer's",
    );
    assert!(
        app.world()
            .resource::<ContentIssues>()
            .errors("consumer_scenario")
            .is_empty(),
        "a declared dep + declared resource merges issue-free",
    );
}

#[test]
fn a_dep_ref_to_a_non_declared_mod_is_an_error() {
    // The consumer references `art` but does NOT declare it as a dependency.
    let app = merge_cross_mod("dep://art/textures/sky.png", &[], &["textures/sky.png"]);
    let issues = app.world().resource::<ContentIssues>();
    let errors = issues.errors("consumer_scenario");
    assert_eq!(errors.len(), 1, "one gate error: {:?}", issues.0);
    assert!(
        errors[0].message.contains("not a declared dependency"),
        "the error explains the missing dependency: {}",
        errors[0].message,
    );
    // The ungated ref is left LITERAL (fails to load loudly), never rewritten
    // into a mod the consumer may not reach.
    let scenarios = app.world().resource::<GameScenarios>();
    assert_eq!(
        scenarios.get("consumer_scenario").unwrap().cubemap.path(),
        Some("dep://art/textures/sky.png"),
    );
}

#[test]
fn a_dep_ref_to_an_undeclared_resource_of_a_dependency_is_an_error() {
    // `art` is a declared dependency but ships no `textures/missing.png`.
    let app = merge_cross_mod(
        "dep://art/textures/missing.png",
        &["art"],
        &["textures/sky.png"],
    );
    let issues = app.world().resource::<ContentIssues>();
    let errors = issues.errors("consumer_scenario");
    assert_eq!(errors.len(), 1, "one gate error: {:?}", issues.0);
    assert!(
        errors[0]
            .message
            .contains("undeclared resource 'dep://art/textures/missing.png'"),
        "the error names the undeclared dependency resource: {}",
        errors[0].message,
    );
}

#[test]
fn a_nested_dep_ref_is_rewritten() {
    // The whole point of the generic serde-value walk is catching refs BURIED in
    // the content tree, not just top-level `cubemap`. Here the dep:// ref is an
    // asteroid texture nested in a spawn action.
    let mut app = headless_app();
    let art_bundle = app
        .world_mut()
        .resource_mut::<Assets<BundleAsset>>()
        .add(BundleAsset {
            content: vec![],
            meta: ModMeta::default(),
            new_game_scenario: None,
            resources: vec!["textures/rock.png".to_string()],
            resource_base: "mods/art".to_string(),
        });
    let ron = r#"(
        id: "consumer_scenario",
        name: "Consumer",
        description: "",
        cubemap: "textures/base.png",
        events: [
            (
                name: OnStart,
                filters: [],
                actions: [
                    SpawnScenarioObject((
                        base: (
                            id: "rock",
                            name: "Rock",
                            position: (0.0, 0.0, -10.0),
                            rotation: (0.0, 0.0, 0.0, 1.0),
                        ),
                        kind: Asteroid((
                            radius: 2.0,
                            texture: "dep://art/textures/rock.png",
                            health: 50.0,
                            surface_gravity: None,
                            invulnerable: false,
                            lock_signature: None,
                        )),
                    )),
                ],
            ),
        ],
    )"#;
    let scenario: ScenarioConfig = ron::from_str(ron).expect("scenario parses");
    let content = app
        .world_mut()
        .resource_mut::<Assets<ContentAsset>>()
        .add(ContentAsset(vec![Content::Scenario(scenario)]));
    let consumer_bundle = app
        .world_mut()
        .resource_mut::<Assets<BundleAsset>>()
        .add(BundleAsset {
            content: vec![content],
            meta: ModMeta {
                dependencies: vec!["art".to_string()],
                ..Default::default()
            },
            new_game_scenario: None,
            resources: vec![],
            resource_base: "mods/consumer".to_string(),
        });
    let entry = |id: &str, bundle| CatalogEntry {
        decl: ModEntry {
            id: id.to_string(),
            bundle: format!("mods/{id}/{id}.bundle.ron"),
            base: false,
            hidden: false,
        },
        bundle,
    };
    let catalog = InstalledCatalog {
        entries: vec![entry("art", art_bundle), entry("consumer", consumer_bundle)],
    };
    let handle = app
        .world_mut()
        .resource_mut::<Assets<InstalledCatalog>>()
        .add(catalog);
    app.world_mut()
        .insert_resource(game_assets_with_catalog(handle));
    app.world_mut().insert_resource(EnabledMods(
        ["art".to_string(), "consumer".to_string()]
            .into_iter()
            .collect(),
    ));
    app.world_mut()
        .run_system_once(nova_assets::register_bundles_for_test)
        .expect("register bundles");

    let scenarios = app.world().resource::<GameScenarios>();
    let scenario = scenarios
        .get("consumer_scenario")
        .expect("the consumer scenario merged");
    let json = serde_json::to_string(scenario).expect("scenario serializes");
    assert!(
        json.contains("mods/art/textures/rock.png"),
        "the NESTED asteroid-texture dep:// ref is rewritten against the dependency folder: {json}",
    );
    assert!(
        !json.contains("dep://"),
        "no dep:// sentinel survives the merge: {json}",
    );
    assert!(
        app.world()
            .resource::<ContentIssues>()
            .errors("consumer_scenario")
            .is_empty(),
        "a declared dep + declared resource merges issue-free",
    );
}

#[test]
fn a_dep_ref_to_base_resolves_against_base_folder_without_declaring_base() {
    // `base` is the implicit universal dependency: a consumer references
    // `dep://base/X` WITHOUT listing base in meta.dependencies, and it resolves
    // against base's own folder. (Synthetic base bundle; the real base art moves
    // under assets/base/ in the migration task.)
    let mut app = headless_app();
    let base_bundle = app
        .world_mut()
        .resource_mut::<Assets<BundleAsset>>()
        .add(BundleAsset {
            content: vec![],
            meta: ModMeta::default(),
            new_game_scenario: None,
            resources: vec!["textures/cubemap.png".to_string()],
            resource_base: "base".to_string(),
        });
    let scenario = ScenarioConfig {
        id: "consumer_scenario".to_string(),
        name: "Consumer".to_string(),
        description: "reuses base art".to_string(),
        cubemap: nova_gameplay::prelude::AssetRef::from(
            "dep://base/textures/cubemap.png".to_string(),
        ),
        ..Default::default() // NOTE: no `dependencies: ["base"]` - base is implicit
    };
    let content = app
        .world_mut()
        .resource_mut::<Assets<ContentAsset>>()
        .add(ContentAsset(vec![Content::Scenario(scenario)]));
    let consumer_bundle = app
        .world_mut()
        .resource_mut::<Assets<BundleAsset>>()
        .add(BundleAsset {
            content: vec![content],
            meta: ModMeta::default(),
            new_game_scenario: None,
            resources: vec![],
            resource_base: "mods/consumer".to_string(),
        });
    let catalog = InstalledCatalog {
        entries: vec![
            CatalogEntry {
                decl: ModEntry {
                    id: "base".to_string(),
                    bundle: "base/base.bundle.ron".to_string(),
                    base: true,
                    hidden: false,
                },
                bundle: base_bundle,
            },
            CatalogEntry {
                decl: ModEntry {
                    id: "consumer".to_string(),
                    bundle: "mods/consumer/consumer.bundle.ron".to_string(),
                    base: false,
                    hidden: false,
                },
                bundle: consumer_bundle,
            },
        ],
    };
    let handle = app
        .world_mut()
        .resource_mut::<Assets<InstalledCatalog>>()
        .add(catalog);
    app.world_mut()
        .insert_resource(game_assets_with_catalog(handle));
    app.world_mut().insert_resource(EnabledMods(
        ["base".to_string(), "consumer".to_string()]
            .into_iter()
            .collect(),
    ));
    app.world_mut()
        .run_system_once(nova_assets::register_bundles_for_test)
        .expect("register bundles");

    let scenarios = app.world().resource::<GameScenarios>();
    let scenario = scenarios
        .get("consumer_scenario")
        .expect("the consumer scenario merged");
    assert_eq!(
        scenario.cubemap.path(),
        Some("base/textures/cubemap.png"),
        "dep://base resolves against base's own folder",
    );
    assert!(
        app.world()
            .resource::<ContentIssues>()
            .errors("consumer_scenario")
            .is_empty(),
        "dep://base to a declared base resource merges issue-free (base implicit)",
    );
}
