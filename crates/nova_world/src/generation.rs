//! The generation mechanisms: the untrusted [`SectorManifest`] a generator
//! returns, and the check that turns it into a trusted [`SectorDescription`]
//! before a worker prepares it.
//!
//! PURE. Nothing here touches a `World`, reads a resource or draws from the
//! ambient RNG, which is what lets [`prepare_sector`] run on a worker and what
//! makes a cell the same cell in any visit order. A [`SectorGenerator`] is
//! held to the same rule: it gets a seed, an edge and a coordinate, and
//! nothing else.

use std::{collections::BTreeSet, fmt};

use bevy::{log::info_span, math::Quat};
use nova_events::prelude::{Meters, Meters3, MetersPerSecond3};
use nova_gameplay::prelude::{Fnv32, SeedStream, ShipInventoryStock};
use nova_scenario::prelude::{
    is_asteroid_kind, prepare_asteroid_geometry, prepare_planet, AsteroidKindId, PlanetConfig,
    PreparedAsteroid, PreparedPlanet, SectionSource, ShipDesign, ASTEROID_GEOMETRIC_FACTOR_MAX,
};

use crate::{SectorCoord, SectorFault, SectorGenerationInput, SectorGenerator, WorldConfig};

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
    pub kind: AsteroidKindId,
    /// Its silhouette seed.
    pub seed: u32,
    /// Explicit initial motion in meters per second. Zero is an intentional
    /// coasting start and does not disable gravity.
    pub initial_velocity: MetersPerSecond3,
}

/// One generated planetoid: a real [`PlanetConfig`], not a big rock.
#[derive(Clone, Debug)]
pub struct SectorPlanet {
    /// The planetoid's scenario id, prefixed with its owning cell's slug.
    pub id: String,
    /// Where it stands, in meters from the world origin.
    pub position: Meters3,
    /// The world it is, ready for `prepare_planet`.
    pub config: PlanetConfig,
}

/// Whether a generated ship still works or is a derelict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

/// One generated ship: a hull with nobody aboard and nobody's side.
#[derive(Clone, Debug)]
pub struct SectorShip {
    /// The ship's scenario id, prefixed with its owning cell's slug.
    pub id: String,
    /// Where its root floats, in meters from the world origin.
    pub position: Meters3,
    /// Its full orientation: a finite unit quaternion.
    pub rotation: Quat,
    /// Explicit initial motion in meters per second.
    pub initial_velocity: MetersPerSecond3,
    /// The radius around the root that holds the whole hull in any
    /// orientation. At most [`SECTOR_SHIP_CLEARANCE_MAX`].
    pub clearance: Meters,
    /// The generated design, carried inline: every section names a section
    /// prototype of the loaded catalog.
    pub design: ShipDesign,
    /// Whether it works or is a derelict.
    pub condition: SectorShipConditionType,
    /// The civilization it belongs to, or belonged to as a derelict.
    pub civilization: CivilizationId,
    /// What it was built for. A derelict keeps its former role.
    pub role: ShipRoleType,
    /// What its hold carries when it spawns. The spawn refuses stock past
    /// the hold its resolved design gives it.
    pub stock: ShipInventoryStock,
    /// Its credit balance when it spawns.
    pub credits: u32,
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
    /// same when this matches. Positions and radii are printed at centimeter
    /// resolution and yaw at a ten-thousandth of a radian, deliberately
    /// rounded, so the comparison is a fact about the generator and not about
    /// the last bit of an f32. A rock's velocity and a planetoid's
    /// fields - relief, sea level, mass and lock signature - are authoring
    /// values, not placement: they are printed exact, as the shortest text
    /// that reads back to the same f32, and `None` prints apart from every
    /// `Some`. Rounding them would call two different worlds the same one.
    /// A ship's stock prints every stack and count, and its credits their
    /// balance. Its civilization, role,
    /// design integrity, design presentation and each section's prototype
    /// patch print through `Debug`: field by field in declaration order, a
    /// float as the shortest text that reads back to it, a non-finite float
    /// as `NaN` or `inf`, apart from `None`. An asset handle prints its
    /// runtime id, so a design that carries one compares equal only within
    /// one run; generated designs carry none.
    pub fn canonical(&self) -> String {
        let point = |position: Meters3| {
            let p = position.get();
            format!("{:.2} {:.2} {:.2}", p.x, p.y, p.z)
        };
        let mut out = format!("{}\n", self.coord);
        for body in &self.asteroids {
            out.push_str(&format!(
                "asteroid {} {} r{:.2} {} s{} velocity {:?}\n",
                body.id,
                point(body.position),
                body.radius.get(),
                body.kind,
                body.seed,
                body.initial_velocity.get()
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
            out.push_str(&format!(
                "ship {} {} {} velocity {:?} c{:.2} {} {} {} stock {:?} credits {} integrity {:?} \
                 presentation {:?}\n",
                ship.id,
                point(ship.position),
                canonical_rotation(ship.rotation),
                ship.initial_velocity.get(),
                ship.clearance.get(),
                ship.condition.label(),
                ship.civilization,
                ship.role.label(),
                ship.stock.stacks().collect::<Vec<_>>(),
                ship.credits,
                ship.design.integrity,
                ship.design.presentation,
            ));
            for section in &ship.design.sections {
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
/// `materialize_sector` needs that a worker can produce.
///
/// Built only by [`prepare_sector`], and the fields are private, so
/// `asteroids` is always one prepared rock per description asteroid and
/// `planets` one prepared world per description planetoid, both in order. The
/// two halves are not the same shape: a [`PreparedAsteroid`] is the GEOMETRY
/// only, and the rock's config is rebuilt from the description at spawn, while
/// a [`PreparedPlanet`] carries its config beside its visual. A ship needs no
/// preparation: its inline design's sections are resolved against the section
/// catalog on the main thread, which is the one place the catalog exists.
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
pub fn sector_id(coord: SectorCoord, name: &str, index: usize) -> String {
    format!("{}_{name}_{index}", coord.slug())
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
/// velocities; and planet configs that [`PlanetConfig::validate`] accepts. How many bodies a generator places
/// and how far apart it spaces them is its own policy; this check only refuses
/// what cannot be materialized. A ship's rotation must be a finite unit
/// quaternion, its declared clearance positive and at most
/// [`SECTOR_SHIP_CLEARANCE_MAX`], and its clearance sphere is the one that
/// must stand inside the cell. Its inline design is checked for what needs no
/// catalog; the strict resolve against the loaded sections is main-thread work
/// in `materialize_sector`.
///
/// # Errors
///
/// [`SectorFault::Config`] on `sector_edge` for an edge that is not a finite
/// positive length, and [`SectorFault::InvalidGeometry`] for a cell whose
/// centre has no finite position in meters;
/// [`SectorFault::Manifest`] for the wrong cell, an object outside its cell or
/// overlapping another, an id another cell owns, a non-finite initial
/// velocity, a planet config
/// [`PlanetConfig::validate`] refuses, a ship clearance above the maximum, or
/// a ship design with no sections, a repeated section id, an inline section or
/// a non-finite section pose;
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
        if !body.initial_velocity.is_finite() {
            return Err(refuse(
                &body.id,
                "initial_velocity",
                format!("{:?}", body.initial_velocity.get()),
            ));
        }
        let clearance = Meters(body.radius.get() * ASTEROID_GEOMETRIC_FACTOR_MAX);
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
                id: planet.id.clone(),
            });
        }
        stand_inside(input, &mut standing, &planet.id, planet.position, clearance)?;
    }
    for ship in &manifest.ships {
        own(&ship.id)?;
        if !ship.initial_velocity.is_finite() {
            return Err(refuse(
                &ship.id,
                "initial_velocity",
                format!("{:?}", ship.initial_velocity.get()),
            ));
        }
        if !ship.rotation.is_finite()
            || (ship.rotation.length() - 1.0).abs() > ROTATION_LENGTH_TOLERANCE
        {
            return Err(SectorFault::InvalidGeometry {
                id: ship.id.clone(),
            });
        }
        let clearance = ship.clearance.get();
        if !clearance.is_finite() || clearance <= 0.0 {
            return Err(SectorFault::InvalidGeometry {
                id: ship.id.clone(),
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
        check_ship_design(ship).map_err(|value| refuse(&ship.id, "design", value))?;
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

/// Refuse a design the worker can tell is broken without the catalog: no
/// sections, a section id used twice, an inline section rather than a
/// prototype reference, or a section pose that is not finite. Whether each
/// prototype resolves is main-thread work in `materialize_sector`.
fn check_ship_design(ship: &SectorShip) -> Result<(), String> {
    if ship.design.sections.is_empty() {
        return Err("no sections".to_string());
    }
    let mut ids = BTreeSet::new();
    for section in &ship.design.sections {
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
    let _job = info_span!("nova_world::prepare_sector", cell = %coord).entered();
    let description = generate_sector(&config, coord)?;
    let asteroids = info_span!("nova_world::prepare_asteroids").in_scope(|| {
        description
            .asteroids
            .iter()
            .map(|body| prepare_asteroid_geometry(body.seed, body.radius))
            .collect()
    });
    let planets = info_span!("nova_world::prepare_planets").in_scope(|| {
        description
            .planets
            .iter()
            .map(|planet| prepare_planet(planet.config.clone()))
            .collect()
    });
    Ok(PreparedSector {
        description,
        asteroids,
        planets,
    })
}
