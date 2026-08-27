//! The persisted form of the player settings.
//!
//! The settings menu writes four Bevy resources; this module snapshots them into
//! one versionable blob and names the store key. Storage, and its best-effort
//! semantics, belong to [`nova_assets::persist`].

use std::collections::BTreeMap;

use nova_assets::persist;
use nova_gameplay::prelude::{GraphicsQuality, MasterVolume};
use nova_input::prelude::{BindingSpec, InputBindings};
use nova_os_ui::prelude::NovaOsMonitorSettings;
use nova_ui::prelude::UiSkin;
use serde::{Deserialize, Serialize};

use crate::settings::WindowModeSetting;

/// The persisted form of the settings: plain, versionable data decoupled from
/// the live resources. Missing/extra fields are tolerated by serde defaults so
/// an older or newer file still loads.
/// Not `Copy`: `keybinds` owns heap data. Nothing needed the bound - the
/// value is built once per save and read through `&self` - so dropping it
/// cost no call site.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct PersistedSettings {
    /// Linear master volume `0.0..=1.0`.
    #[serde(default = "default_volume")]
    pub master_volume: f32,
    /// The graphics-quality preset.
    #[serde(default)]
    pub graphics_quality: GraphicsQuality,
    /// The UI skin (phosphor terminal vs hardware casing).
    #[serde(default)]
    pub ui_skin: UiSkin,
    /// NOVA OS BRIGHT knob detent.
    #[serde(default = "default_bright_detent")]
    pub nova_os_bright_detent: usize,
    /// NOVA OS SCAN knob detent.
    #[serde(default = "default_scan_detent")]
    pub nova_os_scan_detent: usize,
    /// NOVA OS SND speaker toggle (default ON).
    #[serde(default = "default_sound_enabled")]
    pub nova_os_sound_enabled: bool,
    /// Windowed or borderless fullscreen. Written on every platform; only the
    /// native build has a row that moves it.
    #[serde(default)]
    pub window_mode: WindowModeSetting,
    /// Keybinds the player moved, by action name. Only the CHANGED rows are
    /// here, so a default the game later moves reaches a player who never
    /// touched that row.
    #[serde(default)]
    pub keybinds: BTreeMap<String, BindingSpec>,
}

fn default_volume() -> f32 {
    MasterVolume::default().0
}

fn default_bright_detent() -> usize {
    NovaOsMonitorSettings::default().bright_detent
}

fn default_scan_detent() -> usize {
    NovaOsMonitorSettings::default().scan_detent
}

fn default_sound_enabled() -> bool {
    NovaOsMonitorSettings::default().sound_enabled
}

impl Default for PersistedSettings {
    fn default() -> Self {
        let monitor = NovaOsMonitorSettings::default();
        Self {
            master_volume: MasterVolume::default().0,
            graphics_quality: GraphicsQuality::default(),
            ui_skin: UiSkin::default(),
            nova_os_bright_detent: monitor.bright_detent,
            nova_os_scan_detent: monitor.scan_detent,
            nova_os_sound_enabled: monitor.sound_enabled,
            window_mode: WindowModeSetting::default(),
            keybinds: BTreeMap::new(),
        }
    }
}

impl PersistedSettings {
    /// Snapshot the live resources into a persistable value.
    pub fn from_resources(
        volume: MasterVolume,
        quality: GraphicsQuality,
        skin: UiSkin,
        monitor: NovaOsMonitorSettings,
        window_mode: WindowModeSetting,
        bindings: &InputBindings,
    ) -> Self {
        Self {
            master_volume: volume.factor(),
            graphics_quality: quality,
            ui_skin: skin,
            nova_os_bright_detent: monitor.bright_detent,
            nova_os_scan_detent: monitor.scan_detent,
            nova_os_sound_enabled: monitor.sound_enabled,
            window_mode,
            keybinds: bindings.overrides(),
        }
    }

    /// The persisted NOVA OS monitor settings as the live resource.
    pub fn nova_os_monitor(&self) -> NovaOsMonitorSettings {
        let mut monitor = NovaOsMonitorSettings {
            bright_detent: self.nova_os_bright_detent,
            scan_detent: self.nova_os_scan_detent,
            sound_enabled: self.nova_os_sound_enabled,
        };
        // Clamp like the volume beside it: a corrupt index survives the read
        // paths (they clamp) but breaks the wrapping knob cycle.
        monitor.clamp_detents();
        monitor
    }
}

/// The store key: `<config_dir>/nova-protocol/settings.ron` on native,
/// `nova_protocol.settings` in localStorage on the web.
pub(crate) const KEY: &str = "settings";

/// The saved settings, or `None` if nothing has been saved yet (or the store is
/// unreadable/corrupt). `None` means "use the defaults".
pub fn load_settings() -> Option<PersistedSettings> {
    persist::load(KEY)
}

/// Persist the settings. Best-effort - failures are logged, not returned.
pub fn save_settings(settings: &PersistedSettings) {
    persist::save(KEY, settings);
}

// The storage backends live in `nova_assets::persist` and are tested there. What
// is left to pin here is the VALUE: that every field round-trips, and that an
// older store missing a field still loads on its serde default - so adding a
// setting never invalidates a player's saved store.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::collections::BTreeMap;

    use nova_assets::{
        persist::{load_from, save_to},
        storage::NativeStorage,
    };
    use nova_gameplay::prelude::GraphicsQuality;
    use nova_os_ui::prelude::NovaOsMonitorSettings;
    use nova_ui::prelude::UiSkin;

    use super::{PersistedSettings, KEY};
    use crate::settings::WindowModeSetting;

    fn temp_store(name: &str) -> NativeStorage {
        NativeStorage::at(std::env::temp_dir().join(format!("nova_settings_{name}")))
    }

    fn clear(store: &NativeStorage) {
        let _ = std::fs::remove_dir_all(store.path(KEY).parent().unwrap());
    }

    /// Plant bytes the codec must survive, bypassing it.
    fn write_raw(store: &NativeStorage, bytes: &[u8]) {
        let path = store.path(KEY);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
    }

    #[test]
    fn save_then_load_round_trips() {
        let store = temp_store("round_trip");
        clear(&store);

        // Non-default monitor detents + SND off, so the round-trip proves the NOVA OS
        // chin fields persist.
        let settings = PersistedSettings {
            master_volume: 0.4,
            graphics_quality: GraphicsQuality::Low,
            ui_skin: UiSkin::Hardware,
            nova_os_bright_detent: 3,
            nova_os_scan_detent: 0,
            nova_os_sound_enabled: false,
            window_mode: WindowModeSetting::Borderless,
            keybinds: BTreeMap::new(),
        };
        save_to(&store, KEY, &settings);
        assert!(
            store.path(KEY).exists(),
            "save_to must create the file and its parent dir"
        );
        assert_eq!(
            load_from::<PersistedSettings>(&store, KEY),
            Some(settings.clone()),
            "settings round-trip through RON"
        );
        assert_eq!(
            load_from::<PersistedSettings>(&store, KEY)
                .unwrap()
                .nova_os_monitor(),
            NovaOsMonitorSettings {
                bright_detent: 3,
                scan_detent: 0,
                sound_enabled: false,
            },
            "the persisted NOVA OS fields rebuild the live monitor resource"
        );

        clear(&store);
    }

    #[test]
    fn missing_file_loads_none() {
        let store = temp_store("missing");
        clear(&store);
        assert_eq!(
            load_from::<PersistedSettings>(&store, KEY),
            None,
            "a missing file reads as no saved settings"
        );
    }

    #[test]
    fn corrupt_file_loads_none() {
        let store = temp_store("corrupt");
        clear(&store);
        write_raw(&store, b"not ron {{{");
        assert_eq!(
            load_from::<PersistedSettings>(&store, KEY),
            None,
            "corrupt data reads as none, not a panic"
        );
        clear(&store);
    }

    /// An older file missing the graphics field still loads (serde default),
    /// so adding a setting never invalidates a saved store.
    #[test]
    fn partial_file_uses_defaults() {
        let store = temp_store("partial");
        clear(&store);
        write_raw(&store, b"(master_volume: 0.5)");
        assert_eq!(
            load_from::<PersistedSettings>(&store, KEY),
            Some(PersistedSettings {
                master_volume: 0.5,
                graphics_quality: GraphicsQuality::default(),
                ui_skin: UiSkin::default(),
                nova_os_bright_detent: NovaOsMonitorSettings::default().bright_detent,
                nova_os_scan_detent: NovaOsMonitorSettings::default().scan_detent,
                nova_os_sound_enabled: NovaOsMonitorSettings::default().sound_enabled,
                window_mode: WindowModeSetting::default(),
                keybinds: BTreeMap::new(),
            }),
            "a missing field falls back to its serde default"
        );
        clear(&store);
    }

    /// The UI skin choice survives a save/load round-trip (DoD 2). Default is Phosphor,
    /// so a Hardware choice is the non-default proof; and an older store lacking the
    /// field defaults to Phosphor rather than failing to load.
    #[test]
    fn ui_skin_setting_persists_across_save_load() {
        let store = temp_store("ui_skin");
        clear(&store);

        let settings = PersistedSettings {
            ui_skin: UiSkin::Hardware,
            ..PersistedSettings::default()
        };
        save_to(&store, KEY, &settings);
        assert_eq!(
            load_from::<PersistedSettings>(&store, KEY).map(|s| s.ui_skin),
            Some(UiSkin::Hardware),
            "the Hardware skin choice round-trips through the store"
        );

        // A pre-skin store still loads, defaulting the skin to Phosphor.
        write_raw(&store, b"(master_volume: 0.5)");
        assert_eq!(
            load_from::<PersistedSettings>(&store, KEY).map(|s| s.ui_skin),
            Some(UiSkin::Phosphor),
            "a store written before the ui_skin field defaults to Phosphor"
        );
        clear(&store);
    }

    /// A moved keybind survives the round-trip, and a store written before
    /// keybinds were persisted still loads - which is the whole reason only
    /// the CHANGED rows go in the file.
    #[test]
    fn a_moved_keybind_persists_and_an_older_store_still_loads() {
        use nova_input::prelude::{BindingSpec, InputSource};

        let store = temp_store("keybinds");
        clear(&store);

        let mut keybinds = BTreeMap::new();
        keybinds.insert(
            "main_drive".to_string(),
            BindingSpec {
                keyboard: vec![InputSource::Keyboard(bevy::prelude::KeyCode::KeyJ)],
                gamepad: vec![],
            },
        );
        let settings = PersistedSettings {
            keybinds: keybinds.clone(),
            ..PersistedSettings::default()
        };
        save_to(&store, KEY, &settings);
        assert_eq!(
            load_from::<PersistedSettings>(&store, KEY).map(|s| s.keybinds),
            Some(keybinds),
            "the moved row round-trips through RON"
        );

        write_raw(&store, b"(master_volume: 0.5)");
        assert_eq!(
            load_from::<PersistedSettings>(&store, KEY).map(|s| s.keybinds),
            Some(BTreeMap::new()),
            "a store written before keybinds reads as no overrides"
        );
        clear(&store);
    }
}
