//! lesson_build_geometry: the three SHIPBUILDING lessons about WHERE a part
//! sits, shot off one build in the sandbox editor - `build_turning` (a long
//! spine and the ceiling its length costs), `build_weapon_mounts` (a turret on
//! a flank and a railgun on the nose) and `build_docking_port` (a port seated
//! on a face of the same hull).
//!
//! One producer, three frames, one ship, for the reason `lesson_build_sections`
//! gives: these lessons are a sequence about the same hull, and three unrelated
//! ships teach nothing after the first picture. The walk runs a spine out five
//! cells and shoots it, bolts the two weapons on and shoots that, then seats a
//! docking port on the spine and shoots that. Every gesture is a real pointer
//! gesture through the editor's own gallery and faces (`shared/ui_walk.rs`).
//!
//! ## Why three STILLS and not three loops
//!
//! The subject of all three is PRINTED, not moving. `build_turning` is about
//! the rail's engineer readout - `Turn <n> rad/s2` over
//! `structure-limited - shorten the hull` (`nova_editor/src/readout.rs`) - and
//! the other two are about the inspector down the right edge naming the part
//! and its health. A loop cell is 960x540, which is half the linear size a
//! still ships at, and rail type at half size is a grey smear: the reader
//! would be shown the claim and unable to read it. Nothing in any of the three
//! MOVES, either - an editor with the part put down is a still frame - so the
//! loop would spend twenty cells repeating one unreadable picture.
//!
//! The one thing here that does move is the readout falling as the hull grows,
//! and that is a lesson this producer would have to shoot at half size to
//! tell. It is left as the honest trade rather than taken.
//!
//! ## Why the hull is the shape it is
//!
//! A spine five cells long is fifty meters of hull on ONE basic controller,
//! which is the case `build_turning` is about: past a certain length the 8 G
//! limit at the furthest face binds before the computers do, and the rail says
//! so in its own words. The walk asserts the rail reached that note rather
//! than trusting the length, because the arithmetic belongs to the ship crate
//! and a rebalance there would otherwise quietly reshoot this lesson as a
//! picture of the OTHER limit.
//!
//! The weapons go on where the lesson's claim can be seen: the turret out on a
//! flank, where nothing of the hull is in the way of a traverse, and the
//! railgun on the nose face, where its barrel runs straight out along the face
//! it is bolted to. The port goes on the spine's top face, hatch outward and
//! clear - the seat the lesson describes.
//!
//! The derived skin is deliberately NOT on, the same call
//! `lesson_build_sections` makes: a clad hull is one smooth shape, and every
//! one of these lessons is about a part you can point at.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - build the ship, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the three PNGs (staged
//!   under `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_build_geometry --features debug
//! ```

#[cfg(feature = "debug")]
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
#[command(name = "lesson_build_geometry")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's turn-rate, weapon-mount and docking-port demonstrations", long_about = None)]
struct Cli;

/// The still for "What decides your turn rate".
#[cfg(feature = "debug")]
const TURNING_SHOT: &str = "build_turning.png";
/// The still for "Where a weapon can point".
#[cfg(feature = "debug")]
const MOUNTS_SHOT: &str = "build_weapon_mounts.png";
/// The still for "Bolting on a docking port".
#[cfg(feature = "debug")]
const PORT_SHOT: &str = "build_docking_port.png";

/// The spine: a controller and four hull sections in a line.
#[cfg(feature = "debug")]
const SPINE_SECTIONS: usize = 5;
/// The same tally once both weapons are on.
#[cfg(feature = "debug")]
const ARMED_SECTIONS: usize = 7;
/// The same tally once the port is seated.
#[cfg(feature = "debug")]
const PORTED_SECTIONS: usize = 8;

/// The words the rail prints under the block when the hull's LENGTH is what
/// holds its turn rate down, verbatim from `nova_editor/src/readout.rs`.
///
/// Asserted, not assumed: the picture is only this lesson's picture while the
/// ceiling the spine runs into is the structural one.
#[cfg(feature = "debug")]
const STRUCTURE_NOTE: &str = "structure-limited";

/// The middle of the spine: five sections one build cell apart from the ship's
/// own origin, so the middle sits two cells out. One cell is ten meters.
#[cfg(feature = "debug")]
const TURNING_LOOK: Meters3 = Meters3::new(20.0, 0.0, 0.0);

/// The middle of the ARMED hull: the turret stands on the spine's top face
/// three cells out, the nose gun runs a cell past the end of the spine, and
/// the frame has to hold both.
#[cfg(feature = "debug")]
const MOUNTS_LOOK: Meters3 = Meters3::new(38.0, 5.0, 0.0);

/// Where the camera stands to BOLT the nose gun on.
///
/// Not a framing - a reach. The build pose is centred near the controller and
/// the editor's inspector owns the right edge of the screen, so the far nose
/// of a five-cell spine projects under the panel and the pointer can never hit
/// it: the walk stalled there with the solver holding no placement at all.
/// Sliding the camera down the spine, on the same bearing, puts that face in
/// the middle of the frame where the pointer can reach it.
#[cfg(feature = "debug")]
const NOSE_LOOK: Meters3 = Meters3::new(40.0, 0.0, 0.0);
/// How far off the nose that reach stands.
#[cfg(feature = "debug")]
const NOSE_STAND_OFF: f32 = 0.7;

/// The port itself, seated on the spine's top face one cell out from the
/// controller - this shot is of the PART, not of the ship it is on.
#[cfg(feature = "debug")]
const PORT_LOOK: Meters3 = Meters3::new(10.0, 10.0, 0.0);

/// How far each shot stands off its subject, as a fraction of the build pose's
/// own stand-off (see `shot_eye_on`).
///
/// The spine is fifty meters end to end, so the turn-rate shot keeps the whole
/// build pose distance; the armed hull is longer again across the diagonal and
/// stands back further still. The port is one part, and a frame of the ship it
/// is on would not show the seat.
#[cfg(feature = "debug")]
const TURNING_STAND_OFF: f32 = 1.05;
/// The same, for the armed hull.
#[cfg(feature = "debug")]
const MOUNTS_STAND_OFF: f32 = 0.85;
/// The same, for the port.
#[cfg(feature = "debug")]
const PORT_STAND_OFF: f32 = 0.62;

/// How close the camera has to be to a shot pose before the shot is taken.
/// Compared against the camera Transform, so it is an engine world-unit
/// figure, and the same epsilon the build pose uses.
#[cfg(feature = "debug")]
const POSE_EPSILON: f32 = 1e-2;

/// Where a shot camera stands: the build pose's bearing, at `stand_off` of its
/// distance, centred on `look`.
#[cfg(feature = "debug")]
fn shot_eye_on(look: Meters3, stand_off: f32) -> Meters3 {
    look + Meters3((EDITOR_EYE - EDITOR_LOOK).0 * stand_off)
}

/// Pin a shot pose. The editor camera is a free-fly WASD camera whose
/// controller rewrites the Transform every frame, so this pins a
/// `ScriptedCameraPose` exactly as the build pose does rather than writing the
/// Transform once.
#[cfg(feature = "debug")]
fn stand_at(world: &mut World, look: Meters3, stand_off: f32) {
    let camera = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the editor is up, so it has a 3D camera");
    world.entity_mut(camera).insert(ScriptedCameraPose {
        position: shot_eye_on(look, stand_off),
        look_at: look,
    });
}

/// Advance once the camera has REACHED a shot pose - the loader's enforcer
/// applies a pinned pose a system later, and a shot taken before then is of
/// the build framing the step exists to leave.
#[cfg(feature = "debug")]
fn the_camera_stands_at(
    look: Meters3,
    stand_off: f32,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    let eye = shot_eye_on(look, stand_off).to_engine();
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

/// Whether any text on screen carries `needle`.
///
/// The rail's readout nodes are `pub(crate)` to the editor, so this reads what
/// is DRAWN rather than the resource behind it - which is the right end for a
/// still anyway: the claim is about what the picture shows, and a sweep of the
/// live text says exactly that.
#[cfg(feature = "debug")]
fn on_screen(world: &World, needle: &str) -> bool {
    world
        .try_query::<&Text>()
        .is_some_and(|mut texts| texts.iter(world).any(|text| text.0.contains(needle)))
}

/// Advance once the rail prints `needle`.
#[cfg(feature = "debug")]
fn the_rail_says(
    needle: &'static str,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| on_screen(world, needle))
}

/// Whether the editor has a section marked.
#[cfg(feature = "debug")]
fn a_section_is_marked() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<EditorProbe>(|editor| editor.selected_node.is_some())
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
        app.add_plugins(build_geometry_script());
    }

    app.run()
}

/// Put the armed part down, so the picture is of the ship and not of every
/// free socket on it.
#[cfg(feature = "debug")]
fn put_the_part_down(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    beat: &'static str,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    script
        .step(format!("{beat}: press Escape to put the part down"))
        .on_enter(press_key(KeyCode::Escape))
        .until(editor_tool_is(EditorTool::Select))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step(format!("{beat}: release Escape"))
        .on_enter(release_key(KeyCode::Escape))
        .add()
}

/// Mark the section whose `face` face is showing at `on`, so the inspector
/// down the right edge names it in the shot.
#[cfg(feature = "debug")]
fn mark_the_part(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    beat: &'static str,
    on: Vec3,
    face: Vec3,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    script
        .step(format!("{beat}: point at it"))
        .on_enter(move |world: &mut World| {
            let at = aim_at_face(world, on, face);
            move_cursor(at)(world);
        })
        .until(the_pointer_is_on_a_section())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step(format!("{beat}: press to mark it"))
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step(format!("{beat}: release"))
        .on_enter(release_mouse(MouseButton::Left))
        .until(pointer_released())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step(format!("{beat}: it is marked"))
        .until(a_section_is_marked())
        .deadline(STEP_DEADLINE_SECS)
        .add()
}

/// Menu -> editor -> run a spine out and shoot its ceiling, bolt the two
/// weapons on and shoot them, seat a port and shoot that.
#[cfg(feature = "debug")]
fn build_geometry_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
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
        // WHAT DECIDES YOUR TURN RATE: one computer, and then as much hull as
        // the lesson's claim needs.
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
        .place("hull on the nose", Vec3::new(3.0, 0.0, 0.0), Vec3::X)
        .step("the spine is long")
        .on_enter(|world: &mut World| {
            let sections = count_sections(world);
            assert_eq!(
                sections, SPINE_SECTIONS,
                "the turn-rate lesson's picture is a computer under fifty meters of hull"
            );
            info!("lesson build: {sections} sections on the spine");
        })
        .add();
    let script = put_the_part_down(script, "the spine")
        .step("the rail reports the structural ceiling")
        .until(the_rail_says(STRUCTURE_NOTE))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("stand off the spine")
        .on_enter(|world: &mut World| stand_at(world, TURNING_LOOK, TURNING_STAND_OFF))
        .until(the_camera_stands_at(TURNING_LOOK, TURNING_STAND_OFF))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the spine and its ceiling")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            shoot(world, TURNING_SHOT);
        })
        .until(shot_written(TURNING_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // WHERE A WEAPON CAN POINT: a turret out on the flank, where a
        // traverse has nothing in its way, and a railgun on the nose face,
        // where the barrel runs out along the face it is bolted to.
        .arm("arm the turret", "pdc_kinetic_turret_section")
        .place(
            "turret on the spine's top face",
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::Y,
        )
        .arm("arm the railgun", "railgun_lance_section")
        .step("bring the nose into reach")
        .on_enter(|world: &mut World| stand_at(world, NOSE_LOOK, NOSE_STAND_OFF))
        .until(the_camera_stands_at(NOSE_LOOK, NOSE_STAND_OFF))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .place("railgun on the nose", Vec3::new(4.0, 0.0, 0.0), Vec3::X)
        .step("the hull is armed")
        .on_enter(|world: &mut World| {
            let sections = count_sections(world);
            assert_eq!(
                sections, ARMED_SECTIONS,
                "the weapon-mount lesson's picture is a turret AND a railgun on one hull"
            );
            info!("lesson build: {sections} sections on the armed hull");
        })
        .add();
    let script = put_the_part_down(script, "the weapons")
        .step("stand off the armed hull")
        .on_enter(|world: &mut World| stand_at(world, MOUNTS_LOOK, MOUNTS_STAND_OFF))
        .until(the_camera_stands_at(MOUNTS_LOOK, MOUNTS_STAND_OFF))
        .deadline(STEP_DEADLINE_SECS)
        .add();
    let script = mark_the_part(script, "the turret", Vec3::new(3.0, 1.0, 0.0), Vec3::Z)
        .step("shoot the two mounts")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            shoot(world, MOUNTS_SHOT);
        })
        .until(shot_written(MOUNTS_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // BOLTING ON A DOCKING PORT: seated on the spine's top face, hatch
        // outward, on a ship that had no way to dock a moment ago.
        .arm("arm the docking port", "docking_port_section")
        .place(
            "port on the spine's top face",
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::Y,
        )
        .step("the hull carries a port")
        .on_enter(|world: &mut World| {
            let sections = count_sections(world);
            assert_eq!(
                sections, PORTED_SECTIONS,
                "the docking-port lesson's picture is a port seated on a hull face"
            );
            info!("lesson build: {sections} sections with the port on");
        })
        .add();
    let script = put_the_part_down(script, "the port")
        .step("close in on the port")
        .on_enter(|world: &mut World| stand_at(world, PORT_LOOK, PORT_STAND_OFF))
        .until(the_camera_stands_at(PORT_LOOK, PORT_STAND_OFF))
        .deadline(STEP_DEADLINE_SECS)
        .add();
    mark_the_part(script, "the port", Vec3::new(1.0, 1.0, 0.0), Vec3::Z)
        .step("shoot the seated port")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            shoot(world, PORT_SHOT);
        })
        .until(shot_written(PORT_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
