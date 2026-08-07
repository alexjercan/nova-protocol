//! The CATALOG half: fetch `catalog.json`, gate it on the portal schema
//! version, and persist the last-good body as the Explore tab's offline
//! fallback.

use bevy::prelude::*;
use nova_mod_format::{PortalCatalog, MAX_CATALOG_BYTES, PORTAL_SCHEMA_VERSION};

use super::{transport::FetchResult, PortalChannel, PortalMsg};
use crate::portal::{config::PortalConfig, transport::PortalClient};

/// Trigger: (re)fetch the portal catalog. Result lands in [`RemoteCatalog`].
#[derive(Event, Clone, Debug)]
pub struct FetchPortalCatalog;

/// The fetched portal catalog: the fetch state machine the Explore UI renders
/// plus the LAST-GOOD catalog (the offline fallback). State transitions never
/// clear `last_good`: a successful fetch refreshes it (and persists the raw
/// body through `last_good_store`), startup loads it back, and every failure
/// leaves it standing for the UI's stale rendering.
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

/// Kick a catalog fetch (idempotent while one is in flight).
pub(super) fn on_fetch_portal_catalog(
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
pub(super) fn decode_catalog(result: FetchResult) -> RemoteCatalogState {
    let bytes = match result {
        Ok(bytes) => bytes,
        Err(error) => {
            return RemoteCatalogState::Error(format!("portal catalog fetch failed: {error}"))
        }
    };
    // The size gate runs before EITHER parse. `ehttp` buffers a whole response
    // body before the callback, so this is the earliest point the client
    // controls - bounding the decode, not the socket read.
    if bytes.len() > MAX_CATALOG_BYTES {
        return RemoteCatalogState::Error(format!(
            "portal catalog is {} bytes (max {MAX_CATALOG_BYTES}); refusing to parse it",
            bytes.len()
        ));
    }
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
        Ok(catalog) => match catalog.check_size() {
            Ok(()) => RemoteCatalogState::Ready(catalog),
            Err(error) => RemoteCatalogState::Error(error),
        },
        Err(error) => RemoteCatalogState::Error(format!("portal catalog does not parse: {error}")),
    }
}

/// Gate a persisted last-good body exactly like a fetched one: only a catalog
/// that (still) passes the schema gate becomes the offline fallback. A store
/// written by a different build, carrying a schema this one does not read,
/// is dropped - never half-trusted.
pub(super) fn decode_last_good(bytes: Vec<u8>) -> Option<PortalCatalog> {
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
pub(super) mod last_good_store {
    /// Store cap - a cap, not a quota: the whole real catalog is a few KiB, and
    /// a body too large to be worth caching (or a hostile one) is simply not
    /// persisted; the in-memory `last_good` still serves the session. Enforced
    /// on BOTH directions: the store file is user-writable input, so the
    /// startup load checks the size before reading a byte, never slurping an
    /// unbounded blob.
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
            if let Err(e) = crate::persist::write_atomic(path, bytes) {
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

#[cfg(test)]
mod tests {
    use super::*;

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

    /// F13: the body is untrusted and was read fully into memory and parsed
    /// TWICE with no size bound at all - the 256 KiB cap in `last_good_store`
    /// gates persistence only, never the fetch. Both caps refuse BEFORE either
    /// parse walks the body.
    #[test]
    fn decode_catalog_refuses_an_oversized_body_and_an_absurd_entry_count() {
        let mut oversized = catalog_json(PORTAL_SCHEMA_VERSION);
        oversized.resize(MAX_CATALOG_BYTES + 1, b' ');
        match decode_catalog(Ok(oversized)) {
            RemoteCatalogState::Error(error) => assert!(
                error.contains("bytes"),
                "the error must name the size: {error}"
            ),
            other => panic!("an oversized catalog must be an Error, got {other:?}"),
        }

        let many = serde_json::to_vec(&PortalCatalog {
            schema_version: PORTAL_SCHEMA_VERSION,
            entries: (0..nova_mod_format::MAX_CATALOG_ENTRIES + 1)
                .map(|i| entry(&format!("m{i}"), "1.0.0", "b.bundle.ron", &[]))
                .collect(),
        })
        .unwrap();
        assert!(
            many.len() <= MAX_CATALOG_BYTES,
            "the entry-count case must not be caught by the byte cap instead"
        );
        match decode_catalog(Ok(many)) {
            RemoteCatalogState::Error(error) => assert!(
                error.contains("entries"),
                "the error must name the entry count: {error}"
            ),
            other => panic!("an over-count catalog must be an Error, got {other:?}"),
        }
    }

    fn entry(
        id: &str,
        version: &str,
        bundle: &str,
        paths: &[&str],
    ) -> nova_mod_format::PortalEntry {
        nova_mod_format::PortalEntry {
            id: id.to_string(),
            version: version.to_string(),
            bundle: bundle.to_string(),
            meta: nova_mod_format::ModMeta::default(),
            files: paths
                .iter()
                .map(|p| nova_mod_format::PortalFile {
                    path: p.to_string(),
                    size: 1,
                    sha256: "00".repeat(32),
                })
                .collect(),
            total_size: paths.len() as u64,
        }
    }

    /// The last-good persistence round-trip: the raw catalog JSON saves through
    /// the pure store backend, reads back byte-identical, and re-passes the
    /// startup decode gate into a usable catalog. Deleting the store helpers
    /// (or the decode gate) fails this test.
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

    /// The store CAP, both directions (a cap, not a quota): a body over 256 KiB
    /// is not written at all, and - - an OVERSIZED STORE FILE (user-writable
    /// input) is dropped at load before a byte of it is buffered. Deleting the
    /// cap check in `save_to` writes the file and fails the absence assert;
    /// deleting the metadata gate in `load_from` slurps the planted blob and
    /// fails the None assert.
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
}
