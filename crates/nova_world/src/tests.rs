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
//! crate - the base game's in `nova_world_base`, the uniform baseline with the
//! examples - and are proved where they live.

use bevy::prelude::*;
use nova_events::prelude::{Meters, Meters3};
use nova_scenario::prelude::{PlanetConfig, PlanetType, ASTEROID_GEOMETRIC_FACTOR_MAX, KIND_ROCK};

use crate::{
    generate_sector, prepare_sector, sector_id, validate_manifest, NovaWorldPlugin, SectorAsteroid,
    SectorCoord, SectorFault, SectorGenerationInput, SectorGenerator, SectorManifest, SectorPlanet,
    SectorShip, WorldConfig, WorldGeometry, ACTIVE_WINDOW_SECTORS_MAX,
};

/// The fixture generator's placement inset: its rocks stand at a quarter
/// edge from the centre, well inside it.
const ROCKS_INSET: f32 = 0.7;

/// Stands four rocks at the corners of a square around each cell's
/// centre, and refuses an edge too narrow to own its widest rock. No draw and
/// no retry: placement is each generator's own, and this one needs neither.
#[derive(Clone, Debug, PartialEq)]
struct Rocks {
    radius_max: Meters,
}

impl SectorGenerator for Rocks {
    fn validate(&self, geometry: WorldGeometry) -> Result<(), SectorFault> {
        geometry.require_owning_edge(
            ROCKS_INSET,
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
                kind: KIND_ROCK.into(),
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
    assert_eq!(description.asteroids().len(), 4);
}

/// A generator outside the crate is checked, not trusted.
///
/// Each answer below breaks one rule `materialize_sector` relies on, and each
/// is refused by [`prepare_sector`] - the function a worker runs - so the
/// refusal lands before a mesh is built or an entity exists. A rock across a
/// face would be retired with the neighbour; two ids in one cell cannot be
/// found again; a ship with no design cannot be resolved. A planetoid is
/// refused by the same [`PlanetConfig::validate`] an authored planet meets in
/// the lint.
#[test]
fn a_malformed_generator_answer_is_refused_before_preparation() {
    fn rock(input: SectorGenerationInput, id: &str, offset: f32) -> SectorAsteroid {
        SectorAsteroid {
            id: format!("{}_{id}", input.coord.slug()),
            position: input.coord.centre(input.geometry.sector_edge)
                + Meters3::new(offset, 0.0, 0.0),
            radius: Meters(40.0),
            kind: KIND_ROCK.into(),
            seed: 7,
        }
    }
    let cases: [(
        &str,
        fn(SectorGenerationInput) -> SectorManifest,
        fn(&SectorFault) -> bool,
    ); 9] = [
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
            "two overlapping rocks",
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
                body.kind = "obsidian".into();
                empty(input.coord, vec![body])
            },
            |fault| matches!(fault, SectorFault::UnknownKind { kind } if kind.as_str() == "obsidian"),
        ),
        (
            "a planetoid whose well has a NaN mass",
            |input| {
                let mut manifest = empty(input.coord, Vec::new());
                manifest.planets.push(SectorPlanet {
                    id: format!("{}_planet_0", input.coord.slug()),
                    position: input.coord.centre(input.geometry.sector_edge),
                    config: PlanetConfig::new(PlanetType::BarrenRock, Meters(800.0), 3)
                        .anchored(f32::NAN),
                });
                manifest
            },
            |fault| matches!(fault, SectorFault::Manifest { field: "mass", .. }),
        ),
        (
            "a rock at a non-finite position",
            |input| {
                let mut body = rock(input, "body_0", 0.0);
                body.position = Meters3::new(f32::NAN, 0.0, 0.0);
                empty(input.coord, vec![body])
            },
            |fault| matches!(fault, SectorFault::InvalidGeometry { id } if id.ends_with("body_0")),
        ),
        (
            "a ship with a blank design",
            |input| {
                let mut manifest = empty(input.coord, Vec::new());
                manifest.ships.push(SectorShip {
                    id: format!("{}_ship_0", input.coord.slug()),
                    position: input.coord.centre(input.geometry.sector_edge),
                    yaw: 0.0,
                    design: " ".into(),
                });
                manifest
            },
            |fault| {
                matches!(
                    fault,
                    SectorFault::Manifest {
                        field: "design",
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

/// The core check refuses only bodies that overlap: spacing is each
/// generator's own policy. Two 40 m rocks claim 240 m of clearance each, so
/// 580 m apart leaves 100 m between their clearance spheres - less than the
/// 500 m margin `nova_world_base` keeps - and the manifest still describes.
#[test]
fn bodies_closer_than_a_generator_margin_but_not_overlapping_are_accepted() {
    let description = generate_sector(
        &answering(|input| {
            let centre = input.coord.centre(input.geometry.sector_edge);
            let rock = |index: usize, offset: f32| SectorAsteroid {
                id: sector_id(input.coord, "body", index),
                position: centre + Meters3::new(offset, 0.0, 0.0),
                radius: Meters(40.0),
                kind: KIND_ROCK.into(),
                seed: index as u32,
            };
            empty(input.coord, vec![rock(0, 0.0), rock(1, 580.0)])
        }),
        SectorCoord::ORIGIN,
    )
    .expect("two rocks whose clearance spheres do not touch must describe");
    assert_eq!(description.asteroids().len(), 2);
}

/// `rocks` rocks, one planetoid and ships to fill `count` bodies, each on its
/// own 3 km grid point inside the cell's inset.
fn populated(input: SectorGenerationInput, rocks: usize, count: usize) -> SectorManifest {
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
    let asteroids = (0..rocks)
        .map(|index| SectorAsteroid {
            id: sector_id(coord, "body", index),
            position: point(index),
            radius: Meters(40.0),
            kind: KIND_ROCK.into(),
            seed: index as u32,
        })
        .collect();
    let mut manifest = empty(coord, asteroids);
    manifest.planets.push(SectorPlanet {
        id: sector_id(coord, "planet", 0),
        position: point(rocks),
        config: PlanetConfig::new(PlanetType::BarrenRock, Meters(800.0), 3),
    });
    manifest.ships = (rocks + 1..count)
        .map(|index| SectorShip {
            id: sector_id(coord, "ship", index),
            position: point(index),
            yaw: 0.0,
            design: "block_wreck_plate".into(),
        })
        .collect();
    manifest
}

/// The world sets no count limit: density is the generator's policy. A cell
/// of eight rocks and twenty bodies, every one inside the cell and clear of
/// the others, is described whole.
#[test]
fn a_manifest_of_many_valid_bodies_is_described_whole() {
    let description = generate_sector(
        &answering(|input| populated(input, 8, 20)),
        SectorCoord::ORIGIN,
    )
    .expect("a dense manifest of valid bodies must describe");
    assert_eq!(description.asteroids().len(), 8);
    assert_eq!(
        description.asteroids().len() + description.planets().len() + description.ships().len(),
        20
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
/// the fixture's 0.7 inset leaves only the outer 15% of each half edge for it
/// to occupy. Under that floor the body crosses a face its cell does not own,
/// and retiring the neighbour takes geometry standing beside the observer -
/// the one thing a placement inset is for.
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

/// A generator's placement inset outside `[0, 1)` is refused, not trusted:
/// a `NaN` inset makes the edge floor `NaN`, and every edge would pass it.
#[test]
fn a_placement_inset_outside_zero_to_one_is_refused() {
    let geometry = WorldGeometry {
        sector_edge: Meters(32_000.0),
    };
    for inset in [f32::NAN, -0.1, 1.0, f32::INFINITY] {
        let fault = geometry
            .require_owning_edge(inset, Meters(360.0), "a test rock")
            .expect_err("an inset outside 0 to under 1 must refuse");
        assert!(
            matches!(
                &fault,
                SectorFault::Config {
                    field: "generator.placement_inset",
                    ..
                }
            ),
            "a {inset} inset must refuse on the inset, got {fault:?}"
        );
    }
    geometry
        .require_owning_edge(0.0, Meters(360.0), "a test rock")
        .expect("a zero inset keeps every centre at the cell centre");
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

/// `validate_manifest` is public, so it refuses a cell with no valid geometry
/// on its own and does not rely on [`WorldConfig::validate`] having run.
///
/// A NaN edge passes every containment test, so without this check an empty
/// manifest, or even rocks, pass as a trusted description.
#[test]
fn a_manifest_for_a_cell_with_no_valid_geometry_is_refused() {
    for edge in [0.0, f32::NAN, f32::INFINITY] {
        let fault = validate_manifest(
            SectorGenerationInput {
                seed: 20_260_922,
                geometry: WorldGeometry {
                    sector_edge: Meters(edge),
                },
                coord: SectorCoord::ORIGIN,
            },
            empty(SectorCoord::ORIGIN, Vec::new()),
        )
        .expect_err("a manifest for a cell with no valid edge must refuse");
        assert!(
            matches!(
                fault,
                SectorFault::Config {
                    field: "sector_edge",
                    ..
                }
            ),
            "a {edge} m edge must refuse on the edge, got {fault:?}"
        );
    }

    // A finite edge that puts the far cell's centre past f32.
    let far = SectorCoord::new(i32::MAX, 0, 0);
    let fault = validate_manifest(
        SectorGenerationInput {
            seed: 20_260_922,
            geometry: WorldGeometry {
                sector_edge: Meters(1.0e30),
            },
            coord: far,
        },
        empty(far, Vec::new()),
    )
    .expect_err("a manifest for a cell with no finite centre must refuse");
    assert_eq!(fault, SectorFault::InvalidGeometry { id: far.slug() });
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
