//! menu_newgame: the shipped boot flow, driven by a real pointer click.
//!
//! Boots the exact app the `nova_protocol` binary runs (via the shared
//! [`editor_app`]: main menu over the ambience backdrop) and clicks New Game
//! the way a player would - by name, at the button's own screen position, not
//! by triggering its observer.
//!
//! ONE SUBJECT: the boot flow. The run asserts that clicking New Game tears the
//! menu down and reaches gameplay state, and NOTHING about what
//! `shakedown_run` then contains - that is story, and story is covered by the
//! `story/` examples. An assertion here that grew into scenario content would
//! mean this run had drifted (task 20260804-094021).
//!
//! The `NOVA_MENU_PATH=editorplay` branch this example used to carry is GONE:
//! `examples/ui/editor.rs` now owns the create-a-ship-and-Play sequence end to
//! end, and two runs covering one transition is the duplication the roster
//! spike (`20260804-003244`) existed to cut.
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
//! NOVA_AUTOPILOT=1 cargo run --example menu_newgame --features debug
//! # look for: `menu_newgame: clicked New Game`,
//! #           `nova harness: reached Playing`,
//! #           `menu_newgame: the menu tore down and gameplay state is up`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "menu_newgame")]
#[command(version = "1.0.0")]
#[command(about = "The shipped menu boot flow, driven by a real pointer click", long_about = None)]
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
        // its NOVA_PERF_* env): run timeline + engine-bound invariants +
        // frame-time capture, so `probe run` can measure this example.
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(menu_script());
        app.add_plugins(nova_screenshot());
    }

    app.run()
}

/// Frames a beat waits for a gesture to land: the picking backend needs a frame
/// to raycast the new pointer position and the menu's observers a frame to
/// react. Generous rather than tight - this runs on a software-rendered CI GPU.
#[cfg(feature = "debug")]
const SETTLE: u32 = 10;

/// Seconds the run gives the New Game click to reach gameplay state. Sized to
/// outlast `shakedown_run`'s load on a software-rendered CI GPU, and kept UNDER
/// the harness completion deadline (`NOVA_AUTOPILOT_DEADLINE`, default 120 s)
/// so a stall is an error naming THIS beat rather than a generic deadline.
#[cfg(feature = "debug")]
const BOOT_SECS: f32 = 90.0;

/// The boot flow, one beat per gesture.
#[cfg(feature = "debug")]
fn menu_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("menu_newgame: reach the main menu")
        .until(state_is(GameStates::MainMenu))
        .add()
        .step("menu_newgame: let the menu lay out")
        .until(frames(SETTLE))
        .add()
        .step("menu_newgame: click New Game")
        .on_enter(click_named("New Game Button"))
        .until(frames(SETTLE))
        .add()
        // The menu buttons act on `Activate`, which fires on RELEASE over the
        // same widget - so a click is two beats.
        .step("menu_newgame: release New Game")
        .on_enter(|world: &mut World| {
            release_mouse(MouseButton::Left)(world);
            info!("menu_newgame: clicked New Game");
        })
        .until(state_is(GameStates::Playing))
        .deadline(BOOT_SECS)
        .add()
        .step("menu_newgame: let the teardown finish")
        .until(frames(SETTLE))
        .add()
        .step("menu_newgame: the boot flow completed")
        .on_enter(|world: &mut World| {
            // The whole claim: gameplay state is up and the menu is GONE. What
            // the scenario then contains is deliberately not asserted.
            assert_eq!(
                *world.resource::<State<GameStates>>().get(),
                GameStates::Playing,
                "New Game must reach gameplay state"
            );
            let menu_buttons = world
                .query::<&Name>()
                .iter(world)
                .filter(|name| name.as_str() == "New Game Button")
                .count();
            assert_eq!(
                menu_buttons, 0,
                "the main menu must be torn down once gameplay is up; \
                 {menu_buttons} New Game button(s) survived"
            );
            info!("menu_newgame: the menu tore down and gameplay state is up");
        })
        .until(frames(1))
        .add()
}
