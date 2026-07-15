//! The PORTAL CLIENT (task 20260715-163508): fetch the static mod portal's
//! `catalog.json` and install/uninstall portal mods over the wire, on native
//! and wasm - the network half that fills the local mod cache (142906).
//!
//! No UI lives here. The mods menu (task 142916) binds to the EVENT/RESOURCE
//! API only: trigger [`FetchPortalCatalog`] / [`InstallPortalMod`] /
//! [`UninstallPortalMod`], read [`RemoteCatalog`] and [`InstallJobs`]. The
//! transport sits behind the [`PortalTransport`] trait ([`EhttpTransport`] in
//! production - one `ehttp` call surface for the native ureq thread and the
//! browser fetch), swapped by tests via the [`PortalClient`] resource.
//!
//! Flow shape (the mod_cache hydration idiom): observers KICK transport
//! fetches whose completion callbacks post [`PortalMsg`]s into a channel; the
//! [`poll_portal_messages`] Update system consumes them and advances the
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
//! transaction awaited to its `complete` event per review 142906 R1.4, then
//! the index). A failure at ANY stage leaves the cache without the mod: no
//! files, no index entry. On success the record joins [`DownloadedMods`] and
//! the EXISTING load/mark/merge machinery takes over; installs stay DISABLED
//! until the player enables them.
//!
//! UNINSTALL reverses install and also strips the id from [`EnabledMods`]
//! (persisted by the existing save system), resolving 142906's R1.7: a
//! reinstall starts disabled, matching the documented install default.
//!
//! WEDGE RECOVERY (task 142916, resolving 163508's R1.3 note): an install
//! whose transport callback never fires is failed by
//! [`timeout_wedged_fetches`] once its `Fetching` stage stalls past
//! [`PortalFetchTimeout`] (progress resets the window; `Committing` is a
//! LOCAL commit and is deliberately not timed out - see the constant's doc),
//! landing it on the standard `Failed` surface the menu answers with
//! Retry/Dismiss. A wedged CATALOG fetch has no client-side timeout; the
//! menu's Retry affordance force-resets the state before re-triggering.

use std::{
    collections::HashMap,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    time::Duration,
};

use bevy::{platform::time::Instant, prelude::*};
use nova_mod_format::{PortalCatalog, PortalEntry, PORTAL_SCHEMA_VERSION};
use nova_modding::prelude::InstalledCatalog;
use sha2::{Digest, Sha256};

use crate::{
    mod_cache::{self, InstalledModRecord},
    DownloadedMod, DownloadedMods, EnabledMods, GameAssets,
};

/// The production portal base URL - the GitHub Pages tree `nova_portal_gen`
/// publishes next to the wasm build (web/src/wiki/dev/mod-portal.md).
pub const DEFAULT_PORTAL_URL: &str = "https://alexjercan.github.io/nova-protocol/mods";

/// Where the portal lives: `<base_url>/catalog.json` +
/// `<base_url>/<id>/<version>/<files...>`.
///
/// Defaults per platform, resolved once at plugin build by
/// [`PortalConfig::from_environment`]:
/// - native: [`DEFAULT_PORTAL_URL`], overridable via the `NOVA_PORTAL_URL`
///   environment variable (dev/test builds point at localhost);
/// - wasm: derived from `window.location` (the game is served at
///   `<root>/play/`, the portal is its SIBLING `<root>/mods` - so a fork's
///   Pages deploy fetches its own portal with zero config), overridable via a
///   `?portal=<url>` query parameter.
#[derive(Resource, Clone, Debug)]
pub struct PortalConfig {
    /// The portal tree's base URL, no trailing slash required.
    pub base_url: String,
}

impl PortalConfig {
    /// Resolve the platform default + override chain described on the type.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_environment() -> Self {
        let base_url = std::env::var("NOVA_PORTAL_URL")
            .ok()
            .filter(|url| !url.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_PORTAL_URL.to_string());
        Self { base_url }
    }

    /// Resolve the platform default + override chain described on the type.
    #[cfg(target_arch = "wasm32")]
    pub fn from_environment() -> Self {
        let window = web_sys::window();
        let base_url = window
            .as_ref()
            .map(|window| {
                let location = window.location();
                if let Some(url) =
                    portal_override_from_query(&location.search().unwrap_or_default())
                {
                    return url;
                }
                portal_base_from_href(&location.href().unwrap_or_default())
            })
            .unwrap_or_else(|| DEFAULT_PORTAL_URL.to_string());

        // Proactive cross-origin heads-up. A CORS failure reaches JS as an
        // opaque `TypeError: Failed to fetch` (indistinguishable from a refused
        // connection), so the post-failure error cannot name it - but comparing
        // the resolved base origin to the page origin is a reliable signal we
        // have BEFORE the fetch. Fires only when the portal is pointed
        // cross-origin (a `?portal=` to another host); the same-origin default
        // never trips it.
        if let Some(window) = window.as_ref() {
            if let (Ok(page_origin), Some(base_origin)) =
                (window.location().origin(), url_origin(&base_url))
            {
                if base_origin != page_origin {
                    warn!(
                        "portal: base '{base_url}' is cross-origin to the page ({page_origin}); \
                         the browser will block the catalog/file fetch unless the portal sends an \
                         Access-Control-Allow-Origin header. For local dev, serve the portal \
                         same-origin instead (see mod-portal.md, \"Local development\")."
                    );
                }
            }
        }

        Self { base_url }
    }

    /// The catalog's URL: `<base>/catalog.json`.
    pub fn catalog_url(&self) -> String {
        join_url(&self.base_url, "catalog.json")
    }

    /// One mod file's URL: `<base>/<id>/<version>/<path>` (the tree layout
    /// `nova_portal_gen` writes).
    pub fn file_url(&self, id: &str, version: &str, path: &str) -> String {
        join_url(&self.base_url, &format!("{id}/{version}/{path}"))
    }
}

/// `<base>/<path>` with the trailing-slash seam normalized (a configured base
/// may or may not carry one).
fn join_url(base: &str, path: &str) -> String {
    format!("{}/{path}", base.trim_end_matches('/'))
}

/// The origin (`scheme://host[:port]`) of an absolute URL, or `None` when it
/// carries no `scheme://` (a relative base). Used to detect a cross-origin
/// portal config on wasm before the browser's opaque CORS failure. Pure and
/// cfg-independent for the native test pin, like the derivation fns below.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn url_origin(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    if host.is_empty() {
        return None;
    }
    Some(format!("{scheme}://{host}"))
}

/// Derive the portal base from the page's own URL: drop the query/fragment
/// and any document file name, step out of a trailing `/play` segment (the
/// deploy serves the game at `<root>/play/`, the portal at `<root>/mods/`),
/// and append `mods`. Pure and cfg-independent ON PURPOSE: the only caller is
/// wasm, but the native unit tests are what pin its behavior.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn portal_base_from_href(href: &str) -> String {
    let trimmed = href.split(['#', '?']).next().unwrap_or(href);
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return format!("{}/mods", trimmed.trim_end_matches('/'));
    };
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let mut segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    // A final dotted segment is the document (index.html), not a directory.
    if segments.last().is_some_and(|s| s.contains('.')) {
        segments.pop();
    }
    if segments.last() == Some(&"play") {
        segments.pop();
    }
    segments.push("mods");
    format!("{scheme}://{host}/{}", segments.join("/"))
}

/// The `?portal=<url>` dev override, from a `Location::search` string
/// (`?a=b&portal=...`). Percent/plus-decoded; empty values do not override.
/// Cfg-independent like [`portal_base_from_href`], for the native test pin.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn portal_override_from_query(search: &str) -> Option<String> {
    search
        .trim_start_matches('?')
        .split('&')
        .find_map(|pair| pair.strip_prefix("portal="))
        .map(percent_decode)
        .filter(|url| !url.is_empty())
}

/// Minimal application/x-www-form-urlencoded decode (`%XX` + `+` as space) -
/// enough for a URL-valued query parameter without a url-crate dependency.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    None => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A fetched body, or a human-readable failure (transport error or a non-2xx
/// status).
pub type FetchResult = Result<Vec<u8>, String>;

/// One-shot byte fetches, callback-completed - the whole transport surface
/// the portal client needs. Object-safe and `Send + Sync` so tests inject
/// mocks/failures through the [`PortalClient`] resource without touching the
/// state machines.
pub trait PortalTransport: Send + Sync + 'static {
    /// Fetch `url`; deliver the raw body (non-2xx statuses are `Err`) to
    /// `on_done` from whatever context the implementation completes on (a
    /// worker thread natively, the JS microtask queue on wasm - the callback
    /// must only post messages, never touch world state).
    fn fetch(&self, url: &str, on_done: Box<dyn FnOnce(FetchResult) + Send>);
}

/// The production transport: `ehttp` GETs (native: a ureq call on a spawned
/// thread; wasm: the browser `fetch` API) under the one cross-platform API.
pub struct EhttpTransport;

impl PortalTransport for EhttpTransport {
    fn fetch(&self, url: &str, on_done: Box<dyn FnOnce(FetchResult) + Send>) {
        ehttp::fetch(ehttp::Request::get(url), move |result| {
            on_done(match result {
                Ok(response) if response.ok => Ok(response.bytes),
                Ok(response) => Err(format!("HTTP {} {}", response.status, response.status_text)),
                Err(error) => Err(error),
            });
        });
    }
}

/// The swappable transport handle. Production inserts [`EhttpTransport`];
/// tests replace the resource with a mock after adding [`PortalPlugin`].
#[derive(Resource, Clone)]
pub struct PortalClient(pub Arc<dyn PortalTransport>);

/// Trigger: (re)fetch the portal catalog. Result lands in [`RemoteCatalog`].
#[derive(Event, Clone, Debug)]
pub struct FetchPortalCatalog;

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

/// The fetched portal catalog: the fetch state machine the Explore UI renders
/// plus the LAST-GOOD catalog (task 142916's offline fallback). State
/// transitions never clear `last_good`: a successful fetch refreshes it (and
/// persists the raw body through [`last_good_store`]), startup loads it back,
/// and every failure leaves it standing for the UI's stale rendering.
#[derive(Resource, Clone, Debug, Default)]
pub struct RemoteCatalog {
    /// The current fetch's state.
    pub state: RemoteCatalogState,
    /// The most recent catalog that passed the schema gate - this session's
    /// or, via the persisted store, a previous run's. `None` until a first
    /// successful fetch ever.
    pub last_good: Option<PortalCatalog>,
}

/// The catalog fetch's state machine - what the Explore UI renders. Entries
/// keep the catalog's own order (sorted by id at generation).
#[derive(Clone, Debug, Default)]
pub enum RemoteCatalogState {
    /// Nothing fetched yet.
    #[default]
    Idle,
    /// A fetch is in flight.
    Fetching,
    /// The catalog arrived and passed the schema gate.
    Ready(PortalCatalog),
    /// The fetch or decode failed (message is user-presentable).
    Error(String),
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

/// A completed transport call, posted from its callback into the channel the
/// poll system drains. `job` is the install-job generation that sent the
/// request - a stale callback (from a job that already failed and was
/// retried) must not feed the successor.
enum PortalMsg {
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
struct PortalChannel {
    tx: Sender<PortalMsg>,
    rx: Mutex<Receiver<PortalMsg>>,
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

/// One staged install's private state: the (validated) portal entry driving
/// it and the verified files held in memory until the commit.
struct ActiveInstall {
    job: u64,
    entry: PortalEntry,
    files: Vec<(String, Vec<u8>)>,
    /// Last evidence the transport is alive (job start, then each verified
    /// file); [`timeout_wedged_fetches`] fails the job when this goes stale.
    last_progress: Instant,
}

/// The staged installs by mod id, plus the monotonically increasing job
/// generation that lets stale transport callbacks be told apart from a
/// retry's (see [`PortalMsg`]).
#[derive(Resource, Default)]
struct ActiveInstalls {
    jobs: HashMap<String, ActiveInstall>,
    next_job: u64,
}

impl ActiveInstalls {
    /// Register a fresh job for `entry` and return its generation.
    fn begin(&mut self, entry: PortalEntry) -> u64 {
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
/// file completing) before [`timeout_wedged_fetches`] fails it - the 163508
/// R1.3 recovery for a transport callback that never fires. Progress resets
/// the window, so a slow-but-alive multi-file download never trips it.
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

/// The [`FETCH_STALL_TIMEOUT`] as a resource, so tests (and future settings)
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
fn timeout_wedged_fetches(
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

/// Ids whose uninstall FILE REMOVAL is still in flight. Only wasm ever fills
/// it (its removal is a detached IndexedDB task; native removal is
/// synchronous): an install admitted while the removal still runs could have
/// its fresh writes deleted under it, so [`on_install_portal_mod`] rejects
/// those ids until the task reports back through [`PortalMsg::Removed`].
/// Cfg-INDEPENDENT so the guard itself is unit-tested natively. Pub (with the
/// set readable) because the menu's update choreography (task 142916) uses it
/// as its second guard: an update's install must not fire while the id's
/// uninstall removal is still in flight.
#[derive(Resource, Default)]
pub struct PendingRemovals(pub std::collections::HashSet<String>);

/// Kick a catalog fetch (idempotent while one is in flight).
fn on_fetch_portal_catalog(
    _: On<FetchPortalCatalog>,
    config: Res<PortalConfig>,
    client: Res<PortalClient>,
    channel: Res<PortalChannel>,
    mut remote: ResMut<RemoteCatalog>,
) {
    if matches!(remote.state, RemoteCatalogState::Fetching) {
        warn!("portal: a catalog fetch is already in flight; ignoring the re-trigger");
        return;
    }
    remote.state = RemoteCatalogState::Fetching;
    let tx = channel.tx.clone();
    client.0.fetch(
        &config.catalog_url(),
        Box::new(move |result| {
            let _ = tx.send(PortalMsg::Catalog(result));
        }),
    );
}

/// Decode a fetched catalog body. The SCHEMA GATE runs first, on a minimal
/// probe of `schema_version` alone: an unknown version must be reported AS
/// unknown, never as a misparse of a shape this build does not know (and a
/// same-shaped future catalog must not silently half-parse either).
fn decode_catalog(result: FetchResult) -> RemoteCatalogState {
    let bytes = match result {
        Ok(bytes) => bytes,
        Err(error) => {
            return RemoteCatalogState::Error(format!("portal catalog fetch failed: {error}"))
        }
    };
    #[derive(serde::Deserialize)]
    struct SchemaProbe {
        schema_version: u32,
    }
    let probe: SchemaProbe = match serde_json::from_slice(&bytes) {
        Ok(probe) => probe,
        Err(error) => {
            return RemoteCatalogState::Error(format!("portal catalog does not parse: {error}"))
        }
    };
    if probe.schema_version != PORTAL_SCHEMA_VERSION {
        return RemoteCatalogState::Error(format!(
            "portal catalog schema_version {} is not supported (this build reads {}); \
             update the game to browse this portal",
            probe.schema_version, PORTAL_SCHEMA_VERSION
        ));
    }
    match serde_json::from_slice::<PortalCatalog>(&bytes) {
        Ok(catalog) => RemoteCatalogState::Ready(catalog),
        Err(error) => RemoteCatalogState::Error(format!("portal catalog does not parse: {error}")),
    }
}

/// Gate a persisted last-good body exactly like a fetched one: only a catalog
/// that (still) passes the schema gate becomes the offline fallback. A store
/// written by a different build, carrying a schema this one does not read,
/// is dropped - never half-trusted.
fn decode_last_good(bytes: Vec<u8>) -> Option<PortalCatalog> {
    match decode_catalog(Ok(bytes)) {
        RemoteCatalogState::Ready(catalog) => Some(catalog),
        _ => None,
    }
}

/// Cross-platform persistence of the last-good portal catalog (the Explore
/// tab's offline fallback) - the mod_prefs small-store idiom: best-effort, a
/// missing/corrupt store reads as `None`, write failures are logged and never
/// fatal. The stored value is the RAW fetched JSON (not a re-encoding), so
/// the startup load runs the exact decode + schema gate a live fetch does.
///
/// The native file lives under the MOD CACHE's data root, not the config dir:
/// the catalog is cached wire data, not a user preference, and the cache
/// root's `NOVA_MOD_CACHE_ROOT` override is what keeps the integration rigs
/// (which fetch localhost catalogs through the real plugin) from writing into
/// the developer's real store.
mod last_good_store {
    /// Store cap - a cap, not a quota: the whole real catalog is a few KiB,
    /// and a body too large to be worth caching (or a hostile one) is simply
    /// not persisted; the in-memory `last_good` still serves the session.
    /// Enforced on BOTH directions (review 142916 R1.2): the store file is
    /// user-writable input, so the startup load checks the size before
    /// reading a byte, never slurping an unbounded blob.
    pub const MAX_LAST_GOOD_BYTES: usize = 256 * 1024;

    pub fn load() -> Option<Vec<u8>> {
        backend::load()
    }

    pub fn save(bytes: &[u8]) {
        backend::save(bytes);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub mod backend {
        use std::path::{Path, PathBuf};

        use bevy::log::warn;

        use super::MAX_LAST_GOOD_BYTES;

        /// `<data_root>/portal_catalog.json` (the mod cache's root resolution,
        /// including its test override).
        fn store_path() -> Option<PathBuf> {
            crate::mod_cache::portal_catalog_store_path()
        }

        pub fn load() -> Option<Vec<u8>> {
            load_from(&store_path()?)
        }

        pub fn save(bytes: &[u8]) {
            let Some(path) = store_path() else {
                warn!("portal: no data dir available; the last-good catalog will not persist");
                return;
            };
            save_to(&path, bytes);
        }

        /// Pure (path in), so the unit tests pin the round-trip. The size cap
        /// gates the READ too: the store is user-writable input, and an
        /// oversized file is dropped before a byte of it is buffered.
        pub fn load_from(path: &Path) -> Option<Vec<u8>> {
            let size = std::fs::metadata(path).ok()?.len();
            if size > MAX_LAST_GOOD_BYTES as u64 {
                warn!(
                    "portal: the last-good store is {size} bytes (cap {MAX_LAST_GOOD_BYTES}); \
                     ignoring it"
                );
                return None;
            }
            std::fs::read(path).ok()
        }

        /// Pure (path in); the size cap is enforced HERE so the unit tests
        /// pin it alongside the round-trip.
        pub fn save_to(path: &Path, bytes: &[u8]) {
            if bytes.len() > MAX_LAST_GOOD_BYTES {
                warn!(
                    "portal: the catalog is {} bytes (cap {MAX_LAST_GOOD_BYTES}); not persisting",
                    bytes.len()
                );
                return;
            }
            if let Some(parent) = path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    warn!("portal: could not create {}: {e}", parent.display());
                    return;
                }
            }
            if let Err(e) = std::fs::write(path, bytes) {
                warn!("portal: could not write {}: {e}", path.display());
            }
        }
    }

    // Reviewed statically like the other wasm store backends (mod_prefs,
    // mod_cache): the wasm target is compiled only by the manual web deploy,
    // so this stays a minimal mirror of the native backend.
    #[cfg(target_arch = "wasm32")]
    pub mod backend {
        use bevy::log::warn;

        use super::MAX_LAST_GOOD_BYTES;

        /// The localStorage key; namespaced like the other nova stores.
        const KEY: &str = "nova_protocol.portal_catalog";

        fn storage() -> Option<web_sys::Storage> {
            web_sys::window()?.local_storage().ok()?
        }

        pub fn load() -> Option<Vec<u8>> {
            let raw = storage()?.get_item(KEY).ok()??;
            // The read-side cap, mirroring the native load_from: the store
            // is user-writable input (String::len is bytes).
            if raw.len() > MAX_LAST_GOOD_BYTES {
                warn!(
                    "portal: the last-good store is {} bytes (cap {MAX_LAST_GOOD_BYTES}); \
                     ignoring it",
                    raw.len()
                );
                return None;
            }
            Some(raw.into_bytes())
        }

        pub fn save(bytes: &[u8]) {
            // Mirrors the native save_to: cap first, then best-effort.
            if bytes.len() > MAX_LAST_GOOD_BYTES {
                warn!(
                    "portal: the catalog is {} bytes (cap {MAX_LAST_GOOD_BYTES}); not persisting",
                    bytes.len()
                );
                return;
            }
            let Some(storage) = storage() else {
                warn!("portal: no localStorage available; the last-good catalog will not persist");
                return;
            };
            // Only schema-gated (JSON-parsed, hence UTF-8) bodies reach a
            // save; the guard keeps a future misuse loud instead of lossy.
            let Ok(text) = std::str::from_utf8(bytes) else {
                warn!("portal: the catalog body is not UTF-8; not persisting");
                return;
            };
            if storage.set_item(KEY, text).is_err() {
                warn!("portal: localStorage write failed; the last-good catalog was not saved");
            }
        }
    }
}

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
/// `-` and `.` for ids/versions (`validate_id` in nova_portal_gen is even
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
    let mut seen_paths = std::collections::HashSet::new();
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
fn fail_install(jobs: &mut InstallJobs, active: &mut ActiveInstalls, id: &str, reason: String) {
    warn!("portal: install of '{id}' failed: {reason}");
    active.jobs.remove(id);
    jobs.0.insert(id.to_string(), InstallStatus::Failed(reason));
}

/// Kick the fetch of `entry.files[index]` for job `job`.
fn fetch_file(
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
fn on_install_portal_mod(
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

    let total = entry.files.len();
    jobs.0
        .insert(id.clone(), InstallStatus::Fetching { done: 0, total });
    let entry = entry.clone();
    let job = active.begin(entry.clone());
    fetch_file(&config, &client, &channel.tx, job, &entry, 0);
}

/// Uninstall a DOWNLOADED mod: index entry first (the index must never point
/// at missing files), files second, then the runtime record, any stale
/// `Failed` job entry, and - resolving 142906's R1.7 - the id's
/// [`EnabledMods`] entry, so a reinstall starts disabled like any fresh
/// install (the existing change-gated save system persists the strip). A mod
/// whose install is still in flight has nothing committed to uninstall; the
/// trigger is ignored with a warning.
fn on_uninstall_portal_mod(
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
fn start_commit(tx: &Sender<PortalMsg>, install: ActiveInstall, _dir: ()) {
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
/// channel. Wasm: an IoTaskPool task awaits the single IndexedDB transaction
/// to its COMMIT (review 142906 R1.4), then writes the index and inserts the
/// files into the shared `mods://` memory `Dir` (the reader the bundle load
/// will hit - the startup hydrator only runs once, so a live install must
/// feed the Dir itself).
#[cfg(target_arch = "wasm32")]
fn start_commit(
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
                let expected = install.entry.files[index].clone();
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

#[cfg(test)]
mod tests {
    use nova_mod_format::{ModMeta, PortalFile};

    use super::*;

    #[test]
    fn join_url_normalizes_the_slash_seam() {
        assert_eq!(
            join_url("http://x/mods", "catalog.json"),
            "http://x/mods/catalog.json"
        );
        assert_eq!(
            join_url("http://x/mods/", "catalog.json"),
            "http://x/mods/catalog.json"
        );
    }

    /// The wasm default derivation, pinned natively (the fn is pure): the
    /// game under `<root>/play/` fetches the SIBLING `<root>/mods`, documents
    /// and query/fragment noise are dropped, and a root-served page just
    /// appends `mods`.
    #[test]
    fn portal_base_derives_from_the_page_location() {
        assert_eq!(
            portal_base_from_href("https://alexjercan.github.io/nova-protocol/play/index.html"),
            "https://alexjercan.github.io/nova-protocol/mods"
        );
        assert_eq!(
            portal_base_from_href("https://alexjercan.github.io/nova-protocol/play/"),
            "https://alexjercan.github.io/nova-protocol/mods"
        );
        assert_eq!(
            portal_base_from_href("https://example.com/play/?seed=3#frag"),
            "https://example.com/mods"
        );
        assert_eq!(
            portal_base_from_href("http://localhost:8080/"),
            "http://localhost:8080/mods"
        );
        assert_eq!(
            portal_base_from_href("http://localhost:8080/index.html"),
            "http://localhost:8080/mods"
        );
    }

    /// The `?portal=` override wins over the location-derived default and is
    /// percent-decoded; other params and an empty value do not override.
    #[test]
    fn portal_query_override_parses() {
        assert_eq!(
            portal_override_from_query("?portal=http%3A%2F%2Flocalhost%3A8000%2Fmods"),
            Some("http://localhost:8000/mods".to_string())
        );
        assert_eq!(
            portal_override_from_query("?seed=1&portal=http://localhost:8000/mods"),
            Some("http://localhost:8000/mods".to_string())
        );
        assert_eq!(portal_override_from_query("?seed=1"), None);
        assert_eq!(portal_override_from_query("?portal="), None);
        assert_eq!(portal_override_from_query(""), None);
    }

    /// The cross-origin detector behind the wasm heads-up: origin is
    /// `scheme://host[:port]`, path/query/fragment dropped; a same-origin base
    /// matches the page origin, a different port/host does not; a relative base
    /// has no origin.
    #[test]
    fn url_origin_extracts_scheme_host_port() {
        assert_eq!(
            url_origin("http://localhost:8000/mods"),
            Some("http://localhost:8000".to_string())
        );
        assert_eq!(
            url_origin("https://alexjercan.github.io/nova-protocol/mods"),
            Some("https://alexjercan.github.io".to_string())
        );
        // Same host, different port is still cross-origin (the reported bug:
        // page :8090, portal :8000).
        assert_ne!(
            url_origin("http://localhost:8000/mods"),
            url_origin("http://localhost:8090/play/"),
        );
        // Same origin, different path -> equal origins (the same-origin default).
        assert_eq!(
            url_origin("http://localhost:8080/mods"),
            url_origin("http://localhost:8080/"),
        );
        // A relative base has no origin (never flagged cross-origin).
        assert_eq!(url_origin("/mods"), None);
        assert_eq!(url_origin("mods"), None);
    }

    fn catalog_json(schema_version: u32) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "schema_version": schema_version,
            "entries": [],
        }))
        .unwrap()
    }

    /// The schema gate: a matching version parses to Ready, an unknown one is
    /// an Error NAMING the version mismatch (never a misparse or a silent
    /// half-parse), and garbage is a parse Error. Deleting the
    /// `schema_version` check in `decode_catalog` fails the middle assertion:
    /// the v999 body is shape-compatible and would decode Ready.
    #[test]
    fn decode_catalog_gates_on_schema_version() {
        assert!(matches!(
            decode_catalog(Ok(catalog_json(PORTAL_SCHEMA_VERSION))),
            RemoteCatalogState::Ready(_)
        ));
        match decode_catalog(Ok(catalog_json(999))) {
            RemoteCatalogState::Error(error) => {
                assert!(
                    error.contains("schema_version 999"),
                    "the error must name the unknown version: {error}"
                );
            }
            other => panic!("a v999 catalog must be an Error, got {other:?}"),
        }
        assert!(matches!(
            decode_catalog(Ok(b"not json".to_vec())),
            RemoteCatalogState::Error(_)
        ));
        assert!(matches!(
            decode_catalog(Err("connection refused".to_string())),
            RemoteCatalogState::Error(_)
        ));
    }

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

    /// Review 163508 R1.1: the local `Path`-component gates are NOT URL
    /// containment - a WHATWG-conformant fetcher percent-decodes segments,
    /// so `%2e%2e` is a dot-dot on the wire while being a plain `Normal`
    /// component locally. The charset gate must reject any segment outside
    /// the generator's published alphabet BEFORE any fetch.
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

    /// Review 163508 R1.4: the anti-absurdity caps - a catalog cannot make
    /// the client stage absurd amounts of memory (or requests). One entry
    /// per cap: per-file size, file count, summed declared size.
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

    /// The last-good persistence round-trip (task 142916): the raw catalog
    /// JSON saves through the pure store backend, reads back byte-identical,
    /// and re-passes the startup decode gate into a usable catalog. Deleting
    /// the store helpers (or the decode gate) fails this test.
    #[test]
    fn last_good_store_round_trips_the_catalog() {
        use super::last_good_store::backend::{load_from, save_to};

        let path = std::env::temp_dir().join("nova_portal_lastgood_round_trip/portal_catalog.json");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());

        assert_eq!(load_from(&path), None, "a missing store reads as None");

        let catalog = PortalCatalog {
            schema_version: PORTAL_SCHEMA_VERSION,
            entries: vec![entry(
                "pack",
                "1.0.0",
                "pack.bundle.ron",
                &["pack.bundle.ron"],
            )],
        };
        let bytes = serde_json::to_vec(&catalog).unwrap();
        save_to(&path, &bytes);
        let loaded = load_from(&path).expect("the saved store reads back");
        assert_eq!(loaded, bytes, "the store persists the raw bytes verbatim");
        let decoded = decode_last_good(loaded).expect("the round-tripped catalog decodes Ready");
        assert_eq!(decoded.entries.len(), 1);
        assert_eq!(decoded.entries[0].id, "pack");

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// The store CAP, both directions (a cap, not a quota): a body over
    /// 256 KiB is not written at all, and - review 142916 R1.2 - an OVERSIZED
    /// STORE FILE (user-writable input) is dropped at load before a byte of
    /// it is buffered. Deleting the cap check in `save_to` writes the file
    /// and fails the absence assert; deleting the metadata gate in
    /// `load_from` slurps the planted blob and fails the None assert.
    #[test]
    fn last_good_store_enforces_the_size_cap() {
        use super::last_good_store::{
            backend::{load_from, save_to},
            MAX_LAST_GOOD_BYTES,
        };

        let path = std::env::temp_dir().join("nova_portal_lastgood_cap/portal_catalog.json");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());

        let oversized = vec![b'x'; MAX_LAST_GOOD_BYTES + 1];
        save_to(&path, &oversized);
        assert!(!path.exists(), "an oversized body is not persisted");

        // The read side: plant an oversized store file directly (what a user
        // or another program could do) - the load must refuse it.
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &oversized).unwrap();
        assert_eq!(
            load_from(&path),
            None,
            "an oversized store file is refused at load"
        );

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// The startup load re-runs the SCHEMA gate: a store written by a build
    /// with a different `PORTAL_SCHEMA_VERSION` (or corrupted on disk) is
    /// dropped, never half-trusted as the offline fallback. Deleting the
    /// `decode_catalog` reuse in `decode_last_good` (e.g. parsing the shape
    /// directly) lets the v999 body through and fails the None assert.
    #[test]
    fn stale_last_good_with_unknown_schema_is_dropped_at_load() {
        assert!(
            decode_last_good(catalog_json(PORTAL_SCHEMA_VERSION)).is_some(),
            "a current-schema store loads"
        );
        assert!(
            decode_last_good(catalog_json(999)).is_none(),
            "an unknown-schema store is dropped"
        );
        assert!(
            decode_last_good(b"corrupt {{{".to_vec()).is_none(),
            "a corrupt store is dropped, not a panic"
        );
    }

    /// Review 163508 R1.2: an install for an id whose uninstall file-removal
    /// is still in flight (wasm's is a detached task) is rejected before
    /// anything else - a fresh write could be deleted under it. The guard
    /// and resource are cfg-independent (only wasm ever fills the set), so
    /// this native test pins the exact production observer.
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
