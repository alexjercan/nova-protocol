//! system_field_controls: the inspector's number rows, driven by the pointer.
//!
//! Every leaf used to render as a text box, so a type's constraints had nowhere
//! to live: the fields a kind shows first and the rules those fields are typed
//! under were two hand-kept lists over the same names, and a name could sit in
//! one and not the other. One `FieldSpec` per field is now both - the pick, the
//! unit, the floor and the step a drag lands on.
//!
//! The visible half of that is a control a number can be SCRUBBED with, which
//! is what the run drives:
//!
//! 1. A rock's Radius wears the unit its declaration gives it.
//! 2. Its NAME is the grip. Dragging the name right moves the number by the
//!    step the field was declared with - forty pixels at `0.5` is twenty meters.
//!    A number reached this way cannot be `nan`, which is the point: the value
//!    the typed box has to refuse is one this control cannot express.
//! 3. Dragging far the other way ARRIVES at the floor instead of being refused.
//!    A typed negative radius is a mistake; a drag that keeps going is a
//!    builder asking for the smallest value there is.
//! 4. A row that is not a number has no grip: a flag is ticked, not scrubbed.
//! 5. The grip on ONE AXIS of a vector moves by the ROW's step. The step used
//!    to be resolved a second time from the axis path, where `x` matches no
//!    declaration: the drag was scaled by the axis fallback rather than the
//!    row's own step and the result snapped onto a different grid, so a pose
//!    scrub stood still at most coordinates.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=280 \
//!   cargo run --example system_field_controls --features debug
//! # look for: `fields: ...` verdict lines per beat,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_field_controls")]
#[command(version = "1.0.0")]
#[command(about = "The inspector's number rows, scrubbed by the pointer. Autopilot-only correctness range - one declaration per field drives its control", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same editor app the game binary runs - not a bespoke copy.
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(field_script());
    }

    app.run()
}

/// The top-bar menu carrying the object palette.
#[cfg(feature = "debug")]
const ADD_MENU: &str = "Add Menu Button";

/// The palette row the walk adds its object from. A rock, because its config is
/// the one that carries all four shapes at once: a plain number, an optional
/// number, a flag and a whole number.
#[cfg(feature = "debug")]
const OBJECT_ITEM: &str = "Add Asteroid";

/// The row the walk scrubs, and the grip that scrubs it.
#[cfg(feature = "debug")]
const RADIUS_ROW: &str = "Radius";
#[cfg(feature = "debug")]
const RADIUS_GRIP: &str = "Inspector Grip Radius";

/// The row that must NOT have one.
#[cfg(feature = "debug")]
const FLAG_GRIP: &str = "Inspector Grip Invulnerable";

/// The grip on one axis of a VECTOR row, and the step that row declares. The
/// axis letter is the grip, because the panel is 240px wide.
#[cfg(feature = "debug")]
const POSE_GRIP: &str = "Inspector Grip Position X";
#[cfg(feature = "debug")]
const POSE_STEP: f32 = 0.5;

/// The unit `radius` is declared with, and the step it is dragged by. Meters:
/// an asteroid's radius is a [`Meters`] on the config, which is what the
/// inspector reads the row's dimension off.
#[cfg(feature = "debug")]
const RADIUS_UNIT: &str = "m";
#[cfg(feature = "debug")]
const RADIUS_STEP: f32 = 0.5;

/// How far the scrub pulls the grip, in pixels. Right first, then far enough
/// left that the floor is the only thing that could stop it.
#[cfg(feature = "debug")]
const PULL_PX: f32 = 40.0;
#[cfg(feature = "debug")]
const OVERPULL_PX: f32 = 600.0;

/// What the inspector reads for the row called `label`, off the document.
#[cfg(feature = "debug")]
fn inspector_says(world: &World, label: &str) -> Option<String> {
    world.get_resource::<EditorProbe>().and_then(|probe| {
        probe
            .inspector
            .iter()
            .find(|(name, _)| name == label)
            .map(|(_, value)| value.clone())
    })
}

/// The number the Radius row holds right now.
#[cfg(feature = "debug")]
fn radius_now(world: &World) -> f32 {
    inspector_says(world, RADIUS_ROW)
        .expect("the rock's inspector is up and shows its radius")
        .parse()
        .expect("a radius reads as a number")
}

/// Advance once the pointer is resting on the UI node called `name`.
#[cfg(feature = "debug")]
fn the_pointer_is_on(
    name: &'static str,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let Some(hit) = world
            .try_query::<&bevy::picking::pointer::PointerInteraction>()
            .and_then(|mut pointers| {
                pointers
                    .iter(world)
                    .filter_map(|interaction| interaction.get_nearest_hit())
                    .map(|(entity, _)| *entity)
                    .next()
            })
        else {
            return false;
        };
        world
            .get::<Name>(hit)
            .is_some_and(|named| named.as_str() == name)
    })
}

/// Advance once the radius has left the value the last stamp took.
#[cfg(feature = "debug")]
fn the_radius_moved() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(before) = world.get_resource::<RadiusBefore>() else {
            return false;
        };
        (radius_now(world) - before.0).abs() > f32::EPSILON
    })
}

/// Where the selected node sits, off the same snapshot a drag beat reads.
#[cfg(feature = "debug")]
fn position_of(world: &World) -> Option<Vec3> {
    let probe = world.get_resource::<EditorProbe>()?;
    let id = probe.selected_node.clone()?;
    probe
        .node_positions
        .iter()
        .find(|(node, _)| *node == id)
        .map(|(_, at)| *at)
}

/// Advance once the rock has left the place the last stamp took.
#[cfg(feature = "debug")]
fn the_pose_moved() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(before) = world.get_resource::<PoseBefore>() else {
            return false;
        };
        position_of(world).is_some_and(|at| (at.x - before.0.x).abs() > f32::EPSILON)
    })
}

/// Where the rock sat before the drag that must move it.
#[cfg(feature = "debug")]
#[derive(Resource, Debug, Clone, Copy)]
struct PoseBefore(Vec3);

/// Stamp the pose the next verdict is read against.
#[cfg(feature = "debug")]
fn stamp_the_pose(world: &mut World) {
    let at = position_of(world).expect("the placed rock is selected and has a pose");
    world.insert_resource(PoseBefore(at));
    info!("fields: the rock sits at {at}");
}

/// What the radius read before the drag that must change it.
#[cfg(feature = "debug")]
#[derive(Resource, Debug, Clone, Copy)]
struct RadiusBefore(f32);

/// Stamp the reading the next verdict is read against.
#[cfg(feature = "debug")]
fn stamp_the_radius(world: &mut World) {
    let radius = radius_now(world);
    world.insert_resource(RadiusBefore(radius));
    info!("fields: the rock's radius reads {radius}");
}

/// The walk: menu -> editor -> a rock -> its radius pulled two ways.
#[cfg(feature = "debug")]
fn field_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("fields: reach the main menu")
        .until(state_is(GameStates::MainMenu))
        .deadline(90.0)
        .add()
        .step("fields: let the menu lay out")
        .until(ui_node_present("Sandbox Button"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: click Sandbox")
        .on_enter(click_named("Sandbox Button"))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: release Sandbox")
        .on_enter(release_mouse(MouseButton::Left))
        .until(state_is(GameStates::Playing))
        .deadline(90.0)
        .add()
        .step("fields: let the editor lay out")
        .until(ui_node_present(ADD_MENU))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // The palette lives in the Add menu, which spawns its rows when it
        // opens, so the menu drops first.
        .step("fields: drop the Add menu")
        .on_enter(click_named(ADD_MENU))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: release the Add menu")
        .on_enter(release_mouse(MouseButton::Left))
        .until(ui_node_present(OBJECT_ITEM))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: click Add Asteroid")
        .on_enter(click_named(OBJECT_ITEM))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // Placed objects are marked on arrival, so the inspector opens on the
        // rock and its rows are there to be read.
        .step("fields: release Add Asteroid")
        .on_enter(release_mouse(MouseButton::Left))
        .until(ui_node_present(RADIUS_GRIP))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: the rows wear what their declarations give them")
        .on_enter(read_the_rows_match_their_types)
        .add()
        // The scrub. The grip is the row's NAME - the panel is 240px wide, and
        // a row that spends pixels on a grip of its own spends them on the box
        // holding the number.
        .step("fields: stamp the radius")
        .on_enter(stamp_the_radius)
        .add()
        .step("fields: aim at the radius grip")
        .on_enter(hover_named(RADIUS_GRIP))
        .until(the_pointer_is_on(RADIUS_GRIP))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: take the grip")
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: pull the radius up")
        .on_enter(|world: &mut World| {
            let at = ui_node_centre(world, RADIUS_GRIP).expect("the grip is on screen");
            move_cursor(at + Vec2::new(PULL_PX, 0.0))(world);
        })
        .until(the_radius_moved())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: let the grip go")
        .on_enter(release_mouse(MouseButton::Left))
        .until(pointer_released())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: the number followed the pointer by its own step")
        .on_enter(read_the_scrub_moved_by_its_step)
        .add()
        // And the other way, past where the value stops being one.
        .step("fields: stamp the radius again")
        .on_enter(stamp_the_radius)
        .add()
        .step("fields: aim at the radius grip again")
        .on_enter(hover_named(RADIUS_GRIP))
        .until(the_pointer_is_on(RADIUS_GRIP))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: take the grip again")
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: pull the radius through the floor")
        .on_enter(|world: &mut World| {
            let at = ui_node_centre(world, RADIUS_GRIP).expect("the grip is on screen");
            move_cursor(at - Vec2::new(OVERPULL_PX, 0.0))(world);
        })
        .until(the_radius_moved())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: let the grip go again")
        .on_enter(release_mouse(MouseButton::Left))
        .until(pointer_released())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: the scrub arrived at the floor")
        .on_enter(read_the_scrub_stopped_at_the_floor)
        .add()
        // And the same gesture on ONE AXIS of a vector, which is the row whose
        // step had nowhere to be looked up from.
        .step("fields: stamp the pose")
        .on_enter(stamp_the_pose)
        .add()
        .step("fields: aim at the X grip")
        .on_enter(hover_named(POSE_GRIP))
        .until(the_pointer_is_on(POSE_GRIP))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: take the X grip")
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: pull X along")
        .on_enter(|world: &mut World| {
            let at = ui_node_centre(world, POSE_GRIP).expect("the X grip is on screen");
            move_cursor(at + Vec2::new(PULL_PX, 0.0))(world);
        })
        .until(the_pose_moved())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: let the X grip go")
        .on_enter(release_mouse(MouseButton::Left))
        .until(pointer_released())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("fields: the axis moved by the row's own step")
        .on_enter(read_the_axis_scrub_moved_by_its_step)
        .add()
}

/// The unit beside the radius, and the grip a flag must not have.
#[cfg(feature = "debug")]
fn read_the_rows_match_their_types(world: &mut World) {
    let unit = world
        .try_query::<(&Name, &Text)>()
        .and_then(|mut labels| {
            labels
                .iter(world)
                .find(|(name, _)| name.as_str() == format!("Inspector Unit {RADIUS_ROW}"))
                .map(|(_, text)| text.0.clone())
        })
        .expect("the radius row has a unit slot");
    assert_eq!(
        unit, RADIUS_UNIT,
        "the radius wears the unit its declaration gives it: a number a builder cannot read the \
         units of is a number they have to guess at"
    );
    assert!(
        ui_node_rect(world, RADIUS_GRIP).is_some(),
        "a number's name is its grip"
    );
    assert!(
        ui_node_rect(world, FLAG_GRIP).is_none(),
        "a flag is ticked, not scrubbed: a grip on it would offer a gesture with nothing to do"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a declared field wears its own unit",
        serde_json::json!({ "unit": unit }),
    );
    info!("fields: the radius reads in `{unit}`, and only the numbers have grips");
}

#[cfg(feature = "debug")]
fn read_the_scrub_moved_by_its_step(world: &mut World) {
    let before = world.resource::<RadiusBefore>().0;
    let now = radius_now(world);
    let wanted = before + PULL_PX * RADIUS_STEP;
    assert!(
        (now - wanted).abs() < RADIUS_STEP,
        "the scrub moves the number by the step the field was declared with: {PULL_PX} px at \
         {RADIUS_STEP} is {wanted}, and the row reads {now}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a number is scrubbed by its own name",
        serde_json::json!({ "before": before, "after": now, "step": RADIUS_STEP }),
    );
    info!("fields: the radius went {before} -> {now} on a {PULL_PX}px pull");
}

#[cfg(feature = "debug")]
fn read_the_axis_scrub_moved_by_its_step(world: &mut World) {
    let before = world.resource::<PoseBefore>().0;
    let now = position_of(world).expect("the rock still has a pose");
    let wanted = before.x + PULL_PX * POSE_STEP;
    assert!(
        (now.x - wanted).abs() < POSE_STEP,
        "a grip on one axis moves by the ROW's step: {PULL_PX} px at {POSE_STEP} is {wanted}, and          the rock sits at {}. A stall here means the step is being resolved a second time from          the axis path, where no declaration matches it",
        now.x
    );
    nova_probe::probe_marker(
        world,
        "outcome: a vector axis is scrubbed by its row's step",
        serde_json::json!({ "before": before.x, "after": now.x, "step": POSE_STEP }),
    );
    info!(
        "fields: X went {} -> {} on a {PULL_PX}px pull",
        before.x, now.x
    );
}

#[cfg(feature = "debug")]
fn read_the_scrub_stopped_at_the_floor(world: &mut World) {
    let now = radius_now(world);
    assert!(
        now.abs() < f32::EPSILON,
        "a scrub through the floor ARRIVES at it: the row reads {now}, so either the floor is not \
         declared or a drag is being refused the way a typed number is"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a scrub arrives at the floor",
        serde_json::json!({ "after": now }),
    );
    info!("fields: the radius stopped at {now} instead of going negative");
}
