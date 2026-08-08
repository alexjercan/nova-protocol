//! Persistent key-value storage, one impl per platform.
//!
//! Mirrors `PortalTransport`: the trait exists so the call sites stop carrying
//! `#[cfg(target_arch = "wasm32")]` and so a test can substitute a fake. Every
//! platform gate for persisted values lives in THIS module now - [`persist`]
//! above it is pure codec, and reads the same on both targets.
//!
//! [`persist`]: crate::persist
//!
//! Keys are opaque names ("settings", "enabled_mods"); the impl decides where
//! the bytes land:
//!
//! - Native: `<root>/<key>.ron`, root being `$NOVA_CONFIG_ROOT` or
//!   `dirs::config_dir()/nova-protocol`.
//! - Wasm: `window.localStorage` under `nova_protocol.<key>`.
//!
//! There is no `remove`: nothing in the game deletes a persisted value, and an
//! unused trait method is a contract no impl is held to.

use std::fmt;

/// Why a store write failed. Opaque on purpose - every caller logs it and
/// carries on with the previous value, so nothing branches on the cause.
#[derive(Debug)]
pub struct StorageError(String);

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for StorageError {}

/// Persistent key-value storage.
pub trait Storage: Send + Sync + 'static {
    /// The bytes stored under `key`, or `None` when nothing is stored there or
    /// the store is unreadable. A caller reads `None` as "use the default".
    fn read(&self, key: &str) -> Option<Vec<u8>>;

    /// Replace `key`'s value with `bytes`.
    ///
    /// MUST be atomic: temp file + fsync + rename on native, a single
    /// `set_item` on wasm. A kill mid-write leaves the previous value intact.
    fn write(&self, key: &str, bytes: &[u8]) -> Result<(), StorageError>;
}

/// The store type this build talks to.
#[cfg(not(target_arch = "wasm32"))]
pub type PlatformStorage = NativeStorage;

/// The store type this build talks to.
#[cfg(target_arch = "wasm32")]
pub type PlatformStorage = WebStorage;

/// The store for this build, or `None` when the platform offers none (no
/// config dir; localStorage disabled by the browser). The one place either
/// backend is chosen.
pub fn platform() -> Option<PlatformStorage> {
    PlatformStorage::available()
}

/// Files under a config root, one RON file per key.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct NativeStorage {
    root: std::path::PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl NativeStorage {
    /// A store rooted at an explicit directory. The seam a test writes
    /// through so it cannot touch the developer's real config.
    pub fn at(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The store at `$NOVA_CONFIG_ROOT`, else `dirs::config_dir()/nova-protocol`.
    ///
    /// The override is the config-dir twin of `NOVA_MOD_CACHE_ROOT` and exists
    /// for the same reason: a test or tool that exercises the save path must
    /// not overwrite the developer's real settings.
    pub fn available() -> Option<Self> {
        let root = match std::env::var_os("NOVA_CONFIG_ROOT") {
            Some(root) => std::path::absolute(std::path::PathBuf::from(root)).ok()?,
            None => dirs::config_dir()?.join("nova-protocol"),
        };
        Some(Self::at(root))
    }

    /// `<root>/<key>.ron`. The suffix is fixed here rather than by the codec
    /// above so the on-disk names a player already has stay in one place.
    pub fn path(&self, key: &str) -> std::path::PathBuf {
        self.root.join(format!("{key}.ron"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Storage for NativeStorage {
    fn read(&self, key: &str) -> Option<Vec<u8>> {
        std::fs::read(self.path(key)).ok()
    }

    fn write(&self, key: &str, bytes: &[u8]) -> Result<(), StorageError> {
        write_atomic(&self.path(key), bytes).map_err(|e| StorageError(e.to_string()))
    }
}

/// `window.localStorage`, namespaced.
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Copy)]
pub struct WebStorage;

/// `window.localStorage`, namespaced.
///
/// Present on native too, carrying nothing but [`WebStorage::key`], so that
/// derivation stays under test on the only target CI builds: a typo there
/// silently orphans every value a web player has saved, and no wasm test would
/// catch it.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy)]
pub struct WebStorage;

impl WebStorage {
    /// The `localStorage` key for `key`. Namespaced so it cannot collide with
    /// other app state.
    pub fn key(key: &str) -> String {
        format!("nova_protocol.{key}")
    }
}

#[cfg(target_arch = "wasm32")]
impl WebStorage {
    /// The store, or `None` when the browser withholds localStorage (private
    /// mode with storage off).
    pub fn available() -> Option<Self> {
        Self.storage().map(|_| Self)
    }

    fn storage(&self) -> Option<web_sys::Storage> {
        // `local_storage()` is `Result<Option<Storage>>`: Err if disabled by
        // the browser, Ok(None) if unavailable.
        web_sys::window()?.local_storage().ok()?
    }
}

#[cfg(target_arch = "wasm32")]
impl Storage for WebStorage {
    fn read(&self, key: &str) -> Option<Vec<u8>> {
        let raw = self.storage()?.get_item(&Self::key(key)).ok()??;
        Some(raw.into_bytes())
    }

    fn write(&self, key: &str, bytes: &[u8]) -> Result<(), StorageError> {
        let storage = self
            .storage()
            .ok_or_else(|| StorageError("no localStorage available".to_string()))?;
        let value = std::str::from_utf8(bytes)
            .map_err(|e| StorageError(format!("value is not utf-8: {e}")))?;
        storage
            .set_item(&Self::key(key), value)
            .map_err(|_| StorageError("localStorage write failed".to_string()))
    }
}

/// Replace `path`'s contents with `bytes`, or leave the previous file exactly
/// as it was.
///
/// The write goes to a sibling temp file which is flushed to the device before
/// the rename publishes it, so a kill (or a full disk) mid-write can never
/// leave a zero-length or half-encoded file where a readable one used to be.
/// [`NativeStorage`] writes through here, and so does every store in this crate
/// that owns its own path (the mod cache index, the portal catalog, the content
/// generator); that is the contract, not an optimization.
///
/// Native only - the wasm backend hands a whole string to `localStorage`, which
/// is already all-or-nothing.
#[cfg(not(target_arch = "wasm32"))]
pub fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;

    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("write_atomic: {} is not a file path", path.display()),
        ));
    };
    std::fs::create_dir_all(parent)?;

    // Same directory, or the rename would cross a filesystem and stop being
    // atomic. Pid-suffixed so two processes writing one store do not share a
    // temp file.
    let temp = parent.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    let written = (|| {
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&temp);
        return Err(e);
    }
    std::fs::rename(&temp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temp);
    })
}

/// Glob-import surface: `use nova_assets::storage::prelude::*` brings the
/// storage trait, its platform selection and the error type into scope.
pub mod prelude {
    pub use super::{platform, PlatformStorage, Storage, StorageError};
}

// The native backend is exercised directly below. The wasm localStorage
// backend is cfg-guarded and can only be reviewed statically: the wasm target
// is built neither by the local test runner NOR by the automated PR/master CI
// (`ci.yaml`) - only the manual `workflow_dispatch` web deploy compiles it. So
// static review (checked against the web-sys 0.3 API) is the real guard for
// that path; keep it a minimal mirror of the native one.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{write_atomic, NativeStorage, Storage, WebStorage};

    /// The two locations the store inherited from the modules it replaced. A
    /// change here is a silent data loss for every existing player, so both are
    /// pinned as literals rather than derived from the code under test.
    #[test]
    fn the_storage_locations_match_the_stores_this_replaced() {
        let native = NativeStorage::available().expect("a config dir on the test host");
        assert!(
            native
                .path("enabled_mods")
                .ends_with("nova-protocol/enabled_mods.ron"),
            "native mod prefs moved: {}",
            native.path("enabled_mods").display()
        );
        assert!(native
            .path("settings")
            .ends_with("nova-protocol/settings.ron"));
        assert_eq!(
            WebStorage::key("enabled_mods"),
            "nova_protocol.enabled_mods"
        );
        assert_eq!(WebStorage::key("settings"), "nova_protocol.settings");
    }

    /// A unique temp root per test; the test cleans it up.
    fn temp_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("nova_storage_{name}"))
    }

    #[test]
    fn write_then_read_round_trips_the_bytes() {
        let root = temp_root("round_trip");
        let _ = std::fs::remove_dir_all(&root);

        let store = NativeStorage::at(&root);
        store.write("value", b"payload").expect("the write lands");
        assert_eq!(
            store.read("value").as_deref(),
            Some(b"payload".as_slice()),
            "the bytes round-trip, and the write created the root dir"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_key_reads_none() {
        let root = temp_root("missing");
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(
            NativeStorage::at(&root).read("value"),
            None,
            "a missing file reads as nothing saved"
        );
    }

    /// The property the four stores depend on: a failed write leaves the
    /// PREVIOUS contents readable. Here the failure is a target that cannot be
    /// renamed onto - a directory - which is the same rename step a kill
    /// mid-write never reaches.
    #[test]
    fn a_failed_atomic_write_leaves_the_previous_contents() {
        let dir = temp_root("atomic");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("value.ron");
        write_atomic(&path, b"good").unwrap();

        let blocked = dir.join("blocked");
        std::fs::create_dir_all(&blocked).unwrap();
        assert!(
            write_atomic(&blocked, b"never").is_err(),
            "renaming onto a directory must fail"
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"good",
            "the earlier value is untouched"
        );
        let strays: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
            .filter(|name| name.ends_with(".tmp"))
            .collect();
        assert!(strays.is_empty(), "the failed write cleans up: {strays:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
