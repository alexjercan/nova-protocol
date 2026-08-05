//! screenshot_sections: capture the wiki ship-section detail shots - a
//! closeup of each section type on one built ship - using the screenshot reel.
//!
//! It builds a ship carrying all five section types (controller, hull, thruster,
//! turret, torpedo bay) and steps the reel camera to a closeup of each, writing
//! `wiki-section-<kind>.png`. The scenario camera is posed per beat by the reel
//! plugin, and the scene is frozen so every section sits still for its shot.
//!
//! The ship is the ENGINE's own section prototypes, not a Kenney hull: these
//! shots document what each section type IS, so a reader has to be able to tell
//! the parts apart. A pretty hull would hide exactly the thing being shown.
//!
//! Framing is a turntable, not a camera fly-around. The photo rig
//! (`shared/kit.rs`) is fixed in WORLD space, so orbiting the camera would light
//! each of the five shots differently - one crisp, one flat, one backlit. The
//! camera instead stays on one bearing inside the rig's good wedge and the SHIP
//! yaws to bring each section round to it, so all five closeups get the same
//! key, the same rim and the same read.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/reel NOVA_REEL=1 \
//!   cargo run --example screenshot_sections --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_sections --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[path = "shared/kit.rs"]
mod kit;

#[derive(Parser)]
#[command(name = "screenshot_sections")]
#[command(version = "1.0.0")]
#[command(about = "Capture the wiki ship-section detail shots", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Smoke path: reach Playing on the built scene and exit clean.
        app.add_plugins(nova_autopilot());
        // Capture path: pose the camera at each section and shoot.
        app.add_plugins(nova_reel(section_beats()));
    }

    app.run()
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
            // The scene lights itself: the engine spawns no light, so a
            // scenario that authors none renders black.
            actions: [
                vec![EventActionConfig::SpawnScenarioObject(
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
                ThreePointRig::around("showcase", Vec3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..Default::default()
    }
}

/// Where the camera stands, as a direction from the section it is framing.
///
/// Picked off the rig in `shared/kit.rs`, not off the ship: the key comes from
/// `(-6, 5, 6)` and the rim from `(3, 4, -8)`, so a camera on the far side of
/// the key from the rim gets the key raking ACROSS the subject (form, not a
/// flat front-lit face) while the rim draws the far edge against the skybox.
/// Standing on the key's own bearing would light every closeup flat.
const CAMERA_BEARING: Vec3 = Vec3::new(0.78, 0.36, 0.51);

/// One closeup: which section, which way it has to be turned, how close.
#[cfg(feature = "debug")]
struct SectionShot {
    /// The section's spot on the ship, in the ship's own space - the same
    /// coordinates `section_ship` mounts it at.
    mount: Vec3,
    /// The direction, in the ship's own space, this section's identifying face
    /// points. The turntable yaws the ship until this points at the camera:
    /// the thruster nozzle, the turret barrel and the bay doors are each on a
    /// different face, and each is the reason its shot exists.
    faces: Vec3,
    /// Camera distance. Small enough that the section fills the frame, large
    /// enough that its neighbours still place it on a ship.
    distance: f32,
    path: &'static str,
}

/// A closeup of each section: the ship yaws to present it, the camera holds one
/// bearing. Mount points match the ship layout in `section_ship`.
#[cfg(feature = "debug")]
fn section_beats() -> Vec<ReelBeat> {
    let shots = [
        // Controller: the bridge, read across the spine from the front quarter.
        SectionShot {
            mount: Vec3::ZERO,
            faces: Vec3::NEG_Z,
            distance: 5.0,
            path: "wiki-section-controller.png",
        },
        // Front hull: plating and frame, taken off the nose quarter. Not the
        // broadside - that puts the turret barrel straight across the subject.
        SectionShot {
            mount: Vec3::new(0.0, 0.0, -1.0),
            faces: Vec3::new(-0.35, 0.0, -1.0),
            distance: 4.0,
            path: "wiki-section-hull.png",
        },
        // Thruster: off the nozzle's axis, not down it. Dead astern points the
        // plume at the lens and the bloom eats the bell that is the subject.
        SectionShot {
            mount: Vec3::new(0.0, 0.0, 2.0),
            faces: Vec3::new(0.55, 0.0, 1.0),
            distance: 4.4,
            path: "wiki-section-thruster.png",
        },
        // Turret: the flank it is mounted on, turned enough that the barrel
        // rakes across the frame instead of foreshortening into a dot.
        SectionShot {
            mount: Vec3::new(1.0, 0.0, 0.0),
            faces: Vec3::new(1.0, 0.0, 0.55),
            distance: 3.6,
            path: "wiki-section-turret.png",
        },
        // Torpedo bay: the opposite flank, so the ship turns most of a
        // revolution. Angled to keep the launch slot on the top face readable.
        SectionShot {
            mount: Vec3::new(-1.0, 0.0, 0.0),
            faces: Vec3::new(-1.0, 0.0, -0.5),
            distance: 4.0,
            path: "wiki-section-torpedo-bay.png",
        },
    ];
    shots.into_iter().map(section_beat).collect()
}

/// Turn the ship so `shot.faces` points at the camera, then frame the section.
#[cfg(feature = "debug")]
fn section_beat(shot: SectionShot) -> ReelBeat {
    ReelBeat::new(shot.path).apply(move |world: &mut World| {
        let yaw = presenting_yaw(shot.faces);
        yaw_ship(world, yaw);
        // The mount rides round with the hull, so the framed point is the
        // yawed one - not the authored coordinate.
        let subject = yaw * shot.mount;
        let eye = subject + CAMERA_BEARING.normalize() * shot.distance;
        reel_pose_camera(world, eye, subject);
    })
}

/// The yaw that brings `faces` round to the camera. Both vectors are flattened
/// onto the ground plane first: the rig's height is what puts the camera above
/// the ship, and tilting the hull to meet it would read as a ship adrift rather
/// than a part on a bench.
#[cfg(feature = "debug")]
fn presenting_yaw(faces: Vec3) -> Quat {
    let flatten = |v: Vec3| Vec3::new(v.x, 0.0, v.z).normalize_or_zero();
    let (from, to) = (flatten(faces), flatten(CAMERA_BEARING));
    if from == Vec3::ZERO || to == Vec3::ZERO {
        return Quat::IDENTITY;
    }
    Quat::from_rotation_arc(from, to)
}

/// Set the showcase ship's rotation. The scene is frozen for the reel, so this
/// sticks for the beat's settle frames and nothing drifts it back.
#[cfg(feature = "debug")]
fn yaw_ship(world: &mut World, yaw: Quat) {
    let mut ships = world.query_filtered::<&mut Transform, With<SpaceshipRootMarker>>();
    for mut transform in ships.iter_mut(world) {
        transform.rotation = yaw;
    }
}
