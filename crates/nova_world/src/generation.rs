//! The generator: a world seed and a cell coordinate in, a validated manifest
//! out.
//!
//! PURE. Nothing here touches a `World`, reads a resource or draws from the
//! ambient RNG, which is what lets [`prepare_sector`] run on a worker and what
//! makes a cell the same cell in any visit order.
//!
//! # The feature field
//!
//! One layer per kind of place ([`FeatureLayer`]), and the layers are
//! INDEPENDENT: each has its own noise field, its own threshold and its own
//! radius band, and a planet sphere is free to sit inside an asteroid sphere.
//! Only same-layer crowding is thinned, and it is thinned by a rule that reads
//! the same from anywhere - a candidate loses to any HIGHER-PRIORITY candidate
//! of its own layer that overlaps it, whether or not that one survived its own
//! thinning. A survivor-cascade rule would make the answer depend on where the
//! walk started; this one is a pure function of the candidate set, and the
//! candidate set is a pure function of the lattice node.
//!
//! The halo searched for those rivals is FINITE and derived, not guessed:
//! two same-layer spheres can only overlap if their centres are within
//! `2 * FEATURE_RADIUS_MAX`, and a centre moves off its node by one jitter
//! draw and one inset pull, so [`FEATURE_HALO`] nodes is provably enough.
//! [`feature_halo_covers_overlap`] is the arithmetic, and the inset pull grows
//! with the cell edge, so a wide enough edge outruns the halo:
//! [`crate::WorldConfig::validate`] REFUSES such a config in every build. A
//! debug assertion would have let a release run ship a world with two belts
//! inside each other.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use bevy::prelude::*;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use nova_events::prelude::{Meters, Meters3};
use nova_gameplay::prelude::{Fnv32, SeedStream};
use nova_scenario::prelude::{
    asteroid_seed_from_id, prepare_asteroid_geometry, prepare_planet, PlanetConfig,
    PreparedAsteroidGeometry, PreparedPlanet, ASTEROID_GEOMETRIC_FACTOR_MAX,
};

use crate::{index_slug, SectorCoord, SectorFault, SectorGeneration, WorldConfig, PLACEMENT_INSET};

/// The kinds of PLACE the feature field gates, one independent noise layer
/// each.
///
/// Independent is the design, not an accident: every layer has its own field,
/// its own threshold and its own radius band, and nothing stops a planet
/// sphere from sitting inside an asteroid sphere. That overlap is what makes a
/// blended region - a rock field with a world in it - instead of a mosaic of
/// single-purpose tiles. Only SAME-layer crowding is thinned.
///
/// `Ord` is derived so the per-layer arrays and the diagnostic ordering are
/// both a fact about the enum rather than about a walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeatureLayer {
    /// A rock field: raises the body count of every cell it reaches.
    Asteroid,
    /// A world: one planetoid, in the cell its centre falls in.
    Planet,
    /// A mooring: a few neutral hulls parked with nobody aboard.
    Anchorage,
}

impl FeatureLayer {
    /// Every layer, in the order the diagnostics and the per-layer arrays
    /// read.
    pub const ALL: [Self; 3] = [Self::Asteroid, Self::Planet, Self::Anchorage];

    /// How many layers there are: the width of every per-layer array.
    pub const COUNT: usize = Self::ALL.len();

    /// The layer's slug: its noise-field domain, its id prefix, and what a
    /// readout calls it.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Asteroid => "asteroid",
            Self::Planet => "planet",
            Self::Anchorage => "anchorage",
        }
    }

    /// The layer's index into a per-layer array.
    pub const fn index(self) -> usize {
        match self {
            Self::Asteroid => 0,
            Self::Planet => 1,
            Self::Anchorage => 2,
        }
    }

    /// How high the layer's field has to read before a lattice node becomes a
    /// candidate.
    ///
    /// The three FIXED numbers the whole field is tuned on, and the only dials
    /// here chosen by looking. Against a pinned seed they put empty cells,
    /// single-layer cells and blended cells in one window, which is what makes
    /// a world read as places rather than as a mosaic.
    ///
    /// Three octaves of Perlin reads far narrower than its nominal `[-1, 1]` -
    /// these thresholds sit inside the band the field actually occupies, which
    /// is also what the crate-private `FEATURE_CEILING` is measured from.
    pub const fn threshold(self) -> f32 {
        match self {
            // The common one: rock is the background of the setting, so its
            // gate is the loosest of the three.
            Self::Asteroid => 0.02,
            // The rarest: a world is a landmark, and a landmark in every
            // second cell is scenery.
            Self::Planet => 0.16,
            // In between, and deliberately not aligned with either - an
            // anchorage that only ever appeared beside a world would be a
            // dependent layer wearing an independent one's clothes.
            Self::Anchorage => 0.14,
        }
    }

    /// The layer's radius band. A sphere's radius is drawn across it from the
    /// node's own jitter stream, so size is decided with position rather than
    /// after it.
    pub const fn radius_band(self) -> (Meters, Meters) {
        match self {
            // The widest, and wider than a lattice cell: a belt has to be able
            // to cover several sectors or a "field" is one cell with rocks in
            // it.
            Self::Asteroid => (Meters(48_000.0), Meters(96_000.0)),
            Self::Planet => (Meters(48_000.0), Meters(80_000.0)),
            // The tightest: a mooring is a place, not a region.
            Self::Anchorage => (Meters(32_000.0), Meters(64_000.0)),
        }
    }
}

impl std::fmt::Display for FeatureLayer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.slug())
    }
}

/// The spacing of the candidate lattice: one candidate per layer per node.
///
/// Four sector edges at the selected 32 km cell. Coarse enough that a feature
/// is a REGION rather than a cell's decoration, and fine enough that a 160 km
/// active window contains several nodes, so a hand-flown crossing walks into
/// and out of features instead of living inside one.
pub const FEATURE_LATTICE: Meters = Meters(128_000.0);

/// The macro wavelength of every layer's field: how far apart two independent
/// readings of the same layer are.
///
/// Two lattice spacings, so a field decides REGIONS of candidate nodes
/// together - a run of empty nodes, then a run of accepted ones - rather than
/// flipping a coin at each node. A wavelength at or below the lattice would
/// make the noise a second random number per node and the field would buy
/// nothing over a hash.
pub const FEATURE_WAVELENGTH: Meters = Meters(256_000.0);

/// How many octaves each layer's field is built from.
///
/// Three: enough that a region has an interior and a ragged edge, few enough
/// that the finest octave is still 64 km and so still larger than a sector.
const FEATURE_OCTAVES: usize = 3;

/// How far a candidate's centre may be jittered off its lattice node, as a
/// fraction of the half-spacing.
///
/// Jitter is what stops accepted features from reading as a grid. It is
/// bounded rather than free because the thinning halo is derived from it: a
/// centre that could wander a whole cell would need a wider halo to be sure
/// nothing outside it could overlap.
const FEATURE_JITTER: f32 = 0.45;

/// The largest radius any layer draws. The overlap reach the thinning halo is
/// sized from.
const FEATURE_RADIUS_MAX: Meters = Meters(96_000.0);

/// Where the field is READ from, relative to the lattice.
///
/// Not decoration and not a seed: Perlin is identically zero on its own
/// integer grid, and [`FEATURE_WAVELENGTH`] is exactly two lattice spacings,
/// so every node with all-even indices - half the lattice, including
/// `(0, 0, 0)` - would sample a gridpoint and read exactly `0.0` on all three
/// layers. That is a structural hole, not a rare seed: it would reject half
/// the world's candidate nodes whatever the thresholds said. The offset is a
/// fixed displacement that is a multiple of neither the lattice nor the
/// wavelength, so no node lands on a gridpoint.
const FEATURE_FIELD_ORIGIN: Meters3 = Meters3::new(91_000.0, 57_000.0, 131_000.0);

/// The field reading a layer's [`FeatureLayer::threshold`] is normalized
/// against: the value that counts as full strength.
///
/// Measured off the field rather than assumed. Three octaves at persistence
/// 0.5 sum to a value well inside the `[-1, 1]` a single Perlin octave spans -
/// over the 343 nodes within three lattice spacings of the origin the highest
/// any layer reads is about 0.57, and a typical accepted node reads 0.2-0.35.
/// Normalizing a margin against 1.0 instead would make every sphere weak,
/// every cell round down to no rocks, and the whole field invisible.
pub(crate) const FEATURE_CEILING: f32 = 0.22;

/// How many lattice nodes out the same-layer thinning looks.
///
/// DERIVED, not chosen: see the crate-private `feature_halo_covers_overlap`.
/// Two same-layer spheres overlap only if their centres are within
/// `2 * FEATURE_RADIUS_MAX`, and a centre sits within one jitter draw plus one
/// inset pull of its node, so a node further out than this cannot hold a
/// rival. The inset pull grows with the cell edge, which is why a layered
/// [`crate::WorldConfig`] with too wide an edge is refused rather than thinned
/// against a halo that no longer reaches.
pub const FEATURE_HALO: i32 = 2;

/// Whether [`FEATURE_HALO`] really covers every node that can hold an
/// overlapping same-layer rival.
///
/// The arithmetic the halo constant is derived from, written as a function so
/// it is checked rather than asserted in a comment.
/// [`crate::WorldConfig::validate`] calls it for every layered config in every
/// build, so raising a radius band or the jitter without widening the halo
/// refuses at the config instead of silently letting two belts overlap, and an
/// authored cell edge too wide for the halo is refused before a cell is
/// described.
pub(crate) fn feature_halo_covers_overlap(edge: Meters) -> bool {
    // A centre leaves its node twice: the jitter draw, and then the pull onto
    // its owner cell's inset, which can move it another `1 - PLACEMENT_INSET`
    // of a half-edge on an axis.
    let drift =
        FEATURE_JITTER * FEATURE_LATTICE.get() * 0.5 + (1.0 - PLACEMENT_INSET) * edge.get() * 0.5;
    let reach = 2.0 * FEATURE_RADIUS_MAX.get() + 2.0 * drift;
    // A node the halo does NOT hold is at least `FEATURE_HALO + 1` spacings
    // away on some axis, and a separation on one axis is a lower bound on the
    // distance between the centres.
    reach <= (FEATURE_HALO + 1) as f32 * FEATURE_LATTICE.get()
}

/// The three global noise fields, one per layer, assembled once and then
/// sampled.
///
/// `Fbm::new` seeds a permutation table per octave, so rebuilding the graph
/// per sample would cost far more than sampling it. One set per
/// [`generate_sector`] call, which is once per sector per worker.
pub struct FeatureFields([Fbm<Perlin>; FeatureLayer::COUNT]);

impl FeatureFields {
    /// The fields this world seed opens.
    pub fn new(world_seed: u32) -> Self {
        let field = |layer: FeatureLayer| {
            // `build_sources` seeds octave `n` with `seed + n`, so a seed
            // within `octaves` of the ceiling overflows in any build with
            // overflow checks. A layer seed comes off a hash and is uniform
            // over the whole range, so this is reachable by an unlucky world
            // seed rather than only by a test poking `u32::MAX`.
            let seed = Fnv32::new()
                .write(&world_seed.to_le_bytes())
                .write(b"feature_field")
                .write(layer.slug().as_bytes())
                .finish()
                % (u32::MAX - FEATURE_OCTAVES as u32);
            Fbm::<Perlin>::new(seed)
                .set_frequency(1.0 / f64::from(FEATURE_WAVELENGTH.get()))
                .set_octaves(FEATURE_OCTAVES)
        };
        Self(FeatureLayer::ALL.map(field))
    }

    /// What `layer`'s RAW field reads at `position`.
    ///
    /// The unthinned, ungated noise value, not a strength and not a density: a
    /// reading at or below [`FeatureLayer::threshold`] gates no candidate at
    /// all, and a reading above it is only a candidate until the thinning has
    /// looked at its rivals.
    ///
    /// # Errors
    ///
    /// [`SectorFault::Noise`] when the field returns a non-finite reading. A
    /// `NaN` compared against a threshold is silently false, which would make
    /// a whole region of the world quietly empty.
    pub fn sample(&self, layer: FeatureLayer, position: Meters3) -> Result<f32, SectorFault> {
        let point = (position + FEATURE_FIELD_ORIGIN).get();
        let value =
            self.0[layer.index()].get([f64::from(point.x), f64::from(point.y), f64::from(point.z)])
                as f32;
        if !value.is_finite() {
            let at = position.get();
            return Err(SectorFault::Noise {
                layer,
                at: format!("{:.0} {:.0} {:.0} m", at.x, at.y, at.z),
            });
        }
        Ok(value)
    }

    /// How far above its threshold `value` reads, in `[0, 1]`.
    ///
    /// Zero at and below the gate, one at the measured ceiling. The same
    /// normalization a candidate's [`FeatureSphere::strength`] is taken
    /// through, exposed so a diagnostic can colour a raw reading the way the
    /// generator weighs it.
    pub fn strength_of(layer: FeatureLayer, value: f32) -> f32 {
        let threshold = layer.threshold();
        if !value.is_finite() || value <= threshold {
            return 0.0;
        }
        ((value - threshold) / (FEATURE_CEILING - threshold)).clamp(0.0, 1.0)
    }
}

/// One accepted feature: a sphere of influence over the world, and PURE DATA.
///
/// It is not an entity, it owns no geometry, and nothing about it depends on
/// which cell asked. Two neighbouring sectors that both overlap one sphere are
/// handed the same `id`, `centre`, `radius` and `strength`, because all four
/// are functions of the lattice node alone - which is what lets a belt cross a
/// sector boundary without either side owning half of it.
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureSphere {
    /// The sphere's stable id: its layer and its lattice node, never its
    /// sector. A sphere keeps this id from every cell that can see it.
    pub id: String,
    /// Which layer gated it.
    pub layer: FeatureLayer,
    /// The one cell that MATERIALIZES what this sphere places: the cell its
    /// centre falls in. Every other cell the sphere reaches reads it and
    /// spawns nothing for it.
    pub owner: SectorCoord,
    /// Where the sphere is centred, in meters from the world origin.
    pub centre: Meters3,
    /// How far its influence reaches.
    pub radius: Meters,
    /// How hard the field read above the layer's threshold, in `[0, 1]`.
    /// Drives the body count an asteroid sphere asks for and the priority the
    /// thinning ranks by.
    pub strength: f32,
}

impl FeatureSphere {
    /// How strongly the sphere influences `position`: `strength` at the
    /// centre, falling smoothly to nothing at the rim.
    ///
    /// A squared inverse-quadratic, not a linear ramp: a belt with a hard
    /// linear edge shows the sphere, and a sphere the player can see the shape
    /// of is a tile with extra steps.
    pub fn influence(&self, position: Meters3) -> f32 {
        let reach = self.radius.get();
        if reach <= 0.0 {
            return 0.0;
        }
        let t = self.centre.distance(position).get() / reach;
        if t >= 1.0 {
            return 0.0;
        }
        let falloff = 1.0 - t * t;
        self.strength * falloff * falloff
    }

    /// Whether the sphere reaches any part of the axis-aligned box `centre`
    /// and `half_edge` describe.
    fn reaches_box(&self, centre: Meters3, half_edge: Meters) -> bool {
        let delta = (self.centre.get() - centre.get()).abs() - Vec3::splat(half_edge.get());
        let outside = delta.max(Vec3::ZERO);
        outside.length() <= self.radius.get()
    }

    /// Whether two spheres of one layer claim the same ground.
    fn overlaps(&self, other: &Self) -> bool {
        self.centre.distance(other.centre) < self.radius + other.radius
    }

    /// Whether `self` beats `other` for the ground they share.
    ///
    /// Strength first, then the id, so the answer is total and is a fact about
    /// the pair rather than about which one was generated first.
    fn outranks(&self, other: &Self) -> bool {
        match self.strength.partial_cmp(&other.strength) {
            Some(std::cmp::Ordering::Greater) => true,
            Some(std::cmp::Ordering::Less) | None => false,
            Some(std::cmp::Ordering::Equal) => self.id < other.id,
        }
    }

    /// Refuse a sphere that cannot be drawn or reasoned about.
    fn validate(&self, edge: Meters) -> Result<(), SectorFault> {
        let refuse = |field: &'static str, value: String| {
            Err(SectorFault::Feature {
                id: self.id.clone(),
                field,
                value,
            })
        };
        if !position_is_finite(self.centre) {
            return refuse("centre", format!("{:?}", self.centre.get()));
        }
        if !self.radius.get().is_finite() || self.radius.get() <= 0.0 {
            return refuse("radius", format!("{} m", self.radius.get()));
        }
        if !self.strength.is_finite() || !(0.0..=1.0).contains(&self.strength) {
            return refuse("strength", self.strength.to_string());
        }
        let owner = SectorCoord::containing(self.centre, edge);
        if owner != self.owner {
            return refuse("owner", format!("{owner}, not the recorded {}", self.owner));
        }
        Ok(())
    }
}

/// One generated rock, in world meters.
#[derive(Clone, Debug, PartialEq)]
pub struct SectorAsteroid {
    /// The rock's scenario id, prefixed with its owning cell's slug.
    pub id: String,
    /// Where it stands, in meters from the world origin.
    pub position: Meters3,
    /// Its nominal radius.
    pub radius: Meters,
    /// Its asteroid kind id, drawn from the config's table.
    pub kind: String,
    /// Its silhouette seed.
    pub seed: u32,
}

/// One generated planetoid: a real [`PlanetConfig`], not a big rock.
#[derive(Clone, Debug)]
pub struct SectorPlanet {
    /// The planetoid's scenario id, prefixed with its owning cell's slug.
    pub id: String,
    /// The planet sphere that placed it.
    pub feature: String,
    /// Where it stands: the sphere's centre, which is inside this cell's inset
    /// by construction.
    pub position: Meters3,
    /// The world it is, ready for `prepare_planet`.
    pub config: PlanetConfig,
}

/// One moored hull: a neutral ship with nobody aboard.
#[derive(Clone, Debug)]
pub struct SectorAnchorage {
    /// The hull's scenario id, prefixed with its owning cell's slug.
    pub id: String,
    /// The anchorage sphere that moored it.
    pub feature: String,
    /// Where it floats, in meters from the world origin.
    pub position: Meters3,
    /// Which way it is pointing. Yaw only - a moored hull sits level.
    pub yaw: f32,
    /// The catalog design it is built from, from the config.
    pub design: String,
}

/// One sector's validated contents: what `materialize_sector` is allowed to
/// spawn, and the feature data that explains it.
///
/// Only [`generate_sector`] builds one, and it only returns one that passed
/// every check. A description in hand IS the readiness gate.
#[derive(Clone, Debug)]
pub struct SectorDescription {
    /// The cell described.
    pub coord: SectorCoord,
    /// Every feature sphere that reaches this cell, whoever owns it, ordered
    /// by layer and then id. Empty under
    /// [`SectorGeneration::UniformAsteroids`].
    pub features: Vec<FeatureSphere>,
    /// The combined influence of each layer at the cell's centre, indexed by
    /// [`FeatureLayer::index`].
    pub strengths: [f32; FeatureLayer::COUNT],
    /// The rocks, in generation order.
    pub asteroids: Vec<SectorAsteroid>,
    /// The planetoids this cell OWNS, in generation order.
    pub planets: Vec<SectorPlanet>,
    /// The hulls this cell OWNS, in generation order.
    pub anchorages: Vec<SectorAnchorage>,
}

impl SectorDescription {
    /// Every object id this cell spawns, in spawn order: rocks, then
    /// planetoids, then hulls.
    pub fn object_ids(&self) -> Vec<String> {
        self.asteroids
            .iter()
            .map(|body| body.id.clone())
            .chain(self.planets.iter().map(|planet| planet.id.clone()))
            .chain(self.anchorages.iter().map(|hull| hull.id.clone()))
            .collect()
    }

    /// How many entities the cell's root will own.
    pub fn object_count(&self) -> usize {
        self.asteroids.len() + self.planets.len() + self.anchorages.len()
    }

    /// The description as one comparable block of text.
    ///
    /// What "the same sector" MEANS, written down: two generations are the
    /// same when this matches. Positions are printed at centimeter resolution
    /// rather than compared as floats, so the comparison is a fact about the
    /// generator and not about the last bit of an f32.
    pub fn canonical(&self) -> String {
        let point = |position: Meters3| {
            let p = position.get();
            format!("{:.2} {:.2} {:.2}", p.x, p.y, p.z)
        };
        let mut out = format!("{}\n", self.coord);
        for sphere in &self.features {
            out.push_str(&format!(
                "feature {} {} {} {} r{:.2} s{:.4}\n",
                sphere.id,
                sphere.layer,
                sphere.owner,
                point(sphere.centre),
                sphere.radius.get(),
                sphere.strength
            ));
        }
        for layer in FeatureLayer::ALL {
            out.push_str(&format!(
                "strength {layer} {:.4}\n",
                self.strengths[layer.index()]
            ));
        }
        for body in &self.asteroids {
            out.push_str(&format!(
                "asteroid {} {} r{:.2} {} s{}\n",
                body.id,
                point(body.position),
                body.radius.get(),
                body.kind,
                body.seed
            ));
        }
        for planet in &self.planets {
            out.push_str(&format!(
                "planet {} {} {} {:?} r{:.2} s{}\n",
                planet.id,
                planet.feature,
                point(planet.position),
                planet.config.planet_type,
                planet.config.radius.get(),
                planet.config.seed
            ));
        }
        for hull in &self.anchorages {
            out.push_str(&format!(
                "hull {} {} {} y{:.4} {}\n",
                hull.id,
                hull.feature,
                point(hull.position),
                hull.yaw,
                hull.design
            ));
        }
        out
    }
}

/// One sector described, validated and meshed: everything
/// `materialize_sector` needs that a worker can produce.
///
/// Built only by [`prepare_sector`], so `asteroid_geometry` is one prepared
/// rock per `description.asteroids` entry and `planet_surfaces` one prepared
/// world per `description.planets` entry, both in order. A moored hull needs
/// no preparation: its sections are resolved from the catalog on the main
/// thread, which is the one place the catalog exists.
#[derive(Debug)]
pub struct PreparedSector {
    /// The validated description this was prepared from.
    pub description: SectorDescription,
    /// One prepared rock per asteroid, in `description.asteroids` order.
    pub asteroid_geometry: Vec<PreparedAsteroidGeometry>,
    /// One prepared world per planetoid, in `description.planets` order.
    pub planet_surfaces: Vec<PreparedPlanet>,
}

/// The most rocks a featured cell generates, at full asteroid influence.
const MAX_SECTOR_ASTEROIDS: usize = 4;

/// The nominal radius band a featured cell draws rocks from.
///
/// The meshed rock reaches 3.5-6x past the nominal figure, so 30-60 m nominal
/// draws about 210-720 m diameters - scattered landmarks in a 32 km cell.
const FEATURE_ASTEROID_RADIUS: (Meters, Meters) = (Meters(30.0), Meters(60.0));

/// The mean-radius band a gated planetoid is drawn from.
///
/// 600-1,200 m: big enough to read as a WORLD against a 60 m rock beside it,
/// small enough that a 32 km cell is still a place you fly across rather than
/// a place a single body fills. A planet's radius is its real size, not a
/// designation, so this is what it draws.
const PLANETOID_RADIUS: (Meters, Meters) = (Meters(600.0), Meters(1_200.0));

/// How many hulls one anchorage sphere moors.
const ANCHORAGE_HULLS: (usize, usize) = (1, 3);

/// How far a moored hull may drift from its anchorage sphere's centre.
///
/// A CLUSTER radius, not a scatter: an anchorage reads as a place because its
/// hulls are near each other. Wide enough that the hulls still clear a
/// planetoid sharing the cell without exhausting their candidates.
const ANCHORAGE_MOOR: Meters = Meters(6_000.0);

/// How much room a moored hull claims for clearance.
///
/// A radius around the hull root, not a measured bound: a hull's sections are
/// resolved from the catalog on the main thread, and a worker deciding where
/// it stands cannot see them. Generous on purpose.
pub const MOORED_HULL_CLEARANCE: Meters = Meters(400.0);

/// Extra room every pair of objects keeps between their clearance radii.
///
/// A sector whose bodies merely fail to intersect still reads as a pile. The
/// margin is what makes a generated cell look placed.
pub const CLEARANCE_MARGIN: Meters = Meters(500.0);

/// How many deterministic candidates an object gets before the sector refuses.
///
/// A cap rather than a loop until it fits: the draw is deterministic, so if
/// the cell really has no room the search does not terminate on its own, and
/// a sector that silently dropped the object would hide a field asking for
/// more than a cell can hold.
const PLACEMENT_ATTEMPTS: usize = 64;

/// One object already standing in the cell, and how much room it claims.
#[derive(Clone, Copy, Debug)]
struct Occupied {
    centre: Meters3,
    clearance: Meters,
}

/// The seed for one named domain of one cell.
///
/// Coordinate-derived and purpose-separated, so visit order cannot reach it
/// and adding a second domain later cannot move the bodies this one placed.
/// `SeedStream` is then walked in a fixed order within the sector, which is
/// the only ordering the result depends on.
fn sector_seed(world_seed: u32, coord: SectorCoord, purpose: &str) -> u32 {
    Fnv32::new()
        .write(&world_seed.to_le_bytes())
        .write(&coord.x.to_le_bytes())
        .write(&coord.y.to_le_bytes())
        .write(&coord.z.to_le_bytes())
        .write(purpose.as_bytes())
        .finish()
}

/// The seed for one lattice node of one layer. The node's jitter, radius and
/// contents all come off this, so a sphere is the same sphere from every cell
/// that can see it.
fn node_seed(world_seed: u32, layer: FeatureLayer, node: [i32; 3]) -> u32 {
    Fnv32::new()
        .write(&world_seed.to_le_bytes())
        .write(b"feature_node")
        .write(layer.slug().as_bytes())
        .write(&node[0].to_le_bytes())
        .write(&node[1].to_le_bytes())
        .write(&node[2].to_le_bytes())
        .finish()
}

fn position_is_finite(position: Meters3) -> bool {
    position.get().is_finite()
}

/// The candidate `layer` gates at `node`, before same-layer thinning.
///
/// `Ok(None)` is the ordinary answer: the field did not read high enough
/// there. The centre is jittered off the node and then pulled onto its OWNER's
/// placement inset - one continuous map, not a clamp after the fact - because
/// a planet sphere places a real body at its centre and that body has to stand
/// inside the cell that owns it. The pull is a cell-scale correction on a
/// lattice-scale jitter, so it moves where a sphere sits inside its cell and
/// not which region of the world it belongs to.
fn feature_candidate(
    fields: &FeatureFields,
    world_seed: u32,
    layer: FeatureLayer,
    node: [i32; 3],
    edge: Meters,
) -> Result<Option<FeatureSphere>, SectorFault> {
    let lattice = FEATURE_LATTICE.get();
    let node_point = Meters3::new(
        node[0] as f32 * lattice,
        node[1] as f32 * lattice,
        node[2] as f32 * lattice,
    );
    if !position_is_finite(node_point) {
        return Ok(None);
    }

    let value = fields.sample(layer, node_point)?;
    if value <= layer.threshold() {
        return Ok(None);
    }
    let strength = FeatureFields::strength_of(layer, value);

    let mut stream = SeedStream::new(node_seed(world_seed, layer, node));
    let jitter = FEATURE_JITTER * lattice * 0.5;
    let drawn = node_point
        + Meters3::new(
            stream.signed() * jitter,
            stream.signed() * jitter,
            stream.signed() * jitter,
        );
    let owner = SectorCoord::containing(drawn, edge);
    let owner_centre = owner.centre(edge);
    let centre = owner_centre + (drawn - owner_centre) * PLACEMENT_INSET;

    let (radius_min, radius_max) = layer.radius_band();
    let radius = radius_min + (radius_max - radius_min) * stream.unit();

    let sphere = FeatureSphere {
        id: format!(
            "feature_{}_{}_{}_{}",
            layer.slug(),
            index_slug(node[0]),
            index_slug(node[1]),
            index_slug(node[2])
        ),
        layer,
        owner,
        centre,
        radius,
        strength,
    };
    sphere.validate(edge)?;
    Ok(Some(sphere))
}

/// The candidate at `node`, if it survives same-layer thinning.
///
/// A candidate loses to ANY higher-priority same-layer candidate in the halo
/// that overlaps it, survivor or not. That is deliberately not a cascade: a
/// cascade's answer depends on the order rivals are resolved in, and the whole
/// point of a coordinate-addressable field is that no cell's walk can change
/// what another cell sees.
fn thinned_candidate(
    fields: &FeatureFields,
    world_seed: u32,
    layer: FeatureLayer,
    node: [i32; 3],
    edge: Meters,
    cache: &mut BTreeMap<(FeatureLayer, [i32; 3]), Option<FeatureSphere>>,
) -> Result<Option<FeatureSphere>, SectorFault> {
    let Some(candidate) = cached_candidate(fields, world_seed, layer, node, edge, cache)?.clone()
    else {
        return Ok(None);
    };

    for dx in -FEATURE_HALO..=FEATURE_HALO {
        for dy in -FEATURE_HALO..=FEATURE_HALO {
            for dz in -FEATURE_HALO..=FEATURE_HALO {
                if (dx, dy, dz) == (0, 0, 0) {
                    continue;
                }
                let Some(neighbour) = node[0]
                    .checked_add(dx)
                    .zip(node[1].checked_add(dy))
                    .zip(node[2].checked_add(dz))
                    .map(|((x, y), z)| [x, y, z])
                else {
                    continue;
                };
                let rival = cached_candidate(fields, world_seed, layer, neighbour, edge, cache)?;
                if let Some(rival) = rival {
                    if rival.outranks(&candidate) && rival.overlaps(&candidate) {
                        return Ok(None);
                    }
                }
            }
        }
    }
    Ok(Some(candidate))
}

/// [`feature_candidate`], memoized for one [`generate_sector`] call.
///
/// The halo makes each node's candidate asked for up to `(2 * FEATURE_HALO +
/// 1)^3` times. Sampling three octaves of Perlin that many times per node is
/// the whole cost of the featured generator, and the cache is what keeps a
/// sector's field work in the same order as its geometry work.
fn cached_candidate<'a>(
    fields: &FeatureFields,
    world_seed: u32,
    layer: FeatureLayer,
    node: [i32; 3],
    edge: Meters,
    cache: &'a mut BTreeMap<(FeatureLayer, [i32; 3]), Option<FeatureSphere>>,
) -> Result<&'a Option<FeatureSphere>, SectorFault> {
    match cache.entry((layer, node)) {
        Entry::Occupied(slot) => Ok(slot.into_mut()),
        Entry::Vacant(slot) => {
            let candidate = feature_candidate(fields, world_seed, layer, node, edge)?;
            Ok(slot.insert(candidate))
        }
    }
}

/// Every accepted feature sphere that reaches `coord`, ordered by layer and
/// then id.
///
/// The cell asks the FIELD, not its neighbours: the node range is derived from
/// the cell's own box and the widest radius any layer draws, so two adjacent
/// cells that both overlap one sphere are handed the same sphere rather than
/// two views of it.
///
/// # Errors
///
/// Whatever [`WorldConfig::validate`], the field or a candidate refuses, plus
/// [`SectorFault::DuplicateFeature`] if two spheres ever claim one id.
pub fn sector_features(
    config: &WorldConfig,
    coord: SectorCoord,
) -> Result<Vec<FeatureSphere>, SectorFault> {
    config.validate()?;
    let fields = FeatureFields::new(config.seed);
    sector_features_from(&fields, config.seed, coord, config.sector_edge)
}

fn sector_features_from(
    fields: &FeatureFields,
    world_seed: u32,
    coord: SectorCoord,
    edge: Meters,
) -> Result<Vec<FeatureSphere>, SectorFault> {
    let centre = coord.centre(edge);
    let half_edge = Meters(edge.get() * 0.5);
    // The furthest a node can be and still reach this cell: the widest radius,
    // the cell's own corner, and both ways a centre leaves its node - the
    // jitter draw and the pull onto its owner cell's inset. Miss the pull and
    // a sphere one cell can see is invisible to its neighbour.
    let reach = f64::from(FEATURE_RADIUS_MAX.get())
        + f64::from(half_edge.get()) * 3.0_f64.sqrt()
        + f64::from(FEATURE_JITTER * FEATURE_LATTICE.get() * 0.5)
        + f64::from((1.0 - PLACEMENT_INSET) * half_edge.get());
    let lattice = f64::from(FEATURE_LATTICE.get());

    let bounds = |axis: f32| -> Option<(i32, i32)> {
        let axis = f64::from(axis);
        if !axis.is_finite() {
            return None;
        }
        let low = ((axis - reach) / lattice).floor();
        let high = ((axis + reach) / lattice).ceil();
        if !low.is_finite() || !high.is_finite() {
            return None;
        }
        Some((
            low.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32,
            high.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32,
        ))
    };
    let point = centre.get();
    let (Some(x), Some(y), Some(z)) = (bounds(point.x), bounds(point.y), bounds(point.z)) else {
        return Err(SectorFault::InvalidGeometry { id: coord.slug() });
    };

    let mut cache = BTreeMap::new();
    let mut claimed = BTreeSet::new();
    let mut spheres = Vec::new();
    for layer in FeatureLayer::ALL {
        for i in x.0..=x.1 {
            for j in y.0..=y.1 {
                for k in z.0..=z.1 {
                    let Some(sphere) =
                        thinned_candidate(fields, world_seed, layer, [i, j, k], edge, &mut cache)?
                    else {
                        continue;
                    };
                    if !sphere.reaches_box(centre, half_edge) {
                        continue;
                    }
                    if !claimed.insert(sphere.id.clone()) {
                        return Err(SectorFault::DuplicateFeature { id: sphere.id });
                    }
                    spheres.push(sphere);
                }
            }
        }
    }
    // Layer order comes from the outer loop; the node loops are already
    // lexicographic, so this is a total order without a sort.
    Ok(spheres)
}

/// Draw a clear place for one object inside `coord`'s inset.
///
/// Candidates come off `stream` in a fixed order, so the placement is a
/// function of the cell and not of the wall clock: `anchor` is what the object
/// wants to be near (the cell's centre for a rock, an anchorage sphere's
/// centre for a hull) and `reach` how far it may drift from it. A candidate is
/// taken only if it is inside the cell's inset AND clears everything already
/// standing.
#[expect(
    clippy::too_many_arguments,
    reason = "every argument is a distinct placement input; bundling them into a struct would name the same seven values twice"
)]
fn place_object(
    stream: &mut SeedStream,
    id: &str,
    coord: SectorCoord,
    edge: Meters,
    anchor: Meters3,
    reach: Meters,
    clearance: Meters,
    standing: &[Occupied],
) -> Result<Meters3, SectorFault> {
    let inset = edge.get() * 0.5 * PLACEMENT_INSET;
    let cell_centre = coord.centre(edge);

    for _ in 0..PLACEMENT_ATTEMPTS {
        let candidate = anchor
            + Meters3::new(
                stream.signed() * reach.get(),
                stream.signed() * reach.get(),
                stream.signed() * reach.get(),
            );
        if !position_is_finite(candidate) {
            return Err(SectorFault::InvalidGeometry { id: id.to_string() });
        }
        let offset = (candidate.get() - cell_centre.get()).abs();
        if offset.max_element() > inset {
            continue;
        }
        if standing.iter().any(|other| {
            other.centre.distance(candidate) < other.clearance + clearance + CLEARANCE_MARGIN
        }) {
            continue;
        }
        return Ok(candidate);
    }
    Err(SectorFault::Clearance {
        id: id.to_string(),
        attempts: PLACEMENT_ATTEMPTS,
    })
}

/// Take an object id for this sector, or refuse it to a second claimant.
///
/// Never resolved by spawn order: an id is how an object is found again, and
/// two of them is a sector nobody can name their way back into.
fn claim_id(id: String, claimed: &mut BTreeSet<String>) -> Result<String, SectorFault> {
    if !claimed.insert(id.clone()) {
        return Err(SectorFault::DuplicateId { id });
    }
    Ok(id)
}

/// The rocks, planetoids and hulls a layered cell holds.
type LayeredContents = (
    Vec<FeatureSphere>,
    [f32; FeatureLayer::COUNT],
    Vec<SectorPlanet>,
    Vec<SectorAnchorage>,
    usize,
);

/// Describe what the feature field puts in one cell.
fn layered_contents(
    config: &WorldConfig,
    layered: &crate::LayeredFeatureConfig,
    coord: SectorCoord,
    claimed: &mut BTreeSet<String>,
    standing: &mut Vec<Occupied>,
) -> Result<LayeredContents, SectorFault> {
    let edge = config.sector_edge;
    let centre = coord.centre(edge);
    let fields = FeatureFields::new(config.seed);
    let features = sector_features_from(&fields, config.seed, coord, edge)?;
    let mut strengths = [0.0; FeatureLayer::COUNT];
    for sphere in &features {
        strengths[sphere.layer.index()] += sphere.influence(centre);
    }
    for strength in &mut strengths {
        *strength = strength.min(1.0);
    }

    // Planetoids first: a planet sphere places its world AT its centre, which
    // is the one pose in a cell nothing else gets to move. Everything after it
    // works around what is already there.
    let mut stream = SeedStream::new(sector_seed(config.seed, coord, "planets"));
    let mut planets = Vec::new();
    for sphere in features
        .iter()
        .filter(|sphere| sphere.layer == FeatureLayer::Planet && sphere.owner == coord)
    {
        let id = claim_id(
            format!("{}_planet_{}", coord.slug(), planets.len()),
            claimed,
        )?;
        let (radius_min, radius_max) = PLANETOID_RADIUS;
        let radius = radius_min + (radius_max - radius_min) * stream.unit();
        let planet_type =
            layered.planet_types[stream.next_u32() as usize % layered.planet_types.len()];
        let config = PlanetConfig::new(planet_type, radius, stream.next_u32());
        if !position_is_finite(sphere.centre) || !radius.get().is_finite() {
            return Err(SectorFault::InvalidGeometry { id });
        }
        let clearance = config.body_radius();
        if standing.iter().any(|other| {
            other.centre.distance(sphere.centre) < other.clearance + clearance + CLEARANCE_MARGIN
        }) {
            return Err(SectorFault::Clearance { id, attempts: 1 });
        }
        standing.push(Occupied {
            centre: sphere.centre,
            clearance,
        });
        planets.push(SectorPlanet {
            id,
            feature: sphere.id.clone(),
            position: sphere.centre,
            config,
        });
    }

    let mut stream = SeedStream::new(sector_seed(config.seed, coord, "anchorages"));
    let mut anchorages = Vec::new();
    for sphere in features
        .iter()
        .filter(|sphere| sphere.layer == FeatureLayer::Anchorage && sphere.owner == coord)
    {
        let (hulls_min, hulls_max) = ANCHORAGE_HULLS;
        let span = hulls_max - hulls_min + 1;
        let hulls = hulls_min + stream.next_u32() as usize % span;
        for _ in 0..hulls {
            let id = claim_id(
                format!("{}_hull_{}", coord.slug(), anchorages.len()),
                claimed,
            )?;
            let position = place_object(
                &mut stream,
                &id,
                coord,
                edge,
                sphere.centre,
                ANCHORAGE_MOOR,
                MOORED_HULL_CLEARANCE,
                standing,
            )?;
            standing.push(Occupied {
                centre: position,
                clearance: MOORED_HULL_CLEARANCE,
            });
            anchorages.push(SectorAnchorage {
                id,
                feature: sphere.id.clone(),
                position,
                yaw: stream.unit() * std::f32::consts::TAU,
                design: layered.anchorage_design.clone(),
            });
        }
    }

    // Rocks come off the COMBINED asteroid influence AT THE CELL CENTRE, so
    // two spheres covering that one point fill the cell more than either would
    // alone. A sphere contributes nothing to a cell whose centre it does not
    // reach - including a fringe cell it only clips a corner of, which
    // `sector_features_from` still lists among the cell's spheres because
    // `reaches_box` tests the whole box.
    // CEILING, not rounding, over the cells that ARE covered: one whose centre
    // a belt reaches at all holds at least one rock. Rounding put a whole
    // outer shell of covered cells at zero rocks, so the belt had a hard edge
    // one cell inside its own rim and the falloff bought nothing.
    let count =
        (strengths[FeatureLayer::Asteroid.index()] * MAX_SECTOR_ASTEROIDS as f32).ceil() as usize;
    Ok((
        features,
        strengths,
        planets,
        anchorages,
        count.min(MAX_SECTOR_ASTEROIDS),
    ))
}

/// Describe one cell. PURE: the same config and coordinate give the same
/// description, on any call, in any order, with nothing live.
///
/// # Errors
///
/// Every [`SectorFault`] a worker can raise: [`SectorFault::Config`] and
/// [`SectorFault::UnknownKind`] for a config that cannot describe a sector,
/// [`SectorFault::Noise`] and [`SectorFault::Feature`] from the feature field,
/// [`SectorFault::DuplicateId`] and [`SectorFault::DuplicateFeature`] when two
/// things claim one id, [`SectorFault::InvalidGeometry`] when finite inputs
/// overflow while deriving a pose, and [`SectorFault::Clearance`] when a cell
/// has no room left. All are refusals before anything spawns.
pub fn generate_sector(
    config: &WorldConfig,
    coord: SectorCoord,
) -> Result<SectorDescription, SectorFault> {
    config.validate()?;
    let edge = config.sector_edge;
    let centre = coord.centre(edge);
    let mut claimed = BTreeSet::new();
    let mut standing: Vec<Occupied> = Vec::new();
    let (features, strengths, planets, anchorages, asteroid_count, asteroid_radius, kinds) =
        match &config.generation {
            SectorGeneration::UniformAsteroids(uniform) => (
                Vec::new(),
                [0.0; FeatureLayer::COUNT],
                Vec::new(),
                Vec::new(),
                uniform.body_count,
                (uniform.radius_min, uniform.radius_max),
                &uniform.asteroid_kinds,
            ),
            SectorGeneration::LayeredFeatures(layered) => {
                let (features, strengths, planets, anchorages, count) =
                    layered_contents(config, layered, coord, &mut claimed, &mut standing)?;
                (
                    features,
                    strengths,
                    planets,
                    anchorages,
                    count,
                    FEATURE_ASTEROID_RADIUS,
                    &layered.asteroid_kinds,
                )
            }
        };

    let mut stream = SeedStream::new(sector_seed(config.seed, coord, "bodies"));
    let (radius_min, radius_max) = asteroid_radius;
    let span = radius_max - radius_min;
    let inset_reach = Meters(edge.get() * 0.5 * PLACEMENT_INSET);
    let mut asteroids = Vec::with_capacity(asteroid_count);
    for index in 0..asteroid_count {
        let id = claim_id(format!("{}_body_{index}", coord.slug()), &mut claimed)?;
        let radius = radius_min + span * stream.unit();
        if !radius.get().is_finite() {
            return Err(SectorFault::InvalidGeometry { id });
        }
        // The meshed rock reaches several times past its nominal radius, so
        // clearance is measured on the WORST case a seed can draw - a sector
        // spaced on the designation would still overlap on screen.
        let clearance = Meters(radius.get() * ASTEROID_GEOMETRIC_FACTOR_MAX);
        let position = place_object(
            &mut stream,
            &id,
            coord,
            edge,
            centre,
            inset_reach,
            clearance,
            &standing,
        )?;
        // The table was checked whole by `WorldConfig::validate`, so a draw
        // from it cannot name a kind the game does not ship.
        let kind = kinds[stream.next_u32() as usize % kinds.len()].clone();
        standing.push(Occupied {
            centre: position,
            clearance,
        });
        asteroids.push(SectorAsteroid {
            seed: asteroid_seed_from_id(&id),
            id,
            position,
            radius,
            kind,
        });
    }

    Ok(SectorDescription {
        coord,
        features,
        strengths,
        asteroids,
        planets,
        anchorages,
    })
}

/// Describe and prepare one cell: everything a sector costs except the
/// spawning. PURE, so [`crate::SectorJob`] can run it on a worker.
///
/// Takes the config by value because a job answers the question it was asked:
/// a dial changed mid-flight must not silently re-aim work already in the air.
///
/// # Errors
///
/// Whatever [`generate_sector`] refuses. A sector that cannot be described is
/// never meshed, let alone spawned.
pub fn prepare_sector(
    config: WorldConfig,
    coord: SectorCoord,
) -> Result<PreparedSector, SectorFault> {
    let description = generate_sector(&config, coord)?;
    let asteroid_geometry = description
        .asteroids
        .iter()
        .map(|body| prepare_asteroid_geometry(body.seed, body.radius))
        .collect();
    let planet_surfaces = description
        .planets
        .iter()
        .map(|planet| prepare_planet(planet.config.clone()))
        .collect();
    Ok(PreparedSector {
        description,
        asteroid_geometry,
        planet_surfaces,
    })
}
