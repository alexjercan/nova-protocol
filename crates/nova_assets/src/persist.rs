//! Cross-platform persistence for one small named value, stored as RON.
//!
//! [`load`] and [`save`] are the whole surface: a key names the value,
//! [`storage`] decides where it lands, and both directions are best-effort. A
//! missing or corrupt store reads as `None` ("use the default"); a write
//! failure is logged, never fatal.
//!
//! [`storage`]: crate::storage
//!
//! This module is the CODEC only - RON in, RON out, identical on every target.
//! The platform split lives one layer down behind [`Storage`], so nothing here
//! is cfg-gated and a test can drive the codec against any store.
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

use bevy::prelude::*;
use serde::{de::DeserializeOwned, Serialize};

use crate::storage::{self, Storage};

/// The stored value for `key`, or `None` when nothing has been saved yet or the
/// store is unreadable/corrupt. `None` means "use the default"; `Some` is
/// authoritative.
pub fn load<T: DeserializeOwned>(key: &str) -> Option<T> {
    load_from(&storage::platform()?, key)
}

/// Persist `value` under `key`. Best-effort - failures are logged, not returned.
pub fn save<T: Serialize>(key: &str, value: &T) {
    let Some(store) = storage::platform() else {
        warn!("persist[{key}]: no store available; the value will not persist");
        return;
    };
    save_to(&store, key, value);
}

/// [`load`] against an explicit store. The seam a test reads through so it
/// cannot touch the developer's real config.
pub fn load_from<T: DeserializeOwned>(store: &impl Storage, key: &str) -> Option<T> {
    ron::de::from_bytes::<T>(&store.read(key)?).ok()
}

/// [`save`] against an explicit store; see [`load_from`].
pub fn save_to<T: Serialize>(store: &impl Storage, key: &str, value: &T) {
    match ron::ser::to_string(value) {
        Ok(s) => {
            if let Err(e) = store.write(key, s.as_bytes()) {
                warn!("persist[{key}]: could not write the value: {e}");
            }
        }
        Err(e) => warn!("persist[{key}]: could not encode the value: {e}"),
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{load_from, save_to};
    use crate::storage::NativeStorage;

    /// A unique temp root per test; the test cleans it up.
    fn temp_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("nova_persist_{name}"))
    }

    #[test]
    fn save_then_load_round_trips_the_value() {
        let root = temp_root("round_trip");
        let _ = std::fs::remove_dir_all(&root);
        let store = NativeStorage::at(&root);

        let value = vec!["base".to_string(), "demo".to_string()];
        save_to(&store, "mods", &value);
        assert_eq!(
            load_from::<Vec<String>>(&store, "mods"),
            Some(value),
            "the value round-trips through RON"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_value_loads_none() {
        let root = temp_root("missing");
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(
            load_from::<Vec<String>>(&NativeStorage::at(&root), "mods"),
            None,
            "a missing value reads as nothing saved"
        );
    }

    #[test]
    fn corrupt_value_loads_none() {
        let root = temp_root("corrupt");
        let _ = std::fs::remove_dir_all(&root);
        let store = NativeStorage::at(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(store.path("mods"), b"this is not ron {{{").unwrap();
        assert_eq!(
            load_from::<Vec<String>>(&store, "mods"),
            None,
            "corrupt data reads as nothing saved (not a panic)"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
