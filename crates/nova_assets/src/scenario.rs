use bevy::{platform::collections::HashMap, prelude::*};
use nova_gameplay::prelude::*;
use nova_modding::prelude::ScenarioAsset;
use nova_scenario::prelude::*;

pub mod shakedown;

/// The seed of the built-in scenarios' scatter fields. A fixed value: the
/// ports replaced the old per-launch RNG rock loops with a single seeded
/// `ScatterObjects` action each, so the layout is now deterministic content
/// (reproducible across loads) rather than random per launch.
const SCATTER_SEED: u64 = 0x0605_0403_0201_0000;

pub fn register_scenario(
    mut commands: Commands,
    game_assets: Res<super::GameAssets>,
    scenario_assets: Res<Assets<ScenarioAsset>>,
) {
    let mut scenarios = HashMap::new();

    // The four built-ins are now RON data files loaded into the GameAssets
    // collection (like the demo below); look each up by its handle. On a miss,
    // log and skip - never panic - exactly as the demo does.
    let built_ins = [
        &game_assets.asteroid_field_scenario,
        &game_assets.asteroid_next_scenario,
        &game_assets.menu_ambience_scenario,
        &game_assets.shakedown_scenario,
        &game_assets.demo_scenario,
    ];
    for handle in built_ins {
        match scenario_assets.get(handle) {
            Some(asset) => {
                scenarios.insert(asset.0.id.clone(), asset.0.clone());
            }
            None => {
                error!(
                    "register_scenario: a scenario asset was not loaded; skipping it \
                     (the other scenarios still register)"
                );
            }
        }
    }

    commands.insert_resource(GameScenarios(scenarios));
}

/// The main menu's living backdrop (task 20260711-180455): a big planetoid
/// with a real gravity well, a scatter of rocks, and one AI ship flying a
/// thruster-driven orbit around the planetoid (orbit directive, task
/// 20260711-212504). No
/// player, no objectives, no areas - the scene exists to be looked at.
pub(crate) fn menu_ambience(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
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
            texture: asteroid_texture.clone(),
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
    //
    // This is now a single seeded ScatterObjects action (below, in the OnStart
    // event) rather than a per-launch RNG loop: the layout is deterministic
    // content, reproducible across loads.
    let menu_rock_scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "menu_rock_".to_string(),
        count: 14,
        seed: SCATTER_SEED,
        region: ScatterRegion::Ring {
            inner: 170.0,
            outer: 240.0,
            y_min: -70.0,
            y_max: -30.0,
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "menu_rock_".to_string(),
                name: "Menu Rock".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                radius: 1.0,
                texture: asteroid_texture.clone(),
                health: 100.0,
                surface_gravity: None,
                invulnerable: false,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((1.0, 3.0)),
    });

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
                    source: SectionSource::Prototype("basic_controller_section".to_string()),
                    modifications: vec![],
                },
                SpaceshipSectionConfig {
                    id: "hull_front".to_string(),
                    position: Vec3::new(0.0, 0.0, 1.0),
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Prototype("reinforced_hull_section".to_string()),
                    modifications: vec![],
                },
                SpaceshipSectionConfig {
                    id: "thruster".to_string(),
                    position: Vec3::new(0.0, 0.0, 2.0),
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Prototype("basic_thruster_section".to_string()),
                    modifications: vec![],
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
            .chain([menu_rock_scatter])
            .collect::<_>(),
    }];

    ScenarioConfig {
        id: "menu_ambience".to_string(),
        name: "Menu Ambience".to_string(),
        description: "The main menu's living backdrop.".to_string(),
        cubemap,
        events,
    }
}

pub(crate) fn asteroid_field(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    // The field scatter is now a single seeded ScatterObjects action (added to
    // the OnStart event below) rather than a per-launch RNG loop: the layout is
    // deterministic content, reproducible across loads.
    let asteroid_scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "asteroid_".to_string(),
        count: 20,
        seed: SCATTER_SEED,
        region: ScatterRegion::Box {
            min: Vec3::new(-100.0, -20.0, -100.0),
            max: Vec3::new(100.0, 20.0, 100.0),
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "asteroid_".to_string(),
                name: "Asteroid".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                radius: 1.0,
                texture: asteroid_texture.clone(),
                health: 100.0,
                surface_gravity: None,
                invulnerable: false,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((1.0, 3.0)),
    });

    let mut objects = Vec::new();

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
            texture: asteroid_texture.clone(),
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
                source: SectionSource::Prototype("basic_controller_section".to_string()),
                modifications: vec![],
            },
            SpaceshipSectionConfig {
                id: "hull_front".to_string(),
                position: Vec3::new(0.0, 0.0, 1.0),
                rotation: Quat::IDENTITY,
                source: SectionSource::Prototype("reinforced_hull_section".to_string()),
                modifications: vec![],
            },
            SpaceshipSectionConfig {
                id: "hull_back".to_string(),
                position: Vec3::new(0.0, 0.0, -1.0),
                rotation: Quat::IDENTITY,
                source: SectionSource::Prototype("reinforced_hull_section".to_string()),
                modifications: vec![],
            },
            SpaceshipSectionConfig {
                id: "thruster".to_string(),
                position: Vec3::new(0.0, 0.0, 2.0),
                rotation: Quat::IDENTITY,
                source: SectionSource::Prototype("basic_thruster_section".to_string()),
                modifications: vec![],
            },
            SpaceshipSectionConfig {
                id: "turret".to_string(),
                position: Vec3::new(0.0, 0.0, -2.0),
                rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                source: SectionSource::Prototype("better_turret_section".to_string()),
                modifications: vec![],
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
                source: SectionSource::Prototype("basic_controller_section".to_string()),
                modifications: vec![],
            },
            SpaceshipSectionConfig {
                id: "hull_front".to_string(),
                position: Vec3::new(0.0, 0.0, 1.0),
                rotation: Quat::IDENTITY,
                source: SectionSource::Prototype("reinforced_hull_section".to_string()),
                modifications: vec![],
            },
            SpaceshipSectionConfig {
                id: "hull_back".to_string(),
                position: Vec3::new(0.0, 0.0, -1.0),
                rotation: Quat::IDENTITY,
                source: SectionSource::Prototype("reinforced_hull_section".to_string()),
                modifications: vec![],
            },
            SpaceshipSectionConfig {
                id: "thruster".to_string(),
                position: Vec3::new(0.0, 0.0, 2.0),
                rotation: Quat::IDENTITY,
                source: SectionSource::Prototype("basic_thruster_section".to_string()),
                modifications: vec![],
            },
            SpaceshipSectionConfig {
                id: "turret".to_string(),
                position: Vec3::new(0.0, 0.0, -2.0),
                rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                source: SectionSource::Prototype("better_turret_section".to_string()),
                modifications: vec![],
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
                .chain([asteroid_scatter])
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
        cubemap,
        events,
    }
}

pub(crate) fn asteroid_next(cubemap: AssetRef<Image>) -> ScenarioConfig {
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
        cubemap,
        events,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The menu backdrop's contract (task 20260711-212504): the orbiter is
    /// an AI ship directed to orbit the planetoid on its own thrusters -
    /// controller + thruster sections aboard, directive pointing at an
    /// object that actually exists in the same scenario and carries an
    /// authored surface gravity (so it gets a well at spawn).
    #[test]
    fn menu_orbiter_is_an_ai_ship_directed_at_the_planetoid() {
        let scenario = menu_ambience(AssetRef::default(), AssetRef::default());

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
        // Sections now reference the catalog by prototype id; match on the id.
        let references = |needle: &str| {
            ship.sections.iter().any(|section| {
                matches!(&section.source, SectionSource::Prototype(id) if id.contains(needle))
            })
        };
        assert!(
            references("controller"),
            "a controller section flies the autopilot's attitude commands"
        );
        assert!(
            references("thruster"),
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
