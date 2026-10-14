//! A station bought from its civilization: it becomes the player's.
//!
//! A living generated station is for sale to a ship docked with it. The price
//! is [`station_price_cr`]: what the station is worth at ask (every live
//! section's recipe, its hold's stock and its credits) times
//! [`STATION_OWNERSHIP_FACTOR`]. A buy charges the player the price and
//! makes the station the player's (`OwnedStationMarker`); the price goes to
//! the civilization, so the station keeps its own balance and stock. The
//! player's storage there was already the player's and is not priced.
//!
//! An owned station earns nothing yet: income needs the play-time clock of
//! the economy strand (see the `TODO(20261012-100028)` in the scenario
//! crate's `thaw_station`).

use nova_gameplay::prelude::{BuildRecipe, GameItems, ShipInventory};

use super::hull_buy::hull_price_cr;

/// The price rule, the sale terms, the plan and its refusals.
pub mod prelude {
    pub use super::{
        plan_station_buy, station_price_cr, StationBuyPlan, StationBuyRefusalType, StationSaleType,
        STATION_OWNERSHIP_FACTOR,
    };
}

/// The ownership factor, the price over what the station is worth at ask, as
/// a fraction (2/1).
///
/// A station is more than its parts: it carries a cell's trade, it is the
/// one place its services stand, and its civilization gives it up for good.
/// Twice its worth puts it above an escort hire (3/2 of the hull), so a
/// station stays a late goal, and makes buying one to strip it lose money.
pub const STATION_OWNERSHIP_FACTOR: (u64, u64) = (2, 1);

/// What a station is worth at ask, times [`STATION_OWNERSHIP_FACTOR`],
/// rounded down: every live section's recipe ([`hull_price_cr`]), every
/// stack of its hold at [`ItemDesign::ask_cr`](nova_gameplay::prelude::ItemDesign)
/// and its credits. Wide so no station overflows it.
pub fn station_price_cr<'a>(
    items: &GameItems,
    recipes: impl IntoIterator<Item = &'a BuildRecipe>,
    stock: &ShipInventory,
    credits_cr: u32,
) -> u64 {
    let stock_cr: u64 = stock
        .stacks()
        .map(|(item, count)| u64::from(count) * u64::from(items.design(item).ask_cr))
        .sum();
    let (numerator, denominator) = STATION_OWNERSHIP_FACTOR;
    (hull_price_cr(items, recipes) + stock_cr + u64::from(credits_cr)).saturating_mul(numerator)
        / denominator
}

/// Whether the docked partner is for sale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StationSaleType {
    /// A ship, a dead station, or a station a scenario placed by hand: no
    /// civilization sells it.
    NotForSale,
    /// A living station a civilization keeps, generated with the world.
    ForSale,
    /// A station the player owns already.
    Owned,
}

/// Why a buy changes nothing, in check order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StationBuyRefusalType {
    /// The ship has no docked partner.
    NotDocked,
    /// The partner is not a living generated station.
    NotForSale,
    /// The player owns the station already.
    AlreadyOwned,
    /// The ship's balance does not cover the price.
    InsufficientCredits {
        /// The price.
        need: u64,
        /// The ship's balance.
        have: u32,
    },
}

/// What an allowed buy does: the price and the ship's balance after.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StationBuyPlan {
    /// The price, see [`station_price_cr`].
    pub price_cr: u32,
    /// The ship's balance after the buy.
    pub own_cr: u32,
}

/// Plan buying the docked station at `price_cr` ([`station_price_cr`]) with
/// a balance of `own_cr`. `sale` is `None` while the ship is not docked.
/// Checks run in [`StationBuyRefusalType`] order. A refusal changes nothing.
pub fn plan_station_buy(
    price_cr: u64,
    sale: Option<StationSaleType>,
    own_cr: u32,
) -> Result<StationBuyPlan, StationBuyRefusalType> {
    match sale.ok_or(StationBuyRefusalType::NotDocked)? {
        StationSaleType::NotForSale => return Err(StationBuyRefusalType::NotForSale),
        StationSaleType::Owned => return Err(StationBuyRefusalType::AlreadyOwned),
        StationSaleType::ForSale => {}
    }
    let left = u64::from(own_cr).checked_sub(price_cr).ok_or(
        StationBuyRefusalType::InsufficientCredits {
            need: price_cr,
            have: own_cr,
        },
    )?;
    // Both fit: the price is at most the balance, and what is left is less.
    Ok(StationBuyPlan {
        price_cr: price_cr as u32,
        own_cr: left as u32,
    })
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::ItemDesignId;

    use super::{super::hull_buy::tests::items, *};

    /// The price is every recipe at ask, the hold at ask and the credits,
    /// twice over.
    #[test]
    fn a_station_costs_twice_its_sections_stock_and_credits_at_ask() {
        let items = items();
        let plate = ItemDesignId::from("plate");
        let recipe = BuildRecipe {
            credits: 30,
            parts: [(plate.clone(), 2)].into(),
            strip_yield: 0.5,
        };
        // Two sections at 30 + 2 x 10 each; a hold of 5 plates at ask 10.
        let stock = ShipInventory::new(&items, 1_000_000, [(plate, 5)]);
        assert_eq!(
            station_price_cr(&items, [&recipe, &recipe], &stock, 400),
            (2 * (30 + 2 * 10) + 5 * 10 + 400) * 2
        );
        assert_eq!(
            station_price_cr(&items, [], &ShipInventory::default(), 0),
            0
        );
    }

    /// A buy takes the price from the balance; it refuses an undocked ship,
    /// a partner not for sale, an owned station and a short balance, in
    /// that order.
    #[test]
    fn a_station_buy_charges_the_price_and_refuses_in_order() {
        use StationBuyRefusalType::*;
        assert_eq!(
            plan_station_buy(700, Some(StationSaleType::ForSale), 1_000),
            Ok(StationBuyPlan {
                price_cr: 700,
                own_cr: 300
            })
        );
        assert_eq!(
            plan_station_buy(1_000, Some(StationSaleType::ForSale), 1_000),
            Ok(StationBuyPlan {
                price_cr: 1_000,
                own_cr: 0
            }),
            "an exact balance buys"
        );
        assert_eq!(plan_station_buy(0, None, 1_000), Err(NotDocked));
        assert_eq!(
            plan_station_buy(0, Some(StationSaleType::NotForSale), 1_000),
            Err(NotForSale)
        );
        assert_eq!(
            plan_station_buy(0, Some(StationSaleType::Owned), 1_000),
            Err(AlreadyOwned)
        );
        assert_eq!(
            plan_station_buy(u64::from(u32::MAX) + 1, Some(StationSaleType::ForSale), 5),
            Err(InsufficientCredits {
                need: u64::from(u32::MAX) + 1,
                have: 5
            })
        );
    }
}
