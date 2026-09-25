//! The generation mechanisms: the untrusted [`SectorManifest`] a generator
//! returns, and the check that turns it into a trusted [`SectorDescription`]
//! before a worker prepares it.
//!
//! PURE. Nothing here touches a `World`, reads a resource or draws from the
//! ambient RNG, which is what lets [`prepare_sector`] run on a worker and what
//! makes a cell the same cell in any visit order. A [`SectorGenerator`] is
//! held to the same rule: it gets a seed, an edge and a coordinate, and
//! nothing else.

use std::collections::BTreeSet;

use bevy::log::info_span;
use nova_events::prelude::{Meters, Meters3};
use nova_scenario::prelude::{
    is_asteroid_kind, prepare_asteroid_geometry, prepare_planet, AsteroidKindId, PlanetConfig,
    PreparedAsteroid, PreparedPlanet, ShipDesignId, ASTEROID_GEOMETRIC_FACTOR_MAX,
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

/// One generated ship: a hull with nobody aboard and nobody's side.
#[derive(Clone, Debug)]
pub struct SectorShip {
    /// The ship's scenario id, prefixed with its owning cell's slug.
    pub id: String,
    /// Where it floats, in meters from the world origin.
    pub position: Meters3,
    /// Which way it is pointing. Yaw only - the hull sits level.
    pub yaw: f32,
    /// The catalog design it is built from.
    pub design: ShipDesignId,
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
    /// same when this matches. Positions are printed at centimeter resolution
    /// rather than compared as floats, so the comparison is a fact about the
    /// generator and not about the last bit of an f32.
    pub fn canonical(&self) -> String {
        let point = |position: Meters3| {
            let p = position.get();
            format!("{:.2} {:.2} {:.2}", p.x, p.y, p.z)
        };
        let mut out = format!("{}\n", self.coord);
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
                "planet {} {} {:?} r{:.2} s{}\n",
                planet.id,
                point(planet.position),
                planet.config.planet_type,
                planet.config.radius.get(),
                planet.config.seed
            ));
        }
        for ship in &self.ships {
            out.push_str(&format!(
                "ship {} {} y{:.4} {}\n",
                ship.id,
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

/// How much room [`validate_manifest`] assumes a generated ship fills.
///
/// A materialization safety approximation, not a distribution rule: a ship's
/// sections are resolved from the catalog on the main thread, so a worker
/// checking where the ship stands cannot measure the hull. This radius around
/// the ship root stands in for it when the check keeps a ship inside its cell
/// and clear of the other bodies. Generous on purpose. A generator may space
/// its ships wider, but never narrower.
pub const SECTOR_SHIP_CLEARANCE: Meters = Meters(400.0);

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
/// it; no two bodies overlapping; shipped asteroid kinds; and planet configs
/// that [`PlanetConfig::validate`] accepts. How many bodies a generator places
/// and how far apart it spaces them is its own policy; this check only refuses
/// what cannot be materialized. The ship design is only checked for a blank id
/// here; the catalog lookup is main-thread work in `materialize_sector`.
///
/// # Errors
///
/// [`SectorFault::Config`] on `sector_edge` for an edge that is not a finite
/// positive length, and [`SectorFault::InvalidGeometry`] for a cell whose
/// centre has no finite position in meters;
/// [`SectorFault::Manifest`] for the wrong cell, an object outside its cell or
/// overlapping another, an id another cell owns, a planet config
/// [`PlanetConfig::validate`] refuses, or a blank ship design;
/// [`SectorFault::InvalidGeometry`] for non-finite geometry;
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
        if !ship.yaw.is_finite() {
            return Err(SectorFault::InvalidGeometry {
                id: ship.id.clone(),
            });
        }
        if ship.design.as_str().trim().is_empty() {
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
