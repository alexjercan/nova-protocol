//! End-to-end proof of the LOCAL mod cache + `mods://` source (task
//! 20260715-142906): the REAL gauntlet mod (webmods/) is installed into a temp
//! cache root with `install_local`, the `mods://` source is registered through
//! the PRODUCTION registration (`mod_cache::register_mods_source` - the same
//! call `AppBuilder::new` makes, pointed at the temp root via the
//! `NOVA_MOD_CACHE_ROOT` override both it and the cache helpers read), the
//! production startup system reads the index and kicks the bundle load, and
//! the production merge wiring (register_bundles gated on
//! EnabledMods-or-DownloadedMods changes, plus `mark_downloaded_bundles_loaded`)
//! puts `gauntlet_run` into `GameScenarios` when the mod is enabled - and takes
//! it back out on uninstall.
//!
//! The env override is PROCESS-GLOBAL, so every test here serializes on one
//! lock and owns a fresh temp root while it holds it (separate test binaries
//! are separate processes and cannot interfere).

use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
    time::{Duration, Instant},
};

use bevy::{
    asset::{AssetPlugin, RecursiveDependencyLoadState, UntypedAssetId},
    ecs::system::RunSystemOnce,
    prelude::*,
};
use nova_assets::{
    mod_cache::{self, InstalledModRecord},
    prelude::*,
};
use nova_modding::prelude::{BundleAsset, InstalledCatalog, NovaModdingPlugin};
use nova_scenario::prelude::GameScenarios;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Serializes the tests in this binary and points `NOVA_MOD_CACHE_ROOT` at a
/// fresh temp dir for the guard's lifetime (the cache helpers AND the source
/// registration read the var at call time, so everything a test does while
/// holding this sees the same isolated root).
struct CacheRootGuard {
    _lock: MutexGuard<'static, ()>,
    root: tempfile::TempDir,
}

fn cache_root_guard() -> CacheRootGuard {
    // A panicked test poisons the lock; the lock only serializes, so continue.
    let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = tempfile::tempdir().expect("temp cache root");
    std::env::set_var("NOVA_MOD_CACHE_ROOT", root.path());
    CacheRootGuard { _lock: lock, root }
}

/// The real gauntlet mod's files, read from the repo `webmods/` source - the
/// same bytes the portal would serve (tests run with the crate root as cwd).
fn gauntlet_files() -> Vec<(String, Vec<u8>)> {
    let dir = Path::new("../../webmods/gauntlet");
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).expect("webmods/gauntlet exists at the repo root") {
        let path = entry.expect("readable entry").path();
        assert!(path.is_file(), "the gauntlet bundle dir is flat");
        files.push((
            path.file_name().unwrap().to_string_lossy().to_string(),
            std::fs::read(&path).expect("readable mod file"),
        ));
    }
    assert!(
        files.iter().any(|(name, _)| name == "gauntlet.bundle.ron"),
        "the gauntlet entry-point manifest is among the files"
    );
    files
}

/// A headless app carrying the PRODUCTION `mods://` registration (registered
/// before AssetPlugin, exactly as `AppBuilder::new` orders it) next to the
/// workspace `assets/` as the default source, plus the modding loaders and the
/// resources `GameAssetsPlugin` always inits for the systems under test.
fn app_with_mods_source() -> App {
    let mut app = App::new();
    mod_cache::register_mods_source(&mut app);
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            // Tests run with the crate root as cwd; assets live at the workspace root.
            file_path: "../../assets".to_string(),
            ..default()
        },
    ));
    app.add_plugins(NovaModdingPlugin);
    // GameAssetsPlugin inits all three; the run conditions and systems under
    // test read them (a condition's Res param must exist even while another
    // condition gates the system off).
    app.init_resource::<EnabledMods>();
    app.init_resource::<DownloadedMods>();
    app.init_resource::<ModCatalog>();
    app
}

/// Pump updates until `handle`'s recursive dependency load state is `Loaded`,
/// panicking on failure or timeout (the demo_scenario rig idiom).
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

/// A `GameAssets` with default raw handles (never resolved by the systems under
/// test) and the given loaded catalog handle (the demo_scenario rig idiom).
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

/// The full install -> enable -> uninstall arc through the production pieces:
/// `install_local` seeds the cache, `load_downloaded_mods` (the native startup
/// system) reads the index and loads the bundle THROUGH the `mods://` source,
/// and the production re-merge wiring registers `gauntlet_run` only while the
/// mod is both installed and enabled. Every step degrades this test: delete the
/// source registration and the bundle load fails; delete the downloaded-merge
/// arm of `register_bundles` and the enable step never registers the scenario;
/// delete the DownloadedMods change re-run and the uninstall step stays merged.
#[test]
fn installed_gauntlet_merges_when_enabled_and_unmerges_on_uninstall() {
    let guard = cache_root_guard();
    let files = gauntlet_files();
    mod_cache::install_local("gauntlet", "1.0.0", "gauntlet.bundle.ron", &files)
        .expect("install into the temp cache root");
    assert_eq!(
        mod_cache::read_index(),
        Some(vec![InstalledModRecord {
            id: "gauntlet".to_string(),
            version: "1.0.0".to_string(),
            bundle: "gauntlet.bundle.ron".to_string(),
        }]),
        "the public (env-rooted) index round-trips the installed record"
    );

    let mut app = app_with_mods_source();
    // The production re-merge wiring (GameAssetsPlugin's Update systems, minus
    // the loading-state gate this asset-only rig has no states for): the
    // loaded-event marker plus the merge gated on either half of the installed
    // set changing.
    app.add_systems(Update, nova_assets::mark_downloaded_bundles_loaded);
    app.add_systems(
        Update,
        nova_assets::register_bundles_for_test
            .run_if(resource_exists::<GameAssets>)
            // The exact production condition (shared with the plugin wiring).
            .run_if(nova_assets::installed_set_changed),
    );

    // The production startup system: index -> DownloadedMods + mods:// loads.
    app.world_mut()
        .run_system_once(nova_assets::load_downloaded_mods)
        .expect("load downloaded mods");
    let bundle_id = {
        let downloaded = app.world().resource::<DownloadedMods>();
        assert_eq!(downloaded.0.len(), 1, "one installed record, one entry");
        assert_eq!(downloaded.0[0].record.id, "gauntlet");
        downloaded.0[0].bundle.id().untyped()
    };

    let asset_server = app.world().resource::<AssetServer>().clone();
    let catalog: Handle<InstalledCatalog> = asset_server.load("mods.catalog.ron");
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        catalog.id().untyped(),
        "the mods catalog",
    );
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        bundle_id,
        "the downloaded gauntlet bundle (via mods://)",
    );

    app.world_mut()
        .insert_resource(game_assets_with_catalog(catalog));
    app.world_mut()
        .insert_resource(EnabledMods(["base".to_string()].into_iter().collect()));

    // Installed but DISABLED (the install default): the bundle is loaded, the
    // merge ran (base content present), and the gauntlet scenario stays out.
    app.update();
    {
        let scenarios = app.world().resource::<GameScenarios>();
        assert!(
            scenarios.contains_key("demo"),
            "the base merge ran (its scenario registered)"
        );
        assert!(
            !scenarios.contains_key("gauntlet_run"),
            "a downloaded mod installs DISABLED - loaded, never merged"
        );
    }

    // Enable it -> the EnabledMods change re-merges live, now including the
    // downloaded bundle AFTER the shipped ones.
    app.world_mut()
        .resource_mut::<EnabledMods>()
        .0
        .insert("gauntlet".to_string());
    app.update();
    assert!(
        app.world()
            .resource::<GameScenarios>()
            .contains_key("gauntlet_run"),
        "enabling the downloaded mod must merge its scenario in"
    );

    // Uninstall: remove the cached files + the index record, then drop it from
    // the runtime set - the DownloadedMods change alone (EnabledMods untouched)
    // must re-merge it away.
    let paths: Vec<String> = files.iter().map(|(name, _)| name.clone()).collect();
    mod_cache::remove_mod_files("gauntlet", &paths).expect("remove cached files");
    mod_cache::write_index(&[]);
    assert!(
        !guard.root.path().join("mods").join("gauntlet").exists(),
        "the cache dir is gone after uninstall"
    );
    assert_eq!(
        mod_cache::read_index(),
        Some(vec![]),
        "the index no longer lists the mod"
    );
    app.world_mut().resource_mut::<DownloadedMods>().0.clear();
    app.update();
    let scenarios = app.world().resource::<GameScenarios>();
    assert!(
        !scenarios.contains_key("gauntlet_run"),
        "uninstalling must re-merge the downloaded scenario away"
    );
    assert!(
        scenarios.contains_key("demo"),
        "shipped content is untouched by the uninstall"
    );
}

/// `build_mod_catalog` appends the downloaded mod to the player-facing rows:
/// decl-only (name = id) while its bundle is still in flight, upgraded to the
/// bundle's authored meta once loaded - the strings prove the meta is read from
/// the CACHED bundle through `mods://`, not from the record.
#[test]
fn mod_catalog_lists_the_downloaded_mod_with_its_bundle_meta() {
    let _guard = cache_root_guard();
    mod_cache::install_local(
        "gauntlet",
        "1.0.0",
        "gauntlet.bundle.ron",
        &gauntlet_files(),
    )
    .expect("install into the temp cache root");

    let mut app = app_with_mods_source();
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

    // Kick the downloaded load and build the rows WITHOUT pumping a frame in
    // between: asset insertion happens during update, so the bundle is
    // deterministically not loaded yet - the row must fall back to the id.
    app.world_mut()
        .run_system_once(nova_assets::load_downloaded_mods)
        .expect("load downloaded mods");
    app.world_mut()
        .run_system_once(nova_assets::build_mod_catalog)
        .expect("build mod catalog");
    {
        let mods = &app.world().resource::<ModCatalog>().0;
        assert_eq!(
            mods.len(),
            3,
            "base + demo (shipped) + gauntlet (downloaded)"
        );
        assert_eq!(
            mods[2].id, "gauntlet",
            "downloaded rows append after shipped"
        );
        assert!(!mods[2].base, "a downloaded mod is never the base entry");
        assert_eq!(
            mods[2].meta.name, "gauntlet",
            "an in-flight bundle degrades to the decl-only row (name = id)"
        );
    }

    // Once the bundle (and its content) is loaded, a rebuild composes the row
    // with the meta AUTHORED IN THE CACHED BUNDLE.
    let bundle_id = app.world().resource::<DownloadedMods>().0[0]
        .bundle
        .id()
        .untyped();
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        bundle_id,
        "the downloaded gauntlet bundle (via mods://)",
    );
    app.world_mut()
        .run_system_once(nova_assets::build_mod_catalog)
        .expect("rebuild mod catalog");
    let mods = &app.world().resource::<ModCatalog>().0;
    assert_eq!(mods.len(), 3);
    assert_eq!(mods[2].id, "gauntlet");
    assert_eq!(
        mods[2].meta.name, "Gauntlet Run",
        "the display name comes from the cached bundle's meta"
    );
    assert_eq!(mods[2].meta.version, "1.0.0");
    assert_eq!(mods[2].meta.author, "Nova Protocol");
    assert_eq!(
        mods[2].meta.description, "A beacon slalom course: thread the gates from start to finish.",
        "the description comes from the cached bundle's meta"
    );
}

/// A mod ENABLED while its bundle is still in flight merges as soon as the
/// load lands, with NO further resource mutation after the kick - the startup
/// path for a downloaded mod restored as enabled from prefs. The merge first
/// runs with the bundle enabled-but-unloaded and must SKIP it (no panic, no
/// wedge), and the scenario must arrive once the load completes. Sabotage
/// record: deleting the downloaded-merge arm of `register_bundles` OR the mark
/// system's flagging fails the final assert (both verified; the mark pin here
/// relies on `installed_set_changed` consuming both change ticks together -
/// the earlier `or_else` chain left a primed tick that re-fired on load
/// timing). The deterministic, timing-free pin of the mark mechanism is
/// `loaded_event_flags_downloaded_mods_changed` below.
#[test]
fn enabled_mod_merges_when_its_bundle_load_lands() {
    let _guard = cache_root_guard();
    mod_cache::install_local(
        "gauntlet",
        "1.0.0",
        "gauntlet.bundle.ron",
        &gauntlet_files(),
    )
    .expect("install into the temp cache root");

    let mut app = app_with_mods_source();
    app.add_systems(Update, nova_assets::mark_downloaded_bundles_loaded);
    app.add_systems(
        Update,
        nova_assets::register_bundles_for_test
            .run_if(resource_exists::<GameAssets>)
            // The exact production condition (shared with the plugin wiring).
            .run_if(nova_assets::installed_set_changed),
    );

    // The shipped catalog must be loaded before GameAssets points at it; the
    // gauntlet bundle is deliberately NOT waited for.
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
    app.world_mut().insert_resource(EnabledMods(
        ["base".to_string(), "gauntlet".to_string()]
            .into_iter()
            .collect(),
    ));

    // Kick the mods:// load and merge WITHOUT pumping a frame in between
    // (asset insertion happens during update, so the bundle is
    // deterministically still in flight): the merge must skip the
    // enabled-but-unloaded mod, never block or panic on it.
    app.world_mut()
        .run_system_once(nova_assets::load_downloaded_mods)
        .expect("load downloaded mods");
    let bundle_id = app.world().resource::<DownloadedMods>().0[0]
        .bundle
        .id()
        .untyped();
    app.world_mut()
        .run_system_once(nova_assets::register_bundles_for_test)
        .expect("register bundles");
    assert!(
        !app.world()
            .resource::<GameScenarios>()
            .contains_key("gauntlet_run"),
        "the merge ran before the bundle loaded - the scenario cannot be in yet"
    );

    // No resource is touched from here on: the load completing must re-merge
    // by itself. Two extra frames bound the event -> mark -> re-merge relay
    // (mark and the merge are unordered within one Update).
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        bundle_id,
        "the downloaded gauntlet bundle (via mods://)",
    );
    app.update();
    app.update();
    assert!(
        app.world()
            .resource::<GameScenarios>()
            .contains_key("gauntlet_run"),
        "the finished load must merge the enabled mod in with no manual re-trigger"
    );
}

/// `mark_downloaded_bundles_loaded` at its OWN boundary, deterministically (no
/// real asset IO - the events are written by hand): a LoadedWithDependencies
/// event for a DOWNLOADED bundle flags `DownloadedMods` changed (what re-runs
/// the change-gated merge/catalog systems); an unrelated bundle's event must
/// not. Delete the flagging and the matched frame stops observing a change.
#[test]
fn loaded_event_flags_downloaded_mods_changed() {
    #[derive(Resource, Default)]
    struct ChangedFrames(u32);

    let mut app = App::new();
    app.add_message::<AssetEvent<BundleAsset>>();
    app.init_resource::<DownloadedMods>();
    app.init_resource::<ChangedFrames>();
    app.add_systems(
        Update,
        (
            nova_assets::mark_downloaded_bundles_loaded,
            // The observer half of the pin: counts the frames on which the
            // resource reads as changed, the exact signal the production
            // run conditions consume.
            |downloaded: Res<DownloadedMods>, mut frames: ResMut<ChangedFrames>| {
                if downloaded.is_changed() {
                    frames.0 += 1;
                }
            },
        )
            .chain(),
    );

    // A stable uuid handle stands in for the mods:// loaded bundle; distinct
    // from both `Handle::default()` (DEFAULT_UUID) and `AssetId::invalid()`.
    let bundle: Handle<BundleAsset> =
        bevy::asset::uuid_handle!("7b0e2ae2-6f4f-4c11-a327-40a336a4a3bc");
    app.world_mut()
        .resource_mut::<DownloadedMods>()
        .0
        .push(DownloadedMod {
            record: InstalledModRecord {
                id: "gauntlet".to_string(),
                version: "1.0.0".to_string(),
                bundle: "gauntlet.bundle.ron".to_string(),
            },
            bundle: bundle.clone(),
        });

    // Drain the setup mutation so later frames isolate the event signal.
    app.update();
    let after_setup = app.world().resource::<ChangedFrames>().0;

    // An unrelated bundle's load event must NOT flag the downloaded set.
    app.world_mut()
        .write_message(AssetEvent::<BundleAsset>::LoadedWithDependencies {
            id: AssetId::invalid(),
        });
    app.update();
    assert_eq!(
        app.world().resource::<ChangedFrames>().0,
        after_setup,
        "an unrelated bundle load must not flag DownloadedMods"
    );

    // The downloaded bundle's own load event must flag it.
    app.world_mut()
        .write_message(AssetEvent::LoadedWithDependencies { id: bundle.id() });
    app.update();
    assert_eq!(
        app.world().resource::<ChangedFrames>().0,
        after_setup + 1,
        "the downloaded bundle's load must flag DownloadedMods changed"
    );
}

/// Review 142906 R1.1(a): the on-disk index is DOWNLOADED input - a record
/// whose id or bundle path could escape the cache (a `..` component, a nested
/// id) is skipped with a warning before any `mods://` path is built from it.
/// The poisoned index is written BY HAND (the real attack shape; the public
/// write path validates and would refuse to produce it).
#[test]
fn unsafe_index_records_are_skipped_at_load() {
    let guard = cache_root_guard();
    std::fs::write(
        guard.root.path().join("installed.mods.ron"),
        r#"[
            (id: "good", version: "1.0.0", bundle: "good.bundle.ron"),
            (id: "../escape", version: "1.0.0", bundle: "x.bundle.ron"),
            (id: "sneaky", version: "1.0.0", bundle: "../../x.bundle.ron"),
            (id: "nested/id", version: "1.0.0", bundle: "x.bundle.ron"),
        ]"#,
    )
    .expect("write the poisoned index");

    let mut app = app_with_mods_source();
    app.world_mut()
        .run_system_once(nova_assets::load_downloaded_mods)
        .expect("load downloaded mods");

    let downloaded = app.world().resource::<DownloadedMods>();
    assert_eq!(
        downloaded.0.len(),
        1,
        "only the safe record survives the index read"
    );
    assert_eq!(downloaded.0[0].record.id, "good");
}

/// Review 142906 R1.1(b): a malicious bundle MANIFEST can request an escaping
/// content path without touching the index (`AssetPath::resolve` preserves an
/// underflowing `..`), so record validation alone cannot stop it - the
/// SANDBOXED native source must. A decoy content file sits OUTSIDE the mods
/// root (at the data root, exactly where `../../` from the mod dir lands): the
/// bundle load must FAIL (the sandbox answers not-found), and the decoy's
/// scenario must never register even with the mod enabled.
#[test]
fn escaping_bundle_manifest_cannot_read_outside_the_mods_root() {
    let guard = cache_root_guard();
    std::fs::write(
        guard.root.path().join("evil.content.ron"),
        r#"[
            Scenario((
                id: "evil_escape",
                name: "Evil",
                description: "decoy outside the mods root",
                cubemap: "textures/cubemap.png",
                events: [],
            )),
        ]"#,
    )
    .expect("write the decoy outside the mods root");
    // The manifest itself is a VALID install (safe file paths); the escape is
    // inside the manifest's content list, which install_local never parses.
    mod_cache::install_local(
        "sneaky",
        "1.0.0",
        "sneaky.bundle.ron",
        &[(
            "sneaky.bundle.ron".to_string(),
            b"(content: [\"../../evil.content.ron\"])".to_vec(),
        )],
    )
    .expect("install the mod with the escaping manifest");

    let mut app = app_with_mods_source();
    app.world_mut()
        .run_system_once(nova_assets::load_downloaded_mods)
        .expect("load downloaded mods");
    let bundle_id = app.world().resource::<DownloadedMods>().0[0]
        .bundle
        .id()
        .untyped();

    // The load must come to a FAILED end state, not a loaded one: the sandbox
    // rejects the escaping request even though the decoy file exists.
    let asset_server = app.world().resource::<AssetServer>().clone();
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        app.update();
        match asset_server.get_recursive_dependency_load_state(bundle_id) {
            Some(RecursiveDependencyLoadState::Failed(_)) => break,
            Some(RecursiveDependencyLoadState::Loaded) => {
                panic!("the escaping content path must NOT load (sandbox bypassed)")
            }
            _ => {}
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for the sandboxed load to fail"
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    // And the production merge path never registers the decoy scenario, even
    // with the mod's id enabled: the failed bundle is skipped.
    let catalog: Handle<InstalledCatalog> = asset_server.load("mods.catalog.ron");
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        catalog.id().untyped(),
        "the mods catalog",
    );
    app.world_mut()
        .insert_resource(game_assets_with_catalog(catalog));
    app.world_mut().insert_resource(EnabledMods(
        ["base".to_string(), "sneaky".to_string()]
            .into_iter()
            .collect(),
    ));
    app.world_mut()
        .run_system_once(nova_assets::register_bundles_for_test)
        .expect("register bundles");
    assert!(
        !app.world()
            .resource::<GameScenarios>()
            .contains_key("evil_escape"),
        "the decoy scenario outside the cache root must never register"
    );
}

/// Review 142906 R1.2: a downloaded record whose id matches a SHIPPED catalog
/// entry is skipped with a warning (the portal generator's no-shadowing rule,
/// re-enforced at the consumers because the index is downloaded input) - one
/// toggle must never drive two bundles or two rows. The downloaded copy here
/// is the REAL gauntlet content installed under the shipped id "demo", so the
/// assertions can tell the two bundles apart by their scenarios and meta.
#[test]
fn downloaded_id_shadowing_a_shipped_mod_is_skipped() {
    let _guard = cache_root_guard();
    mod_cache::install_local("demo", "9.9.9", "gauntlet.bundle.ron", &gauntlet_files())
        .expect("install a downloaded mod under the shipped 'demo' id");

    let mut app = app_with_mods_source();
    app.world_mut()
        .run_system_once(nova_assets::load_downloaded_mods)
        .expect("load downloaded mods");
    let bundle_id = app.world().resource::<DownloadedMods>().0[0]
        .bundle
        .id()
        .untyped();

    let asset_server = app.world().resource::<AssetServer>().clone();
    let catalog: Handle<InstalledCatalog> = asset_server.load("mods.catalog.ron");
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        catalog.id().untyped(),
        "the mods catalog",
    );
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        bundle_id,
        "the shadowing downloaded bundle",
    );
    app.world_mut()
        .insert_resource(game_assets_with_catalog(catalog));
    app.world_mut().insert_resource(EnabledMods(
        ["base".to_string(), "demo".to_string()]
            .into_iter()
            .collect(),
    ));

    // The rows: no third entry, and "demo" keeps its SHIPPED meta.
    app.world_mut()
        .run_system_once(nova_assets::build_mod_catalog)
        .expect("build mod catalog");
    let mods = &app.world().resource::<ModCatalog>().0;
    assert_eq!(mods.len(), 2, "the shadowing downloaded row is hidden");
    assert_eq!(mods[1].id, "demo");
    assert_eq!(
        mods[1].meta.name, "Demo Mod",
        "the shipped meta wins - the downloaded copy's 'Gauntlet Run' meta must not surface"
    );

    // The merge: the shipped demo's content registers, the downloaded copy's
    // does not - one enabled id, one bundle.
    app.world_mut()
        .run_system_once(nova_assets::register_bundles_for_test)
        .expect("register bundles");
    let scenarios = app.world().resource::<GameScenarios>();
    assert!(
        scenarios.contains_key("demo_mod_arena"),
        "the SHIPPED demo bundle merges as usual"
    );
    assert!(
        !scenarios.contains_key("gauntlet_run"),
        "the shadowing DOWNLOADED bundle must be skipped by the merge"
    );
}
