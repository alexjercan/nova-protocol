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
mod machinery;
mod routing;

#[cfg(test)]
mod test_support;

use self::{
    combat::{
        on_collapse_play_hull_loss, on_damage_play_impact, on_destroyed_play_explosion,
        on_railgun_fire_play_sfx, on_reload_complete_play_sfx, on_torpedo_launch_play_sfx,
        on_turret_fire_play_sfx,
    },
    cues::{
        play_dry_fire_cue, play_hull_warning_cue, play_lock_cues, play_safety_engaged_cue,
        play_threat_lock_cue,
    },
    loops::{drive_railgun_charge_loops, drive_rcs_loops, drive_thruster_loops},
    machinery::{on_bay_doors_play_sfx, on_stow_doors_play_sfx},
};

/// Per-cue *base* playback volumes (at point-blank; distance attenuation scales
/// them down from here). The PDC fires ~100 rounds/s and impacts arrive in
/// blast-sized bursts, so those two are quiet; destruction and launch are the
/// punchy moments. Kept modest so nothing is harsh up close.
const TURRET_FIRE_VOLUME: f32 = 0.10;
const IMPACT_VOLUME: f32 = 0.22;
const EXPLOSION_VOLUME: f32 = 0.40;
/// Was 0.45, which measured as the LOUDEST cue in the game A-weighted - louder
/// than the explosion it is quieter than on paper, and 8 dB over the railgun
/// the comment below calls the loudest. Same spectral trap as
/// [`levels::RCS_MAX_VOLUME`]: the launch's energy sits an octave and a half
/// above the boom's, so equal numbers are not equal loudness. 0.30 puts it
/// just under the explosion, where a launch belongs.
const TORPEDO_LAUNCH_VOLUME: f32 = 0.30;
/// The loudest cue in the set BY NUMBER, and the intent is that it be the
/// loudest by ear too: a lance fires once a reload cycle, so it can afford to
/// be the punctuation the PDC stream never is. It is not there yet - the slug
/// report is almost all sub-100 Hz, so A-weighted it measures ~7 dB under the
/// explosion. Raising it is a call for the seat, not for a spreadsheet.
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

/// The retractable PDC housing, rising and folding. Background machinery, so
/// both land just under the dry-fire click and above the PDC's own report.
///
/// The two numbers differ because the two RECORDINGS differ - matching the
/// gains would put the fold 4 dB over the rise. Set A-weighted, like every
/// level in this table now is.
const STOW_OPEN_VOLUME: f32 = 0.09;
const STOW_CLOSE_VOLUME: f32 = 0.14;

/// The magazine gauge inside the cockpit, alongside the gun's own dead-trigger
/// click ([`DRY_FIRE_VOLUME`]) out on the mount. About 4 dB over it: the panel
/// is nearer than the mount, and it is the half of the pair that says WHY.
const AMMO_DRY_VOLUME: f32 = 0.17;

/// The threat alarm. Louder than the ship's own lock chirp ([`LOCK_ON_VOLUME`])
/// - about 4 dB - because acquiring a target is a thing the pilot chose and
/// being acquired is not. Still under the guns: it has to cut through a fight
/// without being the fight.
const WARN_LOCK_VOLUME: f32 = 0.37;

/// The hull alarm - the gravest thing the cockpit says, and about 4 dB over the
/// threat alarm ([`WARN_LOCK_VOLUME`]) by the same step that one takes over the
/// lock chirp: being aimed at is bad, and being nearly dead is worse. It stops
/// just under a section failing ([`EXPLOSION_VOLUME`]), which is the ceiling a
/// warning must not cross - the instrument reporting the hull coming apart
/// cannot be louder than the hull coming apart.
const WARN_HULL_VOLUME: f32 = 0.32;

/// The torpedo bay's muzzle iris. Machinery, in the same band as the PDC stow
/// doors, and the two numbers differ for the same reason theirs do: seating is
/// a heavier event than unseating, and one file plays both directions.
const BAY_DOOR_OPEN_VOLUME: f32 = 0.16;
const BAY_DOOR_CLOSE_VOLUME: f32 = 0.20;

/// A shell going back into the lance. Well under the shot
/// ([`RAILGUN_FIRE_VOLUME`], about 6 dB) - it is an answer to a silence, not an
/// event in the fight - but clearly above the machinery band, because twelve
/// seconds of nothing is exactly when a pilot has stopped listening.
const RAILGUN_RELOAD_VOLUME: f32 = 0.17;

/// The capacitor bank, at FULL charge. The loop is driven up to this from
/// [`RAILGUN_CHARGE_FLOOR`] as the shot approaches, and its playback rate rises
/// with it - so the gun sounds like it is arriving at something rather than
/// holding a note. Sits under the main drive: a lance charging is a telegraph,
/// and a telegraph that drowns the burn is a mix error.
const RAILGUN_CHARGE_MAX_VOLUME: f32 = 0.15;
/// Fraction of [`RAILGUN_CHARGE_MAX_VOLUME`] the bank starts at, so the commit
/// is audible without the build having nowhere left to go.
const RAILGUN_CHARGE_FLOOR: f32 = 0.35;
/// Playback rate at full charge. The loop is authored on even partials for
/// exactly this (see `scripts/gen-world-sfx.py`), so the seam stays silent at
/// any rate in between.
const RAILGUN_CHARGE_TOP_SPEED: f32 = 1.6;

/// A whole hull failing, on the STRUCTURAL COLLAPSE edge. About 4 dB over the
/// section explosion ([`EXPLOSION_VOLUME`]) and the loudest thing in the game,
/// which is the point: a ship coming apart has to be obviously bigger than a
/// piece of one coming off, and the only two levers are length and level. The
/// cue uses both - 2.4 seconds of debris over the frames the sections actually
/// peel away.
const DESTROY_SHIP_VOLUME: f32 = 0.65;

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

/// One report per SALVO. Every bay on a ship shares a trigger and a 1 s reload,
/// so a multi-bay hull launches its whole load on one frame; without this the
/// pilot hears N launch thumps summing at once, which is loud rather than
/// impressive. Comfortably under the bay's own cycle, so a second volley still
/// sounds.
const TORPEDO_LAUNCH_MIN_INTERVAL: f32 = 0.15;

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
        app.add_observer(on_stow_doors_play_sfx);
        app.add_observer(on_bay_doors_play_sfx);
        app.add_observer(on_reload_complete_play_sfx);
        app.add_observer(on_collapse_play_hull_loss);

        // Lock/safety UI cues: message-driven one-shots, so no gating needed
        // - the writers (radar search, tap observer) are themselves
        // pause-gated. The dry-fire click polls turret input/ammo and
        // edge-latches per turret; a pause freezes the input so no fresh edge
        // fires while paused.
        app.add_systems(
            Update,
            (
                play_lock_cues,
                play_safety_engaged_cue,
                play_dry_fire_cue,
                play_threat_lock_cue,
                play_hull_warning_cue,
            ),
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
            (
                drive_thruster_loops,
                drive_rcs_loops,
                drive_railgun_charge_loops,
            )
                .in_set(SpaceshipSectionSystems),
        );
    }
}
