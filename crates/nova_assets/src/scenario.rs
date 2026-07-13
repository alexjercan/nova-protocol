use bevy::{platform::collections::HashMap, prelude::*};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use rand::prelude::*;

pub mod shakedown;

pub(crate) fn register_scenario(
    mut commands: Commands,
    game_assets: Res<super::GameAssets>,
    sections: Res<GameSections>,
) {
    commands.insert_resource(GameScenarios(HashMap::from([
        (
            "asteroid_field".to_string(),
            asteroid_field(&game_assets, &sections),
        ),
        (
            "asteroid_next".to_string(),
            asteroid_next(&game_assets, &sections),
        ),
        (
            "menu_ambience".to_string(),
            menu_ambience(&game_assets, &sections),
        ),
        (
            shakedown::SHAKEDOWN_SCENARIO_ID.to_string(),
            shakedown::shakedown_run(&game_assets, &sections),
        ),
    ])));
}

/// The main menu's living backdrop (task 20260711-180455): a big planetoid
/// with a real gravity well, a scatter of rocks, and one AI ship flying a
/// thruster-driven orbit around the planetoid (orbit directive, task
/// 20260711-212504). No
/// player, no objectives, no areas - the scene exists to be looked at.
pub fn menu_ambience(game_assets: &super::GameAssets, sections: &GameSections) -> ScenarioConfig {
    let mut rng = rand::rng();

    let mut objects = Vec::new();

    // The stage: a nominally-20u planetoid at the origin with an authored
    // surface gravity of 6 u/s^2. The nominal numbers are only the inputs:
    // the runtime well derives mu and SOI from the GEOMETRIC collider
    // radius (observed ~80-91u across seeds; insert_asteroid_gravity_well),
    // so the real mu lands well above the nominal 6 * 20^2 = 2400.
    objects.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "menu_planetoid".to_string(),
            name: "Menu Planetoid".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: 20.0,
            texture: game_assets.asteroid_texture.clone(),
            health: 2000.0,
            surface_gravity: Some(6.0),
            invulnerable: true,
            lock_signature: None,
        }),
    });

    // A loose ring of small rocks for depth, kept strictly out of harm's way:
    // the planetoid's GEOMETRIC radius runs several times its nominal 20u
    // (observed ~80-91u across seeds), and rocks that spawn inside that mesh
    // get penetration-resolved with impulses whose collision damage destroyed
    // the planetoid (and its gravity well) within a second - twice, in two
    // different ring layouts. So the ring starts past any plausible geometric
    // radius AND sits below the orbit plane (the orbiter circles at y=0 at
    // roughly body_radius + 40), keeping it clear of the orbit across collider
    // seeds (worst-case clearance is on the order of 10u, not unbounded - if
    // the planetoid's nominal radius grows, regrow this ring floor with it).
    for i in 0..14 {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let dist = rng.random_range(170.0..240.0);
        let pos = Vec3::new(
            angle.cos() * dist,
            rng.random_range(-70.0..-30.0),
            angle.sin() * dist,
        );
        objects.push(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: format!("menu_rock_{}", i),
                name: format!("Menu Rock {}", i),
                position: pos,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                radius: rng.random_range(1.0..3.0),
                texture: game_assets.asteroid_texture.clone(),
                health: 100.0,
                surface_gravity: None,
                invulnerable: false,
                lock_signature: None,
            }),
        });
    }

    // The actor: an AI ship directed to orbit the planetoid on its own
    // thrusters (task 20260711-212504) - the ORBIT autopilot plans its ring
    // from the well's runtime geometry, so no staging math lives here or in
    // nova_menu. It spawns comfortably outside the planetoid's geometric
    // surface (the noise mesh reaches several times past the nominal 20u)
    // and inside its SOI, and flies itself in from there. WARNING: the
    // spaceship input/section sets ARE live in MainMenu - this scenario is
    // a loaded scenario like any other (scenario_is_live gating,
    // nova_scenario) - so keep ambience ships off
    // SpaceshipController::Player.
    objects.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "menu_orbiter".to_string(),
            name: "Menu Orbiter".to_string(),
            position: Vec3::new(140.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig {
                orbit: Some("menu_planetoid".to_string()),
                ..Default::default()
            }),
            sections: vec![
                SpaceshipSectionConfig {
                    id: "controller".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    config: sections
                        .get_section("basic_controller_section")
                        .unwrap()
                        .clone(),
                },
                SpaceshipSectionConfig {
                    id: "hull_front".to_string(),
                    position: Vec3::new(0.0, 0.0, 1.0),
                    rotation: Quat::IDENTITY,
                    config: sections
                        .get_section("reinforced_hull_section")
                        .unwrap()
                        .clone(),
                },
                SpaceshipSectionConfig {
                    id: "thruster".to_string(),
                    position: Vec3::new(0.0, 0.0, 2.0),
                    rotation: Quat::IDENTITY,
                    config: sections
                        .get_section("basic_thruster_section")
                        .unwrap()
                        .clone(),
                },
            ],
        }),
    });

    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: objects
            .into_iter()
            .map(EventActionConfig::SpawnScenarioObject)
            .collect::<_>(),
    }];

    ScenarioConfig {
        id: "menu_ambience".to_string(),
        name: "Menu Ambience".to_string(),
        description: "The main menu's living backdrop.".to_string(),
        cubemap: game_assets.cubemap.clone(),
        events,
    }
}

pub fn asteroid_field(game_assets: &super::GameAssets, sections: &GameSections) -> ScenarioConfig {
    let mut rng = rand::rng();

    let mut objects = Vec::new();
    for i in 0..20 {
        let pos = Vec3::new(
            rng.random_range(-100.0..100.0),
            rng.random_range(-20.0..20.0),
            rng.random_range(-100.0..100.0),
        );
        let radius = rng.random_range(1.0..3.0);
        let texture = game_assets.asteroid_texture.clone();

        objects.push(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: format!("asteroid_{}", i),
                name: format!("Asteroid {}", i),
                position: pos,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                radius,
                texture,
                health: 100.0,
                surface_gravity: None,
                invulnerable: false,
                lock_signature: None,
            }),
        });
    }

    // One large designated body clear of the combat field (+X, past the
    // scatter cube), so the gravity well is playtestable: 20u rock at the
    // retuned default strength (6 u/s^2 surface, SOI 160u, v_circ ~ 6.9
    // u/s at r = 50u; doubled per the 2026-07-10 playtest). The field
    // rocks above stay well-free via the radius threshold in
    // GravitySettings. Invulnerable so turret fire (now gravity-affected)
    // cannot destroy the well mid-playtest and take the orbit demo with it.
    objects.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "asteroid_grav".to_string(),
            name: "Gravity Rock".to_string(),
            position: Vec3::new(250.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: 20.0,
            texture: game_assets.asteroid_texture.clone(),
            health: 2000.0,
            surface_gravity: Some(6.0),
            invulnerable: true,
            lock_signature: None,
        }),
    });

    let spaceship = SpaceshipConfig {
        controller: SpaceshipController::Player(PlayerControllerConfig {
            // Translation is no longer a per-section binding: the flight
            // layer (nova_gameplay::flight) owns it through the flight input
            // rig (W/Space/right trigger analog burn; X autopilot STOP, G
            // autopilot GOTO the lock, Z autopilot off). Direct per-thruster
            // bindings remain supported for ships without a flight computer.
            input_mapping: HashMap::from([(
                "turret".to_string(),
                vec![
                    MouseButton::Left.into(),
                    GamepadButton::RightTrigger2.into(),
                ],
            )]),

            speed_cap: None,
            // The sandbox scenario keeps the authored finite magazines.
            infinite_ammo: false,
        }),
        sections: vec![
            SpaceshipSectionConfig {
                id: "controller".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("basic_controller_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "hull_front".to_string(),
                position: Vec3::new(0.0, 0.0, 1.0),
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("reinforced_hull_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "hull_back".to_string(),
                position: Vec3::new(0.0, 0.0, -1.0),
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("reinforced_hull_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "thruster".to_string(),
                position: Vec3::new(0.0, 0.0, 2.0),
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("basic_thruster_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "turret".to_string(),
                position: Vec3::new(0.0, 0.0, -2.0),
                rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                config: sections
                    .get_section("better_turret_section")
                    .unwrap()
                    .clone(),
            },
        ],
    };
    objects.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "player_spaceship".to_string(),
            name: "Player Spaceship".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(spaceship),
    });

    let spaceship = SpaceshipConfig {
        controller: SpaceshipController::None,
        sections: vec![
            SpaceshipSectionConfig {
                id: "controller".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("basic_controller_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "hull_front".to_string(),
                position: Vec3::new(0.0, 0.0, 1.0),
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("reinforced_hull_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "hull_back".to_string(),
                position: Vec3::new(0.0, 0.0, -1.0),
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("reinforced_hull_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "thruster".to_string(),
                position: Vec3::new(0.0, 0.0, 2.0),
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("basic_thruster_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "turret".to_string(),
                position: Vec3::new(0.0, 0.0, -2.0),
                rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                config: sections
                    .get_section("better_turret_section")
                    .unwrap()
                    .clone(),
            },
        ],
    };
    objects.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "other_spaceship".to_string(),
            name: "Other Spaceship".to_string(),
            position: Vec3::new(10.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(spaceship),
    });

    let events = vec![
        // OnStart: Create the scenario objects
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: objects
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .collect::<_>(),
        },
        // OnStart: Create the safe zone
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: vec![EventActionConfig::CreateScenarioArea(ScenarioAreaConfig {
                id: "asteroid_zone".to_string(),
                name: "Asteroid Zone".to_string(),
                position: Vec3::new(0.0, 0.0, -100.0),
                rotation: Quat::IDENTITY,
                radius: 10.0,
            })],
        },
        // OnStart: Create the destroy asteroids objective
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: vec![EventActionConfig::Objective(ObjectiveActionConfig::new(
                "destroy_asteroids",
                "Objective: Destroy 5 asteroids!",
            ))],
        },
        // OnStart: Initialize variables
        // asteroids_destroyed = 0
        // objective_destroy_asteroids = false
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: vec![
                EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "asteroids_destroyed".to_string(),
                    expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(0.0)),
                    )),
                }),
                EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "objective_destroy_asteroids".to_string(),
                    expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Boolean(false)),
                    )),
                }),
            ],
        },
        // OnDestroyed: If player spaceship destroyed, show message and go to next scenario
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: Some("player_spaceship".to_string()),
                type_name: None,
                ..default()
            })],
            actions: vec![
                EventActionConfig::DebugMessage(DebugMessageActionConfig {
                    message: "The player's spaceship was destroyed!".to_string(),
                }),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: "asteroid_field".to_string(),
                    linger: true,
                }),
            ],
        },
        // OnDestroyed: If an asteroid is destroyed, increment asteroids_destroyed
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: None,
                type_name: Some("asteroid".to_string()),
                ..default()
            })],
            actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                key: "asteroids_destroyed".to_string(),
                expression: VariableExpressionNode::new_add(
                    VariableTermNode::new_factor(VariableFactorNode::new_name(
                        "asteroids_destroyed".to_string(),
                    )),
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                    )),
                ),
            })],
        },
        // OnDestroyed: If an asteroid is destroyed and asteroids_destroyed > 4 and objective not
        // complete, complete objective and create new objective to reach safe zone
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![
                EventFilterConfig::Entity(EntityFilterConfig {
                    id: None,
                    type_name: Some("asteroid".to_string()),
                    ..default()
                }),
                EventFilterConfig::Expression(ExpressionFilterConfig(
                    VariableConditionNode::new_greater_than(
                        VariableExpressionNode::new_term(VariableTermNode::new_factor(
                            VariableFactorNode::new_name("asteroids_destroyed".to_string()),
                        )),
                        VariableExpressionNode::new_term(VariableTermNode::new_factor(
                            VariableFactorNode::new_literal(VariableLiteral::Number(4.0)),
                        )),
                    ),
                )),
                EventFilterConfig::Expression(ExpressionFilterConfig(
                    VariableConditionNode::new_equals(
                        VariableExpressionNode::new_term(VariableTermNode::new_factor(
                            VariableFactorNode::new_name("objective_destroy_asteroids".to_string()),
                        )),
                        VariableExpressionNode::new_term(VariableTermNode::new_factor(
                            VariableFactorNode::new_literal(VariableLiteral::Boolean(false)),
                        )),
                    ),
                )),
            ],
            actions: vec![
                EventActionConfig::DebugMessage(DebugMessageActionConfig {
                    message: "Objective Complete: Destroyed 5 asteroids!".to_string(),
                }),
                EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "objective_destroy_asteroids".to_string(),
                    expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
                    )),
                }),
                EventActionConfig::ObjectiveComplete(ObjectiveCompleteActionConfig {
                    id: "destroy_asteroids".to_string(),
                }),
                EventActionConfig::Objective(ObjectiveActionConfig::new(
                    "reach_zone",
                    "Objective: Reach the safe zone!",
                )),
            ],
        },
        // OnEnter: If player spaceship enters safe zone and the destroy asteroids objective is
        // complete, complete the scenario
        ScenarioEventConfig {
            name: EventConfig::OnEnter,
            filters: vec![
                EventFilterConfig::Entity(EntityFilterConfig {
                    id: Some("asteroid_zone".to_string()),
                    other_id: Some("player_spaceship".to_string()),
                    ..default()
                }),
                EventFilterConfig::Expression(ExpressionFilterConfig(
                    VariableConditionNode::new_equals(
                        VariableExpressionNode::new_term(VariableTermNode::new_factor(
                            VariableFactorNode::new_name("objective_destroy_asteroids".to_string()),
                        )),
                        VariableExpressionNode::new_term(VariableTermNode::new_factor(
                            VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
                        )),
                    ),
                )),
            ],
            actions: vec![
                EventActionConfig::ObjectiveComplete(ObjectiveCompleteActionConfig {
                    id: "reach_zone".to_string(),
                }),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: "asteroid_next".to_string(),
                    linger: true,
                }),
            ],
        },
    ];

    ScenarioConfig {
        id: "asteroid_field".to_string(),
        name: "Asteroid Field".to_string(),
        description: "A dense asteroid field.".to_string(),
        cubemap: game_assets.cubemap.clone(),
        events,
    }
}

pub fn asteroid_next(game_assets: &super::GameAssets, _sections: &GameSections) -> ScenarioConfig {
    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
            scenario_id: "asteroid_field".to_string(),
            linger: true,
        })],
    }];

    ScenarioConfig {
        id: "asteroid_next".to_string(),
        name: "Asteroid Field - Next".to_string(),
        description: "The next scenario after the asteroid field.".to_string(),
        cubemap: game_assets.cubemap.clone(),
        events,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// A GameAssets with default handles - fine for config-shape tests,
    /// which never resolve the assets. pub(crate): the shakedown child
    /// module's tests build on the same helpers.
    pub(crate) fn dummy_assets() -> crate::GameAssets {
        crate::GameAssets {
            cubemap: Handle::default(),
            asteroid_texture: Handle::default(),
            hull_01: Handle::default(),
            turret_yaw_01: Handle::default(),
            turret_pitch_01: Handle::default(),
            turret_barrel_01: Handle::default(),
            torpedo_bay_01: Handle::default(),
            fps_icon: Handle::default(),
            target_sprite: Handle::default(),
        }
    }

    /// The real section registry, built by the production register_sections
    /// system against the dummy assets.
    pub(crate) fn real_sections() -> GameSections {
        let mut world = World::new();
        world.insert_resource(dummy_assets());
        world
            .run_system_once(crate::sections::register_sections)
            .unwrap();
        world.remove_resource::<GameSections>().unwrap()
    }

    /// New Game's contract with nova_menu: the shakedown scenario is
    /// actually registered under the id the menu hardcodes (the menu
    /// panics at runtime on a miss, which a config typo here would cause).
    #[test]
    fn shakedown_run_is_registered() {
        let mut world = World::new();
        world.insert_resource(dummy_assets());
        world.insert_resource(real_sections());
        world.run_system_once(register_scenario).unwrap();
        let scenarios = world.resource::<GameScenarios>();
        assert!(
            scenarios.contains_key(shakedown::SHAKEDOWN_SCENARIO_ID),
            "shakedown_run must be in GameScenarios"
        );
    }

    /// The menu backdrop's contract (task 20260711-212504): the orbiter is
    /// an AI ship directed to orbit the planetoid on its own thrusters -
    /// controller + thruster sections aboard, directive pointing at an
    /// object that actually exists in the same scenario and carries an
    /// authored surface gravity (so it gets a well at spawn).
    #[test]
    fn menu_orbiter_is_an_ai_ship_directed_at_the_planetoid() {
        let assets = dummy_assets();
        let scenario = menu_ambience(&assets, &real_sections());

        let spawns: Vec<_> = scenario
            .events
            .iter()
            .flat_map(|event| event.actions.iter())
            .filter_map(|action| match action {
                EventActionConfig::SpawnScenarioObject(object) => Some(object),
                _ => None,
            })
            .collect();

        let orbiter = spawns
            .iter()
            .find(|object| object.base.id == "menu_orbiter")
            .expect("the backdrop spawns the orbiter");
        let ScenarioObjectKind::Spaceship(ship) = &orbiter.kind else {
            panic!("the orbiter is a spaceship");
        };
        let SpaceshipController::AI(ai) = &ship.controller else {
            panic!("the orbiter is AI-controlled, got {:?}", ship.controller);
        };
        assert_eq!(
            ai.orbit.as_deref(),
            Some("menu_planetoid"),
            "the directive targets the planetoid"
        );
        let has = |kind: fn(&SectionKind) -> bool| {
            ship.sections
                .iter()
                .any(|section| kind(&section.config.kind))
        };
        assert!(
            has(|kind| matches!(kind, SectionKind::Controller(_))),
            "a controller section flies the autopilot's attitude commands"
        );
        assert!(
            has(|kind| matches!(kind, SectionKind::Thruster(_))),
            "a thruster section provides the burn"
        );

        // The directive's target exists and gets a gravity well at spawn
        // (authored surface gravity), so the ORBIT autopilot can engage.
        let planetoid = spawns
            .iter()
            .find(|object| object.base.id == "menu_planetoid")
            .expect("the backdrop spawns the planetoid the directive names");
        let ScenarioObjectKind::Asteroid(rock) = &planetoid.kind else {
            panic!("the planetoid is an asteroid body");
        };
        assert!(
            rock.surface_gravity.is_some(),
            "authored surface gravity is what spawns the planetoid's well"
        );
    }
}
