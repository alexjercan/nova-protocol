//! What the world refuses: a config nobody could generate from, a generator
//! answer nobody could materialize, and a second generator in one app.
//!
//! Pure validation, so a unit test is the cheapest thing that observes it. The
//! claim each one carries is that a bad input is a REFUSAL and never a silent
//! fallback: an oversized window is not clamped, a rock outside its cell is
//! not moved, and an unshipped kind id is not skipped. Each of those would put
//! a world nobody authored in front of a player.
//!
//! The generators here are test-local. The shipped policies live outside this
//! crate - the base game's in `nova_authoring`, the uniform baseline with the
//! examples - and are proved where they live.

use bevy::prelude::*;
use nova_events::prelude::{Meters, Meters3};
use nova_scenario::prelude::{PlanetConfig, PlanetType, ASTEROID_GEOMETRIC_FACTOR_MAX, KIND_ROCK};

use crate::{
    generate_sector, prepare_sector, sector_features, sector_id, validate_feature_geometry,
    FeatureLayer, FeatureSphere, NovaWorldPlugin, SectorAsteroid, SectorCoord, SectorFault,
    SectorGenerationInput, SectorGenerator, SectorManifest, SectorPlanet, SectorShip, WorldConfig,
    WorldGeometry, ACTIVE_WINDOW_SECTORS_MAX, SECTOR_ASTEROIDS_MAX, SECTOR_BODIES_MAX,
    SECTOR_FEATURES_MAX,
};

/// Stands the measured rock cap at the corners of a square around each cell's
/// centre, and refuses an edge too narrow to own its widest rock. No draw and
/// no retry: placement is each generator's own, and this one needs neither.
#[derive(Clone, Debug, PartialEq)]
struct Rocks {
    radius_max: Meters,
}

impl SectorGenerator for Rocks {
    fn validate(&self, geometry: WorldGeometry) -> Result<(), SectorFault> {
        geometry.require_owning_edge(
            Meters(self.radius_max.get() * ASTEROID_GEOMETRIC_FACTOR_MAX),
            "a test rock",
        )
    }

    fn generate(&self, input: SectorGenerationInput) -> Result<SectorManifest, SectorFault> {
        let edge = input.geometry.sector_edge;
        let quarter = edge.get() * 0.25;
        let corners = [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)];
        let asteroids = corners
            .into_iter()
            .enumerate()
            .map(|(index, (x, z))| SectorAsteroid {
                id: sector_id(input.coord, "body", index),
                position: input.coord.centre(edge) + Meters3::new(x * quarter, 0.0, z * quarter),
                radius: self.radius_max,
                kind: KIND_ROCK.to_string(),
                seed: index as u32,
            })
            .collect();
        Ok(empty(input.coord, asteroids))
    }
}

/// Answers whatever its function builds, and checks nothing: the shape of a
/// generator outside this crate with a bug in it.
#[derive(Clone, Debug)]
struct Answers(fn(SectorGenerationInput) -> SectorManifest);

impl SectorGenerator for Answers {
    fn validate(&self, _geometry: WorldGeometry) -> Result<(), SectorFault> {
        Ok(())
    }

    fn generate(&self, input: SectorGenerationInput) -> Result<SectorManifest, SectorFault> {
        Ok((self.0)(input))
    }
}

/// A manifest of `coord` holding only `asteroids`.
fn empty(coord: SectorCoord, asteroids: Vec<SectorAsteroid>) -> SectorManifest {
    SectorManifest {
        coord,
        features: Vec::new(),
        strengths: [0.0; FeatureLayer::COUNT],
        asteroids,
        planets: Vec::new(),
        ships: Vec::new(),
    }
}

/// A config that describes a sector, and the base every refusal below breaks
/// exactly one field of.
fn rocks() -> WorldConfig<Rocks> {
    WorldConfig {
        seed: 20_260_922,
        sector_edge: Meters(32_000.0),
        active_radius: 2,
        generator: Rocks {
            radius_max: Meters(60.0),
        },
    }
}

/// The same dials around a generator that answers with `answer`.
fn answering(answer: fn(SectorGenerationInput) -> SectorManifest) -> WorldConfig<Answers> {
    WorldConfig {
        seed: 20_260_922,
        sector_edge: Meters(32_000.0),
        active_radius: 2,
        generator: Answers(answer),
    }
}

/// Break one field of `config` and return what the generator said.
fn fault_of<G: SectorGenerator>(config: &WorldConfig<G>) -> SectorFault {
    generate_sector(config, SectorCoord::ORIGIN).expect_err("the config must be refused")
}

#[test]
fn a_valid_config_describes_a_sector() {
    let description =
        generate_sector(&rocks(), SectorCoord::ORIGIN).expect("a valid config must describe");
    assert_eq!(description.asteroids().len(), SECTOR_ASTEROIDS_MAX);
}

/// A generator outside the crate is checked, not trusted.
///
/// Each answer below breaks one rule `materialize_sector` relies on, and each
/// is refused by [`prepare_sector`] - the function a worker runs - so the
/// refusal lands before a mesh is built or an entity exists. A rock across a
/// face would be retired with the neighbour; two ids in one cell cannot be
/// found again; a ship naming a sphere another cell owns would be spawned by
/// both. A planetoid is refused by the same [`PlanetConfig::validate`] an
/// authored planet meets in the lint.
#[test]
fn a_malformed_generator_answer_is_refused_before_preparation() {
    fn rock(input: SectorGenerationInput, id: &str, offset: f32) -> SectorAsteroid {
        SectorAsteroid {
            id: format!("{}_{id}", input.coord.slug()),
            position: input.coord.centre(input.geometry.sector_edge)
                + Meters3::new(offset, 0.0, 0.0),
            radius: Meters(40.0),
            kind: KIND_ROCK.to_string(),
            seed: 7,
        }
    }
    fn sphere(
        input: SectorGenerationInput,
        layer: FeatureLayer,
        owner: SectorCoord,
    ) -> FeatureSphere {
        let edge = input.geometry.sector_edge;
        FeatureSphere {
            id: format!("feature_{layer}_{}", owner.slug()),
            layer,
            owner,
            centre: owner.centre(edge),
            radius: Meters(edge.get() * 2.0),
            strength: 0.5,
        }
    }
    let cases: [(
        &str,
        fn(SectorGenerationInput) -> SectorManifest,
        fn(&SectorFault) -> bool,
    ); 10] = [
        (
            "the wrong cell",
            |input| empty(input.coord.offset(1, 0, 0), Vec::new()),
            |fault| matches!(fault, SectorFault::Manifest { field: "coord", .. }),
        ),
        (
            "an id another cell owns",
            |input| {
                let mut body = rock(input, "body_0", 0.0);
                body.id = "sector_1_0_0_body_0".to_string();
                empty(input.coord, vec![body])
            },
            |fault| matches!(fault, SectorFault::Manifest { field: "id", .. }),
        ),
        (
            "one id twice",
            |input| {
                empty(
                    input.coord,
                    vec![rock(input, "body_0", 0.0), rock(input, "body_0", 9_000.0)],
                )
            },
            |fault| matches!(fault, SectorFault::DuplicateId { .. }),
        ),
        (
            "a rock across a face",
            |input| empty(input.coord, vec![rock(input, "body_0", 15_900.0)]),
            |fault| {
                matches!(
                    fault,
                    SectorFault::Manifest {
                        field: "position",
                        ..
                    }
                )
            },
        ),
        (
            "two rocks inside each other's margin",
            |input| {
                empty(
                    input.coord,
                    vec![rock(input, "body_0", 0.0), rock(input, "body_1", 100.0)],
                )
            },
            |fault| {
                matches!(
                    fault,
                    SectorFault::Manifest {
                        field: "position",
                        ..
                    }
                )
            },
        ),
        (
            "an asteroid kind the game does not ship",
            |input| {
                let mut body = rock(input, "body_0", 0.0);
                body.kind = "obsidian".to_string();
                empty(input.coord, vec![body])
            },
            |fault| matches!(fault, SectorFault::UnknownKind { kind } if kind == "obsidian"),
        ),
        (
            "more rocks than a cell holds",
            |input| {
                let bodies = (0..=SECTOR_ASTEROIDS_MAX)
                    .map(|index| {
                        rock(
                            input,
                            &format!("body_{index}"),
                            index as f32 * 2_000.0 - 5_000.0,
                        )
                    })
                    .collect();
                empty(input.coord, bodies)
            },
            |fault| {
                matches!(
                    fault,
                    SectorFault::Manifest {
                        field: "asteroids",
                        ..
                    }
                )
            },
        ),
        (
            "a planetoid placed by a sphere another cell owns",
            |input| {
                let owner = input.coord.offset(1, 0, 0);
                let mut manifest = empty(input.coord, Vec::new());
                let feature = sphere(input, FeatureLayer::Planet, owner);
                manifest.planets.push(SectorPlanet {
                    id: format!("{}_planet_0", input.coord.slug()),
                    feature: feature.id.clone(),
                    position: input.coord.centre(input.geometry.sector_edge),
                    config: PlanetConfig::new(PlanetType::BarrenRock, Meters(800.0), 3),
                });
                manifest.features.push(feature);
                manifest
            },
            |fault| {
                matches!(
                    fault,
                    SectorFault::Manifest {
                        field: "feature",
                        ..
                    }
                )
            },
        ),
        (
            "a planetoid whose well has a NaN mass",
            |input| {
                let mut manifest = empty(input.coord, Vec::new());
                let feature = sphere(input, FeatureLayer::Planet, input.coord);
                manifest.planets.push(SectorPlanet {
                    id: format!("{}_planet_0", input.coord.slug()),
                    feature: feature.id.clone(),
                    position: input.coord.centre(input.geometry.sector_edge),
                    config: PlanetConfig::new(PlanetType::BarrenRock, Meters(800.0), 3)
                        .anchored(f32::NAN),
                });
                manifest.features.push(feature);
                manifest
            },
            |fault| matches!(fault, SectorFault::Manifest { field: "mass", .. }),
        ),
        (
            "a ship placed by a sphere the manifest does not list",
            |input| {
                let mut manifest = empty(input.coord, Vec::new());
                manifest.ships.push(SectorShip {
                    id: format!("{}_ship_0", input.coord.slug()),
                    feature: "feature_derelict_nowhere".to_string(),
                    position: input.coord.centre(input.geometry.sector_edge),
                    yaw: 0.0,
                    design: "block_wreck_plate".to_string(),
                });
                manifest
            },
            |fault| {
                matches!(
                    fault,
                    SectorFault::Manifest {
                        field: "feature",
                        ..
                    }
                )
            },
        ),
    ];
    for (what, answer, expected) in cases {
        let fault = prepare_sector(answering(answer), SectorCoord::ORIGIN)
            .expect_err("a malformed answer must not be prepared");
        assert!(
            expected(&fault),
            "{what} must be refused before preparation, got {fault:?}"
        );
    }
}

/// `count` distinct spheres at the cell's centre, each reaching the whole cell.
fn listing(input: SectorGenerationInput, count: usize) -> SectorManifest {
    let edge = input.geometry.sector_edge;
    let mut manifest = empty(input.coord, Vec::new());
    manifest.features = (0..count)
        .map(|index| FeatureSphere {
            id: format!("feature_{index}"),
            layer: FeatureLayer::Asteroid,
            owner: input.coord,
            centre: input.coord.centre(edge),
            radius: edge,
            strength: 0.5,
        })
        .collect();
    manifest
}

/// One sphere past the cap is refused as too many, and the cap itself is a
/// cell the world accepts.
#[test]
fn a_manifest_listing_more_spheres_than_a_cell_holds_is_refused() {
    generate_sector(
        &answering(|input| listing(input, SECTOR_FEATURES_MAX)),
        SectorCoord::ORIGIN,
    )
    .expect("a manifest listing the sphere cap must describe");
    let fault = prepare_sector(
        answering(|input| listing(input, SECTOR_FEATURES_MAX + 1)),
        SectorCoord::ORIGIN,
    )
    .expect_err("a manifest listing one sphere past the cap must not be prepared");
    assert!(
        matches!(
            &fault,
            SectorFault::Manifest {
                field: "features",
                ..
            }
        ),
        "one sphere past the cap must be refused as too many, got {fault:?}"
    );
}

/// The rock cap's rocks, one planetoid and ships to fill `count` bodies, each
/// on its own 3 km grid point inside the cell's inset.
fn populated(input: SectorGenerationInput, count: usize) -> SectorManifest {
    let coord = input.coord;
    let centre = coord.centre(input.geometry.sector_edge);
    let point = |index: usize| {
        centre
            + Meters3::new(
                (index % 5) as f32 * 3_000.0 - 6_000.0,
                0.0,
                (index / 5) as f32 * 3_000.0 - 6_000.0,
            )
    };
    let planet_sphere = FeatureSphere {
        id: "feature_planet".to_string(),
        layer: FeatureLayer::Planet,
        owner: coord,
        centre,
        radius: input.geometry.sector_edge,
        strength: 0.5,
    };
    let derelict_sphere = FeatureSphere {
        id: "feature_derelict".to_string(),
        layer: FeatureLayer::Derelict,
        ..planet_sphere.clone()
    };
    let asteroids = (0..SECTOR_ASTEROIDS_MAX)
        .map(|index| SectorAsteroid {
            id: sector_id(coord, "body", index),
            position: point(index),
            radius: Meters(40.0),
            kind: KIND_ROCK.to_string(),
            seed: index as u32,
        })
        .collect();
    let mut manifest = empty(coord, asteroids);
    manifest.planets.push(SectorPlanet {
        id: sector_id(coord, "planet", 0),
        feature: planet_sphere.id.clone(),
        position: point(SECTOR_ASTEROIDS_MAX),
        config: PlanetConfig::new(PlanetType::BarrenRock, Meters(800.0), 3),
    });
    manifest.ships = (SECTOR_ASTEROIDS_MAX + 1..count)
        .map(|index| SectorShip {
            id: sector_id(coord, "ship", index),
            feature: derelict_sphere.id.clone(),
            position: point(index),
            yaw: 0.0,
            design: "block_wreck_plate".to_string(),
        })
        .collect();
    manifest.features = vec![planet_sphere, derelict_sphere];
    manifest
}

/// The body cap counts rocks, planetoids and ships together: one past it is
/// refused although every kind is under the cap alone, and the cap itself is a
/// cell the world accepts.
#[test]
fn a_manifest_placing_more_bodies_than_a_cell_holds_is_refused() {
    generate_sector(
        &answering(|input| populated(input, SECTOR_BODIES_MAX)),
        SectorCoord::ORIGIN,
    )
    .expect("a manifest placing the body cap must describe");
    let fault = prepare_sector(
        answering(|input| populated(input, SECTOR_BODIES_MAX + 1)),
        SectorCoord::ORIGIN,
    )
    .expect_err("a manifest placing one body past the cap must not be prepared");
    assert!(
        matches!(
            &fault,
            SectorFault::Manifest {
                field: "bodies",
                ..
            }
        ),
        "one body past the cap must be refused as too many, got {fault:?}"
    );
}

/// Exactly one generator per app. Two would stream two worlds over the same
/// roots, jobs and prepared payloads, each retiring the other's cells.
#[test]
#[should_panic(expected = "an app holds exactly one generator")]
fn a_second_generator_plugin_is_refused_while_the_app_is_built() {
    App::new()
        .add_plugins(NovaWorldPlugin::<Rocks>::default())
        .add_plugins(NovaWorldPlugin::<Answers>::default());
}

#[test]
#[should_panic(expected = "runs off the i32 sector grid")]
fn a_window_that_runs_off_the_grid_is_refused() {
    // `SectorCoord::containing` converts with an `as` cast, which saturates,
    // so a far enough observer really does stand in cell `i32::MAX` - and the
    // offsets around it wrap to the far side of the world in a release build.
    let _ = crate::streaming::desired_sectors(SectorCoord::new(i32::MAX, 0, 0), 2);
}

#[test]
fn an_unusable_sector_edge_is_refused() {
    for edge in [Meters(0.0), Meters(-32_000.0), Meters(f32::NAN)] {
        let config = WorldConfig {
            sector_edge: edge,
            ..rocks()
        };
        assert!(
            matches!(
                fault_of(&config),
                SectorFault::Config {
                    field: "sector_edge",
                    ..
                }
            ),
            "a {edge:?} edge must refuse before a cell is described"
        );
    }
}

#[test]
fn a_negative_active_radius_is_refused() {
    let config = WorldConfig {
        active_radius: -1,
        ..rocks()
    };
    assert_eq!(
        fault_of(&config),
        SectorFault::Config {
            field: "active_radius",
            value: "-1".to_string(),
        }
    );
}

#[test]
fn a_window_above_the_cell_maximum_is_refused() {
    // Radius 2 is 125 cells, the measured window, and radius 3 is 343. The
    // huge one is the shape of the finding: the cube overflows the count
    // itself, so the refusal has to come from checked arithmetic rather than
    // from an allocation nobody survives.
    for radius in [3, 1_000, i32::MAX] {
        let config = WorldConfig {
            active_radius: radius,
            ..rocks()
        };
        assert!(
            matches!(
                fault_of(&config),
                SectorFault::Config {
                    field: "active_radius",
                    ..
                }
            ),
            "a radius of {radius} asks for more than {ACTIVE_WINDOW_SECTORS_MAX} cells and must \
             refuse before the desired set is built, not be clamped to one that fits"
        );
    }
    generate_sector(
        &WorldConfig {
            active_radius: 2,
            ..rocks()
        },
        SectorCoord::ORIGIN,
    )
    .expect("the measured 5x5x5 window must still describe");
}

#[test]
#[should_panic(expected = "WorldConfig::active_radius")]
fn the_desired_window_refuses_a_radius_it_was_never_validated_for() {
    // The bound has to live at the ALLOCATION and not only in the config: a
    // caller can swap the resource between the stage that validates a change
    // and the stage that reads it, and this function is where the memory goes.
    let _ = crate::streaming::desired_sectors(SectorCoord::ORIGIN, 1_000);
}

/// A cell too narrow to hold its own rocks is refused, not quietly shared.
///
/// 2.4 km for the fixture's 60 m band: a rock meshes out to
/// `ASTEROID_GEOMETRIC_FACTOR_MAX` times its nominal radius, and
/// `PLACEMENT_INSET` leaves only the outer 15% of each half edge for it to
/// occupy. Under that floor the body crosses a face its cell does not own, and
/// retiring the neighbour takes geometry standing beside the observer - the
/// one thing `PLACEMENT_INSET` is documented to prevent.
#[test]
fn an_edge_too_narrow_to_own_its_rocks_is_refused() {
    let fault = fault_of(&WorldConfig {
        sector_edge: Meters(2_399.0),
        ..rocks()
    });
    assert!(
        matches!(
            &fault,
            SectorFault::Config { field: "sector_edge", value }
                if value.contains("a test rock")
        ),
        "a cell narrower than its own rocks must refuse and name the body, got {fault:?}"
    );
    let wide_enough = WorldConfig {
        sector_edge: Meters(2_401.0),
        ..rocks()
    };
    assert!(
        wide_enough.validate().is_ok(),
        "a cell just wide enough to own a 60 m rock must arm"
    );
}

/// A cell edge so wide that the ORIGIN window has no representable face must
/// refuse at `validate`, not on a worker. `collect_sector_jobs` panics on a
/// fault, so an edge that passes validation and then overflows takes the
/// session down after the world armed - the one ordering a fail-loud world
/// must not have.
#[test]
fn an_edge_too_wide_for_its_own_window_is_refused() {
    let fault = WorldConfig {
        sector_edge: Meters(f32::MAX),
        ..rocks()
    }
    .validate()
    .expect_err("a window two cells wide cannot reach the far face of an f32::MAX cell");
    assert!(
        matches!(
            &fault,
            SectorFault::Config {
                field: "sector_edge",
                ..
            }
        ),
        "an unreachable window must refuse on sector_edge, got {fault:?}"
    );

    // The delivery guard: a quarter of that edge still reaches, so the
    // refusal is the overflow and not a new ceiling on how wide a cell may be.
    WorldConfig {
        sector_edge: Meters(f32::MAX / 4.0),
        ..rocks()
    }
    .validate()
    .expect("a window that still has a finite far face arms");
}

/// A cell whose node range runs off the lattice is refused, not clipped.
///
/// A clipped range would sweep the whole lattice from one far cell and answer
/// for ground that cell never reaches. 400 km is inside the thinning halo's
/// span, so the geometry itself is valid; it is the CELL, out at the end of
/// the i32 grid, that has no node range anyone can address.
#[test]
fn a_feature_query_that_runs_off_the_node_lattice_is_refused() {
    let geometry = WorldGeometry {
        sector_edge: Meters(400_000.0),
    };
    validate_feature_geometry(geometry).expect("a 400 km cell is inside the thinning halo's reach");
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

#[test]
fn a_feature_edge_wider_than_the_thinning_halo_is_refused() {
    // In EVERY build, not a debug assertion: a release run that accepted this
    // edge would inspect the same 2-node halo and ship a world where two
    // same-layer spheres outside it both survive the thinning.
    let geometry = WorldGeometry {
        sector_edge: Meters(500_000.0),
    };
    let fault = sector_features(SectorGenerationInput {
        seed: 20_260_922,
        geometry,
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
    generate_sector(
        &WorldConfig {
            sector_edge: Meters(500_000.0),
            ..rocks()
        },
        SectorCoord::ORIGIN,
    )
    .expect("a generator that reads no field has no halo, so the same edge is fine");
}

/// The refusal the ARMING frame owes the main thread.
///
/// [`crate::WorldConfig::validate`] also runs inside [`generate_sector`], but
/// that is on a WORKER and one job too late for a dial the main thread pays
/// for first: `desired_sectors` enumerates the whole window three times a
/// frame, starting in the stage right after the observer is read. So the
/// window bound is only a bound if the config is refused HERE.
mod arming {
    use bevy::{ecs::system::RunSystemOnce, prelude::*};
    use nova_events::prelude::Meters;

    use super::{rocks, Rocks};
    use crate::{prelude::WorldObserver, WorldConfig};

    /// A world with `config` armed and exactly one observer standing in it,
    /// taken through the arming frame's [`crate::NovaWorldSystems::Cleanup`].
    ///
    /// Cleanup is run for real rather than faked, because it is what records
    /// the config version the stages below it check themselves against. A
    /// helper that skipped it would arm every test into the very refusal
    /// `a_config_written_after_the_world_was_cleared_is_refused` pins.
    fn armed(config: WorldConfig<Rocks>) -> World {
        let mut world = World::new();
        world.insert_resource(config);
        world.init_resource::<crate::streaming::ReadySectors>();
        world.init_resource::<crate::SectorJobStats>();
        world.init_resource::<crate::streaming::ClearedConfig>();
        world.spawn((WorldObserver, GlobalTransform::default()));
        world
            .run_system_once(crate::streaming::clear_sector_work::<Rocks>)
            .expect("the arming frame must clear the world it replaces");
        world
    }

    #[test]
    #[should_panic(expected = "WorldConfig::active_radius")]
    fn an_oversized_window_is_refused_before_it_is_enumerated() {
        let mut world = armed(WorldConfig {
            active_radius: 1_000,
            ..rocks()
        });
        let _ = world.run_system_once(crate::streaming::track_current_sector::<Rocks>);
    }

    #[test]
    #[should_panic(expected = "WorldConfig::sector_edge")]
    fn a_generator_refusal_is_raised_when_the_world_is_armed() {
        let mut world = armed(WorldConfig {
            sector_edge: Meters(2_000.0),
            ..rocks()
        });
        let _ = world.run_system_once(crate::streaming::track_current_sector::<Rocks>);
    }

    #[test]
    fn the_measured_window_arms() {
        let mut world = armed(rocks());
        world
            .run_system_once(crate::streaming::track_current_sector::<Rocks>)
            .expect("the measured 5x5x5 window must arm");
    }

    /// A `WorldConfig` written behind `Cleanup`'s back is refused, not mixed.
    ///
    /// The one frame this pins is the one nothing downstream could catch: a
    /// root, a running job and a prepared payload are all keyed by COORDINATE,
    /// so the old world's work would be collected and spawned under the new
    /// config and look exactly like the new world's own cell. The next frame
    /// does clear up, which is what would make it a silent flicker of two
    /// worlds rather than a crash.
    #[test]
    #[should_panic(expected = "changed after NovaWorldSystems::Cleanup ran")]
    fn a_config_written_after_the_world_was_cleared_is_refused() {
        let mut world = armed(rocks());
        world.resource_mut::<WorldConfig<Rocks>>().seed += 1;
        let _ = world.run_system_once(crate::streaming::track_current_sector::<Rocks>);
    }
}

/// The observer rule, which is the one thing a caller MUST wire.
///
/// The positive path is observed by every example that streams: the window
/// follows the camera. What no run observes is the refusal, and that is the
/// half worth pinning - the loop reads its centre from the observer, so an
/// unmarked session would otherwise stream around the origin and look like a
/// world that simply starts somewhere else.
mod observer {
    use bevy::{ecs::system::RunSystemOnce, prelude::*};

    use super::{rocks, Rocks};
    use crate::prelude::{CurrentSector, SectorCoord, WorldObserver};

    /// A world with the fixture config in it, taken through the arming
    /// frame's [`crate::NovaWorldSystems::Cleanup`] and holding nothing else.
    fn world() -> World {
        let mut world = World::new();
        world.insert_resource(rocks());
        world.init_resource::<crate::streaming::ReadySectors>();
        world.init_resource::<crate::SectorJobStats>();
        world.init_resource::<crate::streaming::ClearedConfig>();
        world
            .run_system_once(crate::streaming::clear_sector_work::<Rocks>)
            .expect("the arming frame must clear the world it replaces");
        world
    }

    #[test]
    #[should_panic(expected = "there is not exactly one WorldObserver")]
    fn a_session_with_no_observer_is_refused() {
        let _ = world().run_system_once(crate::streaming::track_current_sector::<Rocks>);
    }

    #[test]
    #[should_panic(expected = "there is not exactly one WorldObserver")]
    fn a_session_with_two_observers_is_refused() {
        let mut world = world();
        world.spawn((WorldObserver, GlobalTransform::default()));
        world.spawn((WorldObserver, GlobalTransform::default()));
        let _ = world.run_system_once(crate::streaming::track_current_sector::<Rocks>);
    }

    #[test]
    fn the_observer_names_the_cell_it_stands_in() {
        let mut world = world();
        // Engine boundary: a transform counts world units, and one unit is
        // 10 m, so this is 40 km along +X - the next cell at a 32 km edge.
        world.spawn((
            WorldObserver,
            GlobalTransform::from(Transform::from_xyz(4_000.0, 0.0, 0.0)),
        ));
        world
            .run_system_once(crate::streaming::track_current_sector::<Rocks>)
            .expect("the observer system must run");
        assert_eq!(
            world.resource::<CurrentSector>().0,
            SectorCoord::new(1, 0, 0),
            "the streamed centre must be the cell the observer stands in"
        );
    }
}
