//! Cross-platform persistence for one small named value, stored as RON.
//!
//! [`load`] and [`save`] are the whole surface: a key names the value, the
//! platform decides where it lands, and both directions are best-effort. A
//! missing or corrupt store reads as `None` ("use the default"); a write
//! failure is logged, never fatal.
//!
//! - Native: `dirs::config_dir()/nova-protocol/<key>.ron`, or
//!   `$NOVA_CONFIG_ROOT/<key>.ron` when that override is set.
//! - Wasm: `window.localStorage` under `nova_protocol.<key>`.
//!
//! Deliberately a store, not a plugin. Both callers project a Bevy resource
//! through a policy the value type does not know about - the settings menu
//! debounces a slider drag into one write and snapshots four resources into one
//! blob, the mod set sorts a `HashSet` for a diff-friendly file - so a
//! load-on-build / save-on-`resource_changed` plugin would have to be bypassed
//! by both. Modelled as the deliberate counterpart to the load-on-build /
//! save-on-`resource_changed` plugin shape, which fits the case where the
//! resource IS the stored value and neither caller here is.
//!
//! Hand-rolled rather than taken from a persistence crate for the same reason
//! the two stores it replaces were: Bevy 0.19 is bleeding-edge, and this is a
//! best-effort read/write of one small file.

use serde::{de::DeserializeOwned, Serialize};

/// The stored value for `key`, or `None` when nothing has been saved yet or the
/// store is unreadable/corrupt. `None` means "use the default"; `Some` is
/// authoritative.
pub fn load<T: DeserializeOwned>(key: &str) -> Option<T> {
    backend::load(key)
}

/// Persist `value` under `key`. Best-effort - failures are logged, not returned.
pub fn save<T: Serialize>(key: &str, value: &T) {
    backend::save(key, value)
}

/// The `localStorage` key for `key`. Namespaced so it cannot collide with other
/// app state.
///
/// Lives outside the wasm backend, and is therefore dead code on native, so the
/// derivation stays under test on the only target CI builds: a typo here
/// silently orphans every value a web player has saved, and no wasm test would
/// catch it.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn storage_key(key: &str) -> String {
    format!("nova_protocol.{key}")
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use std::path::{Path, PathBuf};

    use bevy::prelude::*;
    use serde::{de::DeserializeOwned, Serialize};

    pub fn load<T: DeserializeOwned>(key: &str) -> Option<T> {
        load_from(&config_path(key)?)
    }

    pub fn save<T: Serialize>(key: &str, value: &T) {
        let Some(path) = config_path(key) else {
            warn!("persist[{key}]: no config dir available; the value will not persist");
            return;
        };
        save_to(&path, value);
    }

    /// `<config_root>/<key>.ron`, where the root is
    /// `$NOVA_CONFIG_ROOT` if set, else `dirs::config_dir()/nova-protocol`.
    ///
    /// The override is the config-dir twin of `NOVA_MOD_CACHE_ROOT` and exists
    /// for the same reason: a test or tool that exercises the save path must
    /// not overwrite the developer's real settings.
    pub(super) fn config_path(key: &str) -> Option<PathBuf> {
        let root = match std::env::var_os("NOVA_CONFIG_ROOT") {
            Some(root) => std::path::absolute(PathBuf::from(root)).ok()?,
            None => dirs::config_dir()?.join("nova-protocol"),
        };
        Some(root.join(format!("{key}.ron")))
    }

    /// Read + decode a RON file. `None` on missing / unreadable / corrupt - the
    /// caller falls back to defaults. Pure (path in), so unit-testable.
    pub fn load_from<T: DeserializeOwned>(path: &Path) -> Option<T> {
        let bytes = std::fs::read(path).ok()?;
        ron::de::from_bytes::<T>(&bytes).ok()
    }

    /// Encode + write as RON, creating the parent dir first. Best-effort.
    pub fn save_to<T: Serialize>(path: &Path, value: &T) {
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                warn!("persist: could not create {}: {e}", parent.display());
                return;
            }
        }
        match ron::ser::to_string(value) {
            Ok(s) => {
                if let Err(e) = super::write_atomic(path, s.as_bytes()) {
                    warn!("persist: could not write {}: {e}", path.display());
                }
            }
            Err(e) => warn!("persist: could not encode {}: {e}", path.display()),
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use bevy::prelude::*;
    use serde::{de::DeserializeOwned, Serialize};

    use super::storage_key;

    fn storage() -> Option<web_sys::Storage> {
        // `local_storage()` is `Result<Option<Storage>>`: Err if disabled by the
        // browser (e.g. private mode with storage off), Ok(None) if unavailable.
        web_sys::window()?.local_storage().ok()?
    }

    pub fn load<T: DeserializeOwned>(key: &str) -> Option<T> {
        let raw = storage()?.get_item(&storage_key(key)).ok()??;
        ron::de::from_str::<T>(&raw).ok()
    }

    pub fn save<T: Serialize>(key: &str, value: &T) {
        let Some(storage) = storage() else {
            warn!("persist[{key}]: no localStorage available; the value will not persist");
            return;
        };
        match ron::ser::to_string(value) {
            Ok(s) => {
                if storage.set_item(&storage_key(key), &s).is_err() {
                    warn!("persist[{key}]: localStorage write failed; the value was not saved");
                }
            }
            Err(e) => warn!("persist[{key}]: could not encode the value: {e}"),
        }
    }
}

/// Replace `path`'s contents with `bytes`, or leave the previous file exactly
/// as it was.
///
/// The write goes to a sibling temp file which is flushed to the device before
/// the rename publishes it, so a kill (or a full disk) mid-write can never
/// leave a zero-length or half-encoded file where a readable one used to be.
/// Every persistent store in this crate writes through here; that is the
/// contract, not an optimization.
///
/// Native only - the wasm backends hand a whole string to `localStorage`,
/// which is already all-or-nothing.
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

/// Read + decode a RON file at an explicit path. `None` on missing / unreadable /
/// corrupt. The path-explicit form of [`load`], for round-trip tests that must not
/// touch the real config dir.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_from<T: DeserializeOwned>(path: &std::path::Path) -> Option<T> {
    backend::load_from(path)
}

/// Encode + write as RON at an explicit path, creating the parent dir first.
/// Best-effort. The path-explicit form of [`save`]; see [`load_from`].
#[cfg(not(target_arch = "wasm32"))]
pub fn save_to<T: Serialize>(path: &std::path::Path, value: &T) {
    backend::save_to(path, value)
}

// The native file backend is unit-testable via its pure `load_from`/`save_to`. The
// wasm localStorage backend is cfg-guarded and can only be reviewed statically here:
// the wasm target is built neither by the local test runner NOR by the automated
// PR/master CI (`ci.yaml`) - only the manual `workflow_dispatch` web deploy compiles
// it. So static review (checked against the web-sys 0.3 API) is the real guard for
// this path; keep it a minimal mirror of the native backend.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{backend::config_path, load_from, save_to, storage_key};

    /// The two locations the store inherited from the modules it replaced. A
    /// change here is a silent data loss for every existing player, so both are
    /// pinned as literals rather than derived from the code under test.
    #[test]
    fn the_storage_locations_match_the_stores_this_replaced() {
        let path = config_path("enabled_mods").expect("a config dir on the test host");
        assert!(
            path.ends_with("nova-protocol/enabled_mods.ron"),
            "native mod prefs moved: {}",
            path.display()
        );
        assert!(config_path("settings")
            .expect("a config dir on the test host")
            .ends_with("nova-protocol/settings.ron"));
        assert_eq!(storage_key("enabled_mods"), "nova_protocol.enabled_mods");
        assert_eq!(storage_key("settings"), "nova_protocol.settings");
    }

    /// A unique temp path per test (no tempfile crate); the test cleans it up.
    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("nova_persist_{name}/value.ron"))
    }

    #[test]
    fn save_then_load_round_trips_the_value() {
        let path = temp_path("round_trip");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());

        let value = vec!["base".to_string(), "demo".to_string()];
        save_to(&path, &value);
        assert!(
            path.exists(),
            "save_to must create the file (and its parent dir)"
        );
        assert_eq!(
            load_from::<Vec<String>>(&path),
            Some(value),
            "the value round-trips through RON"
        );

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// The property the four stores depend on: a failed write leaves the
    /// PREVIOUS contents readable. Here the failure is a target that cannot be
    /// renamed onto - a directory - which is the same rename step a kill
    /// mid-write never reaches.
    #[test]
    fn a_failed_atomic_write_leaves_the_previous_contents() {
        use super::write_atomic;

        let dir = std::env::temp_dir().join("nova_persist_atomic");
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

    #[test]
    fn missing_file_loads_none() {
        let path = temp_path("missing");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        assert_eq!(
            load_from::<Vec<String>>(&path),
            None,
            "a missing file reads as nothing saved"
        );
    }

    #[test]
    fn corrupt_file_loads_none() {
        let path = temp_path("corrupt");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"this is not ron {{{").unwrap();
        assert_eq!(
            load_from::<Vec<String>>(&path),
            None,
            "corrupt data reads as nothing saved (not a panic)"
        );
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
