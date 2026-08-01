//! The player's target locks: DELIBERATE radar acquisition onto two sticky
//! ship-root lock slots.
//!
//! - [`TravelLock`] (white crosshair): the nav designation GOTO reads.
//! - [`CombatLock`] (red crosshair): what guns/torpedoes/focus/inset read.
//! - Hold CTRL (`RadarHoldInput`, a `Hold` condition) = radar on: the
//!   picker live-retargets to the best body on the ACTIVE look ray
//!   ([`ActiveLookRay`]). At the hold THRESHOLD the destination slot is
//!   latched from the CURRENT raised stance (combat while [`WeaponsRaised`],
//!   else travel) and written LIVE with the candidate every held frame
//!   (keep-last: sweeping over empty space never drops the lock). Releasing
//!   just ends the search - the lock sticks. A hold that never resolves a
//!   candidate leaves the slots untouched.
//! - Tap CTRL (`RadarClearInput`, a `Tap` condition) = staged clear: the
//!   combat lock first, then the travel lock (disengaging an engaged GOTO);
//!   while raised, only ever the combat lock.
//! - NOTHING locks passively: the old aim-assist cone auto-pick and the
//!   close-range signature auto-acquire are gone. Locks clear naturally on
//!   death/despawn, out-of-range, a hostile target turning non-hostile, and
//!   the combat lock decays after `COMBAT_DECAY_SECS` without combat
//!   activity - where combat activity is the raised stance OR a held weapon
//!   trigger. Every one of those drops names itself: a `debug!` line plus a
//!   [`CombatLockDropped`] message carrying the [`CombatLockDrop`] branch, so
//!   "why did my lock let go?" is answerable from a run rather than guessed.
//!
//! The scanner-wave RANGE model (LockSignature) survives as the radar
//! picker's gate, and [`ThreatContacts`] keeps the ranked hostile set alive
//! for the edge-indicator arrows. All state lives on the PLAYER ship root as
//! components (respawn hygiene; the AI mirrors the same components).

use bevy::prelude::*;

use crate::prelude::*;

pub mod component_lock;
pub mod contacts;
pub mod gesture;
pub mod radar;
pub mod safety;
pub mod state;

use component_lock::{on_component_cycle_prev, update_component_lock};
use contacts::{tick_lock_focus, update_contacts_and_locks};
use gesture::{on_lock_clear_tap, on_radar_cancel, on_radar_commit, on_radar_start};
use radar::update_radar_search;
use safety::{enforce_safety_trigger_interrupt, update_weapons_safety};

#[cfg(test)]
pub(crate) use self::safety::update_weapons_safety_for_tests;
pub(crate) use self::{
    component_lock::{on_component_cycle_next, ComponentCycleNextInput, ComponentCyclePrevInput},
    gesture::{RadarClearInput, RadarHoldInput},
};
pub use self::{
    component_lock::{ComponentLock, ComponentLockMode},
    contacts::{LockFocus, COMBAT_DECAY_SECS},
    gesture::RADAR_TAP_SECS,
    state::{
        targeting_state, CombatDecay, CombatLock, CombatLockDrop, CombatLockDropped,
        LockClearedToast, LockSignature, RadarDenied, RadarLockAcquired, RadarRetargeted,
        RadarSlot, RadarState, TargetingSettings, ThreatContacts, TravelLock, WeaponsHot,
    },
};

/// Glob-import surface: `use nova_gameplay::input::targeting::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{
        targeting_state, CombatDecay, CombatLock, CombatLockDrop, CombatLockDropped, ComponentLock,
        ComponentLockMode, LockClearedToast, LockFocus, LockSignature, RadarDenied,
        RadarLockAcquired, RadarRetargeted, RadarSlot, RadarState, SpaceshipTargetingPlugin,
        SpaceshipTargetingSystems, TargetingSettings, ThreatContacts, TravelLock, WeaponsHot,
        COMBAT_DECAY_SECS, RADAR_TAP_SECS,
    };
}

/// System set for the lock update, so consumers (torpedo commit, turret
/// feed) can order after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipTargetingSystems;

/// Plugin owning the lock components, the radar gesture and the per-frame
/// contact/validity upkeep.
pub struct SpaceshipTargetingPlugin;

impl Plugin for SpaceshipTargetingPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipTargetingPlugin: build");

        app.init_resource::<TargetingSettings>();
        app.register_type::<TargetingSettings>();
        app.register_type::<LockSignature>();
        app.register_type::<TravelLock>();
        app.register_type::<CombatLock>();
        app.register_type::<RadarState>();
        app.register_type::<RadarSlot>();
        app.register_type::<CombatDecay>();
        app.register_type::<ThreatContacts>();
        app.register_type::<WeaponsHot>();
        app.register_type::<LockFocus>();
        app.register_type::<ComponentLock>();
        app.add_message::<LockClearedToast>();
        app.add_message::<RadarLockAcquired>();
        app.add_message::<RadarRetargeted>();
        app.add_message::<RadarDenied>();
        app.add_message::<CombatLockDropped>();
        app.register_type::<CombatLockDrop>();

        // NOTE: the state bundle rides the player marker wherever ships spawn
        // (observer-over-spawn-site).
        app.add_observer(insert_targeting_state);

        app.add_systems(
            Update,
            (
                update_contacts_and_locks,
                update_radar_search,
                update_weapons_safety,
                enforce_safety_trigger_interrupt,
                tick_lock_focus,
                update_component_lock,
            )
                .chain()
                .in_set(SpaceshipTargetingSystems)
                .in_set(super::SpaceshipInputSystems),
        );
        app.add_observer(on_radar_start);
        app.add_observer(on_radar_commit);
        app.add_observer(on_radar_cancel);
        app.add_observer(on_lock_clear_tap);
        app.add_observer(on_component_cycle_next);
        app.add_observer(on_component_cycle_prev);
    }
}

/// Give every player ship root its targeting state the moment it is marked.
fn insert_targeting_state(add: On<Add, PlayerSpaceshipMarker>, mut commands: Commands) {
    if let Ok(mut ship) = commands.get_entity(add.entity) {
        ship.insert(targeting_state());
    }
}
