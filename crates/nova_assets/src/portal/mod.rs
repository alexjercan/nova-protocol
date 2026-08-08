//! The PORTAL CLIENT: fetch the static mod portal's `catalog.json` and
//! install/uninstall portal mods over the wire, on native and wasm - the
//! network half that fills the local mod cache (142906).
//!
//! No UI lives here. The mods menu binds to the EVENT/RESOURCE API only:
//! trigger [`FetchPortalCatalog`] / [`InstallPortalMod`] /
//! [`UninstallPortalMod`], read [`RemoteCatalog`] and [`InstallJobs`]. The
//! transport sits behind the [`PortalTransport`] trait ([`EhttpTransport`] in
//! production - one `ehttp` call surface for the native ureq thread and the
//! browser fetch), swapped by tests via the [`PortalClient`] resource.
//!
//! Flow shape (the mod_cache hydration idiom): observers KICK transport
//! fetches whose completion callbacks post `PortalMsg`s into a channel; the
//! `poll_portal_messages` Update system consumes them and advances the
//! state machines. Everything the wire returns is UNTRUSTED: the catalog is
//! schema-version-gated before it is trusted at all, and every id/path of an
//! entry passes the shared `mod_cache` safety gates BEFORE the first byte of
//! it is fetched.
//!
//! INSTALLS ARE STAGED: every file is fetched sequentially (per-file progress
//! for the UI), verified against the catalog's size + sha256 as it arrives
//! (fail fast - a corrupt first file stops a ten-file download), and held in
//! memory; only after the LAST file verifies does the commit write the cache
//! (files first, index last - `install_local` natively; on wasm one IndexedDB
//! transaction awaited to its `complete` event per, then the index). A failure
//! at ANY stage leaves the cache without the mod: no files, no index entry. On
//! success the record joins [`DownloadedMods`] and the EXISTING load/mark/merge
//! machinery takes over; installs stay DISABLED until the player enables them.
//!
//! UNINSTALL reverses install and also strips the id from
//! [`EnabledMods`](crate::EnabledMods) (persisted by the existing save system),
//! resolving: a reinstall starts disabled, matching the documented install
//! default.
//!
//! WEDGE RECOVERY (note): an install whose transport callback never fires is
//! failed by `timeout_wedged_fetches` once its `Fetching` stage stalls past
//! [`PortalFetchTimeout`] (progress resets the window; `Committing` is a LOCAL
//! commit and is deliberately not timed out - see the constant's doc), landing
//! it on the standard `Failed` surface the menu answers with Retry/Dismiss. A
//! wedged CATALOG fetch has no client-side timeout; the menu's Retry affordance
//! force-resets the state before re-triggering.

/// Glob-import surface: `use nova_assets::portal::prelude::*` re-exports the
/// public API of this module - the event/resource surface the mods menu binds
/// to, plus the transport seam tests swap.
pub mod prelude {
    pub use super::{
        EhttpTransport, FetchPortalCatalog, FetchResult, InstallJobs, InstallPortalMod,
        InstallStatus, PendingRemovals, PortalClient, PortalConfig, PortalFetchTimeout,
        PortalPlugin, PortalTransport, RemoteCatalog, RemoteCatalogState, UninstallPortalMod,
        DEFAULT_PORTAL_URL,
    };
}

mod catalog;
mod config;
mod install;
mod transport;

use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};

use bevy::{platform::time::Instant, prelude::*};
use catalog::{decode_catalog, decode_last_good, last_good_store, on_fetch_portal_catalog};
pub use catalog::{FetchPortalCatalog, RemoteCatalog, RemoteCatalogState};
pub use config::{PortalConfig, DEFAULT_PORTAL_URL};
use install::{
    fail_install, fetch_file, on_install_portal_mod, on_uninstall_portal_mod, start_commit,
    timeout_wedged_fetches, ActiveInstalls,
};
pub use install::{
    InstallJobs, InstallPortalMod, InstallStatus, PendingRemovals, PortalFetchTimeout,
    UninstallPortalMod,
};
use sha2::{Digest, Sha256};
pub use transport::{EhttpTransport, FetchResult, PortalClient, PortalTransport};

use crate::{
    mod_cache::{self, InstalledModRecord},
    mod_set::{DownloadedMod, DownloadedMods},
};

/// A completed transport call, posted from its callback into the channel the
/// poll system drains. `job` is the install-job generation that sent the
/// request - a stale callback (from a job that already failed and was
/// retried) must not feed the successor.
pub(super) enum PortalMsg {
    /// The catalog body (or fetch error).
    Catalog(FetchResult),
    /// File `index` of install job `job` for mod `id`.
    File {
        job: u64,
        id: String,
        index: usize,
        result: FetchResult,
    },
    /// An install job finished its cache commit. No `job` generation needed:
    /// a commit is only ever in flight while its id's status is `Committing`,
    /// which blocks any retry until this message lands - there is no
    /// same-id successor to confuse it with.
    Committed {
        record: InstalledModRecord,
        result: Result<(), String>,
    },
    /// An uninstall's ASYNC file removal settled (wasm only - native removal
    /// is synchronous); clears the id from [`PendingRemovals`]. Sent on
    /// success AND failure: either way the removal task is no longer racing
    /// a reinstall's writes.
    #[cfg(target_arch = "wasm32")]
    Removed { id: String },
}

/// The callback -> poll-system bridge. `std::sync::mpsc` (no new dependency):
/// the senders live in `Send` callbacks, the receiver is drained single-file
/// by [`poll_portal_messages`] (the `Mutex` exists only to make the resource
/// `Sync`; it is never contended).
#[derive(Resource)]
#[allow(missing_docs)]
pub(super) struct PortalChannel {
    pub(super) tx: Sender<PortalMsg>,
    pub(super) rx: Mutex<Receiver<PortalMsg>>,
}

impl Default for PortalChannel {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            tx,
            rx: Mutex::new(rx),
        }
    }
}

/// Drain the transport channel and advance the state machines: catalog
/// results into [`RemoteCatalog`], file results through verify -> next fetch
/// -> commit, commit results into [`DownloadedMods`] / `Failed`. Runs every
/// frame; an empty channel is a cheap `try_recv` miss.
fn poll_portal_messages(
    channel: Res<PortalChannel>,
    config: Res<PortalConfig>,
    client: Res<PortalClient>,
    asset_server: Res<AssetServer>,
    mut remote: ResMut<RemoteCatalog>,
    mut jobs: ResMut<InstallJobs>,
    mut active: ResMut<ActiveInstalls>,
    mut downloaded: ResMut<DownloadedMods>,
    #[cfg(target_arch = "wasm32")] mut pending: ResMut<PendingRemovals>,
    #[cfg(target_arch = "wasm32")] dir: Option<Res<mod_cache::ModsSourceDir>>,
) {
    loop {
        // The guard is dropped per-iteration so a handler's inline send (the
        // native commit) is picked up by the NEXT recv of this same drain.
        let message = channel.rx.lock().unwrap().try_recv();
        let Ok(message) = message else {
            break;
        };
        match message {
            PortalMsg::Catalog(result) => {
                // Keep the raw body: what the last-good store persists is the
                // exact wire JSON, re-gated by decode_catalog at next startup.
                let raw = result.as_ref().ok().cloned();
                remote.state = decode_catalog(result);
                match &remote.state {
                    RemoteCatalogState::Ready(catalog) => {
                        remote.last_good = Some(catalog.clone());
                        if let Some(bytes) = &raw {
                            last_good_store::save(bytes);
                        }
                    }
                    RemoteCatalogState::Error(error) => warn!("portal: {error}"),
                    _ => {}
                }
            }
            PortalMsg::File {
                job,
                id,
                index,
                result,
            } => {
                let Some(install) = active.jobs.get_mut(&id) else {
                    continue; // stale callback of an abandoned job
                };
                if install.job != job || install.files.len() != index {
                    continue; // stale callback of a superseded job
                }
                // `install.files.len() != index` above bounds the callback
                // against progress so far, NOT against the entry's file list -
                // a wire-supplied index past its end used to panic here.
                let Some(expected) = install.entry.files.get(index).cloned() else {
                    fail_install(
                        &mut jobs,
                        &mut active,
                        &id,
                        format!("file callback index {index} is outside the entry's file list"),
                    );
                    continue;
                };
                let bytes = match result {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        fail_install(
                            &mut jobs,
                            &mut active,
                            &id,
                            format!("fetching '{}' failed: {error}", expected.path),
                        );
                        continue;
                    }
                };
                let last = index + 1 == install.entry.files.len();
                if last {
                    // The machine is in its final integrity pass; on wasm the
                    // UI can catch this stage, natively it flips same-frame.
                    jobs.0.insert(id.clone(), InstallStatus::Verifying);
                }
                if bytes.len() as u64 != expected.size {
                    fail_install(
                        &mut jobs,
                        &mut active,
                        &id,
                        format!(
                            "file '{}' size mismatch: got {} bytes, expected {}",
                            expected.path,
                            bytes.len(),
                            expected.size
                        ),
                    );
                    continue;
                }
                let digest = format!("{:x}", Sha256::digest(&bytes));
                if digest != expected.sha256 {
                    fail_install(
                        &mut jobs,
                        &mut active,
                        &id,
                        format!("file '{}' failed its sha256 check", expected.path),
                    );
                    continue;
                }
                // A verified file is fresh evidence the transport is alive;
                // the stall timeout window restarts.
                install.last_progress = Instant::now();
                install.files.push((expected.path, bytes));
                if last {
                    jobs.0.insert(id.clone(), InstallStatus::Committing);
                    let install = active
                        .jobs
                        .remove(&id)
                        .expect("the job was just mutated under this id");
                    #[cfg(not(target_arch = "wasm32"))]
                    start_commit(&channel.tx, install, ());
                    #[cfg(target_arch = "wasm32")]
                    start_commit(&channel.tx, install, dir.as_ref().map(|d| d.0.clone()));
                } else {
                    let total = install.entry.files.len();
                    let done = install.files.len();
                    jobs.0
                        .insert(id.clone(), InstallStatus::Fetching { done, total });
                    let entry = install.entry.clone();
                    fetch_file(&config, &client, &channel.tx, job, &entry, index + 1);
                }
            }
            PortalMsg::Committed { record, result } => match result {
                Ok(()) => {
                    // The job entry disappears on success: DownloadedMods is
                    // the truth from here, and the EXISTING load/mark/merge
                    // machinery reacts to this push. Installs stay disabled.
                    jobs.0.remove(&record.id);
                    let path = format!(
                        "{}://{}/{}",
                        mod_cache::MODS_SOURCE,
                        record.id,
                        record.bundle
                    );
                    info!(
                        "portal: installed '{}' v{} ({} into the local cache)",
                        record.id, record.version, path
                    );
                    downloaded.0.push(DownloadedMod {
                        bundle: asset_server.load(path),
                        record,
                    });
                }
                Err(error) => {
                    jobs.0.insert(
                        record.id.clone(),
                        InstallStatus::Failed(format!("cache commit failed: {error}")),
                    );
                    warn!("portal: install of '{}' failed: {error}", record.id);
                }
            },
            #[cfg(target_arch = "wasm32")]
            PortalMsg::Removed { id } => {
                // The uninstall's file removal settled; installs of this id
                // are admitted again.
                pending.0.remove(&id);
            }
        }
    }
}

/// The portal client's wiring: config + transport + state resources, the
/// three trigger observers, and the channel poll. Added by `GameAssetsPlugin`;
/// test rigs add it directly and then swap [`PortalClient`]/[`PortalConfig`].
pub struct PortalPlugin;

impl Plugin for PortalPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PortalConfig::from_environment());
        app.insert_resource(PortalClient(Arc::new(EhttpTransport)));
        app.init_resource::<PortalChannel>();
        // The startup load of the last-good catalog (the offline fallback),
        // schema-gated again: a store from an older build must not smuggle an
        // unsupported schema past decode_catalog.
        app.insert_resource(RemoteCatalog {
            state: RemoteCatalogState::Idle,
            last_good: last_good_store::load().and_then(decode_last_good),
        });
        app.init_resource::<InstallJobs>();
        app.init_resource::<ActiveInstalls>();
        app.init_resource::<PendingRemovals>();
        app.init_resource::<PortalFetchTimeout>();
        app.add_observer(on_fetch_portal_catalog);
        app.add_observer(on_install_portal_mod);
        app.add_observer(on_uninstall_portal_mod);
        app.add_systems(
            Update,
            (poll_portal_messages, timeout_wedged_fetches).chain(),
        );
    }
}
