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
    UniformAsteroidConfig, WorldConfig,
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
fn a_uniform_config_with_no_bodies_is_refused() {
    let mut config = uniform();
    let SectorGeneration::UniformAsteroids(uniform) = &mut config.generation else {
        unreachable!("the fixture is the uniform generator");
    };
    uniform.body_count = 0;
    assert_eq!(
        fault_of(&config),
        SectorFault::Config {
            field: "generation.body_count",
            value: "0".to_string(),
        }
    );
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

    /// A world with the fixture config in it and nothing else.
    fn world() -> World {
        let mut world = World::new();
        world.insert_resource(uniform());
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
