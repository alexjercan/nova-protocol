//! A whole hull bought from a station, the old hull traded in and parked.
//!
//! A living generated station parks every hull it sells at its docking
//! ports, one hull per port and one port kept free for the player: each is a
//! [`ParkedHull`] of the station, docked, inert and listed in the station's
//! [`ParkedHulls`]. A buy at a docked station that offers Refit swaps the
//! player's hull with a parked one: the price of a hull for sale is every
//! section's recipe at ask ([`hull_price_cr`]); the station pays for the old
//! hull its strip value at bid, scaled by each live section's health
//! ([`hull_trade_in_cr`]). The old hull stays docked where it was as a
//! parked hull for rebuy ([`ParkedHullType::Rebuy`]) at the trade-in it was
//! paid, its hold whole: the player's hold while it is parked
//! ([`OwnedShipMarker`]). [`plan_hull_buy`] checks the station and both
//! balances, and a [`HullBuyRequest`] asks the scenario crate to do the
//! swap.
//!
//! A station's escorts for hire are its hulls too: each coasts beside it,
//! undocked, listed [`ParkedHullType::Hire`], where [`hire_offer_offsets`]
//! puts it (`crate::sections::escort_hire`).

use bevy::prelude::*;
use nova_gameplay::prelude::{BuildRecipe, GameItems};

use super::{docking_section::PORT_FACE, prelude::*};

/// The parked hulls, the price and trade-in rules, the plan, the pose rules
/// and the buy request.
pub mod prelude {
    pub use super::{
        bought_hull_pose, design_reach, hire_offer_offsets, hull_price_cr, hull_trade_in_cr,
        parked_hull_clear_pose, parked_hull_pose, plan_hull_buy, posed_colliders, HullBuyPartner,
        HullBuyPlan, HullBuyRefusalType, HullBuyRequest, HullOfferType, ParkedHull, ParkedHullType,
        ParkedHulls, HIRE_OFFER_CLEARANCE,
    };
}

/// The terms a station offers one catalogue hull on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum HullOfferType {
    /// The player buys the hull: it spawns parked at one of the station's
    /// ports ([`ParkedHull`]) and a buy swaps it for the player's own.
    Sale,
    /// The player hires the hull, crewed, as an escort that joins the fleet
    /// (`crate::sections::escort_hire`). It coasts beside the station until
    /// then ([`ParkedHullType::Hire`]).
    Hire {
        /// The escort's slot in the station's roster: the index of its
        /// generated id, so a hired escort keeps one id whatever was hired
        /// before it.
        slot: u32,
    },
}

/// How a parked hull is listed: a component of every [`ParkedHull`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum ParkedHullType {
    /// A new hull for sale at its recipes' price ([`hull_price_cr`]).
    Sale,
    /// A hull the player traded in, for rebuy at the trade-in it was paid.
    Rebuy {
        /// The rebuy price: the trade-in the station paid for it.
        price_cr: u32,
    },
    /// An escort for hire, coasting beside the station, not docked.
    Hire {
        /// The escort's roster slot, see [`HullOfferType::Hire`].
        slot: u32,
    },
}

/// A ship root the station it names holds, listed by its
/// [`ParkedHullType`]: for sale or for rebuy, docked at one of its ports, or
/// for hire, coasting beside it.
///
/// A parked hull is inert: nobody drives it, it is Neutral, every
/// capability is off, and it is neither lootable nor a body of any cell. It
/// freezes and thaws inside the station's record, and it despawns with the
/// station ([`ParkedHulls`]). A buy swaps a docked one with the player's
/// hull ([`HullBuyRequest`]); a hire makes one for hire the player's escort.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[relationship(relationship_target = ParkedHulls)]
pub struct ParkedHull(pub Entity);

/// The hulls parked at a station, in park order. Despawning the station
/// despawns them.
#[derive(Component, Debug, Default, Reflect)]
#[relationship_target(relationship = ParkedHull, linked_spawn)]
pub struct ParkedHulls(Vec<Entity>);

/// Asks to swap ship `entity`'s hull with the parked hull `hull` of
/// `station`: the ship root takes the parked hull's design, state and
/// place, docked at its port, and the old hull stays docked where it was as
/// a hull for rebuy at `trade_in_cr`. Raised by a Station pane buy after the
/// buy rule moved the credits; the scenario crate observes it.
#[derive(EntityEvent, Clone, Debug)]
pub struct HullBuyRequest {
    /// The ship root.
    pub entity: Entity,
    /// The station root the hull is parked at.
    pub station: Entity,
    /// The parked hull bought.
    pub hull: Entity,
    /// What the station paid for the old hull: its rebuy price.
    pub trade_in_cr: u32,
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
/// offers Refit, and its balance.
#[derive(Clone, Copy, Debug)]
pub struct HullBuyPartner {
    /// The partner is a living station whose design offers Refit.
    pub offers: bool,
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

/// What an allowed buy does: the price, the trade-in and the balances after.
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
}

/// Plan buying a parked hull at `price` while docked with `partner`, trading
/// in a hull worth `trade_in_cr` ([`hull_trade_in_cr`]). The price is
/// [`hull_price_cr`] for a hull for sale and the rebuy price for a hull
/// traded in ([`ParkedHullType`]).
///
/// `partner` is `None` while the ship is not docked. The old hold stays in
/// the old hull. Checks run in [`HullBuyRefusalType`] order. A refusal
/// changes nothing.
pub fn plan_hull_buy(
    price: u64,
    trade_in_cr: u64,
    partner: Option<HullBuyPartner>,
    own_cr: u32,
) -> Result<HullBuyPlan, HullBuyRefusalType> {
    let partner = partner.ok_or(HullBuyRefusalType::NotDocked)?;
    if !partner.offers {
        return Err(HullBuyRefusalType::NoRefitService);
    }
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

/// Where a parked hull's root stands so its port lies on a station port:
/// the hull port's retracted face on the station port's, facing it.
///
/// `station` is the station root's world position and rotation.
/// `station_port` and `hull_port` are each port's position and rotation in
/// its own root's frame. The rotation is the station's turned by the
/// shortest arc that points the hull port's outward axis against the
/// station port's.
pub fn parked_hull_pose(
    station: (Vec3, Quat),
    station_port: (Vec3, Quat),
    hull_port: (Vec3, Quat),
) -> (Vec3, Quat) {
    let (position, rotation) = station;
    let port_turn = rotation * station_port.1;
    let face = position + rotation * station_port.0 + port_turn * PORT_FACE;
    let against = port_turn * Vec3::Z;
    let arc = Quat::from_rotation_arc((rotation * hull_port.1 * Vec3::NEG_Z).normalize(), against);
    let turned = (arc * rotation).normalize();
    let reach = turned * (hull_port.0 + hull_port.1 * PORT_FACE);
    (face - reach, turned)
}

/// Where a hull parks at station port `berth` clear of the station and of
/// `obstacles`, as `(position, rotation)` in the station root's frame, or
/// `None` when it fits there at no roll.
///
/// `station` and `hull` are each design's sections in its own root's frame,
/// `hull_port` the hull section that docks and `obstacles` colliders in the
/// station root's frame, such as hulls parked before. The hull is posed by
/// [`parked_hull_pose`], then rolled a quarter turn at a time about the
/// station port's outward axis through its face, and the first roll where no
/// hull section overlaps a station section or an obstacle wins
/// ([`section_colliders_overlap`]). The docking pair is exempt: its faces
/// meet.
///
/// # Panics
///
/// When `berth` is not in `station` or `hull_port` not in `hull`.
pub fn parked_hull_clear_pose(
    station: &[DesignSection],
    berth: usize,
    hull: &[DesignSection],
    hull_port: usize,
    obstacles: &[PlacedSectionCollider],
) -> Option<(Vec3, Quat)> {
    let port = &station[berth];
    let (at, turn) = parked_hull_pose(
        (Vec3::ZERO, Quat::IDENTITY),
        (port.position, port.rotation),
        (hull[hull_port].position, hull[hull_port].rotation),
    );
    let face = port.position + port.rotation * PORT_FACE;
    let axis = port.rotation * Vec3::NEG_Z;
    (0..4u8)
        .map(|quarter| {
            let roll =
                Quat::from_axis_angle(axis, std::f32::consts::FRAC_PI_2 * f32::from(quarter));
            (face + roll * (at - face), (roll * turn).normalize())
        })
        .find(|&pose| {
            posed_colliders(hull, pose)
                .enumerate()
                .all(|(index, posed)| {
                    station.iter().enumerate().all(|(other, section)| {
                        (index == hull_port && other == berth)
                            || !section_colliders_overlap(posed, posed_collider(section))
                    }) && obstacles
                        .iter()
                        .all(|&obstacle| !section_colliders_overlap(posed, obstacle))
                })
        })
}

/// The daylight (world units, 100 m) an escort for hire keeps to its
/// station's reach and to the escort before it on its side of the line.
pub const HIRE_OFFER_CLEARANCE: f32 = 10.0;

/// The reach (world units) of a design from its root: the furthest corner of
/// any section's collider box.
pub fn design_reach(sections: &[DesignSection]) -> f32 {
    sections
        .iter()
        .map(|section| {
            section.position.length()
                + section
                    .config
                    .base
                    .collider
                    .unwrap_or_default()
                    .aabb_half_extents()
                    .length()
        })
        .fold(0.0, f32::max)
}

/// Where each escort for hire coasts beside its station, as a signed
/// distance (world units) from the station root along one line: the first
/// ahead, the second behind, the third ahead past the first, and so on.
///
/// `station_reach` is the station's [`design_reach`] and `reaches` each
/// escort's, in list order. Each keeps [`HIRE_OFFER_CLEARANCE`] between its
/// reach and the station's, and between its reach and the escort's before
/// it on its side.
pub fn hire_offer_offsets(station_reach: f32, reaches: &[f32]) -> Vec<f32> {
    let mut edges = [station_reach; 2];
    reaches
        .iter()
        .enumerate()
        .map(|(index, &reach)| {
            let side = index % 2;
            let centre = edges[side] + HIRE_OFFER_CLEARANCE + reach;
            edges[side] = centre + reach;
            if side == 0 {
                centre
            } else {
                -centre
            }
        })
        .collect()
}

/// Every section of `sections` posed by a root at `(position, rotation)`,
/// in design order.
pub fn posed_colliders(
    sections: &[DesignSection],
    (position, rotation): (Vec3, Quat),
) -> impl Iterator<Item = PlacedSectionCollider> + '_ {
    sections.iter().map(move |section| PlacedSectionCollider {
        position: position + rotation * section.position,
        rotation: rotation * section.rotation,
        collider: section.config.base.collider.unwrap_or_default(),
    })
}

/// One section's collider in its root's frame.
fn posed_collider(section: &DesignSection) -> PlacedSectionCollider {
    PlacedSectionCollider {
        position: section.position,
        rotation: section.rotation,
        collider: section.config.base.collider.unwrap_or_default(),
    }
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

    pub(crate) fn offer() -> Vec<DesignSection> {
        vec![
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
        ]
    }

    /// Escorts for hire alternate ahead and behind, each clear of the
    /// station's reach and of the escort before it on its side.
    #[test]
    fn escorts_for_hire_coast_on_both_sides_clear_of_the_station_and_each_other() {
        let reaches = [2.0, 3.0, 2.0];
        let offsets = hire_offer_offsets(10.0, &reaches);
        assert_eq!(offsets, vec![22.0, -23.0, 36.0]);
        for (offset, reach) in offsets.iter().zip(reaches) {
            assert!(offset.abs() - reach >= 10.0 + HIRE_OFFER_CLEARANCE);
        }
        assert!(offsets[2] - reaches[2] - (offsets[0] + reaches[0]) >= HIRE_OFFER_CLEARANCE);
    }

    /// The price is every recipe at ask, and the trade-in each live
    /// section's strip at bid scaled by its health, rounded down apiece.
    #[test]
    fn the_price_is_every_recipe_at_ask_and_the_trade_in_the_strip_scaled_by_health() {
        let items = items();
        let offer = offer();
        // 100 + 4 * 10 and 50 + 2 * 10.
        assert_eq!(
            hull_price_cr(&items, offer.iter().map(|s| &s.config.base.build)),
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

    /// The credits move as balance less price plus trade-in, and the
    /// station's as balance plus price less trade-in.
    #[test]
    fn a_buy_charges_the_price_less_the_trade_in() {
        let partner = HullBuyPartner {
            offers: true,
            credits_cr: 1_000,
        };
        let plan = plan_hull_buy(210, 40, Some(partner), 500).expect("the buy is allowed");
        assert_eq!(plan.price_cr, 210);
        assert_eq!(plan.trade_in_cr, 40);
        assert_eq!(plan.own_cr, 500 - 210 + 40);
        assert_eq!(plan.station_cr, 1_000 + 210 - 40);
    }

    /// A buy is refused, in order, off a station, at one without Refit, and
    /// when the credits fall short either way.
    #[test]
    fn a_buy_is_refused_for_the_station_or_the_credits() {
        let at = |offers, credits_cr| Some(HullBuyPartner { offers, credits_cr });
        assert_eq!(
            plan_hull_buy(210, 0, None, 500),
            Err(HullBuyRefusalType::NotDocked)
        );
        assert_eq!(
            plan_hull_buy(210, 0, at(false, 0), 500),
            Err(HullBuyRefusalType::NoRefitService)
        );
        assert_eq!(
            plan_hull_buy(210, 10, at(true, 0), 199),
            Err(HullBuyRefusalType::InsufficientCredits {
                need: 210,
                have: 209
            })
        );
        assert_eq!(
            plan_hull_buy(210, 300, at(true, 5), 0),
            Err(HullBuyRefusalType::StationShortOfCredits { need: 90, have: 5 })
        );
    }

    /// A parked hull's port face lands on the station port's face and points
    /// against it, whatever the two ports' places in their hulls.
    #[test]
    fn a_parked_hull_is_posed_with_its_port_face_on_the_station_port_facing_it() {
        let station = (
            Vec3::new(10.0, -4.0, 2.0),
            Quat::from_rotation_y(0.7) * Quat::from_rotation_x(0.2),
        );
        let station_port = (
            Vec3::new(3.0, 0.0, 0.0),
            Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        );
        let hull_port = (Vec3::new(0.0, 1.0, -2.0), Quat::IDENTITY);
        let (position, rotation) = parked_hull_pose(station, station_port, hull_port);
        let face = |(p, r): (Vec3, Quat), (at, turn): (Vec3, Quat)| {
            (
                p + r * at + r * turn * PORT_FACE,
                (r * turn * Vec3::NEG_Z).normalize(),
            )
        };
        let (station_face, station_axis) = face(station, station_port);
        let (hull_face, hull_axis) = face((position, rotation), hull_port);
        assert!(
            hull_face.abs_diff_eq(station_face, 1e-4),
            "{hull_face} vs {station_face}"
        );
        assert!(
            hull_axis.abs_diff_eq(-station_axis, 1e-4),
            "{hull_axis} vs {station_axis}"
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

    /// A part posed in a root's frame, a cuboid of `size`.
    fn part(id: &str, position: Vec3, rotation: Quat, size: Vec3) -> DesignSection {
        let mut part = section(
            id,
            SectionKind::Hull(HullSectionConfig::default()),
            recipe(0, 0),
        );
        part.position = position;
        part.rotation = rotation;
        part.config.base.collider = Some(SectionCollider::Cuboid { size });
        part
    }

    /// A station with a port facing +X beside `beside`, and a hull whose
    /// three-cell wing stands one cell behind its port.
    fn east_collar(beside: &[Vec3]) -> (Vec<DesignSection>, Vec<DesignSection>) {
        let mut station = vec![
            part("core", Vec3::ZERO, Quat::IDENTITY, Vec3::ONE),
            part(
                "collar_east",
                Vec3::new(2.0, 0.0, 0.0),
                Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                Vec3::ONE,
            ),
        ];
        station.extend(
            beside
                .iter()
                .map(|&at| part("solar", at, Quat::IDENTITY, Vec3::ONE)),
        );
        let hull = vec![
            part("dock", Vec3::ZERO, Quat::IDENTITY, Vec3::ONE),
            part(
                "wing",
                Vec3::new(0.0, 0.0, 1.0),
                Quat::IDENTITY,
                Vec3::new(3.0, 1.0, 1.0),
            ),
        ];
        (station, hull)
    }

    /// The yard station split when a parked hull's wing landed in the solar
    /// row beside its collar: the hull rolls about the port until it clears.
    #[test]
    fn a_hull_for_sale_rolls_about_its_port_clear_of_a_station_part() {
        let (station, hull) = east_collar(&[Vec3::new(4.0, 0.0, 1.5)]);
        let unrolled = parked_hull_pose(
            (Vec3::ZERO, Quat::IDENTITY),
            (station[1].position, station[1].rotation),
            (hull[0].position, hull[0].rotation),
        );
        assert!(posed_colliders(&hull, unrolled)
            .any(|posed| section_colliders_overlap(posed, posed_collider(&station[2]))));
        let pose = parked_hull_clear_pose(&station, 1, &hull, 0, &[])
            .expect("a quarter roll clears the solar part");
        for posed in posed_colliders(&hull, pose) {
            for section in [&station[0], &station[2]] {
                assert!(!section_colliders_overlap(posed, posed_collider(section)));
            }
        }
        let (dock, _) = pose;
        assert!(dock.abs_diff_eq(Vec3::new(3.0, 0.0, 0.0), 1e-4), "{dock}");
    }

    /// A hull blocked at every roll, by the station or by an obstacle, is
    /// not parked at that port.
    #[test]
    fn a_hull_blocked_at_every_roll_does_not_park_at_the_port() {
        let (station, hull) = east_collar(&[Vec3::new(4.0, 0.0, 1.5), Vec3::new(4.0, 0.0, -1.5)]);
        let obstacle = |at: Vec3| PlacedSectionCollider {
            position: at,
            rotation: Quat::IDENTITY,
            collider: SectionCollider::Cuboid { size: Vec3::ONE },
        };
        assert!(parked_hull_clear_pose(&station, 1, &hull, 0, &[]).is_some());
        assert_eq!(
            parked_hull_clear_pose(&station, 1, &hull, 0, &[obstacle(Vec3::new(4.0, 1.5, 0.0))]),
            None
        );
    }
}
