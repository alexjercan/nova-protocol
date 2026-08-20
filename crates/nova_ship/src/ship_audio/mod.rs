//! The ship's soundtrack: the mapping from Nova gameplay events to sounds,
//! played through the generic engine in [`nova_gameplay::audio`].
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
//! The four one-shots are distance-attenuated by the engine's rolloff. The
//! thruster hum attenuates per SHIP: each ship's throttle-driven contribution
//! is scaled by its root's distance to the listener and the loudest wins PER
//! HUM SOUND, except the player's own ship, which is never attenuated (the
//! camera rig sits 11-32 u out by mode and the orbit survey dolly stretches it
//! to 250 u, deep in the rolloff band; see `compute_thruster_hum_volume`).
//!
//! Every system here degrades gracefully (does nothing) until the resources it
//! needs exist. World sounds carry no bank at all - each cue resolves its
//! target's authored `AssetRef` (authored-or-silent).

use bevy::prelude::*;

use crate::prelude::*;

mod combat;
mod cues;
mod levels;
mod loops;

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
};

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

/// The turret dry-fire click and the radar retarget tick. The retarget tick is
/// the quietest cue of the set - it can repeat several times within one held
/// gesture, so it must stay well under the once-per-gesture
/// [`LOCK_ON_VOLUME`] acquire cue.
const DRY_FIRE_VOLUME: f32 = 0.22;
const RADAR_RETARGET_VOLUME: f32 = 0.18;

/// Minimum seconds between successive turret-fire and impact one-shots. Without
/// this the ~100/s PDC and the many-collider blast hits would each spawn a
/// storm of overlapping audio entities that reads as a wall of noise;
/// collapsing them to a bounded rate keeps the cue legible and the entity churn
/// sane.
const TURRET_FIRE_MIN_INTERVAL: f32 = 0.05;
const IMPACT_MIN_INTERVAL: f32 = 0.04;

/// A dying multi-section ship marks every section destroyed in the same frame;
/// this collapses that burst into a single explosion instead of N overlapping
/// ones (which would clip). Short enough that genuinely separate kills >60ms
/// apart still each sound.
const EXPLOSION_MIN_INTERVAL: f32 = 0.06;

/// Plugin wiring the ship's gameplay events to sound effects: the combat
/// observers, the cockpit cues, and the thruster/RCS loops. Requires the
/// engine ([`nova_gameplay::audio::NovaAudioPlugin`]), which it adds if absent.
#[derive(Default)]
pub struct ShipAudioPlugin;

impl Plugin for ShipAudioPlugin {
    fn build(&self, app: &mut App) {
        trace!("ShipAudioPlugin: build");

        if !app.is_plugin_added::<nova_gameplay::audio::NovaAudioPlugin>() {
            app.add_plugins(nova_gameplay::audio::NovaAudioPlugin);
        }

        // Audio sinks do not follow Time<Virtual>: without this the thruster
        // hum keeps roaring at its last volume behind a frozen sim. BOTH
        // frozen overlays need it - the pause overlay and the Tab
        // ship-computer NOVA OS, which freezes the same way (see
        // PauseStates::is_frozen).
        app.add_systems(OnEnter(nova_gameplay::PauseStates::Paused), pause_loops);
        app.add_systems(OnExit(nova_gameplay::PauseStates::Paused), resume_loops);
        app.add_systems(OnEnter(nova_gameplay::PauseStates::NovaOs), pause_loops);
        app.add_systems(OnExit(nova_gameplay::PauseStates::NovaOs), resume_loops);
        app.add_systems(
            OnExit(nova_gameplay::GameStates::Playing),
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
    }
}
