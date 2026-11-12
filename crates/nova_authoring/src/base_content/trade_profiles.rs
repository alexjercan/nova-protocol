//! Built-in TRADE PROFILE content: what a visiting ship trades and which jobs
//! it posts.
//!
//! `content gen` serializes this builder into
//! `assets/base/trade_profiles/base.content.ron`. Every limit derives from the
//! item catalog's reference prices, so a price change moves the limits with it.

use nova_gameplay::prelude::{
    GameItems, ItemDesignId, ItemRoleType, ItemTradeType, ShipRoleType, TradeIntent,
    TradeProfileDesign, TradeProfileDesignId, TradeProfileRoleWeight, VisitJobType,
};

use super::items::{item_catalog, UNMARKED_CORES};

/// A Sell earns at least this share of the reference bid, per unit: half.
const SELL_LIMIT_DIVISOR: u32 = 2;
/// A Buy pays at most this multiple of the reference ask, per unit, as a
/// numerator over [`BUY_LIMIT_DENOMINATOR`]: one and a half times.
const BUY_LIMIT_NUMERATOR: u32 = 3;
/// See [`BUY_LIMIT_NUMERATOR`].
const BUY_LIMIT_DENOMINATOR: u32 = 2;

/// The base game's trade profiles, in the order the file carries them.
pub(crate) fn trade_profile_catalog() -> Vec<TradeProfileDesign> {
    let items = GameItems::new(item_catalog());
    // A reference price covers a lot of items, so the per-unit limit divides
    // by the lot and never drops below one credit.
    let sell = |id: ItemDesignId, quantity: u32| {
        let item = items
            .get(&id)
            .unwrap_or_else(|| panic!("trade profile names '{id}', which is no base item"));
        let limit_cr = (item.bid_cr / (SELL_LIMIT_DIVISOR * item.price_lot)).max(1);
        TradeIntent {
            item: id,
            side: ItemTradeType::Sell,
            quantity,
            limit_cr,
        }
    };
    let buy = |role: ItemRoleType, quantity: u32| {
        let id = ItemDesignId::from(role);
        let item = items
            .get(&id)
            .unwrap_or_else(|| panic!("trade profile names '{id}', which is no base item"));
        let limit_cr = (item.ask_cr * BUY_LIMIT_NUMERATOR)
            .div_ceil(BUY_LIMIT_DENOMINATOR * item.price_lot)
            .max(1);
        TradeIntent {
            item: id,
            side: ItemTradeType::Buy,
            quantity,
            limit_cr,
        }
    };
    let procure = |role: ItemRoleType, count: u32, reward_cr: u32| VisitJobType::Procure {
        item: ItemDesignId::from(role),
        count,
        reward_cr,
    };
    let generated = |weights: &[(ShipRoleType, u32)]| {
        weights
            .iter()
            .map(|&(role, weight)| TradeProfileRoleWeight { role, weight })
            .collect::<Vec<_>>()
    };
    vec![
        TradeProfileDesign {
            id: TradeProfileDesignId::from("base/ore_hauler"),
            generated: generated(&[(ShipRoleType::Industrial, 3)]),
            trades: vec![
                sell(ItemRoleType::IronOre.into(), 12),
                sell(ItemRoleType::StoneOre.into(), 12),
                sell(ItemRoleType::CarbonOre.into(), 8),
                buy(ItemRoleType::Rations, 4),
            ],
            jobs: vec![
                procure(ItemRoleType::WaterIce, 6, 180),
                procure(ItemRoleType::SalvagedParts, 3, 150),
            ],
        },
        TradeProfileDesign {
            id: TradeProfileDesignId::from("base/provisioner"),
            generated: generated(&[(ShipRoleType::Civilian, 3)]),
            trades: vec![
                sell(ItemRoleType::Rations.into(), 10),
                sell(ItemRoleType::Medkit.into(), 4),
                buy(ItemRoleType::SalvagedParts, 4),
                buy(ItemRoleType::WaterIce, 6),
            ],
            jobs: vec![procure(ItemRoleType::HullPlate, 3, 220)],
        },
        TradeProfileDesign {
            id: TradeProfileDesignId::from("base/salvage_broker"),
            generated: generated(&[(ShipRoleType::Scavenger, 2)]),
            trades: vec![
                sell(ItemRoleType::SalvagedParts.into(), 6),
                sell(UNMARKED_CORES.into(), 2),
                buy(ItemRoleType::HullPlate, 2),
                buy(ItemRoleType::PdcRound, 20),
            ],
            jobs: Vec::new(),
        },
        TradeProfileDesign {
            id: TradeProfileDesignId::from("base/garrison_supply"),
            generated: generated(&[(ShipRoleType::Armored, 1)]),
            trades: Vec::new(),
            jobs: vec![
                procure(ItemRoleType::PdcRound, 40, 200),
                procure(ItemRoleType::Rations, 6, 160),
                procure(ItemRoleType::Medkit, 2, 220),
            ],
        },
        // Valid traffic that only visits: no trade, no job.
        TradeProfileDesign {
            id: TradeProfileDesignId::from("base/idle_traffic"),
            generated: generated(&[(ShipRoleType::Civilian, 1), (ShipRoleType::Industrial, 1)]),
            trades: Vec::new(),
            jobs: Vec::new(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::trade_profile_faults;

    use super::*;

    #[test]
    fn base_trade_profiles_have_no_faults() {
        let items = GameItems::new(item_catalog());
        for profile in trade_profile_catalog() {
            assert_eq!(
                trade_profile_faults(&profile, &items),
                Vec::<String>::new(),
                "{}",
                profile.id
            );
        }
    }
}
