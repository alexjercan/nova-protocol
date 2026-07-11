//! 09_editor: the ship editor, wired to the headless smoke-test harness.
//!
//! This runs the exact same editor the `nova_protocol` binary launches (via the shared
//! [`editor_app`]), just with the autopilot + screenshot harness attached. The point is to
//! have the editor covered by the same headless smoke test as the gameplay examples: it boots
//! into the editor, the autopilot drives a real editor action (create a ship that has a
//! controller section), and the run exits without panicking.
//!
//! Driving the "new ship with a controller" path also keeps the editor-preview controller fix
//! (task 20260706-212909) honest: a live controller on the non-physics preview root used to
//! flood the log with "root not found" every frame; the preview now uses an inert render-only
//! controller, so this run stays quiet.
//!
//! The autopilot then drives section *placement* the way a user would: it selects a hull section
//! and clicks on the ship, simulating the mouse through the real picking pipeline (a synthetic
//! `PointerInput` over a section, which avian's physics-picking backend raycasts to a hit) so
//! the editor's own `on_click_spaceship_section` observer places the new section. No editor code
//! is changed - everything is driven through public input.
//!
//! Controls (interactive run): use the on-screen buttons to create ships and place sections.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 09_editor --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `editor autopilot: created a ship with a controller`,
//! #           `editor autopilot: placed a section ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

// Only the debug-gated autopilot below names bevy types directly.
#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "09_editor")]
#[command(version = "1.0.0")]
#[command(about = "The nova_protocol ship editor, wired to the smoke-test harness", long_about = None)]
struct Cli;

fn main() {
    let _ = Cli::parse();

    // The same editor app the game/binary runs - not a bespoke copy.
    let mut app = editor_app(true);

    // Headless smoke-test harness: inert in a normal run (gated on BCS_AUTOPILOT / BCS_SHOT).
    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_autopilot().input(editor_autopilot));
        app.add_plugins(nova_screenshot());
    }

    app.run();
}

/// The autopilot's step through the editor: create a ship, select a section, then drive the
/// mouse to place it.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    CreateShip,
    SelectSection,
    Aim,
    Press,
    Release,
    Verify,
    Done,
}

/// Autopilot progress. Held as a resource so the `fn(&mut World)` closure can run a small frame-
/// paced state machine (each phase waits a few frames for the editor/physics to catch up).
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct EditorAutopilot {
    phase: Phase,
    /// Frames to wait before acting on the current phase.
    wait: u32,
    /// The screen-space location to click, computed once we aim.
    location: Option<bevy::picking::pointer::Location>,
    /// Section count captured before the placement click, to confirm it added one.
    sections_before: usize,
}

/// Autopilot: once the editor is up, create a ship, select a hull section, and click on the ship
/// to place it - a full editor interaction driven headless.
#[cfg(feature = "debug")]
fn editor_autopilot(world: &mut World, _elapsed: f32) {
    use bevy::{
        picking::pointer::{PointerAction, PointerButton},
        ui::Pressed,
        ui_widgets::Activate,
    };

    // The app now boots into the main menu; drive it the way a user would by
    // clicking Sandbox (which is also the smoke coverage for the menu itself).
    // Repeat clicks while the state transition is pending are idempotent.
    match *world.resource::<State<GameStates>>().get() {
        GameStates::MainMenu => {
            if let Some(button) = button_by_name(world, "Sandbox Button") {
                world.trigger(Activate { entity: button });
                info!("editor autopilot: clicked Sandbox in the main menu");
            }
            return;
        }
        // The editor lives inside GameStates::Playing (it switches its own inner state to
        // Editor there); do nothing until the loader has reached the menu or gameplay.
        GameStates::Loading => return,
        GameStates::Playing => {}
    }

    if !world.contains_resource::<EditorAutopilot>() {
        world.insert_resource(EditorAutopilot::default());
    }
    let mut state = world.remove_resource::<EditorAutopilot>().unwrap();

    if state.wait > 0 {
        state.wait -= 1;
        world.insert_resource(state);
        return;
    }

    match state.phase {
        Phase::CreateShip => {
            // The editor builds its buttons a couple of frames after entering; wait for it.
            if let Some(button) = button_by_name(world, "Create New Spaceship Button V2") {
                world.trigger(Activate { entity: button });
                info!("editor autopilot: created a ship with a controller");
                state.phase = Phase::SelectSection;
                // Let the ship spawn and avian prepare its section colliders before we click.
                state.wait = 30;
            }
        }
        Phase::SelectSection => {
            if let Some(name) = hull_section_name(world) {
                if let Some(button) = button_by_name(world, &name) {
                    // Section buttons set `SectionChoice` on `Add<Pressed>` (see the editor's
                    // `button_on_setting`), so inserting `Pressed` selects the section.
                    world.entity_mut(button).insert(Pressed);
                    info!("editor autopilot: selected the '{name}' section");
                    state.phase = Phase::Aim;
                    state.wait = 2;
                }
            }
        }
        Phase::Aim => {
            if let Some((location, count)) = aim_at_a_section(world) {
                send_pointer(world, &location, PointerAction::Move { delta: Vec2::ZERO });
                state.location = Some(location);
                state.sections_before = count;
                state.phase = Phase::Press;
                // Let the picking backend raycast the new pointer position and hover the section.
                state.wait = 2;
            }
            // else: no section/camera ready yet - retry next frame.
        }
        Phase::Press => {
            if let Some(location) = state.location.clone() {
                // Keep hovering, then press the primary button.
                send_pointer(world, &location, PointerAction::Move { delta: Vec2::ZERO });
                send_pointer(
                    world,
                    &location,
                    PointerAction::Press(PointerButton::Primary),
                );
            }
            state.phase = Phase::Release;
            state.wait = 1;
        }
        Phase::Release => {
            if let Some(location) = state.location.clone() {
                send_pointer(
                    world,
                    &location,
                    PointerAction::Release(PointerButton::Primary),
                );
            }
            state.phase = Phase::Verify;
            state.wait = 3;
        }
        Phase::Verify => {
            let count = count_sections(world);
            if count > state.sections_before {
                info!(
                    "editor autopilot: placed a section ({} -> {} sections)",
                    state.sections_before, count
                );
            } else {
                warn!(
                    "editor autopilot: placement click did not add a section ({} -> {})",
                    state.sections_before, count
                );
            }
            state.phase = Phase::Done;
        }
        Phase::Done => {}
    }

    world.insert_resource(state);
}

/// Find a UI entity by its `Name`.
#[cfg(feature = "debug")]
fn button_by_name(world: &mut World, name: &str) -> Option<Entity> {
    let mut q = world.query::<(Entity, &Name)>();
    q.iter(world)
        .find(|(_, n)| n.as_str() == name)
        .map(|(entity, _)| entity)
}

/// The display name of any hull section in the catalog (the section the autopilot places).
#[cfg(feature = "debug")]
fn hull_section_name(world: &World) -> Option<String> {
    world
        .resource::<GameSections>()
        .iter()
        .find(|section| matches!(section.kind, SectionKind::Hull(_)))
        .map(|section| section.base.name.clone())
}

/// Count the preview ship's sections.
#[cfg(feature = "debug")]
fn count_sections(world: &mut World) -> usize {
    let mut q = world.query_filtered::<(), With<SectionMarker>>();
    q.iter(world).count()
}

/// Project a preview section onto the screen, returning the pointer [`Location`] to click and
/// the current section count. `None` until a section, the 3D camera, and the window all exist.
#[cfg(feature = "debug")]
fn aim_at_a_section(world: &mut World) -> Option<(bevy::picking::pointer::Location, usize)> {
    // The world-space position of a preview section (and how many there are).
    let mut q_sections = world.query_filtered::<&GlobalTransform, With<SectionMarker>>();
    let section_pos = q_sections.iter(world).next()?.translation();
    let count = q_sections.iter(world).count();

    let camera_entity = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()?;
    let window_entity = world
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .iter(world)
        .next()?;

    // `&Camera` + `&GlobalTransform` + `&RenderTarget` coexist as shared borrows of the world
    // (in bevy 0.19 the render target is a separate component the `Camera` requires).
    let camera = world.get::<Camera>(camera_entity)?;
    let camera_transform = world.get::<GlobalTransform>(camera_entity)?;
    let render_target = world.get::<bevy::camera::RenderTarget>(camera_entity)?;
    let position = camera
        .world_to_viewport(camera_transform, section_pos)
        .ok()?;
    let target = render_target.normalize(Some(window_entity))?;

    Some((bevy::picking::pointer::Location { target, position }, count))
}

/// Write a synthetic mouse `PointerInput` at `location`, as if the user moved/clicked there.
#[cfg(feature = "debug")]
fn send_pointer(
    world: &mut World,
    location: &bevy::picking::pointer::Location,
    action: bevy::picking::pointer::PointerAction,
) {
    use bevy::picking::pointer::{PointerId, PointerInput};
    world.write_message(PointerInput::new(
        PointerId::Mouse,
        location.clone(),
        action,
    ));
}
