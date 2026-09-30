//! The open world's ship layouts: one generated [`ShipDesign`] from a
//! [`ShipPartSnapshot`], a role, an advancement and a stable hull seed.
//!
//! SHAPE FIRST. A layout plans a symmetric spine of structural cubes along the
//! ship's length, from a one-cell nose to a stern that matches its drive, and
//! varies the cross-section station by station. Then it mounts the drive, a
//! centred single or a mirrored pair, the flight computer, and the role's
//! weapons or cargo intake. Every part stands on the cell grid through
//! [`oriented_part`], so a mod part joins by its type, footprint and sockets.
//!
//! A layout draws a preferred source per hull and keeps civilian, industrial
//! and armored ships to it where that source has a usable part; scavengers mix
//! sources. Each source weighs the same, whatever its part count.
//!
//! Every layout is checked after it is built: unique section ids, usable and
//! eligible prototypes, the role's families, mated contacts, mirror symmetry,
//! one connected socket graph, clear exit lanes and the advancement's size
//! ceiling. A seeded layout that fails is redrawn a fixed number of times,
//! then the request fails with its seed, civilization, role and the last
//! failed constraint. Nothing substitutes an authored hull.
//!
//! WRECKS. [`generate_wreck`] ruins the intact ship of the same request: it
//! keeps the role, source, advancement, fittings and cell bounds, and omits
//! seeded breaches of structural cubes, each growing inward from the outer
//! hull, while the rest stays one connected ship. A wreck digs toward a
//! fifth of its cubes off its mirror image, so it cannot read as a sparse
//! intact hull. A thin hull that cannot give that many settles for as many
//! outer cubes as it can lose one at a time, down to one. A hull that cannot
//! lose even one off-centre outer cube fails the request; nothing rerolls or
//! skips it.
//! The generator has no weathered or damaged part to add; holes are its only
//! ruin cue.
//!
//! PURE, and read by tests and the `world_ships` debug example only: the open
//! world does not spawn generated ships or wrecks yet, and every size, share
//! and weight here is provisional until generated ships are reviewed.

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt,
};

use bevy::prelude::*;
use nova_events::prelude::Meters;
use nova_gameplay::prelude::{Fnv32, SeedStream};
use nova_scenario::prelude::{SectionSource, ShipDesign, SpaceshipSectionConfig};
use nova_ship::prelude::{
    blocked_exits, cube_rotations, derive_link_point_graph, exit_normal, mirror_face,
    mirror_rotation, oriented_part, placement_blocks_an_exit, read_structure, ship_exits,
    LinkPointGraphError, OrientedPart, PlacedPart, PlacedSectionLinkPoints, SectionConfig,
    SectionFootprint, SectionKind, GRID_EPSILON,
};

use crate::{
    civilizations::{CivilizationId, ShipRoleType},
    ship_parts::{ShipPart, ShipPartFamilyType, ShipPartSnapshot},
};

/// How many seeded layouts a request draws before it fails.
const MAX_LAYOUT_ATTEMPTS: u32 = 8;

/// The largest hull clearance radius at advancement 0: room for the minimum
/// flyable civilian and industrial ship.
const MIN_HULL_CLEARANCE: Meters = Meters(80.0);

/// The largest hull clearance radius at advancement 1, the open world's fixed
/// ship reservation.
const MAX_HULL_CLEARANCE: Meters = Meters(400.0);

/// Cells a spine plan keeps free on every side for the fittings mounted on
/// it, so a planned spine rarely fails the size ceiling once fitted.
const FITTING_REACH: i32 = 2;

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

/// How a generated ship carries its drive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShipDriveLayoutType {
    /// One drive on the centreline.
    CenteredSingle,
    /// Two drives mirrored across the centreline.
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
    /// The hull clearance radius: half the diagonal of its cell bounds.
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
    /// A wreck omits too few hull sections off its mirror image to read as a
    /// ruin: fewer than the hull's outer off-centre cubes allow, and never
    /// fewer than one.
    Unruined {
        /// The most omitted hull sections whose mirror image remains, over
        /// every omission plan.
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
    /// The constraint the last layout failed, or for a wreck whose every
    /// plan held but fell short, [`ShipLayoutConstraintType::Unruined`] with
    /// the most any plan omitted.
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
/// that gets there is the wreck. When none does, the plan with the most wins
/// if it reaches the hull's floor, see [`wreck_floor`].
///
/// Fails as the intact ship fails, with
/// [`ShipLayoutConstraintType::Unruined`] when no plan within the bound
/// reaches the floor, or with the last failed constraint when no plan holds.
pub fn generate_wreck(
    snapshot: &ShipPartSnapshot,
    request: ShipLayoutRequest,
) -> Result<ShipLayout, ShipLayoutFailure> {
    let intact = generate_ship(snapshot, request)?;
    let (cells, cubes) = filled_cells(snapshot, &intact.design);
    let target = wreck_omissions(cubes.len());
    let floor = wreck_floor(&cells, &cubes, target);
    let mut best: Option<(ShipDesign, usize)> = None;
    let mut most_short = None;
    let mut last = None;
    for attempt in 0..MAX_WRECK_ATTEMPTS {
        let (design, omitted) = ruin(&cells, &cubes, request, &intact.design, attempt, target);
        match check_wreck(snapshot, request, &intact.design, &design, floor) {
            Ok(()) if omitted >= target => return Ok(ShipLayout { design, ..intact }),
            Ok(()) => {
                if best.as_ref().is_none_or(|(_, most)| omitted > *most) {
                    best = Some((design, omitted));
                }
            }
            Err(ShipLayoutConstraintType::Unruined { omitted, .. }) => {
                most_short = most_short.max(Some(omitted));
            }
            Err(constraint) => last = Some(constraint),
        }
    }
    if let Some((design, _)) = best {
        return Ok(ShipLayout { design, ..intact });
    }
    let constraint = match most_short {
        Some(omitted) => ShipLayoutConstraintType::Unruined {
            omitted,
            needed: floor,
        },
        None => last.expect("at least one wreck attempt runs"),
    };
    Err(ShipLayoutFailure {
        request,
        attempts: MAX_WRECK_ATTEMPTS,
        constraint,
    })
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

/// The minimum and maximum cells of `cells`.
fn cell_bounds(cells: &HashMap<IVec3, (usize, [bool; 6])>) -> (IVec3, IVec3) {
    cells
        .keys()
        .fold((IVec3::MAX, IVec3::MIN), |(low, high), cell| {
            (low.min(*cell), high.max(*cell))
        })
}

/// The fewest omissions off its mirror image a wreck of the hull filling
/// `cells` must reach: `target`, or fewer on a hull with fewer mirror pairs of
/// off-centre outer cubes, one of which can go alone with the cell bounds and
/// one socket graph kept; never fewer than one. A hull with at least `target`
/// such pairs keeps its full target, so it cannot settle for a trivial
/// breach.
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
    pairs.len().clamp(1, target)
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
    let design = ShipDesign {
        sections: intact
            .sections
            .iter()
            .enumerate()
            .filter(|(index, _)| !gone.contains(index))
            .map(|(_, section)| section.clone())
            .collect(),
        ..intact.clone()
    };
    (design, off_mirror)
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

/// The clearance radius of the cells from `low` to `high`, both included.
fn clearance(low: IVec3, high: IVec3) -> Meters {
    Meters::from_engine(((high - low + IVec3::ONE).as_vec3() * 0.5).length())
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

    fn insert(&mut self, id: String, part: &'a ShipPart, oriented: OrientedPart, anchor: IVec3) {
        for cell in &oriented.cells {
            self.filled.insert(
                anchor + cell.cell.as_ivec3(),
                Filled {
                    faces: cell.faces,
                    exit: cell.exit,
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

/// How an intake may point: under the hull first, never into the plume.
fn intake_aim(_: &SectionKind, aims: Option<usize>) -> Option<u32> {
    match aims? {
        VENTRAL => Some(3),
        STARBOARD | PORT => Some(2),
        FORWARD | DORSAL => Some(1),
        _ => None,
    }
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
    /// then the other sources, each source's parts shuffled.
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
            .flat_map(|(_, parts)| shuffled(parts, &mut stream))
            .collect()
    }

    fn layout(&self) -> Result<ShipLayout, ShipLayoutConstraintType> {
        let request = self.request;

        // The structural cube every station is built from, and with it the
        // hull's preferred source.
        let cube = *self
            .ranked(ShipPartFamilyType::Hull, None, b"hull", is_structural_cube)
            .first()
            .ok_or(ShipLayoutConstraintType::Unplaced(ShipPartFamilyType::Hull))?;
        let source = cube.source.as_str();
        let cube_cell = oriented_part(&cube.config, Quat::IDENTITY)
            .expect("a structural cube stands on the grid");

        // The drive, turned to fire aft, and whether it is one or a pair. An
        // even-width drive has no centre column, so it only pairs.
        let mut drive_stream = self.stream(b"drive_layout");
        let (drive, drive_cells, layout) = self
            .ranked(ShipPartFamilyType::Thruster, Some(source), b"drive", |_| {
                true
            })
            .into_iter()
            .find_map(|part| {
                let oriented = cube_rotations()
                    .into_iter()
                    .filter_map(|rotation| oriented_part(&part.config, rotation).ok())
                    .find(|oriented| oriented.aims == Some(AFT))?;
                let port = oriented_part(&part.config, mirror_rotation(oriented.rotation)).ok()?;
                let pairs = reflects(&oriented, &port);
                let single = oriented.span.x % 2 == 1 && reflects(&oriented, &oriented);
                let layout = match (single, pairs) {
                    (true, true) if drive_stream.unit() < 0.5 => ShipDriveLayoutType::MirroredPair,
                    (true, _) => ShipDriveLayoutType::CenteredSingle,
                    (false, true) => ShipDriveLayoutType::MirroredPair,
                    (false, false) => return None,
                };
                Some((part, oriented, layout))
            })
            .ok_or(ShipLayoutConstraintType::Unplaced(
                ShipPartFamilyType::Thruster,
            ))?;
        let span = drive_cells.span.as_ivec3();
        let stern = Section {
            half_width: match layout {
                ShipDriveLayoutType::CenteredSingle => (span.x - 1) / 2,
                ShipDriveLayoutType::MirroredPair => span.x,
            },
            low: -(span.y - 1) / 2,
            high: -(span.y - 1) / 2 + span.y - 1,
        };

        let stations = self.plan(stern, span.z)?;
        let length = stations.len() as i32;

        // The spine, bow at station 0. The bridge cell is left for the flight
        // computer.
        let bridge = IVec3::new(0, 0, (length / 4).max(1));
        let mut grid = Grid::default();
        for (z, section) in stations.iter().enumerate() {
            let z = z as i32;
            for x in -section.half_width..=section.half_width {
                for y in section.low..=section.high {
                    let cell = IVec3::new(x, y, z);
                    if cell != bridge {
                        grid.insert(
                            format!("hull_z{z}_x{}_y{}", signed(x), signed(y)),
                            cube,
                            cube_cell.clone(),
                            cell,
                        );
                    }
                }
            }
        }

        // The drive behind the stern, its mount face against the stern
        // station. A pair leaves the centre column between its twins empty.
        let drive_anchor = IVec3::new(0, stern.low, length);
        let mounts = match layout {
            ShipDriveLayoutType::CenteredSingle => vec![(
                "drive".to_string(),
                drive_cells,
                drive_anchor - IVec3::X * ((span.x - 1) / 2),
            )],
            ShipDriveLayoutType::MirroredPair => {
                let port = oriented_part(&drive.config, mirror_rotation(drive_cells.rotation))
                    .expect("a paired drive stands on the grid mirrored");
                vec![
                    (
                        "drive_starboard".to_string(),
                        drive_cells,
                        drive_anchor + IVec3::X,
                    ),
                    (
                        "drive_port".to_string(),
                        port,
                        drive_anchor - IVec3::X * span.x,
                    ),
                ]
            }
        };
        for (id, oriented, anchor) in mounts {
            if !grid.fits(&oriented, anchor) {
                return Err(ShipLayoutConstraintType::Unplaced(
                    ShipPartFamilyType::Thruster,
                ));
            }
            grid.insert(id, drive, oriented, anchor);
            if grid.last_blocks_an_exit() {
                let id = grid.placements.last().expect("the drive").id.clone();
                return Err(ShipLayoutConstraintType::BlockedExit(id));
            }
        }

        // The flight computer: in the bridge cell when it fits there,
        // otherwise on the hull's surface like any fitting.
        let controllers = self.ranked(
            ShipPartFamilyType::Controller,
            Some(source),
            b"controller",
            |_| true,
        );
        let inside = controllers.iter().find_map(|part| {
            cube_rotations()
                .into_iter()
                .filter_map(|rotation| oriented_part(&part.config, rotation).ok())
                .find(|oriented| {
                    oriented.span == UVec3::ONE
                        && reflects(oriented, oriented)
                        && oriented.origin.abs_diff_eq(Vec3::ZERO, GRID_EPSILON)
                        && grid.fits(oriented, bridge)
                })
                .map(|oriented| (*part, oriented))
        });
        match inside {
            Some((part, oriented)) => grid.insert("controller".to_string(), part, oriented, bridge),
            None => {
                grid.insert(
                    format!("hull_z{}_x0_y0", bridge.z),
                    cube,
                    cube_cell.clone(),
                    bridge,
                );
                if !self.fit(
                    &mut grid,
                    &controllers,
                    "controller",
                    controller_aim,
                    b"controller_slot",
                ) {
                    return Err(ShipLayoutConstraintType::Unplaced(
                        ShipPartFamilyType::Controller,
                    ));
                }
            }
        }

        // Role equipment. The first weapon and the intake are required; a
        // later weapon slot that finds no room is left out.
        let weapon_slots = match request.role {
            ShipRoleType::Civilian | ShipRoleType::Industrial => 0,
            ShipRoleType::Scavenger => 1,
            ShipRoleType::Armored => 1 + (request.advancement.clamp(0.0, 1.0) * 2.0).round() as u32,
        };
        for slot in 0..weapon_slots {
            let aspect = format!("weapon_{slot}");
            let weapons = self.ranked(
                ShipPartFamilyType::Weapon,
                Some(source),
                aspect.as_bytes(),
                |_| true,
            );
            let placed = self.fit(
                &mut grid,
                &weapons,
                &format!("weapon_{slot}"),
                weapon_aim,
                format!("{aspect}_slot").as_bytes(),
            );
            if !placed && slot == 0 {
                return Err(ShipLayoutConstraintType::Unplaced(
                    ShipPartFamilyType::Weapon,
                ));
            }
        }
        if ShipPartFamilyType::CargoIntake.required_by(request.role) {
            let intakes = self.ranked(
                ShipPartFamilyType::CargoIntake,
                Some(source),
                b"intake",
                |_| true,
            );
            if !self.fit(&mut grid, &intakes, "intake", intake_aim, b"intake_slot") {
                return Err(ShipLayoutConstraintType::Unplaced(
                    ShipPartFamilyType::CargoIntake,
                ));
            }
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

        Ok(ShipLayout {
            design: ShipDesign {
                sections,
                ..default()
            },
            drive: layout,
            source: source.to_string(),
            clearance: clearance(low, high),
            attempt: self.attempt,
        })
    }

    /// The spine's stations, bow first, ending in `stern`: a nose that widens
    /// from one cell, a body whose cross-section changes along its length,
    /// and the stern the drive mates to. Shrunk until the spine and a drive
    /// `drive_depth` cells deep fit the advancement's size ceiling with room
    /// for fittings.
    fn plan(
        &self,
        stern: Section,
        drive_depth: i32,
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
        let mut length = (length.round() as i32).max(MIN_STATIONS);
        let mut width = (width.round() as i32).max(1) + widen;
        let mut height = (height.round() as i32).max(0) + heighten;
        // The body's change along its length: a waist or shoulder at the
        // middle and a taper or flare toward the stern.
        let middle = [stream.signed(), stream.signed()];
        let aft = [stream.unit(), stream.unit()];

        let ceiling = ceiling(request.advancement);
        loop {
            let stations = stations(length, width, height, stern, middle, aft);
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
            let planned = clearance(low - reach, high + reach);
            if planned.0 <= ceiling.0 {
                return Ok(stations);
            }
            if length > MIN_STATIONS {
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
    let nose = 1 + length / 5;
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

impl Draw<'_> {
    /// Mount the first of `parts` that fits on the hull's surface, on the
    /// centreline or as a mirrored pair, where `aim` scores it best and no
    /// exit lane is blocked. Candidates of one score are tried in a seeded
    /// order. Returns whether a part was mounted.
    fn fit<'a>(
        &self,
        grid: &mut Grid<'a>,
        parts: &[&'a ShipPart],
        slot: &str,
        aim: fn(&SectionKind, Option<usize>) -> Option<u32>,
        aspect: &[u8],
    ) -> bool {
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
                        if (on_centreline && centred) || (starboard && port.is_some()) {
                            anchors.insert(anchor.to_array());
                        }
                    }
                }
                for anchor in anchors {
                    let anchor = IVec3::from_array(anchor);
                    if grid.fits(&oriented, anchor) {
                        candidates.push((score, oriented.clone(), port.clone(), anchor));
                    }
                }
            }
            let mut stream = self.stream(aspect);
            let mut candidates = shuffled(candidates, &mut stream);
            candidates.sort_by_key(|(score, ..)| std::cmp::Reverse(*score));

            for (_, oriented, port, anchor) in candidates {
                let span = oriented.span.as_ivec3();
                if anchor.x >= 1 {
                    let port = port.expect("a starboard candidate pairs");
                    let port_anchor = IVec3::new(-(anchor.x + span.x - 1), anchor.y, anchor.z);
                    grid.insert(format!("{slot}_starboard"), part, oriented, anchor);
                    if grid.last_blocks_an_exit() {
                        grid.pop();
                        continue;
                    }
                    if !grid.fits(&port, port_anchor) {
                        grid.pop();
                        continue;
                    }
                    grid.insert(format!("{slot}_port"), part, port, port_anchor);
                    if grid.last_blocks_an_exit() {
                        grid.pop();
                        grid.pop();
                        continue;
                    }
                } else {
                    grid.insert(slot.to_string(), part, oriented, anchor);
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
    for (section, part) in design.sections.iter().zip(&resolved) {
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

    connected_and_clear(request, design, &resolved, bounds)
}

/// Every constraint a wreck of `intact` must hold: an intact ship's, but for
/// its mirror symmetry, plus `needed` omissions off its mirror image. Its
/// omissions keep the intact cell bounds and remove structural cubes only;
/// either failing is a generator fault, not a failed layout.
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
mod tests {
    use nova_events::prelude::MetersPerSecond;
    use nova_ship::prelude::{
        BaseSectionConfig, CargoIntakeSectionConfig, ControllerSectionConfig, HullSectionConfig,
        LinkPoint, MuzzleConfig, SectionCollider, ThrusterSectionConfig, TurretSectionConfig,
        CELL_FACES,
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

    fn cube(id: &str, health: f32) -> SectionConfig {
        let kind = SectionKind::Hull(HullSectionConfig::default());
        block(id, health, kind, UVec3::ONE, &ALL_FACES)
    }

    fn controller(id: &str) -> SectionConfig {
        let kind = SectionKind::Controller(ControllerSectionConfig::default());
        block(id, 100.0, kind, UVec3::ONE, &ALL_FACES)
    }

    fn drive(id: &str, magnitude: f32, size: UVec3) -> SectionConfig {
        let kind = SectionKind::Thruster(ThrusterSectionConfig {
            magnitude,
            ..default()
        });
        block(id, 100.0, kind, size, &[Vec3::NEG_Z])
    }

    fn turret(id: &str, fire_rate: f32) -> SectionConfig {
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

    fn intake(id: &str) -> SectionConfig {
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
            maximum_capture_speed: MetersPerSecond(5.0),
            eject_speed: MetersPerSecond(3.0),
        });
        let faces = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z];
        block(id, 100.0, kind, UVec3::new(3, 2, 1), &faces)
    }

    fn pack(id: &str, dependencies: &[&str], sections: Vec<SectionConfig>) -> ShipPartPack {
        ShipPartPack {
            id: id.to_string(),
            dependencies: dependencies.iter().map(|id| id.to_string()).collect(),
            sections,
        }
    }

    /// Base content with a weak and a strong part of every family, and a mod
    /// with its own hull, drive and turret.
    fn packs() -> Vec<ShipPartPack> {
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
                    // A thin low-advancement hull that cannot lose one
                    // off-centre outer cube fails, and only that one.
                    let wreck = match result {
                        Err(failure) if advancement == 0.0 => {
                            assert_eq!(
                                failure.constraint,
                                ShipLayoutConstraintType::Unruined {
                                    omitted: 0,
                                    needed: 1
                                },
                                "{context}"
                            );
                            continue;
                        }
                        result => result.unwrap_or_else(|failure| panic!("{context}: {failure}")),
                    };
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
                    let needed = if advancement == 1.0 {
                        target
                    } else {
                        wreck_floor(&cells, &cubes, target)
                    };
                    assert!(unmirrored >= needed, "{context}: {unmirrored} < {needed}");
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
    fn a_hull_with_no_removable_outer_cube_fails_as_unruined() {
        // A drive six cells deep shrinks the hull to its three stations, whose
        // every cube holds a bound or the drive, so not even one breach fits.
        let short = ShipPartSnapshot::build(&[pack(
            "base",
            &[],
            vec![
                cube("cube", 100.0),
                controller("controller"),
                drive("deep_drive", 1.0, UVec3::new(1, 1, 6)),
                intake("intake"),
            ],
        )])
        .expect("the packs build");

        let request = request(3, ShipRoleType::Civilian, 0.0);
        generate_ship(&short, request).expect("the intact ship generates");
        let failure = generate_wreck(&short, request).expect_err("no ruin fits the hull");

        assert_eq!(failure.attempts, MAX_WRECK_ATTEMPTS);
        assert!(
            matches!(
                failure.constraint,
                ShipLayoutConstraintType::Unruined {
                    omitted: 0,
                    needed: 1
                }
            ),
            "{failure}"
        );
        let message = failure.to_string();
        for named in ["ship seed 3", "civ_1_n2_0@7", "civilian", "visible ruin"] {
            assert!(message.contains(named), "{message}");
        }
    }
}
