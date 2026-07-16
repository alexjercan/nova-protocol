//! A Bevy plugin for loading game assets and initializing asset resources.

use std::collections::{HashMap, HashSet};

use bevy::{
    prelude::*,
    render::render_resource::{TextureViewDescriptor, TextureViewDimension},
};
use bevy_asset_loader::prelude::*;
use nova_gameplay::prelude::*;
use nova_modding::prelude::{
    BundleAsset, Content, ContentAsset, InstalledCatalog, ModEntry, ModMeta,
};
use nova_scenario::prelude::{GameScenarios, NewGameStart};

pub mod mod_cache;
pub mod mod_prefs;
pub mod portal;
mod scenario;
mod sections;

pub mod prelude {
    pub use nova_mod_format::{PortalCatalog, PortalEntry};
    pub use nova_modding::prelude::ModMeta;

    pub use super::{
        portal::{
            FetchPortalCatalog, InstallJobs, InstallPortalMod, InstallStatus, PendingRemovals,
            PortalConfig, PortalFetchTimeout, RemoteCatalog, RemoteCatalogState,
            UninstallPortalMod,
        },
        DownloadedMod, DownloadedMods, EnabledMods, GameAssets, GameAssetsPlugin, GameAssetsStates,
        ModCatalog, ModInfo,
    };
}

/// The RON generation surface for the built-in scenarios (task 20260525-133028
/// follow-up). The scenario builders are the single definition of each
/// built-in; production loads their serialized RON. This module rebuilds them
/// with PATH-based asset refs and serializes them deterministically for two
/// consumers that must agree byte for byte: the `gen_content` bin WRITES the
/// committed files (`cargo run -p nova_assets --bin gen_content`, task
/// 20260716-155823) and the `content_ron_parity` integration test ASSERTS
/// them. Not part of the game's public API.
///
/// The `ScenarioConfig` serde derives are already present in this crate's
/// build: `nova_modding` (a dependency) turns on `nova_scenario/serde`, and
/// Cargo feature unification carries it here.
#[doc(hidden)]
pub mod scenario_generation {
    use nova_gameplay::prelude::{AssetRef, SectionConfig};
    use nova_modding::prelude::Content;
    use nova_scenario::prelude::ScenarioConfig;

    use crate::sections::{build_sections, SectionMeshRefs};

    /// The skybox cubemap asset path (matches `GameAssets::cubemap`).
    const CUBEMAP_PATH: &str = "textures/cubemap.png";
    /// Broadside's deep-field sky: the alt cubemap, so chapter two reads as
    /// a different place than the trainer belt.
    const CUBEMAP_ALT2_PATH: &str = "textures/cubemap_alt2.png";
    /// The asteroid texture asset path (matches `GameAssets::asteroid_texture`).
    const ASTEROID_TEXTURE_PATH: &str = "textures/asteroid.png";

    /// The section-prototype catalog built from PATH-based mesh refs - the source
    /// the content parity test wraps as `Content::Section` items and serializes
    /// into `assets/base/sections/base.content.ron` (production loads that file
    /// via the base bundle and routes its items into `GameSections` via
    /// `register_bundles`).
    pub fn build_section_catalog() -> Vec<SectionConfig> {
        build_sections(&SectionMeshRefs::from_paths())
    }

    /// Build the built-in configs with path-based asset refs, in a stable
    /// order. This is the source the parity test serializes and compares. The
    /// ships now reference the section catalog by prototype id, so the scenario
    /// generators no longer need the resolved `GameSections`.
    pub fn build_scenarios() -> Vec<ScenarioConfig> {
        let cubemap = || AssetRef::from(CUBEMAP_PATH.to_string());
        let texture = || AssetRef::from(ASTEROID_TEXTURE_PATH.to_string());

        vec![
            crate::scenario::asteroid_next(cubemap()),
            crate::scenario::asteroid_field(cubemap(), texture()),
            crate::scenario::menu_ambience(cubemap(), texture()),
            crate::scenario::menu_waystation(cubemap(), texture()),
            crate::scenario::menu_scrapyard(cubemap(), texture()),
            crate::scenario::shakedown::shakedown_run(cubemap(), texture()),
            crate::scenario::broadside::broadside(
                AssetRef::from(CUBEMAP_ALT2_PATH.to_string()),
                texture(),
            ),
        ]
    }

    /// The section catalog wrapped as one `Vec<Content>` of `Content::Section`
    /// items - the shape the committed `assets/base/sections/base.content.ron` file
    /// carries. The parity test serializes this.
    pub fn build_section_content() -> Vec<Content> {
        build_section_catalog()
            .into_iter()
            .map(Content::Section)
            .collect()
    }

    /// The built-in scenarios, each wrapped as its own single-item
    /// `Vec<Content>` (`[Content::Scenario(..)]`) keyed by scenario id - the
    /// shape each committed `assets/scenarios/<id>.content.ron` file carries. The
    /// parity test serializes each.
    pub fn build_scenario_contents() -> Vec<(String, Vec<Content>)> {
        build_scenarios()
            .into_iter()
            .map(|scenario| (scenario.id.clone(), vec![Content::Scenario(scenario)]))
            .collect()
    }

    /// The deterministic pretty-printer for the built-in content RON. Matches
    /// the hand-authored mod content style (e.g. `assets/mods/demo/mod.content.ron`):
    /// struct names omitted, indented, so the data files stay diff-friendly and
    /// reviewable.
    pub fn pretty_config() -> ron::ser::PrettyConfig {
        ron::ser::PrettyConfig::default()
            .struct_names(false)
            .separate_tuple_members(true)
            .enumerate_arrays(false)
    }

    /// Serialize one content `Vec` the way the committed files are authored:
    /// the deterministic pretty config plus a trailing newline (POSIX-clean).
    pub fn serialize_content(content: &[Content]) -> String {
        let body = ron::ser::to_string_pretty(&content.to_vec(), pretty_config())
            .expect("serialize content Vec");
        format!("{body}\n")
    }

    /// Every builder-backed content file as (assets-root-relative path,
    /// serialized body), in a stable order. The single file map both the
    /// `gen_content` bin (writes) and the parity test (asserts) walk, so the
    /// two can never disagree about what exists or what it contains.
    pub fn content_files() -> Vec<(String, String)> {
        let mut files = vec![(
            "base/sections/base.content.ron".to_string(),
            serialize_content(&build_section_content()),
        )];
        files.extend(build_scenario_contents().into_iter().map(|(id, content)| {
            (
                format!("base/scenarios/{id}.content.ron"),
                serialize_content(&content),
            )
        }));
        files
    }
}

/// The production `register_bundles` system, re-exported for the crate's
/// integration tests (which drive the RON modding pipeline end to end: load the
/// base bundle + its content files and route their items into `GameSections` /
/// `GameScenarios`). Not part of the public API.
#[doc(hidden)]
pub use crate::register_bundles as register_bundles_for_test;

/// One DOWNLOADED mod's runtime state: its cache-index record plus the live
/// handle for its bundle, loaded from the `mods://` source
/// (`mods://<id>/<bundle>`) through the same loaders as a shipped bundle.
#[derive(Clone, Debug)]
pub struct DownloadedMod {
    /// The cache-index record (id, version, bundle path).
    pub record: mod_cache::InstalledModRecord,
    /// The bundle handle, held here so the asset stays alive while installed.
    pub bundle: Handle<BundleAsset>,
}

/// The DOWNLOADED half of the installed set, in cache-index order - the
/// runtime view of `mod_cache::read_index()` with each record's bundle loading
/// via `mods://`. The shipped half stays the `InstalledCatalog` asset.
///
/// Filled at startup (natively straight from the index; on the web after the
/// IndexedDB hydration task completes) and mutated by the future
/// install/uninstall flow (task 163508). `build_mod_catalog` appends these as
/// player-facing rows and `register_bundles` merges the ENABLED ones after the
/// shipped bundles; both re-run when this resource changes, and
/// [`mark_downloaded_bundles_loaded`] flags a change when a bundle's async
/// load completes so a mod never stays merged-out just because it loaded late.
///
/// Downloaded mods install DISABLED: nothing here touches [`EnabledMods`], so a
/// fresh install only renders a row until the player toggles it on.
#[derive(Resource, Clone, Debug, Default)]
pub struct DownloadedMods(pub Vec<DownloadedMod>);

/// The set of ENABLED mod ids (catalog entry ids). `register_bundles` merges only
/// the cataloged bundles whose id is in this set, in catalog order.
///
/// Runtime state, NOT baked into any read-only asset: `seed_enabled_mods` fills it
/// from the catalog's `base` entries at startup (persistence, task 174131, will load
/// a saved set instead), and the mods menu (task 174126) toggles ids in and out. It
/// is `Changed`-watched so a toggle re-runs the merge live.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct EnabledMods(pub HashSet<String>);

/// One PLAYER-FACING installed mod: the catalog declaration's identity + flags
/// composed with the mod's [`ModMeta`] self-description from its own bundle.
///
/// Built by [`ModInfo::new`], which normalizes an empty meta name to the id so a
/// meta-less mod still renders a usable row.
#[derive(Clone, Debug)]
pub struct ModInfo {
    /// Stable id - the enable/disable key (from the catalog declaration).
    pub id: String,
    /// True for the base game's entry (locked on in the UI).
    pub base: bool,
    /// The mod's self-description, from its bundle's `meta` block; `name` is
    /// guaranteed non-empty (falls back to `id`).
    pub meta: ModMeta,
}

impl ModInfo {
    /// Compose a catalog declaration with its bundle's meta (if the bundle is
    /// loaded); an empty meta name falls back to the id.
    pub fn new(decl: &ModEntry, meta: Option<&ModMeta>) -> Self {
        let mut meta = meta.cloned().unwrap_or_default();
        if meta.name.is_empty() {
            meta.name = decl.id.clone();
        }
        Self {
            id: decl.id.clone(),
            base: decl.base,
            meta,
        }
    }
}

/// The PLAYER-FACING installed-mods list, in catalog order - the menu's view of
/// the [`InstalledCatalog`] asset composed with each mod's bundle [`ModMeta`],
/// with `hidden: true` entries (dev/tooling mods) filtered out.
///
/// Built once from the loaded catalog at `OnEnter(Processing)` by
/// [`build_mod_catalog`]. The mods menu reads this (plus [`EnabledMods`]) to render
/// its list without touching the asset machinery. Empty until the catalog loads.
/// Hidden mods stay installed and enableable by id (`register_bundles` reads the
/// full catalog, not this view); they just never reach the menu.
#[derive(Resource, Clone, Debug, Default)]
pub struct ModCatalog(pub Vec<ModInfo>);

/// Fill [`ModCatalog`] from the loaded [`InstalledCatalog`] asset, composing each
/// non-`hidden` declaration with its bundle's [`ModMeta`], in catalog order, then
/// append one row per DOWNLOADED mod ([`DownloadedMods`], cache-index order).
/// Runs at `OnEnter(Processing)`, before `seed_enabled_mods`, and re-runs when
/// `DownloadedMods` changes (install/uninstall, or a downloaded bundle's async
/// load completing) so the rows track the cache. A missing/unloaded bundle is
/// logged and degrades to a decl-only row (name = id), never a panic.
pub fn build_mod_catalog(
    game_assets: Res<GameAssets>,
    catalogs: Res<Assets<InstalledCatalog>>,
    bundles: Res<Assets<BundleAsset>>,
    downloaded: Res<DownloadedMods>,
    mut mod_catalog: ResMut<ModCatalog>,
) {
    let Some(catalog) = catalogs.get(&game_assets.catalog) else {
        error!("build_mod_catalog: the mods catalog was not loaded; the mods list is empty");
        return;
    };
    mod_catalog.0 = catalog
        .entries
        .iter()
        .filter(|e| !e.decl.hidden)
        .map(|e| {
            let meta = bundles.get(&e.bundle).map(|b| &b.meta);
            if meta.is_none() {
                error!(
                    "build_mod_catalog: bundle for mod '{}' not loaded; using its id as the name",
                    e.decl.id
                );
            }
            ModInfo::new(&e.decl, meta)
        })
        .collect();
    for m in &downloaded.0 {
        // A downloaded id shadowing a SHIPPED catalog entry (hidden ones
        // included - one id space) is skipped, mirroring the portal
        // generator's no-shadowing rule; otherwise one toggle would drive two
        // rows/bundles (review 142906 R1.2). `register_bundles` skips the
        // same records, so the pair stays consistent.
        if catalog.entries.iter().any(|e| e.decl.id == m.record.id) {
            warn!(
                "build_mod_catalog: downloaded mod '{}' shadows a shipped mod id; \
                 hiding the downloaded row",
                m.record.id
            );
            continue;
        }
        // A downloaded bundle loads ASYNC via mods:// (it is not part of the
        // GameAssets collection gate), so a not-yet-loaded meta is normal here -
        // the row starts decl-only (name = id) and upgrades on the re-run that
        // `mark_downloaded_bundles_loaded` triggers. No `hidden`/`base` flags:
        // downloaded records carry neither concept.
        let meta = bundles.get(&m.bundle).map(|b| &b.meta);
        let decl = ModEntry {
            id: m.record.id.clone(),
            bundle: m.record.bundle.clone(),
            base: false,
            hidden: false,
        };
        mod_catalog.0.push(ModInfo::new(&decl, meta));
    }
}

/// Reconcile [`EnabledMods`] with the catalog: union `base: true` ids in, strip
/// `hidden` (non-base) ids out.
///
/// The UNION keeps base enabled regardless of what `load_enabled_mods` restored -
/// base is locked on in the UI, so it must always be active - while preserving any
/// persisted or toggled non-base choices. The STRIP makes a hidden (dev/tooling)
/// mod's enablement SESSION-ONLY: without it, an example run that enables a hidden
/// mod persists the id, and a later normal run would restore-and-merge a mod the
/// menu has no row to disable (task 20260715-142844 R1.1). Examples re-enable by id
/// at `OnEnter(Loaded)`, after this chain, so they are unaffected; the cleaned set
/// is re-saved on the same change, so a polluted prefs store self-heals. The `!base`
/// guard keeps a pathological hidden+base entry force-enabled. Runs at
/// `OnEnter(Processing)`, after `load_enabled_mods` and before the merge. Idempotent.
pub fn seed_enabled_mods(
    game_assets: Res<GameAssets>,
    catalogs: Res<Assets<InstalledCatalog>>,
    mut enabled: ResMut<EnabledMods>,
) {
    let Some(catalog) = catalogs.get(&game_assets.catalog) else {
        error!("seed_enabled_mods: the mods catalog was not loaded; nothing enabled by default");
        return;
    };
    for entry in &catalog.entries {
        if entry.decl.base {
            enabled.0.insert(entry.decl.id.clone());
        } else if entry.decl.hidden {
            enabled.0.remove(&entry.decl.id);
        }
    }
}

/// Restore the saved enabled-mods set at startup, if any (task 174131).
///
/// Runs FIRST in the `OnEnter(Processing)` chain, before `seed_enabled_mods`. When
/// the platform store holds a saved set it becomes `EnabledMods`; `seed_enabled_mods`
/// then unions base in (so base is always on), and the merge reflects the restored
/// choices. With NO saved set, `EnabledMods` stays empty here and `seed_enabled_mods`
/// falls back to the base-only default - identical to pre-persistence startup.
pub fn load_enabled_mods(mut enabled: ResMut<EnabledMods>) {
    if let Some(ids) = mod_prefs::load_enabled_ids() {
        enabled.0 = ids.into_iter().collect();
    }
}

/// Persist [`EnabledMods`] whenever it changes (a menu toggle, or the startup seed).
/// Runs in `Update`, gated on `resource_changed::<EnabledMods>`.
pub fn save_enabled_mods(enabled: Res<EnabledMods>) {
    let mut ids: Vec<String> = enabled.0.iter().cloned().collect();
    // Sort for a stable, diff-friendly on-disk file (HashSet order is arbitrary).
    ids.sort();
    mod_prefs::save_enabled_ids(&ids);
}

/// Turn the cache-index records into [`DownloadedMods`], kicking each bundle's
/// load from the `mods://` source. Shared by the native startup read and the
/// wasm post-hydration poll. Loading through the asset server here (not the
/// `GameAssets` collection) is deliberate: downloaded mods appear and disappear
/// at runtime, so they cannot sit behind the one-shot collection gate.
///
/// The on-disk index is DOWNLOADED input: a record whose id or bundle path
/// could escape the cache (a `..` component, an absolute path, a nested id)
/// is skipped with a warning before any asset path is built from it (review
/// 142906 R1.1; the native source is additionally sandboxed, since a malicious
/// bundle MANIFEST can request an escaping path without touching the index).
fn start_downloaded_loads(
    records: Vec<mod_cache::InstalledModRecord>,
    asset_server: &AssetServer,
    downloaded: &mut DownloadedMods,
) {
    downloaded.0 = records
        .into_iter()
        .filter_map(|record| {
            if !mod_cache::is_safe_id(&record.id) || !mod_cache::is_safe_rel_path(&record.bundle) {
                warn!(
                    "mod cache: skipping downloaded mod record with an unsafe id or bundle \
                     path (id '{}', bundle '{}')",
                    record.id, record.bundle
                );
                return None;
            }
            let path = format!(
                "{}://{}/{}",
                mod_cache::MODS_SOURCE,
                record.id,
                record.bundle
            );
            Some(DownloadedMod {
                bundle: asset_server.load(path),
                record,
            })
        })
        .collect();
}

/// Native startup: read the downloaded-mods index and kick each bundle's
/// `mods://` load (the `FileAssetReader` reads the cache live - no hydration
/// step). The web target replaces this with the hydrate-then-poll pair below,
/// because its memory-backed source must be filled from IndexedDB first.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_downloaded_mods(
    asset_server: Res<AssetServer>,
    mut downloaded: ResMut<DownloadedMods>,
) {
    let records = mod_cache::read_index().unwrap_or_default();
    start_downloaded_loads(records, &asset_server, &mut downloaded);
}

/// The in-flight IndexedDB hydration: the spawned task parks the index records
/// here once every cached file sits in the `mods://` memory `Dir`. Removed by
/// [`poll_mod_cache_hydration`] when consumed.
#[cfg(target_arch = "wasm32")]
#[derive(Resource)]
pub struct ModCacheHydration(
    std::sync::Arc<std::sync::Mutex<Option<Vec<mod_cache::InstalledModRecord>>>>,
);

/// Web startup: hydrate the `mods://` memory `Dir` from IndexedDB in an
/// `IoTaskPool` task (on wasm the pool drives futures via the browser event
/// loop, and spawn accepts non-Send futures). The bundle loads must NOT be
/// kicked until hydration completes - a memory-source read of a missing path
/// fails the load permanently - so the task only publishes the index records
/// for [`poll_mod_cache_hydration`] to consume. Gated on [`ModsSourceDir`]
/// existing (it is inserted by `mod_cache::register_mods_source`; an app built
/// without the source has nothing to hydrate).
#[cfg(target_arch = "wasm32")]
pub fn start_mod_cache_hydration(mut commands: Commands, dir: Res<mod_cache::ModsSourceDir>) {
    let dir = dir.0.clone();
    let slot = std::sync::Arc::new(std::sync::Mutex::new(None));
    let done = slot.clone();
    bevy::tasks::IoTaskPool::get()
        .spawn(async move {
            for (key, bytes) in mod_cache::read_all_files().await {
                dir.insert_asset(std::path::Path::new(&key), bytes);
            }
            let records = mod_cache::read_index().unwrap_or_default();
            *done.lock().unwrap() = Some(records);
        })
        .detach();
    commands.insert_resource(ModCacheHydration(slot));
}

/// Web: once the hydration task has published the index records, kick the
/// bundle loads (same shared path as native) and drop the marker resource so
/// this system stops running.
#[cfg(target_arch = "wasm32")]
pub fn poll_mod_cache_hydration(
    mut commands: Commands,
    hydration: Res<ModCacheHydration>,
    asset_server: Res<AssetServer>,
    mut downloaded: ResMut<DownloadedMods>,
) {
    let Some(records) = hydration.0.lock().unwrap().take() else {
        return;
    };
    start_downloaded_loads(records, &asset_server, &mut downloaded);
    commands.remove_resource::<ModCacheHydration>();
}

/// The run condition for the installed-set-driven re-merge: EITHER half of the
/// installed set changed - [`EnabledMods`] (a menu toggle, the startup seed) or
/// [`DownloadedMods`] (install/uninstall, or a downloaded bundle's load landing
/// via [`mark_downloaded_bundles_loaded`]). One reader consuming both change
/// ticks together, which two chained `resource_changed` conditions would not do
/// (their or-combinator short-circuits and leaves the second tick primed).
/// Public so the integration rigs gate on the exact production condition.
pub fn installed_set_changed(enabled: Res<EnabledMods>, downloaded: Res<DownloadedMods>) -> bool {
    enabled.is_changed() || downloaded.is_changed()
}

/// Flag [`DownloadedMods`] as changed when one of its bundles finishes loading
/// (recursively, content files included). Downloaded bundles load async - they
/// are outside the `GameAssets` collection gate - so without this the
/// change-gated re-runs of `register_bundles` / `build_mod_catalog` would never
/// see a bundle that finished AFTER the last resource mutation, and an enabled
/// downloaded mod would stay merged-out until some unrelated toggle.
pub fn mark_downloaded_bundles_loaded(
    mut events: MessageReader<AssetEvent<BundleAsset>>,
    mut downloaded: ResMut<DownloadedMods>,
) {
    for event in events.read() {
        let AssetEvent::LoadedWithDependencies { id } = event else {
            continue;
        };
        if downloaded.0.iter().any(|m| m.bundle.id() == *id) {
            downloaded.set_changed();
        }
    }
}

/// Route every ENABLED cataloged bundle's content into the id-keyed game registries,
/// with load-order overlay.
///
/// It walks the catalog in order, keeps the entries whose id is in [`EnabledMods`]
/// (base first, by catalog order), flattens each kept bundle's content (in manifest
/// order, across its content files), and hands the whole ordered list to
/// [`merge_bundles`]. A LATER (mod) bundle wins on an id collision with the base
/// (load-order overlay); a duplicate id WITHIN one bundle is a conflict, logged and
/// skipped. Both resources are always inserted (empty if nothing enabled/loaded).
///
/// The catalog is part of the `GameAssets` collection and visits every installed
/// bundle as a dependency, so bevy_asset_loader gates the collection on the whole
/// tree's RECURSIVE load state - every installed bundle + content file is loaded
/// before this first runs `OnEnter(Processing)`, regardless of which are enabled. A
/// handle whose asset is somehow not loaded is logged and skipped (never a panic).
/// Re-runs whenever `EnabledMods` changes so a menu toggle applies live.
///
/// ENABLED DOWNLOADED bundles ([`DownloadedMods`]) merge AFTER the shipped ones,
/// in cache-index order, through the same overlay rules. They sit outside the
/// collection gate (loaded async via `mods://`), so a still-loading bundle is
/// skipped with a warning; [`mark_downloaded_bundles_loaded`] re-triggers this
/// system when the load lands, and a `DownloadedMods` change (install/uninstall)
/// re-triggers it too.
pub fn register_bundles(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    enabled: Res<EnabledMods>,
    downloaded: Res<DownloadedMods>,
    catalogs: Res<Assets<InstalledCatalog>>,
    bundles: Res<Assets<BundleAsset>>,
    contents: Res<Assets<ContentAsset>>,
) {
    // Ordered ENABLED bundle handles: catalog order (base first), keeping only
    // entries whose id is enabled.
    let catalog = catalogs.get(&game_assets.catalog);
    if catalog.is_none() {
        error!("register_bundles: the mods catalog was not loaded; registering nothing");
    }
    // Enabled (id, bundle) pairs in catalog order (base first) then downloaded
    // order - the stable tiebreak the dependency sort keeps below.
    let mut ordered: Vec<(&str, &Handle<BundleAsset>)> = Vec::new();
    if let Some(catalog) = catalog {
        for entry in &catalog.entries {
            if enabled.0.contains(&entry.decl.id) {
                ordered.push((entry.decl.id.as_str(), &entry.bundle));
            }
        }
    }
    for m in &downloaded.0 {
        if !enabled.0.contains(&m.record.id) {
            continue;
        }
        // A downloaded id shadowing a SHIPPED catalog entry is skipped (the
        // portal generator's no-shadowing rule, enforced again at the merge
        // because the index is downloaded input) - otherwise one enabled id
        // would merge two bundles (review 142906 R1.2). `build_mod_catalog`
        // hides the same records from the rows.
        if catalog.is_some_and(|c| c.entries.iter().any(|e| e.decl.id == m.record.id)) {
            warn!(
                "register_bundles: downloaded mod '{}' shadows a shipped mod id; \
                 skipping the downloaded copy",
                m.record.id
            );
            continue;
        }
        // Unlike the shipped entries above (gated loaded by the collection), a
        // downloaded bundle may still be in flight; skipping it here is a
        // TRANSIENT state, not the shared "somehow not loaded" error below -
        // the loaded-event re-run merges it in.
        if bundles.contains(&m.bundle) {
            ordered.push((m.record.id.as_str(), &m.bundle));
        } else {
            warn!(
                "register_bundles: downloaded mod '{}' is enabled but its bundle has not \
                 loaded yet; it merges when the load completes",
                m.record.id
            );
        }
    }

    // Dependency-respecting merge order (task 20260715-142931): a mod's Content
    // overlays its dependencies' (last-wins by id), so a dependency must merge
    // BEFORE its dependents. Build the id->deps graph from the loaded bundles'
    // meta and topologically sort, keeping the catalog-then-download order as the
    // stable tiebreak. `base` is implicit (first in catalog order, no incoming
    // edges) so it stays first. A cycle - which the portal generator rejects at
    // publish, but a hand-installed set could carry - warns and falls back to
    // input order.
    //
    // The graph only carries edges for bundles that are LOADED (`bundles.get`);
    // an enabled dependent whose bundle is still loading contributes no edges and
    // may briefly merge before its dependency, but that is transient - the
    // loaded-event re-run of this system (above) rebuilds with the full graph.
    let graph: nova_mod_format::deps::DepGraph = ordered
        .iter()
        .filter_map(|(id, handle)| {
            bundles
                .get(*handle)
                .map(|b| (id.to_string(), b.meta.dependencies.clone()))
        })
        .collect();
    let ids: Vec<String> = ordered.iter().map(|(id, _)| id.to_string()).collect();
    let topo = nova_mod_format::deps::topological_order(&ids, &graph);
    if topo.cycle {
        warn!(
            "register_bundles: a dependency cycle among enabled mods prevents a full \
             topological order; merging the cyclic mods in catalog order"
        );
    }
    // `ordered`'s ids are unique (a downloaded id that shadows a shipped one is
    // skipped above), so this id->handle map never drops a bundle.
    let by_id: HashMap<&str, &Handle<BundleAsset>> =
        ordered.iter().map(|(id, h)| (*id, *h)).collect();
    let bundle_handles: Vec<&Handle<BundleAsset>> = topo
        .order
        .iter()
        .filter_map(|id| by_id.get(id.as_str()).copied())
        .collect();

    // Flatten each enabled bundle into its ordered `&Content` items (missing content
    // is logged and skipped). Kept as one Vec per bundle so `merge_bundles` can tell
    // intra-bundle duplicates from cross-bundle overlay.
    let mut bundle_items: Vec<Vec<&Content>> = Vec::new();
    for bundle_handle in bundle_handles {
        let Some(bundle) = bundles.get(bundle_handle) else {
            error!(
                "register_bundles: a bundle asset was not loaded; skipping it \
                 (the other bundles still register)"
            );
            continue;
        };
        let mut items: Vec<&Content> = Vec::new();
        for content_handle in &bundle.content {
            let Some(content) = contents.get(content_handle) else {
                error!(
                    "register_bundles: a content asset was not loaded; skipping it \
                     (the other content still registers)"
                );
                continue;
            };
            items.extend(content.0.iter());
        }
        bundle_items.push(items);
    }

    let outcome = merge_bundles(bundle_items.iter().map(|items| items.iter().copied()));
    for conflict in &outcome.conflicts {
        error!("register_bundles: {conflict}");
    }

    // The New Game start comes from the BASE bundle's manifest and ONLY from
    // it (task 20260716-155849): any other bundle declaring
    // `new_game_scenario` - shipped or downloaded - is warned about and
    // ignored, so a mod can never redirect what New Game launches.
    let mut new_game: Option<String> = None;
    if let Some(catalog) = catalog {
        for entry in &catalog.entries {
            let Some(bundle) = bundles.get(&entry.bundle) else {
                continue;
            };
            let Some(declared) = &bundle.new_game_scenario else {
                continue;
            };
            if entry.decl.base {
                new_game = Some(declared.clone());
            } else {
                warn!(
                    "register_bundles: mod '{}' declares new_game_scenario '{declared}'; \
                     ignored - only the base bundle picks the New Game start",
                    entry.decl.id
                );
            }
        }
    }
    for m in &downloaded.0 {
        if let Some(declared) = bundles
            .get(&m.bundle)
            .and_then(|b| b.new_game_scenario.as_ref())
        {
            warn!(
                "register_bundles: downloaded mod '{}' declares new_game_scenario '{declared}'; \
                 ignored - only the base bundle picks the New Game start",
                m.record.id
            );
        }
    }
    commands.insert_resource(NewGameStart(new_game));

    commands.insert_resource(GameSections(outcome.sections));
    commands.insert_resource(outcome.scenarios);
}

/// The result of merging an ordered list of bundles: the id-keyed registries plus
/// any intra-bundle id conflicts that were detected (and skipped).
pub struct MergeOutcome {
    /// Sections in registration order (base then mods), overlaid last-wins by id.
    pub sections: Vec<SectionConfig>,
    /// Scenarios keyed by id, overlaid last-wins.
    pub scenarios: GameScenarios,
    /// Human-readable messages, one per intra-bundle duplicate id that was
    /// skipped. Empty on clean data.
    pub conflicts: Vec<String>,
}

/// Merge an ORDERED list of bundles into the id-keyed registries. Each bundle is
/// an ordered list of its `&Content` items (already flattened across the bundle's
/// content files).
///
/// Two overlay rules, mirroring Wesnoth's base+addons model:
/// - CROSS-bundle (a later bundle vs an earlier one): last-wins overlay by id -
///   a mod's `Content` with the same id as the base REPLACES it. This is the
///   whole point of mods.
/// - INTRA-bundle (the same id twice in ONE bundle - including the BASE bundle,
///   whose content files flatten into one bundle): a conflict. The first item is
///   kept, the duplicate is skipped, and a message is recorded. This is an
///   authoring error in any pack, surfaced loudly (the caller logs it) rather than
///   silently last-wins-overlaid like the cross-bundle case - but NOT a panic, so
///   bad mod (or base) data cannot crash the app.
pub fn merge_bundles<'a, B, I>(bundles: B) -> MergeOutcome
where
    B: IntoIterator<Item = I>,
    I: IntoIterator<Item = &'a Content>,
{
    let mut sections: Vec<SectionConfig> = Vec::new();
    let mut scenarios = GameScenarios::default();
    let mut conflicts: Vec<String> = Vec::new();

    for bundle in bundles {
        // Ids seen in THIS bundle, per kind - reset each bundle so a later bundle
        // may overlay an earlier one, while a repeat within one bundle conflicts.
        let mut seen_sections: HashSet<&str> = HashSet::new();
        let mut seen_scenarios: HashSet<&str> = HashSet::new();

        for item in bundle {
            match item {
                Content::Section(cfg) => {
                    if !seen_sections.insert(cfg.base.id.as_str()) {
                        conflicts.push(format!(
                            "section id '{}' appears more than once in one bundle; \
                             keeping the first, skipping the duplicate",
                            cfg.base.id
                        ));
                        continue;
                    }
                    merge_content_item(item, &mut sections, &mut scenarios);
                }
                Content::Scenario(cfg) => {
                    if !seen_scenarios.insert(cfg.id.as_str()) {
                        conflicts.push(format!(
                            "scenario id '{}' appears more than once in one bundle; \
                             keeping the first, skipping the duplicate",
                            cfg.id
                        ));
                        continue;
                    }
                    merge_content_item(item, &mut sections, &mut scenarios);
                }
            }
        }
    }

    MergeOutcome {
        sections,
        scenarios,
        conflicts,
    }
}

/// Route one content item into the accumulating registries with last-wins
/// overlay by id. Both kinds overlay identically: a later item (from a later
/// bundle) with the same id replaces the earlier one rather than appending a
/// shadowed duplicate. Sections keep a Vec (order matters for the editor palette)
/// so overlay is a linear replace-in-place; scenarios are a map so overlay is a
/// plain `insert`. Called by [`merge_bundles`] once per accepted item.
fn merge_content_item(
    item: &Content,
    sections: &mut Vec<SectionConfig>,
    scenarios: &mut GameScenarios,
) {
    match item {
        Content::Section(cfg) => match sections.iter_mut().find(|s| s.base.id == cfg.base.id) {
            Some(existing) => *existing = cfg.clone(),
            None => sections.push(cfg.clone()),
        },
        Content::Scenario(cfg) => {
            scenarios.insert(cfg.id.clone(), cfg.clone());
        }
    }
}

/// Game states for the asset loader.
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameAssetsStates {
    #[default]
    Loading,
    Processing,
    Loaded,
}

/// A plugin that loads game assets and sets up the game.
pub struct GameAssetsPlugin;

impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut App) {
        debug!("GameAssetsPlugin: build");

        // The modding plugin registers the `*.content.ron` asset + loader.
        // Add it before the loading state runs so the loader exists when
        // bevy_asset_loader starts loading the content files below.
        app.add_plugins(nova_modding::prelude::NovaModdingPlugin);
        // The portal client (fetch catalog + install/uninstall over the wire,
        // task 20260715-163508) - event/resource API only; the UI binds later.
        app.add_plugins(portal::PortalPlugin);

        // The enabled-mods set drives which cataloged bundles merge. Seeded from
        // the catalog's base entries at Processing; toggled by the mods menu.
        app.init_resource::<EnabledMods>();
        // The menu-facing installed-mods metadata, filled from the catalog at
        // Processing.
        app.init_resource::<ModCatalog>();
        // The downloaded half of the installed set, from the local mod cache.
        app.init_resource::<DownloadedMods>();

        // Read the cache index and kick the mods:// bundle loads. Native reads
        // the filesystem cache directly; the web must first hydrate the
        // memory-backed source from IndexedDB, then poll for completion.
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Startup, load_downloaded_mods);
        #[cfg(target_arch = "wasm32")]
        {
            app.add_systems(
                Startup,
                start_mod_cache_hydration.run_if(resource_exists::<mod_cache::ModsSourceDir>),
            );
            app.add_systems(
                Update,
                poll_mod_cache_hydration.run_if(resource_exists::<ModCacheHydration>),
            );
        }
        // A downloaded bundle finishing its async load must re-trigger the
        // DownloadedMods-gated re-runs below.
        app.add_systems(Update, mark_downloaded_bundles_loaded);

        // Setup the asset loader to load assets during the loading state.
        app.init_state::<GameAssetsStates>();
        app.add_loading_state(
            LoadingState::new(GameAssetsStates::Loading)
                .continue_to_state(GameAssetsStates::Processing)
                .load_collection::<GameAssets>(),
        );

        app.add_systems(
            OnEnter(GameAssetsStates::Processing),
            (
                prepare_cubemap_view,
                build_mod_catalog,
                load_enabled_mods,
                seed_enabled_mods,
                register_bundles,
                register_sounds,
                update_nova_hud_assets,
                |mut state: ResMut<NextState<GameAssetsStates>>| {
                    state.set(GameAssetsStates::Loaded);
                },
            )
                .chain(),
        );

        // Re-merge live when the installed set changes in either half, once the
        // catalog is loaded. The condition also fires on the initial inserts,
        // which is harmless (idempotent re-merge); it is skipped while still
        // loading because the catalog is not yet present (register_bundles logs
        // + no-ops).
        app.add_systems(
            Update,
            register_bundles
                .run_if(resource_exists::<GameAssets>)
                .run_if(installed_set_changed)
                .run_if(not(in_state(GameAssetsStates::Loading))),
        );

        // Rebuild the player-facing rows on the same downloaded-set changes, so
        // an install shows up and a loaded bundle's meta replaces its id-only
        // fallback row. EnabledMods changes do not alter the rows, so this one
        // watches only DownloadedMods.
        app.add_systems(
            Update,
            build_mod_catalog
                .run_if(resource_exists::<GameAssets>)
                .run_if(resource_changed::<DownloadedMods>)
                .run_if(not(in_state(GameAssetsStates::Loading))),
        );

        // Persist the enabled set whenever it changes (a menu toggle, or the startup
        // seed). Gated the same way as the re-merge so it only fires with the real
        // set present, not during the empty-init on Loading.
        app.add_systems(
            Update,
            save_enabled_mods
                .run_if(resource_exists::<GameAssets>)
                .run_if(resource_changed::<EnabledMods>)
                .run_if(not(in_state(GameAssetsStates::Loading))),
        );
    }
}

#[derive(AssetCollection, Resource, Clone)]
pub struct GameAssets {
    #[asset(path = "textures/cubemap.png")]
    pub cubemap: Handle<Image>,
    #[asset(path = "textures/asteroid.png")]
    pub asteroid_texture: Handle<Image>,
    #[asset(path = "gltf/hull-01.glb#Scene0")]
    pub hull_01: Handle<WorldAsset>,
    #[asset(path = "gltf/turret-yaw-01.glb#Scene0")]
    pub turret_yaw_01: Handle<WorldAsset>,
    #[asset(path = "gltf/turret-pitch-01.glb#Scene0")]
    pub turret_pitch_01: Handle<WorldAsset>,
    #[asset(path = "gltf/turret-barrel-01.glb#Scene0")]
    pub turret_barrel_01: Handle<WorldAsset>,
    #[asset(path = "gltf/torpedo-bay-01.glb#Scene0")]
    pub torpedo_bay_01: Handle<WorldAsset>,
    #[asset(path = "icons/fps.png")]
    pub fps_icon: Handle<Image>,
    #[asset(path = "icons/target.png")]
    pub target_sprite: Handle<Image>,
    /// The installed-mods catalog (`assets/mods.catalog.ron`): every installed mod
    /// (base first, then mods) with metadata + a `BundleAsset` handle. The
    /// `InstalledCatalog` asset visits EVERY entry's bundle as a dependency, so
    /// bevy_asset_loader gates the collection on the whole tree's RECURSIVE load
    /// state - every installed bundle + its content is loaded before
    /// `register_bundles` runs at `OnEnter(Processing)`, regardless of which mods
    /// are enabled. `EnabledMods` then selects which cataloged bundles actually
    /// merge (base enabled by default; the mods menu toggles the rest).
    ///
    /// The `<name>.catalog.ron` STEM is load-bearing: bevy_asset_loader kicks off
    /// each collection field with an UNTYPED `load_untyped`, which resolves the
    /// loader by extension only. Bevy's full extension is everything after the FIRST
    /// dot, so a bare `catalog.ron` resolves to `ron` (no loader) and fails;
    /// `mods.catalog.ron` resolves to `catalog.ron` and matches `CatalogLoader`.
    #[asset(path = "mods.catalog.ron")]
    pub catalog: Handle<InstalledCatalog>,
}

/// Give the skybox cubemap its cube texture view.
///
/// The stacked `textures/cubemap.png` is reinterpreted into a 6 layer array
/// at load time by its `.meta` loader settings (`array_layout: RowCount`).
/// Doing it at load time matters: the renderer eagerly uploads every loaded
/// image, and the raw stacked form is 24576 px tall - over the 16384 texture
/// limit of smaller GPUs (e.g. CI's llvmpipe), where the upload becomes a
/// fatal validation error. Whether the old on-insert reinterpret in
/// `SkyboxPlugin` beat that upload depended on which frame the asset
/// finished loading, so the failure was flaky.
///
/// The loader settings cannot express a texture view, so the cube view is
/// set here, in the Processing state - after the collection is loaded and
/// before anything spawns a camera. `SkyboxPlugin` sees the layers and view
/// already prepared and just attaches the `Skybox` component.
///
/// If the meta was not applied (the image still has a single layer), leave
/// the image alone so the `SkyboxPlugin` fallback reinterpret still works.
fn prepare_cubemap_view(mut images: ResMut<Assets<Image>>, game_assets: Res<GameAssets>) {
    let Some(mut image) = images.get_mut(&game_assets.cubemap) else {
        error!("prepare_cubemap_view: cubemap image not loaded");
        return;
    };
    if image.texture_descriptor.array_layer_count() > 1 {
        image.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..default()
        });
    } else {
        warn!(
            "prepare_cubemap_view: cubemap loaded as a single layer image; \
             was the `cubemap.png.meta` array_layout applied?"
        );
    }
}

/// Load the Nova sound effects into a keyed [`SoundBank`] the audio module reads.
///
/// Uses `SoundBank::load` (the bcs registry) rather than the `GameAssets`
/// collection because the bank has no public "build from existing handles"
/// constructor; loading here kicks the (tiny) WAVs off well before the first
/// gameplay sound plays. The `sounds/<name>.wav` convention is applied by the
/// bank, and `NOVA_SFX_FILES` is the single source of truth for the key->file map.
fn register_sounds(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(SoundBank::load(&assets, NOVA_SFX_FILES));
}

// TODO(20260525-133028): Probably need to refactor this somehow
fn update_nova_hud_assets(
    mut nova_hud_assets: ResMut<NovaHudAssets>,
    game_assets: Res<GameAssets>,
) {
    nova_hud_assets.target_sprite = game_assets.target_sprite.clone();
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::{BaseSectionConfig, HullSectionConfig, SectionKind};

    use super::*;

    fn section(id: &str, health: f32) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                health,
                ..Default::default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        }
    }

    /// A later content item with the same section id overlays the earlier one
    /// (last-wins) instead of appending a shadowed duplicate, and does so
    /// in-place so the palette order is preserved. This is the seam mods
    /// (20260714-134127) rely on, mirroring the scenario map's insert-overlay.
    #[test]
    fn later_section_overlays_earlier_by_id_in_place() {
        let mut sections: Vec<SectionConfig> = Vec::new();
        let mut scenarios = GameScenarios::default();

        // Base bundle: two sections in palette order.
        merge_content_item(
            &Content::Section(section("hull", 100.0)),
            &mut sections,
            &mut scenarios,
        );
        merge_content_item(
            &Content::Section(section("thruster", 50.0)),
            &mut sections,
            &mut scenarios,
        );

        // Mod bundle: overlays "hull" with a new health, leaves "thruster".
        merge_content_item(
            &Content::Section(section("hull", 999.0)),
            &mut sections,
            &mut scenarios,
        );

        // No duplicate appended: still two sections, original order kept.
        assert_eq!(sections.len(), 2, "overlay must replace, not append");
        assert_eq!(sections[0].base.id, "hull", "palette order preserved");
        assert_eq!(sections[1].base.id, "thruster");
        // Last-wins: the overlaid value took effect.
        assert_eq!(sections[0].base.health, 999.0, "later section must win");
    }

    /// A later scenario with the same id overlays the earlier one, same as
    /// sections - the two kinds must behave identically under overlay.
    #[test]
    fn later_scenario_overlays_earlier_by_id() {
        let mut sections: Vec<SectionConfig> = Vec::new();
        let mut scenarios = GameScenarios::default();

        // Reuse a real built scenario (no Default on ScenarioConfig) and overlay
        // a second config sharing its id but with a different name.
        let mut base = scenario_generation::build_scenarios()
            .into_iter()
            .next()
            .expect("build_scenarios yields at least one scenario");
        let id = base.id.clone();
        base.name = "base".to_string();
        let mut modded = base.clone();
        modded.name = "modded".to_string();

        merge_content_item(&Content::Scenario(base), &mut sections, &mut scenarios);
        merge_content_item(&Content::Scenario(modded), &mut sections, &mut scenarios);

        assert_eq!(scenarios.len(), 1, "overlay must replace, not add");
        assert_eq!(
            scenarios.get(&id).unwrap().name,
            "modded",
            "later scenario must win"
        );
    }

    /// A later bundle (a mod) overlays an earlier bundle (the base) by id:
    /// last-wins across bundles, with a fresh section left added. No conflicts -
    /// same id in DIFFERENT bundles is the intended overlay, not an error.
    #[test]
    fn merge_bundles_overlays_later_bundle_by_id() {
        let base = vec![
            Content::Section(section("hull", 100.0)),
            Content::Section(section("thruster", 50.0)),
        ];
        let modded = vec![
            // Overrides the base hull by id.
            Content::Section(section("hull", 999.0)),
            // Adds a brand-new section.
            Content::Section(section("shield", 25.0)),
        ];

        let outcome = merge_bundles([base.iter(), modded.iter()]);

        assert!(
            outcome.conflicts.is_empty(),
            "same id across bundles is overlay, not a conflict: {:?}",
            outcome.conflicts
        );
        // hull overlaid in place (order preserved), thruster kept, shield appended.
        assert_eq!(
            outcome
                .sections
                .iter()
                .map(|s| s.base.id.as_str())
                .collect::<Vec<_>>(),
            vec!["hull", "thruster", "shield"]
        );
        assert_eq!(
            outcome.sections[0].base.health, 999.0,
            "the mod's hull must win over the base's"
        );
    }

    /// Dependency order drives the merge (task 20260715-142931): a DEPENDENT
    /// mod overlays its DEPENDENCY, so the topological order (dependency before
    /// dependent) must merge the dependent LAST even when it comes FIRST in
    /// catalog order. This is `register_bundles`'s ordering step
    /// (`topological_order` + `merge_bundles`) in miniature; without the topo
    /// reorder the merge would keep catalog order and the dependency would
    /// wrongly win.
    #[test]
    fn dependency_order_merges_a_dependent_after_its_dependency() {
        use nova_mod_format::deps::{topological_order, DepGraph};

        // `dependent` (id "mod") overrides the `hull` section that `dependency`
        // (id "dep") defines. Catalog/input order lists the dependent FIRST.
        let dependency = vec![Content::Section(section("hull", 100.0))];
        let dependent = vec![Content::Section(section("hull", 999.0))];
        let bundles: HashMap<&str, &Vec<Content>> =
            HashMap::from([("dep", &dependency), ("mod", &dependent)]);
        let ids = vec!["mod".to_string(), "dep".to_string()];
        let graph: DepGraph = HashMap::from([("mod".to_string(), vec!["dep".to_string()])]);

        let topo = topological_order(&ids, &graph);
        assert!(!topo.cycle);
        assert_eq!(topo.order, vec!["dep".to_string(), "mod".to_string()]);

        let outcome = merge_bundles(topo.order.iter().map(|id| bundles[id.as_str()].iter()));
        assert_eq!(outcome.sections.len(), 1);
        assert_eq!(
            outcome.sections[0].base.health, 999.0,
            "the dependent overlays its dependency regardless of catalog order"
        );
    }

    /// The SAME id twice within ONE bundle is a conflict: the first is kept, the
    /// duplicate is skipped and recorded. This is the "intra-bundle duplicate is
    /// an error" rule (surfaced loudly by the caller), distinct from cross-bundle
    /// overlay.
    #[test]
    fn merge_bundles_intra_bundle_duplicate_is_a_conflict() {
        let bundle = vec![
            Content::Section(section("hull", 100.0)),
            // Duplicate id in the SAME bundle - a conflict, not an overlay.
            Content::Section(section("hull", 999.0)),
        ];

        let outcome = merge_bundles([bundle.iter()]);

        assert_eq!(outcome.sections.len(), 1, "the duplicate must be skipped");
        assert_eq!(
            outcome.sections[0].base.health, 100.0,
            "the FIRST occurrence is kept within a bundle"
        );
        assert_eq!(outcome.conflicts.len(), 1, "the conflict is recorded");
        assert!(
            outcome.conflicts[0].contains("hull"),
            "the conflict names the offending id: {}",
            outcome.conflicts[0]
        );
    }
}
