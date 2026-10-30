//! A station bought from its civilization: it becomes the player's.
//!
//! A living generated station of a civilization that is an Ally of the
//! player is for sale to a ship docked with it. The price
//! is [`station_price_cr`]: what the station is worth at ask (every live
//! section's recipe, its hold's stock and its credits) times
//! [`STATION_OWNERSHIP_FACTOR`]. A buy charges the player the price and
//! makes the station the player's ([`owned_station`]): it leaves its
//! civilization, takes the Player side and drops the civilization's name.
//! The price goes to the civilization, so the station keeps its own balance
//! and stock. The player's storage there was already the player's and is not
//! priced.
//!
//! An owned station earns nothing: no income rule exists.

use bevy::prelude::Name;
use nova_gameplay::prelude::{
    Allegiance, BuildRecipe, GameItems, OwnedStationMarker, ShipInventory,
};

use super::hull_buy::hull_price_cr;

/// The price rule, the sale terms, the plan, its refusals and what a bought
/// station becomes.
pub mod prelude {
    pub use super::{
        owned_station, plan_station_buy, station_price_cr, StationBuyPlan, StationBuyRefusalType,
        StationSaleType, OWNED_STATION_NAME, STATION_OWNERSHIP_FACTOR,
    };
}

/// The name a station the player owns carries in place of its
/// civilization's (`"<civilization> station"`). It ends in `" station"` as
/// every generated station name does, so a bought station that dies is
/// renamed as a generated one is (`"Owned dead station"`).
pub const OWNED_STATION_NAME: &str = "Owned station";

/// What a bought station's root takes: [`OwnedStationMarker`], the Player
/// side and [`OWNED_STATION_NAME`].
///
/// The station is no longer its civilization's: its guns and the relation
/// model read it as the player's side, and its name no longer names the
/// civilization. The marker is the one record of ownership, frozen as
/// `FrozenStation.owned`; the side and the name freeze with the hull and the
/// body as every ship's do. A generated civilization can be on the Player
/// side, so the side alone does not say who owns a station.
pub fn owned_station() -> (OwnedStationMarker, Allegiance, Name) {
    (
        OwnedStationMarker,
        Allegiance::Player,
        Name::new(OWNED_STATION_NAME),
    )
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
/// stack of its hold at the ask of its reference price
/// ([`ItemPrice::buy_cr`](nova_gameplay::prelude::ItemPrice::buy_cr)) and its
/// credits. Wide so no station overflows it.
pub fn station_price_cr<'a>(
    items: &GameItems,
    recipes: impl IntoIterator<Item = &'a BuildRecipe>,
    stock: &ShipInventory,
    credits_cr: u32,
) -> u64 {
    let stock_cr: u64 = stock
        .stacks()
        .map(|(item, count)| items.design(item).reference_price().buy_cr(count))
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
    /// A living generated station of a civilization that is not an Ally of
    /// the player: it sells to allies only.
    NotAllied,
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
    /// The partner's civilization is not an Ally of the player: it sells to
    /// allies only.
    NotAllied,
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
/// Checks run in [`StationBuyRefusalType`] order: not docked, not for sale,
/// already owned, not allied, then the balance. A refusal changes nothing.
pub fn plan_station_buy(
    price_cr: u64,
    sale: Option<StationSaleType>,
    own_cr: u32,
) -> Result<StationBuyPlan, StationBuyRefusalType> {
    match sale.ok_or(StationBuyRefusalType::NotDocked)? {
        StationSaleType::NotForSale => return Err(StationBuyRefusalType::NotForSale),
        StationSaleType::Owned => return Err(StationBuyRefusalType::AlreadyOwned),
        StationSaleType::NotAllied => return Err(StationBuyRefusalType::NotAllied),
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
            plan_station_buy(0, Some(StationSaleType::NotAllied), 1_000),
            Err(NotAllied),
            "a civilization not an Ally sells to nobody"
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
