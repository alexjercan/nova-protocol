//! lesson_flight_aim: the training handbook's demonstration for "Turn, then
//! thrust" (`assets/base/training/flight_aim.webp`).
//!
//! A corvette holding a heading with its main drive lit, recorded as the 4x5
//! sprite sheet the lesson authors. The plume leaves the tail along the line
//! the nose points down, which is the lesson's whole claim, and the camera
//! drifts across it on a closed arc so the sheet can play on a cycle - see
//! `shared/lesson.rs` for why a lesson loop is a camera move rather than a
//! clip of flight.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - fly the whole script, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the sheet (staged under
//!   `NOVA_CAPTURE_DIR`). `scripts/capture-lesson-media.sh` runs this and
//!   packages the result into the base bundle.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_flight_aim --features debug
//! ```

#[path = "shared/hollow.rs"]
mod hollow;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, sweep_lesson_camera, LessonSweep, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_flight_aim")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's turn-then-thrust demonstration", long_about = None)]
struct Cli;

/// The lesson this records for. The sheet is named for it, so the tiled file
/// is `flight_aim.png` - the base bundle path the lesson already declares.
#[cfg(feature = "debug")]
const LESSON: &str = "flight_aim";

/// Where the camera stands off the parked player, which sits at the origin.
///
/// Set by the SUBJECT'S SIZE, not by taste: clad, `block_gunship` is
/// 85 x 50 x 48 m - a 54.7 m bounding radius - so an eye at 70 m is a close-up
/// of hull plating with the nose and the drive both out of frame. This sits at
/// about 2.7 times the radius, which holds the whole corvette in a cell.
#[cfg(feature = "debug")]
const SWEEP_RANGE: Meters = Meters(150.0);
/// How far above the hull the camera rides: a slight downward look, so the
/// deck reads and the drive is still side-on.
#[cfg(feature = "debug")]
const SWEEP_HEIGHT: Meters = Meters(40.0);
/// The bearing the sweep is centred on: the ship's rear quarter. It noses down
/// world -Z, so a camera behind and to starboard sees the nose line and the
/// plume leaving along it.
#[cfg(feature = "debug")]
const SWEEP_BEARING_DEGREES: f32 = 35.0;
/// Half the sweep's width. Small: this is parallax, not a tour. Five degrees
/// at this range is a 13 m swing either way - enough that the hull turns
/// against the rocks behind it, little enough that the ship holds its place in
/// the cell.
#[cfg(feature = "debug")]
const SWEEP_ARC_DEGREES: f32 = 5.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            lesson_profile(),
        ));
        app.add_plugins(flight_aim_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        // The subject holds its pose while the camera moves: the drive is lit
        // for the plume, and a ship that actually accelerated would leave the
        // frame the sweep is measured in. `freeze_bodies` is idempotent, and
        // the sweep is inert until its step inserts the resource.
        app.add_systems(Update, (freeze_bodies, sweep_lesson_camera));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    commands.trigger(LoadScenario(hollow::ordnance_hollow(&game_assets, &ships)));
}

/// Light the drive, then record one period of the sweep across it.
#[cfg(feature = "debug")]
fn flight_aim_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the hollow")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle the parked ship")
        .on_enter(hide_hud)
        .until(elapsed(1.0))
        .add()
        .step("light the main drive")
        .on_enter(press_action("main_drive"))
        .until(elapsed(0.8))
        .add()
        // One step: the grid is the length, so the sheet closes itself at its
        // own twelve frames and the wait is the ack that it is on disk.
        .step("record the turn-then-thrust sheet")
        .on_enter(|world: &mut World| {
            world.insert_resource(LessonSweep::new(
                Meters3::ZERO,
                SWEEP_RANGE,
                SWEEP_HEIGHT,
                SWEEP_BEARING_DEGREES,
                SWEEP_ARC_DEGREES,
            ));
            sheet_start(world, LESSON, LESSON_GRID);
        })
        .until(sheet_written(LESSON))
        .deadline(60.0)
        .add()
        .step("release the main drive")
        .on_enter(release_action("main_drive"))
        .add()
}
