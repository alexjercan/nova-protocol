//! Nova's audio wiring: map gameplay events to placeholder sound effects.
//!
//! The generic playback machinery is nova's too, in the private `sfx` and
//! `registry` submodules: [`SfxPlugin`] spawns a self-despawning audio entity
//! for every [`PlaySfx`], and [`SoundBank`] is a keyed registry of loaded
//! handles. The rest of this module is the *game-specific* part - the mapping
//! from Nova gameplay events to sounds - and the split is kept so the reusable
//! half stays extractable once the game is done.
//!
//! Four cues are one-shots fired from existing seams via observers, so no
//! gameplay system has to know about audio:
//! - a section/asteroid destroyed or a torpedo detonating -> `Explosion`
//!   (`On<Add, IntegrityDestroyMarker>`);
//! - damage applied to a target -> `Impact` (`On<HealthApplyDamage>`);
//! - a turret round spawned -> the firing turret's authored `fire_sound`
//!   (`On<Add, TurretBulletProjectileMarker>`, authored-or-silent);
//! - a torpedo spawned -> the bay's authored `launch_sound`
//!   (`On<Add, TorpedoProjectileMarker>`, authored-or-silent).
//!
//! The fifth cue, the thruster engine hum, is continuous: one looping audio
//! entity per DISTINCT authored `loop_sound` (thrusters sharing a sound share a
//! loop), each tracking how hard the ships burning that sound are thrusting.
//!
//! The four one-shots are **distance-attenuated**: their volume is scaled by
//! how far the event is from the listener (the camera carrying
//! [`SfxListenerMarker`], i.e. the gameplay camera), so a distant explosion is
//! quieter than one next to you. This is a volume-only rolloff for the
//! cinematic feel, not true spatialization - stereo panning would need bevy
//! spatial audio (`SpatialListener` + `spatial: true`) and is a future step.
//! The thruster hum attenuates per SHIP: each ship's throttle-driven
//! contribution is scaled by its root's distance to the listener and the
//! loudest wins PER HUM SOUND, except the player's own ship, which is never
//! attenuated (the camera rig sits 11-32 u out by mode and the orbit survey
//! dolly stretches it to 250 u, deep in the rolloff band; see
//! `compute_thruster_hum_volume`).
//!
//! The [`SoundBank<UiSfx>`] resource is inserted by `nova_assets` once assets
//! load; every system here degrades gracefully (does nothing) until the
//! resources it needs exist. World sounds carry no bank at all - each cue
//! resolves its target's authored `AssetRef` (authored-or-silent).

use bevy::prelude::*;

use crate::prelude::*;

mod combat;
mod cues;
mod loops;
mod mixing;
mod registry;
mod sfx;

#[cfg(test)]
mod test_support;

use self::{
    combat::{
        on_damage_play_impact, on_destroyed_play_explosion, on_torpedo_launch_play_sfx,
        on_turret_fire_play_sfx,
    },
    cues::{play_dry_fire_cue, play_lock_cues, play_safety_engaged_cue},
    loops::{
        apply_rcs_loop_volume, apply_thruster_loop_volume, compute_rcs_loop_volume,
        compute_thruster_hum_volume, ensure_rcs_loops, ensure_thruster_loops, pause_loops,
        resume_loops, silence_loops_on_scenario_unload, RcsLoopVolume, ThrusterHumVolume,
    },
    mixing::{prune_sfx_throttle, SfxThrottle},
};
pub use self::{
    mixing::SfxListenerMarker,
    registry::{sounds_loaded, SoundBank},
    sfx::{PlaySfx, SfxCommandsExt, SfxMasterVolume, SfxPlugin},
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

/// Per-cue *base* playback volumes (at point-blank; distance attenuation scales
/// them down from here). The PDC fires ~100 rounds/s and impacts arrive in
/// blast-sized bursts, so those two are quiet; destruction and launch are the
/// punchy moments. Kept modest so nothing is harsh up close.
const TURRET_FIRE_VOLUME: f32 = 0.10;
const IMPACT_VOLUME: f32 = 0.22;
const EXPLOSION_VOLUME: f32 = 0.40;
const TORPEDO_LAUNCH_VOLUME: f32 = 0.45;

/// UI (non-positional) volumes for the lock/safety cues - informational
/// ticks, kept under the combat sounds.
const LOCK_ON_VOLUME: f32 = 0.30;
const LOCK_OFF_VOLUME: f32 = 0.28;
const SAFETY_ON_VOLUME: f32 = 0.30;
const RADAR_DENY_VOLUME: f32 = 0.26;

/// The salvage-pickup "ding". Deliberately quieter than the objective chime
/// (`OBJECTIVE_COMPLETE_VOLUME` 0.38 / `OBJECTIVE_NEW_VOLUME` 0.30) so a crate
/// pickup reads as a light per-item confirmation, not a beat completion. `pub`
/// because the cue is fired from `nova_scenario`'s salvage plugin (which owns
/// `SalvageCrateMarker`), keeping every cue volume defined here in the audio
/// module.
pub const SALVAGE_PICKUP_VOLUME: f32 = 0.22;

/// UI/feedback volumes for the menu and turret/radar cues. All non-positional,
/// kept in the informational-tick band with the lock/safety cues. The retarget
/// tick is the quietest - it can repeat several times within one held gesture,
/// so it must stay well under the once-per-gesture `LOCK_ON_VOLUME` acquire
/// cue. The dry-fire and retarget cues are played here in this module
/// (private), but `MENU_SELECT`/`UI_TOGGLE` are fired from `nova_menu`, so
/// those two are `pub` - keeping every cue volume defined here in the audio
/// module.
pub const MENU_SELECT_VOLUME: f32 = 0.28;
/// Volume for the HUD-toggle click cue, fired from `nova_menu`.
pub const UI_TOGGLE_VOLUME: f32 = 0.24;
const DRY_FIRE_VOLUME: f32 = 0.22;
const RADAR_RETARGET_VOLUME: f32 = 0.18;

/// NOVA OS terminal cue volumes. The `nova_*.wav` files are peak-normalized to
/// -3 dBFS, so these linear factors are what place each cue in the
/// informational-tick band. Typing is the quietest and is throttled
/// ([`NOVA_OS_KEY_MIN_INTERVAL`]) so a held key cannot machine-gun; the coil
/// and power sweeps are the loudest "moment" cues. `pub(crate)` because the
/// cues are fired from `hud::nova_os`, keeping every cue volume defined in this
/// module.
pub(crate) const NOVA_OS_KEY_VOLUME: f32 = 0.10;
pub(crate) const NOVA_OS_BACK_VOLUME: f32 = 0.12;
pub(crate) const NOVA_OS_ENTER_VOLUME: f32 = 0.18;
pub(crate) const NOVA_OS_OK_VOLUME: f32 = 0.20;
pub(crate) const NOVA_OS_ERROR_VOLUME: f32 = 0.22;
pub(crate) const NOVA_OS_TICK_VOLUME: f32 = 0.12;
pub(crate) const NOVA_OS_COIL_VOLUME: f32 = 0.26;
pub(crate) const NOVA_OS_POWER_VOLUME: f32 = 0.30;
/// Base volume of the ambient bed loop, before [`MasterVolume`]. The bed WAV is
/// authored quiet (~-35 dBFS), so this sits it as a soft under-hum.
pub(crate) const NOVA_OS_BED_VOLUME: f32 = 0.7;
/// Minimum real seconds between successive typing clicks, so OS key-repeat on a
/// held key does not spawn a storm of click one-shots (the `SfxThrottle`
/// precedent, applied inline with a `Local` since this is one global stream).
pub(crate) const NOVA_OS_KEY_MIN_INTERVAL: f32 = 0.03;

/// Plugin wiring Nova's gameplay events to sound effects. Adds the reusable
/// [`SfxPlugin`] and Nova's own observers + the thruster-loop systems.
#[derive(Default)]
pub struct NovaAudioPlugin;

impl Plugin for NovaAudioPlugin {
    fn build(&self, app: &mut App) {
        debug!("NovaAudioPlugin: build");

        // Generic fire-and-forget SFX playback (PlaySfx / SoundBank live here).
        if !app.is_plugin_added::<SfxPlugin>() {
            app.add_plugins(SfxPlugin);
        }

        app.init_resource::<SfxThrottle>();
        app.register_type::<SfxListenerMarker>();

        // Audio sinks do not follow Time<Virtual>: without this the thruster
        // hum keeps roaring at its last volume behind a frozen sim. BOTH
        // frozen overlays need it - the pause overlay and the Tab
        // ship-computer NOVA OS, which freezes the same way (see
        // PauseStates::is_frozen).
        app.add_systems(OnEnter(crate::PauseStates::Paused), pause_loops);
        app.add_systems(OnExit(crate::PauseStates::Paused), resume_loops);
        app.add_systems(OnEnter(crate::PauseStates::NovaOs), pause_loops);
        app.add_systems(OnExit(crate::PauseStates::NovaOs), resume_loops);
        app.add_systems(
            OnExit(crate::GameStates::Playing),
            silence_loops_on_scenario_unload,
        );

        app.add_observer(on_destroyed_play_explosion);
        app.add_observer(on_damage_play_impact);
        app.add_observer(on_turret_fire_play_sfx);
        app.add_observer(on_torpedo_launch_play_sfx);

        // Lock/safety UI cues: message-driven one-shots, so no gating needed
        // - the writers (radar search, tap observer) are themselves
        // pause-gated. The dry-fire click polls turret input/ammo and
        // edge-latches per turret; a pause freezes the input so no fresh edge
        // fires while paused.
        app.add_systems(
            Update,
            (play_lock_cues, play_safety_engaged_cue, play_dry_fire_cue),
        );

        // The thruster hum polls `ThrusterSectionInput`, so it must be gated to
        // the running simulation exactly like the thruster physics/shader. Joining
        // `SpaceshipSectionSystems` inherits whatever run condition consumers of
        // that input use - crucially nova_scenario's `run_if(scenario_is_live)`
        // - so the hum stays silent while building in the editor (no scenario is
        // loaded there) and plays wherever one is live, the main menu's ambience
        // backdrop included. (The one-shot cues need no gating: they fire on
        // spawn/damage/destroy events that only occur inside this same gated set.)
        app.init_resource::<ThrusterHumVolume>();
        app.add_systems(
            Update,
            (
                ensure_thruster_loops,
                compute_thruster_hum_volume,
                apply_thruster_loop_volume,
            )
                .chain()
                .in_set(SpaceshipSectionSystems),
        );

        // The RCS fine-adjust loop polls `RcsIntent`, written by the player
        // modal and the autopilot both, so it joins the same scenario-gated set
        // as the thruster hum for the same reasons (silent in the editor, muted
        // on pause).
        app.init_resource::<RcsLoopVolume>();
        app.add_systems(
            Update,
            (
                ensure_rcs_loops,
                compute_rcs_loop_volume,
                apply_rcs_loop_volume,
            )
                .chain()
                .in_set(SpaceshipSectionSystems),
        );

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
