//! Restock on a thaw: the economy's consumer of the play-time clock.
//!
//! Nothing frozen simulates, so a trader's stock stands still while its
//! cell is off the window and catches up the moment the cell thaws:
//! [`restock_thawed_traders`] reads the [`ThawedAfter`] stamp the thaw put
//! on the body and moves every targeted item of its [`TraderMarket`] toward
//! the target by the table's [`RestockRule`]
//! (`nova_gameplay::market::restocked_count`). A trader thaws through the
//! section spawn, which puts its hold on the root once, so the restock keys
//! on the hold landing. A neutralized or derelict hull keeps its
//! [`TraderMarket`] for the stance graph (`stance`) but trades no more, so
//! what the player looted stays looted: the restock skips it.
//!
//! A station's credits catch up the same way: [`refill_thawed_station_credits`]
//! moves a living station's balance up toward its start balance
//! ([`station_start_credits`]), so a station the player's hand-ins paid down
//! can pay again. A station the player owns is skipped: it trades with its
//! owner at one price both ways, so credits it regained would be credits the
//! player draws out for nothing.

use bevy::prelude::*;
use nova_gameplay::prelude::{
    restocked_count, restocked_credits, DerelictShipMarker, GameItems, GamePriceTable,
    NeutralizedMarker, OwnedStationMarker, ShipCredits, ShipInventory, StationAdvancement,
    StationMarker, TraderMarket,
};
use nova_world::prelude::ThawedAfter;

use crate::station_layout::station_start_credits;

/// Move every targeted item of a trader whose hold just landed after a thaw
/// toward its target by the play time it stood frozen. An item adds only
/// what the hold has room for: a hold the player filled with sold goods
/// restocks what fits. A neutralized or derelict hull is skipped: nobody
/// aboard trades, and a looted hold that refilled on every thaw would be
/// unlimited loot.
pub(crate) fn restock_thawed_traders(
    items: Res<GameItems>,
    table: Res<GamePriceTable>,
    mut traders: Query<
        (
            Entity,
            Option<&Name>,
            &ThawedAfter,
            &TraderMarket,
            &mut ShipInventory,
        ),
        (
            Added<ShipInventory>,
            Without<NeutralizedMarker>,
            Without<DerelictShipMarker>,
        ),
    >,
) {
    for (entity, name, thawed, market, mut hold) in &mut traders {
        let elapsed = thawed.play_seconds;
        let label = name.map_or_else(|| format!("{entity:?}"), |name| name.to_string());
        for (item, target) in &market.target {
            let held = hold.count(item);
            let wanted = restocked_count(held, *target, elapsed, table.0.restock);
            if wanted > held {
                let fits = items.design(item).mass_g.max(1);
                let room = hold.free_g(&items) / fits;
                let added = (wanted - held).min(room);
                if added > 0 {
                    hold.add(&items, item, added);
                }
                info!(
                    "restock: '{label}' stood frozen {elapsed:.1} s, {item} {held} -> {} of {target}",
                    held + added
                );
            } else if wanted < held {
                hold.remove(item, held - wanted);
                info!(
                    "restock: '{label}' stood frozen {elapsed:.1} s, {item} {held} -> {wanted} of {target}"
                );
            }
        }
    }
}

/// Move the balance of a living station that just thawed up toward its
/// start balance by the play time it stood frozen, by the table's
/// [`RestockRule`](nova_gameplay::prelude::RestockRule). The root's balance
/// lands once, with the thaw, so the refill keys on it. A balance at or over
/// the start is left alone. A neutralized or derelict station pays nobody,
/// and a station the player owns hands its balance to its owner at no
/// spread: neither regains a credit.
pub(crate) fn refill_thawed_station_credits(
    table: Res<GamePriceTable>,
    mut stations: Query<
        (
            Entity,
            Option<&Name>,
            &ThawedAfter,
            &StationAdvancement,
            &mut ShipCredits,
        ),
        (
            Added<ShipCredits>,
            With<StationMarker>,
            Without<NeutralizedMarker>,
            Without<DerelictShipMarker>,
            Without<OwnedStationMarker>,
        ),
    >,
) {
    for (entity, name, thawed, advancement, mut credits) in &mut stations {
        let start = station_start_credits(advancement.0);
        let refilled = restocked_credits(credits.0, start, thawed.play_seconds, table.0.restock);
        if refilled != credits.0 {
            let label = name.map_or_else(|| format!("{entity:?}"), |name| name.to_string());
            info!(
                "restock: '{label}' stood frozen {:.1} s, {} -> {refilled} cr of {start}",
                thawed.play_seconds, credits.0
            );
            credits.0 = refilled;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bevy::ecs::system::RunSystemOnce;
    use nova_gameplay::prelude::{
        CivilizationId, ClusterType, DerelictShipMarker, ItemDesignId, ItemRoleType, JobOfferRule,
        NeutralizedMarker, PriceClamp, PriceFactor, PriceTable, RestockRule, ScarcityRule,
    };

    use super::*;

    const RECOVERY_SECONDS: f32 = 1_000.0;

    /// A world with the test catalog and a table whose stock recovers in
    /// [`RECOVERY_SECONDS`].
    fn world() -> World {
        let factor = PriceFactor { ask: 1.0, bid: 1.0 };
        let mut world = World::new();
        world.insert_resource(nova_gameplay::test_support::test_items());
        world.insert_resource(GamePriceTable(PriceTable {
            clamp: PriceClamp {
                min: 0.25,
                max: 3.0,
            },
            scarcity: ScarcityRule {
                starved: factor,
                glut: factor,
                glut_share: 3.0,
                unkept: factor,
            },
            restock: RestockRule {
                recovery_seconds: RECOVERY_SECONDS,
            },
            jobs: JobOfferRule {
                offer_seconds: 1_200.0,
                escort_offer_seconds: 3_600.0,
            },
            clusters: Vec::new(),
            roles: Vec::new(),
        }));
        world
    }

    /// A trader whose hold the player emptied, thawed after a full recovery.
    fn looted_trader(world: &mut World, ore: &ItemDesignId) -> Entity {
        let items = world.resource::<GameItems>().clone();
        world
            .spawn((
                ThawedAfter {
                    play_seconds: f64::from(RECOVERY_SECONDS) * 2.0,
                },
                TraderMarket {
                    cluster: ClusterType::RockOnly,
                    civilization: CivilizationId {
                        world_seed: 1,
                        node: [0, 0, 0],
                    },
                    role_preference: [1.0, 1.0, 0.0, 0.0],
                    target: BTreeMap::from([(ore.clone(), 10)]),
                    profile: None,
                },
                ShipInventory::new(&items, 1_000_000, []),
            ))
            .id()
    }

    #[test]
    fn a_neutralized_or_derelict_trader_keeps_its_looted_hold_across_a_thaw() {
        let ore = ItemDesignId::from(ItemRoleType::IronOre);
        let mut world = world();
        let intact = looted_trader(&mut world, &ore);
        let neutralized = looted_trader(&mut world, &ore);
        world.entity_mut(neutralized).insert(NeutralizedMarker);
        let derelict = looted_trader(&mut world, &ore);
        world.entity_mut(derelict).insert(DerelictShipMarker);

        world.run_system_once(restock_thawed_traders).unwrap();

        let count =
            |world: &World, trader: Entity| world.get::<ShipInventory>(trader).unwrap().count(&ore);
        assert_eq!(
            count(&world, intact),
            10,
            "a living trader recovers to its target"
        );
        assert_eq!(
            count(&world, neutralized),
            0,
            "a neutralized hull stays looted"
        );
        assert_eq!(count(&world, derelict), 0, "a derelict hull stays looted");
    }

    /// A station at advancement 0 with `balance` credits, thawed after
    /// `share` of a recovery.
    fn paid_down_station(world: &mut World, balance: u32, share: f64) -> Entity {
        world
            .spawn((
                StationMarker,
                StationAdvancement(0.0),
                ShipCredits(balance),
                ThawedAfter {
                    play_seconds: f64::from(RECOVERY_SECONDS) * share,
                },
            ))
            .id()
    }

    #[test]
    fn a_living_station_regains_credits_up_to_its_start_balance_across_a_thaw() {
        let start = station_start_credits(0.0);
        let mut world = world();
        let half = paid_down_station(&mut world, 0, 0.5);
        let whole = paid_down_station(&mut world, 1_000, 3.0);
        let rich = paid_down_station(&mut world, start + 5_000, 3.0);
        let neutralized = paid_down_station(&mut world, 0, 3.0);
        world.entity_mut(neutralized).insert(NeutralizedMarker);
        let derelict = paid_down_station(&mut world, 0, 3.0);
        world.entity_mut(derelict).insert(DerelictShipMarker);
        let owned = paid_down_station(&mut world, 0, 3.0);
        world.entity_mut(owned).insert(OwnedStationMarker);

        world
            .run_system_once(refill_thawed_station_credits)
            .unwrap();

        let balance = |station: Entity| world.get::<ShipCredits>(station).unwrap().0;
        assert_eq!(balance(half), start / 2, "half a recovery, half the start");
        assert_eq!(balance(whole), start, "never over the start balance");
        assert_eq!(balance(rich), start + 5_000, "a richer station keeps it");
        assert_eq!(balance(neutralized), 0, "a neutralized station pays nobody");
        assert_eq!(balance(derelict), 0, "a derelict station pays nobody");
        assert_eq!(balance(owned), 0, "an owned station mints nothing");
    }
}
