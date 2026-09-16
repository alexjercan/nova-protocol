//! lesson_combat_torpedoes: the training handbook's demonstration for
//! "Torpedoes" (`assets/base/training/combat_torpedoes.webp`).
//!
//! An ACTION loop (see `shared/lesson.rs`): the launch happens INSIDE the
//! recording. The sheet opens on a boat with its bays shut, the round drops
//! out COLD, the drive catches a moment later, and the rest of the cells are
//! the torpedo climbing away toward the ship it was committed to - which is
//! the lesson's own sentence, in the order it happens.
//!
//! So the camera does not move. It stands off the boat ACROSS the run rather
//! than behind it: the two things the lesson claims - that the round leaves the
//! hull under nothing but the ejector, and that the drive lights only once it
//! is clear - are both read off how far the torpedo has travelled when the
//! plume appears, and a camera looking down the run sees that distance
//! foreshortened to nothing.
//!
//! The set is `hollow::ordnance_hollow`, the same quiet three-ship pocket
//! `screenshot_torpedo_run` uses, and the launch is the production path from
//! the trigger down: the bays fire, the projectile spawns, and one scripted
//! write commits it to the raider, exactly as the player's crosshair lock or
//! the AI's envelope would.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole script, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the sheet (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_combat_torpedoes --features debug
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
#[command(name = "lesson_combat_torpedoes")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's torpedo demonstration", long_about = None)]
struct Cli;

/// The lesson this records for, and so the name of the tiled sheet.
#[cfg(feature = "debug")]
const LESSON: &str = "combat_torpedoes";

/// How far ahead of the BAY the camera looks, in meters along the boat's own
/// heading.
///
/// Not at the bay: a torpedo leaves and does not come back, so a view centred
/// on the hull spends most of its cells watching the round crawl off one edge.
/// Centred ahead of it, the boat sits at one side of the cell and the round
/// crosses the middle.
#[cfg(feature = "debug")]
const RUN_LEAD: f32 = 55.0;
/// How far the camera stands off that point, square across the ejection line.
///
/// Wide enough to hold the whole of the first two seconds: the ejector puts
/// the round out at about 60 m/s before the drive adds to it, so the run is a
/// good hundred and fifty meters long by the last cell, and a closer eye ends
/// the sheet on empty space.
#[cfg(feature = "debug")]
const RUN_STANDOFF: f32 = 170.0;
/// How far above it the camera rides. A shallow downward look, so the run is
/// seen against the field rather than against empty sky.
#[cfg(feature = "debug")]
const RUN_RISE: f32 = 16.0;

/// Cells before the bays fire.
///
/// The sheet has to open on a boat that has NOT launched, or the loop is a
/// round already in flight and the drop the lesson is about is off the front
/// of it.
#[cfg(feature = "debug")]
const LEAD_IN_CELLS: u32 = 3;

/// Where the boat's bay is, and which way the round will leave it.
///
/// Measured off the LIVE hull rather than written down, and off the BAY rather
/// than off the ship. A bay ejects along its own mount axis, and on this boat
/// that axis is broadside - square across a hull that is itself pointed at the
/// raider. The guidance only bends the round onto the target once the drive
/// has caught, so for the two seconds this sheet is long the run is the BAY's
/// line. The first cut of this frame took the ship's heading for the run and
/// spent twenty cells on rocks while the round left behind the camera.
///
/// Returns the bay, the line the round leaves on, and a direction square
/// across that line for the camera to stand on.
#[cfg(feature = "debug")]
fn the_bay(world: &mut World) -> Option<(Meters3, Vec3, Vec3)> {
    let lance = hollow::ship_by_id(world, hollow::LANCE_ID)?;
    let bay = world
        .query_filtered::<(&GlobalTransform, &ChildOf), With<TorpedoSectionMarker>>()
        .iter(world)
        .find(|(_, ChildOf(parent))| *parent == lance)
        .map(|(transform, _)| *transform)?;
    let run = bay.forward().as_vec3().normalize_or_zero();
    let across = run.cross(Vec3::Y).normalize_or_zero();
    Some((Meters3::from_engine(bay.translation()), run, across))
}

/// Stand the camera across the torpedo's run, holding still.
#[cfg(feature = "debug")]
fn frame_the_run(world: &mut World) {
    let Some((bay, run, across)) = the_bay(world) else {
        warn!("lesson_combat_torpedoes: no torpedo bay on the boat to frame");
        return;
    };
    let subject = bay + Meters3(run * RUN_LEAD);
    // A zero-arc sweep is how an action loop holds a camera (see
    // `shared/lesson.rs`), and its bearing IS the across-the-run direction:
    // the sweep stands its eye at `range * (sin b, _, cos b)` from the
    // subject, so the bearing that reproduces `across` is its own atan2.
    world.insert_resource(LessonSweep::new(
        subject,
        Meters(RUN_STANDOFF),
        Meters(RUN_RISE),
        across.x.atan2(across.z).to_degrees(),
        0.0,
    ));
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            lesson_profile(),
        ));
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(torpedo_lesson_script());
        // NOTHING is frozen here, unlike the other action loops: the subject
        // is a projectile, and a blanket freeze turns it static the frame it
        // spawns. The set is the quiet one, so what the freeze would have
        // stopped - rocks and two parked hulls - barely moves in two seconds
        // anyway.
        app.add_systems(Update, sweep_lesson_camera);
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    commands.trigger(LoadScenario(hollow::ordnance_hollow(&game_assets, &ships)));
}

/// Settle the pocket, frame the run, then launch inside the recording.
#[cfg(feature = "debug")]
fn torpedo_lesson_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the ordnance hollow")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle the ordnance hollow")
        .until(elapsed(1.0))
        .add()
        // The camera has left the player's ship, so its instruments are chrome
        // over a picture of somebody else's ordnance - the same call
        // `screenshot_torpedo_run` makes for the same reason.
        .step("clear the screen and frame the run")
        .on_enter(|world: &mut World| {
            hollow::hud_cinematic(world);
            frame_the_run(world);
        })
        .until(elapsed(0.5))
        .add()
        .step("open the sheet on a boat with its bays shut")
        .on_enter(|world: &mut World| sheet_start(world, LESSON, LESSON_GRID))
        .until(frames(LEAD_IN_CELLS))
        .add()
        .step("loose the torpedo inside the recording")
        .on_enter(hollow::loose_torpedoes)
        .until(hollow::torpedo_salvo_in_flight(
            hollow::EXPECTED_TORPEDO_COUNT,
        ))
        .deadline(6.0)
        .add()
        // A torpedo's target is decided exactly once, the frame after launch.
        // Committing it here is what makes the rest of the sheet a round that
        // STEERS rather than one coasting off on its ejector heading.
        .step("commit it to the raider")
        .on_enter(hollow::commit_torpedoes)
        .until(sheet_written(LESSON))
        .deadline(60.0)
        .add()
        // Two seconds is well inside the run-in, so a sheet that closed with
        // nothing in flight means the round died on launch - and that ships as
        // a demonstration of an empty pocket rather than as an error.
        .step("the sheet caught the round")
        .on_enter(|world: &mut World| hollow::assert_salvo_still_live(world, 0.0, 0))
        .until(frames(1))
        .add()
}
