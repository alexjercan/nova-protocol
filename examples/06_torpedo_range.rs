//! 06_torpedo_range: a focused test range for the torpedo bay section.
//!
//! One player ship carrying a single torpedo bay sits at the origin facing a
//! spread of asteroid "gates" - near, mid and far straight ahead, one off to the
//! side, and one that drifts slowly across the range to judge homing and lead.
//! Fire torpedoes at them and watch how they arm, home and detonate. This is the
//! interactive harness for the torpedo work (arming `20260707-100003`, target
//! loss `20260707-100004`, PN guidance `20260525-133021`, blast tuning
//! `20260706-162913`).
//!
//! What it shows:
//! - Guidance gizmos: each torpedo draws a red line to the point it is steering
//!   toward, a red sphere at that target point, and a sphere on the torpedo that
//!   is yellow while un-armed and green once armed (so you can see the arming
//!   delay from `20260707-100003` directly).
//! - Lifecycle logging: `range: torpedo fired`, `range: torpedo ... armed`, and
//!   `range: torpedo detonated` trace one shot from launch to blast.
//!
//! Targeting: for the range each fresh torpedo is auto-assigned the nearest gate
//! (so homing is always exercised, even hands-off); in the full game you aim with
//! the mouse and the player targeting picks the target instead.
//!
//! Controls:
//! - Space (or right trigger): fire a torpedo. Held, it fires at the bay's rate.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 06_torpedo_range --features debug
//! # look for: `nova harness: reached Playing`, `range: torpedo fired`,
//! #           `range: torpedo ... armed`, `autopilot: cycle complete, no panic`
//! ```

use avian3d::prelude::*;
use bevy::{color::palettes::tailwind, platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "06_torpedo_range")]
#[command(version = "1.0.0")]
#[command(about = "A test range for the torpedo bay section in nova_protocol", long_about = None)]
struct Cli;

/// Id of the one gate that drifts, so the range can single it out and drive it.
const MOVING_GATE_ID: &str = "gate_moving";

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // Headless smoke-test harness: inert in a normal run. Under BCS_AUTOPILOT it
    // drives Loading -> Playing, holds the fire key, and exits without panic;
    // under BCS_SHOT it captures a PNG. The scene is built on
    // `GameAssetsStates::Loaded` (below) so the screenshot's forced Playing does
    // not re-run setup.
    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_autopilot().input(autopilot_fire));
        app.add_plugins(nova_screenshot());
    }

    app.run();
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
    app.add_systems(
        Update,
        (
            range_autotarget,
            drive_moving_gate,
            log_torpedo_armed,
            draw_torpedo_guidance,
        ),
    );
    app.add_observer(tag_gate);
    app.add_observer(log_torpedo_fired);
    app.add_observer(log_torpedo_detonated);
}

/// Marks an asteroid the range treats as a target gate.
#[derive(Component)]
struct RangeGateMarker;

/// Marks the single drifting gate.
#[derive(Component)]
struct RangeMovingTarget;

/// Marks a torpedo whose "armed" transition has already been logged.
#[derive(Component)]
struct RangeArmLogged;

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(torpedo_range(&game_assets, &sections)));
}

/// Build the range scenario: one player torpedo ship plus the target gates.
fn torpedo_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };

    // Player ship: a controller (so it holds attitude and is aimable) and the
    // torpedo bay under test. The bay's section id "torpedo" is what the input
    // mapping binds the fire key to.
    let ship = SpaceshipConfig {
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::from([(
                "torpedo".to_string(),
                vec![
                    KeyCode::Space.into(),
                    GamepadButton::RightTrigger.into(),
                ],
            )]),
        }),
        sections: vec![
            SpaceshipSectionConfig {
                id: "controller".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                config: section("basic_controller_section"),
            },
            SpaceshipSectionConfig {
                id: "hull".to_string(),
                position: Vec3::new(0.0, 0.0, 1.0),
                rotation: Quat::IDENTITY,
                config: section("reinforced_hull_section"),
            },
            SpaceshipSectionConfig {
                id: "torpedo".to_string(),
                position: Vec3::new(0.0, 0.0, -1.0),
                rotation: Quat::IDENTITY,
                config: section("torpedo_section"),
            },
        ],
    };

    // Gates ahead of the ship (forward is -Z). Health is low enough that a
    // torpedo blast destroys one, so you get arm -> home -> hit feedback.
    let gate = |id: &str, name: &str, pos: Vec3, radius: f32| ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position: pos,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius,
            texture: game_assets.asteroid_texture.clone(),
            health: 60.0,
        }),
    };

    let mut objects = vec![ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "player_ship".to_string(),
            name: "Torpedo Ship".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(ship),
    }];
    objects.push(gate("gate_near", "Near Gate", Vec3::new(0.0, 0.0, -30.0), 2.0));
    objects.push(gate("gate_mid", "Mid Gate", Vec3::new(0.0, 0.0, -70.0), 3.0));
    objects.push(gate("gate_far", "Far Gate", Vec3::new(6.0, 0.0, -120.0), 3.0));
    objects.push(gate(
        "gate_side",
        "Side Gate",
        Vec3::new(-25.0, 0.0, -60.0),
        2.0,
    ));
    objects.push(gate(
        MOVING_GATE_ID,
        "Moving Gate",
        Vec3::new(25.0, 0.0, -90.0),
        2.0,
    ));

    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: objects
            .into_iter()
            .map(EventActionConfig::SpawnScenarioObject)
            .collect(),
    }];

    ScenarioConfig {
        id: "torpedo_range".to_string(),
        name: "Torpedo Range".to_string(),
        description: "A test range for the torpedo bay section.".to_string(),
        cubemap: game_assets.cubemap.clone(),
        events,
    }
}

/// Tag each spawned asteroid as a gate, and single out the drifting one.
fn tag_gate(add: On<Add, AsteroidMarker>, mut commands: Commands, q_id: Query<&EntityId>) {
    let entity = add.entity;
    commands.entity(entity).insert(RangeGateMarker);

    if q_id.get(entity).map(|id| id.0 == MOVING_GATE_ID).unwrap_or(false) {
        commands
            .entity(entity)
            .insert((RangeMovingTarget, LinearVelocity::default()));
    }
}

/// Drift the moving gate side to side so torpedoes have to lead it.
fn drive_moving_gate(time: Res<Time>, mut q_gate: Query<&mut LinearVelocity, With<RangeMovingTarget>>) {
    let speed = (time.elapsed_secs() * 0.5).sin() * 10.0;
    for mut velocity in &mut q_gate {
        velocity.0 = Vec3::new(speed, 0.0, 0.0);
    }
}

/// Range convenience: assign each fresh torpedo the nearest gate so homing is
/// always exercised. In the full game the player's aim picks the target instead.
fn range_autotarget(
    mut commands: Commands,
    q_torpedo: Query<
        (Entity, &GlobalTransform),
        (With<TorpedoProjectileMarker>, Without<TorpedoTargetEntity>),
    >,
    q_gates: Query<(Entity, &GlobalTransform), With<RangeGateMarker>>,
) {
    for (torpedo, torpedo_transform) in &q_torpedo {
        let origin = torpedo_transform.translation();
        let nearest = q_gates
            .iter()
            .min_by(|(_, a), (_, b)| {
                let da = a.translation().distance_squared(origin);
                let db = b.translation().distance_squared(origin);
                da.total_cmp(&db)
            })
            .map(|(gate, _)| gate);

        if let Some(gate) = nearest {
            commands.entity(torpedo).insert(TorpedoTargetEntity(gate));
        }
    }
}

/// Draw the torpedo -> target line-of-sight and an armed/un-armed status sphere.
fn draw_torpedo_guidance(
    mut gizmos: Gizmos,
    q_torpedo: Query<
        (&GlobalTransform, &TorpedoTargetPosition, &TorpedoArming),
        With<TorpedoProjectileMarker>,
    >,
) {
    for (torpedo_transform, target, arming) in &q_torpedo {
        let pos = torpedo_transform.translation();
        let status = if arming.is_armed() {
            tailwind::GREEN_400
        } else {
            tailwind::YELLOW_400
        };
        gizmos.sphere(Isometry3d::from_translation(pos), 0.6, status);
        gizmos.line(pos, **target, tailwind::RED_400);
        gizmos.sphere(Isometry3d::from_translation(**target), 1.0, tailwind::RED_400);
    }
}

fn log_torpedo_fired(add: On<Add, TorpedoProjectileMarker>) {
    info!("range: torpedo fired ({:?})", add.entity);
}

/// Log the moment a torpedo arms (once), so the arming delay is visible in logs.
fn log_torpedo_armed(
    mut commands: Commands,
    q_torpedo: Query<
        (Entity, &TorpedoArming),
        (With<TorpedoProjectileMarker>, Without<RangeArmLogged>),
    >,
) {
    for (torpedo, arming) in &q_torpedo {
        if arming.is_armed() {
            info!("range: torpedo {:?} armed", torpedo);
            commands.entity(torpedo).insert(RangeArmLogged);
        }
    }
}

fn log_torpedo_detonated(add: On<Add, BlastDamageMarker>) {
    info!("range: torpedo detonated ({:?})", add.entity);
}

/// Autopilot input: hold the fire key while in Playing so the range fires
/// torpedoes headlessly.
#[cfg(feature = "debug")]
fn autopilot_fire(world: &mut World, _elapsed: f32) {
    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
}
