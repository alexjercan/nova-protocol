//! The open world's ship layouts: one generated [`ShipDesign`] from a
//! [`ShipPartSnapshot`], a role, an advancement and a stable hull seed.
//!
//! SHAPE FIRST. A layout plans a symmetric spine of structural cubes along the
//! ship's length, from a one-cell nose to a stern that matches its drive bank,
//! and varies the cross-section station by station. The spine mixes up to
//! [`STRUCTURAL_PALETTE`] structural cube types in mirror image. Then it mounts
//! the drive bank, copies of one drive around a centred single or in mirrored
//! pairs, the flight computers, the role's weapons or side cargo intakes, and
//! the docking ports: a mirrored pair out of the hull's sides where one fits,
//! else one port alone.
//! Every part stands on the cell grid through [`oriented_part`], so a mod part
//! joins by its type, footprint and sockets.
//!
//! MASS. A section's mass is its collider volume at density 1, so a structural
//! cube weighs one cell. The drive bank grows until it pushes
//! [`MIN_THRUST_PER_CELL`] per cell of the whole ship, and the ship carries one
//! flight computer per [`CELLS_PER_CONTROLLER`] cells.
//!
//! A layout draws a preferred source per hull and keeps civilian, industrial
//! and armored ships to it where that source has a usable part; scavengers mix
//! sources. Each source weighs the same, whatever its part count.
//!
//! Every layout is checked after it is built: unique section ids, usable and
//! eligible prototypes, the role's families, mated contacts, mirror symmetry
//! but for a lone docking port, intakes and docking ports that open sideways
//! with structural cubes across their backs, one connected socket graph, clear
//! exit and docking lanes, the advancement's size ceiling, the thrust floor,
//! the flight computer count and one off-centre outer cube a wreck could lose
//! alone, so every intact ship can be ruined. A seeded layout that fails
//! is redrawn a fixed number of times, then the request fails with its seed,
//! civilization, role and the last failed constraint. Nothing substitutes an
//! authored hull. Every layout wears the derived skin in its role's base
//! style, [`role_style_id`](crate::role_style_id).
//!
//! WRECKS. [`generate_wreck`] ruins the intact ship of the same request: it
//! keeps the role, style, source, advancement, fittings and cell bounds, and
//! omits seeded breaches of structural cubes, each growing inward from the
//! outer hull, while the rest stays one connected ship. The cubes across a docking
//! port's back never go, so a wreck keeps a backed port. A wreck digs toward a
//! fifth of its cubes off its mirror image, so it cannot read as a sparse
//! intact hull. A thin hull that cannot give that many settles for a floor:
//! as many outer cubes as it can lose one at a time, and no more than one
//! deterministic pass can lose together, down to one. When every seeded plan
//! falls short of that floor, the wreck is that pass. A pass that fails a
//! wreck constraint fails the request, and so fails the sector that planned
//! it; nothing rerolls or skips it.
//! The generator has no weathered or damaged part to add; holes are its only
//! ruin cue.
//!
//! PURE. The open world lays out every ship it spawns through
//! [`generate_ship`] and [`generate_wreck`], and every size, share and weight
//! here is provisional until generated ships are reviewed.

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt,
};

use bevy::prelude::*;
use nova_events::prelude::Meters;
use nova_gameplay::prelude::{Fnv32, SeedStream};
use nova_scenario::prelude::{
    SectionSource, ShipDesign, ShipPresentationConfig, SpaceshipSectionConfig,
};
use nova_ship::prelude::{
    blocked_exits, cube_rotations, derive_link_point_graph, exit_normal, mirror_face,
    mirror_rotation, oriented_part, placement_blocks_an_exit, read_structure, ship_exits,
    LinkPointGraphError, OrientedPart, PlacedPart, PlacedSectionLinkPoints, SectionCollider,
    SectionConfig, SectionFootprint, SectionKind, GRID_EPSILON,
};
use nova_world::prelude::{CivilizationId, ShipRoleType, SECTOR_SHIP_CLEARANCE_MAX};

use crate::{
    role_style_id,
    ship_parts::{ShipPart, ShipPartFamilyType, ShipPartSnapshot},
};

/// How many seeded layouts a request draws before it fails.
const MAX_LAYOUT_ATTEMPTS: u32 = 8;

/// The largest hull clearance radius at advancement 0: room for the minimum
/// flyable civilian and industrial ship.
const MIN_HULL_CLEARANCE: Meters = Meters(80.0);

/// The largest hull clearance radius at advancement 1: the most a sector
/// manifest accepts.
const MAX_HULL_CLEARANCE: Meters = SECTOR_SHIP_CLEARANCE_MAX;

/// Cells a spine plan keeps free on every side for the fittings mounted on
/// it, so a planned spine rarely fails the size ceiling once fitted.
const FITTING_REACH: i32 = 2;

/// The least drive thrust a ship carries per cell of its mass.
const MIN_THRUST_PER_CELL: f32 = 0.06;

/// The most cells of mass one flight computer turns.
const CELLS_PER_CONTROLLER: f32 = 50.0;

/// The most structural cube types one hull mixes.
const STRUCTURAL_PALETTE: usize = 3;

/// How far below a request's advancement a drive or weapon may score and still
/// be drawn before the weaker parts of its source.
const TIER_BAND: f32 = 0.5;

/// The fewest stations a spine has: a nose, a body and a stern.
const MIN_STATIONS: i32 = 3;

/// The share of an intact hull's structural cubes its wreck digs toward
/// omitting off its mirror image.
const WRECK_OMISSION_SHARE: f32 = 0.2;

/// The fewest structural cubes a wreck digs toward omitting off its mirror
/// image: fewer read as a sparse hull, not a ruin, on any hull that can lose
/// them.
const MIN_WRECK_OMISSIONS: usize = 3;

/// How many breaches a wreck's omissions are split into, so the ruin is
/// several holes rather than one bite.
const WRECK_BREACHES: usize = 3;

/// How many seeded omission plans a wreck request draws before it fails.
const MAX_WRECK_ATTEMPTS: u32 = 8;

/// The six cardinal steps, in [`CELL_FACES`](nova_ship::prelude::CELL_FACES) order.
const STEPS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];

/// The face indices a generated ship reads by name, in [`CELL_FACES`](nova_ship::prelude::CELL_FACES) order.
const STARBOARD: usize = 0;
const PORT: usize = 1;
const DORSAL: usize = 2;
const VENTRAL: usize = 3;
const AFT: usize = 4;
const FORWARD: usize = 5;

/// What a generated ship is asked to be.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShipLayoutRequest {
    /// The stable hull seed. Every draw of the layout comes from it.
    pub seed: u32,
    /// The civilization that builds the ship. Only a failure reads it.
    pub civilization: CivilizationId,
    /// The role the ship flies.
    pub role: ShipRoleType,
    /// The civilization's advancement in `[0, 1]`: the part and size ceiling.
    pub advancement: f32,
}

/// How a generated ship's drive bank stands behind its stern. A bank is
/// copies of one drive, in as many columns and rows as its thrust floor
/// needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShipDriveLayoutType {
    /// A column of drives on the centreline, with any mirrored pairs beside
    /// it.
    CenteredSingle,
    /// Mirrored pairs only, with the centre column open.
    MirroredPair,
}

/// One generated ship.
#[derive(Clone, Debug)]
pub struct ShipLayout {
    /// The design. Every section refers to a snapshot part by prototype id,
    /// and each section id names its planned slot.
    pub design: ShipDesign,
    /// How it carries its drive.
    pub drive: ShipDriveLayoutType,
    /// The source the hull preferred.
    pub source: String,
    /// The hull clearance radius: the distance from the design origin to the
    /// farthest corner of its cell bounds, so a sphere of it around the ship
    /// root holds the hull in any orientation.
    pub clearance: Meters,
    /// The attempt that built the intact design, from 0. A wreck keeps its
    /// intact ship's.
    pub attempt: u32,
}

/// The constraint a generated ship failed.
#[derive(Clone, Debug, PartialEq)]
pub enum ShipLayoutConstraintType {
    /// The snapshot has no part of a family the role needs at this
    /// advancement.
    NoEligiblePart(ShipPartFamilyType),
    /// Eligible parts of a family exist, and none can stand in the layout.
    Unplaced(ShipPartFamilyType),
    /// The ship carries no part of a family its role needs.
    MissingPart(ShipPartFamilyType),
    /// The ship carries a part of a family its role must not.
    ForbiddenPart {
        /// The section id.
        section: String,
        /// Its family.
        family: ShipPartFamilyType,
    },
    /// Two sections share an id.
    DuplicateSectionId(String),
    /// A section refers to no usable part of the snapshot.
    UnknownPrototype {
        /// The section id.
        section: String,
        /// The prototype id it names.
        prototype: String,
    },
    /// A section's part needs more advancement than the request has.
    AboveAdvancement(String),
    /// Two sections fill one cell.
    Overlap {
        /// The first section id, in list order.
        first: String,
        /// The second.
        second: String,
    },
    /// A socket presses into a face that has none, or a part fires into a
    /// neighbour.
    UnmatedContact {
        /// The first section id, in list order.
        first: String,
        /// The second.
        second: String,
    },
    /// A section has no mirror image across the centreline.
    Asymmetric(String),
    /// The sockets do not join the sections into one ship.
    Disconnected {
        /// How many separate pieces they form.
        components: usize,
    },
    /// The sockets are invalid or ambiguous, see [`LinkPointGraphError`].
    InvalidSockets,
    /// Something stands in, or asks for cladding across, a section's exit
    /// lane.
    BlockedExit(String),
    /// A cargo intake does not open sideways with a structural cube behind
    /// every cell of its back.
    UnbackedIntake(String),
    /// A docking port does not open sideways with a structural cube behind
    /// every cell of its back.
    UnbackedDock(String),
    /// The drives push less than the ship's mass needs.
    Underpowered {
        /// The drives' summed thrust.
        thrust: f32,
        /// The thrust the ship's mass needs.
        needed: f32,
    },
    /// The ship carries fewer flight computers than its mass needs.
    Undercontrolled {
        /// The flight computers it carries.
        controllers: usize,
        /// How many its mass needs.
        needed: usize,
    },
    /// The hull has no off-centre outer structural cube, outside the cubes
    /// across a docking port's back, that its wreck could omit alone and keep
    /// its cell bounds and one socket graph. Every generated ship can be
    /// ruined, so an intact ship and its wreck share one layout.
    Unruinable,
    /// A wreck omits too few hull sections off its mirror image to read as a
    /// ruin: fewer than its hull's floor, see [`generate_wreck`].
    Unruined {
        /// The omitted hull sections whose mirror image remains.
        omitted: usize,
        /// How many a visible ruin of this hull needs.
        needed: usize,
    },
    /// The hull clearance radius exceeds the advancement's ceiling.
    TooLarge {
        /// The hull's clearance radius.
        clearance: Meters,
        /// The ceiling.
        ceiling: Meters,
    },
}

impl fmt::Display for ShipLayoutConstraintType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoEligiblePart(family) => write!(f, "no eligible {family} part"),
            Self::Unplaced(family) => write!(f, "no eligible {family} part fits the layout"),
            Self::MissingPart(family) => write!(f, "the ship carries no {family}"),
            Self::ForbiddenPart { section, family } => {
                write!(
                    f,
                    "section '{section}' is a {family}, which the role must not carry"
                )
            }
            Self::DuplicateSectionId(id) => write!(f, "section id '{id}' is used twice"),
            Self::UnknownPrototype { section, prototype } => write!(
                f,
                "section '{section}' names '{prototype}', which is not a usable part"
            ),
            Self::AboveAdvancement(id) => {
                write!(
                    f,
                    "section '{id}' needs more advancement than the civilization has"
                )
            }
            Self::Overlap { first, second } => {
                write!(f, "sections '{first}' and '{second}' fill one cell")
            }
            Self::UnmatedContact { first, second } => write!(
                f,
                "sections '{first}' and '{second}' touch where a socket meets no socket or a \
                 part fires"
            ),
            Self::Asymmetric(id) => {
                write!(
                    f,
                    "section '{id}' has no mirror image across the centreline"
                )
            }
            Self::Disconnected { components } => {
                write!(f, "the sockets join the ship into {components} pieces")
            }
            Self::InvalidSockets => write!(f, "the socket graph is invalid or ambiguous"),
            Self::BlockedExit(id) => write!(f, "section '{id}' cannot fire down a clear lane"),
            Self::UnbackedIntake(id) => write!(
                f,
                "intake '{id}' does not open sideways with structural cubes across its back"
            ),
            Self::UnbackedDock(id) => write!(
                f,
                "docking port '{id}' does not open sideways with structural cubes across its back"
            ),
            Self::Underpowered { thrust, needed } => write!(
                f,
                "the drives push {thrust:.2} where the ship's mass needs {needed:.2}"
            ),
            Self::Undercontrolled {
                controllers,
                needed,
            } => write!(
                f,
                "the ship carries {controllers} flight computers where its mass needs {needed}"
            ),
            Self::Unruinable => write!(
                f,
                "the hull has no off-centre outer cube its wreck could omit alone"
            ),
            Self::Unruined { omitted, needed } => write!(
                f,
                "the wreck omits {omitted} hull sections off its mirror image where a visible \
                 ruin needs {needed}"
            ),
            Self::TooLarge { clearance, ceiling } => write!(
                f,
                "the hull clearance radius is {:.0} m, above the {:.0} m ceiling",
                clearance.0, ceiling.0
            ),
        }
    }
}

/// A request no seeded layout satisfied.
#[derive(Clone, Debug, PartialEq)]
pub struct ShipLayoutFailure {
    /// The request.
    pub request: ShipLayoutRequest,
    /// How many layouts were drawn: 0 when the snapshot cannot serve the
    /// role at all.
    pub attempts: u32,
    /// The constraint the last layout failed.
    pub constraint: ShipLayoutConstraintType,
}

impl fmt::Display for ShipLayoutFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ShipLayoutRequest {
            seed,
            civilization,
            role,
            advancement,
        } = self.request;
        write!(
            f,
            "ship seed {seed} of {civilization} ({}, advancement {advancement:.2}) failed after \
             {} layout attempts: {}",
            role.label(),
            self.attempts,
            self.constraint
        )
    }
}

/// Generate the ship `request` asks for from `snapshot`.
///
/// The same request and the same effective catalog give the same design,
/// whatever order its packs were given in.
pub fn generate_ship(
    snapshot: &ShipPartSnapshot,
    request: ShipLayoutRequest,
) -> Result<ShipLayout, ShipLayoutFailure> {
    let fail = |attempts, constraint| ShipLayoutFailure {
        request,
        attempts,
        constraint,
    };
    for family in ShipPartFamilyType::ALL {
        if family.required_by(request.role)
            && snapshot
                .eligible(request.role, family, request.advancement)
                .is_empty()
        {
            return Err(fail(0, ShipLayoutConstraintType::NoEligiblePart(family)));
        }
    }

    let mut last = None;
    for attempt in 0..MAX_LAYOUT_ATTEMPTS {
        let built = Draw::new(snapshot, request, attempt)
            .layout()
            .and_then(|layout| check(snapshot, request, &layout.design).map(|()| layout));
        match built {
            Ok(layout) => return Ok(layout),
            Err(constraint) => last = Some(constraint),
        }
    }
    Err(fail(
        MAX_LAYOUT_ATTEMPTS,
        last.expect("at least one layout attempt runs"),
    ))
}

/// Generate the wreck of the ship `request` asks for from `snapshot`: the
/// intact design [`generate_ship`] lays out for the same request, with seeded
/// breaches through its structural cubes.
///
/// Every omission plan digs toward a fifth of the hull's structural cubes
/// (at least [`MIN_WRECK_OMISSIONS`]) off its mirror image, and the first
/// that gets there is the wreck. No plan omits a cube across a docking port's
/// back. When none does, the plan with the most wins if it reaches the hull's
/// floor, see [`wreck_floor`] and `cumulative_ruin`. When no seeded plan
/// reaches the floor, the wreck is the one plan `cumulative_ruin` proves the
/// floor with.
///
/// Fails as the intact ship fails, or with the constraint that proof plan
/// fails.
pub fn generate_wreck(
    snapshot: &ShipPartSnapshot,
    request: ShipLayoutRequest,
) -> Result<ShipLayout, ShipLayoutFailure> {
    let intact = generate_ship(snapshot, request)?;
    let (cells, cubes) = filled_cells(snapshot, &intact.design);
    let target = wreck_omissions(cubes.len());
    let backing = dock_backing(snapshot, &intact.design);
    let cubes: BTreeSet<[i32; 3]> = cubes.difference(&backing).copied().collect();
    let witness = cumulative_ruin(&cells, &cubes, target);
    let floor = wreck_floor(&cells, &cubes, target).min(witness.len());
    let mut best: Option<(ShipDesign, usize)> = None;
    for attempt in 0..MAX_WRECK_ATTEMPTS {
        let (design, omitted) = ruin(&cells, &cubes, request, &intact.design, attempt, target);
        if check_wreck(snapshot, request, &intact.design, &design, floor).is_err() {
            continue;
        }
        if omitted >= target {
            return Ok(ShipLayout { design, ..intact });
        }
        if best.as_ref().is_none_or(|(_, most)| omitted > *most) {
            best = Some((design, omitted));
        }
    }
    if let Some((design, _)) = best {
        return Ok(ShipLayout { design, ..intact });
    }
    let design = omit(&intact.design, &witness);
    check_wreck(snapshot, request, &intact.design, &design, floor).map_err(|constraint| {
        ShipLayoutFailure {
            request,
            attempts: MAX_WRECK_ATTEMPTS + 1,
            constraint,
        }
    })?;
    Ok(ShipLayout { design, ..intact })
}

/// How many structural cubes off its mirror image a wreck of a hull with
/// `cubes` of them digs toward omitting.
fn wreck_omissions(cubes: usize) -> usize {
    MIN_WRECK_OMISSIONS.max((cubes as f32 * WRECK_OMISSION_SHARE).round() as usize)
}

/// Every cell `design` fills, with its section index and the faces it offers
/// a socket on, and the cell of every structural cube.
fn filled_cells(
    snapshot: &ShipPartSnapshot,
    design: &ShipDesign,
) -> (HashMap<IVec3, (usize, [bool; 6])>, BTreeSet<[i32; 3]>) {
    let parts: HashMap<&str, &ShipPart> = snapshot
        .parts()
        .iter()
        .map(|part| (part.id(), part))
        .collect();
    let mut cells = HashMap::new();
    let mut cubes = BTreeSet::new();
    for (index, section) in design.sections.iter().enumerate() {
        let part = parts[prototype_of(section).expect("a generated section names a prototype")];
        let oriented = oriented_part(&part.config, section.rotation)
            .expect("a snapshot part at a generated rotation stands on the grid");
        let anchor = (section.position - oriented.origin).round().as_ivec3();
        for cell in &oriented.cells {
            cells.insert(anchor + cell.cell.as_ivec3(), (index, cell.faces));
        }
        if is_structural_cube(part) {
            cubes.insert(anchor.to_array());
        }
    }
    (cells, cubes)
}

/// The cells of the structural cubes across the back of every docking port
/// of `design`.
fn dock_backing(snapshot: &ShipPartSnapshot, design: &ShipDesign) -> BTreeSet<[i32; 3]> {
    let parts: HashMap<&str, &ShipPart> = snapshot
        .parts()
        .iter()
        .map(|part| (part.id(), part))
        .collect();
    design
        .sections
        .iter()
        .map(|section| {
            let part = parts[prototype_of(section).expect("a generated section names a prototype")];
            (section, part)
        })
        .filter(|(_, part)| part.family == ShipPartFamilyType::Docking)
        .flat_map(|(section, part)| side_back(part, section).unwrap_or_default())
        .map(|cell| cell.to_array())
        .collect()
}

/// The minimum and maximum cells of `cells`.
fn cell_bounds(cells: &HashMap<IVec3, (usize, [bool; 6])>) -> (IVec3, IVec3) {
    cells
        .keys()
        .fold((IVec3::MAX, IVec3::MIN), |(low, high), cell| {
            (low.min(*cell), high.max(*cell))
        })
}

/// How many mirror pairs of off-centre outer cubes of the hull filling
/// `cells` have one cube that can go alone with the cell bounds and one socket
/// graph kept, capped at `target`. A wreck's floor is this count or
/// [`cumulative_ruin`]'s, whichever is fewer. A hull with at least `target`
/// such pairs that one pass can lose together keeps its full target, so it
/// cannot settle for a trivial breach. A hull with none is
/// [`ShipLayoutConstraintType::Unruinable`], so a generated ship's floor is
/// at least one.
fn wreck_floor(
    cells: &HashMap<IVec3, (usize, [bool; 6])>,
    cubes: &BTreeSet<[i32; 3]>,
    target: usize,
) -> usize {
    let intact_bounds = cell_bounds(cells);
    let mut open = cells.clone();
    let mut pairs = BTreeSet::new();
    for cube in cubes {
        if pairs.len() >= target {
            break;
        }
        let cell = IVec3::from_array(*cube);
        let pair = [cell.x.abs(), cell.y, cell.z];
        let outer = STEPS
            .iter()
            .any(|step| !cells.contains_key(&(cell + *step)));
        if cell.x == 0 || !outer || pairs.contains(&pair) {
            continue;
        }
        let held = open.remove(&cell).expect("a cube is filled");
        if cell_bounds(&open) == intact_bounds && joined(&open) {
            pairs.insert(pair);
        }
        open.insert(cell, held);
    }
    pairs.len().min(target)
}

/// The deterministic omission plan that proves a floor for the wreck of the
/// hull filling `cells`: one pass over `cubes` in order, omitting each
/// off-centre cube that is outer in the hull left so far, whose mirror image
/// it has not omitted, and whose omission keeps the cell bounds and one
/// socket graph. It stops at `target`. Returns the omitted cubes, each off
/// its mirror image.
///
/// [`wreck_floor`] counts cubes that can each go alone; together, an earlier
/// omission can make a later cube hold the bounds or the socket graph, so no
/// seeded plan may reach that count. This plan's count caps the floor, so the
/// plan itself always reaches it. An [`ShipLayoutConstraintType::Unruinable`]
/// hull is refused first, so it omits at least one cube.
fn cumulative_ruin(
    cells: &HashMap<IVec3, (usize, [bool; 6])>,
    cubes: &BTreeSet<[i32; 3]>,
    target: usize,
) -> BTreeSet<[i32; 3]> {
    let intact_bounds = cell_bounds(cells);
    let mut open = cells.clone();
    let mut omitted = BTreeSet::new();
    for cube in cubes {
        if omitted.len() >= target {
            break;
        }
        let cell = IVec3::from_array(*cube);
        let image = [-cell.x, cell.y, cell.z];
        let outer = STEPS.iter().any(|step| !open.contains_key(&(cell + *step)));
        if cell.x == 0 || !outer || omitted.contains(&image) {
            continue;
        }
        let held = open.remove(&cell).expect("a cube is filled");
        if cell_bounds(&open) == intact_bounds && joined(&open) {
            omitted.insert(*cube);
        } else {
            open.insert(cell, held);
        }
    }
    omitted
}

/// One seeded omission plan for the wreck of the intact design filling
/// `cells`, digging toward `target` omissions off its mirror image. Returns
/// the ruined design and how many omitted cubes' mirror images remain, fewer
/// than `target` when no cube is left to try.
///
/// Each breach starts at a structural cube on the outer hull and grows
/// through the cubes beside it, so a hole digs past the surface. A cube stays
/// when omitting it would split the ship or shrink its cell bounds.
fn ruin(
    cells: &HashMap<IVec3, (usize, [bool; 6])>,
    cubes: &BTreeSet<[i32; 3]>,
    request: ShipLayoutRequest,
    intact: &ShipDesign,
    attempt: u32,
    target: usize,
) -> (ShipDesign, usize) {
    let intact_bounds = cell_bounds(cells);
    let breach_size = target.div_ceil(WRECK_BREACHES);
    let mirror = |cell: IVec3| IVec3::new(-cell.x, cell.y, cell.z);

    let mut stream = stream(request, attempt, b"wreck");
    let mut open = cells.clone();
    let mut omitted = BTreeSet::new();
    let mut tried = BTreeSet::new();
    let mut breach = Vec::new();
    let mut off_mirror = 0;
    while off_mirror < target {
        let beside = |cell: &[i32; 3], of: &dyn Fn(IVec3) -> bool| {
            STEPS
                .iter()
                .any(|step| of(IVec3::from_array(*cell) + *step))
        };
        // A cube whose twin is already omitted would only make the pair of
        // holes symmetric again.
        let untried = cubes.iter().filter(|cell| {
            let cell = IVec3::from_array(**cell);
            open.contains_key(&cell)
                && !tried.contains(&cell.to_array())
                && !omitted.contains(&mirror(cell).to_array())
        });
        let grow: Vec<&[i32; 3]> = if breach.len() < breach_size {
            untried
                .clone()
                .filter(|cell| beside(cell, &|other| breach.contains(&other.to_array())))
                .collect()
        } else {
            Vec::new()
        };
        let pool = if grow.is_empty() {
            breach.clear();
            untried
                .filter(|cell| beside(cell, &|other| !open.contains_key(&other)))
                .collect()
        } else {
            grow
        };
        let Some(pick) = pool
            .get(((stream.unit() * pool.len() as f32) as usize).min(pool.len().saturating_sub(1)))
            .map(|cell| **cell)
        else {
            break;
        };
        tried.insert(pick);
        let cell = IVec3::from_array(pick);
        let held = open.remove(&cell).expect("a picked cube is filled");
        if cell_bounds(&open) != intact_bounds || !joined(&open) {
            open.insert(cell, held);
            continue;
        }
        omitted.insert(pick);
        breach.push(pick);
        let image = mirror(cell);
        off_mirror += usize::from(open.contains_key(&image));
    }

    (omit(intact, &omitted), off_mirror)
}

/// `intact` without the structural cubes at `omitted`.
fn omit(intact: &ShipDesign, omitted: &BTreeSet<[i32; 3]>) -> ShipDesign {
    let gone: HashSet<usize> = omitted
        .iter()
        .map(|cell| {
            intact
                .sections
                .iter()
                .position(|section| section.position.round().as_ivec3() == IVec3::from_array(*cell))
                .expect("an omitted cube is a section")
        })
        .collect();
    ShipDesign {
        sections: intact
            .sections
            .iter()
            .enumerate()
            .filter(|(index, _)| !gone.contains(index))
            .map(|(_, section)| section.clone())
            .collect(),
        ..intact.clone()
    }
}

/// Whether the sections filling `cells` join into one ship through sockets
/// that meet face to face, as the grid mates them.
fn joined(cells: &HashMap<IVec3, (usize, [bool; 6])>) -> bool {
    let mut sections: HashMap<usize, Vec<IVec3>> = HashMap::new();
    for (cell, (section, _)) in cells {
        sections.entry(*section).or_default().push(*cell);
    }
    let Some(first) = sections.keys().min().copied() else {
        return true;
    };
    let mut reached = HashSet::from([first]);
    let mut queue = vec![first];
    while let Some(section) = queue.pop() {
        for cell in &sections[&section] {
            let (_, faces) = cells[cell];
            for (face, step) in STEPS.iter().enumerate() {
                let Some((other, other_faces)) = cells.get(&(*cell + *step)) else {
                    continue;
                };
                if faces[face] && other_faces[face ^ 1] && reached.insert(*other) {
                    queue.push(*other);
                }
            }
        }
    }
    reached.len() == sections.len()
}

/// The hull clearance ceiling at `advancement`.
fn ceiling(advancement: f32) -> Meters {
    let span = MAX_HULL_CLEARANCE.0 - MIN_HULL_CLEARANCE.0;
    Meters(MIN_HULL_CLEARANCE.0 + span * advancement.clamp(0.0, 1.0))
}

/// The clearance radius around the design origin of the cells from `low` to
/// `high`, both included: the distance to the farthest corner. A cell is
/// centred on its integer coordinate.
fn clearance(low: IVec3, high: IVec3) -> Meters {
    let reach = (low.as_vec3() - Vec3::splat(0.5))
        .abs()
        .max((high.as_vec3() + Vec3::splat(0.5)).abs());
    Meters::from_engine(reach.length())
}

/// The [`clearance`] the cells from `low` to `high` have once the ship is
/// centred on its bounds. Centring moves the origin by whole cells, so along
/// an axis with an even cell count it stands half a cell off the bounds'
/// centre. The x axis is the mirror plane and always has an odd count.
fn centred_clearance(low: IVec3, high: IVec3) -> Meters {
    let cells = high - low + IVec3::ONE;
    let even = (cells % 2).cmpeq(IVec3::ZERO);
    let half = cells.as_vec3() * 0.5 + Vec3::select(even, Vec3::splat(0.5), Vec3::ZERO);
    Meters::from_engine(half.length())
}

/// The draw stream for one aspect of one layout attempt.
fn stream(request: ShipLayoutRequest, attempt: u32, aspect: &[u8]) -> SeedStream {
    SeedStream::new(
        Fnv32::new()
            .write(&request.seed.to_le_bytes())
            .write(b"ship_layout")
            .write(&attempt.to_le_bytes())
            .write(aspect)
            .finish(),
    )
}

/// `items` in a seeded order.
fn shuffled<T>(mut items: Vec<T>, stream: &mut SeedStream) -> Vec<T> {
    for index in (1..items.len()).rev() {
        let pick = ((stream.unit() * (index + 1) as f32) as usize).min(index);
        items.swap(index, pick);
    }
    items
}

/// A signed index as a section id writes it: `n<abs>` when negative.
fn signed(value: i32) -> String {
    if value < 0 {
        format!("n{}", value.unsigned_abs())
    } else {
        value.to_string()
    }
}

/// Whether `port` is `starboard` reflected through its block's `x` centre:
/// the same cells with mirrored sockets and exits, and a mirrored origin.
fn reflects(starboard: &OrientedPart, port: &OrientedPart) -> bool {
    if starboard.span != port.span
        || (port.origin.x - (starboard.span.x - 1) as f32 + starboard.origin.x).abs() > GRID_EPSILON
        || !port
            .origin
            .yz()
            .abs_diff_eq(starboard.origin.yz(), GRID_EPSILON)
    {
        return false;
    }
    starboard.cells.iter().all(|cell| {
        let mut image = cell.cell;
        image.x = starboard.span.x - 1 - image.x;
        port.cells.iter().any(|other| {
            other.cell == image
                && other.exit == cell.exit.map(mirror_face)
                && (0..6).all(|face| other.faces[mirror_face(face)] == cell.faces[face])
        })
    })
}

/// One part mounted in a layout under construction.
struct Placement<'a> {
    id: String,
    part: &'a ShipPart,
    oriented: OrientedPart,
    /// The grid cell of the block's minimum corner.
    anchor: IVec3,
}

impl Placement<'_> {
    fn position(&self) -> Vec3 {
        self.anchor.as_vec3() + self.oriented.origin
    }

    fn placed(&self) -> PlacedPart<'_> {
        placed_part(&self.part.config, self.position(), self.oriented.rotation)
    }
}

/// A section as the skin and lane rule read it.
fn placed_part(config: &SectionConfig, position: Vec3, rotation: Quat) -> PlacedPart<'_> {
    PlacedPart {
        position,
        rotation,
        link_points: &config.base.link_points,
        footprint: SectionFootprint::from_collider(config.base.collider.unwrap_or_default()).0,
        exit: exit_normal(&config.kind),
    }
}

/// One filled grid cell.
#[derive(Clone, Copy)]
struct Filled {
    faces: [bool; 6],
    exit: Option<usize>,
    /// Whether a structural cube fills it.
    structural: bool,
}

/// A layout's cells while it is built.
#[derive(Default)]
struct Grid<'a> {
    filled: HashMap<IVec3, Filled>,
    placements: Vec<Placement<'a>>,
}

impl<'a> Grid<'a> {
    /// Whether `oriented` may stand at `anchor`: every cell empty, every
    /// contact socket against socket or blind against blind, nothing fired
    /// into a neighbour, and at least one socket mated.
    fn fits(&self, oriented: &OrientedPart, anchor: IVec3) -> bool {
        let own: HashSet<IVec3> = oriented
            .cells
            .iter()
            .map(|cell| anchor + cell.cell.as_ivec3())
            .collect();
        if own.iter().any(|cell| self.filled.contains_key(cell)) {
            return false;
        }
        let mut mates = 0;
        for cell in &oriented.cells {
            let at = anchor + cell.cell.as_ivec3();
            for (face, step) in STEPS.iter().enumerate() {
                let beside = at + *step;
                if own.contains(&beside) {
                    continue;
                }
                let Some(other) = self.filled.get(&beside) else {
                    continue;
                };
                if cell.exit == Some(face)
                    || other.exit == Some(face ^ 1)
                    || cell.faces[face] != other.faces[face ^ 1]
                {
                    return false;
                }
                mates += usize::from(cell.faces[face]);
            }
        }
        mates > 0
    }

    /// Whether `oriented` at `anchor` has a structural cube behind every
    /// cell of its back, the face opposite its exit.
    fn backed(&self, oriented: &OrientedPart, anchor: IVec3) -> bool {
        let Some(exit) = oriented.aims else {
            return false;
        };
        let own: HashSet<IVec3> = oriented
            .cells
            .iter()
            .map(|cell| anchor + cell.cell.as_ivec3())
            .collect();
        own.iter().all(|cell| {
            let behind = *cell + STEPS[exit ^ 1];
            own.contains(&behind)
                || self
                    .filled
                    .get(&behind)
                    .is_some_and(|filled| filled.structural)
        })
    }

    /// Put `part` at `anchor`. `structural` marks a structural cube, which a
    /// backed part may stand against.
    fn insert(
        &mut self,
        id: String,
        part: &'a ShipPart,
        oriented: OrientedPart,
        anchor: IVec3,
        structural: bool,
    ) {
        for cell in &oriented.cells {
            self.filled.insert(
                anchor + cell.cell.as_ivec3(),
                Filled {
                    faces: cell.faces,
                    exit: cell.exit,
                    structural,
                },
            );
        }
        self.placements.push(Placement {
            id,
            part,
            oriented,
            anchor,
        });
    }

    /// Take the last placement back out.
    fn pop(&mut self) {
        let placement = self.placements.pop().expect("a placement to take back");
        for cell in &placement.oriented.cells {
            self.filled
                .remove(&(placement.anchor + cell.cell.as_ivec3()));
        }
    }

    /// Whether the last placement blocks an exit lane, its own included.
    fn last_blocks_an_exit(&self) -> bool {
        let (last, ship) = self.placements.split_last().expect("a placement to check");
        let ship: Vec<PlacedPart> = ship.iter().map(Placement::placed).collect();
        placement_blocks_an_exit(&ship, &last.placed())
    }

    /// Every empty cell beside a filled one on the starboard half or the
    /// centreline, in a fixed order.
    fn surface(&self) -> BTreeSet<[i32; 3]> {
        self.filled
            .keys()
            .flat_map(|cell| STEPS.map(|step| *cell + step))
            .filter(|cell| cell.x >= 0 && !self.filled.contains_key(cell))
            .map(|cell| cell.to_array())
            .collect()
    }

    /// The minimum and maximum filled cells.
    fn bounds(&self) -> (IVec3, IVec3) {
        self.filled
            .keys()
            .fold((IVec3::MAX, IVec3::MIN), |(low, high), cell| {
                (low.min(*cell), high.max(*cell))
            })
    }
}

/// One station's cross-section: `x` in `[-half_width, half_width]` and `y` in
/// `[low, high]`. Every one holds the centre cell, so consecutive stations
/// always share a column of cubes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Section {
    half_width: i32,
    low: i32,
    high: i32,
}

/// The spine size a role reaches, as `(advancement 0, advancement 1)` pairs of
/// cells: stations, half-width and half-height of the body.
fn role_spine(role: ShipRoleType) -> [(f32, f32); 3] {
    match role {
        // A clean, narrow fuselage.
        ShipRoleType::Civilian => [(6.0, 22.0), (1.0, 1.0), (0.0, 1.0)],
        // Bulky working volume.
        ShipRoleType::Industrial => [(5.0, 16.0), (1.0, 3.0), (1.0, 2.0)],
        // Rough and small.
        ShipRoleType::Scavenger => [(5.0, 14.0), (1.0, 2.0), (0.0, 1.0)],
        // Dense and long.
        ShipRoleType::Armored => [(6.0, 24.0), (1.0, 3.0), (1.0, 2.0)],
    }
}

/// How a weapon may point. A turret traverses, so any outward face but aft
/// serves; a fixed bore fires forward only.
fn weapon_aim(kind: &SectionKind, aims: Option<usize>) -> Option<u32> {
    let turret = matches!(kind, SectionKind::Turret(_));
    match aims? {
        FORWARD if !turret => Some(3),
        _ if !turret => None,
        DORSAL => Some(3),
        STARBOARD | PORT => Some(2),
        VENTRAL | FORWARD => Some(1),
        _ => None,
    }
}

/// How an intake or a docking port may point: out of the hull's side.
fn flank_aim(_: &SectionKind, aims: Option<usize>) -> Option<u32> {
    matches!(aims?, STARBOARD | PORT).then_some(1)
}

/// How many weapon slots a role fills at `advancement`. The first is
/// required; a later slot that finds no room is left out.
fn weapon_slots(role: ShipRoleType, advancement: f32) -> u32 {
    let advancement = advancement.clamp(0.0, 1.0);
    match role {
        ShipRoleType::Civilian | ShipRoleType::Industrial => 0,
        ShipRoleType::Scavenger => 1 + (advancement * 2.0).round() as u32,
        ShipRoleType::Armored => 1 + (advancement * 4.0).round() as u32,
    }
}

/// A section's mass: its collider's volume, at the density 1 every section
/// spawns with.
fn mass(config: &SectionConfig) -> f32 {
    use std::f32::consts::PI;
    match config.base.collider.unwrap_or_default() {
        SectionCollider::Cuboid { size } => size.x * size.y * size.z,
        SectionCollider::Sphere { radius } => 4.0 / 3.0 * PI * radius.powi(3),
        SectionCollider::Capsule { radius, length } => {
            PI * radius * radius * (length + 4.0 / 3.0 * radius)
        }
        SectionCollider::Cylinder { radius, height } => PI * radius * radius * height,
    }
}

/// What a ship's parts add up to.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ShipMeasure {
    /// Mass, in cells.
    mass: f32,
    /// Summed drive thrust.
    thrust: f32,
    /// Flight computers.
    controllers: usize,
}

impl ShipMeasure {
    fn of<'p>(parts: impl IntoIterator<Item = &'p ShipPart>) -> Self {
        let mut measure = Self {
            mass: 0.0,
            thrust: 0.0,
            controllers: 0,
        };
        for part in parts {
            measure.mass += mass(&part.config);
            match &part.config.kind {
                SectionKind::Thruster(thruster) => measure.thrust += thruster.magnitude,
                SectionKind::Controller(_) => measure.controllers += 1,
                _ => {}
            }
        }
        measure
    }

    fn needed_thrust(self) -> f32 {
        MIN_THRUST_PER_CELL * self.mass
    }

    fn needed_controllers(self) -> usize {
        ((self.mass / CELLS_PER_CONTROLLER).ceil() as usize).max(1)
    }

    /// The first floor the ship falls short of.
    fn shortfall(self) -> Option<ShipLayoutConstraintType> {
        if self.thrust < self.needed_thrust() {
            return Some(ShipLayoutConstraintType::Underpowered {
                thrust: self.thrust,
                needed: self.needed_thrust(),
            });
        }
        (self.controllers < self.needed_controllers()).then(|| {
            ShipLayoutConstraintType::Undercontrolled {
                controllers: self.controllers,
                needed: self.needed_controllers(),
            }
        })
    }
}

/// How many stations of a spine `length` long the nose takes.
fn nose_length(length: i32) -> i32 {
    1 + length / 5
}

/// How a hull lays its second palette cube over the spine. The third, when
/// the palette has one, trims the nose and the stern.
#[derive(Clone, Copy, Debug)]
enum PalettePatternType {
    /// Rings of `width` stations along the length, every other ring.
    Rings { width: i32, offset: i32 },
    /// Everything above the keel.
    Deck,
    /// The outer shell of every cross-section.
    Shell,
}

impl PalettePatternType {
    fn draw(stream: &mut SeedStream) -> Self {
        let pick = stream.unit();
        let width = 2 + (stream.unit() * 3.0) as i32;
        let offset = (stream.unit() * width as f32) as i32;
        if pick < 0.4 {
            Self::Rings { width, offset }
        } else if pick < 0.7 {
            Self::Deck
        } else {
            Self::Shell
        }
    }

    /// The palette index of the spine cell `cell` of `stations`, for a
    /// palette of `size` cubes. A function of `|x|`, so the spine stays its
    /// own mirror image.
    fn cube(self, stations: &[Section], cell: IVec3, size: usize) -> usize {
        let length = stations.len() as i32;
        if size >= 3 && (cell.z < nose_length(length) || cell.z == length - 1) {
            return 2;
        }
        let station = stations[cell.z as usize];
        let second = match self {
            Self::Rings { width, offset } => ((cell.z + offset) / width) % 2 == 1,
            Self::Deck => cell.y > 0,
            Self::Shell => {
                cell.x.abs() == station.half_width
                    || cell.y == station.low
                    || cell.y == station.high
            }
        };
        usize::from(size >= 2 && second)
    }
}

/// One bank of copies of one drive behind the stern: `columns` across, an
/// odd count around a centre column or an even count of mirrored pairs, and
/// `rows` stacked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DriveBank {
    columns: i32,
    rows: i32,
}

impl DriveBank {
    /// The one drive or the one pair a layout starts from.
    fn first(layout: ShipDriveLayoutType) -> Self {
        let columns = match layout {
            ShipDriveLayoutType::CenteredSingle => 1,
            ShipDriveLayoutType::MirroredPair => 2,
        };
        Self { columns, rows: 1 }
    }

    fn drives(self) -> i32 {
        self.columns * self.rows
    }

    /// The smallest bank at least this large that holds `drives` drives of
    /// `span` cells: widened by a pair or raised by a row, whichever keeps it
    /// squarer. A drive with no mirrored twin only stacks.
    fn holding(mut self, drives: i32, span: IVec3, pairs: bool) -> Self {
        while self.drives() < drives {
            if pairs && self.columns * span.x <= self.rows * span.y {
                self.columns += 2;
            } else {
                self.rows += 1;
            }
        }
        self
    }

    /// The stern station the bank mates to.
    fn stern(self, layout: ShipDriveLayoutType, span: IVec3) -> Section {
        let height = self.rows * span.y;
        let low = -(height - 1) / 2;
        Section {
            half_width: match layout {
                ShipDriveLayoutType::CenteredSingle => (self.columns * span.x - 1) / 2,
                ShipDriveLayoutType::MirroredPair => self.columns / 2 * span.x,
            },
            low,
            high: low + height - 1,
        }
    }
}

/// The drive a layout draws, turned to fire aft.
struct DrivePlan<'a> {
    part: &'a ShipPart,
    /// The drive on the centreline or the starboard side.
    starboard: OrientedPart,
    /// Its mirrored twin, when the drive has one.
    port: Option<OrientedPart>,
    layout: ShipDriveLayoutType,
}

/// Centreline cells inside the spine for `count` flight computers: the first
/// a quarter back from the bow, the rest spread along the keel, then above
/// and below it. Fewer when the spine has too few. The nose and the stern
/// hold none.
fn bridges(stations: &[Section], count: usize) -> Vec<IVec3> {
    let length = stations.len() as i32;
    let first = IVec3::new(0, 0, (length / 4).max(1));
    let keel: Vec<IVec3> = (1..length - 1)
        .map(|z| IVec3::new(0, 0, z))
        .filter(|cell| *cell != first)
        .collect();
    let beside: Vec<IVec3> = (1..length - 1)
        .flat_map(|z| {
            let station = stations[z as usize];
            (station.low..=station.high)
                .filter(|y| *y != 0)
                .map(move |y| IVec3::new(0, y, z))
        })
        .collect();
    let mut cells = vec![first];
    for pool in [keel, beside] {
        let wanted = count.saturating_sub(cells.len()).min(pool.len());
        cells.extend((0..wanted).map(|index| pool[(2 * index + 1) * pool.len() / (2 * wanted)]));
    }
    cells.truncate(count);
    cells
}

/// A flight computer fires nowhere and may stand anywhere it mates.
fn controller_aim(_: &SectionKind, aims: Option<usize>) -> Option<u32> {
    aims.is_none().then_some(1)
}

/// One layout attempt.
struct Draw<'a> {
    snapshot: &'a ShipPartSnapshot,
    request: ShipLayoutRequest,
    attempt: u32,
}

impl<'a> Draw<'a> {
    fn new(snapshot: &'a ShipPartSnapshot, request: ShipLayoutRequest, attempt: u32) -> Self {
        Self {
            snapshot,
            request,
            attempt,
        }
    }

    fn stream(&self, aspect: &[u8]) -> SeedStream {
        stream(self.request, self.attempt, aspect)
    }

    /// The eligible parts of `family` that pass `usable`, in the order this
    /// attempt tries them: the preferred source first for a coherent role,
    /// then the other sources, each source's parts shuffled. A drive or weapon
    /// within [`TIER_BAND`] of the request's advancement comes before its
    /// source's weaker parts.
    fn ranked(
        &self,
        family: ShipPartFamilyType,
        preferred: Option<&str>,
        aspect: &[u8],
        usable: impl Fn(&ShipPart) -> bool,
    ) -> Vec<&'a ShipPart> {
        let mut stream = self.stream(aspect);
        let coherent = self.request.role != ShipRoleType::Scavenger;
        let sources: Vec<(&str, Vec<&ShipPart>)> = self
            .snapshot
            .eligible(self.request.role, family, self.request.advancement)
            .into_iter()
            .map(|(source, parts)| {
                (
                    source,
                    parts
                        .into_iter()
                        .filter(|part| usable(part))
                        .collect::<Vec<_>>(),
                )
            })
            .filter(|(_, parts)| !parts.is_empty())
            .collect();
        let mut sources = shuffled(sources, &mut stream);
        let banded = matches!(
            family,
            ShipPartFamilyType::Thruster | ShipPartFamilyType::Weapon
        );
        let floor = self.request.advancement - TIER_BAND;
        if coherent {
            if let Some(index) = sources
                .iter()
                .position(|(source, _)| Some(*source) == preferred)
            {
                let first = sources.remove(index);
                sources.insert(0, first);
            }
        }
        sources
            .into_iter()
            .flat_map(|(_, parts)| {
                let (tier, weaker): (Vec<_>, Vec<_>) = shuffled(parts, &mut stream)
                    .into_iter()
                    .partition(|part| !banded || part.advancement >= floor);
                tier.into_iter().chain(weaker)
            })
            .collect()
    }

    /// Build the ship, growing its drive bank and its flight computers until
    /// they meet its mass's floors.
    fn layout(&self) -> Result<ShipLayout, ShipLayoutConstraintType> {
        let palette = self.palette()?;
        let drive = self.drive(palette[0].source.as_str())?;
        let span = drive.starboard.span.as_ivec3();
        let SectionKind::Thruster(thruster) = &drive.part.config.kind else {
            unreachable!("a thruster part is a thruster section");
        };
        let mut bank = DriveBank::first(drive.layout);
        let mut controllers = 1;
        let mut short = None;
        loop {
            let (layout, measure) = match self.build(&palette, &drive, bank, controllers) {
                Ok(built) => built,
                // A bank grown past the size ceiling failed on thrust.
                Err(ShipLayoutConstraintType::TooLarge { .. }) if short.is_some() => {
                    return Err(short.expect("a shortfall"));
                }
                Err(constraint) => return Err(constraint),
            };
            let Some(shortfall) = measure.shortfall() else {
                return Ok(layout);
            };
            let drives = (measure.needed_thrust() / thruster.magnitude).ceil() as i32;
            let grown = bank.holding(drives, span, drive.port.is_some());
            let needed = controllers.max(measure.needed_controllers());
            if (grown, needed) == (bank, controllers) {
                return Err(shortfall);
            }
            (bank, controllers, short) = (grown, needed, Some(shortfall));
        }
    }

    /// The structural cubes the spine mixes, main cube first: up to
    /// [`STRUCTURAL_PALETTE`] parts in this attempt's order, all from the main
    /// cube's source but on a scavenger.
    fn palette(&self) -> Result<Vec<&'a ShipPart>, ShipLayoutConstraintType> {
        let ranked = self.ranked(ShipPartFamilyType::Hull, None, b"hull", is_structural_cube);
        let main = *ranked
            .first()
            .ok_or(ShipLayoutConstraintType::Unplaced(ShipPartFamilyType::Hull))?;
        let mixed = self.request.role == ShipRoleType::Scavenger;
        Ok(ranked
            .into_iter()
            .filter(|part| mixed || part.source == main.source)
            .take(STRUCTURAL_PALETTE)
            .collect())
    }

    /// The drive, turned to fire aft, and whether its bank centres a column
    /// or pairs. An even-width drive has no centre column, so it only pairs.
    fn drive(&self, source: &str) -> Result<DrivePlan<'a>, ShipLayoutConstraintType> {
        let mut drive_stream = self.stream(b"drive_layout");
        self.ranked(ShipPartFamilyType::Thruster, Some(source), b"drive", |_| {
            true
        })
        .into_iter()
        .find_map(|part| {
            let starboard = cube_rotations()
                .into_iter()
                .filter_map(|rotation| oriented_part(&part.config, rotation).ok())
                .find(|oriented| oriented.aims == Some(AFT))?;
            let port = oriented_part(&part.config, mirror_rotation(starboard.rotation))
                .ok()
                .filter(|port| reflects(&starboard, port));
            let single = starboard.span.x % 2 == 1 && reflects(&starboard, &starboard);
            let layout = match (single, port.is_some()) {
                (true, true) if drive_stream.unit() < 0.5 => ShipDriveLayoutType::MirroredPair,
                (true, _) => ShipDriveLayoutType::CenteredSingle,
                (false, true) => ShipDriveLayoutType::MirroredPair,
                (false, false) => return None,
            };
            Some(DrivePlan {
                part,
                starboard,
                port,
                layout,
            })
        })
        .ok_or(ShipLayoutConstraintType::Unplaced(
            ShipPartFamilyType::Thruster,
        ))
    }

    /// Build the ship with `bank` behind its stern and `controllers` flight
    /// computers.
    fn build(
        &self,
        palette: &[&'a ShipPart],
        drive: &DrivePlan<'a>,
        bank: DriveBank,
        controllers: usize,
    ) -> Result<(ShipLayout, ShipMeasure), ShipLayoutConstraintType> {
        let request = self.request;
        let source = palette[0].source.as_str();
        let cube_cell = oriented_part(&palette[0].config, Quat::IDENTITY)
            .expect("a structural cube stands on the grid");
        let span = drive.starboard.span.as_ivec3();
        let stern = bank.stern(drive.layout, span);

        let intake = ShipPartFamilyType::CargoIntake.required_by(request.role);
        let intakes = if intake {
            self.ranked(
                ShipPartFamilyType::CargoIntake,
                Some(source),
                b"intake",
                |_| true,
            )
        } else {
            Vec::new()
        };
        let docks = self.ranked(ShipPartFamilyType::Docking, Some(source), b"dock", |_| true);
        let flank = if intake {
            Some(flank_run(&intakes, &docks)?)
        } else {
            None
        };

        let stations = self.plan(stern, span.z, flank)?;
        let length = stations.len() as i32;
        let pattern = PalettePatternType::draw(&mut self.stream(b"palette"));
        let cube_at = |cell: IVec3| palette[pattern.cube(&stations, cell, palette.len())];

        // The spine, bow at station 0. The bridge cells are left for the
        // flight computers.
        let bridges = bridges(&stations, controllers);
        let mut grid = Grid::default();
        for (z, section) in stations.iter().enumerate() {
            let z = z as i32;
            for x in -section.half_width..=section.half_width {
                for y in section.low..=section.high {
                    let cell = IVec3::new(x, y, z);
                    if !bridges.contains(&cell) {
                        grid.insert(
                            format!("hull_z{z}_x{}_y{}", signed(x), signed(y)),
                            cube_at(cell),
                            cube_cell.clone(),
                            cell,
                            true,
                        );
                    }
                }
            }
        }

        // The bank behind the stern, row by row, each drive's mount face
        // against the stern station. A centred bank starts from its centre
        // column; a paired one leaves that column open.
        for row in 0..bank.rows {
            let y = stern.low + row * span.y;
            let mut mounts = Vec::new();
            let pairs_from = match drive.layout {
                ShipDriveLayoutType::CenteredSingle => {
                    mounts.push((IVec3::new(-(span.x - 1) / 2, y, length), &drive.starboard));
                    (span.x - 1) / 2 + 1
                }
                ShipDriveLayoutType::MirroredPair => 1,
            };
            for pair in 0..bank.columns / 2 {
                let port = drive
                    .port
                    .as_ref()
                    .expect("a bank with pairs has a mirrored drive");
                let x = pairs_from + pair * span.x;
                mounts.push((IVec3::new(x, y, length), &drive.starboard));
                mounts.push((IVec3::new(-(x + span.x - 1), y, length), port));
            }
            for (anchor, oriented) in mounts {
                if !grid.fits(oriented, anchor) {
                    return Err(ShipLayoutConstraintType::Unplaced(
                        ShipPartFamilyType::Thruster,
                    ));
                }
                grid.insert(
                    format!("drive_x{}_y{}", signed(anchor.x), signed(anchor.y)),
                    drive.part,
                    oriented.clone(),
                    anchor,
                    false,
                );
                if grid.last_blocks_an_exit() {
                    let id = grid.placements.last().expect("the drive").id.clone();
                    return Err(ShipLayoutConstraintType::BlockedExit(id));
                }
            }
        }

        // The flight computers: in the bridge cells where they fit there,
        // otherwise on the hull's surface like any fitting.
        let parts = self.ranked(
            ShipPartFamilyType::Controller,
            Some(source),
            b"controller",
            |_| true,
        );
        for index in 0..controllers {
            let slot = format!("controller_{index}");
            let bridge = bridges.get(index).copied();
            let inside = bridge.and_then(|bridge| {
                parts.iter().find_map(|part| {
                    cube_rotations()
                        .into_iter()
                        .filter_map(|rotation| oriented_part(&part.config, rotation).ok())
                        .find(|oriented| {
                            oriented.span == UVec3::ONE
                                && reflects(oriented, oriented)
                                && oriented.origin.abs_diff_eq(Vec3::ZERO, GRID_EPSILON)
                                && grid.fits(oriented, bridge)
                        })
                        .map(|oriented| (*part, oriented, bridge))
                })
            });
            if let Some((part, oriented, bridge)) = inside {
                grid.insert(slot, part, oriented, bridge, false);
                continue;
            }
            if let Some(bridge) = bridge {
                grid.insert(
                    format!("hull_z{}_x0_y{}", bridge.z, signed(bridge.y)),
                    cube_at(bridge),
                    cube_cell.clone(),
                    bridge,
                    true,
                );
            }
            let aspect = format!("{slot}_slot");
            if !self.fit(
                &mut grid,
                &parts,
                &slot,
                controller_aim,
                MountType::Mirrored,
                aspect.as_bytes(),
            ) {
                return Err(ShipLayoutConstraintType::Unplaced(
                    ShipPartFamilyType::Controller,
                ));
            }
        }

        // Role equipment. Each weapon slot tries the hull's source first on a
        // coherent role, then its tier's parts and, within a tier, the parts
        // no earlier slot mounted.
        let mut mounted: HashSet<&str> = HashSet::new();
        let floor = request.advancement - TIER_BAND;
        let coherent = request.role != ShipRoleType::Scavenger;
        for slot in 0..weapon_slots(request.role, request.advancement) {
            let aspect = format!("weapon_{slot}");
            let mut weapons = self.ranked(
                ShipPartFamilyType::Weapon,
                Some(source),
                aspect.as_bytes(),
                |_| true,
            );
            weapons.sort_by_key(|part| {
                (
                    coherent && part.source != source,
                    part.advancement < floor,
                    mounted.contains(part.id()),
                )
            });
            let placed = self.fit(
                &mut grid,
                &weapons,
                &aspect,
                weapon_aim,
                MountType::Mirrored,
                format!("{aspect}_slot").as_bytes(),
            );
            if placed {
                mounted.insert(grid.placements.last().expect("a weapon").part.id());
            } else if slot == 0 {
                return Err(ShipLayoutConstraintType::Unplaced(
                    ShipPartFamilyType::Weapon,
                ));
            }
        }
        if intake
            && !self.fit(
                &mut grid,
                &intakes,
                "intake",
                flank_aim,
                MountType::BackedMirrored,
                b"intake_slot",
            )
        {
            return Err(ShipLayoutConstraintType::Unplaced(
                ShipPartFamilyType::CargoIntake,
            ));
        }

        // Every ship docks: a mirrored pair of flank ports where one fits,
        // else one port alone, each with its lane clear and cubes across its
        // back.
        let docked = [MountType::BackedMirrored, MountType::BackedLone]
            .into_iter()
            .any(|mount| self.fit(&mut grid, &docks, "dock", flank_aim, mount, b"dock_slot"));
        if !docked {
            return Err(ShipLayoutConstraintType::Unplaced(
                ShipPartFamilyType::Docking,
            ));
        }

        // Centre the ship on its bounds along y and z; x is already the
        // mirror plane.
        let (low, high) = grid.bounds();
        let mut shift = -(low + high).div_euclid(IVec3::splat(2));
        shift.x = 0;
        let sections = grid
            .placements
            .iter()
            .map(|placement| SpaceshipSectionConfig {
                id: placement.id.clone(),
                position: placement.position() + shift.as_vec3(),
                rotation: placement.oriented.rotation,
                source: SectionSource::Prototype {
                    id: placement.part.id().to_string(),
                    patch: default(),
                },
            })
            .collect();

        let measure = ShipMeasure::of(grid.placements.iter().map(|placement| placement.part));
        Ok((
            ShipLayout {
                design: ShipDesign {
                    sections,
                    presentation: ShipPresentationConfig {
                        skin: true,
                        style: Some(role_style_id(self.request.role).to_string()),
                        ..default()
                    },
                    ..default()
                },
                drive: drive.layout,
                source: source.to_string(),
                clearance: clearance(low + shift, high + shift),
                attempt: self.attempt,
            },
            measure,
        ))
    }

    /// The spine's stations, bow first, ending in `stern`: a nose that widens
    /// from one cell, a body whose cross-section changes along its length,
    /// and the stern the drive bank mates to. A `flank` run holds its flat
    /// stations at every size. Shrunk until the spine and a bank `drive_depth`
    /// cells deep fit the advancement's size ceiling with room for fittings.
    fn plan(
        &self,
        stern: Section,
        drive_depth: i32,
        flank: Option<FlankRun>,
    ) -> Result<Vec<Section>, ShipLayoutConstraintType> {
        let request = self.request;
        let mut stream = self.stream(b"shape");
        let advancement = request.advancement.clamp(0.0, 1.0);
        let [length, width, height] =
            role_spine(request.role).map(|(low, high)| low + (high - low) * advancement);
        let scale = 0.8 + 0.4 * stream.unit();
        // Most hulls are long; a minority are deliberately compact or broad.
        let variant = stream.unit();
        let (length, widen, heighten) = if variant < 0.7 {
            (length * scale, 0, 0)
        } else if variant < 0.85 {
            (length * scale * 0.65, 1, 0)
        } else {
            (length * scale * 0.85, 1, 1)
        };
        // The fewest stations whose body, between the nose and the stern,
        // holds the flank run.
        let shortest = (MIN_STATIONS..)
            .find(|length| flank.is_none_or(|run| length - nose_length(*length) > run.length))
            .expect("a long enough spine holds any run");
        let mut length = (length.round() as i32).max(shortest);
        let mut width = (width.round() as i32).max(1) + widen;
        let mut height = (height.round() as i32).max(0) + heighten;
        // The body's change along its length: a waist or shoulder at the
        // middle and a taper or flare toward the stern.
        let middle = [stream.signed(), stream.signed()];
        let aft = [stream.unit(), stream.unit()];

        let ceiling = ceiling(request.advancement);
        loop {
            let mut stations = stations(length, width, height, stern, middle, aft);
            if let Some(run) = flank {
                hold_flank(&mut stations, nose_length(length), run);
            }
            let (low, high) = stations.iter().enumerate().fold(
                (IVec3::MAX, IVec3::MIN),
                |(low, high), (z, section)| {
                    let z = z as i32;
                    (
                        low.min(IVec3::new(-section.half_width, section.low, z)),
                        high.max(IVec3::new(section.half_width, section.high, z)),
                    )
                },
            );
            let high = high.max(IVec3::new(high.x, high.y, length + drive_depth - 1));
            let reach = IVec3::splat(FITTING_REACH);
            let planned = centred_clearance(low - reach, high + reach);
            if planned.0 <= ceiling.0 {
                return Ok(stations);
            }
            if length > shortest {
                length -= 1;
            } else if width > 1 {
                width -= 1;
            } else if height > 0 {
                height -= 1;
            } else {
                return Err(ShipLayoutConstraintType::TooLarge {
                    clearance: planned,
                    ceiling,
                });
            }
        }
    }
}

/// `length` stations for a body `width` and `height` cells from its centre,
/// ending in `stern`. `middle` moves the body's half-width and half-height at
/// its middle by up to one cell; `aft` blends them toward the stern's.
fn stations(
    length: i32,
    width: i32,
    height: i32,
    stern: Section,
    middle: [f32; 2],
    aft: [f32; 2],
) -> Vec<Section> {
    let nose = nose_length(length);
    let body = length - 1;
    let stern_half = [stern.half_width, (stern.high - stern.low) / 2];
    (0..length)
        .map(|z| {
            if z == length - 1 {
                return stern;
            }
            let [half_width, half_height] = [(width, 0), (height, 1)].map(|(size, axis)| {
                let size = size as f32;
                if z < nose {
                    return (size * z as f32 / nose as f32).round() as i32;
                }
                // 0 at the nose's end, 1 at the stern.
                let t = (z - nose) as f32 / (body - nose).max(1) as f32;
                let bulge = middle[axis] * (1.0 - (2.0 * t - 1.0).abs());
                let towards = aft[axis] * t * t;
                let value = size + bulge + (stern_half[axis] as f32 - size) * towards;
                value.round() as i32
            });
            let floor = usize::from(z >= nose) as i32;
            let half_height = half_height.max(0);
            Section {
                half_width: half_width.max(floor),
                low: -half_height,
                high: half_height,
            }
        })
        .collect()
}

/// The flat flank an intake-carrying spine holds so its mirrored intake pair
/// and one docking port fit side by side along it: `length` body stations of
/// one cross-section, at least one cell wide and `half_height` cells above and
/// below the centre.
#[derive(Clone, Copy, Debug)]
struct FlankRun {
    length: i32,
    half_height: i32,
}

/// The flank run the first of `intakes` that mounts as a mirrored pair and the
/// first of `docks` that mounts on the flank need, in the flank rotation of
/// each with the fewest cells above and below the centre, then the fewest
/// stations.
fn flank_run(
    intakes: &[&ShipPart],
    docks: &[&ShipPart],
) -> Result<FlankRun, ShipLayoutConstraintType> {
    let footprint = |part: &ShipPart, mirrored: bool| {
        cube_rotations()
            .into_iter()
            .filter_map(|rotation| {
                let oriented = oriented_part(&part.config, rotation).ok()?;
                let twin = oriented_part(&part.config, mirror_rotation(rotation))
                    .is_ok_and(|port| reflects(&oriented, &port));
                (oriented.aims == Some(STARBOARD) && (twin || !mirrored)).then(|| {
                    let span = oriented.span.as_ivec3();
                    (span.y / 2, span.z)
                })
            })
            .min()
    };
    let (intake_height, intake_length) = intakes
        .iter()
        .find_map(|part| footprint(part, true))
        .ok_or(ShipLayoutConstraintType::Unplaced(
            ShipPartFamilyType::CargoIntake,
        ))?;
    let (dock_height, dock_length) = docks.iter().find_map(|part| footprint(part, false)).ok_or(
        ShipLayoutConstraintType::Unplaced(ShipPartFamilyType::Docking),
    )?;
    Ok(FlankRun {
        length: intake_length + dock_length,
        half_height: intake_height.max(dock_height),
    })
}

/// Keep `run` on the body of `stations`, whose nose is `nose` stations long:
/// a body that already holds the run is left as drawn; otherwise the first
/// body stations are flattened to the first one's cross-section, widened and
/// raised to the run's.
fn hold_flank(stations: &mut [Section], nose: i32, run: FlankRun) {
    let (nose, length) = (nose as usize, run.length as usize);
    let body = &stations[nose..stations.len() - 1];
    let held = body.windows(length).any(|window| {
        window.iter().all(|section| *section == window[0])
            && window[0].half_width >= 1
            && window[0].high >= run.half_height
    });
    if held {
        return;
    }
    let first = stations[nose];
    let half_height = first.high.max(run.half_height);
    stations[nose..nose + length].fill(Section {
        half_width: first.half_width.max(1),
        low: -half_height,
        high: half_height,
    });
}

/// Whether a part can be the spine's structural cube: one cell, filled, with
/// a socket on every face.
fn is_structural_cube(part: &ShipPart) -> bool {
    part.family == ShipPartFamilyType::Hull
        && oriented_part(&part.config, Quat::IDENTITY).is_ok_and(|oriented| {
            oriented.span == UVec3::ONE
                && oriented.origin.abs_diff_eq(Vec3::ZERO, GRID_EPSILON)
                && oriented.cells[0].faces == [true; 6]
        })
}

/// How [`Draw::fit`] may mount a part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MountType {
    /// On the centreline or as a mirrored pair.
    Mirrored,
    /// On the centreline or as a mirrored pair, with a structural cube behind
    /// every cell of each part's back.
    BackedMirrored,
    /// One part with no twin, on the centreline or the starboard half, with a
    /// structural cube behind every cell of its back.
    BackedLone,
}

impl Draw<'_> {
    /// Mount the first of `parts` that fits on the hull's surface as `mount`
    /// allows, where `aim` scores it best and no exit lane is blocked.
    /// Candidates of one score are tried in a seeded order. Returns whether a
    /// part was mounted.
    fn fit<'a>(
        &self,
        grid: &mut Grid<'a>,
        parts: &[&'a ShipPart],
        slot: &str,
        aim: fn(&SectionKind, Option<usize>) -> Option<u32>,
        mount: MountType,
        aspect: &[u8],
    ) -> bool {
        let backed = mount != MountType::Mirrored;
        let lone = mount == MountType::BackedLone;
        let surface = grid.surface();
        for part in parts {
            let mut candidates = Vec::new();
            for rotation in cube_rotations() {
                let Ok(oriented) = oriented_part(&part.config, rotation) else {
                    continue;
                };
                let Some(score) = aim(&part.config.kind, oriented.aims) else {
                    continue;
                };
                let span = oriented.span.as_ivec3();
                let centred = reflects(&oriented, &oriented);
                let port = oriented_part(&part.config, mirror_rotation(rotation))
                    .ok()
                    .filter(|port| reflects(&oriented, port));
                let mut anchors = BTreeSet::new();
                for cell in &surface {
                    for own in &oriented.cells {
                        let anchor = IVec3::from_array(*cell) - own.cell.as_ivec3();
                        let on_centreline = anchor.x == -(anchor.x + span.x - 1);
                        let starboard = anchor.x >= 1;
                        let admitted = if lone {
                            anchor.x >= 0
                        } else {
                            (on_centreline && centred) || (starboard && port.is_some())
                        };
                        if admitted {
                            anchors.insert(anchor.to_array());
                        }
                    }
                }
                for anchor in anchors {
                    let anchor = IVec3::from_array(anchor);
                    if grid.fits(&oriented, anchor) && (!backed || grid.backed(&oriented, anchor)) {
                        candidates.push((score, oriented.clone(), port.clone(), anchor));
                    }
                }
            }
            let mut stream = self.stream(aspect);
            let mut candidates = shuffled(candidates, &mut stream);
            candidates.sort_by_key(|(score, ..)| std::cmp::Reverse(*score));

            for (_, oriented, port, anchor) in candidates {
                let span = oriented.span.as_ivec3();
                if anchor.x >= 1 && !lone {
                    let port = port.expect("a starboard candidate pairs");
                    let port_anchor = IVec3::new(-(anchor.x + span.x - 1), anchor.y, anchor.z);
                    grid.insert(format!("{slot}_starboard"), part, oriented, anchor, false);
                    if grid.last_blocks_an_exit() {
                        grid.pop();
                        continue;
                    }
                    if !grid.fits(&port, port_anchor)
                        || (backed && !grid.backed(&port, port_anchor))
                    {
                        grid.pop();
                        continue;
                    }
                    grid.insert(format!("{slot}_port"), part, port, port_anchor, false);
                    if grid.last_blocks_an_exit() {
                        grid.pop();
                        grid.pop();
                        continue;
                    }
                } else {
                    grid.insert(slot.to_string(), part, oriented, anchor, false);
                    if grid.last_blocks_an_exit() {
                        grid.pop();
                        continue;
                    }
                }
                return true;
            }
        }
        false
    }
}

/// Every constraint an intact generated design must hold, in the order they
/// are checked.
fn check(
    snapshot: &ShipPartSnapshot,
    request: ShipLayoutRequest,
    design: &ShipDesign,
) -> Result<(), ShipLayoutConstraintType> {
    use ShipLayoutConstraintType as Constraint;

    let (resolved, bounds) = resolve_and_mate(snapshot, request, design)?;
    let docks = resolved
        .iter()
        .filter(|part| part.family == ShipPartFamilyType::Docking)
        .count();
    for (section, part) in design.sections.iter().zip(&resolved) {
        // A ship with one docking port carries it with no mirror image.
        if part.family == ShipPartFamilyType::Docking && docks == 1 {
            continue;
        }
        let image = section.position * Vec3::new(-1.0, 1.0, 1.0);
        let oriented = oriented_part(&part.config, section.rotation)
            .expect("a snapshot part at a generated rotation stands on the grid");
        let twin = design
            .sections
            .iter()
            .zip(&resolved)
            .any(|(other, other_part)| {
                prototype_of(other) == prototype_of(section)
                    && other.position.abs_diff_eq(image, GRID_EPSILON)
                    && oriented_part(&other_part.config, other.rotation)
                        .is_ok_and(|other| reflects(&oriented, &other))
            });
        if !twin {
            return Err(Constraint::Asymmetric(section.id.clone()));
        }
    }

    let (cells, cubes) = filled_cells(snapshot, design);
    for (section, part) in design.sections.iter().zip(&resolved) {
        if part.family == ShipPartFamilyType::CargoIntake && !side_backed(&cubes, part, section) {
            return Err(Constraint::UnbackedIntake(section.id.clone()));
        }
        if part.family == ShipPartFamilyType::Docking && !side_backed(&cubes, part, section) {
            return Err(Constraint::UnbackedDock(section.id.clone()));
        }
    }

    connected_and_clear(request, design, &resolved, bounds)?;
    if let Some(shortfall) = ShipMeasure::of(resolved).shortfall() {
        return Err(shortfall);
    }
    let loose: BTreeSet<[i32; 3]> = cubes
        .difference(&dock_backing(snapshot, design))
        .copied()
        .collect();
    if wreck_floor(&cells, &loose, 1) == 0 {
        return Err(Constraint::Unruinable);
    }
    Ok(())
}

/// Whether `section`, a `part`, opens out of the hull's side with a
/// structural cube of `cubes` behind every cell of its back.
fn side_backed(
    cubes: &BTreeSet<[i32; 3]>,
    part: &ShipPart,
    section: &SpaceshipSectionConfig,
) -> bool {
    side_back(part, section)
        .is_some_and(|back| back.iter().all(|cell| cubes.contains(&cell.to_array())))
}

/// The cells behind the back of `section`, a `part`, outside its own cells,
/// or `None` when it does not open out of the hull's side.
fn side_back(part: &ShipPart, section: &SpaceshipSectionConfig) -> Option<Vec<IVec3>> {
    let oriented = oriented_part(&part.config, section.rotation)
        .expect("a snapshot part at a generated rotation stands on the grid");
    let exit @ (STARBOARD | PORT) = oriented.aims? else {
        return None;
    };
    let anchor = (section.position - oriented.origin).round().as_ivec3();
    let own: HashSet<IVec3> = oriented
        .cells
        .iter()
        .map(|cell| anchor + cell.cell.as_ivec3())
        .collect();
    Some(
        own.iter()
            .map(|cell| *cell + STEPS[exit ^ 1])
            .filter(|behind| !own.contains(behind))
            .collect(),
    )
}

/// Every constraint a wreck of `intact` must hold: an intact ship's, but for
/// its mirror symmetry, plus `needed` omissions off its mirror image. Its
/// omissions keep the intact cell bounds and every docking port's backing,
/// and remove structural cubes only; any of those failing is a generator
/// fault, not a failed layout.
fn check_wreck(
    snapshot: &ShipPartSnapshot,
    request: ShipLayoutRequest,
    intact: &ShipDesign,
    design: &ShipDesign,
    needed: usize,
) -> Result<(), ShipLayoutConstraintType> {
    let (resolved, bounds) = resolve_and_mate(snapshot, request, design)?;
    connected_and_clear(request, design, &resolved, bounds)?;

    let (intact_parts, intact_bounds) = resolve_and_mate(snapshot, request, intact)
        .expect("a wreck is ruined from a valid intact design");
    assert_eq!(
        bounds, intact_bounds,
        "a wreck keeps its intact cell bounds"
    );
    let kept: HashSet<&str> = design
        .sections
        .iter()
        .map(|section| section.id.as_str())
        .collect();
    let mut omitted = 0;
    for (section, part) in intact.sections.iter().zip(&intact_parts) {
        if kept.contains(section.id.as_str()) {
            continue;
        }
        assert!(
            is_structural_cube(part),
            "a wreck omits structural cubes only, not '{}'",
            section.id
        );
        let image = section.position * Vec3::new(-1.0, 1.0, 1.0);
        omitted += usize::from(
            design
                .sections
                .iter()
                .any(|other| other.position.abs_diff_eq(image, GRID_EPSILON)),
        );
    }
    let (_, cubes) = filled_cells(snapshot, design);
    for (section, part) in design.sections.iter().zip(&resolved) {
        assert!(
            part.family != ShipPartFamilyType::Docking || side_backed(&cubes, part, section),
            "a wreck keeps the cubes across docking port '{}'",
            section.id
        );
    }
    if omitted < needed {
        return Err(ShipLayoutConstraintType::Unruined { omitted, needed });
    }
    Ok(())
}

/// The part of every section of `design` and the minimum and maximum cells
/// they fill: unique section ids, usable and eligible prototypes, the role's
/// families, no shared cell and mated contacts.
fn resolve_and_mate<'a>(
    snapshot: &'a ShipPartSnapshot,
    request: ShipLayoutRequest,
    design: &ShipDesign,
) -> Result<(Vec<&'a ShipPart>, (IVec3, IVec3)), ShipLayoutConstraintType> {
    use ShipLayoutConstraintType as Constraint;

    let mut ids = HashSet::new();
    for section in &design.sections {
        if !ids.insert(section.id.as_str()) {
            return Err(Constraint::DuplicateSectionId(section.id.clone()));
        }
    }

    let parts: HashMap<&str, &ShipPart> = snapshot
        .parts()
        .iter()
        .map(|part| (part.id(), part))
        .collect();
    let mut resolved = Vec::with_capacity(design.sections.len());
    for section in &design.sections {
        let prototype = match &section.source {
            SectionSource::Prototype { id, .. } => id.as_str(),
            SectionSource::Inline(config) => config.base.id.as_str(),
        };
        let part = match (&section.source, parts.get(prototype)) {
            (SectionSource::Prototype { .. }, Some(part)) => *part,
            _ => {
                return Err(Constraint::UnknownPrototype {
                    section: section.id.clone(),
                    prototype: prototype.to_string(),
                })
            }
        };
        if part.advancement > request.advancement {
            return Err(Constraint::AboveAdvancement(section.id.clone()));
        }
        if !part.family.allowed_on(request.role) {
            return Err(Constraint::ForbiddenPart {
                section: section.id.clone(),
                family: part.family,
            });
        }
        resolved.push(part);
    }
    for family in ShipPartFamilyType::ALL {
        if family.required_by(request.role) && !resolved.iter().any(|part| part.family == family) {
            return Err(Constraint::MissingPart(family));
        }
    }

    // The cells each section fills, and what each presents.
    let mut filled: HashMap<IVec3, (usize, [bool; 6], Option<usize>)> = HashMap::new();
    for (index, (section, part)) in design.sections.iter().zip(&resolved).enumerate() {
        let oriented = oriented_part(&part.config, section.rotation)
            .expect("a snapshot part at a generated rotation stands on the grid");
        let anchor = (section.position - oriented.origin).round().as_ivec3();
        for cell in &oriented.cells {
            let at = anchor + cell.cell.as_ivec3();
            if let Some((other, ..)) = filled.insert(at, (index, cell.faces, cell.exit)) {
                return Err(Constraint::Overlap {
                    first: design.sections[other].id.clone(),
                    second: section.id.clone(),
                });
            }
        }
    }
    let mut cells: Vec<_> = filled.iter().collect();
    cells.sort_by_key(|(cell, _)| cell.to_array());
    for (cell, (index, faces, exit)) in &cells {
        for (face, step) in STEPS.iter().enumerate() {
            let Some((other, other_faces, other_exit)) = filled.get(&(**cell + *step)) else {
                continue;
            };
            if other == index {
                continue;
            }
            if *exit == Some(face)
                || *other_exit == Some(face ^ 1)
                || faces[face] != other_faces[face ^ 1]
            {
                let (first, second) = ((*index).min(*other), (*index).max(*other));
                return Err(Constraint::UnmatedContact {
                    first: design.sections[first].id.clone(),
                    second: design.sections[second].id.clone(),
                });
            }
        }
    }

    let bounds = filled
        .keys()
        .fold((IVec3::MAX, IVec3::MIN), |(low, high), cell| {
            (low.min(*cell), high.max(*cell))
        });
    Ok((resolved, bounds))
}

/// The constraints after the parts mate: one connected socket graph, clear
/// exit lanes and the advancement's size ceiling.
fn connected_and_clear(
    request: ShipLayoutRequest,
    design: &ShipDesign,
    resolved: &[&ShipPart],
    (low, high): (IVec3, IVec3),
) -> Result<(), ShipLayoutConstraintType> {
    use ShipLayoutConstraintType as Constraint;

    let links: Vec<PlacedSectionLinkPoints> = design
        .sections
        .iter()
        .zip(resolved)
        .map(|(section, part)| PlacedSectionLinkPoints {
            position: section.position,
            rotation: section.rotation,
            link_points: &part.config.base.link_points,
        })
        .collect();
    if let Err(errors) = derive_link_point_graph(&links) {
        return Err(match errors.as_slice() {
            [LinkPointGraphError::Disconnected { components }] => Constraint::Disconnected {
                components: components.len(),
            },
            _ => Constraint::InvalidSockets,
        });
    }

    let placed: Vec<PlacedPart> = design
        .sections
        .iter()
        .zip(resolved)
        .map(|(section, part)| placed_part(&part.config, section.position, section.rotation))
        .collect();
    let (structure, _, occupied) = read_structure(&placed);
    let exits = ship_exits(&placed, &occupied);
    if let Some(blocked) = blocked_exits(&structure, &exits).first() {
        let index = occupied
            .iter()
            .position(|cells| cells.contains(&blocked.part))
            .expect("a blocked exit stands in a section's cell");
        return Err(Constraint::BlockedExit(design.sections[index].id.clone()));
    }

    let clearance = clearance(low, high);
    let ceiling = ceiling(request.advancement);
    if clearance.0 > ceiling.0 {
        return Err(Constraint::TooLarge { clearance, ceiling });
    }
    Ok(())
}

/// The prototype id a section names, or `None` for an inline section.
fn prototype_of(section: &SpaceshipSectionConfig) -> Option<&str> {
    match &section.source {
        SectionSource::Prototype { id, .. } => Some(id),
        SectionSource::Inline(_) => None,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use nova_events::prelude::MetersPerSecond;
    use nova_ship::prelude::{
        BaseSectionConfig, CargoIntakeSectionConfig, ControllerSectionConfig, DockingSectionConfig,
        HullSectionConfig, LinkPoint, MuzzleConfig, SectionCollider, ThrusterSectionConfig,
        TurretSectionConfig, CELL_FACES,
    };

    use super::*;
    use crate::ship_parts::ShipPartPack;

    /// A block of `size` cells with a socket on the centre of every cell
    /// face it shows on each of `faces`.
    fn block(
        id: &str,
        health: f32,
        kind: SectionKind,
        size: UVec3,
        faces: &[Vec3],
    ) -> SectionConfig {
        let extent = size.as_vec3();
        let mut link_points = Vec::new();
        for x in 0..size.x {
            for y in 0..size.y {
                for z in 0..size.z {
                    let centre = UVec3::new(x, y, z).as_vec3() - (extent - Vec3::ONE) * 0.5;
                    for face in faces {
                        let outer = (centre + *face).abs().cmpgt(extent * 0.5).any();
                        if outer {
                            link_points.push(LinkPoint {
                                id: format!("link_{}", link_points.len()),
                                position: centre + *face * 0.5,
                                normal: *face,
                            });
                        }
                    }
                }
            }
        }
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                health,
                collider: (size != UVec3::ONE).then_some(SectionCollider::Cuboid { size: extent }),
                link_points,
                ..default()
            },
            kind,
        }
    }

    const ALL_FACES: [Vec3; 6] = CELL_FACES;

    pub(crate) fn cube(id: &str, health: f32) -> SectionConfig {
        let kind = SectionKind::Hull(HullSectionConfig::default());
        block(id, health, kind, UVec3::ONE, &ALL_FACES)
    }

    pub(crate) fn controller(id: &str) -> SectionConfig {
        let kind = SectionKind::Controller(ControllerSectionConfig::default());
        block(id, 100.0, kind, UVec3::ONE, &ALL_FACES)
    }

    pub(crate) fn drive(id: &str, magnitude: f32, size: UVec3) -> SectionConfig {
        let kind = SectionKind::Thruster(ThrusterSectionConfig {
            magnitude,
            ..default()
        });
        block(id, 100.0, kind, size, &[Vec3::NEG_Z])
    }

    pub(crate) fn turret(id: &str, fire_rate: f32) -> SectionConfig {
        let mut turret = TurretSectionConfig {
            bullet_damage: 10.0,
            ..default()
        };
        turret.root.muzzle = Some(MuzzleConfig {
            id: "muzzle".to_string(),
            fire_rate,
            muzzle_effect: None,
        });
        block(
            id,
            100.0,
            SectionKind::Turret(turret),
            UVec3::ONE,
            &[Vec3::NEG_Y],
        )
    }

    pub(crate) fn intake(id: &str) -> SectionConfig {
        let kind = SectionKind::CargoIntake(CargoIntakeSectionConfig {
            render_mesh: "intake.glb#Scene0".into(),
            render_mesh_transform: None,
            canister_mesh: "canister.glb#Scene0".into(),
            door_sound: "door.wav".into(),
            eject_sound: "eject.wav".into(),
            take_sound: "take.wav".into(),
            detection_range: Meters(40.0),
            capture_gap: Meters(1.0),
            aperture_width: Meters(8.0),
            aperture_height: Meters(8.0),
            eject_speed: MetersPerSecond(3.0),
        });
        let faces = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z];
        block(id, 100.0, kind, UVec3::new(3, 2, 1), &faces)
    }

    /// A port with a socket on each of `faces`; it docks through -Z.
    pub(crate) fn dock(id: &str, faces: &[Vec3]) -> SectionConfig {
        let kind = SectionKind::Docking(DockingSectionConfig::default());
        block(id, 90.0, kind, UVec3::ONE, faces)
    }

    /// Every face but the mouth, so the port is its own mirror image.
    pub(crate) const DOCK_FACES: [Vec3; 5] = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z];

    pub(crate) fn pack(
        id: &str,
        dependencies: &[&str],
        sections: Vec<SectionConfig>,
    ) -> ShipPartPack {
        ShipPartPack {
            id: id.to_string(),
            dependencies: dependencies.iter().map(|id| id.to_string()).collect(),
            sections,
        }
    }

    /// Base content with a weak and a strong part of every family, and a mod
    /// with its own hull, drive and turret.
    pub(crate) fn packs() -> Vec<ShipPartPack> {
        vec![
            pack(
                "base",
                &[],
                vec![
                    cube("cube", 100.0),
                    cube("heavy_cube", 400.0),
                    controller("controller"),
                    drive("small_drive", 1.0, UVec3::ONE),
                    drive("wide_drive", 9.0, UVec3::new(3, 3, 2)),
                    turret("turret", 1.0),
                    turret("fast_turret", 4.0),
                    intake("intake"),
                    dock("dock", &DOCK_FACES),
                ],
            ),
            pack(
                "mod",
                &["base"],
                vec![
                    cube("mod_cube", 200.0),
                    drive("mod_drive", 3.0, UVec3::ONE),
                    turret("mod_turret", 2.0),
                ],
            ),
        ]
    }

    fn request(seed: u32, role: ShipRoleType, advancement: f32) -> ShipLayoutRequest {
        ShipLayoutRequest {
            seed,
            civilization: CivilizationId {
                world_seed: 7,
                node: [1, -2, 0],
            },
            role,
            advancement,
        }
    }

    #[test]
    fn pack_and_section_order_do_not_change_a_generated_ship() {
        let forward = ShipPartSnapshot::build(&packs()).expect("the packs build");
        let mut reversed = packs();
        reversed.reverse();
        for pack in &mut reversed {
            pack.sections.reverse();
        }
        let backward = ShipPartSnapshot::build(&reversed).expect("the reversed packs build");

        let mut layouts = BTreeSet::new();
        let mut sources = BTreeSet::new();
        for seed in 0..12 {
            for role in ShipRoleType::ALL {
                let request = request(seed, role, 1.0);
                let one = generate_ship(&forward, request).expect("the ship generates");
                let other = generate_ship(&backward, request).expect("the ship generates");
                assert_eq!(
                    format!("{:?}", one.design),
                    format!("{:?}", other.design),
                    "seed {seed}, {role:?}"
                );
                assert_eq!((one.drive, &one.source), (other.drive, &other.source));
                layouts.insert(one.drive);
                sources.insert(one.source);
            }
        }
        assert_eq!(layouts.len(), 2, "the seeds reach both drive layouts");
        assert_eq!(sources.len(), 2, "the seeds prefer both sources");
    }

    #[test]
    fn a_generated_ship_carries_only_what_its_role_and_advancement_allow() {
        let snapshot = ShipPartSnapshot::build(&packs()).expect("the packs build");
        let parts: HashMap<&str, &ShipPart> = snapshot
            .parts()
            .iter()
            .map(|part| (part.id(), part))
            .collect();
        for advancement in [0.0, 1.0] {
            for seed in 0..8 {
                for role in ShipRoleType::ALL {
                    let layout = generate_ship(&snapshot, request(seed, role, advancement))
                        .expect("the ship generates");
                    let carried: Vec<&ShipPart> = layout
                        .design
                        .sections
                        .iter()
                        .map(|section| parts[prototype_of(section).expect("a prototype")])
                        .collect();
                    let count =
                        |family| carried.iter().filter(|part| part.family == family).count();
                    let context = format!("seed {seed}, {role:?}, advancement {advancement}");

                    assert!(count(ShipPartFamilyType::Controller) >= 1, "{context}");
                    assert!(count(ShipPartFamilyType::Thruster) >= 1, "{context}");
                    let armed = matches!(role, ShipRoleType::Scavenger | ShipRoleType::Armored);
                    assert_eq!(count(ShipPartFamilyType::Weapon) >= 1, armed, "{context}");
                    if role == ShipRoleType::Industrial {
                        assert!(count(ShipPartFamilyType::CargoIntake) >= 1, "{context}");
                    }
                    assert!(
                        carried.iter().all(|part| part.advancement <= advancement),
                        "{context}"
                    );
                }
            }
        }

        // A fighter with no eligible weapon is refused, not built unarmed.
        let mut unarmed = packs();
        for pack in &mut unarmed {
            pack.sections
                .retain(|section| !matches!(section.kind, SectionKind::Turret(_)));
        }
        let unarmed = ShipPartSnapshot::build(&unarmed).expect("the unarmed packs build");
        let failure = generate_ship(&unarmed, request(0, ShipRoleType::Scavenger, 1.0))
            .expect_err("a scavenger needs a weapon");
        assert_eq!(failure.attempts, 0);
        assert_eq!(
            failure.constraint,
            ShipLayoutConstraintType::NoEligiblePart(ShipPartFamilyType::Weapon)
        );
    }

    #[test]
    fn a_ship_no_layout_can_fit_fails_after_the_attempt_bound_with_its_constraint() {
        let huge = ShipPartSnapshot::build(&[pack(
            "base",
            &[],
            vec![
                cube("cube", 100.0),
                controller("controller"),
                drive("huge_drive", 1.0, UVec3::new(15, 15, 1)),
                intake("intake"),
                dock("dock", &DOCK_FACES),
            ],
        )])
        .expect("the packs build");

        let request = request(42, ShipRoleType::Civilian, 0.0);
        let failure = generate_ship(&huge, request).expect_err("no hull fits the ceiling");

        assert_eq!(failure.attempts, MAX_LAYOUT_ATTEMPTS);
        assert!(
            matches!(
                failure.constraint,
                ShipLayoutConstraintType::TooLarge { .. }
            ),
            "{failure}"
        );
        let message = failure.to_string();
        for named in ["ship seed 42", "civ_1_n2_0@7", "civilian", "ceiling"] {
            assert!(message.contains(named), "{message}");
        }
    }

    #[test]
    fn every_role_wears_its_style_intact_and_wrecked() {
        let snapshot = ShipPartSnapshot::build(&packs()).expect("the packs build");
        for (role, style) in [
            (ShipRoleType::Civilian, "civilian"),
            (ShipRoleType::Industrial, "industrial"),
            (ShipRoleType::Scavenger, "salvage"),
            (ShipRoleType::Armored, "armoured"),
        ] {
            let request = request(1, role, 1.0);
            let intact = generate_ship(&snapshot, request).expect("the ship generates");
            let wreck = generate_wreck(&snapshot, request).expect("the wreck generates");
            for (condition, layout) in [("intact", intact), ("wrecked", wreck)] {
                let presentation = &layout.design.presentation;
                assert!(presentation.skin, "{role:?} {condition} stands bare");
                assert_eq!(
                    presentation.style.as_deref(),
                    Some(style),
                    "{role:?} {condition} wears the wrong style"
                );
            }
        }
    }

    #[test]
    fn a_wreck_keeps_its_fittings_size_and_connection_and_breaks_its_mirror() {
        let snapshot = ShipPartSnapshot::build(&packs()).expect("the packs build");
        let mut reversed = packs();
        reversed.reverse();
        for pack in &mut reversed {
            pack.sections.reverse();
        }
        let backward = ShipPartSnapshot::build(&reversed).expect("the reversed packs build");
        let parts: HashMap<&str, &ShipPart> = snapshot
            .parts()
            .iter()
            .map(|part| (part.id(), part))
            .collect();
        for advancement in [0.0, 1.0] {
            let mut ruined = BTreeSet::new();
            for seed in 0..8 {
                for role in ShipRoleType::ALL {
                    let request = request(seed, role, advancement);
                    let context = format!("seed {seed}, {role:?}, advancement {advancement}");
                    let intact = generate_ship(&snapshot, request).expect("the ship generates");
                    let result = generate_wreck(&snapshot, request);
                    assert_eq!(
                        format!("{result:?}"),
                        format!("{:?}", generate_wreck(&backward, request)),
                        "{context}"
                    );
                    let wreck = result.unwrap_or_else(|failure| panic!("{context}: {failure}"));
                    ruined.insert(role.label());
                    assert_eq!(
                        (wreck.drive, &wreck.source, wreck.clearance, wreck.attempt),
                        (
                            intact.drive,
                            &intact.source,
                            intact.clearance,
                            intact.attempt
                        ),
                        "{context}"
                    );

                    let kept: HashMap<&str, &SpaceshipSectionConfig> = wreck
                        .design
                        .sections
                        .iter()
                        .map(|section| (section.id.as_str(), section))
                        .collect();
                    assert_eq!(kept.len(), wreck.design.sections.len(), "{context}");
                    for section in &intact.design.sections {
                        let part = parts[prototype_of(section).expect("a prototype")];
                        match kept.get(section.id.as_str()) {
                            Some(kept) => {
                                assert_eq!(format!("{kept:?}"), format!("{section:?}"), "{context}")
                            }
                            None => assert!(is_structural_cube(part), "{context}: {}", section.id),
                        }
                    }
                    let unmirrored = wreck
                        .design
                        .sections
                        .iter()
                        .filter(|section| {
                            let image = section.position * Vec3::new(-1.0, 1.0, 1.0);
                            !wreck
                                .design
                                .sections
                                .iter()
                                .any(|other| other.position.abs_diff_eq(image, GRID_EPSILON))
                        })
                        .count();
                    // A large hull keeps its full target; a thin one still
                    // breaks its mirror.
                    let (cells, cubes) = filled_cells(&snapshot, &intact.design);
                    let target = wreck_omissions(cubes.len());
                    let loose: BTreeSet<[i32; 3]> = cubes
                        .difference(&dock_backing(&snapshot, &intact.design))
                        .copied()
                        .collect();
                    let needed = if advancement == 1.0 {
                        target
                    } else {
                        wreck_floor(&cells, &loose, target)
                            .min(cumulative_ruin(&cells, &loose, target).len())
                    };
                    assert!(unmirrored >= needed, "{context}: {unmirrored} < {needed}");
                    let (_, wreck_cubes) = filled_cells(&snapshot, &wreck.design);
                    let docks: Vec<&SpaceshipSectionConfig> = wreck
                        .design
                        .sections
                        .iter()
                        .filter(|section| {
                            parts[prototype_of(section).expect("a prototype")].family
                                == ShipPartFamilyType::Docking
                        })
                        .collect();
                    assert_eq!(docks.len(), 2, "{context}: the intact pair of ports");
                    for dock in docks {
                        let part = parts[prototype_of(dock).expect("a prototype")];
                        assert!(
                            side_backed(&wreck_cubes, part, dock),
                            "{context}: {}",
                            dock.id
                        );
                    }
                    let links: Vec<PlacedSectionLinkPoints> = wreck
                        .design
                        .sections
                        .iter()
                        .map(|section| PlacedSectionLinkPoints {
                            position: section.position,
                            rotation: section.rotation,
                            link_points: &parts[prototype_of(section).expect("a prototype")]
                                .config
                                .base
                                .link_points,
                        })
                        .collect();
                    assert!(derive_link_point_graph(&links).is_ok(), "{context}");
                }
            }
            assert_eq!(
                ruined.len(),
                ShipRoleType::ALL.len(),
                "advancement {advancement}"
            );
        }
    }

    #[test]
    fn a_layout_with_no_removable_outer_cube_is_refused_as_unruinable() {
        // A drive six cells deep shrinks the hull to its three stations. On
        // this seed one layout docks its ports off the body's flanks, so every
        // cube holds a bound, the drive or a port's back, and its wreck could
        // not lose even one. The ship is drawn again instead.
        let short = ShipPartSnapshot::build(&[pack(
            "base",
            &[],
            vec![
                cube("cube", 100.0),
                controller("controller"),
                drive("deep_drive", 1.0, UVec3::new(1, 1, 6)),
                intake("intake"),
                dock("dock", &DOCK_FACES),
            ],
        )])
        .expect("the packs build");

        let request = request(2, ShipRoleType::Civilian, 0.0);
        let refused = (0..MAX_LAYOUT_ATTEMPTS)
            .find(|attempt| {
                Draw::new(&short, request, *attempt)
                    .layout()
                    .and_then(|layout| check(&short, request, &layout.design))
                    == Err(ShipLayoutConstraintType::Unruinable)
            })
            .expect("one layout cannot be ruined");
        let intact = generate_ship(&short, request).expect("a later layout generates");

        assert!(intact.attempt > refused, "{} <= {refused}", intact.attempt);
        generate_wreck(&short, request).expect("the generated ship can be ruined");
    }

    #[test]
    fn a_generated_ship_has_the_thrust_and_flight_computers_its_mass_needs() {
        let snapshot = ShipPartSnapshot::build(&packs()).expect("the packs build");
        let parts: HashMap<&str, &ShipPart> = snapshot
            .parts()
            .iter()
            .map(|part| (part.id(), part))
            .collect();
        let (mut most_drives, mut most_controllers) = (0, 0);
        for advancement in [0.0, 1.0] {
            for seed in 0..8 {
                for role in ShipRoleType::ALL {
                    let layout = generate_ship(&snapshot, request(seed, role, advancement))
                        .expect("the ship generates");
                    let carried: Vec<&ShipPart> = layout
                        .design
                        .sections
                        .iter()
                        .map(|section| parts[prototype_of(section).expect("a prototype")])
                        .collect();
                    let measure = ShipMeasure::of(carried.iter().copied());
                    let context = format!("seed {seed}, {role:?}, advancement {advancement}");
                    assert!(
                        measure.thrust >= MIN_THRUST_PER_CELL * measure.mass,
                        "{context}: {measure:?}"
                    );
                    assert!(
                        measure.controllers as f32 * CELLS_PER_CONTROLLER >= measure.mass,
                        "{context}: {measure:?}"
                    );
                    let drives = carried
                        .iter()
                        .filter(|part| part.family == ShipPartFamilyType::Thruster)
                        .count();
                    most_drives = most_drives.max(drives);
                    most_controllers = most_controllers.max(measure.controllers);
                }
            }
        }
        assert!(most_drives > 2, "no hull needed more than a pair of drives");
        assert!(
            most_controllers > 1,
            "no hull needed a second flight computer"
        );
    }

    #[test]
    fn an_industrial_intake_opens_sideways_with_structural_cubes_across_its_back() {
        let snapshot = ShipPartSnapshot::build(&packs()).expect("the packs build");
        let parts: HashMap<&str, &ShipPart> = snapshot
            .parts()
            .iter()
            .map(|part| (part.id(), part))
            .collect();
        let intakes = |design: &ShipDesign| -> Vec<SpaceshipSectionConfig> {
            design
                .sections
                .iter()
                .filter(|section| {
                    parts[prototype_of(section).expect("a prototype")].family
                        == ShipPartFamilyType::CargoIntake
                })
                .cloned()
                .collect()
        };
        for advancement in [0.0, 1.0] {
            for seed in 0..8 {
                let request = request(seed, ShipRoleType::Industrial, advancement);
                let context = format!("seed {seed}, advancement {advancement}");
                let layout = generate_ship(&snapshot, request).expect("the ship generates");
                let (_, cubes) = filled_cells(&snapshot, &layout.design);
                let fitted = intakes(&layout.design);
                assert_eq!(fitted.len(), 2, "{context}: a mirrored pair");
                for intake in &fitted {
                    let part = parts[prototype_of(intake).expect("a prototype")];
                    assert!(
                        side_backed(&cubes, part, intake),
                        "{context}: {}",
                        intake.id
                    );
                }

                // Without the cubes behind them the same intakes are refused.
                let mut behind = HashSet::new();
                for intake in &fitted {
                    let part = parts[prototype_of(intake).expect("a prototype")];
                    let oriented = oriented_part(&part.config, intake.rotation)
                        .expect("the intake stands on the grid");
                    let back = STEPS[oriented.aims.expect("an intake opens") ^ 1];
                    let anchor = (intake.position - oriented.origin).round().as_ivec3();
                    for cell in &oriented.cells {
                        behind.insert(anchor + cell.cell.as_ivec3() + back);
                    }
                }
                let mut bare = layout.design.clone();
                bare.sections
                    .retain(|section| !behind.contains(&section.position.round().as_ivec3()));
                assert_eq!(
                    check(&snapshot, request, &bare),
                    Err(ShipLayoutConstraintType::UnbackedIntake(
                        fitted[0].id.clone()
                    )),
                    "{context}"
                );
            }
        }
    }

    #[test]
    fn a_high_advancement_ship_draws_its_drives_and_weapons_from_its_tier() {
        let snapshot = ShipPartSnapshot::build(&packs()).expect("the packs build");
        let parts: HashMap<&str, &ShipPart> = snapshot
            .parts()
            .iter()
            .map(|part| (part.id(), part))
            .collect();
        let mut drives = BTreeSet::new();
        for seed in 0..12 {
            for role in ShipRoleType::ALL {
                let layout =
                    generate_ship(&snapshot, request(seed, role, 1.0)).expect("the ship generates");
                for section in &layout.design.sections {
                    let part = parts[prototype_of(section).expect("a prototype")];
                    assert!(
                        !matches!(part.id(), "small_drive" | "turret"),
                        "seed {seed}, {role:?}: {} mounts {}",
                        section.id,
                        part.id()
                    );
                    if part.family == ShipPartFamilyType::Thruster {
                        drives.insert(part.id());
                    }
                }
            }
        }
        assert_eq!(
            drives,
            BTreeSet::from(["mod_drive", "wide_drive"]),
            "the seeds vary the drive"
        );
    }

    #[test]
    fn a_generated_hull_mixes_at_most_three_structural_cubes_from_its_source() {
        let mut packs = packs();
        packs[0].sections.push(cube("plate_cube", 150.0));
        packs[0].sections.push(cube("rib_cube", 300.0));
        let snapshot = ShipPartSnapshot::build(&packs).expect("the packs build");
        let parts: HashMap<&str, &ShipPart> = snapshot
            .parts()
            .iter()
            .map(|part| (part.id(), part))
            .collect();
        let mut widest = 0;
        for advancement in [0.0, 1.0] {
            for seed in 0..16 {
                for role in ShipRoleType::ALL {
                    let layout = generate_ship(&snapshot, request(seed, role, advancement))
                        .expect("the ship generates");
                    let cubes: BTreeSet<&str> = layout
                        .design
                        .sections
                        .iter()
                        .map(|section| parts[prototype_of(section).expect("a prototype")])
                        .filter(|part| is_structural_cube(part))
                        .map(|part| {
                            if role != ShipRoleType::Scavenger {
                                assert_eq!(part.source, layout.source, "seed {seed}, {role:?}");
                            }
                            part.id()
                        })
                        .collect();
                    let context = format!("seed {seed}, {role:?}, advancement {advancement}");
                    if advancement == 0.0 {
                        assert_eq!(cubes, BTreeSet::from(["cube"]), "{context}");
                    }
                    assert!(cubes.len() <= STRUCTURAL_PALETTE, "{context}: {cubes:?}");
                    widest = widest.max(cubes.len());
                }
            }
        }
        assert_eq!(widest, STRUCTURAL_PALETTE, "no hull mixed a full palette");
    }

    /// The docking sections of `design`, with their section index.
    fn docks<'s>(
        parts: &HashMap<&str, &ShipPart>,
        design: &'s ShipDesign,
    ) -> Vec<(usize, &'s SpaceshipSectionConfig)> {
        design
            .sections
            .iter()
            .enumerate()
            .filter(|(_, section)| {
                parts[prototype_of(section).expect("a prototype")].family
                    == ShipPartFamilyType::Docking
            })
            .collect()
    }

    #[test]
    fn every_generated_ship_docks_through_a_backed_flank_pair_down_a_clear_lane() {
        let snapshot = ShipPartSnapshot::build(&packs()).expect("the packs build");
        let parts: HashMap<&str, &ShipPart> = snapshot
            .parts()
            .iter()
            .map(|part| (part.id(), part))
            .collect();
        for advancement in [0.0, 1.0] {
            for seed in 0..8 {
                for role in ShipRoleType::ALL {
                    let request = request(seed, role, advancement);
                    let context = format!("seed {seed}, {role:?}, advancement {advancement}");
                    let layout = generate_ship(&snapshot, request).expect("the ship generates");
                    let (_, cubes) = filled_cells(&snapshot, &layout.design);
                    let docks = docks(&parts, &layout.design);
                    let ids: Vec<&str> = docks.iter().map(|(_, dock)| dock.id.as_str()).collect();
                    assert_eq!(ids, ["dock_starboard", "dock_port"], "{context}");

                    // Each port docks out of the hull's side down a lane the
                    // exit rule reads, and nothing stands in it.
                    let placed: Vec<PlacedPart> = layout
                        .design
                        .sections
                        .iter()
                        .map(|section| {
                            let part = parts[prototype_of(section).expect("a prototype")];
                            placed_part(&part.config, section.position, section.rotation)
                        })
                        .collect();
                    let (structure, _, occupied) = read_structure(&placed);
                    let exits = ship_exits(&placed, &occupied);
                    assert!(blocked_exits(&structure, &exits).is_empty(), "{context}");
                    for (index, dock) in &docks {
                        let part = parts[prototype_of(dock).expect("a prototype")];
                        assert!(side_backed(&cubes, part, dock), "{context}: {}", dock.id);
                        assert!(
                            exits.iter().any(|exit| {
                                occupied[*index].contains(&exit.cell)
                                    && matches!(exit.out, STARBOARD | PORT)
                            }),
                            "{context}: {} has no flank lane",
                            dock.id
                        );
                    }

                    // Without the cubes behind them the same ports are
                    // refused.
                    let behind: HashSet<IVec3> = docks
                        .iter()
                        .flat_map(|(_, dock)| {
                            let part = parts[prototype_of(dock).expect("a prototype")];
                            side_back(part, dock).expect("a port opens sideways")
                        })
                        .collect();
                    let mut bare = layout.design.clone();
                    bare.sections
                        .retain(|section| !behind.contains(&section.position.round().as_ivec3()));
                    assert_eq!(
                        check(&snapshot, request, &bare),
                        Err(ShipLayoutConstraintType::UnbackedDock(
                            "dock_starboard".to_string()
                        )),
                        "{context}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_short_industrial_spine_holds_a_flat_flank_for_its_intake_pair_and_a_dock() {
        // A clusters scan at world seed 17 failed this request with no intake
        // fitting: no body run was long enough for the pair and a port.
        let snapshot = ShipPartSnapshot::build(&packs()).expect("the packs build");
        let parts: HashMap<&str, &ShipPart> = snapshot
            .parts()
            .iter()
            .map(|part| (part.id(), part))
            .collect();
        let request = ShipLayoutRequest {
            seed: 1_049_378_793,
            civilization: CivilizationId {
                world_seed: 17,
                node: [0, 0, 0],
            },
            role: ShipRoleType::Industrial,
            advancement: 0.0,
        };
        let layout =
            generate_ship(&snapshot, request).unwrap_or_else(|failure| panic!("{failure}"));

        // Every intake and port opens out of the hull's side, backed, down a
        // lane nothing stands in.
        let (_, cubes) = filled_cells(&snapshot, &layout.design);
        let placed: Vec<PlacedPart> = layout
            .design
            .sections
            .iter()
            .map(|section| {
                let part = parts[prototype_of(section).expect("a prototype")];
                placed_part(&part.config, section.position, section.rotation)
            })
            .collect();
        let (structure, _, occupied) = read_structure(&placed);
        let exits = ship_exits(&placed, &occupied);
        assert!(blocked_exits(&structure, &exits).is_empty());
        let mut fitted = HashMap::new();
        for (index, section) in layout.design.sections.iter().enumerate() {
            let part = parts[prototype_of(section).expect("a prototype")];
            if matches!(
                part.family,
                ShipPartFamilyType::CargoIntake | ShipPartFamilyType::Docking
            ) {
                *fitted.entry(part.family).or_insert(0) += 1;
                assert!(side_backed(&cubes, part, section), "{}", section.id);
                assert!(
                    exits.iter().any(|exit| occupied[index].contains(&exit.cell)
                        && matches!(exit.out, STARBOARD | PORT)),
                    "{} has no flank lane",
                    section.id
                );
            }
        }
        assert_eq!(fitted[&ShipPartFamilyType::CargoIntake], 2);
        assert!(fitted[&ShipPartFamilyType::Docking] >= 1);
    }

    #[test]
    fn a_docking_port_with_no_mirrored_twin_mounts_alone() {
        // Sockets on the back and one side only: no rotation reflects the
        // port, so it cannot pair.
        let mut packs = packs();
        for section in &mut packs[0].sections {
            if section.base.id == "dock" {
                *section = dock("dock", &[Vec3::Z, Vec3::X]);
            }
        }
        let snapshot = ShipPartSnapshot::build(&packs).expect("the packs build");
        let parts: HashMap<&str, &ShipPart> = snapshot
            .parts()
            .iter()
            .map(|part| (part.id(), part))
            .collect();
        for advancement in [0.0, 1.0] {
            for seed in 0..8 {
                for role in ShipRoleType::ALL {
                    let context = format!("seed {seed}, {role:?}, advancement {advancement}");
                    let layout = generate_ship(&snapshot, request(seed, role, advancement))
                        .unwrap_or_else(|failure| panic!("{context}: {failure}"));
                    let (_, cubes) = filled_cells(&snapshot, &layout.design);
                    let docks = docks(&parts, &layout.design);
                    let [(_, dock)] = docks.as_slice() else {
                        panic!("{context}: {} ports", docks.len());
                    };
                    assert_eq!(dock.id, "dock", "{context}");
                    let part = parts[prototype_of(dock).expect("a prototype")];
                    assert!(side_backed(&cubes, part, dock), "{context}");
                }
            }
        }
    }

    #[test]
    fn a_wreck_whose_seeded_plans_all_fall_short_is_its_cumulative_pass() {
        // A sector fault: 4 outer cubes of this hull can each go alone and one
        // pass loses 4 together, but every seeded plan's earlier omissions
        // leave the rest holding its bounds or its socket graph.
        let snapshot = ShipPartSnapshot::build(&packs()).expect("the packs build");
        let request = ShipLayoutRequest {
            seed: 245_496_831,
            civilization: CivilizationId {
                world_seed: 4,
                node: [0, 0, 0],
            },
            role: ShipRoleType::Scavenger,
            advancement: 0.1,
        };
        let intact = generate_ship(&snapshot, request).expect("the ship generates");
        let (cells, cubes) = filled_cells(&snapshot, &intact.design);
        let target = wreck_omissions(cubes.len());
        let loose: BTreeSet<[i32; 3]> = cubes
            .difference(&dock_backing(&snapshot, &intact.design))
            .copied()
            .collect();
        let witness = cumulative_ruin(&cells, &loose, target);
        let floor = wreck_floor(&cells, &loose, target).min(witness.len());
        assert_eq!((floor, target), (4, 4));
        for attempt in 0..MAX_WRECK_ATTEMPTS {
            let (design, omitted) = ruin(&cells, &loose, request, &intact.design, attempt, target);
            assert_eq!(
                check_wreck(&snapshot, request, &intact.design, &design, floor),
                Err(ShipLayoutConstraintType::Unruined {
                    omitted,
                    needed: floor
                }),
                "attempt {attempt}"
            );
        }

        let wreck = generate_wreck(&snapshot, request).expect("the cumulative pass is the wreck");

        assert_eq!(
            format!("{:?}", wreck.design),
            format!("{:?}", omit(&intact.design, &witness))
        );
    }
}
