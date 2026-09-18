//! Safe mode: the optional half of the installed set, loaded OUTSIDE the boot
//! collection, and what happens to a mod whose content will not load.
//!
//! The rule this module exists for: a mod the player merely has installed must
//! never be able to stop the game from starting. Only the base game is
//! mandatory. Everything else - a shipped optional mod from
//! `mods.catalog.ron`, a mod downloaded from the portal - loads through the
//! asset server on its own, and a failure disables that mod, says so once, and
//! leaves the files where they are.
//!
//! Ownership: `nova_modding`'s [`CatalogLoader`](nova_modding::prelude::CatalogLoader)
//! decides what is mandatory (it loads a handle only for `base`); this module
//! owns everything the catalog did not load, and the quarantine that reads the
//! outcome.

/// Glob-import surface: `use nova_assets::safe_mode::prelude::*` re-exports the
/// public API of this module.
pub mod prelude {
    pub use super::{
        catalog_bundle, clear_quarantine, optional_loads_settled, quarantine_failed_mods,
        start_optional_loads, DisabledMod, FatalAssetFailure, ModQuarantine, OptionalBundle,
        OptionalBundles,
    };
}

use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    prelude::*,
};
use nova_modding::prelude::{BundleAsset, CatalogEntry, InstalledCatalog};

use crate::{
    collections::GameAssets,
    mod_set::{DownloadedMods, EnabledMods},
};

/// How long `Processing` waits for the optional bundles before going on without
/// them, seconds of [`Time<Real>`].
///
/// A bound, not a schedule: the local files these are normally read from settle
/// in a frame or two. What it protects against is a source that answers neither
/// way - a stalled network mount, a web source waiting on a request nobody will
/// answer - which would otherwise hold the loading screen forever. A bundle
/// that lands after the wait still merges, on the same change-driven re-merge a
/// downloaded mod uses.
const OPTIONAL_SETTLE_SECS: f32 = 30.0;

/// One optional cataloged mod's runtime bundle load.
#[derive(Clone, Debug)]
pub struct OptionalBundle {
    /// The catalog id this bundle belongs to.
    pub id: String,
    /// The handle, loaded through the asset server rather than the catalog, so
    /// it is nobody's recursive dependency.
    pub bundle: Handle<BundleAsset>,
}

/// The OPTIONAL half of the shipped catalog: one runtime load per entry the
/// catalog declared but did not load.
///
/// The shipped mirror of [`DownloadedMods`], and for the same reason - a mod
/// that is merely installed is not something the boot may wait on. Filled at
/// `OnEnter(Processing)` from the loaded catalog, and refilled on every content
/// restart.
#[derive(Resource, Clone, Debug, Default)]
pub struct OptionalBundles(pub Vec<OptionalBundle>);

impl OptionalBundles {
    /// The runtime-loaded bundle for `id`, if this is one of the optional mods.
    pub fn handle(&self, id: &str) -> Option<&Handle<BundleAsset>> {
        self.0
            .iter()
            .find(|loaded| loaded.id == id)
            .map(|loaded| &loaded.bundle)
    }
}

/// The bundle handle a catalog entry reads through: the catalog's own when it
/// loaded one, else the runtime load started for an optional entry.
///
/// Every consumer of the catalog goes through this, so "which bundle is this
/// mod" has one answer. A synthetic catalog built in a test may carry a handle
/// on any entry; a shipped one carries `base`'s only.
pub fn catalog_bundle<'a>(
    entry: &'a CatalogEntry,
    optional: &'a OptionalBundles,
) -> Option<&'a Handle<BundleAsset>> {
    entry
        .bundle
        .as_ref()
        .or_else(|| optional.handle(&entry.decl.id))
}

/// One mod that was switched off because its content would not load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisabledMod {
    /// The catalog / cache id - the enable key that was removed.
    pub id: String,
    /// Why, in one line a player can act on.
    pub reason: String,
}

/// The quarantine: what safe mode switched off, and whether the player has been
/// told yet.
///
/// One EPISODE is one recovery: everything that failed between a boot (or a
/// content restart) and the report the player acknowledges. The report is owed
/// once per episode - a mod that stays disabled is a row in the Mods screen
/// from then on, not a modal on every launch.
#[derive(Resource, Clone, Debug, Default)]
pub struct ModQuarantine {
    /// Every mod disabled in this episode, in the order it failed.
    pub disabled: Vec<DisabledMod>,
    /// True while the aggregate report has not been acknowledged.
    pub report_pending: bool,
}

impl ModQuarantine {
    /// Record a failure, unless this id is already in the episode.
    fn record(&mut self, id: &str, reason: String) -> bool {
        if self.disabled.iter().any(|m| m.id == id) {
            return false;
        }
        self.disabled.push(DisabledMod {
            id: id.to_string(),
            reason,
        });
        self.report_pending = true;
        true
    }
}

/// The verdict on a boot that cannot continue: a MANDATORY asset did not load.
///
/// Inserted at `OnEnter(GameAssetsStates::Failed)` and read by the fatal screen
/// in `nova_core`. Present at all is the whole signal; the fields are what the
/// screen can say about it.
#[derive(Resource, Clone, Debug)]
pub struct FatalAssetFailure {
    /// Which collection did not resolve, in the words the screen shows.
    pub detail: String,
    /// Whether the BOOT collection is what failed - the UI font itself, which
    /// means the screen must draw in the engine's default face.
    pub boot: bool,
}

/// Kick the load of every optional cataloged bundle.
///
/// Runs at `OnEnter(Processing)`, where the catalog is loaded and its entries
/// are readable. A restart re-enters the state and refills the list, so a mod
/// installed since the last pass is picked up and one uninstalled is dropped.
pub fn start_optional_loads(
    asset_server: Res<AssetServer>,
    game_assets: Res<GameAssets>,
    catalogs: Res<Assets<InstalledCatalog>>,
    mut optional: ResMut<OptionalBundles>,
) {
    let Some(catalog) = catalogs.get(&game_assets.catalog) else {
        error!("start_optional_loads: the mods catalog was not loaded; no optional mod loads");
        return;
    };
    optional.0 = catalog
        .entries
        .iter()
        .filter(|entry| entry.bundle.is_none())
        .map(|entry| OptionalBundle {
            id: entry.decl.id.clone(),
            bundle: asset_server.load(entry.decl.bundle.clone()),
        })
        .collect();
}

/// Whether every optional cataloged bundle has settled - loaded, or failed.
///
/// The `Processing` gate. Downloaded mods are deliberately NOT waited on: they
/// come off a cache that the web target is still hydrating from IndexedDB when
/// this runs, and holding the boot on a network-installed mod is the failure
/// this whole module exists to prevent. They merge when they land, and safe
/// mode quarantines them there just the same.
pub fn optional_loads_settled(
    asset_server: Res<AssetServer>,
    optional: Res<OptionalBundles>,
    time: Res<Time<Real>>,
    mut waiting_since: Local<Option<f32>>,
) -> bool {
    let started = *waiting_since.get_or_insert_with(|| time.elapsed_secs());
    let settled = optional
        .0
        .iter()
        .all(|loaded| settled_state(&asset_server, &loaded.bundle).is_some());
    if settled || time.elapsed_secs() - started >= OPTIONAL_SETTLE_SECS {
        *waiting_since = None;
        if !settled {
            warn!(
                "safe mode: optional mods have not finished loading after \
                 {OPTIONAL_SETTLE_SECS}s; continuing without them - they merge if they land"
            );
        }
        return true;
    }
    false
}

/// A settled load's verdict: `Some(Ok(()))` loaded, `Some(Err(reason))` failed,
/// `None` still in flight.
///
/// RECURSIVE, because a bundle whose manifest parsed but whose content file did
/// not is broken in exactly the way the player cares about, and the bundle's own
/// [`LoadState`] is `Loaded` in that case.
fn settled_state(
    asset_server: &AssetServer,
    bundle: &Handle<BundleAsset>,
) -> Option<Result<(), String>> {
    match asset_server.load_state(bundle) {
        LoadState::Failed(error) => return Some(Err(one_line(&error.to_string()))),
        LoadState::Loaded => {}
        LoadState::NotLoaded | LoadState::Loading => return None,
    }
    match asset_server.recursive_dependency_load_state(bundle) {
        RecursiveDependencyLoadState::Failed(error) => Some(Err(one_line(&error.to_string()))),
        RecursiveDependencyLoadState::Loaded => Some(Ok(())),
        RecursiveDependencyLoadState::NotLoaded | RecursiveDependencyLoadState::Loading => None,
    }
}

/// The first line of an asset error, capped.
///
/// A `BevyError` prints its whole backtrace, which is a developer's artifact:
/// the report this reason ends up in is a PLAYER's, and the first line is the
/// part that names the file and what was wrong with it.
fn one_line(error: &str) -> String {
    let first = error.lines().next().unwrap_or(error).trim();
    match first.char_indices().nth(REASON_CHARS) {
        Some((cut, _)) => format!("{}...", &first[..cut]),
        None => first.to_string(),
    }
}

/// How much of a failure reason the report carries. Long enough for the asset
/// path and the loader's complaint, short enough to stay one line of a modal.
const REASON_CHARS: usize = 160;

/// Disable every optional or downloaded mod whose bundle failed to load, and
/// remember why.
///
/// Runs in `Update` for the life of the app rather than once at boot: a
/// downloaded bundle finishes long after `Processing`, and an install done from
/// the Mods screen fails at the moment it is tried. Removing the id from
/// [`EnabledMods`] is what makes the next merge skip it, and the existing
/// change-gated `save_enabled_mods` is what writes the choice to disk - so the
/// mod is off on the next launch too, with its files still installed for the
/// player to update or remove.
pub fn quarantine_failed_mods(
    asset_server: Res<AssetServer>,
    optional: Res<OptionalBundles>,
    downloaded: Res<DownloadedMods>,
    mut enabled: ResMut<EnabledMods>,
    mut quarantine: ResMut<ModQuarantine>,
) {
    let shipped = optional
        .0
        .iter()
        .map(|loaded| (loaded.id.as_str(), &loaded.bundle));
    let cached = downloaded
        .0
        .iter()
        .map(|installed| (installed.record.id.as_str(), &installed.bundle));
    for (id, bundle) in shipped.chain(cached) {
        // ENABLED only. A broken mod the player has switched off is breaking
        // nothing, and reporting it would put the same modal in front of them
        // on every launch for as long as the files stay installed - which is the
        // one thing an episode-scoped report must not do. Switching it back on
        // is what asks the question again, and answers it.
        if !enabled.0.contains(id) {
            continue;
        }
        let Some(Err(reason)) = settled_state(&asset_server, bundle) else {
            continue;
        };
        if !quarantine.record(id, reason.clone()) {
            continue;
        }
        enabled.0.remove(id);
        // WARN, not ERROR: the game handled this. The run continues, the
        // player is told, and the level a harness treats as a failed run
        // belongs to the boot that cannot continue at all.
        warn!("safe mode: mod '{id}' is disabled because its content failed to load: {reason}");
    }
}

/// A content restart opens a new episode: what the last one disabled is on the
/// Mods screen now, not owed as a modal again.
///
/// `OnEnter(GameAssetsStates::Loading)` - every boot and every restart passes
/// through it, and the quarantine below is refilled from what this pass finds.
pub fn clear_quarantine(mut quarantine: ResMut<ModQuarantine>) {
    *quarantine = ModQuarantine::default();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quarantine_with(ids: &[&str]) -> ModQuarantine {
        let mut quarantine = ModQuarantine::default();
        for id in ids {
            quarantine.record(id, "broken".to_string());
        }
        quarantine
    }

    /// The reason a player reads is one line: a `BevyError`'s own `Display`
    /// carries its backtrace, which would put a stack trace in a modal.
    #[test]
    fn a_failure_reason_is_one_capped_line() {
        let reason = one_line(
            "Failed to load asset 'mods/x/x.bundle.ron': expected colon\n   0: from<Error>\n   1: more",
        );
        assert_eq!(
            reason,
            "Failed to load asset 'mods/x/x.bundle.ron': expected colon"
        );

        let long = "x".repeat(REASON_CHARS * 2);
        let capped = one_line(&long);
        assert_eq!(capped.chars().count(), REASON_CHARS + 3, "capped: {capped}");
        assert!(capped.ends_with("..."));
    }

    /// One episode reports each mod once. A bundle sits in `Failed` for every
    /// frame after it fails, so a quarantine that recorded on each of them
    /// would grow a list of repeats and re-arm the report the player just
    /// acknowledged.
    #[test]
    fn a_repeated_failure_is_recorded_once() {
        let mut quarantine = quarantine_with(&["broken", "broken", "other"]);
        assert_eq!(
            quarantine
                .disabled
                .iter()
                .map(|m| m.id.as_str())
                .collect::<Vec<_>>(),
            vec!["broken", "other"]
        );
        assert!(quarantine.report_pending);
        quarantine.report_pending = false;
        assert!(
            !quarantine.record("broken", "broken".to_string()),
            "an id already in the episode does not re-arm the report"
        );
        assert!(!quarantine.report_pending);
    }

    /// The lookup answers from the catalog's own handle first, so a synthetic
    /// catalog that carries handles on every entry (every merge test builds
    /// one) reads exactly as it did before optional mods moved out of the boot
    /// collection.
    #[test]
    fn a_catalog_handle_wins_over_the_runtime_load() {
        let carried = Handle::<BundleAsset>::default();
        let entry = CatalogEntry {
            decl: nova_modding::prelude::ModEntry {
                id: "art".to_string(),
                bundle: "mods/art/art.bundle.ron".to_string(),
                base: false,
            },
            bundle: Some(carried.clone()),
        };
        let optional = OptionalBundles(vec![OptionalBundle {
            id: "art".to_string(),
            bundle: Handle::default(),
        }]);
        assert_eq!(catalog_bundle(&entry, &optional), Some(&carried));

        let unloaded = CatalogEntry {
            bundle: None,
            ..entry
        };
        assert_eq!(
            catalog_bundle(&unloaded, &optional),
            optional.handle("art"),
            "an entry the catalog did not load reads through the runtime load"
        );
    }
}
