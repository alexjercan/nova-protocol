//! 18_screenshot_orbit: capture the `tutorial-orbit` shot - a ship flying a clean
//! ORBIT ring around a planetoid, with the orbit radius spoke on the HUD.
//!
//! It spawns a gravity planetoid and a player ship out at the ring radius, engages
//! the ORBIT autopilot on the well with an explicit orbit plan, lets the ship
//! settle onto the ring (the maneuver HUD draws the ring + radius spoke), and
//! captures with the HUD at its instrument tier.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/reel BCS_AUTOPILOT=1 BCS_REEL=1 \
//!   cargo run --example 18_screenshot_orbit --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 18_screenshot_orbit --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "18_screenshot_orbit")]
#[command(version = "1.0.0")]
#[command(about = "Capture the orbit tutorial shot", long_about = None)]
struct Cli;

/// The orbit ring radius the ship holds around the planetoid.
const ORBIT_RADIUS: f32 = 45.0;

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<OrbitScript>();
        app.add_plugins(
            AutopilotPlugin::<GameStates>::new()
                .hold(GameStates::Loading, 12.0)
                .input(orbit_capture_script),
        );
        app.add_systems(Startup, (force_resolution, hide_dev_overlays));
    }

    app.run();
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

/// Progress of the scripted capture run.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct OrbitScript {
    playing_since: Option<f32>,
    engaged: bool,
    captured: bool,
}

/// Engage the ORBIT autopilot on the planetoid, let the ship settle on the ring,
/// then capture. Captures only when `BCS_REEL` is set.
#[cfg(feature = "debug")]
fn orbit_capture_script(world: &mut World, elapsed: f32) {
    let capturing = std::env::var_os("BCS_REEL").is_some();

    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }
    if world.resource::<OrbitScript>().captured {
        return;
    }

    let playing_since = {
        let mut script = world.resource_mut::<OrbitScript>();
        *script.playing_since.get_or_insert(elapsed)
    };
    let t = elapsed - playing_since;

    // Engage ORBIT on the planetoid's gravity well with an explicit plan.
    if t > 0.4 && !world.resource::<OrbitScript>().engaged {
        world.resource_mut::<OrbitScript>().engaged = true;
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

    // Let the ship settle onto the ring (the maneuver HUD draws the ring +
    // radius spoke), then capture at the instrument HUD tier.
    if t > 6.0 && !world.resource::<OrbitScript>().captured {
        if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
            *hud = HudVisibility::Minimal;
        }
        if capturing {
            capture_window(world, "tutorial-orbit.png");
            info!("orbit capture: tutorial-orbit.png");
        }
        world.resource_mut::<OrbitScript>().captured = true;
    }
}
