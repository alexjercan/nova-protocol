//! lesson_build_rail: the two SHIPBUILDING stills whose subject is the editor
//! RAIL rather than the ship on the stage - `build_readout` (the stat block and
//! the limit note under it) and `build_stacking` (the same block refusing to
//! move when a second flight computer goes on).
//!
//! Its own producer rather than two more frames on `lesson_build_sections`,
//! which is already at the three-frame cap the example catalog sets
//! (`Cargo.toml`). The split falls on a seam: that walk photographs a SHIP
//! growing, and this one photographs what the rail SAYS about it.
//!
//! ## One hull, photographed twice
//!
//! The stacking lesson claims a number does not change, and a claim about a
//! number that does not change is only worth a picture if the reader can see
//! it was the same number before. So both stills come off the same build: a
//! five-part spine is shot, one flight computer is bolted to it, and the spine
//! is shot again. The walk reads the rail's own text at both stops and asserts
//! the turn ceiling is character-for-character what it was
//! (`crates/nova_editor/src/readout.rs`), so a change in how the ceiling is
//! derived fails this run instead of shipping a still that contradicts its
//! lesson.
//!
//! ## The hull is long on purpose
//!
//! The structural ceiling is `LOAD_LIMIT / arm`
//! (`crates/nova_ship/src/physics/attitude.rs`), so a SHORT hull has a high one
//! and can end up torque-limited - which is the case where a second computer
//! DOES buy turn rate, and the opposite of what the lesson says. A spine four
//! cells long keeps the arm big enough that the structure binds, and the walk
//! asserts the note says so rather than trusting the geometry.
//!
//! ## The interface is photographed at twice its scale
//!
//! The rail is about 200 of 1920 pixels wide, and the handbook draws a still
//! into a pane about half the width it was shot at, so the stat block would
//! arrive as texture rather than as numbers. `UiScale` is raised for the shots
//! only - the same widgets carrying the same text, laid out larger. The
//! scale goes up for each frame and comes straight back down, so every gesture
//! happens at the scale the editor's own hit testing was tuned at.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - build the ship, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the two PNGs (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_build_rail --features debug
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

// The pointer gestures, shared with the other editor walks.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use ui_walk::{
    count_sections, pose_editor_camera, the_build_camera_is_posed, the_editor_is_inside_a_ship,
    Gestures, EDITOR_EYE, EDITOR_LOOK,
};

#[derive(Parser)]
#[command(name = "lesson_build_rail")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's build readout demonstrations", long_about = None)]
struct Cli;

/// The still for "The build readout".
#[cfg(feature = "debug")]
const READOUT_SHOT: &str = "build_readout.png";
/// The still for "A second flight computer".
#[cfg(feature = "debug")]
const STACKING_SHOT: &str = "build_stacking.png";

/// The rail node carrying the stat block, and the one carrying the line under
/// it. Read by NAME rather than by marker: both markers are `pub(crate)` to
/// `nova_editor`, and the name is the same handle the pointer gestures use.
#[cfg(feature = "debug")]
const READOUT_NODE: &str = "Ship Readout";
/// The line under the block.
#[cfg(feature = "debug")]
const NOTE_NODE: &str = "Ship Readout Note";

/// What the note has to say for the stacking lesson to be about this hull.
///
/// The torque-limited hull is the one where a second computer DOES raise the
/// ceiling, so a build that drifted into it would make the next still a
/// picture of the opposite claim.
#[cfg(feature = "debug")]
const STRUCTURE_NOTE: &str = "structure-limited - shorten the hull";

/// The labels the readout lesson says are in the block, in the order it says
/// they are in.
#[cfg(feature = "debug")]
const READOUT_LABELS: [&str; 5] = ["Turn", "Mass", "Thrust", "HP", "Parts"];

/// The spine, before the second computer: a controller, three hulls and a
/// thruster.
#[cfg(feature = "debug")]
const SPINE_SECTIONS: usize = 5;
/// The same spine with the second computer on it.
#[cfg(feature = "debug")]
const STACKED_SECTIONS: usize = 6;

/// The middle of the spine: five cells of ship growing out of a controller at
/// the ship's own origin. One cell is ten meters.
#[cfg(feature = "debug")]
const SPINE_LOOK: Meters3 = Meters3::new(20.0, 0.0, 0.0);

/// The same middle once a computer sits a cell above the spine.
#[cfg(feature = "debug")]
const STACKED_LOOK: Meters3 = Meters3::new(20.0, 4.0, 0.0);

/// How far in the shot camera closes from the BUILD pose.
///
/// Further out than the `lesson_build_sections` stills stand, and for the
/// opposite reason: those frames are about the ship, and these two are about
/// the rail beside it, so the ship is the thing that gives up room. Far enough
/// that the hull clears the inspector as well as the rail - at twice the scale
/// the two panels between them leave about a third of the width, and a hull
/// running under either edge reads as a screenshot taken carelessly.
#[cfg(feature = "debug")]
const SHOT_CLOSE_IN: f32 = 1.18;

/// What the interface is scaled to for the shots.
///
/// Two, not more: at three the rail and the inspector between them leave no
/// stage, and the picture stops being an editor with a ship in it.
#[cfg(feature = "debug")]
const SHOT_UI_SCALE: f32 = 2.0;

/// How close the camera has to be to the shot pose before the shot is taken,
/// as an engine world-unit figure - the epsilon the build pose uses.
#[cfg(feature = "debug")]
const POSE_EPSILON: f32 = 1e-2;

/// The turn ceiling the rail printed with ONE computer on the hull, kept for
/// the assertion the stacking lesson exists to make.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct TurnWithOneComputer(String);

/// Where a shot camera stands: the build pose's bearing, closed in on `look`.
#[cfg(feature = "debug")]
fn shot_eye_on(look: Meters3) -> Meters3 {
    look + Meters3((EDITOR_EYE - EDITOR_LOOK).0 * SHOT_CLOSE_IN)
}

/// Pin the shot pose. The editor camera is a free-fly WASD camera whose
/// controller rewrites the Transform every frame, so this pins a
/// `ScriptedCameraPose` rather than writing the Transform once.
#[cfg(feature = "debug")]
fn close_in_on(world: &mut World, look: Meters3) {
    let camera = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the editor is up, so it has a 3D camera");
    world.entity_mut(camera).insert(ScriptedCameraPose {
        position: shot_eye_on(look),
        look_at: look,
    });
}

/// Advance once the camera has REACHED the shot pose - the loader's enforcer
/// applies a pinned pose a system later, and a shot taken before then is of the
/// build framing the step exists to leave.
#[cfg(feature = "debug")]
fn the_camera_is_posed_on(
    look: Meters3,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    let eye = shot_eye_on(look).to_engine();
    std::sync::Arc::new(move |world: &World| {
        world
            .try_query_filtered::<&Transform, With<Camera3d>>()
            .is_some_and(|mut cameras| {
                cameras
                    .iter(world)
                    .any(|camera| camera.translation.abs_diff_eq(eye, POSE_EPSILON))
            })
    })
}

/// What a named UI node currently reads.
#[cfg(feature = "debug")]
fn text_of(world: &mut World, name: &str) -> String {
    let mut nodes = world.query::<(&Name, &Text)>();
    nodes
        .iter(world)
        .find(|(node, _)| node.as_str() == name)
        .map(|(_, text)| text.0.clone())
        .unwrap_or_else(|| panic!("the editor rail is up, so it carries a node named {name}"))
}

/// The number out of a turn line, for comparing two of them.
///
/// Parsed off the rail's own text rather than derived again here: the still
/// shows what the rail printed, so the assertion has to be about that and not
/// about a second opinion computed beside it.
#[cfg(feature = "debug")]
fn ceiling_of(line: &str) -> f32 {
    line.split_whitespace()
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("the rail prints a turn ceiling, and this reads {line:?}"))
}

/// The rail's turn line: the first line of the stat block.
#[cfg(feature = "debug")]
fn turn_line(world: &mut World) -> String {
    text_of(world, READOUT_NODE)
        .lines()
        .next()
        .unwrap_or_default()
        .to_string()
}

/// Lay the interface out at reading size for a shot.
#[cfg(feature = "debug")]
fn scale_the_interface_up(world: &mut World) {
    world.insert_resource(UiScale(SHOT_UI_SCALE));
}

/// Put it back at the scale the pointer gestures work at.
///
/// Not cosmetic housekeeping: a gesture aims at a widget's rect, and at twice
/// the scale the rail's own buttons have moved out from under the aim the walk
/// computed, so the next click lands on the neighbouring menu. The scale goes
/// up for the frame and comes straight back down.
#[cfg(feature = "debug")]
fn scale_the_interface_back(world: &mut World) {
    world.insert_resource(UiScale(1.0));
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
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(build_rail_script());
    }

    app.run()
}

/// Menu -> editor -> a five-part spine -> shoot the rail -> one more computer
/// -> shoot the rail again.
#[cfg(feature = "debug")]
fn build_rail_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
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
        // Built the way a player builds it: arm a part from the gallery, then
        // click the face it grows out of. Four cells of spine, because the
        // structural ceiling is the arm's reciprocal and a stubby hull runs
        // out of computer before it runs out of metal.
        .arm("arm the controller", "basic_controller_section")
        .found("found the ship")
        .arm("arm the hull", "reinforced_hull_section")
        .place("hull ahead of the controller", Vec3::ZERO, Vec3::X)
        .place("hull ahead of that", Vec3::new(1.0, 0.0, 0.0), Vec3::X)
        .place(
            "hull ahead of that again",
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::X,
        )
        .arm("arm the thruster", "basic_thruster_section")
        .place("thruster on the tail", Vec3::new(3.0, 0.0, 0.0), Vec3::X)
        .step("the spine carries its parts")
        .on_enter(|world: &mut World| {
            let sections = count_sections(world);
            assert_eq!(
                sections, SPINE_SECTIONS,
                "the readout lesson's picture is a controller, three hulls and a thruster"
            );
        })
        .add()
        // Put the part down: a builder holding one sees every free socket drawn
        // on the ship, and this picture is about the rail, not about sockets.
        .step("the spine: press Escape to put the part down")
        .on_enter(press_key(KeyCode::Escape))
        .until(editor_tool_is(EditorTool::Select))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("the spine: release Escape")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        // THE BUILD READOUT: the block the lesson lists, and the note the
        // lesson says sits under it. Read off the rail's own text, so a label
        // that was renamed or a note that stopped printing fails here.
        .step("the rail prints the block and the limit under it")
        .on_enter(|world: &mut World| {
            let block = text_of(world, READOUT_NODE);
            for label in READOUT_LABELS {
                assert!(
                    block.contains(label),
                    "the readout lesson names {label}, and the rail reads {block:?}"
                );
            }
            let note = text_of(world, NOTE_NODE);
            assert_eq!(
                note, STRUCTURE_NOTE,
                "the stacking lesson is about a hull its structure holds down"
            );
            let turn = turn_line(world);
            info!("lesson build rail: one computer, {turn}");
            world.insert_resource(TurnWithOneComputer(turn));
        })
        .add()
        .step("close in on the spine")
        .on_enter(|world: &mut World| close_in_on(world, SPINE_LOOK))
        .until(the_camera_is_posed_on(SPINE_LOOK))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("lay the interface out at reading size")
        .on_enter(scale_the_interface_up)
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the readout")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            shoot(world, READOUT_SHOT);
        })
        .until(shot_written(READOUT_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("lay the interface back out at pointer size")
        .on_enter(scale_the_interface_back)
        .until(frames(SETTLE_FRAMES))
        .add()
        // A SECOND FLIGHT COMPUTER: a cell above the MIDDLE of the spine. The
        // cell is chosen by the arm rather than by the composition: the part is
        // mass on a lever, so wherever it lands it moves the centre it is
        // measured from, and the middle is where that movement is smallest and
        // square to the hull rather than along it.
        .arm("arm the second computer", "basic_controller_section")
        .place("computer on the spine", Vec3::new(2.0, 0.0, 0.0), Vec3::Y)
        .step("the spine carries two computers")
        .on_enter(|world: &mut World| {
            let sections = count_sections(world);
            assert_eq!(
                sections, STACKED_SECTIONS,
                "the stacking lesson's picture is the same spine with one more computer"
            );
        })
        .add()
        .step("stacked: press Escape to put the part down")
        .on_enter(press_key(KeyCode::Escape))
        .until(editor_tool_is(EditorTool::Select))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("stacked: release Escape")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        // The claim, asserted rather than photographed and hoped for: the
        // ceiling the rail prints is the one it printed before the part landed.
        .step("the turn ceiling did not move")
        .on_enter(|world: &mut World| {
            let turn = turn_line(world);
            let before = world
                .get_resource::<TurnWithOneComputer>()
                .expect("the readout step ran, so it stored the first ceiling")
                .0
                .clone();
            // The claim is that the ceiling does not RISE, which is the whole
            // of what the lesson says a second computer buys nothing of. It is
            // allowed to fall a hair: the part is mass on a lever and the
            // structural ceiling is that lever's reciprocal, so a computer that
            // cost the hull NOTHING would mean the arm had stopped being
            // measured.
            assert!(
                ceiling_of(&turn) <= ceiling_of(&before),
                "the stacking lesson says a second computer buys no turn rate on this hull, \
                 and the rail went from {before:?} to {turn:?}"
            );
            let note = text_of(world, NOTE_NODE);
            assert_eq!(
                note, STRUCTURE_NOTE,
                "and that the structure is still what holds it down"
            );
            info!("lesson build rail: two computers, {turn}");
        })
        .add()
        .step("stand back from the stacked spine")
        .on_enter(|world: &mut World| close_in_on(world, STACKED_LOOK))
        .until(the_camera_is_posed_on(STACKED_LOOK))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("lay the interface out at reading size again")
        .on_enter(scale_the_interface_up)
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the unmoved ceiling")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            shoot(world, STACKING_SHOT);
        })
        .until(shot_written(STACKING_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
