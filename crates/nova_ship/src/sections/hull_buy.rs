//! A whole hull bought from a station, the old hull kept and parked.
//!
//! A living generated station parks every hull it sells at its docking
//! ports, one hull per port and one port kept free for the player: each is a
//! [`ParkedHull`] of the station, docked, inert and listed in the station's
//! [`ParkedHulls`]. A buy at a docked station that offers Refit swaps the
//! player's hull with a parked one for its full price, every section's
//! recipe at ask ([`hull_price_cr`]). The old hull stays the player's: it
//! stays docked where it was as an owned parked hull
//! ([`ParkedHullType::Owned`], [`OwnedShipMarker`]), its hold whole, and a
//! board swaps the player back into it for free. [`plan_hull_buy`] checks
//! the station and the balance, and a [`HullBuyRequest`] asks the scenario
//! crate to do the swap.
//!
//! A station's escorts for hire are its hulls too: each coasts beside it,
//! undocked, listed [`ParkedHullType::Hire`], where [`hire_offer_offsets`]
//! puts it (`crate::sections::escort_hire`).

use bevy::prelude::*;
use nova_gameplay::prelude::{BuildRecipe, GameItems, SectionClass};

use super::{docking_section::PORT_FACE, prelude::*};

/// The parked hulls, the price rule, the plan, the pose rules
/// and the buy request.
pub mod prelude {
    pub use super::{
        bought_hull_pose, design_envelope, design_reach, hire_offer_offsets, hull_price_cr,
        outward_berth, parked_hull_clear_pose, parked_hull_pose, plan_hull_buy, HullBuyPartner,
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
    /// The player's own ship, parked after a buy or a board: a board swaps
    /// the player into it for free. It carries [`OwnedShipMarker`].
    Owned,
    /// An escort for hire, coasting beside the station, not docked.
    Hire {
        /// The escort's roster slot, see [`HullOfferType::Hire`].
        slot: u32,
    },
}

/// A ship root the station it names holds, listed by its
/// [`ParkedHullType`]: for sale or the player's own, docked at one of its ports, or
/// for hire, coasting beside it.
///
/// A parked hull is inert: nobody drives it, it is Neutral, every
/// capability is off, and it is neither lootable nor a body of any cell. It
/// freezes and thaws inside the station's record, and it despawns with the
/// station ([`ParkedHulls`]). A buy or a board swaps a docked one with the
/// player's hull ([`HullBuyRequest`]); a hire makes one for hire the
/// player's escort.
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
/// the player's own ([`ParkedHullType::Owned`]). Raised by a Station pane
/// buy after the buy rule moved the credits, or by a board, which moves
/// none; the scenario crate observes it.
#[derive(EntityEvent, Clone, Debug)]
pub struct HullBuyRequest {
    /// The ship root.
    pub entity: Entity,
    /// The station root the hull is parked at.
    pub station: Entity,
    /// The parked hull bought or boarded.
    pub hull: Entity,
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
    /// The ship's balance does not cover the price.
    InsufficientCredits {
        /// The price.
        need: u64,
        /// The ship's balance.
        have: u32,
    },
    /// A balance after the buy would not fit in a credit balance.
    CreditOverflow,
}

/// What an allowed buy does: the price and the balances after.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HullBuyPlan {
    /// The new hull's price, see [`hull_price_cr`].
    pub price_cr: u32,
    /// The ship's balance after the buy: before, less the price.
    pub own_cr: u32,
    /// The station's balance after the buy: before, plus the price.
    pub station_cr: u32,
}

/// Plan buying a hull for sale at `price` ([`hull_price_cr`]) while docked
/// with `partner`. The old hull stays the player's, so nothing is paid for
/// it.
///
/// `partner` is `None` while the ship is not docked. The old hold stays in
/// the old hull. Checks run in [`HullBuyRefusalType`] order. A refusal
/// changes nothing.
pub fn plan_hull_buy(
    price: u64,
    partner: Option<HullBuyPartner>,
    own_cr: u32,
) -> Result<HullBuyPlan, HullBuyRefusalType> {
    let partner = partner.ok_or(HullBuyRefusalType::NotDocked)?;
    if !partner.offers {
        return Err(HullBuyRefusalType::NoRefitService);
    }
    let Some(own_after) = u64::from(own_cr).checked_sub(price) else {
        return Err(HullBuyRefusalType::InsufficientCredits {
            need: price,
            have: own_cr,
        });
    };
    let fit = |value: u64| u32::try_from(value).map_err(|_| HullBuyRefusalType::CreditOverflow);
    Ok(HullBuyPlan {
        price_cr: fit(price)?,
        own_cr: fit(own_after)?,
        station_cr: fit(u64::from(partner.credits_cr) + price)?,
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
/// `hull_port` the hull section that docks and `obstacles` boxes in the
/// station root's frame, such as the envelopes of hulls parked before.
/// `station_envelope` and `hull_envelope` are each design's
/// [`design_envelope`] with its root at the origin, its sections first in
/// design order: the caller derives each once for every port and hull it
/// tries, since a clad design's envelope is its whole skin. The hull is posed by
/// [`parked_hull_pose`], then rolled a quarter turn at a time about the
/// station port's outward axis through its face, and the first roll where
/// no box of the hull's envelope overlaps one of the station's or an
/// obstacle wins ([`section_colliders_overlap`]). The docking pair is
/// exempt: its faces meet.
///
/// # Panics
///
/// When `berth` is not in `station` or `hull_port` not in `hull`.
pub fn parked_hull_clear_pose(
    station: &[DesignSection],
    station_envelope: &[PlacedSectionCollider],
    berth: usize,
    hull: &[DesignSection],
    hull_envelope: &[PlacedSectionCollider],
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
        .find(|&(position, rotation)| {
            hull_envelope
                .iter()
                .map(|boxed| PlacedSectionCollider {
                    position: position + rotation * boxed.position,
                    rotation: rotation * boxed.rotation,
                    collider: boxed.collider,
                })
                .enumerate()
                .all(|(index, posed)| {
                    station_envelope.iter().enumerate().all(|(other, &boxed)| {
                        (index == hull_port && other == berth)
                            || !section_colliders_overlap(posed, boxed)
                    }) && obstacles
                        .iter()
                        .all(|&obstacle| !section_colliders_overlap(posed, obstacle))
                })
        })
}

/// Where a fresh player ship berths at its starting station, as `(station
/// port, ship port, position, rotation)` with the pose in the station
/// root's frame, or `None` when it stands clear at no port.
///
/// The station's docking ports not in `taken` are tried in turn, the one
/// that faces most squarely away from the station root first and the lowest
/// design index first among equals: a port low in the design can face into
/// the station, where a berth crushes both hulls. At each, every docking
/// port of `ship` in design order, through [`parked_hull_clear_pose`].
/// `station_skin` and `ship_skin` say whether each design wears the derived
/// skin ([`design_envelope`]).
///
/// ONE rule for two callers that must agree: the sale hulls park round the
/// berth this picks before the ship exists, and the ship then berths by it
/// with those hulls as `obstacles`, which leaves the same port and pose.
pub fn outward_berth(
    station: &[DesignSection],
    station_skin: bool,
    ship: &[DesignSection],
    ship_skin: bool,
    taken: &[usize],
    obstacles: &[PlacedSectionCollider],
) -> Option<(usize, usize, Vec3, Quat)> {
    let ports = |sections: &[DesignSection]| -> Vec<usize> {
        (0..sections.len())
            .filter(|&index| sections[index].config.kind.class() == SectionClass::Docking)
            .collect()
    };
    let facing = |index: usize| {
        let port = &station[index];
        (port.rotation * Vec3::NEG_Z).dot(port.position.normalize_or_zero())
    };
    let mut berths: Vec<usize> = ports(station)
        .into_iter()
        .filter(|index| !taken.contains(index))
        .collect();
    berths.sort_by(|&a, &b| facing(b).total_cmp(&facing(a)).then(a.cmp(&b)));
    let own = ports(ship);
    let station_envelope = design_envelope(station, station_skin, (Vec3::ZERO, Quat::IDENTITY));
    let ship_envelope = design_envelope(ship, ship_skin, (Vec3::ZERO, Quat::IDENTITY));
    berths.into_iter().find_map(|berth| {
        own.iter().find_map(|&port| {
            parked_hull_clear_pose(
                station,
                &station_envelope,
                berth,
                ship,
                &ship_envelope,
                port,
                obstacles,
            )
            .map(|(at, turn)| (berth, port, at, turn))
        })
    })
}

/// Every box a design takes up with its root at `(position, rotation)`:
/// each section's collider, in design order, then, when `skin` says the
/// design wears the derived skin, each plate [`derive_skin`] lays as the
/// box its collider is: its cell's footprint, as tall as the plate's
/// volume, standing on the cell floor it bolts to. A plate stands in an
/// empty cell beside the design's sections, so two clad designs whose
/// sections only touch still clad into each other; their envelopes overlap.
pub fn design_envelope(
    sections: &[DesignSection],
    skin: bool,
    (position, rotation): (Vec3, Quat),
) -> Vec<PlacedSectionCollider> {
    let mut envelope: Vec<PlacedSectionCollider> = sections
        .iter()
        .map(|section| PlacedSectionCollider {
            position: position + rotation * section.position,
            rotation: rotation * section.rotation,
            collider: section.config.base.collider.unwrap_or_default(),
        })
        .collect();
    if skin {
        let placed: Vec<PlacedPart> = sections
            .iter()
            .map(|section| {
                RefitPlacedSection {
                    position: section.position,
                    rotation: section.rotation,
                    config: &section.config,
                }
                .placed_part()
            })
            .collect();
        let (structure, phase, _) = read_structure(&placed);
        envelope.extend(derive_skin(&structure).into_iter().map(|plate| {
            // As `plate_body` builds it: the box hangs off the floor
            // of the cell, in the plate's own turn.
            let height = plate.shape.volume();
            let centre = plate.cell.as_vec3()
                + phase
                + plate.rotation * Vec3::new(0.0, (height - 1.0) * 0.5, 0.0);
            PlacedSectionCollider {
                position: position + rotation * centre,
                rotation: rotation * plate.rotation,
                collider: SectionCollider::Cuboid {
                    size: Vec3::new(1.0, height, 1.0),
                },
            }
        }));
    }
    envelope
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

    /// The price is every recipe at ask.
    #[test]
    fn the_price_is_every_recipe_at_ask() {
        let items = items();
        let offer = offer();
        // 100 + 4 * 10 and 50 + 2 * 10.
        assert_eq!(
            hull_price_cr(&items, offer.iter().map(|s| &s.config.base.build)),
            210
        );
    }

    /// A buy charges the full price: nothing is paid for the old hull,
    /// which stays the player's.
    #[test]
    fn a_buy_charges_the_full_price() {
        let partner = HullBuyPartner {
            offers: true,
            credits_cr: 1_000,
        };
        let plan = plan_hull_buy(210, Some(partner), 500).expect("the buy is allowed");
        assert_eq!(plan.price_cr, 210);
        assert_eq!(plan.own_cr, 500 - 210);
        assert_eq!(plan.station_cr, 1_000 + 210);
    }

    /// A buy is refused, in order, off a station, at one without Refit, and
    /// when the balance falls short of the price.
    #[test]
    fn a_buy_is_refused_for_the_station_or_the_credits() {
        let at = |offers, credits_cr| Some(HullBuyPartner { offers, credits_cr });
        assert_eq!(
            plan_hull_buy(210, None, 500),
            Err(HullBuyRefusalType::NotDocked)
        );
        assert_eq!(
            plan_hull_buy(210, at(false, 0), 500),
            Err(HullBuyRefusalType::NoRefitService)
        );
        assert_eq!(
            plan_hull_buy(210, at(true, 0), 209),
            Err(HullBuyRefusalType::InsufficientCredits {
                need: 210,
                have: 209
            })
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
        let station_boxes = design_envelope(&station, false, (Vec3::ZERO, Quat::IDENTITY));
        assert!(design_envelope(&hull, false, unrolled)
            .into_iter()
            .any(|posed| section_colliders_overlap(posed, station_boxes[2])));
        let hull_boxes = design_envelope(&hull, false, (Vec3::ZERO, Quat::IDENTITY));
        let pose = parked_hull_clear_pose(&station, &station_boxes, 1, &hull, &hull_boxes, 0, &[])
            .expect("a quarter roll clears the solar part");
        for posed in design_envelope(&hull, false, pose) {
            for section in [station_boxes[0], station_boxes[2]] {
                assert!(!section_colliders_overlap(posed, section));
            }
        }
        let (dock, _) = pose;
        assert!(dock.abs_diff_eq(Vec3::new(3.0, 0.0, 0.0), 1e-4), "{dock}");
    }

    /// The start station's industrial hull parked with a section flush on a
    /// station block, and each skin clad into the other's cells. A plate
    /// fills the empty cell beside its section, so a hull does not park
    /// where its sections only touch the station when either design is
    /// clad; with neither clad, no plate is laid and it does.
    #[test]
    fn a_hull_flush_beside_a_station_block_does_not_park_where_its_skin_clads_the_block() {
        // A block with an open socket on every face, which the skin clads.
        let socketed = |mut block: DesignSection| {
            block.config.base.link_points = [
                Vec3::X,
                Vec3::NEG_X,
                Vec3::Y,
                Vec3::NEG_Y,
                Vec3::Z,
                Vec3::NEG_Z,
            ]
            .into_iter()
            .map(|normal| LinkPoint {
                id: format!("{normal}"),
                position: normal * 0.5,
                normal,
                skin: LinkPointSkinType::Clad,
            })
            .collect();
            block
        };
        let east = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
        let station = vec![
            part("core", Vec3::ZERO, Quat::IDENTITY, Vec3::ONE),
            part("collar_east", Vec3::X, east, Vec3::ONE),
            socketed(part(
                "arm",
                Vec3::new(2.0, 0.0, -2.0),
                Quat::IDENTITY,
                Vec3::ONE,
            )),
        ];
        let hull = vec![
            part("dock", Vec3::ZERO, Quat::IDENTITY, Vec3::ONE),
            socketed(part("flank", Vec3::X, Quat::IDENTITY, Vec3::ONE)),
        ];
        let flush = parked_hull_pose(
            (Vec3::ZERO, Quat::IDENTITY),
            (station[1].position, station[1].rotation),
            (hull[0].position, hull[0].rotation),
        );
        let flank_at = |pose| design_envelope(&hull, false, pose)[1].position;
        assert!(
            flank_at(flush).abs_diff_eq(Vec3::new(2.0, 0.0, -1.0), 1e-4),
            "flush, the flank touches the arm: {}",
            flank_at(flush)
        );
        let envelope = |sections: &[DesignSection], skin: bool| {
            design_envelope(sections, skin, (Vec3::ZERO, Quat::IDENTITY))
        };
        let (at, turn) = parked_hull_clear_pose(
            &station,
            &envelope(&station, false),
            1,
            &hull,
            &envelope(&hull, false),
            0,
            &[],
        )
        .expect("an unclad hull parks flush");
        assert!(at.abs_diff_eq(flush.0, 1e-4) && turn.abs_diff_eq(flush.1, 1e-4));
        for (station_skin, hull_skin) in [(false, true), (true, false), (true, true)] {
            let clad = parked_hull_clear_pose(
                &station,
                &envelope(&station, station_skin),
                1,
                &hull,
                &envelope(&hull, hull_skin),
                0,
                &[],
            )
            .expect("a quarter roll takes the plating clear of the arm");
            assert!(
                flank_at(clad).distance(station[2].position) > 2.0,
                "station clad {station_skin}, hull clad {hull_skin}: the flank rolls away from \
                 the arm, got {}",
                flank_at(clad)
            );
        }
    }

    /// The fresh player ship takes the free station port that faces most
    /// squarely away from the station, whatever its design index, and the
    /// next one out when that one is taken or blocked at every roll.
    #[test]
    fn the_start_berth_takes_the_outward_most_port_where_the_ship_stands_clear() {
        let port = |id: &str, position: Vec3, rotation: Quat| {
            let mut port = part(id, position, rotation, Vec3::ONE);
            port.config.kind = SectionKind::Docking(DockingSectionConfig::default());
            port
        };
        let station = vec![
            part("core", Vec3::ZERO, Quat::IDENTITY, Vec3::ONE),
            // Faces -Z, square to the line from the core.
            port("side", Vec3::X, Quat::IDENTITY),
            // Faces -X, straight out from the core.
            port(
                "out",
                Vec3::NEG_X,
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            ),
            // Faces -Z, back into the core.
            port("in", Vec3::Z, Quat::IDENTITY),
        ];
        let ship = vec![
            port("dock", Vec3::ZERO, Quat::IDENTITY),
            part(
                "wing",
                Vec3::new(0.0, 0.0, 1.0),
                Quat::IDENTITY,
                Vec3::new(3.0, 1.0, 1.0),
            ),
        ];
        let berth = |taken: &[usize], obstacles: &[PlacedSectionCollider]| {
            outward_berth(&station, false, &ship, false, taken, obstacles)
                .map(|(berth, port, _, _)| (berth, port))
        };
        assert_eq!(berth(&[], &[]), Some((2, 0)));
        assert_eq!(berth(&[2], &[]), Some((1, 0)));
        let (_, _, at, _) = outward_berth(&station, false, &ship, false, &[], &[])
            .expect("the outward port is clear");
        assert!(at.abs_diff_eq(Vec3::new(-2.0, 0.0, 0.0), 1e-4), "{at}");
        // On the port's axis, where the wing stands at every roll.
        let wing_row = PlacedSectionCollider {
            position: Vec3::new(-3.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            collider: SectionCollider::Cuboid { size: Vec3::ONE },
        };
        assert_eq!(berth(&[], &[wing_row]), Some((1, 0)));
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
        let station_boxes = design_envelope(&station, false, (Vec3::ZERO, Quat::IDENTITY));
        let hull_boxes = design_envelope(&hull, false, (Vec3::ZERO, Quat::IDENTITY));
        assert!(
            parked_hull_clear_pose(&station, &station_boxes, 1, &hull, &hull_boxes, 0, &[])
                .is_some()
        );
        assert_eq!(
            parked_hull_clear_pose(
                &station,
                &station_boxes,
                1,
                &hull,
                &hull_boxes,
                0,
                &[obstacle(Vec3::new(4.0, 1.5, 0.0))]
            ),
            None
        );
    }
}
