//! Cross-platform persistence of the enabled-mods set (task 20260714-174131).
//!
//! The enabled mod ids are stored as a RON `Vec<String>`. On native they live in a
//! file under the user config dir (`dirs::config_dir()/nova-protocol/enabled_mods.ron`);
//! on the web they live in `window.localStorage` under a fixed key. Both are
//! best-effort: a missing or corrupt store reads as `None` (fall back to the base
//! default), and write failures are logged, never fatal.
//!
//! This is a small hand-rolled store on purpose: Bevy 0.19 is bleeding-edge and a
//! third-party persistence crate would be a version-compat liability for a UI feature.

/// The saved enabled mod ids, or `None` if nothing has been saved yet (or the store
/// is unreadable/corrupt). `None` means "use the default"; `Some` is authoritative.
pub fn load_enabled_ids() -> Option<Vec<String>> {
    backend::load()
}

/// Persist the enabled mod ids. Best-effort - failures are logged, not returned.
pub fn save_enabled_ids(ids: &[String]) {
    backend::save(ids);
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use std::path::{Path, PathBuf};

    use bevy::prelude::*;

    pub fn load() -> Option<Vec<String>> {
        load_from(&config_path()?)
    }

    pub fn save(ids: &[String]) {
        let Some(path) = config_path() else {
            warn!("mod prefs: no config dir available; enabled mods will not persist");
            return;
        };
        save_to(&path, ids);
    }

    /// `<config_dir>/nova-protocol/enabled_mods.ron`.
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("nova-protocol").join("enabled_mods.ron"))
    }

    /// Read + decode the ids from a RON file. `None` on missing / unreadable /
    /// corrupt - the caller falls back to defaults. Pure (path in), so unit-testable.
    pub fn load_from(path: &Path) -> Option<Vec<String>> {
        let bytes = std::fs::read(path).ok()?;
        ron::de::from_bytes::<Vec<String>>(&bytes).ok()
    }

    /// Encode + write the ids as RON, creating the parent dir first. Best-effort.
    pub fn save_to(path: &Path, ids: &[String]) {
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                warn!("mod prefs: could not create {}: {e}", parent.display());
                return;
            }
        }
        match ron::ser::to_string(ids) {
            Ok(s) => {
                if let Err(e) = std::fs::write(path, s) {
                    warn!("mod prefs: could not write {}: {e}", path.display());
                }
            }
            Err(e) => warn!("mod prefs: could not encode enabled mods: {e}"),
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use bevy::prelude::*;

    /// The localStorage key. Namespaced so it cannot collide with other app state.
    const KEY: &str = "nova_protocol.enabled_mods";

    fn storage() -> Option<web_sys::Storage> {
        // `local_storage()` is `Result<Option<Storage>>`: Err if disabled by the
        // browser (e.g. private mode with storage off), Ok(None) if unavailable.
        web_sys::window()?.local_storage().ok()?
    }

    pub fn load() -> Option<Vec<String>> {
        let raw = storage()?.get_item(KEY).ok()??;
        ron::de::from_str::<Vec<String>>(&raw).ok()
    }

    pub fn save(ids: &[String]) {
        let Some(storage) = storage() else {
            warn!("mod prefs: no localStorage available; enabled mods will not persist");
            return;
        };
        match ron::ser::to_string(ids) {
            Ok(s) => {
                if storage.set_item(KEY, &s).is_err() {
                    warn!("mod prefs: localStorage write failed; enabled mods not saved");
                }
            }
            Err(e) => warn!("mod prefs: could not encode enabled mods: {e}"),
        }
    }
}

// The native file backend is unit-testable via its pure `load_from`/`save_to`. The
// wasm localStorage backend is cfg-guarded and can only be reviewed statically here:
// the wasm target is built neither by the local test runner NOR by the automated
// PR/master CI (`ci.yaml`) - only the manual `workflow_dispatch` web deploy compiles
// it. So static review (checked against the web-sys 0.3 API) is the real guard for
// this path; keep it a minimal mirror of the native backend.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::backend::{load_from, save_to};

    /// A unique temp path per test (no tempfile crate); the test cleans it up.
    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("nova_modprefs_{name}/enabled_mods.ron"))
    }

    #[test]
    fn save_then_load_round_trips_the_ids() {
        let path = temp_path("round_trip");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());

        let ids = vec!["base".to_string(), "demo".to_string()];
        save_to(&path, &ids);
        assert!(
            path.exists(),
            "save_to must create the file (and its parent dir)"
        );
        assert_eq!(
            load_from(&path),
            Some(ids),
            "the ids round-trip through RON"
        );

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn missing_file_loads_none() {
        let path = temp_path("missing");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        assert_eq!(
            load_from(&path),
            None,
            "a missing file reads as no saved prefs"
        );
    }

    #[test]
    fn corrupt_file_loads_none() {
        let path = temp_path("corrupt");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"this is not ron {{{").unwrap();
        assert_eq!(
            load_from(&path),
            None,
            "corrupt data reads as no saved prefs (not a panic)"
        );
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
