//! Trade profiles: what a ship trades and which jobs it posts when it
//! visits a station.
//!
//! A [`TradeProfileDesign`] is authored content. Its trade list and its job
//! list are independent: either or both may be empty, and an empty profile
//! is a valid ship that only visits. The open world draws a profile for a
//! generated ship by its role ([`TradeProfileDesign::generated`]) and keeps
//! it on the ship's [`TraderMarket`](crate::market::TraderMarket); a
//! scenario script may set or clear it.
//!
//! A trade intention never creates cargo or credits: the visit plans each
//! one against the ship's hold and balance and the station's stock, balance
//! and price ([`plan_visit_trade`]).

use std::{collections::BTreeMap, fmt};

use bevy::prelude::*;

use crate::{
    inventory::prelude::{
        plan_item_trade, GameItems, ItemDesignId, ItemTrade, ItemTradeType, ShipInventory,
        TradePriceType,
    },
    market::prelude::{price, PriceTable, ShipRoleType, TraderMarket},
    station_visits::prelude::VisitTradeLine,
};

/// Glob-import surface: `use nova_gameplay::trade_profile::prelude::*`.
pub mod prelude {
    pub use super::{
        apply_visit_trades, plan_visit_trade, trade_profile_faults, GameTradeProfiles, TradeIntent,
        TradeProfileDesign, TradeProfileDesignId, TradeProfileRoleWeight, VisitJobType,
        TRADE_INTENT_QUANTITY_MAX,
    };
}

/// The most units one trade intention may name. A visit plans an intention
/// by counting down from its quantity, so the bound keeps that search short.
pub const TRADE_INTENT_QUANTITY_MAX: u32 = 1_000;

/// The authored id of a trade profile.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct TradeProfileDesignId(pub String);

impl TradeProfileDesignId {
    /// The id as authored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TradeProfileDesignId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for TradeProfileDesignId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

/// How often the open world draws a profile for a generated ship of one
/// role, against every other profile that names the role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct TradeProfileRoleWeight {
    /// The role.
    pub role: ShipRoleType,
    /// The draw weight, above zero.
    pub weight: u32,
}

/// One thing a visiting ship wants to trade with the station.
#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct TradeIntent {
    /// The item.
    pub item: ItemDesignId,
    /// The direction, seen from the visiting ship: a Buy moves items from
    /// the station into the ship, a Sell from the ship into the station.
    pub side: ItemTradeType,
    /// The most units the ship trades on one visit, above zero and at most
    /// [`TRADE_INTENT_QUANTITY_MAX`].
    pub quantity: u32,
    /// The price limit per unit, above zero: a Buy pays at most this times
    /// the count, a Sell earns at least this times the count.
    pub limit_cr: u32,
}

/// One job a visiting ship posts on the station's board when it docks.
#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VisitJobType {
    /// Bring `count` of `item` to the station, for `reward_cr`.
    Procure {
        /// The item wanted.
        item: ItemDesignId,
        /// How many units, above zero.
        count: u32,
        /// What the station pays on the hand-in, above zero.
        reward_cr: u32,
    },
}

/// One authored trade profile.
#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct TradeProfileDesign {
    /// Its id.
    pub id: TradeProfileDesignId,
    /// The roles of generated ships the open world draws it for, with their
    /// weights. Empty: only a script gives a ship this profile.
    pub generated: Vec<TradeProfileRoleWeight>,
    /// What the ship trades on each visit, in order.
    pub trades: Vec<TradeIntent>,
    /// What the ship posts on the station's board on each visit, in order.
    pub jobs: Vec<VisitJobType>,
}

/// Every fault of `profile` against `items`: an unknown item, a zero or
/// oversized quantity, a zero limit, count, reward or weight, a repeated
/// item and side, and a repeated role. The merge and the lint share it.
pub fn trade_profile_faults(profile: &TradeProfileDesign, items: &GameItems) -> Vec<String> {
    let id = &profile.id;
    let mut faults = Vec::new();
    if id.0.is_empty() {
        faults.push("a trade profile has an empty id".to_string());
    }
    let mut roles = Vec::new();
    for weight in &profile.generated {
        if weight.weight == 0 {
            faults.push(format!(
                "the trade profile '{id}' draws the {} role at weight 0",
                weight.role.label()
            ));
        }
        if roles.contains(&weight.role) {
            faults.push(format!(
                "the trade profile '{id}' names the {} role twice",
                weight.role.label()
            ));
        }
        roles.push(weight.role);
    }
    let mut sides = Vec::new();
    for trade in &profile.trades {
        if items.get(&trade.item).is_none() {
            faults.push(format!(
                "the trade profile '{id}' trades '{}', which is no item",
                trade.item
            ));
        }
        if trade.quantity == 0 || trade.quantity > TRADE_INTENT_QUANTITY_MAX {
            faults.push(format!(
                "the trade profile '{id}' trades {} of '{}', not 1..={TRADE_INTENT_QUANTITY_MAX}",
                trade.quantity, trade.item
            ));
        }
        if trade.limit_cr == 0 {
            faults.push(format!(
                "the trade profile '{id}' limits '{}' to 0 cr",
                trade.item
            ));
        }
        let key = (trade.item.clone(), trade.side);
        if sides.contains(&key) {
            faults.push(format!(
                "the trade profile '{id}' names '{}' on one side twice",
                trade.item
            ));
        }
        sides.push(key);
    }
    for job in &profile.jobs {
        let VisitJobType::Procure {
            item,
            count,
            reward_cr,
        } = job;
        if items.get(item).is_none() {
            faults.push(format!(
                "the trade profile '{id}' posts a job for '{item}', which is no item"
            ));
        }
        if *count == 0 || *reward_cr == 0 {
            faults.push(format!(
                "the trade profile '{id}' posts a job for {count} '{item}' paying {reward_cr} cr; \
                 both must be above zero"
            ));
        }
    }
    faults
}

/// The merged trade profiles, by id.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct GameTradeProfiles(pub BTreeMap<TradeProfileDesignId, TradeProfileDesign>);

impl GameTradeProfiles {
    /// The profile `id`, if the catalog holds it.
    pub fn get(&self, id: &TradeProfileDesignId) -> Option<&TradeProfileDesign> {
        self.0.get(id)
    }

    /// The profile a generated ship of `role` takes for a `draw` in
    /// `[0, 1)`: by weight among the profiles that name the role, in id
    /// order. `None` when no profile names the role: the ship never visits.
    pub fn draw(&self, role: ShipRoleType, draw: f32) -> Option<&TradeProfileDesign> {
        let named: Vec<(&TradeProfileDesign, u32)> = self
            .0
            .values()
            .filter_map(|profile| {
                profile
                    .generated
                    .iter()
                    .find(|weight| weight.role == role)
                    .map(|weight| (profile, weight.weight))
            })
            .collect();
        let total: u64 = named.iter().map(|(_, weight)| u64::from(*weight)).sum();
        if total == 0 {
            return None;
        }
        let mut pick = ((f64::from(draw.clamp(0.0, 1.0)) * total as f64) as u64).min(total - 1);
        for (profile, weight) in named {
            if pick < u64::from(weight) {
                return Some(profile);
            }
            pick -= u64::from(weight);
        }
        unreachable!("the pick is below the weight total")
    }
}

/// Plan one trade intention of a visiting ship against the station it is
/// docked at: the largest count up to the intention's quantity that
/// [`plan_item_trade`] accepts from the ship's side (`own`, `own_cr`) and
/// the station's (`station`, `station_cr`) at `price`, the station's quote
/// for the item, and whose lot price keeps the limit: a Buy pays at most
/// `limit_cr` per unit, a Sell earns at least that. `None` when no count
/// does: the station lacks the stock or the room, a side lacks the
/// credits, or the price breaks the limit.
pub fn plan_visit_trade(
    items: &GameItems,
    intent: &TradeIntent,
    price: TradePriceType,
    own: &ShipInventory,
    own_cr: u32,
    station: &ShipInventory,
    station_cr: u32,
) -> Option<ItemTrade> {
    (1..=intent.quantity.min(TRADE_INTENT_QUANTITY_MAX))
        .rev()
        .find_map(|count| {
            let trade = plan_item_trade(
                items,
                intent.side,
                Some(price),
                &intent.item,
                Some(count),
                own,
                own_cr,
                station,
                station_cr,
            )
            .ok()?;
            let limit = u64::from(intent.limit_cr) * u64::from(count);
            let keeps = match intent.side {
                ItemTradeType::Buy => u64::from(trade.price_cr) <= limit,
                ItemTradeType::Sell => u64::from(trade.price_cr) >= limit,
            };
            keeps.then_some(trade)
        })
}

/// Run `trades`, a visitor's trade intentions, in order against the station
/// it is docked at, and give back what each one moved.
///
/// Each intention is planned afresh ([`plan_visit_trade`]) against the
/// holds and balances the earlier ones left and the station's quote by
/// `table` and `market` at its current stock of the item, then applied
/// whole: the items leave one hold and enter the other, the price leaves
/// one balance and enters the other. Nothing is created or destroyed. An
/// intention no count can trade moves nothing and is listed with a count of
/// zero.
#[expect(
    clippy::too_many_arguments,
    reason = "both parties' holds and balances, changed as one atomic run"
)]
pub fn apply_visit_trades(
    items: &GameItems,
    table: &PriceTable,
    market: &TraderMarket,
    trades: &[TradeIntent],
    ship_hold: &mut ShipInventory,
    ship_cr: &mut u32,
    station_hold: &mut ShipInventory,
    station_cr: &mut u32,
) -> Vec<VisitTradeLine> {
    trades
        .iter()
        .map(|intent| {
            let design = items.design(&intent.item);
            let quote = price(table, design, market, station_hold.count(&intent.item));
            let planned = plan_visit_trade(
                items,
                intent,
                TradePriceType::Market(quote.price),
                ship_hold,
                *ship_cr,
                station_hold,
                *station_cr,
            );
            let (count, price_cr) = planned.map_or((0, 0), |trade| (trade.count, trade.price_cr));
            if count > 0 {
                let (seller, buyer, seller_cr, buyer_cr) = match intent.side {
                    ItemTradeType::Buy => (
                        &mut *station_hold,
                        &mut *ship_hold,
                        &mut *station_cr,
                        &mut *ship_cr,
                    ),
                    ItemTradeType::Sell => (
                        &mut *ship_hold,
                        &mut *station_hold,
                        &mut *ship_cr,
                        &mut *station_cr,
                    ),
                };
                seller.remove(&intent.item, count);
                buyer.add(items, &intent.item, count);
                *buyer_cr -= price_cr;
                *seller_cr += price_cr;
            }
            VisitTradeLine {
                item: intent.item.clone(),
                side: intent.side,
                count,
                price_cr,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        chart::prelude::ClusterType,
        civilization::prelude::CivilizationId,
        inventory::prelude::ItemRoleType,
        market::prelude::{JobOfferRule, PriceClamp, PriceFactor, RestockRule, ScarcityRule},
    };

    fn table() -> PriceTable {
        let one = PriceFactor::ONE;
        PriceTable {
            clamp: PriceClamp {
                min: 0.25,
                max: 3.0,
            },
            scarcity: ScarcityRule {
                starved: one,
                glut: one,
                glut_share: 3.0,
                unkept: one,
            },
            restock: RestockRule {
                recovery_seconds: 1_000.0,
            },
            jobs: JobOfferRule {
                offer_seconds: 1_200.0,
                escort_offer_seconds: 3_600.0,
            },
            clusters: Vec::new(),
            roles: Vec::new(),
        }
    }

    fn market() -> TraderMarket {
        TraderMarket {
            cluster: ClusterType::RockOnly,
            civilization: CivilizationId {
                world_seed: 1,
                node: [0, 0, 0],
            },
            role_preference: [1.0, 0.0, 0.0, 0.0],
            target: BTreeMap::new(),
            profile: None,
        }
    }

    /// Several items move both ways at the station's price, each held to its
    /// quantity, its limit and what both sides hold, and the run creates no
    /// item and no credit.
    #[test]
    fn a_visit_trades_several_items_within_its_limits_and_conserves_goods_and_credits() {
        let items = crate::test_support::test_items();
        let ore = ItemDesignId::from(ItemRoleType::IronOre);
        let rations = ItemDesignId::from(ItemRoleType::Rations);
        let ore_bid = items.design(&ore).bid_cr;
        let rations_ask = items.design(&rations).ask_cr;
        let mut ship = ShipInventory::new(&items, 1_000_000, [(ore.clone(), 10)]);
        let mut station = ShipInventory::new(&items, 1_000_000, [(rations.clone(), 3)]);
        let (mut ship_cr, mut station_cr) = (10_000_u32, 10_000_u32);
        let trades = [
            TradeIntent {
                item: ore.clone(),
                side: ItemTradeType::Sell,
                quantity: 6,
                limit_cr: 1,
            },
            TradeIntent {
                item: rations.clone(),
                side: ItemTradeType::Buy,
                quantity: 5,
                limit_cr: rations_ask * 2,
            },
            TradeIntent {
                item: ore.clone(),
                side: ItemTradeType::Buy,
                quantity: 2,
                limit_cr: 0,
            },
        ];
        let units = |ship: &ShipInventory, station: &ShipInventory, item: &ItemDesignId| {
            ship.count(item) + station.count(item)
        };
        let before = (
            units(&ship, &station, &ore),
            units(&ship, &station, &rations),
            ship_cr + station_cr,
        );

        let lines = apply_visit_trades(
            &items,
            &table(),
            &market(),
            &trades,
            &mut ship,
            &mut ship_cr,
            &mut station,
            &mut station_cr,
        );

        assert_eq!(lines[0].count, 6, "the ship sells its whole intention");
        assert!(
            lines[0].price_cr >= ore_bid * 6 - ore_bid,
            "sold near the bid"
        );
        assert_eq!(
            lines[1].count, 3,
            "the station sells only the rations it holds"
        );
        assert_eq!(lines[2].count, 0, "a limit no price keeps trades nothing");
        assert_eq!(lines[2].price_cr, 0);
        assert_eq!((ship.count(&ore), station.count(&ore)), (4, 6));
        assert_eq!((ship.count(&rations), station.count(&rations)), (3, 0));
        assert_eq!(
            (
                units(&ship, &station, &ore),
                units(&ship, &station, &rations),
                ship_cr + station_cr,
            ),
            before,
            "no item or credit is created or destroyed"
        );
        assert_eq!(ship_cr, 10_000 + lines[0].price_cr - lines[1].price_cr);
    }
}
