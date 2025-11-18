use bevy::{platform::collections::HashMap, prelude::*};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use rand::prelude::*;

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
    ])));
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
                health: 100.0,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig { radius, texture }),
        });
    }

    let spaceship = SpaceshipConfig {
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::from([
                (
                    "thruster".to_string(),
                    vec![KeyCode::Space.into(), GamepadButton::RightTrigger.into()],
                ),
                (
                    "turret".to_string(),
                    vec![
                        MouseButton::Left.into(),
                        GamepadButton::RightTrigger2.into(),
                    ],
                ),
            ]),
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
            health: 500.0,
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
            health: 100.0,
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
