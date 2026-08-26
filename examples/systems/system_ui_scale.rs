//! system_ui_scale: a world-anchored label stands in the same place at any
//! landscape size and any DPI scale.
//!
//! `ComputedNode::size()` is PHYSICAL pixels and `Node::left`/`top` are
//! LOGICAL. They are the same number at 1x, which is why the editor's chips,
//! plates and callouts read fine on a desk monitor and misplaced themselves by
//! their own height on a HiDPI screen. The four places that projected a world
//! point, measured a label and clamped it inside the viewport now share one
//! `nova_ui::screen::hang_at`, so there is one place left to get the units
//! right.
//!
//! The run proves the invariant rather than the arithmetic: a keybind chip
//! hangs a fixed LOGICAL distance over the part it names, and that distance
//! does not move when the scale factor doubles or the window changes shape.
//!
//! 1. Stamp the gap and the lead at 1024x768, scale 1.
//! 2. Double the scale factor. The chip keeps the same logical offsets.
//! 3. Go wide, then narrow. It keeps them again, and stays inside the viewport.
//!
//! The top bar is read at every shape as well. A number measured at 760x600
//! says nothing about whether the bar at 760x600 can be READ: a flex column
//! allowed below its own content overflows rather than clipping, and the run
//! that only stamped the chip's offsets watched Play and the Ship menu draw
//! over each other.
//!
//! The stage's nameplates are read at the end, from the scenario the editor
//! opens with: several hulks a hand's width apart project to the same few
//! pixels, and a placement with no de-collision draws `Derelict Hulk 1` and
//! `Derelict Hulk 0` into one unreadable line.
//!
//! Portrait is out of scope on purpose: the bound is what makes this
//! finishable.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=280 \
//!   cargo run --example system_ui_scale --features debug
//! # look for: `scale: ...` verdict lines per beat,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::{prelude::*, window::PrimaryWindow};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_ui_scale")]
#[command(version = "1.0.0")]
#[command(about = "A world-anchored editor label, swept across DPI scales and landscape window sizes. Autopilot-only correctness range", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same editor app the game binary runs - not a bespoke copy.
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(scale_script());
    }

    app.run()
}

/// In-step seconds a gesture beat gets before the run gives up on it.
#[cfg(feature = "debug")]
const BEAT_DEADLINE_SECS: f32 = 20.0;

/// Frames a resize is given to reach layout in.
///
/// A window change is not one frame's work: winit answers the request, the UI
/// re-lays out, and the chip reads its own size from the pass before it.
#[cfg(feature = "debug")]
const SETTLE_FRAMES: u32 = 24;

/// The top-bar menu carrying Add Ship.
#[cfg(feature = "debug")]
const ADD_MENU: &str = "Add Menu Button";

/// The part the walk founds its ship with. A thruster because the chip only
/// exists for a BINDABLE section: a hull takes no key and wears no chip.
#[cfg(feature = "debug")]
const FOUNDING_PART: &str = "basic_thruster_section";

/// The world-anchored label under test.
#[cfg(feature = "debug")]
const CHIP: &str = "Section Keybind Label";

/// The top-bar controls that must never be drawn over one another: the five
/// menus on the left, and Play in the middle of the screen.
///
/// The breadcrumb is not here because it can legitimately measure zero wide -
/// it is inside a clip, which is the whole point of it.
#[cfg(feature = "debug")]
const BAR_CONTROLS: [&str; 6] = [
    "File Menu Button",
    "Edit Menu Button",
    "View Menu Button",
    "Add Menu Button",
    "Ship Menu Button",
    "Play Button",
];

/// A viewport point (logical px) with nothing under it on the 1024x768 window
/// the app opens - where the founding click lands. The rail takes the left 210
/// and the inspector the right 300, so the clear band is narrow and off-centre.
#[cfg(feature = "debug")]
const EMPTY_SPACE: Vec2 = Vec2::new(460.0, 660.0);

/// How far the chip may drift from the offsets it was stamped with, in logical
/// pixels.
///
/// Not zero: the chip reads its own height from the pass before, so a frame in
/// which the label's text changed width lands a pixel out. The defect this
/// range exists for moves it by the chip's whole height.
#[cfg(feature = "debug")]
const DRIFT_PX: f32 = 2.0;

/// Where the chip stands relative to the part it names, in logical pixels: the
/// gap above the part, and the lead to the right of it.
#[cfg(feature = "debug")]
#[derive(Resource, Debug, Clone, Copy)]
struct ChipOffsets {
    gap: f32,
    lead: f32,
}

/// The viewport point the lowest visible section of the edited ship projects
/// to - the world anchor the chip is placed against.
#[cfg(feature = "debug")]
fn part_on_screen(world: &mut World) -> Option<Vec2> {
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

/// The viewport, in the logical pixels a `Node` is placed in.
#[cfg(feature = "debug")]
fn viewport(world: &mut World) -> Vec2 {
    let camera_entity = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the editor's 3D camera is up");
    world
        .get::<Camera>(camera_entity)
        .and_then(Camera::logical_viewport_size)
        .expect("the camera has a viewport")
}

/// Where the chip stands right now, against the part it names.
#[cfg(feature = "debug")]
fn offsets_now(world: &mut World) -> ChipOffsets {
    let anchor = part_on_screen(world).expect("the founded thruster is on screen");
    let rect = ui_node_rect(world, CHIP).expect("the thruster wears its keybind chip");
    let screen = viewport(world);
    assert!(
        rect.min.x >= -DRIFT_PX
            && rect.min.y >= -DRIFT_PX
            && rect.max.x <= screen.x + DRIFT_PX
            && rect.max.y <= screen.y + DRIFT_PX,
        "the chip left the viewport: {rect:?} is not inside {screen:?}"
    );
    ChipOffsets {
        gap: anchor.y - rect.max.y,
        lead: rect.min.x - anchor.x,
    }
}

/// Every top-bar control stands in its own pixels.
#[cfg(feature = "debug")]
fn read_the_bar(shape: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let placed: Vec<(&str, Rect)> = BAR_CONTROLS
            .into_iter()
            .map(|name| {
                let rect = ui_node_rect(world, name)
                    .unwrap_or_else(|| panic!("at {shape} the top bar draws `{name}`"));
                (name, rect)
            })
            .collect();
        for (index, (name, rect)) in placed.iter().enumerate() {
            for (other, second) in &placed[index + 1..] {
                let apart = rect.max.x <= second.min.x
                    || second.max.x <= rect.min.x
                    || rect.max.y <= second.min.y
                    || second.max.y <= rect.min.y;
                assert!(
                    apart,
                    "at {shape} `{name}` {rect:?} is drawn over `{other}` {second:?}: a bar \
                     column allowed below its own content overflows instead of clipping, and \
                     both labels come out unreadable"
                );
            }
        }
        nova_probe::probe_marker(
            world,
            "outcome: the top bar keeps its controls apart",
            serde_json::json!({ "shape": shape, "controls": BAR_CONTROLS.len() }),
        );
        info!("scale: at {shape} the top bar still reads");
    }
}

/// How tall the key legend may be, in logical pixels - the editor's own bound,
/// which is the three rows the stock shape already draws.
#[cfg(feature = "debug")]
const LEGEND_MAX_H: f32 = 56.0;

/// The key legend keeps to that bound at every shape.
///
/// It wraps by design, and a narrower band wraps it further: the same nine
/// cells measured 56 tall at 1024x768 and 96 at 760x600, over the stage the
/// hints are about.
#[cfg(feature = "debug")]
fn read_the_legend(shape: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let rect =
            ui_node_rect(world, "Editor Key Legend").expect("the editor draws its key legend");
        assert!(
            rect.height() <= LEGEND_MAX_H + DRIFT_PX,
            "at {shape} the legend is {} tall: a hint strip that grows with how narrow the \
             window is eats the build area it is a hint about",
            rect.height()
        );
        info!("scale: at {shape} the legend is {} tall", rect.height());
    }
}

/// The prefix every stage nameplate's `Name` carries.
#[cfg(feature = "debug")]
const PLATE: &str = "Name Plate ";

/// Every nameplate on the stage stands in its own pixels.
#[cfg(feature = "debug")]
fn read_the_plates(world: &mut World) {
    let mut plates = world.query::<(
        &Name,
        &bevy::ui::UiGlobalTransform,
        &ComputedNode,
        &InheritedVisibility,
    )>();
    let placed: Vec<(String, Rect)> = plates
        .iter(world)
        .filter(|(name, _, _, visible)| visible.get() && name.as_str().starts_with(PLATE))
        .map(|(name, transform, computed, _)| {
            let scale = computed.inverse_scale_factor();
            (
                name.as_str().to_string(),
                Rect::from_center_size(transform.translation * scale, computed.size() * scale),
            )
        })
        .filter(|(_, rect)| rect.width() > 0.0 && rect.height() > 0.0)
        .collect();
    assert!(
        placed.len() >= 2,
        "the scenario the editor opens with names several hulks, and the frame put {} plate(s) \
         on screen - there is nothing here to collide",
        placed.len()
    );
    for (index, (name, rect)) in placed.iter().enumerate() {
        for (other, second) in &placed[index + 1..] {
            let apart = rect.max.x <= second.min.x
                || second.max.x <= rect.min.x
                || rect.max.y <= second.min.y
                || second.max.y <= rect.min.y;
            assert!(
                apart,
                "`{name}` {rect:?} is drawn over `{other}` {second:?}: two hulls a hand's width \
                 apart project to the same few pixels, and a pile of names names nothing"
            );
        }
    }
    nova_probe::probe_marker(
        world,
        "outcome: the stage's names stand apart",
        serde_json::json!({ "plates": placed.len() }),
    );
    info!("scale: {} nameplates, none over another", placed.len());
}

/// Advance once the editor is back out at the scenario.
#[cfg(feature = "debug")]
fn outside_the_ship() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<EditorProbe>()
            .is_some_and(|probe| probe.inside.is_none())
    })
}

/// Set the window's scale factor, the way a HiDPI screen does.
#[cfg(feature = "debug")]
fn set_scale(factor: f32) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
        let mut window = windows.single_mut(world).expect("one primary window");
        window.resolution.set_scale_factor_override(Some(factor));
    }
}

/// Reshape the window, the way dragging its corner does.
#[cfg(feature = "debug")]
fn set_size(width: f32, height: f32) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
        let mut window = windows.single_mut(world).expect("one primary window");
        window.resolution.set(width, height);
    }
}

/// Advance once the editor is inside a ship - what Add Ship does.
#[cfg(feature = "debug")]
fn inside_a_ship() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<EditorProbe>()
            .is_some_and(|probe| probe.inside.is_some())
    })
}

/// Advance once the ship being edited holds a section.
#[cfg(feature = "debug")]
fn the_ship_is_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<EditorProbe>()
            .is_some_and(|probe| !probe.ship.is_empty())
    })
}

/// The walk: menu -> editor -> a ship founded with one bindable part -> the
/// chip over it, read at three shapes and two scales.
#[cfg(feature = "debug")]
#[expect(
    clippy::too_many_lines,
    reason = "one walk, one beat per gesture: splitting it hides the order the shapes are swept in"
)]
fn scale_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("scale: reach the main menu")
        .until(state_is(GameStates::MainMenu))
        .deadline(90.0)
        .add()
        .step("scale: let the menu lay out")
        .until(ui_node_present("Sandbox Button"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: click Sandbox")
        .on_enter(click_named("Sandbox Button"))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: release Sandbox")
        .on_enter(release_mouse(MouseButton::Left))
        .until(state_is(GameStates::Playing))
        .deadline(90.0)
        .add()
        .step("scale: let the editor lay out")
        .until(ui_node_present(ADD_MENU))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: drop the Add menu")
        .on_enter(click_named(ADD_MENU))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: release the Add menu")
        .on_enter(release_mouse(MouseButton::Left))
        .until(ui_node_present("Add Ship Button"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: click Add Ship")
        .on_enter(click_named("Add Ship Button"))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: release Add Ship")
        .on_enter(release_mouse(MouseButton::Left))
        .until(inside_a_ship())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // A bindable part to hang a chip off: the ship starts blank, so the
        // walk founds it with a thruster from the gallery.
        .step("scale: open the gallery")
        .on_enter(press_key(KeyCode::Tab))
        .until(editor_gallery_open())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: release Tab")
        .on_enter(release_key(KeyCode::Tab))
        .add()
        .step("scale: put the caret in the filter")
        .on_enter(press_key(KeyCode::Slash))
        .until(editor_filter_focused())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: release /")
        .on_enter(release_key(KeyCode::Slash))
        .add()
        .step("scale: filter to the thruster")
        .on_enter(type_text(FOUNDING_PART))
        .until(editor_gallery_selected(FOUNDING_PART))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: Enter to focus")
        .on_enter(press_key(KeyCode::Enter))
        .until(ui_node_present("Gallery Focus Card"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: release Enter")
        .on_enter(release_key(KeyCode::Enter))
        .add()
        .step("scale: Enter to take the part")
        .on_enter(press_key(KeyCode::Enter))
        .until(editor_gallery_closed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: release Enter again")
        .on_enter(release_key(KeyCode::Enter))
        .add()
        .step("scale: point at empty space")
        .on_enter(move_cursor(EMPTY_SPACE))
        .until(and(pointer_at(EMPTY_SPACE), editor_placement_clear()))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: found the ship")
        .on_enter(press_mouse(MouseButton::Left))
        .until(the_ship_is_up())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: release the founding click")
        .on_enter(release_mouse(MouseButton::Left))
        .until(pointer_released())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: put the part down")
        .on_enter(press_key(KeyCode::Escape))
        .until(editor_tool_is(EditorTool::Select))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: release Escape")
        .on_enter(release_key(KeyCode::Escape))
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // The baseline every later shape is read against.
        .step("scale: stamp where the chip stands")
        .on_enter(|world: &mut World| {
            let offsets = offsets_now(world);
            world.insert_resource(offsets);
            read_the_bar("1024x768")(world);
            read_the_legend("1024x768")(world);
            info!(
                "scale: at 1024x768 the chip hangs {} over the part, {} to the right",
                offsets.gap, offsets.lead
            );
        })
        .add()
        // DPI. The one the units bug is invisible without: every number in the
        // placement is the same at 1x.
        .step("scale: double the scale factor")
        .on_enter(set_scale(2.0))
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: reframe at 2x")
        .on_enter(frame_the_ship)
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: the chip hangs the same at 2x")
        .on_enter(read_the_chip_held("2x"))
        .add()
        .step("scale: the bar reads at 2x")
        .on_enter(read_the_bar("2x"))
        .add()
        // And back, so the shapes below are read in the scale they were sized
        // in.
        .step("scale: back to 1x")
        .on_enter(set_scale(1.0))
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: go wide")
        .on_enter(set_size(1280.0, 600.0))
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: reframe wide")
        .on_enter(frame_the_ship)
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: the chip hangs the same wide")
        .on_enter(read_the_chip_held("1280x600"))
        .add()
        .step("scale: the bar reads wide")
        .on_enter(read_the_bar("1280x600"))
        .add()
        .step("scale: go narrow")
        .on_enter(set_size(760.0, 600.0))
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: reframe narrow")
        .on_enter(frame_the_ship)
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: the chip hangs the same narrow")
        .on_enter(read_the_chip_held("760x600"))
        .add()
        // The width this task's own Done-when nominates, and the one the bar
        // came apart at.
        .step("scale: the bar reads narrow")
        .on_enter(read_the_bar("760x600"))
        .add()
        .step("scale: the legend keeps its bound narrow")
        .on_enter(read_the_legend("760x600"))
        .add()
        // Out to the scenario, where the stage wears the document's names.
        .step("scale: back to the stock shape")
        .on_enter(set_size(1024.0, 768.0))
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: leave the ship")
        .on_enter(press_key(KeyCode::Escape))
        .until(outside_the_ship())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: release Escape again")
        .on_enter(release_key(KeyCode::Escape))
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: frame the whole scenario")
        .on_enter(frame_the_ship)
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("scale: the plates stand apart")
        .on_enter(read_the_plates)
        .add()
}

/// Put the ship back in the middle of whatever shape the window now is - the
/// editor's own Frame verb, so the part is on screen to be measured.
#[cfg(feature = "debug")]
fn frame_the_ship(world: &mut World) {
    press_key(KeyCode::KeyF)(world);
    release_key(KeyCode::KeyF)(world);
}

/// Read the chip against the offsets the baseline stamped.
#[cfg(feature = "debug")]
fn read_the_chip_held(shape: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let stamped = *world
            .get_resource::<ChipOffsets>()
            .expect("the baseline beat stamped the offsets");
        let now = offsets_now(world);
        assert!(
            (now.gap - stamped.gap).abs() < DRIFT_PX,
            "at {shape} the chip hangs {} over its part instead of {}: a placement that reads a \
             PHYSICAL size into a LOGICAL position moves by the label's own height",
            now.gap,
            stamped.gap
        );
        assert!(
            (now.lead - stamped.lead).abs() < DRIFT_PX,
            "at {shape} the chip stands {} to the right of its part instead of {}",
            now.lead,
            stamped.lead
        );
        nova_probe::probe_marker(
            world,
            "outcome: a world-anchored label keeps its logical place",
            serde_json::json!({ "shape": shape, "gap": now.gap, "lead": now.lead }),
        );
        info!(
            "scale: at {shape} the chip still hangs {} over the part",
            now.gap
        );
    }
}
