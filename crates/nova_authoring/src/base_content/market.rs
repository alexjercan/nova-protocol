//! Built-in PRICE TABLE content: the factors a generated trader prices by.
//!
//! `content gen` serializes this builder into
//! `assets/base/market/base.content.ron`. A generated trader asks and bids
//! `reference x scarcity x cluster x civilization`, clamped
//! (`nova_gameplay::market::price`). A factor of 1 leaves the reference price.

use nova_gameplay::prelude::{
    ClusterPriceRule, ClusterType, ItemCategoryType, JobOfferRule, PriceClamp, PriceFactor,
    PriceTable, RestockRule, RobberyRule, RolePriceRule, ScarcityRule, ShipRoleType,
};

/// The lowest total factor: no trader sells below a quarter of reference.
const CLAMP_MIN: f32 = 0.25;
/// The highest total factor: no trader asks above three times reference.
const CLAMP_MAX: f32 = 3.0;

/// The scarcity factor at an empty hold: the trader asks more and pays more
/// for what it has run out of.
const STARVED: PriceFactor = PriceFactor {
    ask: 1.5,
    bid: 1.25,
};
/// The scarcity factor at [`GLUT_SHARE`] times the target: a full hold sells
/// cheap and buys cheaper.
const GLUT: PriceFactor = PriceFactor { ask: 0.9, bid: 0.6 };
/// The stock share of the target at which the glut factor applies in full.
const GLUT_SHARE: f32 = 3.0;
/// The scarcity factor of an item the trader does not keep: it pays a little
/// under reference for something it only resells.
const UNKEPT: PriceFactor = PriceFactor { ask: 1.0, bid: 0.9 };

/// The play seconds a trader's stock takes to return to its target from
/// none: thirty minutes. A cell crossing at cruise takes a few minutes, so
/// a station the player sold out is noticeably back after a round trip to
/// the next cluster, and whole again within one session; a reload alone
/// restocks nothing, since the clock stands still while paused.
const RESTOCK_RECOVERY_SECONDS: f32 = 1_800.0;
/// The play seconds a civilization's stations refuse the player after the
/// player neutralizes one of its hulls: fifteen minutes. Longer than a round
/// trip to the next cluster, so waiting at the door does not pay, and
/// shorter than a session, so the sanction ends while the player still knows
/// what earned it (Elite's hour-per-level notoriety is the pitfall).
const ROBBERY_REFUSAL_SECONDS: f32 = 900.0;
/// The play seconds an unheld job offer stands before its slot posts a new
/// one: twenty minutes. Longer than one round trip to the next cluster, so
/// an offer the player left to prepare for is still there once; shorter than
/// the restock recovery, so a board turns over before a station's stock
/// does. A held job never expires.
const JOB_OFFER_SECONDS: f32 = 1_200.0;

/// Raw material in an asteroid-rich cluster: ore is everywhere.
const ASTEROID_RICH_RAW: f32 = 0.6;
/// Raw material in a rock-only cluster: stone is common, metal less so.
const ROCK_ONLY_RAW: f32 = 0.8;
/// Raw material in a planet-heavy cluster: few rocks to mine.
const PLANET_HEAVY_RAW: f32 = 1.5;
/// Parts in a derelict field: wrecks give them up.
const DERELICT_PARTS: f32 = 0.7;
/// Repair stock in a derelict field: wrecks give plate up too.
const DERELICT_REPAIR: f32 = 0.85;

/// Raw material at an industrial civilization: it mines its own.
const INDUSTRIAL_RAW: f32 = 0.8;
/// Food at a civilian civilization: crews and passengers eat it.
const CIVILIAN_FOOD: f32 = 1.25;
/// Parts at a scavenger civilization: it salvages its own.
const SCAVENGER_PARTS: f32 = 0.8;
/// Ammunition at an armored civilization: it fires what it buys.
const ARMORED_AMMO: f32 = 1.25;
/// Repair stock at an armored civilization: it patches what it fights with.
const ARMORED_REPAIR: f32 = 1.15;

/// The base game's price table.
pub(crate) fn price_table() -> PriceTable {
    PriceTable {
        clamp: PriceClamp {
            min: CLAMP_MIN,
            max: CLAMP_MAX,
        },
        scarcity: ScarcityRule {
            starved: STARVED,
            glut: GLUT,
            glut_share: GLUT_SHARE,
            unkept: UNKEPT,
        },
        restock: RestockRule {
            recovery_seconds: RESTOCK_RECOVERY_SECONDS,
        },
        robbery: RobberyRule {
            refusal_seconds: ROBBERY_REFUSAL_SECONDS,
        },
        jobs: JobOfferRule {
            offer_seconds: JOB_OFFER_SECONDS,
        },
        clusters: vec![
            cluster(
                ClusterType::AsteroidRich,
                ItemCategoryType::Raw,
                ASTEROID_RICH_RAW,
            ),
            cluster(ClusterType::RockOnly, ItemCategoryType::Raw, ROCK_ONLY_RAW),
            cluster(
                ClusterType::PlanetHeavy,
                ItemCategoryType::Raw,
                PLANET_HEAVY_RAW,
            ),
            cluster(
                ClusterType::DerelictField,
                ItemCategoryType::Parts,
                DERELICT_PARTS,
            ),
            cluster(
                ClusterType::DerelictField,
                ItemCategoryType::Repair,
                DERELICT_REPAIR,
            ),
        ],
        roles: vec![
            role(
                ShipRoleType::Industrial,
                ItemCategoryType::Raw,
                INDUSTRIAL_RAW,
            ),
            role(
                ShipRoleType::Civilian,
                ItemCategoryType::Food,
                CIVILIAN_FOOD,
            ),
            role(
                ShipRoleType::Scavenger,
                ItemCategoryType::Parts,
                SCAVENGER_PARTS,
            ),
            role(ShipRoleType::Armored, ItemCategoryType::Ammo, ARMORED_AMMO),
            role(
                ShipRoleType::Armored,
                ItemCategoryType::Repair,
                ARMORED_REPAIR,
            ),
        ],
    }
}

/// A demand rule that moves ask and bid by one factor, so it keeps the spread.
fn cluster(cluster: ClusterType, category: ItemCategoryType, factor: f32) -> ClusterPriceRule {
    ClusterPriceRule {
        cluster,
        category,
        factor: PriceFactor {
            ask: factor,
            bid: factor,
        },
    }
}

/// A civilization demand rule that moves ask and bid by one factor.
fn role(role: ShipRoleType, category: ItemCategoryType, factor: f32) -> RolePriceRule {
    RolePriceRule {
        role,
        category,
        factor: PriceFactor {
            ask: factor,
            bid: factor,
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nova_gameplay::prelude::{
        price, price_table_faults, CivilizationId, GameItems, ItemRoleType, TraderMarket,
    };

    use super::*;
    use crate::base_content::items::item_catalog;

    /// A trader of an all-industrial civilization in `cluster` that keeps no ore.
    fn ore_buyer(cluster: ClusterType) -> TraderMarket {
        TraderMarket {
            cluster,
            civilization: CivilizationId {
                world_seed: 1,
                node: [0, 0, 0],
            },
            role_preference: [0.0, 1.0, 0.0, 0.0],
            target: BTreeMap::new(),
        }
    }

    #[test]
    fn base_price_table_has_no_faults() {
        assert_eq!(price_table_faults(&price_table()), Vec::<String>::new());
    }

    #[test]
    fn base_iron_ore_sells_cheaper_in_an_asteroid_rich_cluster_than_a_planet_heavy_one() {
        let table = price_table();
        let items = GameItems::new(item_catalog());
        let ore = items.role(ItemRoleType::IronOre);
        let rich = price(&table, ore, &ore_buyer(ClusterType::AsteroidRich), 0);
        let heavy = price(&table, ore, &ore_buyer(ClusterType::PlanetHeavy), 0);
        // Iron ore is 16/12 at reference; unkept bids 0.9, industrial raw 0.8.
        assert_eq!((rich.price.ask_cr, rich.price.bid_cr), (8, 5));
        assert_eq!((heavy.price.ask_cr, heavy.price.bid_cr), (19, 13));
        assert!(rich.price.bid_cr < heavy.price.bid_cr);
    }
}
