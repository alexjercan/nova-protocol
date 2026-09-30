//! The open world's generated ships: which civilization and role each hull a
//! cluster plans belongs to, the design [`generate_ship`] or
//! [`generate_wreck`] lays out for it, and the pose it stands at.
//!
//! PURE. Every draw comes from a stream keyed by the world seed, the
//! cluster's lattice node and the hull's planned slot, before any cell checks
//! where the hull fits. So a ship is the same ship from every cell and in any
//! visit order, and a hull a cell skips leaves a gap in the ids without
//! moving any other ship.
//!
//! A cluster draws one primary civilization from those in reach of its
//! anchor, weighted by their outward-biased selection weight. Its first hull
//! is the primary's; each later hull has a [`SECONDARY_CIVILIZATION_CHANCE`]
//! of belonging to another civilization in reach of the hull, weighted by its
//! influence there, and stays the primary's when no other is in reach. The
//! civilization's role weights at the cluster's environment pick the role,
//! among the roles its advancement can build. A living civilization's hull is
//! an intact ship, level in its local frame with the cluster's seeded yaw; an
//! extinct one's is a wreck at a seeded 3D orientation.
//!
//! An intact ship's hold starts empty. A wreck carries [`WRECK_PLATES`] hull
//! plates drawn from its own stream, keyed like the ship, at most what its
//! remaining hull sections hold, so a docked ship may Take them. Nothing keeps
//! what was taken: a retired cell comes back with its wrecks' stock whole.
//!
//! FAIL LOUD. A layout or wreck that fails for the selected civilization and
//! role fails the sector; only a cell's spatial checks may skip a planned
//! hull.
//!
//! Every chance here is provisional until generated ships are reviewed in
//! travel.

use std::ops::RangeInclusive;

use bevy::prelude::{Quat, Vec3};
use nova_events::prelude::{Meters, Meters3};
use nova_gameplay::prelude::{Fnv32, ItemType, SeedStream, ShipInventoryStock};
use nova_scenario::prelude::{ShipDesign, HULL_SECTION_CARGO_G};
use nova_ship::prelude::SectionKind;
use nova_world::prelude::*;

use crate::{
    civilizations::{
        AdvancementCurveType, Civilization, CivilizationField, CivilizationStatusType, ShipRoleType,
    },
    clusters::pick,
    environment::Environment,
    ship_layout::{generate_ship, generate_wreck, ShipLayoutRequest},
    ship_parts::ShipPartSnapshot,
};

/// The advancement curve the live world reads.
///
/// TODO(20260923-110307): the task's starting ramp until the owner picks the
/// shipped curve from the diagnostic maps; delete with
/// [`AdvancementCurveType`].
pub const SHIP_ADVANCEMENT_CURVE: AdvancementCurveType = AdvancementCurveType::Linear;

/// The chance that a cluster's hull after its first belongs to another
/// civilization in reach rather than the cluster's primary.
const SECONDARY_CIVILIZATION_CHANCE: f32 = 0.1;

/// How many hull plates a wreck draws for its hold, before its hull caps
/// them.
pub const WRECK_PLATES: RangeInclusive<u32> = 1..=8;

/// One hull a cluster planned, as a cell that owns it hands it over.
#[derive(Clone, Copy, Debug)]
pub struct HullSlot<'a> {
    /// The ship id the cell gives it.
    pub id: &'a str,
    /// The cluster's lattice node.
    pub node: [i32; 3],
    /// The hull's planned slot in its cluster.
    pub slot: usize,
    /// The cluster's anchor.
    pub anchor: Meters3,
    /// The environment at the cluster's drawn anchor.
    pub environment: Environment,
    /// The centres of the cluster's planetoids.
    pub planetoids: &'a [Meters3],
    /// Where the hull's root stands.
    pub position: Meters3,
    /// Its seeded yaw, in radians.
    pub yaw: f32,
    /// Its draw for a secondary civilization, in `[0, 1)`.
    pub lineage: f32,
}

/// One generated ship a cell owns, laid out before the cell checks where it
/// fits.
#[derive(Clone, Debug)]
pub struct PlannedShip {
    /// The generated design.
    pub design: ShipDesign,
    /// Its full orientation.
    pub rotation: Quat,
    /// The radius around its root that holds the hull.
    pub clearance: Meters,
    /// Whether it is intact or a derelict.
    pub condition: SectorShipConditionType,
    /// What its hold carries: nothing when intact, [`wreck_stock`] when a
    /// derelict.
    pub stock: ShipInventoryStock,
}

/// Lay out the ship `hull` stands for.
///
/// # Errors
///
/// Whatever [`CivilizationField::in_reach`] refuses at the anchor or the
/// hull; [`SectorFault::Generation`] on `role` when the civilization's
/// advancement builds no role, on `layout` with the failed request when the
/// layout fails, on `frame` when an intact hull stands at its planetoid's
/// centre, where it has no outward direction to stand level on, and on
/// `stock` when a wreck has no hull section left to hold a plate.
pub fn plan_ship(
    parts: &ShipPartSnapshot,
    civilizations: &CivilizationField,
    seed: u32,
    hull: HullSlot<'_>,
) -> Result<PlannedShip, SectorFault> {
    let refuse = |field: &'static str, value: String| SectorFault::Generation {
        id: hull.id.to_string(),
        field,
        value,
    };
    let key = |aspect: &[u8]| {
        Fnv32::new()
            .write(&seed.to_le_bytes())
            .write(aspect)
            .write(&hull.node[0].to_le_bytes())
            .write(&hull.node[1].to_le_bytes())
            .write(&hull.node[2].to_le_bytes())
            .write(&(hull.slot as u64).to_le_bytes())
            .finish()
    };
    let mut stream = SeedStream::new(key(b"sector_ship"));
    // A fixed number of draws, in a fixed order, whatever they decide.
    let civilization_draw = stream.unit();
    let role_draw = stream.unit();
    let turn = [stream.unit(), stream.unit(), stream.unit()];

    let primary = primary_civilization(civilizations, seed, hull.node, hull.anchor)?;
    let civilization = if hull.slot > 0 && hull.lineage < SECONDARY_CIVILIZATION_CHANCE {
        let others: Vec<(Civilization, f32)> = civilizations
            .in_reach(hull.position)?
            .into_iter()
            .filter(|reach| reach.civilization.id != primary.id)
            .map(|reach| (reach.civilization, reach.influence))
            .collect();
        if others.is_empty() {
            primary
        } else {
            pick(&others, civilization_draw)
        }
    } else {
        primary
    };

    let buildable = parts.eligible_roles(civilization.advancement);
    let weights = civilization.role_weights(hull.environment);
    let roles: Vec<(ShipRoleType, f32)> = ShipRoleType::ALL
        .into_iter()
        .zip(weights)
        .filter(|(role, weight)| buildable.contains(role) && *weight > 0.0)
        .collect();
    if roles.is_empty() {
        return Err(refuse(
            "role",
            format!(
                "{} builds no role at advancement {:.2}",
                civilization.id, civilization.advancement
            ),
        ));
    }
    let role = pick(&roles, role_draw);

    let request = ShipLayoutRequest {
        seed: key(b"sector_ship_hull"),
        civilization: civilization.id,
        role,
        advancement: civilization.advancement,
    };
    let (layout, condition) = match civilization.status {
        CivilizationStatusType::Living => (
            generate_ship(parts, request),
            SectorShipConditionType::Intact,
        ),
        CivilizationStatusType::Extinct => (
            generate_wreck(parts, request),
            SectorShipConditionType::Derelict,
        ),
    };
    let layout = layout.map_err(|failure| refuse("layout", failure.to_string()))?;

    let stock = match condition {
        SectorShipConditionType::Intact => ShipInventoryStock::default(),
        SectorShipConditionType::Derelict => {
            let draw = SeedStream::new(key(b"sector_ship_stock")).next_u32();
            wreck_stock(parts, &layout.design, draw).ok_or_else(|| {
                refuse(
                    "stock",
                    "the wreck has no hull section to hold a plate".to_string(),
                )
            })?
        }
    };

    let rotation = match condition {
        SectorShipConditionType::Intact => {
            let up = match nearest(hull.planetoids, hull.position) {
                Some(centre) => {
                    let outward = (hull.position.get() - centre.get()).normalize_or_zero();
                    if outward == Vec3::ZERO {
                        return Err(refuse(
                            "frame",
                            "the hull stands at its planetoid's centre".to_string(),
                        ));
                    }
                    outward
                }
                None => Vec3::Y,
            };
            Quat::from_rotation_arc(Vec3::Y, up) * Quat::from_rotation_y(hull.yaw)
        }
        SectorShipConditionType::Derelict => uniform_rotation(turn),
    };

    Ok(PlannedShip {
        design: layout.design,
        rotation,
        clearance: layout.clearance,
        condition,
        stock,
    })
}

/// The hull plates a wreck of `design` carries: a count in [`WRECK_PLATES`]
/// chosen by `draw`, cut to what its hull sections hold at
/// [`HULL_SECTION_CARGO_G`] each, the rule the spawn sizes its hold by.
///
/// `None` when the hull holds less than one plate: a wreck is a loot source,
/// so a design that cannot carry any fails its sector rather than spawning
/// empty.
///
/// # Panics
///
/// When a section of `design` names no part of `parts`: every generated
/// design is built from the snapshot it is checked against.
pub fn wreck_stock(
    parts: &ShipPartSnapshot,
    design: &ShipDesign,
    draw: u32,
) -> Option<ShipInventoryStock> {
    let hulls = design
        .sections
        .iter()
        .filter(|section| {
            let id = section.source.prototype_id();
            let part = parts
                .parts()
                .iter()
                .find(|part| part.id() == id)
                .unwrap_or_else(|| {
                    panic!("generated section '{}' names no part '{id}'", section.id)
                });
            matches!(part.config.kind, SectionKind::Hull(_))
        })
        .count() as u64;
    let room = hulls * u64::from(HULL_SECTION_CARGO_G) / u64::from(ItemType::HullPlate.mass_g());
    let (least, most) = (*WRECK_PLATES.start(), *WRECK_PLATES.end());
    let drawn = least + draw % (most - least + 1);
    let plates = u64::from(drawn).min(room) as u32;
    (plates > 0).then(|| ShipInventoryStock::new([(ItemType::HullPlate, plates)]))
}

/// The civilization a cluster's hulls come from first: one of those in reach
/// of its anchor, by selection weight, drawn from a stream keyed by the seed
/// and the cluster's node alone.
fn primary_civilization(
    civilizations: &CivilizationField,
    seed: u32,
    node: [i32; 3],
    anchor: Meters3,
) -> Result<Civilization, SectorFault> {
    let draw = SeedStream::new(
        Fnv32::new()
            .write(&seed.to_le_bytes())
            .write(b"cluster_civilization")
            .write(&node[0].to_le_bytes())
            .write(&node[1].to_le_bytes())
            .write(&node[2].to_le_bytes())
            .finish(),
    )
    .unit();
    let reach: Vec<(Civilization, f32)> = civilizations
        .in_reach(anchor)?
        .into_iter()
        .map(|reach| (reach.civilization, reach.selection_weight))
        .collect();
    Ok(pick(&reach, draw))
}

/// The centre of `planetoids` nearest to `position`, if there is one.
fn nearest(planetoids: &[Meters3], position: Meters3) -> Option<Meters3> {
    planetoids.iter().copied().min_by(|a, b| {
        a.distance(position)
            .get()
            .total_cmp(&b.distance(position).get())
    })
}

/// A rotation drawn uniformly from every orientation, from three draws in
/// `[0, 1)` (Shoemake's method).
fn uniform_rotation([u1, u2, u3]: [f32; 3]) -> Quat {
    let tau = std::f32::consts::TAU;
    let (low, high) = ((1.0 - u1).sqrt(), u1.sqrt());
    Quat::from_xyzw(
        low * (tau * u2).sin(),
        low * (tau * u2).cos(),
        high * (tau * u3).sin(),
        high * (tau * u3).cos(),
    )
    .normalize()
}
