//! screenshot_sections: capture the wiki ship-section detail shots - a
//! closeup of each section type on one built ship - using the screenshot reel.
//!
//! It builds a ship carrying all five section types (controller, hull, thruster,
//! turret, torpedo bay) and steps the reel camera to a closeup of each, writing
//! `wiki-section-<kind>.png`. The scenario camera is posed per beat by the reel
//! plugin, and the scene is frozen so every section sits still for its shot.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/reel BCS_REEL=1 \
//!   cargo run --example screenshot_sections --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example screenshot_sections --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_sections")]
#[command(version = "1.0.0")]
#[command(about = "Capture the wiki ship-section detail shots", long_about = None)]
struct Cli;

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Smoke path: reach Playing on the built scene and exit clean.
        // Probe wiring (task 20260719-210443; each plugin is inert without
        // its NOVA_PERF_* env): run timeline + engine-bound invariants +
        // frame-time capture, so `probe run` can measure this example.
        app.add_plugins(nova_probe::nova_timeline());
        app.add_plugins(nova_probe::nova_invariants());
        app.add_plugins(nova_probe::nova_frametime());
        app.add_plugins(nova_autopilot());
        // Capture path: pose the camera at each section and shoot.
        app.add_plugins(ScreenshotReelPlugin::new(section_beats()));
    }

    app.run();
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_ship);
}

fn setup_ship(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(section_ship(&game_assets, &sections)));
}

/// A single ship carrying every section type, laid out along its axis so each
/// sits at a known spot the reel camera can frame:
/// torpedo(-2) turret(-1) controller(0) hull(+1) thruster(+2).
fn section_ship(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
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
    let upright = Quat::IDENTITY;
    // The turret: 90 deg clockwise about Z, then X, then Y, so it sits on the
    // right flank facing out (barrel along the hull) instead of into the ship.
    let turret_rot = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)
        * Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)
        * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);

    // A real ship shape: a spine (front hull -> controller -> rear hull ->
    // thruster) along -/+Z, with the turret and the torpedo bay mounted on the
    // left/right flanks rather than stacked in front.
    let ship = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::None,
        sections: vec![
            at(
                "controller",
                "basic_controller_section",
                Vec3::new(0.0, 0.0, 0.0),
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
            // Turret on the right flank, torpedo bay on the left - both upright.
            at(
                "turret",
                "better_turret_section",
                Vec3::new(1.0, 0.0, 0.0),
                turret_rot,
            ),
            at(
                "torpedo",
                "torpedo_section",
                Vec3::new(-1.0, 0.0, 0.0),
                upright,
            ),
        ],
    };

    ScenarioConfig {
        id: "section_showcase".to_string(),
        name: "Section Showcase".to_string(),
        description: "A ship carrying every section type for the wiki shots.".to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: vec![EventActionConfig::SpawnScenarioObject(
                ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "showcase_ship".to_string(),
                        name: "Showcase Ship".to_string(),
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Spaceship(ship),
                },
            )],
        }],
        ..Default::default()
    }
}

/// A closeup of each section, each framed from the side that shows it while the
/// whole ship still reads. Positions match the ship layout in `section_ship`.
#[cfg(feature = "debug")]
fn section_beats() -> Vec<ReelBeat> {
    let beat = |eye: Vec3, look: Vec3, name: &str| ReelBeat::new(ReelCamera::new(eye, look), name);
    vec![
        // Controller: from the front-right, looking back down the spine.
        beat(
            Vec3::new(3.5, 2.4, -5.5),
            Vec3::new(0.0, 0.2, 0.0),
            "wiki-section-controller.png",
        ),
        // Front hull: closer on the nose.
        beat(
            Vec3::new(3.5, 2.2, -6.0),
            Vec3::new(0.0, 0.0, -1.0),
            "wiki-section-hull.png",
        ),
        // Thruster: from behind, the plume nozzle toward camera.
        beat(
            Vec3::new(3.5, 2.2, 6.0),
            Vec3::new(0.0, 0.0, 1.5),
            "wiki-section-thruster.png",
        ),
        // Turret: from the right flank where it is mounted.
        beat(
            Vec3::new(6.0, 3.0, 2.0),
            Vec3::new(1.0, 0.6, 0.0),
            "wiki-section-turret.png",
        ),
        // Torpedo bay: from the left flank.
        beat(
            Vec3::new(-6.0, 3.0, 2.0),
            Vec3::new(-1.0, 0.6, 0.0),
            "wiki-section-torpedo-bay.png",
        ),
    ]
}
