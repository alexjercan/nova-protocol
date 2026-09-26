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

/// How much of a sector's half-edge a rock's centre is drawn across. The rest
/// is the room that keeps the whole rock inside its own cell.
const PLACEMENT_INSET: f32 = 0.7;

/// Extra room every pair of rocks keeps between their clearance spheres, so a
/// cell reads as placed rather than piled.
const CLEARANCE_MARGIN: Meters = Meters(500.0);

/// Every cell gets the same treatment out of its own seed: `body_count` rocks
/// scattered across its inset, nominal radius drawn from the band.
///
/// Nothing about the WORLD can explain away a sector that failed to come up,
/// which is what makes it the right generator for judging a retirement or a
/// crossing.
///
/// Its placement is its own and deliberately plain: each rock draws candidates
/// across [`PLACEMENT_INSET`] of the cell until one keeps [`CLEARANCE_MARGIN`]
/// from every rock before it.
///
/// No `Default`: a body count and a radius band nobody chose are how a number
/// nobody chose reaches a frame.
#[derive(Clone, Debug, PartialEq)]
pub struct UniformAsteroids {
    /// Rocks generated per sector, at least one. A cell with no room for the
    /// next rock refuses with [`SectorFault::Clearance`].
    pub body_count: usize,
    /// Smallest nominal body radius drawn.
    pub radius_min: Meters,
    /// Largest nominal body radius drawn.
    pub radius_max: Meters,
    /// The asteroid kind ids a body may be drawn from.
    pub asteroid_kinds: Vec<AsteroidKindId>,
}

impl SectorGenerator for UniformAsteroids {
    /// Refuse a count, a band or a kind table that cannot describe a sector,
    /// and an edge too narrow to own the widest rock. An empty table is a
    /// refusal and never a silent fallback to a house rock.
    fn validate(&self, geometry: WorldGeometry) -> Result<(), SectorFault> {
        let refuse = |field: &'static str, value: String| Err(SectorFault::Config { field, value });
        if self.body_count == 0 {
            return refuse(
                "generator.body_count",
                "0, expected at least one rock".to_string(),
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
            return Err(SectorFault::UnknownKind { kind: kind.clone() });
        }
        geometry.require_owning_edge(
            PLACEMENT_INSET,
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
                        bodies_clear(
                            rock.position,
                            rock_clearance,
                            *candidate,
                            clearance,
                            CLEARANCE_MARGIN,
                        )
                    })
                })
            else {
                return Err(SectorFault::Clearance {
                    id,
                    attempts: PLACEMENT_ATTEMPTS,
                });
            };
            let kind =
                self.asteroid_kinds[stream.next_u32() as usize % self.asteroid_kinds.len()].clone();
            asteroids.push(SectorAsteroid {
                seed: asteroid_seed_from_id(&id),
                id,
                position,
                radius,
                kind,
                mass: super::rock_mass(radius),
            });
        }
        Ok(SectorManifest {
            coord: input.coord,
            asteroids,
            planets: Vec::new(),
            ships: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> UniformAsteroids {
        UniformAsteroids {
            body_count: 4,
            radius_min: Meters(30.0),
            radius_max: Meters(60.0),
            asteroid_kinds: [KIND_ROCK, KIND_METAL, KIND_ICE, KIND_CARBON]
                .map(Into::into)
                .to_vec(),
        }
    }

    fn validate(generator: UniformAsteroids) -> Result<(), SectorFault> {
        generator.validate(WorldGeometry {
            sector_edge: Meters(32_000.0),
        })
    }

    fn refused_field(generator: UniformAsteroids) -> &'static str {
        match validate(generator.clone()) {
            Err(SectorFault::Config { field, .. }) => field,
            other => panic!("{generator:?} must refuse as a config fault, got {other:?}"),
        }
    }

    /// A cell with no rocks judges nothing, and no count above zero is refused
    /// at the config: room is the placement's refusal, not a count limit.
    #[test]
    fn a_zero_body_count_is_refused_and_any_other_count_arms() {
        assert_eq!(
            refused_field(UniformAsteroids {
                body_count: 0,
                ..baseline()
            }),
            "generator.body_count",
        );
        for body_count in [1, 4, 5, 64] {
            validate(UniformAsteroids {
                body_count,
                ..baseline()
            })
            .expect("a body count of one or more must arm");
        }
    }

    #[test]
    fn a_radius_band_that_is_not_finite_positive_and_ordered_is_refused() {
        for radius_min in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(
                refused_field(UniformAsteroids {
                    radius_min: Meters(radius_min),
                    ..baseline()
                }),
                "generator.radius_min",
            );
        }
        for radius_max in [29.0, f32::NAN, f32::INFINITY] {
            assert_eq!(
                refused_field(UniformAsteroids {
                    radius_max: Meters(radius_max),
                    ..baseline()
                }),
                "generator.radius_max",
            );
        }
        validate(UniformAsteroids {
            radius_max: Meters(30.0),
            ..baseline()
        })
        .expect("a band of one radius must arm");
    }

    /// An empty table is a refusal, never a silent fallback to a house rock.
    #[test]
    fn an_empty_or_unshipped_asteroid_kind_list_is_refused() {
        assert_eq!(
            refused_field(UniformAsteroids {
                asteroid_kinds: Vec::new(),
                ..baseline()
            }),
            "generator.asteroid_kinds",
        );
        let fault = validate(UniformAsteroids {
            asteroid_kinds: vec![KIND_ROCK.into(), "not_a_kind".into()],
            ..baseline()
        })
        .expect_err("an unshipped kind must refuse");
        assert!(
            matches!(&fault, SectorFault::UnknownKind { kind } if kind.as_str() == "not_a_kind"),
            "an unshipped kind must be named, got {fault:?}"
        );
    }
}
