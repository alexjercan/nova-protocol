//! The generation mechanisms: the feature field, the placement rules, the
//! untrusted [`SectorManifest`] a generator returns, and the check that turns
//! it into a trusted [`SectorDescription`] before a worker prepares it.
//!
//! PURE. Nothing here touches a `World`, reads a resource or draws from the
//! ambient RNG, which is what lets [`prepare_sector`] run on a worker and what
//! makes a cell the same cell in any visit order. A [`SectorGenerator`] is
//! held to the same rule: it gets a seed, an edge and a coordinate, and
//! nothing else.
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
//! [`validate_feature_geometry`] REFUSES such an edge in every build. A debug
//! assertion would have let a release run ship a world with two belts inside
//! each other.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use bevy::prelude::*;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use nova_events::prelude::{Meters, Meters3};
use nova_gameplay::prelude::{Fnv32, SeedStream};
use nova_scenario::prelude::{
    is_asteroid_kind, prepare_asteroid_geometry, prepare_planet, PlanetConfig, PreparedAsteroid,
    PreparedPlanet, ASTEROID_GEOMETRIC_FACTOR_MAX,
};

use crate::{
    index_slug, SectorCoord, SectorFault, SectorGenerationInput, SectorGenerator, WorldConfig,
    WorldGeometry, PLACEMENT_INSET,
};

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
    /// A derelict field: a few dead hulls adrift with nobody aboard.
    Derelict,
}

impl FeatureLayer {
    /// Every layer, in the order the diagnostics and the per-layer arrays
    /// read.
    pub const ALL: [Self; 3] = [Self::Asteroid, Self::Planet, Self::Derelict];

    /// How many layers there are: the width of every per-layer array.
    pub const COUNT: usize = Self::ALL.len();

    /// The layer's slug: its noise-field domain, its id prefix, and what a
    /// readout calls it.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Asteroid => "asteroid",
            Self::Planet => "planet",
            Self::Derelict => "derelict",
        }
    }

    /// The layer's index into a per-layer array.
    pub const fn index(self) -> usize {
        match self {
            Self::Asteroid => 0,
            Self::Planet => 1,
            Self::Derelict => 2,
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
            // In between, and deliberately not aligned with either - a
            // derelict field that only ever appeared beside a world would be a
            // dependent layer wearing an independent one's clothes.
            Self::Derelict => 0.14,
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
            // The tightest: a derelict field is a place, not a region.
            Self::Derelict => (Meters(32_000.0), Meters(64_000.0)),
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
/// rival. The inset pull grows with the cell edge, which is why
/// [`validate_feature_geometry`] refuses too wide an edge rather than thinning
/// against a halo that no longer reaches.
pub const FEATURE_HALO: i32 = 2;

/// Whether [`FEATURE_HALO`] really covers every node that can hold an
/// overlapping same-layer rival.
///
/// The arithmetic the halo constant is derived from, written as a function so
/// it is checked rather than asserted in a comment.
/// [`validate_feature_geometry`] calls it in every build, so raising a radius
/// band or the jitter without widening the halo refuses at the config instead
/// of silently letting two belts overlap, and an authored cell edge too wide
/// for the halo is refused before a cell is described.
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
/// [`sector_features`] call, which is once per sector per worker.
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
    /// Its asteroid kind id, drawn from the generator's table.
    pub kind: String,
    /// Its silhouette seed.
    pub seed: u32,
}

/// One generated planetoid: a real [`PlanetConfig`], not a big rock.
#[derive(Clone, Debug)]
pub struct SectorPlanet {
    /// The planetoid's scenario id, prefixed with its owning cell's slug.
    pub id: String,
    /// The planet sphere that placed it. The sphere is listed in the same
    /// manifest and owned by the same cell.
    pub feature: String,
    /// Where it stands, in meters from the world origin.
    pub position: Meters3,
    /// The world it is, ready for `prepare_planet`.
    pub config: PlanetConfig,
}

/// One generated ship: a hull with nobody aboard and nobody's side.
#[derive(Clone, Debug)]
pub struct SectorShip {
    /// The ship's scenario id, prefixed with its owning cell's slug.
    pub id: String,
    /// The derelict sphere that placed it. The sphere is listed in the same
    /// manifest and owned by the same cell.
    pub feature: String,
    /// Where it floats, in meters from the world origin.
    pub position: Meters3,
    /// Which way it is pointing. Yaw only - the hull sits level.
    pub yaw: f32,
    /// The catalog design it is built from.
    pub design: String,
}

/// What a [`SectorGenerator`] says one cell holds. UNTRUSTED.
///
/// Public fields, because a generator outside this crate builds it. Nothing
/// downstream reads one: [`validate_manifest`] checks it against every rule
/// materialization relies on and only then hands back a
/// [`SectorDescription`].
#[derive(Clone, Debug)]
pub struct SectorManifest {
    /// The cell described.
    pub coord: SectorCoord,
    /// Every feature sphere that reaches this cell, whoever owns it. Empty for
    /// a generator that reads no feature field.
    pub features: Vec<FeatureSphere>,
    /// The combined influence of each layer at the cell's centre, indexed by
    /// [`FeatureLayer::index`].
    pub strengths: [f32; FeatureLayer::COUNT],
    /// The rocks, in generation order.
    pub asteroids: Vec<SectorAsteroid>,
    /// The planetoids this cell OWNS, in generation order.
    pub planets: Vec<SectorPlanet>,
    /// The ships this cell OWNS, in generation order.
    pub ships: Vec<SectorShip>,
}

/// One sector's contents after [`validate_manifest`] accepted them: what
/// `materialize_sector` is allowed to spawn, and the feature data that
/// explains it.
///
/// TRUSTED. The fields are private and there is no other constructor, so a
/// value of this type is proof that every rule materialization relies on
/// held, whichever generator wrote the manifest.
#[derive(Clone, Debug)]
pub struct SectorDescription {
    pub(crate) coord: SectorCoord,
    pub(crate) features: Vec<FeatureSphere>,
    pub(crate) strengths: [f32; FeatureLayer::COUNT],
    pub(crate) asteroids: Vec<SectorAsteroid>,
    pub(crate) planets: Vec<SectorPlanet>,
    pub(crate) ships: Vec<SectorShip>,
}

impl SectorDescription {
    /// The cell described.
    pub fn coord(&self) -> SectorCoord {
        self.coord
    }

    /// Every feature sphere that reaches this cell, whoever owns it.
    pub fn features(&self) -> &[FeatureSphere] {
        &self.features
    }

    /// The rocks, in generation order.
    pub fn asteroids(&self) -> &[SectorAsteroid] {
        &self.asteroids
    }

    /// The planetoids this cell owns, in generation order.
    pub fn planets(&self) -> &[SectorPlanet] {
        &self.planets
    }

    /// The ships this cell owns, in generation order.
    pub fn ships(&self) -> &[SectorShip] {
        &self.ships
    }

    /// Every object id this cell spawns, in spawn order: rocks, then
    /// planetoids, then ships.
    pub fn object_ids(&self) -> Vec<String> {
        self.asteroids
            .iter()
            .map(|body| body.id.clone())
            .chain(self.planets.iter().map(|planet| planet.id.clone()))
            .chain(self.ships.iter().map(|ship| ship.id.clone()))
            .collect()
    }

    /// How many entities the cell's root will own.
    pub fn object_count(&self) -> usize {
        self.asteroids.len() + self.planets.len() + self.ships.len()
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
        for ship in &self.ships {
            out.push_str(&format!(
                "ship {} {} {} y{:.4} {}\n",
                ship.id,
                ship.feature,
                point(ship.position),
                ship.yaw,
                ship.design
            ));
        }
        out
    }
}

/// One sector described, validated and prepared: everything
/// `materialize_sector` needs that a worker can produce.
///
/// Built only by [`prepare_sector`], and the fields are private, so
/// `asteroids` is always one prepared rock per description asteroid and
/// `planets` one prepared world per description planetoid, both in order. The
/// two halves are not the same shape: a [`PreparedAsteroid`] is the GEOMETRY
/// only, and the rock's config is rebuilt from the description at spawn, while
/// a [`PreparedPlanet`] carries its config beside its visual. A ship needs no
/// preparation: its sections are resolved from the catalog on the main
/// thread, which is the one place the catalog exists.
#[derive(Debug)]
pub struct PreparedSector {
    pub(crate) description: SectorDescription,
    pub(crate) asteroids: Vec<PreparedAsteroid>,
    pub(crate) planets: Vec<PreparedPlanet>,
}

impl PreparedSector {
    /// The validated description this was prepared from.
    pub fn description(&self) -> &SectorDescription {
        &self.description
    }
}

/// The most rocks ANY cell holds, and the ceiling [`validate_manifest`] holds
/// every manifest to.
///
/// Four, which is the density the examples fly and the only one measured. It
/// is one cap for every generator on purpose: the uniform baseline is what a
/// featured cell is judged against, so a baseline that could be denser than
/// anything the field produces would be judging the loop against a world it
/// never streams. A denser cell is a measurement and a change here, not a
/// number a generator can reach.
pub const SECTOR_ASTEROIDS_MAX: usize = 4;

/// The most feature spheres [`validate_manifest`] accepts in one manifest.
///
/// Sixteen, about twice the nine the base game's generator listed at most
/// over 32 seeds and 2,331 cells a seed at five edges from 8.5 km to 128 km.
/// The count grows with the cell's volume past that - 21 spheres at a 256 km
/// edge and 31 at 447 km - which is why that generator refuses an edge wider
/// than 128 km rather than let this cap refuse a cell it drew.
pub const SECTOR_FEATURES_MAX: usize = 16;

/// The most bodies - rocks, planetoids and ships together -
/// [`validate_manifest`] accepts in one manifest.
///
/// One cap over all three rather than one per kind: the clearance check,
/// the preparation and the spawn are each paid per body, whatever it is.
/// Sixteen, twice the eight the base game's generator placed at most over the
/// same census as [`SECTOR_FEATURES_MAX`]. [`SECTOR_ASTEROIDS_MAX`] still caps
/// the rocks inside it.
pub const SECTOR_BODIES_MAX: usize = 16;

/// How much room a generated ship claims for clearance.
///
/// A radius around the ship root, not a measured bound: a ship's sections are
/// resolved from the catalog on the main thread, and a worker deciding where
/// it stands cannot see them. Generous on purpose.
pub const SECTOR_SHIP_CLEARANCE: Meters = Meters(400.0);

/// Extra room every pair of objects keeps between their clearance radii.
///
/// A sector whose bodies merely fail to intersect still reads as a pile. The
/// margin is what makes a generated cell look placed.
pub const CLEARANCE_MARGIN: Meters = Meters(500.0);

/// Whether two bodies keep [`CLEARANCE_MARGIN`] between their clearance
/// spheres.
///
/// The one spacing formula: [`validate_manifest`] refuses a pair this calls
/// crowded, and a generator's placement search asks the same question, so the
/// two cannot drift apart and a search cannot accept a pose the check
/// refuses.
pub fn bodies_clear(
    a_position: Meters3,
    a_clearance: Meters,
    b_position: Meters3,
    b_clearance: Meters,
) -> bool {
    a_position.distance(b_position) >= a_clearance + b_clearance + CLEARANCE_MARGIN
}

/// One object already standing in the cell, and how much room it claims.
#[derive(Clone, Copy, Debug)]
struct Occupied {
    centre: Meters3,
    clearance: Meters,
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

/// [`feature_candidate`], memoized for one [`sector_features`] call.
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

/// Refuse a cell edge the feature field cannot answer for.
///
/// Release-visible, not a debug assertion: the thinning halo is a FINITE node
/// search sized from the widest radius, the jitter draw and the inset pull,
/// and the inset pull grows with the cell edge. Past the edge the halo covers,
/// two same-layer spheres can both survive and the world ships with belts
/// sitting inside each other. [`sector_features`] refuses such an edge on
/// every call; a generator that reads the field calls this from
/// [`SectorGenerator::validate`] as well, so the refusal lands when the world
/// is armed rather than on a worker.
///
/// # Errors
///
/// [`SectorFault::Config`] on `sector_edge` when it is not a finite positive
/// length, or when it is wider than [`FEATURE_HALO`] nodes can cover.
pub fn validate_feature_geometry(geometry: WorldGeometry) -> Result<(), SectorFault> {
    let edge = geometry.sector_edge;
    if !edge.get().is_finite() || edge.get() <= 0.0 {
        return Err(SectorFault::Config {
            field: "sector_edge",
            value: format!("{} m", edge.get()),
        });
    }
    if !feature_halo_covers_overlap(edge) {
        return Err(SectorFault::Config {
            field: "sector_edge",
            value: format!(
                "{} m, wider than a {FEATURE_HALO}-node thinning halo can reach across at a {} m \
                 feature lattice",
                edge.get(),
                FEATURE_LATTICE.get()
            ),
        });
    }
    Ok(())
}

/// Every accepted feature sphere that reaches `input.coord`, ordered by layer
/// and then lattice node.
///
/// The cell asks the FIELD, not its neighbours: the node range is derived from
/// the cell's own box and the widest radius any layer draws, so two adjacent
/// cells that both overlap one sphere are handed the same sphere rather than
/// two views of it.
///
/// # Errors
///
/// Whatever [`validate_feature_geometry`] refuses, and
/// [`SectorFault::InvalidGeometry`] for a cell whose node range runs off the
/// lattice. Plus whatever the field or a candidate refuses, and
/// [`SectorFault::DuplicateFeature`] if two spheres ever claim one id.
pub fn sector_features(input: SectorGenerationInput) -> Result<Vec<FeatureSphere>, SectorFault> {
    validate_feature_geometry(input.geometry)?;
    let fields = FeatureFields::new(input.seed);
    sector_features_from(&fields, input.seed, input.coord, input.geometry.sector_edge)
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
        // Refused, not clamped. A clamp turns a cell nobody can address into a
        // node sweep over the whole lattice, and at the far end of the grid it
        // would silently answer for ground the cell never reaches.
        let node = |value: f64| {
            (value.is_finite() && value >= f64::from(i32::MIN) && value <= f64::from(i32::MAX))
                .then_some(value as i32)
        };
        Some((node(low)?, node(high)?))
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

/// The id `<cell slug>_<name>_<index>` of one object a generator places in
/// `coord`.
///
/// Formatting only. The slug prefix is the cross-generator contract:
/// [`validate_manifest`] refuses an id without it, so no two cells can claim
/// one object, and refuses an id claimed twice inside the cell.
pub fn sector_id(coord: SectorCoord, name: &str, index: usize) -> String {
    format!("{}_{name}_{index}", coord.slug())
}

/// Turn a generator's manifest into a trusted [`SectorDescription`], or refuse
/// it. The only constructor a description has.
///
/// A generator is outside this crate, so its answer is checked against every
/// rule `materialize_sector` and the streaming loop rely on: the cell it was
/// asked for; feature spheres that are drawable, unique and really reach the
/// cell; finite geometry; ids unique and prefixed with the cell's slug, so two
/// cells never claim one object; every body standing inside its own cell with
/// its whole clearance sphere, so retiring a neighbour never takes it; every
/// pair of bodies [`CLEARANCE_MARGIN`] apart; at most [`SECTOR_FEATURES_MAX`]
/// spheres and [`SECTOR_BODIES_MAX`] bodies, refused before any is read;
/// shipped asteroid kinds and at most [`SECTOR_ASTEROIDS_MAX`] rocks; planet
/// configs that [`PlanetConfig::validate`] accepts; and each planetoid and
/// ship placed by a sphere of its own layer that THIS cell owns, so a feature
/// reaching a dozen cells is spawned once. The ship design is only checked for
/// a blank id here; the catalog lookup is main-thread work in
/// `materialize_sector`.
///
/// # Errors
///
/// [`SectorFault::Manifest`] for the wrong cell, an object outside its cell or
/// crowding another, an id another cell owns, a feature reference this cell
/// does not own, a strength outside `[0, 1]`, a planet config
/// [`PlanetConfig::validate`] refuses, a blank ship design, or more spheres,
/// bodies or rocks than a cell holds;
/// [`SectorFault::Feature`] and [`SectorFault::DuplicateFeature`] for its
/// feature spheres; [`SectorFault::InvalidGeometry`] for non-finite
/// geometry; [`SectorFault::DuplicateId`] and [`SectorFault::UnknownKind`].
pub fn validate_manifest(
    input: SectorGenerationInput,
    manifest: SectorManifest,
) -> Result<SectorDescription, SectorFault> {
    let coord = input.coord;
    let edge = input.geometry.sector_edge;
    let centre = coord.centre(edge);
    let half_edge = Meters(edge.get() * 0.5);
    let refuse = |id: &str, field: &'static str, value: String| SectorFault::Manifest {
        id: id.to_string(),
        field,
        value,
    };

    if manifest.coord != coord {
        return Err(refuse(
            &coord.slug(),
            "coord",
            format!("{}, not the requested {coord}", manifest.coord),
        ));
    }

    // Counts first: every check below walks the lists, and the clearance check
    // compares each body with every body before it.
    if manifest.features.len() > SECTOR_FEATURES_MAX {
        return Err(refuse(
            &coord.slug(),
            "features",
            format!(
                "{} spheres, above the {SECTOR_FEATURES_MAX} a cell lists",
                manifest.features.len()
            ),
        ));
    }
    let bodies = manifest.asteroids.len() + manifest.planets.len() + manifest.ships.len();
    if bodies > SECTOR_BODIES_MAX {
        return Err(refuse(
            &coord.slug(),
            "bodies",
            format!(
                "{bodies} ({} rocks, {} planetoids, {} ships), above the {SECTOR_BODIES_MAX} a \
                 cell holds",
                manifest.asteroids.len(),
                manifest.planets.len(),
                manifest.ships.len()
            ),
        ));
    }
    if manifest.asteroids.len() > SECTOR_ASTEROIDS_MAX {
        return Err(refuse(
            &coord.slug(),
            "asteroids",
            format!(
                "{} rocks, above the {SECTOR_ASTEROIDS_MAX} a cell holds",
                manifest.asteroids.len()
            ),
        ));
    }

    let mut features = BTreeMap::new();
    for sphere in &manifest.features {
        sphere.validate(edge)?;
        if !sphere.reaches_box(centre, half_edge) {
            return Err(SectorFault::Feature {
                id: sphere.id.clone(),
                field: "radius",
                value: format!("{} m, which does not reach {coord}", sphere.radius.get()),
            });
        }
        if features.insert(sphere.id.as_str(), sphere).is_some() {
            return Err(SectorFault::DuplicateFeature {
                id: sphere.id.clone(),
            });
        }
    }
    for layer in FeatureLayer::ALL {
        let strength = manifest.strengths[layer.index()];
        if !strength.is_finite() || !(0.0..=1.0).contains(&strength) {
            return Err(refuse(
                &coord.slug(),
                "strengths",
                format!("{strength} on the {layer} layer, outside 0 to 1"),
            ));
        }
    }

    let mut ids = BTreeSet::new();
    let mut standing = Vec::new();
    let mut own = |id: &str| {
        if !id.starts_with(&format!("{}_", coord.slug())) {
            return Err(refuse(
                id,
                "id",
                format!("'{id}', not prefixed with {}", coord.slug()),
            ));
        }
        if !ids.insert(id.to_string()) {
            return Err(SectorFault::DuplicateId { id: id.to_string() });
        }
        Ok(())
    };

    for body in &manifest.asteroids {
        own(&body.id)?;
        if !body.radius.get().is_finite() || body.radius.get() <= 0.0 {
            return Err(SectorFault::InvalidGeometry {
                id: body.id.clone(),
            });
        }
        if !is_asteroid_kind(&body.kind) {
            return Err(SectorFault::UnknownKind {
                kind: body.kind.clone(),
            });
        }
        let clearance = Meters(body.radius.get() * ASTEROID_GEOMETRIC_FACTOR_MAX);
        stand_inside(input, &mut standing, &body.id, body.position, clearance)?;
    }
    for planet in &manifest.planets {
        own(&planet.id)?;
        owned_feature(
            &features,
            coord,
            &planet.id,
            &planet.feature,
            FeatureLayer::Planet,
        )?;
        planet
            .config
            .validate()
            .map_err(|fault| refuse(&planet.id, fault.field, fault.value))?;
        let clearance = planet.config.body_radius();
        if !clearance.get().is_finite() || clearance.get() <= 0.0 {
            return Err(SectorFault::InvalidGeometry {
                id: planet.id.clone(),
            });
        }
        stand_inside(input, &mut standing, &planet.id, planet.position, clearance)?;
    }
    for ship in &manifest.ships {
        own(&ship.id)?;
        owned_feature(
            &features,
            coord,
            &ship.id,
            &ship.feature,
            FeatureLayer::Derelict,
        )?;
        if !ship.yaw.is_finite() {
            return Err(SectorFault::InvalidGeometry {
                id: ship.id.clone(),
            });
        }
        if ship.design.trim().is_empty() {
            return Err(refuse(&ship.id, "design", "an empty id".to_string()));
        }
        stand_inside(
            input,
            &mut standing,
            &ship.id,
            ship.position,
            SECTOR_SHIP_CLEARANCE,
        )?;
    }
    let SectorManifest {
        coord,
        features,
        strengths,
        asteroids,
        planets,
        ships,
    } = manifest;
    Ok(SectorDescription {
        coord,
        features,
        strengths,
        asteroids,
        planets,
        ships,
    })
}

/// Refuse a planetoid or ship whose feature is not a listed sphere of `layer`
/// that `coord` owns.
fn owned_feature(
    features: &BTreeMap<&str, &FeatureSphere>,
    coord: SectorCoord,
    id: &str,
    feature: &str,
    layer: FeatureLayer,
) -> Result<(), SectorFault> {
    match features.get(feature) {
        Some(sphere) if sphere.layer == layer && sphere.owner == coord => Ok(()),
        Some(sphere) => Err(SectorFault::Manifest {
            id: id.to_string(),
            field: "feature",
            value: format!(
                "'{feature}', a {} sphere owned by {}, not a {layer} sphere owned by {coord}",
                sphere.layer, sphere.owner
            ),
        }),
        None => Err(SectorFault::Manifest {
            id: id.to_string(),
            field: "feature",
            value: format!("'{feature}', which the manifest does not list"),
        }),
    }
}

/// Refuse a body that does not stand wholly inside its own cell, or that
/// crowds a body already checked.
///
/// The whole clearance sphere, not only the centre: [`PLACEMENT_INSET`] is how
/// a generator keeps a body inside its cell, and this is the fact the inset
/// exists for. A body across a face would be retired with the neighbour.
fn stand_inside(
    input: SectorGenerationInput,
    standing: &mut Vec<Occupied>,
    id: &str,
    position: Meters3,
    clearance: Meters,
) -> Result<(), SectorFault> {
    if !position_is_finite(position) {
        return Err(SectorFault::InvalidGeometry { id: id.to_string() });
    }
    let edge = input.geometry.sector_edge;
    let offset = (position.get() - input.coord.centre(edge).get()).abs();
    if SectorCoord::containing(position, edge) != input.coord
        || offset.max_element() + clearance.get() > edge.get() * 0.5
    {
        let at = position.get();
        return Err(SectorFault::Manifest {
            id: id.to_string(),
            field: "position",
            value: format!(
                "{:.0} {:.0} {:.0} m with {} m of clearance, not inside {}",
                at.x,
                at.y,
                at.z,
                clearance.get(),
                input.coord
            ),
        });
    }
    if let Some(other) = standing
        .iter()
        .find(|other| !bodies_clear(other.centre, other.clearance, position, clearance))
    {
        let at = other.centre.get();
        return Err(SectorFault::Manifest {
            id: id.to_string(),
            field: "position",
            value: format!(
                "closer than {} m of margin to the body at {:.0} {:.0} {:.0} m",
                CLEARANCE_MARGIN.get(),
                at.x,
                at.y,
                at.z
            ),
        });
    }
    standing.push(Occupied {
        centre: position,
        clearance,
    });
    Ok(())
}

/// Describe one cell and refuse the answer unless the world can materialize
/// it. PURE: the same config and coordinate give the same description, on any
/// call, in any order, with nothing live.
///
/// # Errors
///
/// Whatever [`WorldConfig::validate`], the generator or [`validate_manifest`]
/// refuses, and [`SectorFault::InvalidGeometry`] for a cell whose centre has
/// no finite position in meters - refused before [`SectorGenerator::generate`]
/// is asked, so no generator places around a centre that cannot exist. All
/// are refusals before anything spawns.
pub fn generate_sector<G: SectorGenerator>(
    config: &WorldConfig<G>,
    coord: SectorCoord,
) -> Result<SectorDescription, SectorFault> {
    config.validate()?;
    if !position_is_finite(coord.centre(config.sector_edge)) {
        return Err(SectorFault::InvalidGeometry { id: coord.slug() });
    }
    let input = config.input(coord);
    validate_manifest(input, config.generator.generate(input)?)
}

/// Describe, check and prepare one cell: everything a sector costs except
/// the spawning. PURE, so [`crate::SectorJob`] can run it on a worker.
///
/// ONE implementation for every generator: a generator describes, and the
/// preparation of a checked description is the same work whoever wrote it.
///
/// Takes the config by value because a job answers the question it was asked:
/// a dial changed mid-flight must not silently re-aim work already in the air.
///
/// # Errors
///
/// Whatever [`generate_sector`] refuses. A sector that cannot be described is
/// never meshed, let alone spawned.
pub fn prepare_sector<G: SectorGenerator>(
    config: WorldConfig<G>,
    coord: SectorCoord,
) -> Result<PreparedSector, SectorFault> {
    let description = generate_sector(&config, coord)?;
    let asteroids = description
        .asteroids
        .iter()
        .map(|body| prepare_asteroid_geometry(body.seed, body.radius))
        .collect();
    let planets = description
        .planets
        .iter()
        .map(|planet| prepare_planet(planet.config.clone()))
        .collect();
    Ok(PreparedSector {
        description,
        asteroids,
        planets,
    })
}
