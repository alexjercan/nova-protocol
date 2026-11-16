//! The low-ammo warning: the player ship's hold reserve of each ammunition
//! item its live weapons reload from, measured against one magazine.
//!
//! The line is per ITEM, not per weapon: every mount that reloads from PDC
//! rounds draws on the same hold stack. It is the largest magazine among the
//! player's live mounts of that item, and never under [`AMMO_RESERVE_FLOOR`],
//! so a one-shell lance warns at its last spare slug, not after it. LOW is a
//! reserve under that line: the next empty magazine will not refill whole.
//! DRY is an empty reserve: what is in the magazines is all that is left.
//!
//! [`track_player_ammo_reserve`] keeps the level of each item on the player
//! root in [`AmmoReserveWarnings`] and triggers [`AmmoReserveCrossed`] on a
//! WORSENING edge only. The first reading of an item (a spawn, a Load, a
//! rebuilt mount, unlimited ammunition switched off) records its level and
//! says nothing: no reserve crossed a line, and a start kit under one
//! magazine is the steady HUD mark's to show, not news. A restock that lifts
//! the reserve to or over the line clears the level, so the next drain warns
//! again; one that lifts it only off zero leaves LOW, so the next empty
//! hold warns DRY again.
//!
//! A weapon with no [`SectionReload`] never draws on the hold (authored
//! unlimited, reload disabled, or the unlimited cheat), so it sets no line.
//! A disabled section can never fire again and sets none either.

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_gameplay::prelude::{
    ItemDesignId, PlayerSpaceshipMarker, SectionInactiveMarker, ShipInventory, SpaceshipRootMarker,
};

use super::ammo::{SectionAmmo, SectionReload};

/// The warning rule, its record on the player root and its event.
pub mod prelude {
    pub use super::{
        ammo_reserve_warning, AmmoReserveCrossed, AmmoReserveWarningType, AmmoReserveWarnings,
        AMMO_RESERVE_FLOOR,
    };
}

/// The fewest rounds the LOW line ever sits at, so a one-round magazine still
/// warns while one spare is left.
pub const AMMO_RESERVE_FLOOR: u32 = 2;

/// How short a hold reserve is. Ordered by severity: `Low < Dry`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum AmmoReserveWarningType {
    /// Under one full magazine, but not empty.
    Low,
    /// Empty: no reload can land.
    Dry,
}

/// The warning for `reserve` rounds in the hold against a `magazine` of the
/// item: `Dry` at zero, `Low` under `max(magazine, AMMO_RESERVE_FLOOR)`, and
/// `None` at or over that line.
pub fn ammo_reserve_warning(reserve: u32, magazine: u32) -> Option<AmmoReserveWarningType> {
    if reserve == 0 {
        Some(AmmoReserveWarningType::Dry)
    } else if reserve < magazine.max(AMMO_RESERVE_FLOOR) {
        Some(AmmoReserveWarningType::Low)
    } else {
        None
    }
}

/// The last warning level of each ammunition item the player's live weapons
/// reload from, on the player ship root. An item no live weapon uses has no
/// entry. Written by [`track_player_ammo_reserve`]; the HUD gauges read it.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct AmmoReserveWarnings(BTreeMap<ItemDesignId, Option<AmmoReserveWarningType>>);

impl AmmoReserveWarnings {
    /// The warning on `item`, `None` when its reserve is at or over the line
    /// or no live weapon uses it.
    pub fn warning(&self, item: &ItemDesignId) -> Option<AmmoReserveWarningType> {
        self.0.get(item).copied().flatten()
    }
}

/// The player's hold reserve of `item` just crossed into `warning`.
///
/// Triggered once per worsening edge by [`track_player_ammo_reserve`]: into
/// LOW, into DRY, or straight to DRY when one change empties a reserve that
/// was over the line.
#[derive(Event, Clone, Debug, PartialEq)]
pub struct AmmoReserveCrossed {
    /// The player ship root.
    pub ship: Entity,
    /// The ammunition item.
    pub item: ItemDesignId,
    /// The level it crossed into.
    pub warning: AmmoReserveWarningType,
    /// Rounds of `item` left in the hold.
    pub reserve: u32,
}

/// Read the player's reserve of every item its live weapons reload from,
/// record the levels in [`AmmoReserveWarnings`] and trigger
/// [`AmmoReserveCrossed`] on each worsening edge.
///
/// In Update [`SpaceshipSectionSystems`](super::SpaceshipSectionSystems): the
/// reserve drains in the FixedUpdate reload pass and fills from market,
/// job and cheat flows in Update, and a level read every frame sees both.
pub fn track_player_ammo_reserve(
    mut commands: Commands,
    mut q_player: Query<
        (Entity, &ShipInventory, Option<&mut AmmoReserveWarnings>),
        (With<PlayerSpaceshipMarker>, With<SpaceshipRootMarker>),
    >,
    q_weapons: Query<(&SectionAmmo, &SectionReload, &ChildOf), Without<SectionInactiveMarker>>,
) {
    for (ship, inventory, recorded) in &mut q_player {
        let mut magazines: BTreeMap<ItemDesignId, u32> = BTreeMap::new();
        for (ammo, reload, &ChildOf(parent)) in &q_weapons {
            if parent != ship {
                continue;
            }
            let line = magazines.entry(reload.item.clone()).or_default();
            *line = (*line).max(ammo.capacity);
        }
        let mut levels = BTreeMap::new();
        for (item, magazine) in magazines {
            let reserve = inventory.count(&item);
            let warning = ammo_reserve_warning(reserve, magazine);
            let previous = recorded
                .as_ref()
                .and_then(|recorded| recorded.0.get(&item).copied());
            if let (Some(previous), Some(crossed)) = (previous, warning) {
                if warning > previous {
                    debug!("ammo_warning: {item} {crossed:?} at {reserve} rounds");
                    commands.trigger(AmmoReserveCrossed {
                        ship,
                        item: item.clone(),
                        warning: crossed,
                        reserve,
                    });
                }
            }
            levels.insert(item, warning);
        }
        match recorded {
            Some(mut recorded) => {
                if recorded.0 != levels {
                    recorded.0 = levels;
                }
            }
            None => {
                commands.entity(ship).insert(AmmoReserveWarnings(levels));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use nova_gameplay::{prelude::ItemRoleType, test_support::test_items};

    use super::*;
    use crate::sections::ammo::{tick_section_reload, SectionReloadConfig};

    #[test]
    fn a_reserve_under_one_magazine_is_low_and_an_empty_one_is_dry() {
        // PDC: 500 rounds a magazine.
        assert_eq!(ammo_reserve_warning(500, 500), None);
        assert_eq!(
            ammo_reserve_warning(499, 500),
            Some(AmmoReserveWarningType::Low)
        );
        assert_eq!(
            ammo_reserve_warning(1, 500),
            Some(AmmoReserveWarningType::Low)
        );
        assert_eq!(
            ammo_reserve_warning(0, 500),
            Some(AmmoReserveWarningType::Dry)
        );
        // Torpedo: 6 a rack.
        assert_eq!(ammo_reserve_warning(6, 6), None);
        assert_eq!(
            ammo_reserve_warning(5, 6),
            Some(AmmoReserveWarningType::Low)
        );
    }

    #[test]
    fn a_one_round_magazine_warns_low_at_its_last_spare() {
        assert_eq!(ammo_reserve_warning(AMMO_RESERVE_FLOOR, 1), None);
        assert_eq!(
            ammo_reserve_warning(1, 1),
            Some(AmmoReserveWarningType::Low)
        );
        assert_eq!(
            ammo_reserve_warning(0, 1),
            Some(AmmoReserveWarningType::Dry)
        );
    }

    /// Every crossing the tracker triggered, in order.
    #[derive(Resource, Default)]
    struct Crossings(Vec<(AmmoReserveWarningType, u32)>);

    /// The reload pass and the tracker on a one-second manual clock, with a
    /// player ship carrying one PDC mount of 500 rounds that reloads 200 per
    /// 1 s idle from a hold of `reserve` rounds.
    fn tracker_app(reserve: u32) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Lift the virtual clamp, or each 1 s step lands as 0.25 s.
        let mut virtual_time = Time::<bevy::time::Virtual>::default();
        virtual_time.set_max_delta(core::time::Duration::from_secs(3600));
        app.insert_resource(virtual_time);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            core::time::Duration::from_secs(1),
        ));
        app.init_resource::<Crossings>();
        app.add_observer(
            |crossed: On<AmmoReserveCrossed>, mut seen: ResMut<Crossings>| {
                seen.0.push((crossed.warning, crossed.reserve));
            },
        );
        app.add_systems(
            Update,
            (tick_section_reload, track_player_ammo_reserve).chain(),
        );
        let items = test_items();
        let ship = app
            .world_mut()
            .spawn((
                PlayerSpaceshipMarker,
                SpaceshipRootMarker,
                ShipInventory::new(
                    &items,
                    1_000_000,
                    [(ItemRoleType::PdcRound.into(), reserve)],
                ),
            ))
            .id();
        let mut magazine = SectionAmmo::new(500);
        magazine.rounds = 0;
        let pdc = app
            .world_mut()
            .spawn((
                magazine,
                SectionReload::from_config(
                    SectionReloadConfig {
                        delay: 1.0,
                        amount: 200,
                    },
                    ItemRoleType::PdcRound.into(),
                ),
                ChildOf(ship),
            ))
            .id();
        // The first frame advances no time.
        app.update();
        (app, ship, pdc)
    }

    fn crossings(app: &App) -> Vec<(AmmoReserveWarningType, u32)> {
        app.world().resource::<Crossings>().0.clone()
    }

    fn reserve(app: &App, ship: Entity) -> u32 {
        app.world()
            .get::<ShipInventory>(ship)
            .expect("the ship carries a hold")
            .count(&ItemRoleType::PdcRound.into())
    }

    /// Empty the mount's magazine, as a burst that runs it dry does.
    fn fire_out(app: &mut App, pdc: Entity) {
        app.world_mut()
            .get_mut::<SectionAmmo>(pdc)
            .expect("the mount has a magazine")
            .rounds = 0;
    }

    #[test]
    fn reloading_down_past_the_lines_fires_one_low_then_one_dry_and_a_restock_rearms() {
        let (mut app, ship, pdc) = tracker_app(600);
        assert!(
            crossings(&app).is_empty(),
            "the first reading records a level and says nothing"
        );

        // 600 -> 400: under one magazine.
        app.update();
        assert_eq!(reserve(&app, ship), 400);
        assert_eq!(crossings(&app), [(AmmoReserveWarningType::Low, 400)]);

        // 400 -> 200 -> 0: still LOW, then DRY once.
        fire_out(&mut app, pdc);
        app.update();
        assert_eq!(reserve(&app, ship), 200);
        fire_out(&mut app, pdc);
        app.update();
        assert_eq!(reserve(&app, ship), 0);
        app.update();
        app.update();
        assert_eq!(
            crossings(&app),
            [
                (AmmoReserveWarningType::Low, 400),
                (AmmoReserveWarningType::Dry, 0)
            ],
            "one LOW, then one DRY, and a held DRY says nothing more"
        );
        assert_eq!(
            app.world()
                .get::<AmmoReserveWarnings>(ship)
                .expect("the player root carries the levels")
                .warning(&ItemRoleType::PdcRound.into()),
            Some(AmmoReserveWarningType::Dry)
        );

        // A restock over the line re-arms both edges. The magazine is full
        // first, so the restock frame loads nothing.
        app.world_mut()
            .get_mut::<SectionAmmo>(pdc)
            .expect("the mount has a magazine")
            .rounds = 500;
        app.world_mut()
            .get_mut::<ShipInventory>(ship)
            .expect("the ship carries a hold")
            .add(&test_items(), &ItemRoleType::PdcRound.into(), 600);
        app.update();
        assert_eq!(crossings(&app).len(), 2, "a restock is not a warning");
        assert_eq!(
            app.world()
                .get::<AmmoReserveWarnings>(ship)
                .expect("the player root carries the levels")
                .warning(&ItemRoleType::PdcRound.into()),
            None
        );
        fire_out(&mut app, pdc);
        app.update();
        assert_eq!(reserve(&app, ship), 400);
        assert_eq!(
            crossings(&app).last(),
            Some(&(AmmoReserveWarningType::Low, 400)),
            "the next drain warns LOW again"
        );
    }

    #[test]
    fn another_ships_reserve_never_warns() {
        let (mut app, ship, _) = tracker_app(600);
        app.world_mut()
            .entity_mut(ship)
            .remove::<PlayerSpaceshipMarker>();
        app.update();
        assert!(crossings(&app).is_empty());
    }
}
