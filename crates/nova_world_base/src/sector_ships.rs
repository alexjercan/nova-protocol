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
//! Every ship starts with goods and credits, each drawn from its own stream
//! keyed like the ship and scaled by its civilization's advancement. Its hold
//! carries [`ship_stock`]: a role's item mix toward a seeded share of the
//! hold its hull sections give, a wreck's toward half that share of what its
//! remaining hull holds, drawn apart from the intact ship's. Its balance
//! rises with advancement whatever its role or hold; a wreck keeps a seeded
//! share of an intact balance, at least one credit. Nothing keeps what was
//! taken: a retired cell comes back with its ships' stock and credits whole.
//!
//! FAIL LOUD. A layout or wreck that fails for the selected civilization and
//! role fails the sector; only a cell's spatial checks may skip a planned
//! hull.
//!
//! Every chance here is provisional until generated ships are reviewed in
//! travel.

use bevy::prelude::{Quat, Vec3};
use nova_events::prelude::{Meters, Meters3};
use nova_gameplay::prelude::{Fnv32, ItemType, SeedStream, ShipInventoryStock};
use nova_scenario::prelude::{ShipDesign, HULL_SECTION_CARGO_G};
use nova_ship::prelude::SectionKind;
use nova_world::prelude::*;

use crate::{
    civilizations::{
        AdvancementCurveType, Civilization, CivilizationField, CivilizationStatusType,
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

/// How far each waypoint of a generated patrol loop stands from its ship.
const PATROL_RADIUS: Meters = Meters(1_500.0);

/// How many waypoints a generated patrol loop tries, evenly around its ship.
const PATROL_WAYPOINTS: usize = 4;

/// The fewest waypoints that make a loop. A ship with fewer that fit holds
/// its spawn point.
const PATROL_WAYPOINTS_MIN: usize = 2;

/// The shortest and longest seeded hold at a waypoint, in seconds.
const PATROL_STOP: (f32, f32) = (30.0, 60.0);

/// How far a generated crew chases from its patrol centre.
const CREW_LEASH: Meters = Meters(5_000.0);

/// The share of its hold an intact ship's stock aims for, as a `(least,
/// most)` band at advancement 0 and at advancement 1, interpolated linearly
/// between. A target, not a fill: item masses round it down.
const STOCK_SHARE: [(f32, f32); 2] = [(0.05, 0.15), (0.20, 0.40)];

/// What a wreck's stock share is of an intact ship's band.
const WRECK_STOCK_SCALE: f32 = 0.5;

/// The most distinct items one hold draws.
const STOCK_STACKS: usize = 3;

/// An intact ship's credit balance, as a `(least, most)` band at advancement
/// 0 and at advancement 1, interpolated linearly between.
const CREDITS: [(f32, f32); 2] = [(50.0, 200.0), (500.0, 2_000.0)];

/// The share of an intact balance a wreck keeps, as a `(least, most)` band.
const WRECK_CREDIT_SHARE: (f32, f32) = (0.10, 0.25);

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
    /// The civilization that built it.
    pub civilization: CivilizationId,
    /// The role it was laid out for. A wreck keeps the role it was built for.
    pub role: ShipRoleType,
    /// What its hold carries, see [`ship_stock`].
    pub stock: ShipInventoryStock,
    /// Its credit balance.
    pub credits: u32,
    /// Who flies it: `Some` exactly when it is intact, with no patrol until
    /// the cell gives it one with `plan_patrol` after it places every body.
    pub crew: Option<SectorShipCrew>,
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
/// `stock` when its hold cannot fit one unit of its role's lightest item.
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
    let key = |aspect: &[u8]| ship_key(seed, aspect, hull.node, hull.slot);
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

    let stock_key = match condition {
        SectorShipConditionType::Intact => key(b"sector_ship_stock"),
        SectorShipConditionType::Derelict => key(b"sector_ship_wreck_stock"),
    };
    let stock = ship_stock(
        parts,
        &layout.design,
        role,
        condition,
        civilization.advancement,
        stock_key,
    )
    .ok_or_else(|| {
        refuse(
            "stock",
            format!(
                "the {} hold cannot fit one unit of its lightest item",
                role.label()
            ),
        )
    })?;
    let credits = ship_credits(
        civilization.advancement,
        condition,
        key(b"sector_ship_credits"),
    );

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

    let crew = (condition == SectorShipConditionType::Intact).then(|| SectorShipCrew {
        allegiance: civilization.allegiance,
        patrol: Vec::new(),
        stops: Vec::new(),
        leash: CREW_LEASH,
    });

    Ok(PlannedShip {
        design: layout.design,
        rotation,
        clearance: layout.clearance,
        condition,
        civilization: civilization.id,
        role,
        stock,
        credits,
        crew,
    })
}

/// The seed of one `aspect` of the hull in `slot` of the cluster at `node`.
fn ship_key(seed: u32, aspect: &[u8], node: [i32; 3], slot: usize) -> u32 {
    Fnv32::new()
        .write(&seed.to_le_bytes())
        .write(aspect)
        .write(&node[0].to_le_bytes())
        .write(&node[1].to_le_bytes())
        .write(&node[2].to_le_bytes())
        .write(&(slot as u64).to_le_bytes())
        .finish()
}

/// The patrol loop and its holds for the placed ship in `slot` of the cluster
/// at `node`, standing at `position` turned by `rotation`.
///
/// [`PATROL_WAYPOINTS`] waypoints at [`PATROL_RADIUS`], evenly around the
/// ship in its own level plane from a seeded phase, each with a seeded hold
/// in [`PATROL_STOP`]. A waypoint is kept only where `fits` accepts it: the
/// cell checks the ship's whole clearance there against every body it
/// placed. Fewer than [`PATROL_WAYPOINTS_MIN`] that fit give an empty loop.
/// A fixed number of draws, so a refused waypoint never shifts a later hold.
pub(crate) fn plan_patrol(
    seed: u32,
    node: [i32; 3],
    slot: usize,
    position: Meters3,
    rotation: Quat,
    fits: impl Fn(Meters3) -> bool,
) -> (Vec<Meters3>, Vec<f32>) {
    let mut stream = SeedStream::new(ship_key(seed, b"sector_ship_patrol", node, slot));
    let phase = stream.unit() * std::f32::consts::TAU;
    let holds: [f32; PATROL_WAYPOINTS] =
        std::array::from_fn(|_| PATROL_STOP.0 + (PATROL_STOP.1 - PATROL_STOP.0) * stream.unit());
    let step = std::f32::consts::TAU / PATROL_WAYPOINTS as f32;
    let (patrol, stops): (Vec<Meters3>, Vec<f32>) = holds
        .into_iter()
        .enumerate()
        .map(|(index, hold)| {
            let angle = phase + step * index as f32;
            let offset = rotation * Vec3::new(angle.cos(), 0.0, angle.sin());
            (Meters3(position.get() + offset * PATROL_RADIUS.get()), hold)
        })
        .filter(|(waypoint, _)| fits(*waypoint))
        .unzip();
    if patrol.len() < PATROL_WAYPOINTS_MIN {
        return (Vec::new(), Vec::new());
    }
    (patrol, stops)
}

/// The stock a ship of `role` and `condition`, laid out as `design`, starts
/// with at `advancement`, drawn from a stream seeded by `draw`.
///
/// The hold is what `design`'s hull sections give at
/// [`HULL_SECTION_CARGO_G`] each, the rule the spawn sizes it by, so a
/// wreck's is what its remaining hull holds. The stock aims for a share of it
/// drawn from [`STOCK_SHARE`] at `advancement`, scaled by
/// [`WRECK_STOCK_SCALE`] for a derelict. Up to [`STOCK_STACKS`] distinct items
/// of the role's mix are drawn by weight and split the target by seeded
/// shares; each takes the whole units its share holds, so a heavy item may
/// take none. Stock that rounds to nothing is one unit of the role's lightest
/// item, so every ship carries goods. Never heavier than the hold.
///
/// `None` when the hold cannot fit one unit of the role's lightest item: a
/// ship is a goods source, so a design that cannot carry any fails its
/// sector rather than spawning empty.
///
/// # Panics
///
/// When a section of `design` names no part of `parts`: every generated
/// design is built from the snapshot it is checked against.
pub fn ship_stock(
    parts: &ShipPartSnapshot,
    design: &ShipDesign,
    role: ShipRoleType,
    condition: SectorShipConditionType,
    advancement: f32,
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
    let hold = hulls * u64::from(HULL_SECTION_CARGO_G);
    // Ammunition of every kind on an armed ship, whichever its weapons fire.
    let mix: &[(ItemType, f32)] = match role {
        ShipRoleType::Industrial => &[
            (ItemType::StoneOre, 2.0),
            (ItemType::IronOre, 2.0),
            (ItemType::WaterIce, 2.0),
            (ItemType::CarbonOre, 2.0),
            (ItemType::SalvagedParts, 3.0),
            (ItemType::Rations, 1.0),
        ],
        ShipRoleType::Civilian => &[
            (ItemType::Rations, 5.0),
            (ItemType::SalvagedParts, 3.0),
            (ItemType::WaterIce, 1.0),
            (ItemType::HullPlate, 1.0),
        ],
        ShipRoleType::Scavenger => &[
            (ItemType::PdcRound, 2.0),
            (ItemType::RailSlug, 1.0),
            (ItemType::Torpedo, 1.0),
            (ItemType::HullPlate, 3.0),
            (ItemType::SalvagedParts, 3.0),
            (ItemType::Rations, 1.0),
        ],
        ShipRoleType::Armored => &[
            (ItemType::PdcRound, 3.0),
            (ItemType::RailSlug, 2.0),
            (ItemType::Torpedo, 1.0),
            (ItemType::HullPlate, 3.0),
            (ItemType::SalvagedParts, 2.0),
            (ItemType::Rations, 1.0),
        ],
    };
    let lightest = mix
        .iter()
        .map(|(item, _)| *item)
        .min_by_key(|item| item.mass_g())
        .expect("every role mixes at least one item");
    if hold < u64::from(lightest.mass_g()) {
        return None;
    }

    let mut stream = SeedStream::new(draw);
    let advancement = advancement.clamp(0.0, 1.0);
    let [(low_least, low_most), (high_least, high_most)] = STOCK_SHARE;
    let least = low_least + (high_least - low_least) * advancement;
    let most = low_most + (high_most - low_most) * advancement;
    let mut share = least + (most - least) * stream.unit();
    if condition == SectorShipConditionType::Derelict {
        share *= WRECK_STOCK_SCALE;
    }
    let target = (hold as f64 * f64::from(share)) as u64;

    let kinds = (1 + (stream.unit() * STOCK_STACKS as f32) as usize).min(STOCK_STACKS);
    let mut left = mix.to_vec();
    let mut drawn = Vec::with_capacity(kinds);
    for _ in 0..kinds {
        let item = pick(&left, stream.unit());
        left.retain(|(other, _)| *other != item);
        drawn.push((item, 0.5 + stream.unit()));
    }
    let total: f32 = drawn.iter().map(|(_, weight)| weight).sum();
    let mut stacks: Vec<(ItemType, u32)> = drawn
        .into_iter()
        .filter_map(|(item, weight)| {
            let portion = (target as f64 * f64::from(weight / total)) as u64;
            let units = portion / u64::from(item.mass_g());
            let units = u32::try_from(units).expect("a generated hold's units fit a u32");
            (units > 0).then_some((item, units))
        })
        .collect();
    if stacks.is_empty() {
        stacks.push((lightest, 1));
    }
    Some(ShipInventoryStock::new(stacks))
}

/// The credit balance a ship of `condition` starts with at `advancement`,
/// drawn from a stream seeded by `draw`, whatever its role or hold.
///
/// An intact ship draws a balance from [`CREDITS`] at `advancement`. A wreck
/// draws one the same way and keeps a share of it from
/// [`WRECK_CREDIT_SHARE`], at least one credit.
fn ship_credits(advancement: f32, condition: SectorShipConditionType, draw: u32) -> u32 {
    let mut stream = SeedStream::new(draw);
    let advancement = advancement.clamp(0.0, 1.0);
    let [(low_least, low_most), (high_least, high_most)] = CREDITS;
    let least = low_least + (high_least - low_least) * advancement;
    let most = low_most + (high_most - low_most) * advancement;
    let budget = least + (most - least) * stream.unit();
    match condition {
        SectorShipConditionType::Intact => budget.round() as u32,
        SectorShipConditionType::Derelict => {
            let (low, high) = WRECK_CREDIT_SHARE;
            let kept = low + (high - low) * stream.unit();
            ((budget * kept).round() as u32).max(1)
        }
    }
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
