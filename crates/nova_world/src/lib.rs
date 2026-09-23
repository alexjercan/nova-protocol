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
//! [`SectorGeneration`] is the one thing two worlds can disagree about; the
//! window, the edge and the whole job lifetime are shared.
//!
//! - [`SectorGeneration::UniformAsteroids`] fills every cell the same way, out
//!   of the cell's own seed. It is the streaming baseline: nothing about the
//!   world can explain away a sector that failed to come up.
//! - [`SectorGeneration::LayeredFeatures`] fills a cell from a world that
//!   exists ABOVE it. Three independent global noise fields gate candidate
//!   [`FeatureSphere`]s on a coarse lattice; the spheres are pure data,
//!   addressed by lattice node rather than by sector, and a cell asks which of
//!   them reach it. That is what makes two neighbouring cells agree about a
//!   belt that crosses both of them without either one owning it.
//!
//! Neither generator has a hidden table. Every content id a cell can draw -
//! the asteroid kinds, the planet archetypes, the moored hull's design - is
//! named in the config, and a config naming an id the game does not ship is
//! refused by [`WorldConfig::validate`] before a single cell is described.
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
//!   bodies, planetoids and moored hulls and nothing else. A feature sphere is
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
//! coordinate alone, so an old seed, edge or content table would materialize
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

use bevy::{ecs::change_detection::CheckChangeTicks, prelude::*};
use nova_events::prelude::{Meters, Meters3};
use nova_scenario::prelude::{
    is_asteroid_kind, scenario_is_live, CurrentScenario, PlanetType, ShipDesignId,
};

mod generation;
mod streaming;

#[cfg(test)]
mod tests;

pub use crate::{
    generation::{
        generate_sector, prepare_sector, sector_features, FeatureFields, FeatureLayer,
        FeatureSphere, PreparedSector, SectorAnchorage, SectorAsteroid, SectorDescription,
        SectorPlanet, CLEARANCE_MARGIN, FEATURE_HALO, FEATURE_LATTICE, FEATURE_WAVELENGTH,
        MOORED_HULL_CLEARANCE, SECTOR_ASTEROIDS_MAX,
    },
    streaming::{
        clear_sector_work, collect_sector_jobs, desired_sectors, live_sectors,
        materialize_ready_sector, materialize_sector, request_sectors, retire_sectors,
        track_current_sector, CurrentSector, ReadySectors, SectorFeatureSpheres, SectorJob,
        SectorJobStats, SectorRoot, SectorStrengths, WorldObserver,
    },
};

/// Glob-import surface: `use nova_world::prelude::*` brings the config, the
/// generator, the streaming components and the plugin into scope.
pub mod prelude {
    pub use super::{
        generate_sector, prepare_sector, sector_features, FeatureFields, FeatureLayer,
        FeatureSphere, LayeredFeatureConfig, NovaWorldPlugin, NovaWorldSystems, PreparedSector,
        SectorAnchorage, SectorAsteroid, SectorCoord, SectorDescription, SectorFault,
        SectorGeneration, SectorPlanet, UniformAsteroidConfig, WorldConfig,
        ACTIVE_WINDOW_SECTORS_MAX, CLEARANCE_MARGIN, FEATURE_HALO, FEATURE_LATTICE,
        FEATURE_WAVELENGTH, MOORED_HULL_CLEARANCE, PLACEMENT_INSET, SECTOR_ASTEROIDS_MAX,
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

/// Every rock a [`SectorGeneration::UniformAsteroids`] cell draws from.
///
/// No `Default`: a body count and a radius band nobody chose are how a number
/// nobody chose reaches a frame.
#[derive(Clone, Debug, PartialEq)]
pub struct UniformAsteroidConfig {
    /// Rocks generated per sector. Zero is refused - a uniform world with no
    /// bodies in it is a configuration mistake, not a world - and so is
    /// anything above [`SECTOR_ASTEROIDS_MAX`], the one density either
    /// generator has been measured at.
    pub body_count: usize,
    /// Smallest nominal body radius drawn.
    pub radius_min: Meters,
    /// Largest nominal body radius drawn.
    pub radius_max: Meters,
    /// The asteroid kind ids a body may be drawn from. Every id is checked
    /// against the shipped kind table by [`WorldConfig::validate`].
    pub asteroid_kinds: Vec<String>,
}

/// The content a [`SectorGeneration::LayeredFeatures`] cell draws from.
///
/// Content only. The lattice, the thresholds and the radius bands are
/// properties of a [`FeatureLayer`] rather than dials here, because a
/// per-caller override of a global field would be two worlds with one seed.
#[derive(Clone, Debug, PartialEq)]
pub struct LayeredFeatureConfig {
    /// The asteroid kind ids a rock field may be drawn from.
    pub asteroid_kinds: Vec<String>,
    /// The archetypes a gated planetoid may be drawn from.
    pub planet_types: Vec<PlanetType>,
    /// The catalog design every moored hull is built from. Checked against the
    /// loaded catalog on the main thread, which is the one place the catalog
    /// exists.
    pub anchorage_design: ShipDesignId,
}

/// What a cell is FILLED with.
#[derive(Clone, Debug, PartialEq)]
pub enum SectorGeneration {
    /// Every cell gets the same treatment out of its own seed: `body_count`
    /// rocks scattered across its inset, nominal radius drawn from the band.
    ///
    /// The streaming baseline. Nothing about the WORLD can explain away a
    /// sector that failed to come up, which is what makes it the right
    /// generator for judging a retirement or a crossing.
    UniformAsteroids(UniformAsteroidConfig),
    /// The cell asks the feature field what reaches it, and fills itself from
    /// the answer: rocks from the combined asteroid influence, one planetoid
    /// per owned planet sphere, a few moored hulls per owned anchorage sphere.
    LayeredFeatures(LayeredFeatureConfig),
}

/// The world's dials: its seed, how big a sector is, how far the desired set
/// reaches, and what a sector is filled with.
///
/// A resource, and the ARMING switch: the streaming systems do nothing until
/// it is inserted.
///
/// No `Default`. A world seed belongs to a save and a cell edge belongs to a
/// design; a config with no author is how a number nobody chose reaches a
/// frame.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct WorldConfig {
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
    pub generation: SectorGeneration,
}

impl WorldConfig {
    /// Refuse dials that cannot describe a sector, and content ids the game
    /// does not ship.
    ///
    /// Called by [`generate_sector`] on a worker, and again by
    /// [`track_current_sector`] on the frame a config is armed or replaced, so
    /// an invalid value stops the run before anything is spawned rather than
    /// producing a sector nobody can stand in - and before the main thread
    /// enumerates a window it cannot afford.
    ///
    /// # Errors
    ///
    /// [`SectorFault::Config`] for a dial or a list that cannot describe a
    /// sector, and [`SectorFault::UnknownKind`] for an asteroid kind id the
    /// game does not ship. An empty list is a refusal and never a silent
    /// fallback: a generator that quietly drew `rock` because nobody named a
    /// kind would hide the authoring mistake it was handed. A window above
    /// [`ACTIVE_WINDOW_SECTORS_MAX`] and a layered cell edge wider than the
    /// thinning halo covers are refused the same way, and neither is clamped:
    /// a clamp would stream a window nobody asked for and thin a field nobody
    /// could reason about.
    pub fn validate(&self) -> Result<(), SectorFault> {
        let refuse = |field: &'static str, value: String| Err(SectorFault::Config { field, value });
        if !self.sector_edge.get().is_finite() || self.sector_edge.get() <= 0.0 {
            return refuse("sector_edge", format!("{} m", self.sector_edge.get()));
        }
        window_cells(self.active_radius)?;
        let kinds = match &self.generation {
            SectorGeneration::UniformAsteroids(uniform) => {
                // Both ends, and both BEFORE `generate_sector` reserves a
                // vector of this size on a worker. An authored count is the
                // one number here a caller types straight into an allocation.
                if uniform.body_count == 0 || uniform.body_count > SECTOR_ASTEROIDS_MAX {
                    return refuse(
                        "generation.body_count",
                        format!(
                            "{}, outside the 1 to {SECTOR_ASTEROIDS_MAX} rocks a cell holds",
                            uniform.body_count
                        ),
                    );
                }
                if !uniform.radius_min.get().is_finite() || uniform.radius_min.get() <= 0.0 {
                    return refuse(
                        "generation.radius_min",
                        format!("{} m", uniform.radius_min.get()),
                    );
                }
                if !uniform.radius_max.get().is_finite() || uniform.radius_max < uniform.radius_min
                {
                    return refuse(
                        "generation.radius_max",
                        format!(
                            "{} m, expected a finite value at least radius_min {} m",
                            uniform.radius_max.get(),
                            uniform.radius_min.get()
                        ),
                    );
                }
                &uniform.asteroid_kinds
            }
            SectorGeneration::LayeredFeatures(layered) => {
                // Release-visible, not a debug assertion: the thinning halo is
                // a FINITE node search sized from the widest radius, the
                // jitter draw and the inset pull, and the inset pull grows
                // with the cell edge. Past the edge the halo covers, two
                // same-layer spheres can both survive and the world ships with
                // belts sitting inside each other. The uniform generator has
                // no halo and no thinning, so its edge is not this refusal's
                // business.
                if !generation::feature_halo_covers_overlap(self.sector_edge) {
                    return refuse(
                        "sector_edge",
                        format!(
                            "{} m, wider than a {FEATURE_HALO}-node thinning halo can reach \
                             across at a {} m feature lattice",
                            self.sector_edge.get(),
                            FEATURE_LATTICE.get()
                        ),
                    );
                }
                if layered.planet_types.is_empty() {
                    return refuse("generation.planet_types", "an empty list".to_string());
                }
                if layered.anchorage_design.trim().is_empty() {
                    return refuse("generation.anchorage_design", "an empty id".to_string());
                }
                &layered.asteroid_kinds
            }
        };
        if kinds.is_empty() {
            return refuse("generation.asteroid_kinds", "an empty list".to_string());
        }
        for kind in kinds {
            if !is_asteroid_kind(kind) {
                return Err(SectorFault::UnknownKind { kind: kind.clone() });
            }
        }
        Ok(())
    }
}

/// Why a sector refused. Every variant is a REFUSAL BEFORE MATERIALIZATION:
/// the generator will not describe a sector it cannot describe correctly, and
/// the streaming loop will not spawn into a world it cannot read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SectorFault {
    /// A dial or a content list cannot describe a sector.
    Config {
        /// The [`WorldConfig`] field at fault.
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
    /// The config names an asteroid kind the game does not ship. A mod's
    /// content table is exactly where an id nobody registered comes from.
    UnknownKind {
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
    /// with a valid config when a coordinate-to-meter conversion overflows.
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
                "the config names asteroid kind '{kind}', which the game does not ship"
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

/// The streamed world, in `Update`.
///
/// OPT-IN: `AppBuilder` does not add it. Where a caller does add it, the
/// plugin sits after the scenario plugins, because every stage but
/// [`NovaWorldSystems::Cleanup`] is gated on a live scenario session and
/// materialization calls the scenario object factories.
///
/// The stages are chained, so bevy applies each one's commands before the next
/// queries: the session check runs first, so no frame can hand a new session
/// work the previous one asked for; the observer is read next, so a crossing
/// is acted on in the frame it is noticed; requesting before polling is what
/// lets a job be started and collected in the same frame if a worker is that
/// fast; retiring last is what keeps a sector materialized this frame from
/// being taken back by the same frame that made it.
pub struct NovaWorldPlugin;

impl Plugin for NovaWorldPlugin {
    fn build(&self, app: &mut App) {
        trace!("NovaWorldPlugin: build");

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
                    resource_removed::<WorldConfig>
                        .or_else(not(scenario_is_live))
                        .or_else(resource_changed::<CurrentScenario>)
                        .or_else(resource_exists_and_changed::<WorldConfig>),
                ),
                (
                    NovaWorldSystems::Observe.run_if(resource_exists::<WorldConfig>),
                    (
                        NovaWorldSystems::Request,
                        NovaWorldSystems::Collect,
                        NovaWorldSystems::Materialize,
                        NovaWorldSystems::Retire,
                    )
                        .chain()
                        .run_if(
                            resource_exists::<WorldConfig>
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
                clear_sector_work.in_set(NovaWorldSystems::Cleanup),
                track_current_sector.in_set(NovaWorldSystems::Observe),
                request_sectors.in_set(NovaWorldSystems::Request),
                collect_sector_jobs.in_set(NovaWorldSystems::Collect),
                materialize_ready_sector.in_set(NovaWorldSystems::Materialize),
                retire_sectors.in_set(NovaWorldSystems::Retire),
            ),
        );
    }
}
