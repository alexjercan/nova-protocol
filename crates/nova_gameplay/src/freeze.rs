//! Who is holding the simulation still.
//!
//! Several surfaces stop the world - the pause overlay, the CRT terminal, an
//! outcome banner - and before this module each of them paused and unpaused the
//! clocks unconditionally. That is only correct while exactly one of them can be
//! up: opening the terminal over a paused game and closing it again resumed a
//! game the pause menu still had frozen.
//!
//! [`ClockFreeze`] makes the hold explicit and counted by owner. A clock runs
//! only when nobody holds it, so surfaces may nest and overlap in any order and
//! the last release is the one that starts the world again.

use avian3d::schedule::{Physics, PhysicsTime};
use bevy::{ecs::system::SystemParam, prelude::*};

/// Glob-import surface: `use nova_gameplay::freeze::prelude::*`.
pub mod prelude {
    pub use super::{ClockFreeze, Clocks, FreezeOwner};
}

/// A surface that can stop the simulation.
///
/// Named rather than counted: a bare counter cannot tell a double-hold by one
/// owner (a state re-entry) from two genuine owners, and the first bug this
/// module fixes was exactly a mismatched pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum FreezeOwner {
    /// The pause overlay (`PauseStates::Paused`), and the outcome banner that
    /// forces it.
    PauseMenu,
    /// The CRT terminal, in either shell. It owns the freeze for as long as it
    /// is open, so switching shells never unpauses and re-pauses the world.
    Terminal,
}

impl FreezeOwner {
    /// Every owner, for the fixed-size hold table.
    const ALL: [FreezeOwner; 2] = [FreezeOwner::PauseMenu, FreezeOwner::Terminal];

    fn index(self) -> usize {
        match self {
            FreezeOwner::PauseMenu => 0,
            FreezeOwner::Terminal => 1,
        }
    }
}

/// Which surfaces are currently holding the simulation still.
///
/// The record, not the mechanism: [`Clocks`] is what actually pauses and
/// resumes `Time<Virtual>` and `Time<Physics>`, and it does so the moment a hold
/// is taken or dropped, so a state-transition hook still freezes the world in
/// the same schedule it always did.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Resource)]
pub struct ClockFreeze {
    holds: [bool; FreezeOwner::ALL.len()],
}

impl ClockFreeze {
    /// Whether anything is holding the clocks.
    pub fn is_held(&self) -> bool {
        self.holds.iter().any(|held| *held)
    }

    /// Whether this owner is holding the clocks.
    pub fn is_held_by(&self, owner: FreezeOwner) -> bool {
        self.holds[owner.index()]
    }

    /// Every owner currently holding, in [`FreezeOwner`] order.
    pub fn owners(&self) -> impl Iterator<Item = FreezeOwner> + '_ {
        FreezeOwner::ALL
            .into_iter()
            .filter(|owner| self.is_held_by(*owner))
    }

    fn set(&mut self, owner: FreezeOwner, held: bool) -> bool {
        let at = owner.index();
        let changed = self.holds[at] != held;
        self.holds[at] = held;
        changed
    }
}

/// Take and drop simulation holds, applying them to the clocks immediately.
///
/// `Time<Physics>` is avian's and is optional here so a headless or
/// physics-less rig can still freeze virtual time.
#[derive(SystemParam)]
pub struct Clocks<'w> {
    freeze: ResMut<'w, ClockFreeze>,
    virtual_time: ResMut<'w, Time<Virtual>>,
    physics_time: Option<ResMut<'w, Time<Physics>>>,
}

impl Clocks<'_> {
    /// Stop the simulation on this owner's behalf. Idempotent.
    pub fn hold(&mut self, owner: FreezeOwner) {
        if self.freeze.set(owner, true) {
            self.apply();
        }
    }

    /// Drop this owner's hold. The clocks resume only if nobody else is
    /// holding them - which is the whole point of the resource.
    pub fn release(&mut self, owner: FreezeOwner) {
        if self.freeze.set(owner, false) {
            self.apply();
        }
    }

    /// Whether anything is holding the clocks right now.
    pub fn is_held(&self) -> bool {
        self.freeze.is_held()
    }

    /// Drop every hold. The escape hatch for leaving `GameStates::Playing`,
    /// where no surface is left to release its own.
    pub fn release_all(&mut self) {
        let mut changed = false;
        for owner in FreezeOwner::ALL {
            changed |= self.freeze.set(owner, false);
        }
        if changed {
            self.apply();
        }
    }

    fn apply(&mut self) {
        if self.freeze.is_held() {
            self.virtual_time.pause();
            if let Some(physics) = self.physics_time.as_mut() {
                physics.pause();
            }
        } else {
            self.virtual_time.unpause();
            if let Some(physics) = self.physics_time.as_mut() {
                physics.unpause();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn frozen_world() -> World {
        let mut world = World::new();
        world.init_resource::<ClockFreeze>();
        world.init_resource::<Time<Virtual>>();
        world
    }

    fn is_paused(world: &World) -> bool {
        world.resource::<Time<Virtual>>().is_paused()
    }

    /// Two overlapping surfaces are the case the old unconditional unpause got
    /// wrong: the terminal opened over a paused game, and closing it resumed a
    /// game the pause menu still had frozen.
    #[test]
    fn the_world_runs_again_only_when_the_last_holder_lets_go() {
        let mut world = frozen_world();

        world
            .run_system_once(|mut clocks: Clocks| clocks.hold(FreezeOwner::PauseMenu))
            .unwrap();
        assert!(is_paused(&world));

        world
            .run_system_once(|mut clocks: Clocks| clocks.hold(FreezeOwner::Terminal))
            .unwrap();
        world
            .run_system_once(|mut clocks: Clocks| clocks.release(FreezeOwner::Terminal))
            .unwrap();
        assert!(is_paused(&world), "the pause menu still holds the world");

        world
            .run_system_once(|mut clocks: Clocks| clocks.release(FreezeOwner::PauseMenu))
            .unwrap();
        assert!(!is_paused(&world));
        assert!(!world.resource::<ClockFreeze>().is_held());
    }

    /// A hold taken twice is one hold: re-entering a state must not need two
    /// releases to give the world back.
    #[test]
    fn holds_are_idempotent_per_owner() {
        let mut world = frozen_world();
        world
            .run_system_once(|mut clocks: Clocks| {
                clocks.hold(FreezeOwner::Terminal);
                clocks.hold(FreezeOwner::Terminal);
                clocks.release(FreezeOwner::Terminal);
            })
            .unwrap();
        assert!(!is_paused(&world));
    }

    /// Leaving Playing drops every hold at once, because the surfaces that took
    /// them are being torn down and will never release their own.
    #[test]
    fn release_all_gives_the_world_back_whatever_was_holding_it() {
        let mut world = frozen_world();
        world
            .run_system_once(|mut clocks: Clocks| {
                clocks.hold(FreezeOwner::PauseMenu);
                clocks.hold(FreezeOwner::Terminal);
                clocks.release_all();
            })
            .unwrap();
        assert!(!is_paused(&world));
        assert_eq!(world.resource::<ClockFreeze>().owners().count(), 0);
    }
}
