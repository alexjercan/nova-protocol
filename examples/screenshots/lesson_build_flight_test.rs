//! lesson_build_flight_test: the training handbook's demonstration for "Fly
//! what you built" (`assets/base/training/build_flight_test.webp`).
//!
//! An ACTION loop (see `shared/lesson.rs`), and the only lesson whose walk
//! crosses from the editor into the game: a short ship is built in the sandbox
//! by real pointer gestures, Play hands it to the sandbox range, and the sheet
//! records it leaving under its own drive. Nothing here is a stand-in - the
//! hull in the cells is the hull the gestures bolted together a second
//! earlier, which is the whole claim of the lesson.
//!
//! The camera is FIXED IN THE WORLD rather than riding the hull, and that is
//! what makes the loop say "leaving". A chase rig holds the ship at the same
//! size in every cell, so a departure under power looks like a parked ship
//! with a lit tail; a camera standing still watches it go, and the sandbox
//! range's own asteroid belt gives the eye something to measure the going
//! against.
//!
//! The lead-in cells are at REST, with the drive cold. The lesson is about
//! taking a new ship out and running it up, so the sheet has to open on the
//! ship that has not moved yet.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - build, launch, fly, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the sheet (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_build_flight_test --features debug
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

// The pointer gestures, shared with the other editor walks. Script-only, so
// the whole module sits behind one gate here.
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, sweep_lesson_camera, LessonSweep, LESSON_GRID};
#[cfg(feature = "debug")]
use ui_walk::{
    count_sections, pose_editor_camera, the_build_camera_is_posed, the_editor_can_play,
    the_editor_is_inside_a_ship, Gestures,
};

#[derive(Parser)]
#[command(name = "lesson_build_flight_test")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's flight-test demonstration", long_about = None)]
struct Cli;

/// The lesson this records for, and so the name of the tiled sheet.
#[cfg(feature = "debug")]
const LESSON: &str = "build_flight_test";

/// What the gestures leave on the frame: a controller, two hulls and a
/// thruster. The same short ship the sections lesson photographs,
/// deliberately - a reader meets it in the editor two lessons earlier and then
/// watches it fly.
#[cfg(feature = "debug")]
const BUILT_SECTIONS: usize = 4;

/// The panel the game puts up while a scenario swaps in
/// (`crates/nova_core/src/loading_screen.rs`).
///
/// The walk waits for it to appear and then to go, which is the only honest
/// test that the range is UP. `player_ship_present` is true through the whole
/// load - the editor hands a player ship straight across - so a settle counted
/// from the Play click alone spends its frames photographing a loading screen,
/// which is what the first cut of this producer recorded.
#[cfg(feature = "debug")]
const LOAD_SCREEN: &str = "Scenario Loading Screen";

/// Frames the sandbox range is given to settle after the panel comes down.
#[cfg(feature = "debug")]
const RANGE_SETTLE_FRAMES: u32 = 30;
/// How long the walk waits for the range to come up, in real seconds.
#[cfg(feature = "debug")]
const RANGE_DEADLINE_SECS: f32 = 90.0;

/// Cells before the drive lights.
///
/// The sheet opens on a ship at rest: a loop that started under power would
/// show a ship flying, and the lesson is about TAKING ONE OUT.
#[cfg(feature = "debug")]
const LEAD_IN_CELLS: u32 = 3;

/// How far down its own departure the camera looks, in meters ahead of where
/// the ship starts.
///
/// Enough that the ship opens the sheet a quarter of the way across the cell
/// and closes near the far edge, rather than starting cut off by the near one.
///
/// The run this is set against is MEASURED off the capture, not derived: one
/// basic thruster on a four-section frame carries this hull about 95 m over
/// the seventeen cells that have the drive lit. Derived numbers were wrong by
/// a factor of two here - the engine eases its throttle in, and a departure
/// from REST spends most of the sheet still gathering speed.
#[cfg(feature = "debug")]
const DEPARTURE_LEAD: f32 = 45.0;
/// How far the camera stands off that point, square across the departure.
///
/// Set by the run rather than by the ship: the whole 95 m has to cross most of
/// the cell without leaving it, and at this range that run is about 40 degrees
/// of a frame whose half-width is 36. Close enough, too, that a 40 m hull is a
/// quarter of the cell wide instead of a chip of metal on a starfield.
#[cfg(feature = "debug")]
const DEPARTURE_STANDOFF: f32 = 120.0;
/// How far above it the camera rides.
#[cfg(feature = "debug")]
const DEPARTURE_RISE: f32 = 22.0;

/// Stand the camera across the ship's departure, holding still.
///
/// Measured off the live hull: the sandbox range puts the ship where it puts
/// it, and a written-down eye is a constant that stops framing anything the
/// day the range moves.
#[cfg(feature = "debug")]
fn frame_the_departure(world: &mut World) {
    let hull = world
        .query_filtered::<&GlobalTransform, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
        .iter(world)
        .next()
        .copied();
    let Some(hull) = hull else {
        warn!("lesson_build_flight_test: no built ship on the range to frame");
        return;
    };
    let ship = Meters3::from_engine(hull.translation());
    let run = hull.forward().as_vec3().normalize_or_zero();
    let across = run.cross(Vec3::Y).normalize_or_zero();
    let subject = ship + Meters3(run * DEPARTURE_LEAD);
    // A zero-arc sweep is how an action loop holds a camera (see
    // `shared/lesson.rs`); its bearing is the across-the-run direction's own
    // atan2, because the sweep stands its eye at `range * (sin b, _, cos b)`.
    world.insert_resource(LessonSweep::new(
        subject,
        Meters(DEPARTURE_STANDOFF),
        Meters(DEPARTURE_RISE),
        across.x.atan2(across.z).to_degrees(),
        0.0,
    ));
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same app the game's Sandbox button opens.
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            lesson_profile(),
        ));
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(flight_test_script());
        // Inert until the departure beat inserts its sweep - which is after
        // Play, so the editor's own free-fly camera is left alone for the
        // whole build.
        app.add_systems(Update, sweep_lesson_camera);
    }

    app.run()
}

/// Menu -> editor -> build a short ship -> leave it -> Play -> fly it away.
#[cfg(feature = "debug")]
fn flight_test_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .enter(GameStates::Loading)
        .until(state_is(GameStates::MainMenu))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .click("leave for the editor", "Sandbox Button")
        .step("reach the editor")
        .until(state_is(GameStates::Playing))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("pose the editor camera off the axis")
        .on_enter(pose_editor_camera)
        .until(the_build_camera_is_posed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .click("open the Add menu", "Add Menu Button")
        .click("create the ship", "Add Ship Button")
        .step("the blank ship is entered")
        .until(the_editor_is_inside_a_ship())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .arm("arm the controller", "basic_controller_section")
        .found("found the ship")
        // ALONG +Z, and it has to be. A thruster's line of thrust is its own
        // section forward, a part placed on face F ends up thrusting along -F,
        // and the flight layer only budgets the burn against the engines
        // aligned with the hull's -Z (`is_forward_aligned`). So a spine built
        // out along +X carries an engine that pushes along -X, which the main
        // drive never commands: the first cut of this producer built exactly
        // that and photographed a ship at a dead stop with the throttle open.
        .arm("arm the hull", "reinforced_hull_section")
        .place("hull behind the controller", Vec3::ZERO, Vec3::Z)
        .place("hull behind that", Vec3::new(0.0, 0.0, 1.0), Vec3::Z)
        .arm("arm the thruster", "basic_thruster_section")
        .place("thruster on the tail", Vec3::new(0.0, 0.0, 2.0), Vec3::Z)
        .step("the frame carries its parts")
        .on_enter(|world: &mut World| {
            let sections = count_sections(world);
            assert_eq!(
                sections, BUILT_SECTIONS,
                "the lesson flies a controller, two hulls and a thruster"
            );
            info!("lesson build: {sections} sections on the frame");
        })
        .add()
        .step("press Escape to put the part down")
        .on_enter(press_key(KeyCode::Escape))
        .until(editor_tool_is(EditorTool::Select))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("release Escape")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        // Play flies the whole SCENARIO, so the editor refuses it from inside
        // a ship: the walk steps back out to the scenario row first, the way
        // `screenshot_editor` does.
        .click("leave the ship", "Scene Row scenario")
        .step("Play is reachable")
        .until(the_editor_can_play())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .click("launch the built ship", "Play Button")
        .step("the range takes the ship")
        .until(ui_node_present(LOAD_SCREEN))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // `predicate::not`, spelled out: bevy's prelude carries a
        // run-condition `not` of its own, and the unqualified name is that one.
        .step("wait the scenario in")
        .until(and(
            nova_autopilot::predicate::not(ui_node_present(LOAD_SCREEN)),
            player_ship_present(),
        ))
        .deadline(RANGE_DEADLINE_SECS)
        .add()
        .step("let the range settle")
        .until(frames(RANGE_SETTLE_FRAMES))
        .add()
        // The camera has left the ship's own rig, so the HUD's instruments are
        // chrome over a picture of somebody else flying - and the subject here
        // is the hull, not what it reads.
        .step("clear the screen and frame the departure")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            frame_the_departure(world);
        })
        .until(elapsed(0.5))
        .add()
        .step("open the sheet on a ship at rest")
        .on_enter(|world: &mut World| sheet_start(world, LESSON, LESSON_GRID))
        .until(frames(LEAD_IN_CELLS))
        .add()
        // SPACE, not the `main_drive` action.
        //
        // The editor gives a placed thruster its own key (`default_binds_for`
        // -> Space for a thruster), and a BOUND engine is excluded from the
        // main drive's allocation set (`Without<SpaceshipThrusterInputBinding>`
        // in `manual_burn_system`) - it answers its own key instead. The
        // action's first keyboard source is W, so a beat that pressed
        // `main_drive` on this hull commanded an allocation set with nothing
        // in it, which is a throttle open on a ship that never moves.
        .step("run the drive up inside the recording")
        .on_enter(press_key(KeyCode::Space))
        .until(sheet_written(LESSON))
        .deadline(60.0)
        .add()
        .step("cut the drive")
        .on_enter(release_key(KeyCode::Space))
        .until(frames(1))
        .add()
}
