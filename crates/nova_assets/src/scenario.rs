use bevy::{platform::collections::HashMap, prelude::*};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

pub mod broadside;
pub(crate) mod craft;
pub(crate) mod menu;
pub mod shakedown;

/// The seed of the built-in scenarios' scatter fields. A fixed value: the
/// ports replaced the old per-launch RNG rock loops with a single seeded
/// `ScatterObjects` action each, so the layout is now deterministic content
/// (reproducible across loads) rather than random per launch.
pub(crate) const SCATTER_SEED: u64 = 0x0605_0403_0201_0000;

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
                impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
                destroy_sound: Some(AssetRef::from("self://sounds/explosion.wav")),
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
            impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
            destroy_sound: Some(AssetRef::from("self://sounds/explosion.wav")),
            radius: 20.0,
            texture: asteroid_texture.clone(),
            health: 2000.0,
            surface_gravity: Some(6.0),
            invulnerable: true,
            lock_signature: None,
        }),
    });

    let spaceship = SpaceshipConfig {
        allegiance: None,
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
            lock_refire_secs: None,
        }),
        sections: vec![
            SpaceshipSectionConfig {
                id: "controller".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::Prototype("basic_controller_section".to_string()),
                modifications: vec![SectionModification::DisableVerb(FlightVerb::Rcs)],
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
        allegiance: None,
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
                // The outcome frame's Defeat + lingering retry (retrofit,
                // outcome review R1.8): before it this restart was silent.
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "Your ship is one more rock in the field.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: "asteroid_field".to_string(),
                    linger: true,
                    delay: None,
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
                // Clearing the field is a win; the overlay's Continue rides
                // the lingering loop through asteroid_next (retrofit,
                // outcome review R1.8; re-landed by slice review R1.1 after
                // the first application was lost in a retry).
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Victory,
                    "Zone reached - the field is yours to run again.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: "asteroid_next".to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
    ];

    ScenarioConfig {
        id: "asteroid_field".to_string(),
        name: "Asteroid Field".to_string(),
        description: "A dense asteroid field.".to_string(),
        cubemap,
        // The combat/gravity sandbox, listed in the Scenarios picker. It was
        // hidden as "a mid-story stage reached by chaining from the shakedown
        // run" - a premise that was never true: this was the ORIGINAL New
        // Game scenario until the shakedown replaced it, and nothing but its
        // own asteroid_next relay ever chained here. Unhidden (task
        // 20260721-160842); the player wiki advertises it as a picker
        // sandbox. Placeholder thumbnail, real art is task 20260715-220011.
        thumbnail: Some(AssetRef::from("self://banner.png")),
        events,
        ..Default::default()
    }
}

pub(crate) fn asteroid_next(cubemap: AssetRef<Image>) -> ScenarioConfig {
    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        // Non-lingering cut (task 20260717-201534): this relay carries no
        // Outcome overlay, so a lingering switch would strand the player in
        // an empty scenario until a stray Enter press. linger: false makes it
        // an immediate cut back into the field - one acknowledgement, seamless
        // loop.
        actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
            scenario_id: "asteroid_field".to_string(),
            linger: false,
            delay: None,
        })],
    }];

    ScenarioConfig {
        id: "asteroid_next".to_string(),
        name: "Asteroid Field - Next".to_string(),
        description: "The next scenario after the asteroid field.".to_string(),
        cubemap,
        // A continuation reached only via NextScenario chaining, not an entry point.
        hidden: true,
        events,
        ..Default::default()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The asteroid_next relay is a pure OnStart bridge with no Outcome
    /// overlay, so its switch must be a non-lingering cut (task
    /// 20260717-201534): a lingering switch here would strand the player in
    /// this empty scenario until a stray Enter press. Mirrors the
    /// filter-events shape of shakedown's player_death_routes_back test.
    #[test]
    fn asteroid_next_bridge_is_a_non_lingering_cut() {
        let config = asteroid_next(AssetRef::default());

        let bridges: Vec<&NextScenarioActionConfig> = config
            .events
            .iter()
            .filter(|event| matches!(event.name, EventConfig::OnStart))
            .flat_map(|event| event.actions.iter())
            .filter_map(|action| match action {
                EventActionConfig::NextScenario(next) => Some(next),
                _ => None,
            })
            .collect();

        assert_eq!(bridges.len(), 1, "the relay has exactly one OnStart cut");
        assert_eq!(bridges[0].scenario_id, "asteroid_field");
        assert!(
            !bridges[0].linger,
            "a bare relay with no Outcome overlay must cut immediately, not linger"
        );
    }
}
