//! The clustered world: bodies that come in GROUPS, and groups that cross
//! sector faces.
//!
//! Example-owned, like [`super::UniformAsteroids`]. It implements
//! [`SectorGenerator`] from outside `nova_world`, so every manifest it returns
//! goes through the same `validate_manifest` check as the base game's.
//!
//! # Groups are global, cells only own bodies
//!
//! A group is decided at a node of a global 48 km lattice, from a stream keyed
//! by the world seed and the node alone. Its parent - a planetoid or a
//! derelict hull - and every member around it have one world position, and
//! the cell a body's centre falls in OWNS that body. A cell asks every node
//! whose group could put a body inside it (the halo, [`halo_nodes`]), replays
//! each group the same way any other cell would, and keeps the bodies it owns.
//! So a group whose members stand on both sides of a face is one group, and a
//! neighbour generated first, last or never cannot change it.
//!
//! # Every planned body is accounted for
//!
//! A cell resolves the bodies it owns in one fixed order - parents, then group
//! members by group and index, then the background rock - and each one is
//! either placed or skipped for a named [`SkipReason`]: its clearance crosses
//! a face, it overlaps a body placed before it, or a workload cap is full.
//! Nothing is dropped without a reason in the [`CellPlan`]. A PARENT is never
//! skipped: the lattice spacing and the parent inset make a skipped parent
//! impossible, so one is a generator bug and a loud [`SectorFault`].
//!
//! # The environment decides
//!
//! Chances and kind mixes read the three [`super::environment`] fields at the
//! group's anchor or the cell's centre, and read them together: a planetoid
//! wants dense material in quiet space, a derelict group wants traffic and
//! grows debris where the traffic worked material, and volatiles turn the
//! rocks and worlds to ice and carbon.

use bevy::prelude::Vec3;
use nova_protocol::prelude::*;
use nova_world::prelude::*;

use super::environment::{Environment, EnvironmentFields};

/// The spacing of the group lattice: at most one group per node.
///
/// One and a half sector edges at 32 km. Not a whole number of edges: a node
/// on every second cell centre put every parent in an even cell. At one and a
/// half, every other node sits on a cell face, so parents fall near cell
/// centres and near faces alike, and a group near a face is what crosses it.
pub const GROUP_LATTICE: Meters = Meters(48_000.0);

/// How far a group's anchor may move off its node, as a fraction of the half
/// spacing. Jitter is what keeps groups off a visible grid.
///
/// Bounded, because two proofs rest on it: two anchors stay at least
/// `GROUP_LATTICE * (1 - GROUP_JITTER)` apart on the axis their nodes differ
/// on, which is what keeps two parents out of one cell, and the halo is sized
/// from it. 0.33 keeps that separation at 32.16 km, just over the 32 km edge.
const GROUP_JITTER: f32 = 0.33;

/// How much of its owner cell's half edge a parent's centre is pulled into.
///
/// A parent is the one body of a group that must be placed, so its whole
/// clearance stays inside its cell; its members are placed where they fall.
/// 0.9 leaves 1.6 km at a 32 km edge, over the widest world's 1,272 m, and
/// lets a parent stand close enough to a face for its group to cross it.
const PARENT_INSET: f32 = 0.9;

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

/// How many rocks a planetoid group places around its world.
const PLANETOID_COMPANIONS: (usize, usize) = (2, 6);

/// How far from its world's centre a planetoid companion stands.
///
/// The inner edge clears the widest world (1,272 m), the widest rock (360 m)
/// and the margin, so a companion never overlaps its own parent.
const PLANETOID_RING: (Meters, Meters) = (Meters(2_000.0), Meters(8_000.0));

/// How many bodies a derelict group places around its lead hull.
const DERELICT_COMPANIONS: (usize, usize) = (1, 4);

/// How far from the lead hull a derelict companion stands.
///
/// The inner edge clears two ship clearances and the margin.
const DERELICT_RING: (Meters, Meters) = (Meters(1_200.0), Meters(6_000.0));

/// The highest chance a node grows a planetoid group.
const PLANETOID_CHANCE_MAX: f32 = 0.45;

/// The highest chance a node grows a derelict group. With
/// [`PLANETOID_CHANCE_MAX`] it sums under one, so a draw can always say "no
/// group".
const DERELICT_CHANCE_MAX: f32 = 0.5;

/// The highest chance a cell draws its one background rock.
const BACKGROUND_CHANCE_MAX: f32 = 0.6;

/// The highest chance a derelict companion is a debris rock instead of a hull.
const DEBRIS_CHANCE_MAX: f32 = 0.6;

/// Extra reach the halo is given past its derived bound, for the rounding of
/// an `f32` position at a few hundred kilometres.
const HALO_SLACK: Meters = Meters(1.0);

/// Every natural asteroid kind. `plain` is the rendering control.
const ROCK_KINDS: [&str; 4] = [KIND_ROCK, KIND_METAL, KIND_ICE, KIND_CARBON];

/// The shipped hulls a derelict group is drawn from: the damaged frame tender
/// and loose wreck plating.
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
    /// Refuse an edge wide enough to hold two parents, and an edge too narrow
    /// to hold the widest parent inside [`PARENT_INSET`].
    fn validate(&self, geometry: WorldGeometry) -> Result<(), SectorFault> {
        let edge = geometry.sector_edge;
        let one_parent = GROUP_LATTICE.get() * (1.0 - GROUP_JITTER);
        // A `NaN` edge compares as `None`, so it refuses too.
        if edge.get().partial_cmp(&one_parent) != Some(std::cmp::Ordering::Less) {
            return Err(SectorFault::Config {
                field: "sector_edge",
                value: format!(
                    "{} m, not under the {one_parent} m two group anchors stay apart, so one \
                     cell could hold two parents",
                    edge.get()
                ),
            });
        }
        let (clearance, body) = widest_parent();
        geometry.require_owning_edge(PARENT_INSET, clearance, &body)
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

/// What a group is built around.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupKind {
    /// A planetoid with companion rocks.
    Planetoid,
    /// A lead derelict hull with companion hulls and debris rocks.
    Derelict,
}

impl GroupKind {
    /// What a readout calls the kind.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Planetoid => "planetoid",
            Self::Derelict => "derelict",
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
    /// What it is built around.
    pub kind: GroupKind,
    /// The fields at its anchor, which every chance and kind in it read.
    pub environment: Environment,
    /// The planetoid or lead hull.
    pub parent: GroupBody,
    /// The bodies around the parent, in draw order.
    pub members: Vec<GroupBody>,
}

impl ClusterGroup {
    /// Whether its bodies are owned by more than one cell.
    pub fn spans_seam(&self) -> bool {
        self.members
            .iter()
            .any(|member| member.owner != self.parent.owner)
    }
}

/// Where a planned body came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BodySource {
    /// A group's parent.
    Parent(GroupId),
    /// A group's member, by its index in [`ClusterGroup::members`].
    Member(GroupId, usize),
    /// The cell's own background rock.
    Background,
}

/// Why a planned body was not placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkipReason {
    /// Its clearance sphere crosses a face of the cell its centre is in.
    Face,
    /// It overlaps, or comes within [`MEMBER_MARGIN`] of, a body placed
    /// before it.
    Clearance,
    /// The cell already holds [`SECTOR_ASTEROIDS_MAX`] rocks.
    AsteroidCap,
    /// The cell already holds [`SECTOR_BODIES_MAX`] bodies.
    BodyCap,
}

impl SkipReason {
    /// Every reason, in the order a readout lists them.
    pub const ALL: [Self; 4] = [
        Self::Face,
        Self::Clearance,
        Self::AsteroidCap,
        Self::BodyCap,
    ];

    /// What a readout calls the reason.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Face => "face",
            Self::Clearance => "clearance",
            Self::AsteroidCap => "rock cap",
            Self::BodyCap => "body cap",
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

/// The chances a node grows each kind of group.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParentChances {
    /// Chance of a planetoid group.
    pub planetoid: f32,
    /// Chance of a derelict group.
    pub derelict: f32,
}

/// The chances the fields at an anchor give each kind of group.
///
/// A planetoid wants dense material in QUIET space: traffic takes a third to
/// three fifths of the chance away. A derelict group wants traffic, and twice
/// as much of it where the traffic worked dense material.
pub fn parent_chances(environment: Environment) -> ParentChances {
    let Environment {
        material_density,
        human_activity,
        ..
    } = environment;
    ParentChances {
        planetoid: PLANETOID_CHANCE_MAX
            * ramp(material_density, 0.45, 0.85)
            * (1.0 - 0.6 * human_activity),
        derelict: DERELICT_CHANCE_MAX
            * ramp(human_activity, 0.4, 0.8)
            * (0.4 + 0.6 * material_density),
    }
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
/// Every weight is positive by construction, so the total is too.
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

/// How many members a group of `band` places when `richness` reads in `[0, 1]`:
/// a richer anchor widens the draw toward the top of the band.
fn member_count(band: (usize, usize), richness: f32, draw: f32) -> usize {
    let (min, max) = band;
    let span = 1.0 + (max - min) as f32 * richness;
    (min + (draw * span) as usize).min(max)
}

fn rock(environment: Environment, stream: &mut SeedStream) -> ClusterBody {
    let (radius_min, radius_max) = ROCK_RADIUS;
    let radius = radius_min + (radius_max - radius_min) * stream.unit();
    ClusterBody::Rock {
        radius,
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
/// PURE, and keyed by the seed and the node alone - never by the cell asking -
/// so every cell that replays a node gets the same group. The draws come off
/// the node's stream in one fixed order.
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
    let anchor = Meters3::new(
        x + stream.signed() * jitter,
        y + stream.signed() * jitter,
        z + stream.signed() * jitter,
    );
    let environment = fields.sample(anchor)?;
    let chances = parent_chances(environment);
    let draw = stream.unit();
    let kind = if draw < chances.planetoid {
        GroupKind::Planetoid
    } else if draw < chances.planetoid + chances.derelict {
        GroupKind::Derelict
    } else {
        return Ok(None);
    };

    // The parent is pulled into its owner cell's inset, which is where the
    // one-parent-per-cell and the inside-its-cell proofs hold.
    let owner = SectorCoord::containing(anchor, edge);
    let owner_centre = owner.centre(edge).get();
    let inset = Vec3::splat(edge.get() * 0.5 * PARENT_INSET);
    let parent_position =
        Meters3(owner_centre + (anchor.get() - owner_centre).clamp(-inset, inset));
    let invalid = || SectorFault::InvalidGeometry { id: id.slug() };
    if !parent_position.get().is_finite() {
        return Err(invalid());
    }

    let (parent_body, ring, count) = match kind {
        GroupKind::Planetoid => {
            let (radius_min, radius_max) = PLANETOID_RADIUS;
            let radius = radius_min + (radius_max - radius_min) * stream.unit();
            let planet_type = planet_type(environment, stream.unit());
            let config = PlanetConfig::new(planet_type, radius, stream.next_u32());
            let count = member_count(
                PLANETOID_COMPANIONS,
                environment.material_density,
                stream.unit(),
            );
            (ClusterBody::Planetoid(config), PLANETOID_RING, count)
        }
        GroupKind::Derelict => {
            let lead = hull(environment, &mut stream);
            let count = member_count(
                DERELICT_COMPANIONS,
                environment.human_activity,
                stream.unit(),
            );
            (lead, DERELICT_RING, count)
        }
    };

    let mut members = Vec::with_capacity(count);
    for _ in 0..count {
        let direction = unit_sphere_point(stream.next_u32());
        let distance = ring.0 + (ring.1 - ring.0) * stream.unit();
        let position = Meters3(parent_position.get() + direction * distance.get());
        if !position.get().is_finite() {
            return Err(invalid());
        }
        let body = match kind {
            GroupKind::Planetoid => rock(environment, &mut stream),
            GroupKind::Derelict => {
                if stream.unit() < DEBRIS_CHANCE_MAX * environment.material_density {
                    rock(environment, &mut stream)
                } else {
                    hull(environment, &mut stream)
                }
            }
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
        parent: GroupBody {
            body: parent_body,
            position: parent_position,
            owner,
        },
        members,
    }))
}

/// How far on one axis a group's anchor node can be from a body centre the
/// group places, at this edge.
///
/// A parent leaves its node by one jitter draw and one inset pull, which moves
/// it at most `1 - PARENT_INSET` of a half edge. A member stands within its
/// ring's outer edge of the parent. The distance along one axis is at most the
/// distance itself, so this bounds every axis.
fn group_reach(edge: Meters) -> Meters {
    let drift = GROUP_JITTER * GROUP_LATTICE.get() * 0.5 + (1.0 - PARENT_INSET) * edge.get() * 0.5;
    Meters(drift) + PLANETOID_RING.1.max(DERELICT_RING.1)
}

/// Every lattice node whose group could place a body centre in `coord`, in
/// node order.
///
/// A body centre in the cell is within half an edge of its centre on each
/// axis, and within [`group_reach`] of its node, so a node further than both
/// together on any axis cannot reach the cell. At a 32 km edge the reach is
/// 33.6 km each way, under two lattice spacings across, so the halo is at
/// most two nodes an axis and eight in all.
fn halo_nodes(coord: SectorCoord, edge: Meters) -> Vec<[i32; 3]> {
    let reach = edge.get() * 0.5 + group_reach(edge).get() + HALO_SLACK.get();
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
        let mut owns = false;
        if group.parent.owner == coord {
            parents.push(Candidate {
                id: sector_id(coord, &format!("{stem}_parent"), 0),
                source: BodySource::Parent(group.id),
                body: group.parent.body.clone(),
                position: group.parent.position,
            });
            owns = true;
        }
        for (index, member) in group.members.iter().enumerate() {
            if member.owner == coord {
                members.push(Candidate {
                    id: sector_id(coord, &format!("{stem}_member"), index),
                    source: BodySource::Member(group.id, index),
                    body: member.body.clone(),
                    position: member.position,
                });
                owns = true;
            }
        }
        if owns {
            groups.push(group);
        }
    }

    let centre = coord.centre(edge);
    let environment = fields.sample(centre)?;
    let mut background = Vec::new();
    let mut stream = input.stream("background");
    if stream.unit() < background_chance(environment) {
        let reach = edge.get() * 0.5 * PARENT_INSET;
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
/// the cell, no overlap, the two caps - so a placed body is one the check
/// accepts, and each refusal becomes a counted skip instead of a refused
/// manifest.
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
    let mut rocks = 0;
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
        let is_rock = matches!(body, ClusterBody::Rock { .. });
        let inside = SectorCoord::containing(position, edge) == input.coord
            && (position.get() - centre).abs().max_element() + clearance.get() <= edge.get() * 0.5;
        let outcome = if !inside {
            Outcome::Skipped(SkipReason::Face)
        } else if !standing.iter().all(|&(other, other_clearance)| {
            bodies_clear(other, other_clearance, position, clearance, MEMBER_MARGIN)
        }) {
            Outcome::Skipped(SkipReason::Clearance)
        } else if is_rock && rocks == SECTOR_ASTEROIDS_MAX {
            Outcome::Skipped(SkipReason::AsteroidCap)
        } else if standing.len() == SECTOR_BODIES_MAX {
            Outcome::Skipped(SkipReason::BodyCap)
        } else {
            Outcome::Placed
        };
        if let (BodySource::Parent(_), Outcome::Skipped(reason)) = (source, outcome) {
            return Err(SectorFault::Generation {
                id,
                field: "parent",
                value: format!("skipped for {}", reason.label()),
            });
        }
        if outcome == Outcome::Placed {
            standing.push((position, clearance));
            if is_rock {
                rocks += 1;
            }
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

/// The widest parent clearance this generator can draw, and what draws it.
///
/// A planetoid's is the OUTER radius of the widest type at the top of the
/// band, which is what `validate_manifest` measures it by.
fn widest_parent() -> (Meters, String) {
    let planet = PlanetType::ALL
        .iter()
        .map(|planet_type| PlanetConfig::new(*planet_type, PLANETOID_RADIUS.1, 0).body_radius())
        .fold(Meters(0.0), |widest, radius| widest.max(radius));
    if planet >= SECTOR_SHIP_CLEARANCE {
        (
            planet,
            format!("a group planetoid reaching {} m", planet.get()),
        )
    } else {
        (
            SECTOR_SHIP_CLEARANCE,
            format!("a lead derelict at {} m", SECTOR_SHIP_CLEARANCE.get()),
        )
    }
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

    fn group_of(source: BodySource) -> Option<GroupId> {
        match source {
            BodySource::Parent(group) | BodySource::Member(group, _) => Some(group),
            BodySource::Background => None,
        }
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

    /// The home window holds a planetoid group and a derelict group that each
    /// PLACE bodies in two cells, and the owning cell's streamed description
    /// holds each of those bodies where the group put it.
    #[test]
    fn groups_in_the_home_window_place_bodies_on_both_sides_of_a_face() {
        let config = clustered_world_config();
        let plans = window_plans();
        let mut cells: BTreeMap<GroupId, BTreeSet<SectorCoord>> = BTreeMap::new();
        let mut kinds = BTreeMap::new();
        for plan in plans.values() {
            for group in &plan.groups {
                kinds.insert(group.id, group.kind);
            }
            for body in &plan.bodies {
                if let (Some(group), Outcome::Placed) = (group_of(body.source), body.outcome) {
                    cells.entry(group).or_default().insert(plan.coord);
                }
            }
        }
        for kind in [GroupKind::Planetoid, GroupKind::Derelict] {
            let spanning: Vec<GroupId> = cells
                .iter()
                .filter(|(group, owners)| kinds[*group] == kind && owners.len() >= 2)
                .map(|(group, _)| *group)
                .collect();
            assert!(
                !spanning.is_empty(),
                "the home window must hold a {} group placed in two cells",
                kind.label()
            );
            for group in spanning {
                for coord in &cells[&group] {
                    let description = generate_sector(&config, *coord)
                        .unwrap_or_else(|fault| panic!("{coord}: {fault}"));
                    let canonical = description.canonical();
                    for body in plans[coord].bodies.iter().filter(|body| {
                        group_of(body.source) == Some(group) && body.outcome == Outcome::Placed
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
    /// count is what it streams.
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
                .filter(|body| group_of(body.source).is_some())
            {
                owners.entry(body.source).or_default().push(plan.coord);
            }
            for group in &plan.groups {
                groups.insert(group.id, group.clone());
            }
        }
        // A group whose parent stands one cell inside the window has every
        // member inside it: the widest ring is 8 km and a cell is 32 km.
        let inner = desired_sectors(CLUSTER_HOME, EXAMPLE_ACTIVE_RADIUS - 1);
        let mut checked = 0;
        for group in groups.values() {
            if !inner.contains(&group.parent.owner) {
                continue;
            }
            let bodies = std::iter::once((BodySource::Parent(group.id), &group.parent)).chain(
                group
                    .members
                    .iter()
                    .enumerate()
                    .map(|(index, member)| (BodySource::Member(group.id, index), member)),
            );
            for (source, body) in bodies {
                assert_eq!(
                    owners.get(&source),
                    Some(&vec![body.owner]),
                    "{source:?} of {} must be planned once, by its owner",
                    group.id.slug()
                );
            }
            checked += 1;
        }
        assert!(checked > 0, "the inner window must own a group's parent");
        assert!(
            owners.values().all(|cells| cells.len() == 1),
            "no group body may be planned by two cells"
        );
    }

    /// A cell full of rocks skips the next rock by the rock cap, a cell full of
    /// bodies skips the next hull by the body cap, both counted - and a parent
    /// that would be skipped is a fault, never a skip.
    #[test]
    fn a_full_cell_skips_by_cap_and_counts_it_but_never_skips_a_parent() {
        let input = clustered_world_config().input(SectorCoord::ORIGIN);
        let group = GroupId([0, 0, 0]);
        let rock = |index: usize| Candidate {
            id: format!("rock_{index}"),
            source: BodySource::Member(group, index),
            body: ClusterBody::Rock {
                radius: Meters(30.0),
                kind: KIND_ROCK,
            },
            position: Meters3::new(-12_000.0 + 2_000.0 * index as f32, 12_000.0, 0.0),
        };
        let hull = |index: usize| Candidate {
            id: format!("hull_{index}"),
            source: BodySource::Member(group, 100 + index),
            body: ClusterBody::Hull {
                design: BLOCK_WRECK_PLATE_SHIP_ID,
                yaw: 0.0,
            },
            position: Meters3::new(
                -12_000.0 + 2_000.0 * (index % 8) as f32,
                -12_000.0 + 4_000.0 * (index / 8) as f32,
                0.0,
            ),
        };
        let planned = resolve(input, (0..5).map(rock).chain((0..13).map(hull)))
            .expect("members are skipped, never refused");
        let skipped = |reason| {
            planned
                .iter()
                .filter(|body| body.outcome == Outcome::Skipped(reason))
                .map(|body| body.id.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(skipped(SkipReason::AsteroidCap), ["rock_4"]);
        assert_eq!(skipped(SkipReason::BodyCap), ["hull_12"]);
        assert_eq!(
            planned
                .iter()
                .filter(|body| body.outcome == Outcome::Placed)
                .count(),
            SECTOR_BODIES_MAX
        );

        let parent = Candidate {
            id: "parent".to_string(),
            source: BodySource::Parent(group),
            body: ClusterBody::Hull {
                design: BLOCK_WRECK_PLATE_SHIP_ID,
                yaw: 0.0,
            },
            position: Meters3::new(0.0, 0.0, 12_000.0),
        };
        let fault = resolve(input, (0..16).map(hull).chain([parent]))
            .expect_err("a parent over a full cell must refuse");
        assert!(
            matches!(&fault, SectorFault::Generation { id, field: "parent", .. } if id == "parent"),
            "got {fault:?}"
        );
    }

    #[test]
    fn an_edge_that_could_hold_two_parents_or_not_hold_one_is_refused() {
        let refuses = |edge: f32, names: &str| {
            let fault = ClusteredWorld
                .validate(WorldGeometry {
                    sector_edge: Meters(edge),
                })
                .expect_err("the edge must refuse");
            assert!(
                matches!(&fault, SectorFault::Config { field: "sector_edge", value } if value.contains(names)),
                "{edge} m must refuse naming {names}, got {fault:?}"
            );
        };
        refuses(32_200.0, "two parents");
        refuses(f32::NAN, "two parents");
        refuses(25_000.0, "planetoid");
        for edge in [25_500.0, 32_000.0] {
            ClusteredWorld
                .validate(WorldGeometry {
                    sector_edge: Meters(edge),
                })
                .unwrap_or_else(|fault| panic!("{edge} m must arm, got {fault}"));
        }
    }

    /// Chances stay under their maxima and sum under one, members stand in
    /// their ring, and the halo stays at eight nodes at every edge the
    /// generator arms.
    #[test]
    fn chances_rings_and_halo_stay_inside_their_bounds() {
        let steps = [0.0, 0.25, 0.5, 0.75, 1.0];
        for material_density in steps {
            for volatiles in steps {
                for human_activity in steps {
                    let environment = Environment {
                        material_density,
                        volatiles,
                        human_activity,
                    };
                    let chances = parent_chances(environment);
                    assert!((0.0..=PLANETOID_CHANCE_MAX).contains(&chances.planetoid));
                    assert!((0.0..=DERELICT_CHANCE_MAX).contains(&chances.derelict));
                    assert!(
                        chances.planetoid + chances.derelict < 1.0,
                        "{environment:?}"
                    );
                    assert!((0.0..=BACKGROUND_CHANCE_MAX).contains(&background_chance(environment)));
                }
            }
        }

        for plan in window_plans().values() {
            for group in &plan.groups {
                let ring = match group.kind {
                    GroupKind::Planetoid => PLANETOID_RING,
                    GroupKind::Derelict => DERELICT_RING,
                };
                for member in &group.members {
                    let distance = member.position.distance(group.parent.position);
                    assert!(
                        distance.get() >= ring.0.get() - 0.1
                            && distance.get() <= ring.1.get() + 0.1,
                        "{} member at {} m",
                        group.id.slug(),
                        distance.get()
                    );
                }
            }
        }

        for edge in [25_500.0, 32_000.0, 32_150.0] {
            for coord in desired_sectors(SectorCoord::new(-3, 5, 11), 2) {
                let nodes = halo_nodes(coord, Meters(edge)).len();
                assert!(nodes <= 8, "{coord} at {edge} m has a halo of {nodes}");
            }
        }
    }
}
