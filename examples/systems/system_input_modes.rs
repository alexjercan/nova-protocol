//! system_input_modes: the editor's keyboard has ONE owner at a time, proved by
//! pressing the keys that used to reach two.
//!
//! Every keyboard system used to carry its own list of the things it must not
//! fire under, so a new mode suppressed nothing until somebody hit the
//! collision and added a line. `nova_ui::input_mode` replaced the lists with an
//! arbiter: a verb answers in `Normal` alone, a mode's own systems answer in
//! their mode and in `Normal`, and the enum's order settles a contested frame.
//!
//! The run presses one key per mode - the key that, without the arbiter, two
//! owners would both answer:
//!
//! 1. INSERT. Delete, with the caret in an object's Name field. The tree keeps
//!    the object; the character is the field's business.
//! 2. NORMAL. The same Delete once the field is let go. The object goes.
//! 3. BROWSE. Escape, with the parts gallery up. The gallery closes and the
//!    editor stays inside the ship - one rung, not two.
//! 4. BIND. Delete, with a rebind waiting for a key. This is the defect the
//!    modes were built for: the capture read the press and so did the tree, so
//!    binding Delete to a part deleted the part on the way in.
//!
//! Each verdict is a NEGATIVE - the thing that must not have happened - so each
//! is read after a bounded settle rather than on an ack. There is no event for
//! a key that went nowhere.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=280 \
//!   cargo run --example system_input_modes --features debug
//! # look for: `modes: ...` verdict lines per beat,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::{input::keyboard::Key, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_ui::prelude::InputMode;

#[derive(Parser)]
#[command(name = "system_input_modes")]
#[command(version = "1.0.0")]
#[command(about = "The editor keyboard, driven through every input mode. Autopilot-only correctness range - one owner holds the keys at a time", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same editor app the game binary runs - not a bespoke copy.
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(mode_script());
    }

    app.run()
}

/// In-step seconds a gesture beat gets before the run gives up on it.
#[cfg(feature = "debug")]
const BEAT_DEADLINE_SECS: f32 = 20.0;

/// Frames a key that must do NOTHING is given to do it in.
///
/// The other beats in this walk wait on an ack, because something happened.
/// These four wait on a count, because the verdict is that nothing did: there
/// is no message for a verb that never ran, and a press read one frame after it
/// was made would pass whether the arbiter worked or not.
#[cfg(feature = "debug")]
const SETTLE_FRAMES: u32 = 8;

/// The top-bar menu carrying Add Ship and the object palette.
#[cfg(feature = "debug")]
const ADD_MENU: &str = "Add Menu Button";

/// The palette row the walk adds its object from. A beacon carries a Name
/// field, which is the text field the Insert beats need.
#[cfg(feature = "debug")]
const OBJECT_ITEM: &str = "Add Beacon";

/// The object's Name field, in the inspector that opens on it.
#[cfg(feature = "debug")]
const NAME_FIELD: &str = "Inspector Field Name";

/// The top-bar menu carrying Rebind Key.
#[cfg(feature = "debug")]
const SHIP_MENU: &str = "Ship Menu Button";

/// The row that arms a keybind capture on the selected section.
#[cfg(feature = "debug")]
const REBIND_ITEM: &str = "Rebind Key Item";

/// The part the walk founds its ship with. A thruster because Bind needs a
/// BINDABLE section: a hull takes no key, and Rebind Key stays greyed on one.
#[cfg(feature = "debug")]
const FOUNDING_PART: &str = "basic_thruster_section";

/// The inspector row a bindable section shows and nothing else does - the ack
/// that the stage click marked the part rather than missing it.
#[cfg(feature = "debug")]
const KEY_ROW: &str = "Inspector Row Key";

/// A viewport point (logical px) with nothing under it on the 1024x768 window
/// the app opens - where the founding click lands. The rail takes the left 210
/// and the inspector the right 300, so the clear band is narrow and off-centre.
#[cfg(feature = "debug")]
const EMPTY_SPACE: Vec2 = Vec2::new(460.0, 660.0);

/// Advance once the keyboard belongs to `mode`.
#[cfg(feature = "debug")]
fn the_mode_is(mode: InputMode) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<InputMode>()
            .is_some_and(|current| *current == mode)
    })
}

/// Advance once the editor is inside a ship - what Add Ship does.
///
/// False while there is no [`EditorProbe`] at all: the probe arrives with the
/// editor and this walk starts in the menu.
#[cfg(feature = "debug")]
fn inside_a_ship() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<EditorProbe>()
            .is_some_and(|probe| probe.inside.is_some())
    })
}

/// Advance once the pointer is resting on a document node - the ack that the
/// aim reached the part rather than the empty stage behind it.
#[cfg(feature = "debug")]
fn the_pointer_rests_on_a_node() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<EditorProbe>()
            .is_some_and(|probe| probe.hovered_node.is_some())
    })
}

/// Advance once the Scene tree has a node marked - what the stage click does.
#[cfg(feature = "debug")]
fn something_is_marked() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<EditorProbe>()
            .is_some_and(|probe| probe.selected_node.is_some())
    })
}

/// How many nodes the current context holds - ships and objects at the scenario
/// node, sections inside a ship.
#[cfg(feature = "debug")]
fn nodes_here(world: &World) -> usize {
    world
        .get_resource::<EditorProbe>()
        .map_or(0, |probe| probe.context_nodes.len())
}

/// The viewport point the lowest visible section of the edited ship projects
/// to - where a click reaches the part itself.
///
/// Visible scopes it to the ship being built: the document opens seeded with a
/// stock range whose hulks are ship nodes with sections of their own, and
/// entering a ship takes those off the stage.
#[cfg(feature = "debug")]
fn aim_at_a_section(world: &mut World) -> Option<Vec2> {
    let mut q_sections =
        world.query_filtered::<(&GlobalTransform, &InheritedVisibility), With<SectionMarker>>();
    let at = q_sections
        .iter(world)
        .filter(|(_, visible)| visible.get())
        .map(|(pose, _)| pose.translation())
        .next()?;
    let camera_entity = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()?;
    let camera = world.get::<Camera>(camera_entity)?;
    let camera_pose = world.get::<GlobalTransform>(camera_entity)?;
    camera.world_to_viewport(camera_pose, at).ok()
}

/// The walk: menu -> editor -> an object typed into -> a ship browsed -> a
/// rebind armed, with one key pressed against each mode.
#[cfg(feature = "debug")]
#[expect(
    clippy::too_many_lines,
    reason = "one walk, one beat per gesture: splitting it hides the order the modes are entered in"
)]
fn mode_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("modes: reach the main menu")
        .until(state_is(GameStates::MainMenu))
        .deadline(90.0)
        .add()
        .step("modes: let the menu lay out")
        .until(ui_node_present("Sandbox Button"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: click Sandbox")
        .on_enter(click_named("Sandbox Button"))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release Sandbox")
        .on_enter(release_mouse(MouseButton::Left))
        .until(state_is(GameStates::Playing))
        .deadline(90.0)
        .add()
        .step("modes: let the editor lay out")
        .until(ui_node_present(ADD_MENU))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: the keyboard starts normal")
        .until(the_mode_is(InputMode::Normal))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // INSERT. The palette lives in the Add menu, which spawns its rows when
        // it opens, so the menu drops first.
        .step("modes: drop the Add menu")
        .on_enter(click_named(ADD_MENU))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release the Add menu")
        .on_enter(release_mouse(MouseButton::Left))
        .until(ui_node_present(OBJECT_ITEM))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: click Add Beacon")
        .on_enter(click_named(OBJECT_ITEM))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // Placed objects are marked on arrival, so the inspector opens on the
        // beacon and the Name field is there to be typed into.
        .step("modes: release Add Beacon")
        .on_enter(release_mouse(MouseButton::Left))
        .until(ui_node_present(NAME_FIELD))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: click the Name field")
        .on_enter(click_named(NAME_FIELD))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: the field takes the keyboard")
        .on_enter(release_mouse(MouseButton::Left))
        .until(and(editor_field_focused(), the_mode_is(InputMode::Insert)))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: count what the document holds")
        .on_enter(stamp_the_count)
        .add()
        .step("modes: press Delete with the caret in the field")
        .on_enter(press_key(KeyCode::Delete))
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release Delete")
        .on_enter(release_key(KeyCode::Delete))
        .add()
        .step("modes: the field kept the key")
        .on_enter(read_insert_kept_the_object)
        .add()
        // NORMAL. Escape is the field's own rung while it holds the caret: it
        // puts back what was there and lets go, and the editor does not back
        // out from under it.
        // Both halves of a real press: the button edge every verb reads, and
        // the logical key a text field reads. The field takes it and the
        // back-out verb does not, which is the whole claim.
        .step("modes: Escape leaves the field")
        .on_enter(|world: &mut World| {
            press_key(KeyCode::Escape)(world);
            press_edit_key(Key::Escape)(world);
        })
        .until(the_mode_is(InputMode::Normal))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release Escape")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        .step("modes: press Delete with the keyboard free")
        .on_enter(press_key(KeyCode::Delete))
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release Delete again")
        .on_enter(release_key(KeyCode::Delete))
        .add()
        .step("modes: the same key took the object")
        .on_enter(read_normal_took_the_object)
        .add()
        // BROWSE. A ship to be inside, and a gallery over it.
        .step("modes: drop the Add menu again")
        .on_enter(click_named(ADD_MENU))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release the Add menu again")
        .on_enter(release_mouse(MouseButton::Left))
        .until(ui_node_present("Add Ship Button"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: click Add Ship")
        .on_enter(click_named("Add Ship Button"))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release Add Ship")
        .on_enter(release_mouse(MouseButton::Left))
        .until(inside_a_ship())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: open the gallery")
        .on_enter(press_key(KeyCode::Tab))
        .until(and(editor_gallery_open(), the_mode_is(InputMode::Browse)))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release Tab")
        .on_enter(release_key(KeyCode::Tab))
        .add()
        .step("modes: Escape closes the gallery")
        .on_enter(press_key(KeyCode::Escape))
        .until(editor_gallery_closed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release Escape again")
        .on_enter(release_key(KeyCode::Escape))
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: one press spent one rung")
        .on_enter(read_browse_kept_the_ship)
        .add()
        // BIND. A bindable part to arm the capture on: the ship starts blank,
        // so the walk founds it with a thruster from the gallery.
        .step("modes: open the gallery again")
        .on_enter(press_key(KeyCode::Tab))
        .until(editor_gallery_open())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release Tab again")
        .on_enter(release_key(KeyCode::Tab))
        .add()
        .step("modes: put the caret in the filter")
        .on_enter(press_key(KeyCode::Slash))
        .until(editor_filter_focused())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release /")
        .on_enter(release_key(KeyCode::Slash))
        .add()
        .step("modes: filter to the thruster")
        .on_enter(type_text(FOUNDING_PART))
        .until(editor_gallery_selected(FOUNDING_PART))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: Enter to focus")
        .on_enter(press_key(KeyCode::Enter))
        .until(ui_node_present("Gallery Focus Card"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release Enter")
        .on_enter(release_key(KeyCode::Enter))
        .add()
        .step("modes: Enter to take the part")
        .on_enter(press_key(KeyCode::Enter))
        .until(editor_gallery_closed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release Enter again")
        .on_enter(release_key(KeyCode::Enter))
        .add()
        .step("modes: point at empty space")
        .on_enter(move_cursor(EMPTY_SPACE))
        .until(and(pointer_at(EMPTY_SPACE), editor_placement_clear()))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: found the ship")
        .on_enter(press_mouse(MouseButton::Left))
        .until(the_ship_is_up())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release the founding click")
        .on_enter(release_mouse(MouseButton::Left))
        .until(pointer_released())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: put the part down")
        .on_enter(press_key(KeyCode::Escape))
        .until(editor_tool_is(EditorTool::Select))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release Escape a third time")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        // The mark, taken by clicking the part itself: Rebind Key acts on what
        // the Scene tree has marked, and stays greyed with nothing bindable
        // under it.
        .step("modes: aim at the thruster")
        .on_enter(|world: &mut World| {
            let at = aim_at_a_section(world)
                .expect("the founded thruster, the 3D camera and the window are all up");
            move_cursor(at)(world);
        })
        .until(the_pointer_rests_on_a_node())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: press on the thruster")
        .on_enter(press_mouse(MouseButton::Left))
        .until(something_is_marked())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release on the thruster")
        .on_enter(release_mouse(MouseButton::Left))
        .until(ui_node_present(KEY_ROW))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: drop the Ship menu")
        .on_enter(click_named(SHIP_MENU))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release the Ship menu")
        .on_enter(release_mouse(MouseButton::Left))
        .until(ui_node_present(REBIND_ITEM))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: click Rebind Key")
        .on_enter(click_named(REBIND_ITEM))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // Armed by a click, the capture waits for that click to be released
        // before it reads a press - so the arming button is not itself bound.
        .step("modes: the capture takes the keyboard")
        .on_enter(release_mouse(MouseButton::Left))
        .until(the_mode_is(InputMode::Bind))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: count the sections on the ship")
        .on_enter(stamp_the_count)
        .add()
        .step("modes: press Delete while the capture waits")
        .on_enter(press_key(KeyCode::Delete))
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("modes: release Delete a third time")
        .on_enter(release_key(KeyCode::Delete))
        .add()
        .step("modes: the part survived the key it was bound to")
        .on_enter(read_bind_kept_the_part)
        .add()
}

/// What the context held before the key that must not change it.
#[cfg(feature = "debug")]
#[derive(Resource, Debug, Clone, Copy)]
struct NodesBefore(usize);

/// Stamp the count the next verdict is read against.
#[cfg(feature = "debug")]
fn stamp_the_count(world: &mut World) {
    let count = nodes_here(world);
    assert!(
        count > 0,
        "the walk stamped an empty context, so the verdict after it would pass on nothing"
    );
    world.insert_resource(NodesBefore(count));
    info!("modes: the context holds {count} nodes");
}

/// The count stamped by the beat before, and what it holds now.
#[cfg(feature = "debug")]
fn counts(world: &World) -> (usize, usize) {
    let before = world
        .get_resource::<NodesBefore>()
        .expect("a beat stamped the count before this one")
        .0;
    (before, nodes_here(world))
}

#[cfg(feature = "debug")]
fn read_insert_kept_the_object(world: &mut World) {
    let (before, now) = counts(world);
    assert_eq!(
        now, before,
        "Delete with the caret in a field took a node off the document: the character belongs to \
         the field, and the tree is not the keyboard's owner while one is focused"
    );
    nova_probe::probe_marker(
        world,
        "outcome: insert mode keeps delete off the tree",
        serde_json::json!({ "nodes_before": before, "nodes_after": now }),
    );
    info!("modes: the field took Delete, and the beacon stayed");
}

#[cfg(feature = "debug")]
fn read_normal_took_the_object(world: &mut World) {
    let (before, now) = counts(world);
    assert_eq!(
        now,
        before - 1,
        "the same Delete on a free keyboard left the document alone, so the mode never came back \
         to Normal and every verb in the editor is dead"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the keyboard comes back to normal",
        serde_json::json!({ "nodes_before": before, "nodes_after": now }),
    );
    info!("modes: the keyboard came back, and the same key took the beacon");
}

#[cfg(feature = "debug")]
fn read_browse_kept_the_ship(world: &mut World) {
    let probe = world
        .get_resource::<EditorProbe>()
        .expect("the editor is up")
        .clone();
    assert!(
        !probe.gallery_open,
        "the gallery is still up, so the Escape never reached its owner"
    );
    assert!(
        probe.inside.is_some(),
        "one Escape closed the gallery AND left the ship: the back-out verb answered a key the \
         gallery owned, which is two rungs of context for one press"
    );
    nova_probe::probe_marker(
        world,
        "outcome: browse mode keeps escape off the back-out",
        serde_json::json!({ "inside": probe.inside }),
    );
    info!("modes: the gallery took Escape, and the editor stayed inside the ship");
}

#[cfg(feature = "debug")]
fn read_bind_kept_the_part(world: &mut World) {
    let (before, now) = counts(world);
    assert_eq!(
        now, before,
        "Delete pressed to BIND Delete deleted the section it was being bound to - the capture \
         read the press and so did the tree"
    );
    nova_probe::probe_marker(
        world,
        "outcome: bind mode keeps delete off the tree",
        serde_json::json!({ "sections_before": before, "sections_after": now }),
    );
    info!("modes: the capture took Delete, and the thruster stayed");
}

/// Advance once the ship being EDITED has a section on it.
#[cfg(feature = "debug")]
fn the_ship_is_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<EditorProbe>()
            .is_some_and(|probe| !probe.ship.is_empty())
    })
}
