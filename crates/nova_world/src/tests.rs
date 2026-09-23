//! What [`WorldConfig::validate`] refuses.
//!
//! Pure validation, so a unit test is the cheapest thing that observes it. The
//! claim each one carries is that a config nobody could generate from is a
//! REFUSAL and never a silent fallback: an empty kind list is not "rock", an
//! unshipped kind id is not skipped, and a zero body count is not one body.
//! Each of those would put a world nobody authored in front of a player, and
//! the streaming loop's own faults are unreachable from a config that got
//! this far.

use nova_events::prelude::Meters;
use nova_scenario::prelude::{PlanetType, KIND_ICE, KIND_ROCK};

use crate::{
    generate_sector, LayeredFeatureConfig, SectorCoord, SectorFault, SectorGeneration,
    UniformAsteroidConfig, WorldConfig, ACTIVE_WINDOW_SECTORS_MAX, SECTOR_ASTEROIDS_MAX,
};

/// A config that describes a sector, and the base every refusal below breaks
/// exactly one field of.
fn uniform() -> WorldConfig {
    WorldConfig {
        seed: 20_260_922,
        sector_edge: Meters(32_000.0),
        active_radius: 2,
        generation: SectorGeneration::UniformAsteroids(UniformAsteroidConfig {
            body_count: 4,
            radius_min: Meters(30.0),
            radius_max: Meters(60.0),
            asteroid_kinds: vec![KIND_ROCK.to_string(), KIND_ICE.to_string()],
        }),
    }
}

/// The layered counterpart, with every content list filled.
fn layered() -> WorldConfig {
    WorldConfig {
        generation: SectorGeneration::LayeredFeatures(LayeredFeatureConfig {
            asteroid_kinds: vec![KIND_ROCK.to_string()],
            planet_types: vec![PlanetType::BarrenRock],
            anchorage_design: "block_hauler".to_string(),
        }),
        ..uniform()
    }
}

/// Break one field of `config` and return what the generator said.
fn fault_of(config: &WorldConfig) -> SectorFault {
    generate_sector(config, SectorCoord::ORIGIN)
        .expect_err("the config must be refused")
        .clone()
}

#[test]
fn a_valid_config_describes_a_sector() {
    let description =
        generate_sector(&uniform(), SectorCoord::ORIGIN).expect("a valid config must describe");
    assert_eq!(description.asteroids.len(), 4);
    generate_sector(&layered(), SectorCoord::ORIGIN).expect("a valid layered config must describe");
}

#[test]
fn an_empty_asteroid_kind_list_is_refused() {
    for mut config in [uniform(), layered()] {
        match &mut config.generation {
            SectorGeneration::UniformAsteroids(uniform) => uniform.asteroid_kinds.clear(),
            SectorGeneration::LayeredFeatures(layered) => layered.asteroid_kinds.clear(),
        }
        assert_eq!(
            fault_of(&config),
            SectorFault::Config {
                field: "generation.asteroid_kinds",
                value: "an empty list".to_string(),
            },
            "an empty kind list must refuse rather than fall back to a house rock"
        );
    }
}

#[test]
fn an_unshipped_asteroid_kind_is_refused() {
    let mut config = uniform();
    let SectorGeneration::UniformAsteroids(uniform) = &mut config.generation else {
        unreachable!("the fixture is the uniform generator");
    };
    uniform.asteroid_kinds = vec![KIND_ROCK.to_string(), "obsidian".to_string()];
    assert_eq!(
        fault_of(&config),
        SectorFault::UnknownKind {
            kind: "obsidian".to_string(),
        },
        "a kind the game does not ship must refuse the whole config, not be skipped"
    );
}

#[test]
fn a_uniform_body_count_outside_the_measured_density_is_refused() {
    // Both ends. Zero is a world with nothing in it, and anything above the
    // cap reaches `Vec::with_capacity` on a worker straight off a public
    // field - `usize::MAX` is the shape of it, `SECTOR_ASTEROIDS_MAX + 1` is
    // the boundary.
    for count in [0, SECTOR_ASTEROIDS_MAX + 1, usize::MAX] {
        let mut config = uniform();
        let SectorGeneration::UniformAsteroids(uniform) = &mut config.generation else {
            unreachable!("the fixture is the uniform generator");
        };
        uniform.body_count = count;
        assert!(
            matches!(
                fault_of(&config),
                SectorFault::Config {
                    field: "generation.body_count",
                    ..
                }
            ),
            "a body count of {count} must refuse before the generator reserves for it"
        );
    }
    let mut config = uniform();
    let SectorGeneration::UniformAsteroids(uniform) = &mut config.generation else {
        unreachable!("the fixture is the uniform generator");
    };
    uniform.body_count = SECTOR_ASTEROIDS_MAX;
    let description =
        generate_sector(&config, SectorCoord::ORIGIN).expect("the measured density must describe");
    assert_eq!(description.asteroids.len(), SECTOR_ASTEROIDS_MAX);
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
fn an_inverted_radius_band_is_refused() {
    let mut config = uniform();
    let SectorGeneration::UniformAsteroids(uniform) = &mut config.generation else {
        unreachable!("the fixture is the uniform generator");
    };
    uniform.radius_min = Meters(90.0);
    assert!(
        matches!(
            fault_of(&config),
            SectorFault::Config {
                field: "generation.radius_max",
                ..
            }
        ),
        "a maximum under the minimum must refuse rather than draw an empty band"
    );
}

#[test]
fn a_layered_config_with_no_planet_types_is_refused() {
    let mut config = layered();
    let SectorGeneration::LayeredFeatures(layered) = &mut config.generation else {
        unreachable!("the fixture is the layered generator");
    };
    layered.planet_types.clear();
    assert_eq!(
        fault_of(&config),
        SectorFault::Config {
            field: "generation.planet_types",
            value: "an empty list".to_string(),
        },
        "an empty archetype list must refuse rather than pick a house world"
    );
}

#[test]
fn a_layered_config_with_no_anchorage_design_is_refused() {
    let mut config = layered();
    let SectorGeneration::LayeredFeatures(layered) = &mut config.generation else {
        unreachable!("the fixture is the layered generator");
    };
    layered.anchorage_design = "   ".to_string();
    assert_eq!(
        fault_of(&config),
        SectorFault::Config {
            field: "generation.anchorage_design",
            value: "an empty id".to_string(),
        },
        "a blank design id must refuse here rather than at the catalog lookup one frame \
         before a hull spawns"
    );
}

#[test]
fn an_unusable_sector_edge_is_refused() {
    for edge in [Meters(0.0), Meters(-32_000.0), Meters(f32::NAN)] {
        let config = WorldConfig {
            sector_edge: edge,
            ..uniform()
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
        ..uniform()
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
            ..uniform()
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
            ..uniform()
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
fn a_uniform_edge_too_narrow_to_own_its_rocks_is_refused() {
    let fault = fault_of(&WorldConfig {
        sector_edge: Meters(2_399.0),
        ..uniform()
    });
    assert!(
        matches!(
            &fault,
            SectorFault::Config { field: "sector_edge", value }
                if value.contains("generation.radius_max")
        ),
        "a cell narrower than its own rocks must refuse and name the body, got {fault:?}"
    );
    let wide_enough = WorldConfig {
        sector_edge: Meters(2_401.0),
        ..uniform()
    };
    assert!(
        wide_enough.validate().is_ok(),
        "a cell just wide enough to own a 60 m rock must arm"
    );
}

/// The layered floor is set by the planetoid, not the rock or the hull.
///
/// 8 km, because a gated planetoid draws up to 1,200 m of REAL radius against
/// a gated rock's 360 m of meshed reach and a moored hull's 400 m. Naming
/// which body sets the floor is the point: a caller who narrows the cell is
/// told what they would have to shrink first.
#[test]
fn a_layered_edge_too_narrow_to_own_its_planetoids_is_refused() {
    let fault = fault_of(&WorldConfig {
        sector_edge: Meters(7_999.0),
        ..layered()
    });
    assert!(
        matches!(
            &fault,
            SectorFault::Config { field: "sector_edge", value }
                if value.contains("planetoid")
        ),
        "a cell narrower than its own planetoids must refuse and name them, got {fault:?}"
    );
    let wide_enough = WorldConfig {
        sector_edge: Meters(8_001.0),
        ..layered()
    };
    assert!(
        wide_enough.validate().is_ok(),
        "a cell just wide enough to own a 1,200 m planetoid must arm"
    );
}

#[test]
fn a_layered_edge_wider_than_the_thinning_halo_is_refused() {
    // In EVERY build, not a debug assertion: a release run that accepted this
    // edge would inspect the same 2-node halo and ship a world where two
    // same-layer spheres outside it both survive the thinning.
    let config = WorldConfig {
        sector_edge: Meters(500_000.0),
        ..layered()
    };
    assert!(
        matches!(
            fault_of(&config),
            SectorFault::Config {
                field: "sector_edge",
                ..
            }
        ),
        "a cell edge the thinning halo cannot reach across must refuse the config"
    );
    generate_sector(
        &WorldConfig {
            sector_edge: Meters(500_000.0),
            ..uniform()
        },
        SectorCoord::ORIGIN,
    )
    .expect("the uniform generator has no halo and no thinning, so the same edge is fine");
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

    use super::{layered, uniform};
    use crate::{prelude::WorldObserver, WorldConfig};

    /// A world with `config` armed and exactly one observer standing in it,
    /// taken through the arming frame's [`crate::NovaWorldSystems::Cleanup`].
    ///
    /// Cleanup is run for real rather than faked, because it is what records
    /// the config version the stages below it check themselves against. A
    /// helper that skipped it would arm every test into the very refusal
    /// `a_config_written_after_the_world_was_cleared_is_refused` pins.
    fn armed(config: WorldConfig) -> World {
        let mut world = World::new();
        world.insert_resource(config);
        world.init_resource::<crate::streaming::ReadySectors>();
        world.init_resource::<crate::SectorJobStats>();
        world.init_resource::<crate::streaming::ClearedConfig>();
        world.spawn((WorldObserver, GlobalTransform::default()));
        world
            .run_system_once(crate::streaming::clear_sector_work)
            .expect("the arming frame must clear the world it replaces");
        world
    }

    #[test]
    #[should_panic(expected = "WorldConfig::active_radius")]
    fn an_oversized_window_is_refused_before_it_is_enumerated() {
        let mut world = armed(WorldConfig {
            active_radius: 1_000,
            ..uniform()
        });
        let _ = world.run_system_once(crate::streaming::track_current_sector);
    }

    #[test]
    #[should_panic(expected = "WorldConfig::sector_edge")]
    fn a_layered_edge_the_halo_cannot_cover_is_refused_when_it_is_armed() {
        let mut world = armed(WorldConfig {
            sector_edge: Meters(500_000.0),
            ..layered()
        });
        let _ = world.run_system_once(crate::streaming::track_current_sector);
    }

    #[test]
    fn the_measured_window_arms() {
        let mut world = armed(uniform());
        world
            .run_system_once(crate::streaming::track_current_sector)
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
        let mut world = armed(uniform());
        world.resource_mut::<WorldConfig>().seed += 1;
        let _ = world.run_system_once(crate::streaming::track_current_sector);
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

    use super::uniform;
    use crate::prelude::{CurrentSector, SectorCoord, WorldObserver};

    /// A world with the fixture config in it, taken through the arming
    /// frame's [`crate::NovaWorldSystems::Cleanup`] and holding nothing else.
    fn world() -> World {
        let mut world = World::new();
        world.insert_resource(uniform());
        world.init_resource::<crate::streaming::ReadySectors>();
        world.init_resource::<crate::SectorJobStats>();
        world.init_resource::<crate::streaming::ClearedConfig>();
        world
            .run_system_once(crate::streaming::clear_sector_work)
            .expect("the arming frame must clear the world it replaces");
        world
    }

    #[test]
    #[should_panic(expected = "there is not exactly one WorldObserver")]
    fn a_session_with_no_observer_is_refused() {
        let _ = world().run_system_once(crate::streaming::track_current_sector);
    }

    #[test]
    #[should_panic(expected = "there is not exactly one WorldObserver")]
    fn a_session_with_two_observers_is_refused() {
        let mut world = world();
        world.spawn((WorldObserver, GlobalTransform::default()));
        world.spawn((WorldObserver, GlobalTransform::default()));
        let _ = world.run_system_once(crate::streaming::track_current_sector);
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
            .run_system_once(crate::streaming::track_current_sector)
            .expect("the observer system must run");
        assert_eq!(
            world.resource::<CurrentSector>().0,
            SectorCoord::new(1, 0, 0),
            "the streamed centre must be the cell the observer stands in"
        );
    }
}
