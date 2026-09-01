//! The ship's soundtrack: the mapping from Nova gameplay events to sounds,
//! played through the generic engine in [`nova_gameplay::audio`].
//!
//! Five cues are one-shots fired from existing seams via observers, so no
//! gameplay system has to know about audio:
//! - a section/asteroid destroyed or a torpedo detonating -> `Explosion`
//!   (`On<Add, IntegrityDestroyMarker>`);
//! - damage applied to a target -> `Impact` (`On<HealthApplyDamage>`);
//! - a turret round spawned -> the firing turret's authored `fire_sound`
//!   (`On<Add, TurretBulletProjectileMarker>`, authored-or-silent);
//! - a torpedo spawned -> the bay's authored `launch_sound`
//!   (`On<Add, TorpedoProjectileMarker>`, authored-or-silent);
//! - a lance discharging -> the railgun's authored `fire_sound` (`On<RailgunFired>`).
//!
//! The continuous cues - the thruster hum and the RCS hiss - are one looping
//! voice per (ship, authored sound) pair, each following its own ship.
//!
//! Every world cue is ROUTED by `routing`: a cue from the player's own ship is
//! [`AudioRoute::Hull`] and one from anything else is [`AudioRoute::Exterior`].
//! That single fact is what decides whether it is distance-attenuated and
//! panned, so nothing here computes a distance or a bearing - the engine reads
//! the route and does it. The cockpit cues (lock, safety, dry fire) are Hull
//! too: they are the player's own computer talking.
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
mod routing;

#[cfg(test)]
mod test_support;

use self::{
    combat::{
        on_damage_play_impact, on_destroyed_play_explosion, on_railgun_fire_play_sfx,
        on_torpedo_launch_play_sfx, on_turret_fire_play_sfx,
    },
    cues::{play_dry_fire_cue, play_lock_cues, play_safety_engaged_cue},
    loops::{drive_rcs_loops, drive_thruster_loops},
};

/// Per-cue *base* playback volumes (at point-blank; distance attenuation scales
/// them down from here). The PDC fires ~100 rounds/s and impacts arrive in
/// blast-sized bursts, so those two are quiet; destruction and launch are the
/// punchy moments. Kept modest so nothing is harsh up close.
const TURRET_FIRE_VOLUME: f32 = 0.10;
const IMPACT_VOLUME: f32 = 0.22;
const EXPLOSION_VOLUME: f32 = 0.40;
const TORPEDO_LAUNCH_VOLUME: f32 = 0.45;
/// The loudest cue in the set, and the only one that should be: a lance fires
/// once a reload cycle, so it can afford to be the punctuation the PDC stream
/// never is.
const RAILGUN_FIRE_VOLUME: f32 = 0.55;

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

        app.add_observer(on_destroyed_play_explosion);
        app.add_observer(on_damage_play_impact);
        app.add_observer(on_turret_fire_play_sfx);
        app.add_observer(on_torpedo_launch_play_sfx);
        app.add_observer(on_railgun_fire_play_sfx);

        // Lock/safety UI cues: message-driven one-shots, so no gating needed
        // - the writers (radar search, tap observer) are themselves
        // pause-gated. The dry-fire click polls turret input/ammo and
        // edge-latches per turret; a pause freezes the input so no fresh edge
        // fires while paused.
        app.add_systems(
            Update,
            (play_lock_cues, play_safety_engaged_cue, play_dry_fire_cue),
        );

        // The loop passes poll `ThrusterSectionInput` and `RcsIntent`, so they
        // must be gated to the running simulation exactly like the thruster
        // physics/shader. Joining `SpaceshipSectionSystems` inherits whatever
        // run condition consumers of that input use - crucially nova_scenario's
        // `run_if(scenario_is_live)` - so the loops stay silent while building
        // in the editor (no scenario is loaded there) and play wherever one is
        // live, the main menu's ambience backdrop included. (The one-shot cues
        // need no gating: they fire on spawn/damage/destroy events that only
        // occur inside this same gated set.)
        //
        // Both write the loops' LEVELS in `Update`; the engine's
        // `AudioSystems` pass reads them in `PostUpdate` and owns everything
        // after - the rolloff, the pan, the master, the freeze behind a paused
        // sim and the teardown on scenario unload.
        app.add_systems(
            Update,
            (drive_thruster_loops, drive_rcs_loops).in_set(SpaceshipSectionSystems),
        );
    }
}
