//! End-to-end proof of the PORTAL CLIENT (task 20260715-163508): the game
//! fetches a portal catalog and installs/uninstalls mods over the wire,
//! committing through the 142906 local cache and registering into the live
//! installed set.
//!
//! Two rigs, both driving the PRODUCTION wiring (`PortalPlugin` - the real
//! observers, channel and poll system; `register_mods_source`; the real merge
//! condition):
//!
//! - THE REAL WIRE: `scripts/gen-portal.py` (the production generator, run as a
//!   python3 subprocess) builds a portal tree from a
//!   synthetic fixture source (no real mod named - task 20260716-155839),
//!   `tiny_http` serves it on localhost, and the REAL `EhttpTransport`
//!   fetches it - catalog to `RemoteCatalog::Ready`, install to cached
//!   files + a `DownloadedMods` record, enable to the fixture's scenario
//!   in `GameScenarios`, uninstall all the way back out (including the
//!   `EnabledMods` strip that resolves 142906's R1.7).
//! - FAILURE INJECTION: a mock `PortalTransport` serves corrupted/truncated
//!   bodies, unknown schema versions and mid-install transport errors; every
//!   failure asserts the ABSENCE evidence through the cache API (no files, no
//!   index entry - the staged-commit discipline).
//!
//! The `NOVA_MOD_CACHE_ROOT`/`NOVA_PORTAL_URL` env overrides are
//! PROCESS-GLOBAL, so every test serializes on one lock and owns a fresh temp
//! root while holding it (separate test binaries are separate processes).

use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use bevy::{
    asset::{AssetPlugin, RecursiveDependencyLoadState, UntypedAssetId},
    prelude::*,
};
use nova_assets::{
    mod_cache,
    portal::{
        FetchPortalCatalog, FetchResult, InstallJobs, InstallPortalMod, InstallStatus,
        PortalClient, PortalConfig, PortalFetchTimeout, PortalPlugin, PortalTransport,
        RemoteCatalog, RemoteCatalogState, UninstallPortalMod,
    },
    prelude::*,
};
use nova_mod_format::{ModMeta, PortalCatalog, PortalEntry, PortalFile, PORTAL_SCHEMA_VERSION};
use nova_modding::prelude::{InstalledCatalog, NovaModdingPlugin};
use nova_scenario::prelude::GameScenarios;
use sha2::{Digest, Sha256};

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Serializes the tests in this binary, points `NOVA_MOD_CACHE_ROOT` at a
/// fresh temp dir and CLEARS `NOVA_PORTAL_URL` (the wire test sets it; mock
/// tests must not inherit a previous test's server).
struct CacheRootGuard {
    _lock: MutexGuard<'static, ()>,
    root: tempfile::TempDir,
}

fn cache_root_guard() -> CacheRootGuard {
    // A panicked test poisons the lock; the lock only serializes, so continue.
    let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = tempfile::tempdir().expect("temp cache root");
    std::env::set_var("NOVA_MOD_CACHE_ROOT", root.path());
    std::env::remove_var("NOVA_PORTAL_URL");
    CacheRootGuard { _lock: lock, root }
}

/// A headless app with the PRODUCTION `mods://` registration, the modding
/// loaders, the portal plugin (real transport until a test swaps it) and the
/// production re-merge wiring (the mod_cache_install rig plus `PortalPlugin`).
fn portal_app() -> App {
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
    app.init_resource::<EnabledMods>();
    app.init_resource::<DownloadedMods>();
    app.init_resource::<ModCatalog>();
    app.add_plugins(PortalPlugin);
    app.add_systems(Update, nova_assets::mark_downloaded_bundles_loaded);
    app.add_systems(
        Update,
        nova_assets::register_bundles_for_test
            .run_if(resource_exists::<GameAssets>)
            // The exact production condition (shared with the plugin wiring).
            .run_if(nova_assets::installed_set_changed),
    );
    app
}

/// Pump updates until `f` yields, panicking after a deadline.
fn pump_until<T>(app: &mut App, what: &str, mut f: impl FnMut(&mut App) -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        app.update();
        if let Some(value) = f(app) {
            return value;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Pump updates until `handle`'s recursive dependency load state is `Loaded`,
/// panicking on failure or timeout (the example_scenario rig idiom).
fn wait_recursive_loaded(
    app: &mut App,
    asset_server: &AssetServer,
    handle: UntypedAssetId,
    what: &str,
) {
    let server = asset_server.clone();
    let what_owned = what.to_string();
    pump_until(app, what, move |_| {
        match server.get_recursive_dependency_load_state(handle) {
            Some(RecursiveDependencyLoadState::Loaded) => Some(()),
            Some(RecursiveDependencyLoadState::Failed(err)) => {
                panic!("{what_owned} failed to load: {err}")
            }
            _ => None,
        }
    });
}

/// A `GameAssets` with default raw handles (never resolved by the systems
/// under test) and the given loaded catalog handle (the example_scenario rig
/// idiom).
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

/// Load the SHIPPED catalog and stand up `GameAssets` + a base-enabled
/// `EnabledMods` - the state the portal observers' guards read.
fn ready_shipped_catalog(app: &mut App) {
    let asset_server = app.world().resource::<AssetServer>().clone();
    let catalog: Handle<InstalledCatalog> = asset_server.load("mods.catalog.ron");
    wait_recursive_loaded(
        app,
        &asset_server,
        catalog.id().untyped(),
        "the mods catalog",
    );
    app.world_mut()
        .insert_resource(game_assets_with_catalog(catalog));
    app.world_mut()
        .insert_resource(EnabledMods(["base".to_string()].into_iter().collect()));
    app.update();
}

/// Pump until the install of `id` either lands in `DownloadedMods` (success)
/// or reports `Failed` (panic with the reason, so a broken install names
/// itself instead of timing out).
fn pump_install_success(app: &mut App, id: &str) {
    let id = id.to_string();
    pump_until(app, "the install to finish", move |app| {
        if let Some(InstallStatus::Failed(reason)) =
            app.world().resource::<InstallJobs>().0.get(&id)
        {
            panic!("install of '{id}' failed: {reason}");
        }
        app.world()
            .resource::<DownloadedMods>()
            .0
            .iter()
            .any(|m| m.record.id == id)
            .then_some(())
    });
}

/// Pump until the install of `id` FAILS, returning the reason. An install
/// that wrongly SUCCEEDS clears its job entry and never turns `Failed`, so
/// this times out (and the caller's absence asserts would fail too).
fn pump_install_failure(app: &mut App, id: &str) -> String {
    let id = id.to_string();
    pump_until(app, "the install to fail", move |app| {
        match app.world().resource::<InstallJobs>().0.get(&id) {
            Some(InstallStatus::Failed(reason)) => Some(reason.clone()),
            _ => None,
        }
    })
}

/// The ABSENCE evidence every failure test pins, through the cache API: no
/// index record, no cached file bytes, no cache directory, no runtime record.
/// This is the staged-commit discipline made observable - if ANY stage leaked
/// a write before full verification, one of these fires.
fn assert_nothing_committed(app: &App, guard: &CacheRootGuard, id: &str, paths: &[String]) {
    assert!(
        !mod_cache::read_index()
            .unwrap_or_default()
            .iter()
            .any(|r| r.id == id),
        "a failed install must not leave an index record for '{id}'"
    );
    for path in paths {
        assert_eq!(
            mod_cache::read_mod_file(id, path),
            None,
            "a failed install must not leave cached bytes for '{id}/{path}'"
        );
    }
    assert!(
        !guard.root.path().join("mods").join(id).exists(),
        "a failed install must not leave a cache directory for '{id}'"
    );
    assert!(
        !app.world()
            .resource::<DownloadedMods>()
            .0
            .iter()
            .any(|m| m.record.id == id),
        "a failed install must not reach DownloadedMods"
    );
}

// ---------------------------------------------------------------------------
// The real wire: gen-portal.py tree + tiny_http + the production
// EhttpTransport.
// ---------------------------------------------------------------------------

/// The synthetic portal mod the wire test installs (task 20260716-155839:
/// core tests must not depend on any REAL mod, so mods can be renamed or
/// removed without touching CI). Same shape as a webmods/ source: one dir
/// per mod, flat files, and a real Scenario so the enable step can assert
/// registration through the actual merge machinery.
const FIXTURE_ID: &str = "fixture-slalom";
const FIXTURE_SCENARIO_ID: &str = "fixture_slalom_run";

/// Write the fixture mod's source tree under `root/<FIXTURE_ID>/` and return
/// its files - the same bytes the portal serves, for byte-identity
/// assertions after install.
fn write_fixture_mod(root: &Path) -> Vec<(String, Vec<u8>)> {
    let bundle = r#"(
    content: ["fixture-slalom.content.ron"],
    meta: (
        name: "Fixture Slalom",
        description: "Synthetic install fixture.",
        author: "tests",
        version: "1.0.0",
    ),
)
"#;
    let content = r#"[
    Scenario((
        id: "fixture_slalom_run",
        name: "Fixture Slalom Run",
        description: "A minimal scenario for install-pipeline assertions.",
        cubemap: "dep://base/textures/cubemap.png",
    )),
]
"#;
    let files = vec![
        (
            "fixture-slalom.bundle.ron".to_string(),
            bundle.as_bytes().to_vec(),
        ),
        (
            "fixture-slalom.content.ron".to_string(),
            content.as_bytes().to_vec(),
        ),
    ];
    let dir = root.join(FIXTURE_ID);
    std::fs::create_dir_all(&dir).expect("create the fixture mod dir");
    for (name, bytes) in &files {
        std::fs::write(dir.join(name), bytes).expect("write a fixture mod file");
    }
    files
}

/// Serve `root` statically on an OS-assigned localhost port; returns the base
/// URL. The thread lives for the whole test process (requests just stop
/// coming), which is fine for a test binary.
fn serve_portal_tree(root: std::path::PathBuf) -> String {
    let server = tiny_http::Server::http("127.0.0.1:0").expect("bind a localhost test server");
    let addr = server
        .server_addr()
        .to_ip()
        .expect("the test server has an IP address");
    std::thread::spawn(move || {
        for request in server.incoming_requests() {
            let rel = request.url().trim_start_matches('/').to_string();
            match std::fs::read(root.join(&rel)) {
                Ok(bytes) => {
                    let _ = request.respond(tiny_http::Response::from_data(bytes));
                }
                Err(_) => {
                    let _ = request.respond(tiny_http::Response::empty(404));
                }
            }
        }
    });
    format!("http://{addr}")
}

/// The whole arc over a REAL socket with the REAL transport: generate the
/// portal from a synthetic source tree (the production generator, the same
/// invocation shape the deploy job makes), serve it, fetch the catalog
/// (`Ready` lists the fixture), install (verified files land in the cache,
/// the record joins `DownloadedMods`, the job entry clears, and the mod is
/// DISABLED), enable (the existing merge machinery registers the fixture's
/// scenario), then uninstall (files + index + record + the `EnabledMods`
/// entry all gone, the scenario unregisters). Deleting any stage of the
/// portal client fails this test: no fetch -> no Ready; no commit -> no
/// cached files; no DownloadedMods push -> no merge; no EnabledMods strip ->
/// the last assert. (Whether the real webmods/ tree publishes is
/// gen_portal_gate.rs's real-webmods-publishes test, not this one.)
#[test]
fn portal_fetch_install_enable_uninstall_over_the_wire() {
    let guard = cache_root_guard();

    // The production generator over the fixture source, against the REAL
    // shipped catalog (so the shipped-id collision gate stays exercised).
    let source_dir = tempfile::tempdir().expect("temp mod source tree");
    let source_files = write_fixture_mod(source_dir.path());
    let portal_dir = tempfile::tempdir().expect("temp portal tree");
    // Drive the PRODUCTION generator (scripts/gen-portal.py) the same way the
    // deploy job does - a subprocess with --source/--shipped/--out. The crate
    // was retired (task 20260720-230924); its gate coverage moved to
    // tests/gen_portal_gate.rs. Absolute paths so cwd never matters; the shipped
    // catalog stays wired in so the shipped-id collision gate is still exercised.
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root resolves from CARGO_MANIFEST_DIR");
    let status = std::process::Command::new("python3")
        .arg(repo_root.join("scripts/gen-portal.py"))
        .arg("--source")
        .arg(source_dir.path())
        .arg("--shipped")
        .arg(repo_root.join("assets/mods.catalog.ron"))
        .arg("--out")
        .arg(portal_dir.path())
        .status()
        .expect("gen-portal.py runs (python3 on PATH)");
    assert!(
        status.success(),
        "gen-portal.py must publish the fixture source tree"
    );
    let catalog_json = std::fs::read_to_string(portal_dir.path().join("catalog.json"))
        .expect("catalog.json written by gen-portal.py");
    assert!(
        catalog_json.contains(&format!("\"{FIXTURE_ID}\"")),
        "the fixture mod publishes (catalog.json names {FIXTURE_ID}):\n{catalog_json}"
    );

    let base_url = serve_portal_tree(portal_dir.path().to_path_buf());
    // The production override path: PortalPlugin reads this at build.
    std::env::set_var("NOVA_PORTAL_URL", &base_url);

    let mut app = portal_app();
    assert_eq!(
        app.world().resource::<PortalConfig>().base_url,
        base_url,
        "NOVA_PORTAL_URL steers the production config"
    );
    ready_shipped_catalog(&mut app);

    // FETCH: Idle -> (Fetching ->) Ready listing the fixture's meta.
    app.world_mut().trigger(FetchPortalCatalog);
    let entry = pump_until(&mut app, "the portal catalog", |app| {
        match &app.world().resource::<RemoteCatalog>().state {
            RemoteCatalogState::Ready(catalog) => Some(
                catalog
                    .entries
                    .iter()
                    .find(|e| e.id == FIXTURE_ID)
                    .expect("the fetched catalog lists the fixture mod")
                    .clone(),
            ),
            RemoteCatalogState::Error(error) => panic!("catalog fetch failed: {error}"),
            _ => None,
        }
    });
    // The successful fetch also stamps the LAST-GOOD fallback (task 142916)
    // and persists it under the (test-overridden) cache root; the persisted
    // bytes re-pass the schema gate - the exact startup-load path.
    assert!(
        app.world()
            .resource::<RemoteCatalog>()
            .last_good
            .as_ref()
            .is_some_and(|c| c.entries.iter().any(|e| e.id == FIXTURE_ID)),
        "a Ready fetch must stamp last_good"
    );
    let store = guard.root.path().join("portal_catalog.json");
    let stored =
        std::fs::read(&store).expect("the last-good store was written under the cache root");
    assert!(
        serde_json::from_slice::<PortalCatalog>(&stored).is_ok(),
        "the store holds the raw catalog JSON"
    );
    assert_eq!(entry.meta.name, "Fixture Slalom");
    assert_eq!(entry.version, "1.0.0");
    assert_eq!(entry.bundle, "fixture-slalom.bundle.ron");

    // INSTALL: files fetched + verified + committed, record registered live.
    app.world_mut().trigger(InstallPortalMod {
        id: FIXTURE_ID.to_string(),
    });
    pump_install_success(&mut app, FIXTURE_ID);
    assert!(
        app.world().resource::<InstallJobs>().0.is_empty(),
        "a successful install clears its job entry"
    );
    assert_eq!(
        mod_cache::read_index(),
        Some(vec![mod_cache::InstalledModRecord {
            id: FIXTURE_ID.to_string(),
            version: "1.0.0".to_string(),
            bundle: "fixture-slalom.bundle.ron".to_string(),
        }]),
        "the committed index carries the installed record"
    );
    for (path, bytes) in &source_files {
        assert_eq!(
            mod_cache::read_mod_file(FIXTURE_ID, path).as_ref(),
            Some(bytes),
            "the cached '{path}' is byte-identical to the fixture source"
        );
    }

    // Installed but DISABLED: the bundle loads, the merge re-ran (on the
    // DownloadedMods change), and the scenario stays out.
    let asset_server = app.world().resource::<AssetServer>().clone();
    let bundle_id = app.world().resource::<DownloadedMods>().0[0]
        .bundle
        .id()
        .untyped();
    wait_recursive_loaded(
        &mut app,
        &asset_server,
        bundle_id,
        "the installed fixture bundle (via mods://)",
    );
    app.update();
    assert!(
        !app.world()
            .resource::<GameScenarios>()
            .contains_key(FIXTURE_SCENARIO_ID),
        "a fresh install stays disabled - no merge until the player enables it"
    );

    // ENABLE: the normal EnabledMods toggle merges the downloaded bundle in.
    app.world_mut()
        .resource_mut::<EnabledMods>()
        .0
        .insert(FIXTURE_ID.to_string());
    app.update();
    assert!(
        app.world()
            .resource::<GameScenarios>()
            .contains_key(FIXTURE_SCENARIO_ID),
        "enabling the installed mod must register its scenario"
    );

    // UNINSTALL: everything reverses, INCLUDING the enablement (R1.7).
    app.world_mut().trigger(UninstallPortalMod {
        id: FIXTURE_ID.to_string(),
    });
    app.update();
    assert!(
        app.world().resource::<DownloadedMods>().0.is_empty(),
        "uninstall drops the runtime record"
    );
    assert_eq!(
        mod_cache::read_index(),
        Some(vec![]),
        "uninstall removes the index record"
    );
    assert!(
        !guard.root.path().join("mods").join(FIXTURE_ID).exists(),
        "uninstall removes the cached files"
    );
    assert!(
        !app.world()
            .resource::<GameScenarios>()
            .contains_key(FIXTURE_SCENARIO_ID),
        "uninstall re-merges the scenario away"
    );
    assert!(
        !app.world().resource::<EnabledMods>().0.contains(FIXTURE_ID),
        "uninstall strips the id from EnabledMods, so a reinstall starts disabled"
    );
    assert!(
        app.world().resource::<EnabledMods>().0.contains("base"),
        "other enablements are untouched"
    );
}

// ---------------------------------------------------------------------------
// Failure injection through a mock transport.
// ---------------------------------------------------------------------------

/// A canned URL -> response transport; unknown URLs 404. Delivery is
/// synchronous (the callback runs inside `fetch`), which the channel-based
/// client absorbs by design - messages are consumed on the next poll.
struct MockTransport(HashMap<String, FetchResult>);

impl PortalTransport for MockTransport {
    fn fetch(&self, url: &str, on_done: Box<dyn FnOnce(FetchResult) + Send>) {
        on_done(
            self.0
                .get(url)
                .cloned()
                .unwrap_or_else(|| Err(format!("HTTP 404 Not Found ({url})"))),
        );
    }
}

const MOCK_BASE: &str = "http://portal.test/mods";

/// A tiny synthetic mod whose files are REAL loadable RON (a success through
/// the mock commits and loads them via mods://).
fn mock_files() -> Vec<(String, Vec<u8>)> {
    vec![
        (
            "mockmod.bundle.ron".to_string(),
            b"(content: [\"scenarios/mock.content.ron\"], meta: (name: \"Mock Mod\", version: \"1.0.0\"))"
                .to_vec(),
        ),
        ("scenarios/mock.content.ron".to_string(), b"[]".to_vec()),
    ]
}

/// A catalog entry whose sizes/hashes MATCH `files` (what the generator would
/// publish for them).
fn entry_for(id: &str, files: &[(String, Vec<u8>)]) -> PortalEntry {
    PortalEntry {
        id: id.to_string(),
        version: "1.0.0".to_string(),
        bundle: files[0].0.clone(),
        meta: ModMeta {
            name: "Mock Mod".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        },
        files: files
            .iter()
            .map(|(path, bytes)| PortalFile {
                path: path.clone(),
                size: bytes.len() as u64,
                sha256: format!("{:x}", Sha256::digest(bytes)),
            })
            .collect(),
        total_size: files.iter().map(|(_, b)| b.len() as u64).sum(),
    }
}

fn catalog_bytes(entries: Vec<PortalEntry>) -> Vec<u8> {
    serde_json::to_vec(&PortalCatalog {
        schema_version: PORTAL_SCHEMA_VERSION,
        entries,
    })
    .expect("serialize the mock catalog")
}

/// Routes serving `entry` + `served` bodies under [`MOCK_BASE`] (the bodies
/// may deliberately mismatch the entry's hashes - that is the point).
fn mock_routes(
    entry: &PortalEntry,
    served: &[(String, FetchResult)],
) -> HashMap<String, FetchResult> {
    let mut routes = HashMap::new();
    routes.insert(
        format!("{MOCK_BASE}/catalog.json"),
        Ok(catalog_bytes(vec![entry.clone()])),
    );
    for (path, result) in served {
        routes.insert(
            format!("{MOCK_BASE}/{}/{}/{path}", entry.id, entry.version),
            result.clone(),
        );
    }
    routes
}

/// A ready-to-install app over `routes`: mock transport + mock base URL +
/// shipped catalog + a Ready RemoteCatalog.
fn mock_app(routes: HashMap<String, FetchResult>) -> App {
    let mut app = portal_app();
    app.insert_resource(PortalClient(Arc::new(MockTransport(routes))));
    app.insert_resource(PortalConfig {
        base_url: MOCK_BASE.to_string(),
    });
    ready_shipped_catalog(&mut app);
    app.world_mut().trigger(FetchPortalCatalog);
    pump_until(&mut app, "the mock catalog", |app| {
        match &app.world().resource::<RemoteCatalog>().state {
            RemoteCatalogState::Ready(_) => Some(()),
            RemoteCatalogState::Error(error) => panic!("mock catalog fetch failed: {error}"),
            _ => None,
        }
    });
    app
}

fn paths_of(files: &[(String, Vec<u8>)]) -> Vec<String> {
    files.iter().map(|(p, _)| p.clone()).collect()
}

/// A corrupted body (same length, one byte flipped) fails the sha256 check
/// and commits NOTHING. Deleting the sha256 comparison in the poll system's
/// verify step turns the flip invisible and the install succeeds - this test
/// then fails its Failed-status wait and the absence asserts.
#[test]
fn sha_mismatch_fails_the_install_and_commits_nothing() {
    let guard = cache_root_guard();
    let files = mock_files();
    let entry = entry_for("mockmod", &files);

    let mut corrupted = files[1].1.clone();
    corrupted[0] ^= 0xff;
    let served = vec![
        (files[0].0.clone(), Ok(files[0].1.clone())),
        (files[1].0.clone(), Ok(corrupted)),
    ];
    let mut app = mock_app(mock_routes(&entry, &served));

    app.world_mut().trigger(InstallPortalMod {
        id: "mockmod".to_string(),
    });
    let reason = pump_install_failure(&mut app, "mockmod");
    assert!(
        reason.contains("sha256"),
        "the failure names the integrity check: {reason}"
    );
    assert_nothing_committed(&app, &guard, "mockmod", &paths_of(&files));
}

/// A truncated body (catalog size not met) fails the size check and commits
/// NOTHING. Deleting the size comparison lets the truncated body reach the
/// sha stage (a different message) or, with both checks gone, the cache.
#[test]
fn size_mismatch_fails_the_install_and_commits_nothing() {
    let guard = cache_root_guard();
    let files = mock_files();
    let entry = entry_for("mockmod", &files);

    let truncated = files[1].1[..files[1].1.len() - 1].to_vec();
    let served = vec![
        (files[0].0.clone(), Ok(files[0].1.clone())),
        (files[1].0.clone(), Ok(truncated)),
    ];
    let mut app = mock_app(mock_routes(&entry, &served));

    app.world_mut().trigger(InstallPortalMod {
        id: "mockmod".to_string(),
    });
    let reason = pump_install_failure(&mut app, "mockmod");
    assert!(
        reason.contains("size mismatch"),
        "the failure names the size check: {reason}"
    );
    assert_nothing_committed(&app, &guard, "mockmod", &paths_of(&files));
}

/// A transport failure MID-install (file 1 of 2 already fetched and verified)
/// fails the job and commits NOTHING - the staged discipline: verified bytes
/// are held in memory, never written until ALL files verified. Committing
/// files as they arrive (the sabotage) leaves file 0 in the cache and fails
/// the absence asserts.
#[test]
fn mid_install_transport_failure_commits_nothing() {
    let guard = cache_root_guard();
    let files = mock_files();
    let entry = entry_for("mockmod", &files);

    let served = vec![
        (files[0].0.clone(), Ok(files[0].1.clone())),
        (
            files[1].0.clone(),
            Err("connection reset by peer".to_string()),
        ),
    ];
    let mut app = mock_app(mock_routes(&entry, &served));

    app.world_mut().trigger(InstallPortalMod {
        id: "mockmod".to_string(),
    });
    let reason = pump_install_failure(&mut app, "mockmod");
    assert!(
        reason.contains("connection reset"),
        "the failure carries the transport error: {reason}"
    );
    // The FIRST file arrived intact and verified - it must still not be in
    // the cache (this is the assert the per-file-commit sabotage fails).
    assert_nothing_committed(&app, &guard, "mockmod", &paths_of(&files));
}

/// An unknown `schema_version` is rejected AS a version mismatch (never
/// half-parsed into Ready, never a confusing shape error), and no install can
/// start from it. Deleting the schema gate in `decode_catalog` parses this
/// same-shaped body into Ready and fails the Error wait.
#[test]
fn unknown_schema_version_is_rejected_not_misparsed() {
    let _guard = cache_root_guard();
    let files = mock_files();
    let entry = entry_for("mockmod", &files);

    // Same shape as a real catalog - only the version is from the future.
    let mut catalog = serde_json::to_value(&PortalCatalog {
        schema_version: PORTAL_SCHEMA_VERSION,
        entries: vec![entry],
    })
    .unwrap();
    catalog["schema_version"] = serde_json::json!(999);
    let mut routes = HashMap::new();
    routes.insert(
        format!("{MOCK_BASE}/catalog.json"),
        Ok(serde_json::to_vec(&catalog).unwrap()),
    );

    let mut app = portal_app();
    app.insert_resource(PortalClient(Arc::new(MockTransport(routes))));
    app.insert_resource(PortalConfig {
        base_url: MOCK_BASE.to_string(),
    });
    ready_shipped_catalog(&mut app);

    app.world_mut().trigger(FetchPortalCatalog);
    let error = pump_until(&mut app, "the catalog rejection", |app| {
        match &app.world().resource::<RemoteCatalog>().state {
            RemoteCatalogState::Error(error) => Some(error.clone()),
            RemoteCatalogState::Ready(_) => {
                panic!("a schema_version-999 catalog must never become Ready")
            }
            _ => None,
        }
    });
    assert!(
        error.contains("schema_version 999"),
        "the error names the unknown version: {error}"
    );

    // No Ready catalog -> an install trigger fails fast, touching nothing.
    app.world_mut().trigger(InstallPortalMod {
        id: "mockmod".to_string(),
    });
    let reason = pump_install_failure(&mut app, "mockmod");
    assert!(
        reason.contains("not loaded"),
        "an install without a Ready catalog is rejected: {reason}"
    );
}

/// Review 142916 R1.3: an install whose file fetch NEVER calls back (the
/// pathological transport 163508 documented) is failed by the stall timeout
/// instead of wedging in `Fetching` forever, landing on the standard Failed
/// surface (the menu's Retry/Dismiss) with nothing committed. The tiny
/// injected `PortalFetchTimeout` drives the REAL timeout system across
/// frames; deleting `timeout_wedged_fetches` (or its Fetching filter) makes
/// this pump time out with the job still in `Fetching`.
#[test]
fn a_wedged_file_fetch_times_out_into_failed() {
    let guard = cache_root_guard();
    let files = mock_files();
    let entry = entry_for("mockmod", &files);

    /// Serves the catalog; DROPS every file fetch's callback on the floor.
    struct WedgedTransport {
        catalog: Vec<u8>,
    }
    impl PortalTransport for WedgedTransport {
        fn fetch(&self, url: &str, on_done: Box<dyn FnOnce(FetchResult) + Send>) {
            if url.ends_with("/catalog.json") {
                on_done(Ok(self.catalog.clone()));
            }
            // else: the callback is dropped - it will never fire.
        }
    }

    let mut app = portal_app();
    app.insert_resource(PortalClient(Arc::new(WedgedTransport {
        catalog: catalog_bytes(vec![entry]),
    })));
    app.insert_resource(PortalConfig {
        base_url: MOCK_BASE.to_string(),
    });
    ready_shipped_catalog(&mut app);
    app.world_mut().trigger(FetchPortalCatalog);
    pump_until(&mut app, "the mock catalog", |app| {
        match &app.world().resource::<RemoteCatalog>().state {
            RemoteCatalogState::Ready(_) => Some(()),
            RemoteCatalogState::Error(error) => panic!("mock catalog fetch failed: {error}"),
            _ => None,
        }
    });
    // Shrink the stall window so the test drives the real system quickly.
    app.insert_resource(PortalFetchTimeout(Duration::from_millis(50)));

    app.world_mut().trigger(InstallPortalMod {
        id: "mockmod".to_string(),
    });
    let reason = pump_install_failure(&mut app, "mockmod");
    assert!(
        reason.contains("timed out"),
        "the failure names the stall timeout: {reason}"
    );
    assert_nothing_committed(&app, &guard, "mockmod", &paths_of(&files));
}

/// The install guards, through a SUCCESSFUL mock install: a portal id
/// shadowing a SHIPPED mod is rejected before any fetch, and an id that is
/// already downloaded is rejected on the re-trigger (while the first install
/// stays intact). Also the mock-side proof that a clean install commits:
/// files + index + record all present (the presence contrast that validates
/// the other tests' absence asserts).
#[test]
fn install_guards_reject_shadowing_and_double_install() {
    let guard = cache_root_guard();
    let files = mock_files();
    let mock_entry = entry_for("mockmod", &files);
    // "example" is a SHIPPED catalog id; the portal generator refuses to publish
    // it, but the client must not trust the catalog on that.
    let example_entry = entry_for("example", &files);

    let served: Vec<(String, FetchResult)> = files
        .iter()
        .map(|(path, bytes)| (path.clone(), Ok(bytes.clone())))
        .collect();
    let mut routes = mock_routes(&mock_entry, &served);
    routes.insert(
        format!("{MOCK_BASE}/catalog.json"),
        Ok(catalog_bytes(vec![example_entry, mock_entry.clone()])),
    );
    let mut app = mock_app(routes);

    // Shadowing: rejected with no job files fetched (the mock has no example
    // file routes to hit - a fetch attempt would 404 into a different error).
    app.world_mut().trigger(InstallPortalMod {
        id: "example".to_string(),
    });
    let reason = pump_install_failure(&mut app, "example");
    assert!(
        reason.contains("shadows a shipped mod"),
        "the failure names the shadowing rule: {reason}"
    );
    assert_nothing_committed(&app, &guard, "example", &paths_of(&files));

    // A clean install commits (presence contrast for the absence asserts).
    app.world_mut().trigger(InstallPortalMod {
        id: "mockmod".to_string(),
    });
    pump_install_success(&mut app, "mockmod");
    assert!(
        mod_cache::read_index()
            .unwrap_or_default()
            .iter()
            .any(|r| r.id == "mockmod"),
        "the successful install writes its index record"
    );
    for (path, bytes) in &files {
        assert_eq!(
            mod_cache::read_mod_file("mockmod", path).as_ref(),
            Some(bytes),
            "the successful install caches '{path}'"
        );
    }

    // Double install: rejected, and the installed state is untouched.
    app.world_mut().trigger(InstallPortalMod {
        id: "mockmod".to_string(),
    });
    let reason = pump_install_failure(&mut app, "mockmod");
    assert!(
        reason.contains("already installed"),
        "the failure names the double-install guard: {reason}"
    );
    assert_eq!(
        app.world()
            .resource::<DownloadedMods>()
            .0
            .iter()
            .filter(|m| m.record.id == "mockmod")
            .count(),
        1,
        "the first install survives the rejected retry"
    );
}

/// Installing a mod whose PORTAL entry declares a dependency also installs the
/// dependency from the same portal first (task 20260715-142931): triggering the
/// install of `cool` (deps [lib]) pulls `lib` too, so BOTH land in the cache.
/// Deleting the dependency-resolution loop leaves `lib` uninstalled and this
/// test's `lib` wait times out.
#[test]
fn installing_a_mod_auto_installs_its_portal_dependency() {
    let _guard = cache_root_guard();
    let files = mock_files();
    let lib = entry_for("lib", &files);
    let mut cool = entry_for("cool", &files);
    cool.meta.dependencies = vec!["lib".to_string()];

    // A catalog listing BOTH mods, plus each mod's files under its own prefix.
    let mut routes = HashMap::new();
    routes.insert(
        format!("{MOCK_BASE}/catalog.json"),
        Ok(catalog_bytes(vec![cool.clone(), lib.clone()])),
    );
    for entry in [&cool, &lib] {
        for (path, bytes) in &files {
            routes.insert(
                format!("{MOCK_BASE}/{}/{}/{path}", entry.id, entry.version),
                Ok(bytes.clone()),
            );
        }
    }
    let mut app = mock_app(routes);

    app.world_mut().trigger(InstallPortalMod {
        id: "cool".to_string(),
    });
    pump_install_success(&mut app, "lib");
    pump_install_success(&mut app, "cool");

    let downloaded = app.world().resource::<DownloadedMods>();
    assert!(
        downloaded.0.iter().any(|m| m.record.id == "cool"),
        "the requested mod installed"
    );
    assert!(
        downloaded.0.iter().any(|m| m.record.id == "lib"),
        "its declared dependency was auto-installed"
    );
}

/// A dependency that is neither installed nor in the portal fails the install
/// up front, NAMING the missing dependency, and commits nothing.
#[test]
fn installing_a_mod_with_an_unavailable_dependency_fails_naming_it() {
    let _guard = cache_root_guard();
    let files = mock_files();
    let mut cool = entry_for("cool", &files);
    cool.meta.dependencies = vec!["ghost".to_string()];
    let served: Vec<(String, FetchResult)> = files
        .iter()
        .map(|(p, b)| (p.clone(), Ok(b.clone())))
        .collect();
    let mut app = mock_app(mock_routes(&cool, &served));

    app.world_mut().trigger(InstallPortalMod {
        id: "cool".to_string(),
    });
    let reason = pump_install_failure(&mut app, "cool");
    assert!(
        reason.contains("ghost"),
        "the failure names the missing dependency: {reason}"
    );
    assert!(
        !app.world()
            .resource::<DownloadedMods>()
            .0
            .iter()
            .any(|m| m.record.id == "cool"),
        "the mod did not install when its dependency was unavailable"
    );
}
