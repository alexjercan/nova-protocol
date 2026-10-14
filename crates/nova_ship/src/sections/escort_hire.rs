//! An escort hired from a station's catalogue into the player's fleet.
//!
//! A living generated station hires out a few crewed hulls beside the ones it
//! sells ([`HullOfferType::Hire`]). A hire at a docked station that offers
//! Refit charges [`escort_price_cr`] and, while the fleet has a free slot
//! ([`FLEET_SIDES`]), raises an [`EscortHireRequest`]: the scenario crate
//! takes the offer off the catalogue and spawns the escort beside the player,
//! following it. The escort's id is [`escort_id`], derived from the station's
//! id and the offer's roster slot, so it is the same whenever it is hired.

use bevy::prelude::*;
use nova_events::prelude::GeneratedBodyId;
use nova_gameplay::prelude::GameItems;

use super::{hull_buy::hull_price_cr, prelude::*};

/// The hire factor, the price over every section's recipe at ask, as a
/// fraction (3/2): a hire pays for the crew and the guns on top of the hull.
pub const ESCORT_HIRE_FACTOR: (u64, u64) = (3, 2);

/// The slots of the player's fleet, in fill order: each is the side, in the
/// formation's frame (`formation_goal`), an escort flies on (starboard, then
/// port).
/// The fleet cap is their count.
pub const FLEET_SIDES: [Vec3; 2] = [Vec3::X, Vec3::NEG_X];

/// The offers' terms, the price rule, the plan and the hire request.
pub mod prelude {
    pub use super::{
        escort_id, escort_price_cr, plan_escort_hire, EscortHirePartner, EscortHirePlan,
        EscortHireRefusalType, EscortHireRequest, ESCORT_HIRE_FACTOR, FLEET_SIDES,
    };
}

/// Asks to hire offer `offer` of the station `station` into the fleet of the
/// ship `entity`, on side `side`. Raised by a Ship pane hire after the hire
/// rule moved the credits; the scenario crate observes it.
#[derive(EntityEvent, Clone, Debug)]
pub struct EscortHireRequest {
    /// The hiring ship root, which the escort follows.
    pub entity: Entity,
    /// The station root that hires the escort out.
    pub station: Entity,
    /// The offer's index in the station's [`HullOffers`].
    pub offer: usize,
    /// The escort's scenario id, see [`escort_id`].
    pub escort: GeneratedBodyId,
    /// The fleet slot the escort takes, one of [`FLEET_SIDES`].
    pub side: Vec3,
}

/// The id of the escort in roster slot `slot` of the station `station`: in
/// the station's cell, its stem the station's stem and index then `escort`,
/// its index the slot.
pub fn escort_id(station: &GeneratedBodyId, slot: u32) -> GeneratedBodyId {
    GeneratedBodyId::new(
        station.origin,
        format!("{}_{}_escort", station.stem, station.index),
        slot,
    )
}

/// An escort's hire price: every section's recipe at ask
/// ([`hull_price_cr`]) times [`ESCORT_HIRE_FACTOR`], rounded down.
pub fn escort_price_cr(items: &GameItems, offer: &HullOffer) -> u64 {
    let (numerator, denominator) = ESCORT_HIRE_FACTOR;
    hull_price_cr(
        items,
        offer
            .sections
            .iter()
            .map(|section| &section.config.base.build),
    )
    .saturating_mul(numerator)
        / denominator
}

/// The docked station a hire reads: whether it is a living station that
/// offers Refit, and its balance.
#[derive(Clone, Copy, Debug)]
pub struct EscortHirePartner {
    /// The partner is a living station whose design offers Refit.
    pub offers: bool,
    /// The partner's credits.
    pub credits_cr: u32,
}

/// Why a hire changes nothing, in check order.
#[derive(Clone, Debug, PartialEq)]
pub enum EscortHireRefusalType {
    /// The ship has no docked partner.
    NotDocked,
    /// The partner is not a living station that offers Refit.
    NoRefitService,
    /// Every fleet slot holds an escort.
    FleetFull {
        /// How many escorts the fleet holds at most.
        cap: usize,
    },
    /// The ship's balance does not cover the price.
    InsufficientCredits {
        /// The price.
        need: u64,
        /// The ship's balance.
        have: u32,
    },
    /// A balance after the hire would not fit in a credit balance.
    CreditOverflow,
}

/// What an allowed hire does: the price, both balances after, and the slot
/// the escort takes.
#[derive(Clone, Debug, PartialEq)]
pub struct EscortHirePlan {
    /// The hire price, see [`escort_price_cr`].
    pub price_cr: u32,
    /// The ship's balance after the hire.
    pub own_cr: u32,
    /// The station's balance after the hire.
    pub station_cr: u32,
    /// The roster slot of the offer, see [`escort_id`].
    pub slot: u32,
    /// The fleet slot the escort takes: the first of [`FLEET_SIDES`] no
    /// escort holds.
    pub side: Vec3,
}

/// Plan hiring `offer` while docked with `partner`, with escorts already on
/// the sides `held`. `partner` is `None` while the ship is not docked. Checks
/// run in [`EscortHireRefusalType`] order. A refusal changes nothing.
///
/// # Panics
///
/// On an offer for sale: the Ship pane lists those apart, so a hire of one is
/// an assembly fault.
pub fn plan_escort_hire(
    items: &GameItems,
    offer: &HullOffer,
    partner: Option<EscortHirePartner>,
    held: &[Vec3],
    own_cr: u32,
) -> Result<EscortHirePlan, EscortHireRefusalType> {
    let HullOfferType::Hire { slot } = offer.terms else {
        panic!(
            "plan_escort_hire: '{}' is for sale, not for hire",
            offer.name
        );
    };
    let partner = partner.ok_or(EscortHireRefusalType::NotDocked)?;
    if !partner.offers {
        return Err(EscortHireRefusalType::NoRefitService);
    }
    let side = FLEET_SIDES
        .into_iter()
        .find(|side| !held.contains(side))
        .ok_or(EscortHireRefusalType::FleetFull {
            cap: FLEET_SIDES.len(),
        })?;
    let price = escort_price_cr(items, offer);
    if price > u64::from(own_cr) {
        return Err(EscortHireRefusalType::InsufficientCredits {
            need: price,
            have: own_cr,
        });
    }
    let fit = |value: u64| u32::try_from(value).map_err(|_| EscortHireRefusalType::CreditOverflow);
    Ok(EscortHirePlan {
        price_cr: fit(price)?,
        own_cr: fit(u64::from(own_cr) - price)?,
        station_cr: fit(u64::from(partner.credits_cr) + price)?,
        slot,
        side,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        super::hull_buy::tests::{items, offer},
        *,
    };

    /// A hire pays the recipes at ask times 3/2 and takes the first free
    /// side; a full fleet and a short balance refuse it.
    #[test]
    fn a_hire_pays_half_again_the_hull_and_fills_starboard_then_port() {
        let items = items();
        let escort = HullOffer {
            terms: HullOfferType::Hire { slot: 1 },
            ..offer()
        };
        let recipes = hull_price_cr(
            &items,
            escort
                .sections
                .iter()
                .map(|section| &section.config.base.build),
        );
        let partner = Some(EscortHirePartner {
            offers: true,
            credits_cr: 100,
        });
        let plan = plan_escort_hire(&items, &escort, partner, &[Vec3::X], 10_000)
            .expect("one side is free and the balance covers the price");
        assert_eq!(u64::from(plan.price_cr), recipes * 3 / 2);
        assert_eq!(plan.own_cr, 10_000 - plan.price_cr);
        assert_eq!(plan.station_cr, 100 + plan.price_cr);
        assert_eq!((plan.slot, plan.side), (1, Vec3::NEG_X));
        assert_eq!(
            plan_escort_hire(&items, &escort, partner, &FLEET_SIDES, 10_000),
            Err(EscortHireRefusalType::FleetFull { cap: 2 })
        );
        assert_eq!(
            plan_escort_hire(&items, &escort, partner, &[], 1),
            Err(EscortHireRefusalType::InsufficientCredits {
                need: recipes * 3 / 2,
                have: 1
            })
        );
    }
}
