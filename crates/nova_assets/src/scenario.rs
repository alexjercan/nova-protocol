use bevy::{platform::collections::HashMap, prelude::*};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

pub mod broadside;
pub(crate) mod craft;
pub mod shakedown;

/// The seed of the built-in scenarios' scatter fields. A fixed value: the
/// ports replaced the old per-launch RNG rock loops with a single seeded
/// `ScatterObjects` action each, so the layout is now deterministic content
/// (reproducible across loads) rather than random per launch.
const SCATTER_SEED: u64 = 0x0605_0403_0201_0000;

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
            allegiance: None,
            controller: SpaceshipController::AI(AIControllerConfig {
                orbit: Some("menu_planetoid".to_string()),
                ..Default::default()
            }),
            // The menu orbiter flies the racer (craft-ships-into-base) - a
            // detailed silhouette drifting the backdrop reads far cooler than the
            // old trainer cube.
            sections: craft::racer_sections(craft::ShipGrade::Player, vec![]),
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
        // The menu backdrop is never a player-facing scenario (hidden from
        // the picker) but IS in the menu's backdrop rotation (menu_backdrop):
        // the menu picks one flagged scenario at random on entry.
        hidden: true,
        menu_backdrop: true,
        events,
        ..Default::default()
    }
}

/// The shared backdrop stage: the camera-framing planetoid every menu
/// backdrop must carry (id `menu_planetoid` - the contract
/// `stage_menu_camera` frames by; see the scenario authoring guide). Same
/// physique as menu_ambience's: nominal 20u, authored 6 u/s^2 surface
/// gravity, invulnerable (the runtime well derives mu/SOI from the
/// GEOMETRIC collider radius, observed ~80-91u across seeds).
fn backdrop_planetoid(asteroid_texture: AssetRef<Image>) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "menu_planetoid".to_string(),
            name: "Menu Planetoid".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
            destroy_sound: Some(AssetRef::from("self://sounds/explosion.wav")),
            radius: 20.0,
            texture: asteroid_texture,
            health: 2000.0,
            surface_gravity: Some(6.0),
            invulnerable: true,
            lock_signature: None,
        }),
    }
}

/// A small AI ship on the orbit directive around the backdrop planetoid -
/// the proven menu actor (the ORBIT autopilot plans its ring from the
/// well's runtime geometry). `extra_hull` adds a mid hull segment for a
/// longer, hauler-ish silhouette. Never `SpaceshipController::Player`: the
/// spaceship input sets are LIVE in MainMenu (see menu_ambience's warning).
fn backdrop_orbiter(
    id: &str,
    name: &str,
    position: Vec3,
    // Kept for call-site intent; every backdrop orbiter is now the same full
    // racer (craft-ships-into-base), so the old extra-hull silhouette knob is a
    // no-op.
    _extra_hull: bool,
) -> ScenarioObjectConfig {
    let sections = craft::racer_sections(craft::ShipGrade::Player, vec![]);
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::AI(AIControllerConfig {
                orbit: Some("menu_planetoid".to_string()),
                ..Default::default()
            }),
            sections,
        }),
    }
}

/// A static dressing beacon (label + warm little light). Below the orbit
/// plane and outside the planetoid's geometric radius, like everything
/// else in a backdrop.
fn backdrop_beacon(id: &str, label: &str, position: Vec3, color: Color) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: label.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: label.to_string(),
            radius: 2.0,
            color,
            area_radius: None,
            lock_signature: None,
        }),
    }
}

/// Menu backdrop two (task 20260716-180352): a freight waystation going
/// about its day. Two named haulers circle the planetoid in convoy
/// (opposite phases on the same autopilot ring, so they chase and never
/// meet), dock beacons glow below the lane, and a flat band of cargo rocks
/// reads as the shipping lane. Same safety envelope as menu_ambience:
/// everything static sits past the geometric radius and below the orbit
/// plane.
pub(crate) fn menu_waystation(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let objects = vec![
        backdrop_planetoid(asteroid_texture.clone()),
        backdrop_orbiter(
            "waystation_hauler_a",
            "Hauler Biscuit",
            Vec3::new(140.0, 0.0, 0.0),
            true,
        ),
        backdrop_orbiter(
            "waystation_hauler_b",
            "Hauler Kettle",
            Vec3::new(-140.0, 0.0, 0.0),
            true,
        ),
        backdrop_beacon(
            "waystation_dock_a",
            "DOCK-A",
            Vec3::new(170.0, -25.0, 60.0),
            Color::srgb(1.0, 0.7, 0.2),
        ),
        backdrop_beacon(
            "waystation_dock_b",
            "DOCK-B",
            Vec3::new(150.0, -30.0, -90.0),
            Color::srgb(1.0, 0.7, 0.2),
        ),
        backdrop_beacon(
            "waystation_traffic",
            "TRAFFIC",
            Vec3::new(-180.0, -20.0, 40.0),
            Color::srgb(0.3, 0.9, 1.0),
        ),
    ];

    // The shipping lane: a flatter, slightly denser band than menu_ambience's
    // ring, same safety floor (inner past any plausible geometric radius,
    // whole band below the orbit plane).
    let lane_scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "waystation_cargo_".to_string(),
        count: 18,
        seed: SCATTER_SEED ^ 0x1,
        region: ScatterRegion::Ring {
            inner: 180.0,
            outer: 230.0,
            y_min: -60.0,
            y_max: -25.0,
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "waystation_cargo_".to_string(),
                name: "Cargo Rock".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
                destroy_sound: Some(AssetRef::from("self://sounds/explosion.wav")),
                radius: 1.0,
                texture: asteroid_texture,
                health: 100.0,
                surface_gravity: None,
                invulnerable: false,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((1.0, 2.5)),
    });

    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: objects
            .into_iter()
            .map(EventActionConfig::SpawnScenarioObject)
            .chain([lane_scatter])
            .collect::<_>(),
    }];

    ScenarioConfig {
        id: "menu_waystation".to_string(),
        name: "Waystation Traffic".to_string(),
        description: "A freight waystation going about its day.".to_string(),
        cubemap,
        hidden: true,
        menu_backdrop: true,
        events,
        ..Default::default()
    }
}

/// Menu backdrop three (task 20260716-180352): a quiet salvage yard. One
/// lazy tug circles the planetoid, salvage crates tumble in a drifting band
/// (crates are on-rails statics with a render-child tumble - zero physics
/// risk), two big wreck rocks anchor the yard, and a single warm beacon
/// marks it. Cozy, not busy.
pub(crate) fn menu_scrapyard(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let objects = vec![
        backdrop_planetoid(asteroid_texture.clone()),
        backdrop_orbiter(
            "scrapyard_tug",
            "Tug Pebble",
            Vec3::new(0.0, 0.0, 140.0),
            false,
        ),
        // Two derelict hulks: plain big rocks at fixed spots, breakable (they
        // are dressing; nothing in the backdrop shoots).
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "scrapyard_wreck_a".to_string(),
                name: "Wreck".to_string(),
                position: Vec3::new(200.0, -18.0, 80.0),
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
                destroy_sound: Some(AssetRef::from("self://sounds/explosion.wav")),
                radius: 6.0,
                texture: asteroid_texture.clone(),
                health: 400.0,
                surface_gravity: None,
                invulnerable: false,
                lock_signature: None,
            }),
        },
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "scrapyard_wreck_b".to_string(),
                name: "Wreck".to_string(),
                position: Vec3::new(-180.0, -30.0, -120.0),
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                impact_sound: Some(AssetRef::from("self://sounds/impact.wav")),
                destroy_sound: Some(AssetRef::from("self://sounds/explosion.wav")),
                radius: 8.0,
                texture: asteroid_texture.clone(),
                health: 400.0,
                surface_gravity: None,
                invulnerable: false,
                lock_signature: None,
            }),
        },
        backdrop_beacon(
            "scrapyard_marker",
            "YARD",
            Vec3::new(165.0, -12.0, -55.0),
            Color::srgb(1.0, 0.55, 0.15),
        ),
    ];

    // The drifting cargo: on-rails salvage crates (visual tumble, no
    // physics), scattered in the same safe band as the rocks would be.
    let crate_scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "scrapyard_crate_".to_string(),
        count: 10,
        seed: SCATTER_SEED ^ 0x2,
        region: ScatterRegion::Ring {
            inner: 150.0,
            outer: 210.0,
            y_min: -50.0,
            y_max: -10.0,
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "scrapyard_crate_".to_string(),
                name: "Drifting Crate".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::SalvageCrate(SalvageCrateConfig {
                size: 2.0,
                area_radius: 3.0,
                pickup_sound: Some(AssetRef::from("self://sounds/salvage_pickup.wav")),
            }),
        },
        asteroid_radius: None,
    });

    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: objects
            .into_iter()
            .map(EventActionConfig::SpawnScenarioObject)
            .chain([crate_scatter])
            .collect::<_>(),
    }];

    ScenarioConfig {
        id: "menu_scrapyard".to_string(),
        name: "Scrapyard Drift".to_string(),
        description: "A quiet salvage yard tumbling in orbit.".to_string(),
        cubemap,
        hidden: true,
        menu_backdrop: true,
        events,
        ..Default::default()
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
        // A mid-story stage reached by chaining from the shakedown run, not a
        // Scenarios-picker entry point.
        hidden: true,
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
        // The orbiter flies the racer now, whose section prototypes are named by
        // cut-cube id, so resolve each ref's KIND against the base catalog.
        let catalog =
            crate::sections::build_sections(&crate::sections::SectionMeshRefs::from_paths());
        let has_kind = |want: fn(&SectionKind) -> bool| {
            ship.sections.iter().any(|section| match &section.source {
                SectionSource::Prototype(id) => catalog
                    .iter()
                    .find(|c| c.base.id == *id)
                    .is_some_and(|c| want(&c.kind)),
                SectionSource::Inline(c) => want(&c.kind),
            })
        };
        assert!(
            has_kind(|k| matches!(k, SectionKind::Controller(_))),
            "a controller section flies the autopilot's attitude commands"
        );
        assert!(
            has_kind(|k| matches!(k, SectionKind::Thruster(_))),
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
