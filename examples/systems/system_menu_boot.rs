//! system_menu_boot: the shipped boot flow, driven by a real pointer click.
//!
//! Boots the exact app the `nova_protocol` binary runs (via the shared
//! [`editor_app`]: main menu over the ambience backdrop) and clicks New Game
//! the way a player would - by name, at the button's own screen position, not
//! by triggering its observer.
//!
//! ONE SUBJECT: the boot flow. The run asserts that clicking New Game tears the
//! menu down and reaches gameplay state, and NOTHING about what
//! `first_shift` then contains - that is story, and story is covered by the
//! `story/` examples. An assertion here that grew into scenario content would
//! mean this run had drifted (task 20260804-094021).
//!
//! No `NOVA_MENU_PATH=editorplay` branch: `examples/ui/editor.rs` owns the
//! create-a-ship-and-Play sequence end to end, and two runs covering one
//! transition is duplication.
//!
//! Under `NOVA_AUTOPILOT` the ECS fallback error handler is swapped to panic,
//! so an UNHANDLED command error (e.g. a plain `insert` on an entity the
//! menu/scenario teardown already despawned) aborts the smoke run. (Bevy
//! 0.19's default severity already panics these; the explicit swap pins the
//! contract against upstream default changes.) Coverage caveat (task 20260713-203709):
//! `remove`/`despawn` bake in the WARN handler at queue time, so their errors
//! never reach this handler - the smoke suite's stderr grep for "Encountered an
//! error in command" is what gates that flavor. Together they pin the
//! investigation of task 20260713-175352 (an "Entity despawned" command error
//! on this transition in the v0.5.0 web build, not reproduced natively): if the
//! race exists natively and ever fires, CI catches it.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_menu_boot --features debug
//! # look for: `menu_boot: clicked New Game`,
//! #           `nova harness: reached Playing`,
//! #           `menu_boot: the menu tore down and gameplay state is up`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_menu_boot")]
#[command(version = "1.0.0")]
#[command(about = "The shipped menu boot flow, driven by a real pointer click. Autopilot-only correctness range - run the game to use the menu", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same app the game/binary runs - not a bespoke copy.
    let mut app = editor_app(true, None);

    // Headless smoke-test harness: inert in a normal run (gated on NOVA_AUTOPILOT).
    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            // Turn command errors (despawned-entity targets and friends) into
            // panics so the autopilot run fails loudly on them.
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // Probe wiring (task 20260719-210443; each plugin is inert without
        // its NOVA_PROBE_* env): run timeline + engine-bound invariants +
        // frame-time capture, so `probe run` can measure this example.
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(menu_script());
    }

    app.run()
}

/// The button the walk clicks, and the node whose absence proves the teardown.
#[cfg(feature = "debug")]
const NEW_GAME_BUTTON: &str = "New Game Button";

/// How many nodes carry [`NEW_GAME_BUTTON`], visible or not.
///
/// Counted by `Name` rather than by layout: a menu that merely HID itself is
/// not a menu that was torn down, and this range's claim is the second one.
#[cfg(feature = "debug")]
fn menu_buttons(world: &World) -> usize {
    world.try_query::<&Name>().map_or(0, |mut names| {
        names
            .iter(world)
            .filter(|name| name.as_str() == NEW_GAME_BUTTON)
            .count()
    })
}

/// Advance once no [`NEW_GAME_BUTTON`] is left in the world.
#[cfg(feature = "debug")]
fn the_menu_tore_down() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| menu_buttons(world) == 0)
}

/// Seconds the run gives the New Game click to reach gameplay state. Sized to
/// outlast `first_shift`'s load on a software-rendered CI GPU, and kept UNDER
/// the harness completion deadline (`NOVA_AUTOPILOT_DEADLINE`, default 120 s)
/// so a stall is an error naming THIS beat rather than a generic deadline.
#[cfg(feature = "debug")]
const BOOT_SECS: f32 = 90.0;

/// The boot flow, one beat per gesture.
///
/// The picture is taken on the LAID-OUT MENU, which is what this range is
/// about. `nova_screenshot` appends its beat to whatever it is handed, and the
/// beats after this call leave the app inside the scenario load - a shot behind
/// them photographs the loading screen.
#[cfg(feature = "debug")]
fn menu_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_screenshot(
        nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            .step("menu_boot: reach the main menu")
            .until(state_is(GameStates::MainMenu))
            .deadline(BOOT_SECS)
            .add()
            .step("menu_boot: let the menu lay out")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(BEAT_DEADLINE_SECS)
            .add(),
    )
    // F5 in the menu is a RESTART: the content is read off disk again and the
    // game comes back up on it. The menu going away and coming back is the
    // whole of what a player sees, and it is what tells a live reload apart
    // from this - a live reload would have left the menu standing.
    .step("menu_boot: press F5 in the menu")
    .on_enter(press_key(RELOAD_KEY))
    .until(the_menu_tore_down())
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    .step("menu_boot: let F5 go")
    .on_enter(release_key(RELOAD_KEY))
    .add()
    .step("menu_boot: the game comes back up on the menu")
    .until(ui_node_present(NEW_GAME_BUTTON))
    .deadline(BOOT_SECS)
    .add()
    .step("menu_boot: the restart landed back in the menu")
    .on_enter(|world: &mut World| {
        assert_eq!(
            *world.resource::<State<GameStates>>().get(),
            GameStates::MainMenu,
            "a content restart must hand the player back to the menu it took"
        );
        nova_probe::probe_marker(
            world,
            "outcome: F5 restarts the game onto the content on disk",
            serde_json::json!({}),
        );
        info!("menu_boot: F5 took the menu down and the restart put it back");
    })
    .add()
    .step("menu_boot: click New Game")
    .on_enter(click_named(NEW_GAME_BUTTON))
    .until(pointer_pressed())
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    // The menu buttons act on `Activate`, which fires on RELEASE over the
    // same widget - so a click is two beats.
    .step("menu_boot: release New Game")
    .on_enter(|world: &mut World| {
        release_mouse(MouseButton::Left)(world);
        info!("menu_boot: clicked New Game");
    })
    .until(state_is(GameStates::Playing))
    .deadline(BOOT_SECS)
    .add()
    .step("menu_boot: let the teardown finish")
    .until(the_menu_tore_down())
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    .step("menu_boot: the boot flow completed")
    .on_enter(|world: &mut World| {
        // The whole claim: gameplay state is up and the menu is GONE. What
        // the scenario then contains is deliberately not asserted.
        assert_eq!(
            *world.resource::<State<GameStates>>().get(),
            GameStates::Playing,
            "New Game must reach gameplay state"
        );
        nova_probe::probe_marker(
            world,
            "outcome: new game reaches gameplay",
            serde_json::json!({}),
        );
        let menu_buttons = menu_buttons(world);
        assert_eq!(
            menu_buttons, 0,
            "the main menu must be torn down once gameplay is up; \
                 {menu_buttons} New Game button(s) survived"
        );
        nova_probe::probe_marker(world, "outcome: the menu tore down", serde_json::json!({}));
        info!("menu_boot: the menu tore down and gameplay state is up");
    })
    .add()
}
