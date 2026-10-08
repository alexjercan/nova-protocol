//! What the world refuses: a config nobody could generate from, a generator
//! answer nobody could materialize, and a second generator in one app.
//!
//! Mostly pure validation, so a unit test is the cheapest thing that observes
//! it. The claim each one carries is that a bad input is a REFUSAL and never a
//! silent fallback: an oversized window is not clamped, a rock outside its
//! cell is not moved, and an unshipped kind id is not skipped. Each of those
//! would put a world nobody authored in front of a player.
//!
//! Materialization proofs check a rock's gravity opt-in and hold a ship the
//! observer overlaps instead of spawning it. Another proves an unresolved
//! ship design is refused at spawn. The `frozen` proofs cover which cell owns
//! a body and what an off-window cell keeps.
//!
//! The generators here are test-local. The shipped policies live outside this
//! crate - the base game's in `nova_world_base`, the uniform baseline with the
//! examples - and are proved where they live.

use avian3d::prelude::{LinearVelocity, RigidBody};
use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nova_events::prelude::{Meters, Meters3, MetersPerSecond3};
use nova_gameplay::prelude::{
    Allegiance, AssetRef, DerelictShipMarker, GravityAffected, GravityWell, IntegrityEnvelope,
    ItemType, LootableShipMarker, ShipCredits, ShipInventoryStock,
};
use nova_scenario::prelude::{
    AIControllerConfig, AsteroidMarker, AsteroidPlugin, PlanetConfig, PlanetType, SectionSource,
    ShipDesign, SpaceshipController, SpaceshipSectionConfig, ASTEROID_GEOMETRIC_FACTOR_MAX,
    KIND_ROCK,
};
use nova_ship::prelude::{
    BaseSectionConfig, GameSections, HullSectionConfig, SectionConfig, SectionKind,
};

mod frozen;

use crate::{
    generate_sector, materialize_pending_ships, materialize_sector, prepare_sector, sector_id,
    validate_manifest, CivilizationId, NovaWorldPlugin, ObserverBody, PendingSectorShip,
    SectorAsteroid, SectorCoord, SectorFault, SectorGenerationInput, SectorGenerator,
    SectorManifest, SectorPlanet, SectorShip, SectorShipConditionType, SectorShipCrew,
    ShipRoleType, WorldConfig, WorldGeometry, WorldObserver, ACTIVE_WINDOW_SECTORS_MAX,
    SECTOR_SHIP_CLEARANCE_MAX,
};

/// The section prototype every test ship is built from.
const TEST_HULL_SECTION_ID: &str = "test_hull";

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
                initial_velocity: MetersPerSecond3::ZERO,
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

/// A one-section intact ship at `position`, built from the section prototype
/// `prototype`, with a 20 m clearance.
fn ship(id: String, position: Meters3, prototype: &str) -> SectorShip {
    SectorShip {
        id,
        position,
        initial_velocity: MetersPerSecond3::ZERO,
        rotation: Quat::IDENTITY,
        clearance: Meters(20.0),
        design: ShipDesign {
            sections: vec![SpaceshipSectionConfig {
                id: "hull".into(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::prototype(prototype),
            }],
            ..default()
        },
        condition: SectorShipConditionType::Intact,
        crew: Some(SectorShipCrew {
            allegiance: Allegiance::Neutral,
            patrol: Vec::new(),
            stops: Vec::new(),
            leash: Meters(5_000.0),
        }),
        civilization: CivilizationId {
            world_seed: 0,
            node: [0, 0, 0],
        },
        role: ShipRoleType::Civilian,
        stock: ShipInventoryStock::default(),
        credits: 120,
    }
}

/// A manifest of `input`'s cell holding one [`ship`] at its centre.
fn one_ship(input: SectorGenerationInput) -> SectorManifest {
    let mut manifest = empty(input.coord, Vec::new());
    manifest.ships.push(ship(
        sector_id(input.coord, "ship", 0),
        input.coord.centre(input.geometry.sector_edge),
        TEST_HULL_SECTION_ID,
    ));
    manifest
}

/// The loaded sections [`ship`] resolves against: one hull prototype.
fn test_sections() -> GameSections {
    GameSections(vec![SectionConfig {
        base: BaseSectionConfig {
            id: TEST_HULL_SECTION_ID.to_string(),
            ..default()
        },
        kind: SectionKind::Hull(HullSectionConfig::default()),
    }])
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
            initial_velocity: MetersPerSecond3::ZERO,
            kind: KIND_ROCK.into(),
            seed: 7,
        }
    }
    let cases: [(
        &str,
        fn(SectorGenerationInput) -> SectorManifest,
        fn(&SectorFault) -> bool,
    ); 18] = [
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
            "an asteroid with a non-finite initial velocity",
            |input| {
                let mut body = rock(input, "body_0", 0.0);
                body.initial_velocity = MetersPerSecond3::new(f32::NAN, 0.0, 0.0);
                empty(input.coord, vec![body])
            },
            |fault| {
                matches!(
                    fault,
                    SectorFault::Manifest {
                        field: "initial_velocity",
                        ..
                    }
                )
            },
        ),
        (
            "a ship with a non-finite initial velocity",
            |input| {
                let mut manifest = one_ship(input);
                manifest.ships[0].initial_velocity = MetersPerSecond3::new(0.0, f32::INFINITY, 0.0);
                manifest
            },
            |fault| {
                matches!(
                    fault,
                    SectorFault::Manifest {
                        field: "initial_velocity",
                        ..
                    }
                )
            },
        ),
        (
            "a planetoid whose well has a NaN mass",
            |input| {
                let mut manifest = empty(input.coord, Vec::new());
                manifest.planets.push(SectorPlanet {
                    id: format!("{}_planet_0", input.coord.slug()),
                    position: input.coord.centre(input.geometry.sector_edge),
                    config: PlanetConfig::new(PlanetType::BarrenRock, Meters(800.0), 3, f32::NAN),
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
            "a ship with no sections",
            |input| {
                let mut manifest = one_ship(input);
                manifest.ships[0].design.sections.clear();
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
        (
            "a ship with a non-finite rotation",
            |input| {
                let mut manifest = one_ship(input);
                manifest.ships[0].rotation = Quat::from_xyzw(f32::NAN, 0.0, 0.0, 1.0);
                manifest
            },
            |fault| matches!(fault, SectorFault::InvalidGeometry { id } if id.ends_with("ship_0")),
        ),
        (
            "a ship whose rotation is not a unit quaternion",
            |input| {
                let mut manifest = one_ship(input);
                manifest.ships[0].rotation = Quat::from_xyzw(0.0, 0.0, 0.0, 1.01);
                manifest
            },
            |fault| matches!(fault, SectorFault::InvalidGeometry { id } if id.ends_with("ship_0")),
        ),
        (
            "a ship whose clearance is above the maximum",
            |input| {
                let mut manifest = one_ship(input);
                manifest.ships[0].clearance = SECTOR_SHIP_CLEARANCE_MAX + Meters(1.0);
                manifest
            },
            |fault| {
                matches!(
                    fault,
                    SectorFault::Manifest {
                        field: "clearance",
                        ..
                    }
                )
            },
        ),
        (
            "an intact ship with no crew",
            |input| {
                let mut manifest = one_ship(input);
                manifest.ships[0].crew = None;
                manifest
            },
            |fault| matches!(fault, SectorFault::Manifest { field: "crew", .. }),
        ),
        (
            "a derelict with a crew",
            |input| {
                let mut manifest = one_ship(input);
                manifest.ships[0].condition = SectorShipConditionType::Derelict;
                manifest
            },
            |fault| matches!(fault, SectorFault::Manifest { field: "crew", .. }),
        ),
        (
            "a crew with a stop for a missing waypoint",
            |input| {
                let mut manifest = one_ship(input);
                manifest.ships[0]
                    .crew
                    .as_mut()
                    .expect("the fixture ship is crewed")
                    .stops = vec![30.0];
                manifest
            },
            |fault| matches!(fault, SectorFault::Manifest { field: "crew", .. }),
        ),
        (
            "a crew with a zero leash",
            |input| {
                let mut manifest = one_ship(input);
                manifest.ships[0]
                    .crew
                    .as_mut()
                    .expect("the fixture ship is crewed")
                    .leash = Meters(0.0);
                manifest
            },
            |fault| matches!(fault, SectorFault::Manifest { field: "crew", .. }),
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
                initial_velocity: MetersPerSecond3::ZERO,
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
            initial_velocity: MetersPerSecond3::ZERO,
            kind: KIND_ROCK.into(),
            seed: index as u32,
        })
        .collect();
    let mut manifest = empty(coord, asteroids);
    manifest.planets.push(SectorPlanet {
        id: sector_id(coord, "planet", 0),
        position: point(rocks),
        config: PlanetConfig::new(PlanetType::BarrenRock, Meters(800.0), 3, 5.0),
    });
    manifest.ships = (rocks + 1..count)
        .map(|index| SectorShip {
            clearance: SECTOR_SHIP_CLEARANCE_MAX,
            ..ship(
                sector_id(coord, "ship", index),
                point(index),
                TEST_HULL_SECTION_ID,
            )
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

/// A planetoid's authored fields are part of "the same sector": changing
/// any one of them, including its required mass, changes the canonical
/// description.
#[test]
fn every_planetoid_field_changes_the_canonical_description() {
    let base = PlanetConfig::new(PlanetType::BarrenRock, Meters(800.0), 3, 5.0);
    let variants = [
        base.clone(),
        PlanetConfig {
            relief: Some(Meters(40.0)),
            ..base.clone()
        },
        PlanetConfig {
            relief: Some(Meters(41.0)),
            ..base.clone()
        },
        PlanetConfig {
            sea_level: Some(0.0),
            ..base.clone()
        },
        PlanetConfig {
            sea_level: Some(0.5),
            ..base.clone()
        },
        PlanetConfig::new(PlanetType::BarrenRock, Meters(800.0), 3, 5.004),
        PlanetConfig::new(PlanetType::BarrenRock, Meters(800.0), 3, 5.008),
        PlanetConfig {
            lock_signature: Some(Meters(900.0)),
            ..base.clone()
        },
        PlanetConfig {
            lock_signature: Some(Meters(901.0)),
            ..base
        },
    ];
    let input = rocks().input(SectorCoord::ORIGIN);
    let described: Vec<String> = variants
        .into_iter()
        .map(|config| {
            let mut manifest = empty(input.coord, Vec::new());
            manifest.planets.push(SectorPlanet {
                id: sector_id(input.coord, "planet", 0),
                position: input.coord.centre(input.geometry.sector_edge),
                config,
            });
            validate_manifest(input, manifest)
                .expect("a valid planetoid must describe")
                .canonical()
        })
        .collect();

    for (index, text) in described.iter().enumerate() {
        for other in &described[index + 1..] {
            assert_ne!(text, other, "two different planetoids described the same");
        }
    }
}

/// A rock's initial velocity is part of "the same sector": distinct finite
/// velocities describe distinct cells.
#[test]
fn canonical_text_tells_asteroid_velocities_apart() {
    let input = rocks().input(SectorCoord::ORIGIN);
    let described: Vec<String> = [
        MetersPerSecond3::ZERO,
        MetersPerSecond3::new(0.0, 0.0, 1.0),
        MetersPerSecond3::new(0.0, 0.0, 4_000.0),
        MetersPerSecond3::new(0.0, 0.0, 4_000.001),
    ]
    .into_iter()
    .map(|initial_velocity| {
        let body = SectorAsteroid {
            id: sector_id(input.coord, "body", 0),
            position: input.coord.centre(input.geometry.sector_edge),
            radius: Meters(40.0),
            initial_velocity,
            kind: KIND_ROCK.into(),
            seed: 7,
        };
        validate_manifest(input, empty(input.coord, vec![body]))
            .expect("a valid rock must describe")
            .canonical()
    })
    .collect();

    for (index, text) in described.iter().enumerate() {
        for other in &described[index + 1..] {
            assert_ne!(
                text, other,
                "two different rock velocities described the same"
            );
        }
    }
}

/// A streamed asteroid is dynamic and receives its manifest velocity.
#[test]
fn a_materialized_asteroid_opts_into_gravity() {
    let config = answering(|input| {
        let rock = SectorAsteroid {
            id: sector_id(input.coord, "body", 0),
            position: input.coord.centre(input.geometry.sector_edge),
            radius: Meters(60.0),
            initial_velocity: MetersPerSecond3::new(24.0, -8.0, 4.0),
            kind: KIND_ROCK.into(),
            seed: 7,
        };
        empty(input.coord, vec![rock])
    });
    let prepared =
        prepare_sector(config, SectorCoord::ORIGIN).expect("a valid mobile rock must be prepared");

    let mut app = App::new();
    app.add_plugins(AsteroidPlugin { render: false });
    let world = app.world_mut();
    let root = materialize_sector(
        &mut world.commands(),
        prepared,
        None,
        &AssetRef::default(),
        &GameSections::default(),
        ObserverBody {
            position: Meters3::new(0.0, 0.0, 0.0),
            reach: Meters::ZERO,
        },
    );
    world.flush();
    app.update();

    let mut mobile = app.world_mut().query_filtered::<(
        &Name,
        &RigidBody,
        &LinearVelocity,
        &ChildOf,
        Has<GravityAffected>,
        Has<GravityWell>,
    ), With<AsteroidMarker>>();
    let materialized: Vec<_> = mobile
        .iter(app.world())
        .map(|(name, body, velocity, owner, affected, well)| {
            (
                name.to_string(),
                *body,
                **velocity,
                owner.parent(),
                affected,
                well,
            )
        })
        .collect();
    assert_eq!(
        materialized,
        vec![(
            sector_id(SectorCoord::ORIGIN, "body", 0),
            RigidBody::Dynamic,
            Vec3::new(2.4, -0.8, 0.4),
            root,
            true,
            false
        )],
        "a materialized asteroid must use the manifest motion and cannot source a well"
    );
}

/// `q` and `-q` are one rotation, so they describe one sector; a different
/// rotation describes a different one.
#[test]
fn canonical_text_describes_a_rotation_and_its_negation_alike() {
    let input = rocks().input(SectorCoord::ORIGIN);
    let turned = Quat::from_euler(EulerRot::YXZ, 0.7, -0.3, 1.9);
    let described: Vec<String> = [turned, -turned, Quat::from_rotation_y(0.7)]
        .into_iter()
        .map(|rotation| {
            let mut manifest = one_ship(input);
            manifest.ships[0].rotation = rotation;
            validate_manifest(input, manifest)
                .expect("a valid ship must describe")
                .canonical()
        })
        .collect();

    assert_eq!(described[0], described[1], "q and -q must describe alike");
    assert_ne!(
        described[0], described[2],
        "two different rotations described the same"
    );
}

/// Every ship field that changes what spawns, or whose ship it is, is part of
/// "the same sector": the design's integrity and presentation, a section's
/// prototype patch, the civilization and the role each describe a different
/// cell. A non-finite value prints apart from `None` and from another
/// non-finite value, never as one shared `null`.
#[test]
fn canonical_text_tells_ship_design_fields_and_identity_apart() {
    fn patch_health(ship: &mut SectorShip, health: f32) {
        let SectionSource::Prototype { patch, .. } = &mut ship.design.sections[0].source else {
            panic!("the fixture ship's hull is a prototype reference");
        };
        patch.health = Some(health);
    }
    let input = rocks().input(SectorCoord::ORIGIN);
    let variants: [(&str, fn(&mut SectorShip)); 11] = [
        ("as built", |_| {}),
        ("collapse 0.1", |ship| {
            ship.design.integrity.collapse_threshold = Some(0.1);
        }),
        ("collapse NaN", |ship| {
            ship.design.integrity.collapse_threshold = Some(f32::NAN);
        }),
        ("collapse inf", |ship| {
            ship.design.integrity.collapse_threshold = Some(f32::INFINITY);
        }),
        ("skinned", |ship| ship.design.presentation.skin = true),
        ("styled", |ship| {
            ship.design.presentation.style = Some("armoured".to_string());
        }),
        ("patched health 50", |ship| patch_health(ship, 50.0)),
        ("patched health NaN", |ship| patch_health(ship, f32::NAN)),
        ("patched health -inf", |ship| {
            patch_health(ship, f32::NEG_INFINITY);
        }),
        ("another civilization", |ship| {
            ship.civilization.node = [1, 0, 0]
        }),
        ("another role", |ship| ship.role = ShipRoleType::Armored),
    ];
    let described: Vec<(&str, String)> = variants
        .into_iter()
        .map(|(label, change)| {
            let mut manifest = one_ship(input);
            change(&mut manifest.ships[0]);
            let text = validate_manifest(input, manifest)
                .expect("a valid ship must describe")
                .canonical();
            (label, text)
        })
        .collect();

    for (index, (label, text)) in described.iter().enumerate() {
        for (other_label, other) in &described[index + 1..] {
            assert_ne!(text, other, "{label} and {other_label} described the same");
        }
    }
}

/// The observer's body where it overlaps [`one_ship`]'s clearance, and where
/// it is clear of it.
fn observer_at(input: SectorGenerationInput, offset: f32) -> ObserverBody {
    ObserverBody {
        position: input.coord.centre(input.geometry.sector_edge) + Meters3::new(offset, 0.0, 0.0),
        reach: Meters(15.0),
    }
}

/// Materialize [`one_ship`]'s cell into `world` with the observer's body at
/// `offset` from the ship. Returns the root.
fn materialize_one_ship(world: &mut World, offset: f32) -> Entity {
    let config = answering(one_ship);
    let prepared =
        prepare_sector(config.clone(), SectorCoord::ORIGIN).expect("one valid ship must prepare");
    let root = materialize_sector(
        &mut world.commands(),
        prepared,
        None,
        &AssetRef::default(),
        &test_sections(),
        observer_at(config.input(SectorCoord::ORIGIN), offset),
    );
    world.flush();
    root
}

/// The ids of the spawned ships under `root`, and of the held ones.
fn ships_under(world: &mut World, root: Entity) -> (Vec<String>, Vec<String>) {
    let spawned = world
        .query_filtered::<(&Name, &ChildOf), Without<PendingSectorShip>>()
        .iter(world)
        .filter(|(_, child_of)| child_of.parent() == root)
        .map(|(name, _)| name.to_string())
        .collect();
    let held = world
        .query::<(&PendingSectorShip, &ChildOf)>()
        .iter(world)
        .filter(|(_, child_of)| child_of.parent() == root)
        .map(|(held, _)| held.ship().id.clone())
        .collect();
    (spawned, held)
}

/// A ship whose clearance the observer's body overlaps is held under its
/// cell's root, stays held while the overlap lasts, and spawns as its
/// manifest entry once the observer is clear.
#[test]
fn a_ship_the_observer_overlaps_is_held_until_the_observer_is_clear() {
    let mut world = World::new();
    // 20 m of ship clearance and 15 m of observer reach overlap at 30 m.
    let root = materialize_one_ship(&mut world, 30.0);
    let id = sector_id(SectorCoord::ORIGIN, "ship", 0);
    assert_eq!(
        ships_under(&mut world, root),
        (Vec::new(), vec![id.clone()]),
        "an overlapped ship must be held, not spawned"
    );

    let input = answering(one_ship).input(SectorCoord::ORIGIN);
    let observer = world
        .spawn((
            WorldObserver,
            GlobalTransform::from_translation(observer_at(input, 30.0).position.to_engine()),
            IntegrityEnvelope(observer_at(input, 30.0).reach.to_engine()),
        ))
        .id();
    world
        .run_system_once(materialize_pending_ships)
        .expect("materialize_pending_ships runs");
    world.flush();
    assert_eq!(
        ships_under(&mut world, root),
        (Vec::new(), vec![id.clone()]),
        "a ship must stay held while the observer overlaps it"
    );

    // 20 m + 15 m clear at 40 m.
    world
        .entity_mut(observer)
        .insert(GlobalTransform::from_translation(
            observer_at(input, 40.0).position.to_engine(),
        ));
    world
        .run_system_once(materialize_pending_ships)
        .expect("materialize_pending_ships runs");
    world.flush();
    assert_eq!(
        ships_under(&mut world, root),
        (vec!["Solmar civilian".to_string()], Vec::new()),
        "a cleared ship must spawn under its root and stop being held"
    );
}

/// A manifest of `input`'s cell holding an intact [`ship`] at its centre with
/// an Enemy crew on a two-waypoint loop, and a derelict one 1 km off carrying
/// five hull plates and 15 credits.
fn intact_and_derelict(input: SectorGenerationInput) -> SectorManifest {
    let centre = input.coord.centre(input.geometry.sector_edge);
    let mut manifest = one_ship(input);
    manifest.ships[0].crew = Some(SectorShipCrew {
        allegiance: Allegiance::Enemy,
        patrol: vec![
            centre + Meters3::new(0.0, 0.0, 1_500.0),
            centre + Meters3::new(0.0, 0.0, -1_500.0),
        ],
        stops: vec![30.0, 45.0],
        leash: Meters(5_000.0),
    });
    manifest.ships.push(SectorShip {
        condition: SectorShipConditionType::Derelict,
        crew: None,
        stock: ShipInventoryStock::new([(ItemType::HullPlate, 5)]),
        credits: 15,
        ..ship(
            sector_id(input.coord, "ship", 1),
            centre + Meters3::new(1_000.0, 0.0, 0.0),
            TEST_HULL_SECTION_ID,
        )
    });
    manifest
}

/// A derelict spawns lootable with its manifest stock, so a docked ship may
/// Take it, and flown by nobody on nobody's side; an intact ship spawns
/// neither lootable nor stocked, flown by an AI on its crew's side along its
/// crew's loop, an Enemy crew holding fire for its arrival grace. Each spawns
/// with its manifest credits.
#[test]
fn a_materialized_derelict_is_lootable_and_unflown_and_an_intact_ship_is_flown_by_its_crew() {
    let config = answering(intact_and_derelict);
    let prepared =
        prepare_sector(config.clone(), SectorCoord::ORIGIN).expect("two valid ships must prepare");
    let mut world = World::new();
    materialize_sector(
        &mut world.commands(),
        prepared,
        None,
        &AssetRef::default(),
        &test_sections(),
        observer_at(config.input(SectorCoord::ORIGIN), 5_000.0),
    );
    world.flush();

    let mut spawned: Vec<(String, bool, bool, Vec<(ItemType, u32)>, u32)> = world
        .query::<(
            &Name,
            Has<DerelictShipMarker>,
            Has<LootableShipMarker>,
            &ShipInventoryStock,
            &ShipCredits,
        )>()
        .iter(&world)
        .map(|(name, derelict, lootable, stock, credits)| {
            (
                name.to_string(),
                derelict,
                lootable,
                stock.stacks().collect(),
                credits.0,
            )
        })
        .collect();
    spawned.sort();
    assert_eq!(
        spawned,
        vec![
            ("Solmar civilian".to_string(), false, false, vec![], 120),
            (
                "Solmar derelict, former civilian".to_string(),
                true,
                true,
                vec![(ItemType::HullPlate, 5)],
                15
            ),
        ]
    );

    let mut flown: Vec<(String, Allegiance, Option<AIControllerConfig>)> = world
        .query::<(&Name, &Allegiance, &SpaceshipController)>()
        .iter(&world)
        .map(|(name, allegiance, controller)| {
            let crew = match controller {
                SpaceshipController::AI(config) => Some(config.clone()),
                SpaceshipController::None => None,
                SpaceshipController::Player(_) => panic!("{name} spawned player-flown"),
            };
            (name.to_string(), *allegiance, crew)
        })
        .collect();
    flown.sort_by(|a, b| a.0.cmp(&b.0));
    let [(_, intact_side, Some(crew)), (_, derelict_side, None)] = flown.as_slice() else {
        panic!("the intact ship must be AI-flown and the derelict unflown: {flown:?}");
    };
    let centre = SectorCoord::ORIGIN.centre(config.geometry().sector_edge);
    assert_eq!(*intact_side, Allegiance::Enemy);
    assert_eq!(
        crew.patrol,
        [
            centre + Meters3::new(0.0, 0.0, 1_500.0),
            centre + Meters3::new(0.0, 0.0, -1_500.0),
        ]
    );
    assert_eq!(crew.patrol_stops, [30.0, 45.0]);
    assert_eq!(crew.leash, Some(Meters(5_000.0)));
    assert_eq!(
        crew.engage_delay,
        Some(8.0),
        "an Enemy crew arrives holding fire"
    );
    assert_eq!(*derelict_side, Allegiance::Neutral);
}

/// The spawn's resolver skips a section it cannot resolve and flies the
/// rest, so `materialize_sector` resolves every design strictly first: a
/// section prototype the loaded catalog does not hold is refused before the
/// cell exists.
#[test]
#[should_panic(expected = "has a design the loaded sections do not resolve")]
fn a_ship_design_the_loaded_sections_do_not_resolve_is_refused_at_spawn() {
    let prepared = prepare_sector(
        answering(|input| {
            let mut manifest = one_ship(input);
            manifest.ships[0]
                .design
                .sections
                .push(SpaceshipSectionConfig {
                    id: "missing".into(),
                    position: Vec3::X,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::prototype("not_in_the_catalog"),
                });
            manifest
        }),
        SectorCoord::ORIGIN,
    )
    .expect("the worker cannot see the catalog, so the ship must prepare");
    let mut world = World::new();
    materialize_sector(
        &mut world.commands(),
        prepared,
        None,
        &AssetRef::default(),
        &test_sections(),
        ObserverBody {
            position: Meters3::new(0.0, 0.0, 0.0),
            reach: Meters::ZERO,
        },
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

/// A negative radius has no window, and a radius of 0 is a window of one
/// cell that a ship docked across its face would leave.
#[test]
fn an_active_radius_below_one_is_refused() {
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
    let config = WorldConfig {
        active_radius: 0,
        ..rocks()
    };
    assert_eq!(
        fault_of(&config),
        SectorFault::Config {
            field: "active_radius",
            value: "0, a window of one cell, which a ship docked across its face would leave"
                .to_string(),
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
        world.init_resource::<crate::FrozenSectors>();
        world.init_resource::<crate::frozen::SettlingBodies>();
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
        world.init_resource::<crate::FrozenSectors>();
        world.init_resource::<crate::frozen::SettlingBodies>();
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
