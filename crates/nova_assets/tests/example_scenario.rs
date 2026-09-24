//! End-to-end proof of the catalog-driven modding pipeline on a headless asset
//! server. The real `mods.catalog.ron` loads through `nova_modding`'s
//! `CatalogLoader`, which loads the MANDATORY `base` bundle and, through it,
//! its `*.content.ron` files; an optional entry (example) is only declared
//! there, and is loaded here the way the game loads it - through the asset
//! server, as its own root. Then the real `register_bundles` system merges only
//! the ENABLED subset (`EnabledMods`) into `GameSections` / `GameScenarios`,
//! base first.
//!
//! The asset IO reads the real workspace `assets/` dir (tests run with the crate root
//! as cwd).

use std::time::{Duration, Instant};

use bevy::{
    asset::{AssetPlugin, RecursiveDependencyLoadState, UntypedAssetId},
    ecs::system::RunSystemOnce,
    prelude::*,
};
use nova_assets::prelude::*;
use nova_modding::prelude::{
    BundleAsset, CatalogEntry, Content, ContentAsset, InstalledCatalog, ModEntry, NovaModdingPlugin,
};
use nova_scenario::prelude::{ContentIssues, GameScenarios, NewGameStart, ScenarioConfig};
use nova_ship::prelude::GameSections;

/// A headless app with the asset server pointed at the workspace `assets/` and the
/// modding plugin (which registers the content/bundle/catalog loaders).
fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            // Tests run with the crate root as cwd; assets live at the workspace root.
            file_path: "../../assets".to_string(),
            ..default()
        },
    ));
    app.add_plugins(NovaModdingPlugin);
    // Production (GameAssetsPlugin) always inits the downloaded half of the
    // installed set; register_bundles/build_mod_catalog read it. Empty here -
    // the download path has its own rig (tests/mod_cache_install.rs).
    app.init_resource::<DownloadedMods>();
    // ...and the optional half, which is where a non-base catalog entry's
    // bundle handle lives now.
    app.init_resource::<OptionalBundles>();
    app
}

/// Pump updates until `handle`'s recursive dependency load state is `Loaded`,
/// panicking on failure or timeout.
fn wait_recursive_loaded(
    app: &mut App,
    asset_server: &AssetServer,
    handle: UntypedAssetId,
    what: &str,
) {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        app.update();
        match asset_server.get_recursive_dependency_load_state(handle) {
            Some(RecursiveDependencyLoadState::Loaded) => break,
            Some(RecursiveDependencyLoadState::Failed(err)) => {
                panic!("{what} failed to load: {err}")
            }
            _ => {}
        }
        assert!(Instant::now() < deadline, "timed out loading {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Load every OPTIONAL catalog entry's bundle and publish [`OptionalBundles`] -
/// the runtime half the game builds at `Processing`. The catalog itself loads
/// only `base`, so without this an optional mod has no bundle to merge.
fn load_optional_bundles(
    app: &mut App,
    asset_server: &AssetServer,
    catalog: &Handle<InstalledCatalog>,
) {
    let optional: Vec<OptionalBundle> = {
        let catalogs = app.world().resource::<Assets<InstalledCatalog>>();
        let installed = catalogs.get(catalog).expect("catalog loaded");
        installed
            .entries
            .iter()
            .filter(|entry| entry.bundle.is_none())
            .map(|entry| OptionalBundle {
                id: entry.decl.id.clone(),
                bundle: asset_server.load(entry.decl.bundle.clone()),
            })
            .collect()
    };
    for loaded in &optional {
        wait_recursive_loaded(
            app,
            asset_server,
            loaded.bundle.id().untyped(),
            &format!("bundle '{}'", loaded.id),
        );
    }
    app.world_mut().insert_resource(OptionalBundles(optional));
}

/// A `GameAssets` with real defaults for the raw handles (register_bundles never
/// resolves them - AssetRef stays a path) and the given loaded catalog handle.
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
        nova_crt_mark: Handle::default(),
        ui_sfx: Default::default(),
        key_glyphs: Default::default(),
        catalog,
    }
}

/// An app whose `GameAssets.catalog` points at a SYNTHETIC catalog: the real
/// base + example entries plus one extra declaration ("extra-fixture") whose
/// bundle handle REUSES the loaded example bundle. Real loaders and real
/// content still back every handle, no fixture files.
fn app_with_extra_catalog_entry() -> App {
    let mut app = headless_app();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let catalog: Handle<InstalledCatalog> = asset_server.load("mods.catalog.ron");
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        catalog.id().untyped(),
        "the mods catalog",
    );

    // The example bundle is OPTIONAL, so the catalog declares it without
    // loading it: load it here as the game does, and publish it as the optional
    // half both entries read through.
    let example_bundle: Handle<BundleAsset> = asset_server.load("mods/example/example.bundle.ron");
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        example_bundle.id().untyped(),
        "the example bundle",
    );
    app.world_mut().insert_resource(OptionalBundles(vec![
        OptionalBundle {
            id: "example".to_string(),
            bundle: example_bundle.clone(),
        },
        OptionalBundle {
            id: "extra-fixture".to_string(),
            bundle: example_bundle,
        },
    ]));

    let synthetic = {
        let catalogs = app.world().resource::<Assets<InstalledCatalog>>();
        let real = catalogs.get(&catalog).expect("catalog loaded");
        let mut entries = real.entries.clone();
        entries.push(CatalogEntry {
            decl: ModEntry {
                id: "extra-fixture".to_string(),
                bundle: "mods/example/example.bundle.ron".to_string(),
                base: false,
            },
            bundle: None,
        });
        InstalledCatalog { entries }
    };
    let handle = app
        .world_mut()
        .resource_mut::<Assets<InstalledCatalog>>()
        .add(synthetic);
    app.world_mut()
        .insert_resource(game_assets_with_catalog(handle));
    app
}

/// Load the real catalog and run `register_bundles` once with the given enabled set,
/// returning the resulting `(GameSections, GameScenarios)`.
fn merge_with_enabled(enabled: &[&str]) -> (GameSections, GameScenarios) {
    let mut app = headless_app();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let catalog: Handle<InstalledCatalog> = asset_server.load("mods.catalog.ron");
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        catalog.id().untyped(),
        "the mods catalog",
    );

    load_optional_bundles(&mut app, &asset_server, &catalog);
    app.world_mut()
        .insert_resource(game_assets_with_catalog(catalog));
    app.world_mut()
        .insert_resource(EnabledMods(enabled.iter().map(|s| s.to_string()).collect()));
    app.world_mut()
        .run_system_once(nova_assets::register_bundles_for_test)
        .expect("register bundles");

    let sections = app.world().resource::<GameSections>().clone();
    let scenarios = app.world().resource::<GameScenarios>().clone();
    (sections, scenarios)
}

/// The DEEP loader gate: every bundle the installed catalog names loads through
/// the REAL loaders - manifest, every content file, every config tree - to a
/// recursive `Loaded`. The publish tooling (`scripts/gen-portal.py`)
/// deliberately validates only what a manifest gate can, so this is the "does
/// the content actually load" half, and it runs on the bundles this repository
/// OWNS: an installed mod's own authors and linters cover theirs.
///
/// Only `base` is loaded BY the catalog; the optional entries are rooted here,
/// which is the same shape the game gives them and keeps the gate over all of
/// them.
#[test]
fn every_installed_bundle_loads_recursively() {
    let mut app = headless_app();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let catalog: Handle<InstalledCatalog> = asset_server.load("mods.catalog.ron");
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        catalog.id().untyped(),
        "the mods catalog",
    );

    // Handles, not ids: a dropped strong handle cancels the very load this
    // gate is waiting on.
    let bundles: Vec<(String, Handle<BundleAsset>)> = {
        let catalogs = app.world().resource::<Assets<InstalledCatalog>>();
        let installed = catalogs.get(&catalog).expect("catalog loaded");
        installed
            .entries
            .iter()
            .map(|entry| {
                let bundle = entry
                    .bundle
                    .clone()
                    .unwrap_or_else(|| asset_server.load::<BundleAsset>(entry.decl.bundle.clone()));
                (entry.decl.id.clone(), bundle)
            })
            .collect()
    };
    assert!(
        !bundles.is_empty(),
        "the catalog must name at least one bundle"
    );
    for (id, bundle) in &bundles {
        wait_recursive_loaded(
            &mut app,
            &asset_server,
            bundle.id().untyped(),
            &format!("bundle '{id}'"),
        );
    }
}

/// `build_mod_catalog` fills the PLAYER-FACING `ModCatalog` with the installed
/// mods, in catalog order (base first), composing each entry with the `meta`
/// block AUTHORED IN ITS OWN BUNDLE - the thin catalog carries no metadata, so
/// the exact strings below passing proves the plumbing reads the bundle. That
/// EVERY installed entry reaches the list is pinned separately by
/// `every_installed_catalog_entry_reaches_the_mod_catalog`.
#[test]
fn mod_catalog_lists_installed_mods_metadata() {
    let mut app = headless_app();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let catalog: Handle<InstalledCatalog> = asset_server.load("mods.catalog.ron");
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        catalog.id().untyped(),
        "the mods catalog",
    );

    load_optional_bundles(&mut app, &asset_server, &catalog);
    app.world_mut()
        .insert_resource(game_assets_with_catalog(catalog));
    app.world_mut().init_resource::<ModCatalog>();
    app.world_mut()
        .run_system_once(nova_assets::build_mod_catalog)
        .expect("build mod catalog");

    let mods = &app.world().resource::<ModCatalog>().0;
    assert_eq!(mods.len(), 2, "base + example are the installed catalog");
    assert_eq!(mods[0].id, "base", "base is first (load order)");
    assert!(mods[0].base, "base is flagged");
    assert_eq!(
        mods[0].meta.name, "Base Game",
        "base's display name comes from base.bundle.ron's meta"
    );
    assert_eq!(mods[1].id, "example");
    assert!(!mods[1].base);
    assert_eq!(
        mods[1].meta.name, "Example Mod",
        "example's display name comes from example.bundle.ron's meta"
    );
    assert_eq!(
        mods[1].meta.description,
        "The copy-me tutorial mod: a section overlay, a new section, a playable arena, mod-shipped art, and a menu backdrop - a little of everything.",
        "example's description comes from its bundle meta (the catalog has none)"
    );
    assert_eq!(mods[1].meta.version, "1.3.0", "bundle meta version decodes");
    assert_eq!(mods[1].meta.author, "Nova Protocol");
}

/// `build_mod_catalog` gives EVERY installed declaration a player-facing row,
/// in catalog order. There is no way to install content the player cannot see
/// or switch off, so the row count is the entry count - proved against a
/// catalog carrying one more entry than the shipped one, so a filter that
/// dropped any class of entry would fail here.
#[test]
fn every_installed_catalog_entry_reaches_the_mod_catalog() {
    let mut app = app_with_extra_catalog_entry();
    app.world_mut().init_resource::<ModCatalog>();
    app.world_mut()
        .run_system_once(nova_assets::build_mod_catalog)
        .expect("build mod catalog");

    let declared: Vec<String> = {
        let game_assets = app.world().resource::<GameAssets>();
        let catalogs = app.world().resource::<Assets<InstalledCatalog>>();
        catalogs
            .get(&game_assets.catalog)
            .expect("catalog loaded")
            .entries
            .iter()
            .map(|e| e.decl.id.clone())
            .collect()
    };
    let rows: Vec<String> = app
        .world()
        .resource::<ModCatalog>()
        .0
        .iter()
        .map(|m| m.id.clone())
        .collect();
    assert_eq!(
        rows, declared,
        "every installed entry has a row, in catalog order"
    );
    assert!(
        rows.contains(&"extra-fixture".to_string()),
        "including one no shipped catalog declares: {rows:?}"
    );
}

/// `ModInfo::new` normalizes: no bundle meta (or an empty name) falls back to the
/// catalog id, so a meta-less mod still renders a usable row; an authored meta
/// passes through untouched.
#[test]
fn mod_info_falls_back_to_id_when_meta_is_missing() {
    let decl = ModEntry {
        id: "bare-mod".to_string(),
        bundle: "mods/bare/bare.bundle.ron".to_string(),
        base: false,
    };
    let info = ModInfo::new(&decl, None);
    assert_eq!(info.meta.name, "bare-mod", "missing meta -> name = id");
    assert!(info.meta.description.is_empty());

    let authored = ModMeta {
        name: "Bare".to_string(),
        ..Default::default()
    };
    let info = ModInfo::new(&decl, Some(&authored));
    assert_eq!(info.meta.name, "Bare", "authored meta passes through");
}

/// Run `seed_enabled_mods` with `EnabledMods` pre-set to `preset` and return
/// the resulting set. Exercises the "union base ids" behaviour.
fn seed_from(preset: &[&str]) -> std::collections::HashSet<String> {
    let mut app = headless_app();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let catalog: Handle<InstalledCatalog> = asset_server.load("mods.catalog.ron");
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        catalog.id().untyped(),
        "the mods catalog",
    );

    app.world_mut()
        .insert_resource(game_assets_with_catalog(catalog));
    app.world_mut()
        .insert_resource(EnabledMods(preset.iter().map(|s| s.to_string()).collect()));
    app.world_mut()
        .run_system_once(nova_assets::seed_enabled_mods)
        .expect("seed enabled mods");
    app.world().resource::<EnabledMods>().0.clone()
}

/// `seed_enabled_mods` unions the catalog's `base:true` ids in: from empty it
/// yields the fresh-install set, which is base and nothing else, and it
/// preserves a restored non-base choice while still forcing base on (base is
/// locked in the UI).
#[test]
fn seed_enabled_mods_unions_base_over_any_restored_set() {
    // No restored prefs -> the fresh install. Base is the only entry the
    // catalog can switch on by itself, so that is base alone.
    let from_empty = seed_from(&[]);
    assert!(from_empty.contains("base"), "base is enabled by default");
    assert!(!from_empty.contains("example"), "example is off by default");

    // A restored set with a non-base mod (and NO base) -> keep the example choice AND
    // force base on (so base is never left disabled, even from a base-less set).
    let from_example = seed_from(&["example"]);
    assert!(
        from_example.contains("example"),
        "the restored example choice is preserved"
    );
    assert!(
        from_example.contains("base"),
        "base is forced on regardless of the restored set"
    );
}

/// `seed_enabled_mods` strips NOTHING. Every installed id has a menu row, so a
/// restored enablement is always one the player can undo from the Mods screen -
/// there is no class of id whose enablement has to be forced back off at boot.
/// Synthetic-catalog rig, so the set restored here names a mod the shipped
/// catalog does not.
#[test]
fn seed_enabled_mods_keeps_every_restored_installed_id() {
    let mut app = app_with_extra_catalog_entry();
    app.world_mut().insert_resource(EnabledMods(
        ["example".to_string(), "extra-fixture".to_string()]
            .into_iter()
            .collect(),
    ));
    app.world_mut()
        .run_system_once(nova_assets::seed_enabled_mods)
        .expect("seed enabled mods");
    let seeded = &app.world().resource::<EnabledMods>().0;
    assert!(
        seeded.contains("extra-fixture"),
        "a restored id survives: the player can switch it off from its own row"
    );
    assert!(seeded.contains("example"), "restored choices survive");
    assert!(seeded.contains("base"), "base is still forced on");
}

#[test]
fn catalog_loads_and_base_only_merges_by_default() {
    // Only base enabled (the startup default) -> base content merges, the example mod
    // is loaded-but-not-merged.
    let (sections, scenarios) = merge_with_enabled(&["base"]);

    assert!(
        sections.get_section("basic_controller_section").is_some(),
        "the base section catalog loaded into GameSections"
    );
    // Base hull is the base's 200, NOT the example mod's 400 override (example disabled).
    assert_eq!(
        sections
            .get_section("reinforced_hull_section")
            .expect("base hull present")
            .base
            .health,
        200.0,
        "with example disabled, the base section is un-overridden"
    );

    // The four carousel backdrops plus the training range. Mirrors
    // `base_content::scenarios::catalog`.
    for built_in in [
        "tutorial",
        "menu_duel",
        "menu_gauntlet",
        "menu_waystation",
        "menu_weave",
    ] {
        assert!(
            scenarios.contains_key(built_in),
            "built-in scenario '{built_in}' present"
        );
    }
    assert!(
        !scenarios.contains_key("example_arena"),
        "the example mod's scenario must NOT be registered while it is disabled"
    );
}

#[test]
fn enabling_example_overrides_a_section_and_adds_a_scenario() {
    // base + example enabled -> the example mod overlays the base by id and adds its scenario.
    let (sections, scenarios) = merge_with_enabled(&["base", "example"]);

    let hull = sections
        .get_section("reinforced_hull_section")
        .expect("the overridden base section is present");
    assert_eq!(
        hull.base.health, 400.0,
        "the enabled example mod must overlay the base section"
    );
    assert!(
        hull.base.name.contains("Example Mod"),
        "the mod's renamed label won: {}",
        hull.base.name
    );

    assert!(
        scenarios.contains_key("example_arena"),
        "the enabled example mod's scenario must be registered"
    );
    assert!(
        scenarios.contains_key("tutorial"),
        "base scenarios remain after the overlay"
    );
}

/// Toggling `EnabledMods` re-runs the merge LIVE (the production wiring: the same
/// `register_bundles` system gated on `resource_changed::<EnabledMods>`). This is the
/// mechanism the mods menu drives - proven across real frames, not a single
/// `run_system_once` (per the change-detection lesson).
#[test]
fn toggling_enabled_mods_remerges_live() {
    let mut app = headless_app();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let catalog: Handle<InstalledCatalog> = asset_server.load("mods.catalog.ron");
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        catalog.id().untyped(),
        "the mods catalog",
    );

    load_optional_bundles(&mut app, &asset_server, &catalog);
    app.world_mut()
        .insert_resource(game_assets_with_catalog(catalog));
    app.world_mut()
        .insert_resource(EnabledMods(["base".to_string()].into_iter().collect()));
    // The production run condition: re-merge whenever the enabled set changes.
    app.add_systems(
        Update,
        nova_assets::register_bundles_for_test.run_if(resource_changed::<EnabledMods>),
    );

    // First frame: EnabledMods just inserted (changed) -> base-only merge.
    app.update();
    assert!(
        !app.world()
            .resource::<GameScenarios>()
            .contains_key("example_arena"),
        "example disabled -> its scenario absent"
    );

    // Enable example -> the change triggers a live re-merge on the next frame.
    app.world_mut()
        .resource_mut::<EnabledMods>()
        .0
        .insert("example".to_string());
    app.update();
    assert!(
        app.world()
            .resource::<GameScenarios>()
            .contains_key("example_arena"),
        "enabling example must re-merge live and register its scenario"
    );
    assert_eq!(
        app.world()
            .resource::<GameSections>()
            .get_section("reinforced_hull_section")
            .unwrap()
            .base
            .health,
        400.0,
        "the re-merge applied the example mod's section override"
    );
}

/// Regression guard for the in-game load path. The catalog is a `GameAssets`
/// field, so bevy_asset_loader loads it UNTYPED, resolving the loader by the
/// file's FULL extension only (everything after the first dot). A bare
/// `catalog.ron` would resolve to `ron` (no loader) and fail in-game;
/// `mods.catalog.ron` yields `catalog.ron`, which `CatalogLoader` registers.
/// Loading it untyped here (the game's path) must resolve and reach `Loaded`,
/// pulling in every installed bundle.
#[test]
fn catalog_untyped_load_resolves_the_loader() {
    let mut app = headless_app();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let handle = asset_server.load_builder().load_untyped("mods.catalog.ron");
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        handle.id().untyped(),
        "the untyped mods catalog",
    );
}

/// Lower-level proof of the overlay itself: load the base and example bundles
/// directly, flatten to `Content`, and run the pure `merge_bundles` with the
/// mod after the base. Complements the system-level tests above by pinning the
/// merge core independent of the catalog/EnabledMods plumbing.
#[test]
fn merge_bundles_overlays_example_over_base() {
    let mut app = headless_app();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let base: Handle<BundleAsset> = asset_server.load("base/base.bundle.ron");
    let example: Handle<BundleAsset> = asset_server.load("mods/example/example.bundle.ron");
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        base.id().untyped(),
        "the base bundle",
    );
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        example.id().untyped(),
        "the example bundle",
    );

    let bundles = app.world().resource::<Assets<BundleAsset>>();
    let contents = app.world().resource::<Assets<ContentAsset>>();
    let flatten = |bundle: &Handle<BundleAsset>| -> Vec<Content> {
        bundles
            .get(bundle)
            .expect("bundle loaded")
            .content
            .iter()
            .flat_map(|h| contents.get(h).expect("content loaded").0.iter().cloned())
            .collect()
    };
    let base_items = flatten(&base);
    let example_items = flatten(&example);

    let outcome = nova_assets::merge_bundles([base_items.iter(), example_items.iter()]);
    assert!(
        outcome.conflicts.is_empty(),
        "clean data has no intra-bundle conflicts: {:?}",
        outcome.conflicts
    );
    let hull = outcome
        .sections
        .iter()
        .find(|s| s.base.id == "reinforced_hull_section")
        .expect("the overridden section is present");
    assert_eq!(
        hull.base.health, 400.0,
        "the mod's section overlays the base"
    );
    assert!(
        outcome.scenarios.contains_key("example_arena"),
        "the mod's new scenario is added"
    );
    assert!(
        outcome.scenarios.contains_key("tutorial"),
        "a base scenario remains after overlay"
    );
}

/// The shipped base bundle declares the New Game start: after the real merge,
/// `NewGameStart` carries `base.bundle.ron`'s `new_game_scenario` (the menu
/// reads this resource instead of naming any id).
#[test]
fn base_bundle_declares_the_new_game_start() {
    let mut app = headless_app();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let catalog: Handle<InstalledCatalog> = asset_server.load("mods.catalog.ron");
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        catalog.id().untyped(),
        "the mods catalog",
    );
    app.world_mut()
        .insert_resource(game_assets_with_catalog(catalog));
    app.world_mut()
        .insert_resource(EnabledMods(["base".to_string()].into_iter().collect()));
    app.world_mut()
        .run_system_once(nova_assets::register_bundles_for_test)
        .expect("register bundles");

    assert_eq!(
        app.world().resource::<NewGameStart>(),
        &NewGameStart(Some("open_world".to_string())),
        "the merge writes the base bundle's declared start"
    );
}

/// Only the BASE bundle's `new_game_scenario` is honored: a non-base bundle
/// declaring one is ignored (warned), so a mod cannot redirect New Game (the
/// trust rule).
#[test]
fn new_game_declaration_is_honored_only_from_base() {
    let mut app = headless_app();
    let (base_bundle, mod_bundle) = {
        let mut bundles = app.world_mut().resource_mut::<Assets<BundleAsset>>();
        (
            bundles.add(BundleAsset {
                content: vec![],
                meta: ModMeta::default(),
                new_game_scenario: Some("base_start".to_string()),
                resources: vec![],
                resource_base: "base".to_string(),
            }),
            bundles.add(BundleAsset {
                content: vec![],
                meta: ModMeta::default(),
                new_game_scenario: Some("hijacked_start".to_string()),
                resources: vec![],
                resource_base: "mods/hijack".to_string(),
            }),
        )
    };
    let synthetic = InstalledCatalog {
        entries: vec![
            CatalogEntry {
                decl: ModEntry {
                    id: "base".to_string(),
                    bundle: "base/base.bundle.ron".to_string(),
                    base: true,
                },
                bundle: Some(base_bundle),
            },
            CatalogEntry {
                decl: ModEntry {
                    id: "sneaky".to_string(),
                    bundle: "mods/sneaky/sneaky.bundle.ron".to_string(),
                    base: false,
                },
                bundle: Some(mod_bundle),
            },
        ],
    };
    let handle = app
        .world_mut()
        .resource_mut::<Assets<InstalledCatalog>>()
        .add(synthetic);
    app.world_mut()
        .insert_resource(game_assets_with_catalog(handle));
    // BOTH enabled: enablement must not grant the non-base bundle the start.
    app.world_mut().insert_resource(EnabledMods(
        ["base".to_string(), "sneaky".to_string()]
            .into_iter()
            .collect(),
    ));
    app.world_mut()
        .run_system_once(nova_assets::register_bundles_for_test)
        .expect("register bundles");

    assert_eq!(
        app.world().resource::<NewGameStart>(),
        &NewGameStart(Some("base_start".to_string())),
        "the enabled non-base declaration must not override the base one"
    );
}

/// The runtime content gate's merge sweep: the real shipped catalog merges with
/// ZERO content issues (the clean-tree pin the static gate also enforces), and
/// a synthetic bundle whose scenario names a missing prototype lands in
/// `ContentIssues` keyed by scenario id.
#[test]
fn merge_sweep_flags_bad_content_and_passes_the_shipped_tree() {
    // Clean pin: the real catalog.
    let mut app = headless_app();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let catalog: Handle<InstalledCatalog> = asset_server.load("mods.catalog.ron");
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        catalog.id().untyped(),
        "the mods catalog",
    );
    app.world_mut()
        .insert_resource(game_assets_with_catalog(catalog));
    app.world_mut()
        .insert_resource(EnabledMods(["base".to_string()].into_iter().collect()));
    app.world_mut()
        .run_system_once(nova_assets::register_bundles_for_test)
        .expect("register bundles");
    assert!(
        app.world().resource::<ContentIssues>().0.is_empty(),
        "the shipped tree must merge issue-free: {:?}",
        app.world().resource::<ContentIssues>().0
    );

    // Sweep pin: a synthetic bundle with a broken scenario.
    let mut app = headless_app();
    let broken = ScenarioConfig {
        description: "merge sweep pin".to_string(),
        events: vec![nova_scenario::prelude::ScenarioEventConfig {
            label: None,
            name: nova_scenario::prelude::EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: vec![nova_scenario::prelude::EventActionConfig::NextScenario(
                nova_scenario::prelude::NextScenarioActionConfig {
                    scenario_id: "no_such_chapter".to_string(),
                    linger: true,
                    delay: None,
                },
            )],
        }],
        ..ScenarioConfig::new(
            "broken_scenario".to_string(),
            "Broken".to_string(),
            nova_gameplay::prelude::AssetRef::from("textures/x.png".to_string()),
        )
    };
    let content = app
        .world_mut()
        .resource_mut::<Assets<ContentAsset>>()
        .add(ContentAsset(vec![Content::Scenario(broken)]));
    let bundle = app
        .world_mut()
        .resource_mut::<Assets<BundleAsset>>()
        .add(BundleAsset {
            content: vec![content],
            meta: ModMeta::default(),
            new_game_scenario: None,
            resources: vec![],
            resource_base: "mods/broken".to_string(),
        });
    let synthetic = InstalledCatalog {
        entries: vec![CatalogEntry {
            decl: ModEntry {
                id: "base".to_string(),
                bundle: "base/base.bundle.ron".to_string(),
                base: true,
            },
            bundle: Some(bundle),
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
    let errors = issues.errors("broken_scenario");
    assert_eq!(errors.len(), 1, "{:?}", issues.0);
    assert!(errors[0].message.contains("no_such_chapter"));
}
