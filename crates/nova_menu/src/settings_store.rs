//! Cross-platform persistence of the player settings (task 20260711-180511).
//!
//! The settings menu writes two Bevy resources - `MasterVolume` and
//! `GraphicsQuality` (both in `nova_gameplay`) - and this module makes them
//! survive a restart. It is a direct mirror of `nova_assets::mod_prefs`: a
//! RON blob stored on native under the user config dir
//! (`dirs::config_dir()/nova-protocol/settings.ron`) and on the web in
//! `window.localStorage` under a namespaced key. Both are best-effort: a
//! missing or corrupt store reads as `None` (fall back to defaults), and write
//! failures are logged, never fatal.
//!
//! A small hand-rolled store on purpose, for the same reason mod_prefs is: Bevy
//! 0.19 is bleeding-edge and a third-party settings crate would be a
//! version-compat liability for a UI feature.

use nova_gameplay::prelude::{GraphicsQuality, MasterVolume};
use serde::{Deserialize, Serialize};

/// The persisted form of the settings: plain, versionable data decoupled from
/// the live resources. Missing/extra fields are tolerated by serde defaults so
/// an older or newer file still loads.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct PersistedSettings {
    /// Linear master volume `0.0..=1.0`.
    #[serde(default = "default_volume")]
    pub master_volume: f32,
    /// The graphics-quality preset.
    #[serde(default)]
    pub graphics_quality: GraphicsQuality,
}

fn default_volume() -> f32 {
    MasterVolume::default().0
}

impl Default for PersistedSettings {
    fn default() -> Self {
        Self {
            master_volume: MasterVolume::default().0,
            graphics_quality: GraphicsQuality::default(),
        }
    }
}

impl PersistedSettings {
    /// Snapshot the live resources into a persistable value.
    pub fn from_resources(volume: MasterVolume, quality: GraphicsQuality) -> Self {
        Self {
            master_volume: volume.factor(),
            graphics_quality: quality,
        }
    }
}

/// The saved settings, or `None` if nothing has been saved yet (or the store is
/// unreadable/corrupt). `None` means "use the defaults".
pub fn load_settings() -> Option<PersistedSettings> {
    backend::load()
}

/// Persist the settings. Best-effort - failures are logged, not returned.
pub fn save_settings(settings: &PersistedSettings) {
    backend::save(settings);
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use std::path::{Path, PathBuf};

    use bevy::prelude::*;

    use super::PersistedSettings;

    pub fn load() -> Option<PersistedSettings> {
        load_from(&config_path()?)
    }

    pub fn save(settings: &PersistedSettings) {
        let Some(path) = config_path() else {
            warn!("settings: no config dir available; settings will not persist");
            return;
        };
        save_to(&path, settings);
    }

    /// `<config_dir>/nova-protocol/settings.ron`.
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("nova-protocol").join("settings.ron"))
    }

    /// Read + decode from a RON file. `None` on missing / unreadable / corrupt -
    /// the caller falls back to defaults. Pure (path in), so unit-testable.
    pub fn load_from(path: &Path) -> Option<PersistedSettings> {
        let bytes = std::fs::read(path).ok()?;
        ron::de::from_bytes::<PersistedSettings>(&bytes).ok()
    }

    /// Encode + write as RON, creating the parent dir first. Best-effort.
    pub fn save_to(path: &Path, settings: &PersistedSettings) {
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                warn!("settings: could not create {}: {e}", parent.display());
                return;
            }
        }
        match ron::ser::to_string(settings) {
            Ok(s) => {
                if let Err(e) = std::fs::write(path, s) {
                    warn!("settings: could not write {}: {e}", path.display());
                }
            }
            Err(e) => warn!("settings: could not encode settings: {e}"),
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use bevy::prelude::*;

    use super::PersistedSettings;

    /// The localStorage key. Namespaced so it cannot collide with other app state.
    const KEY: &str = "nova_protocol.settings";

    fn storage() -> Option<web_sys::Storage> {
        // `local_storage()` is `Result<Option<Storage>>`: Err if disabled by the
        // browser (e.g. private mode with storage off), Ok(None) if unavailable.
        web_sys::window()?.local_storage().ok()?
    }

    pub fn load() -> Option<PersistedSettings> {
        let raw = storage()?.get_item(KEY).ok()??;
        ron::de::from_str::<PersistedSettings>(&raw).ok()
    }

    pub fn save(settings: &PersistedSettings) {
        let Some(storage) = storage() else {
            warn!("settings: no localStorage available; settings will not persist");
            return;
        };
        match ron::ser::to_string(settings) {
            Ok(s) => {
                if storage.set_item(KEY, &s).is_err() {
                    warn!("settings: localStorage write failed; settings not saved");
                }
            }
            Err(e) => warn!("settings: could not encode settings: {e}"),
        }
    }
}

// The native file backend is unit-testable via its pure `load_from`/`save_to`.
// The wasm localStorage backend is cfg-guarded and reviewed statically (same as
// mod_prefs): the wasm target is built by neither the local test runner nor the
// PR/master CI - only the manual web-deploy workflow - so static review against
// the web-sys 0.3 API is the guard. Keep it a minimal mirror of the native one.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use nova_gameplay::prelude::GraphicsQuality;

    use super::{
        backend::{load_from, save_to},
        PersistedSettings,
    };

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("nova_settings_{name}/settings.ron"))
    }

    #[test]
    fn save_then_load_round_trips() {
        let path = temp_path("round_trip");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());

        let settings = PersistedSettings {
            master_volume: 0.4,
            graphics_quality: GraphicsQuality::Low,
        };
        save_to(&path, &settings);
        assert!(
            path.exists(),
            "save_to must create the file and its parent dir"
        );
        assert_eq!(
            load_from(&path),
            Some(settings),
            "settings round-trip through RON"
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
            "a missing file reads as no saved settings"
        );
    }

    #[test]
    fn corrupt_file_loads_none() {
        let path = temp_path("corrupt");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"not ron {{{").unwrap();
        assert_eq!(
            load_from(&path),
            None,
            "corrupt data reads as none, not a panic"
        );
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// An older file missing the graphics field still loads (serde default),
    /// so adding a setting never invalidates a saved store.
    #[test]
    fn partial_file_uses_defaults() {
        let path = temp_path("partial");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"(master_volume: 0.5)").unwrap();
        assert_eq!(
            load_from(&path),
            Some(PersistedSettings {
                master_volume: 0.5,
                graphics_quality: GraphicsQuality::default(),
            }),
            "a missing field falls back to its serde default"
        );
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
