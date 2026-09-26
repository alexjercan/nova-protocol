//! The base game's cluster policy: where the open world's places are and what
//! each one holds.
//!
//! PURE, like every generator: nothing here touches a `World`, reads a
//! resource or draws from the ambient RNG.
//!
//! # Clusters are global, cells only own bodies
//!
//! A cluster is decided at a node of a global 50 km lattice, from a stream
//! keyed by the world seed and the node alone. It is one of four
//! [`ClusterType`]s, or nothing, and every body in it has one world position.
//! The cell a body's centre falls in OWNS that body. A cell asks every node
//! whose cluster could put a body inside it (the halo, [`halo_nodes`]),
//! replays each cluster the same way any other cell would, and keeps the
//! bodies it owns. So a cluster whose bodies stand on both sides of a face is
//! one cluster, and a neighbour generated first, last or never cannot change
//! it.
//!
//! # Every planned body is accounted for
//!
//! A cell resolves the bodies it owns in one fixed order - every cluster's
//! parents, then every cluster's rocks by node and index, then every
//! cluster's hulls by node and index, then its background scatter - and each
//! one is placed or skipped for a named reason: its clearance sphere crosses a
//! face of its cell, it comes within [`CLEARANCE_MARGIN`] of a body placed
//! before it, or it is a hull with no companion. [`SectorClusters`] counts
//! all three. A PARENT - a cluster's planetoid - is never skipped: the core
//! pull keeps it inside its cell and the lattice keeps it clear of every other
//! parent, so a parent that does not fit is a generator bug and a loud
//! [`SectorFault::Generation`].
//!
//! # A hull never floats alone
//!
//! A cell places a hull only beside a rock or planetoid of the hull's OWN
//! cluster placed in the SAME cell, so every streamed wreck is part of a
//! place and never an orphan across a face from its field. Hulls resolve after
//! every rock for that reason. A hull whose cluster placed nothing else in the
//! cell tries its escorts - rocks its cluster planned around that hull, on the
//! side that faces the anchor - in order, and places the first that fits in
//! the cell beside it. Each escort it tried and could not fit is skipped and
//! counted, and a hull none of whose escorts fits is skipped as
//! [`SkipType::Companion`]. Escorts are drawn at the node like every other
//! body, so they count toward the cluster's extent, the halo and the gap
//! between clusters, and a cell never looks past its own faces for one.
//!
//! # The environment decides
//!
//! The type, its body counts and the rock, world and hull mixes read the
//! three [`crate::EnvironmentFields`] at the cluster's drawn anchor, TOGETHER
//! and continuously: dense material grows asteroid-rich, rock-only and
//! planet-heavy clusters, traffic takes the worlds and grows wreck fields,
//! volatiles favour worlds and turn rocks and worlds to ice and carbon, and
//! thin quiet space grows nothing. A cell that owns no cluster body may draw a
//! small background scatter instead, so open space is not always empty.

use bevy::prelude::{Quat, Vec3};
use nova_events::prelude::{Meters, Meters3, MetersPerSecondSquared};
use nova_gameplay::prelude::{unit_sphere_point, Fnv32, GravitySettings, SeedStream};
use nova_scenario::prelude::{
    asteroid_seed_from_id, PlanetConfig, PlanetType, ASTEROID_GEOMETRIC_FACTOR_MAX, KIND_CARBON,
    KIND_ICE, KIND_METAL, KIND_ROCK,
};
use nova_world::prelude::*;

use crate::{
    environment::{Environment, EnvironmentFields},
    BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID, BLOCK_WRECK_PLATE_SHIP_ID, CLEARANCE_MARGIN,
};

/// The spacing of the cluster lattice: at most one cluster per node.
///
/// About one interesting place per 50 km of travel on each axis, so a 160 km
/// window holds a few clusters and long runs of open space between them.
const CLUSTER_LATTICE: Meters = Meters(50_000.0);

/// How far a cluster's anchor may move off its node, as a fraction of the half
/// spacing: +/-10 km on each axis. Jitter is what keeps clusters off a
/// visible grid.
///
/// Bounded, because the gap between clusters rests on it: two anchors stay at
/// least `CLUSTER_LATTICE * (1 - CLUSTER_JITTER)`, 30 km, apart on the axis
/// their nodes differ on.
const CLUSTER_JITTER: f32 = 0.4;

/// The farthest any body's clearance sphere may reach from its cluster's
/// anchor.
///
/// A cluster is a PLACE a pilot flies into, not a region: every body of one
/// stands within this of its anchor. `validate` refuses the generator when its
/// own bands derive a wider cluster, so retuning a band cannot quietly grow
/// clusters into each other.
const CLUSTER_EXTENT_MAX: Meters = Meters(8_000.0);

/// The nominal radius band a rock is drawn from. The meshed rock reaches up to
/// [`ASTEROID_GEOMETRIC_FACTOR_MAX`] times past it, which is the clearance it
/// is spaced by.
const ROCK_RADIUS: (Meters, Meters) = (Meters(25.0), Meters(120.0));

/// The power a rock's radius draw is raised to before it crosses
/// [`ROCK_RADIUS`]: above one, most rocks stay small and a few are large.
const ROCK_RADIUS_SKEW: i32 = 3;

/// The mean radius band a planetoid is drawn from.
///
/// 500-1,600 m: big enough to read as a WORLD against a 120 m rock beside it,
/// small enough that a 32 km cell is still a place you fly across.
const PLANETOID_RADIUS: (Meters, Meters) = (Meters(500.0), Meters(1_600.0));

/// How far a planetoid's sphere of influence reaches, in multiples of its
/// outer body radius, under the default [`GravitySettings`].
///
/// The widest world reaches about 5.9 km, and the wells of a cluster's worlds
/// overlap: [`WORLD_SPACING`] keeps their BODIES apart, not their wells. The
/// surface pull this asks for is the same for every size, and `validate`
/// refuses it past the gravity cap rather than let the cap shrink every well.
const PLANETOID_SOI_REACH: f32 = 3.5;

/// How far apart the centres of two neighbouring worlds of one cluster stand.
///
/// Two to four worlds stand at the corners of a pair, a triangle or a
/// tetrahedron around the anchor, every edge one drawn spacing long. The inner
/// edge has to clear two of the widest worlds and [`CLEARANCE_MARGIN`];
/// `validate` refuses it otherwise.
const WORLD_SPACING: (Meters, Meters) = (Meters(3_900.0), Meters(4_300.0));

/// How many worlds a planet-heavy cluster places, more with more volatiles.
const PLANET_HEAVY_WORLDS: (usize, usize) = (2, 4);

/// Where each of a cluster's worlds stands around its anchor, by count, for a
/// spacing of one: one world on the anchor, then a pair, an equilateral
/// triangle and a regular tetrahedron, every edge one long. A cluster turns
/// its layout by a drawn rotation and scales it by a drawn [`WORLD_SPACING`].
const WORLD_CORNERS: [&[Vec3]; 5] = {
    // Half an edge, the circumradius of a unit triangle (1 / sqrt 3), and
    // the corner coordinate of a unit tetrahedron (1 / (2 sqrt 2)).
    const H: f32 = 0.5;
    const T: f32 = 0.577_350_3;
    const Q: f32 = 0.353_553_4;
    [
        &[],
        &[Vec3::ZERO],
        &[Vec3::new(H, 0.0, 0.0), Vec3::new(-H, 0.0, 0.0)],
        &[
            Vec3::new(T, 0.0, 0.0),
            Vec3::new(-T * 0.5, H, 0.0),
            Vec3::new(-T * 0.5, -H, 0.0),
        ],
        &[
            Vec3::new(Q, Q, Q),
            Vec3::new(Q, -Q, -Q),
            Vec3::new(-Q, Q, -Q),
            Vec3::new(-Q, -Q, Q),
        ],
    ]
};
const _: () = assert!(
    PLANET_HEAVY_WORLDS.1 < WORLD_CORNERS.len(),
    "a planet-heavy cluster may draw more worlds than there are layouts for"
);

/// How deep the shell of members around a cluster's worlds is, past the core
/// they clear.
///
/// Narrower than [`OPEN_SPREAD`]: the core already makes a world cluster the
/// widest, and [`CLUSTER_EXTENT_MAX`] is paid out of core and shell together.
const WORLD_SPREAD: (Meters, Meters) = (Meters(1_000.0), Meters(1_700.0));

/// How far from its anchor a member of a cluster with no worlds may stand.
const OPEN_SPREAD: (Meters, Meters) = (Meters(2_000.0), Meters(4_000.0));

/// How many rocks each type places, from the bottom of the band in thin
/// material to the top in dense material.
const ASTEROID_RICH_ROCKS: (usize, usize) = (12, 24);
const ROCK_ONLY_ROCKS: (usize, usize) = (10, 20);
const PLANET_HEAVY_ROCKS: (usize, usize) = (4, 10);
const DERELICT_FIELD_ROCKS: (usize, usize) = (6, 12);

/// How many hulls a derelict field places, more with more traffic.
///
/// Hulls are the costliest body to materialize, so no type places many.
const DERELICT_FIELD_HULLS: (usize, usize) = (2, 3);

/// How many escort rocks a cluster plans around each of its hulls. A cell
/// places at most one per hull, and only for a hull its cluster left alone
/// there.
const HULL_ESCORTS: usize = 3;

/// How far from its hull an escort rock stands, always on the side of the hull
/// that faces the anchor, so an escort adds less to a cluster's reach than its
/// distance.
///
/// The inner edge has to clear a hull, the widest rock and
/// [`CLEARANCE_MARGIN`]; `validate` refuses it otherwise, so a cell never
/// checks an escort against its own hull.
const ESCORT_DISTANCE: (Meters, Meters) = (Meters(1_650.0), Meters(2_200.0));

/// The highest chance an asteroid-rich cluster places its one hull, at full
/// traffic.
const ASTEROID_RICH_HULL_CHANCE_MAX: f32 = 0.4;

/// The highest chance an asteroid-rich cluster places one or two worlds, and
/// the highest chance it places two, both at full quiet material.
const ASTEROID_RICH_WORLD_CHANCE_MAX: f32 = 0.55;
const ASTEROID_RICH_TWIN_CHANCE_MAX: f32 = 0.2;

/// The highest chance a cell that owns no cluster body draws a background
/// scatter.
const BACKGROUND_CHANCE_MAX: f32 = 0.35;

/// How many rocks a background scatter places, drawn evenly across the band:
/// the material already decided whether the scatter exists.
const BACKGROUND_ROCKS: (usize, usize) = (1, 5);

/// How far from its centre a background scatter's rocks may stand.
const BACKGROUND_SPREAD: (Meters, Meters) = (Meters(1_000.0), Meters(3_000.0));

/// Extra reach given past a derived bound, for the rounding of an `f32`
/// position at a few hundred kilometres.
const ROUNDING_SLACK: Meters = Meters(1.0);

/// Every natural asteroid kind. `plain` is the rendering control, not a rock a
/// world would contain.
const ROCK_KINDS: [&str; 4] = [KIND_ROCK, KIND_METAL, KIND_ICE, KIND_CARBON];

/// The shipped hulls a cluster's derelicts are drawn from: the damaged frame
/// tender and loose wreck plating, the two base hulls that are already
/// wrecks.
const DERELICT_DESIGNS: [&str; 2] = [
    BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID,
    BLOCK_WRECK_PLATE_SHIP_ID,
];

/// What a cluster is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClusterType {
    /// Many rocks, and in quiet material one or two planetoids, and with
    /// traffic one derelict hull.
    AsteroidRich,
    /// Rocks and nothing else.
    RockOnly,
    /// Two to four planetoids with a few rocks around them.
    PlanetHeavy,
    /// Derelict hulls in a thin rock field.
    DerelictField,
}

impl ClusterType {
    /// The fewest and most worlds a cluster of this type draws.
    const fn world_band(self) -> (usize, usize) {
        match self {
            Self::AsteroidRich => (0, 2),
            Self::RockOnly | Self::DerelictField => (0, 0),
            Self::PlanetHeavy => PLANET_HEAVY_WORLDS,
        }
    }

    /// The fewest and most hulls a cluster of this type draws.
    const fn hull_band(self) -> (usize, usize) {
        match self {
            Self::AsteroidRich => (0, 1),
            Self::RockOnly | Self::PlanetHeavy => (0, 0),
            Self::DerelictField => DERELICT_FIELD_HULLS,
        }
    }

    /// Every type, in the order a readout and a legend list them.
    pub const ALL: [Self; 4] = [
        Self::AsteroidRich,
        Self::RockOnly,
        Self::PlanetHeavy,
        Self::DerelictField,
    ];

    /// What a readout calls the type.
    pub const fn label(self) -> &'static str {
        match self {
            Self::AsteroidRich => "asteroid-rich",
            Self::RockOnly => "rock-only",
            Self::PlanetHeavy => "planet-heavy",
            Self::DerelictField => "derelict-field",
        }
    }
}

/// One cluster as one sector sees it: the same identity and geometry from
/// every sector, and this sector's share of its bodies.
#[derive(Clone, Debug, PartialEq)]
pub struct ClusterSummary {
    /// The cluster's id, `cluster_<x>_<y>_<z>` from its lattice node, with a
    /// negative index written `n<abs>`. Also the stem of every body id it
    /// names.
    pub id: String,
    /// What it is.
    pub cluster_type: ClusterType,
    /// Its centre: every body stands around it.
    pub anchor: Meters3,
    /// The cell the anchor is in, which owns every planetoid of the cluster.
    pub home: SectorCoord,
    /// How far the farthest clearance sphere of the whole cluster reaches from
    /// the anchor, in every cell. At most 8 km.
    pub extent: Meters,
    /// How many of its bodies this sector placed.
    pub placed: usize,
    /// How many of its bodies this sector owns and skipped because the
    /// clearance sphere crosses a face.
    pub skipped_face: usize,
    /// How many of its bodies this sector owns and skipped because they come
    /// within [`CLEARANCE_MARGIN`] of a body placed before them.
    pub skipped_clearance: usize,
    /// How many of its hulls this sector owns and skipped because neither the
    /// cluster nor any of the hull's escorts placed a rock or planetoid in
    /// this sector.
    pub skipped_companion: usize,
    /// How many escort rocks this sector placed for its hulls. Counted in
    /// `placed` too.
    pub escorts: usize,
}

/// What one sector's plan did: the clusters it owns bodies of, its background
/// scatter, and every placement it made or skipped.
///
/// Diagnostics only. The streamed manifest carries bodies; a debug view or a
/// range asks this for the same numbers the generator used.
#[derive(Clone, Debug, PartialEq)]
pub struct SectorClusters {
    /// The fields at the sector's centre, which its background scatter reads.
    pub environment: Environment,
    /// Every cluster with at least one body this sector owns, in node order.
    pub clusters: Vec<ClusterSummary>,
    /// How many background rocks this sector placed. Zero in a sector that
    /// owns any cluster body.
    pub background_rocks: usize,
    /// How many bodies this sector placed in all: its manifest's object count.
    pub placed: usize,
    /// How many owned bodies this sector skipped at a face.
    pub skipped_face: usize,
    /// How many owned bodies this sector skipped for clearance.
    pub skipped_clearance: usize,
    /// How many owned hulls this sector skipped for want of a companion.
    pub skipped_companion: usize,
    /// How many escort rocks this sector placed. Counted in `placed` too.
    pub escorts: usize,
}

/// What one sector's plan did, from the same plan the generator streams.
///
/// # Errors
///
/// Whatever [`crate::NovaLayeredWorld`] would refuse the sector for.
pub fn sector_clusters(input: SectorGenerationInput) -> Result<SectorClusters, SectorFault> {
    Ok(plan_sector(&EnvironmentFields::new(input.seed), input)?.summary())
}

/// Refuse a geometry this policy cannot fill.
///
/// A planetoid's derived mass has to stay under the default surface gravity
/// cap, two neighbouring worlds and an escort and its hull have to clear each
/// other, the widest core of worlds has to fit inside one cell, the widest
/// background scatter too, and the bands have to keep every cluster inside
/// [`CLUSTER_EXTENT_MAX`].
pub(crate) fn validate_cluster_geometry(geometry: WorldGeometry) -> Result<(), SectorFault> {
    let settings = GravitySettings::default();
    let surface = settings.soi_cutoff_accel * PLANETOID_SOI_REACH * PLANETOID_SOI_REACH;
    if surface > settings.max_surface_gravity {
        return Err(SectorFault::Config {
            field: "generator.planetoid_soi_reach",
            value: format!(
                "{PLANETOID_SOI_REACH} body radii, a {} m/s^2 surface pull past the {} m/s^2 cap",
                MetersPerSecondSquared::from_engine(surface).get(),
                MetersPerSecondSquared::from_engine(settings.max_surface_gravity).get()
            ),
        });
    }
    let worlds = widest_planetoid() * 2.0 + CLEARANCE_MARGIN;
    if WORLD_SPACING.0 < worlds {
        return Err(SectorFault::Config {
            field: "generator.world_spacing",
            value: format!(
                "{} m, under the {} m two of the widest worlds need",
                WORLD_SPACING.0.get(),
                worlds.get()
            ),
        });
    }
    let escort = SECTOR_SHIP_CLEARANCE + rock_clearance_max() + CLEARANCE_MARGIN;
    if ESCORT_DISTANCE.0 < escort {
        return Err(SectorFault::Config {
            field: "generator.escort_distance",
            value: format!(
                "{} m, under the {} m a hull and the widest rock need",
                ESCORT_DISTANCE.0.get(),
                escort.get()
            ),
        });
    }
    if extent_max() > CLUSTER_EXTENT_MAX {
        return Err(SectorFault::Config {
            field: "generator.cluster_extent",
            value: format!(
                "{} m, past the {} m cluster extent limit",
                extent_max().get(),
                CLUSTER_EXTENT_MAX.get()
            ),
        });
    }
    let core = core_reach_max() + ROUNDING_SLACK;
    geometry.require_owning_edge(
        0.0,
        core,
        &format!("a core of planetoids reaching {} m", core.get()),
    )?;
    let scatter = background_reach_max() + ROUNDING_SLACK;
    geometry.require_owning_edge(
        0.0,
        scatter,
        &format!("a background scatter reaching {} m", scatter.get()),
    )
}

/// One body the policy plans, before a cell decides whether it fits.
#[derive(Clone, Debug)]
enum ClusterBody {
    Rock { radius: Meters, kind: &'static str },
    Planetoid(PlanetConfig),
    Hull { design: &'static str, yaw: f32 },
}

impl ClusterBody {
    /// The clearance sphere `validate_manifest` measures the body by.
    fn clearance(&self) -> Meters {
        match self {
            Self::Rock { radius, .. } => Meters(radius.get() * ASTEROID_GEOMETRIC_FACTOR_MAX),
            Self::Planetoid(config) => config.body_radius(),
            Self::Hull { .. } => SECTOR_SHIP_CLEARANCE,
        }
    }
}

/// One body of a cluster, where it stands, and the cell that owns it.
#[derive(Clone, Debug)]
struct ClusterMember {
    body: ClusterBody,
    position: Meters3,
    owner: SectorCoord,
}

/// One hull of a cluster and the escort rocks planned around it.
#[derive(Clone, Debug)]
struct ClusterHull {
    hull: ClusterMember,
    /// [`HULL_ESCORTS`] rocks around the hull, in the order a cell tries
    /// them. Only the cell that owns the hull places one, and only one.
    escorts: Vec<ClusterMember>,
}

/// One cluster, decided at its lattice node. The same value from every cell.
#[derive(Clone, Debug)]
struct Cluster {
    node: [i32; 3],
    cluster_type: ClusterType,
    anchor: Meters3,
    home: SectorCoord,
    /// Its planetoids, none to four. Each one must be placed.
    parents: Vec<ClusterMember>,
    /// Its rocks. Each one may be skipped.
    rocks: Vec<ClusterMember>,
    /// Its hulls. Each one may be skipped, and is without a companion.
    hulls: Vec<ClusterHull>,
}

impl Cluster {
    fn slug(node: [i32; 3]) -> String {
        let [x, y, z] = node.map(|index| {
            if index < 0 {
                format!("n{}", index.unsigned_abs())
            } else {
                index.to_string()
            }
        });
        format!("cluster_{x}_{y}_{z}")
    }

    /// Every body it plans, every escort included.
    fn bodies(&self) -> impl Iterator<Item = &ClusterMember> {
        self.parents.iter().chain(&self.rocks).chain(
            self.hulls
                .iter()
                .flat_map(|hull| std::iter::once(&hull.hull).chain(&hull.escorts)),
        )
    }

    /// How far its farthest clearance sphere reaches from the anchor, every
    /// escort included.
    fn extent(&self) -> Meters {
        self.bodies().fold(Meters::ZERO, |widest, member| {
            widest.max(member.position.distance(self.anchor) + member.body.clearance())
        })
    }
}

/// Where a planned body came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum BodySource {
    Parent([i32; 3], usize),
    Rock([i32; 3], usize),
    Hull([i32; 3], usize),
    /// An escort of the hull at that index, and its own index.
    Escort([i32; 3], usize, usize),
    Background,
}

impl BodySource {
    fn node(self) -> Option<[i32; 3]> {
        match self {
            Self::Parent(node, _)
            | Self::Rock(node, _)
            | Self::Hull(node, _)
            | Self::Escort(node, ..) => Some(node),
            Self::Background => None,
        }
    }
}

/// Why a planned body was not placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SkipType {
    /// Its clearance sphere crosses a face of the cell its centre is in.
    Face,
    /// It comes within [`CLEARANCE_MARGIN`] of a body placed before it.
    Clearance,
    /// It is a hull, its cluster placed no rock or planetoid in its cell, and
    /// none of its escorts fits there.
    Companion,
}

impl SkipType {
    const fn label(self) -> &'static str {
        match self {
            Self::Face => "face",
            Self::Clearance => "clearance",
            Self::Companion => "companion",
        }
    }
}

/// One body a cell owns, and what the cell did with it.
#[derive(Clone, Debug)]
struct PlannedBody {
    id: String,
    source: BodySource,
    body: ClusterBody,
    position: Meters3,
    /// `None` when placed.
    skipped: Option<SkipType>,
}

/// Everything one cell planned.
#[derive(Clone, Debug)]
pub(crate) struct SectorPlan {
    coord: SectorCoord,
    environment: Environment,
    /// Every cluster with at least one body this cell owns, in node order.
    clusters: Vec<Cluster>,
    /// Every body this cell owns, in resolve order.
    bodies: Vec<PlannedBody>,
}

impl SectorPlan {
    /// The placed bodies as the manifest `nova_world` checks.
    pub(crate) fn manifest(&self) -> SectorManifest {
        let mut manifest = SectorManifest {
            coord: self.coord,
            asteroids: Vec::new(),
            planets: Vec::new(),
            ships: Vec::new(),
        };
        for planned in self.bodies.iter().filter(|body| body.skipped.is_none()) {
            let id = planned.id.clone();
            let position = planned.position;
            match &planned.body {
                ClusterBody::Rock { radius, kind } => manifest.asteroids.push(SectorAsteroid {
                    seed: asteroid_seed_from_id(&id),
                    id,
                    position,
                    radius: *radius,
                    kind: (*kind).into(),
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
                    design: (*design).into(),
                }),
            }
        }
        manifest
    }

    fn summary(&self) -> SectorClusters {
        let count = |node: Option<[i32; 3]>, skipped: Option<SkipType>| {
            self.bodies
                .iter()
                .filter(|body| node.is_none_or(|node| body.source.node() == Some(node)))
                .filter(|body| body.skipped == skipped)
                .count()
        };
        let escorts = |node: Option<[i32; 3]>| {
            self.bodies
                .iter()
                .filter(|body| body.skipped.is_none())
                .filter(|body| match body.source {
                    BodySource::Escort(escort, ..) => node.is_none_or(|node| escort == node),
                    _ => false,
                })
                .count()
        };
        SectorClusters {
            environment: self.environment,
            clusters: self
                .clusters
                .iter()
                .map(|cluster| ClusterSummary {
                    id: Cluster::slug(cluster.node),
                    cluster_type: cluster.cluster_type,
                    anchor: cluster.anchor,
                    home: cluster.home,
                    extent: cluster.extent(),
                    placed: count(Some(cluster.node), None),
                    skipped_face: count(Some(cluster.node), Some(SkipType::Face)),
                    skipped_clearance: count(Some(cluster.node), Some(SkipType::Clearance)),
                    skipped_companion: count(Some(cluster.node), Some(SkipType::Companion)),
                    escorts: escorts(Some(cluster.node)),
                })
                .collect(),
            background_rocks: self
                .bodies
                .iter()
                .filter(|body| body.source == BodySource::Background && body.skipped.is_none())
                .count(),
            placed: count(None, None),
            skipped_face: count(None, Some(SkipType::Face)),
            skipped_clearance: count(None, Some(SkipType::Clearance)),
            skipped_companion: count(None, Some(SkipType::Companion)),
            escorts: escorts(None),
        }
    }
}

/// The weight of each type, in [`ClusterType::ALL`] order, and of no cluster
/// last. The no-cluster weight never reaches zero, so the total is always
/// positive.
fn cluster_weights(environment: Environment) -> [(Option<ClusterType>, f32); 5] {
    let Environment {
        material_density: m,
        volatiles: v,
        human_activity: h,
    } = environment;
    [
        (Some(ClusterType::AsteroidRich), 0.9 * m),
        (Some(ClusterType::RockOnly), 0.5 * m * (1.0 - h)),
        (
            Some(ClusterType::PlanetHeavy),
            0.6 * ramp(m, 0.45, 0.85) * (1.0 - 0.7 * h) * (0.6 + 0.4 * v),
        ),
        (
            Some(ClusterType::DerelictField),
            0.7 * ramp(h, 0.4, 0.8) * (1.0 - 0.5 * m),
        ),
        (None, 0.2 + 0.4 * (1.0 - m) + 0.4 * (1.0 - m) * (1.0 - h)),
    ]
}

/// The chance a cell that owns no cluster body draws a background scatter:
/// dense material, cleared by traffic.
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

/// How many of a `band` a cluster places when `richness` reads in `[0, 1]`: a
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
        radius: across(ROCK_RADIUS, stream.unit().powi(ROCK_RADIUS_SKEW)),
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

/// The cluster decided at `node`, or `None` for a node that grows none.
///
/// PURE, and never keyed by the cell asking, so every cell that replays a node
/// gets the same cluster. Every draw comes off one stream keyed by the seed
/// and the node alone, in one fixed order. The edge draws nothing: it only
/// names the home cell and each member's owning cell, and pulls a core with
/// worlds inside the home cell.
///
/// # Errors
///
/// Whatever the environment refuses, and [`SectorFault::InvalidGeometry`] for
/// a body with no finite position.
fn cluster_at(
    fields: &EnvironmentFields,
    seed: u32,
    edge: Meters,
    node: [i32; 3],
) -> Result<Option<Cluster>, SectorFault> {
    let mut stream = SeedStream::new(
        Fnv32::new()
            .write(&seed.to_le_bytes())
            .write(b"cluster")
            .write(&node[0].to_le_bytes())
            .write(&node[1].to_le_bytes())
            .write(&node[2].to_le_bytes())
            .finish(),
    );
    let lattice = CLUSTER_LATTICE.get();
    let jitter = CLUSTER_JITTER * lattice * 0.5;
    let [x, y, z] = node.map(|index| index as f32 * lattice);
    let drawn = Meters3::new(
        x + stream.signed() * jitter,
        y + stream.signed() * jitter,
        z + stream.signed() * jitter,
    );
    let environment = fields.sample(drawn)?;
    let Some(cluster_type) = pick(&cluster_weights(environment), stream.unit()) else {
        return Ok(None);
    };
    let Environment {
        material_density: m,
        volatiles: v,
        human_activity: h,
    } = environment;

    let worlds = match cluster_type {
        ClusterType::PlanetHeavy => member_count(PLANET_HEAVY_WORLDS, v, stream.unit()),
        ClusterType::AsteroidRich => {
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
        ClusterType::RockOnly | ClusterType::DerelictField => 0,
    };
    let hulls = match cluster_type {
        ClusterType::DerelictField => member_count(DERELICT_FIELD_HULLS, h, stream.unit()),
        ClusterType::AsteroidRich => usize::from(stream.unit() < ASTEROID_RICH_HULL_CHANCE_MAX * h),
        ClusterType::RockOnly | ClusterType::PlanetHeavy => 0,
    };
    let rocks = match cluster_type {
        ClusterType::AsteroidRich => ASTEROID_RICH_ROCKS,
        ClusterType::RockOnly => ROCK_ONLY_ROCKS,
        ClusterType::PlanetHeavy => PLANET_HEAVY_ROCKS,
        ClusterType::DerelictField => DERELICT_FIELD_ROCKS,
    };
    let rocks = member_count(rocks, m, stream.unit());

    // Worlds first: their size decides how far the anchor must stand from
    // every face for each world to sit wholly inside the anchor's cell.
    let configs: Vec<PlanetConfig> = (0..worlds)
        .map(|_| {
            let radius = across(PLANETOID_RADIUS, stream.unit());
            let planet_type = planet_type(environment, stream.unit());
            let mut config = PlanetConfig::new(planet_type, radius, stream.next_u32());
            config.mass = Some(planetoid_mass(config.body_radius()));
            config
        })
        .collect();
    let offsets: Vec<Vec3> = if worlds >= 2 {
        let spacing = across(WORLD_SPACING, stream.unit()).get();
        let axis = unit_sphere_point(stream.next_u32());
        let spin = stream.unit() * std::f32::consts::TAU;
        let rotation = Quat::from_rotation_arc(Vec3::X, axis) * Quat::from_rotation_x(spin);
        WORLD_CORNERS[worlds]
            .iter()
            .map(|corner| rotation * *corner * spacing)
            .collect()
    } else {
        vec![Vec3::ZERO; worlds]
    };
    let core = configs
        .iter()
        .zip(&offsets)
        .fold(Meters::ZERO, |widest, (config, offset)| {
            widest.max(Meters(offset.length()) + config.body_radius())
        });

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
    let invalid = || SectorFault::InvalidGeometry {
        id: Cluster::slug(node),
    };
    if !anchor.get().is_finite() {
        return Err(invalid());
    }
    let parents = configs
        .into_iter()
        .zip(offsets)
        .map(|(config, offset)| ClusterMember {
            body: ClusterBody::Planetoid(config),
            position: Meters3(anchor.get() + offset),
            owner: home,
        })
        .collect();

    // Members stand in a shell past the core, denser toward its inner edge.
    let (inner, spread) = if worlds == 0 {
        (Meters::ZERO, across(OPEN_SPREAD, stream.unit()))
    } else {
        (
            core + member_clearance_max() + CLEARANCE_MARGIN,
            across(WORLD_SPREAD, stream.unit()),
        )
    };
    let member = |position: Meters3, body: ClusterBody| {
        if position.get().is_finite() {
            Ok(ClusterMember {
                body,
                position,
                owner: SectorCoord::containing(position, edge),
            })
        } else {
            Err(invalid())
        }
    };
    let mut rock_members = Vec::with_capacity(rocks);
    let mut hull_members = Vec::with_capacity(hulls);
    for index in 0..hulls + rocks {
        let direction = unit_sphere_point(stream.next_u32());
        let distance = inner + spread * stream.unit().sqrt();
        let position = Meters3(anchor.get() + direction * distance.get());
        if index >= hulls {
            rock_members.push(member(position, rock(environment, &mut stream))?);
            continue;
        }
        let hull = member(position, hull(environment, &mut stream))?;
        let outward = hull.position.get() - anchor.get();
        let escorts = (0..HULL_ESCORTS)
            .map(|_| {
                let drawn = unit_sphere_point(stream.next_u32());
                // Turned to face the anchor, which bounds its reach.
                let direction = if drawn.dot(outward) > 0.0 {
                    -drawn
                } else {
                    drawn
                };
                let distance = across(ESCORT_DISTANCE, stream.unit());
                let position = Meters3(hull.position.get() + direction * distance.get());
                member(position, rock(environment, &mut stream))
            })
            .collect::<Result<_, _>>()?;
        hull_members.push(ClusterHull { hull, escorts });
    }

    Ok(Some(Cluster {
        node,
        cluster_type,
        anchor,
        home,
        parents,
        rocks: rock_members,
        hulls: hull_members,
    }))
}

/// The mass that gives a planetoid of this outer radius a sphere of influence
/// of [`PLANETOID_SOI_REACH`] radii: `mu = soi_cutoff_accel * soi^2`.
fn planetoid_mass(body_radius: Meters) -> f32 {
    let soi = PLANETOID_SOI_REACH * body_radius.to_engine();
    GravitySettings::default().soi_cutoff_accel * soi * soi
}

/// The widest clearance a member can have: a hull's, or the widest rock's.
fn member_clearance_max() -> Meters {
    rock_clearance_max().max(SECTOR_SHIP_CLEARANCE)
}

fn rock_clearance_max() -> Meters {
    Meters(ROCK_RADIUS.1.get() * ASTEROID_GEOMETRIC_FACTOR_MAX)
}

/// The widest planetoid clearance this policy can draw: the OUTER radius of
/// the widest type at the top of the band, which is what `validate_manifest`
/// measures it by.
fn widest_planetoid() -> Meters {
    PlanetType::ALL
        .iter()
        .map(|planet_type| PlanetConfig::new(*planet_type, PLANETOID_RADIUS.1, 0).body_radius())
        .fold(Meters::ZERO, Meters::max)
}

/// How far from its anchor the widest core of a type reaches: a world at the
/// corner of the widest layout the type draws, at the outer spacing, with the
/// widest clearance. Also the most its anchor is moved on each axis, less
/// [`ROUNDING_SLACK`]. Zero for a type with no worlds.
fn core_reach(cluster_type: ClusterType) -> Meters {
    let (_, most) = cluster_type.world_band();
    if most == 0 {
        return Meters::ZERO;
    }
    let corner = WORLD_CORNERS[..=most]
        .iter()
        .flat_map(|corners| corners.iter())
        .fold(0.0, |widest: f32, corner| widest.max(corner.length()));
    WORLD_SPACING.1 * corner + widest_planetoid()
}

/// The widest core of any type.
fn core_reach_max() -> Meters {
    ClusterType::ALL
        .into_iter()
        .map(core_reach)
        .fold(Meters::ZERO, Meters::max)
}

/// How far from its anchor a rock or hull centre of a type can stand: the
/// open spread when it may draw no worlds, the shell past its widest core
/// when it may draw one.
fn shell_reach(cluster_type: ClusterType) -> Meters {
    let (fewest, most) = cluster_type.world_band();
    let open = if fewest == 0 {
        OPEN_SPREAD.1
    } else {
        Meters::ZERO
    };
    let shell = if most > 0 {
        core_reach(cluster_type) + member_clearance_max() + CLEARANCE_MARGIN + WORLD_SPREAD.1
    } else {
        Meters::ZERO
    };
    open.max(shell)
}

/// How far from its anchor a member centre of a type can stand. An escort
/// stands on the side of its hull that faces the anchor, so it is at most the
/// hypotenuse of the shell and [`ESCORT_DISTANCE`] away.
fn member_reach(cluster_type: ClusterType) -> Meters {
    let shell = shell_reach(cluster_type);
    if cluster_type.hull_band().1 == 0 {
        shell
    } else {
        Meters(shell.get().hypot(ESCORT_DISTANCE.1.get())) + ROUNDING_SLACK
    }
}

/// How far from its anchor any clearance sphere of a type can reach.
fn extent(cluster_type: ClusterType) -> Meters {
    (member_reach(cluster_type) + member_clearance_max()).max(core_reach(cluster_type))
        + ROUNDING_SLACK
}

/// How far from its anchor any clearance sphere of any cluster can reach.
fn extent_max() -> Meters {
    ClusterType::ALL
        .into_iter()
        .map(extent)
        .fold(Meters::ZERO, Meters::max)
}

/// How far from its centre a background scatter's clearance can reach.
fn background_reach_max() -> Meters {
    BACKGROUND_SPREAD.1 + rock_clearance_max()
}

/// How far on one axis a cluster's node can be from a body centre the cluster
/// plans.
///
/// An anchor leaves its node by one jitter draw and, with worlds, one core
/// pull. A parent stands within the core and a member within
/// [`member_reach`] of the anchor. The distance along one axis is at most
/// the distance itself, so this bounds every axis.
fn cluster_reach() -> Meters {
    let reach = ClusterType::ALL
        .into_iter()
        .map(|cluster_type| core_reach(cluster_type) + ROUNDING_SLACK + member_reach(cluster_type))
        .fold(Meters::ZERO, Meters::max);
    Meters(CLUSTER_JITTER * CLUSTER_LATTICE.get() * 0.5) + reach
}

/// Every lattice node whose cluster could place a body centre in `coord`, in
/// node order.
///
/// A body centre in the cell is within half an edge of its centre on each
/// axis, and within [`cluster_reach`] of its node, so a node further than both
/// together on any axis cannot reach the cell. At a 32 km edge that is about
/// 36 km each way: one or two nodes an axis, 1 to 8 in all.
fn halo_nodes(coord: SectorCoord, edge: Meters) -> Vec<[i32; 3]> {
    let reach = edge.get() * 0.5 + cluster_reach().get() + ROUNDING_SLACK.get();
    let lattice = CLUSTER_LATTICE.get();
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

/// Plan one cell: replay every cluster in its halo, keep the bodies it owns,
/// draw a background scatter if it owns none, and resolve them all.
///
/// PURE: the same fields and input give the same plan, on any call, in any
/// order.
///
/// # Errors
///
/// Whatever the environment refuses, [`SectorFault::InvalidGeometry`] for a
/// body with no finite position, and [`SectorFault::Generation`] for a parent
/// that could not be placed.
pub(crate) fn plan_sector(
    fields: &EnvironmentFields,
    input: SectorGenerationInput,
) -> Result<SectorPlan, SectorFault> {
    let coord = input.coord;
    let edge = input.geometry.sector_edge;
    let mut clusters = Vec::new();
    let mut parents = Vec::new();
    let mut rocks = Vec::new();
    let mut hulls = Vec::new();
    for node in halo_nodes(coord, edge) {
        let Some(cluster) = cluster_at(fields, input.seed, edge, node)? else {
            continue;
        };
        let stem = Cluster::slug(cluster.node);
        let candidate =
            |name: &str, index: usize, source: BodySource, member: &ClusterMember| Candidate {
                id: sector_id(coord, &format!("{stem}_{name}"), index),
                source,
                body: member.body.clone(),
                position: member.position,
                escorts: Vec::new(),
            };
        let before = parents.len() + rocks.len() + hulls.len();
        parents.extend(
            cluster
                .parents
                .iter()
                .enumerate()
                .filter(|(_, member)| member.owner == coord)
                .map(|(index, member)| {
                    candidate("parent", index, BodySource::Parent(node, index), member)
                }),
        );
        rocks.extend(
            cluster
                .rocks
                .iter()
                .enumerate()
                .filter(|(_, member)| member.owner == coord)
                .map(|(index, member)| {
                    candidate("rock", index, BodySource::Rock(node, index), member)
                }),
        );
        hulls.extend(
            cluster
                .hulls
                .iter()
                .enumerate()
                .filter(|(_, hull)| hull.hull.owner == coord)
                .map(|(index, hull)| Candidate {
                    escorts: hull
                        .escorts
                        .iter()
                        .enumerate()
                        .map(|(escort, member)| {
                            candidate(
                                &format!("hull_{index}_escort"),
                                escort,
                                BodySource::Escort(node, index, escort),
                                member,
                            )
                        })
                        .collect(),
                    ..candidate("hull", index, BodySource::Hull(node, index), &hull.hull)
                }),
        );
        if parents.len() + rocks.len() + hulls.len() > before {
            clusters.push(cluster);
        }
    }

    let centre = coord.centre(edge);
    let environment = fields.sample(centre)?;
    let mut background = Vec::new();
    let mut stream = input.stream("background");
    if clusters.is_empty() && stream.unit() < background_chance(environment) {
        // The scatter's centre stays far enough inside the cell that its
        // widest reach does too; `validate` refused an edge where that box
        // would be inverted.
        let room = edge.get() * 0.5 - (background_reach_max() + ROUNDING_SLACK).get();
        let middle = centre
            + Meters3::new(
                stream.signed() * room,
                stream.signed() * room,
                stream.signed() * room,
            );
        let count = member_count(BACKGROUND_ROCKS, 1.0, stream.unit());
        let spread = across(BACKGROUND_SPREAD, stream.unit());
        for index in 0..count {
            let direction = unit_sphere_point(stream.next_u32());
            let position =
                Meters3(middle.get() + direction * (spread * stream.unit().sqrt()).get());
            background.push(Candidate {
                id: sector_id(coord, "background_rock", index),
                source: BodySource::Background,
                body: rock(environment, &mut stream),
                position,
                escorts: Vec::new(),
            });
        }
    }

    let candidates = parents
        .into_iter()
        .chain(rocks)
        .chain(hulls)
        .chain(background);
    Ok(SectorPlan {
        coord,
        environment,
        clusters,
        bodies: resolve(input, candidates)?,
    })
}

/// One body a cell owns, before it is resolved.
struct Candidate {
    id: String,
    source: BodySource,
    body: ClusterBody,
    position: Meters3,
    /// A hull's escorts, in the order they are tried. Empty for every other
    /// body.
    escorts: Vec<Candidate>,
}

/// Place or skip every candidate, in the order given.
///
/// The checks mirror `validate_manifest` - the whole clearance sphere inside
/// the cell, no overlap - plus this policy's own [`CLEARANCE_MARGIN`], so a
/// placed body is one the check accepts and each refusal becomes a counted
/// skip instead of a refused manifest. A hull that fits is placed only when a
/// rock or planetoid of its own cluster was placed before it, or when one of
/// its escorts fits beside it and is placed first; every escort tried before
/// that one is skipped and counted.
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
    let fits = |standing: &[(Meters3, Meters)], position: Meters3, clearance: Meters| {
        let inside = SectorCoord::containing(position, edge) == input.coord
            && (position.get() - centre).abs().max_element() + clearance.get() <= edge.get() * 0.5;
        if !inside {
            Some(SkipType::Face)
        } else if !standing.iter().all(|&(other, other_clearance)| {
            bodies_clear(
                other,
                other_clearance,
                position,
                clearance,
                CLEARANCE_MARGIN,
            )
        }) {
            Some(SkipType::Clearance)
        } else {
            None
        }
    };
    let mut standing: Vec<(Meters3, Meters)> = Vec::new();
    let mut planned: Vec<PlannedBody> = Vec::new();
    for Candidate {
        id,
        source,
        body,
        position,
        escorts,
    } in candidates
    {
        if !position.get().is_finite() {
            return Err(SectorFault::InvalidGeometry { id });
        }
        let clearance = body.clearance();
        let mut skipped = fits(&standing, position, clearance);
        if let (BodySource::Parent(..), Some(reason)) = (source, skipped) {
            return Err(SectorFault::Generation {
                id,
                field: "parent",
                value: format!("skipped for {}", reason.label()),
            });
        }
        if let (BodySource::Hull(node, _), None) = (source, skipped) {
            let accompanied = planned.iter().any(|other| {
                other.skipped.is_none()
                    && other.source.node() == Some(node)
                    && !matches!(other.body, ClusterBody::Hull { .. })
            });
            if !accompanied {
                skipped = Some(SkipType::Companion);
                for escort in escorts {
                    if !escort.position.get().is_finite() {
                        return Err(SectorFault::InvalidGeometry { id: escort.id });
                    }
                    let escort_clearance = escort.body.clearance();
                    // `validate` keeps the escort band clear of its hull.
                    let escort_skipped = fits(&standing, escort.position, escort_clearance);
                    if escort_skipped.is_none() {
                        standing.push((escort.position, escort_clearance));
                    }
                    planned.push(PlannedBody {
                        id: escort.id,
                        source: escort.source,
                        body: escort.body,
                        position: escort.position,
                        skipped: escort_skipped,
                    });
                    if escort_skipped.is_none() {
                        skipped = None;
                        break;
                    }
                }
            }
        }
        if skipped.is_none() {
            standing.push((position, clearance));
        }
        planned.push(PlannedBody {
            id,
            source,
            body,
            position,
            skipped,
        });
    }
    Ok(planned)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use nova_gameplay::prelude::GravityWell;

    use super::*;
    use crate::NovaLayeredWorld;

    const SEED: u32 = 20_260_922;

    fn config() -> WorldConfig<NovaLayeredWorld> {
        WorldConfig {
            seed: SEED,
            sector_edge: Meters(32_000.0),
            active_radius: 2,
            generator: NovaLayeredWorld,
        }
    }

    /// The 125 plans around the origin, the window the examples open in.
    fn window_plans() -> BTreeMap<SectorCoord, SectorPlan> {
        let config = config();
        let fields = EnvironmentFields::new(SEED);
        desired_sectors(SectorCoord::ORIGIN, config.active_radius)
            .into_iter()
            .map(|coord| {
                let plan = plan_sector(&fields, config.input(coord))
                    .unwrap_or_else(|fault| panic!("{coord}: {fault}"));
                (coord, plan)
            })
            .collect()
    }

    /// Every cluster decided at the nodes of an 8-node cube around the
    /// origin: 512 nodes, every type many times over.
    fn scanned_clusters() -> Vec<Cluster> {
        let fields = EnvironmentFields::new(SEED);
        let edge = config().sector_edge;
        let mut clusters = Vec::new();
        for x in -4..4 {
            for y in -4..4 {
                for z in -4..4 {
                    let cluster = cluster_at(&fields, SEED, edge, [x, y, z])
                        .unwrap_or_else(|fault| panic!("[{x}, {y}, {z}]: {fault}"));
                    clusters.extend(cluster);
                }
            }
        }
        clusters
    }

    /// Every cell that replays a node gets the same cluster, down to the last
    /// body, and the same cluster a direct call at the node gets.
    #[test]
    fn a_cluster_is_the_same_cluster_from_every_cell_that_replays_it() {
        let fields = EnvironmentFields::new(SEED);
        let mut seen: BTreeMap<[i32; 3], (String, usize)> = BTreeMap::new();
        for plan in window_plans().values() {
            for cluster in &plan.clusters {
                let text = format!("{cluster:?}");
                let (first, cells) = seen
                    .entry(cluster.node)
                    .or_insert_with(|| (text.clone(), 0));
                assert_eq!(
                    *first,
                    text,
                    "{} differs from {}",
                    Cluster::slug(cluster.node),
                    plan.coord
                );
                *cells += 1;
            }
        }
        assert!(
            seen.values().any(|(_, cells)| *cells >= 2),
            "some cluster must be replayed by more than one cell"
        );
        for (node, (text, _)) in &seen {
            let direct = cluster_at(&fields, SEED, config().sector_edge, *node)
                .unwrap_or_else(|fault| panic!("{node:?}: {fault}"))
                .unwrap_or_else(|| panic!("{node:?} grew no cluster at its node"));
            assert_eq!(*text, format!("{direct:?}"), "{node:?} at its node");
        }
    }

    /// Every body a cluster plans is owned by exactly one cell - the one its
    /// centre falls in - and placed or skipped there, and an escort is tried
    /// only by its hull's cell; a cell streams exactly what it placed; and the
    /// window holds a cluster with a planetoid that PLACES bodies on both
    /// sides of a face.
    #[test]
    fn every_cluster_body_has_one_owner_and_clusters_cross_faces() {
        let config = config();
        let plans = window_plans();
        let mut owners: BTreeMap<BodySource, Vec<SectorCoord>> = BTreeMap::new();
        let mut clusters = BTreeMap::new();
        let mut placing: BTreeMap<[i32; 3], BTreeSet<SectorCoord>> = BTreeMap::new();
        let mut placed_sources = BTreeSet::new();
        for plan in plans.values() {
            let description = generate_sector(&config, plan.coord)
                .unwrap_or_else(|fault| panic!("{}: {fault}", plan.coord));
            assert_eq!(
                description.object_count(),
                plan.summary().placed,
                "{}",
                plan.coord
            );
            for body in &plan.bodies {
                let Some(node) = body.source.node() else {
                    continue;
                };
                owners.entry(body.source).or_default().push(plan.coord);
                if body.skipped.is_none() {
                    placing.entry(node).or_default().insert(plan.coord);
                    placed_sources.insert(body.source);
                }
            }
            for cluster in &plan.clusters {
                clusters.insert(cluster.node, cluster.clone());
            }
        }
        // A cluster whose anchor stands one cell inside the window has every
        // body inside it: no body stands 8 km from its anchor, and a cell is
        // 32 km.
        let inner = desired_sectors(SectorCoord::ORIGIN, config.active_radius - 1);
        let mut checked = 0;
        for cluster in clusters
            .values()
            .filter(|cluster| inner.contains(&cluster.home))
        {
            let node = cluster.node;
            let sources = (cluster.parents.iter().enumerate())
                .map(|(index, body)| (BodySource::Parent(node, index), body))
                .chain(
                    (cluster.rocks.iter().enumerate())
                        .map(|(index, body)| (BodySource::Rock(node, index), body)),
                )
                .chain(
                    (cluster.hulls.iter().enumerate())
                        .map(|(index, hull)| (BodySource::Hull(node, index), &hull.hull)),
                );
            for (source, body) in sources {
                assert_eq!(
                    owners.get(&source),
                    Some(&vec![body.owner]),
                    "{source:?} of {} must be planned once, by its owner",
                    Cluster::slug(node)
                );
            }
            for (index, hull) in cluster.hulls.iter().enumerate() {
                for (escort, body) in hull.escorts.iter().enumerate() {
                    let source = BodySource::Escort(node, index, escort);
                    if let Some(cells) = owners.get(&source) {
                        assert_eq!(
                            cells,
                            &vec![hull.hull.owner],
                            "{source:?} of {} must be tried only by its hull's cell",
                            Cluster::slug(node)
                        );
                        if body.owner != hull.hull.owner {
                            assert!(
                                !placed_sources.contains(&source),
                                "{source:?} crossed a face"
                            );
                        }
                    }
                }
            }
            checked += 1;
        }
        assert!(checked > 0, "the inner window must hold a cluster's anchor");
        assert!(
            owners.values().all(|cells| cells.len() == 1),
            "no cluster body may be planned by two cells"
        );

        assert!(
            placing
                .iter()
                .any(|(node, cells)| cells.len() >= 2 && !clusters[node].parents.is_empty()),
            "the window must hold a cluster with a planetoid placing bodies in two cells"
        );
    }

    /// A rock over a body placed before it is skipped by clearance, a rock
    /// whose clearance crosses a face is skipped at the face, both counted -
    /// and a parent that would be skipped is a fault, never a skip.
    #[test]
    fn a_crowded_member_is_skipped_but_a_crowded_parent_is_refused() {
        let input = config().input(SectorCoord::ORIGIN);
        let node = [0, 0, 0];
        let planned = resolve(
            input,
            [
                rock_at("first", BodySource::Rock(node, 0), 12_000.0),
                rock_at("second", BodySource::Rock(node, 1), 12_000.0),
                rock_at("third", BodySource::Rock(node, 2), 15_800.0),
            ],
        )
        .expect("members are skipped, never refused");
        assert_eq!(
            planned.iter().map(|body| body.skipped).collect::<Vec<_>>(),
            [None, Some(SkipType::Clearance), Some(SkipType::Face)]
        );

        let fault = resolve(
            input,
            [
                rock_at("first", BodySource::Rock(node, 0), 12_000.0),
                Candidate {
                    source: BodySource::Parent(node, 0),
                    ..rock_at("parent", BodySource::Rock(node, 1), 12_000.0)
                },
            ],
        )
        .expect_err("a parent over a placed body must refuse");
        assert!(
            matches!(&fault, SectorFault::Generation { id, field: "parent", .. } if id == "parent"),
            "got {fault:?}"
        );
    }

    /// A 40 m rock candidate on the z axis of the origin cell.
    fn rock_at(id: &str, source: BodySource, z: f32) -> Candidate {
        Candidate {
            id: id.to_string(),
            source,
            body: ClusterBody::Rock {
                radius: Meters(40.0),
                kind: KIND_ROCK,
            },
            position: Meters3::new(0.0, 0.0, z),
            escorts: Vec::new(),
        }
    }

    /// A hull candidate on the z axis of the origin cell, with escorts at the
    /// given z.
    fn hull_at(node: [i32; 3], z: f32, escorts: &[f32]) -> Candidate {
        Candidate {
            id: "hull".to_string(),
            source: BodySource::Hull(node, 0),
            body: ClusterBody::Hull {
                design: BLOCK_WRECK_PLATE_SHIP_ID,
                yaw: 0.0,
            },
            position: Meters3::new(0.0, 0.0, z),
            escorts: escorts
                .iter()
                .enumerate()
                .map(|(index, z)| {
                    rock_at(
                        &format!("escort_{index}"),
                        BodySource::Escort(node, 0, index),
                        *z,
                    )
                })
                .collect(),
        }
    }

    /// A hull is placed beside a rock its own cluster placed in the cell
    /// without an escort; alone, it places the first escort that fits, before
    /// itself, and counts each escort it tried first. When no escort fits
    /// (past the face or over a rock of another cluster), it is skipped as
    /// companion and places nothing.
    #[test]
    fn a_hull_alone_in_its_cell_places_its_first_fitting_escort_or_is_skipped() {
        let input = config().input(SectorCoord::ORIGIN);
        let (own, other) = ([0, 0, 0], [1, 0, 0]);
        let outcome = |candidates: Vec<Candidate>| {
            resolve(input, candidates)
                .expect("a hull is skipped, never refused")
                .into_iter()
                .map(|body| (body.id, body.skipped))
                .collect::<Vec<_>>()
        };

        assert_eq!(
            outcome(vec![
                rock_at("rock", BodySource::Rock(own, 0), 0.0),
                hull_at(own, 12_000.0, &[13_500.0]),
            ]),
            [("rock".into(), None), ("hull".into(), None)],
            "a hull beside its own cluster's rock needs no escort"
        );
        assert_eq!(
            outcome(vec![hull_at(
                own,
                12_000.0,
                &[15_900.0, 13_500.0, 10_500.0]
            )]),
            [
                ("escort_0".into(), Some(SkipType::Face)),
                ("escort_1".into(), None),
                ("hull".into(), None)
            ],
            "a lone hull places its first escort that fits, counts the one it \
             tried before it, and tries no more"
        );
        assert_eq!(
            outcome(vec![
                rock_at("rock", BodySource::Rock(other, 0), 10_500.0),
                hull_at(own, 12_000.0, &[15_900.0, 10_500.0]),
            ]),
            [
                ("rock".into(), None),
                ("escort_0".into(), Some(SkipType::Face)),
                ("escort_1".into(), Some(SkipType::Clearance)),
                ("hull".into(), Some(SkipType::Companion))
            ],
            "another cluster's rock is no companion, and no escort fits"
        );
    }

    /// Across several seeds, every hull placed in any cell a scanned cluster
    /// owns stands where its own cluster placed a rock or planetoid; and the
    /// scan holds such a hull placed across a face from its cluster's home
    /// cell, and a hull whose companion is its own placed escort.
    #[test]
    fn every_placed_hull_has_a_companion_of_its_own_cluster_in_its_cell() {
        let (mut away, mut escorted) = (0, 0);
        for seed in [SEED, 1, 7, 42] {
            let config = WorldConfig { seed, ..config() };
            let fields = EnvironmentFields::new(seed);
            let mut homes = BTreeMap::new();
            let mut coords = BTreeSet::new();
            for x in -3..3 {
                for y in -3..3 {
                    for z in -3..3 {
                        let Some(cluster) =
                            cluster_at(&fields, seed, config.sector_edge, [x, y, z])
                                .unwrap_or_else(|fault| panic!("{seed} [{x}, {y}, {z}]: {fault}"))
                        else {
                            continue;
                        };
                        homes.insert(cluster.node, cluster.home);
                        coords.extend(cluster.hulls.iter().map(|hull| hull.hull.owner));
                    }
                }
            }
            for coord in coords {
                let plan = plan_sector(&fields, config.input(coord))
                    .unwrap_or_else(|fault| panic!("{seed} {coord}: {fault}"));
                let placed = || plan.bodies.iter().filter(|body| body.skipped.is_none());
                for body in placed() {
                    let BodySource::Hull(node, _) = body.source else {
                        continue;
                    };
                    let companions: Vec<BodySource> = placed()
                        .filter(|other| {
                            other.source.node() == Some(node)
                                && !matches!(other.body, ClusterBody::Hull { .. })
                        })
                        .map(|other| other.source)
                        .collect();
                    assert!(
                        !companions.is_empty(),
                        "{seed} {coord}: {} stands with no companion",
                        body.id
                    );
                    if homes.get(&node).is_some_and(|home| *home != coord) {
                        away += 1;
                    }
                    if companions
                        .iter()
                        .all(|source| matches!(source, BodySource::Escort(..)))
                    {
                        escorted += 1;
                    }
                }
            }
        }
        assert!(
            away > 0 && escorted > 0,
            "{away} hulls placed across a face from home, {escorted} beside only an escort"
        );
    }

    /// Each type places only the bodies it names, in the counts its bands
    /// allow, and every hull plans its escorts; every world is whole inside
    /// its anchor's cell and clear of its cluster's other worlds; no
    /// clearance sphere reaches past the 8 km extent limit; and the scan
    /// grows every type and every planet-heavy world count.
    #[test]
    fn each_type_places_only_its_own_bodies_inside_the_extent_limit() {
        let edge = config().sector_edge;
        assert!(extent_max() <= CLUSTER_EXTENT_MAX);
        let mut grown = BTreeSet::new();
        let mut heavy = BTreeSet::new();
        for cluster in scanned_clusters() {
            grown.insert(cluster.cluster_type);
            let slug = Cluster::slug(cluster.node);
            let (rocks, hulls, worlds) = (
                cluster.rocks.len(),
                cluster.hulls.len(),
                cluster.parents.len(),
            );
            let within = |value: usize, (min, max): (usize, usize)| (min..=max).contains(&value);
            let composed = match cluster.cluster_type {
                ClusterType::AsteroidRich => {
                    worlds <= 2 && hulls <= 1 && within(rocks, ASTEROID_RICH_ROCKS)
                }
                ClusterType::RockOnly => {
                    worlds == 0 && hulls == 0 && within(rocks, ROCK_ONLY_ROCKS)
                }
                ClusterType::PlanetHeavy => {
                    within(worlds, PLANET_HEAVY_WORLDS)
                        && hulls == 0
                        && within(rocks, PLANET_HEAVY_ROCKS)
                }
                ClusterType::DerelictField => {
                    worlds == 0
                        && within(hulls, DERELICT_FIELD_HULLS)
                        && within(rocks, DERELICT_FIELD_ROCKS)
                }
            };
            if cluster.cluster_type == ClusterType::PlanetHeavy {
                heavy.insert(worlds);
            }
            assert!(
                cluster
                    .hulls
                    .iter()
                    .all(|hull| hull.escorts.len() == HULL_ESCORTS),
                "{slug}: every hull plans {HULL_ESCORTS} escorts"
            );
            assert!(
                composed,
                "{slug} is {} with {worlds} worlds, {hulls} hulls and {rocks} rocks",
                cluster.cluster_type.label()
            );
            let centre = cluster.home.centre(edge).get();
            for parent in &cluster.parents {
                assert!(
                    (parent.position.get() - centre).abs().max_element()
                        + parent.body.clearance().get()
                        <= edge.get() * 0.5,
                    "{slug}: a world crosses a face of {}",
                    cluster.home
                );
            }
            for (index, one) in cluster.parents.iter().enumerate() {
                for other in &cluster.parents[index + 1..] {
                    assert!(
                        bodies_clear(
                            one.position,
                            one.body.clearance(),
                            other.position,
                            other.body.clearance(),
                            CLEARANCE_MARGIN,
                        ),
                        "{slug}: two worlds stand within the clearance margin"
                    );
                }
            }
            assert!(
                cluster.extent() <= CLUSTER_EXTENT_MAX,
                "{slug} reaches {} m from its anchor",
                cluster.extent().get()
            );
        }
        assert_eq!(grown, ClusterType::ALL.into_iter().collect::<BTreeSet<_>>());
        assert_eq!(
            heavy,
            (PLANET_HEAVY_WORLDS.0..=PLANET_HEAVY_WORLDS.1).collect::<BTreeSet<_>>(),
            "the scan must grow planet-heavy clusters of every world count"
        );
    }

    /// Every body a cluster plans - every escort too, tried or not - stands
    /// inside its type's derived extent and within the derived reach of its
    /// node on each axis, and every body a cluster places inside a cell comes
    /// from a node in the cell's halo, for several seeds at every edge the
    /// generator arms: a node the halo missed would be a body no cell plans.
    #[test]
    fn every_planned_body_stays_inside_its_derived_bounds_and_its_halo() {
        let reach = cluster_reach().get();
        let mut escorts = 0;
        for seed in [SEED, 1, 7, 42, 1_234] {
            let fields = EnvironmentFields::new(seed);
            for edge in [Meters(32_000.0), Meters(64_000.0), Meters(128_000.0)] {
                let mut owned: BTreeMap<SectorCoord, BTreeSet<[i32; 3]>> = BTreeMap::new();
                for x in -4..4 {
                    for y in -4..4 {
                        for z in -4..4 {
                            let Some(cluster) = cluster_at(&fields, seed, edge, [x, y, z])
                                .unwrap_or_else(|fault| panic!("{seed} [{x}, {y}, {z}]: {fault}"))
                            else {
                                continue;
                            };
                            let slug = Cluster::slug(cluster.node);
                            let bound = extent(cluster.cluster_type);
                            assert!(
                                cluster.extent() <= bound,
                                "{seed} {slug} reaches {} m, past its type's {} m",
                                cluster.extent().get(),
                                bound.get()
                            );
                            let node = Vec3::from_array(cluster.node.map(|index| index as f32))
                                * CLUSTER_LATTICE.get();
                            for body in cluster.bodies() {
                                let axis = (body.position.get() - node).abs().max_element();
                                assert!(
                                    axis <= reach,
                                    "{seed} {slug} stands {axis} m off its node"
                                );
                                owned.entry(body.owner).or_default().insert(cluster.node);
                            }
                            escorts += cluster
                                .hulls
                                .iter()
                                .map(|hull| hull.escorts.len())
                                .sum::<usize>();
                        }
                    }
                }
                for coord in desired_sectors(SectorCoord::ORIGIN, 1) {
                    let halo: BTreeSet<[i32; 3]> = halo_nodes(coord, edge).into_iter().collect();
                    let missed: Vec<_> = owned
                        .get(&coord)
                        .into_iter()
                        .flatten()
                        .filter(|node| !halo.contains(*node))
                        .collect();
                    assert!(
                        missed.is_empty(),
                        "{seed} {coord} at {edge:?} misses {missed:?}"
                    );
                }
            }
        }
        assert!(escorts > 0, "the scan must plan escorts");
    }

    /// No body of one cluster comes within the derived floor of a body of
    /// another, placed or skipped: two clusters never merge, and neither a
    /// parent refusal nor a clearance skip is ever between clusters.
    #[test]
    fn two_clusters_never_come_closer_than_the_gap_floor() {
        // The anchors' separation on the axis their nodes differ on, less two
        // core pulls and two of the widest extents.
        let floor = CLUSTER_LATTICE * (1.0 - CLUSTER_JITTER)
            - (core_reach_max() + ROUNDING_SLACK) * 2.0
            - extent_max() * 2.0;
        assert!(floor > CLEARANCE_MARGIN, "the floor is {} m", floor.get());
        let clusters = scanned_clusters();
        let mut pairs = 0;
        for (index, a) in clusters.iter().enumerate() {
            for b in &clusters[index + 1..] {
                if (0..3).any(|axis| (a.node[axis] - b.node[axis]).abs() > 1) {
                    continue;
                }
                pairs += 1;
                for one in a.bodies() {
                    for other in b.bodies() {
                        let gap = one.position.distance(other.position)
                            - one.body.clearance()
                            - other.body.clearance();
                        assert!(
                            gap >= floor,
                            "{} and {} come {} m apart",
                            Cluster::slug(a.node),
                            Cluster::slug(b.node),
                            gap.get()
                        );
                    }
                }
            }
        }
        assert!(pairs > 0, "the scan must hold neighbouring clusters");
    }

    /// A cell that owns any cluster body draws no background scatter; a cell
    /// that owns none draws either nothing or a scatter of one to five rocks,
    /// each placed or counted as skipped; and the scan holds both.
    #[test]
    fn only_a_cell_with_no_cluster_body_draws_a_background_scatter() {
        let config = config();
        let fields = EnvironmentFields::new(SEED);
        let (mut scatters, mut empty) = (0, 0);
        for x in -6..6 {
            for z in -6..6 {
                let plan = plan_sector(&fields, config.input(SectorCoord::new(x, 0, z)))
                    .unwrap_or_else(|fault| panic!("[{x}, 0, {z}]: {fault}"));
                let background: Vec<&PlannedBody> = plan
                    .bodies
                    .iter()
                    .filter(|body| body.source == BodySource::Background)
                    .collect();
                if background.len() < plan.bodies.len() {
                    assert!(background.is_empty(), "{} owns a cluster body", plan.coord);
                    continue;
                }
                if background.is_empty() {
                    empty += 1;
                    continue;
                }
                scatters += 1;
                assert!(
                    (BACKGROUND_ROCKS.0..=BACKGROUND_ROCKS.1).contains(&background.len()),
                    "{} plans {} background rocks",
                    plan.coord,
                    background.len()
                );
                let summary = plan.summary();
                assert_eq!(
                    summary.background_rocks + summary.skipped_face + summary.skipped_clearance,
                    background.len(),
                    "{}",
                    plan.coord
                );
            }
        }
        assert!(
            scatters > 0 && empty > 0,
            "{scatters} scatters, {empty} empty"
        );
    }

    /// Every planetoid carries the mass that reaches [`PLANETOID_SOI_REACH`]
    /// body radii, and the default surface gravity cap leaves that mass whole.
    #[test]
    fn every_planetoid_well_reaches_its_soi_multiple_under_the_surface_cap() {
        let settings = GravitySettings::default();
        let mut planetoids = 0;
        for cluster in scanned_clusters() {
            for parent in &cluster.parents {
                let ClusterBody::Planetoid(config) = &parent.body else {
                    panic!(
                        "{}: a parent must be a planetoid",
                        Cluster::slug(cluster.node)
                    );
                };
                let mass = config.mass.expect("a planetoid must author its mass");
                let body_radius = config.body_radius().to_engine();
                let well = GravityWell::from_mass(mass, body_radius, &settings);
                assert_eq!(well.mu, mass, "the surface gravity cap clamped {mass}");
                let reach = well.soi_radius / body_radius;
                assert!(
                    (reach - PLANETOID_SOI_REACH).abs() < 1e-3,
                    "a {body_radius} u planetoid reaches {reach} body radii"
                );
                planetoids += 1;
            }
        }
        assert!(planetoids > 0, "the scan must hold planetoids");
    }
}
