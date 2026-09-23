//! The base game's feature field: where its asteroid fields, planetoids and
//! derelict fields are.
//!
//! PURE, like every generator: nothing here touches a `World`, reads a
//! resource or draws from the ambient RNG.
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
use nova_world::prelude::*;

use crate::PLACEMENT_INSET;

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
    pub(crate) const fn radius_band(self) -> (Meters, Meters) {
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
const FEATURE_LATTICE: Meters = Meters(128_000.0);

/// The macro wavelength of every layer's field: how far apart two independent
/// readings of the same layer are.
///
/// Two lattice spacings, so a field decides REGIONS of candidate nodes
/// together - a run of empty nodes, then a run of accepted ones - rather than
/// flipping a coin at each node. A wavelength at or below the lattice would
/// make the noise a second random number per node and the field would buy
/// nothing over a hash.
const FEATURE_WAVELENGTH: Meters = Meters(256_000.0);

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
const FEATURE_CEILING: f32 = 0.22;

/// How many lattice nodes out the same-layer thinning looks.
///
/// DERIVED, not chosen: see the crate-private `feature_halo_covers_overlap`.
/// Two same-layer spheres overlap only if their centres are within
/// `2 * FEATURE_RADIUS_MAX`, and a centre sits within one jitter draw plus one
/// inset pull of its node, so a node further out than this cannot hold a
/// rival. The inset pull grows with the cell edge, which is why
/// [`validate_feature_geometry`] refuses too wide an edge rather than thinning
/// against a halo that no longer reaches.
const FEATURE_HALO: i32 = 2;

/// Whether [`FEATURE_HALO`] really covers every node that can hold an
/// overlapping same-layer rival.
///
/// The arithmetic the halo constant is derived from, written as a function so
/// it is checked rather than asserted in a comment.
/// [`validate_feature_geometry`] calls it in every build, so raising a radius
/// band or the jitter without widening the halo refuses at the config instead
/// of silently letting two belts overlap, and an authored cell edge too wide
/// for the halo is refused before a cell is described.
fn feature_halo_covers_overlap(edge: Meters) -> bool {
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
    /// [`SectorFault::Generation`] when the field returns a non-finite
    /// reading. A `NaN` compared against a threshold is silently false, which
    /// would make a whole region of the world quietly empty.
    pub fn sample(&self, layer: FeatureLayer, position: Meters3) -> Result<f32, SectorFault> {
        let point = (position + FEATURE_FIELD_ORIGIN).get();
        let value =
            self.0[layer.index()].get([f64::from(point.x), f64::from(point.y), f64::from(point.z)])
                as f32;
        if !value.is_finite() {
            let at = position.get();
            return Err(SectorFault::Generation {
                id: format!("feature_field_{layer}"),
                field: "reading",
                value: format!("{value} at {:.0} {:.0} {:.0} m", at.x, at.y, at.z),
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
    pub(crate) fn influence(&self, position: Meters3) -> f32 {
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
            Err(SectorFault::Generation {
                id: self.id.clone(),
                field,
                value,
            })
        };
        if !position_is_finite(self.centre) || !self.radius.get().is_finite() {
            return Err(SectorFault::InvalidGeometry {
                id: self.id.clone(),
            });
        }
        if self.radius.get() <= 0.0 {
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

fn position_is_finite(position: Meters3) -> bool {
    position.get().is_finite()
}

/// One signed lattice index as a snake_case-safe slug. `n` reads as minus,
/// because a `-` in an id is not a slug.
fn node_slug(index: i32) -> String {
    if index < 0 {
        format!("n{}", index.unsigned_abs())
    } else {
        index.to_string()
    }
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
            node_slug(node[0]),
            node_slug(node[1]),
            node_slug(node[2])
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
pub(crate) fn validate_feature_geometry(geometry: WorldGeometry) -> Result<(), SectorFault> {
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
/// [`SectorFault::Config`] on `sector_edge` for an edge that is not a finite
/// positive length or that the thinning halo cannot reach across, and
/// [`SectorFault::InvalidGeometry`] for a cell whose node range runs off the
/// lattice. Plus whatever the field or a candidate refuses, and
/// [`SectorFault::DuplicateId`] if two spheres ever claim one id.
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
                        return Err(SectorFault::DuplicateId { id: sphere.id });
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

/// The combined influence of each layer at `centre`, indexed by
/// [`FeatureLayer::index`] and capped at one.
///
/// What [`crate::NovaLayeredWorld`] fills a cell from: the asteroid entry,
/// taken at the cell's centre, sets how many rocks the cell holds. Two spheres
/// covering one point add, so a place where belts meet is denser than either.
/// Public so a diagnostic reads the same number the generator used.
pub fn sector_strengths(features: &[FeatureSphere], centre: Meters3) -> [f32; FeatureLayer::COUNT] {
    let mut strengths = [0.0; FeatureLayer::COUNT];
    for sphere in features {
        strengths[sphere.layer.index()] += sphere.influence(centre);
    }
    for strength in &mut strengths {
        *strength = strength.min(1.0);
    }
    strengths
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    /// The examples' seed, edge and home window: the one near the origin that
    /// holds shared spheres, same-layer neighbours and blended cells at once,
    /// so none of the claims below is vacuous.
    fn home_window() -> Vec<SectorGenerationInput> {
        let geometry = WorldGeometry {
            sector_edge: Meters(32_000.0),
        };
        desired_sectors(SectorCoord::new(2, 1, 2), 2)
            .into_iter()
            .map(|coord| SectorGenerationInput {
                seed: 20_260_922,
                geometry,
                coord,
            })
            .collect()
    }

    fn features_of(input: SectorGenerationInput) -> Vec<FeatureSphere> {
        sector_features(input).unwrap_or_else(|fault| panic!("{}: {fault}", input.coord))
    }

    /// Every distinct sphere the home window sees, by id.
    fn window_spheres() -> BTreeMap<String, FeatureSphere> {
        home_window()
            .into_iter()
            .flat_map(features_of)
            .map(|sphere| (sphere.id.clone(), sphere))
            .collect()
    }

    /// A sphere is a function of its lattice node, so no walk can change
    /// what a cell reads.
    #[test]
    fn a_cell_reads_the_same_spheres_in_any_visit_order() {
        let cells = home_window();
        let read_all = |order: &[SectorGenerationInput]| {
            order
                .iter()
                .map(|input| (input.coord, features_of(*input)))
                .collect::<BTreeMap<_, _>>()
        };
        let forward = read_all(&cells);
        let reverse = read_all(&cells.iter().rev().copied().collect::<Vec<_>>());
        let strided = read_all(
            &(0..cells.len())
                .map(|step| cells[step * 7 % cells.len()])
                .collect::<Vec<_>>(),
        );
        assert_eq!(forward.len(), 125);
        assert_eq!(forward, reverse, "a reverse walk changed a cell's spheres");
        assert_eq!(forward, strided, "a strided walk changed a cell's spheres");
    }

    /// Two neighbouring cells agree about a sphere neither of them has to
    /// own, and only the cell its centre falls in owns it.
    #[test]
    fn a_sphere_reads_the_same_from_every_cell_it_reaches_and_has_one_owner() {
        let mut seen: BTreeMap<String, Vec<(SectorCoord, FeatureSphere)>> = BTreeMap::new();
        for input in home_window() {
            for sphere in features_of(input) {
                seen.entry(sphere.id.clone())
                    .or_default()
                    .push((input.coord, sphere));
            }
        }
        let mut shared = 0;
        for (id, copies) in &seen {
            let (first_cell, first) = &copies[0];
            for (cell, other) in &copies[1..] {
                assert_eq!(
                    first, other,
                    "'{id}' reads differently in {cell} and {first_cell}"
                );
            }
            if copies.len() > 1 {
                shared += 1;
            }
            let owners = copies
                .iter()
                .filter(|(cell, sphere)| sphere.owner == *cell)
                .count();
            assert!(
                owners <= 1,
                "'{id}' is owned by {owners} of the cells that see it"
            );
        }
        assert!(
            shared > 0,
            "no sphere crosses a cell boundary in the home window"
        );
    }

    /// Same-layer crowding is thinned by rank, so no two accepted spheres of
    /// one layer claim the same ground.
    #[test]
    fn same_layer_spheres_never_overlap() {
        let spheres: Vec<FeatureSphere> = window_spheres().into_values().collect();
        let mut pairs = 0;
        for (index, a) in spheres.iter().enumerate() {
            for b in spheres[index + 1..].iter().filter(|b| b.layer == a.layer) {
                assert!(
                    !a.overlaps(b),
                    "{} spheres '{}' and '{}' overlap",
                    a.layer,
                    a.id,
                    b.id
                );
                pairs += 1;
            }
        }
        assert!(pairs > 0, "no two spheres share a layer in the home window");
    }

    /// The layers are independent: spheres of different layers do overlap,
    /// which is what makes a blended place rather than a mosaic.
    #[test]
    fn spheres_of_different_layers_may_overlap() {
        let spheres: Vec<FeatureSphere> = window_spheres().into_values().collect();
        let crossing = spheres
            .iter()
            .enumerate()
            .flat_map(|(index, a)| spheres[index + 1..].iter().map(move |b| (a, b)))
            .filter(|(a, b)| a.layer != b.layer && a.overlaps(b))
            .count();
        assert!(
            crossing > 0,
            "no two layers overlap anywhere in the home window"
        );
    }

    /// A cell whose node range runs off the lattice is refused, not clipped.
    ///
    /// A clipped range would sweep the whole lattice from one far cell and
    /// answer for ground that cell never reaches. 400 km is inside the
    /// thinning halo's span, so the geometry itself is valid; it is the CELL,
    /// out at the end of the i32 grid, that has no node range anyone can
    /// address.
    #[test]
    fn a_feature_query_that_runs_off_the_node_lattice_is_refused() {
        let geometry = WorldGeometry {
            sector_edge: Meters(400_000.0),
        };
        validate_feature_geometry(geometry)
            .expect("a 400 km cell is inside the thinning halo's reach");
        let fault = sector_features(SectorGenerationInput {
            seed: 20_260_922,
            geometry,
            coord: SectorCoord::new(i32::MAX, 0, 0),
        })
        .expect_err("a cell at the end of the grid has no representable node range");
        assert!(
            matches!(&fault, SectorFault::InvalidGeometry { .. }),
            "a node range off the lattice must refuse, got {fault:?}"
        );
    }

    /// In EVERY build, not a debug assertion: a release run that accepted
    /// this edge would inspect the same 2-node halo and ship a world where two
    /// same-layer spheres outside it both survive the thinning.
    #[test]
    fn a_feature_edge_wider_than_the_thinning_halo_is_refused() {
        let fault = sector_features(SectorGenerationInput {
            seed: 20_260_922,
            geometry: WorldGeometry {
                sector_edge: Meters(500_000.0),
            },
            coord: SectorCoord::ORIGIN,
        })
        .expect_err("a cell edge the thinning halo cannot reach across must refuse");
        assert!(
            matches!(
                fault,
                SectorFault::Config {
                    field: "sector_edge",
                    ..
                }
            ),
            "the halo refusal must name the edge, got {fault:?}"
        );
    }
}
