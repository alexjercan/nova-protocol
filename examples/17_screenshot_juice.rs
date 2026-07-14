//! 17_screenshot_juice: capture the `feature-juice` shot - a section blown off a
//! hull mid-fight, framed close so the mesh fragments and hit rings read.
//!
//! The combat range films from the chase camera behind the player, which is too
//! far to see a section shatter. Here there is no player ship, so the scenario's
//! free-fly camera stays active; the script poses it right on a target ship,
//! blows one section off through the production damage path, and captures a few
//! frames later while the fragments are still in the air.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/reel BCS_AUTOPILOT=1 BCS_REEL=1 \
//!   cargo run --example 17_screenshot_juice --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 17_screenshot_juice --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "17_screenshot_juice")]
#[command(version = "1.0.0")]
#[command(about = "Capture the section-destruction juice shot", long_about = None)]
struct Cli;

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<JuiceScript>();
        app.add_plugins(
            AutopilotPlugin::<GameStates>::new()
                .hold(GameStates::Loading, 8.0)
                .input(juice_capture_script),
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
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_target);
}

fn setup_target(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(juice_target(&game_assets, &sections)));
}

/// A lone target ship (no player, so the free-fly camera stays available) built
/// from a controller, two hulls, a thruster and a turret so there is a section
/// to blow off.
fn juice_target(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, position: Vec3, rotation: Quat| SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation,
        source: SectionSource::Inline(section(kind)),
        modifications: vec![],
    };
    let upright = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);

    let ship = SpaceshipConfig {
        controller: SpaceshipController::None,
        sections: vec![
            at(
                "controller",
                "basic_controller_section",
                Vec3::ZERO,
                Quat::IDENTITY,
            ),
            at(
                "hull_front",
                "reinforced_hull_section",
                Vec3::new(0.0, 0.0, -1.0),
                Quat::IDENTITY,
            ),
            at(
                "hull_rear",
                "reinforced_hull_section",
                Vec3::new(0.0, 0.0, 1.0),
                Quat::IDENTITY,
            ),
            at(
                "thruster",
                "basic_thruster_section",
                Vec3::new(0.0, 0.0, 2.0),
                Quat::IDENTITY,
            ),
            at(
                "turret",
                "better_turret_section",
                Vec3::new(1.0, 0.0, 0.0),
                upright,
            ),
        ],
    };

    ScenarioConfig {
        id: "juice_target".to_string(),
        name: "Juice Target".to_string(),
        description: "A target ship to blow a section off for the juice shot.".to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: vec![EventActionConfig::SpawnScenarioObject(
                ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "target_ship".to_string(),
                        name: "Target".to_string(),
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Spaceship(ship),
                },
            )],
        }],
    }
}

/// A `Health`-carrying node under the target's hull-front section, so blowing it
/// takes a visible chunk off the front rather than the central controller.
#[cfg(feature = "debug")]
fn front_hull_health_node(world: &mut World) -> Option<Entity> {
    let ship = world
        .query_filtered::<Entity, With<SpaceshipRootMarker>>()
        .iter(world)
        .next()?;
    // Health nodes live under the section entities; walk each up to the ship and
    // pick one whose section sits forward of the hull (most negative Z child).
    use std::collections::HashMap;
    let parents: HashMap<Entity, Entity> = {
        let mut q = world.query::<(Entity, &ChildOf)>();
        q.iter(world).map(|(e, c)| (e, c.parent())).collect()
    };
    let mut best: Option<(Entity, f32)> = None;
    let nodes: Vec<Entity> = {
        let mut q = world.query_filtered::<Entity, With<Health>>();
        q.iter(world).collect()
    };
    for node in nodes {
        // Confirm the node belongs to the target ship (walk up to it).
        let mut current = node;
        let mut under_ship = false;
        for _ in 0..4 {
            match parents.get(&current) {
                Some(&p) if p == ship => {
                    under_ship = true;
                    break;
                }
                Some(&p) => current = p,
                None => break,
            }
        }
        if !under_ship {
            continue;
        }
        let z = world
            .get::<GlobalTransform>(node)
            .map(|t| t.translation().z)
            .unwrap_or(0.0);
        if best.map(|(_, bz)| z < bz).unwrap_or(true) {
            best = Some((node, z));
        }
    }
    best.map(|(node, _)| node)
}

/// Progress of the scripted capture run.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct JuiceScript {
    playing_since: Option<f32>,
    posed: bool,
    blown: bool,
    captured: bool,
}

/// Pose the camera on the target, blow the front hull off, then capture while the
/// fragments/rings are live. Captures only when `BCS_REEL` is set.
#[cfg(feature = "debug")]
fn juice_capture_script(world: &mut World, elapsed: f32) {
    let capturing = std::env::var_os("BCS_REEL").is_some();

    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }
    if world.resource::<JuiceScript>().captured {
        return;
    }

    let playing_since = {
        let mut script = world.resource_mut::<JuiceScript>();
        *script.playing_since.get_or_insert(elapsed)
    };
    let t = elapsed - playing_since;

    // Frame the target close, from the front-quarter so the front hull is toward
    // the camera.
    if t > 0.3 && !world.resource::<JuiceScript>().posed {
        world.resource_mut::<JuiceScript>().posed = true;
        reel_pose_camera(world, Vec3::new(-5.0, 2.5, -6.0), Vec3::new(0.0, 0.0, -0.5));
    }

    // Blow the front hull section off through the production damage path.
    if t > 1.2 && !world.resource::<JuiceScript>().blown {
        world.resource_mut::<JuiceScript>().blown = true;
        if let Some(node) = front_hull_health_node(world) {
            world.trigger(HealthApplyDamage {
                entity: node,
                source: None,
                amount: 1.0e6,
            });
            info!("juice: blew the front hull section");
        }
    }

    // A few frames after the hit, while the fragments and hit rings are live.
    if t > 1.32 && !world.resource::<JuiceScript>().captured {
        if capturing {
            capture_window(world, "feature-juice.png");
            info!("juice capture: feature-juice.png");
        }
        world.resource_mut::<JuiceScript>().captured = true;
    }
}
