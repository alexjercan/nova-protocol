//! The scenario the editor hands off to on Play: an open range for the ship
//! you just built.
//!
//! It is a SANDBOX, so it deliberately has no win: one standing objective that
//! never completes, no chapter to chain into, and the only outcome is your own
//! death - which offers a retry of this same range. F1 is the way out.
//!
//! What is out there: two seeded rock belts, a corridor of inert target hulks
//! to shoot, three DORMANT pickets that only fight once you paint or crowd
//! them, two beacons that swap the sky, and one planetoid parked far enough
//! away to be scenery rather than a wall (see [`PLANETOID_POSITION`]).

use bevy::prelude::*;
use nova_assets::prelude::*;
use nova_gameplay::prelude::{Allegiance, AssetRef};
use nova_scenario::prelude::*;

use crate::config::PlayerSpaceshipConfig;

/// The sandbox's scenario id. Registered in [`GameScenarios`] on hand-off so
/// the DEFEAT overlay's Retry can reload it by id like any other scenario;
/// `hidden` keeps it out of the Scenarios picker, which lists shipped content.
const SANDBOX_ID: &str = "editor_sandbox";
/// The player ship's scenario id, referenced by every handler that scopes to
/// the player and by the editor's input mapping.
const PLAYER_ID: &str = "player_spaceship";

/// Where the ship you built starts. Everything else is laid out around this,
/// and the layout tests measure from it.
const PLAYER_SPAWN: Vec3 = Vec3::ZERO;

/// The planetoid: far enough that it reads as a destination.
///
/// It used to sit 314u from the spawn, which put the player INSIDE it in every
/// sense that matters: an asteroid's real surface is `radius *` 3.5-6.0 (the
/// noise mesh displaces outward - see `ASTEROID_GEOMETRIC_FACTOR_MAX`), so a
/// 55u planetoid was a ~250u ball of rock, and its well reached
/// `sqrt(mu / soi_cutoff_accel)` ~ 1095u. You spawned ~60u off its surface and
/// fell in. Both numbers are derived from authored data, so
/// `the_spawn_is_clear_of_the_planetoid` recomputes them rather than trusting
/// this comment.
const PLANETOID_POSITION: Vec3 = Vec3::new(-560.0, -110.0, -380.0);
/// Nominal planetoid radius; the drawn body is 3.5-6.0x this.
const PLANETOID_RADIUS: f32 = 24.0;
/// Planetoid mass parameter, authored by the REACH it buys:
/// `mu = soi_cutoff_accel * soi^2` at the shipped 0.25 cutoff gives a 400u
/// sphere of influence - a well the player can go looking for, and cannot fall
/// into by accident from the spawn.
const PLANETOID_MASS: f32 = 40_000.0;
/// Pinned silhouette. An unseeded rock redraws itself every load, and this
/// body is the one thing here whose real radius the layout is measured
/// against.
const PLANETOID_SEED: u32 = 20_260_815;

/// The inert hulks, port-forward of the spawn: a corridor of things to shoot
/// that shoot back at nobody. Clear of both belt boxes by construction.
const HULK_POSITIONS: [Vec3; 5] = [
    Vec3::new(-60.0, 20.0, -170.0),
    Vec3::new(-140.0, -25.0, -240.0),
    Vec3::new(-70.0, 50.0, -300.0),
    Vec3::new(-200.0, 15.0, -360.0),
    Vec3::new(-95.0, -55.0, -430.0),
];

/// One dormant picket: where it sits, how far its proximity trip reaches, and
/// the callsign it answers on when it wakes.
struct Picket {
    id: &'static str,
    name: &'static str,
    callsign: &'static str,
    position: Vec3,
    /// Radius of the sphere that wakes it when the player flies in.
    trip_radius: f32,
}

/// The three pickets. They spawn NEUTRAL with a live AI pilot, which is what
/// makes them dormant rather than scripted-asleep: the AI's target acquisition
/// only considers `Relation::Hostile` contacts, and a neutral ship has none.
/// Flipping the allegiance is the whole wake-up.
const PICKETS: [Picket; 3] = [
    Picket {
        id: "picket_warden",
        name: "Picket Warden",
        callsign: "Warden",
        position: Vec3::new(520.0, -45.0, -290.0),
        trip_radius: 150.0,
    },
    Picket {
        id: "picket_sentinel",
        name: "Picket Sentinel",
        callsign: "Sentinel",
        position: Vec3::new(-300.0, 70.0, -680.0),
        trip_radius: 150.0,
    },
    Picket {
        id: "picket_lance",
        name: "Picket Lance",
        callsign: "Lance",
        position: Vec3::new(-20.0, 130.0, -740.0),
        trip_radius: 170.0,
    },
];

/// One rock belt: a seeded box of asteroids.
struct Belt {
    id_prefix: &'static str,
    name: &'static str,
    seed: u64,
    count: u32,
    min: Vec3,
    max: Vec3,
    radius: (f32, f32),
    /// Centre-to-centre clearance. Sized on the widest two rocks side by side
    /// (`radius.1 * ASTEROID_GEOMETRIC_FACTOR_MAX`, doubled): overlapping
    /// dynamic bodies are shoved apart on the first physics step hard enough
    /// to damage each other. `ScatterObjects` measures against every body
    /// scattered so far, this scenario's earlier belts included, so the two
    /// boxes may abut.
    separation: f32,
}

/// The belts, as boxes rather than rings so the hand-placed hulks, pickets and
/// beacons can be kept out of them by inspection - which
/// `hand_placed_bodies_stay_out_of_the_belts` then checks.
const BELTS: [Belt; 2] = [
    Belt {
        id_prefix: "shallow_rock_",
        name: "Shallow Rock",
        seed: 0x5A11_0B37,
        count: 30,
        min: Vec3::new(70.0, -50.0, -440.0),
        max: Vec3::new(430.0, 50.0, -40.0),
        radius: (1.0, 3.0),
        separation: 45.0,
    },
    Belt {
        id_prefix: "deep_rock_",
        name: "Deep Rock",
        seed: 0xDEE9_0C11,
        count: 34,
        min: Vec3::new(-280.0, -60.0, -840.0),
        max: Vec3::new(300.0, 60.0, -540.0),
        radius: (1.5, 4.0),
        separation: 55.0,
    },
];

/// One sky beacon: fly through it and the cubemap changes.
struct SkyBeacon {
    id: &'static str,
    name: &'static str,
    label: &'static str,
    position: Vec3,
    trip_radius: f32,
    color: Color,
    /// The cubemap entering it installs.
    cubemap: &'static str,
    /// The comms line it answers with.
    line: &'static str,
}

/// The two beacons, one per shipped cubemap. Both are DIRECT asset paths for
/// the same reason the sounds below are: this scenario is built at runtime,
/// outside the mod merge, so a `self://` or `dep://` scheme ref would never be
/// rewritten.
const SKY_BEACONS: [SkyBeacon; 2] = [
    SkyBeacon {
        id: "beacon_veil",
        name: "Veil Beacon",
        label: "VEIL",
        position: Vec3::new(-430.0, 140.0, -690.0),
        trip_radius: 130.0,
        color: Color::srgb(0.70, 0.35, 1.0),
        cubemap: "base/textures/cubemap_alt.png",
        line: "Veil relay - you are under the other sky now.",
    },
    SkyBeacon {
        id: "beacon_home",
        name: "Home Beacon",
        label: "HOME",
        position: Vec3::new(40.0, 0.0, 260.0),
        trip_radius: 70.0,
        color: Color::srgb(0.20, 0.90, 1.0),
        cubemap: "base/textures/cubemap.png",
        line: "Home relay - the old sky is back.",
    },
];

/// Lock range a beacon needs to be designatable from the spawn: the radar
/// gives a signed body `signature_range_per_unit` (30) x this. 30 buys 900u,
/// which covers the deepest beacon.
const BEACON_LOCK_SIGNATURE: f32 = 30.0;

pub(crate) fn setup_scenario(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    player_config: Res<PlayerSpaceshipConfig>,
    // Optional: the registry is written by the bundle merge, and a rig that
    // never merged content must still be able to fly the sandbox - it just
    // does not get the retry.
    scenarios: Option<ResMut<GameScenarios>>,
) {
    let scenario = sandbox_scenario(&game_assets, &player_config);

    // Register before loading: the DEFEAT overlay's Retry releases a queued
    // `NextScenario`, which resolves the id against this registry. Without the
    // entry the retry logs "not found" and unloads.
    if let Some(mut scenarios) = scenarios {
        scenarios.insert(scenario.id.clone(), scenario.clone());
    }

    commands.trigger(LoadScenario(scenario));
}

/// Build the sandbox: two rock belts, a hulk corridor, three dormant pickets,
/// two sky beacons, one distant planetoid, and the ship the editor just built.
pub(crate) fn sandbox_scenario(
    game_assets: &GameAssets,
    player_config: &PlayerSpaceshipConfig,
) -> ScenarioConfig {
    let asteroid_texture = game_assets.asteroid_texture.clone();
    let objects = sandbox_objects(player_config, asteroid_texture.clone());

    ScenarioConfig {
        description: "A free-flight range: rocks, target hulks, dormant pickets and a planetoid."
            .to_string(),
        // Registered so Retry can find it (see `setup_scenario`), hidden so it
        // never shows up in the Scenarios picker next to shipped content.
        hidden: true,
        events: sandbox_events(objects, asteroid_texture),
        ..ScenarioConfig::new(
            SANDBOX_ID.to_string(),
            "Editor Sandbox".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The hand-placed objects: the planetoid, the hulk corridor, the pickets, the
/// beacons and the player. The rock belts are NOT here - they are seeded
/// scatter actions (see [`belt_scatter`]), so the field is the same field on
/// every load and on every retry.
fn sandbox_objects(
    player_config: &PlayerSpaceshipConfig,
    asteroid_texture: Handle<Image>,
) -> Vec<ScenarioObjectConfig> {
    let mut objects = vec![planetoid(asteroid_texture)];

    objects.extend(
        HULK_POSITIONS
            .iter()
            .enumerate()
            .map(|(index, position)| target_hulk(index, *position)),
    );
    objects.extend(PICKETS.iter().map(picket_ship));
    objects.extend(SKY_BEACONS.iter().map(sky_beacon));
    objects.push(player_ship(player_config));

    objects
}

/// The planetoid: a large, invulnerable, PINNED rock with an explicit mass, so
/// it reads as a proper well and as scenery rather than a target.
fn planetoid(asteroid_texture: Handle<Image>) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "planetoid".to_string(),
            name: "Planetoid".to_string(),
            position: PLANETOID_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            // DIRECT paths, not dep://: the editor sandbox is built at
            // runtime outside the mod merge (its texture is a raw
            // GameAssets handle), so scheme refs would never rewrite.
            impact_sound: Some(AssetRef::from("base/sounds/impact.wav")),
            destroy_sound: Some(AssetRef::from("base/sounds/explosion.wav")),
            radius: PLANETOID_RADIUS,
            texture: AssetRef::from(asteroid_texture),
            health: 100.0,
            mass: Some(PLANETOID_MASS),
            invulnerable: true,
            seed: Some(PLANETOID_SEED),
            lock_signature: None,
        }),
    }
}

/// One inert target: a ship-shaped cross of bare hull, no controller, no
/// pilot, no allegiance. It has never carried a weapon section, so the
/// integrity layer never NEUTRALIZES it either - it is a silhouette that takes
/// damage and comes apart, and nothing else.
fn target_hulk(index: usize, position: Vec3) -> ScenarioObjectConfig {
    let hull = |id: &str, offset: Vec3, prototype: &str| SpaceshipSectionConfig {
        id: id.to_string(),
        position: offset,
        rotation: Quat::IDENTITY,
        source: SectionSource::Prototype(prototype.to_string()),
        modifications: vec![],
    };

    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: format!("hulk_{index}"),
            name: format!("Derelict Hulk {index}"),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            hull: ShipSource::Inline(ShipHull {
                sections: vec![
                    hull("spine", Vec3::ZERO, "reinforced_hull_section"),
                    hull("bow", Vec3::new(0.0, 0.0, -1.0), "light_hull_section"),
                    hull("stern", Vec3::new(0.0, 0.0, 1.0), "light_hull_section"),
                    hull("port", Vec3::new(-1.0, 0.0, 0.0), "light_hull_section"),
                    hull("starboard", Vec3::new(1.0, 0.0, 0.0), "light_hull_section"),
                ],
                ..default()
            }),
            ..default()
        }),
    }
}

/// One dormant picket: a real armed corvette under AI, spawned NEUTRAL.
///
/// Neutral is the dormancy. The AI runs its passive routine (here: station
/// keeping, no patrol) and never acquires, because acquisition only looks at
/// hostile contacts. `wake_picket` flips the allegiance and the same pilot
/// starts fighting - no controller swap, no second spawn.
fn picket_ship(picket: &Picket) -> ScenarioObjectConfig {
    let section =
        |id: &str, offset: Vec3, rotation: Quat, prototype: &str| SpaceshipSectionConfig {
            id: id.to_string(),
            position: offset,
            rotation,
            source: SectionSource::Prototype(prototype.to_string()),
            modifications: vec![],
        };

    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: picket.id.to_string(),
            name: picket.name.to_string(),
            position: picket.position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Neutral),
            controller: SpaceshipController::AI(AIControllerConfig {
                // Territorial: a woken picket fights over its own patch and
                // gives up if the player runs. Otherwise waking one drags a
                // pursuer across the whole range.
                leash: Some(400.0),
                // It wakes hot - the wake IS the warning, and the trip sphere
                // gives the player a beat before the guns bear.
                ..default()
            }),
            hull: ShipSource::Inline(ShipHull {
                sections: vec![
                    section(
                        "controller",
                        Vec3::ZERO,
                        Quat::IDENTITY,
                        "basic_controller_section",
                    ),
                    section(
                        "hull_front",
                        Vec3::new(0.0, 0.0, 1.0),
                        Quat::IDENTITY,
                        "reinforced_hull_section",
                    ),
                    section(
                        "hull_back",
                        Vec3::new(0.0, 0.0, -1.0),
                        Quat::IDENTITY,
                        "reinforced_hull_section",
                    ),
                    section(
                        "thruster",
                        Vec3::new(0.0, 0.0, 2.0),
                        Quat::IDENTITY,
                        "basic_thruster_section",
                    ),
                    section(
                        "turret",
                        Vec3::new(0.0, 0.0, -2.0),
                        Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                        "better_turret_section",
                    ),
                ],
                ..default()
            }),
            ..default()
        }),
    }
}

/// One sky beacon: a lockable nav orb that is its own trigger sphere.
fn sky_beacon(beacon: &SkyBeacon) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: beacon.id.to_string(),
            name: beacon.name.to_string(),
            position: beacon.position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: beacon.label.to_string(),
            radius: 3.0,
            color: beacon.color,
            area_radius: Some(beacon.trip_radius),
            lock_signature: Some(BEACON_LOCK_SIGNATURE),
        }),
    }
}

/// The ship the editor just built, with the keybinds it was built with.
fn player_ship(player_config: &PlayerSpaceshipConfig) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: "Player's Spaceship".to_string(),
            position: PLAYER_SPAWN,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: player_config
                    .inputs
                    .iter()
                    .map(|(entity, key)| (entity.to_string(), key.clone()))
                    .collect(),

                speed_cap: None,
                // The editor sandbox keeps normal finite magazines. Safe even
                // on a range built for shooting: weapons auto-reload, so a dry
                // gun is a cadence beat rather than a permanent disarm.
                infinite_ammo: false,
            }),
            // What the builder saw is what they fly. The editor shows the same
            // derived skin over the same structure, so the flown ship must not
            // come up bare (or clad) against it.
            hull: ShipSource::Inline(ShipHull {
                sections: player_config.sections.values().cloned().collect(),
                skin: player_config.skin,
                style: player_config.style.clone(),
                ..default()
            }),
            ..default()
        }),
    }
}

/// One belt as a seeded scatter action.
fn belt_scatter(belt: &Belt, asteroid_texture: Handle<Image>) -> EventActionConfig {
    EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: belt.id_prefix.to_string(),
        count: belt.count,
        seed: belt.seed,
        region: ScatterRegion::Box {
            min: belt.min,
            max: belt.max,
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: belt.id_prefix.to_string(),
                name: belt.name.to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                // DIRECT paths, not dep:// - see `planetoid`.
                impact_sound: Some(AssetRef::from("base/sounds/impact.wav")),
                destroy_sound: Some(AssetRef::from("base/sounds/explosion.wav")),
                radius: belt.radius.0,
                texture: AssetRef::from(asteroid_texture),
                health: 100.0,
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some(belt.radius),
        min_separation: Some(belt.separation),
    })
}

/// A boolean expression node, for the once-only wake guards.
fn boolean(value: bool) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(
        VariableFactorNode::new_literal(VariableLiteral::Boolean(value)),
    ))
}

/// A named-variable expression node.
fn variable(name: &str) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(VariableFactorNode::new_name(
        name.to_string(),
    )))
}

/// The scenario variable that remembers a picket has already woken, so the
/// comms line fires once rather than on every re-entry of its trip sphere.
fn woke_key(picket: &Picket) -> String {
    format!("{}_awake", picket.id)
}

/// The two ways a picket wakes: the player PAINTS it (combat lock), or the
/// player CROWDS it (flies into its trip sphere). Same actions either way.
fn wake_picket(picket: &Picket) -> Vec<ScenarioEventConfig> {
    let key = woke_key(picket);
    let still_asleep = EventFilterConfig::Expression(ExpressionFilterConfig(
        VariableConditionNode::new_equals(variable(&key), boolean(false)),
    ));
    let wake = vec![
        EventActionConfig::VariableSet(VariableSetActionConfig {
            key: key.clone(),
            expression: boolean(true),
        }),
        EventActionConfig::SetAllegiance(SetAllegianceActionConfig {
            id: picket.id.to_string(),
            allegiance: Allegiance::Enemy,
        }),
        EventActionConfig::StoryMessage(StoryMessageActionConfig {
            speaker: picket.callsign.to_string(),
            text: "Contact acknowledged. Weapons free.".to_string(),
            dwell: None,
            icon: None,
        }),
    ];

    vec![
        // Painted: the lock event's primary entity is the TARGET, the other is
        // the ship that locked it.
        ScenarioEventConfig {
            name: EventConfig::OnCombatLockStart,
            filters: vec![
                EventFilterConfig::Entity(EntityFilterConfig {
                    id: Some(picket.id.to_string()),
                    other_id: Some(PLAYER_ID.to_string()),
                    ..default()
                }),
                still_asleep.clone(),
            ],
            actions: wake.clone(),
        },
        // Crowded: the trip sphere's own id is the primary entity.
        ScenarioEventConfig {
            name: EventConfig::OnEnter,
            filters: vec![
                EventFilterConfig::Entity(EntityFilterConfig {
                    id: Some(trip_area_id(picket)),
                    other_id: Some(PLAYER_ID.to_string()),
                    ..default()
                }),
                still_asleep,
            ],
            actions: wake,
        },
    ]
}

/// The scenario area id of a picket's proximity trip.
fn trip_area_id(picket: &Picket) -> String {
    format!("{}_trip", picket.id)
}

/// The proximity trip sphere around one picket.
fn trip_area(picket: &Picket) -> EventActionConfig {
    EventActionConfig::CreateScenarioArea(ScenarioAreaConfig {
        id: trip_area_id(picket),
        name: format!("{} Trip", picket.name),
        position: picket.position,
        rotation: Quat::IDENTITY,
        radius: picket.trip_radius,
    })
}

/// Flying through a beacon swaps the sky and says so. Deliberately UNGUARDED,
/// unlike the picket wakes: re-entering a beacon is meant to swap the sky
/// again, which is the whole point of having two.
fn beacon_swaps_the_sky(beacon: &SkyBeacon) -> ScenarioEventConfig {
    ScenarioEventConfig {
        name: EventConfig::OnEnter,
        filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
            id: Some(beacon.id.to_string()),
            other_id: Some(PLAYER_ID.to_string()),
            ..default()
        })],
        actions: vec![
            EventActionConfig::SetSkybox(SetSkyboxActionConfig::new(beacon.cubemap)),
            EventActionConfig::StoryMessage(StoryMessageActionConfig {
                speaker: beacon.label.to_string(),
                text: beacon.line.to_string(),
                dwell: None,
                icon: None,
            }),
        ],
    }
}

/// The scenario events: spawn the range on start, wake the pickets on lock or
/// proximity, swap the sky at the beacons, and offer a retry when the player
/// dies. No completion path: the standing objective is never completed and
/// nothing chains anywhere but back here.
fn sandbox_events(
    objects: Vec<ScenarioObjectConfig>,
    asteroid_texture: Handle<Image>,
) -> Vec<ScenarioEventConfig> {
    // The one outcome the sandbox has, and the retry it queues. `linger` holds
    // the switch until the overlay's Retry (or Enter) releases it.
    let retry = |message: &str| {
        vec![
            EventActionConfig::Outcome(OutcomeActionConfig::new(
                ScenarioOutcomeKind::Defeat,
                message,
            )),
            EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: SANDBOX_ID.to_string(),
                linger: true,
                delay: None,
            }),
        ]
    };
    let player = |id: &str| {
        vec![EventFilterConfig::Entity(EntityFilterConfig {
            id: Some(id.to_string()),
            type_name: None,
            ..default()
        })]
    };

    let mut events = vec![
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            // The sandbox lights itself: the engine spawns no light, so a
            // scenario that authors none renders black. (The editor VIEW has
            // its own light - `ui/mod.rs` - which is a different surface.)
            actions: objects
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .chain(
                    BELTS
                        .iter()
                        .map(|belt| belt_scatter(belt, asteroid_texture.clone())),
                )
                .chain(PICKETS.iter().map(trip_area))
                // Every picket starts asleep, and its guard variable has to
                // exist before the first filter reads it.
                .chain(PICKETS.iter().map(|picket| {
                    EventActionConfig::VariableSet(VariableSetActionConfig {
                        key: woke_key(picket),
                        expression: boolean(false),
                    })
                }))
                .chain(ThreePointRig::around("sandbox", Vec3::ZERO, 10.0).actions())
                .collect::<_>(),
        },
        // The standing objective. It is never completed - there is nothing to
        // complete - so it stays on the HUD as the sandbox's only instruction.
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: vec![
                EventActionConfig::Objective(ObjectiveActionConfig::new(
                    "sandbox_free_flight",
                    "Free flight: press F1 to return to the editor",
                )),
                EventActionConfig::StoryMessage(StoryMessageActionConfig {
                    speaker: "Range Control".to_string(),
                    text: "Range is yours. Hulks to port, live pickets deeper in - they wake if \
                           you paint them or crowd them. F1 puts you back on the build deck."
                        .to_string(),
                    dwell: None,
                    icon: None,
                }),
            ],
        },
        // Death is the only outcome, and it offers this same range again.
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: player(PLAYER_ID),
            actions: [EventActionConfig::DebugMessage(DebugMessageActionConfig {
                message: "The player's spaceship was destroyed!".to_string(),
            })]
            .into_iter()
            .chain(retry(
                "Range is quiet. Your ship is part of the scenery now.",
            ))
            .collect(),
        },
        // Shot to pieces without dying: the same offer. A ship that never
        // carried a weapon cannot reach this, so an unarmed build is not
        // instantly "neutralized" off the line.
        ScenarioEventConfig {
            name: EventConfig::OnNeutralized,
            filters: player(PLAYER_ID),
            actions: retry("Nothing left to fight with - you drift the range dead."),
        },
    ];

    events.extend(PICKETS.iter().flat_map(wake_picket));
    events.extend(SKY_BEACONS.iter().map(beacon_swaps_the_sky));
    events
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::{GravitySettings, GravityWell};

    use super::*;

    /// The camera's far plane (bevy's `PerspectiveProjection` default, which
    /// nothing in the game overrides). A body authored past this is simply not
    /// drawn until the player closes on it, so the whole range stays inside it.
    const CAMERA_FAR: f32 = 1000.0;

    fn objects() -> Vec<ScenarioObjectConfig> {
        sandbox_objects(&PlayerSpaceshipConfig::default(), Handle::default())
    }

    fn events() -> Vec<ScenarioEventConfig> {
        sandbox_events(objects(), Handle::default())
    }

    fn find(objects: &[ScenarioObjectConfig], id: &str) -> ScenarioObjectConfig {
        objects
            .iter()
            .find(|object| object.base.id == id)
            .unwrap_or_else(|| panic!("the sandbox spawns '{id}'"))
            .clone()
    }

    /// The planetoid's WORST-CASE drawn radius and its well, recomputed from
    /// the authored numbers exactly as the engine derives them.
    fn planetoid_reach() -> (f32, GravityWell) {
        let settings = GravitySettings::default();
        let body_radius = PLANETOID_RADIUS * ASTEROID_GEOMETRIC_FACTOR_MAX;
        (
            body_radius,
            GravityWell::from_mass(PLANETOID_MASS, body_radius, &settings),
        )
    }

    /// The bug this layout exists to fix: the player used to spawn inside the
    /// planetoid's sphere of influence AND within ~60u of its real surface,
    /// and simply fell into it. Both bounds are derived here rather than
    /// asserted as distances, so retuning the mass or the radius re-checks the
    /// spawn instead of silently re-breaking it.
    #[test]
    fn the_spawn_is_clear_of_the_planetoid() {
        let (body_radius, well) = planetoid_reach();
        let distance = PLAYER_SPAWN.distance(PLANETOID_POSITION);

        assert!(
            distance > well.soi_radius,
            "the spawn ({distance:.0}u out) must sit outside the planetoid's \
             {:.0}u sphere of influence - inside it the ship falls in on its own",
            well.soi_radius
        );
        // And with room to spare, not one unit outside: the SOI edge is where
        // the pull starts, not where the trouble starts.
        assert!(
            distance > well.soi_radius + 200.0,
            "the spawn wants clear air past the SOI edge, not a hairline"
        );
        assert!(
            distance > body_radius * 2.0,
            "the drawn planetoid reaches {body_radius:.0}u; the spawn must not \
             be parked against it"
        );
    }

    /// Nothing is authored past the camera's far plane, measured from the
    /// spawn: a body beyond it is simply not drawn, so a "landmark" out there
    /// is an invisible one.
    #[test]
    fn the_whole_range_is_inside_the_camera_far_plane() {
        for object in objects() {
            let distance = PLAYER_SPAWN.distance(object.base.position);
            assert!(
                distance < CAMERA_FAR,
                "'{}' is {distance:.0}u from the spawn, past the {CAMERA_FAR:.0}u far plane",
                object.base.id
            );
        }
        // The belts are scattered, so check their corners instead.
        for belt in &BELTS {
            for corner in [belt.min, belt.max] {
                let distance = PLAYER_SPAWN.distance(corner);
                assert!(
                    distance < CAMERA_FAR,
                    "belt '{}' reaches {distance:.0}u, past the far plane",
                    belt.id_prefix
                );
            }
        }
    }

    /// Hand-placed bodies stay OUT of the scatter boxes. `ScatterObjects`
    /// separates its rocks from each other and from earlier belts, but it
    /// knows nothing about the hulks, pickets and beacons - and two
    /// overlapping dynamic bodies are shoved apart on the first physics step
    /// hard enough to damage each other, so a hulk dropped in a belt would
    /// come apart as the scenario loads.
    #[test]
    fn hand_placed_bodies_stay_out_of_the_belts() {
        for object in objects() {
            // The player is at the origin, clear of both boxes; the planetoid
            // is checked by size below.
            for belt in &BELTS {
                let position = object.base.position;
                let inside = (belt.min.cmple(position) & belt.max.cmpge(position)).all();
                assert!(
                    !inside,
                    "'{}' sits inside belt '{}' and would be shoved apart at spawn",
                    object.base.id, belt.id_prefix
                );
            }
        }

        // The planetoid is not a point: its drawn body must clear the boxes
        // too, or rocks spawn inside the rock.
        let (body_radius, _) = planetoid_reach();
        for belt in &BELTS {
            let nearest = PLANETOID_POSITION.clamp(belt.min, belt.max);
            let gap = PLANETOID_POSITION.distance(nearest);
            assert!(
                gap > body_radius,
                "belt '{}' comes within {gap:.0}u of the planetoid's centre, \
                 inside its {body_radius:.0}u body",
                belt.id_prefix
            );
        }
    }

    /// Every rock belt keeps its copies apart, sized on the WIDEST pair of
    /// rocks it can draw. Without this a belt explodes as it spawns.
    #[test]
    fn every_belt_separates_its_widest_rocks() {
        for belt in &BELTS {
            let widest = belt.radius.1 * ASTEROID_GEOMETRIC_FACTOR_MAX;
            assert!(
                belt.separation >= widest * 2.0,
                "belt '{}' separates by {}u but can draw two {widest:.0}u rocks",
                belt.id_prefix,
                belt.separation
            );
        }
    }

    /// The targets are inert: bare hull, nobody driving, no side. A weapon
    /// section on one of these would make it a combatant that the integrity
    /// layer can neutralize, and a controller would make it fly away.
    #[test]
    fn the_target_hulks_are_unarmed_and_unpiloted() {
        let objects = objects();
        let hulks: Vec<&ScenarioObjectConfig> = objects
            .iter()
            .filter(|object| object.base.id.starts_with("hulk_"))
            .collect();
        assert_eq!(hulks.len(), HULK_POSITIONS.len(), "every hulk spawns");

        for hulk in hulks {
            let ScenarioObjectKind::Spaceship(ship) = &hulk.kind else {
                panic!("'{}' should be a spaceship", hulk.base.id);
            };
            assert!(
                matches!(ship.controller, SpaceshipController::None),
                "'{}' must have nobody driving it",
                hulk.base.id
            );
            assert_eq!(ship.allegiance, None, "'{}' takes no side", hulk.base.id);
            let ShipSource::Inline(hull) = &ship.hull else {
                panic!("'{}' authors its hull inline", hulk.base.id);
            };
            assert!(
                hull.sections.iter().all(|section| matches!(
                    &section.source,
                    SectionSource::Prototype(id) if id.contains("hull")
                )),
                "'{}' is hull only",
                hulk.base.id
            );
        }
    }

    /// A picket is dormant because it is NEUTRAL, not because it is asleep: it
    /// spawns with a live AI pilot, and the AI finds nothing to shoot while it
    /// takes no side. Both wake routes must then flip it to Enemy - one for
    /// the lock, one for the trip sphere - or half the promise is missing.
    #[test]
    fn every_picket_spawns_neutral_and_wakes_two_ways() {
        let objects = objects();
        let events = events();

        for picket in &PICKETS {
            let ScenarioObjectKind::Spaceship(ship) = &find(&objects, picket.id).kind else {
                panic!("'{}' should be a spaceship", picket.id);
            };
            assert_eq!(
                ship.allegiance,
                Some(Allegiance::Neutral),
                "'{}' must spawn neutral - that IS its dormancy",
                picket.id
            );
            assert!(
                matches!(ship.controller, SpaceshipController::AI(_)),
                "'{}' carries a live AI pilot while dormant",
                picket.id
            );
            let ShipSource::Inline(hull) = &ship.hull else {
                panic!("'{}' authors its hull inline", picket.id);
            };
            assert!(
                hull.sections.iter().any(|section| matches!(
                    &section.source,
                    SectionSource::Prototype(id) if id.contains("turret")
                )),
                "'{}' has something to fight with once woken",
                picket.id
            );

            let wakes: Vec<&ScenarioEventConfig> = events
                .iter()
                .filter(|event| {
                    event.actions.iter().any(|action| {
                        matches!(
                            action,
                            EventActionConfig::SetAllegiance(set)
                                if set.id == picket.id && set.allegiance == Allegiance::Enemy
                        )
                    })
                })
                .collect();
            // EventConfig has no PartialEq, so match the two shapes by hand.
            let names: Vec<&EventConfig> = wakes.iter().map(|event| &event.name).collect();
            assert!(
                names
                    .iter()
                    .any(|name| matches!(name, EventConfig::OnCombatLockStart)),
                "'{}' wakes when the player paints it: {names:?}",
                picket.id
            );
            assert!(
                names
                    .iter()
                    .any(|name| matches!(name, EventConfig::OnEnter)),
                "'{}' wakes when the player crowds it: {names:?}",
                picket.id
            );
        }

        // Each trip sphere is actually created, or the OnEnter half never fires.
        let areas: Vec<String> = events
            .iter()
            .flat_map(|event| &event.actions)
            .filter_map(|action| match action {
                EventActionConfig::CreateScenarioArea(area) => Some(area.id.clone()),
                _ => None,
            })
            .collect();
        for picket in &PICKETS {
            assert!(
                areas.contains(&trip_area_id(picket)),
                "'{}' has no trip sphere: {areas:?}",
                picket.id
            );
        }
    }

    /// Every beacon swaps the sky when the player flies through it, and the
    /// two of them cover both shipped cubemaps - one out, one back.
    #[test]
    fn the_beacons_swap_the_sky_both_ways() {
        let events = events();
        let swaps: Vec<String> = events
            .iter()
            .filter(|event| matches!(event.name, EventConfig::OnEnter))
            .flat_map(|event| &event.actions)
            .filter_map(|action| match action {
                EventActionConfig::SetSkybox(skybox) => skybox.cubemap.path().map(str::to_string),
                _ => None,
            })
            .collect();

        for beacon in &SKY_BEACONS {
            assert!(
                swaps.iter().any(|path| path == beacon.cubemap),
                "'{}' installs {}: {swaps:?}",
                beacon.id,
                beacon.cubemap
            );
        }
        assert_eq!(
            swaps.len(),
            SKY_BEACONS.len(),
            "one swap per beacon, no strays"
        );
    }

    /// The sandbox never ends. Nothing completes an objective, nothing
    /// declares a Victory, and the only scenario it ever chains to is itself -
    /// the retry the DEFEAT overlay offers.
    #[test]
    fn the_sandbox_has_no_ending_but_death_offers_a_retry() {
        let events = events();
        let actions: Vec<&EventActionConfig> =
            events.iter().flat_map(|event| &event.actions).collect();

        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, EventActionConfig::ObjectiveComplete(_))),
            "nothing in a sandbox is ever complete"
        );
        assert!(
            actions.iter().any(|action| matches!(
                action,
                EventActionConfig::Objective(objective) if objective.message.contains("F1")
            )),
            "the standing objective names the way out"
        );

        let outcomes: Vec<ScenarioOutcomeKind> = actions
            .iter()
            .filter_map(|action| match action {
                EventActionConfig::Outcome(outcome) => Some(outcome.outcome),
                _ => None,
            })
            .collect();
        assert!(
            !outcomes.is_empty() && outcomes.iter().all(|k| *k == ScenarioOutcomeKind::Defeat),
            "death is the only outcome the range declares: {outcomes:?}"
        );

        let chains: Vec<&NextScenarioActionConfig> = actions
            .iter()
            .filter_map(|action| match action {
                EventActionConfig::NextScenario(next) => Some(next),
                _ => None,
            })
            .collect();
        assert!(!chains.is_empty(), "death queues a retry");
        for next in chains {
            assert_eq!(
                next.scenario_id, SANDBOX_ID,
                "the range only ever chains back to itself"
            );
            assert!(
                next.linger,
                "the retry waits for the player's Retry, it does not cut"
            );
        }
    }

    /// The retry is only reachable because the sandbox registers itself: the
    /// switch resolves the queued id against `GameScenarios`, and this
    /// scenario is built at runtime rather than merged from content. It is
    /// hidden so that registration cannot leak it into the picker.
    #[test]
    fn the_sandbox_registers_itself_hidden_so_the_retry_resolves() {
        let scenario = ScenarioConfig {
            hidden: true,
            events: vec![],
            ..ScenarioConfig::new(
                SANDBOX_ID.to_string(),
                "Editor Sandbox".to_string(),
                AssetRef::from("base/textures/cubemap.png"),
            )
        };
        let mut scenarios = GameScenarios::default();
        scenarios.insert(scenario.id.clone(), scenario);

        let registered = scenarios
            .get(SANDBOX_ID)
            .expect("the retry's id resolves against the registry");
        assert!(
            registered.hidden,
            "a runtime scenario in the registry must stay out of the picker"
        );
        assert!(
            !registered.menu_backdrop,
            "and out of the menu's backdrop rotation"
        );
    }

    /// What you see in the editor is what you fly: the cladding toggle rides
    /// the hand-off, so a ship built clad does not come up bare.
    #[test]
    fn the_cladding_toggle_reaches_the_flown_ship() {
        for clad in [false, true] {
            let config = PlayerSpaceshipConfig {
                skin: clad,
                style: None,
                ..default()
            };
            let player = find(&sandbox_objects(&config, Handle::default()), PLAYER_ID);
            let ScenarioObjectKind::Spaceship(ship) = player.kind else {
                panic!("the player object is a spaceship");
            };
            let ShipSource::Inline(hull) = &ship.hull else {
                panic!("the editor hands off an inline hull");
            };
            assert_eq!(
                hull.skin, clad,
                "the editor's toggle decides whether the flown ship is clad"
            );
        }
    }
}
