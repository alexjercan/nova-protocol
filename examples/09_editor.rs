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
//! Controls (interactive run): use the on-screen buttons to create ships and place sections.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 09_editor --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `editor autopilot: created a ship with a controller`,
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

/// Autopilot: once the editor is up, click the "create a ship with a controller" button exactly
/// once, so the run exercises the editor's controller-preview path headless.
#[cfg(feature = "debug")]
fn editor_autopilot(world: &mut World, _elapsed: f32) {
    use bevy::ui_widgets::Activate;

    // The editor lives inside GameStates::Playing (it switches its own inner state to Editor
    // there); do nothing until the loader has reached it.
    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }

    // Fire the action only once.
    if world
        .get_resource::<EditorAutopilotDone>()
        .is_some_and(|done| done.0)
    {
        return;
    }

    // The editor builds its buttons a frame or two after entering the editor; find the
    // "create with controller" button by name and activate it when it appears.
    let mut q = world.query::<(Entity, &Name)>();
    let button = q
        .iter(world)
        .find(|(_, name)| name.as_str() == "Create New Spaceship Button V2")
        .map(|(entity, _)| entity);

    if let Some(button) = button {
        world.trigger(Activate { entity: button });
        world.insert_resource(EditorAutopilotDone(true));
        info!("editor autopilot: created a ship with a controller");
    }
}

/// Marks that the autopilot has already fired its one editor action.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct EditorAutopilotDone(bool);
