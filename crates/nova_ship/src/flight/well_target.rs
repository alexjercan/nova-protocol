//! Which gravity well an orbit circles, and the one resolver the scripted
//! ORBIT order and the AI's passive orbit routine both read the live world
//! through.
//!
//! One resolver because the two callers must agree on what a target means:
//! an authored id that reached a streamed planetoid in one of them and not the
//! other would make the same RON line orbit different bodies depending on who
//! flies the ship.
//!
//! Engine units: well positions are avian `Position` and radii are world
//! units. Both callers pass the ship root's `Transform` translation: an
//! orderable ship root is top-level, so that is world space, and it is valid
//! on the spawn frame, before avian writes the root's first `Position`.

use avian3d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use super::{orbit_radius_band, BodyRadius, FlightSettings};

/// Which gravity well an orbit order or an AI orbit routine circles.
#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WellTargetType {
    /// The scenario id of one well the scenario spawned by name.
    ///
    /// Resolves only against wells carrying [`ScenarioAddressableMarker`], so
    /// a body a generator named - a streamed sector's planetoid - never
    /// answers to an id an author wrote. Two addressable wells with the id are
    /// refused rather than picked between.
    Authored(String),
    /// The loaded well whose surface is nearest the ship when the orbit
    /// engages, among wells with a stable band ORBIT can plan.
    ///
    /// Any loaded well qualifies, streamed or authored. Ties on surface
    /// distance fall to the lower [`EntityId`]; a tie on both is refused.
    ///
    /// Ranked again at EVERY engage, not once per target: an ORBIT order
    /// resumed after an interruption and an AI routine back from a fight each
    /// take the well nearest the ship at that moment, which can be another
    /// well than the one before. An engaged ring is never re-ranked.
    NearestToShip,
}

impl std::fmt::Display for WellTargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WellTargetType::Authored(id) => write!(f, "authored well '{id}'"),
            WellTargetType::NearestToShip => write!(f, "nearest orbitable well"),
        }
    }
}

/// Why a [`WellTargetType`] resolved to no well.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WellTargetFault {
    /// No loaded well answers the target. A normal state while a well has not
    /// spawned or streamed in yet.
    Missing(WellTargetType),
    /// More than one loaded well answers the target equally. An authoring
    /// fault: picking one would depend on spawn order.
    Ambiguous {
        /// The target that matched more than once.
        target: WellTargetType,
        /// How many wells matched it.
        count: usize,
    },
}

impl std::fmt::Display for WellTargetFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WellTargetFault::Missing(target) => {
                write!(f, "{target} matches no loaded gravity well")
            }
            WellTargetFault::Ambiguous { target, count } => {
                write!(f, "{target} matches {count} loaded gravity wells equally")
            }
        }
    }
}

/// Every loaded gravity well a [`WellTargetType`] can resolve against.
///
/// Ship roots are never wells, the same rule the ORBIT autopilot's own well
/// lookup states: it would read a ship target as "well gone" and disengage.
#[derive(SystemParam)]
pub(crate) struct LiveWells<'w, 's> {
    wells: Query<
        'w,
        's,
        (
            Entity,
            &'static EntityId,
            &'static Position,
            &'static GravityWell,
            Option<&'static BodyRadius>,
            Has<ScenarioAddressableMarker>,
        ),
        Without<SpaceshipRootMarker>,
    >,
    gravity: Res<'w, GravitySettings>,
    flight: Res<'w, FlightSettings>,
}

impl LiveWells<'_, '_> {
    /// The one well `target` names in the live world for a ship at
    /// `ship_position`.
    pub(crate) fn resolve(
        &self,
        target: &WellTargetType,
        ship_position: Vec3,
    ) -> Result<Entity, WellTargetFault> {
        match target {
            WellTargetType::Authored(wanted) => {
                let mut matches = self
                    .wells
                    .iter()
                    .filter(|(_, id, _, _, _, addressable)| *addressable && id.0 == *wanted)
                    .map(|(entity, ..)| entity);
                match (matches.next(), matches.count()) {
                    (None, _) => Err(WellTargetFault::Missing(target.clone())),
                    (Some(entity), 0) => Ok(entity),
                    (Some(_), rest) => Err(WellTargetFault::Ambiguous {
                        target: target.clone(),
                        count: rest + 1,
                    }),
                }
            }
            WellTargetType::NearestToShip => {
                let mut ranked: Vec<(f32, &str, Entity)> = self
                    .wells
                    .iter()
                    .filter_map(|(entity, id, position, well, body, _)| {
                        // The same geometric radius the ORBIT plan clears: a
                        // generated rock's mesh reaches past its nominal
                        // designation radius.
                        let radius = well.body_radius.max(body.map_or(0.0, |body| **body));
                        let banded = GravityWell {
                            body_radius: radius,
                            ..well.clone()
                        };
                        orbit_radius_band(&banded, &self.gravity, &self.flight)?;
                        // A non-finite distance is a well avian has not placed
                        // yet (its `Position` placeholder is `Vec3::MAX`) or a
                        // broken one. Neither ranks, and either would slip
                        // past the tie check below.
                        let surface = ship_position.distance(position.0) - radius;
                        surface
                            .is_finite()
                            .then_some((surface, id.0.as_str(), entity))
                    })
                    .collect();
                ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(b.1)));
                let Some(&(surface, id, entity)) = ranked.first() else {
                    return Err(WellTargetFault::Missing(target.clone()));
                };
                let tied = ranked
                    .iter()
                    .take_while(|(other_surface, other_id, _)| {
                        *other_surface == surface && *other_id == id
                    })
                    .count();
                if tied > 1 {
                    return Err(WellTargetFault::Ambiguous {
                        target: target.clone(),
                        count: tied,
                    });
                }
                Ok(entity)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn world() -> World {
        let mut world = World::new();
        world.init_resource::<GravitySettings>();
        world.init_resource::<FlightSettings>();
        world
    }

    /// An orbitable well with its centre at `x` and a 20-unit body.
    fn well(world: &mut World, id: &str, x: f32, addressable: bool) -> Entity {
        let mut entity = world.spawn((
            GravityWell::from_mass(2_400.0, 20.0, &GravitySettings::default()),
            EntityId::new(id),
            Position(Vec3::new(x, 0.0, 0.0)),
        ));
        if addressable {
            entity.insert(ScenarioAddressableMarker);
        }
        entity.id()
    }

    fn resolve(world: &mut World, target: WellTargetType) -> Result<Entity, WellTargetFault> {
        world
            .run_system_once(move |wells: LiveWells| wells.resolve(&target, Vec3::ZERO))
            .unwrap()
    }

    /// An authored id names exactly one addressable well. A generated well
    /// with the same id is not a candidate, and two addressable wells with it
    /// are refused rather than picked between by spawn order.
    #[test]
    fn an_authored_well_resolves_only_to_one_addressable_match() {
        let mut world = world();
        let generated = well(&mut world, "planetoid", 100.0, false);
        assert_eq!(
            resolve(&mut world, WellTargetType::Authored("planetoid".into())),
            Err(WellTargetFault::Missing(WellTargetType::Authored(
                "planetoid".into()
            ))),
            "a generated well never answers an authored id"
        );

        let authored = well(&mut world, "planetoid", 300.0, true);
        assert_eq!(
            resolve(&mut world, WellTargetType::Authored("planetoid".into())),
            Ok(authored)
        );
        assert_ne!(generated, authored);

        well(&mut world, "planetoid", 500.0, true);
        assert_eq!(
            resolve(&mut world, WellTargetType::Authored("planetoid".into())),
            Err(WellTargetFault::Ambiguous {
                target: WellTargetType::Authored("planetoid".into()),
                count: 2,
            })
        );
    }

    /// The nearest target ranks every loaded orbitable well by surface
    /// distance, addressable or not, breaks a distance tie by the lower id,
    /// skips a well with no stable band or no placed position, and refuses a
    /// tie on both.
    #[test]
    fn the_nearest_well_ranks_by_surface_then_id_and_refuses_a_full_tie() {
        let mut world = world();
        assert_eq!(
            resolve(&mut world, WellTargetType::NearestToShip),
            Err(WellTargetFault::Missing(WellTargetType::NearestToShip))
        );

        // Closest centre, but no band ORBIT can plan: never a candidate.
        world.spawn((
            GravityWell {
                mu: 100.0,
                body_radius: 10.0,
                soi_radius: 12.0,
            },
            EntityId::new("pebble"),
            Position(Vec3::new(30.0, 0.0, 0.0)),
        ));
        // Spawned, but avian has not written its first `Position` yet.
        world.spawn((
            GravityWell::from_mass(2_400.0, 20.0, &GravitySettings::default()),
            EntityId::new("unplaced"),
            Position::PLACEHOLDER,
        ));
        assert_eq!(
            resolve(&mut world, WellTargetType::NearestToShip),
            Err(WellTargetFault::Missing(WellTargetType::NearestToShip)),
            "neither a bandless well nor an unplaced one is a candidate"
        );
        let far = well(&mut world, "far", 400.0, true);
        // A generated rock whose mesh reaches far past its nominal radius: a
        // farther centre and a nearer surface.
        let big = world
            .spawn((
                GravityWell {
                    mu: 200_000.0,
                    body_radius: 20.0,
                    soi_radius: 2_000.0,
                },
                EntityId::new("big"),
                Position(Vec3::new(-300.0, 0.0, 0.0)),
                BodyRadius(250.0),
            ))
            .id();
        assert_eq!(
            resolve(&mut world, WellTargetType::NearestToShip),
            Ok(big),
            "surface distance, not centre distance, and the streamed well counts"
        );
        assert_ne!(far, big);

        let beta = well(&mut world, "beta", 40.0, false);
        let alpha = well(&mut world, "alpha", -40.0, false);
        assert_eq!(
            resolve(&mut world, WellTargetType::NearestToShip),
            Ok(alpha),
            "an equal surface distance falls to the lower id"
        );
        assert_ne!(alpha, beta);

        well(&mut world, "alpha", 40.0, false);
        assert_eq!(
            resolve(&mut world, WellTargetType::NearestToShip),
            Err(WellTargetFault::Ambiguous {
                target: WellTargetType::NearestToShip,
                count: 2,
            })
        );
    }
}
