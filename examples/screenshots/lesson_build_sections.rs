//! lesson_build_sections: the three SHIPBUILDING stills, shot off one build in
//! the sandbox editor - `build_sections` (a frame and the parts on it),
//! `build_mass` (the same frame grown heavy, with the numbers that say so) and
//! `build_balance` (thrusters bolted well off the centre line).
//!
//! One producer, three frames, one ship, and that is the point of shooting
//! them together: the lessons are a sequence about the SAME hull, and a reader
//! who meets three unrelated ships learns nothing from the second and third
//! pictures. So the walk builds a short frame and shoots it, bolts six more
//! hull sections on and shoots that, then runs an outrigger out to one side
//! with a thruster on the end of it and shoots that. Every gesture is a real
//! pointer gesture through the editor's own gallery and faces.
//!
//! The editor's rail carries the engineer readout - mass, thrust, turn rate,
//! parts - beside the stage, so the numbers the mass lesson is about are in
//! the frame without anything being staged for them.
//!
//! The derived skin is deliberately NOT on. A clad hull is one smooth shape,
//! which is the opposite of what these lessons say; bare, the frame and the
//! parts on it are what the stills show.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - build the ship, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the three PNGs (staged
//!   under `NOVA_CAPTURE_DIR`). `scripts/capture-lesson-media.sh` runs this
//!   and encodes them as the WebPs the handbook ships.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_build_sections --features debug
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

// The pointer gestures, shared with the other editor walks. Script-only, so
// the whole module sits behind one gate here.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use ui_walk::{
    aim_at_face, count_sections, pose_editor_camera, the_build_camera_is_posed,
    the_editor_is_inside_a_ship, Gestures, EDITOR_EYE, EDITOR_LOOK,
};

#[derive(Parser)]
#[command(name = "lesson_build_sections")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's sections demonstration", long_about = None)]
struct Cli;

/// The still for "Ships are built from sections".
#[cfg(feature = "debug")]
const SECTIONS_SHOT: &str = "build_sections.png";
/// The still for "Mass and thrust".
#[cfg(feature = "debug")]
const MASS_SHOT: &str = "build_mass.png";
/// The still for "Thruster placement".
#[cfg(feature = "debug")]
const BALANCE_SHOT: &str = "build_balance.png";

/// What the gestures leave on the frame at each shot: the founding controller,
/// two hulls and a thruster; then six more hulls around them; then a
/// three-part outrigger. Asserted rather than assumed at each stop - a missed
/// face would leave a thinner ship than the lesson's picture claims, and the
/// mass lesson would photograph a number that is not the one it is about.
#[cfg(feature = "debug")]
const BUILT_SECTIONS: usize = 4;
/// The same tally after the heavy loadout goes on.
#[cfg(feature = "debug")]
const HEAVY_SECTIONS: usize = 10;
/// The same tally after the outrigger and its thruster.
#[cfg(feature = "debug")]
const LOPSIDED_SECTIONS: usize = 13;

/// The middle of the built ship: four sections one build cell apart, growing
/// out of a controller at the ship's own origin. One cell is ten meters.
#[cfg(feature = "debug")]
const SHOT_LOOK: Meters3 = Meters3::new(15.0, 0.0, 0.0);

/// The middle of the HEAVY ship: the same spine with a course of hull on four
/// of its faces, so the middle rises half a cell and stays on the spine.
#[cfg(feature = "debug")]
const MASS_LOOK: Meters3 = Meters3::new(15.0, 5.0, 0.0);

/// The middle of the LOPSIDED ship: the outrigger runs three cells out to one
/// side, so the middle of what is on screen moves with it.
#[cfg(feature = "debug")]
const BALANCE_LOOK: Meters3 = Meters3::new(13.0, 4.0, 13.0);

/// How far in the shot camera closes from the BUILD pose.
///
/// The build pose is framed for a pointer that has to reach every face of a
/// growing ship, so it stands well back; a still that keeps it shows a small
/// ship in a large editor, and the pane draws it at about half the width it
/// was shot at. Same bearing, closer in, looking at the ship's own middle
/// rather than at the build pose's rail-clearing offset.
#[cfg(feature = "debug")]
const SHOT_CLOSE_IN: f32 = 0.65;

/// The same fraction for the heavy ship, and for the lopsided one.
///
/// Each shot closes in less than the one before it, because each ship is
/// bigger than the one before it: the frame the sections lesson fills with
/// four parts would cut the outrigger off at the edge.
#[cfg(feature = "debug")]
const MASS_CLOSE_IN: f32 = 0.78;
/// The same, for the lopsided ship.
#[cfg(feature = "debug")]
const BALANCE_CLOSE_IN: f32 = 0.95;

/// How close the camera has to be to the shot pose before the shot is taken.
/// Compared against the camera Transform, so it is an engine world-unit
/// figure, and the same epsilon the build pose uses.
#[cfg(feature = "debug")]
const POSE_EPSILON: f32 = 1e-2;

/// Where a shot camera stands: the build pose's bearing, closed in on `look`.
#[cfg(feature = "debug")]
fn shot_eye_on(look: Meters3, close_in: f32) -> Meters3 {
    look + Meters3((EDITOR_EYE - EDITOR_LOOK).0 * close_in)
}

/// Where the sections shot stands.
#[cfg(feature = "debug")]
fn shot_eye() -> Meters3 {
    shot_eye_on(SHOT_LOOK, SHOT_CLOSE_IN)
}

/// Pin the shot pose. The editor camera is a free-fly WASD camera whose
/// controller rewrites the Transform every frame, so this pins a
/// `ScriptedCameraPose` exactly as the build pose does rather than writing the
/// Transform once.
#[cfg(feature = "debug")]
fn close_in_on(world: &mut World, look: Meters3, close_in: f32) {
    let camera = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the editor is up, so it has a 3D camera");
    world.entity_mut(camera).insert(ScriptedCameraPose {
        position: shot_eye_on(look, close_in),
        look_at: look,
    });
}

/// Pin the sections shot's pose.
#[cfg(feature = "debug")]
fn close_in_on_the_frame(world: &mut World) {
    close_in_on(world, SHOT_LOOK, SHOT_CLOSE_IN);
}

/// Advance once the camera has REACHED the shot pose - the loader's enforcer
/// applies a pinned pose a system later, and a shot taken before then is of
/// the build framing this step exists to leave.
#[cfg(feature = "debug")]
fn the_camera_is_posed_on(
    look: Meters3,
    close_in: f32,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    let eye = shot_eye_on(look, close_in).to_engine();
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

/// Advance once the camera has reached the sections shot's pose.
#[cfg(feature = "debug")]
fn the_shot_camera_is_posed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    the_camera_is_posed_on(SHOT_LOOK, SHOT_CLOSE_IN)
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
        app.add_plugins(build_sections_script());
    }

    app.run()
}

/// Whether the pointer is over a section of the ship - the gate the selecting
/// click needs, because a press on empty space marks nothing.
#[cfg(feature = "debug")]
fn the_pointer_is_on_a_section() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query::<&bevy::picking::pointer::PointerInteraction>()
            .and_then(|mut pointers| {
                pointers
                    .iter(world)
                    .filter_map(|interaction| interaction.get_nearest_hit())
                    .map(|(entity, _)| *entity)
                    .next()
            })
            .is_some_and(|hit| world.get::<SectionMarker>(hit).is_some())
    })
}

/// Whether the editor has a section marked. Read off `nova_editor`'s own probe
/// rather than off the panel, so the still's subject is the editor's state and
/// not a widget that happens to look highlighted.
#[cfg(feature = "debug")]
fn a_section_is_marked() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<EditorProbe>(|editor| editor.selected_node.is_some())
}

/// Menu -> editor -> build four sections -> mark one -> shoot it, then grow
/// the same hull twice and shoot each stage.
#[cfg(feature = "debug")]
fn build_sections_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
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
        // click the face it grows out of.
        .arm("arm the controller", "basic_controller_section")
        .found("found the ship")
        .arm("arm the hull", "reinforced_hull_section")
        .place("hull ahead of the controller", Vec3::ZERO, Vec3::X)
        .place("hull ahead of that", Vec3::new(1.0, 0.0, 0.0), Vec3::X)
        .arm("arm the thruster", "basic_thruster_section")
        .place("thruster on the tail", Vec3::new(2.0, 0.0, 0.0), Vec3::X)
        .step("the frame carries its parts")
        .on_enter(|world: &mut World| {
            let sections = count_sections(world);
            assert_eq!(
                sections, BUILT_SECTIONS,
                "the lesson's picture is a controller, two hulls and a thruster"
            );
            info!("lesson build: {sections} sections on the frame");
        })
        .add()
        // Put the part down: a builder holding one sees every free socket drawn
        // on the ship, which is clutter in a picture of the finished frame.
        .step("the frame: press Escape to put the part down")
        .on_enter(press_key(KeyCode::Escape))
        .until(editor_tool_is(EditorTool::Select))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("the frame: release Escape")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        // Mark one section, which is the half of the picture the lesson's text
        // is about: a part you can point at, on a frame.
        .step("point at the hull amidships")
        .on_enter(|world: &mut World| {
            let at = aim_at_face(world, Vec3::new(1.0, 0.0, 0.0), Vec3::Y);
            move_cursor(at)(world);
        })
        .until(the_pointer_is_on_a_section())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("press to mark it")
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("release")
        .on_enter(release_mouse(MouseButton::Left))
        .until(pointer_released())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("the section is marked")
        .until(a_section_is_marked())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("close in on the frame")
        .on_enter(close_in_on_the_frame)
        .until(the_shot_camera_is_posed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the frame and its parts")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            shoot(world, SECTIONS_SHOT);
        })
        .until(shot_written(SECTIONS_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // MASS AND THRUST: the same spine, with a course of hull bolted onto
        // four of its faces and NO second thruster. That asymmetry is the
        // lesson - the drive did not change and the ship it has to move and
        // stop did - and the rail's readout carries the arithmetic beside it.
        .arm("arm the hull again", "reinforced_hull_section")
        .place(
            "hull to starboard amidships",
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::Z,
        )
        .place("hull to port amidships", Vec3::new(1.0, 0.0, 0.0), -Vec3::Z)
        .place("hull to starboard aft", Vec3::new(2.0, 0.0, 0.0), Vec3::Z)
        .place("hull to port aft", Vec3::new(2.0, 0.0, 0.0), -Vec3::Z)
        .place("hull on the spine", Vec3::new(1.0, 0.0, 0.0), Vec3::Y)
        .place("hull on the spine aft", Vec3::new(2.0, 0.0, 0.0), Vec3::Y)
        .step("the frame is heavy")
        .on_enter(|world: &mut World| {
            let sections = count_sections(world);
            assert_eq!(
                sections, HEAVY_SECTIONS,
                "the mass lesson's picture is the same spine under a course of hull"
            );
            info!("lesson build: {sections} sections on the heavy frame");
        })
        .add()
        .step("heavy: press Escape to put the part down")
        .on_enter(press_key(KeyCode::Escape))
        .until(editor_tool_is(EditorTool::Select))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("heavy: release Escape")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        .step("stand back from the heavy frame")
        .on_enter(|world: &mut World| close_in_on(world, MASS_LOOK, MASS_CLOSE_IN))
        .until(the_camera_is_posed_on(MASS_LOOK, MASS_CLOSE_IN))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the heavy frame and its numbers")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            shoot(world, MASS_SHOT);
        })
        .until(shot_written(MASS_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // THRUSTER PLACEMENT: an outrigger three cells out to ONE side with a
        // thruster on the end of it. Nothing balances it, which is exactly the
        // ship the lesson describes - the one the flight computer has to fly
        // straight anyway.
        .arm("arm the hull for the outrigger", "reinforced_hull_section")
        .place("outrigger, first cell", Vec3::new(1.0, 0.0, 1.0), Vec3::Z)
        .place("outrigger, second cell", Vec3::new(1.0, 0.0, 2.0), Vec3::Z)
        .arm("arm the outboard thruster", "basic_thruster_section")
        .place(
            "thruster on the outrigger",
            Vec3::new(1.0, 0.0, 3.0),
            Vec3::X,
        )
        .step("the frame is lopsided")
        .on_enter(|world: &mut World| {
            let sections = count_sections(world);
            assert_eq!(
                sections, LOPSIDED_SECTIONS,
                "the balance lesson's picture is an arm of hull with a drive on the end of it"
            );
            info!("lesson build: {sections} sections on the lopsided frame");
        })
        .add()
        .step("lopsided: press Escape to put the part down")
        .on_enter(press_key(KeyCode::Escape))
        .until(editor_tool_is(EditorTool::Select))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("lopsided: release Escape")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        .step("stand back from the lopsided frame")
        .on_enter(|world: &mut World| close_in_on(world, BALANCE_LOOK, BALANCE_CLOSE_IN))
        .until(the_camera_is_posed_on(BALANCE_LOOK, BALANCE_CLOSE_IN))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the lopsided frame")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            shoot(world, BALANCE_SHOT);
        })
        .until(shot_written(BALANCE_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
