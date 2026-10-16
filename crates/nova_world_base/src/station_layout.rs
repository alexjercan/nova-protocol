//! The open world's station layouts: one generated station hull from a
//! [`ShipPartSnapshot`], an archetype, an advancement and a stable seed.
//!
//! FRAME FIRST. A layout plans a frame of cells in one of three archetypes,
//! then stands a structural part in every cell:
//!
//! - the RING: a solid hub, one to three rings round it in its plane and four
//!   axis spokes, with four more stepped diagonal spokes from advancement 0.5;
//! - the SPIRE: two to five solid decks stacked on columns, with wings off the
//!   decks' flanks;
//! - the YARD: a long spine between two end blocks round a solid core, with
//!   rows of arms off ribs in the spine.
//!
//! Structure parts are classified by their sockets. A BLOCK has a socket on
//! every face, a STRUT has two on one axis, and a PANEL is a flat strut. Solid
//! frame cells are blocks. A run of frame cells is a strut where the cell's
//! only neighbours lie along one axis, and a wing or an arm is panels. A cell
//! takes the next sturdier part where its own is not eligible or would press a
//! blind face against a socket: a panel falls back to a strut, a strut to a
//! block. No station stands a panel below [`PANEL_ADVANCEMENT`], whatever its
//! part's tier. A blocks-only station is valid, and it is what advancement 0
//! builds from the base parts.
//!
//! FITTINGS. Flight computers replace solid cells inside the frame, one per
//! [`CELLS_PER_CONTROLLER`](nova_ship::prelude::CELLS_PER_CONTROLLER) cells of
//! mass, as for a ship. Then the docking ports, `2 + floor(6a)`, open level out
//! of a block, and the turrets, `4 + floor(12a)`, stand on the surface, up and
//! down in turn. Each fitting stands as far from the others of its kind as the
//! surface allows, with its lane clear.
//!
//! SIZE. The clearance radius rises linearly from 100 m at advancement 0 to
//! 300 m at 1, under the 400 m a sector body may declare. A plan that reaches
//! past it with room for its fittings shrinks until it fits.
//!
//! Every layout is checked after it is built: unique section ids, usable and
//! eligible station parts, no drive, mated contacts, one connected socket
//! graph, clear exit and docking lanes, the port and turret counts, the flight
//! computer count and the size ceiling. A seeded layout that fails is redrawn
//! a fixed number of times, then the request fails with its seed,
//! civilization, archetype, advancement and the last failed constraint.
//! Nothing substitutes an authored hull.
//!
//! PURE. Every size, share and count here is provisional until generated
//! stations are reviewed in travel.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    f32::consts::TAU,
    fmt,
};

use bevy::prelude::*;
use nova_events::prelude::Meters;
use nova_gameplay::prelude::{
    CivilizationId, Fnv32, GameItems, ItemDesignId, ItemRoleType, SeedStream, ShipInventoryStock,
};
use nova_scenario::prelude::{
    SectionSource, ShipDesign, ShipIntegrityConfig, ShipPresentationConfig, SpaceshipSectionConfig,
    StationDesign, StationServiceType, HULL_SECTION_CARGO_G, STATION_COLLAPSE_THRESHOLD,
};
use nova_ship::prelude::{
    blocked_exits, cube_rotations, derive_link_point_graph, oriented_part, read_structure,
    ship_exits, LinkPointGraphError, OrientedPart, PlacedPart, PlacedSectionLinkPoints,
    SectionCollider, SectionKind, ShipMeasure, GRID_EPSILON,
};

use crate::{
    civilizations::Civilization,
    ship_layout::{armament_tier_floor, placed_part, shuffled, signed, tier_floor, Grid, STEPS},
    ship_parts::{ShipPart, ShipPartFamilyType, ShipPartSnapshot},
    ARMOURED_STYLE_ID, INDUSTRIAL_STYLE_ID, STATION_STYLE_ID,
};

/// The families every generated station carries: structure, flight
/// computers, docking ports and turrets. Arming refuses a catalog with no
/// part of one of them at advancement 0.
pub(crate) const STATION_FAMILIES: [ShipPartFamilyType; 4] = [
    ShipPartFamilyType::Hull,
    ShipPartFamilyType::Controller,
    ShipPartFamilyType::Docking,
    ShipPartFamilyType::Weapon,
];

/// How many seeded layouts a request draws before it fails.
const MAX_STATION_ATTEMPTS: u32 = 8;

/// The largest station clearance radius at advancement 0.
const MIN_STATION_CLEARANCE: Meters = Meters(100.0);

/// The advancement from which a station stands panels, such as the base
/// solar panel, in its wings and arms. Below it those cells fall back to
/// struts, so a young civilization's station is all structure.
const PANEL_ADVANCEMENT: f32 = 0.5;

/// The largest station clearance radius at advancement 1.
const MAX_STATION_CLEARANCE: Meters = Meters(300.0);

/// How much a plan that does not fit its ceiling shrinks per step, and the
/// most steps it takes before the request fails as too large.
const SHRINK: (f32, u32) = (0.9, 24);

/// The cargo hold share a station's opening stock fills, at advancement 0 and
/// at advancement 1, read linearly between. Half is goods, half is the parts
/// its own sections are built from.
const STOCK_SHARE: (f32, f32) = (0.04, 0.08);

/// The goods a station stocks, by weight. Rounds for the guns of the ships it
/// serves, plates to patch them, food and salvage to trade.
const GOODS: [(ItemRoleType, f32); 4] = [
    (ItemRoleType::PdcRound, 2.0),
    (ItemRoleType::HullPlate, 3.0),
    (ItemRoleType::Rations, 3.0),
    (ItemRoleType::SalvagedParts, 2.0),
];

/// The grams a station stores for the ships that dock with it, at
/// advancement 0 and at advancement 1.
const STORAGE_CAP_G: (f32, f32) = (100_000.0, 400_000.0);

/// The credits a station trades with at spawn, at advancement 0 and at
/// advancement 1.
const CREDITS: (f32, f32) = (10_000.0, 40_000.0);

/// The services every generated station offers.
const SERVICES: [StationServiceType; 5] = [
    StationServiceType::Rebuild,
    StationServiceType::Refit,
    StationServiceType::Trade,
    StationServiceType::Storage,
    StationServiceType::Jobs,
];

/// The shape a generated station is built in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StationArchetypeType {
    /// A hub inside one to three rings on spokes.
    Ring,
    /// Decks stacked on columns, with wings.
    Spire,
    /// A long spine with rows of arms.
    Yard,
}

impl StationArchetypeType {
    /// Every archetype, in declaration order.
    pub const ALL: [Self; 3] = [Self::Ring, Self::Spire, Self::Yard];

    /// The lowercase name a failure or a label reads.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ring => "ring",
            Self::Spire => "spire",
            Self::Yard => "yard",
        }
    }

    /// The style a station of this archetype wears.
    pub fn style_id(self) -> &'static str {
        match self {
            Self::Ring => STATION_STYLE_ID,
            Self::Spire => ARMOURED_STYLE_ID,
            Self::Yard => INDUSTRIAL_STYLE_ID,
        }
    }
}

impl fmt::Display for StationArchetypeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// What a generated station is asked to be.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StationLayoutRequest {
    /// The stable station seed. Every draw of the layout comes from it.
    pub seed: u32,
    /// The civilization that builds the station. Only a failure reads it.
    pub civilization: CivilizationId,
    /// The shape it is built in.
    pub archetype: StationArchetypeType,
    /// The civilization's advancement in `[0, 1]`: the part and size ceiling
    /// and the port and turret counts.
    pub advancement: f32,
}

/// One generated station hull.
#[derive(Clone, Debug)]
pub struct StationLayout {
    /// The hull. Every section refers to a snapshot part by prototype id.
    pub design: ShipDesign,
    /// The shape it was built in.
    pub archetype: StationArchetypeType,
    /// The clearance radius: the distance from the design origin to the
    /// farthest corner of any cell it fills, so a sphere of it around the
    /// station root holds the hull in any orientation.
    pub clearance: Meters,
    /// The docking ports it carries.
    pub ports: usize,
    /// The turrets it carries.
    pub turrets: usize,
    /// The attempt that built it, from 0.
    pub attempt: u32,
}

/// The constraint a generated station failed.
#[derive(Clone, Debug, PartialEq)]
pub enum StationLayoutConstraintType {
    /// The snapshot has no station part of a family every station needs at
    /// this advancement.
    NoEligiblePart(ShipPartFamilyType),
    /// Fewer parts of a family stand in the layout than it needs.
    Unplaced {
        /// The family.
        family: ShipPartFamilyType,
        /// How many stand.
        placed: usize,
        /// How many it needs.
        wanted: usize,
    },
    /// Two sections share an id.
    DuplicateSectionId(String),
    /// A section refers to no part a station may carry at this advancement.
    UnknownPart {
        /// The section id.
        section: String,
        /// The prototype id it names.
        prototype: String,
    },
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
    /// The sockets do not join the sections into one station.
    Disconnected {
        /// How many separate pieces they form.
        components: usize,
    },
    /// The sockets are invalid or ambiguous, see [`LinkPointGraphError`].
    InvalidSockets,
    /// Something stands in, or asks for cladding across, a section's exit
    /// lane.
    BlockedExit(String),
    /// The station carries fewer flight computers than its mass needs.
    Undercontrolled {
        /// The flight computers it carries.
        controllers: usize,
        /// How many its mass needs.
        needed: usize,
    },
    /// The clearance radius exceeds the advancement's ceiling.
    TooLarge {
        /// The station's clearance radius.
        clearance: Meters,
        /// The ceiling.
        ceiling: Meters,
    },
}

impl fmt::Display for StationLayoutConstraintType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoEligiblePart(family) => write!(f, "no eligible station {family} part"),
            Self::Unplaced {
                family,
                placed,
                wanted,
            } => write!(f, "{placed} of {wanted} {family} parts fit the layout"),
            Self::DuplicateSectionId(id) => write!(f, "section id '{id}' is used twice"),
            Self::UnknownPart { section, prototype } => write!(
                f,
                "section '{section}' names '{prototype}', which is not an eligible station part"
            ),
            Self::Overlap { first, second } => {
                write!(f, "sections '{first}' and '{second}' fill one cell")
            }
            Self::UnmatedContact { first, second } => write!(
                f,
                "sections '{first}' and '{second}' touch where a socket meets no socket or a \
                 part fires"
            ),
            Self::Disconnected { components } => {
                write!(f, "the sockets join the station into {components} pieces")
            }
            Self::InvalidSockets => write!(f, "the socket graph is invalid or ambiguous"),
            Self::BlockedExit(id) => {
                write!(f, "section '{id}' cannot fire or dock down a clear lane")
            }
            Self::Undercontrolled {
                controllers,
                needed,
            } => write!(
                f,
                "the station carries {controllers} flight computers where its mass needs {needed}"
            ),
            Self::TooLarge { clearance, ceiling } => write!(
                f,
                "the station clearance radius is {:.0} m, above the {:.0} m ceiling",
                clearance.0, ceiling.0
            ),
        }
    }
}

/// A request no seeded layout satisfied.
#[derive(Clone, Debug, PartialEq)]
pub struct StationLayoutFailure {
    /// The request.
    pub request: StationLayoutRequest,
    /// How many layouts were drawn: 0 when the snapshot cannot serve a
    /// station at all.
    pub attempts: u32,
    /// The constraint the last layout failed.
    pub constraint: StationLayoutConstraintType,
}

impl fmt::Display for StationLayoutFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let StationLayoutRequest {
            seed,
            civilization,
            archetype,
            advancement,
        } = self.request;
        write!(
            f,
            "station seed {seed} of {civilization} ({archetype}, advancement {advancement:.2}) \
             failed after {} layout attempts: {}",
            self.attempts, self.constraint
        )
    }
}

/// The station clearance ceiling at `advancement`: 100 m at 0 to 300 m at 1,
/// linearly.
pub fn station_ceiling(advancement: f32) -> Meters {
    let span = MAX_STATION_CLEARANCE.0 - MIN_STATION_CLEARANCE.0;
    Meters(MIN_STATION_CLEARANCE.0 + span * advancement.clamp(0.0, 1.0))
}

/// The archetype `civilization` builds its stations in: one even draw off the
/// civilization's own stream, so every station of one civilization shares a
/// shape and neighbours differ.
pub fn station_archetype(civilization: &Civilization) -> StationArchetypeType {
    let count = StationArchetypeType::ALL.len();
    let draw = civilization.id.stream(b"station_archetype").unit();
    StationArchetypeType::ALL[((draw * count as f32) as usize).min(count - 1)]
}

/// The docking ports a station carries at `advancement`: `2 + floor(6a)`.
fn station_ports(advancement: f32) -> usize {
    2 + (6.0 * advancement.clamp(0.0, 1.0)).floor() as usize
}

/// The turrets a station carries at `advancement`: `4 + floor(12a)`.
fn station_turrets(advancement: f32) -> usize {
    4 + (12.0 * advancement.clamp(0.0, 1.0)).floor() as usize
}

/// `low` to `high` read at `advancement`.
fn lerp((low, high): (f32, f32), advancement: f32) -> f32 {
    low + (high - low) * advancement.clamp(0.0, 1.0)
}

/// Generate the station hull `request` asks for from `parts`.
///
/// The same request and the same effective catalog give the same design,
/// whatever order its packs were given in.
pub fn generate_station(
    parts: &ShipPartSnapshot,
    request: StationLayoutRequest,
) -> Result<StationLayout, StationLayoutFailure> {
    let fail = |attempts, constraint| StationLayoutFailure {
        request,
        attempts,
        constraint,
    };
    for family in STATION_FAMILIES {
        if parts.station_parts(family, request.advancement).is_empty() {
            return Err(fail(0, StationLayoutConstraintType::NoEligiblePart(family)));
        }
    }
    let mut last = None;
    for attempt in 0..MAX_STATION_ATTEMPTS {
        let draw = StationDraw {
            parts,
            request,
            attempt,
        };
        match draw.layout() {
            Ok(layout) => return Ok(layout),
            Err(constraint) => last = Some(constraint),
        }
    }
    Err(fail(
        MAX_STATION_ATTEMPTS,
        last.expect("at least one layout attempt runs"),
    ))
}

/// The station design `civilization` keeps, built on `layout`: the four dock
/// services, storage and credits that rise with advancement, and an opening
/// stock that fills a share of the hold, half goods and half the parts its
/// own sections are built from. An extinct civilization's dead station keeps
/// the same stock and credits as loot.
///
/// PURE: the same layout and civilization give the same design.
///
/// # Panics
///
/// When a section names no part of `parts`, or a goods role or a recipe part
/// is missing from `items`: content lint refuses both first.
pub fn generated_station(
    parts: &ShipPartSnapshot,
    items: &GameItems,
    layout: &StationLayout,
    civilization: &Civilization,
) -> StationDesign {
    let advancement = civilization.advancement.clamp(0.0, 1.0);
    let by_id: HashMap<&str, &ShipPart> =
        parts.parts().iter().map(|part| (part.id(), part)).collect();
    let mut prototypes: BTreeMap<&str, &ShipPart> = BTreeMap::new();
    let mut hulls = 0_u64;
    for section in &layout.design.sections {
        let id = section.source.prototype_id();
        let part = by_id
            .get(id)
            .unwrap_or_else(|| panic!("generated section '{}' names no part '{id}'", section.id));
        hulls += u64::from(matches!(part.config.kind, SectionKind::Hull(_)));
        prototypes.insert(id, part);
    }
    let hold = hulls * u64::from(HULL_SECTION_CARGO_G);
    let budget = (hold as f64 * f64::from(lerp(STOCK_SHARE, advancement)) / 2.0) as u64;

    let mut stock: BTreeMap<ItemDesignId, u32> = BTreeMap::new();
    let total: f32 = GOODS.iter().map(|(_, weight)| weight).sum();
    for (role, weight) in GOODS {
        let design = items.role(role);
        let portion = (budget as f64 * f64::from(weight / total)) as u64;
        let units = u32::try_from(portion / u64::from(design.mass_g))
            .expect("a station hold's units fit a u32");
        if units > 0 {
            *stock.entry(design.id.clone()).or_default() += units;
        }
    }
    let mut kit: BTreeMap<&ItemDesignId, u32> = BTreeMap::new();
    for part in prototypes.values() {
        for (item, count) in &part.config.base.build.parts {
            *kit.entry(item).or_default() += count;
        }
    }
    let kit_g: u64 = kit
        .iter()
        .map(|(item, count)| u64::from(items.design(item).mass_g) * u64::from(*count))
        .sum();
    // Each part item takes its share of the kit's mass, so a hold too small
    // for a whole kit still stocks the parts it needs most of.
    if kit_g > 0 {
        for (item, count) in kit {
            let units = u32::try_from(budget * u64::from(count) / kit_g)
                .expect("a station hold's units fit a u32");
            if units > 0 {
                *stock.entry(item.clone()).or_default() += units;
            }
        }
    }

    StationDesign {
        hull: layout.design.clone(),
        services: SERVICES.to_vec(),
        storage_cap_g: lerp(STORAGE_CAP_G, advancement).round() as u32,
        stock: ShipInventoryStock::new(stock),
        credits: lerp(CREDITS, advancement).round() as u32,
    }
}

/// What a frame cell is planned as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameType {
    /// Solid structure: a block.
    Solid,
    /// A run: a strut where its only neighbours lie along one axis.
    Line,
    /// A wing or arm: a panel whose sockets lie along `axis` and whose flat
    /// face looks along `normal`. Axes are 0, 1 and 2 for x, y and z.
    Sheet { axis: usize, normal: usize },
}

impl FrameType {
    /// Which of two plans for one cell wins: the sturdier.
    fn rank(self) -> u8 {
        match self {
            Self::Solid => 2,
            Self::Line => 1,
            Self::Sheet { .. } => 0,
        }
    }
}

/// A planned frame, by cell.
#[derive(Default)]
struct Frame(BTreeMap<[i32; 3], FrameType>);

impl Frame {
    /// Plan `cell` as `kind`, unless it is already planned sturdier.
    fn add(&mut self, cell: IVec3, kind: FrameType) {
        let entry = self.0.entry(cell.to_array()).or_insert(kind);
        if kind.rank() > entry.rank() {
            *entry = kind;
        }
    }

    /// Plan every cell from `low` to `high`, both included, as solid.
    fn solid(&mut self, low: IVec3, high: IVec3) {
        for x in low.x..=high.x {
            for y in low.y..=high.y {
                for z in low.z..=high.z {
                    self.add(IVec3::new(x, y, z), FrameType::Solid);
                }
            }
        }
    }

    /// The clearance radius the frame needs with a fitting in any cell beside
    /// it.
    fn reach(&self) -> Meters {
        let reach = self
            .0
            .keys()
            .flat_map(|cell| STEPS.map(|step| IVec3::from_array(*cell) + step))
            .map(|cell| (cell.abs().as_vec3() + Vec3::splat(0.5)).length())
            .fold(0.0, f32::max);
        Meters::from_engine(reach)
    }
}

/// A frame cell's resolved structure part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CellType {
    /// A block. `core` marks a solid frame cell, which takes the main block.
    Block { core: bool },
    /// A strut or, with a `normal`, a panel, its sockets along `axis`.
    Thin { axis: usize, normal: Option<usize> },
}

/// How a structure part stands in a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StructureType {
    Block,
    Strut,
    Panel,
}

/// How a hull part can stand in a frame, from its sockets and collider: a
/// block has a socket on every face, a strut two on one axis, and a panel is
/// a strut whose collider is under half a cell thick across it.
fn structure_type(part: &ShipPart) -> Option<StructureType> {
    if part.family != ShipPartFamilyType::Hull {
        return None;
    }
    let oriented = oriented_part(&part.config, Quat::IDENTITY).ok()?;
    if oriented.span != UVec3::ONE || oriented.cells.len() != 1 {
        return None;
    }
    let faces = oriented.cells[0].faces;
    if faces == [true; 6] {
        return oriented
            .origin
            .abs_diff_eq(Vec3::ZERO, GRID_EPSILON)
            .then_some(StructureType::Block);
    }
    let axis = (0..3).find(|axis| faces[2 * axis] && faces[2 * axis + 1])?;
    if faces.iter().filter(|face| **face).count() != 2 {
        return None;
    }
    let flat = match part.config.base.collider.unwrap_or_default() {
        SectionCollider::Cuboid { size } => {
            let across: Vec<f32> = (0..3)
                .filter(|other| *other != axis)
                .map(|other| size[other])
                .collect();
            across.iter().any(|extent| *extent < 0.5)
        }
        _ => false,
    };
    Some(if flat {
        StructureType::Panel
    } else {
        StructureType::Strut
    })
}

/// The axis a one-cell part's two sockets lie along at its orientation.
fn socket_axis(oriented: &OrientedPart) -> Option<usize> {
    let faces = oriented.cells.first()?.faces;
    (0..3).find(|axis| faces[2 * axis] && faces[2 * axis + 1])
}

/// The axis a part's collider is thinnest along at `rotation`.
fn thin_axis(part: &ShipPart, rotation: Quat) -> usize {
    let size = match part.config.base.collider.unwrap_or_default() {
        SectionCollider::Cuboid { size } => size,
        _ => Vec3::ONE,
    };
    let turned = (rotation * size).abs();
    (0..3)
        .min_by(|a, b| turned[*a].total_cmp(&turned[*b]))
        .expect("three axes")
}

/// One layout attempt.
struct StationDraw<'a> {
    parts: &'a ShipPartSnapshot,
    request: StationLayoutRequest,
    attempt: u32,
}

/// The structure parts one attempt builds with.
struct Structure<'a> {
    /// The block the solid cells take.
    main: &'a ShipPart,
    /// The block the other cells take.
    trim: &'a ShipPart,
    /// The strut, turned to lay its sockets along each axis.
    strut: Option<(&'a ShipPart, [Option<OrientedPart>; 3])>,
    /// The panel, turned for each socket axis and flat face.
    panel: Option<(&'a ShipPart, HashMap<(usize, usize), OrientedPart>)>,
}

impl<'a> StationDraw<'a> {
    /// The draw stream for one aspect of this attempt.
    fn stream(&self, aspect: &[u8]) -> SeedStream {
        SeedStream::new(
            Fnv32::new()
                .write(&self.request.seed.to_le_bytes())
                .write(b"station_layout")
                .write(&self.attempt.to_le_bytes())
                .write(aspect)
                .finish(),
        )
    }

    /// The station parts of `family` that pass `usable`, in the order this
    /// attempt tries them: sources shuffled, each source's parts shuffled,
    /// those at or above the request's [`tier_floor`] before the weaker
    /// ones. A turret's floor is the request's [`armament_tier_floor`].
    fn ranked(
        &self,
        family: ShipPartFamilyType,
        aspect: &[u8],
        usable: impl Fn(&ShipPart) -> bool,
    ) -> Vec<&'a ShipPart> {
        let mut stream = self.stream(aspect);
        let sources: Vec<Vec<&ShipPart>> = self
            .parts
            .station_parts(family, self.request.advancement)
            .into_values()
            .map(|parts| {
                parts
                    .into_iter()
                    .filter(|part| usable(part))
                    .collect::<Vec<_>>()
            })
            .filter(|parts| !parts.is_empty())
            .collect();
        let floor = match family {
            ShipPartFamilyType::Weapon => armament_tier_floor(self.request.advancement),
            _ => tier_floor(self.request.advancement),
        };
        shuffled(sources, &mut stream)
            .into_iter()
            .flat_map(|parts| {
                let (tier, weaker): (Vec<_>, Vec<_>) = shuffled(parts, &mut stream)
                    .into_iter()
                    .partition(|part| part.advancement >= floor);
                tier.into_iter().chain(weaker)
            })
            .collect()
    }

    /// The structure parts: the two first blocks, a strut where one is
    /// eligible, and a panel where one is eligible from
    /// [`PANEL_ADVANCEMENT`].
    fn structure(&self) -> Result<Structure<'a>, StationLayoutConstraintType> {
        let of = |kind: StructureType, aspect: &[u8]| {
            self.ranked(ShipPartFamilyType::Hull, aspect, |part| {
                structure_type(part) == Some(kind)
            })
        };
        let blocks = of(StructureType::Block, b"block");
        let main = *blocks
            .first()
            .ok_or(StationLayoutConstraintType::NoEligiblePart(
                ShipPartFamilyType::Hull,
            ))?;
        let trim = blocks.get(1).copied().unwrap_or(main);
        let strut = of(StructureType::Strut, b"strut").first().map(|part| {
            let mut turned: [Option<OrientedPart>; 3] = [None, None, None];
            for rotation in cube_rotations() {
                let Ok(oriented) = oriented_part(&part.config, rotation) else {
                    continue;
                };
                if let Some(axis) = socket_axis(&oriented) {
                    turned[axis].get_or_insert(oriented);
                }
            }
            (*part, turned)
        });
        let panels = if self.request.advancement >= PANEL_ADVANCEMENT {
            of(StructureType::Panel, b"panel")
        } else {
            Vec::new()
        };
        let panel = panels.first().map(|part| {
            let mut turned = HashMap::new();
            for rotation in cube_rotations() {
                let Ok(oriented) = oriented_part(&part.config, rotation) else {
                    continue;
                };
                if let Some(axis) = socket_axis(&oriented) {
                    turned
                        .entry((axis, thin_axis(part, rotation)))
                        .or_insert(oriented);
                }
            }
            (*part, turned)
        });
        Ok(Structure {
            main,
            trim,
            strut,
            panel,
        })
    }

    /// Build the station, adding flight computers until they meet its mass's
    /// floor.
    fn layout(&self) -> Result<StationLayout, StationLayoutConstraintType> {
        let structure = self.structure()?;
        let frame = self.plan()?;
        let cells = resolve(&frame, &structure);
        let mut controllers = 1;
        loop {
            let layout = self.build(&structure, &cells, controllers)?;
            let measure = ShipMeasure::of(layout.design.sections.iter().map(|section| {
                &self
                    .parts
                    .parts()
                    .iter()
                    .find(|part| part.id() == section.source.prototype_id())
                    .expect("a built section names a snapshot part")
                    .config
            }));
            let needed = measure.needed_controllers();
            if measure.controllers >= needed {
                self.check(&layout)?;
                return Ok(layout);
            }
            if needed <= controllers {
                return Err(StationLayoutConstraintType::Undercontrolled {
                    controllers: measure.controllers,
                    needed,
                });
            }
            controllers = needed;
        }
    }

    /// The archetype's frame, shrunk until it fits the advancement's ceiling
    /// with room for its fittings.
    fn plan(&self) -> Result<Frame, StationLayoutConstraintType> {
        let mut stream = self.stream(b"shape");
        let jitter = [stream.unit(), stream.unit(), stream.unit()];
        let wings = self.stream(b"wings");
        let advancement = self.request.advancement.clamp(0.0, 1.0);
        let ceiling = station_ceiling(advancement);
        let (step, steps) = SHRINK;
        let mut scale = 1.0;
        let mut planned = Meters(0.0);
        for _ in 0..steps {
            let frame = match self.request.archetype {
                StationArchetypeType::Ring => ring_frame(advancement, jitter, scale),
                StationArchetypeType::Spire => {
                    spire_frame(advancement, jitter, scale, &mut wings.clone())
                }
                StationArchetypeType::Yard => yard_frame(advancement, jitter, scale),
            };
            planned = frame.reach();
            if planned.0 <= ceiling.0 {
                return Ok(frame);
            }
            scale *= step;
        }
        Err(StationLayoutConstraintType::TooLarge {
            clearance: planned,
            ceiling,
        })
    }

    /// Build the station on `cells` with `controllers` flight computers.
    fn build(
        &self,
        structure: &Structure<'a>,
        cells: &BTreeMap<[i32; 3], CellType>,
        controllers: usize,
    ) -> Result<StationLayout, StationLayoutConstraintType> {
        let advancement = self.request.advancement.clamp(0.0, 1.0);
        let computers = self.ranked(ShipPartFamilyType::Controller, b"controller", |_| true);
        let computer =
            computers
                .first()
                .copied()
                .ok_or(StationLayoutConstraintType::NoEligiblePart(
                    ShipPartFamilyType::Controller,
                ))?;
        let inline_computer =
            oriented_part(&computer.config, Quat::IDENTITY)
                .ok()
                .filter(|oriented| {
                    oriented.span == UVec3::ONE
                        && oriented.cells.len() == 1
                        && oriented.cells[0].faces == [true; 6]
                        && oriented.origin.abs_diff_eq(Vec3::ZERO, GRID_EPSILON)
                });

        // Flight computers in solid cells, inner cells first, spread apart.
        let mut bridges = Vec::new();
        if inline_computer.is_some() {
            let candidates: Vec<(u32, IVec3)> = cells
                .iter()
                .filter(|(_, kind)| matches!(kind, CellType::Block { .. }))
                .map(|(cell, _)| {
                    let cell = IVec3::from_array(*cell);
                    let inner = STEPS
                        .iter()
                        .all(|step| cells.contains_key(&(cell + *step).to_array()));
                    (u32::from(inner), cell)
                })
                .collect();
            let candidates = shuffled(candidates, &mut self.stream(b"bridges"));
            bridges = spread(candidates, controllers);
        }
        let bridge_cells: HashSet<[i32; 3]> = bridges.iter().map(|cell| cell.to_array()).collect();

        let mut grid = Grid::default();
        let block = oriented_part(&structure.main.config, Quat::IDENTITY)
            .expect("a block stands on the grid");
        for (cell, kind) in cells {
            let at = IVec3::from_array(*cell);
            let name = format!("x{}_y{}_z{}", signed(at.x), signed(at.y), signed(at.z));
            if bridge_cells.contains(cell) {
                continue;
            }
            match *kind {
                CellType::Block { core } => {
                    let part = if core { structure.main } else { structure.trim };
                    grid.insert(format!("block_{name}"), part, block.clone(), at, true);
                }
                CellType::Thin { axis, normal } => {
                    let (prefix, part, oriented) =
                        match (normal, &structure.panel, &structure.strut) {
                            (Some(normal), Some((panel, turned)), _) => {
                                ("panel", *panel, turned[&(axis, normal)].clone())
                            }
                            (_, _, Some((strut, turned))) => (
                                "strut",
                                *strut,
                                turned[axis]
                                    .clone()
                                    .expect("a resolved strut axis is turned"),
                            ),
                            _ => unreachable!("a thin cell resolves only to an eligible part"),
                        };
                    grid.insert(format!("{prefix}_{name}"), part, oriented, at, false);
                }
            }
        }
        if let Some(oriented) = &inline_computer {
            for (index, cell) in bridges.iter().enumerate() {
                grid.insert(
                    format!("computer_{index}"),
                    computer,
                    oriented.clone(),
                    *cell,
                    false,
                );
            }
        } else {
            let placed = self.mount(
                &mut grid,
                &computers,
                "computer",
                controllers,
                false,
                b"computer_slot",
                |aims, _| aims.is_none().then_some(1),
            );
            if placed < controllers {
                return Err(StationLayoutConstraintType::Unplaced {
                    family: ShipPartFamilyType::Controller,
                    placed,
                    wanted: controllers,
                });
            }
        }

        // The docking ports open level, backed by a block.
        let ports = station_ports(advancement);
        let docks = self.ranked(ShipPartFamilyType::Docking, b"dock", |_| true);
        let placed = self.mount(
            &mut grid,
            &docks,
            "dock",
            ports,
            true,
            b"dock_slot",
            |aims, _| matches!(aims?, 0 | 1 | 4 | 5).then_some(1),
        );
        if placed < ports {
            return Err(StationLayoutConstraintType::Unplaced {
                family: ShipPartFamilyType::Docking,
                placed,
                wanted: ports,
            });
        }

        // The turrets, up and down in turn, then out of a side.
        let turrets = station_turrets(advancement);
        let guns = self.ranked(ShipPartFamilyType::Weapon, b"turret", |_| true);
        let placed = self.mount(
            &mut grid,
            &guns,
            "turret",
            turrets,
            false,
            b"turret_slot",
            |aims, index| match (aims?, index % 2) {
                (2, 0) | (3, 1) => Some(2),
                (0 | 1 | 4 | 5, _) => Some(1),
                _ => Some(0),
            },
        );
        if placed < turrets {
            return Err(StationLayoutConstraintType::Unplaced {
                family: ShipPartFamilyType::Weapon,
                placed,
                wanted: turrets,
            });
        }

        let reach = grid
            .filled
            .keys()
            .map(|cell| (cell.abs().as_vec3() + Vec3::splat(0.5)).length())
            .fold(0.0, f32::max);
        let sections = grid
            .placements
            .iter()
            .map(|placement| SpaceshipSectionConfig {
                id: placement.id.clone(),
                position: placement.position(),
                rotation: placement.oriented.rotation,
                source: SectionSource::Prototype {
                    id: placement.part.id().to_string(),
                    patch: default(),
                },
            })
            .collect();
        Ok(StationLayout {
            design: ShipDesign {
                sections,
                integrity: ShipIntegrityConfig {
                    collapse_threshold: Some(STATION_COLLAPSE_THRESHOLD),
                },
                // The base voice, as a generated ship wears it: an unset cue
                // reads as a mute hull.
                presentation: ShipPresentationConfig {
                    skin: true,
                    style: Some(self.request.archetype.style_id().to_string()),
                    ..ShipPresentationConfig::base_voice()
                },
            },
            archetype: self.request.archetype,
            clearance: Meters::from_engine(reach),
            ports,
            turrets,
            attempt: self.attempt,
        })
    }

    /// Mount up to `count` of the first of `parts` that fits, each on the
    /// surface where `aim` scores it best and as far from the others as the
    /// surface allows, with every exit lane clear. A `backed` part needs a
    /// block behind its back. `aim` reads a turned part's exit face and how
    /// many are already mounted. Returns how many were mounted.
    #[expect(
        clippy::too_many_arguments,
        reason = "one mount call names the slot, the count, the backing and the aim"
    )]
    fn mount(
        &self,
        grid: &mut Grid<'a>,
        parts: &[&'a ShipPart],
        slot: &str,
        count: usize,
        backed: bool,
        aspect: &[u8],
        aim: impl Fn(Option<usize>, usize) -> Option<u32>,
    ) -> usize {
        let surface: BTreeSet<[i32; 3]> = grid
            .filled
            .keys()
            .flat_map(|cell| STEPS.map(|step| *cell + step))
            .filter(|cell| !grid.filled.contains_key(cell))
            .map(|cell| cell.to_array())
            .collect();
        let (low, high) = grid
            .filled
            .keys()
            .fold((IVec3::MAX, IVec3::MIN), |(low, high), cell| {
                (low.min(*cell), high.max(*cell))
            });
        for part in parts {
            // One turned copy per distinct footprint and exit.
            let mut variants: Vec<OrientedPart> = Vec::new();
            for rotation in cube_rotations() {
                let Ok(oriented) = oriented_part(&part.config, rotation) else {
                    continue;
                };
                if (0..count).all(|index| aim(oriented.aims, index).is_none()) {
                    continue;
                }
                let same = variants.iter().any(|other| {
                    other.aims == oriented.aims
                        && other.cells == oriented.cells
                        && other.origin.abs_diff_eq(oriented.origin, GRID_EPSILON)
                });
                if !same {
                    variants.push(oriented);
                }
            }
            let mut candidates: Vec<(usize, IVec3)> = Vec::new();
            for (index, oriented) in variants.iter().enumerate() {
                let mut anchors = BTreeSet::new();
                for cell in &surface {
                    for own in &oriented.cells {
                        anchors.insert((IVec3::from_array(*cell) - own.cell.as_ivec3()).to_array());
                    }
                }
                for anchor in anchors {
                    let anchor = IVec3::from_array(anchor);
                    if grid.fits(oriented, anchor)
                        && (!backed || grid.backed(oriented, anchor))
                        && lane_open(grid, oriented, anchor, (low, high))
                    {
                        candidates.push((index, anchor));
                    }
                }
            }
            let mut pool = shuffled(candidates, &mut self.stream(aspect));
            let mut mounted: Vec<IVec3> = Vec::new();
            while mounted.len() < count && !pool.is_empty() {
                let index = mounted.len();
                let best = pool
                    .iter()
                    .enumerate()
                    .filter_map(|(at, (variant, anchor))| {
                        let score = aim(variants[*variant].aims, index)?;
                        let apart = mounted
                            .iter()
                            .map(|other| other.distance_squared(*anchor))
                            .min()
                            .unwrap_or(i32::MAX);
                        Some((score, apart, std::cmp::Reverse(at)))
                    })
                    .max();
                let Some((_, _, std::cmp::Reverse(at))) = best else {
                    break;
                };
                let (variant, anchor) = pool.remove(at);
                let oriented = &variants[variant];
                if !grid.fits(oriented, anchor) || (backed && !grid.backed(oriented, anchor)) {
                    continue;
                }
                grid.insert(
                    format!("{slot}_{index}"),
                    part,
                    oriented.clone(),
                    anchor,
                    false,
                );
                if grid.last_blocks_an_exit() {
                    grid.pop();
                    continue;
                }
                mounted.push(anchor);
            }
            if mounted.len() == count {
                return count;
            }
            for _ in 0..mounted.len() {
                grid.pop();
            }
        }
        0
    }

    /// Every constraint a generated station must hold, in the order they are
    /// checked.
    fn check(&self, layout: &StationLayout) -> Result<(), StationLayoutConstraintType> {
        use StationLayoutConstraintType as Constraint;
        let design = &layout.design;
        let advancement = self.request.advancement;

        let mut ids = HashSet::new();
        for section in &design.sections {
            if !ids.insert(section.id.as_str()) {
                return Err(Constraint::DuplicateSectionId(section.id.clone()));
            }
        }
        let eligible: HashMap<&str, &ShipPart> = [
            ShipPartFamilyType::Hull,
            ShipPartFamilyType::Controller,
            ShipPartFamilyType::Docking,
            ShipPartFamilyType::Weapon,
        ]
        .into_iter()
        .flat_map(|family| self.parts.station_parts(family, advancement).into_values())
        .flatten()
        .map(|part| (part.id(), part))
        .collect();
        let mut resolved = Vec::with_capacity(design.sections.len());
        for section in &design.sections {
            let part = match &section.source {
                SectionSource::Prototype { id, .. } => eligible.get(id.as_str()),
                SectionSource::Inline(_) => None,
            };
            let Some(part) = part else {
                return Err(Constraint::UnknownPart {
                    section: section.id.clone(),
                    prototype: section.source.prototype_id().to_string(),
                });
            };
            resolved.push(*part);
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

        let links: Vec<PlacedSectionLinkPoints> = design
            .sections
            .iter()
            .zip(&resolved)
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
            .zip(&resolved)
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

        for (family, wanted) in [
            (ShipPartFamilyType::Docking, station_ports(advancement)),
            (ShipPartFamilyType::Weapon, station_turrets(advancement)),
        ] {
            let placed = resolved.iter().filter(|part| part.family == family).count();
            if placed != wanted {
                return Err(Constraint::Unplaced {
                    family,
                    placed,
                    wanted,
                });
            }
        }

        let ceiling = station_ceiling(advancement);
        if layout.clearance.0 > ceiling.0 {
            return Err(Constraint::TooLarge {
                clearance: layout.clearance,
                ceiling,
            });
        }
        Ok(())
    }
}

/// Whether nothing filled stands in the lane `oriented` at `anchor` fires or
/// docks down, out to the station's bounds `low` to `high`. A cheap first
/// look; the cladding rule is checked when the part stands.
fn lane_open(
    grid: &Grid<'_>,
    oriented: &OrientedPart,
    anchor: IVec3,
    (low, high): (IVec3, IVec3),
) -> bool {
    let Some(exit) = oriented.aims else {
        return true;
    };
    let step = STEPS[exit];
    oriented.cells.iter().all(|cell| {
        let mut at = anchor + cell.cell.as_ivec3() + step;
        while at.cmpge(low - IVec3::ONE).all() && at.cmple(high + IVec3::ONE).all() {
            if grid.filled.contains_key(&at) {
                return false;
            }
            at += step;
        }
        true
    })
}

/// Up to `count` of `candidates`, scored, taken best score first and then
/// farthest from those already taken. Ties keep the candidates' order.
fn spread(mut pool: Vec<(u32, IVec3)>, count: usize) -> Vec<IVec3> {
    let mut taken: Vec<IVec3> = Vec::with_capacity(count);
    while taken.len() < count && !pool.is_empty() {
        let (at, _) = pool
            .iter()
            .enumerate()
            .map(|(at, (score, cell))| {
                let apart = taken
                    .iter()
                    .map(|other| other.distance_squared(*cell))
                    .min()
                    .unwrap_or(i32::MAX);
                (at, (*score, apart, std::cmp::Reverse(at)))
            })
            .max_by_key(|(_, key)| *key)
            .expect("a candidate is left");
        taken.push(pool.remove(at).1);
    }
    taken
}

/// The structure part every frame cell takes: a solid cell a block, a run a
/// strut where its only neighbours lie along one axis, a sheet a panel. A cell
/// whose part is not eligible takes the next sturdier, and a strut or panel
/// that would press a blind face against a socket, or meet a strut or panel
/// across another axis end to end, becomes a block, until none does.
fn resolve(frame: &Frame, structure: &Structure<'_>) -> BTreeMap<[i32; 3], CellType> {
    let strut = |axis: usize| {
        structure
            .strut
            .as_ref()
            .is_some_and(|(_, turned)| turned[axis].is_some())
    };
    let panel = |axis: usize, normal: usize| {
        structure
            .panel
            .as_ref()
            .is_some_and(|(_, turned)| turned.contains_key(&(axis, normal)))
    };
    let mut cells: BTreeMap<[i32; 3], CellType> = frame
        .0
        .iter()
        .map(|(cell, kind)| {
            let at = IVec3::from_array(*cell);
            let resolved = match *kind {
                FrameType::Solid => CellType::Block { core: true },
                FrameType::Line => {
                    let axes: BTreeSet<usize> = (0..6)
                        .filter(|face| frame.0.contains_key(&(at + STEPS[*face]).to_array()))
                        .map(|face| face / 2)
                        .collect();
                    match axes.iter().collect::<Vec<_>>().as_slice() {
                        [axis] if strut(**axis) => CellType::Thin {
                            axis: **axis,
                            normal: None,
                        },
                        _ => CellType::Block { core: false },
                    }
                }
                FrameType::Sheet { axis, normal } if panel(axis, normal) => CellType::Thin {
                    axis,
                    normal: Some(normal),
                },
                FrameType::Sheet { axis, .. } if strut(axis) => {
                    CellType::Thin { axis, normal: None }
                }
                FrameType::Sheet { .. } => CellType::Block { core: false },
            };
            (*cell, resolved)
        })
        .collect();
    loop {
        let demoted: Vec<[i32; 3]> = cells
            .iter()
            .filter_map(|(cell, kind)| {
                let CellType::Thin { axis, .. } = *kind else {
                    return None;
                };
                let at = IVec3::from_array(*cell);
                let clashes = (0..6).any(|face| {
                    let Some(other) = cells.get(&(at + STEPS[face]).to_array()) else {
                        return false;
                    };
                    let thin_along = |wanted: usize| {
                        matches!(other, CellType::Thin { axis: other_axis, .. } if *other_axis == wanted)
                    };
                    if face / 2 == axis {
                        matches!(other, CellType::Thin { .. }) && !thin_along(axis)
                    } else {
                        !thin_along(axis)
                    }
                });
                clashes.then_some(*cell)
            })
            .collect();
        if demoted.is_empty() {
            return cells;
        }
        for cell in demoted {
            cells.insert(cell, CellType::Block { core: false });
        }
    }
}

/// The cells of a ring of `radius` round the origin in the XZ plane, every
/// step face to face.
fn ring_cells(radius: i32) -> BTreeSet<[i32; 3]> {
    let mut cells = BTreeSet::new();
    let steps = (radius * 16).max(64);
    let mut last: Option<IVec3> = None;
    for index in 0..=steps {
        let angle = index as f32 / steps as f32 * TAU;
        let at = IVec3::new(
            (radius as f32 * angle.cos()).round() as i32,
            0,
            (radius as f32 * angle.sin()).round() as i32,
        );
        if let Some(last) = last {
            if last.x != at.x && last.z != at.z {
                cells.insert([at.x, 0, last.z]);
            }
        }
        cells.insert(at.to_array());
        last = Some(at);
    }
    cells
}

/// The ring: a solid hub, one ring at advancement 0 to three at 1 inside the
/// ceiling, four axis spokes and, from advancement 0.5, four stepped diagonal
/// spokes.
fn ring_frame(advancement: f32, jitter: [f32; 3], scale: f32) -> Frame {
    let mut frame = Frame::default();
    let hub = 1 + (2.0 * advancement).round() as i32;
    let rings = 1 + (2.0 * advancement).round() as i32;
    let room = station_ceiling(advancement).to_engine() - 2.0;
    let outer = ((room * (0.8 + 0.2 * jitter[0]) * scale).floor() as i32).max(hub + 3);
    frame.solid(IVec3::splat(-hub), IVec3::splat(hub));
    let gap = (outer - hub - 2) as f32 / rings as f32;
    for ring in 0..rings {
        let radius = outer - (ring as f32 * gap).round() as i32;
        for cell in ring_cells(radius) {
            frame.add(IVec3::from_array(cell), FrameType::Line);
        }
    }
    for out in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
        for step in hub + 1..outer {
            frame.add(out * step, FrameType::Line);
        }
    }
    if advancement >= 0.5 {
        for (sx, sz) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let (mut x, mut z) = (hub, hub);
            while ((x * x + z * z) as f32).sqrt() < outer as f32 - 0.5 {
                if x <= z {
                    x += 1;
                } else {
                    z += 1;
                }
                frame.add(IVec3::new(sx * x, 0, sz * z), FrameType::Solid);
            }
        }
    }
    frame
}

/// The spire: two decks 3 by 3 at advancement 0 to five decks 7 by 7 at 1,
/// stacked on corner and centre columns, with three to nine wings off the
/// decks' flanks.
fn spire_frame(advancement: f32, jitter: [f32; 3], scale: f32, stream: &mut SeedStream) -> Frame {
    let mut frame = Frame::default();
    let decks = 2 + (3.0 * advancement).round() as i32;
    let half = 1 + (2.0 * advancement).round() as i32;
    let thick = 2 + advancement.round() as i32;
    let gap =
        (((2.0 + 4.0 * advancement) * (0.75 + 0.5 * jitter[0]) * scale).round() as i32).max(1);
    let wings = 3 + (6.0 * advancement).round() as usize;
    let rows = (1 + (2.0 * advancement * jitter[1]).round() as i32).min(2 * half + 1);
    let length =
        (((3.0 + 4.0 * advancement) * (0.8 + 0.4 * jitter[2]) * scale).round() as i32).max(1);
    let height = decks * thick + (decks - 1) * gap;
    let bottom = -height / 2;
    let mut slots = Vec::new();
    for deck in 0..decks {
        let base = bottom + deck * (thick + gap);
        frame.solid(
            IVec3::new(-half, base, -half),
            IVec3::new(half, base + thick - 1, half),
        );
        if deck + 1 < decks {
            for y in base + thick..base + thick + gap {
                for (x, z) in [
                    (half, half),
                    (half, -half),
                    (-half, half),
                    (-half, -half),
                    (0, 0),
                ] {
                    frame.add(IVec3::new(x, y, z), FrameType::Line);
                }
            }
        }
        for side in [1, -1] {
            slots.push((base + thick / 2, side));
        }
    }
    for (y, side) in shuffled(slots, stream).into_iter().take(wings) {
        for row in 0..rows {
            let z = row - (rows - 1) / 2;
            for x in half + 1..=half + 2 {
                frame.add(IVec3::new(side * x, y, z), FrameType::Line);
            }
            for x in half + 3..half + 3 + length {
                frame.add(
                    IVec3::new(side * x, y, z),
                    FrameType::Sheet { axis: 0, normal: 1 },
                );
            }
        }
    }
    frame
}

/// The yard: a spine 12 cells long at advancement 0 to 40 at 1 between two
/// end blocks round a solid core, with two to six ribs whose arms reach out
/// either side.
fn yard_frame(advancement: f32, jitter: [f32; 3], scale: f32) -> Frame {
    let mut frame = Frame::default();
    let length =
        (((12.0 + 28.0 * advancement) * (0.85 + 0.15 * jitter[0]) * scale).round() as i32).max(8);
    let rows = 2 + (4.0 * advancement).round() as i32;
    let arm = (((3.0 + 5.0 * advancement) * (0.8 + 0.4 * jitter[1]) * scale).round() as i32).max(1);
    let half = length / 2;
    for z in -half..=half {
        frame.add(IVec3::new(0, 0, z), FrameType::Line);
    }
    frame.solid(IVec3::splat(-1), IVec3::splat(1));
    for sign in [1, -1] {
        let (near, far) = (sign * (half + 1), sign * (half + 2));
        frame.solid(
            IVec3::new(-1, -1, near.min(far)),
            IVec3::new(1, 1, near.max(far)),
        );
    }
    // Ribs from three cells off the core to two short of the end blocks,
    // either side in turn, never beside one another.
    let per_side = (rows + 1) / 2;
    let span = (half - 2 - 3).max(0);
    let pitch = if per_side > 1 {
        span / (per_side - 1)
    } else {
        0
    };
    let offset = (jitter[2] * 2.0) as i32;
    for rib in 0..rows {
        let sign = if rib % 2 == 0 { 1 } else { -1 };
        let index = rib / 2;
        if per_side > 1 && pitch < 2 && index > 0 {
            continue;
        }
        let z = sign * (3 + index * pitch + if per_side > 1 { 0 } else { offset }).min(half - 2);
        frame.add(IVec3::new(0, 0, z), FrameType::Solid);
        for x in 1..=arm {
            for side in [1, -1] {
                frame.add(
                    IVec3::new(side * x, 0, z),
                    FrameType::Sheet { axis: 0, normal: 1 },
                );
            }
        }
    }
    frame
}

#[cfg(test)]
mod tests {
    use nova_ship::prelude::{
        BaseSectionConfig, HullSectionConfig, LinkPoint, LinkPointSkinType, SectionConfig,
    };

    use super::*;
    use crate::{ship_layout::fixture, ship_parts::ShipPartPack};

    /// A one-cell hull part with a socket on the centre of each of `faces`.
    fn frame_part(id: &str, health: f32, faces: &[Vec3], collider: Vec3) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                health,
                collider: Some(SectionCollider::Cuboid { size: collider }),
                link_points: faces
                    .iter()
                    .enumerate()
                    .map(|(index, face)| LinkPoint {
                        id: format!("link_{index}"),
                        position: *face * 0.5,
                        normal: *face,
                        skin: LinkPointSkinType::Bare,
                    })
                    .collect(),
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        }
    }

    /// The ship fixture plus a frame pack with a strut and a panel. As in
    /// the base parts, the strut is eligible above advancement 0 and the
    /// panel only from the top part tier, so advancement 0 builds blocks
    /// only.
    fn snapshot() -> ShipPartSnapshot {
        let mut packs = fixture::packs();
        packs.push(ShipPartPack {
            id: "frame".to_string(),
            dependencies: vec!["base".to_string()],
            sections: vec![
                frame_part("strut", 150.0, &[Vec3::Z, Vec3::NEG_Z], Vec3::ONE),
                frame_part(
                    "panel",
                    50.0,
                    &[Vec3::X, Vec3::NEG_X],
                    Vec3::new(1.0, 0.1, 1.0),
                ),
            ],
        });
        ShipPartSnapshot::build(&packs).expect("the station fixture builds")
    }

    fn request(
        seed: u32,
        archetype: StationArchetypeType,
        advancement: f32,
    ) -> StationLayoutRequest {
        StationLayoutRequest {
            seed,
            civilization: CivilizationId {
                world_seed: 7,
                node: [1, -2, 0],
            },
            archetype,
            advancement,
        }
    }

    const ADVANCEMENTS: [f32; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

    /// No station stands a panel below advancement 0.5, even with a panel
    /// part open from advancement 0, and from 0.5 the spire's wings and the
    /// yard's arms are panels.
    #[test]
    fn a_station_stands_panels_from_mid_advancement_only() {
        let mut packs = fixture::packs();
        packs.push(ShipPartPack {
            id: "frame".to_string(),
            dependencies: vec!["base".to_string()],
            sections: vec![
                frame_part("strut", 150.0, &[Vec3::Z, Vec3::NEG_Z], Vec3::ONE),
                // As sturdy per volume as the weakest cube, so its tier is 0.
                frame_part(
                    "panel",
                    10.0,
                    &[Vec3::X, Vec3::NEG_X],
                    Vec3::new(1.0, 0.1, 1.0),
                ),
            ],
        });
        let parts = ShipPartSnapshot::build(&packs).expect("the panel fixture builds");
        assert!(parts
            .parts()
            .iter()
            .any(|part| part.id() == "panel" && part.advancement == 0.0));
        for archetype in StationArchetypeType::ALL {
            for advancement in [0.0, 0.25, 0.45, 0.5, 0.75, 1.0] {
                let mut panels = 0;
                for seed in 0..8 {
                    let station = generate_station(&parts, request(seed, archetype, advancement))
                        .unwrap_or_else(|failure| {
                            panic!("{archetype} a {advancement} seed {seed}: {failure}")
                        });
                    panels += station
                        .design
                        .sections
                        .iter()
                        .filter(|section| section.source.prototype_id() == "panel")
                        .count();
                }
                let context = format!("{archetype} a {advancement}: {panels} panels");
                if advancement < 0.5 {
                    assert_eq!(panels, 0, "{context}");
                } else if archetype != StationArchetypeType::Ring {
                    assert!(panels > 0, "{context}");
                }
            }
        }
    }

    /// The same request gives the same station, and another seed another.
    #[test]
    fn a_station_request_gives_the_same_hull_every_time() {
        let parts = snapshot();
        for archetype in StationArchetypeType::ALL {
            let first = generate_station(&parts, request(3, archetype, 0.5)).expect("generates");
            let again = generate_station(&parts, request(3, archetype, 0.5)).expect("generates");
            assert_eq!(
                format!("{:?}", first.design),
                format!("{:?}", again.design),
                "{archetype}"
            );
            assert_eq!(first.clearance, again.clearance, "{archetype}");
        }
    }

    /// Every station carries `2 + floor(6a)` docking ports and
    /// `4 + floor(12a)` turrets, enough flight computers for its mass and no
    /// drive, inside its advancement's ceiling.
    #[test]
    fn a_station_carries_its_ports_turrets_and_computers_within_its_ceiling() {
        let parts = snapshot();
        let family = |section: &SpaceshipSectionConfig| {
            parts
                .parts()
                .iter()
                .find(|part| part.id() == section.source.prototype_id())
                .expect("a snapshot part")
        };
        for archetype in StationArchetypeType::ALL {
            for advancement in ADVANCEMENTS {
                for seed in 0..4 {
                    let context = format!("{archetype} a {advancement} seed {seed}");
                    let station = generate_station(&parts, request(seed, archetype, advancement))
                        .unwrap_or_else(|failure| panic!("{context}: {failure}"));
                    let count = |wanted: ShipPartFamilyType| {
                        station
                            .design
                            .sections
                            .iter()
                            .filter(|section| family(section).family == wanted)
                            .count()
                    };
                    let ports = 2 + (6.0 * advancement).floor() as usize;
                    let turrets = 4 + (12.0 * advancement).floor() as usize;
                    assert_eq!(count(ShipPartFamilyType::Docking), ports, "{context}");
                    assert_eq!(count(ShipPartFamilyType::Weapon), turrets, "{context}");
                    assert_eq!(
                        (station.ports, station.turrets),
                        (ports, turrets),
                        "{context}"
                    );
                    assert_eq!(count(ShipPartFamilyType::Thruster), 0, "{context}");
                    let measure = ShipMeasure::of(
                        station
                            .design
                            .sections
                            .iter()
                            .map(|section| &family(section).config),
                    );
                    assert!(
                        measure.controllers >= measure.needed_controllers(),
                        "{context}: {measure:?}"
                    );
                    assert!(
                        station.clearance.0 <= station_ceiling(advancement).0,
                        "{context}: {:?}",
                        station.clearance
                    );
                }
            }
        }
    }

    /// Every station is one structure, and every docking port and turret
    /// fires or docks down a clear lane.
    #[test]
    fn a_station_is_one_structure_with_clear_lanes() {
        let parts = snapshot();
        for archetype in StationArchetypeType::ALL {
            for advancement in [0.0, 1.0] {
                let station = generate_station(&parts, request(5, archetype, advancement))
                    .expect("generates");
                let resolved: Vec<&SectionConfig> = station
                    .design
                    .sections
                    .iter()
                    .map(|section| {
                        &parts
                            .parts()
                            .iter()
                            .find(|part| part.id() == section.source.prototype_id())
                            .expect("a snapshot part")
                            .config
                    })
                    .collect();
                let links: Vec<PlacedSectionLinkPoints> = station
                    .design
                    .sections
                    .iter()
                    .zip(&resolved)
                    .map(|(section, config)| PlacedSectionLinkPoints {
                        position: section.position,
                        rotation: section.rotation,
                        link_points: &config.base.link_points,
                    })
                    .collect();
                assert!(
                    derive_link_point_graph(&links).is_ok(),
                    "{archetype} at {advancement} is one structure"
                );
                let placed: Vec<PlacedPart> = station
                    .design
                    .sections
                    .iter()
                    .zip(&resolved)
                    .map(|(section, config)| {
                        placed_part(config, section.position, section.rotation)
                    })
                    .collect();
                let (structure, _, occupied) = read_structure(&placed);
                let exits = ship_exits(&placed, &occupied);
                assert!(
                    exits.len() >= station.ports + station.turrets,
                    "{archetype} at {advancement}"
                );
                assert!(
                    blocked_exits(&structure, &exits).is_empty(),
                    "{archetype} at {advancement} has a blocked lane"
                );
            }
        }
    }

    /// A more advanced civilization builds a larger station: the median
    /// clearance over seeds never shrinks as advancement rises.
    #[test]
    fn the_median_station_grows_with_advancement() {
        let parts = snapshot();
        for archetype in StationArchetypeType::ALL {
            let medians: Vec<f32> = ADVANCEMENTS
                .iter()
                .map(|advancement| {
                    let mut sizes: Vec<f32> = (0..5)
                        .map(|seed| {
                            generate_station(&parts, request(seed, archetype, *advancement))
                                .expect("generates")
                                .clearance
                                .0
                        })
                        .collect();
                    sizes.sort_by(f32::total_cmp);
                    sizes[sizes.len() / 2]
                })
                .collect();
            assert!(
                medians.windows(2).all(|pair| pair[0] <= pair[1]),
                "{archetype}: {medians:?}"
            );
        }
    }

    /// A snapshot with no eligible turret cannot arm a station.
    #[test]
    fn a_station_with_no_eligible_turret_fails_before_any_attempt() {
        let mut packs = fixture::packs();
        for pack in &mut packs {
            pack.sections
                .retain(|section| !matches!(section.kind, SectionKind::Turret(_)));
        }
        let parts = ShipPartSnapshot::build(&packs).expect("the fixture builds without turrets");
        let failure = generate_station(&parts, request(1, StationArchetypeType::Ring, 0.0))
            .expect_err("no turret");
        assert_eq!(failure.attempts, 0);
        assert_eq!(
            failure.constraint,
            StationLayoutConstraintType::NoEligiblePart(ShipPartFamilyType::Weapon)
        );
    }
}
