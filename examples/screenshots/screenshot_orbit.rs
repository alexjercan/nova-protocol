//! screenshot_orbit: capture the `tutorial-orbit` shot - a ship flying a clean
//! ORBIT ring around a planetoid, with the orbit radius spoke on the HUD.
//!
//! It spawns a gravity planetoid and a player ship out at the ring radius, engages
//! the ORBIT autopilot on the well with an explicit orbit plan, lets the ship
//! settle onto the ring (the maneuver HUD draws the ring + radius spoke), and
//! captures with the HUD on.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/reel NOVA_AUTOPILOT=1 NOVA_REEL=1 \
//!   cargo run --example screenshot_orbit --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_orbit --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_orbit")]
#[command(version = "1.0.0")]
#[command(about = "Capture the orbit tutorial shot", long_about = None)]
struct Cli;

/// The orbit ring radius the ship holds around the planetoid.
const ORBIT_RADIUS: f32 = 45.0;

/// Seconds the ship flies before the shot, to settle onto the ring.
#[cfg(feature = "debug")]
const SETTLE_SECS: f32 = 5.6;

/// Frames a capture step holds after requesting its PNG. `capture_window`
/// spawns a bare `Screenshot` and is NOT a completion collector, so the last
/// step's hold is the only thing giving `save_to_disk` room to land before the
/// driver reports done and the app exits. A smoke run captures nothing and only
/// needs the step to be observable.
#[cfg(feature = "debug")]
fn capture_settle_frames(capturing: bool) -> u32 {
    if capturing {
        20
    } else {
        2
    }
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        let capturing = std::env::var_os(nova_protocol::nova_debug::harness::REEL_ENV).is_some();
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                // Wait for the ship to EXIST rather than for a guessed load
                // duration: a slow load delays the beats instead of eating them.
                .step("load the orbit scene")
                .enter(GameStates::Loading)
                .until(player_ship_present())
                .deadline(30.0)
                .add()
                .step("settle onto the orbit ring")
                .on_enter(engage_orbit)
                .until(elapsed(SETTLE_SECS))
                .add()
                .step("capture the orbit shot")
                .on_enter(move |world| capture_orbit(world, capturing))
                .until(frames(capture_settle_frames(capturing)))
                .add(),
        );
        app.add_systems(Startup, (force_resolution, hide_dev_overlays));
    }

    app.run()
}

/// Force the window to 1920x1080 (the 16:9 the web figures use) at startup.
#[cfg(feature = "debug")]
fn force_resolution(mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>) {
    if let Ok(mut window) = windows.single_mut() {
        window.resolution.set(1920.0, 1080.0);
        window.resizable = false;
    }
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_orbit);
}

fn setup_orbit(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(orbit_scene(&game_assets, &sections)));
}

/// A gravity planetoid at the origin and a player ship out at the ring radius.
fn orbit_scene(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, z: f32| SpaceshipSectionConfig {
        id: id.to_string(),
        position: Vec3::new(0.0, 0.0, z),
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section(kind)),
        modifications: vec![],
    };
    let player = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: bevy::platform::collections::HashMap::new(),
            speed_cap: None,
            infinite_ammo: true,
            lock_refire_secs: None,
        }),
        sections: vec![
            at("controller", "basic_controller_section", 0.0),
            at("hull", "reinforced_hull_section", 1.0),
            at("thruster", "basic_thruster_section", 2.0),
        ],
    };

    ScenarioConfig {
        id: "orbit_scene".to_string(),
        name: "Orbit Scene".to_string(),
        description: "A planetoid and a ship for the orbit tutorial shot.".to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: vec![
                EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "planetoid".to_string(),
                        name: "Planetoid".to_string(),
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                        impact_sound: Some("base/sounds/impact.wav".into()),
                        destroy_sound: Some("base/sounds/explosion.wav".into()),
                        radius: 12.0,
                        texture: game_assets.asteroid_texture.clone().into(),
                        health: 5000.0,
                        surface_gravity: Some(6.0),
                        invulnerable: true,
                        lock_signature: None,
                    }),
                }),
                EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "player_ship".to_string(),
                        name: "Player Ship".to_string(),
                        position: Vec3::new(ORBIT_RADIUS, 0.0, 0.0),
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Spaceship(player),
                }),
            ],
        }],
        ..Default::default()
    }
}

/// Engage the ORBIT autopilot on the planetoid's gravity well with an explicit
/// plan, so the ship flies the ring the shot is about.
#[cfg(feature = "debug")]
fn engage_orbit(world: &mut World) {
    let well = world
        .query_filtered::<Entity, With<GravityWell>>()
        .iter(world)
        .next();
    let player = world
        .query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
        .iter(world)
        .next();
    if let (Some(well), Some(player)) = (well, player) {
        world
            .entity_mut(player)
            .insert(Autopilot::engage(AutopilotAction::Orbit {
                well,
                plan: Some(OrbitPlan {
                    radius: ORBIT_RADIUS,
                    normal: Vec3::Y,
                }),
            }));
        info!("orbit: engaged ORBIT on the planetoid");
    }
}

/// Turn the HUD on (the maneuver HUD draws the ring + radius spoke) and request
/// the shot. Captures only when `NOVA_REEL` is set.
#[cfg(feature = "debug")]
fn capture_orbit(world: &mut World, capturing: bool) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::On;
    }
    if capturing {
        capture_window(world, "tutorial-orbit.png");
        info!("orbit capture: tutorial-orbit.png");
    }
}
