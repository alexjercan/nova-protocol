//! Nova's audio engine: every sound in the game, from the menu click to the
//! lance's report, is mixed and played here.
//!
//! # The rule
//!
//! ALL audio goes through this crate. Nothing else in the game may build a
//! bevy `AudioPlayer` - a voice spawned outside the engine is unrouted,
//! unattenuated, unpannable and deaf to every setting. The rule is enforced,
//! not just written down: `voice`'s
//! `the_engine_is_the_only_place_an_audio_player_is_built` reads the
//! workspace's sources and fails on a constructor outside this crate. To make a
//! sound, spawn an [`SfxVoice`] (a loop) or trigger a [`PlaySfx`] (a one-shot).
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
//! ```
//!
//! "What scales this cue?" has exactly one answer: its bus volume, times
//! [`MasterVolume`]. The old `SfxMasterVolume` is gone - it was a second master
//! with no way to tell the two apart.
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
//! # The modules
//!
//! `bus` decides how loud a route may be, `master` owns the master volume and
//! the harness mute, `loudness` measures every clip as it loads, `mixing` owns
//! the amplitude law (the listener, the tuned rolloff, the per-source
//! throttle), `spatial` owns the stereo placement, `voice` owns playback, `sfx` is the one-shot front door and `registry` is the
//! keyed handle bank. Nothing here knows what a ship is - the mapping from
//! gameplay events to sounds is `nova_ship`'s `ship_audio`.
//!
//! A crate of its own, below `nova_gameplay`, so a crate that makes a sound
//! does not have to reach gameplay to do it. It depends on bevy, and on the
//! rodio decoder bevy plays through, which the loudness measure reads. Game
//! state reaches it through one hook: the world freeze. The engine does not
//! know the game's states; `nova_gameplay` runs
//! [`pause_world_voices`] and [`resume_world_voices`] on its frozen overlays
//! and [`stop_world_voices`] when gameplay ends, and the freeze they set is
//! [`SoundFreeze`].
//!
//! Everything degrades gracefully: a rig missing the mixer resources or the
//! listener plays at full volume rather than panicking or going silent. The
//! [`SoundBank<UiSfx>`] resource is inserted by `nova_assets` once assets load.
//! World sounds carry no bank at all - each cue resolves its target's authored
//! `AssetRef` (authored-or-silent).
#![warn(missing_docs)]

use bevy::prelude::*;

mod bus;
mod category;
mod listener;
pub mod loudness;
mod master;
mod mixing;
mod play;
mod registry;
mod sfx;
mod spatial;
mod voice;

/// Glob-import surface: `use nova_sound::prelude::*`.
///
/// The generic engine only. The ship's soundtrack is `nova_ship`'s
/// `ship_audio`, and the mixing internals (`SfxThrottle`, `area_cell`,
/// `distance_attenuation`, the pan math) stay off the boundary - they are the
/// engine's own machinery, re-exported at the crate root for the tests that pin
/// them.
///
/// Every cue volume is on the boundary because every cue volume is defined here
/// while the cues themselves fire from the crates above.
pub mod prelude {
    pub use super::{
        sounds_loaded, AudioBus, AudioRoute, ClipLoudness, HarnessMute, InterfaceVolume,
        MasterVolume, NovaSoundPlugin, PlaySfx, PlaySound, SfxAudioMarker, SfxCommandsExt,
        SfxListenerMarker, SfxPlugin, SfxSource, SfxVoice, SoundAt, SoundBank, SoundBus,
        SoundCategoryType, SoundFreeze, SoundListenerAnchor, SoundLoop, SoundLoudness,
        SoundSystems, SoundVoice, UiSfx, VoiceMix, WorldVolume, EDITOR_DENY_VOLUME,
        EDITOR_PLACE_VOLUME, EDITOR_REMOVE_VOLUME, EDITOR_ROTATE_VOLUME, MENU_BACK_VOLUME,
        MENU_FOCUS_VOLUME, MENU_SELECT_VOLUME, MINING_PULSE_VOLUME, MINING_REFUSAL_VOLUME,
        MUTE_ENV, NOVA_OS_BACK_VOLUME, NOVA_OS_BED_VOLUME, NOVA_OS_COIL_VOLUME,
        NOVA_OS_ENTER_VOLUME, NOVA_OS_ERROR_VOLUME, NOVA_OS_KEY_MIN_INTERVAL, NOVA_OS_KEY_VOLUME,
        NOVA_OS_OK_VOLUME, NOVA_OS_POWER_VOLUME, NOVA_OS_TICK_VOLUME, OBJECTIVE_FAIL_VOLUME,
        SALVAGE_PICKUP_VOLUME, UI_SFX_FILES, UI_TICK_VOLUME, UI_TOGGLE_VOLUME,
    };
}

pub use self::{
    bus::{
        slider_gain, AudioBus, AudioRoute, InterfaceVolume, Mixer, WorldVolume, SLIDER_FLOOR_DB,
    },
    category::{db_to_gain, SoundBus, SoundCategory, SoundCategoryType},
    listener::{resolve_hearing, Hearing, SoundListenerAnchor},
    loudness::{
        ClipLoudness, LoudnessError, SoundLoudness, LOUDNESS_TARGET_LUFS, SILENT_BELOW_LUFS,
    },
    master::{HarnessMute, MasterVolume, MUTE_ENV},
    mixing::{
        area_cell, cue_group, distance_attenuation, listener_position, CueGroup, CueThrottle,
        SfxListenerMarker, SfxThrottle, ThrottleKey, SFX_AREA_CELL, SFX_AUDIBLE_THRESHOLD,
        SFX_FAR_DISTANCE, SFX_NEAR_DISTANCE,
    },
    play::{
        place_one_shot, sound_root, PlaySound, SoundAt, SoundLoop, SoundRefusal, SoundRepeat,
        SoundVoice, VoiceGroup, VoicePlace,
    },
    registry::{sounds_loaded, SoundBank},
    sfx::{PlaySfx, SfxAudioMarker, SfxCommandsExt, SfxPlugin},
    spatial::{
        emitter_point, listener_ears, local_bearing, pan_compensation, pan_gains, SPATIAL_EAR_GAP,
        SPATIAL_EMITTER_RADIUS,
    },
    voice::{
        pause_world_voices, resume_world_voices, stop_world_voices, SfxSource, SfxVoice,
        SoundFreeze, VoiceMarker, VoiceMix, MAX_EFFECT_LOOP_VOICES,
    },
};
use self::{
    loudness::measure_loaded_sounds,
    master::{apply_master_volume, report_harness_mute},
    mixing::prune_sfx_throttle,
    play::{on_play_sound, on_remove_sound_loop, prune_sound_repeat, sync_sound_loops},
    voice::{drive_sfx_voices, on_add_listener, retire_unplayable_one_shots, start_sfx_voices},
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
    /// A button was pressed - a crisp UI click, from any screen. Fired by
    /// `nova_gameplay`'s `ButtonCuePlugin`, from its global `On<Activate>`
    /// observer.
    MenuSelect,
    /// A button that POPS a panel was pressed - `menu_select` inverted, so
    /// going in and coming back out are audibly a direction rather than two
    /// clicks. Fired from the same observer, by the button's `ButtonCueType`.
    MenuBack,
    /// The pointer arrived on a button. The most-fired cue in the game, and
    /// deliberately the smallest.
    MenuFocus,
    /// A slider crossed a detent. The quietest cue the interface has.
    UiTick,
    /// A run ended in defeat - `objective_complete` inverted, three steps down.
    ObjectiveFail,
    /// A part seated in the editor grid.
    EditorPlace,
    /// A part lifted out of the editor grid.
    EditorRemove,
    /// An editor placement turned one detent.
    EditorRotate,
    /// The editor refused a placement.
    EditorDeny,
    /// A comms line just SHOWED on the panel - a soft radio blip so a story
    /// beat registers mid-fight.
    CommsLine,
    /// A line posted to the HUD notice slot (a station refusing docking) - a
    /// flat ping, played by `nova_hud::hud_notice`.
    HudNotice,
    /// The pause overlay or TAB interface opening or closing - a soft
    /// two-state UI blip.
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
    /// The degauss coil thump.
    NovaOsCoil,
    /// The power-up sweep when the NOVA OS monitor opens.
    NovaOsPowerUp,
    /// The power-down sweep when the NOVA OS monitor closes.
    NovaOsPowerDown,
    /// The live-tube ambient bed loop while the NOVA OS monitor is open.
    NovaOsBed,
}

/// The `(key, base-filename)` pairs for the UI bank. Loaded by
/// `nova_assets::register_sounds` via `SoundBank::load`, whose
/// `sounds/<name>.wav` convention maps these to the root `assets/sounds/` -
/// engine chrome, outside every mod.
pub const UI_SFX_FILES: [(UiSfx, &str); 24] = [
    (UiSfx::ObjectiveNew, "objective_new"),
    (UiSfx::ObjectiveComplete, "objective_complete"),
    (UiSfx::ObjectiveFail, "objective_fail"),
    (UiSfx::MenuSelect, "menu_select"),
    (UiSfx::MenuBack, "menu_back"),
    (UiSfx::MenuFocus, "menu_focus"),
    (UiSfx::UiToggle, "ui_toggle"),
    (UiSfx::UiTick, "ui_tick"),
    (UiSfx::CommsLine, "comms_line"),
    (UiSfx::HudNotice, "hud_notice"),
    (UiSfx::EditorPlace, "editor_place"),
    (UiSfx::EditorRemove, "editor_remove"),
    (UiSfx::EditorRotate, "editor_rotate"),
    (UiSfx::EditorDeny, "editor_deny"),
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

/// The cargo intake's take "ding". Deliberately quieter than the objective
/// chime (`OBJECTIVE_COMPLETE_VOLUME` 0.38 / `OBJECTIVE_NEW_VOLUME` 0.30) so a
/// canister take reads as a light per-item confirmation, not a beat
/// completion. Fired from `nova_ship`'s intake machinery audio.
pub const SALVAGE_PICKUP_VOLUME: f32 = 0.22;

/// One mining beam pulse landing. It repeats once a second while the key is
/// held, so it sits in the machinery band with the intake's door
/// (`nova_ship`'s `INTAKE_DOOR_CLOSE_VOLUME`, 0.20), a little under the
/// salvage pickup it leads to. Fired from `nova_scenario`'s mining plugin,
/// which owns `MiningPulse`.
pub const MINING_PULSE_VOLUME: f32 = 0.18;

/// One refused mining beam pulse. It repeats on the same clock as a landing
/// pulse while the key is held, so it sits at [`MINING_PULSE_VOLUME`]. Fired
/// from `nova_scenario`'s mining plugin, which owns `MiningPulse`.
pub const MINING_REFUSAL_VOLUME: f32 = 0.18;

/// Volume for the button click cue. Non-positional and
/// in the informational-tick band, like the ship's own cue volumes over in
/// `nova_ship`'s `ship_audio`.
pub const MENU_SELECT_VOLUME: f32 = 0.28;
/// Volume for the [`UiSfx::UiToggle`] blip, fired from `nova_menu` and
/// `nova_interface`.
pub const UI_TOGGLE_VOLUME: f32 = 0.24;
/// Volume for the panel-pop cue. Set to land on `MENU_SELECT_VOLUME`'s
/// A-weighted level (about -39.6 dBA at unity master), not to match its linear
/// factor: the two cues are a matched pair and have to read as one gesture in
/// two directions, which is a LOUDNESS match. See `RCS_MAX_VOLUME` for what
/// comparing linear factors instead costs.
pub const MENU_BACK_VOLUME: f32 = 0.28;
/// Volume for the focus-move cue. About 10 dB under the click, because this is
/// the cue a pad user fires on every stick flick: at click level a menu sweep
/// is a machine gun.
pub const MENU_FOCUS_VOLUME: f32 = 0.20;
/// Volume for the slider detent, in the same band as [`MENU_FOCUS_VOLUME`] and
/// for the same reason - a dragged slider fires it many times a second.
pub const UI_TICK_VOLUME: f32 = 0.14;
/// Volume for the defeat cue. Matched to `OBJECTIVE_COMPLETE_VOLUME`'s measured
/// level: the pair is a mirror, so a loss must be exactly as present as a win.
pub const OBJECTIVE_FAIL_VOLUME: f32 = 0.53;
/// Editor cue volumes. The editor is a workspace a player sits in for a long
/// time, so the set is placed a band under the menus rather than at their
/// level, and the two cues that fire on every gesture (rotate, and the deny
/// that answers a dragged illegal ghost) are the quietest of the four.
///
/// A part seating - the heaviest cue the interface has, at click level.
pub const EDITOR_PLACE_VOLUME: f32 = 0.47;
/// A part lifting out, about 2 dB under the seat so the pair reads as a
/// direction.
pub const EDITOR_REMOVE_VOLUME: f32 = 0.21;
/// One rotation detent.
pub const EDITOR_ROTATE_VOLUME: f32 = 0.18;
/// A refused placement.
pub const EDITOR_DENY_VOLUME: f32 = 0.32;
/// NOVA OS terminal cue volumes. The `nova_*.wav` files are peak-normalized to
/// -3 dBFS, so these linear factors are what place each cue in the
/// informational-tick band. Typing is the quietest and is throttled
/// ([`NOVA_OS_KEY_MIN_INTERVAL`]) so a held key cannot machine-gun; the coil
/// and power sweeps are the loudest "moment" cues. `pub` because the cues are
/// fired from `nova_interface`, keeping every cue volume defined in this crate.
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
/// [`MasterVolume`]. The bed WAV is
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
/// `nova_ship`'s `ShipAudioPlugin`, the terminal's by `nova_interface`, the
/// button voice by `nova_gameplay`'s `ButtonCuePlugin`. Build it before any of
/// them: `AppBuilder` adds it after the assets and before gameplay, and
/// `NovaGameplayPlugin` adds it when an app has not.
#[derive(Default)]
pub struct NovaSoundPlugin;

/// The engine's per-frame mixing pass.
///
/// Runs in `PostUpdate` before `TransformSystems::Propagate`, which is what
/// bevy's own audio playback runs AFTER: a voice spawned this frame is placed
/// and levelled before bevy opens its sink, so no cue is ever heard at the
/// wrong volume for a frame. Every owner that drives a loop's level (the ship's
/// thruster and RCS passes, the terminal's bed) writes it in `Update`, upstream
/// of this.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundSystems;

impl Plugin for NovaSoundPlugin {
    fn build(&self, app: &mut App) {
        trace!("NovaSoundPlugin: build");

        // One-shot cues (PlaySfx -> SfxVoice).
        if !app.is_plugin_added::<SfxPlugin>() {
            app.add_plugins(SfxPlugin);
        }

        app.init_resource::<MasterVolume>();
        // Unmuted until the app says otherwise: `nova_gameplay`'s settings
        // plugin resolves the harness mute from the environment, and the game
        // binary's `--mute` inserts it after that.
        app.init_resource::<HarnessMute>();
        app.init_resource::<InterfaceVolume>();
        app.init_resource::<WorldVolume>();
        app.init_resource::<SfxThrottle>();
        app.init_resource::<SoundFreeze>();
        app.init_resource::<SoundLoudness>();
        app.init_resource::<SoundRepeat>();
        app.register_type::<MasterVolume>();
        app.register_type::<InterfaceVolume>();
        app.register_type::<WorldVolume>();
        app.register_type::<SfxListenerMarker>();
        app.register_type::<SoundListenerAnchor>();

        app.add_observer(on_add_listener);
        app.add_observer(on_play_sound);
        app.add_observer(on_remove_sound_loop);

        app.configure_sets(
            PostUpdate,
            SoundSystems.before(bevy::transform::TransformSystems::Propagate),
        );
        app.add_systems(
            PostUpdate,
            (
                sync_sound_loops,
                retire_unplayable_one_shots,
                start_sfx_voices,
                drive_sfx_voices,
            )
                .chain()
                .in_set(SoundSystems),
        );

        // `PostStartup` and not `Startup`: the persisted master volume is
        // loaded by one of `nova_menu`'s own `Startup` systems, and two
        // systems in the same schedule have no order between them. Then on
        // change, for as long as the run lasts; `resource_changed` is true on
        // the first frame too, so the two passes agree.
        app.add_systems(PostStartup, apply_master_volume);
        app.add_systems(
            Update,
            apply_master_volume.run_if(resource_changed::<MasterVolume>),
        );
        // Startup, not build: the game binary's `--mute` inserts the resource
        // AFTER this plugin, so only a system can read the value the run
        // actually uses.
        app.add_systems(Startup, report_harness_mute);

        // Pure map cleanup; harmless to run always and keeps memory bounded.
        app.add_systems(Update, (prune_sfx_throttle, prune_sound_repeat));
        // Every clip is measured once as it loads. Playback reads the table in
        // `PostUpdate`, and a voice whose clip is not measured yet waits.
        app.add_systems(Update, measure_loaded_sounds);
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
            HudNotice,
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
