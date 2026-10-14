//! A whole hull bought from a station's catalogue, the old hull traded in.
//!
//! A living generated station sells a short list of hulls ([`HullOffers`],
//! derived by the scenario crate from the station's catalogue). A buy at a
//! docked station that offers Refit replaces the player's sections with the
//! offer's: the price is every new section's recipe at ask
//! ([`hull_price_cr`]); the station pays for the old hull its strip value at
//! bid, scaled by each live section's health ([`hull_trade_in_cr`]); the old
//! hold moves whole into the station's storage. [`plan_hull_buy`] checks the
//! station, the storage and both balances, and a [`HullBuyRequest`] asks the
//! scenario crate to put the offer in the ship's design, spawn it with the
//! start-role bindings, and pose it so its first docking port lies where the
//! old port was ([`bought_hull_pose`]).

use bevy::prelude::*;
use nova_gameplay::prelude::{
    BuildRecipe, GameItems, ItemDesignId, SectionClass, ShipInventory, StationStorage,
};

use super::{docking_section::PORT_FACE, prelude::*};

/// The offers, the price and trade-in rules, the plan, the pose rule and the
/// buy request.
pub mod prelude {
    pub use super::{
        bought_hull_pose, hull_price_cr, hull_trade_in_cr, plan_hull_buy, HullBuyPartner,
        HullBuyPlan, HullBuyRefusalType, HullBuyRequest, HullOffer, HullOfferType, HullOffers,
    };
}

/// The terms a station offers one catalogue hull on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum HullOfferType {
    /// The player buys the hull and it replaces the player's own
    /// ([`HullBuyRequest`]).
    Sale,
    /// The player hires the hull, crewed, as an escort that joins the fleet
    /// (`crate::sections::escort_hire`). A hire consumes the offer.
    Hire {
        /// The escort's slot in the station's roster: the index of its
        /// generated id, so a hired escort keeps one id whatever was hired
        /// before it.
        slot: u32,
    },
}

/// One hull a station sells, as the buy rule and the Ship pane read it.
#[derive(Clone, Debug)]
pub struct HullOffer {
    /// The catalogue name the list shows (`Vel civilian Mk 1`).
    pub name: String,
    /// Whether the player buys or hires it.
    pub terms: HullOfferType,
    /// Every section the hull places, resolved, in design order.
    pub sections: Vec<DesignSection>,
}

impl HullOffer {
    /// The first docking port in design order: the port a bought hull is
    /// posed and docked by.
    pub fn first_port(&self) -> Option<&DesignSection> {
        self.sections
            .iter()
            .find(|section| section.config.kind.class() == SectionClass::Docking)
    }
}

/// The hulls a station sells, in catalogue order. Inserted on a station
/// root by the scenario crate from the station's catalogue; a station with
/// none sells no hull.
#[derive(Component, Clone, Debug, Default)]
pub struct HullOffers(pub Vec<HullOffer>);

/// Asks to replace ship `entity`'s hull with offer `offer` of the station
/// `station` sells, docked where the old hull was. Raised by a Ship pane
/// buy after the buy rule moved the hold and the credits; the scenario crate
/// observes it.
#[derive(EntityEvent, Clone, Debug)]
pub struct HullBuyRequest {
    /// The ship root.
    pub entity: Entity,
    /// The station root that sells the hull.
    pub station: Entity,
    /// The offer's index in the station's [`HullOffers`].
    pub offer: usize,
}

/// A hull's price: every section's recipe at ask
/// ([`BuildRecipe::value_cr`]).
pub fn hull_price_cr<'a>(
    items: &GameItems,
    recipes: impl IntoIterator<Item = &'a BuildRecipe>,
) -> u64 {
    recipes
        .into_iter()
        .map(|recipe| recipe.value_cr(items))
        .sum()
}

/// What a station pays for a hull: each live section's strip value at bid
/// ([`BuildRecipe::strip_value_cr`]) times its health share, rounded down
/// per section. `live` lists every live section's recipe with its share,
/// current over max health; a share outside `[0, 1]` is clamped. A lost
/// section is not listed and pays nothing.
pub fn hull_trade_in_cr<'a>(
    items: &GameItems,
    live: impl IntoIterator<Item = (&'a BuildRecipe, f32)>,
) -> u64 {
    live.into_iter()
        .map(|(recipe, share)| {
            let share = f64::from(share.clamp(0.0, 1.0));
            // The share is at most one, so the product fits the strip value.
            (recipe.strip_value_cr(items) as f64 * share).floor() as u64
        })
        .sum()
}

/// The docked station a buy reads: whether it is a living station that
/// offers Refit, its storage and its balance.
#[derive(Clone, Copy, Debug)]
pub struct HullBuyPartner<'a> {
    /// The partner is a living station whose design offers Refit.
    pub offers: bool,
    /// The partner's storage, which takes the old hold.
    pub storage: &'a StationStorage,
    /// The partner's credits.
    pub credits_cr: u32,
}

/// Why a buy changes nothing, in check order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HullBuyRefusalType {
    /// The ship has no docked partner.
    NotDocked,
    /// The partner is not a living station that offers Refit.
    NoRefitService,
    /// The station's storage cannot take the old hold.
    StorageFull {
        /// The hold's mass in grams.
        need_g: u32,
        /// What the storage has room for.
        free_g: u32,
    },
    /// The ship's balance and the trade-in do not cover the price.
    InsufficientCredits {
        /// The price.
        need: u64,
        /// The ship's balance plus the trade-in.
        have: u64,
    },
    /// The trade-in is worth more than the price and the station cannot pay
    /// the difference.
    StationShortOfCredits {
        /// The trade-in less the price.
        need: u64,
        /// What the station holds.
        have: u32,
    },
    /// A balance after the buy would not fit in a credit balance.
    CreditOverflow,
}

/// What an allowed buy does: the price, the trade-in, the balances after and
/// the stacks the old hold moves into storage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HullBuyPlan {
    /// The new hull's price, see [`hull_price_cr`].
    pub price_cr: u32,
    /// What the station pays for the old hull, see [`hull_trade_in_cr`].
    pub trade_in_cr: u32,
    /// The ship's balance after the buy: before, less the price, plus the
    /// trade-in.
    pub own_cr: u32,
    /// The station's balance after the buy: before, plus the price, less
    /// the trade-in.
    pub station_cr: u32,
    /// Every stack of the old hold, in id order, which moves into storage.
    pub stored: Vec<(ItemDesignId, u32)>,
}

/// Plan buying `offer` while docked with `partner`, trading in a hull worth
/// `trade_in_cr` ([`hull_trade_in_cr`]).
///
/// `partner` is `None` while the ship is not docked. The whole `hold` moves
/// into the partner's storage. Checks run in [`HullBuyRefusalType`] order. A
/// refusal changes nothing.
pub fn plan_hull_buy(
    items: &GameItems,
    offer: &HullOffer,
    trade_in_cr: u64,
    partner: Option<HullBuyPartner<'_>>,
    hold: &ShipInventory,
    own_cr: u32,
) -> Result<HullBuyPlan, HullBuyRefusalType> {
    let partner = partner.ok_or(HullBuyRefusalType::NotDocked)?;
    if !partner.offers {
        return Err(HullBuyRefusalType::NoRefitService);
    }
    let need_g = hold.used_g(items);
    let free_g = partner.storage.free_g(items);
    if need_g > free_g {
        return Err(HullBuyRefusalType::StorageFull { need_g, free_g });
    }
    let price = hull_price_cr(
        items,
        offer
            .sections
            .iter()
            .map(|section| &section.config.base.build),
    );
    let have = u64::from(own_cr) + trade_in_cr;
    if price > have {
        return Err(HullBuyRefusalType::InsufficientCredits { need: price, have });
    }
    let station_have = u64::from(partner.credits_cr) + price;
    if trade_in_cr > station_have {
        return Err(HullBuyRefusalType::StationShortOfCredits {
            need: trade_in_cr - price,
            have: partner.credits_cr,
        });
    }
    let fit = |value: u64| u32::try_from(value).map_err(|_| HullBuyRefusalType::CreditOverflow);
    Ok(HullBuyPlan {
        price_cr: fit(price)?,
        trade_in_cr: fit(trade_in_cr)?,
        own_cr: fit(have - price)?,
        station_cr: fit(station_have - trade_in_cr)?,
        stored: hold
            .stacks()
            .map(|(item, count)| (item.clone(), count))
            .collect(),
    })
}

/// Where a bought hull's root stands so its port lies where the old port
/// was: the new port's retracted face on the old one's, facing the same
/// way.
///
/// `root` is the old root's world position and rotation. `old_port` and
/// `new_port` are each port's position and rotation in its own root's frame.
/// The rotation is the old root's turned by the shortest arc that lines the
/// new port's outward axis up with the old one's.
pub fn bought_hull_pose(
    root: (Vec3, Quat),
    old_port: (Vec3, Quat),
    new_port: (Vec3, Quat),
) -> (Vec3, Quat) {
    let (position, rotation) = root;
    let old_turn = rotation * old_port.1;
    let face = position + rotation * old_port.0 + old_turn * PORT_FACE;
    let axis = old_turn * Vec3::NEG_Z;
    let arc = Quat::from_rotation_arc((rotation * new_port.1 * Vec3::NEG_Z).normalize(), axis);
    let turned = (arc * rotation).normalize();
    let reach = turned * (new_port.0 + new_port.1 * PORT_FACE);
    (face - reach, turned)
}

#[cfg(test)]
pub(crate) mod tests {
    use nova_gameplay::prelude::{ItemCategoryType, ItemDesign};

    use super::*;

    pub(crate) fn items() -> GameItems {
        GameItems::new([
            ItemDesign {
                id: "plate".into(),
                name: "Plate".to_string(),
                about: String::new(),
                category: ItemCategoryType::Parts,
                mass_g: 1_000,
                ask_cr: 10,
                bid_cr: 6,
            },
            ItemDesign {
                id: "ore".into(),
                name: "Ore".to_string(),
                about: String::new(),
                category: ItemCategoryType::Raw,
                mass_g: 2_000,
                ask_cr: 4,
                bid_cr: 2,
            },
        ])
    }

    fn recipe(credits: u32, plates: u32) -> BuildRecipe {
        BuildRecipe {
            credits,
            parts: [("plate".into(), plates)].into_iter().collect(),
            strip_yield: 0.5,
        }
    }

    fn section(id: &str, kind: SectionKind, build: BuildRecipe) -> DesignSection {
        DesignSection {
            id: id.to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            config: SectionConfig {
                base: BaseSectionConfig {
                    id: id.to_string(),
                    build,
                    ..default()
                },
                kind,
            },
        }
    }

    pub(crate) fn offer() -> HullOffer {
        HullOffer {
            name: "Test hull".to_string(),
            terms: HullOfferType::Sale,
            sections: vec![
                section(
                    "hull",
                    SectionKind::Hull(HullSectionConfig::default()),
                    recipe(100, 4),
                ),
                section(
                    "pdc",
                    SectionKind::Turret(TurretSectionConfig::default()),
                    recipe(50, 2),
                ),
            ],
        }
    }

    fn storage(cap_g: u32) -> StationStorage {
        StationStorage::new(cap_g)
    }

    /// The price is every recipe at ask, and the trade-in each live
    /// section's strip at bid scaled by its health, rounded down apiece.
    #[test]
    fn the_price_is_every_recipe_at_ask_and_the_trade_in_the_strip_scaled_by_health() {
        let items = items();
        let offer = offer();
        // 100 + 4 * 10 and 50 + 2 * 10.
        assert_eq!(
            hull_price_cr(&items, offer.sections.iter().map(|s| &s.config.base.build)),
            210
        );
        // Strips return 2 and 1 plates at 6: 12 whole and 6 at half health.
        let (whole, damaged) = (recipe(100, 4), recipe(50, 2));
        assert_eq!(
            hull_trade_in_cr(&items, [(&whole, 1.0), (&damaged, 0.5)]),
            15
        );
        assert_eq!(hull_trade_in_cr(&items, [(&whole, 0.49)]), 5);
    }

    /// The credits move as balance less price plus trade-in, and the whole
    /// hold is listed to move into storage.
    #[test]
    fn a_buy_charges_the_price_less_the_trade_in_and_stores_the_whole_hold() {
        let items = items();
        let hold = ShipInventory::new(&items, 10_000, [("ore".into(), 3)]);
        let storage = storage(10_000);
        let partner = HullBuyPartner {
            offers: true,
            storage: &storage,
            credits_cr: 1_000,
        };
        let plan = plan_hull_buy(&items, &offer(), 40, Some(partner), &hold, 500)
            .expect("the buy is allowed");
        assert_eq!(plan.price_cr, 210);
        assert_eq!(plan.trade_in_cr, 40);
        assert_eq!(plan.own_cr, 500 - 210 + 40);
        assert_eq!(plan.station_cr, 1_000 + 210 - 40);
        assert_eq!(plan.stored, vec![("ore".into(), 3)]);
    }

    /// A buy is refused, in order, off a station, at one without Refit, with
    /// no room for the hold, and when the credits fall short either way.
    #[test]
    fn a_buy_is_refused_for_the_station_the_storage_or_the_credits() {
        let items = items();
        let hold = ShipInventory::new(&items, 10_000, [("ore".into(), 3)]);
        let (roomy, tight) = (storage(10_000), storage(5_000));
        let at = |storage, offers, credits_cr| {
            Some(HullBuyPartner {
                offers,
                storage,
                credits_cr,
            })
        };
        let plan =
            |partner, trade_in, own| plan_hull_buy(&items, &offer(), trade_in, partner, &hold, own);
        assert_eq!(plan(None, 0, 500), Err(HullBuyRefusalType::NotDocked));
        assert_eq!(
            plan(at(&roomy, false, 0), 0, 500),
            Err(HullBuyRefusalType::NoRefitService)
        );
        assert_eq!(
            plan(at(&tight, true, 0), 0, 500),
            Err(HullBuyRefusalType::StorageFull {
                need_g: 6_000,
                free_g: 5_000
            })
        );
        assert_eq!(
            plan(at(&roomy, true, 0), 10, 199),
            Err(HullBuyRefusalType::InsufficientCredits {
                need: 210,
                have: 209
            })
        );
        assert_eq!(
            plan(at(&roomy, true, 5), 300, 0),
            Err(HullBuyRefusalType::StationShortOfCredits { need: 90, have: 5 })
        );
    }

    /// The new port's face lands on the old port's face, facing the same
    /// way, whatever the two ports' places in their hulls.
    #[test]
    fn a_bought_hull_is_posed_with_its_port_face_on_the_old_one() {
        let root = (
            Vec3::new(10.0, -4.0, 2.0),
            Quat::from_rotation_y(0.7) * Quat::from_rotation_x(0.2),
        );
        let old_port = (Vec3::new(0.0, 0.0, -3.0), Quat::IDENTITY);
        let new_port = (
            Vec3::new(2.0, 1.0, 0.0),
            Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        );
        let (position, rotation) = bought_hull_pose(root, old_port, new_port);
        let face = |(p, r): (Vec3, Quat), (at, turn): (Vec3, Quat)| {
            (
                p + r * at + r * turn * PORT_FACE,
                (r * turn * Vec3::NEG_Z).normalize(),
            )
        };
        let (old_face, old_axis) = face(root, old_port);
        let (new_face, new_axis) = face((position, rotation), new_port);
        assert!(
            new_face.abs_diff_eq(old_face, 1e-4),
            "{new_face} vs {old_face}"
        );
        assert!(
            new_axis.abs_diff_eq(old_axis, 1e-4),
            "{new_axis} vs {old_axis}"
        );
    }
}
