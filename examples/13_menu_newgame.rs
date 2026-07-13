//! 13_menu_newgame: the shipped boot flow, wired to the smoke-test harness.
//!
//! Boots the exact app the `nova_protocol` binary runs (via the shared
//! [`editor_app`]: main menu over the ambience backdrop) and autopilot-drives
//! the menu the way a player would. Two paths, picked by `NOVA_MENU_PATH`:
//!
//! - `newgame` (default): click New Game - the menu teardown +
//!   `shakedown_run` load, the shipped default boot flow, which no other
//!   harnessed example covers (09_editor clicks Sandbox).
//! - `editorplay`: click Sandbox, create a ship, click Play - the editor's
//!   preview-teardown -> play-test transition.
//!
//! Under `BCS_AUTOPILOT` the ECS fallback error handler is swapped to panic,
//! so any command error - e.g. a system queuing a command on an entity the
//! menu/scenario teardown already despawned - fails the smoke run instead of
//! scrolling by as a WARN. This pins the investigation of task
//! 20260713-175352 (an "Entity despawned" command error on this transition in
//! the v0.5.0 web build, not reproduced natively in 6 newgame + 3 editorplay
//! runs): if the race exists natively and ever fires, CI catches it.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 13_menu_newgame --features debug
//! # look for: `probe: clicked New Game Button in the main menu`,
//! #           `nova harness: reached Playing`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "13_menu_newgame")]
#[command(version = "1.0.0")]
#[command(about = "The shipped menu boot flow, wired to the smoke-test harness", long_about = None)]
struct Cli;

fn main() {
    let _ = Cli::parse();

    // The same app the game/binary runs - not a bespoke copy.
    let mut app = editor_app(true);

    // Headless smoke-test harness: inert in a normal run (gated on BCS_AUTOPILOT).
    #[cfg(feature = "debug")]
    {
        if std::env::var_os("BCS_AUTOPILOT").is_some() {
            // Turn command errors (despawned-entity targets and friends) into
            // panics so the autopilot run fails loudly on them.
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.init_resource::<MenuAutopilot>();
        app.add_plugins(nova_autopilot().input(menu_autopilot));
        app.add_plugins(nova_screenshot());
    }

    app.run();
}

/// Frame-paced autopilot state (the editor needs a few frames between actions).
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct MenuAutopilot {
    clicked_menu: bool,
    created_ship: bool,
    clicked_play: bool,
    wait: u32,
}

#[cfg(feature = "debug")]
fn button_by_name(world: &mut World, name: &str) -> Option<Entity> {
    let mut names = world.query::<(Entity, &Name)>();
    names
        .iter(world)
        .find(|(_, n)| n.as_str() == name)
        .map(|(entity, _)| entity)
}

/// Drive the menu like a player: click the chosen menu button, and on the
/// editorplay path continue into ship creation + Play. Runs every autopilot
/// frame; each step fires once and waits where the editor needs to settle.
#[cfg(feature = "debug")]
fn menu_autopilot(world: &mut World, _elapsed: f32) {
    use bevy::ui_widgets::Activate;

    let editor_play = std::env::var("NOVA_MENU_PATH").is_ok_and(|p| p == "editorplay");

    let mut state = world.remove_resource::<MenuAutopilot>().unwrap();
    if state.wait > 0 {
        state.wait -= 1;
        world.insert_resource(state);
        return;
    }

    match *world.resource::<State<GameStates>>().get() {
        GameStates::Loading => {}
        GameStates::MainMenu => {
            if !state.clicked_menu {
                let name = if editor_play {
                    "Sandbox Button"
                } else {
                    "New Game Button"
                };
                if let Some(button) = button_by_name(world, name) {
                    world.trigger(Activate { entity: button });
                    state.clicked_menu = true;
                    info!("probe: clicked {name} in the main menu");
                }
            }
        }
        GameStates::Playing if editor_play => {
            if !state.created_ship {
                if let Some(button) = button_by_name(world, "Create New Spaceship Button V2") {
                    world.trigger(Activate { entity: button });
                    state.created_ship = true;
                    // Let the preview spawn and avian settle before Play.
                    state.wait = 30;
                    info!("probe: created a ship in the editor");
                }
            } else if !state.clicked_play {
                if let Some(button) = button_by_name(world, "Play Button") {
                    world.trigger(Activate { entity: button });
                    state.clicked_play = true;
                    info!("probe: clicked Play in the editor");
                }
            }
        }
        GameStates::Playing => {}
    }

    world.insert_resource(state);
}
