//! The persisted form of the player settings.
//!
//! The settings menu writes a handful of Bevy resources; this module snapshots
//! them into one versionable blob and names the store key. Storage, and its
//! best-effort semantics, belong to [`nova_assets::persist`].
//!
//! Every field carries a serde default, so a store written before a setting
//! existed still loads and picks that setting's default - which is what lets the
//! three mixer-bus volumes join `master_volume` without invalidating anyone's
//! saved file.

use std::collections::BTreeMap;

use nova_assets::persist;
use nova_gameplay::prelude::{
    GraphicsQuality, InterfaceVolume, MasterVolume, MusicVolume, WorldVolume,
};
use nova_input::prelude::{BindingSpec, InputBindings, MousePath, MouseSensitivity};
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
    /// Linear volume of the Interface mixer bus (UI chrome).
    #[serde(default = "default_volume")]
    pub interface_volume: f32,
    /// Linear volume of the World mixer bus (everything diegetic).
    #[serde(default = "default_volume")]
    pub world_volume: f32,
    /// Linear volume of the RESERVED Music mixer bus. Saved so the format does
    /// not break when music lands.
    #[serde(default = "default_volume")]
    pub music_volume: f32,
    /// Raw mouse-look gain (ship steering, free look, turret aim).
    #[serde(default = "default_look_sensitivity")]
    pub mouse_look_sensitivity: f32,
    /// Raw gain of mouse-driven RCS translation.
    #[serde(default = "default_rcs_sensitivity")]
    pub mouse_rcs_sensitivity: f32,
    /// Raw gain of free-camera mouse look.
    #[serde(default = "default_free_camera_sensitivity")]
    pub mouse_free_camera_sensitivity: f32,
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

fn default_look_sensitivity() -> f32 {
    MousePath::Look.default_raw()
}

fn default_rcs_sensitivity() -> f32 {
    MousePath::Rcs.default_raw()
}

fn default_free_camera_sensitivity() -> f32 {
    MousePath::FreeCamera.default_raw()
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
        let sensitivity = MouseSensitivity::default();
        Self {
            master_volume: MasterVolume::default().0,
            interface_volume: InterfaceVolume::default().0,
            world_volume: WorldVolume::default().0,
            music_volume: MusicVolume::default().0,
            mouse_look_sensitivity: sensitivity.look,
            mouse_rcs_sensitivity: sensitivity.rcs,
            mouse_free_camera_sensitivity: sensitivity.free_camera,
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
        interface_volume: InterfaceVolume,
        world_volume: WorldVolume,
        music_volume: MusicVolume,
        sensitivity: MouseSensitivity,
        quality: GraphicsQuality,
        skin: UiSkin,
        monitor: NovaOsMonitorSettings,
        window_mode: WindowModeSetting,
        bindings: &InputBindings,
    ) -> Self {
        Self {
            master_volume: volume.factor(),
            interface_volume: interface_volume.factor(),
            world_volume: world_volume.factor(),
            music_volume: music_volume.factor(),
            mouse_look_sensitivity: sensitivity.raw(MousePath::Look),
            mouse_rcs_sensitivity: sensitivity.raw(MousePath::Rcs),
            mouse_free_camera_sensitivity: sensitivity.raw(MousePath::FreeCamera),
            graphics_quality: quality,
            ui_skin: skin,
            nova_os_bright_detent: monitor.bright_detent,
            nova_os_scan_detent: monitor.scan_detent,
            nova_os_sound_enabled: monitor.sound_enabled,
            window_mode,
            keybinds: bindings.overrides(),
        }
    }

    /// The persisted mouse sensitivities as the live resource, with every
    /// value clamped - a hand-edited or out-of-range number can only ever load
    /// as one the slider could have produced.
    pub fn mouse_sensitivity(&self) -> MouseSensitivity {
        let mut sensitivity = MouseSensitivity::default();
        sensitivity.set_raw(MousePath::Look, self.mouse_look_sensitivity);
        sensitivity.set_raw(MousePath::Rcs, self.mouse_rcs_sensitivity);
        sensitivity.set_raw(MousePath::FreeCamera, self.mouse_free_camera_sensitivity);
        sensitivity
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
    use nova_gameplay::prelude::{GraphicsQuality, InterfaceVolume, MusicVolume, WorldVolume};
    use nova_input::prelude::{MousePath, MouseSensitivity};
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
            interface_volume: 0.6,
            world_volume: 0.7,
            music_volume: 0.2,
            mouse_look_sensitivity: MousePath::Look.range().raw(150.0),
            mouse_rcs_sensitivity: MousePath::Rcs.range().raw(300.0),
            mouse_free_camera_sensitivity: MousePath::FreeCamera.range().raw(250.0),
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

    /// An older file carrying only `master_volume` still loads (serde
    /// defaults), so adding a setting - the three mixer buses included - never
    /// invalidates a saved store.
    #[test]
    fn partial_file_uses_defaults() {
        let store = temp_store("partial");
        clear(&store);
        write_raw(&store, b"(master_volume: 0.5)");
        assert_eq!(
            load_from::<PersistedSettings>(&store, KEY),
            Some(PersistedSettings {
                master_volume: 0.5,
                interface_volume: InterfaceVolume::default().0,
                world_volume: WorldVolume::default().0,
                music_volume: MusicVolume::default().0,
                mouse_look_sensitivity: MousePath::Look.default_raw(),
                mouse_rcs_sensitivity: MousePath::Rcs.default_raw(),
                mouse_free_camera_sensitivity: MousePath::FreeCamera.default_raw(),
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

    /// The three mixer-bus volumes persist, and a store written before they
    /// existed still loads on their defaults - the compatibility the whole
    /// serde-default rule buys, checked on the setting that just used it.
    #[test]
    fn the_mixer_bus_volumes_persist_and_an_older_store_still_loads() {
        let store = temp_store("bus_volumes");
        clear(&store);

        let settings = PersistedSettings {
            interface_volume: 0.35,
            world_volume: 0.85,
            music_volume: 0.15,
            ..PersistedSettings::default()
        };
        save_to(&store, KEY, &settings);
        let saved = load_from::<PersistedSettings>(&store, KEY).expect("the store round-trips");
        assert_eq!(
            (
                saved.interface_volume,
                saved.world_volume,
                saved.music_volume
            ),
            (0.35, 0.85, 0.15)
        );

        // A store written before the buses existed: every one falls back to its
        // default rather than failing the load.
        write_raw(&store, b"(master_volume: 0.5)");
        let older =
            load_from::<PersistedSettings>(&store, KEY).expect("an older store still loads");
        assert_eq!(older.master_volume, 0.5);
        assert_eq!(
            (
                older.interface_volume,
                older.world_volume,
                older.music_volume
            ),
            (
                InterfaceVolume::default().0,
                WorldVolume::default().0,
                MusicVolume::default().0
            )
        );
        clear(&store);
    }

    /// The three mouse sensitivities persist as RAW gains, a store written
    /// before they existed loads on their defaults, and a number no slider
    /// could have produced is clamped on the way back into the resource.
    #[test]
    fn the_mouse_sensitivities_persist_default_and_clamp() {
        let store = temp_store("mouse_sensitivity");
        clear(&store);

        let non_default = PersistedSettings {
            mouse_look_sensitivity: MousePath::Look.range().raw(120.0),
            mouse_rcs_sensitivity: MousePath::Rcs.range().raw(420.0),
            mouse_free_camera_sensitivity: MousePath::FreeCamera.range().raw(280.0),
            ..PersistedSettings::default()
        };
        save_to(&store, KEY, &non_default);
        let saved = load_from::<PersistedSettings>(&store, KEY).expect("the store round-trips");
        let live = saved.mouse_sensitivity();
        assert!((live.percent(MousePath::Look) - 120.0).abs() < 1e-2);
        assert!((live.percent(MousePath::Rcs) - 420.0).abs() < 1e-2);
        assert!((live.percent(MousePath::FreeCamera) - 280.0).abs() < 1e-2);

        // A pre-sensitivity store: every path falls back to its default rather
        // than failing the load, so adding the setting invalidated nobody's
        // saved file.
        write_raw(&store, b"(master_volume: 0.5)");
        let older =
            load_from::<PersistedSettings>(&store, KEY).expect("a pre-sensitivity store loads");
        assert_eq!(older.master_volume, 0.5);
        assert_eq!(older.mouse_sensitivity(), MouseSensitivity::default());

        // A hand-edited store, well past both ends and one value not a number
        // at all: what reaches the resource is always inside the slider's own
        // range.
        write_raw(
            &store,
            b"(mouse_look_sensitivity: 12.0, mouse_rcs_sensitivity: -3.0, \
              mouse_free_camera_sensitivity: NaN)",
        );
        let corrupt = load_from::<PersistedSettings>(&store, KEY)
            .expect("a corrupt-but-parsable store still loads")
            .mouse_sensitivity();
        assert!(
            (corrupt.percent(MousePath::Look) - 300.0).abs() < 1e-2,
            "clamped to the top"
        );
        assert!(
            (corrupt.percent(MousePath::Rcs) - 100.0).abs() < 1e-2,
            "clamped to the bottom"
        );
        assert_eq!(
            corrupt.free_camera,
            MousePath::FreeCamera.default_raw(),
            "a value that is not a number reads as the default"
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
