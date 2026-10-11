//! The generation mechanisms: the untrusted [`SectorManifest`] a generator
//! returns, and the check that turns it into a trusted [`SectorDescription`]
//! before a worker prepares it.
//!
//! PURE. Nothing here touches a `World`, reads a resource or draws from the
//! ambient RNG, which is what lets [`prepare_sector`] run on a worker and what
//! makes a cell the same cell in any visit order. A [`SectorGenerator`] is
//! held to the same rule: it gets a seed, an edge and a coordinate, and
//! nothing else.
//!
//! RECORD RULE. Each generated record is its [`SectorObjectId`], its
//! placement and the whole scenario spawn config the object is built from:
//! [`SectorAsteroid`] carries an [`AsteroidConfig`], [`SectorPlanet`] a
//! [`PlanetConfig`], [`SectorShip`] a [`SpaceshipConfig`]. Facts only the
//! generator knows (a ship's rotation, clearance, condition, civilization and
//! role) sit beside the config, never inside a hidden default.
//! Materialization adds only the pose, the id, the derived name and the
//! derelict marker; rocks and planetoids spawn at identity rotation, named by
//! their id.
//!
//! OWNERSHIP. A worker reads the configs and never resolves an asset or a
//! catalog. Every asset ref in a record must be a path, so the record saves
//! and compares the same in any run; [`validate_manifest`] refuses a runtime
//! handle. A rock's prepared geometry holds only its seed and radius, and the
//! spawn pairs it with the record's config. A planetoid's prepared surface
//! carries its own config. Section prototypes resolve against the catalog on
//! the main thread.

use std::{collections::BTreeSet, fmt, sync::Arc};

use bevy::{log::info_span, math::Quat};
use nova_events::prelude::{EntityId, Meters, Meters3};
use nova_gameplay::prelude::{AssetRef, Fnv32, SeedStream};
use nova_scenario::prelude::{
    is_asteroid_kind, patrol_stops_fault, prepare_asteroid_geometry, prepare_planet,
    AIControllerConfig, AsteroidConfig, PlanetConfig, PreparedAsteroid, PreparedPlanet,
    SectionSource, ShipDesign, ShipDesignSource, ShipPresentationConfig, SpaceshipConfig,
    SpaceshipController, ASTEROID_GEOMETRIC_FACTOR_MAX,
};

use crate::{
    frozen::{FrozenSector, PreparedFrozenBody},
    SectorCoord, SectorFault, SectorGenerationInput, SectorGenerator, WorldConfig,
};

/// A sector object's scenario id: `<cell slug>_<name>_<index>`, built only by
/// [`sector_id`], so every id names the cell that owns it and a scenario
/// cannot author one by accident. One cell cannot hold the same id twice;
/// [`validate_manifest`] refuses the repeat.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SectorObjectId(String);

impl SectorObjectId {
    /// The id as the scenario addresses it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SectorObjectId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<SectorObjectId> for EntityId {
    fn from(id: SectorObjectId) -> Self {
        EntityId(id.0)
    }
}

/// One generated rock, in world meters.
#[derive(Clone, Debug)]
pub struct SectorAsteroid {
    /// The rock's scenario id.
    pub id: SectorObjectId,
    /// Where it stands, in meters from the world origin.
    pub position: Meters3,
    /// The rock as the spawn builds it. `seed` is `Some`: the generator pins
    /// the silhouette, the spawn never derives one. `texture` and
    /// `destroy_sound` are paths: a saved cell names them on disk. A
    /// `lock_signature` override is a finite positive length.
    /// [`validate_manifest`] refuses each otherwise.
    pub config: AsteroidConfig,
}

impl SectorAsteroid {
    /// The silhouette seed the generator pinned.
    ///
    /// # Panics
    ///
    /// On an entry with none. [`validate_manifest`] refuses one, so a
    /// description never panics here.
    pub fn seed(&self) -> u32 {
        self.config.seed.unwrap_or_else(|| {
            panic!(
                "nova_world: rock '{}' pins no silhouette seed; validate_manifest refuses one",
                self.id
            )
        })
    }
}

/// One generated planetoid: a real [`PlanetConfig`], not a big rock.
#[derive(Clone, Debug)]
pub struct SectorPlanet {
    /// The planetoid's scenario id.
    pub id: SectorObjectId,
    /// Where it stands, in meters from the world origin.
    pub position: Meters3,
    /// The world it is, ready for `prepare_planet`.
    pub config: PlanetConfig,
}

/// Whether a generated ship still works or is a derelict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SectorShipConditionType {
    /// Powered and unpiloted: every system works, and nothing flies it.
    Intact,
    /// A derelict: its hull and docking ports stay live, and every other
    /// section spawns inactive.
    Derelict,
}

impl SectorShipConditionType {
    /// What the canonical description calls the condition.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Intact => "intact",
            Self::Derelict => "derelict",
        }
    }
}

/// The chance a name has three syllables rather than two.
const NAME_THREE_SYLLABLE_CHANCE: f32 = 0.5;

/// The syllables a name is composed from. Content: a change renames every
/// civilization in every world.
const NAME_SYLLABLES: [&str; 24] = [
    "an", "bel", "cor", "da", "el", "fen", "gar", "hal", "is", "jor", "ka", "lun", "mar", "nor",
    "os", "pra", "quel", "ren", "sol", "tev", "ur", "vas", "wen", "zo",
];

/// A civilization's stable machine identity: the world seed and its signed
/// lattice node.
///
/// Never a hash and never a name. Two nodes whose draw streams collide still
/// have distinct identities, and two civilizations may share a display name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CivilizationId {
    /// The world seed the civilization was drawn from.
    pub world_seed: u32,
    /// Its node on the civilization lattice.
    pub node: [i32; 3],
}

impl CivilizationId {
    /// The display name: two or three syllables, first letter capitalized.
    ///
    /// A pure function of the identity. Names repeat across a world, so a
    /// name must not be used to find a civilization.
    pub fn name(self) -> String {
        let mut stream = self.stream(b"name");
        let syllables = if stream.unit() < 1.0 - NAME_THREE_SYLLABLE_CHANCE {
            2
        } else {
            3
        };
        // Three syllable draws every time, so the syllable count does not
        // shift any later draw.
        let drawn: [&str; 3] = std::array::from_fn(|_| {
            let index = (stream.unit() * NAME_SYLLABLES.len() as f32) as usize;
            NAME_SYLLABLES[index.min(NAME_SYLLABLES.len() - 1)]
        });
        let joined = drawn[..syllables].concat();
        let mut letters = joined.chars();
        letters.next().map_or_else(String::new, |first| {
            first.to_uppercase().chain(letters).collect()
        })
    }

    /// The draw stream for one aspect of this civilization: the FNV-1a 32
    /// of the world seed, `civilization`, `aspect` and the node, each index
    /// little-endian.
    ///
    /// Public so the generator that draws a civilization's centroid, status,
    /// advancement and roles keys them the same way [`name`](Self::name)
    /// is keyed. Each aspect has its own stream, so retuning one aspect
    /// cannot move another: a new name inventory keeps every centroid and
    /// status. A change to this keying moves every civilization in every
    /// world.
    pub fn stream(self, aspect: &[u8]) -> SeedStream {
        SeedStream::new(
            Fnv32::new()
                .write(&self.world_seed.to_le_bytes())
                .write(b"civilization")
                .write(aspect)
                .write(&self.node[0].to_le_bytes())
                .write(&self.node[1].to_le_bytes())
                .write(&self.node[2].to_le_bytes())
                .finish(),
        )
    }
}

impl fmt::Display for CivilizationId {
    /// `civ_<x>_<y>_<z>@<seed>`, a negative index written `n<abs>`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "civ")?;
        for index in self.node {
            if index < 0 {
                write!(formatter, "_n{}", index.unsigned_abs())?;
            } else {
                write!(formatter, "_{index}")?;
            }
        }
        write!(formatter, "@{}", self.world_seed)
    }
}

/// The four closed ship roles a civilization fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ShipRoleType {
    /// Unarmed traffic.
    Civilian,
    /// Unarmed miners with cargo intake.
    Industrial,
    /// Rough, low-tier fighting ships.
    Scavenger,
    /// Equipped fighting ships.
    Armored,
}

impl ShipRoleType {
    /// Every role, in the order a role array holds them.
    pub const ALL: [Self; 4] = [
        Self::Civilian,
        Self::Industrial,
        Self::Scavenger,
        Self::Armored,
    ];

    /// What a readout or a legend calls the role.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Civilian => "civilian",
            Self::Industrial => "industrial",
            Self::Scavenger => "scavenger",
            Self::Armored => "armored",
        }
    }
}

/// One generated ship: where a hull stands, what it is, and the spawn that
/// builds it.
#[derive(Clone, Debug)]
pub struct SectorShip {
    /// The ship's scenario id.
    pub id: SectorObjectId,
    /// Where its root floats, in meters from the world origin.
    pub position: Meters3,
    /// Its full orientation: a finite unit quaternion.
    pub rotation: Quat,
    /// The radius around the root that holds the whole hull in any
    /// orientation. At most [`SECTOR_SHIP_CLEARANCE_MAX`].
    pub clearance: Meters,
    /// Whether it works or is a derelict. A derelict spawns with every
    /// section but hull and docking port inactive.
    pub condition: SectorShipConditionType,
    /// The civilization it belongs to, or belonged to as a derelict.
    pub civilization: CivilizationId,
    /// What it was built for. A derelict keeps its former role.
    pub role: ShipRoleType,
    /// The ship as the spawn builds it, whole: the generator states the
    /// design (inline, every section a prototype of the loaded catalog, every
    /// presentation sound a path), who
    /// flies it, its side (`Some`), its motion, its stock, whether it may be
    /// looted and its balance. Materialization adds nothing. A derelict is
    /// flown by nobody; [`validate_manifest`] refuses an AI on one, a player
    /// on any, a catalog design and a `None` side.
    pub spawn: SpaceshipConfig,
}

impl SectorShip {
    /// The inline design.
    ///
    /// # Panics
    ///
    /// On a design that names a catalog prototype. [`validate_manifest`]
    /// refuses one, so a description never panics here.
    pub fn design(&self) -> &ShipDesign {
        match &self.spawn.design {
            ShipDesignSource::Inline(design) => design,
            ShipDesignSource::Prototype { id, .. } => panic!(
                "nova_world: ship '{}' names the catalog design '{id}'; validate_manifest refuses \
                 one",
                self.id
            ),
        }
    }
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
    /// The rocks, in generation order.
    pub asteroids: Vec<SectorAsteroid>,
    /// The planetoids, in generation order.
    pub planets: Vec<SectorPlanet>,
    /// The ships, in generation order.
    pub ships: Vec<SectorShip>,
}

/// One sector's contents after [`validate_manifest`] accepted them: what
/// `materialize_sector` is allowed to spawn.
///
/// TRUSTED. The fields are private and there is no other constructor, so a
/// value of this type is proof that every rule materialization relies on
/// held, whichever generator wrote the manifest.
#[derive(Clone, Debug)]
pub struct SectorDescription {
    pub(crate) coord: SectorCoord,
    pub(crate) asteroids: Vec<SectorAsteroid>,
    pub(crate) planets: Vec<SectorPlanet>,
    pub(crate) ships: Vec<SectorShip>,
}

impl SectorDescription {
    /// The cell described.
    pub fn coord(&self) -> SectorCoord {
        self.coord
    }

    /// The rocks, in generation order.
    pub fn asteroids(&self) -> &[SectorAsteroid] {
        &self.asteroids
    }

    /// The planetoids, in generation order.
    pub fn planets(&self) -> &[SectorPlanet] {
        &self.planets
    }

    /// The ships, in generation order.
    pub fn ships(&self) -> &[SectorShip] {
        &self.ships
    }

    /// Every object id this cell spawns, in spawn order: rocks, then
    /// planetoids, then ships.
    pub fn object_ids(&self) -> Vec<SectorObjectId> {
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
    /// same when this matches. Positions and radii are printed at centimeter
    /// resolution and yaw at a ten-thousandth of a radian, deliberately
    /// rounded, so the comparison is a fact about the generator and not about
    /// the last bit of an f32. A rock's velocity and a planetoid's
    /// fields - relief, sea level, mass and lock signature - are authoring
    /// values, not placement: they are printed exact, as the shortest text
    /// that reads back to the same f32, and `None` prints apart from every
    /// `Some`. Rounding them would call two different worlds the same one.
    /// A rock's texture, destroy sound and lock signature, a ship's stock,
    /// side, capabilities, loot flag, civilization, role, design integrity,
    /// design presentation, each section's prototype patch and its controller
    /// apart from its route print through `Debug`: field by field in
    /// declaration order, a float as the shortest text that reads back to it,
    /// a non-finite float as `NaN` or `inf`, apart from `None`. A ship's
    /// credits print their balance; an AI's route prints each waypoint as
    /// placement with its stop. An asset handle prints its runtime id, so a
    /// config that carries one compares equal only within one run;
    /// [`validate_manifest`] refuses one, so a description carries none.
    pub fn canonical(&self) -> String {
        let point = |position: Meters3| {
            let p = position.get();
            format!("{:.2} {:.2} {:.2}", p.x, p.y, p.z)
        };
        let mut out = format!("{}\n", self.coord);
        for body in &self.asteroids {
            let config = &body.config;
            out.push_str(&format!(
                "asteroid {} {} r{:.2} {} seed {:?} velocity {:?} texture {:?} destroy_sound {:?} \
                 lock_signature {:?}\n",
                body.id,
                point(body.position),
                config.radius.get(),
                config.kind,
                config.seed,
                config.initial_velocity.get(),
                config.texture,
                config.destroy_sound,
                config.lock_signature.map(Meters::get),
            ));
        }
        for planet in &self.planets {
            out.push_str(&format!(
                "planet {} {} {:?} r{:.2} s{} relief {:?} sea_level {:?} mass {:?} \
                 lock_signature {:?}\n",
                planet.id,
                point(planet.position),
                planet.config.planet_type,
                planet.config.radius.get(),
                planet.config.seed,
                planet.config.relief.map(Meters::get),
                planet.config.sea_level,
                planet.config.mass,
                planet.config.lock_signature.map(Meters::get),
            ));
        }
        for ship in &self.ships {
            let spawn = &ship.spawn;
            let design = ship.design();
            out.push_str(&format!(
                "ship {} {} {} velocity {:?} c{:.2} {} {} {} stock {:?} credits {} side {:?} \
                 lootable {} capabilities {:?} integrity {:?} presentation {:?}\n",
                ship.id,
                point(ship.position),
                canonical_rotation(ship.rotation),
                spawn.initial_velocity.get(),
                ship.clearance.get(),
                ship.condition.label(),
                ship.civilization,
                ship.role.label(),
                spawn.inventory.stacks().collect::<Vec<_>>(),
                spawn.credits,
                spawn.allegiance,
                spawn.lootable,
                spawn.capabilities,
                design.integrity,
                design.presentation,
            ));
            match &spawn.controller {
                SpaceshipController::None => out.push_str("  controller none\n"),
                SpaceshipController::Player(config) => {
                    out.push_str(&format!("  controller player {config:?}\n"));
                }
                SpaceshipController::AI(config) => {
                    let routine = AIControllerConfig {
                        patrol: Vec::new(),
                        patrol_stops: Vec::new(),
                        ..config.clone()
                    };
                    out.push_str(&format!("  controller ai {routine:?}\n"));
                    for (index, waypoint) in config.patrol.iter().enumerate() {
                        let stop = config.patrol_stops.get(index);
                        out.push_str(&format!("  waypoint {} stop {stop:?}\n", point(*waypoint)));
                    }
                }
            }
            for section in &design.sections {
                let p = section.position;
                out.push_str(&format!(
                    "  section {} {} {:.3} {:.3} {:.3} {} patch {:?}\n",
                    section.id,
                    section.source.prototype_id(),
                    p.x,
                    p.y,
                    p.z,
                    canonical_rotation(section.rotation),
                    section.source.patch(),
                ));
            }
        }
        out
    }
}

/// A rotation as the canonical description prints it: `q` and `-q` are one
/// rotation, so the sign makes the first nonzero of `w`, `x`, `y`, `z`
/// positive, and each component prints to a millionth.
fn canonical_rotation(rotation: Quat) -> String {
    let components = [rotation.w, rotation.x, rotation.y, rotation.z];
    let sign = match components.iter().find(|value| **value != 0.0) {
        Some(first) if *first < 0.0 => -1.0,
        _ => 1.0,
    };
    // Adding zero turns a negative zero into a positive one.
    let [w, x, y, z] = components.map(|value| sign * value + 0.0);
    format!("q{w:.6} {x:.6} {y:.6} {z:.6}")
}

/// One sector described, validated and prepared: everything
/// `materialize_sector` needs that a worker can produce, resolved against the
/// cell's frozen record on the worker.
///
/// Built only by [`prepare_sector`] and a [`crate::SectorJob`], and the fields
/// are private. Each record travels WITH what was prepared for it: a planetoid
/// with its surface, a rock with its geometry, a frozen rock with the geometry
/// its record describes. Nothing is paired by position, so the main thread has
/// nothing to count. A ship needs no preparation: its inline design's sections
/// are resolved against the section catalog on the main thread, which is the
/// one place the catalog exists.
#[derive(Debug)]
pub struct PreparedSector {
    pub(crate) coord: SectorCoord,
    /// The description's planetoids. They never freeze, so a visited cell
    /// spawns them again beside its record.
    pub(crate) planets: Vec<PreparedPlanetoid>,
    pub(crate) bodies: PreparedBodies,
}

impl PreparedSector {
    /// The cell this was prepared for.
    pub fn coord(&self) -> SectorCoord {
        self.coord
    }
}

/// What a cell spawns beside its planetoids, decided once, on the worker,
/// from the cell's frozen record.
#[derive(Debug)]
pub(crate) enum PreparedBodies {
    /// A cell never visited: its generated rocks and ships, plus the bodies
    /// that arrived into it while nobody held it live. A generated body an
    /// arrival overlaps is not removed.
    Fresh {
        rocks: Vec<PreparedRock>,
        ships: Vec<SectorShip>,
        arrivals: Vec<PreparedFrozenBody>,
    },
    /// A visited cell: its record is the whole truth of it, and the
    /// generated rocks and ships are not spawned.
    Thawed(Vec<PreparedFrozenBody>),
}

/// One generated rock with the geometry prepared for it.
#[derive(Debug)]
pub(crate) struct PreparedRock {
    pub body: SectorAsteroid,
    pub geometry: PreparedAsteroid,
}

/// One generated planetoid with the surface prepared for it.
#[derive(Debug)]
pub(crate) struct PreparedPlanetoid {
    pub body: SectorPlanet,
    pub surface: PreparedPlanet,
}

/// The largest hull clearance a generated ship may declare.
///
/// A generator measures each hull and declares the radius around its root
/// that holds it in any orientation; [`validate_manifest`] refuses a larger
/// one. The bound keeps a generator's cluster halo finite: a policy that
/// plans where hulls stand before it lays them out can reserve this radius
/// for each and know the actual hull fits inside.
pub const SECTOR_SHIP_CLEARANCE_MAX: Meters = Meters(400.0);

/// How far a ship's rotation may be from unit length and still count as a
/// rotation. A generator composes it from a few `f32` products, so it cannot
/// be exact.
const ROTATION_LENGTH_TOLERANCE: f32 = 1e-4;

/// Whether two bodies keep `margin` between their clearance spheres.
///
/// The one spacing formula. [`validate_manifest`] asks it with a zero margin,
/// so it refuses only bodies that overlap. A generator's placement search asks
/// it with the margin that generator keeps, so no search can accept a pose the
/// check refuses.
pub fn bodies_clear(
    a_position: Meters3,
    a_clearance: Meters,
    b_position: Meters3,
    b_clearance: Meters,
    margin: Meters,
) -> bool {
    a_position.distance(b_position) >= a_clearance + b_clearance + margin
}

/// One object already standing in the cell, and how much room it claims.
#[derive(Clone, Copy, Debug)]
struct Occupied {
    centre: Meters3,
    clearance: Meters,
}

fn position_is_finite(position: Meters3) -> bool {
    position.get().is_finite()
}

/// The id `<cell slug>_<name>_<index>` of one object a generator places in
/// `coord`.
///
/// Formatting only. The slug prefix is the cross-generator contract:
/// [`validate_manifest`] refuses an id without it, so no two cells can claim
/// one object, and refuses an id claimed twice inside the cell.
pub fn sector_id(coord: SectorCoord, name: &str, index: usize) -> SectorObjectId {
    SectorObjectId(format!("{}_{name}_{index}", coord.slug()))
}

/// Turn a generator's manifest into a trusted [`SectorDescription`], or refuse
/// it. The only constructor a description has.
///
/// A generator is outside this crate, so its answer is checked against every
/// rule `materialize_sector` and the streaming loop rely on: a finite positive
/// cell edge and a finite cell centre, refused before the manifest is read,
/// because a direct caller need not have come through [`WorldConfig::validate`]
/// and a NaN edge makes every containment test below pass; the cell it was
/// asked for; finite geometry; ids unique and prefixed with the cell's slug,
/// so two cells never claim one object; every body standing inside its own
/// cell with its whole clearance sphere, so retiring a neighbour never takes
/// it; no two bodies overlapping; shipped asteroid kinds; finite initial
/// velocities; every asset ref a path, not a runtime handle; asteroid lock
/// signatures that [`AsteroidConfig::validate_lock_signature`] accepts; and
/// planet configs that [`PlanetConfig::validate`] accepts. How many bodies a
/// generator places and how far apart it spaces them is its own policy; this
/// check only refuses what cannot be materialized. A ship's rotation must be
/// a finite unit quaternion, its declared clearance positive and at most
/// [`SECTOR_SHIP_CLEARANCE_MAX`], and its clearance sphere is the one that
/// must stand inside the cell. Its inline design is checked for what needs no
/// catalog; the strict resolve against the loaded sections is main-thread
/// work in `materialize_sector`.
///
/// # Errors
///
/// [`SectorFault::Config`] on `sector_edge` for an edge that is not a finite
/// positive length, and [`SectorFault::InvalidGeometry`] for a cell whose
/// centre has no finite position in meters;
/// [`SectorFault::Manifest`] for the wrong cell, an object outside its cell or
/// overlapping another, an id another cell owns, a non-finite initial
/// velocity, a rock with no silhouette seed, a texture or destroy sound that
/// is a runtime handle or a lock signature that is not a finite positive
/// length, a planet config [`PlanetConfig::validate`] refuses, a ship
/// clearance above the maximum, a ship design that names a catalog prototype,
/// has no sections, a repeated section id, an inline section, a non-finite
/// section pose or a presentation sound that is a runtime handle, a ship with
/// no side, flown by a player, a derelict flown by an AI, or an AI route the
/// controller could not fly;
/// [`SectorFault::InvalidGeometry`] for non-finite geometry, a ship rotation
/// that is not a unit quaternion and a ship clearance that is not positive;
/// [`SectorFault::DuplicateId`] and [`SectorFault::UnknownKind`].
pub fn validate_manifest(
    input: SectorGenerationInput,
    manifest: SectorManifest,
) -> Result<SectorDescription, SectorFault> {
    let coord = input.coord;
    let edge = input.geometry.sector_edge;
    if !edge.get().is_finite() || edge.get() <= 0.0 {
        return Err(SectorFault::Config {
            field: "sector_edge",
            value: format!("{} m", edge.get()),
        });
    }
    let centre = coord.centre(edge);
    if !position_is_finite(centre) {
        return Err(SectorFault::InvalidGeometry { id: coord.slug() });
    }
    let refuse = |id: &SectorObjectId, field: &'static str, value: String| SectorFault::Manifest {
        id: id.to_string(),
        field,
        value,
    };

    if manifest.coord != coord {
        return Err(SectorFault::Manifest {
            id: coord.slug(),
            field: "coord",
            value: format!("{}, not the requested {coord}", manifest.coord),
        });
    }

    let mut ids = BTreeSet::new();
    let mut standing = Vec::new();
    let mut own = |id: &SectorObjectId| {
        if !ids.insert(id.clone()) {
            return Err(SectorFault::DuplicateId { id: id.to_string() });
        }
        Ok(())
    };

    for body in &manifest.asteroids {
        own(&body.id)?;
        let config = &body.config;
        if !config.radius.get().is_finite() || config.radius.get() <= 0.0 {
            return Err(SectorFault::InvalidGeometry {
                id: body.id.to_string(),
            });
        }
        if !is_asteroid_kind(&config.kind) {
            return Err(SectorFault::UnknownKind {
                kind: config.kind.clone(),
            });
        }
        if !config.initial_velocity.is_finite() {
            return Err(refuse(
                &body.id,
                "initial_velocity",
                format!("{:?}", config.initial_velocity.get()),
            ));
        }
        if config.seed.is_none() {
            return Err(refuse(
                &body.id,
                "seed",
                "none; a generated rock pins its silhouette".to_string(),
            ));
        }
        if let AssetRef::Handle(handle) = &config.texture {
            return Err(refuse(
                &body.id,
                "texture",
                format!("the runtime handle {handle:?}, not an asset path"),
            ));
        }
        if let Some(AssetRef::Handle(handle)) = &config.destroy_sound {
            return Err(refuse(
                &body.id,
                "destroy_sound",
                format!("the runtime handle {handle:?}, not an asset path"),
            ));
        }
        config
            .validate_lock_signature()
            .map_err(|value| refuse(&body.id, "lock_signature", value))?;
        let clearance = Meters(config.radius.get() * ASTEROID_GEOMETRIC_FACTOR_MAX);
        stand_inside(input, &mut standing, &body.id, body.position, clearance)?;
    }
    for planet in &manifest.planets {
        own(&planet.id)?;
        planet
            .config
            .validate()
            .map_err(|fault| refuse(&planet.id, fault.field, fault.value))?;
        let clearance = planet.config.body_radius();
        if !clearance.get().is_finite() || clearance.get() <= 0.0 {
            return Err(SectorFault::InvalidGeometry {
                id: planet.id.to_string(),
            });
        }
        stand_inside(input, &mut standing, &planet.id, planet.position, clearance)?;
    }
    for ship in &manifest.ships {
        own(&ship.id)?;
        let spawn = &ship.spawn;
        if !spawn.initial_velocity.is_finite() {
            return Err(refuse(
                &ship.id,
                "initial_velocity",
                format!("{:?}", spawn.initial_velocity.get()),
            ));
        }
        if !ship.rotation.is_finite()
            || (ship.rotation.length() - 1.0).abs() > ROTATION_LENGTH_TOLERANCE
        {
            return Err(SectorFault::InvalidGeometry {
                id: ship.id.to_string(),
            });
        }
        let clearance = ship.clearance.get();
        if !clearance.is_finite() || clearance <= 0.0 {
            return Err(SectorFault::InvalidGeometry {
                id: ship.id.to_string(),
            });
        }
        if ship.clearance > SECTOR_SHIP_CLEARANCE_MAX {
            return Err(refuse(
                &ship.id,
                "clearance",
                format!(
                    "{clearance} m, above the {} m maximum",
                    SECTOR_SHIP_CLEARANCE_MAX.get()
                ),
            ));
        }
        let design = match &spawn.design {
            ShipDesignSource::Inline(design) => design,
            ShipDesignSource::Prototype { id, .. } => {
                return Err(refuse(
                    &ship.id,
                    "design",
                    format!("the catalog design '{id}', not an inline design"),
                ));
            }
        };
        check_ship_design(design).map_err(|value| refuse(&ship.id, "design", value))?;
        if spawn.allegiance.is_none() {
            return Err(refuse(
                &ship.id,
                "allegiance",
                "none; a generated ship states its side".to_string(),
            ));
        }
        check_ship_controller(ship.condition, &spawn.controller)
            .map_err(|value| refuse(&ship.id, "controller", value))?;
        stand_inside(
            input,
            &mut standing,
            &ship.id,
            ship.position,
            ship.clearance,
        )?;
    }
    let SectorManifest {
        coord,
        asteroids,
        planets,
        ships,
    } = manifest;
    Ok(SectorDescription {
        coord,
        asteroids,
        planets,
        ships,
    })
}

/// Refuse a controller the streamed world cannot host: a player (the
/// scenario spawns the one player), an AI on a derelict (its systems spawn
/// inactive), or an AI route the controller could not fly, by the same
/// [`patrol_stops_fault`] the scenario lint and the spawn apply, plus a
/// waypoint that is not finite, a leash or an engage delay that is not
/// finite, or a leash that is not positive.
fn check_ship_controller(
    condition: SectorShipConditionType,
    controller: &SpaceshipController,
) -> Result<(), String> {
    let config = match (condition, controller) {
        (_, SpaceshipController::Player(_)) => {
            return Err("a player; the scenario spawns the player".to_string());
        }
        (_, SpaceshipController::None) => return Ok(()),
        (SectorShipConditionType::Derelict, SpaceshipController::AI(_)) => {
            return Err("an AI on a derelict".to_string());
        }
        (SectorShipConditionType::Intact, SpaceshipController::AI(config)) => config,
    };
    if !config.patrol.iter().copied().all(position_is_finite) {
        return Err("a patrol waypoint is not finite".to_string());
    }
    if let Some(fault) = patrol_stops_fault(config) {
        return Err(fault);
    }
    if let Some(leash) = config
        .leash
        .map(Meters::get)
        .filter(|leash| !leash.is_finite() || *leash <= 0.0)
    {
        return Err(format!("leash {leash} m is not a finite positive distance"));
    }
    if let Some(delay) = config.engage_delay.filter(|delay| !delay.is_finite()) {
        return Err(format!("engage delay {delay} s is not finite"));
    }
    Ok(())
}

/// Refuse a design the worker can tell is broken without the catalog: no
/// sections, a section id used twice, an inline section rather than a
/// prototype reference, a section pose that is not finite, or a presentation
/// sound that is a runtime handle rather than an asset path. Whether each
/// prototype resolves is main-thread work in `materialize_sector`.
fn check_ship_design(design: &ShipDesign) -> Result<(), String> {
    // Exhaustive, so a new presentation field is a compile error here until
    // it is sorted into an asset ref (checked) or a plain value (skipped).
    let ShipPresentationConfig {
        skin: _,
        style: _,
        collapse_sound,
        lock_on_sound,
        lock_off_sound,
        radar_deny_sound,
        radar_retarget_sound,
        safety_on_sound,
        warn_lock_sound,
        ammo_dry_sound,
        warn_hull_sound,
        warn_hull_fraction: _,
        rcs_loop_sound,
    } = &design.presentation;
    for (field, sound) in [
        ("collapse_sound", collapse_sound),
        ("lock_on_sound", lock_on_sound),
        ("lock_off_sound", lock_off_sound),
        ("radar_deny_sound", radar_deny_sound),
        ("radar_retarget_sound", radar_retarget_sound),
        ("safety_on_sound", safety_on_sound),
        ("warn_lock_sound", warn_lock_sound),
        ("ammo_dry_sound", ammo_dry_sound),
        ("warn_hull_sound", warn_hull_sound),
        ("rcs_loop_sound", rcs_loop_sound),
    ] {
        if let Some(AssetRef::Handle(handle)) = sound {
            return Err(format!(
                "presentation {field} is the runtime handle {handle:?}, not an asset path"
            ));
        }
    }
    if design.sections.is_empty() {
        return Err("no sections".to_string());
    }
    let mut ids = BTreeSet::new();
    for section in &design.sections {
        if !ids.insert(section.id.as_str()) {
            return Err(format!("section id '{}' used twice", section.id));
        }
        if let SectionSource::Inline(_) = section.source {
            return Err(format!(
                "section '{}' is inline, not a prototype reference",
                section.id
            ));
        }
        if !section.position.is_finite() || !section.rotation.is_finite() {
            return Err(format!("section '{}' has a non-finite pose", section.id));
        }
    }
    Ok(())
}

/// Refuse a body that does not stand wholly inside its own cell, or that
/// overlaps a body already checked.
///
/// The whole clearance sphere, not only the centre: a body across a face
/// would be retired with the neighbour. A generator's placement inset exists
/// to keep this true, and [`crate::WorldGeometry::require_owning_edge`] is how
/// it refuses a cell too narrow for its inset.
fn stand_inside(
    input: SectorGenerationInput,
    standing: &mut Vec<Occupied>,
    id: &SectorObjectId,
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
    if let Some(other) = standing.iter().find(|other| {
        !bodies_clear(
            other.centre,
            other.clearance,
            position,
            clearance,
            Meters(0.0),
        )
    }) {
        let at = other.centre.get();
        return Err(SectorFault::Manifest {
            id: id.to_string(),
            field: "position",
            value: format!(
                "overlapping the body at {:.0} {:.0} {:.0} m",
                at.x, at.y, at.z
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
    let validate = info_span!("nova_world::validate_config").entered();
    config.validate()?;
    if !position_is_finite(coord.centre(config.sector_edge)) {
        return Err(SectorFault::InvalidGeometry { id: coord.slug() });
    }
    let input = config.input(coord);
    drop(validate);
    let manifest =
        info_span!("nova_world::generate").in_scope(|| config.generator.generate(input))?;
    info_span!("nova_world::validate_manifest").in_scope(|| validate_manifest(input, manifest))
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
    prepare_cell(config, coord, None)
}

/// [`prepare_sector`] for a cell that may hold a frozen record.
///
/// The one place the record rule is decided: a visited cell's record
/// replaces its generated rocks and ships, so they are neither meshed nor
/// returned; a never-visited cell's arrivals join its generated bodies. The
/// description is drawn either way, because planetoids never freeze and a
/// held ship's record names only its id. Each frozen rock is prepared from
/// its record: its pristine geometry, and for a carved one the surface its
/// field meshes to.
pub(crate) fn prepare_cell<G: SectorGenerator>(
    config: WorldConfig<G>,
    coord: SectorCoord,
    record: Option<Arc<FrozenSector>>,
) -> Result<PreparedSector, SectorFault> {
    let _job = info_span!("nova_world::prepare_sector", cell = %coord).entered();
    let description = generate_sector(&config, coord)?;
    let SectorDescription {
        coord,
        asteroids,
        planets,
        ships,
    } = description;
    let planets = info_span!("nova_world::prepare_planets").in_scope(|| {
        planets
            .into_iter()
            .map(|body| PreparedPlanetoid {
                surface: prepare_planet(body.config.clone()),
                body,
            })
            .collect()
    });
    let thaw = |bodies: Vec<_>, ships: &[SectorShip]| {
        info_span!("nova_world::prepare_frozen_bodies").in_scope(|| {
            bodies
                .into_iter()
                .map(|body| PreparedFrozenBody::prepare(coord, body, ships))
                .collect::<Vec<_>>()
        })
    };
    let (visited, arrivals) = match record.map(Arc::unwrap_or_clone) {
        None => (None, Vec::new()),
        Some(FrozenSector::Visited(bodies)) => (Some(bodies), Vec::new()),
        Some(FrozenSector::Arrivals(bodies)) => (None, bodies),
    };
    let bodies = match visited {
        Some(bodies) => PreparedBodies::Thawed(thaw(bodies, &ships)),
        None => {
            let arrivals = thaw(arrivals, &ships);
            let rocks = info_span!("nova_world::prepare_asteroids").in_scope(|| {
                asteroids
                    .into_iter()
                    .map(|body| PreparedRock {
                        geometry: prepare_asteroid_geometry(body.seed(), body.config.radius),
                        body,
                    })
                    .collect()
            });
            PreparedBodies::Fresh {
                rocks,
                ships,
                arrivals,
            }
        }
    };
    Ok(PreparedSector {
        coord,
        planets,
        bodies,
    })
}
