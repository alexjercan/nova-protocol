//! Generic finite ammunition for weapon sections (turret, torpedo).
//!
//! A weapon section may carry a [`SectionAmmo`] capping how many rounds it can
//! fire before it runs dry. The component is deliberately weapon-agnostic: both
//! the turret and the torpedo bay gate their own fire system on it and spend one
//! round per shot, so the two weapons share a single ammo concept instead of
//! each growing a bespoke counter.
//!
//! Absence of the component means unlimited ammo - exactly the pre-ammo
//! behavior - so opting in is per weapon config ([`TurretSectionConfig`] /
//! [`TorpedoSectionConfig`] `ammo_capacity`). That default also keeps every
//! headless firing test that never asked for ammo firing forever, unchanged.
//!
//! Reloading and multiple ammo/bullet types (armor-piercing, EMP) are out of
//! scope here and land in task 20260708-162005; this single pool is the
//! foundation they extend. A future reload is `rounds = capacity`; a future
//! multi-type magazine replaces the scalar pool while keeping the same
//! consume-one-to-fire contract the weapon systems rely on.
//!
//! [`TurretSectionConfig`]: super::turret_section::TurretSectionConfig
//! [`TorpedoSectionConfig`]: super::torpedo_section::TorpedoSectionConfig

use bevy::prelude::*;

pub mod prelude {
    pub use super::SectionAmmo;
}

/// Rounds remaining in a weapon section's magazine.
///
/// Lives on the weapon SECTION entity (the turret or the torpedo bay), the same
/// entity that holds the section's config helper and fire input, so the fire
/// system reads and decrements it with the query it already runs. A section with
/// no `SectionAmmo` fires without limit.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct SectionAmmo {
    /// Rounds left to fire. Never exceeds `capacity`.
    pub rounds: u32,
    /// Magazine size - what a reload would refill `rounds` to (task
    /// 20260708-162005). Kept so the HUD and a future reload have the full/empty
    /// reference without a second source of truth.
    pub capacity: u32,
}

impl SectionAmmo {
    /// A full magazine of `capacity` rounds.
    pub fn new(capacity: u32) -> Self {
        Self {
            rounds: capacity,
            capacity,
        }
    }

    /// True when the magazine is spent and the weapon can no longer fire.
    pub fn is_empty(&self) -> bool {
        self.rounds == 0
    }

    /// Spend one round if any remain. Returns `true` when a round was consumed
    /// (the shot may fire) and `false` when the magazine was already empty (the
    /// shot is suppressed). The single mutation point for ammo, so both weapons
    /// deplete identically.
    pub fn try_consume(&mut self) -> bool {
        if self.rounds == 0 {
            false
        } else {
            self.rounds -= 1;
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_magazine_is_full_and_not_empty() {
        let ammo = SectionAmmo::new(3);
        assert_eq!(ammo.rounds, 3);
        assert_eq!(ammo.capacity, 3);
        assert!(!ammo.is_empty());
    }

    #[test]
    fn try_consume_spends_exactly_the_capacity_then_refuses() {
        let mut ammo = SectionAmmo::new(2);
        assert!(ammo.try_consume());
        assert_eq!(ammo.rounds, 1);
        assert!(ammo.try_consume());
        assert_eq!(ammo.rounds, 0);
        assert!(ammo.is_empty());
        // The empty boundary: a spent magazine consumes nothing and stays at
        // zero, so a held trigger cannot underflow `rounds`.
        assert!(!ammo.try_consume());
        assert_eq!(ammo.rounds, 0);
    }

    #[test]
    fn a_zero_capacity_magazine_starts_empty() {
        let mut ammo = SectionAmmo::new(0);
        assert!(ammo.is_empty());
        assert!(!ammo.try_consume());
    }
}
