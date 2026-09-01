//! Nova's audio engine: every sound in the game, from the menu click to the
//! lance's report, is mixed and played here.
//!
//! # The rule
//!
//! ALL audio goes through this module. Nothing else in the game may build a
//! bevy `AudioPlayer` - a voice spawned outside the engine is unrouted,
//! unattenuated, unpannable and deaf to every setting. The rule is enforced,
//! not just written down: `voice`'s
//! `the_engine_is_the_only_place_an_audio_player_is_built` reads the crates'
//! sources and fails on a second constructor. To make a sound, spawn an
//! [`SfxVoice`] (a loop) or trigger a [`PlaySfx`] (a one-shot).
//!
//! # The buses
//!
//! Every voice declares an [`AudioRoute`], and the route names the one track
//! that scales it. There is no other volume in the system:
//!
//! ```text
//! MasterVolume (settings)  ->  bevy GlobalVolume
//!   Interface  (InterfaceVolume)  UI chrome. Non-positional.
//!   World      (WorldVolume)      Everything diegetic:
//!       Hull       structure-borne through the player's OWN ship
//!       Exterior   everything else out there
//!   Music      (MusicVolume)      RESERVED - nothing routes here yet
//! ```
//!
//! "What scales this cue?" has exactly one answer: its bus volume, times
//! [`MasterVolume`](crate::settings::MasterVolume). The old `SfxMasterVolume`
//! is gone - it was a second master with no way to tell the two apart.
//!
//! # Hull and Exterior
//!
//! The two world routes are one track heard two ways, and the tag IS the
//! attenuation and pan policy:
//!
//! - [`AudioRoute::Hull`] is structure-borne through the player's own ship -
//!   their guns, their engines, their RCS, damage landing on their hull, their
//!   own reload and charge cues. Never distance-attenuated, never panned: it is
//!   the room they are sitting in, not a place out in the world.
//! - [`AudioRoute::Exterior`] is everything else, and is the only route that is
//!   either attenuated or panned.
//!
//! That is a routing fact, not decoration. It REPLACES the old "if this ship is
//! the player, skip the attenuation" special case in the thruster hum: your own
//! engines are Hull by definition, so the exemption stopped being an
//! if-statement. It is also the seam any later "no sound in vacuum" mode would
//! gate on - which is deliberately NOT built here.
//!
//! # The submodules
//!
//! `bus` decides how loud a route may be, `mixing` owns the amplitude law (the
//! listener, the tuned rolloff, the per-source throttle), `spatial` owns the
//! stereo placement, `voice` owns playback, `sfx` is the one-shot front door
//! and `registry` is the keyed handle bank. Nothing here knows what a ship is -
//! the mapping from gameplay events to sounds is `nova_ship`'s `ship_audio`.
//!
//! This stays a MODULE rather than a `nova_sound` crate. It is already
//! self-contained behind its prelude and depends on nothing but bevy and
//! `crate::settings`; lifting it out would buy no capability and cost every
//! import in the game.
//!
//! Everything degrades gracefully: a rig missing the mixer resources or the
//! listener plays at full volume rather than panicking or going silent. The
//! [`SoundBank<UiSfx>`] resource is inserted by `nova_assets` once assets load.
//! World sounds carry no bank at all - each cue resolves its target's authored
//! `AssetRef` (authored-or-silent).

use bevy::prelude::*;

mod bus;
mod mixing;
mod registry;
mod sfx;
mod spatial;
mod voice;

/// Glob-import surface: `use nova_gameplay::audio::prelude::*`.
///
/// The generic engine only. The ship's soundtrack is `nova_ship`'s
/// `ship_audio`, and the mixing internals (`SfxThrottle`, `area_cell`,
/// `distance_attenuation`, the pan math) stay off the boundary - they are the
/// engine's own machinery, re-exported at module root for the tests that pin
/// them.
///
/// The `NOVA_OS_*` cue volumes are on the boundary because every cue volume is
/// defined here while the cues themselves fire from `nova_os_ui`.
pub mod prelude {
    pub use super::{
        sounds_loaded, AudioBus, AudioRoute, InterfaceVolume, MusicVolume, NovaAudioPlugin,
        PlaySfx, SfxAudioMarker, SfxCommandsExt, SfxListenerMarker, SfxPlugin, SfxSource, SfxVoice,
        SoundBank, UiSfx, WorldVolume, MENU_SELECT_VOLUME, NOVA_OS_BACK_VOLUME, NOVA_OS_BED_VOLUME,
        NOVA_OS_COIL_VOLUME, NOVA_OS_ENTER_VOLUME, NOVA_OS_ERROR_VOLUME, NOVA_OS_KEY_MIN_INTERVAL,
        NOVA_OS_KEY_VOLUME, NOVA_OS_OK_VOLUME, NOVA_OS_POWER_VOLUME, NOVA_OS_TICK_VOLUME,
        SALVAGE_PICKUP_VOLUME, UI_SFX_FILES, UI_TOGGLE_VOLUME,
    };
}

pub use self::{
    bus::{bus_gain, AudioBus, AudioRoute, InterfaceVolume, Mixer, MusicVolume, WorldVolume},
    mixing::{
        area_cell, distance_attenuation, listener_position, SfxListenerMarker, SfxThrottle,
        ThrottleKey, SFX_AREA_CELL, SFX_AUDIBLE_THRESHOLD, SFX_FAR_DISTANCE, SFX_NEAR_DISTANCE,
    },
    registry::{sounds_loaded, SoundBank},
    sfx::{PlaySfx, SfxAudioMarker, SfxCommandsExt, SfxPlugin},
    spatial::{
        emitter_point, listener_ears, local_bearing, pan_compensation, pan_gains, SPATIAL_EAR_GAP,
        SPATIAL_EMITTER_RADIUS,
    },
    voice::{SfxSource, SfxVoice, MAX_EXTERIOR_LOOP_VOICES},
};
use self::{
    mixing::prune_sfx_throttle,
    voice::{
        drive_sfx_voices, on_add_listener, pause_world_voices, resume_world_voices,
        start_sfx_voices, stop_world_voices,
    },
};

/// Keys for the game's UI/interface sound effects - engine chrome, like
/// `assets/icons/`: loaded from the root `assets/sounds/`, NOT part of any mod
/// and never referenceable by content. Everything a player would call "the
/// interface" lives here; every world/gameplay sound is mod content, authored
/// on its owning section/object config as an `AssetRef<AudioSource>` field.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum UiSfx {
    /// A new objective was posted to the panel (non-positional).
    ObjectiveNew,
    /// An objective was completed (non-positional).
    ObjectiveComplete,
    /// A menu button was pressed (New Game / Sandbox / Settings / Exit and the
    /// pause/mods buttons) - a crisp UI click. Fired from `nova_menu`'s global
    /// `On<Activate>` observer.
    MenuSelect,
    /// A comms line just SHOWED on the panel - a soft radio blip so a story
    /// beat registers mid-fight. PLACEHOLDER AUDIO: reuses ui_toggle.wav until
    /// real comms art lands (distinct key so the swap is one file-map line).
    CommsLine,
    /// A pause overlay open/close toggle via ESC - a soft two-state UI blip.
    UiToggle,
    // NOVA OS terminal cues: the offline-rendered `nova_*.wav` family
    // (`scripts/gen-nova-os-sfx.py`), mirroring the PoC's WebAudio recipes.
    // Gated on `NovaOsMonitorSettings::sound_enabled` at each fire site.
    /// A keystroke click while typing at the NOVA OS prompt.
    NovaOsKey,
    /// A backspace/delete tick at the NOVA OS prompt.
    NovaOsBack,
    /// The enter "thunk" when a NOVA OS command is submitted.
    NovaOsEnter,
    /// A NOVA OS command succeeded - a soft confirmation beep.
    NovaOsOk,
    /// A NOVA OS command errored (unknown / bad args) - an error buzz.
    NovaOsError,
    /// A NOVA OS Tab completion advanced the prompt - a short tick.
    NovaOsTick,
    /// The degauss coil thump on a NOVA OS app launch/exit.
    NovaOsCoil,
    /// The power-up sweep when the NOVA OS computer opens.
    NovaOsPowerUp,
    /// The power-down sweep when the NOVA OS computer closes.
    NovaOsPowerDown,
    /// The live-tube ambient bed loop while the NOVA OS computer is open.
    NovaOsBed,
}

/// The `(key, base-filename)` pairs for the UI bank. Loaded by
/// `nova_assets::register_sounds` via `SoundBank::load`, whose
/// `sounds/<name>.wav` convention maps these to the root `assets/sounds/` -
/// engine chrome, outside every mod.
pub const UI_SFX_FILES: [(UiSfx, &str); 15] = [
    (UiSfx::ObjectiveNew, "objective_new"),
    (UiSfx::ObjectiveComplete, "objective_complete"),
    (UiSfx::MenuSelect, "menu_select"),
    (UiSfx::UiToggle, "ui_toggle"),
    // Placeholder file (see the key's doc): swap for real comms art.
    (UiSfx::CommsLine, "ui_toggle"),
    // NOVA OS terminal cues.
    (UiSfx::NovaOsKey, "nova_key"),
    (UiSfx::NovaOsBack, "nova_back"),
    (UiSfx::NovaOsEnter, "nova_enter"),
    (UiSfx::NovaOsOk, "nova_ok"),
    (UiSfx::NovaOsError, "nova_error"),
    (UiSfx::NovaOsTick, "nova_tick"),
    (UiSfx::NovaOsCoil, "nova_coil"),
    (UiSfx::NovaOsPowerUp, "nova_powerup"),
    (UiSfx::NovaOsPowerDown, "nova_powerdown"),
    (UiSfx::NovaOsBed, "nova_bed"),
];

/// The salvage-pickup "ding". Deliberately quieter than the objective chime
/// (`OBJECTIVE_COMPLETE_VOLUME` 0.38 / `OBJECTIVE_NEW_VOLUME` 0.30) so a crate
/// pickup reads as a light per-item confirmation, not a beat completion. Fired
/// from `nova_scenario`'s salvage plugin, which owns `SalvageCrateMarker`.
pub const SALVAGE_PICKUP_VOLUME: f32 = 0.22;

/// Volume for the menu click cue, fired from `nova_menu`. Non-positional and
/// in the informational-tick band, like the ship's own cue volumes over in
/// `nova_ship`'s `ship_audio`.
pub const MENU_SELECT_VOLUME: f32 = 0.28;
/// Volume for the HUD-toggle click cue, fired from `nova_menu`.
pub const UI_TOGGLE_VOLUME: f32 = 0.24;
/// NOVA OS terminal cue volumes. The `nova_*.wav` files are peak-normalized to
/// -3 dBFS, so these linear factors are what place each cue in the
/// informational-tick band. Typing is the quietest and is throttled
/// ([`NOVA_OS_KEY_MIN_INTERVAL`]) so a held key cannot machine-gun; the coil
/// and power sweeps are the loudest "moment" cues. `pub` because the cues are
/// fired from `nova_os_ui`, keeping every cue volume defined in this module.
/// One keystroke.
pub const NOVA_OS_KEY_VOLUME: f32 = 0.10;
/// Backspace / delete.
pub const NOVA_OS_BACK_VOLUME: f32 = 0.12;
/// Submitting a command line.
pub const NOVA_OS_ENTER_VOLUME: f32 = 0.18;
/// A command that succeeded.
pub const NOVA_OS_OK_VOLUME: f32 = 0.20;
/// A command that failed.
pub const NOVA_OS_ERROR_VOLUME: f32 = 0.22;
/// A knob detent or selection step.
pub const NOVA_OS_TICK_VOLUME: f32 = 0.12;
/// The degauss coil on a real app exit.
pub const NOVA_OS_COIL_VOLUME: f32 = 0.26;
/// The power-up / power-down sweep.
pub const NOVA_OS_POWER_VOLUME: f32 = 0.30;
/// Base volume of the ambient bed loop, before
/// [`MasterVolume`](crate::settings::MasterVolume). The bed WAV is
/// authored quiet (~-35 dBFS), so this sits it as a soft under-hum.
pub const NOVA_OS_BED_VOLUME: f32 = 0.7;
/// Minimum real seconds between successive typing clicks, so OS key-repeat on a
/// held key does not spawn a storm of click one-shots (the `SfxThrottle`
/// precedent, applied inline with a `Local` since this is one global stream).
pub const NOVA_OS_KEY_MIN_INTERVAL: f32 = 0.03;

/// The audio engine: the buses, the mixer, and the one playback path every
/// sound in the game goes through.
///
/// The cues themselves are added by their own subsystems - the ship's by
/// `nova_ship`'s `ShipAudioPlugin`, the terminal's by `nova_os_ui`.
#[derive(Default)]
pub struct NovaAudioPlugin;

/// The engine's per-frame mixing pass.
///
/// Runs in `PostUpdate` before `TransformSystems::Propagate`, which is what
/// bevy's own audio playback runs AFTER: a voice spawned this frame is placed
/// and levelled before bevy opens its sink, so no cue is ever heard at the
/// wrong volume for a frame. Every owner that drives a loop's level (the ship's
/// thruster and RCS passes, the terminal's bed) writes it in `Update`, upstream
/// of this.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioSystems;

impl Plugin for NovaAudioPlugin {
    fn build(&self, app: &mut App) {
        trace!("NovaAudioPlugin: build");

        // One-shot cues (PlaySfx -> SfxVoice).
        if !app.is_plugin_added::<SfxPlugin>() {
            app.add_plugins(SfxPlugin);
        }

        app.init_resource::<InterfaceVolume>();
        app.init_resource::<WorldVolume>();
        app.init_resource::<MusicVolume>();
        app.init_resource::<SfxThrottle>();
        app.register_type::<InterfaceVolume>();
        app.register_type::<WorldVolume>();
        app.register_type::<MusicVolume>();
        app.register_type::<SfxListenerMarker>();

        app.add_observer(on_add_listener);

        app.configure_sets(
            PostUpdate,
            AudioSystems.before(bevy::transform::TransformSystems::Propagate),
        );
        app.add_systems(
            PostUpdate,
            (start_sfx_voices, drive_sfx_voices)
                .chain()
                .in_set(AudioSystems),
        );

        // Audio sinks do not follow `Time<Virtual>`: without this a loop keeps
        // roaring at its last volume behind a frozen sim. BOTH frozen overlays
        // need it - the pause overlay and the Tab ship-computer NOVA OS.
        app.add_systems(OnEnter(crate::PauseStates::Paused), pause_world_voices);
        app.add_systems(OnExit(crate::PauseStates::Paused), resume_world_voices);
        app.add_systems(OnEnter(crate::PauseStates::NovaOs), pause_world_voices);
        app.add_systems(OnExit(crate::PauseStates::NovaOs), resume_world_voices);
        app.add_systems(OnExit(crate::GameStates::Playing), stop_world_voices);

        // Pure map cleanup; harmless to run always and keeps memory bounded.
        app.add_systems(Update, prune_sfx_throttle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ui_sfx_key_has_a_file() {
        // Guards against adding a UiSfx variant without a placeholder asset.
        use UiSfx::*;
        for key in [
            ObjectiveNew,
            ObjectiveComplete,
            MenuSelect,
            CommsLine,
            UiToggle,
            NovaOsKey,
            NovaOsBack,
            NovaOsEnter,
            NovaOsOk,
            NovaOsError,
            NovaOsTick,
            NovaOsCoil,
            NovaOsPowerUp,
            NovaOsPowerDown,
            NovaOsBed,
        ] {
            assert!(
                UI_SFX_FILES.iter().any(|(k, _)| *k == key),
                "UiSfx::{key:?} is missing from UI_SFX_FILES"
            );
        }
    }
}
