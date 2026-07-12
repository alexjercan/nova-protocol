//! 07_torpedo_guidance: a focused harness for the torpedo's proportional-navigation
//! guidance.
//!
//! Unlike the broader `06_torpedo_range`, this scene is built to answer one
//! question: does the torpedo *lead* a moving target and intercept it? A player
//! torpedo ship sits at the origin; a single target crosses laterally, fast,
//! some way ahead. Each fired torpedo auto-locks the crosser, and the harness
//! reports the closest approach any torpedo achieves - a good PN solution drives
//! that number down to the blast radius or less.
//!
//! What it shows:
//! - Guidance gizmos: each torpedo draws a line to the point it is steering toward
//!   and a status sphere (yellow un-armed, green armed).
//! - `guidance: closest approach N.N` logs every time a torpedo gets nearer the
//!   target than any torpedo has so far - the headline PN quality metric.
//! - `guidance: torpedo speed N.N` (throttled) so you can see whether the torpedo
//!   is actually building enough speed to intercept.
//!
//! Controls: Space (or right trigger) fires. Targeting is automatic (nearest
//! target); in the full game you aim with the mouse.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 07_torpedo_guidance --features debug
//! # look for: `guidance: closest approach ...` shrinking toward the blast radius,
//! #           `range: torpedo detonated`, `autopilot: cycle complete, no panic`
//! ```

use avian3d::prelude::*;
use bevy::{color::palettes::tailwind, platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "07_torpedo_guidance")]
#[command(version = "1.0.0")]
#[command(about = "A harness for the torpedo PN guidance in nova_protocol", long_about = None)]
struct Cli;

/// Lateral speed of the crossing target (units/s).
const TARGET_CROSS_SPEED: f32 = 15.0;

/// The scenario this example builds. Shared between `guidance_scenario` and the
/// smoke-test assertion so both agree on what "loaded" means.
const SCENARIO_ID: &str = "torpedo_guidance";

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // The scenario-loaded assertion fails the headless run if init comes up empty.
    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_autopilot().input(autopilot_fire));
        app.add_plugins(nova_screenshot());
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
    }

    app.run();
}

fn custom_plugin(app: &mut App) {
    app.insert_resource(BestApproach(f32::INFINITY));
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup);
    app.add_systems(
        Update,
        (
            drive_target,
            range_autotarget,
            track_closest_approach,
            report_torpedo_speed,
            draw_guidance,
        ),
    );
    app.add_observer(tag_target);
    app.add_observer(log_torpedo_detonated);
}

/// Marks the crossing target so the harness can drive it and lock onto it.
#[derive(Component)]
struct GuidanceTarget;

/// Smallest torpedo-to-target distance seen so far, the headline PN metric.
#[derive(Resource)]
struct BestApproach(f32);

/// Throttle for the torpedo-speed readout.
#[derive(Component)]
struct SpeedReportTimer(Timer);

fn setup(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.spawn(SpeedReportTimer(Timer::from_seconds(
        0.5,
        TimerMode::Repeating,
    )));
    commands.trigger(LoadScenario(guidance_scenario(&game_assets, &sections)));
}

fn guidance_scenario(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };

    let ship = SpaceshipConfig {
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::from([("torpedo".to_string(), vec![KeyCode::Space.into()])]),
        }),
        sections: vec![
            SpaceshipSectionConfig {
                id: "controller".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                config: section("basic_controller_section"),
            },
            SpaceshipSectionConfig {
                id: "torpedo".to_string(),
                position: Vec3::new(0.0, 0.0, -1.0),
                rotation: Quat::IDENTITY,
                config: section("torpedo_section"),
            },
        ],
    };

    // Give the crossing target a lot of health so it survives being hit and the
    // harness can keep measuring approach across several torpedoes.
    let objects = vec![
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "player_ship".to_string(),
                name: "Torpedo Ship".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(ship),
        },
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "crosser".to_string(),
                name: "Crossing Target".to_string(),
                position: Vec3::new(-40.0, 0.0, -90.0),
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                radius: 3.0,
                texture: game_assets.asteroid_texture.clone(),
                health: 100_000.0,
                surface_gravity: None,
                invulnerable: false,
            }),
        },
    ];

    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: objects
            .into_iter()
            .map(EventActionConfig::SpawnScenarioObject)
            .collect(),
    }];

    ScenarioConfig {
        id: SCENARIO_ID.to_string(),
        name: "Torpedo Guidance".to_string(),
        description: "A harness for the torpedo PN guidance.".to_string(),
        cubemap: game_assets.cubemap.clone(),
        events,
    }
}

fn tag_target(add: On<Add, AsteroidMarker>, mut commands: Commands, q_id: Query<&EntityId>) {
    if q_id
        .get(add.entity)
        .map(|id| id.0 == "crosser")
        .unwrap_or(false)
    {
        commands.entity(add.entity).insert(GuidanceTarget);
    }
}

/// Drive the target across at a constant lateral velocity (the case PN should
/// solve exactly by leading it).
fn drive_target(mut q_target: Query<&mut LinearVelocity, With<GuidanceTarget>>) {
    for mut velocity in &mut q_target {
        velocity.0 = Vec3::new(TARGET_CROSS_SPEED, 0.0, 0.0);
    }
}

/// Lock every fresh torpedo onto the crossing target, once (commit-at-launch,
/// mirroring the game's `TorpedoTargetChosen` contract).
fn range_autotarget(
    mut commands: Commands,
    q_torpedo: Query<
        Entity,
        (
            With<TorpedoProjectileMarker>,
            Without<TorpedoTargetEntity>,
            Without<TorpedoTargetChosen>,
        ),
    >,
    q_target: Query<Entity, With<GuidanceTarget>>,
) {
    let Ok(target) = q_target.single() else {
        return;
    };
    for torpedo in &q_torpedo {
        commands
            .entity(torpedo)
            .insert((TorpedoTargetEntity(target), TorpedoTargetChosen));
    }
}

/// Track the closest any torpedo gets to the target, the headline PN metric.
fn track_closest_approach(
    mut best: ResMut<BestApproach>,
    q_torpedo: Query<(&GlobalTransform, &TorpedoTargetPosition), With<TorpedoProjectileMarker>>,
) {
    for (transform, target) in &q_torpedo {
        let distance = transform.translation().distance(**target);
        if distance < best.0 - 0.1 {
            best.0 = distance;
            info!("guidance: closest approach {:.1}", distance);
        }
    }
}

/// Throttled readout of the fastest torpedo in flight, to reveal whether the
/// torpedo is building enough speed to intercept.
fn report_torpedo_speed(
    time: Res<Time>,
    mut q_timer: Query<&mut SpeedReportTimer>,
    q_torpedo: Query<&LinearVelocity, With<TorpedoProjectileMarker>>,
) {
    let Ok(mut timer) = q_timer.single_mut() else {
        return;
    };
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    if let Some(speed) = q_torpedo.iter().map(|v| v.length()).max_by(f32::total_cmp) {
        info!("guidance: torpedo speed {:.1}", speed);
    }
}

fn draw_guidance(
    mut gizmos: Gizmos,
    q_torpedo: Query<
        (&GlobalTransform, &TorpedoTargetPosition, &TorpedoArming),
        With<TorpedoProjectileMarker>,
    >,
) {
    for (transform, target, arming) in &q_torpedo {
        let pos = transform.translation();
        let status = if arming.is_armed() {
            tailwind::GREEN_400
        } else {
            tailwind::YELLOW_400
        };
        gizmos.sphere(Isometry3d::from_translation(pos), 0.6, status);
        gizmos.line(pos, **target, tailwind::RED_400);
        gizmos.sphere(
            Isometry3d::from_translation(**target),
            1.0,
            tailwind::RED_400,
        );
    }
}

fn log_torpedo_detonated(add: On<Add, BlastDamageMarker>) {
    info!("range: torpedo detonated ({:?})", add.entity);
}

#[cfg(feature = "debug")]
fn autopilot_fire(world: &mut World, _elapsed: f32) {
    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
}
