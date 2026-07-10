//! 10_gameplay: a minimal, self-contained showcase of the `nova_gameplay` building blocks.
//!
//! Where `03_scenario` loads a ready-made named scenario (the `nova_scenario` scripting side),
//! this example assembles a ship *inline* from the section catalog so it reads as "here is how
//! you build a nova_gameplay ship and what its core mechanics are":
//!
//! - **Sections**: a player ship made of a controller, two hull sections, a thruster, and a
//!   turret - one of every structural role, wired up by `SpaceshipConfig`.
//! - **Health / destruction**: a handful of low-health asteroids as targets. The turret's
//!   bullets deal collision damage, the integrity pipeline destroys a depleted asteroid, and
//!   the throttled readout shows both the asteroids remaining and the ship's aggregate health.
//! - **A weapon**: the turret auto-aims at the nearest asteroid and fires while the trigger is
//!   held, so the shoot -> damage -> destroy loop runs end to end.
//!
//! Controls: Space (or right trigger) fires the turret. Aiming is automatic (nearest asteroid).
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 10_gameplay --features debug
//! # look for: `nova harness: reached Playing`, `gameplay: N asteroids left ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "10_gameplay")]
#[command(version = "1.0.0")]
#[command(about = "A minimal nova_gameplay example: a ship with sections, health, and one weapon", long_about = None)]
struct Cli;

/// The scenario this example builds. Shared between `gameplay_scenario` and the
/// smoke-test assertion so both agree on what "loaded" means.
const SCENARIO_ID: &str = "gameplay_minimal";

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // Headless smoke-test harness: inert in a normal run. Scene is built on
    // `GameAssetsStates::Loaded` so the screenshot's forced Playing does not re-run setup.
    // The scenario-loaded assertion fails the run if init comes up empty.
    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_autopilot().input(autopilot_fire));
        app.add_plugins(nova_screenshot());
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
    }

    app.run();
}

fn custom_plugin(app: &mut App) {
    app.insert_resource(StatusTimer(Timer::from_seconds(0.5, TimerMode::Repeating)));
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup);
    app.add_systems(
        Update,
        (
            // Aim after the player crosshair system so this range aim wins.
            aim_turret_at_nearest_asteroid.after(SpaceshipInputSystems),
            report_status,
        ),
    );
}

/// Throttle for the status readout.
#[derive(Resource)]
struct StatusTimer(Timer);

fn setup(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(gameplay_scenario(&game_assets, &sections)));
}

/// A player ship of one-of-each section, plus a short row of low-health asteroids to shoot.
fn gameplay_scenario(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };

    // A complete little ship: controller (PD attitude) + hull (structure/health) + thruster
    // (mobility) + turret (weapon). The turret's section id ("turret") is what the input mapping
    // binds the fire button to.
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
                id: "hull_front".to_string(),
                position: Vec3::new(0.0, 0.0, 1.0),
                rotation: Quat::IDENTITY,
                config: section("reinforced_hull_section"),
            },
            SpaceshipSectionConfig {
                id: "thruster".to_string(),
                position: Vec3::new(0.0, 0.0, 2.0),
                rotation: Quat::IDENTITY,
                config: section("basic_thruster_section"),
            },
            SpaceshipSectionConfig {
                id: "turret".to_string(),
                position: Vec3::new(0.0, 0.0, -1.0),
                // Sit the turret base upright (matches the asteroid_field ship).
                rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                config: section("better_turret_section"),
            },
        ],
    };

    // Low-health asteroids in the turret's arc (ahead, -Z), so the weapon can actually clear them.
    let asteroid = |id: &str, pos: Vec3| ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: id.to_string(),
            position: pos,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: 2.0,
            texture: game_assets.asteroid_texture.clone(),
            health: 30.0,
            surface_gravity: None,
        }),
    };

    let objects = vec![
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "player_ship".to_string(),
                name: "Player Ship".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(ship),
        },
        // Spread wide: an asteroid's collider is a good deal larger than its nominal radius, so
        // targets spawned too close shove each other apart and self-destruct before the turret
        // gets a shot. ~30 units apart keeps them independent until the weapon clears them.
        asteroid("asteroid_left", Vec3::new(-30.0, 2.0, -48.0)),
        asteroid("asteroid_center", Vec3::new(0.0, 3.0, -55.0)),
        asteroid("asteroid_right", Vec3::new(30.0, 2.0, -50.0)),
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
        name: "Gameplay Minimal".to_string(),
        description: "A minimal nova_gameplay ship with sections, health, and one weapon."
            .to_string(),
        cubemap: game_assets.cubemap.clone(),
        events,
    }
}

/// Point the turret at the nearest surviving asteroid so the weapon has something to hit.
fn aim_turret_at_nearest_asteroid(
    mut q_turret: Query<
        (&GlobalTransform, &mut TurretSectionTargetInput),
        With<TurretSectionMarker>,
    >,
    q_asteroids: Query<&GlobalTransform, With<AsteroidMarker>>,
) {
    for (turret_transform, mut target) in &mut q_turret {
        let muzzle = turret_transform.translation();
        let nearest = q_asteroids
            .iter()
            .map(|asteroid| asteroid.translation())
            .min_by(|a, b| {
                muzzle
                    .distance_squared(*a)
                    .total_cmp(&muzzle.distance_squared(*b))
            });
        **target = nearest;
    }
}

/// Throttled readout of the two headline mechanics: how many target asteroids remain, and the
/// player ship's aggregate section health.
fn report_status(
    time: Res<Time>,
    mut timer: ResMut<StatusTimer>,
    q_asteroids: Query<(), With<AsteroidMarker>>,
    q_ship: Query<&Health, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    q_bullets: Query<(), With<TurretBulletProjectileMarker>>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    // Nothing to report until the scenario has spawned the player ship.
    let Some(health) = q_ship.iter().next() else {
        return;
    };
    info!(
        "gameplay: {} asteroids left, ship health {:.0}/{:.0}, {} bullets in flight",
        q_asteroids.iter().count(),
        health.current,
        health.max,
        q_bullets.iter().count()
    );
}

/// Autopilot input: hold the fire key while in Playing so the turret shoots headless.
#[cfg(feature = "debug")]
fn autopilot_fire(world: &mut World, _elapsed: f32) {
    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
}
