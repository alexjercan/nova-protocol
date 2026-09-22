//! The world-sector spike kit: a deterministic sector generator and a 5x5x5
//! streaming loop that runs OVER a live scenario instead of through one.
//!
//! Spike scope, and it ends at the example boundary. Nothing here is a
//! production interface: there is no floating origin, no persistence, no
//! player ship, and the world seed is a constant in this file. What it exists
//! to show is the one question the streamed-world direction stands on - can a
//! sector's contents be generated, validated, prepared off the frame and
//! materialized, and then retired again, while ONE scenario stays loaded the
//! whole time.
//!
//! So routine sector traffic never touches `LoadScenario` and never runs a
//! scenario event action. It calls the same object factories the loader calls
//! ([`base_scenario_object`], [`asteroid_scenario_object_prepared`],
//! [`planet_scenario_object_prepared`], [`spaceship_scenario_object`]), which
//! is what makes a streamed rock the same rock a scenario spawns. The
//! bootstrap scenario is loaded once, is EMPTY, and is the session's outer
//! lifetime.
//!
//! # Two generators, one lifetime
//!
//! [`SectorGeneration`] is what a sector is FILLED with, and it is the only
//! thing the two examples disagree about:
//!
//! - [`SectorGeneration::UniformAsteroids`] fills every cell the same way, out
//!   of the cell's own seed. It is the streaming baseline: 125 identical-
//!   looking cells make a retirement, a crossing and a duplicate root easy to
//!   see, and nothing about the world can explain away a missing sector.
//! - [`SectorGeneration::LayeredFeatures`] fills a cell from a world that
//!   exists ABOVE it. Three independent global noise fields gate candidate
//!   FEATURE SPHERES on a coarse 128 km lattice; the spheres are pure data,
//!   addressed by lattice node rather than by sector, and a cell asks which of
//!   them reach it. That is what makes two neighbouring cells agree about a
//!   belt that crosses both of them without either one owning it.
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
//! [`feature_halo_covers_overlap`] is the arithmetic, checked in a debug
//! build.
//!
//! # The job lifetime
//!
//! Nothing is described and spawned in the same breath. A desired cell that is
//! not live becomes a JOB; the job describes, validates and meshes the whole
//! sector on `AsyncComputeTaskPool`; and the main thread spends its frame only
//! on the one step a worker cannot take, which is turning a prepared sector
//! into entities. Four explicit systems, in this order:
//!
//! 1. [`request_sectors`] starts jobs for the desired cells that are not
//!    already live, running or prepared, NEAREST FIRST and only as many as the
//!    task pool has threads. The rest of the window stays unrequested - not
//!    queued and not deferred - until a slot opens.
//! 2. [`collect_sector_jobs`] polls WITHOUT blocking. A [`SectorFault`] from a
//!    worker is fatal here, on the main thread, where it can name itself; a
//!    completion for a cell nobody wants any more is dropped rather than kept.
//! 3. [`materialize_ready_sector`] spawns AT MOST ONE prepared sector, nearest
//!    first, so a 125-cell window costs 125 frames of spawning instead of one
//!    frame of all of it.
//! 4. [`retire_sectors`] takes back roots, running jobs and prepared results
//!    that fall outside the desired set. Dropping a [`SectorJob`] cancels its
//!    task.
//!
//! Every decision is taken over a TOTAL ORDER - distance from the observer's
//! cell first, the coordinate itself breaking ties - read out of ordered
//! collections: the desired set is a `BTreeSet`, the live roots and prepared
//! results are `BTreeMap`s. So which worker finished first can never change
//! which sector is requested or which one a frame spends its budget on.
//!
//! On wasm `AsyncComputeTaskPool` is the page's own task queue on the one
//! thread the page has, so the split buys no parallelism there. It still buys
//! the frame and the cancellation, which is the same trade `asteroid_carve`
//! already makes for a remesh.
//!
//! # Who owns what
//!
//! Three owners, nested rather than competing:
//!
//! - the scenario owns the session. Every sector root carries
//!   [`ScenarioScopedMarker`] (through `base_scenario_object`), so
//!   `UnloadScenario` is the final sweep and cannot leave a sector behind.
//! - [`SectorRoot`] owns one sector. Retiring it despawns that sector's
//!   bodies, planetoids and moored hulls and nothing else - not the camera,
//!   not the other sectors. A feature sphere is owned by exactly ONE cell
//!   ([`FeatureSphere::owner`], the cell its centre falls in) even where the
//!   sphere reaches across a dozen of them, so a planetoid is spawned once and
//!   retired once.
//! - the plugin owns the WORK. A pending [`SectorJob`] and a prepared
//!   [`ReadySectors`] payload are not scenario objects and the scenario sweep
//!   cannot see them, so [`clear_sector_work`] drops them the moment the
//!   session STOPS BEING THIS SESSION. That is two events, not one: an
//!   unload, and a `LoadScenario` that replaces a live scenario. The second
//!   one never passes through a no-scenario frame - `on_load_scenario` tears
//!   the old session down and writes the new `CurrentScenario` in one observer
//!   call - so liveness alone cannot see it and the condition reads
//!   `CurrentScenario` CHANGING as well. Without that a reloaded session would
//!   materialize sectors the session before it asked for.
//!
//! Shared by three examples on purpose: `system_world_sectors` asserts what
//! `world_sectors` and `world_features` show a human, and a copied generator
//! would let them drift into proving nothing about each other.

// Three example targets, one kit: what one leaves unused another needs.
#![expect(
    dead_code,
    reason = "one source, three example targets: what one range leaves unused the playable observers need"
)]

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use bevy::{
    prelude::*,
    tasks::{block_on, poll_once, AsyncComputeTaskPool, Task},
};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use nova_authoring::prelude::BLOCK_HAULER_SHIP_ID;
use nova_protocol::prelude::*;

/// The spike's world seed.
///
/// A constant, not a setting: a world seed belongs to a save, and this spike
/// has no save. Every run of either example is therefore the same world, which
/// is what lets the range compare a returned sector against the description it
/// had the first time.
pub const WORLD_SEED: u32 = 20_260_922;

/// The kinds the generator draws bodies from.
///
/// The spike's own table, NOT [`ASTEROID_KINDS`] - it is the stand-in for the
/// content table a real generator would pick from, and the thing
/// [`generate_sector`] checks each draw against. `plain` is absent because it
/// is the texture control, not a rock a world would contain.
const SECTOR_KINDS: [&str; 4] = [KIND_ROCK, KIND_METAL, KIND_ICE, KIND_CARBON];

/// The worlds a feature-gated planetoid is drawn from.
///
/// Three of the six [`PlanetType`]s, and the three that read as DEAD ROCK at
/// 600-1,200 m: a temperate ocean world the size of a city block is a joke,
/// and a volcanic one at that radius is a lava ball. The spike's table, the
/// same way [`SECTOR_KINDS`] is.
const SECTOR_PLANET_TYPES: [PlanetType; 3] = [
    PlanetType::BarrenRock,
    PlanetType::DustWorld,
    PlanetType::IceWorld,
];

/// How much of a sector's half-edge a PHYSICAL object may be placed along.
///
/// Objects are owned by a cell, so they have to stay inside it: a rock placed
/// at the face would be half in the neighbour, and retiring the neighbour
/// would look like retiring the wrong sector. It applies to a feature-owned
/// planetoid too, which is why a feature sphere's centre is pulled onto its
/// owner's inset (see [`feature_candidate`]) rather than clamped there after
/// the fact.
pub const PLACEMENT_INSET: f32 = 0.7;

/// An integer sector coordinate: which cell of the world grid, never where in
/// meters.
///
/// `Ord` is derived so a desired set and a live set are both ordered
/// collections. Generation must not depend on iteration order, but a REPORT
/// that lists sectors in hash order is unreadable.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SectorCoord {
    /// Cell index along +X.
    pub x: i32,
    /// Cell index along +Y.
    pub y: i32,
    /// Cell index along +Z.
    pub z: i32,
}

impl SectorCoord {
    /// The cell the world origin falls in.
    pub const ORIGIN: Self = Self { x: 0, y: 0, z: 0 };

    /// The cell at these indices.
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// The cell `position` falls in.
    ///
    /// A cell is CENTRED on its coordinate - cell `n` spans
    /// `[n * edge - edge/2, n * edge + edge/2)` - rather than cornered at it.
    /// That keeps cell (0, 0, 0) around the world origin, which is where the
    /// scenario loader parks a cameraless scene's free-fly camera: a cornered
    /// grid would open every run looking out of the field instead of into it.
    pub fn containing(position: Meters3, edge: Meters) -> Self {
        let cell = |meters: Meters| (meters.get() / edge.get()).round() as i32;
        Self {
            x: cell(position.x()),
            y: cell(position.y()),
            z: cell(position.z()),
        }
    }

    /// The cell's centre, in meters.
    pub fn centre(self, edge: Meters) -> Meters3 {
        let axis = |index: i32| index as f32 * edge.get();
        Meters3::new(axis(self.x), axis(self.y), axis(self.z))
    }

    /// The cell this many cells away.
    pub const fn offset(self, x: i32, y: i32, z: i32) -> Self {
        Self {
            x: self.x + x,
            y: self.y + y,
            z: self.z + z,
        }
    }

    /// The cell's stable id text, and the prefix every body it owns is named
    /// from. `n` reads as minus, because a `-` in a scenario id is not a
    /// snake_case slug.
    pub fn slug(self) -> String {
        format!(
            "sector_{}_{}_{}",
            index_slug(self.x),
            index_slug(self.y),
            index_slug(self.z)
        )
    }
}

impl std::fmt::Display for SectorCoord {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "({}, {}, {})", self.x, self.y, self.z)
    }
}

/// One signed lattice or cell index as a snake_case-safe slug. `n` reads as
/// minus, because a `-` in a scenario id is not a slug.
fn index_slug(index: i32) -> String {
    if index < 0 {
        format!("n{}", index.unsigned_abs())
    } else {
        index.to_string()
    }
}

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
    /// here chosen by looking. Against the pinned [`WORLD_SEED`] they put
    /// empty cells, single-layer cells and blended cells all inside the
    /// window around [`FEATURE_HOME`]: 67 cells of the 125 read nothing at
    /// all, 46 read one layer, 12 read two or more, and the cells that do hold
    /// rocks use the whole 0-4 range. That spread is what makes the
    /// `world_features` example show three things rather than one.
    ///
    /// Three octaves of Perlin reads far narrower than its nominal `[-1, 1]` -
    /// these thresholds sit inside the band the field actually occupies, which
    /// is also what [`FEATURE_CEILING`] is measured from.
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
/// Four sector edges. Coarse enough that a feature is a REGION rather than a
/// cell's decoration, and fine enough that the 160 km active window contains
/// several nodes, so a hand-flown crossing walks into and out of features
/// instead of living inside one.
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
/// Measured off the pinned field rather than assumed. Three octaves at
/// persistence 0.5 sum to a value well inside the `[-1, 1]` a single Perlin
/// octave spans - over the 343 nodes within three lattice spacings of the
/// origin the highest any layer reads is about 0.57, and a typical accepted
/// node reads 0.2-0.35. Normalizing a margin against 1.0 instead would make
/// every sphere weak, every cell round down to no rocks, and the whole field
/// invisible.
const FEATURE_CEILING: f32 = 0.22;

/// How many lattice nodes out the same-layer thinning looks.
///
/// DERIVED, not chosen: see [`feature_halo_covers_overlap`]. Two same-layer
/// spheres overlap only if their centres are within `2 * FEATURE_RADIUS_MAX`,
/// and a centre sits within one jitter draw plus one inset pull of its node,
/// so a node further out than this cannot hold a rival.
pub const FEATURE_HALO: i32 = 2;

/// Whether [`FEATURE_HALO`] really covers every node that can hold an
/// overlapping same-layer rival.
///
/// The arithmetic the halo constant is derived from, written as a function so
/// it is checked rather than asserted in a comment. [`generate_sector`] debug-
/// asserts it, so raising a radius band or the jitter without widening the
/// halo fails a debug run instead of silently letting two belts overlap.
pub fn feature_halo_covers_overlap(edge: Meters) -> bool {
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

    /// What `layer` reads at `position`.
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

/// What a cell is FILLED with. The one thing the spike's examples disagree
/// about; the window, the edge and the whole job lifetime are shared.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SectorGeneration {
    /// Every cell gets the same treatment out of its own seed: `bodies` rocks
    /// scattered across its inset, nominal radius drawn from the band.
    ///
    /// The streaming baseline. Nothing about the WORLD can explain away a
    /// sector that failed to come up, which is what makes it the right
    /// generator for judging a retirement or a crossing.
    UniformAsteroids {
        /// Rocks generated per sector.
        bodies: usize,
        /// Smallest nominal body radius drawn.
        radius_min: Meters,
        /// Largest nominal body radius drawn.
        radius_max: Meters,
    },
    /// The cell asks the feature field what reaches it, and fills itself from
    /// the answer: rocks from the combined asteroid influence, one planetoid
    /// per owned planet sphere, a few moored hulls per owned anchorage sphere.
    ///
    /// No dials. Everything it needs is a property of a [`FeatureLayer`] or of
    /// the lattice, because a per-example override of a global field would be
    /// two worlds with one seed.
    LayeredFeatures,
}

/// The streaming dials: how big a sector is, how far the desired set reaches,
/// and what a sector is filled with.
///
/// A resource, and the ARMING switch: the streaming systems do nothing until
/// it is inserted. That is what lets the range assert the bootstrap scenario is
/// empty before any sector exists.
#[derive(Resource, Clone, Copy, Debug)]
pub struct SectorSettings {
    /// Sector edge length.
    pub edge: Meters,
    /// How many cells out from the current one the desired set reaches. The
    /// desired set is the cube of side `2 * radius + 1`.
    pub radius: i32,
    /// What fills a cell.
    pub generation: SectorGeneration,
}

impl SectorSettings {
    /// The streaming baseline. No `Default`: a settings value with no author
    /// is how a number nobody chose reaches a frame.
    ///
    /// 32 km edges at radius 2 put 125 sectors and 500 bodies live at once
    /// across a 160 km cube, which leaves at least 64 km of live world on
    /// every axis ahead of an observer standing anywhere in the centre cell.
    /// That is a TRAVEL-scale cell, not a see-it-all-at-once one: a
    /// hand-flown crossing is about 530 s at [`WASD_BASE_SPEED`] and about
    /// 17 s held on the ramp
    /// ([`WASD_ACCELERATION_MAX`] is 32x), so a boundary is reached the way a
    /// pilot would reach one, under way, rather than by drifting over a line
    /// 30 s out. Four bodies in a cell this size read as scattered landmarks
    /// rather than a belt, which is what the crossing claim needs and is not a
    /// claim about how dense a real sector should be.
    ///
    /// The nominal radius is 30-60 m and not the 60-120 m this started at,
    /// because an authored radius is not what a rock DRAWS: the meshed radius
    /// reaches 3.5-6x past it
    /// ([`ASTEROID_GEOMETRIC_FACTOR_MIN`]..[`ASTEROID_GEOMETRIC_FACTOR_MAX`]).
    /// The original range could therefore draw 420-1,440 m diameters; the
    /// selected range draws about 210-720 m diameters. The body count and
    /// radius range are spike-visualization dials; the 32 km edge and the
    /// 125-cell active window are the selected baseline, still subject to
    /// production-load proof.
    pub const SPIKE: Self = Self {
        edge: Meters(32_000.0),
        radius: 2,
        generation: SectorGeneration::UniformAsteroids {
            bodies: 4,
            radius_min: Meters(30.0),
            radius_max: Meters(60.0),
        },
    };
}

/// The cell the featured examples open in.
///
/// NOT the origin, and chosen rather than assumed: the window around it is the
/// one near the origin that holds all three layers at once - two asteroid
/// spheres, one planet sphere (owned by cell `(0, 0, 0)` itself) and one
/// anchorage sphere - beside 67 cells the field leaves completely empty. A
/// run that opened at the origin would see a planetoid and a handful of rocks
/// and nothing else, which proves a generator but not a WORLD.
pub const FEATURE_HOME: SectorCoord = SectorCoord::new(-2, -2, 2);

impl SectorSettings {
    /// The same window and the same edge, filled from the feature field.
    ///
    /// Sharing the streaming dials with [`Self::SPIKE`] is the point: what
    /// changes between the two examples is what a cell CONTAINS, so a
    /// difference in how the window behaves cannot be blamed on a different
    /// window.
    pub const FEATURES: Self = Self {
        generation: SectorGeneration::LayeredFeatures,
        ..Self::SPIKE
    };

    /// Refuse dials that cannot describe a sector. Called by
    /// [`generate_sector`], so an invalid value stops the run before anything
    /// is spawned rather than producing a sector nobody can stand in.
    pub fn validate(&self) -> Result<(), SectorFault> {
        let refuse =
            |field: &'static str, value: String| Err(SectorFault::Settings { field, value });
        if !self.edge.get().is_finite() || self.edge.get() <= 0.0 {
            return refuse("edge", format!("{} m", self.edge.get()));
        }
        if self.radius < 0 {
            return refuse("radius", self.radius.to_string());
        }
        match self.generation {
            SectorGeneration::UniformAsteroids {
                bodies,
                radius_min,
                radius_max,
            } => {
                if bodies == 0 {
                    return refuse("bodies", bodies.to_string());
                }
                if !radius_min.get().is_finite() || radius_min.get() <= 0.0 {
                    return refuse("radius_min", format!("{} m", radius_min.get()));
                }
                if !radius_max.get().is_finite() || radius_max < radius_min {
                    return refuse(
                        "radius_max",
                        format!(
                            "{} m, expected a finite value at least radius_min {} m",
                            radius_max.get(),
                            radius_min.get()
                        ),
                    );
                }
            }
            SectorGeneration::LayeredFeatures => {}
        }
        Ok(())
    }
}

/// Marks a sector root and names the cell it owns.
///
/// The narrow cleanup scope. The root's children are the sector's objects, so
/// despawning it retires exactly one sector; the [`ScenarioScopedMarker`] it
/// also carries keeps the session sweep able to take it.
#[derive(Component, Clone, Copy, Debug)]
pub struct SectorRoot(pub SectorCoord);

/// The feature spheres that reach this sector, on its root. DIAGNOSTIC: the
/// generator already used them, and this is how `world_features` draws them
/// and how the range reads one sphere from two cells.
#[derive(Component, Clone, Debug)]
pub struct SectorFeatureSpheres(pub Vec<FeatureSphere>);

/// The combined per-layer influence at this sector's centre, on its root.
/// DIAGNOSTIC, indexed by [`FeatureLayer::index`].
#[derive(Component, Clone, Copy, Debug)]
pub struct SectorStrengths(pub [f32; FeatureLayer::COUNT]);

/// Which cell the observer camera is in. Written by
/// [`track_current_sector`], and the centre every desired-set decision in the
/// job lifetime is taken against.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CurrentSector(pub SectorCoord);

/// Why a sector refused. Every variant is a REFUSAL BEFORE MATERIALIZATION:
/// the generator will not describe a sector it cannot describe correctly, and
/// the streaming loop will not spawn into a world it cannot read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SectorFault {
    /// A dial cannot describe a sector.
    Settings {
        /// The [`SectorSettings`] field at fault.
        field: &'static str,
        /// What it held.
        value: String,
    },
    /// A layer's field returned a non-finite reading. A `NaN` against a
    /// threshold is silently false, so a whole region would go quietly empty.
    Noise {
        /// The layer whose field misread.
        layer: FeatureLayer,
        /// Where it was sampled.
        at: String,
    },
    /// A gated feature sphere cannot be drawn or reasoned about.
    Feature {
        /// The sphere at fault.
        id: String,
        /// Which of its fields.
        field: &'static str,
        /// What that field held.
        value: String,
    },
    /// Two feature spheres claim the same id. A sphere's id is how two cells
    /// agree they are looking at ONE feature, so a collision is not a naming
    /// problem, it is a world with two of something in one place.
    DuplicateFeature {
        /// The id claimed twice.
        id: String,
    },
    /// The generator drew a kind the game does not ship. The spike's
    /// [`SECTOR_KINDS`] standing in for a content table is exactly where a mod
    /// would put an id nobody registered.
    UnknownKind {
        /// The body that named it.
        id: String,
        /// The id nobody answers to.
        kind: String,
    },
    /// A moored hull names a ship design the loaded catalog does not hold.
    /// Checked on the main thread, where the catalog lives, and refused before
    /// the sector spawns anything.
    UnknownShip {
        /// The hull that named it.
        id: String,
        /// The design id nobody answers to.
        design: String,
    },
    /// Two objects in one sector claim the same id. Never resolved by spawn
    /// order: an id is how an object is found again.
    DuplicateId {
        /// The id claimed twice.
        id: String,
    },
    /// A generated object's pose or radius is not finite. This can occur even
    /// with finite settings when a coordinate-to-meter conversion overflows.
    InvalidGeometry {
        /// The object whose geometry cannot be materialized.
        id: String,
    },
    /// Every deterministic placement candidate for an object overlapped
    /// something already placed. The cell is too full for what the field asked
    /// of it, and a sector that quietly dropped the object would hide that.
    Clearance {
        /// The object with nowhere to stand.
        id: String,
        /// How many candidates were drawn and rejected.
        attempts: usize,
    },
    /// Two live roots claim the same cell. One of them is a leak, and which
    /// one is not decidable from here.
    DuplicateRoot {
        /// The cell claimed twice.
        coord: SectorCoord,
    },
    /// The observer is gone, or there is more than one. Streaming has no
    /// centre to stream around, and guessing one silently moves the world.
    AbsentObserver,
}

impl std::fmt::Display for SectorFault {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Settings { field, value } => write!(
                formatter,
                "SectorSettings::{field} is {value}, which cannot describe a sector"
            ),
            Self::Noise { layer, at } => write!(
                formatter,
                "the {layer} feature field read a non-finite value at {at}"
            ),
            Self::Feature { id, field, value } => write!(
                formatter,
                "feature sphere '{id}' has {field} {value}, which cannot be placed"
            ),
            Self::DuplicateFeature { id } => {
                write!(formatter, "two feature spheres claim the id '{id}'")
            }
            Self::UnknownKind { id, kind } => write!(
                formatter,
                "body '{id}' drew asteroid kind '{kind}', which the game does not ship"
            ),
            Self::UnknownShip { id, design } => write!(
                formatter,
                "moored hull '{id}' names ship design '{design}', which the catalog does not hold"
            ),
            Self::DuplicateId { id } => {
                write!(formatter, "two objects in one sector claim the id '{id}'")
            }
            Self::InvalidGeometry { id } => write!(
                formatter,
                "object '{id}' generated a non-finite position or radius"
            ),
            Self::Clearance { id, attempts } => write!(
                formatter,
                "object '{id}' found no clear place in its sector in {attempts} candidates"
            ),
            Self::DuplicateRoot { coord } => {
                write!(formatter, "two live sector roots claim the cell {coord}")
            }
            Self::AbsentObserver => write!(
                formatter,
                "there is not exactly one scenario camera to stream around"
            ),
        }
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
    /// Its asteroid kind id.
    pub kind: &'static str,
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
    /// The world it is, ready for [`prepare_planet`].
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
    /// The catalog design it is built from.
    pub design: &'static str,
}

/// One sector's validated contents: what [`materialize_sector`] is allowed to
/// spawn, and the feature data that explains it.
///
/// Only [`generate_sector`] builds one, and it only returns one that passed
/// every check. A description in hand IS the readiness gate this spike has.
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
/// [`materialize_sector`] needs that a worker can produce.
///
/// Built only by [`prepare_sector`], so `asteroid_geometry` is one prepared
/// rock per `description.asteroids` entry and `planet_surfaces` one prepared
/// world per `description.planets` entry, both in order. A moored hull needs
/// no preparation: its sections are resolved from the catalog by an observer
/// on the main thread, which is the one place the catalog exists.
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
///
/// Four, the same as [`SectorSettings::SPIKE`] - so the two generators put the
/// same load on a cell at their busiest, and a difference between the examples
/// is the FIELD rather than the density.
const MAX_SECTOR_ASTEROIDS: usize = 4;

/// The nominal radius band a featured cell draws rocks from. The same band
/// [`SectorSettings::SPIKE`] uses, for the same reason: the meshed rock reaches
/// 3.5-6x past the nominal figure.
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
/// A radius around the hull root, not a measured bound: the block hauler's
/// sections are resolved from the catalog on the main thread, and a worker
/// deciding where it stands cannot see them. Generous on purpose.
pub const HULL_CLEARANCE: Meters = Meters(400.0);

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
/// inside the cell that owns it. The pull is a 32 km-scale correction on a
/// 128 km-scale jitter, so it moves where a sphere sits inside its cell and
/// not which region of the world it belongs to.
///
/// # Errors
///
/// [`SectorFault::Noise`] for a non-finite field reading and
/// [`SectorFault::Feature`] for a sphere that cannot be placed.
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

    let threshold = layer.threshold();
    let value = fields.sample(layer, node_point)?;
    if value <= threshold {
        return Ok(None);
    }
    let strength = ((value - threshold) / (FEATURE_CEILING - threshold)).clamp(0.0, 1.0);

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
/// Whatever the field or a candidate refuses, plus
/// [`SectorFault::DuplicateFeature`] if two spheres ever claim one id.
pub fn sector_features(
    world_seed: u32,
    coord: SectorCoord,
    edge: Meters,
) -> Result<Vec<FeatureSphere>, SectorFault> {
    let fields = FeatureFields::new(world_seed);
    sector_features_from(&fields, world_seed, coord, edge)
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
///
/// # Errors
///
/// [`SectorFault::InvalidGeometry`] for a candidate that is not finite, and
/// [`SectorFault::Clearance`] when [`PLACEMENT_ATTEMPTS`] candidates all
/// failed - the cell is too full for what the field asked of it, which is a
/// refusal rather than a dropped object.
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

/// Describe one cell. PURE: same world seed, same coordinate and same settings
/// give the same description, on any call, in any order, with nothing live.
///
/// # Errors
///
/// Every [`SectorFault`] a worker can raise: [`SectorFault::Settings`] for
/// dials that cannot describe a sector, [`SectorFault::Noise`] and
/// [`SectorFault::Feature`] from the feature field, [`SectorFault::UnknownKind`]
/// for a drawn kind the game does not ship, [`SectorFault::DuplicateId`] and
/// [`SectorFault::DuplicateFeature`] when two things claim one id,
/// [`SectorFault::InvalidGeometry`] when finite inputs overflow while deriving
/// a pose, and [`SectorFault::Clearance`] when a cell has no room left. All are
/// refusals before anything spawns.
pub fn generate_sector(
    world_seed: u32,
    coord: SectorCoord,
    settings: &SectorSettings,
) -> Result<SectorDescription, SectorFault> {
    settings.validate()?;
    let edge = settings.edge;
    let centre = coord.centre(edge);
    let mut claimed = BTreeSet::new();
    let mut standing: Vec<Occupied> = Vec::new();
    let (features, strengths, planets, anchorages, asteroid_count, asteroid_radius) =
        match settings.generation {
            SectorGeneration::UniformAsteroids {
                bodies,
                radius_min,
                radius_max,
            } => (
                Vec::new(),
                [0.0; FeatureLayer::COUNT],
                Vec::new(),
                Vec::new(),
                bodies,
                (radius_min, radius_max),
            ),
            SectorGeneration::LayeredFeatures => {
                // Here rather than at the top of the call: the halo is
                // arithmetic about THIS generator, and the drift term it
                // covers grows with the cell edge, so a uniform sector with a
                // deliberately absurd edge is not this assertion's business.
                debug_assert!(
                    feature_halo_covers_overlap(edge),
                    "world_sectors: FEATURE_HALO of {FEATURE_HALO} node(s) cannot reach every \
                     same-layer rival a {} m radius, {FEATURE_JITTER} jitter and a {} m cell \
                     can put outside it",
                    FEATURE_RADIUS_MAX.get(),
                    edge.get()
                );
                let features = sector_features(world_seed, coord, edge)?;
                let mut strengths = [0.0; FeatureLayer::COUNT];
                for sphere in &features {
                    strengths[sphere.layer.index()] += sphere.influence(centre);
                }
                for strength in &mut strengths {
                    *strength = strength.min(1.0);
                }

                // Planetoids first: a planet sphere places its world AT its
                // centre, which is the one pose in a cell nothing else gets to
                // move. Everything after it works around what is already
                // there.
                let mut stream = SeedStream::new(sector_seed(world_seed, coord, "planets"));
                let mut planets = Vec::new();
                for sphere in features
                    .iter()
                    .filter(|sphere| sphere.layer == FeatureLayer::Planet && sphere.owner == coord)
                {
                    let id = claim_id(
                        format!("{}_planet_{}", coord.slug(), planets.len()),
                        &mut claimed,
                    )?;
                    let (radius_min, radius_max) = PLANETOID_RADIUS;
                    let radius = radius_min + (radius_max - radius_min) * stream.unit();
                    let planet_type =
                        SECTOR_PLANET_TYPES[stream.next_u32() as usize % SECTOR_PLANET_TYPES.len()];
                    let config = PlanetConfig::new(planet_type, radius, stream.next_u32());
                    if !position_is_finite(sphere.centre) || !radius.get().is_finite() {
                        return Err(SectorFault::InvalidGeometry { id });
                    }
                    let clearance = config.body_radius();
                    if standing.iter().any(|other| {
                        other.centre.distance(sphere.centre)
                            < other.clearance + clearance + CLEARANCE_MARGIN
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

                let mut stream = SeedStream::new(sector_seed(world_seed, coord, "anchorages"));
                let mut anchorages = Vec::new();
                for sphere in features.iter().filter(|sphere| {
                    sphere.layer == FeatureLayer::Anchorage && sphere.owner == coord
                }) {
                    let (hulls_min, hulls_max) = ANCHORAGE_HULLS;
                    let span = hulls_max - hulls_min + 1;
                    let hulls = hulls_min + stream.next_u32() as usize % span;
                    for _ in 0..hulls {
                        let id = claim_id(
                            format!("{}_hull_{}", coord.slug(), anchorages.len()),
                            &mut claimed,
                        )?;
                        let position = place_object(
                            &mut stream,
                            &id,
                            coord,
                            edge,
                            sphere.centre,
                            ANCHORAGE_MOOR,
                            HULL_CLEARANCE,
                            &standing,
                        )?;
                        standing.push(Occupied {
                            centre: position,
                            clearance: HULL_CLEARANCE,
                        });
                        anchorages.push(SectorAnchorage {
                            id,
                            feature: sphere.id.clone(),
                            position,
                            yaw: stream.unit() * std::f32::consts::TAU,
                            design: BLOCK_HAULER_SHIP_ID,
                        });
                    }
                }

                // Rocks come off the COMBINED asteroid influence, so two
                // spheres overlapping a cell fill it more than either would
                // alone and a cell no sphere reaches gets none at all.
                // CEILING, not rounding: a cell a belt reaches at all holds
                // at least one rock. Rounding put a whole outer shell of a
                // sphere at zero rocks, so the belt had a hard edge one cell
                // inside its own rim and the falloff bought nothing.
                let count = (strengths[FeatureLayer::Asteroid.index()]
                    * MAX_SECTOR_ASTEROIDS as f32)
                    .ceil() as usize;
                (
                    features,
                    strengths,
                    planets,
                    anchorages,
                    count.min(MAX_SECTOR_ASTEROIDS),
                    FEATURE_ASTEROID_RADIUS,
                )
            }
        };

    let mut stream = SeedStream::new(sector_seed(world_seed, coord, "bodies"));
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
        let kind = SECTOR_KINDS[stream.next_u32() as usize % SECTOR_KINDS.len()];
        if !is_asteroid_kind(kind) {
            return Err(SectorFault::UnknownKind {
                id,
                kind: kind.to_string(),
            });
        }
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

/// The cells the observer wants live: the cube of side `2 * radius + 1`
/// centred on `centre`.
pub fn desired_sectors(centre: SectorCoord, radius: i32) -> BTreeSet<SectorCoord> {
    let mut desired = BTreeSet::new();
    for x in -radius..=radius {
        for y in -radius..=radius {
            for z in -radius..=radius {
                desired.insert(centre.offset(x, y, z));
            }
        }
    }
    desired
}

/// Describe and prepare one cell: everything a sector costs except the
/// spawning. PURE, so [`SectorJob`] can run it on a worker.
///
/// # Errors
///
/// Whatever [`generate_sector`] refuses. A sector that cannot be described is
/// never meshed, let alone spawned.
pub fn prepare_sector(
    world_seed: u32,
    coord: SectorCoord,
    settings: SectorSettings,
) -> Result<PreparedSector, SectorFault> {
    let description = generate_sector(world_seed, coord, &settings)?;
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

/// Spawn one prepared sector: the sector root, and one child per object the
/// description names - rocks, then planetoids, then moored hulls. Returns the
/// root.
///
/// The root is an OWNERSHIP node and not a pose - it stays at the world origin
/// and its children carry world positions. `base_scenario_object` seeds a
/// `GlobalTransform` from the entity's own `Transform`, and avian's
/// `transform_to_position` reads that seed in `FixedPostUpdate`, a schedule
/// ahead of bevy's `TransformSystems::Propagate`. A root posed at the sector
/// centre would therefore hand every child body a global position equal to its
/// LOCAL offset, and a rigid body's `Position` is authoritative from then on.
///
/// # Panics
///
/// When `prepared` carries a different number of rocks than asteroids or a
/// different number of surfaces than planetoids - zipping a short list would
/// silently spawn a sector missing its tail - and on
/// [`SectorFault::UnknownShip`] when a moored hull names a design the loaded
/// catalog does not hold. The catalog is a main-thread resource, so this is
/// the first place the id CAN be checked, and it is checked before the root is
/// spawned.
pub fn materialize_sector(
    commands: &mut Commands,
    prepared: PreparedSector,
    texture: &AssetRef<Image>,
    designs: &GameShipDesigns,
) -> Entity {
    let PreparedSector {
        description,
        asteroid_geometry,
        planet_surfaces,
    } = prepared;
    let coord = description.coord;
    assert_eq!(
        description.asteroids.len(),
        asteroid_geometry.len(),
        "world_sectors: {coord} was prepared with {} rocks for {} asteroids",
        asteroid_geometry.len(),
        description.asteroids.len()
    );
    assert_eq!(
        description.planets.len(),
        planet_surfaces.len(),
        "world_sectors: {coord} was prepared with {} surfaces for {} planetoids",
        planet_surfaces.len(),
        description.planets.len()
    );
    for hull in &description.anchorages {
        assert!(
            designs.get_design(hull.design).is_some(),
            "world_sectors: {}",
            SectorFault::UnknownShip {
                id: hull.id.clone(),
                design: hull.design.to_string(),
            }
        );
    }

    let objects = description.object_count();
    let root = commands
        .spawn((
            base_scenario_object(&BaseScenarioObjectConfig {
                id: coord.slug(),
                name: format!("Sector {coord}"),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            }),
            SectorRoot(coord),
            SectorStrengths(description.strengths),
            SectorFeatureSpheres(description.features),
        ))
        .id();

    for (body, rock) in description.asteroids.into_iter().zip(asteroid_geometry) {
        let mut entity = commands.spawn((
            base_scenario_object(&BaseScenarioObjectConfig {
                id: body.id.clone(),
                name: body.id.clone(),
                position: body.position,
                rotation: Quat::IDENTITY,
            }),
            ChildOf(root),
        ));
        asteroid_scenario_object_prepared(
            &mut entity,
            AsteroidConfig {
                radius: body.radius,
                texture: texture.clone(),
                kind: body.kind.to_string(),
                destroy_sound: None,
                mass: None,
                invulnerable: true,
                lock_signature: None,
                seed: Some(body.seed),
            },
            body.seed,
            rock,
        );
    }

    for (planet, surface) in description.planets.into_iter().zip(planet_surfaces) {
        let mut entity = commands.spawn((
            base_scenario_object(&BaseScenarioObjectConfig {
                id: planet.id.clone(),
                name: planet.id.clone(),
                position: planet.position,
                rotation: Quat::IDENTITY,
            }),
            ChildOf(root),
        ));
        planet_scenario_object_prepared(&mut entity, surface);
    }

    for hull in description.anchorages {
        commands.spawn((
            base_scenario_object(&BaseScenarioObjectConfig {
                id: hull.id.clone(),
                name: hull.id.clone(),
                position: hull.position,
                rotation: Quat::from_rotation_y(hull.yaw),
            }),
            spaceship_scenario_object(SpaceshipConfig {
                design: ShipDesignSource::Prototype {
                    id: hull.design.to_string(),
                    section_patches: BTreeMap::new(),
                },
                // Nobody aboard and nobody's side: a moored hull is scenery
                // with a hull, and an AI that shot it would be shooting the
                // furniture. The allegiance is inserted beside the bundle for
                // the same reason the scenario loader does it - the controller
                // marker's requirement default would otherwise decide.
                controller: SpaceshipController::None,
                allegiance: Some(Allegiance::Neutral),
                capabilities: ShipCapabilities::default(),
            }),
            Allegiance::Neutral,
            ChildOf(root),
        ));
    }

    debug!("world_sectors: materialized {coord} with {objects} object(s)");
    root
}

/// The live sector roots, keyed by cell.
///
/// # Panics
///
/// [`SectorFault::DuplicateRoot`] when two roots claim one cell. The live set
/// is read from the world every frame rather than mirrored in a resource, so
/// this is the only place the two could ever disagree - and a duplicate means a
/// retirement was missed, which is the leak the whole nested-ownership rule
/// exists to prevent.
pub fn live_sectors(roots: &Query<(Entity, &SectorRoot)>) -> BTreeMap<SectorCoord, Entity> {
    let mut live = BTreeMap::new();
    for (entity, root) in roots {
        if live.insert(root.0, entity).is_some() {
            panic!(
                "world_sectors: {}",
                SectorFault::DuplicateRoot { coord: root.0 }
            );
        }
    }
    live
}

/// Follow the observer: write which cell the scenario camera stands in.
///
/// # Panics
///
/// [`SectorFault::AbsentObserver`] unless there is exactly one scenario
/// camera. Streaming around a guessed centre would move the world without
/// saying so.
pub fn track_current_sector(
    mut commands: Commands,
    settings: Res<SectorSettings>,
    observer: Query<&GlobalTransform, With<ScenarioCameraMarker>>,
    current: Option<ResMut<CurrentSector>>,
) {
    let Ok(transform) = observer.single() else {
        panic!("world_sectors: {}", SectorFault::AbsentObserver);
    };
    // Engine boundary: a bevy transform counts world units, a cell is measured
    // in meters.
    let position = Meters3::from_engine(transform.translation());
    let coord = SectorCoord::containing(position, settings.edge);
    match current {
        Some(mut current) => {
            if current.0 != coord {
                debug!("world_sectors: the observer crossed into {coord}");
                current.0 = coord;
            }
        }
        None => {
            commands.insert_resource(CurrentSector(coord));
        }
    }
}

/// One cell's preparation, running on the entity that owns it.
///
/// An entity rather than a resource entry so a job is a thing the world can be
/// asked about, and so dropping it is how it is cancelled: `Task`'s own `Drop`
/// cancels the work, and despawning the entity is what [`retire_sectors`] and
/// [`clear_sector_work`] both do.
#[derive(Component)]
pub struct SectorJob {
    /// The cell being prepared.
    pub coord: SectorCoord,
    /// The running preparation. Private: a job is started one way, through
    /// [`SectorJob::start`], so a task can never disagree with the coordinate
    /// beside it.
    task: Task<Result<PreparedSector, SectorFault>>,
}

impl SectorJob {
    /// Start one cell's preparation on `AsyncComputeTaskPool`.
    ///
    /// The settings are COPIED into the task rather than read from the
    /// resource when it finishes: a job answers the question it was asked, and
    /// a dial changed mid-flight must not silently re-aim work already in the
    /// air.
    pub fn start(coord: SectorCoord, settings: SectorSettings) -> Self {
        let task = AsyncComputeTaskPool::get()
            .spawn(async move { prepare_sector(WORLD_SEED, coord, settings) });
        Self { coord, task }
    }
}

/// Sectors that finished preparing and are waiting for a frame to be spawned
/// in.
///
/// A `BTreeMap`, so the set a frame picks its one materialization out of is
/// ordered before it is ranked and which worker finished first cannot reach
/// the decision.
#[derive(Resource, Default, Debug)]
pub struct ReadySectors(pub BTreeMap<SectorCoord, PreparedSector>);

/// What the job lifetime has done since the app started.
///
/// Session-lifetime totals, not a live gauge: they are how a reader (and the
/// range) can tell that a live sector was REQUESTED and PREPARED first rather
/// than conjured inside the frame that needed it. Never reset, including
/// across an unload, because the evidence is the point.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectorJobStats {
    /// Jobs started.
    pub requested: usize,
    /// Jobs that came back with a result.
    pub completed: usize,
    /// Prepared sectors spawned into the world.
    pub materialized: usize,
    /// Jobs and prepared sectors dropped without being spawned: cancelled,
    /// finished after nobody wanted them, or swept by an unload.
    pub discarded: usize,
    /// The most jobs in flight at once, sampled before each poll. Above one it
    /// is the direct evidence that preparation outlives the frame that asked
    /// for it.
    pub peak_pending: usize,
}

/// Start jobs for the desired cells that are not already live, running or
/// prepared - NEAREST FIRST, and never more at once than the task pool has
/// threads.
///
/// The cap is `AsyncComputeTaskPool::get().thread_num().max(1)`, which is the
/// number of preparations the machine can actually be running; the `max(1)`
/// covers a single-threaded pool, which is what wasm has. A 125-cell window
/// asks for far more work than that, and the excess is simply NOT REQUESTED -
/// not queued, not deferred, no job entity. Nothing carries it, so a cell that
/// stopped being desired while the window filled was never work anybody has to
/// cancel, and the frame a slot opens asks for whichever cell is nearest BY
/// THEN rather than whichever one was nearest when the observer was somewhere
/// else.
///
/// Nearest first is squared integer cell distance from [`CurrentSector`], with
/// the coordinate breaking ties, so the cells the observer is standing in and
/// flying through are prepared before the corners of the window and two cells
/// the same distance out are always asked for in the same order.
///
/// # Panics
///
/// Through [`live_sectors`] on a duplicate root.
pub fn request_sectors(
    mut commands: Commands,
    settings: Res<SectorSettings>,
    current: Res<CurrentSector>,
    roots: Query<(Entity, &SectorRoot)>,
    jobs: Query<&SectorJob>,
    ready: Res<ReadySectors>,
    mut stats: ResMut<SectorJobStats>,
) {
    let running: BTreeSet<SectorCoord> = jobs.iter().map(|job| job.coord).collect();
    let openings = AsyncComputeTaskPool::get()
        .thread_num()
        .max(1)
        .saturating_sub(running.len());
    if openings == 0 {
        return;
    }

    let live = live_sectors(&roots);
    let centre = current.0;
    let mut missing: Vec<SectorCoord> = desired_sectors(centre, settings.radius)
        .into_iter()
        .filter(|coord| {
            !live.contains_key(coord) && !running.contains(coord) && !ready.0.contains_key(coord)
        })
        .collect();
    missing.sort_by_key(|coord| {
        let x = i128::from(coord.x) - i128::from(centre.x);
        let y = i128::from(coord.y) - i128::from(centre.y);
        let z = i128::from(coord.z) - i128::from(centre.z);
        (x * x + y * y + z * z, *coord)
    });

    for coord in missing.into_iter().take(openings) {
        debug!("world_sectors: requesting {coord}");
        commands.spawn((
            Name::new(format!("Sector Job {coord}")),
            SectorJob::start(coord, *settings),
        ));
        stats.requested += 1;
    }
}

/// Take in what the workers finished, WITHOUT waiting on anything.
///
/// A completion for a cell that is no longer desired is dropped here rather
/// than kept: the observer moved while the job was in the air, and spawning it
/// now would put a sector behind the player that the next frame has to retire.
///
/// # Panics
///
/// On any [`SectorFault`] a worker returns. The fault crosses back to the main
/// thread as a value and fails HERE, where it can name the cell, rather than
/// poisoning a pool thread.
pub fn collect_sector_jobs(
    mut commands: Commands,
    settings: Res<SectorSettings>,
    current: Res<CurrentSector>,
    mut jobs: Query<(Entity, &mut SectorJob)>,
    mut ready: ResMut<ReadySectors>,
    mut stats: ResMut<SectorJobStats>,
) {
    stats.peak_pending = stats.peak_pending.max(jobs.iter().len());
    let desired = desired_sectors(current.0, settings.radius);

    for (entity, mut job) in &mut jobs {
        let Some(result) = block_on(poll_once(&mut job.task)) else {
            continue;
        };
        let coord = job.coord;
        commands.entity(entity).despawn();
        stats.completed += 1;

        let prepared = result.unwrap_or_else(|fault| panic!("world_sectors: {fault}"));
        if !desired.contains(&coord) {
            debug!("world_sectors: dropping the finished job for {coord}, it is no longer desired");
            stats.discarded += 1;
            continue;
        }
        // A `BTreeMap` insert, so the order the world hands jobs back in
        // cannot reach the decision the next system takes.
        ready.0.insert(coord, prepared);
    }
}

/// Spawn AT MOST ONE prepared sector, NEAREST desired cell first.
///
/// One a frame is the whole frame policy: the expensive half already happened
/// on a worker, and what is left is a command batch per body that the main
/// thread has to own. Nearest first - the same squared cell distance and
/// coordinate tie-break [`request_sectors`] asks in - rather than
/// first-prepared-first or lowest-coordinate-first, so the world fills out
/// from where the observer stands instead of from the low corner of the
/// window, and the order sectors appear in is a fact about the set rather than
/// about which worker won.
///
/// # Panics
///
/// Through [`live_sectors`] on a duplicate root.
pub fn materialize_ready_sector(
    mut commands: Commands,
    settings: Res<SectorSettings>,
    current: Res<CurrentSector>,
    roots: Query<(Entity, &SectorRoot)>,
    mut ready: ResMut<ReadySectors>,
    mut stats: ResMut<SectorJobStats>,
    game_assets: Res<GameAssets>,
    designs: Res<GameShipDesigns>,
) {
    let desired = desired_sectors(current.0, settings.radius);
    let live = live_sectors(&roots);
    let centre = current.0;
    // Skipping a cell that is already live is what keeps the second root that
    // `live_sectors` panics on from ever being spawned. It costs a lookup and
    // it is the only thing standing between a hand-fed prepared sector and a
    // duplicate.
    let next = ready
        .0
        .keys()
        .copied()
        .filter(|coord| desired.contains(coord) && !live.contains_key(coord))
        .min_by_key(|coord| {
            let x = i128::from(coord.x) - i128::from(centre.x);
            let y = i128::from(coord.y) - i128::from(centre.y);
            let z = i128::from(coord.z) - i128::from(centre.z);
            (x * x + y * y + z * z, *coord)
        });
    let Some(prepared) = next.and_then(|coord| ready.0.remove(&coord)) else {
        return;
    };

    let texture: AssetRef<Image> = game_assets.asteroid_texture.clone().into();
    materialize_sector(&mut commands, prepared, &texture, &designs);
    stats.materialized += 1;
}

/// Take back everything outside the desired set: live roots, running jobs, and
/// prepared sectors nobody asked for any more.
///
/// # Panics
///
/// Through [`live_sectors`] on a duplicate root.
pub fn retire_sectors(
    mut commands: Commands,
    settings: Res<SectorSettings>,
    current: Res<CurrentSector>,
    roots: Query<(Entity, &SectorRoot)>,
    jobs: Query<(Entity, &SectorJob)>,
    mut ready: ResMut<ReadySectors>,
    mut stats: ResMut<SectorJobStats>,
) {
    let desired = desired_sectors(current.0, settings.radius);

    for (coord, entity) in &live_sectors(&roots) {
        if !desired.contains(coord) {
            debug!("world_sectors: retiring {coord}");
            commands.entity(*entity).despawn();
        }
    }

    for (entity, job) in &jobs {
        if !desired.contains(&job.coord) {
            debug!("world_sectors: cancelling the job for {}", job.coord);
            commands.entity(entity).despawn();
            stats.discarded += 1;
        }
    }

    ready.0.retain(|coord, _| {
        let wanted = desired.contains(coord);
        if !wanted {
            debug!("world_sectors: dropping the prepared sector {coord}");
            stats.discarded += 1;
        }
        wanted
    });
}

/// Drop the work the session no longer owns.
///
/// Sector roots are scenario objects and the scenario sweep takes them. A
/// pending [`SectorJob`] and a prepared [`ReadySectors`] payload are NOT, so
/// nothing else can: without this an unloaded session would leave workers
/// running, and the next session would materialize sectors the previous one
/// asked for.
///
/// Runs on both ways a session ends, which is why its condition is not just
/// `not(scenario_is_live)`: `LoadScenario` over a LIVE scenario swaps the two
/// inside one observer call, so there is no frame where liveness is false to
/// catch it. It runs before the streaming stages, so a session that is
/// replaced and re-armed in one frame cannot spawn the old session's prepared
/// sectors into the new one.
pub fn clear_sector_work(
    mut commands: Commands,
    jobs: Query<Entity, With<SectorJob>>,
    mut ready: ResMut<ReadySectors>,
    mut stats: ResMut<SectorJobStats>,
) {
    if jobs.is_empty() && ready.0.is_empty() {
        return;
    }
    debug!(
        "world_sectors: the session is gone, dropping {} job(s) and {} prepared sector(s)",
        jobs.iter().len(),
        ready.0.len()
    );
    stats.discarded += jobs.iter().len() + ready.0.len();
    for entity in &jobs {
        commands.entity(entity).despawn();
    }
    ready.0.clear();
}

/// The free-play bootstrap: an EMPTY scenario.
///
/// No objects, no handlers, no beat list - it exists to give the session a
/// skybox, a camera and a live scenario lifetime, and everything after that is
/// Bevy-owned. Deliberately NOT paired with `assert_scenario_loaded`, whose
/// smoke contract fails a scenario that spawns zero objects: here that is the
/// claim, not the fault.
pub fn free_play_scenario(game_assets: &GameAssets, id: &str, name: &str) -> ScenarioConfig {
    ScenarioConfig {
        description: "Free play: an empty session the world plugin streams into".to_string(),
        ..ScenarioConfig::new(
            id.to_string(),
            name.to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The streaming loop, in `Update` behind a live scenario and an inserted
/// [`SectorSettings`].
///
/// Ordering inside the frame, chained so bevy applies each stage's commands
/// before the next one queries:
///
/// [`clear_sector_work`] -> `track_current_sector` -> [`request_sectors`] ->
/// [`collect_sector_jobs`] -> [`materialize_ready_sector`] ->
/// [`retire_sectors`]
///
/// The session check is first, so no frame can hand a new session work the
/// previous one asked for. The observer is read next, so a crossing is acted
/// on in the frame it is noticed; requesting before polling is what lets a job
/// be started and collected in the same frame if a worker is that fast;
/// retiring last is what keeps a sector materialized this frame from being
/// taken back by the same frame that made it. A job that finishes or is
/// cancelled frees its slot for the NEXT frame's [`request_sectors`], which is
/// how a capped window walks through a 125-cell set a few sectors at a time
/// instead of stalling once the first batch lands.
///
/// [`clear_sector_work`] is the only stage NOT gated on the session being
/// live: it runs while no scenario is live, and it also runs on the frame a
/// live session is replaced - ahead of the streaming stages, which serve the
/// NEW session on that same frame. Everything else is gated on the session,
/// which is also what stops the loop from rebuilding the world the frame after
/// `UnloadScenario` swept it.
pub struct WorldSectorsPlugin;

impl Plugin for WorldSectorsPlugin {
    fn build(&self, app: &mut App) {
        trace!("WorldSectorsPlugin: build");

        app.init_resource::<ReadySectors>();
        app.init_resource::<SectorJobStats>();

        app.add_systems(
            Update,
            (
                clear_sector_work
                    .run_if(not(scenario_is_live).or_else(resource_changed::<CurrentScenario>)),
                (
                    track_current_sector.run_if(resource_exists::<SectorSettings>),
                    (
                        request_sectors,
                        collect_sector_jobs,
                        materialize_ready_sector,
                        retire_sectors,
                    )
                        .chain()
                        .run_if(
                            resource_exists::<SectorSettings>
                                .and_then(resource_exists::<CurrentSector>),
                        ),
                )
                    .chain()
                    .run_if(scenario_is_live),
            )
                .chain(),
        );
    }
}
