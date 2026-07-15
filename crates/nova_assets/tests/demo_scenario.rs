//! End-to-end proof of the catalog-driven modding pipeline on a headless asset
//! server (task 20260714-174120, on 134119/134127). The real `mods.catalog.ron`
//! loads through `nova_modding`'s `CatalogLoader`, which loads EVERY installed mod's
//! `*.bundle.ron` (base + demo + the hidden screenshot-reel) and, through each, its
//! `*.content.ron` files. Waiting
//! for the catalog's RECURSIVE load state waits for that whole tree. Then the real
//! `register_bundles` system merges only the ENABLED subset (`EnabledMods`) into
//! `GameSections` / `GameScenarios`, base first.
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
use nova_gameplay::prelude::GameSections;
use nova_modding::prelude::{
    BundleAsset, Content, ContentAsset, InstalledCatalog, NovaModdingPlugin,
};
use nova_scenario::prelude::GameScenarios;

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
        catalog,
    }
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

/// `build_mod_catalog` fills the PLAYER-FACING `ModCatalog` with the installed
/// mods, in catalog order (base first), FILTERING `hidden: true` entries and
/// composing each entry with the `meta` block AUTHORED IN ITS OWN BUNDLE - the
/// thin catalog carries no metadata, so the exact strings below passing proves
/// the plumbing reads the bundle (task 20260715-142849 on 142844).
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

    app.world_mut()
        .insert_resource(game_assets_with_catalog(catalog));
    app.world_mut().init_resource::<ModCatalog>();
    app.world_mut()
        .run_system_once(nova_assets::build_mod_catalog)
        .expect("build mod catalog");

    let mods = &app.world().resource::<ModCatalog>().0;
    assert_eq!(
        mods.len(),
        2,
        "base + demo are player-visible; the hidden screenshot-reel is filtered"
    );
    assert_eq!(mods[0].id, "base", "base is first (load order)");
    assert!(mods[0].base, "base is flagged");
    assert_eq!(
        mods[0].meta.name, "Base Game",
        "base's display name comes from base.bundle.ron's meta"
    );
    assert_eq!(mods[1].id, "demo");
    assert!(!mods[1].base);
    assert_eq!(
        mods[1].meta.name, "Demo Mod",
        "demo's display name comes from demo.bundle.ron's meta"
    );
    assert_eq!(
        mods[1].meta.description,
        "Example mod: up-armors a hull section and adds an arena scenario.",
        "demo's description comes from its bundle meta (the catalog has none)"
    );
    assert_eq!(mods[1].meta.version, "1.0.0", "bundle meta version decodes");
    assert_eq!(mods[1].meta.author, "Nova Protocol");
    assert!(
        !mods.iter().any(|m| m.id == "screenshot-reel"),
        "the hidden dev mod must not reach the player-facing list"
    );
}

/// `ModInfo::new` normalizes: no bundle meta (or an empty name) falls back to the
/// catalog id, so a meta-less mod still renders a usable row; an authored meta
/// passes through untouched.
#[test]
fn mod_info_falls_back_to_id_when_meta_is_missing() {
    let decl = nova_modding::prelude::ModEntry {
        id: "bare-mod".to_string(),
        bundle: "mods/bare/bare.bundle.ron".to_string(),
        base: false,
        hidden: false,
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

/// Hidden is NOT disabled: a `hidden: true` catalog entry stays installed and merges
/// through the production `register_bundles` path when its id is enabled - the
/// contract `examples/13_screenshot_reel.rs` relies on (it inserts the id into
/// `EnabledMods` directly, no menu involved).
#[test]
fn hidden_mod_still_merges_when_enabled_by_id() {
    let (_, scenarios) = merge_with_enabled(&["base", "screenshot-reel"]);
    assert!(
        scenarios.contains_key("screenshot_reel"),
        "the hidden mod's scenario must register when the mod is enabled by id"
    );
}

/// Run `seed_enabled_mods` with `EnabledMods` pre-set to `preset` and return the
/// resulting set. Exercises the "union base ids" behaviour (task 174131).
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

/// `seed_enabled_mods` unions the catalog's `base:true` ids in: from empty it yields
/// the base-only default (unchanged pre-persistence startup), and it preserves a
/// restored non-base choice while still forcing base on (base is locked in the UI).
#[test]
fn seed_enabled_mods_unions_base_over_any_restored_set() {
    // No restored prefs -> base-only default.
    let from_empty = seed_from(&[]);
    assert!(from_empty.contains("base"), "base is enabled by default");
    assert!(!from_empty.contains("demo"), "demo is off by default");

    // A restored set with a non-base mod (and NO base) -> keep the demo choice AND
    // force base on (so base is never left disabled, even from a base-less set).
    let from_demo = seed_from(&["demo"]);
    assert!(
        from_demo.contains("demo"),
        "the restored demo choice is preserved"
    );
    assert!(
        from_demo.contains("base"),
        "base is forced on regardless of the restored set"
    );
}

/// `seed_enabled_mods` strips restored HIDDEN ids: a hidden mod's enablement is
/// session-only, so an example run that persisted `screenshot-reel` cannot leave it
/// stuck-enabled with no menu row to disable it (task 20260715-142844 R1.1). The
/// visible restored choice survives; examples re-enable by id AFTER this chain
/// (`OnEnter(Loaded)`), which `hidden_mod_still_merges_when_enabled_by_id` covers.
#[test]
fn seed_enabled_mods_strips_restored_hidden_ids() {
    let seeded = seed_from(&["demo", "screenshot-reel"]);
    assert!(
        !seeded.contains("screenshot-reel"),
        "a restored hidden id must be stripped (session-only enablement)"
    );
    assert!(seeded.contains("demo"), "visible restored choices survive");
    assert!(seeded.contains("base"), "base is still forced on");
}

#[test]
fn catalog_loads_and_base_only_merges_by_default() {
    // Only base enabled (the startup default) -> base content merges, the demo mod
    // is loaded-but-not-merged.
    let (sections, scenarios) = merge_with_enabled(&["base"]);

    assert!(
        sections.get_section("basic_controller_section").is_some(),
        "the base section catalog loaded into GameSections"
    );
    // Base hull is the base's 200, NOT the demo mod's 400 override (demo disabled).
    assert_eq!(
        sections
            .get_section("reinforced_hull_section")
            .expect("base hull present")
            .base
            .health,
        200.0,
        "with demo disabled, the base section is un-overridden"
    );

    assert!(
        scenarios.contains_key("demo"),
        "base 'demo' scenario present"
    );
    for built_in in [
        "asteroid_field",
        "asteroid_next",
        "menu_ambience",
        "shakedown_run",
    ] {
        assert!(
            scenarios.contains_key(built_in),
            "built-in scenario '{built_in}' present"
        );
    }
    assert!(
        !scenarios.contains_key("demo_mod_arena"),
        "the demo mod's scenario must NOT be registered while it is disabled"
    );
}

#[test]
fn enabling_demo_overrides_a_section_and_adds_a_scenario() {
    // base + demo enabled -> the demo mod overlays the base by id and adds its scenario.
    let (sections, scenarios) = merge_with_enabled(&["base", "demo"]);

    let hull = sections
        .get_section("reinforced_hull_section")
        .expect("the overridden base section is present");
    assert_eq!(
        hull.base.health, 400.0,
        "the enabled demo mod must overlay the base section"
    );
    assert!(
        hull.base.name.contains("Demo Mod"),
        "the mod's renamed label won: {}",
        hull.base.name
    );

    assert!(
        scenarios.contains_key("demo_mod_arena"),
        "the enabled demo mod's scenario must be registered"
    );
    assert!(
        scenarios.contains_key("demo"),
        "base scenarios remain after the overlay"
    );
}

/// Toggling `EnabledMods` re-runs the merge LIVE (the production wiring: the same
/// `register_bundles` system gated on `resource_changed::<EnabledMods>`). This is the
/// mechanism the mods menu (174126) drives - proven across real frames, not a single
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
            .contains_key("demo_mod_arena"),
        "demo disabled -> its scenario absent"
    );

    // Enable demo -> the change triggers a live re-merge on the next frame.
    app.world_mut()
        .resource_mut::<EnabledMods>()
        .0
        .insert("demo".to_string());
    app.update();
    assert!(
        app.world()
            .resource::<GameScenarios>()
            .contains_key("demo_mod_arena"),
        "enabling demo must re-merge live and register its scenario"
    );
    assert_eq!(
        app.world()
            .resource::<GameSections>()
            .get_section("reinforced_hull_section")
            .unwrap()
            .base
            .health,
        400.0,
        "the re-merge applied the demo mod's section override"
    );
}

/// Regression guard for the in-game load path (task 20260714-163342). The catalog is
/// a `GameAssets` field, so bevy_asset_loader loads it UNTYPED, resolving the loader
/// by the file's FULL extension only (everything after the first dot). A bare
/// `catalog.ron` would resolve to `ron` (no loader) and fail in-game; `mods.catalog.ron`
/// yields `catalog.ron`, which `CatalogLoader` registers. Loading it untyped here (the
/// game's path) must resolve and reach `Loaded`, pulling in every installed bundle.
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

/// Lower-level proof of the overlay itself (task 20260714-134127): load the base and
/// demo bundles directly, flatten to `Content`, and run the pure `merge_bundles` with
/// the mod after the base. Complements the system-level tests above by pinning the
/// merge core independent of the catalog/EnabledMods plumbing.
#[test]
fn merge_bundles_overlays_demo_over_base() {
    let mut app = headless_app();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let base: Handle<BundleAsset> = asset_server.load("base/base.bundle.ron");
    let demo: Handle<BundleAsset> = asset_server.load("mods/demo/demo.bundle.ron");
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        base.id().untyped(),
        "the base bundle",
    );
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        demo.id().untyped(),
        "the demo bundle",
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
    let demo_items = flatten(&demo);

    let outcome = nova_assets::merge_bundles([base_items.iter(), demo_items.iter()]);
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
        outcome.scenarios.contains_key("demo_mod_arena"),
        "the mod's new scenario is added"
    );
    assert!(
        outcome.scenarios.contains_key("demo"),
        "a base scenario remains after overlay"
    );
}
