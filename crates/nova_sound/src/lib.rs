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
//! workspace's sources and fails on a constructor outside this crate. To make
//! a sound, trigger a [`PlaySound`] (a one-shot) or add a [`SoundLoop`] (a
//! loop). Interface cues are categories on the [`SoundBus::Ui`] rows, played
//! with [`PlaySound::ui`]. Background music is the [`MusicDeck`]'s.
//!
//! # Categories
//!
//! A sound's caller names WHAT it is, a [`SoundCategoryType`], and WHERE it
//! is, a [`SoundAt`]. The category's one table row sets everything else: its
//! bus, its level, its reach, how many voices of one sound one ship may hold,
//! how often it may repeat, and a loop's dB range. No caller passes a gain:
//!
//! ```text
//! gain = clip loudness gain (to -23 LUFS) x category level
//!        x distance fade and pan x bus slider x master
//! ```
//!
//! # The buses
//!
//! ```text
//! MasterVolume (settings)  ->  bevy GlobalVolume
//!   Ui      (InterfaceVolume)  cockpit and menu cues. Heard flat.
//!   Effect  (WorldVolume)      every sound in the world, the player's own
//!                              ship included. Faded and panned.
//!   Music   (MusicVolume)      the MusicDeck's tracks. Heard flat, through
//!                              a frozen sim.
//! ```
//!
//! Each slider goes through the dB taper in [`slider_gain`].
//!
//! # The listener
//!
//! An Effect sound fades with its distance from the player's ship (marked
//! [`SoundListenerAnchor`]; with no ship, the camera) and pans by its bearing
//! in the camera's frame (marked [`SfxListenerMarker`]). With no camera there
//! is no one to hear it, and it is dropped: an Effect sound never falls back
//! to flat.
//!
//! # The modules
//!
//! `category` is the table and the fade, `play` turns requests into voices
//! and holds the repeat interval and voice count per sound and `ChildOf` root,
//! `listener` resolves the listener, `bus` owns the sliders, `master` owns the
//! master volume and the harness mute, `loudness` measures every clip as it
//! loads, `mixing` holds the listener marker and the cell grouping the juice
//! layer shares, `spatial` owns the stereo placement, `voice` owns playback,
//! the loop release and the loop cap, `music` owns the music deck and its
//! crossfades, and `registry` is the keyed handle bank. Nothing here knows
//! what a ship is - the mapping from gameplay events to sounds is
//! `nova_ship`'s `ship_audio`.
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
//! A rig missing the mixer resources plays at full volume rather than
//! panicking. The
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
mod music;
mod play;
mod registry;
mod spatial;
mod voice;

/// Glob-import surface: `use nova_sound::prelude::*`.
///
/// The generic engine only. The ship's soundtrack is `nova_ship`'s
/// `ship_audio`, and the mixing internals (`area_cell`, the fade, the pan
/// math) stay off the boundary - they are the
/// engine's own machinery, re-exported at the crate root for the tests that pin
/// them.
pub mod prelude {
    pub use super::{
        sounds_loaded, ClipLoudness, HarnessMute, InterfaceVolume, MasterVolume, MusicDeck,
        MusicDeckStatusType, MusicVoice, MusicVolume, NovaSoundPlugin, PlaySound, SfxAudioMarker,
        SfxListenerMarker, SoundAt, SoundBank, SoundBus, SoundCategoryType, SoundFreeze,
        SoundListenerAnchor, SoundLoop, SoundLoudness, SoundSystems, SoundVoice, UiSfx, VoiceGroup,
        VoiceMix, VoicePlace, WorldVolume, MUTE_ENV, UI_SFX_FILES,
    };
}

pub use self::{
    bus::{slider_gain, InterfaceVolume, Mixer, MusicVolume, WorldVolume, SLIDER_FLOOR_DB},
    category::{db_to_gain, SoundBus, SoundCategory, SoundCategoryType},
    listener::{resolve_hearing, Hearing, SoundListenerAnchor},
    loudness::{
        ClipLoudness, LoudnessError, SoundLoudness, LOUDNESS_TARGET_LUFS, SILENT_BELOW_LUFS,
    },
    master::{HarnessMute, MasterVolume, MUTE_ENV},
    mixing::{
        area_cell, cue_group, CueGroup, CueThrottle, SfxListenerMarker, SFX_AREA_CELL,
        SFX_AUDIBLE_THRESHOLD,
    },
    music::{MusicDeck, MusicDeckStatusType, MusicVoice},
    play::{
        place_one_shot, sound_root, PlaySound, SfxAudioMarker, SoundAt, SoundGroupBoundary,
        SoundLoop, SoundRefusal, SoundRepeat, SoundVoice, VoiceGroup, VoicePlace,
    },
    registry::{sounds_loaded, SoundBank},
    spatial::{
        emitter_point, listener_ears, local_bearing, pan_compensation, pan_gains, SPATIAL_EAR_GAP,
        SPATIAL_EMITTER_RADIUS,
    },
    voice::{
        pause_world_voices, resume_world_voices, stop_world_voices, SoundFreeze, VoiceMarker,
        VoiceMix, LOOP_RELEASE_SECONDS, MAX_EFFECT_LOOP_VOICES,
    },
};
use self::{
    loudness::measure_loaded_sounds,
    master::{apply_master_volume, report_harness_mute},
    music::{apply_music_requests, drive_music_voices, start_music_voices},
    play::{on_play_sound, on_remove_sound_loop, prune_sound_repeat, sync_sound_loops},
    voice::{
        drive_sfx_voices, on_add_listener, ramp_loop_releases, retire_unplayable_one_shots,
        start_sfx_voices,
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
    /// A job was handed in and paid - three soft steps up. Played by
    /// `nova_interface`'s Jobs pane in place of the action click.
    Payout,
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
pub const UI_SFX_FILES: [(UiSfx, &str); 25] = [
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
    (UiSfx::Payout, "payout"),
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

        app.init_resource::<MasterVolume>();
        // Unmuted until the app says otherwise: `nova_gameplay`'s settings
        // plugin resolves the harness mute from the environment, and the game
        // binary's `--mute` inserts it after that.
        app.init_resource::<HarnessMute>();
        app.init_resource::<InterfaceVolume>();
        app.init_resource::<WorldVolume>();
        app.init_resource::<MusicVolume>();
        app.init_resource::<MusicDeck>();
        app.init_resource::<SoundFreeze>();
        app.init_resource::<SoundLoudness>();
        app.init_resource::<SoundRepeat>();
        app.register_type::<MasterVolume>();
        app.register_type::<InterfaceVolume>();
        app.register_type::<WorldVolume>();
        app.register_type::<MusicVolume>();
        app.register_type::<SfxListenerMarker>();
        app.register_type::<SfxAudioMarker>();
        app.register_type::<SoundListenerAnchor>();
        app.register_type::<SoundGroupBoundary>();

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
                ramp_loop_releases,
                retire_unplayable_one_shots,
                start_sfx_voices,
                drive_sfx_voices,
            )
                .chain()
                .in_set(SoundSystems),
        );
        app.add_systems(
            PostUpdate,
            (apply_music_requests, start_music_voices, drive_music_voices)
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
        app.add_systems(Update, prune_sound_repeat);
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
            Payout,
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
