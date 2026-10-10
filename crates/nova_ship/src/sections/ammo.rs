//! Generic finite ammunition for weapon sections (turret, torpedo).
//!
//! A weapon section may carry a [`SectionAmmo`] capping how many rounds it can
//! fire before it runs dry. The component is deliberately weapon-agnostic: both
//! the turret and the torpedo bay gate their own fire system on it and spend one
//! round per shot, so the two weapons share a single ammo concept instead of
//! each growing a bespoke counter.
//!
//! Absence of the component means unlimited ammo, which is what
//! [`AmmoCapacity::Unlimited`] on the weapon config ([`TurretSectionConfig`] /
//! [`TorpedoSectionConfig`] `ammunition`) asks for. That default also keeps
//! every headless firing test that never asked for ammo firing forever.
//!
//! A section may also carry a [`SectionReload`] (seeded from a
//! [`SectionReloadConfig`] on the weapon config and the weapon's ammunition
//! item id). Every successful shot resets its timer; each uninterrupted
//! delay moves one batch of that item from the parent ship's [`ShipInventory`]
//! into the magazine, never more than the magazine is missing or the ship
//! carries. A ship with no matching item makes no reload progress. An
//! installed magazine starts full without drawing on the inventory. Reload
//! rides on the magazine, so a weapon with no [`SectionAmmo`] never reloads and
//! stays unlimited. Multiple ammo/bullet types (Kinetic, Pierce) landed as the
//! `LoadedBullet` slot; a future per-type magazine would replace the scalar
//! pool while keeping the same consume-one-to-fire contract the weapon systems
//! rely on.
//!
//! [`TurretSectionConfig`]: super::turret_section::TurretSectionConfig
//! [`TorpedoSectionConfig`]: super::torpedo_section::TorpedoSectionConfig

use bevy::prelude::*;
use nova_gameplay::prelude::{ItemDesignId, SectionInactiveMarker, ShipInventory};

/// `SectionAmmo`, `SectionReload` and `SectionReloadConfig`.
pub mod prelude {
    pub use super::{
        AmmoCapacity, ReloadConfig, SectionAmmo, SectionReload, SectionReloadComplete,
        SectionReloadConfig, SuspendedSectionAmmo,
    };
}

/// Rounds remaining in a weapon section's magazine.
///
/// Lives on the weapon SECTION entity (the turret or the torpedo bay), the same
/// entity that holds the section's config helper and fire input, so the fire
/// system reads and decrements it with the query it already runs. A section with
/// no `SectionAmmo` fires without limit.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
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

/// The magazine a section had before unlimited ammunition was switched on.
///
/// Unlimited ammunition works by REMOVING [`SectionAmmo`] and [`SectionReload`],
/// because "no magazine" is already the game's word for a weapon that never
/// runs dry - there is no second code path to keep honest. That leaves the
/// question of what turning it off restores, and this is the answer: the
/// section's authored capacity, full, with no reload in flight.
///
/// The alternative - putting back the round count from the moment the cheat was
/// armed - was rejected. While the cheat is on the weapon never spends a round,
/// so a stale count is not a state the world was ever in; restoring it would
/// invent a half-empty magazine out of a fight the player did not have. Full is
/// what the magazine has effectively held the whole time.
///
/// Only sections that HAD a magazine carry this, so a weapon authored unlimited
/// stays unlimited when the cheat goes off.
#[derive(Component, Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SuspendedSectionAmmo {
    /// The authored magazine size to restore, full.
    pub capacity: u32,
    /// The reload to restore with it, with its ammunition item; `None` for a
    /// magazine that never reloaded.
    pub reload: Option<SectionReload>,
}

/// How much a weapon section can hold.
///
/// A domain mode rather than an `Option<u32>`: "this gun never runs out" is a
/// DECISION about the weapon, not a missing number, and an author reading
/// `ammunition: Unlimited` is told which of the two it is. The editor shows
/// the same two choices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AmmoCapacity {
    /// No magazine at all: the weapon fires without limit and never reloads.
    /// The section gets neither [`SectionAmmo`] nor [`SectionReload`].
    #[default]
    Unlimited,
    /// A magazine of exactly this many rounds.
    Limited(u32),
}

impl AmmoCapacity {
    /// The magazine size, or `None` when the weapon is unlimited.
    pub fn rounds(self) -> Option<u32> {
        match self {
            AmmoCapacity::Unlimited => None,
            AmmoCapacity::Limited(rounds) => Some(rounds),
        }
    }
}

/// Whether a weapon section restores rounds on its own, and how.
///
/// The same reasoning as [`AmmoCapacity`]: a gun that is dry for good is an
/// authored decision, and `Disabled` says so where an absent field could only
/// imply it. A `Batch` on an [`AmmoCapacity::Unlimited`] weapon is a content
/// lint error - there is nothing to refill.
#[derive(Clone, Copy, Debug, PartialEq, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ReloadConfig {
    /// Spent rounds never come back. Once the magazine is empty the section is
    /// dry for the rest of its life.
    #[default]
    Disabled,
    /// One batch returns after each idle delay.
    Batch(SectionReloadConfig),
}

impl ReloadConfig {
    /// The authored batch, or `None` when reloading is off.
    pub fn batch(self) -> Option<SectionReloadConfig> {
        match self {
            ReloadConfig::Disabled => None,
            ReloadConfig::Batch(config) => Some(config),
        }
    }
}

/// Authored reload parameters for a weapon section's magazine.
///
/// Carried by [`ReloadConfig::Batch`] on a section's config; when the section
/// is built with an [`AmmoCapacity::Limited`] magazine a [`SectionReload`] is
/// seeded from this. An [`AmmoCapacity::Unlimited`] weapon gets neither, so
/// the "no [`SectionAmmo`] = unlimited" invariant is untouched.
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
/// [`tick_section_reload`] advances it and refills the magazine from the parent
/// ship's [`ShipInventory`]. Carries the authored parameters plus the in-flight
/// cycle progress so the HUD ammo readout can render a reload/recharge state
/// without a second source of truth.
#[derive(Component, Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SectionReload {
    /// The inventory item one round of this magazine consumes.
    pub item: ItemDesignId,
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
    /// Seed runtime reload state from authored parameters and the weapon's
    /// ammunition `item`.
    pub fn from_config(config: SectionReloadConfig, item: ItemDesignId) -> Self {
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
            item,
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

    /// True while this section is below capacity and `reserve`, the parent
    /// ship's count of [`item`](Self::item), can supply a batch.
    pub fn is_reloading(&self, ammo: &SectionAmmo, reserve: u32) -> bool {
        ammo.rounds < ammo.capacity && reserve > 0
    }

    /// Ammunition after the next batch: the batch is the least of what the
    /// magazine is missing, the authored amount and `reserve`.
    pub fn incoming_rounds(&self, ammo: &SectionAmmo, reserve: u32) -> u32 {
        ammo.rounds + self.batch_rounds(ammo, reserve)
    }

    /// Rounds the next batch moves from `reserve` into the magazine.
    fn batch_rounds(&self, ammo: &SectionAmmo, reserve: u32) -> u32 {
        (ammo.capacity - ammo.rounds).min(self.amount).min(reserve)
    }

    /// Advance by `dt` seconds and move each complete batch of
    /// [`item`](Self::item) from `inventory` into the magazine. Rounds and items
    /// change together, so the pair is conserved. With no matching item the
    /// delay does not advance. A successful shot earlier in this tick wins the
    /// boundary and starts a fresh delay.
    pub fn advance(&mut self, ammo: &mut SectionAmmo, inventory: &mut ShipInventory, dt: f32) {
        if !self.is_reloading(ammo, inventory.count(&self.item)) {
            // Full, or nothing to load: a delay never runs toward a batch
            // that cannot move.
            if ammo.rounds >= ammo.capacity {
                self.elapsed = 0.0;
            }
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
            let rounds = self.batch_rounds(ammo, inventory.count(&self.item));
            inventory.remove(&self.item, rounds);
            ammo.rounds += rounds;
            if !self.is_reloading(ammo, inventory.count(&self.item)) {
                self.elapsed = 0.0;
                break;
            }
        }
    }
}

/// Advance every section's reload cycle and refill its magazine from its
/// parent ship's [`ShipInventory`]. Fire systems run first and call
/// [`SectionReload::on_shot`], so a shot on the completion boundary resets the
/// timer instead of receiving a simultaneous batch.
///
/// A section with no [`SectionReload`] (or no [`SectionAmmo`]) never reloads,
/// preserving the unlimited-ammo default. `Res<Time>` here is the fixed clock.
///
/// Reports [`SectionReloadComplete`] on the batch that brings a magazine back
/// to CAPACITY - see that event for why full, and not each batch, is the thing
/// worth telling anyone about.
///
/// A full magazine moves nothing, so its parent's inventory is not read: an
/// editor preview section starts full under a view entity with no
/// [`ShipInventory`].
///
/// # Panics
///
/// When a section below capacity has a parent with no [`ShipInventory`]:
/// every ship root requires one, so the section is not on a ship.
pub fn tick_section_reload(
    time: Res<Time>,
    mut commands: Commands,
    // A disabled section neither refills nor reports: the gun can never fire
    // again, and the breech clunk would arrive from a piece of wreckage.
    mut q: Query<
        (
            Entity,
            NameOrEntity,
            Option<&ChildOf>,
            &mut SectionAmmo,
            &mut SectionReload,
        ),
        Without<SectionInactiveMarker>,
    >,
    mut q_inventory: Query<&mut ShipInventory>,
) {
    let dt = time.delta_secs();
    for (section, name, parent, mut ammo, mut reload) in &mut q {
        if ammo.rounds >= ammo.capacity {
            reload.elapsed = 0.0;
            reload.interrupted = false;
            continue;
        }
        let mut inventory = parent
            .and_then(|parent| q_inventory.get_mut(parent.parent()).ok())
            .unwrap_or_else(|| {
                panic!("weapon section {name} reloads from a parent with no ShipInventory")
            });
        reload.advance(&mut ammo, &mut inventory, dt);
        if ammo.rounds >= ammo.capacity {
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
    use nova_gameplay::{
        prelude::{ITEM_PDC_ROUND, ITEM_TORPEDO},
        test_support::test_items,
    };

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
        SectionReload::from_config(SectionReloadConfig { delay, amount }, ITEM_PDC_ROUND.into())
    }

    /// A hold of `rounds` PDC rounds with room to spare.
    fn reserve(rounds: u32) -> ShipInventory {
        let items = test_items();
        if rounds == 0 {
            return ShipInventory::new(&items, 1_000_000, []);
        }
        ShipInventory::new(&items, 1_000_000, [(ITEM_PDC_ROUND.into(), rounds)])
    }

    #[test]
    fn idle_cycles_restore_batches_and_clamp_at_capacity() {
        let mut ammo = SectionAmmo::new(500);
        ammo.rounds = 0;
        let mut reload = reload_cfg(3.0, 200);
        let mut inventory = reserve(1000);

        reload.advance(&mut ammo, &mut inventory, 2.0);
        assert_eq!(ammo.rounds, 0);
        assert!((reload.progress() - 2.0 / 3.0).abs() < 1e-6);
        reload.advance(&mut ammo, &mut inventory, 1.0);
        assert_eq!(ammo.rounds, 200);
        assert_eq!(inventory.count(&ITEM_PDC_ROUND.into()), 800);
        assert_eq!(reload.progress(), 0.0);
        reload.advance(&mut ammo, &mut inventory, 6.0);
        assert_eq!(ammo.rounds, 500, "the final batch clamps to capacity");
        assert_eq!(
            inventory.count(&ITEM_PDC_ROUND.into()),
            500,
            "the clamped batch takes only the 100 rounds it loads"
        );
        assert_eq!(reload.progress(), 0.0);
    }

    #[test]
    fn a_short_reserve_loads_a_partial_batch_and_an_empty_one_nothing() {
        let mut ammo = SectionAmmo::new(500);
        ammo.rounds = 100;
        let mut reload = reload_cfg(3.0, 200);
        let mut inventory = reserve(50);
        assert_eq!(reload.incoming_rounds(&ammo, 50), 150);

        reload.advance(&mut ammo, &mut inventory, 3.0);
        assert_eq!(ammo.rounds, 150, "the batch is cut to the 50 in reserve");
        assert!(inventory.is_empty(), "the drained stack is gone");

        // No reserve: not reloading, and the delay does not run.
        assert!(!reload.is_reloading(&ammo, 0));
        assert_eq!(reload.incoming_rounds(&ammo, 0), 150);
        reload.advance(&mut ammo, &mut inventory, 30.0);
        assert_eq!(ammo.rounds, 150);
        assert_eq!(reload.progress(), 0.0);

        // Stock that arrives later starts a fresh delay, not an owed batch.
        inventory.add(&test_items(), &ITEM_PDC_ROUND.into(), 10);
        reload.advance(&mut ammo, &mut inventory, 1.0);
        assert_eq!(ammo.rounds, 150);
        reload.advance(&mut ammo, &mut inventory, 2.0);
        assert_eq!(ammo.rounds, 160);
        assert!(inventory.is_empty());
    }

    #[test]
    fn a_reload_draws_only_its_own_item() {
        let mut ammo = SectionAmmo::new(4);
        ammo.rounds = 0;
        let mut reload = SectionReload::from_config(
            SectionReloadConfig {
                delay: 1.0,
                amount: 1,
            },
            ITEM_TORPEDO.into(),
        );
        let mut inventory = reserve(1000);
        reload.advance(&mut ammo, &mut inventory, 5.0);
        assert_eq!(ammo.rounds, 0, "PDC rounds do not load a torpedo bay");
        assert_eq!(inventory.count(&ITEM_PDC_ROUND.into()), 1000);
    }

    #[test]
    fn a_successful_shot_resets_progress_and_wins_its_tick() {
        let mut ammo = SectionAmmo::new(6);
        ammo.rounds = 5;
        let mut reload = reload_cfg(10.0, 1);
        let mut inventory = reserve(10);
        reload.advance(&mut ammo, &mut inventory, 9.5);
        assert!((reload.progress() - 0.95).abs() < 1e-6);

        assert!(ammo.try_consume());
        reload.on_shot();
        reload.advance(&mut ammo, &mut inventory, 1.0);
        assert_eq!(ammo.rounds, 4, "the shot tick receives no reload batch");
        assert_eq!(reload.progress(), 0.0);
        reload.advance(&mut ammo, &mut inventory, 10.0);
        assert_eq!(ammo.rounds, 5);
    }

    #[test]
    fn an_empty_trigger_pull_does_not_reset_reload() {
        let mut ammo = SectionAmmo::new(1);
        ammo.rounds = 0;
        let mut reload = reload_cfg(2.0, 1);
        let mut inventory = reserve(10);
        reload.advance(&mut ammo, &mut inventory, 1.0);
        assert!(!ammo.try_consume());
        reload.advance(&mut ammo, &mut inventory, 1.0);
        assert_eq!(ammo.rounds, 1);
    }

    #[test]
    fn incoming_rounds_describes_only_the_next_batch() {
        let mut ammo = SectionAmmo::new(500);
        ammo.rounds = 100;
        let reload = reload_cfg(3.0, 200);
        assert_eq!(reload.incoming_rounds(&ammo, 1000), 300);
        ammo.rounds = 450;
        assert_eq!(reload.incoming_rounds(&ammo, 1000), 500);
    }

    #[test]
    fn a_full_magazine_stays_at_rest() {
        let mut ammo = SectionAmmo::new(4);
        let mut reload = reload_cfg(0.5, 2);
        let mut inventory = reserve(10);
        assert!(!reload.is_reloading(&ammo, 10));
        reload.advance(&mut ammo, &mut inventory, 100.0);
        assert_eq!(ammo.rounds, 4);
        assert_eq!(inventory.count(&ITEM_PDC_ROUND.into()), 10);
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

    /// A headless app ticking [`tick_section_reload`] once a second.
    fn reload_app() -> App {
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
        app
    }

    #[test]
    fn scheduled_reload_restores_one_batch_after_the_delay() {
        let mut app = reload_app();
        let ship = app.world_mut().spawn(reserve(1000)).id();
        let mut ammo = SectionAmmo::new(500);
        ammo.rounds = 0;
        let section = app
            .world_mut()
            .spawn((ammo, reload_cfg(2.0, 200), ChildOf(ship)))
            .id();

        app.update();
        assert_eq!(app.world().get::<SectionAmmo>(section).unwrap().rounds, 0);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().get::<SectionAmmo>(section).unwrap().rounds, 200);
    }

    /// The ship editor's preview sections start full under a view entity with
    /// no `ShipInventory`; ticking them crashed the editor.
    #[test]
    fn a_full_magazine_under_no_ship_ticks_without_an_inventory() {
        let mut app = reload_app();
        let view = app.world_mut().spawn_empty().id();
        let section = app
            .world_mut()
            .spawn((SectionAmmo::new(500), reload_cfg(2.0, 200), ChildOf(view)))
            .id();

        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().get::<SectionAmmo>(section).unwrap().rounds, 500);
    }

    /// Two mounts on one ship draw on one reserve: every round a magazine
    /// gains is an item the ship loses, and a short reserve stops both.
    #[test]
    fn mounts_sharing_a_reserve_conserve_rounds_plus_items() {
        let mut app = reload_app();
        let ship = app.world_mut().spawn(reserve(300)).id();
        let mounts: Vec<Entity> = (0..2)
            .map(|_| {
                let mut ammo = SectionAmmo::new(500);
                ammo.rounds = 0;
                app.world_mut()
                    .spawn((ammo, reload_cfg(1.0, 200), ChildOf(ship)))
                    .id()
            })
            .collect();
        let total = |app: &App| {
            let loaded: u32 = mounts
                .iter()
                .map(|&mount| app.world().get::<SectionAmmo>(mount).unwrap().rounds)
                .sum();
            let held = app
                .world()
                .get::<ShipInventory>(ship)
                .unwrap()
                .count(&ITEM_PDC_ROUND.into());
            (loaded, held)
        };

        app.update(); // warm-up, dt 0
        assert_eq!(total(&app), (0, 300));
        app.update();
        assert_eq!(total(&app), (300, 0), "200 then the last 100");
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(total(&app), (300, 0), "an empty reserve loads nothing");
        assert!(app.world().get::<ShipInventory>(ship).unwrap().is_empty());
    }

    /// FULL is the boundary that is reported, and it is reported once. A
    /// magazine that refills in several batches must stay quiet through the
    /// ones that do not finish it, and a magazine already full must not report
    /// every tick it spends sitting there.
    #[test]
    fn a_magazine_reports_the_tick_it_comes_back_to_capacity_and_not_before_or_after() {
        #[derive(Resource, Default)]
        struct Filled(usize);

        let mut app = reload_app();
        app.init_resource::<Filled>();
        app.add_observer(|_: On<SectionReloadComplete>, mut filled: ResMut<Filled>| filled.0 += 1);

        // Two batches to fill: the first must be silent.
        let ship = app.world_mut().spawn(reserve(10)).id();
        let mut ammo = SectionAmmo::new(2);
        ammo.rounds = 0;
        let section = app
            .world_mut()
            .spawn((ammo, reload_cfg(1.0, 1), ChildOf(ship)))
            .id();

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
