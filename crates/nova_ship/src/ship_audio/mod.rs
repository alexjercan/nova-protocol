//! The ship's soundtrack: the mapping from Nova gameplay events to sounds,
//! played through the generic engine in [`nova_sound`].
//!
//! The one-shots are fired from existing seams via observers, so no gameplay
//! system has to know about audio:
//! - a section/asteroid destroyed -> `Explosion`
//!   (`On<Add<IntegrityDestroyMarker>>`);
//! - damage applied to a target -> `Impact` (`On<HealthApplyDamage>`);
//! - a turret round spawned -> the firing turret's authored `fire_sound`
//!   (`On<Add<TurretBulletProjectileMarker>>`, authored-or-silent);
//! - a torpedo spawned -> the bay's authored `launch_sound`
//!   (`On<TorpedoLaunched>`, authored-or-silent);
//! - a torpedo's drive lighting -> the bay's authored `ignition_sound`
//!   (`On<TorpedoIgnited>`);
//! - a torpedo's warhead bursting -> the bay's authored `detonation_sound`
//!   (`On<Add<NovaBlast>>` on a torpedo blast);
//! - a reload batch landing -> the section's authored reload sound, the
//!   lance's breech or the bay's rack (`On<SectionReloadLanded>`);
//! - a lance discharging -> the railgun's authored `fire_sound` (`On<RailgunFired>`);
//! - a torpedo dying unfired -> the bay's authored `dud_sound` or
//!   `intercept_sound` (`On<TorpedoKilled>`);
//! - the player's weapon getting the kill -> the ship's authored
//!   `kill_confirm` stinger (`On<PlayerKillConfirmed>`).
//!
//! The closing tone polls instead: the ship's authored `torpedo_closing` tick,
//! faster as `PlayerTorpedoEta` falls (`play_torpedo_closing_tone`).
//!
//! The continuous cues - the thruster hum and the RCS hiss - are one looping
//! voice per (ship, authored sound) pair, each following its own ship.
//!
//! Every cue names a `nova_sound` category and where it happens. The category
//! table owns the level, the reach and the repeat budget, so nothing here
//! holds a gain. World cues play in the world, the player's own ship
//! included; the cockpit cues are interface feedback, heard flat.
//!
//! Every system here degrades gracefully (does nothing) until the resources it
//! needs exist. World sounds carry no bank at all - each cue resolves its
//! target's authored `AssetRef` (authored-or-silent). The cargo intake's door,
//! eject and take sounds are the exception: content must author all three.

use bevy::prelude::*;

use crate::prelude::*;

/// `ShipAudioPlugin` and its ordering handle.
pub mod prelude {
    pub use super::{feedback::prelude::*, ShipAudioPlugin, ShipAudioSystems};
}

mod combat;
mod cues;
pub mod feedback;
mod loops;
mod machinery;

#[cfg(test)]
mod test_support;

use self::{
    combat::{
        on_collapse_play_hull_loss, on_destroyed_play_explosion, on_player_kill_play_stinger,
        on_railgun_fire_play_sfx, on_reload_landed_play_sfx, on_surface_impact_play_sfx,
        on_torpedo_blast_play_detonation, on_torpedo_ignited_play_sfx, on_torpedo_killed_play_sfx,
        on_torpedo_launch_play_sfx, on_turret_fire_play_sfx,
    },
    cues::{
        on_ammo_reserve_crossed_play_cue, play_dry_fire_cue, play_hull_warning_cue, play_lock_cues,
        play_safety_engaged_cue, play_threat_lock_cue, play_torpedo_closing_tone,
    },
    loops::{drive_railgun_charge_loops, drive_rcs_loops, drive_thruster_loops},
    machinery::{
        on_bay_doors_play_sfx, on_canister_ejected_play_sfx, on_canister_taken_play_sfx,
        on_dock_clamped_play_sfx, on_dock_released_play_sfx, on_intake_door_play_sfx,
        on_mining_doors_play_sfx, on_stow_doors_play_sfx,
    },
};

/// The capacitor loop's playback rate at full charge. The loop is authored on
/// even partials for exactly this (see `scripts/gen-world-sfx.py`), so the
/// seam stays silent at any rate in between.
const RAILGUN_CHARGE_TOP_SPEED: f32 = 1.6;

/// The ship's one-shot CUE systems: the cockpit's lock, safety, dry-fire,
/// threat, hull-warning and torpedo closing voices.
///
/// In `Update`, after [`SpaceshipSectionSystems`]. Four of the six POLL
/// state rather than reading a message - the dry-fire click edge-latches per
/// turret off this frame's trigger and magazine, the hull alarm off this
/// frame's health, and the closing tone off this frame's
/// `PlayerTorpedoEta` - so the set answers the tick the sections just took
/// rather than the one before it. The engine's own `SoundSystems`
/// mixing pass is in `PostUpdate`, so every voice raised here is placed and
/// levelled before a sink opens for it.
///
/// The LOOP passes are deliberately not in this set. They poll
/// `ThrusterSectionInput` and `RcsIntent`, so they belong to
/// [`SpaceshipSectionSystems`] and inherit its scenario and pause gating; a
/// cue fires on a message or an edge whose writer is gated already.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShipAudioSystems;

/// Plugin wiring the ship's gameplay events to sound effects: the combat
/// observers, the cockpit cues, and the thruster/RCS loops. Requires the
/// engine ([`nova_sound::NovaSoundPlugin`]), which it adds if absent.
#[derive(Default)]
pub struct ShipAudioPlugin;

impl Plugin for ShipAudioPlugin {
    fn build(&self, app: &mut App) {
        trace!("ShipAudioPlugin: build");

        if !app.is_plugin_added::<nova_sound::NovaSoundPlugin>() {
            app.add_plugins(nova_sound::NovaSoundPlugin);
        }

        app.add_observer(on_destroyed_play_explosion);
        app.add_observer(on_surface_impact_play_sfx);
        app.add_observer(on_turret_fire_play_sfx);
        app.add_observer(on_torpedo_launch_play_sfx);
        app.add_observer(on_torpedo_ignited_play_sfx);
        app.add_observer(on_torpedo_blast_play_detonation);
        app.add_observer(on_railgun_fire_play_sfx);
        app.add_observer(on_torpedo_killed_play_sfx);
        app.add_observer(on_player_kill_play_stinger);
        app.add_observer(on_stow_doors_play_sfx);
        app.add_observer(on_bay_doors_play_sfx);
        app.add_observer(on_intake_door_play_sfx);
        app.add_observer(on_mining_doors_play_sfx);
        app.add_observer(on_canister_ejected_play_sfx);
        app.add_observer(on_canister_taken_play_sfx);
        app.add_observer(on_dock_clamped_play_sfx);
        app.add_observer(on_dock_released_play_sfx);
        app.add_observer(on_reload_landed_play_sfx);
        app.add_observer(on_collapse_play_hull_loss);
        app.add_observer(on_ammo_reserve_crossed_play_cue);

        // Lock/safety UI cues: message-driven one-shots, so no gating needed
        // - the writers (radar search, tap observer) are themselves
        // pause-gated. The dry-fire click polls turret input/ammo and
        // edge-latches per turret; a pause freezes the input so no fresh edge
        // fires while paused. The SET is where that ordering is stated, so
        // anything outside can order against the ship's voice as a whole -
        // see `ShipAudioSystems`.
        app.configure_sets(Update, ShipAudioSystems.after(SpaceshipSectionSystems));
        app.add_systems(
            Update,
            (
                play_lock_cues,
                play_safety_engaged_cue,
                play_dry_fire_cue,
                play_threat_lock_cue,
                play_hull_warning_cue,
                play_torpedo_closing_tone,
            )
                .in_set(ShipAudioSystems),
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
        // Both write the loops' INTENSITIES in `Update`; the engine's
        // `SoundSystems` pass reads them in `PostUpdate` and owns everything
        // after - the level, the fade, the pan, the master, the freeze behind
        // a paused sim and the teardown on scenario unload.
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
