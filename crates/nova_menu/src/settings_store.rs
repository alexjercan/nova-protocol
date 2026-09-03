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

use bevy::prelude::*;
use nova_assets::persist;
use nova_gameplay::prelude::{
    harness_env_active, GraphicsQuality, InterfaceVolume, MasterVolume, MusicVolume, WorldVolume,
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

// Every field defaults through the `#[serde(default = ...)]` fn beside it, so
// the value a fresh install starts on and the value an older store falls back
// to are the same one number, named once.
impl Default for PersistedSettings {
    fn default() -> Self {
        Self {
            master_volume: default_volume(),
            interface_volume: default_volume(),
            world_volume: default_volume(),
            music_volume: default_volume(),
            mouse_look_sensitivity: default_look_sensitivity(),
            mouse_rcs_sensitivity: default_rcs_sensitivity(),
            mouse_free_camera_sensitivity: default_free_camera_sensitivity(),
            graphics_quality: GraphicsQuality::default(),
            ui_skin: UiSkin::default(),
            nova_os_bright_detent: default_bright_detent(),
            nova_os_scan_detent: default_scan_detent(),
            nova_os_sound_enabled: default_sound_enabled(),
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

/// Where the settings file lands.
///
/// `None` is the player's own store: the platform config directory (or
/// `$NOVA_CONFIG_ROOT`) on native, the origin's localStorage on the web.
///
/// A root exists so a test can drive the load and save paths without moving
/// `NOVA_CONFIG_ROOT`, which is process-wide: a fixture that repoints it is
/// also repointing every fixture running beside it, which is how a settings
/// test once made an unrelated panel test read a look sensitivity of 300
/// percent.
#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct SettingsStoreRoot(pub Option<std::path::PathBuf>);

impl SettingsStoreRoot {
    /// The store this root names, or `None` when the platform offers none.
    fn store(&self) -> Option<nova_assets::storage::PlatformStorage> {
        nova_assets::storage::platform_at(self.0.as_deref())
    }
}

/// How far this app's store reaches into the player's settings file.
///
/// Reading and writing are separate powers, because the apps that want them
/// are different apps. EVERY app reads, so an example that supplies its own
/// game plugins still flies on the player's own sensitivity, keybinds and
/// quality preset. Only an app that carries a settings PANEL writes, because a
/// panel is the only place a player asks for a change to be kept - a bench
/// that pokes `GraphicsQuality` on a keypress is not.
///
/// One enum and not two flags: "writes but does not read" is the combination
/// that must not exist, because it saves the defaults the run started on over
/// whatever the player had.
///
/// Inserted either way, so a scripted run can ASSERT that its store is inert
/// rather than trust that it is: a run whose store went live would start from
/// the developer's own keybinds and end by overwriting them, and both halves of
/// that are silent.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsStoreAccess {
    /// Neither direction: every setting holds its default for the whole run
    /// and no file is touched. What a scripted run gets, and the default here
    /// because a store nobody configured must not write.
    #[default]
    Inert,
    /// Read at startup, never written. What an app with no settings panel
    /// gets.
    Read,
    /// Read at startup and written back as the player edits.
    ReadWrite,
}

impl SettingsStoreAccess {
    /// Whether the store is read into the live resources at startup.
    pub fn reads(self) -> bool {
        !matches!(self, Self::Inert)
    }

    /// Whether an edit reaches the file.
    pub fn writes(self) -> bool {
        matches!(self, Self::ReadWrite)
    }
}

/// Run condition on both write systems, so the write direction can be granted
/// after the plugin is built - which is what [`allow_settings_saves`] does.
fn the_store_writes(access: Res<SettingsStoreAccess>) -> bool {
    access.writes()
}

/// Grant this app the write direction.
///
/// Called by [`NovaMenuPlugin`](crate::NovaMenuPlugin): a settings panel is
/// what makes a setting worth keeping, so it is what turns a reading store
/// into a writing one. An INERT store stays inert - a scripted run must not
/// start saving because it happened to build a menu.
///
/// # Panics
///
/// If no [`SettingsStorePlugin`] has been added. Every caller adds it first,
/// under its own guard; a silent no-op here would be a menu that cannot save.
pub fn allow_settings_saves(app: &mut App) {
    let mut access = app.world_mut().resource_mut::<SettingsStoreAccess>();
    if *access == SettingsStoreAccess::Read {
        *access = SettingsStoreAccess::ReadWrite;
    }
}

/// The saved settings, or `None` if nothing has been saved yet (or the store is
/// unreadable/corrupt). `None` means "use the defaults".
pub fn load_settings(root: &SettingsStoreRoot) -> Option<PersistedSettings> {
    persist::load_from(&root.store()?, KEY)
}

/// Persist the settings. Best-effort - failures are logged, not returned.
pub fn save_settings(root: &SettingsStoreRoot, settings: &PersistedSettings) {
    let Some(store) = root.store() else {
        warn!("persist[{KEY}]: no store available; the value will not persist");
        return;
    };
    persist::save_to(&store, KEY, settings);
}

/// Reads the store into the live settings resources at startup and, where the
/// app may, writes it back as they change.
///
/// Separate from [`NovaMenuPlugin`](crate::NovaMenuPlugin) because a settings
/// PANEL is not what makes a setting apply. `AppBuilder` adds this to every
/// app, so an example that supplies its own game plugins and never builds a
/// menu still flies on the player's own mouse sensitivity, keybinds, volumes
/// and quality preset instead of silently on the defaults - while staying
/// unable to write any of them back, because it has nowhere for a player to
/// ask for that ([`SettingsStoreAccess`]).
///
/// Owns the settings-backed resources as well as both directions, so the
/// plugin stands alone: an app with this and nothing else has a complete,
/// loaded settings state.
pub struct SettingsStorePlugin {
    /// How far this app's store reaches; see [`SettingsStoreAccess`].
    ///
    /// The write direction can also be granted afterwards, by
    /// [`allow_settings_saves`], which is how the menu upgrades the store
    /// `AppBuilder` already gave the app.
    pub access: SettingsStoreAccess,
    /// Where the file lands; see [`SettingsStoreRoot`]. `None` is the player's
    /// own store, which is what every shipped app wants.
    pub root: Option<std::path::PathBuf>,
}

impl SettingsStorePlugin {
    /// A store that READS for a human at the keyboard and is INERT under a
    /// scripted run ([`harness_env_active`]).
    ///
    /// Reading only, because this is what every app gets: an app earns the
    /// write direction by carrying a settings panel, and says so with
    /// [`allow_settings_saves`].
    ///
    /// A capture or a probe sweep must produce the same frames and the same
    /// numbers on any machine, and the developer's own graphics preset, skin
    /// or window mode would otherwise decide what a screenshot shows. The
    /// write direction matters more: a scripted run that saves is a run that
    /// rewrites the settings of whoever launched it, which is how a screenshot
    /// pass once overwrote a keybind table (see `tests::support`).
    ///
    /// NATIVE only: a wasm build has no process environment, so `var_os` is
    /// always `None` and this always reads. A web app that must be inert -
    /// `nova_perf_web` - says so with the field instead.
    pub fn from_env() -> Self {
        Self {
            access: if harness_env_active() {
                SettingsStoreAccess::Inert
            } else {
                SettingsStoreAccess::Read
            },
            root: None,
        }
    }
}

impl Plugin for SettingsStorePlugin {
    fn build(&self, app: &mut App) {
        // Every one of these is owned by some other plugin in the assembled
        // app - the mixer buses and the quality preset by `NovaGameplayPlugin`,
        // the sensitivities by `NovaInputPlugin`, the skin by `NovaUiPlugin`,
        // the monitor knobs by `NovaOsUiPlugin`. `init_resource` is idempotent,
        // so initing them here as well is what lets this plugin be added first,
        // last, or alone.
        app.init_resource::<MasterVolume>();
        app.init_resource::<InterfaceVolume>();
        app.init_resource::<WorldVolume>();
        app.init_resource::<MusicVolume>();
        app.init_resource::<MouseSensitivity>();
        app.init_resource::<GraphicsQuality>();
        app.init_resource::<UiSkin>();
        app.init_resource::<NovaOsMonitorSettings>();
        app.init_resource::<WindowModeSetting>();
        // The keybind overrides land on the same table every rig is built
        // from, so the load needs it present even in an app that has not added
        // `NovaInputPlugin` yet.
        app.init_resource::<InputBindings>();

        app.insert_resource(SettingsStoreRoot(self.root.clone()));
        app.insert_resource(self.access);

        if !self.access.reads() {
            return;
        }
        app.add_systems(Startup, load_persisted_settings);
        app.init_resource::<PendingSettingsSave>();
        // Registered for any READING store and gated on the write direction,
        // because the menu grants that while it builds - which is after this.
        app.add_systems(Update, persist_settings_on_change.run_if(the_store_writes));
        app.add_systems(Last, flush_settings_on_exit.run_if(the_store_writes));
        // Behind the same switch as the load: the mode only ever arrives from
        // the store, so an inert run has no mode to apply and must leave the
        // harness's window alone.
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, apply_window_mode);
    }
}

/// Load the persisted settings once at startup and write them into the live
/// resources. A missing/corrupt store is a no-op (the resources keep their
/// defaults). Runs before the first `Update`, so nova_gameplay's apply systems
/// (gated on `resource_changed`) push the loaded values onto the engine on the
/// first frame.
pub(crate) fn load_persisted_settings(
    mut volume: ResMut<MasterVolume>,
    mut interface_volume: ResMut<InterfaceVolume>,
    mut world_volume: ResMut<WorldVolume>,
    mut music_volume: ResMut<MusicVolume>,
    mut sensitivity: ResMut<MouseSensitivity>,
    mut quality: ResMut<GraphicsQuality>,
    mut skin: ResMut<UiSkin>,
    mut monitor: ResMut<NovaOsMonitorSettings>,
    mut window_mode: ResMut<WindowModeSetting>,
    mut bindings: ResMut<InputBindings>,
    root: Res<SettingsStoreRoot>,
) {
    let Some(saved) = load_settings(&root) else {
        return;
    };
    *volume = MasterVolume(saved.master_volume.clamp(0.0, 1.0));
    *interface_volume = InterfaceVolume(saved.interface_volume.clamp(0.0, 1.0));
    *world_volume = WorldVolume(saved.world_volume.clamp(0.0, 1.0));
    *music_volume = MusicVolume(saved.music_volume.clamp(0.0, 1.0));
    *sensitivity = saved.mouse_sensitivity();
    *quality = saved.graphics_quality;
    *skin = saved.ui_skin;
    *monitor = saved.nova_os_monitor();
    *window_mode = saved.window_mode;
    // Before the first rig is built: the flight rig spawns with the player
    // ship, which is a scenario away, so a saved keybind is on the table by
    // the time anything reads it.
    bindings.apply_overrides(&saved.keybinds);
}

/// Put the chosen window mode on the primary window. Native only - see
/// [`WindowModeSetting`].
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn apply_window_mode(
    setting: Res<WindowModeSetting>,
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
) {
    use bevy::window::{MonitorSelection, WindowMode};

    if !setting.is_changed() {
        return;
    }
    let mode = match *setting {
        WindowModeSetting::Windowed => WindowMode::Windowed,
        WindowModeSetting::Borderless => {
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        }
    };
    for mut window in &mut windows {
        if window.mode != mode {
            window.mode = mode;
        }
    }
}

/// Idle frames a settings value must hold steady before it is written to disk.
/// Debounces the volume slider, whose drag mutates `MasterVolume` every frame:
/// without this, one drag would trigger a full config write per frame. ~0.25s at
/// 60fps - imperceptible for a settings save, and it collapses a whole drag (or
/// a track-click, which emits no final `ValueChange`) into a single write.
pub(crate) const SETTINGS_SAVE_DEBOUNCE_FRAMES: u32 = 15;

/// Persist the settings a short beat after the player stops editing. Any change
/// (re)arms the debounce; the save fires once the value has held steady for
/// [`SETTINGS_SAVE_DEBOUNCE_FRAMES`]. The initial add (startup load /
/// `init_resource`) is skipped via `is_added`, so a launch that changes nothing
/// never arms the debounce and never rewrites the store. `Local` holds the idle
/// countdown: `None` = nothing pending, `Some(n)` = `n` idle frames so far.
pub(crate) fn persist_settings_on_change(
    settings: LiveSettings,
    mut pending: ResMut<PendingSettingsSave>,
    root: Res<SettingsStoreRoot>,
) {
    if settings.edited() {
        // A fresh edit: (re)start the debounce, coalescing a drag's per-frame
        // changes into one pending save.
        pending.idle_frames = Some(0);
        return;
    }
    if let Some(frames) = pending.idle_frames {
        if frames + 1 >= SETTINGS_SAVE_DEBOUNCE_FRAMES {
            // NOTE: `save_settings` fsyncs on the calling thread, and this
            // debounce expires ~0.25s after the player closes the pause menu -
            // inside the first gameplay frames after Resume. Accepted while the
            // blob is this small; the fix, if a frame ever shows it, is the
            // `IoTaskPool` or a flush on `OnExit(Paused)`.
            save_settings(&root, &settings.snapshot());
            pending.idle_frames = None;
        } else {
            pending.idle_frames = Some(frames + 1);
        }
    }
}

/// Every resource the store holds, as one system parameter: the two systems
/// that write the file both need all of them, and a settings added to one and
/// not the other is how a value silently stops being saved.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct LiveSettings<'w> {
    volume: Res<'w, MasterVolume>,
    interface_volume: Res<'w, InterfaceVolume>,
    world_volume: Res<'w, WorldVolume>,
    music_volume: Res<'w, MusicVolume>,
    sensitivity: Res<'w, MouseSensitivity>,
    quality: Res<'w, GraphicsQuality>,
    skin: Res<'w, UiSkin>,
    monitor: Res<'w, NovaOsMonitorSettings>,
    window_mode: Res<'w, WindowModeSetting>,
    bindings: Res<'w, InputBindings>,
}

impl LiveSettings<'_> {
    /// Whether the player moved something this frame. The initial add (startup
    /// load / `init_resource`) does not count: a launch that changes nothing
    /// must not rewrite the store.
    fn edited(&self) -> bool {
        let moved = |changed: bool, added: bool| changed && !added;
        moved(self.volume.is_changed(), self.volume.is_added())
            || moved(
                self.interface_volume.is_changed(),
                self.interface_volume.is_added(),
            )
            || moved(self.world_volume.is_changed(), self.world_volume.is_added())
            || moved(self.music_volume.is_changed(), self.music_volume.is_added())
            || moved(self.sensitivity.is_changed(), self.sensitivity.is_added())
            || moved(self.quality.is_changed(), self.quality.is_added())
            || moved(self.skin.is_changed(), self.skin.is_added())
            || moved(self.monitor.is_changed(), self.monitor.is_added())
            || moved(self.window_mode.is_changed(), self.window_mode.is_added())
            || moved(self.bindings.is_changed(), self.bindings.is_added())
    }

    /// The persistable form of what is live right now.
    fn snapshot(&self) -> PersistedSettings {
        PersistedSettings::from_resources(
            *self.volume,
            *self.interface_volume,
            *self.world_volume,
            *self.music_volume,
            *self.sensitivity,
            *self.quality,
            *self.skin,
            *self.monitor,
            *self.window_mode,
            &self.bindings,
        )
    }
}

/// The debounce countdown, as a resource rather than a `Local` so
/// [`flush_settings_on_exit`] can see that a write is owed. `None` = nothing
/// pending, `Some(n)` = `n` idle frames so far.
#[derive(Resource, Default)]
pub(crate) struct PendingSettingsSave {
    idle_frames: Option<u32>,
}

/// Write an owed settings save before the process goes away.
///
/// The debounce is [`SETTINGS_SAVE_DEBOUNCE_FRAMES`] (~0.25s) and the Exit
/// button writes [`AppExit`] the same frame it is clicked, so a value edited
/// just before quitting is otherwise lost. Runs in `Last`, which the app
/// runner drains `AppExit` after.
pub(crate) fn flush_settings_on_exit(
    mut exits: MessageReader<AppExit>,
    settings: LiveSettings,
    mut pending: ResMut<PendingSettingsSave>,
    root: Res<SettingsStoreRoot>,
) {
    if exits.is_empty() || pending.idle_frames.is_none() {
        return;
    }
    exits.clear();
    save_settings(&root, &settings.snapshot());
    pending.idle_frames = None;
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
