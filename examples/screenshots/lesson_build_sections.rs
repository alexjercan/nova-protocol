//! lesson_build_sections: the training handbook's demonstration for "Ships are
//! built from sections" (`assets/base/training/build_sections.webp`).
//!
//! A short hull built in the sandbox editor by real pointer gestures, with one
//! of its sections marked - the lesson's own picture: sections bolted to a
//! frame, each one a part you can point at.
//!
//! The derived skin is deliberately NOT on. A clad hull is one smooth shape,
//! which is the opposite of what this lesson says; bare, the frame and the
//! four parts on it are what the still shows.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - build the ship, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNG (staged under
//!   `NOVA_CAPTURE_DIR`). `scripts/capture-lesson-media.sh` runs this and
//!   encodes the result as the WebP the handbook ships.
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

/// The lesson this shoots for, and so the name of the still.
#[cfg(feature = "debug")]
const SHOT: &str = "build_sections.png";

/// What the gestures leave on the frame: the founding controller, two hulls
/// and a thruster. Asserted rather than assumed - a missed face would leave a
/// thinner ship than the lesson's picture claims.
#[cfg(feature = "debug")]
const BUILT_SECTIONS: usize = 4;

/// The middle of the built ship: four sections one build cell apart, growing
/// out of a controller at the ship's own origin. One cell is ten meters.
#[cfg(feature = "debug")]
const SHOT_LOOK: Meters3 = Meters3::new(15.0, 0.0, 0.0);

/// How far in the shot camera closes from the BUILD pose.
///
/// The build pose is framed for a pointer that has to reach every face of a
/// growing ship, so it stands well back; a still that keeps it shows a small
/// ship in a large editor, and the pane draws it at about half the width it
/// was shot at. Same bearing, closer in, looking at the ship's own middle
/// rather than at the build pose's rail-clearing offset.
#[cfg(feature = "debug")]
const SHOT_CLOSE_IN: f32 = 0.65;

/// How close the camera has to be to the shot pose before the shot is taken.
/// Compared against the camera Transform, so it is an engine world-unit
/// figure, and the same epsilon the build pose uses.
#[cfg(feature = "debug")]
const POSE_EPSILON: f32 = 1e-2;

/// Where the shot camera stands: the build pose's bearing, closed in.
#[cfg(feature = "debug")]
fn shot_eye() -> Meters3 {
    SHOT_LOOK + Meters3((EDITOR_EYE - EDITOR_LOOK).0 * SHOT_CLOSE_IN)
}

/// Pin the shot pose. The editor camera is a free-fly WASD camera whose
/// controller rewrites the Transform every frame, so this pins a
/// `ScriptedCameraPose` exactly as the build pose does rather than writing the
/// Transform once.
#[cfg(feature = "debug")]
fn close_in_on_the_frame(world: &mut World) {
    let camera = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the editor is up, so it has a 3D camera");
    world.entity_mut(camera).insert(ScriptedCameraPose {
        position: shot_eye(),
        look_at: SHOT_LOOK,
    });
}

/// Advance once the camera has REACHED the shot pose - the loader's enforcer
/// applies a pinned pose a system later, and a shot taken before then is of
/// the build framing this step exists to leave.
#[cfg(feature = "debug")]
fn the_shot_camera_is_posed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&Transform, With<Camera3d>>()
            .is_some_and(|mut cameras| {
                cameras.iter(world).any(|camera| {
                    camera
                        .translation
                        .abs_diff_eq(shot_eye().to_engine(), POSE_EPSILON)
                })
            })
    })
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

/// Menu -> editor -> build four sections -> mark one -> shoot it.
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
        .step("press Escape to put the part down")
        .on_enter(press_key(KeyCode::Escape))
        .until(editor_tool_is(EditorTool::Select))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("release Escape")
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
            shoot(world, SHOT);
        })
        .until(shot_written(SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
