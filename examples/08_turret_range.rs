//! 08_turret_range: a focused test range for the PDC turret section.
//!
//! One player ship carrying a single turret sits at the origin. A spread of
//! asteroid "gates" sits in the turret's firing arc, and one gate sweeps back and
//! forth across the front so you can watch the turret track a mover - the point
//! being to judge how cleanly the turret aims and fires, and to make the aiming
//! cheap to tune and regression-test.
//!
//! The range points the turret at the moving gate (the crosshair does this in the
//! full game); the barrel slews to follow and fires when you hold the trigger.
//!
//! What it shows:
//! - Aim gizmos: a line down the barrel (green when it is on target, yellow while
//!   it lags) and a red line + sphere at the point the turret is aiming for. The
//!   gap between the barrel line and the target line is the tracking lag - the
//!   "clunky" aiming this range exists to expose.
//! - `turret: aim error N.N deg, M bullets in flight` (throttled) so tracking
//!   quality is visible headless.
//!
//! Controls: Space (or right trigger) fires. Aiming is automatic (tracks the
//! moving gate); in the full game you aim with the mouse.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 08_turret_range --features debug
//! # look for: `nova harness: reached Playing`, `turret: aim error ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use avian3d::prelude::*;
use bevy::{color::palettes::tailwind, platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "08_turret_range")]
#[command(version = "1.0.0")]
#[command(about = "A test range for the PDC turret section in nova_protocol", long_about = None)]
struct Cli;

/// Id of the gate that sweeps across the front for the turret to track.
const MOVING_GATE_ID: &str = "gate_moving";

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // Headless smoke-test harness: inert in a normal run. Scene is built on
    // `GameAssetsStates::Loaded` so the screenshot's forced Playing does not
    // re-run setup.
    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_autopilot().input(autopilot_fire));
        app.add_plugins(nova_screenshot());
    }

    app.run();
}

fn custom_plugin(app: &mut App) {
    app.insert_resource(StatusTimer(Timer::from_seconds(0.5, TimerMode::Repeating)));
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
    app.add_systems(
        Update,
        (
            // Aim after the player crosshair system so the range aim wins - the
            // range points the turret at the moving gate rather than straight
            // ahead.
            range_aim.after(SpaceshipInputSystems),
            drive_moving_gate,
            draw_turret_aim,
            report_status,
        ),
    );
    app.add_observer(tag_gate);
}

/// Marks an asteroid the range treats as a target gate.
#[derive(Component)]
struct RangeGateMarker;

/// Marks the gate that sweeps across the front (the turret's tracking target).
#[derive(Component)]
struct RangeMovingTarget;

/// Throttle for the status readout.
#[derive(Resource)]
struct StatusTimer(Timer);

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(turret_range(&game_assets, &sections)));
}

fn turret_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };

    let ship = SpaceshipConfig {
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::from([(
                "turret".to_string(),
                vec![KeyCode::Space.into(), GamepadButton::RightTrigger.into()],
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
                id: "turret".to_string(),
                position: Vec3::new(0.0, 0.5, -1.0),
                // Matches the turret placement in the asteroid_field ship so the
                // base sits upright.
                rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                config: section("better_turret_section"),
            },
        ],
    };

    // Gates in the turret's firing arc (ahead, -Z, within its pitch range), plus
    // the sweeping one. Health is high so they survive and the turret keeps
    // tracking/firing rather than clearing the range in a burst.
    let gate = |id: &str, name: &str, pos: Vec3| ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position: pos,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: 2.0,
            texture: game_assets.asteroid_texture.clone(),
            health: 2000.0,
        }),
    };

    let objects = vec![
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "player_ship".to_string(),
                name: "Turret Ship".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(ship),
        },
        gate("gate_front", "Front Gate", Vec3::new(0.0, 3.0, -55.0)),
        gate("gate_left", "Left Gate", Vec3::new(-32.0, 4.0, -45.0)),
        gate("gate_right", "Right Gate", Vec3::new(30.0, 3.0, -48.0)),
        gate("gate_high", "High Gate", Vec3::new(6.0, 26.0, -40.0)),
        gate(MOVING_GATE_ID, "Moving Gate", Vec3::new(-35.0, 6.0, -55.0)),
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
        id: "turret_range".to_string(),
        name: "Turret Range".to_string(),
        description: "A test range for the PDC turret section.".to_string(),
        cubemap: game_assets.cubemap.clone(),
        events,
    }
}

/// Tag each spawned asteroid as a gate, and single out the sweeping one.
fn tag_gate(add: On<Add, AsteroidMarker>, mut commands: Commands, q_id: Query<&EntityId>) {
    let entity = add.entity;
    commands.entity(entity).insert(RangeGateMarker);

    if q_id.get(entity).map(|id| id.0 == MOVING_GATE_ID).unwrap_or(false) {
        commands
            .entity(entity)
            .insert((RangeMovingTarget, LinearVelocity::default()));
    }
}

/// Sweep the moving gate across the front so the turret has to track it.
fn drive_moving_gate(
    time: Res<Time>,
    mut q_gate: Query<&mut LinearVelocity, With<RangeMovingTarget>>,
) {
    let speed = (time.elapsed_secs() * 0.6).sin() * 18.0;
    for mut velocity in &mut q_gate {
        velocity.0 = Vec3::new(speed, 0.0, 0.0);
    }
}

/// Point the turret at the sweeping gate (falls back to any gate), so the range
/// exercises tracking without a mouse. Runs after the crosshair aim so it wins.
/// Also feeds the gate's velocity so the turret leads the mover.
fn range_aim(
    mut q_turret: Query<
        (&mut TurretSectionTargetInput, &mut TurretSectionTargetVelocity),
        With<TurretSectionMarker>,
    >,
    q_moving: Query<(&GlobalTransform, &LinearVelocity), With<RangeMovingTarget>>,
    q_gates: Query<&GlobalTransform, With<RangeGateMarker>>,
) {
    let (target, velocity) = if let Some((transform, linear_velocity)) = q_moving.iter().next() {
        (Some(transform.translation()), linear_velocity.0)
    } else if let Some(transform) = q_gates.iter().next() {
        (Some(transform.translation()), Vec3::ZERO)
    } else {
        (None, Vec3::ZERO)
    };

    for (mut turret_target, mut turret_velocity) in &mut q_turret {
        **turret_target = target;
        **turret_velocity = velocity;
    }
}

/// Draw the barrel direction (green on target, yellow while lagging) and the line
/// to the turret's lead aim point, so the tracking lag is visible.
fn draw_turret_aim(
    mut gizmos: Gizmos,
    q_muzzle: Query<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>,
    q_turret: Query<&TurretSectionAimPoint, With<TurretSectionMarker>>,
) {
    let Some(aim) = q_turret.iter().next().and_then(|a| **a) else {
        return;
    };
    gizmos.sphere(Isometry3d::from_translation(aim), 1.5, tailwind::RED_400);

    for muzzle in &q_muzzle {
        let pos = muzzle.translation();
        let barrel = muzzle.forward();
        let to_aim = (aim - pos).normalize_or_zero();
        let color = if barrel.dot(to_aim) > 0.999 {
            tailwind::GREEN_400
        } else {
            tailwind::YELLOW_400
        };
        gizmos.line(pos, pos + barrel * 60.0, color);
        gizmos.line(pos, aim, tailwind::RED_400);
    }
}

/// Throttled readout of the aiming error (barrel vs. the turret's lead aim point)
/// and the number of bullets in flight, so tracking quality and firing are legible
/// headless. With the lead solution this should stay in the single digits even
/// against the sweeping gate.
fn report_status(
    time: Res<Time>,
    mut timer: ResMut<StatusTimer>,
    q_muzzle: Query<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>,
    q_turret: Query<&TurretSectionAimPoint, With<TurretSectionMarker>>,
    q_bullets: Query<(), With<TurretBulletProjectileMarker>>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    let Some(aim) = q_turret.iter().next().and_then(|a| **a) else {
        return;
    };
    let Ok(muzzle) = q_muzzle.single() else {
        return;
    };
    let barrel = muzzle.forward();
    let to_aim = (aim - muzzle.translation()).normalize_or_zero();
    let error_deg = barrel.dot(to_aim).clamp(-1.0, 1.0).acos().to_degrees();
    info!(
        "turret: aim error {:.1} deg, {} bullets in flight",
        error_deg,
        q_bullets.iter().count()
    );
}

/// Autopilot input: hold the fire key while in Playing so the range fires headless.
#[cfg(feature = "debug")]
fn autopilot_fire(world: &mut World, _elapsed: f32) {
    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
}
