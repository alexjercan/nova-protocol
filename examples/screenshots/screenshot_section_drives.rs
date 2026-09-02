//! screenshot_section_drives: the shipped drive family side by side -
//! `wiki-section-drives.png`, plus a closeup of each large drive
//! (`wiki-section-vector-drive.png`, `wiki-section-capital-drive.png`).
//!
//! A separate producer from `screenshot_section_frame` because the subject is
//! SCALE. That walk's showcase ship is sized so a unit section fills the frame
//! with its neighbours still in it; standing a 5x5x3 drive on the same spine
//! would bury every other closeup on it. Here the three drives are three
//! ships on one bench, and the shot that matters is the one that holds all
//! three at once.
//!
//! Each rig is a drive on the one hull cell its bow face needs, because a
//! drive mounts out of its -Z face alone (`drive_mount_points`) and a section
//! with nothing to mate to is a disconnected ship.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - walk every framing, exit
//!   clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write each PNG (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_section_drives --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_section_drives --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[cfg(feature = "debug")]
#[path = "shared/showcase.rs"]
mod showcase;

#[derive(Parser)]
#[command(name = "screenshot_section_drives")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the shipped drive family side by side and a closeup of each large drive. Autopilot-only: posed framings on a fixed bench",
    long_about = None
)]
struct Cli;

/// Where each rig stands on the bench, in world X.
///
/// Read left to right as the family grows. The gaps are three units of clear
/// space rather than a constant pitch: a constant pitch puts the capital's
/// 5-wide body almost against its neighbour while the 1-wide basic swims in
/// air, and the row stops reading as a size comparison.
const BASIC_X: f32 = -8.0;
const VECTOR_X: f32 = -3.0;
const CAPITAL_X: f32 = 4.0;

/// The point the family shot frames and the rig lights: the middle of the
/// occupied span, not the origin.
const BENCH_CENTRE: Vec3 = Vec3::new(-1.0, 0.0, 0.0);

/// Where the camera stands, as a direction from the subject.
///
/// Mostly down the drives' own +Z, because the nozzle is what these parts
/// changed and it is the only face that carries one. Off the axis in X and Y
/// so the bells read as cones rather than as rings, and biased the way the row
/// grows so the capital cannot stand in front of the two behind it.
///
/// Only the driven walk poses a camera, so this is gated with it - the scene
/// itself builds under default features and never reads it.
#[cfg(feature = "debug")]
const BENCH_BEARING: Vec3 = Vec3::new(0.42, 0.24, 1.0);

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        // Nothing on this bench is meant to move: the drives are unpowered and
        // the rigs are free bodies, so a stray impulse would drift a subject
        // out of a framing that was posed frames earlier.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
        app.add_plugins(bench_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_bench);
}

fn setup_bench(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(drive_bench(&game_assets, &sections)));
}

/// One drive on the hull cell it mounts to, as a whole ship standing at `x`.
///
/// The hull sits a half cell behind the drive's bow face: a drive's link
/// points are one per cell on its -Z face at `-cells.z / 2`, and a unit hull
/// carries its own at `+0.5`, so the hull's centre is the drive's bow face
/// minus a half unit.
fn drive_rig(
    sections: &GameSections,
    id: &str,
    drive_id: &str,
    cells_z: f32,
    x: f32,
) -> ScenarioObjectConfig {
    let section = |kind: &str| {
        sections
            .get_section(kind)
            .unwrap_or_else(|| panic!("section '{kind}' not found"))
            .clone()
    };
    let at = |name: &str, kind: &str, position: Vec3| SpaceshipSectionConfig {
        id: name.to_string(),
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section(kind)),
        modifications: vec![],
    };
    let bow_face = -cells_z * 0.5;

    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: id.to_string(),
            position: Vec3::new(x, 0.0, 0.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::None,
            hull: ShipSource::Inline(ShipHull {
                sections: vec![
                    at("drive", drive_id, Vec3::ZERO),
                    at(
                        "mount",
                        "reinforced_hull_section",
                        Vec3::new(0.0, 0.0, bow_face - 0.5),
                    ),
                ],
                ..default()
            }),
            ..default()
        }),
    }
}

/// The bench: the three shipped drives, each on its own mount, in one scene.
fn drive_bench(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let rigs = vec![
        drive_rig(
            sections,
            "basic_drive",
            "basic_thruster_section",
            1.0,
            BASIC_X,
        ),
        drive_rig(
            sections,
            "vector_drive",
            "vector_thruster_section",
            2.0,
            VECTOR_X,
        ),
        drive_rig(
            sections,
            "capital_drive",
            "capital_thruster_section",
            3.0,
            CAPITAL_X,
        ),
    ];

    ScenarioConfig {
        description: "The shipped drive family on one bench for the wiki shots.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            // The scene lights itself: the engine spawns no light, so a
            // scenario that authors none renders black.
            actions: [
                rigs.into_iter()
                    .map(EventActionConfig::SpawnScenarioObject)
                    .collect::<Vec<_>>(),
                ThreePointRig::around("drives", BENCH_CENTRE, 4.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "drive_bench".to_string(),
            "Drive Bench".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// One framing: what it looks at, how far back, and the file it writes.
#[cfg(feature = "debug")]
struct BenchShot {
    subject: Vec3,
    distance: f32,
    path: &'static str,
}

/// The family first, then the two parts that are new this release.
#[cfg(feature = "debug")]
fn bench_shots() -> [BenchShot; 3] {
    [
        // All three at once. The distance holds the whole 15-unit span with
        // air at each end, because the shot IS the comparison: a crop that
        // clips the capital would answer the question it was asked to raise.
        BenchShot {
            subject: BENCH_CENTRE,
            distance: 18.0,
            path: "wiki-section-drives.png",
        },
        BenchShot {
            subject: Vec3::new(VECTOR_X, 0.0, 0.0),
            distance: 8.5,
            path: "wiki-section-vector-drive.png",
        },
        BenchShot {
            subject: Vec3::new(CAPITAL_X, 0.0, 0.0),
            distance: 13.0,
            path: "wiki-section-capital-drive.png",
        },
    ]
}

/// Stand the camera off `shot.subject` on the bench bearing.
///
/// The bench does not turn. Every subject on it already presents the same
/// face, so a turntable would only re-light three identical poses.
#[cfg(feature = "debug")]
fn frame_bench(world: &mut World, shot: &BenchShot) {
    let eye = shot.subject + BENCH_BEARING.normalize() * shot.distance;
    pose_camera(world, eye, shot.subject);
}

/// The driven walk: wait for the bench, then frame-settle-shoot each framing.
#[cfg(feature = "debug")]
fn bench_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the drive bench")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add();

    for shot in bench_shots() {
        let path = shot.path;
        script = script
            .step(format!("frame {path}"))
            .on_enter(move |world: &mut World| frame_bench(world, &shot))
            .until(frames(SETTLE_FRAMES))
            .add()
            // The shot step holds until the PNG is on disk, so the next
            // framing cannot move the camera out from under a pending write.
            .step(format!("shoot {path}"))
            .on_enter(move |world: &mut World| shoot(world, path))
            .until(shot_written(path))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
    }
    script
}
