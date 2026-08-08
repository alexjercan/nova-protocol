//! Persistence of the enabled-mods set: the enabled ids as a RON `Vec<String>`.
//!
//! Storage, and its best-effort semantics, belong to [`crate::persist`]; this
//! module owns only the key and the value type.

/// Glob-import surface: `use nova_assets::mod_prefs::prelude::*` re-exports the
/// public API of this module.
pub mod prelude {
    pub use super::{load_enabled_ids, save_enabled_ids};
}

use crate::persist;

/// The store key: `<config_dir>/nova-protocol/enabled_mods.ron` on native,
/// `nova_protocol.enabled_mods` in localStorage on the web.
const KEY: &str = "enabled_mods";

/// The saved enabled mod ids, or `None` if nothing has been saved yet (or the store
/// is unreadable/corrupt). `None` means "use the default"; `Some` is authoritative.
pub fn load_enabled_ids() -> Option<Vec<String>> {
    persist::load(KEY)
}

/// Persist the enabled mod ids. Best-effort - failures are logged, not returned.
pub fn save_enabled_ids(ids: &[String]) {
    persist::save(KEY, &ids);
}
