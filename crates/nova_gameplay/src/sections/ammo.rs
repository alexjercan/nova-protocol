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
//! A section may also carry a [`SectionReload`] (seeded from a
//! [`SectionReloadConfig`] on the weapon config) so a spent magazine refills on
//! its own - discrete auto-reload-on-empty or continuous per-round regen, both
//! from one timer ([`tick_section_reload`]). Reload rides on the magazine, so a
//! weapon with no [`SectionAmmo`] never reloads and stays unlimited (task
//! 20260717-085640). Multiple ammo/bullet types (armor-piercing, EMP) landed as
//! the `LoadedBullet` slot (task 20260712-133349); a future per-type magazine
//! would replace the scalar pool while keeping the same consume-one-to-fire
//! contract the weapon systems rely on.
//!
//! [`TurretSectionConfig`]: super::turret_section::TurretSectionConfig
//! [`TorpedoSectionConfig`]: super::torpedo_section::TorpedoSectionConfig

use bevy::prelude::*;

/// Glob-import surface: `use nova_gameplay::sections::ammo::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{SectionAmmo, SectionReload, SectionReloadConfig};
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

/// Authored reload parameters for a weapon section's magazine.
///
/// Attached to a section's config ([`TurretSectionConfig`] /
/// [`TorpedoSectionConfig`] `reload`); when the section is built WITH a
/// magazine (`ammo_capacity = Some`) a [`SectionReload`] is seeded from this.
/// A weapon with no magazine (unlimited / `infinite_ammo`) gets neither, so the
/// "no [`SectionAmmo`] = unlimited" invariant is untouched.
///
/// One descriptor covers both refill styles the reload spike settled on
/// (tasks/20260716-123556/SPIKE.md):
///
/// - **discrete auto-reload** (`only_when_empty: true`,
///   `rounds_per_cycle = capacity`): the magazine sits until it runs dry, then
///   one `reload_time` cycle refills it to full - the classic "reload on empty";
/// - **continuous regen** (`only_when_empty: false`, `rounds_per_cycle: 1`, a
///   short `reload_time`): the magazine trickles one round back every cycle
///   whenever it is below capacity - a heat-like sustained-fire budget.
///
/// [`TurretSectionConfig`]: super::turret_section::TurretSectionConfig
/// [`TorpedoSectionConfig`]: super::torpedo_section::TorpedoSectionConfig
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SectionReloadConfig {
    /// Seconds one reload cycle takes. Must be > 0 - a non-positive value never
    /// refills (the cycle can't complete), leaving the magazine effectively
    /// non-reloading.
    pub reload_time: f32,
    /// Rounds a completed cycle restores, clamped to `capacity`. Equal to the
    /// magazine capacity gives a full reload; `1` gives per-round regen.
    pub rounds_per_cycle: u32,
    /// When `true` a cycle runs only once the magazine is fully empty (discrete
    /// reload); when `false` it runs whenever `rounds < capacity` (regen).
    pub only_when_empty: bool,
}

/// Runtime reload state for a weapon section, seeded from a
/// [`SectionReloadConfig`]. Lives on the same SECTION entity as [`SectionAmmo`];
/// [`tick_section_reload`] advances it and refills the magazine. Carries the
/// authored parameters plus the in-flight cycle progress so the HUD ammo readout
/// can render a reload/recharge state without a second source of truth.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct SectionReload {
    /// Seconds one reload cycle takes (from config).
    pub reload_time: f32,
    /// Rounds a completed cycle restores (from config).
    pub rounds_per_cycle: u32,
    /// Discrete-on-empty vs continuous-regen trigger (from config).
    pub only_when_empty: bool,
    /// Seconds accumulated into the current cycle. Runtime; 0 at rest.
    pub elapsed: f32,
}

impl SectionReload {
    /// Seed runtime reload state from authored parameters (no cycle in flight).
    pub fn from_config(config: SectionReloadConfig) -> Self {
        debug_assert!(
            config.reload_time > 0.0,
            "SectionReloadConfig.reload_time must be positive to ever refill (got {})",
            config.reload_time,
        );
        Self {
            reload_time: config.reload_time,
            rounds_per_cycle: config.rounds_per_cycle,
            only_when_empty: config.only_when_empty,
            elapsed: 0.0,
        }
    }

    /// Fraction of the current reload cycle completed, in `0.0..=1.0`. Zero at
    /// rest (and whenever `reload_time` is non-positive, which cannot refill).
    /// The single value the HUD reads to draw a reload sweep.
    pub fn progress(&self) -> f32 {
        if self.reload_time > 0.0 {
            (self.elapsed / self.reload_time).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// True while a reload cycle is actually accumulating - the magazine is
    /// below capacity and (for a discrete reload) already empty. When this is
    /// false the state is held at rest with `elapsed = 0`.
    pub fn is_reloading(&self, ammo: &SectionAmmo) -> bool {
        if ammo.rounds >= ammo.capacity {
            return false;
        }
        !(self.only_when_empty && ammo.rounds > 0)
    }

    /// Advance this section's reload cycle by `dt` seconds and refill `ammo`.
    /// Add-only: it only ever grows `rounds` toward `capacity`, never shrinks
    /// it. Holds at rest (`elapsed = 0`) when not reloading. A long `dt` or a
    /// tiny `reload_time` completes multiple cycles in one step, so the refill
    /// is exact rather than dropping the surplus. The pure core of
    /// [`tick_section_reload`], factored out so it is directly unit-testable.
    pub fn advance(&mut self, ammo: &mut SectionAmmo, dt: f32) {
        if !self.is_reloading(ammo) {
            self.elapsed = 0.0;
            return;
        }
        self.elapsed += dt;
        while self.reload_time > 0.0 && self.elapsed >= self.reload_time {
            self.elapsed -= self.reload_time;
            ammo.rounds = (ammo.rounds + self.rounds_per_cycle).min(ammo.capacity);
            if ammo.rounds >= ammo.capacity {
                // Refilled: drop leftover progress so a full mag reads at rest.
                self.elapsed = 0.0;
                break;
            }
        }
    }
}

/// Advance every section's reload cycle and refill its magazine. Add-only: it
/// never removes rounds (the fire systems own consumption), so running it in the
/// same `FixedUpdate` schedule as `shoot_spawn_projectile` needs no ordering
/// against them - one system only grows `rounds`, the other only shrinks it.
///
/// A section with no [`SectionReload`] (or no [`SectionAmmo`]) never reloads,
/// preserving the unlimited-ammo default. `Res<Time>` here is the fixed clock.
pub fn tick_section_reload(time: Res<Time>, mut q: Query<(&mut SectionAmmo, &mut SectionReload)>) {
    let dt = time.delta_secs();
    for (mut ammo, mut reload) in &mut q {
        reload.advance(&mut ammo, dt);
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

    fn reload_cfg(reload_time: f32, rounds_per_cycle: u32, only_when_empty: bool) -> SectionReload {
        SectionReload::from_config(SectionReloadConfig {
            reload_time,
            rounds_per_cycle,
            only_when_empty,
        })
    }

    #[test]
    fn discrete_reload_waits_for_empty_then_refills_to_full() {
        // only_when_empty + a full-capacity cycle = "reload on empty".
        let mut ammo = SectionAmmo::new(10);
        let mut reload = reload_cfg(2.0, 10, true);

        // While rounds remain, the cycle never starts: progress stays at rest
        // no matter how long we wait.
        ammo.rounds = 4;
        reload.advance(&mut ammo, 5.0);
        assert_eq!(ammo.rounds, 4, "a non-empty discrete mag must not reload");
        assert_eq!(reload.progress(), 0.0);

        // Emptied by fire, the reload begins. Half a cycle is only progress, no
        // rounds back yet.
        ammo.rounds = 0;
        reload.advance(&mut ammo, 1.0);
        assert_eq!(ammo.rounds, 0, "mid-cycle refills nothing");
        assert!((reload.progress() - 0.5).abs() < 1e-6);

        // The cycle completing refills the whole magazine and returns to rest.
        reload.advance(&mut ammo, 1.0);
        assert_eq!(
            ammo.rounds, 10,
            "a completed discrete cycle refills to full"
        );
        assert_eq!(reload.progress(), 0.0);
    }

    #[test]
    fn continuous_regen_adds_one_round_per_cycle_and_clamps_at_capacity() {
        // !only_when_empty + a one-round cycle = trickle regen from any level.
        let mut ammo = SectionAmmo::new(3);
        let mut reload = reload_cfg(1.0, 1, false);
        ammo.rounds = 1;

        reload.advance(&mut ammo, 1.0);
        assert_eq!(ammo.rounds, 2, "one cycle restores exactly one round");
        reload.advance(&mut ammo, 1.0);
        assert_eq!(ammo.rounds, 3, "regen reaches capacity");

        // At capacity it must never overfill, and it holds at rest.
        reload.advance(&mut ammo, 10.0);
        assert_eq!(ammo.rounds, 3, "regen never exceeds capacity");
        assert_eq!(reload.progress(), 0.0);
    }

    #[test]
    fn a_long_step_completes_multiple_regen_cycles_exactly() {
        // A dt spanning several cycles must not drop the surplus rounds.
        let mut ammo = SectionAmmo::new(5);
        let mut reload = reload_cfg(1.0, 1, false);
        ammo.rounds = 0;
        reload.advance(&mut ammo, 3.5);
        assert_eq!(ammo.rounds, 3, "3.5 cycles restores 3 whole rounds");
        assert!(
            (reload.progress() - 0.5).abs() < 1e-6,
            "leftover half-cycle kept"
        );
    }

    #[test]
    fn a_full_magazine_never_reloads_and_stays_at_rest() {
        let mut ammo = SectionAmmo::new(4);
        let mut reload = reload_cfg(0.5, 4, false);
        assert!(!reload.is_reloading(&ammo));
        reload.advance(&mut ammo, 100.0);
        assert_eq!(ammo.rounds, 4);
        assert_eq!(reload.progress(), 0.0);
    }

    #[test]
    fn progress_rises_from_zero_toward_one_across_a_cycle() {
        let mut ammo = SectionAmmo::new(2);
        let mut reload = reload_cfg(4.0, 2, true);
        ammo.rounds = 0;
        assert_eq!(reload.progress(), 0.0);
        reload.advance(&mut ammo, 1.0);
        assert!((reload.progress() - 0.25).abs() < 1e-6);
        reload.advance(&mut ammo, 2.0);
        assert!((reload.progress() - 0.75).abs() < 1e-6);
        // progress() is clamped even if elapsed somehow overshoots.
        reload.elapsed = 99.0;
        assert_eq!(reload.progress(), 1.0);
    }

    #[test]
    fn tick_section_reload_system_refills_through_the_schedule() {
        // Prove the SYSTEM (not just the method) refills, driven by a manual
        // clock. `Time<Virtual>` clamps its delta to `max_delta` (0.25s by
        // default), which would swallow our 1s manual step, so raise it to let
        // the full duration through; the per-step timing is covered by the
        // `advance` unit tests. Each update after the zeroed first frame then
        // advances ~1s.
        use bevy::time::{TimeUpdateStrategy, Virtual};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let mut virtual_time = Time::<Virtual>::default();
        virtual_time.set_max_delta(std::time::Duration::from_secs(3600));
        app.insert_resource(virtual_time);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0),
        ));
        app.add_systems(Update, tick_section_reload);
        let mut ammo = SectionAmmo::new(6);
        ammo.rounds = 0;
        let section = app.world_mut().spawn((ammo, reload_cfg(2.0, 6, true))).id();

        // One update advances at most 1s of the 2s cycle, so an empty magazine
        // has not refilled yet - the system does not refill instantly.
        app.update();
        assert_eq!(
            app.world().get::<SectionAmmo>(section).unwrap().rounds,
            0,
            "an empty magazine must not refill within a single sub-cycle tick"
        );
        // Drive well past the 2s cycle: the scheduled system refills to full.
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world().get::<SectionAmmo>(section).unwrap().rounds,
            6,
            "the scheduled system must refill an empty magazine to capacity"
        );
    }
}
