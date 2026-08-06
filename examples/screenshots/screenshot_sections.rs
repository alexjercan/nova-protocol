//! screenshot_sections: capture the wiki ship-section detail shots - a
//! closeup of each section type on one built ship.
//!
//! It builds a ship carrying all five section types (controller, hull, thruster,
//! turret, torpedo bay) and drives an autopilot script that turns the ship and
//! frames a closeup of each, writing `wiki-section-<kind>.png`. The scene is
//! frozen so every section sits still for its shot.
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
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`) - the script is
//! the whole example, and `NOVA_CAPTURE` only decides whether its shot steps
//! write a PNG:
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - turn the ship through every
//!   closeup, reach Playing, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot each closeup (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
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
        // Probe wiring (each plugin is inert without its NOVA_PERF_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::nova_timeline());
        app.add_plugins(nova_probe::nova_invariants());
        // Clean frames at a known 16:9: force the window size, drop the dev
        // overlays and the HUD chrome (the showcase carries no player HUD, so
        // the fps/version bar is just clutter).
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        // The turntable sets the ship's rotation per shot, so nothing may push
        // it back - but only on a capture run, so a plain `cargo run` keeps its
        // physics.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
        app.add_plugins(section_script());
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
/// sits at a known spot the script's camera can frame:
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

/// Seconds a step may sit before it is called a stall. Sized with headroom for
/// a slow software-rendered CI GPU (llvmpipe). An expiry is an error exit
/// naming the step, so a run that never loads the showcase fails loudly instead
/// of producing an unframed shot.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 30.0;

/// A closeup of each section: the ship yaws to present it, the camera holds one
/// bearing. Mount points match the ship layout in `section_ship`.
#[cfg(feature = "debug")]
fn section_shots() -> [SectionShot; 5] {
    [
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
    ]
}

/// The whole driven walk: wait for the showcase, then present-settle-shoot each
/// section in turn.
///
/// Written as autopilot steps rather than a beat list so the turn, the framing
/// and the shot they produce read top-to-bottom in one place, and so the smoke
/// path and the capture path are the SAME walk - the only difference is whether
/// `shoot` is armed.
#[cfg(feature = "debug")]
fn section_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        // The showcase is live once the loader has spawned its camera; posing
        // before that poses nothing.
        .step("wait for the section showcase")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add();

    for shot in section_shots() {
        let path = shot.path;
        script = script
            .step(format!("present {path}"))
            .on_enter(move |world: &mut World| present_section(world, &shot))
            .until(frames(SETTLE_FRAMES))
            .add()
            // The shot step holds until the PNG is on disk, so the next turn
            // cannot swing the ship out from under a pending write.
            .step(format!("shoot {path}"))
            .on_enter(move |world: &mut World| shoot(world, path))
            .until(shot_written(path))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
    }
    script
}

/// Turn the ship so `shot.faces` points at the camera, then frame the section.
#[cfg(feature = "debug")]
fn present_section(world: &mut World, shot: &SectionShot) {
    let yaw = presenting_yaw(shot.faces);
    yaw_ship(world, yaw);
    // The mount rides round with the hull, so the framed point is the yawed one
    // - not the authored coordinate.
    let subject = yaw * shot.mount;
    let eye = subject + CAMERA_BEARING.normalize() * shot.distance;
    pose_camera(world, eye, subject);
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

/// Set the showcase ship's rotation. The scene is frozen on a capture run, so
/// this sticks for the step's settle frames and nothing drifts it back.
#[cfg(feature = "debug")]
fn yaw_ship(world: &mut World, yaw: Quat) {
    let mut ships = world.query_filtered::<&mut Transform, With<SpaceshipRootMarker>>();
    for mut transform in ships.iter_mut(world) {
        transform.rotation = yaw;
    }
}
