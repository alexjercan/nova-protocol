//! The INSTALL/UNINSTALL half: validate a wire entry, stage every file in
//! memory under size/sha256 verification, commit the cache all-or-nothing,
//! and reverse all of it on uninstall. Also owns the stalled-fetch recovery.

use std::{
    collections::{HashMap, HashSet},
    sync::mpsc::Sender,
    time::Duration,
};

use bevy::{platform::time::Instant, prelude::*};
use nova_mod_format::PortalEntry;
use nova_modding::prelude::InstalledCatalog;

use super::{
    catalog::{RemoteCatalog, RemoteCatalogState},
    config::PortalConfig,
    transport::PortalClient,
    PortalChannel, PortalMsg,
};
use crate::{
    collections::GameAssets,
    mod_cache::{self, InstalledModRecord},
    mod_set::{DownloadedMods, EnabledMods},
};

/// Trigger: install the portal mod `id` (an entry of the Ready
/// [`RemoteCatalog`]). Progress/failure lands in [`InstallJobs`]; success
/// lands the mod in [`DownloadedMods`] (disabled).
#[derive(Event, Clone, Debug)]
pub struct InstallPortalMod {
    /// The portal entry's id.
    pub id: String,
}

/// Trigger: uninstall the downloaded mod `id` - files, index entry,
/// [`DownloadedMods`] record AND its [`EnabledMods`] entry (so a reinstall
/// starts disabled, like any fresh install).
#[derive(Event, Clone, Debug)]
pub struct UninstallPortalMod {
    /// The downloaded mod's id.
    pub id: String,
}

/// One install job's UI-visible stage. On native the non-`Fetching` stages
/// flip within a single frame (verification and the fs commit are
/// synchronous); on wasm `Committing` persists while the IndexedDB task runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallStatus {
    /// Downloading file `done + 1` of `total` (each finished file was already
    /// size/sha256-verified - failures surface as `Failed`, not a stall).
    Fetching {
        /// Files fetched AND verified so far.
        done: usize,
        /// Total files in the entry.
        total: usize,
    },
    /// The last file's integrity pass.
    Verifying,
    /// All files verified; the cache commit is in flight.
    Committing,
    /// The install failed; nothing was committed (message is
    /// user-presentable). Cleared by a retried [`InstallPortalMod`].
    Failed(String),
}

/// In-flight/failed install jobs by mod id. An entry is REMOVED on success -
/// [`DownloadedMods`] is then the truth; a `Failed` entry stays for the UI
/// until a retry replaces it.
#[derive(Resource, Clone, Debug, Default)]
pub struct InstallJobs(pub HashMap<String, InstallStatus>);

/// One staged install's private state: the (validated) portal entry driving
/// it and the verified files held in memory until the commit.
pub(super) struct ActiveInstall {
    pub(super) job: u64,
    pub(super) entry: PortalEntry,
    pub(super) files: Vec<(String, Vec<u8>)>,
    /// Last evidence the transport is alive (job start, then each verified
    /// file); [`timeout_wedged_fetches`] fails the job when this goes stale.
    pub(super) last_progress: Instant,
}

/// The staged installs by mod id, plus the monotonically increasing job
/// generation that lets stale transport callbacks be told apart from a
/// retry's (see [`PortalMsg`]).
#[derive(Resource, Default)]
pub(super) struct ActiveInstalls {
    pub(super) jobs: HashMap<String, ActiveInstall>,
    next_job: u64,
    /// dependency id -> the mods whose install pulled it in. A dependency's
    /// job is keyed under the DEPENDENCY, so without this reverse edge a
    /// failed dep leaves no surface on the row the player actually clicked.
    dependents: HashMap<String, HashSet<String>>,
}

impl ActiveInstalls {
    /// Record that `dependent`'s install pulled in `dep`.
    pub(super) fn pulled_in(&mut self, dep: &str, dependent: &str) {
        self.dependents
            .entry(dep.to_string())
            .or_default()
            .insert(dependent.to_string());
    }

    /// The mods that pulled `dep` in, dropping the edge as it is reported.
    fn dependents_of(&mut self, dep: &str) -> Vec<String> {
        let mut out: Vec<String> = self
            .dependents
            .remove(dep)
            .map(|set| set.into_iter().collect())
            .unwrap_or_default();
        out.sort();
        out
    }

    /// Register a fresh job for `entry` and return its generation.
    pub(super) fn begin(&mut self, entry: PortalEntry) -> u64 {
        let job = self.next_job;
        self.next_job += 1;
        self.jobs.insert(
            entry.id.clone(),
            ActiveInstall {
                job,
                entry,
                files: Vec::new(),
                last_progress: Instant::now(),
            },
        );
        job
    }
}

/// How long an install may sit in `Fetching` with NO transport progress (no
/// file completing) before [`timeout_wedged_fetches`] fails it - the recovery
/// for a transport callback that never fires. Progress resets the window, so a
/// slow-but-alive multi-file download never trips it.
///
/// Scoped to `Fetching` ON PURPOSE: `Committing` is a local commit (native:
/// synchronous fs; wasm: one awaited IndexedDB transaction) whose `Committed`
/// message carries no job generation - timing it out could race a late
/// success into "record in [`DownloadedMods`] plus a stale Failed entry"
/// (consistent, but confusing). Within `Fetching` no `Committed` can be in
/// flight, and late `File` callbacks are dropped by the active-entry/
/// generation guards, so a timeout-abort here is clean. Overridable as the
/// [`PortalFetchTimeout`] resource (tests shrink it to drive the real system).
const FETCH_STALL_TIMEOUT: Duration = Duration::from_secs(120);

/// The `FETCH_STALL_TIMEOUT` as a resource, so tests (and future settings)
/// can tune it without forking the system.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PortalFetchTimeout(pub Duration);

impl Default for PortalFetchTimeout {
    fn default() -> Self {
        Self(FETCH_STALL_TIMEOUT)
    }
}

/// Fail any install whose `Fetching` stage stalled past [`PortalFetchTimeout`]
/// (see [`FETCH_STALL_TIMEOUT`] for the scope rationale). The failed job
/// lands on the standard `Failed` surface - the menu's Retry/Dismiss.
pub(super) fn timeout_wedged_fetches(
    timeout: Res<PortalFetchTimeout>,
    mut jobs: ResMut<InstallJobs>,
    mut active: ResMut<ActiveInstalls>,
) {
    if active.jobs.is_empty() {
        return;
    }
    let now = Instant::now();
    let wedged: Vec<String> = active
        .jobs
        .iter()
        .filter(|(id, install)| {
            matches!(jobs.0.get(*id), Some(InstallStatus::Fetching { .. }))
                && now.saturating_duration_since(install.last_progress) > timeout.0
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in wedged {
        fail_install(
            &mut jobs,
            &mut active,
            &id,
            "timed out waiting for the portal".to_string(),
        );
    }
}

/// Ids whose uninstall FILE REMOVAL is still in flight. Only wasm ever fills it
/// (its removal is a detached IndexedDB task; native removal is synchronous):
/// an install admitted while the removal still runs could have its fresh writes
/// deleted under it, so `on_install_portal_mod` rejects those ids until the
/// task reports back through `PortalMsg::Removed`. Cfg-INDEPENDENT so the guard
/// itself is unit-tested natively. Pub (with the set readable) because the
/// menu's update choreography uses it as its second guard: an update's install
/// must not fire while the id's uninstall removal is still in flight.
#[derive(Resource, Default)]
pub struct PendingRemovals(pub HashSet<String>);

/// Anti-absurdity caps on a portal entry, NOT quotas: installs stage every
/// verified file in memory, so a hostile catalog must not be able to command
/// gigabytes of buffering (or tens of thousands of requests) before the
/// commit. Generous against any real mod - the whole shipped webmods set is
/// a few KiB. (A LYING server can still send an oversized body for one
/// request - ehttp buffers it before the size check rejects it - but these
/// caps bound what the catalog can make the client do by design.)
const MAX_FILE_SIZE: u64 = 32 * 1024 * 1024;
const MAX_FILE_COUNT: usize = 256;
const MAX_TOTAL_SIZE: u64 = 128 * 1024 * 1024;

/// The generator's PUBLISHED charset for the URL path segments an entry
/// contributes (`<id>/<version>/<path>`): lowercase ascii alphanumerics plus
/// `-` and `.` for ids/versions (`validate_id` in scripts/gen-portal.py is even
/// tighter - no dots - but versions like `1.0.0` need them). Never a
/// dot-only segment (`.`/`..`).
fn is_url_safe_segment(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
        && s.bytes().any(|b| b != b'.')
}

/// A relative file path whose every `/`-separated component is ascii
/// alphanumeric plus `-`/`_`/`.` and never dot-only. Notably NO `%`, `?`,
/// `#`, `\` or empty components anywhere.
fn is_url_safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.split('/').all(|component| {
            !component.is_empty()
                && component
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
                && component.bytes().any(|b| b != b'.')
        })
}

/// A portal entry is WIRE data - re-validate everything a fetch or a cache
/// path will be built from BEFORE the first request. TWO boundaries are
/// enforced, and only these two, precisely:
///
/// - LOCAL cache containment: the shared `is_safe_*` gates (plain `Path`
///   components only), the same rule the cache API re-checks at commit.
/// - URL charset: every segment must be in the generator's published charset
///   above. The `Path`-based check alone is NOT URL containment - a WHATWG-
///   conformant fetcher (the browser on wasm; many CDNs) percent-decodes
///   path segments, so a catalog component like `%2e%2e` is a plain `Normal`
///   component locally but a dot-dot segment on the wire, steering GETs
///   above the portal base (same-origin, and the bytes stay sha256-pinned,
///   but the request boundary would be a lie). Rejecting `%` (and `?`, `#`,
///   uppercase et al) outright keeps a segment's local and on-the-wire
///   meanings identical.
///
/// Plus the anti-absurdity caps above, a duplicate-path check (a duplicate
/// would double-count progress and self-overwrite in the staging buffer),
/// and the bundle-is-among-the-files invariant.
fn validate_entry(entry: &PortalEntry) -> Result<(), String> {
    if !mod_cache::is_safe_id(&entry.id) || !is_url_safe_segment(&entry.id) {
        return Err(format!("unsafe mod id '{}'", entry.id));
    }
    if !mod_cache::is_safe_id(&entry.version) || !is_url_safe_segment(&entry.version) {
        return Err(format!("unsafe version '{}'", entry.version));
    }
    if entry.files.is_empty() {
        return Err("the entry lists no files".to_string());
    }
    if entry.files.len() > MAX_FILE_COUNT {
        return Err(format!(
            "the entry lists {} files (max {MAX_FILE_COUNT})",
            entry.files.len()
        ));
    }
    let mut seen_paths = HashSet::new();
    let mut total: u64 = 0;
    for file in &entry.files {
        if !mod_cache::is_safe_rel_path(&file.path) || !is_url_safe_path(&file.path) {
            return Err(format!("unsafe file path '{}'", file.path));
        }
        if !seen_paths.insert(file.path.as_str()) {
            return Err(format!("duplicate file path '{}'", file.path));
        }
        if file.size > MAX_FILE_SIZE {
            return Err(format!(
                "file '{}' declares {} bytes (max {MAX_FILE_SIZE})",
                file.path, file.size
            ));
        }
        total = total.saturating_add(file.size);
    }
    if total > MAX_TOTAL_SIZE {
        return Err(format!(
            "the entry declares {total} bytes in total (max {MAX_TOTAL_SIZE})"
        ));
    }
    if !entry.files.iter().any(|f| f.path == entry.bundle) {
        return Err(format!(
            "bundle '{}' is not among the entry's files",
            entry.bundle
        ));
    }
    Ok(())
}

/// Record a failed install: drop the staged state, keep the reason for the UI.
pub(super) fn fail_install(
    jobs: &mut InstallJobs,
    active: &mut ActiveInstalls,
    id: &str,
    reason: String,
) {
    warn!("portal: install of '{id}' failed: {reason}");
    active.jobs.remove(id);
    // Key the failure under every mod that pulled this one in as well: their
    // installs may have succeeded, but they are not usable without it.
    for dependent in active.dependents_of(id) {
        jobs.0.insert(
            dependent,
            InstallStatus::Failed(format!("dependency '{id}' failed: {reason}")),
        );
    }
    jobs.0.insert(id.to_string(), InstallStatus::Failed(reason));
}

/// Kick the fetch of `entry.files[index]` for job `job`.
pub(super) fn fetch_file(
    config: &PortalConfig,
    client: &PortalClient,
    tx: &Sender<PortalMsg>,
    job: u64,
    entry: &PortalEntry,
    index: usize,
) {
    let url = config.file_url(&entry.id, &entry.version, &entry.files[index].path);
    let id = entry.id.clone();
    let tx = tx.clone();
    client.0.fetch(
        &url,
        Box::new(move |result| {
            let _ = tx.send(PortalMsg::File {
                job,
                id,
                index,
                result,
            });
        }),
    );
}

/// Validate + guard an install request, then start its staged download.
///
/// Rejections land as `Failed` in [`InstallJobs`] (the UI's error surface):
/// no Ready catalog / unknown id, an entry failing [`validate_entry`], an id
/// already downloaded, or an id shadowing a SHIPPED catalog entry (the
/// portal generator refuses to publish those, but the catalog is wire data -
/// the client re-enforces the rule, mirroring the cache-side consumers). A
/// re-trigger while a job is live is ignored; a `Failed` entry is a retry.
pub(super) fn on_install_portal_mod(
    event: On<InstallPortalMod>,
    config: Res<PortalConfig>,
    client: Res<PortalClient>,
    channel: Res<PortalChannel>,
    remote: Res<RemoteCatalog>,
    downloaded: Res<DownloadedMods>,
    game_assets: Option<Res<GameAssets>>,
    catalogs: Res<Assets<InstalledCatalog>>,
    pending: Res<PendingRemovals>,
    mut jobs: ResMut<InstallJobs>,
    mut active: ResMut<ActiveInstalls>,
    mut commands: Commands,
) {
    let id = event.id.clone();
    if matches!(jobs.0.get(&id), Some(status) if !matches!(status, InstallStatus::Failed(_))) {
        warn!("portal: an install of '{id}' is already in flight; ignoring the re-trigger");
        return;
    }
    // A still-running uninstall removal (wasm's is async) would delete this
    // install's fresh writes; checked FIRST so the rejection does not depend
    // on any catalog state.
    if pending.0.contains(&id) {
        fail_install(
            &mut jobs,
            &mut active,
            &id,
            "the previous uninstall of this mod is still finishing; try again".to_string(),
        );
        return;
    }
    let RemoteCatalogState::Ready(catalog) = &remote.state else {
        fail_install(
            &mut jobs,
            &mut active,
            &id,
            "the portal catalog is not loaded".to_string(),
        );
        return;
    };
    let Some(entry) = catalog.entries.iter().find(|e| e.id == id) else {
        fail_install(
            &mut jobs,
            &mut active,
            &id,
            "the portal catalog has no such mod".to_string(),
        );
        return;
    };
    if let Err(reason) = validate_entry(entry) {
        fail_install(
            &mut jobs,
            &mut active,
            &id,
            format!("the portal entry is invalid: {reason}"),
        );
        return;
    }
    if downloaded.0.iter().any(|m| m.record.id == id) {
        fail_install(
            &mut jobs,
            &mut active,
            &id,
            "the mod is already installed".to_string(),
        );
        return;
    }
    // The no-shadowing rule needs the SHIPPED catalog; installs only happen
    // from the loaded game (the portal UI lives past the Loaded state), so
    // requiring it here is a conservative guard, not a real-flow limitation.
    let Some(shipped) = game_assets
        .as_ref()
        .and_then(|ga| catalogs.get(&ga.catalog))
    else {
        fail_install(
            &mut jobs,
            &mut active,
            &id,
            "the shipped mods catalog is not loaded yet".to_string(),
        );
        return;
    };
    if shipped.entries.iter().any(|e| e.decl.id == id) {
        fail_install(
            &mut jobs,
            &mut active,
            &id,
            "the id shadows a shipped mod".to_string(),
        );
        return;
    }

    // Dependency resolution: before installing, ensure every transitive
    // dependency is installed or pullable from the portal. The job is recorded
    // FIRST so a dependency CYCLE (the portal generator rejects one at publish,
    // but wire data could carry it) is broken by the in-flight guard at the top
    // of this handler when a dep re-triggers this id.
    let total = entry.files.len();
    let entry = entry.clone();
    jobs.0
        .insert(id.clone(), InstallStatus::Fetching { done: 0, total });

    let graph: nova_mod_format::deps::DepGraph = catalog
        .entries
        .iter()
        .map(|e| (e.id.clone(), e.meta.dependencies.clone()))
        .collect();
    let is_installed = |dep: &str| {
        downloaded.0.iter().any(|m| m.record.id == dep)
            || shipped.entries.iter().any(|e| e.decl.id == dep)
    };
    let needed = match nova_mod_format::deps::transitive_deps(&graph, &id) {
        Ok(needed) => needed,
        Err(e) => {
            fail_install(&mut jobs, &mut active, &id, e.to_string());
            return;
        }
    };
    // Fail fast if any transitive dependency is neither installed nor in the
    // portal (`base` is implicit and always shipped).
    for dep in &needed {
        if dep == "base" || is_installed(dep) {
            continue;
        }
        if !catalog.entries.iter().any(|e| &e.id == dep) {
            fail_install(
                &mut jobs,
                &mut active,
                &id,
                format!("dependency '{dep}' is not installed and not available in the portal"),
            );
            return;
        }
    }
    // Pull the missing dependencies; each resolves its own deps, and the
    // in-flight guard dedupes overlapping installs and breaks cycles. This mod
    // and its deps then download in PARALLEL - there is no join, so atomicity is
    // PER MOD (each mod's staged commit is all-or-nothing), NOT across the set.
    // If a dependency's download fails asynchronously it lands as its own
    // `Failed` job while this mod still installs; the gap is surfaced when the
    // player enables this mod (on_mod_toggle warns about the uninstalled dep),
    // not silently. A true atomic dependency-set install is a possible follow-up.
    for dep in &needed {
        if dep == "base" || is_installed(dep) {
            continue;
        }
        // Record the dependency under THIS mod's id too. The dep's own job is
        // keyed under the dep, so a failure there had no surface on the row the
        // player actually clicked.
        active.pulled_in(dep, &id);
        commands.trigger(InstallPortalMod { id: dep.clone() });
    }

    let job = active.begin(entry.clone());
    fetch_file(&config, &client, &channel.tx, job, &entry, 0);
}

/// Uninstall a DOWNLOADED mod: index entry first (the index must never point at
/// missing files), files second, then the runtime record, any stale `Failed`
/// job entry, and - resolving - the id's [`EnabledMods`] entry, so a reinstall
/// starts disabled like any fresh install (the existing change-gated save
/// system persists the strip). A mod whose install is still in flight has
/// nothing committed to uninstall; the trigger is ignored with a warning.
pub(super) fn on_uninstall_portal_mod(
    event: On<UninstallPortalMod>,
    mut jobs: ResMut<InstallJobs>,
    mut downloaded: ResMut<DownloadedMods>,
    mut enabled: ResMut<EnabledMods>,
    #[cfg(target_arch = "wasm32")] channel: Res<PortalChannel>,
    #[cfg(target_arch = "wasm32")] mut pending: ResMut<PendingRemovals>,
    #[cfg(target_arch = "wasm32")] dir: Option<Res<mod_cache::ModsSourceDir>>,
) {
    let id = event.id.as_str();
    if matches!(jobs.0.get(id), Some(status) if !matches!(status, InstallStatus::Failed(_))) {
        warn!("portal: '{id}' is still installing; nothing committed to uninstall");
        return;
    }
    if !downloaded.0.iter().any(|m| m.record.id == id) {
        warn!("portal: '{id}' is not an installed portal mod; nothing to uninstall");
        return;
    }

    mod_cache::remove_index_record(id);

    #[cfg(not(target_arch = "wasm32"))]
    if let Err(error) = mod_cache::remove_mod(id) {
        // The index entry is already gone, so the leftovers are orphans the
        // next install of this id overwrites - log, do not resurrect.
        warn!("portal: removing '{id}' files from the cache failed: {error}");
    }
    #[cfg(target_arch = "wasm32")]
    {
        // Async on wasm; the record/index are already dropped, so a late (or
        // failed) file removal only leaves harmless orphans. The shared
        // memory Dir must be evicted too - it serves mods:// until reload.
        // The id is HELD in PendingRemovals until the task reports back, so
        // a reinstall cannot write files this removal then deletes.
        pending.0.insert(id.to_string());
        let id = id.to_string();
        let tx = channel.tx.clone();
        let dir = dir.map(|d| d.0.clone());
        bevy::tasks::IoTaskPool::get()
            .spawn(async move {
                match mod_cache::remove_mod(&id).await {
                    Ok(keys) => {
                        if let Some(dir) = dir {
                            for key in keys {
                                dir.remove_asset(std::path::Path::new(&key));
                            }
                        }
                    }
                    Err(error) => {
                        warn!("portal: removing '{id}' files from IndexedDB failed: {error}");
                    }
                }
                let _ = tx.send(PortalMsg::Removed { id });
            })
            .detach();
    }

    downloaded.0.retain(|m| m.record.id != id);
    // A leftover Failed job entry must not outlive the mod it describes.
    jobs.0.remove(id);
    // The contains guard keeps an uninstall of a disabled mod from marking
    // EnabledMods changed (a spurious re-merge + prefs re-save).
    if enabled.0.contains(id) {
        enabled.0.remove(id);
    }
}

/// Commit a fully-verified install into the cache and report through the
/// channel. Native: synchronous `install_local` (files first, index last);
/// the `Committed` message is picked up by the SAME poll-system drain that
/// called this, so the finalize logic stays in one place across platforms.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn start_commit(tx: &Sender<PortalMsg>, install: ActiveInstall, _dir: ()) {
    let ActiveInstall { entry, files, .. } = install;
    let record = InstalledModRecord {
        id: entry.id,
        version: entry.version,
        bundle: entry.bundle,
    };
    let result = mod_cache::install_local(&record.id, &record.version, &record.bundle, &files)
        .map_err(|e| e.to_string());
    if result.is_err() {
        // Nothing may survive a failed commit: sweep any partially-stored
        // files (best-effort; the index entry was the LAST write, so a
        // failure before it leaves no record either way).
        if let Err(error) = mod_cache::remove_mod(&record.id) {
            warn!(
                "portal: cleaning up '{}' after a failed commit also failed: {error}",
                record.id
            );
        }
    }
    let _ = tx.send(PortalMsg::Committed { record, result });
}

/// Commit a fully-verified install into the cache and report through the
/// channel. Wasm: an IoTaskPool task awaits the single IndexedDB transaction to
/// its COMMIT, then writes the index and inserts the files into the shared
/// `mods:/` memory `Dir` (the reader the bundle load will hit - the startup
/// hydrator only runs once, so a live install must feed the Dir itself).
#[cfg(target_arch = "wasm32")]
pub(super) fn start_commit(
    tx: &Sender<PortalMsg>,
    install: ActiveInstall,
    dir: Option<bevy::asset::io::memory::Dir>,
) {
    let tx = tx.clone();
    bevy::tasks::IoTaskPool::get()
        .spawn(async move {
            let ActiveInstall { entry, files, .. } = install;
            let record = InstalledModRecord {
                id: entry.id,
                version: entry.version,
                bundle: entry.bundle,
            };
            let result = match mod_cache::commit_mod_files(&record.id, &files).await {
                Ok(()) => {
                    mod_cache::upsert_index_record(record.clone());
                    if let Some(dir) = &dir {
                        for (path, bytes) in files {
                            let key = format!("{}/{path}", record.id);
                            dir.insert_asset(std::path::Path::new(&key), bytes);
                        }
                    }
                    Ok(())
                }
                Err(error) => {
                    // The transaction rolled back as a unit; nothing to sweep
                    // beyond being explicit that the install failed.
                    Err(error)
                }
            };
            let _ = tx.send(PortalMsg::Committed { record, result });
        })
        .detach();
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nova_mod_format::{ModMeta, PortalFile};

    use super::*;
    use crate::portal::transport::{FetchResult, PortalTransport};

    fn entry(id: &str, version: &str, bundle: &str, paths: &[&str]) -> PortalEntry {
        PortalEntry {
            id: id.to_string(),
            version: version.to_string(),
            bundle: bundle.to_string(),
            meta: ModMeta::default(),
            files: paths
                .iter()
                .map(|p| PortalFile {
                    path: p.to_string(),
                    size: 1,
                    sha256: "00".repeat(32),
                })
                .collect(),
            total_size: paths.len() as u64,
        }
    }

    /// The pre-fetch gate over wire data: escaping ids/versions/paths, a
    /// bundle outside the file list, and duplicate file paths are rejected
    /// before any URL is built. Deleting `validate_entry`'s checks would let
    /// a hostile catalog reach the fetch/commit stages with cache-escaping
    /// paths (or double-count staged progress).
    #[test]
    fn validate_entry_rejects_hostile_catalog_data() {
        let ok = entry("pack", "1.0.0", "pack.bundle.ron", &["pack.bundle.ron"]);
        assert!(validate_entry(&ok).is_ok());

        let bad_id = entry("../pack", "1.0.0", "pack.bundle.ron", &["pack.bundle.ron"]);
        assert!(validate_entry(&bad_id).is_err(), "escaping id");
        let bad_version = entry("pack", "../1.0.0", "pack.bundle.ron", &["pack.bundle.ron"]);
        assert!(validate_entry(&bad_version).is_err(), "escaping version");
        let bad_file = entry("pack", "1.0.0", "pack.bundle.ron", &["../evil.ron"]);
        assert!(validate_entry(&bad_file).is_err(), "escaping file path");
        let no_files = entry("pack", "1.0.0", "pack.bundle.ron", &[]);
        assert!(validate_entry(&no_files).is_err(), "no files");
        let stray_bundle = entry("pack", "1.0.0", "other.bundle.ron", &["pack.bundle.ron"]);
        assert!(
            validate_entry(&stray_bundle).is_err(),
            "the bundle must be among the files"
        );
        let duplicate = entry(
            "pack",
            "1.0.0",
            "pack.bundle.ron",
            &["pack.bundle.ron", "pack.bundle.ron"],
        );
        assert!(
            validate_entry(&duplicate).is_err(),
            "duplicate file paths are rejected (review 163508 R1.6)"
        );
    }

    /// Review: the local `Path`-component gates are NOT URL containment - a
    /// WHATWG-conformant fetcher percent-decodes segments, so `%2e%2e` is a
    /// dot-dot on the wire while being a plain `Normal` component locally. The
    /// charset gate must reject any segment outside the generator's published
    /// alphabet BEFORE any fetch.
    #[test]
    fn validate_entry_rejects_percent_encoded_and_off_charset_segments() {
        let ok = entry("pack", "1.0.0", "pack.bundle.ron", &["pack.bundle.ron"]);
        assert!(validate_entry(&ok).is_ok());

        let encoded_version = entry("pack", "%2e%2e", "pack.bundle.ron", &["pack.bundle.ron"]);
        assert!(
            validate_entry(&encoded_version).is_err(),
            "a percent-encoded dot-dot version must be rejected before any fetch"
        );
        let encoded_id = entry("%2e%2e", "1.0.0", "pack.bundle.ron", &["pack.bundle.ron"]);
        assert!(validate_entry(&encoded_id).is_err(), "encoded id");
        let encoded_path = entry(
            "pack",
            "1.0.0",
            "pack.bundle.ron",
            &["pack.bundle.ron", "%2e%2e/evil.ron"],
        );
        assert!(validate_entry(&encoded_path).is_err(), "encoded file path");
        let query_path = entry(
            "pack",
            "1.0.0",
            "pack.bundle.ron",
            &["pack.bundle.ron", "a?b.ron"],
        );
        assert!(validate_entry(&query_path).is_err(), "query metacharacter");
        let uppercase_id = entry("Pack", "1.0.0", "pack.bundle.ron", &["pack.bundle.ron"]);
        assert!(
            validate_entry(&uppercase_id).is_err(),
            "ids/versions hold to the generator's lowercase charset"
        );
        // Sanity: mixed-case FILE paths stay allowed (only ids/versions are
        // lowercase-bound; file names are authored).
        let mixed_file = entry(
            "pack",
            "1.0.0",
            "pack.bundle.ron",
            &["pack.bundle.ron", "shots/Screen_1.png"],
        );
        assert!(validate_entry(&mixed_file).is_ok());
    }

    /// Review: the anti-absurdity caps - a catalog cannot make the client stage
    /// absurd amounts of memory (or requests). One entry per cap: per-file
    /// size, file count, summed declared size.
    #[test]
    fn validate_entry_enforces_the_staging_caps() {
        let mut oversized_file = entry("pack", "1.0.0", "pack.bundle.ron", &["pack.bundle.ron"]);
        oversized_file.files[0].size = MAX_FILE_SIZE + 1;
        assert!(
            validate_entry(&oversized_file).is_err(),
            "a single file over the cap is rejected"
        );

        let many_paths: Vec<String> = (0..=MAX_FILE_COUNT).map(|i| format!("f{i}.ron")).collect();
        let mut many_refs: Vec<&str> = many_paths.iter().map(String::as_str).collect();
        many_refs[0] = "pack.bundle.ron";
        let too_many = entry("pack", "1.0.0", "pack.bundle.ron", &many_refs);
        assert!(
            validate_entry(&too_many).is_err(),
            "more files than the cap is rejected"
        );

        let mut total_blown = entry(
            "pack",
            "1.0.0",
            "pack.bundle.ron",
            &["pack.bundle.ron", "a.ron", "b.ron", "c.ron", "d.ron"],
        );
        // Each file stays under the per-file cap so ONLY the total trips
        // (5 x (32 MiB - 1) > 128 MiB).
        for file in &mut total_blown.files {
            file.size = MAX_FILE_SIZE - 1;
        }
        assert!(
            validate_entry(&total_blown).is_err(),
            "a summed declared size over the total cap is rejected"
        );
    }

    /// Review: an install for an id whose uninstall file-removal is still in
    /// flight (wasm's is a detached task) is rejected before anything else - a
    /// fresh write could be deleted under it. The guard and resource are
    /// cfg-independent (only wasm ever fills the set), so this native test pins
    /// the exact production observer.
    #[test]
    fn install_is_rejected_while_an_uninstall_removal_is_pending() {
        /// A transport that must never be reached: the pending guard fires
        /// before any catalog/fetch logic.
        struct NeverTransport;
        impl PortalTransport for NeverTransport {
            fn fetch(&self, url: &str, _: Box<dyn FnOnce(FetchResult) + Send>) {
                panic!("no fetch may happen while a removal is pending (got {url})");
            }
        }

        let mut app = App::new();
        app.insert_resource(PortalConfig {
            base_url: "http://portal.test".to_string(),
        });
        app.insert_resource(PortalClient(Arc::new(NeverTransport)));
        app.init_resource::<PortalChannel>();
        app.init_resource::<RemoteCatalog>();
        app.init_resource::<InstallJobs>();
        app.init_resource::<ActiveInstalls>();
        app.init_resource::<PendingRemovals>();
        app.init_resource::<DownloadedMods>();
        app.insert_resource(Assets::<InstalledCatalog>::default());
        app.add_observer(on_install_portal_mod);

        app.world_mut()
            .resource_mut::<PendingRemovals>()
            .0
            .insert("pack".to_string());
        app.world_mut().trigger(InstallPortalMod {
            id: "pack".to_string(),
        });

        match app.world().resource::<InstallJobs>().0.get("pack") {
            Some(InstallStatus::Failed(reason)) => assert!(
                reason.contains("uninstall"),
                "the rejection names the pending uninstall: {reason}"
            ),
            other => panic!("the install must fail while the removal is pending, got {other:?}"),
        }

        // Once the removal reports back (the Removed message path on wasm),
        // the id clears and a retry passes THIS guard (it then fails later,
        // on the empty catalog - proving the pending rejection is gone).
        app.world_mut()
            .resource_mut::<PendingRemovals>()
            .0
            .remove("pack");
        app.world_mut().trigger(InstallPortalMod {
            id: "pack".to_string(),
        });
        match app.world().resource::<InstallJobs>().0.get("pack") {
            Some(InstallStatus::Failed(reason)) => assert!(
                reason.contains("catalog"),
                "with the removal settled the guard no longer fires: {reason}"
            ),
            other => panic!("expected the next guard's failure, got {other:?}"),
        }
    }
}
