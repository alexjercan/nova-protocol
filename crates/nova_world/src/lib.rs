//! `nova_world` is the streamed open world: a deterministic sector generator
//! and the bounded job lifetime that brings its cells up and takes them away
//! again while ONE scenario stays loaded.
//!
//! # Opt-in, and not in the stack
//!
//! [`NovaWorldPlugin`] is not added by `AppBuilder`. A caller adds it, and
//! even then the streaming systems do nothing until a [`WorldConfig`] resource
//! is inserted - that resource is the arming switch, which is what lets a
//! caller assert an empty session before any sector exists.
//!
//! # What a cell is FILLED with
//!
//! A [`SectorGenerator`] is the one thing two worlds can disagree about; the
//! window, the edge and the whole job lifetime are shared. The generator is a
//! TYPE, named by [`WorldConfig<G>`] and [`NovaWorldPlugin<G>`], and an app
//! installs exactly one: a world of another generator is another app, not a
//! value swapped at runtime.
//!
//! This crate owns the mechanisms and none of the content. The feature field
//! is here: three independent global noise fields gate candidate
//! [`FeatureSphere`]s on a coarse lattice, the spheres are pure data addressed
//! by lattice node rather than by sector, and a cell asks which of them reach
//! it ([`sector_features`]) - which is what makes two neighbouring cells agree
//! about a belt that crosses both of them without either one owning it. What
//! a cell holds, and where each object stands in it, is the generator's
//! policy: its placement, retries, id bookkeeping and rock draw are its own.
//! The base game's generator lives in `nova_authoring`, and the uniform
//! streaming baseline lives with the examples. The shared primitives are
//! stateless: the cell's seeded streams, [`sector_id`], the edge floor,
//! [`bodies_clear`] and the spacing constants the check below holds every
//! generator to.
//!
//! A generator's answer is not trusted. It returns a [`SectorManifest`], and
//! [`validate_manifest`] is the only way to turn one into the
//! [`SectorDescription`] preparation and materialization accept: it checks the
//! cell's own edge and centre, the requested cell, the measured sphere, body
//! and rock caps, finite geometry, unique ids the cell owns, bodies wholly
//! inside the cell and clear of each other, shipped asteroid kinds, and
//! feature references the cell owns - before a worker prepares anything. A
//! ship's design is a key into the ship catalog, which is a Bevy resource, so
//! the check can only refuse a blank one. The catalog lookup happens on the
//! main thread in [`materialize_sector`], where an id the game does not ship
//! is a [`SectorFault::UnknownShip`] panic - loud, and before the cell's
//! entities exist, but at materialization and not at arming.
//!
//! # The job lifetime
//!
//! Nothing is described and spawned in the same breath. A desired cell that is
//! not live becomes a [`SectorJob`]; the job describes, validates and meshes
//! the whole sector on `AsyncComputeTaskPool`; and the main thread spends its
//! frame only on the one step a worker cannot take, which is turning a
//! [`PreparedSector`] into entities. Six explicit stages, ordered by
//! [`NovaWorldSystems`]:
//!
//! 1. [`NovaWorldSystems::Cleanup`] drops the work the session no longer owns.
//!    It is the only stage NOT gated on the session being live: it runs while
//!    no scenario is live, it runs on the frame a live session is replaced,
//!    and it runs on the frame the [`WorldConfig`] itself is replaced or
//!    removed - all of them ahead of the streaming stages, which serve the NEW
//!    world on that same frame. So no frame can hand a new world work the
//!    previous one asked for.
//! 2. [`NovaWorldSystems::Observe`] writes which cell the [`WorldObserver`]
//!    stands in, and refuses a newly armed config. The generator validates
//!    too, but on a worker, which is one job too late for a window the main
//!    thread enumerates whole in the very next stage.
//! 3. [`NovaWorldSystems::Request`] starts jobs for the desired cells that are
//!    not already live, running or prepared, NEAREST FIRST and only as many as
//!    the task pool has threads. The rest of the window stays unrequested -
//!    not queued and not deferred - until a slot opens.
//! 4. [`NovaWorldSystems::Collect`] polls WITHOUT blocking. A [`SectorFault`]
//!    from a worker is fatal here, on the main thread, where it can name
//!    itself; a completion for a cell nobody wants any more is dropped.
//! 5. [`NovaWorldSystems::Materialize`] spawns AT MOST ONE prepared sector,
//!    nearest first, so a large window costs one frame of spawning per cell
//!    instead of one frame of all of it.
//! 6. [`NovaWorldSystems::Retire`] takes back roots, running jobs and prepared
//!    results that fall outside the desired set.
//!
//! Every decision is taken over a TOTAL ORDER - distance from the observer's
//! cell first, the coordinate itself breaking ties - read out of ordered
//! collections. So which worker finished first can never change which sector
//! is requested or which one a frame spends its budget on.
//!
//! On wasm `AsyncComputeTaskPool` is the page's own task queue on the one
//! thread the page has, so the split buys no parallelism there. It still buys
//! the frame and the cancellation.
//!
//! # Who owns what
//!
//! Three owners, nested rather than competing:
//!
//! - the scenario owns the session. Every sector root carries
//!   `ScenarioScopedMarker`, so `UnloadScenario` is the final sweep and cannot
//!   leave a sector behind.
//! - [`SectorRoot`] owns one sector. Retiring it despawns that sector's
//!   bodies, planetoids and ships and nothing else. A feature sphere is
//!   owned by exactly ONE cell ([`FeatureSphere::owner`], the cell its centre
//!   falls in) even where the sphere reaches across a dozen of them, so a
//!   planetoid is spawned once and retired once.
//! - the plugin owns the WORK. A pending [`SectorJob`] and a prepared
//!   [`ReadySectors`] payload are not scenario objects and the scenario sweep
//!   cannot see them, so [`NovaWorldSystems::Cleanup`] drops them the moment
//!   the session stops being THIS session. That is two events, not one: an
//!   unload, and a `LoadScenario` that replaces a live scenario. The second
//!   never passes through a no-scenario frame - `on_load_scenario` tears the
//!   old session down and writes the new `CurrentScenario` in one observer
//!   call - so liveness alone cannot see it and the condition reads
//!   `CurrentScenario` CHANGING as well. Without that a replaced session would
//!   materialize sectors the session before it asked for.
//!
//! # Replacing the world under a live session
//!
//! A [`WorldConfig`] is a resource a caller can swap, and a job carries the
//! config it was STARTED with, so a swap is a third way the work on hand stops
//! belonging to the world that asked for it. Nothing downstream could catch
//! it: a root, a running job and a prepared payload are all keyed by
//! coordinate alone, so an old seed, edge or generator value would materialize
//! into the new world looking exactly like the new world's own cell.
//!
//! So a swap is a CLEAR SESSION, not a merge. On the frame the resource is
//! inserted, replaced or removed, [`NovaWorldSystems::Cleanup`] retires every
//! live [`SectorRoot`] and drops every job and prepared result - and because
//! the stages are chained, all of that lands before
//! [`NovaWorldSystems::Request`] asks the new world for anything. The window
//! refills from the new config over the following frames at the usual one
//! sector a frame.
//!
//! That buys the caller ONE ordering rule: **write [`WorldConfig`] ahead of
//! [`NovaWorldSystems::Cleanup`]**. A run condition is evaluated before its
//! set, so `Cleanup` decides whether to clear at the top of the frame; a write
//! that lands behind that decision is not seen until the NEXT frame, and the
//! frame in between would stream the old world's roots and payloads under the
//! new config. Every stage from [`NovaWorldSystems::Observe`] down refuses
//! such a frame rather than mixing the two, and names the rule when it does.
//!
//! A writer in `PreUpdate`, in `Startup`, or in a state-transition schedule
//! such as `OnEnter` is already ahead of the rule, because all of those run
//! before `Update`. A writer inside `Update` must say so:
//!
//! ```
//! # use bevy::prelude::*;
//! # use nova_world::prelude::*;
//! # fn arm_the_world(mut _commands: Commands) {}
//! # let mut app = App::new();
//! app.add_systems(Update, arm_the_world.before(NovaWorldSystems::Cleanup));
//! ```
#![warn(missing_docs)]

use std::{any::type_name, fmt::Debug, marker::PhantomData};

use bevy::{ecs::change_detection::CheckChangeTicks, prelude::*};
use nova_events::prelude::{Meters, Meters3};
use nova_gameplay::prelude::{Fnv32, SeedStream};
use nova_scenario::prelude::{scenario_is_live, CurrentScenario};

mod generation;
mod streaming;

#[cfg(test)]
mod tests;

pub use crate::{
    generation::{
        bodies_clear, generate_sector, prepare_sector, sector_features, sector_id,
        validate_feature_geometry, validate_manifest, FeatureFields, FeatureLayer, FeatureSphere,
        PreparedSector, SectorAsteroid, SectorDescription, SectorManifest, SectorPlanet,
        SectorShip, CLEARANCE_MARGIN, FEATURE_HALO, FEATURE_LATTICE, FEATURE_WAVELENGTH,
        SECTOR_ASTEROIDS_MAX, SECTOR_BODIES_MAX, SECTOR_FEATURES_MAX, SECTOR_SHIP_CLEARANCE,
    },
    streaming::{
        clear_sector_work, collect_sector_jobs, desired_sectors, live_sectors,
        materialize_ready_sector, materialize_sector, request_sectors, retire_sectors,
        track_current_sector, CurrentSector, ReadySectors, SectorFeatureSpheres, SectorJob,
        SectorJobStats, SectorRoot, SectorStrengths, WorldObserver,
    },
};

/// Glob-import surface: `use nova_world::prelude::*` brings the config, the
/// generator interface and the shared primitives it draws on, the streaming components
/// and the plugin into scope.
pub mod prelude {
    pub use super::{
        bodies_clear, generate_sector, prepare_sector, sector_features, sector_id,
        validate_feature_geometry, validate_manifest, FeatureFields, FeatureLayer, FeatureSphere,
        NovaWorldPlugin, NovaWorldSystems, PreparedSector, SectorAsteroid, SectorCoord,
        SectorDescription, SectorFault, SectorGenerationInput, SectorGenerator, SectorManifest,
        SectorPlanet, SectorShip, WorldConfig, WorldGeometry, ACTIVE_WINDOW_SECTORS_MAX,
        CLEARANCE_MARGIN, FEATURE_HALO, FEATURE_LATTICE, FEATURE_WAVELENGTH, PLACEMENT_INSET,
        SECTOR_ASTEROIDS_MAX, SECTOR_BODIES_MAX, SECTOR_FEATURES_MAX, SECTOR_SHIP_CLEARANCE,
    };
    pub use crate::streaming::{
        desired_sectors, CurrentSector, ReadySectors, SectorFeatureSpheres, SectorJob,
        SectorJobStats, SectorRoot, SectorStrengths, WorldObserver,
    };
}

/// How much of a sector's half-edge a PHYSICAL object may be placed along.
///
/// Objects are owned by a cell, so they have to stay inside it: a rock placed
/// at the face would be half in the neighbour, and retiring the neighbour
/// would look like retiring the wrong sector. It applies to a feature-owned
/// planetoid too, which is why a feature sphere's centre is pulled onto its
/// owner's inset rather than clamped there after the fact.
///
/// An inset constrains a CENTRE, so it owns the body only while the body fits
/// in the margin it leaves. [`WorldGeometry::require_owning_edge`] is how a
/// generator refuses a `sector_edge` under `2 * clearance / (1 -
/// PLACEMENT_INSET)` for the widest body it can draw, and [`validate_manifest`]
/// refuses any body whose clearance crosses a face, which is what makes the
/// sentence above true rather than aspirational.
pub const PLACEMENT_INSET: f32 = 0.7;

/// The most cells one desired window may hold.
///
/// 125, which is the 5x5x5 window this crate has actually measured, and not a
/// round number with headroom in it. The desired set is built as a WHOLE set
/// by three stages of every frame, so the cost of `active_radius` is cubic in
/// a field a caller types: radius 1,000 is eight billion coordinates and the
/// job cap never gets a chance to help. A wider production window is a new
/// measurement and a deliberate change to this constant, not something a
/// config can reach at runtime.
pub const ACTIVE_WINDOW_SECTORS_MAX: usize = 125;

/// How many cells a window of this radius holds, refusing one nobody can
/// afford to enumerate.
///
/// CHECKED, and the one place the arithmetic lives: `(2 * radius + 1)^3`
/// leaves `i32` at radius 812 and `u64` well before `i32::MAX`, so the count
/// has to be the thing that is bounded rather than the radius. Both callers
/// need it for the same reason and at different moments -
/// [`WorldConfig::validate`] refuses a config, and [`desired_sectors`] is the
/// allocation itself and cannot be reached with a config it never saw.
///
/// # Errors
///
/// [`SectorFault::Config`] on a negative radius, on a count that overflows,
/// and on any window above [`ACTIVE_WINDOW_SECTORS_MAX`].
pub(crate) fn window_cells(radius: i32) -> Result<usize, SectorFault> {
    let refuse = |value: String| SectorFault::Config {
        field: "active_radius",
        value,
    };
    if radius < 0 {
        return Err(refuse(radius.to_string()));
    }
    let side = i64::from(radius) * 2 + 1;
    match usize::try_from(side)
        .ok()
        .and_then(|side| side.checked_pow(3))
    {
        Some(cells) if cells <= ACTIVE_WINDOW_SECTORS_MAX => Ok(cells),
        Some(cells) => Err(refuse(format!(
            "{radius}, a desired window of {cells} cells, above the \
             {ACTIVE_WINDOW_SECTORS_MAX} cell maximum"
        ))),
        None => Err(refuse(format!(
            "{radius}, a desired window of more cells than this machine can count, above the \
             {ACTIVE_WINDOW_SECTORS_MAX} cell maximum"
        ))),
    }
}

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
    ///
    /// `floor` after the half-cell shift rather than `round`: `f32::round`
    /// goes half AWAY FROM ZERO, so it put `-edge/2` in cell -1 while the span
    /// above says cell 0 owns it. That broke the interval on the negative face
    /// only, and the face a sphere's centre lands on decides which cell owns
    /// the sphere.
    pub fn containing(position: Meters3, edge: Meters) -> Self {
        let cell = |meters: Meters| (meters.get() / edge.get() + 0.5).floor() as i32;
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
pub(crate) fn index_slug(index: i32) -> String {
    if index < 0 {
        format!("n{}", index.unsigned_abs())
    } else {
        index.to_string()
    }
}

/// The grid a world is cut into: what a generator may assume about every cell
/// before it describes one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldGeometry {
    /// Sector edge length.
    pub sector_edge: Meters,
}

impl WorldGeometry {
    /// Refuse a cell too narrow to OWN a body of this clearance.
    ///
    /// A generator keeps a CENTRE within [`PLACEMENT_INSET`] of the half
    /// edge, so the body itself stays inside its cell only while its clearance
    /// sphere fits in the margin the inset leaves: `clearance <= half_edge *
    /// (1 - PLACEMENT_INSET)`. A generator checks its widest body here, from
    /// [`SectorGenerator::validate`], rather than per candidate on a worker: a
    /// cell too narrow for its own bodies is one authored mistake, and the
    /// placement would report it one refused sector at a time for the life of
    /// the session.
    ///
    /// # Errors
    ///
    /// [`SectorFault::Config`] on `sector_edge`, naming `body` so a caller
    /// knows what they would have to shrink.
    pub fn require_owning_edge(self, clearance: Meters, body: &str) -> Result<(), SectorFault> {
        let floor = Meters(clearance.get() * 2.0 / (1.0 - PLACEMENT_INSET));
        if self.sector_edge < floor {
            return Err(SectorFault::Config {
                field: "sector_edge",
                value: format!(
                    "{} m, under the {} m a cell needs to hold {body} inside its own faces",
                    self.sector_edge.get(),
                    floor.get()
                ),
            });
        }
        Ok(())
    }
}

/// What a generator is asked: one cell of one world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectorGenerationInput {
    /// The world seed every coordinate-derived draw and every noise field is
    /// keyed from.
    pub seed: u32,
    /// The grid the cell belongs to.
    pub geometry: WorldGeometry,
    /// The cell to describe.
    pub coord: SectorCoord,
}

impl SectorGenerationInput {
    /// The deterministic draw for one named purpose of this cell.
    ///
    /// Coordinate-derived and purpose-separated, so visit order cannot reach
    /// it and adding a second purpose later cannot move what this one placed.
    /// The stream is then walked in a fixed order within the sector, which is
    /// the only ordering the result depends on.
    pub fn stream(self, purpose: &str) -> SeedStream {
        SeedStream::new(
            Fnv32::new()
                .write(&self.seed.to_le_bytes())
                .write(&self.coord.x.to_le_bytes())
                .write(&self.coord.y.to_le_bytes())
                .write(&self.coord.z.to_le_bytes())
                .write(purpose.as_bytes())
                .finish(),
        )
    }
}

/// What fills a cell.
///
/// Static dispatch: the generator is the `G` of [`WorldConfig<G>`] and
/// [`NovaWorldPlugin<G>`], so the streaming systems call it without a vtable
/// and an app holds exactly one. The value is part of the config, so
/// replacing a config with another of the same `G` is the same clear-session
/// swap as a new seed.
///
/// A generator only DESCRIBES, and its answer is an untrusted
/// [`SectorManifest`]. Preparing and spawning is one implementation in this
/// crate for every generator, and [`validate_manifest`] refuses a manifest
/// that breaks the rules those steps rely on, so a generator bug fails before
/// a worker meshes anything.
pub trait SectorGenerator: Clone + Debug + Send + Sync + 'static {
    /// Refuse a world this generator cannot fill.
    ///
    /// Called from [`WorldConfig::validate`]: on the main thread on the frame
    /// the config is armed or replaced, and on a worker before every cell. So
    /// a refusal here stops the run before anything is spawned, and before
    /// the main thread enumerates a window it cannot afford.
    ///
    /// # Errors
    ///
    /// A [`SectorFault`] naming what cannot describe a sector. Never a silent
    /// fallback: a generator that quietly drew a house default would hide the
    /// mistake it was handed.
    fn validate(&self, geometry: WorldGeometry) -> Result<(), SectorFault>;

    /// Describe one cell.
    ///
    /// Must be PURE: the same input gives the same description, on any call,
    /// in any order, on any thread, with nothing live. Every draw comes off
    /// [`SectorGenerationInput::stream`] or the feature field, never the
    /// ambient RNG.
    ///
    /// # Errors
    ///
    /// A [`SectorFault`] for a cell this generator cannot describe.
    fn generate(&self, input: SectorGenerationInput) -> Result<SectorManifest, SectorFault>;
}

/// The world's dials: its seed, how big a sector is, how far the desired set
/// reaches, and the generator that fills a sector.
///
/// A resource, and the ARMING switch: the streaming systems do nothing until
/// it is inserted.
///
/// No `Default`. A world seed belongs to a save and a cell edge belongs to a
/// design; a config with no author is how a number nobody chose reaches a
/// frame.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct WorldConfig<G: SectorGenerator> {
    /// The world seed every coordinate-derived draw and every noise field is
    /// keyed from.
    pub seed: u32,
    /// Sector edge length.
    pub sector_edge: Meters,
    /// How many cells out from the current one the desired set reaches. The
    /// desired set is the cube of side `2 * active_radius + 1`, and
    /// [`WorldConfig::validate`] refuses a radius whose cube is above
    /// [`ACTIVE_WINDOW_SECTORS_MAX`].
    pub active_radius: i32,
    /// What fills a cell.
    pub generator: G,
}

impl<G: SectorGenerator> WorldConfig<G> {
    /// The grid this config cuts the world into.
    pub fn geometry(&self) -> WorldGeometry {
        WorldGeometry {
            sector_edge: self.sector_edge,
        }
    }

    /// What the generator is asked for `coord`.
    pub fn input(&self, coord: SectorCoord) -> SectorGenerationInput {
        SectorGenerationInput {
            seed: self.seed,
            geometry: self.geometry(),
            coord,
        }
    }

    /// Refuse dials that cannot describe a sector, then whatever the
    /// generator refuses.
    ///
    /// Called by [`generate_sector`] on a worker, and again by
    /// [`track_current_sector`] on the frame a config is armed or replaced, so
    /// an invalid value stops the run before anything is spawned rather than
    /// producing a sector nobody can stand in - and before the main thread
    /// enumerates a window it cannot afford.
    ///
    /// # Errors
    ///
    /// [`SectorFault::Config`] for a cell edge that is not a finite positive
    /// length, for a window above [`ACTIVE_WINDOW_SECTORS_MAX`], and for an
    /// edge so wide that the window around the origin has no representable
    /// face - that cell would fault on a worker, long after the world armed.
    /// None of them is clamped: a clamp would stream a window nobody asked
    /// for. Then whatever [`SectorGenerator::validate`] refuses.
    pub fn validate(&self) -> Result<(), SectorFault> {
        let refuse = |field: &'static str, value: String| Err(SectorFault::Config { field, value });
        if !self.sector_edge.get().is_finite() || self.sector_edge.get() <= 0.0 {
            return refuse("sector_edge", format!("{} m", self.sector_edge.get()));
        }
        window_cells(self.active_radius)?;
        // The window is centred on the ORIGIN here, because that is the only
        // part of it the config fixes - where the observer stands is runtime,
        // and `generate_sector` refuses a far cell on its own. What must not
        // happen is arming a config whose very first window has no
        // representable centre: `collect_sector_jobs` panics on a fault, so
        // by the time the overflow is seen the session is already streaming.
        let reach =
            self.active_radius as f32 * self.sector_edge.get() + self.sector_edge.get() * 0.5;
        if !reach.is_finite() {
            return refuse(
                "sector_edge",
                format!(
                    "{} m, too wide for the {} cells either side of the observer to reach a \
                     representable face",
                    self.sector_edge.get(),
                    self.active_radius
                ),
            );
        }
        self.generator.validate(self.geometry())
    }
}

/// Why a sector refused. Every variant is a REFUSAL BEFORE MATERIALIZATION:
/// the generator will not describe a sector it cannot describe correctly, and
/// the streaming loop will not spawn into a world it cannot read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SectorFault {
    /// A dial cannot describe a sector.
    Config {
        /// The [`WorldConfig`] field at fault, dotted into the generator for
        /// one of its own dials.
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
    /// A generated rock names an asteroid kind the game does not ship. A
    /// mod's content table is exactly where an id nobody registered comes
    /// from.
    UnknownKind {
        /// The id nobody answers to.
        kind: String,
    },
    /// A generated ship names a design the loaded catalog does not hold.
    /// Checked on the main thread, where the catalog lives, and refused before
    /// the sector spawns anything.
    UnknownShip {
        /// The ship that named it.
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
    /// A generated object's pose or radius is not finite, or the cell itself
    /// has no finite centre in meters. This can occur even with a valid config
    /// when a coordinate-to-meter conversion overflows.
    InvalidGeometry {
        /// The object, or for a cell with no finite centre the cell's slug,
        /// whose geometry cannot be materialized.
        id: String,
    },
    /// Every deterministic placement candidate for an object overlapped
    /// something already placed, or fell outside the cell's inset. The cell is
    /// too full for what the generator asked of it, and a sector that quietly
    /// dropped the object would hide that.
    Clearance {
        /// The object with nowhere to stand.
        id: String,
        /// How many candidates were drawn and rejected.
        attempts: usize,
    },
    /// A generator returned a manifest the world cannot materialize: the
    /// wrong cell, an object outside its cell or crowding another, an id
    /// another cell owns, a feature reference this cell does not own, or more
    /// spheres, bodies or rocks than the measured caps. A generator is outside
    /// this crate, so its answer is checked rather than trusted.
    Manifest {
        /// The object at fault, or the cell's slug for a cell-wide rule.
        id: String,
        /// Which of its fields.
        field: &'static str,
        /// What that field held.
        value: String,
    },
    /// Two live roots claim the same cell. One of them is a leak, and which
    /// one is not decidable from here.
    DuplicateRoot {
        /// The cell claimed twice.
        coord: SectorCoord,
    },
    /// There is not exactly one [`WorldObserver`]. Streaming has no centre to
    /// stream around, and guessing one silently moves the world.
    AbsentObserver,
}

impl std::fmt::Display for SectorFault {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config { field, value } => write!(
                formatter,
                "WorldConfig::{field} is {value}, which cannot describe a sector"
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
            Self::UnknownKind { kind } => write!(
                formatter,
                "a generated rock names asteroid kind '{kind}', which the game does not ship"
            ),
            Self::UnknownShip { id, design } => write!(
                formatter,
                "ship '{id}' names design '{design}', which the catalog does not hold"
            ),
            Self::DuplicateId { id } => {
                write!(formatter, "two objects in one sector claim the id '{id}'")
            }
            Self::InvalidGeometry { id } => {
                write!(formatter, "'{id}' has a non-finite position or radius")
            }
            Self::Clearance { id, attempts } => write!(
                formatter,
                "object '{id}' found no clear place in its sector in {attempts} candidates"
            ),
            Self::Manifest { id, field, value } => write!(
                formatter,
                "sector object '{id}' has {field} {value}, which the world cannot materialize"
            ),
            Self::DuplicateRoot { coord } => {
                write!(formatter, "two live sector roots claim the cell {coord}")
            }
            Self::AbsentObserver => write!(
                formatter,
                "there is not exactly one WorldObserver to stream around"
            ),
        }
    }
}

/// The streaming stages, in the order one frame runs them.
///
/// Named so a caller that adds its own world systems can say where they sit
/// rather than guessing. Every stage but [`Self::Cleanup`] runs only behind a
/// live scenario and an inserted [`WorldConfig`]; [`Self::Cleanup`] is the one
/// stage not gated on the session being live, so it runs while no scenario is
/// live, on the frame a live one is replaced, and on the frame the
/// [`WorldConfig`] is inserted, replaced or removed.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NovaWorldSystems {
    /// Drop the work the session no longer owns, and the world a replaced
    /// [`WorldConfig`] no longer describes.
    Cleanup,
    /// Write which cell the [`WorldObserver`] stands in, and refuse a
    /// [`WorldConfig`] the stages after it cannot afford.
    Observe,
    /// Start preparation jobs for the nearest desired cells with a free slot.
    Request,
    /// Take in what the workers finished, without waiting on anything.
    Collect,
    /// Spawn at most one prepared sector.
    Materialize,
    /// Take back everything outside the desired set.
    Retire,
}

/// The streamed world, in `Update`, filled by the generator `G`.
///
/// OPT-IN: `AppBuilder` does not add it.
///
/// ONE per app. [`WorldConfig<G>`] is a resource per `G`, so two plugins of
/// different generators would stream two worlds over the same roots, jobs and
/// prepared payloads, each retiring the other's cells as a stranger's. So a
/// second plugin of another generator panics while the app is being built,
/// and bevy already refuses a second plugin of the same one.
///
/// The dependency on the scenario plugins is a RUNTIME one, not a
/// registration order: every stage but [`NovaWorldSystems::Cleanup`] is gated
/// on a live scenario session, and materialization calls the scenario object
/// factories. Nothing in `build` reads scenario state, so it does not matter
/// that `AppBuilder::with_game_plugins` registers this plugin before
/// `NovaScenarioPlugin` - which is what the examples do.
///
/// The stages are chained, so bevy applies each one's commands before the next
/// queries: the session check runs first, so no frame can hand a new session
/// work the previous one asked for; the observer is read next, so a crossing
/// is acted on in the frame it is noticed; requesting before polling is what
/// lets a job be started and collected in the same frame if a worker is that
/// fast; retiring last is what keeps a sector materialized this frame from
/// being taken back by the same frame that made it.
pub struct NovaWorldPlugin<G: SectorGenerator>(PhantomData<fn() -> G>);

impl<G: SectorGenerator> Default for NovaWorldPlugin<G> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// Which generator's [`NovaWorldPlugin`] this app installed.
///
/// Not generic on purpose: a second plugin of ANOTHER generator has to find
/// the first one's marker, and a `Marker<G>` would be a different resource to
/// it.
#[derive(Resource)]
struct InstalledGenerator(&'static str);

impl<G: SectorGenerator> Plugin for NovaWorldPlugin<G> {
    fn build(&self, app: &mut App) {
        trace!("NovaWorldPlugin<{}>: build", type_name::<G>());

        if let Some(installed) = app.world().get_resource::<InstalledGenerator>() {
            panic!(
                "nova_world: NovaWorldPlugin<{}> is already installed, so NovaWorldPlugin<{}> \
                 would stream a second world over the same sectors; an app holds exactly one \
                 generator",
                installed.0,
                type_name::<G>()
            );
        }
        app.insert_resource(InstalledGenerator(type_name::<G>()));
        app.init_resource::<ReadySectors>();
        app.init_resource::<SectorJobStats>();
        app.init_resource::<crate::streaming::ClearedConfig>();
        // `ClearedConfig` holds a `Tick` in a plain field, which bevy's
        // periodic tick sweep cannot reach on its own. Without this the
        // recorded tick and the config's own would drift apart on a long
        // session and the ordering guard would refuse a healthy frame.
        app.add_observer(
            |check: On<CheckChangeTicks>, mut cleared: ResMut<crate::streaming::ClearedConfig>| {
                cleared.check_tick(*check);
            },
        );

        app.configure_sets(
            Update,
            (
                // `resource_removed` reads a `Local` that only advances on the
                // frames the condition is EVALUATED, and `or_else` short-
                // circuits, so it goes FIRST. Behind any other term it would
                // miss a removal that lands on the frame after a scenario
                // change, and the roots the old config built would stand in an
                // unconfigured world until something else re-armed it.
                NovaWorldSystems::Cleanup.run_if(
                    resource_removed::<WorldConfig<G>>
                        .or_else(not(scenario_is_live))
                        .or_else(resource_changed::<CurrentScenario>)
                        .or_else(resource_exists_and_changed::<WorldConfig<G>>),
                ),
                (
                    NovaWorldSystems::Observe.run_if(resource_exists::<WorldConfig<G>>),
                    (
                        NovaWorldSystems::Request,
                        NovaWorldSystems::Collect,
                        NovaWorldSystems::Materialize,
                        NovaWorldSystems::Retire,
                    )
                        .chain()
                        .run_if(
                            resource_exists::<WorldConfig<G>>
                                .and_then(resource_exists::<CurrentSector>),
                        ),
                )
                    .chain()
                    .run_if(scenario_is_live),
            )
                .chain(),
        );

        app.add_systems(
            Update,
            (
                clear_sector_work::<G>.in_set(NovaWorldSystems::Cleanup),
                track_current_sector::<G>.in_set(NovaWorldSystems::Observe),
                request_sectors::<G>.in_set(NovaWorldSystems::Request),
                collect_sector_jobs::<G>.in_set(NovaWorldSystems::Collect),
                materialize_ready_sector::<G>.in_set(NovaWorldSystems::Materialize),
                retire_sectors::<G>.in_set(NovaWorldSystems::Retire),
            ),
        );
    }
}
