//! lesson_build_generate: two SHIPBUILDING loops off one build - the Generate
//! block rolling a hull out of the catalog into the ship you are inside, and
//! Ship Skin closing plating over the hull it rolled.
//!
//! The second sheet rides the first because of what it needs: the skin lesson
//! is about a WHOLE hull's structure disappearing under cladding, and the only
//! whole hull this walk stands up is the one Generate produces. A three-part
//! stub clads into a brick.
//!
//! `build_generate` is a sheet because the whole claim is a REPLACEMENT: the
//! stage carries a hand-built stub for the first third of the loop, the
//! Generate button is pressed inside the recording, and a whole hull stands
//! there for the rest of it. A still of the result would be a picture of a
//! ship with nothing saying where it came from - which is why this lesson was
//! flipped from `still(...)` to `looping(...)` in
//! `crates/nova_authoring/src/base_content/lessons.rs` and its alt text
//! rewritten to describe the footage.
//!
//! ## Why the seed is typed
//!
//! The seed starts random (`HullSeed::default`), so an untouched Generate
//! rolls a different ship on every capture - a lesson whose art changes shape
//! each time the script is re-run, and a framing that cannot be judged once
//! and kept. The walk therefore does what the lesson says a builder can do: it
//! puts the caret in the seed field, clears it and types one. The hull in the
//! sheet is that seed's hull, the same hull the next capture will roll, and
//! the number is legible in the rail beside it.
//!
//! The clear is [`SEED_CLEAR`] backspaces against a field of at most
//! `SEED_DIGITS`. Counting against the seed that HAPPENS to be there is how
//! the NOVA OS prompt walk lost a beat to an off-by-one, so this counts
//! against the longest value the field can hold, with room to spare - an extra
//! backspace on an empty field costs nothing.
//!
//! The caret is sent to the END before the clear. A click focuses a field at
//! the column it landed in, and the gesture clicks the middle of the widget,
//! so the first run of this walk backspaced over the FRONT of a twenty-digit
//! seed and typed `2026` in front of the digits that survived: the field never
//! read the seed and the beat stalled on its deadline.
//!
//! ## Why the pointer is parked before the sheet opens
//!
//! Twenty cells at ten frames a second is two seconds, and a `click` gesture
//! spends its first beats finding the widget and waiting for the pointer to
//! register over it. Those beats belong BEFORE the recording: the sheet opens
//! with the pointer already resting on Generate, so the only thing inside the
//! twenty cells is the press, the release and the hull that answers them.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - build, type, generate, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the sheet (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_build_generate --features debug
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;
// The pointer gestures, shared with the other editor walks. Script-only, so
// the whole module sits behind one gate here.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, LESSON_GRID, LESSON_STILL};
#[cfg(feature = "debug")]
use ui_walk::{
    count_sections, pose_editor_camera, the_build_camera_is_posed, the_editor_is_inside_a_ship,
    the_skin_is_on, Gestures, EDITOR_EYE, EDITOR_LOOK,
};

#[derive(Parser)]
#[command(name = "lesson_build_generate")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's Generate demonstration", long_about = None)]
struct Cli;

/// The sheet this tiles: "Generate a hull".
#[cfg(feature = "debug")]
const GENERATE_LESSON: &str = "build_generate";

/// The second sheet: "Ship skin".
#[cfg(feature = "debug")]
const SKIN_LESSON: &str = "build_skin";

/// Cells the skin sheet holds the hull BARE before the toggle goes on.
///
/// A quarter of the loop. The claim is that the plating is DERIVED from the
/// structure rather than placed on it, and a reader who never sees the
/// structure has been shown a clad ship and told to take the derivation on
/// trust.
#[cfg(feature = "debug")]
const SKIN_BARE_CELLS: u32 = 5;

/// The seed the walk types, and the hull the handbook therefore ships.
///
/// Short on purpose: the field is in the rail at eleven pixels, and a
/// twenty-digit random number there is a grey bar. A number a reader could
/// actually write down and type back in is the lesson's own claim.
#[cfg(feature = "debug")]
const SEED: &str = "2026";

/// Backspaces that empty the seed field before the seed is typed.
///
/// Against `SEED_DIGITS` (twenty, `nova_editor/src/generate.rs`) rather than
/// against whatever random value is sitting there, with room to spare: a
/// backspace on an empty field is a no-op, and a count that is one short
/// leaves a digit glued to the front of the typed seed.
#[cfg(feature = "debug")]
const SEED_CLEAR: usize = 28;

/// The stub the generator REPLACES: a controller and two hull sections.
#[cfg(feature = "debug")]
const STUB_SECTIONS: usize = 3;

/// How many sections a rolled hull has to carry before the walk calls the
/// replacement done.
///
/// A floor, not a figure: the plan is the catalog's and the count is the
/// seed's, so the walk asserts only that a whole hull arrived where a stub of
/// three parts was.
#[cfg(feature = "debug")]
const ROLLED_SECTIONS: usize = 6;

/// Cells the sheet holds the hand-built stub before the button is pressed.
#[cfg(feature = "debug")]
const STUB_CELLS: u32 = 5;

/// What the sheet looks at: the ship's own origin, which is where the stub
/// stands and where the rolled hull is laid out around.
#[cfg(feature = "debug")]
const GENERATE_LOOK: Meters3 = Meters3::new(6.0, 0.0, 0.0);

/// How far the sheet stands off it, as a fraction of the build pose's own
/// stand-off.
///
/// Well back. The stub is three parts and the hull that replaces it is however
/// many the plan rolls, so the frame is sized for the one that has not
/// happened yet: a shot framed on the stub would have the answer growing out
/// of every edge of it.
#[cfg(feature = "debug")]
const GENERATE_STAND_OFF: f32 = 2.1;

/// How close the camera has to be to the shot pose before the sheet opens.
#[cfg(feature = "debug")]
const POSE_EPSILON: f32 = 1e-2;

/// Where the sheet's camera stands.
#[cfg(feature = "debug")]
fn sheet_eye() -> Meters3 {
    GENERATE_LOOK + Meters3((EDITOR_EYE - EDITOR_LOOK).0 * GENERATE_STAND_OFF)
}

/// Pin the sheet's pose. The editor camera is a free-fly WASD camera whose
/// controller rewrites the Transform every frame, so this pins a
/// `ScriptedCameraPose` rather than writing the Transform once.
#[cfg(feature = "debug")]
fn stand_off_the_stage(world: &mut World) {
    let camera = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the editor is up, so it has a 3D camera");
    world.entity_mut(camera).insert(ScriptedCameraPose {
        position: sheet_eye(),
        look_at: GENERATE_LOOK,
    });
}

/// Advance once the camera has REACHED the sheet's pose - the loader's
/// enforcer applies a pinned pose a system later.
#[cfg(feature = "debug")]
fn the_camera_stands_off() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    let eye = sheet_eye().to_engine();
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

/// Advance once a text field on screen holds exactly the typed seed.
///
/// The seed field's own marker is `pub(crate)` to the editor, so this reads
/// the VALUE rather than the widget: the editor's other fields carry a name
/// and a transform, and none of them is ever the string typed here.
#[cfg(feature = "debug")]
fn the_seed_field_reads_the_seed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate>
{
    std::sync::Arc::new(|world: &World| {
        world
            .try_query::<&nova_ui::prelude::TextFieldValue>()
            .is_some_and(|mut fields| fields.iter(world).any(|value| value.0 == SEED))
    })
}

/// Advance once a whole hull stands where the stub stood.
#[cfg(feature = "debug")]
fn the_hull_was_rolled() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| count_sections(world) >= ROLLED_SECTIONS)
}

/// Where in the window the walk parks the skin toggle before pressing it.
///
/// Low, but clear of the key legend along the bottom edge: the row has to be
/// under the pointer AND in the recorded frame, and the rail scrolls to put it
/// there rather than the camera moving to find it.
#[cfg(feature = "debug")]
const SKIN_TOGGLE_Y: f32 = 800.0;

/// The skin toggle's centre in the window, and its half height.
#[cfg(feature = "debug")]
fn toggle_at(world: &mut World) -> Option<(f32, f32)> {
    let mut rows = world.query::<(&Name, &ComputedNode, &UiGlobalTransform)>();
    rows.iter(world)
        .find(|(name, _, _)| name.as_str() == "Ship Skin Toggle")
        .map(|(_, node, at)| (at.translation.y, node.size().y * 0.5))
}

/// Scroll the rail by however far the toggle still is from where it is wanted.
///
/// Re-measured every frame rather than spent in one notch: the wheel is
/// clamped against the content (`nova_ui::screen::scroll`), and a rolled hull's
/// scene list is long enough that the first notch can be the clamp rather than
/// the gap.
#[cfg(feature = "debug")]
fn scroll_the_toggle_into_view(world: &mut World) {
    let Some((y, _)) = toggle_at(world) else {
        return;
    };
    // Negative is down: the handler subtracts the notch from the stored offset,
    // so a row BELOW the fold is reached with a negative wheel.
    scroll_pixels(-(y - SKIN_TOGGLE_Y))(world);
}

/// Advance once the whole toggle row is inside the window.
#[cfg(feature = "debug")]
fn the_toggle_is_in_view() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(mut rows) = world.try_query::<(&Name, &ComputedNode, &UiGlobalTransform)>() else {
            return false;
        };
        rows.iter(world)
            .find(|(name, _, _)| name.as_str() == "Ship Skin Toggle")
            .is_some_and(|(_, node, at)| {
                let half = node.size().y * 0.5;
                at.translation.y - half > 0.0 && at.translation.y + half < LESSON_STILL.1 as f32
            })
    })
}

/// Where the skin toggle actually is, for the abort path.
///
/// The rail is a scrolling column and a rolled hull is longer than the stub
/// that preceded it, so the row this walk aims at can be under the fold rather
/// than merely unhit - two failures that look identical from a stalled beat.
#[cfg(feature = "debug")]
fn the_toggle_diagnosis(world: &World) -> String {
    let Some(mut rows) = world.try_query::<(&Name, &ComputedNode, &UiGlobalTransform)>() else {
        return "the world has no UI to look at".to_string();
    };
    rows.iter(world)
        .find(|(name, _, _)| name.as_str() == "Ship Skin Toggle")
        .map_or_else(
            || "there is no node named `Ship Skin Toggle` on screen".to_string(),
            |(_, node, at)| {
                let size = node.size();
                format!(
                    "the skin toggle is {}x{} at ({:.0}, {:.0}) on a {}x{} window",
                    size.x,
                    size.y,
                    at.translation.x,
                    at.translation.y,
                    LESSON_STILL.0,
                    LESSON_STILL.1,
                )
            },
        )
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same app the game's Sandbox button opens.
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            lesson_profile(),
        ));
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(build_generate_script());
    }

    app.run()
}

/// Menu -> editor -> a three-part stub -> a typed seed -> Generate, recorded.
#[cfg(feature = "debug")]
fn build_generate_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
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
        // The stub: something built by hand, so the loop has a BEFORE that a
        // reader can see was replaced.
        .arm("arm the controller", "basic_controller_section")
        .found("found the ship")
        .arm("arm the hull", "reinforced_hull_section")
        .place("hull ahead of the controller", Vec3::ZERO, Vec3::X)
        .place("hull ahead of that", Vec3::new(1.0, 0.0, 0.0), Vec3::X)
        .step("the stub stands")
        .on_enter(|world: &mut World| {
            let sections = count_sections(world);
            assert_eq!(
                sections, STUB_SECTIONS,
                "the loop's first cells are a hand-built stub, which Generate then replaces"
            );
            info!("lesson build: {sections} sections on the stub");
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
        // THE SEED, typed the way the lesson says it can be.
        .click("put the caret in the seed field", "Hull Seed Field")
        .step("clear the seed the editor rolled")
        .on_enter(|world: &mut World| {
            // End FIRST. A click puts the caret in the column it landed in
            // (`caret_for_pointer`), and the gesture clicks the middle of the
            // widget, so a clear that starts where the click fell backspaces
            // over the front of the seed and leaves its tail glued to the
            // typed one. The field reads every keystroke of a frame in order,
            // so the whole clear is one beat.
            press_edit_key(bevy::input::keyboard::Key::End)(world);
            for _ in 0..SEED_CLEAR {
                press_edit_key(bevy::input::keyboard::Key::Backspace)(world);
            }
        })
        .until(frames(2))
        .add()
        .step("type the seed")
        .on_enter(|world: &mut World| type_text(SEED.to_string())(world))
        .until(the_seed_field_reads_the_seed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Framed and aimed BEFORE the recording, so the sheet spends its cells
        // on the hull rather than on the pointer finding a button.
        .step("stand off the stage")
        .on_enter(stand_off_the_stage)
        .until(the_camera_stands_off())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("rest the pointer on Generate")
        .on_enter(hover_named("Generate Button"))
        .each(|world: &mut World, _, _| hover_named("Generate Button")(world))
        .until(pointer_over_node("Generate Button"))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("open the sheet on the stub")
        .on_enter(|world: &mut World| sheet_start(world, GENERATE_LESSON, LESSON_GRID))
        .until(frames(STUB_CELLS))
        .add()
        .step("press Generate")
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("release, and let the hull roll in")
        .on_enter(release_mouse(MouseButton::Left))
        .until(and(the_hull_was_rolled(), sheet_written(GENERATE_LESSON)))
        .deadline(60.0)
        .add()
        .step("a whole hull stands where the stub stood")
        .on_enter(|world: &mut World| {
            let sections = count_sections(world);
            assert!(
                sections >= ROLLED_SECTIONS,
                "Generate must replace the stub with a hull: {sections} section(s) on the ship"
            );
            info!("lesson build: {sections} sections on the rolled hull");
        })
        .add()
        // SHIP SKIN, on the hull Generate just rolled. The subject of the skin
        // lesson is a WHOLE hull's structure disappearing under plating, and
        // the rolled hull is the only whole hull this walk ever stands up - a
        // three-part stub clads into a brick and shows nothing.
        //
        // The camera is not re-posed: the generate sheet's stand-off was sized
        // for the hull that had not been rolled yet, and nothing has released
        // that pose.
        // Parked on FRAMES rather than on `pointer_over_node`, which is what
        // the Generate button next to it is gated on. The toggle is a ROW with
        // a box in it (`skin_toggle_row`), so the hit under the pointer is the
        // child and the predicate on the row never reads true - the gesture
        // that does work on it clicks the row and lets the press bubble to the
        // row's own observer (`screenshot_editor.rs`). The real gate is two
        // beats down: the plating either appears or the run says so.
        // The rail is a scrolling column, and the hull Generate rolled makes a
        // scene list long enough to push Ship Settings thousands of pixels
        // under the fold - the first run of this beat found the toggle at
        // y 5441 on a 1080 window. So the rail is scrolled the way a builder
        // scrolls it, with the wheel over the pane.
        .step("put the pointer over the rail")
        .on_enter(hover_named("Scene List"))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("scroll the rail down to the skin toggle")
        .on_enter(scroll_the_toggle_into_view)
        .each(|world: &mut World, _, _| scroll_the_toggle_into_view(world))
        .until(the_toggle_is_in_view())
        .diagnose(the_toggle_diagnosis)
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("rest the pointer on the skin toggle")
        .on_enter(hover_named("Ship Skin Toggle"))
        .each(|world: &mut World, _, _| hover_named("Ship Skin Toggle")(world))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("open the sheet on the bare hull")
        .on_enter(|world: &mut World| sheet_start(world, SKIN_LESSON, LESSON_GRID))
        .until(frames(SKIN_BARE_CELLS))
        .add()
        .step("press the skin toggle")
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("release, and let the plating close over it")
        .on_enter(release_mouse(MouseButton::Left))
        .until(and(the_skin_is_on(), sheet_written(SKIN_LESSON)))
        .diagnose(the_toggle_diagnosis)
        .deadline(60.0)
        .add()
        .step("the hull is clad in plating nothing placed")
        .on_enter(|world: &mut World| {
            let sections = count_sections(world);
            info!("lesson build: skin derived over {sections} sections");
        })
        .add()
}
