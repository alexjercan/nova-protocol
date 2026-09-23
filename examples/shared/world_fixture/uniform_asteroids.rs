//! The streaming baseline generator: every cell filled the same way from its
//! own seed.
//!
//! Example-owned, not a `nova_world` type: it is the world the streaming loop
//! is judged against, not a world the game ships. It implements
//! [`SectorGenerator`] from outside the crate exactly as the base game's
//! generator does, so it goes through the same manifest check.

use nova_protocol::prelude::*;
use nova_world::prelude::*;

/// How many candidates a rock draws before the cell refuses. A cap, because
/// the draw is deterministic and a cell with no room would never finish.
const PLACEMENT_ATTEMPTS: usize = 64;

/// Every cell gets the same treatment out of its own seed: `body_count` rocks
/// scattered across its inset, nominal radius drawn from the band.
///
/// Nothing about the WORLD can explain away a sector that failed to come up,
/// which is what makes it the right generator for judging a retirement or a
/// crossing.
///
/// Its placement is its own and deliberately plain: each rock draws candidates
/// across the cell's inset until one clears every rock before it.
///
/// No `Default`: a body count and a radius band nobody chose are how a number
/// nobody chose reaches a frame.
#[derive(Clone, Debug, PartialEq)]
pub struct UniformAsteroids {
    /// Rocks generated per sector, from one to [`SECTOR_ASTEROIDS_MAX`].
    pub body_count: usize,
    /// Smallest nominal body radius drawn.
    pub radius_min: Meters,
    /// Largest nominal body radius drawn.
    pub radius_max: Meters,
    /// The asteroid kind ids a body may be drawn from.
    pub asteroid_kinds: Vec<&'static str>,
}

impl SectorGenerator for UniformAsteroids {
    /// Refuse a count, a band or a kind table that cannot describe a sector,
    /// and an edge too narrow to own the widest rock. An empty table is a
    /// refusal and never a silent fallback to a house rock.
    fn validate(&self, geometry: WorldGeometry) -> Result<(), SectorFault> {
        let refuse = |field: &'static str, value: String| Err(SectorFault::Config { field, value });
        if self.body_count == 0 || self.body_count > SECTOR_ASTEROIDS_MAX {
            return refuse(
                "generator.body_count",
                format!(
                    "{}, outside the 1 to {SECTOR_ASTEROIDS_MAX} rocks a cell holds",
                    self.body_count
                ),
            );
        }
        if !self.radius_min.get().is_finite() || self.radius_min.get() <= 0.0 {
            return refuse(
                "generator.radius_min",
                format!("{} m", self.radius_min.get()),
            );
        }
        if !self.radius_max.get().is_finite() || self.radius_max < self.radius_min {
            return refuse(
                "generator.radius_max",
                format!(
                    "{} m, expected a finite value at least radius_min {} m",
                    self.radius_max.get(),
                    self.radius_min.get()
                ),
            );
        }
        if self.asteroid_kinds.is_empty() {
            return refuse("generator.asteroid_kinds", "an empty list".to_string());
        }
        if let Some(kind) = self
            .asteroid_kinds
            .iter()
            .find(|kind| !is_asteroid_kind(kind))
        {
            return Err(SectorFault::UnknownKind {
                kind: kind.to_string(),
            });
        }
        geometry.require_owning_edge(
            Meters(self.radius_max.get() * ASTEROID_GEOMETRIC_FACTOR_MAX),
            &format!(
                "an asteroid drawn at generator.radius_max {} m",
                self.radius_max.get()
            ),
        )
    }

    fn generate(&self, input: SectorGenerationInput) -> Result<SectorManifest, SectorFault> {
        let edge = input.geometry.sector_edge;
        let centre = input.coord.centre(edge);
        let reach = edge.get() * 0.5 * PLACEMENT_INSET;
        let mut stream = input.stream("bodies");
        let mut asteroids: Vec<SectorAsteroid> = Vec::with_capacity(self.body_count);
        for index in 0..self.body_count {
            let id = sector_id(input.coord, "body", index);
            let radius = self.radius_min + (self.radius_max - self.radius_min) * stream.unit();
            let clearance = Meters(radius.get() * ASTEROID_GEOMETRIC_FACTOR_MAX);
            let Some(position) = (0..PLACEMENT_ATTEMPTS)
                .map(|_| {
                    centre
                        + Meters3::new(
                            stream.signed() * reach,
                            stream.signed() * reach,
                            stream.signed() * reach,
                        )
                })
                .find(|candidate| {
                    asteroids.iter().all(|rock| {
                        let rock_clearance =
                            Meters(rock.radius.get() * ASTEROID_GEOMETRIC_FACTOR_MAX);
                        bodies_clear(rock.position, rock_clearance, *candidate, clearance)
                    })
                })
            else {
                return Err(SectorFault::Clearance {
                    id,
                    attempts: PLACEMENT_ATTEMPTS,
                });
            };
            let kind = self.asteroid_kinds[stream.next_u32() as usize % self.asteroid_kinds.len()];
            asteroids.push(SectorAsteroid {
                seed: asteroid_seed_from_id(&id),
                id,
                position,
                radius,
                kind: kind.to_string(),
            });
        }
        Ok(SectorManifest {
            coord: input.coord,
            features: Vec::new(),
            strengths: [0.0; FeatureLayer::COUNT],
            asteroids,
            planets: Vec::new(),
            ships: Vec::new(),
        })
    }
}
