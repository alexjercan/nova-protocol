//! The installed-mod SET: the shipped catalog (`InstalledCatalog`) and the
//! downloaded cache composed into the player-facing [`ModCatalog`] rows, plus
//! the [`EnabledMods`] selection that decides which bundles the merge sees.
//! `crate::merge::register_bundles` consumes what this module publishes.

/// Glob-import surface: `use nova_assets::mod_set::prelude::*` re-exports the
/// public API of this module. The two platform halves are gated here as well as
/// at their definitions, so a caller globbing the prelude gets exactly the set
/// that compiles for its target.
pub mod prelude {
    #[cfg(not(target_arch = "wasm32"))]
    pub use super::load_downloaded_mods;
    pub use super::{
        build_mod_catalog, installed_set_changed, load_enabled_mods,
        mark_downloaded_bundles_loaded, save_enabled_mods, seed_enabled_mods, DownloadedMod,
        DownloadedMods, EnabledMods, ModCatalog, ModInfo,
    };
    #[cfg(target_arch = "wasm32")]
    pub use super::{poll_mod_cache_hydration, start_mod_cache_hydration, ModCacheHydration};
}

use std::collections::HashSet;

use bevy::prelude::*;
use nova_modding::prelude::{BundleAsset, InstalledCatalog, ModEntry, ModMeta};

use crate::{collections::GameAssets, mod_cache, mod_prefs};

/// handle for its bundle, loaded from the `mods://` source
/// (`mods://<id>/<bundle>`) through the same loaders as a shipped bundle.
#[derive(Clone, Debug)]
pub struct DownloadedMod {
    /// The cache-index record (id, version, bundle path).
    pub record: mod_cache::InstalledModRecord,
    /// The bundle handle, held here so the asset stays alive while installed.
    pub bundle: Handle<BundleAsset>,
}

/// The DOWNLOADED half of the installed set, in cache-index order - the runtime
/// view of `mod_cache::read_index` with each record's bundle loading via
/// `mods:/`. The shipped half stays the `InstalledCatalog` asset.
///
/// Filled at startup (natively straight from the index; on the web after the
/// IndexedDB hydration task completes) and mutated by the future
/// install/uninstall flow. `build_mod_catalog` appends these as player-facing
/// rows and `register_bundles` merges the ENABLED ones after the shipped
/// bundles; both re-run when this resource changes, and
/// [`mark_downloaded_bundles_loaded`] flags a change when a bundle's async load
/// completes so a mod never stays merged-out just because it loaded late.
///
/// Downloaded mods install DISABLED: nothing here touches [`EnabledMods`], so a
/// fresh install only renders a row until the player toggles it on.
#[derive(Resource, Clone, Debug, Default)]
pub struct DownloadedMods(pub Vec<DownloadedMod>);

/// The set of ENABLED mod ids (catalog entry ids). `register_bundles` merges only
/// the cataloged bundles whose id is in this set, in catalog order.
///
/// Runtime state, NOT baked into any read-only asset: `seed_enabled_mods` fills
/// it from the catalog's `base` entries at startup (persistence, will load a
/// saved set instead), and the mods menu toggles ids in and out. It is
/// `Changed`-watched so a toggle re-runs the merge live.
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
        // included - one id space) is skipped, mirroring the portal generator's
        // no-shadowing rule; otherwise one toggle would drive two rows/bundles.
        // `register_bundles` skips the same records, so the pair stays
        // consistent.
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
/// The UNION keeps base enabled regardless of what `load_enabled_mods`
/// restored - base is locked on in the UI, so it must always be active - while
/// preserving any persisted or toggled non-base choices. The STRIP makes a
/// hidden (dev/tooling) mod's enablement SESSION-ONLY: without it, an example
/// run that enables a hidden mod persists the id, and a later normal run would
/// restore-and-merge a mod the menu has no row to disable. Examples
/// re-enable by id at `OnEnter(Loaded)`, after this chain, so they are
/// unaffected; the cleaned set is re-saved on the same change, so a polluted
/// prefs store self-heals. The `!base` guard keeps a pathological hidden+base
/// entry force-enabled. Runs at `OnEnter(Processing)`, after
/// `load_enabled_mods` and before the merge. Idempotent.
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

/// Restore the saved enabled-mods set at startup, if any.
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
/// load from the `mods:/` source. Shared by the native startup read and the
/// wasm post-hydration poll. Loading through the asset server here (not the
/// `GameAssets` collection) is deliberate: downloaded mods appear and disappear
/// at runtime, so they cannot sit behind the one-shot collection gate.
///
/// The on-disk index is DOWNLOADED input: a record whose id or bundle path
/// could escape the cache (a `..` component, an absolute path, a nested id) is
/// skipped with a warning before any asset path is built from it (the native
/// source is additionally sandboxed, since a malicious bundle MANIFEST can
/// request an escaping path without touching the index).
fn start_downloaded_loads(
    records: Vec<mod_cache::InstalledModRecord>,
    asset_server: &AssetServer,
    downloaded: &mut DownloadedMods,
) {
    // Ids key the enable set and the merge namespace, so a duplicate is not a
    // harmless repeat: two records under one id make "which bundle is this
    // mod" unanswerable, and downstream it reads as a dependency cycle.
    let mut seen_ids = std::collections::HashSet::new();
    downloaded.0 = records
        .into_iter()
        .filter_map(|record| {
            if !seen_ids.insert(record.id.clone()) {
                warn!(
                    "mod cache: skipping a second downloaded mod record for id '{}'",
                    record.id
                );
                return None;
            }
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
