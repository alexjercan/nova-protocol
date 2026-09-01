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
//! [`SectionReloadConfig`] on the weapon config). Every successful shot resets
//! its timer; each uninterrupted delay restores one authored batch until the
//! magazine is full. Reload rides on the magazine, so a weapon with no
//! [`SectionAmmo`] never reloads and stays unlimited. Multiple
//! ammo/bullet types (Kinetic, Pierce) landed as the `LoadedBullet` slot; a
//! future per-type magazine would replace the scalar pool while keeping the
//! same consume-one-to-fire contract the weapon systems rely on.
//!
//! [`TurretSectionConfig`]: super::turret_section::TurretSectionConfig
//! [`TorpedoSectionConfig`]: super::torpedo_section::TorpedoSectionConfig

use bevy::prelude::*;
use nova_gameplay::prelude::SectionInactiveMarker;

/// `SectionAmmo`, `SectionReload` and `SectionReloadConfig`.
pub mod prelude {
    pub use super::{SectionAmmo, SectionReload, SectionReloadComplete, SectionReloadConfig};
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
    /// Magazine size - what a reload refills `rounds` to. Kept so the HUD and
    /// the reload have the full/empty reference without a second source of
    /// truth.
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
/// [`TurretSectionConfig`]: super::turret_section::TurretSectionConfig
/// [`TorpedoSectionConfig`]: super::torpedo_section::TorpedoSectionConfig
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SectionReloadConfig {
    /// Seconds without a successful shot before one batch returns.
    pub delay: f32,
    /// Rounds restored by one completed delay, clamped to magazine capacity.
    pub amount: u32,
}

/// Runtime reload state for a weapon section, seeded from a
/// [`SectionReloadConfig`]. Lives on the same SECTION entity as [`SectionAmmo`];
/// [`tick_section_reload`] advances it and refills the magazine. Carries the
/// authored parameters plus the in-flight cycle progress so the HUD ammo readout
/// can render a reload/recharge state without a second source of truth.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct SectionReload {
    /// Seconds without a shot before one batch returns.
    pub delay: f32,
    /// Rounds restored by one completed delay.
    pub amount: u32,
    /// Seconds accumulated since the last shot or restored batch.
    pub elapsed: f32,
    /// A shot landed earlier in this fixed tick. The reload pass consumes this
    /// edge without advancing, so the authored delay begins after that tick.
    interrupted: bool,
}

impl SectionReload {
    /// Seed runtime reload state from authored parameters.
    pub fn from_config(config: SectionReloadConfig) -> Self {
        debug_assert!(
            config.delay > 0.0 && config.delay.is_finite(),
            "SectionReloadConfig.delay must be positive and finite (got {})",
            config.delay,
        );
        debug_assert!(
            config.amount > 0,
            "SectionReloadConfig.amount must be positive"
        );
        Self {
            delay: config.delay,
            amount: config.amount,
            elapsed: 0.0,
            interrupted: false,
        }
    }

    /// Record a successful shot. Empty trigger pulls never call this.
    pub fn on_shot(&mut self) {
        self.elapsed = 0.0;
        self.interrupted = true;
    }

    /// Fraction of the current delay completed, in `0.0..=1.0`.
    pub fn progress(&self) -> f32 {
        if self.delay > 0.0 {
            (self.elapsed / self.delay).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// True while this section is below capacity and accumulating a batch.
    pub fn is_reloading(&self, ammo: &SectionAmmo) -> bool {
        ammo.rounds < ammo.capacity
    }

    /// Ammunition after the next batch, clamped to magazine capacity.
    pub fn incoming_rounds(&self, ammo: &SectionAmmo) -> u32 {
        ammo.rounds.saturating_add(self.amount).min(ammo.capacity)
    }

    /// Advance by `dt` seconds and restore each complete batch. A successful
    /// shot earlier in this tick wins the boundary and starts a fresh delay.
    pub fn advance(&mut self, ammo: &mut SectionAmmo, dt: f32) {
        if !self.is_reloading(ammo) {
            self.elapsed = 0.0;
            self.interrupted = false;
            return;
        }
        if self.interrupted {
            self.interrupted = false;
            return;
        }
        self.elapsed += dt;
        while self.delay > 0.0 && self.elapsed >= self.delay {
            self.elapsed -= self.delay;
            ammo.rounds = self.incoming_rounds(ammo);
            if ammo.rounds >= ammo.capacity {
                self.elapsed = 0.0;
                break;
            }
        }
    }
}

/// Advance every section's reload cycle and refill its magazine. Fire systems
/// run first and call [`SectionReload::on_shot`], so a shot on the completion
/// boundary resets the timer instead of receiving a simultaneous batch.
///
/// A section with no [`SectionReload`] (or no [`SectionAmmo`]) never reloads,
/// preserving the unlimited-ammo default. `Res<Time>` here is the fixed clock.
///
/// Reports [`SectionReloadComplete`] on the batch that brings a magazine back
/// to CAPACITY - see that event for why full, and not each batch, is the thing
/// worth telling anyone about.
pub fn tick_section_reload(
    time: Res<Time>,
    mut commands: Commands,
    // A disabled section neither refills nor reports: the gun can never fire
    // again, and the breech clunk would arrive from a piece of wreckage.
    mut q: Query<(Entity, &mut SectionAmmo, &mut SectionReload), Without<SectionInactiveMarker>>,
) {
    let dt = time.delta_secs();
    for (section, mut ammo, mut reload) in &mut q {
        let was_full = ammo.rounds >= ammo.capacity;
        reload.advance(&mut ammo, dt);
        if !was_full && ammo.rounds >= ammo.capacity {
            commands.trigger(SectionReloadComplete { entity: section });
        }
    }
}

/// A weapon section's magazine came back to CAPACITY.
///
/// FULL, not "a batch returned", because full is the only reload boundary that
/// means the same thing on every weapon. A PDC trickling rounds back has no
/// moment its reload finished - it never stopped - while a one-shell lance has
/// exactly one, and it is the twelve-second silence ending. Reporting each
/// batch would fire several times a second per mount across a fleet and say
/// nothing at any of them.
///
/// The gameplay seam only: what a section DOES with it (the lance's chamber
/// clunk) is the audio layer's, and a section that authors nothing is silent.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct SectionReloadComplete {
    /// The section whose magazine filled.
    pub entity: Entity,
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

    fn reload_cfg(delay: f32, amount: u32) -> SectionReload {
        SectionReload::from_config(SectionReloadConfig { delay, amount })
    }

    #[test]
    fn idle_cycles_restore_batches_and_clamp_at_capacity() {
        let mut ammo = SectionAmmo::new(500);
        ammo.rounds = 0;
        let mut reload = reload_cfg(3.0, 200);

        reload.advance(&mut ammo, 2.0);
        assert_eq!(ammo.rounds, 0);
        assert!((reload.progress() - 2.0 / 3.0).abs() < 1e-6);
        reload.advance(&mut ammo, 1.0);
        assert_eq!(ammo.rounds, 200);
        assert_eq!(reload.progress(), 0.0);
        reload.advance(&mut ammo, 6.0);
        assert_eq!(ammo.rounds, 500, "the final batch clamps to capacity");
        assert_eq!(reload.progress(), 0.0);
    }

    #[test]
    fn a_successful_shot_resets_progress_and_wins_its_tick() {
        let mut ammo = SectionAmmo::new(6);
        ammo.rounds = 5;
        let mut reload = reload_cfg(10.0, 1);
        reload.advance(&mut ammo, 9.5);
        assert!((reload.progress() - 0.95).abs() < 1e-6);

        assert!(ammo.try_consume());
        reload.on_shot();
        reload.advance(&mut ammo, 1.0);
        assert_eq!(ammo.rounds, 4, "the shot tick receives no reload batch");
        assert_eq!(reload.progress(), 0.0);
        reload.advance(&mut ammo, 10.0);
        assert_eq!(ammo.rounds, 5);
    }

    #[test]
    fn an_empty_trigger_pull_does_not_reset_reload() {
        let mut ammo = SectionAmmo::new(1);
        ammo.rounds = 0;
        let mut reload = reload_cfg(2.0, 1);
        reload.advance(&mut ammo, 1.0);
        assert!(!ammo.try_consume());
        reload.advance(&mut ammo, 1.0);
        assert_eq!(ammo.rounds, 1);
    }

    #[test]
    fn incoming_rounds_describes_only_the_next_batch() {
        let mut ammo = SectionAmmo::new(500);
        ammo.rounds = 100;
        let reload = reload_cfg(3.0, 200);
        assert_eq!(reload.incoming_rounds(&ammo), 300);
        ammo.rounds = 450;
        assert_eq!(reload.incoming_rounds(&ammo), 500);
    }

    #[test]
    fn a_full_magazine_stays_at_rest() {
        let mut ammo = SectionAmmo::new(4);
        let mut reload = reload_cfg(0.5, 2);
        assert!(!reload.is_reloading(&ammo));
        reload.advance(&mut ammo, 100.0);
        assert_eq!(ammo.rounds, 4);
        assert_eq!(reload.progress(), 0.0);
    }

    #[test]
    fn progress_clamps_at_one() {
        let reload = SectionReload {
            elapsed: 99.0,
            ..reload_cfg(4.0, 2)
        };
        assert_eq!(reload.progress(), 1.0);
    }

    #[test]
    fn scheduled_reload_restores_one_batch_after_the_delay() {
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
        let mut ammo = SectionAmmo::new(500);
        ammo.rounds = 0;
        let section = app.world_mut().spawn((ammo, reload_cfg(2.0, 200))).id();

        app.update();
        assert_eq!(app.world().get::<SectionAmmo>(section).unwrap().rounds, 0);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().get::<SectionAmmo>(section).unwrap().rounds, 200);
    }

    /// FULL is the boundary that is reported, and it is reported once. A
    /// magazine that refills in several batches must stay quiet through the
    /// ones that do not finish it, and a magazine already full must not report
    /// every tick it spends sitting there.
    #[test]
    fn a_magazine_reports_the_tick_it_comes_back_to_capacity_and_not_before_or_after() {
        use bevy::time::{TimeUpdateStrategy, Virtual};

        #[derive(Resource, Default)]
        struct Filled(usize);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<Filled>();
        let mut virtual_time = Time::<Virtual>::default();
        virtual_time.set_max_delta(std::time::Duration::from_secs(3600));
        app.insert_resource(virtual_time);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0),
        ));
        app.add_systems(Update, tick_section_reload);
        app.add_observer(|_: On<SectionReloadComplete>, mut filled: ResMut<Filled>| filled.0 += 1);

        // Two batches to fill: the first must be silent.
        let mut ammo = SectionAmmo::new(2);
        ammo.rounds = 0;
        let section = app.world_mut().spawn((ammo, reload_cfg(1.0, 1))).id();

        app.update(); // warm-up, dt 0
        app.update(); // first batch: one of two rounds back
        assert_eq!(app.world().get::<SectionAmmo>(section).unwrap().rounds, 1);
        assert_eq!(
            app.world().resource::<Filled>().0,
            0,
            "a part-filled magazine has not finished reloading"
        );

        app.update(); // second batch: full
        assert_eq!(app.world().get::<SectionAmmo>(section).unwrap().rounds, 2);
        assert_eq!(app.world().resource::<Filled>().0, 1);

        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<Filled>().0,
            1,
            "a magazine sitting full reports nothing"
        );
    }
}
