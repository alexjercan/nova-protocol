//! The clustered world: bodies that come in GROUPS, and groups that cross
//! sector faces.
//!
//! Example-owned, like [`super::UniformAsteroids`]. It implements
//! [`SectorGenerator`] from outside `nova_world`, so every manifest it returns
//! goes through the same `validate_manifest` check as the base game's.
//!
//! # Groups are global, cells only own bodies
//!
//! A group is decided at a node of a global 24 km lattice, from a stream keyed
//! by the world seed and the node alone. It is one of five [`GroupKind`]s -
//! an asteroid-rich field that may hold one or two planetoids and a derelict,
//! a rock-only field, a planet-heavy pair of worlds with a few rocks, a
//! derelict-only wreck field, or a low-rock scatter - and every body in it has
//! one world position. The cell a body's centre falls in OWNS that body. A
//! cell asks every node whose group could put a body inside it (the halo,
//! [`halo_nodes`]), replays each group the same way any other cell would, and
//! keeps the bodies it owns. So a group whose bodies stand on both sides of a
//! face is one group, and a neighbour generated first, last or never cannot
//! change it.
//!
//! # Every planned body is accounted for
//!
//! A cell resolves the bodies it owns in one fixed order - every group's
//! parents, then every group's members by group and index, then the
//! background rock - and each one is either placed or skipped for a named
//! [`SkipReason`]: its clearance crosses a face, or it overlaps a body placed
//! before it. Nothing is dropped without a reason in the [`CellPlan`], and
//! there is no count limit. A PARENT - a group's planetoid - is never
//! skipped: the core clamp keeps it inside its cell and the lattice keeps it
//! clear of every other parent, so a skipped parent is a generator bug and a
//! loud [`SectorFault`]. A cell may hold any number of parents.
//!
//! # Groups keep apart
//!
//! Two anchors stay at least `GROUP_LATTICE * (1 - GROUP_JITTER)` apart on
//! the axis their nodes differ on. A group with worlds moves its anchor at
//! most its core reach on each axis to keep them inside a cell, and no body surface
//! stands further from an anchor than a group's extent. So two groups never
//! come closer than that separation less two core reaches and two extents,
//! about 1.2 km at the widest; the typical gap is what the census measures.
//!
//! # The environment decides
//!
//! The kind, its body counts and the rock, world and hull mixes read the
//! three [`super::environment`] fields at the group's drawn anchor, TOGETHER
//! and continuously: dense material grows asteroid-rich, rock-only and
//! planet-heavy groups, traffic takes the worlds and grows wreck fields,
//! volatiles favour worlds and turn rocks and worlds to ice and carbon, and
//! thin quiet space grows low-rock scatters or nothing.

use bevy::prelude::Vec3;
use nova_protocol::prelude::*;
use nova_world::prelude::*;

use super::environment::{Environment, EnvironmentFields};

/// The spacing of the group lattice: at most one group per node.
///
/// Three quarters of a 32 km edge: 2.4 nodes a cell, and with most nodes
/// growing a group, about two groups a sector. Not a whole number of edges,
/// so anchors fall near cell centres and near faces alike, and a group near a
/// face is what crosses it.
pub const GROUP_LATTICE: Meters = Meters(24_000.0);

/// How far a group's anchor may move off its node, as a fraction of the half
/// spacing. Jitter is what keeps groups off a visible grid.
///
/// Bounded, because the gap between groups rests on it: two anchors stay at
/// least `GROUP_LATTICE * (1 - GROUP_JITTER)`, 19.2 km, apart on the axis
/// their nodes differ on.
const GROUP_JITTER: f32 = 0.2;

/// Extra room every pair of bodies in one cell keeps between their clearance
/// spheres.
///
/// Tighter than the base game's 500 m: a group is meant to read as bodies
/// NEAR each other.
const MEMBER_MARGIN: Meters = Meters(250.0);

/// The nominal radius band a rock is drawn from. The meshed rock reaches up to
/// [`ASTEROID_GEOMETRIC_FACTOR_MAX`] times past it, which is the clearance it
/// is spaced by.
const ROCK_RADIUS: (Meters, Meters) = (Meters(30.0), Meters(60.0));

/// The mean radius band a planetoid is drawn from.
const PLANETOID_RADIUS: (Meters, Meters) = (Meters(600.0), Meters(1_200.0));

/// How far each of a pair of worlds stands from its group's anchor, on
/// opposite sides.
///
/// The inner edge keeps the pair apart: 2.9 km between centres clears two of
/// the widest worlds (1,272 m each) and the margin.
const TWIN_OFFSET: (Meters, Meters) = (Meters(1_450.0), Meters(1_600.0));

/// How deep the shell of members around a group's worlds is, past the core
/// they clear.
///
/// Narrower than [`OPEN_SPREAD`]: the core already makes a world group the
/// widest, and the gap between groups is paid out of both extents.
const WORLD_SPREAD: (Meters, Meters) = (Meters(1_400.0), Meters(2_200.0));

/// How far from its anchor a member of a group with no worlds may stand.
const OPEN_SPREAD: (Meters, Meters) = (Meters(2_000.0), Meters(4_000.0));

/// How many rocks each kind places, from the bottom of the band in thin
/// material to the top in dense material.
const ASTEROID_RICH_ROCKS: (usize, usize) = (8, 18);
const ROCK_ONLY_ROCKS: (usize, usize) = (6, 14);
const PLANET_HEAVY_ROCKS: (usize, usize) = (3, 8);
const LOW_ROCK_ROCKS: (usize, usize) = (2, 4);

/// How many hulls a derelict-only group places, more with more traffic.
///
/// Hulls are the costliest body to materialize, so no kind places many.
const DERELICT_ONLY_HULLS: (usize, usize) = (2, 3);

/// The highest chance an asteroid-rich group places its one hull, at full
/// traffic.
const ASTEROID_RICH_HULL_CHANCE_MAX: f32 = 0.4;

/// The highest chance an asteroid-rich group places one or two worlds, and
/// the highest chance it places two, both at full quiet material.
const ASTEROID_RICH_WORLD_CHANCE_MAX: f32 = 0.55;
const ASTEROID_RICH_TWIN_CHANCE_MAX: f32 = 0.2;

/// The highest chance a cell draws its one background rock.
const BACKGROUND_CHANCE_MAX: f32 = 0.6;

/// How much of its cell's half edge the background rock's centre is drawn in.
const BACKGROUND_INSET: f32 = 0.9;

/// Extra reach given past a derived bound, for the rounding of an `f32`
/// position at a few hundred kilometres.
const ROUNDING_SLACK: Meters = Meters(1.0);

/// Every natural asteroid kind. `plain` is the rendering control.
const ROCK_KINDS: [&str; 4] = [KIND_ROCK, KIND_METAL, KIND_ICE, KIND_CARBON];

/// The shipped hulls a group's derelicts are drawn from: the damaged frame
/// tender and loose wreck plating.
const DERELICT_DESIGNS: [&str; 2] = [
    BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID,
    BLOCK_WRECK_PLATE_SHIP_ID,
];

/// The clustered world's sector generator.
///
/// No fields. Its content is the shipped content named in this module, so
/// there is no list a caller could leave empty or fill with an unknown id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClusteredWorld;

impl SectorGenerator for ClusteredWorld {
    /// Refuse an edge too narrow to hold the widest pair of worlds inside
    /// one cell.
    fn validate(&self, geometry: WorldGeometry) -> Result<(), SectorFault> {
        let core = core_reach_max() + ROUNDING_SLACK;
        geometry.require_owning_edge(
            0.0,
            core,
            &format!("a pair of planetoids reaching {} m", core.get()),
        )
    }

    fn generate(&self, input: SectorGenerationInput) -> Result<SectorManifest, SectorFault> {
        Ok(plan_cell(&EnvironmentFields::new(input.seed), input)?.manifest())
    }
}

/// A group's lattice node: its identity from every cell that can see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupId(pub [i32; 3]);

impl GroupId {
    /// The group's id text, and the stem of every body id it names.
    pub fn slug(self) -> String {
        let [x, y, z] = self.0.map(|index| {
            if index < 0 {
                format!("n{}", index.unsigned_abs())
            } else {
                index.to_string()
            }
        });
        format!("group_{x}_{y}_{z}")
    }
}

/// What a group is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GroupKind {
    /// Many rocks, and in quiet material one or two planetoids, and with
    /// traffic one derelict hull.
    AsteroidRich,
    /// Rocks and nothing else.
    RockOnly,
    /// Two planetoids with a few rocks around them.
    PlanetHeavy,
    /// Derelict hulls and nothing else.
    DerelictOnly,
    /// A few rocks and nothing else.
    LowRock,
}

impl GroupKind {
    /// Every kind, in the order a readout and a legend list them. Declaration
    /// order too, so `kind as usize` indexes an array kept in this order.
    pub const ALL: [Self; 5] = [
        Self::AsteroidRich,
        Self::RockOnly,
        Self::PlanetHeavy,
        Self::DerelictOnly,
        Self::LowRock,
    ];

    /// What a readout calls the kind.
    pub const fn label(self) -> &'static str {
        match self {
            Self::AsteroidRich => "asteroid-rich",
            Self::RockOnly => "rock-only",
            Self::PlanetHeavy => "planet-heavy",
            Self::DerelictOnly => "derelict-only",
            Self::LowRock => "low-rock",
        }
    }
}

/// One body the clustered world plans, before a cell decides whether it fits.
#[derive(Clone, Debug)]
pub enum ClusterBody {
    /// A rock.
    Rock {
        /// Its nominal radius.
        radius: Meters,
        /// Its asteroid kind id.
        kind: &'static str,
    },
    /// A world.
    Planetoid(PlanetConfig),
    /// An inert hull with nobody aboard.
    Hull {
        /// Its catalog design id.
        design: &'static str,
        /// Which way it points. Yaw only.
        yaw: f32,
    },
}

impl ClusterBody {
    /// The clearance sphere `validate_manifest` measures the body by.
    pub fn clearance(&self) -> Meters {
        match self {
            Self::Rock { radius, .. } => Meters(radius.get() * ASTEROID_GEOMETRIC_FACTOR_MAX),
            Self::Planetoid(config) => config.body_radius(),
            Self::Hull { .. } => SECTOR_SHIP_CLEARANCE,
        }
    }
}

/// One body of a group, where it stands, and the cell that owns it.
#[derive(Clone, Debug)]
pub struct GroupBody {
    /// What it is.
    pub body: ClusterBody,
    /// Where its centre stands, in meters from the world origin.
    pub position: Meters3,
    /// The cell its centre falls in: the one cell that places or skips it.
    pub owner: SectorCoord,
}

/// One group, decided at its lattice node. The same value from every cell.
#[derive(Clone, Debug)]
pub struct ClusterGroup {
    /// The node it was decided at.
    pub id: GroupId,
    /// What it is.
    pub kind: GroupKind,
    /// The fields at its drawn anchor, which every chance and kind in it read.
    pub environment: Environment,
    /// The group's centre: every body stands around it. Moved into its cell
    /// only when the group has worlds to keep inside it.
    pub anchor: Meters3,
    /// The cell the anchor is in, and so every parent's owner.
    pub home: SectorCoord,
    /// Its planetoids, none to two. Each one must be placed.
    pub parents: Vec<GroupBody>,
    /// Its hulls, then its rocks, in draw order. Each one may be skipped.
    pub members: Vec<GroupBody>,
}

impl ClusterGroup {
    /// Every body with its source: the parents, then the members.
    pub fn bodies(&self) -> impl Iterator<Item = (BodySource, &GroupBody)> {
        let id = self.id;
        let parents = self
            .parents
            .iter()
            .enumerate()
            .map(move |(index, body)| (BodySource::Parent(id, index), body));
        let members = self
            .members
            .iter()
            .enumerate()
            .map(move |(index, body)| (BodySource::Member(id, index), body));
        parents.chain(members)
    }

    /// Whether its bodies are owned by more than one cell.
    pub fn spans_seam(&self) -> bool {
        let mut owners = self.bodies().map(|(_, body)| body.owner);
        let first = owners.next();
        owners.any(|owner| Some(owner) != first)
    }
}

/// Where a planned body came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BodySource {
    /// A group's parent, by its index in [`ClusterGroup::parents`].
    Parent(GroupId, usize),
    /// A group's member, by its index in [`ClusterGroup::members`].
    Member(GroupId, usize),
    /// The cell's own background rock.
    Background,
}

impl BodySource {
    /// The group the body belongs to, or `None` for a background rock.
    pub fn group(self) -> Option<GroupId> {
        match self {
            Self::Parent(group, _) | Self::Member(group, _) => Some(group),
            Self::Background => None,
        }
    }
}

/// Why a planned body was not placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkipReason {
    /// Its clearance sphere crosses a face of the cell its centre is in.
    Face,
    /// It overlaps, or comes within [`MEMBER_MARGIN`] of, a body placed
    /// before it.
    Clearance,
}

impl SkipReason {
    /// Every reason, in the order a readout lists them. Declaration order
    /// too, so `reason as usize` indexes an array kept in this order.
    pub const ALL: [Self; 2] = [Self::Face, Self::Clearance];

    /// What a readout calls the reason.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Face => "face",
            Self::Clearance => "clearance",
        }
    }
}

/// What happened to a planned body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// It is in the manifest.
    Placed,
    /// It is not, for this reason.
    Skipped(SkipReason),
}

/// One body a cell owns, and what the cell did with it.
#[derive(Clone, Debug)]
pub struct PlannedBody {
    /// Its scenario id, prefixed with the owning cell's slug.
    pub id: String,
    /// Where it came from.
    pub source: BodySource,
    /// What it is.
    pub body: ClusterBody,
    /// Where it stands.
    pub position: Meters3,
    /// Placed, or skipped and why.
    pub outcome: Outcome,
}

/// Everything one cell planned: the groups it owns bodies of, and every body
/// it owns with its outcome.
#[derive(Clone, Debug)]
pub struct CellPlan {
    /// The cell.
    pub coord: SectorCoord,
    /// The fields at the cell's centre, which the background rock reads.
    pub environment: Environment,
    /// Every group with at least one body this cell owns, by node.
    pub groups: Vec<ClusterGroup>,
    /// Every body this cell owns, in resolve order.
    pub bodies: Vec<PlannedBody>,
}

impl CellPlan {
    /// How many planned bodies were placed.
    pub fn placed(&self) -> usize {
        self.bodies
            .iter()
            .filter(|body| body.outcome == Outcome::Placed)
            .count()
    }

    /// How many planned bodies were skipped for `reason`.
    pub fn skipped(&self, reason: SkipReason) -> usize {
        self.bodies
            .iter()
            .filter(|body| body.outcome == Outcome::Skipped(reason))
            .count()
    }

    /// The placed bodies as the manifest `nova_world` checks.
    pub fn manifest(&self) -> SectorManifest {
        let mut manifest = SectorManifest {
            coord: self.coord,
            asteroids: Vec::new(),
            planets: Vec::new(),
            ships: Vec::new(),
        };
        for planned in &self.bodies {
            if planned.outcome != Outcome::Placed {
                continue;
            }
            let id = planned.id.clone();
            let position = planned.position;
            match &planned.body {
                ClusterBody::Rock { radius, kind } => manifest.asteroids.push(SectorAsteroid {
                    seed: asteroid_seed_from_id(&id),
                    id,
                    position,
                    radius: *radius,
                    kind: kind.to_string(),
                }),
                ClusterBody::Planetoid(config) => manifest.planets.push(SectorPlanet {
                    id,
                    position,
                    config: config.clone(),
                }),
                ClusterBody::Hull { design, yaw } => manifest.ships.push(SectorShip {
                    id,
                    position,
                    yaw: *yaw,
                    design: design.to_string(),
                }),
            }
        }
        manifest
    }
}

/// The chances a node grows each kind of group, and none.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroupChances {
    /// The chance of each kind, in [`GroupKind::ALL`] order.
    pub kinds: [f32; GroupKind::ALL.len()],
    /// The chance of no group.
    pub none: f32,
}

/// The chances the fields at an anchor give each kind of group.
///
/// Each kind has a weight read off the fields together, and the chances are
/// the weights over their sum, so no field is a hard edge. Dense material
/// grows asteroid-rich and rock-only groups, dense QUIET material with
/// volatiles grows planet-heavy ones, traffic in thin material grows wreck
/// fields, and thin material grows low-rock scatters or nothing - more often
/// nothing where traffic is low too.
pub fn group_chances(environment: Environment) -> GroupChances {
    let weights = group_weights(environment);
    let total: f32 = weights.iter().map(|(_, weight)| weight).sum();
    GroupChances {
        kinds: GroupKind::ALL.map(|kind| {
            weights
                .iter()
                .find(|(choice, _)| *choice == Some(kind))
                .map_or(0.0, |(_, weight)| weight / total)
        }),
        none: weights[weights.len() - 1].1 / total,
    }
}

/// The weight of each kind, in [`GroupKind::ALL`] order, and of no group
/// last. The low-rock and no-group weights never reach zero, so the total is
/// always positive.
fn group_weights(environment: Environment) -> [(Option<GroupKind>, f32); 6] {
    let Environment {
        material_density: m,
        volatiles: v,
        human_activity: h,
    } = environment;
    [
        (Some(GroupKind::AsteroidRich), 0.9 * m),
        (Some(GroupKind::RockOnly), 0.5 * m * (1.0 - h)),
        (
            Some(GroupKind::PlanetHeavy),
            0.6 * ramp(m, 0.45, 0.85) * (1.0 - 0.7 * h) * (0.6 + 0.4 * v),
        ),
        (
            Some(GroupKind::DerelictOnly),
            0.7 * ramp(h, 0.4, 0.8) * (1.0 - 0.5 * m),
        ),
        (Some(GroupKind::LowRock), 0.1 + 0.4 * (1.0 - m)),
        (None, 0.1 + 0.4 * (1.0 - m) * (1.0 - h)),
    ]
}

/// The chance a cell draws its background rock: dense material, cleared by
/// traffic.
fn background_chance(environment: Environment) -> f32 {
    BACKGROUND_CHANCE_MAX * environment.material_density * (1.0 - 0.7 * environment.human_activity)
}

/// A smoothstep from `low` to `high`.
fn ramp(value: f32, low: f32, high: f32) -> f32 {
    let t = ((value - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Pick one of `choices` by weight with a draw in `[0, 1)`.
///
/// The total weight is positive by construction at every caller.
fn pick<T: Copy>(choices: &[(T, f32)], draw: f32) -> T {
    let total: f32 = choices.iter().map(|(_, weight)| weight).sum();
    let mut target = draw * total;
    for &(choice, weight) in choices {
        if target < weight {
            return choice;
        }
        target -= weight;
    }
    // A draw just under one can round past the last running total.
    choices[choices.len() - 1].0
}

/// A rock's kind: dry dense material is rock and metal, volatile material is
/// ice, and where volatiles meet dense material it is carbon.
fn rock_kind(environment: Environment, draw: f32) -> &'static str {
    let Environment {
        material_density: m,
        volatiles: v,
        human_activity: h,
    } = environment;
    let [rock, metal, ice, carbon] = ROCK_KINDS;
    pick(
        &[
            (rock, 0.3 + 0.4 * m * (1.0 - v)),
            (metal, 0.1 + 0.5 * m * (1.0 - v)),
            // Traffic has already taken the ice.
            (ice, 0.05 + 0.8 * v * (1.0 - h)),
            (carbon, 0.1 + 0.6 * v * m),
        ],
        draw,
    )
}

/// A planetoid's type: dry dense space makes barren and molten worlds,
/// volatile space ice worlds, and volatiles with material or with traffic the
/// hazy and living ones.
fn planet_type(environment: Environment, draw: f32) -> PlanetType {
    let Environment {
        material_density: m,
        volatiles: v,
        human_activity: h,
    } = environment;
    pick(
        &[
            (PlanetType::BarrenRock, 0.2 + 0.6 * m * (1.0 - v)),
            (PlanetType::DustWorld, 0.1 + 0.4 * (1.0 - v)),
            (PlanetType::IceWorld, 0.05 + 0.8 * v),
            (PlanetType::Volcanic, 0.05 + 0.5 * m * (1.0 - v)),
            (PlanetType::Greenhouse, 0.05 + 0.5 * v * m),
            (PlanetType::Temperate, 0.05 + 0.4 * v * h),
        ],
        draw,
    )
}

/// A derelict hull's design: tenders where traffic was, plating where it
/// worked material.
fn derelict_design(environment: Environment, draw: f32) -> &'static str {
    let [tender, plate] = DERELICT_DESIGNS;
    pick(
        &[
            (tender, 0.2 + 0.8 * environment.human_activity),
            (plate, 0.2 + 0.6 * environment.material_density),
        ],
        draw,
    )
}

/// How many of a `band` a group places when `richness` reads in `[0, 1]`: a
/// richer anchor widens the draw toward the top of the band.
fn member_count(band: (usize, usize), richness: f32, draw: f32) -> usize {
    let (min, max) = band;
    let span = 1.0 + (max - min) as f32 * richness;
    (min + (draw * span) as usize).min(max)
}

/// A value `draw` of the way across `band`.
fn across(band: (Meters, Meters), draw: f32) -> Meters {
    band.0 + (band.1 - band.0) * draw
}

fn rock(environment: Environment, stream: &mut SeedStream) -> ClusterBody {
    ClusterBody::Rock {
        radius: across(ROCK_RADIUS, stream.unit()),
        kind: rock_kind(environment, stream.unit()),
    }
}

fn hull(environment: Environment, stream: &mut SeedStream) -> ClusterBody {
    let yaw = stream.unit() * std::f32::consts::TAU;
    ClusterBody::Hull {
        design: derelict_design(environment, stream.unit()),
        yaw,
    }
}

/// The group decided at `node`, or `None` for a node that grows none.
///
/// PURE, and never keyed by the cell asking, so every cell that replays a node
/// gets the same group. Every draw comes off one stream keyed by the seed and
/// the node alone, in one fixed order. The edge draws nothing: it only names
/// the home cell and each member's owning cell, and clamps a core with worlds
/// inside the home cell.
///
/// # Errors
///
/// Whatever the environment refuses, and [`SectorFault::InvalidGeometry`] for
/// a body with no finite position.
pub fn group_at(
    fields: &EnvironmentFields,
    seed: u32,
    edge: Meters,
    node: [i32; 3],
) -> Result<Option<ClusterGroup>, SectorFault> {
    let id = GroupId(node);
    let mut stream = SeedStream::new(
        Fnv32::new()
            .write(&seed.to_le_bytes())
            .write(b"cluster_group")
            .write(&node[0].to_le_bytes())
            .write(&node[1].to_le_bytes())
            .write(&node[2].to_le_bytes())
            .finish(),
    );
    let lattice = GROUP_LATTICE.get();
    let jitter = GROUP_JITTER * lattice * 0.5;
    let [x, y, z] = node.map(|index| index as f32 * lattice);
    let drawn = Meters3::new(
        x + stream.signed() * jitter,
        y + stream.signed() * jitter,
        z + stream.signed() * jitter,
    );
    let environment = fields.sample(drawn)?;
    let Some(kind) = pick(&group_weights(environment), stream.unit()) else {
        return Ok(None);
    };
    let Environment {
        material_density: m,
        human_activity: h,
        ..
    } = environment;

    let worlds = match kind {
        GroupKind::PlanetHeavy => 2,
        GroupKind::AsteroidRich => {
            let quiet_material = m * (1.0 - h);
            let draw = stream.unit();
            if draw < ASTEROID_RICH_TWIN_CHANCE_MAX * quiet_material {
                2
            } else if draw < ASTEROID_RICH_WORLD_CHANCE_MAX * quiet_material {
                1
            } else {
                0
            }
        }
        GroupKind::RockOnly | GroupKind::DerelictOnly | GroupKind::LowRock => 0,
    };
    let hulls = match kind {
        GroupKind::DerelictOnly => member_count(DERELICT_ONLY_HULLS, h, stream.unit()),
        GroupKind::AsteroidRich => usize::from(stream.unit() < ASTEROID_RICH_HULL_CHANCE_MAX * h),
        _ => 0,
    };
    let rocks = match kind {
        GroupKind::AsteroidRich => ASTEROID_RICH_ROCKS,
        GroupKind::RockOnly => ROCK_ONLY_ROCKS,
        GroupKind::PlanetHeavy => PLANET_HEAVY_ROCKS,
        GroupKind::LowRock => LOW_ROCK_ROCKS,
        GroupKind::DerelictOnly => (0, 0),
    };
    let rocks = member_count(rocks, m, stream.unit());

    // Worlds first: their size decides how far the anchor must stand from
    // every face for each world to sit wholly inside the anchor's cell.
    let configs: Vec<PlanetConfig> = (0..worlds)
        .map(|_| {
            let radius = across(PLANETOID_RADIUS, stream.unit());
            let planet_type = planet_type(environment, stream.unit());
            PlanetConfig::new(planet_type, radius, stream.next_u32())
        })
        .collect();
    let (offset, axis) = if worlds == 2 {
        let offset = across(TWIN_OFFSET, stream.unit());
        (offset, unit_sphere_point(stream.next_u32()))
    } else {
        (Meters::ZERO, Vec3::ZERO)
    };
    let core = configs.iter().fold(Meters::ZERO, |widest, config| {
        widest.max(config.body_radius())
    }) + offset;

    let home = SectorCoord::containing(drawn, edge);
    let anchor = if configs.is_empty() {
        drawn
    } else {
        // The anchor is pulled in until the whole core is inside its cell.
        // `validate` refused an edge under two core reaches, so the box is
        // never inverted.
        let centre = home.centre(edge).get();
        let half = Vec3::splat(edge.get() * 0.5 - (core + ROUNDING_SLACK).get());
        Meters3(centre + (drawn.get() - centre).clamp(-half, half))
    };
    let invalid = || SectorFault::InvalidGeometry { id: id.slug() };
    if !anchor.get().is_finite() {
        return Err(invalid());
    }
    let parents = configs
        .into_iter()
        .zip([1.0, -1.0])
        .map(|(config, side)| {
            let position = Meters3(anchor.get() + axis * offset.get() * side);
            GroupBody {
                body: ClusterBody::Planetoid(config),
                position,
                owner: home,
            }
        })
        .collect();

    // Members stand in a shell past the core, denser toward its inner edge.
    let (inner, spread) = if worlds == 0 {
        (Meters::ZERO, across(OPEN_SPREAD, stream.unit()))
    } else {
        (
            core + member_clearance_max() + MEMBER_MARGIN,
            across(WORLD_SPREAD, stream.unit()),
        )
    };
    let mut members = Vec::with_capacity(hulls + rocks);
    for index in 0..hulls + rocks {
        let direction = unit_sphere_point(stream.next_u32());
        let distance = inner + spread * stream.unit().sqrt();
        let position = Meters3(anchor.get() + direction * distance.get());
        if !position.get().is_finite() {
            return Err(invalid());
        }
        let body = if index < hulls {
            hull(environment, &mut stream)
        } else {
            rock(environment, &mut stream)
        };
        members.push(GroupBody {
            body,
            position,
            owner: SectorCoord::containing(position, edge),
        });
    }

    Ok(Some(ClusterGroup {
        id,
        kind,
        environment,
        anchor,
        home,
        parents,
        members,
    }))
}

/// The widest clearance a member can have: a hull's, or the widest rock's.
fn member_clearance_max() -> Meters {
    Meters(ROCK_RADIUS.1.get() * ASTEROID_GEOMETRIC_FACTOR_MAX).max(SECTOR_SHIP_CLEARANCE)
}

/// The widest planetoid clearance this generator can draw: the OUTER radius
/// of the widest type at the top of the band, which is what
/// `validate_manifest` measures it by.
fn widest_planetoid() -> Meters {
    PlanetType::ALL
        .iter()
        .map(|planet_type| PlanetConfig::new(*planet_type, PLANETOID_RADIUS.1, 0).body_radius())
        .fold(Meters::ZERO, Meters::max)
}

/// How far from its anchor the widest core reaches: a world at the outer twin
/// offset with the widest clearance. Also the most the anchor is moved.
fn core_reach_max() -> Meters {
    TWIN_OFFSET.1 + widest_planetoid()
}

/// How far from its anchor a member centre can stand.
fn member_reach_max() -> Meters {
    (core_reach_max() + member_clearance_max() + MEMBER_MARGIN + WORLD_SPREAD.1).max(OPEN_SPREAD.1)
}

/// How far on one axis a group's node can be from a body centre the group
/// places.
///
/// An anchor leaves its node by one jitter draw and, with worlds, one core
/// pull. A parent stands within the core and a member within
/// [`member_reach_max`] of the anchor. The distance along one axis is at most
/// the distance itself, so this bounds every axis.
fn group_reach() -> Meters {
    Meters(GROUP_JITTER * GROUP_LATTICE.get() * 0.5)
        + core_reach_max()
        + ROUNDING_SLACK
        + member_reach_max()
}

/// Every lattice node whose group could place a body centre in `coord`, in
/// node order.
///
/// A body centre in the cell is within half an edge of its centre on each
/// axis, and within [`group_reach`] of its node, so a node further than both
/// together on any axis cannot reach the cell. At a 32 km edge that is about
/// 27 km each way: two or three nodes an axis, 8 to 27 in all.
fn halo_nodes(coord: SectorCoord, edge: Meters) -> Vec<[i32; 3]> {
    let reach = edge.get() * 0.5 + group_reach().get() + ROUNDING_SLACK.get();
    let lattice = GROUP_LATTICE.get();
    let centre = coord.centre(edge).get();
    let span = |axis: f32| {
        ((axis - reach) / lattice).ceil() as i32..=((axis + reach) / lattice).floor() as i32
    };
    let mut nodes = Vec::new();
    for x in span(centre.x) {
        for y in span(centre.y) {
            for z in span(centre.z) {
                nodes.push([x, y, z]);
            }
        }
    }
    nodes
}

/// Plan one cell: replay every group in its halo, keep the bodies it owns,
/// add its background rock, and resolve them all.
///
/// PURE: the same fields and input give the same plan, on any call, in any
/// order.
///
/// # Errors
///
/// Whatever the environment refuses, [`SectorFault::InvalidGeometry`] for a
/// body with no finite position, and [`SectorFault::Generation`] for a parent
/// that could not be placed.
pub fn plan_cell(
    fields: &EnvironmentFields,
    input: SectorGenerationInput,
) -> Result<CellPlan, SectorFault> {
    let coord = input.coord;
    let edge = input.geometry.sector_edge;
    let mut groups = Vec::new();
    let mut parents = Vec::new();
    let mut members = Vec::new();
    for node in halo_nodes(coord, edge) {
        let Some(group) = group_at(fields, input.seed, edge, node)? else {
            continue;
        };
        let stem = group.id.slug();
        let own = |role: &str, index: usize, source: BodySource, body: &GroupBody| {
            (body.owner == coord).then(|| Candidate {
                id: sector_id(coord, &format!("{stem}_{role}"), index),
                source,
                body: body.body.clone(),
                position: body.position,
            })
        };
        let before = parents.len() + members.len();
        parents.extend(
            group
                .parents
                .iter()
                .enumerate()
                .filter_map(|(index, body)| {
                    own("parent", index, BodySource::Parent(group.id, index), body)
                }),
        );
        members.extend(
            group
                .members
                .iter()
                .enumerate()
                .filter_map(|(index, body)| {
                    own("member", index, BodySource::Member(group.id, index), body)
                }),
        );
        if parents.len() + members.len() > before {
            groups.push(group);
        }
    }

    let centre = coord.centre(edge);
    let environment = fields.sample(centre)?;
    let mut background = Vec::new();
    let mut stream = input.stream("background");
    if stream.unit() < background_chance(environment) {
        let reach = edge.get() * 0.5 * BACKGROUND_INSET;
        let position = centre
            + Meters3::new(
                stream.signed() * reach,
                stream.signed() * reach,
                stream.signed() * reach,
            );
        background.push(Candidate {
            id: sector_id(coord, "background_rock", 0),
            source: BodySource::Background,
            body: rock(environment, &mut stream),
            position,
        });
    }

    let candidates = parents.into_iter().chain(members).chain(background);
    Ok(CellPlan {
        coord,
        environment,
        groups,
        bodies: resolve(input, candidates)?,
    })
}

/// One body a cell owns, before it is resolved.
struct Candidate {
    id: String,
    source: BodySource,
    body: ClusterBody,
    position: Meters3,
}

/// Place or skip every candidate, in the order given.
///
/// The checks mirror `validate_manifest` - the whole clearance sphere inside
/// the cell, no overlap - so a placed body is one the check accepts, and each
/// refusal becomes a counted skip instead of a refused manifest.
///
/// # Errors
///
/// [`SectorFault::Generation`] for a parent that would be skipped, and
/// [`SectorFault::InvalidGeometry`] for a position that is not finite.
fn resolve(
    input: SectorGenerationInput,
    candidates: impl IntoIterator<Item = Candidate>,
) -> Result<Vec<PlannedBody>, SectorFault> {
    let edge = input.geometry.sector_edge;
    let centre = input.coord.centre(edge).get();
    let mut standing: Vec<(Meters3, Meters)> = Vec::new();
    let mut planned = Vec::new();
    for Candidate {
        id,
        source,
        body,
        position,
    } in candidates
    {
        if !position.get().is_finite() {
            return Err(SectorFault::InvalidGeometry { id });
        }
        let clearance = body.clearance();
        let inside = SectorCoord::containing(position, edge) == input.coord
            && (position.get() - centre).abs().max_element() + clearance.get() <= edge.get() * 0.5;
        let outcome = if !inside {
            Outcome::Skipped(SkipReason::Face)
        } else if !standing.iter().all(|&(other, other_clearance)| {
            bodies_clear(other, other_clearance, position, clearance, MEMBER_MARGIN)
        }) {
            Outcome::Skipped(SkipReason::Clearance)
        } else {
            Outcome::Placed
        };
        if let (BodySource::Parent(..), Outcome::Skipped(reason)) = (source, outcome) {
            return Err(SectorFault::Generation {
                id,
                field: "parent",
                value: format!("skipped for {}", reason.label()),
            });
        }
        if outcome == Outcome::Placed {
            standing.push((position, clearance));
        }
        planned.push(PlannedBody {
            id,
            source,
            body,
            position,
            outcome,
        });
    }
    Ok(planned)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{
        super::{clustered_world_config, CLUSTER_HOME, EXAMPLE_ACTIVE_RADIUS},
        *,
    };

    fn window_plans() -> BTreeMap<SectorCoord, CellPlan> {
        let config = clustered_world_config();
        let fields = EnvironmentFields::new(config.seed);
        desired_sectors(CLUSTER_HOME, EXAMPLE_ACTIVE_RADIUS)
            .into_iter()
            .map(|coord| {
                let plan = plan_cell(&fields, config.input(coord))
                    .unwrap_or_else(|fault| panic!("{coord}: {fault}"));
                (coord, plan)
            })
            .collect()
    }

    /// Every group decided at the nodes of an 8-node cube around the home
    /// cell: about 400 groups, every kind many times over.
    fn scanned_groups() -> Vec<ClusterGroup> {
        let config = clustered_world_config();
        let fields = EnvironmentFields::new(config.seed);
        let mut groups = Vec::new();
        for x in -4..4 {
            for y in -4..4 {
                for z in -4..4 {
                    let group = group_at(&fields, config.seed, config.sector_edge, [x, y, z])
                        .unwrap_or_else(|fault| panic!("[{x}, {y}, {z}]: {fault}"));
                    groups.extend(group);
                }
            }
        }
        groups
    }

    /// Every cell that replays a node gets the same group, down to the last
    /// body, and the same group a direct call at the node gets.
    #[test]
    fn a_group_is_the_same_group_from_every_cell_that_replays_it() {
        let config = clustered_world_config();
        let fields = EnvironmentFields::new(config.seed);
        let mut seen: BTreeMap<GroupId, (String, usize)> = BTreeMap::new();
        for plan in window_plans().values() {
            for group in &plan.groups {
                let text = format!("{group:?}");
                let (first, cells) = seen.entry(group.id).or_insert_with(|| (text.clone(), 0));
                assert_eq!(
                    *first,
                    text,
                    "{} differs from {}",
                    group.id.slug(),
                    plan.coord
                );
                *cells += 1;
            }
        }
        assert!(
            seen.values().any(|(_, cells)| *cells >= 2),
            "some group must be replayed by more than one cell"
        );
        for (id, (text, _)) in &seen {
            let direct = group_at(&fields, config.seed, config.sector_edge, id.0)
                .unwrap_or_else(|fault| panic!("{}: {fault}", id.slug()))
                .unwrap_or_else(|| panic!("{} grew no group at its node", id.slug()));
            assert_eq!(*text, format!("{direct:?}"), "{} at its node", id.slug());
        }
    }

    /// The home window holds a group with a planetoid and a group with a hull
    /// that each PLACE bodies in two cells, and the owning cell's streamed
    /// description holds each of those bodies where the group put it.
    #[test]
    fn world_and_hull_groups_in_the_home_window_place_bodies_on_both_sides_of_a_face() {
        let config = clustered_world_config();
        let plans = window_plans();
        let mut cells: BTreeMap<GroupId, BTreeSet<SectorCoord>> = BTreeMap::new();
        let mut holds: BTreeMap<GroupId, (bool, bool)> = BTreeMap::new();
        for plan in plans.values() {
            for body in &plan.bodies {
                let (Some(group), Outcome::Placed) = (body.source.group(), body.outcome) else {
                    continue;
                };
                cells.entry(group).or_default().insert(plan.coord);
                let (world, hull) = holds.entry(group).or_default();
                *world |= matches!(body.body, ClusterBody::Planetoid(_));
                *hull |= matches!(body.body, ClusterBody::Hull { .. });
            }
        }
        let wants: [(&str, fn((bool, bool)) -> bool); 2] = [
            ("planetoid", |(world, _)| world),
            ("hull", |(_, hull)| hull),
        ];
        for (holding, holds_it) in wants {
            let spanning: Vec<GroupId> = cells
                .iter()
                .filter(|(group, owners)| holds_it(holds[*group]) && owners.len() >= 2)
                .map(|(group, _)| *group)
                .collect();
            assert!(
                !spanning.is_empty(),
                "the home window must hold a group placing a {holding} and bodies in two cells"
            );
            for group in spanning {
                for coord in &cells[&group] {
                    let description = generate_sector(&config, *coord)
                        .unwrap_or_else(|fault| panic!("{coord}: {fault}"));
                    let canonical = description.canonical();
                    for body in plans[coord].bodies.iter().filter(|body| {
                        body.source.group() == Some(group) && body.outcome == Outcome::Placed
                    }) {
                        let at = body.position.get();
                        let line = format!(" {} {:.2} {:.2} {:.2} ", body.id, at.x, at.y, at.z);
                        assert!(
                            canonical.contains(&line),
                            "{coord} must stream {} where {} put it",
                            body.id,
                            group.slug()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_window_generated_in_reverse_order_is_the_same_window() {
        let config = clustered_world_config();
        let cells: Vec<SectorCoord> = desired_sectors(CLUSTER_HOME, EXAMPLE_ACTIVE_RADIUS)
            .into_iter()
            .collect();
        let describe = |coord: &SectorCoord| {
            let description =
                generate_sector(&config, *coord).unwrap_or_else(|fault| panic!("{coord}: {fault}"));
            (*coord, description.canonical())
        };
        let forward: BTreeMap<_, _> = cells.iter().map(describe).collect();
        let reverse: BTreeMap<_, _> = cells.iter().rev().map(describe).collect();
        assert_eq!(forward, reverse);
    }

    /// Every body a group plans is owned by exactly one cell - the one its
    /// centre falls in - and placed or skipped there, and a cell's placed
    /// count is what it streams, however many that is.
    #[test]
    fn every_planned_body_has_one_owner_that_places_or_skips_it() {
        let config = clustered_world_config();
        let plans = window_plans();
        let mut owners: BTreeMap<BodySource, Vec<SectorCoord>> = BTreeMap::new();
        let mut groups = BTreeMap::new();
        for plan in plans.values() {
            let description = generate_sector(&config, plan.coord)
                .unwrap_or_else(|fault| panic!("{}: {fault}", plan.coord));
            assert_eq!(description.object_count(), plan.placed(), "{}", plan.coord);
            // A background rock is keyed by its cell, so only group bodies
            // can be claimed twice.
            for body in plan
                .bodies
                .iter()
                .filter(|body| body.source.group().is_some())
            {
                owners.entry(body.source).or_default().push(plan.coord);
            }
            for group in &plan.groups {
                groups.insert(group.id, group.clone());
            }
        }
        // A group whose anchor stands one cell inside the window has every
        // body inside it: no body stands 7 km from its anchor, and a cell is
        // 32 km.
        let inner = desired_sectors(CLUSTER_HOME, EXAMPLE_ACTIVE_RADIUS - 1);
        let mut checked = 0;
        for group in groups.values() {
            if !inner.contains(&group.home) {
                continue;
            }
            for (source, body) in group.bodies() {
                assert_eq!(
                    owners.get(&source),
                    Some(&vec![body.owner]),
                    "{source:?} of {} must be planned once, by its owner",
                    group.id.slug()
                );
            }
            checked += 1;
        }
        assert!(checked > 0, "the inner window must hold a group's anchor");
        assert!(
            owners.values().all(|cells| cells.len() == 1),
            "no group body may be planned by two cells"
        );
    }

    /// A member over a body placed before it is skipped by clearance and
    /// counted - and a parent that would be skipped is a fault, never a skip.
    #[test]
    fn a_crowded_member_is_skipped_but_a_crowded_parent_is_refused() {
        let input = clustered_world_config().input(SectorCoord::ORIGIN);
        let group = GroupId([0, 0, 0]);
        let hull = |id: &str, source: BodySource| Candidate {
            id: id.to_string(),
            source,
            body: ClusterBody::Hull {
                design: BLOCK_WRECK_PLATE_SHIP_ID,
                yaw: 0.0,
            },
            position: Meters3::new(0.0, 0.0, 12_000.0),
        };
        let planned = resolve(
            input,
            [
                hull("first", BodySource::Member(group, 0)),
                hull("second", BodySource::Member(group, 1)),
            ],
        )
        .expect("members are skipped, never refused");
        assert_eq!(
            planned.iter().map(|body| body.outcome).collect::<Vec<_>>(),
            [Outcome::Placed, Outcome::Skipped(SkipReason::Clearance)]
        );

        let fault = resolve(
            input,
            [
                hull("first", BodySource::Member(group, 0)),
                hull("parent", BodySource::Parent(group, 0)),
            ],
        )
        .expect_err("a parent over a placed body must refuse");
        assert!(
            matches!(&fault, SectorFault::Generation { id, field: "parent", .. } if id == "parent"),
            "got {fault:?}"
        );
    }

    #[test]
    fn an_edge_too_narrow_to_hold_a_pair_of_worlds_is_refused() {
        let validate = |edge: f32| {
            ClusteredWorld.validate(WorldGeometry {
                sector_edge: Meters(edge),
            })
        };
        let floor = 2.0 * (core_reach_max() + ROUNDING_SLACK).get();
        let fault = validate(floor - 10.0).expect_err("the edge must refuse");
        assert!(
            matches!(&fault, SectorFault::Config { field: "sector_edge", value } if value.contains("planetoids")),
            "got {fault:?}"
        );
        for edge in [floor, 32_000.0, 128_000.0] {
            validate(edge).unwrap_or_else(|fault| panic!("{edge} m must arm, got {fault}"));
        }
    }

    /// Each kind places only the bodies it names, in the counts its bands
    /// allow; every body stands where its shell says; every world is whole
    /// inside its anchor's cell; and the scan grows every kind.
    #[test]
    fn each_kind_places_only_its_own_bodies() {
        let edge = clustered_world_config().sector_edge;
        let mut grown = BTreeSet::new();
        for group in scanned_groups() {
            grown.insert(group.kind);
            let slug = group.id.slug();
            let count = |kind: fn(&ClusterBody) -> bool| {
                group
                    .members
                    .iter()
                    .filter(|member| kind(&member.body))
                    .count()
            };
            let rocks = count(|body| matches!(body, ClusterBody::Rock { .. }));
            let hulls = count(|body| matches!(body, ClusterBody::Hull { .. }));
            let worlds = group.parents.len();
            let within = |value: usize, (min, max): (usize, usize)| (min..=max).contains(&value);
            let composed = match group.kind {
                GroupKind::AsteroidRich => {
                    worlds <= 2 && hulls <= 1 && within(rocks, ASTEROID_RICH_ROCKS)
                }
                GroupKind::RockOnly => worlds == 0 && hulls == 0 && within(rocks, ROCK_ONLY_ROCKS),
                GroupKind::PlanetHeavy => {
                    worlds == 2 && hulls == 0 && within(rocks, PLANET_HEAVY_ROCKS)
                }
                GroupKind::DerelictOnly => {
                    worlds == 0 && rocks == 0 && within(hulls, DERELICT_ONLY_HULLS)
                }
                GroupKind::LowRock => worlds == 0 && hulls == 0 && within(rocks, LOW_ROCK_ROCKS),
            };
            assert!(
                composed,
                "{slug} is {} with {worlds} worlds, {hulls} hulls and {rocks} rocks",
                group.kind.label()
            );

            let centre = group.home.centre(edge).get();
            let mut core = Meters::ZERO;
            for parent in &group.parents {
                let clearance = parent.body.clearance();
                core = core.max(parent.position.distance(group.anchor) + clearance);
                assert!(
                    (parent.position.get() - centre).abs().max_element() + clearance.get()
                        <= edge.get() * 0.5,
                    "{slug}: a world crosses a face of {}",
                    group.home
                );
            }
            let (inner, spread) = if worlds == 0 {
                (Meters::ZERO, OPEN_SPREAD)
            } else {
                (core + member_clearance_max() + MEMBER_MARGIN, WORLD_SPREAD)
            };
            for member in &group.members {
                let distance = member.position.distance(group.anchor).get();
                assert!(
                    distance >= inner.get() - 0.5 && distance <= (inner + spread.1).get() + 0.5,
                    "{slug}: a member {distance} m from its anchor"
                );
            }
        }
        assert_eq!(grown, GroupKind::ALL.into_iter().collect::<BTreeSet<_>>());
    }

    /// The chances are a distribution at every reading, and each field moves
    /// the mix the way the kinds say: dense quiet material grows worlds and
    /// rock fields, traffic in thin material grows wrecks, and thin quiet
    /// space grows scatters or nothing.
    #[test]
    fn the_fields_steer_the_kind_mix() {
        let steps = [0.0, 0.25, 0.5, 0.75, 1.0];
        for material_density in steps {
            for volatiles in steps {
                for human_activity in steps {
                    let environment = Environment {
                        material_density,
                        volatiles,
                        human_activity,
                    };
                    let chances = group_chances(environment);
                    let total = chances.kinds.iter().sum::<f32>() + chances.none;
                    assert!(
                        (total - 1.0).abs() < 1e-5,
                        "{environment:?} sums to {total}"
                    );
                    assert!(chances.none > 0.0, "{environment:?}");
                    assert!((0.0..=BACKGROUND_CHANCE_MAX).contains(&background_chance(environment)));
                }
            }
        }
        let at = |material_density, human_activity| {
            group_chances(Environment {
                material_density,
                volatiles: 0.5,
                human_activity,
            })
        };
        let chance = |chances: GroupChances, kind: GroupKind| {
            chances.kinds[GroupKind::ALL
                .iter()
                .position(|each| *each == kind)
                .unwrap()]
        };
        let (dense_quiet, thin_busy, thin_quiet) = (at(0.9, 0.1), at(0.1, 0.9), at(0.1, 0.1));
        assert!(
            chance(dense_quiet, GroupKind::PlanetHeavy) > chance(thin_busy, GroupKind::PlanetHeavy)
        );
        assert!(
            chance(dense_quiet, GroupKind::AsteroidRich)
                > chance(thin_quiet, GroupKind::AsteroidRich)
        );
        assert_eq!(chance(dense_quiet, GroupKind::DerelictOnly), 0.0);
        assert!(
            chance(thin_busy, GroupKind::DerelictOnly)
                > GroupKind::ALL
                    .into_iter()
                    .filter(|kind| *kind != GroupKind::DerelictOnly)
                    .map(|kind| chance(thin_busy, kind))
                    .fold(0.0, f32::max)
        );
        assert!(
            thin_quiet.none + chance(thin_quiet, GroupKind::LowRock) > 0.5,
            "{thin_quiet:?}"
        );
    }

    /// Every body centre stands within the derived reach of its node on each
    /// axis, and every body a group places inside a cell comes from a node in
    /// the cell's halo: a group the halo missed would be a body no cell plans.
    #[test]
    fn the_halo_holds_every_node_that_places_a_body_in_the_cell() {
        let edge = clustered_world_config().sector_edge;
        let reach = group_reach().get();
        let mut owned: BTreeMap<SectorCoord, BTreeSet<[i32; 3]>> = BTreeMap::new();
        for group in scanned_groups() {
            let node = Vec3::from_array(group.id.0.map(|index| index as f32)) * GROUP_LATTICE.get();
            for (source, body) in group.bodies() {
                let axis = (body.position.get() - node).abs().max_element();
                assert!(
                    axis <= reach,
                    "{source:?} stands {axis} m from its node on an axis, past {reach} m"
                );
                owned.entry(body.owner).or_default().insert(group.id.0);
            }
        }
        let inner = desired_sectors(SectorCoord::ORIGIN, 1);
        for coord in &inner {
            let halo: BTreeSet<[i32; 3]> = halo_nodes(*coord, edge).into_iter().collect();
            let missed: Vec<_> = owned
                .get(coord)
                .into_iter()
                .flatten()
                .filter(|node| !halo.contains(*node))
                .collect();
            assert!(missed.is_empty(), "{coord} halo misses {missed:?}");
        }
    }

    /// No body of one group comes within the derived floor of a body of
    /// another, placed or skipped: two groups never merge, and neither a
    /// parent collision nor a clearance skip is ever between groups.
    #[test]
    fn two_groups_never_come_closer_than_the_gap_floor() {
        // The anchors' separation on the axis their nodes differ on, less two
        // core pulls and two of the widest body extents from an anchor.
        let extent = (member_reach_max() + member_clearance_max()).max(core_reach_max());
        let floor = GROUP_LATTICE * (1.0 - GROUP_JITTER)
            - (core_reach_max() + ROUNDING_SLACK) * 2.0
            - extent * 2.0;
        assert!(floor > MEMBER_MARGIN, "the floor is {} m", floor.get());
        // Nodes two steps apart on an axis keep their anchors 43 km apart
        // there, far past the floor, so only neighbouring nodes are measured.
        let groups = scanned_groups();
        let mut pairs = 0;
        for (index, a) in groups.iter().enumerate() {
            for b in &groups[index + 1..] {
                if (0..3).any(|axis| (a.id.0[axis] - b.id.0[axis]).abs() > 1) {
                    continue;
                }
                pairs += 1;
                for (_, one) in a.bodies() {
                    for (_, other) in b.bodies() {
                        let gap = one.position.distance(other.position)
                            - one.body.clearance()
                            - other.body.clearance();
                        assert!(
                            gap >= floor,
                            "{} and {} come {} m apart",
                            a.id.slug(),
                            b.id.slug(),
                            gap.get()
                        );
                    }
                }
            }
        }
        assert!(pairs > 0, "the scan must hold neighbouring groups");
    }
}
