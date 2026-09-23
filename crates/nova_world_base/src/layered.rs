//! The base game's streamed world: which rocks, worlds and ships a cell holds.
//!
//! Policy, not mechanism. `nova_world` owns the feature field, the placement
//! rules and the check every manifest passes; this module decides what the
//! field's places are FILLED with, out of shipped content, and takes no list
//! from a caller.

use nova_events::prelude::{Meters, Meters3};
use nova_gameplay::prelude::SeedStream;
use nova_scenario::prelude::{
    asteroid_seed_from_id, PlanetConfig, PlanetType, ASTEROID_GEOMETRIC_FACTOR_MAX, KIND_CARBON,
    KIND_ICE, KIND_METAL, KIND_ROCK,
};
use nova_world::prelude::*;

use crate::{BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID, BLOCK_WRECK_PLATE_SHIP_ID};

/// Every natural asteroid kind the game ships. `plain` is absent because it is
/// the rendering control, not a rock a world would contain.
const ASTEROID_KINDS: [&str; 4] = [KIND_ROCK, KIND_METAL, KIND_ICE, KIND_CARBON];

/// The shipped designs a derelict field is drawn from: the damaged frame
/// tender and loose wreck plating, the two base hulls that are already wrecks.
const DERELICT_DESIGNS: [&str; 2] = [
    BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID,
    BLOCK_WRECK_PLATE_SHIP_ID,
];

/// The nominal radius band a cell draws rocks from.
///
/// The meshed rock reaches 3.5-6x past the nominal figure, so 30-60 m nominal
/// draws about 210-720 m diameters - scattered landmarks in a 32 km cell.
const ASTEROID_RADIUS: (Meters, Meters) = (Meters(30.0), Meters(60.0));

/// The mean-radius band a gated planetoid is drawn from.
///
/// 600-1,200 m: big enough to read as a WORLD against a 60 m rock beside it,
/// small enough that a 32 km cell is still a place you fly across rather than
/// a place a single body fills. A planet's radius is its real size, not a
/// designation, so this is what it draws.
const PLANETOID_RADIUS: (Meters, Meters) = (Meters(600.0), Meters(1_200.0));

/// How many ships one derelict sphere places.
const DERELICT_SHIPS: (usize, usize) = (1, 3);

/// How far a derelict may drift from its sphere's centre.
///
/// A CLUSTER radius, not a scatter: a derelict field reads as a place because
/// its wrecks are near each other. Wide enough that they still clear a
/// planetoid sharing the cell without exhausting their candidates.
const DERELICT_SPREAD: Meters = Meters(6_000.0);

/// How many deterministic candidates a derelict or a rock gets before the cell
/// refuses.
///
/// A cap rather than a loop until it fits: the draw is deterministic, so if
/// the cell really has no room the search does not terminate on its own, and
/// a cell that silently dropped the object would hide a field asking for more
/// than a cell can hold.
const PLACEMENT_ATTEMPTS: usize = 64;

/// The widest cell edge this generator fills: the widest edge
/// [`SECTOR_FEATURES_MAX`] and [`SECTOR_BODIES_MAX`] were measured at.
///
/// A cell lists more spheres and places more bodies as its volume grows, up
/// to 31 spheres and 47 bodies at the 447 km edge the thinning halo allows.
/// Refusing the edge at the config fails before anything streams, where a
/// cap refusing a drawn cell would fail mid-stream.
const SECTOR_EDGE_MAX: Meters = Meters(128_000.0);

/// The base game's sector generator: a cell filled from the feature field.
///
/// Each cell asks the field what reaches it and fills itself from the answer:
/// one planetoid per owned planet sphere, one to three derelicts per owned
/// derelict sphere, then rocks from the combined asteroid influence at the
/// cell's centre. Every choice is drawn from the cell's own seeded streams in
/// a fixed order, so a cell is the same cell in any visit order.
///
/// No fields. The content it draws from is the shipped content named in this
/// module - every natural asteroid kind, every [`PlanetType`], and the shipped
/// derelict hulls - so there is no list a caller could leave empty or fill
/// with an id the game does not ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NovaLayeredWorld;

impl SectorGenerator for NovaLayeredWorld {
    /// Refuse an edge the feature field cannot be thinned at, one wider than
    /// [`SECTOR_EDGE_MAX`], and one too narrow to own the widest body this
    /// generator places.
    fn validate(&self, geometry: WorldGeometry) -> Result<(), SectorFault> {
        validate_feature_geometry(geometry)?;
        if geometry.sector_edge > SECTOR_EDGE_MAX {
            return Err(SectorFault::Config {
                field: "sector_edge",
                value: format!(
                    "{} m, wider than the {} m the manifest caps were measured at",
                    geometry.sector_edge.get(),
                    SECTOR_EDGE_MAX.get()
                ),
            });
        }
        let (clearance, body) = widest_body();
        geometry.require_owning_edge(clearance, &body)
    }

    fn generate(&self, input: SectorGenerationInput) -> Result<SectorManifest, SectorFault> {
        let coord = input.coord;
        let centre = coord.centre(input.geometry.sector_edge);
        let features = sector_features(input)?;
        let mut strengths = [0.0; FeatureLayer::COUNT];
        for sphere in &features {
            strengths[sphere.layer.index()] += sphere.influence(centre);
        }
        for strength in &mut strengths {
            *strength = strength.min(1.0);
        }
        let owned = |layer: FeatureLayer| {
            features
                .iter()
                .filter(move |sphere| sphere.layer == layer && sphere.owner == coord)
        };
        let mut layout = Layout::new(input);

        // Planetoids first: a planet sphere places its world AT its centre,
        // which is the one pose in a cell nothing else gets to move.
        // Everything after it works around what is already there.
        let mut stream = input.stream("planets");
        let mut planets = Vec::new();
        for sphere in owned(FeatureLayer::Planet) {
            let id = sector_id(coord, "planet", planets.len());
            let (radius_min, radius_max) = PLANETOID_RADIUS;
            let radius = radius_min + (radius_max - radius_min) * stream.unit();
            let planet_type = PlanetType::ALL[stream.next_u32() as usize % PlanetType::ALL.len()];
            let config = PlanetConfig::new(planet_type, radius, stream.next_u32());
            layout.stand_at(&id, sphere.centre, config.body_radius())?;
            planets.push(SectorPlanet {
                id,
                feature: sphere.id.clone(),
                position: sphere.centre,
                config,
            });
        }

        let mut stream = input.stream("derelicts");
        let mut ships = Vec::new();
        for sphere in owned(FeatureLayer::Derelict) {
            let (ships_min, ships_max) = DERELICT_SHIPS;
            let count = ships_min + stream.next_u32() as usize % (ships_max - ships_min + 1);
            for _ in 0..count {
                let id = sector_id(coord, "ship", ships.len());
                let position = layout.place_near(
                    &mut stream,
                    &id,
                    sphere.centre,
                    DERELICT_SPREAD,
                    SECTOR_SHIP_CLEARANCE,
                )?;
                let yaw = stream.unit() * std::f32::consts::TAU;
                let design = DERELICT_DESIGNS[stream.next_u32() as usize % DERELICT_DESIGNS.len()];
                ships.push(SectorShip {
                    id,
                    feature: sphere.id.clone(),
                    position,
                    yaw,
                    design: design.to_string(),
                });
            }
        }

        // Rocks come off the COMBINED asteroid influence AT THE CELL CENTRE,
        // so two spheres covering that one point fill the cell more than
        // either would alone. A sphere contributes nothing to a cell whose
        // centre it does not reach - including a fringe cell it only clips a
        // corner of, which `sector_features` still lists among the cell's
        // spheres because it tests the whole box. CEILING, not rounding, over
        // the cells that ARE covered: one whose centre a belt reaches at all
        // holds at least one rock. Rounding put a whole outer shell of covered
        // cells at zero rocks, so the belt had a hard edge one cell inside its
        // own rim and the falloff bought nothing.
        let count = ((strengths[FeatureLayer::Asteroid.index()] * SECTOR_ASTEROIDS_MAX as f32)
            .ceil() as usize)
            .min(SECTOR_ASTEROIDS_MAX);
        let edge = input.geometry.sector_edge;
        let reach = Meters(edge.get() * 0.5 * PLACEMENT_INSET);
        let (radius_min, radius_max) = ASTEROID_RADIUS;
        let mut stream = input.stream("bodies");
        let mut asteroids = Vec::with_capacity(count);
        for index in 0..count {
            let id = sector_id(coord, "body", index);
            let radius = radius_min + (radius_max - radius_min) * stream.unit();
            let clearance = Meters(radius.get() * ASTEROID_GEOMETRIC_FACTOR_MAX);
            let position = layout.place_near(&mut stream, &id, centre, reach, clearance)?;
            let kind = ASTEROID_KINDS[stream.next_u32() as usize % ASTEROID_KINDS.len()];
            asteroids.push(SectorAsteroid {
                seed: asteroid_seed_from_id(&id),
                id,
                position,
                radius,
                kind: kind.to_string(),
            });
        }

        Ok(SectorManifest {
            coord,
            features,
            strengths,
            asteroids,
            planets,
            ships,
        })
    }
}

/// The clearance spheres already standing in one cell while
/// [`NovaLayeredWorld`] describes it.
///
/// It keeps the placement rules `validate_manifest` checks afterwards - a body
/// inside the cell's inset, clear of everything placed before it by the same
/// [`bodies_clear`] - while the cell is built, so a crowded cell is reported
/// as the object that did not fit rather than as a manifest refused after the
/// fact.
struct Layout {
    input: SectorGenerationInput,
    standing: Vec<(Meters3, Meters)>,
}

impl Layout {
    fn new(input: SectorGenerationInput) -> Self {
        Self {
            input,
            standing: Vec::new(),
        }
    }

    /// Whether a body at `centre` claiming `clearance` keeps the margin from
    /// everything already standing.
    fn clears(&self, centre: Meters3, clearance: Meters) -> bool {
        self.standing.iter().all(|&(other, other_clearance)| {
            bodies_clear(other, other_clearance, centre, clearance)
        })
    }

    /// Stand a body exactly at `position`: the one pose in a cell nothing else
    /// gets to move, such as a planetoid at its sphere's centre.
    fn stand_at(
        &mut self,
        id: &str,
        position: Meters3,
        clearance: Meters,
    ) -> Result<(), SectorFault> {
        if !self.clears(position, clearance) {
            return Err(SectorFault::Clearance {
                id: id.to_string(),
                attempts: 1,
            });
        }
        self.standing.push((position, clearance));
        Ok(())
    }

    /// Draw a clear place within `reach` of `anchor` and inside the cell's
    /// inset, and stand the body there.
    ///
    /// Candidates come off `stream` in a fixed order, so the placement is a
    /// function of the cell and not of the wall clock.
    fn place_near(
        &mut self,
        stream: &mut SeedStream,
        id: &str,
        anchor: Meters3,
        reach: Meters,
        clearance: Meters,
    ) -> Result<Meters3, SectorFault> {
        let edge = self.input.geometry.sector_edge;
        let inset = edge.get() * 0.5 * PLACEMENT_INSET;
        let cell_centre = self.input.coord.centre(edge);
        for _ in 0..PLACEMENT_ATTEMPTS {
            let candidate = anchor
                + Meters3::new(
                    stream.signed() * reach.get(),
                    stream.signed() * reach.get(),
                    stream.signed() * reach.get(),
                );
            let offset = (candidate.get() - cell_centre.get()).abs();
            if offset.max_element() > inset || !self.clears(candidate, clearance) {
                continue;
            }
            self.standing.push((candidate, clearance));
            return Ok(candidate);
        }
        Err(SectorFault::Clearance {
            id: id.to_string(),
            attempts: PLACEMENT_ATTEMPTS,
        })
    }
}

/// The widest clearance sphere this generator can put in one cell, and what
/// draws it.
///
/// Every quantity here is one the placement already spaces by: a rock's
/// clearance is its nominal radius times [`ASTEROID_GEOMETRIC_FACTOR_MAX`], a
/// planetoid's is the OUTER radius it draws at - the mesh spans `1 +/-
/// relief`, so the mean band alone would under-measure every type by its
/// relief - and a ship's is [`SECTOR_SHIP_CLEARANCE`].
fn widest_body() -> (Meters, String) {
    let rock = Meters(ASTEROID_RADIUS.1.get() * ASTEROID_GEOMETRIC_FACTOR_MAX);
    let planet = PlanetType::ALL
        .iter()
        .map(|planet_type| PlanetConfig::new(*planet_type, PLANETOID_RADIUS.1, 0).body_radius())
        .fold(Meters(0.0), |widest, radius| widest.max(radius));
    if planet >= rock && planet >= SECTOR_SHIP_CLEARANCE {
        (
            planet,
            format!("a gated planetoid reaching {} m", planet.get()),
        )
    } else if rock >= SECTOR_SHIP_CLEARANCE {
        (
            rock,
            format!("a gated asteroid drawn at {} m", ASTEROID_RADIUS.1.get()),
        )
    } else {
        (
            SECTOR_SHIP_CLEARANCE,
            format!("a derelict at {} m", SECTOR_SHIP_CLEARANCE.get()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layered(sector_edge: Meters) -> WorldConfig<NovaLayeredWorld> {
        WorldConfig {
            seed: 20_260_922,
            sector_edge,
            active_radius: 2,
            generator: NovaLayeredWorld,
        }
    }

    /// The floor is set by the widest planetoid, at its OUTER radius.
    ///
    /// About 8.48 km for a volcanic world: the 1,200 m band is a MEAN radius
    /// and the mesh spans `1 +/- relief`, so the body reaches 1,272 m - the
    /// same `body_radius` the placement spaces by. 8,100 m clears the floor
    /// the mean band alone would give and is still a cell the planetoid
    /// crosses. A gated rock reaches 360 m and a derelict 400 m, so naming
    /// which body set the floor tells a caller what they would have to shrink.
    #[test]
    fn an_edge_too_narrow_to_own_its_planetoids_is_refused() {
        for edge in [Meters(7_999.0), Meters(8_100.0), Meters(8_479.0)] {
            let fault = layered(edge)
                .validate()
                .expect_err("a cell narrower than its own planetoids must refuse");
            assert!(
                matches!(
                    &fault,
                    SectorFault::Config { field: "sector_edge", value }
                        if value.contains("planetoid")
                ),
                "a {edge:?} cell must refuse by naming the planetoid, got {fault:?}"
            );
        }
        assert!(
            layered(Meters(8_481.0)).validate().is_ok(),
            "a cell just wide enough to own a 1,272 m planetoid must arm"
        );
    }

    /// The manifest caps were measured at exactly 128 km, so that edge arms
    /// and the next `f32` above it refuses by naming the edge and the cap.
    #[test]
    fn an_edge_wider_than_the_measured_caps_is_refused() {
        assert!(
            layered(SECTOR_EDGE_MAX).validate().is_ok(),
            "the 128 km edge the caps were measured at must arm"
        );
        assert_eq!(
            layered(Meters(SECTOR_EDGE_MAX.get().next_up())).validate(),
            Err(SectorFault::Config {
                field: "sector_edge",
                value: "128000.01 m, wider than the 128000 m the manifest caps were measured at"
                    .to_string(),
            }),
        );
    }
}
