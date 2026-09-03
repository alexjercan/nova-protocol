//! system_headless_novaos: spike 2 for `nova_channel` - NOVA OS off-screen.
//!
//! The design record (task 20260820-174148, nova-channel.html) calls the
//! render gate on `NovaHudPlugin` / `NovaOsUiPlugin` the one real parity gate:
//! a headless run registered 15 of the 33 named actions and had no monitor to
//! type into. This range boots the app with that gate REMOVED (see the spike
//! note in `nova_core/src/lib.rs`) plus the virtual window, and then does the
//! thing the gate made impossible:
//!
//!   - assert the FULL registry is present headless (`novaos_toggle` resolves,
//!     and the table holds every group);
//!   - press Tab - the real key, through the real rig - and reach
//!     `PauseStates::NovaOs`;
//!   - type `map` at the prompt over real `KeyboardInput` messages, watch the
//!     `NovaOsTerminal` RESOURCE (the model the CRT merely projects) take the
//!     characters, and Enter into the map app.
//!
//! The point the spike proves is the design's central claim: the terminal's
//! backend is a plain resource and the screen is only its projection, so with
//! the plugins present the whole monitor works with no GPU behind it - the
//! CRT material and the render-to-texture camera are bevy-guarded no-ops.
//!
//! Run (no display needed):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_headless_novaos --features debug
//! # look for: `headless novaos: registry holds N actions`, then
//! # `headless novaos: PASS typed into NOVA OS with no renderer`.
//! ```

#[cfg(feature = "debug")]
use bevy::{prelude::*, window::PrimaryWindow};
#[cfg(feature = "debug")]
use nova_input::prelude::InputBindings;
#[cfg(feature = "debug")]
use nova_protocol::nova_os_ui::nova_os::prelude::{NovaOsTerminal, TerminalMode};
#[cfg(feature = "debug")]
use nova_protocol::prelude::*;

#[cfg(not(feature = "debug"))]
fn main() {
    eprintln!("system_headless_novaos drives the app through the debug-only autopilot gestures;");
    eprintln!("run it with --features debug");
}

#[cfg(feature = "debug")]
fn main() -> bevy::app::AppExit {
    let mut app = editor_app(false, Some(StartupScenario::Id("first_shift".to_string())));

    // The virtual window, exactly as in `system_headless_pointer`.
    app.world_mut().spawn((
        Window {
            resolution: (1280, 720).into(),
            ..default()
        },
        PrimaryWindow,
    ));

    app.add_plugins(
        nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            .step("headless novaos: reach Playing with no renderer")
            .until(state_is(GameStates::Playing))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            // The registry proof: with the gate removed, the verbs the channel
            // must advertise all exist off-screen too.
            .step("headless novaos: census the action table")
            .on_enter(census_the_registry)
            .add()
            // Tab, held across the enhanced-input read so the edge cannot fall
            // between two collectors, then released once the monitor is up.
            .step("headless novaos: Tab opens the monitor")
            .on_enter(press_key(KeyCode::Tab))
            .until(resource_where::<State<PauseStates>>(|pause| {
                *pause.get() == PauseStates::NovaOs
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless novaos: release Tab")
            .on_enter(release_key(KeyCode::Tab))
            .add()
            // The boot banner reveals on real time; the prompt is only worth
            // typing at once the reveal has drained.
            .step("headless novaos: the boot banner drained")
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal.is_booted() && !terminal.has_pending_boot_rows()
            }))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            // The model proof: the characters land in the RESOURCE, which is
            // the terminal's backend - the CRT texture nobody drew is just its
            // projection.
            .step("headless novaos: the prompt took the typing")
            .on_enter(type_text("map"))
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal.prompt() == "map"
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless novaos: Enter launches the map app")
            .on_enter(press_edit_key(Key::Enter))
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal.active_mode() == (TerminalMode::App { id: "map" })
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless novaos: record the pass")
            .on_enter(|world: &mut World| {
                info!("headless novaos: PASS typed into NOVA OS with no renderer");
                nova_probe::probe_marker(
                    world,
                    "outcome: the terminal takes typing with no renderer",
                    serde_json::json!({}),
                );
            })
            .add(),
    );

    app.run()
}

/// Count the registered actions and state that the table came up whole.
#[cfg(feature = "debug")]
fn census_the_registry(world: &mut World) {
    let bindings = world.resource::<InputBindings>();
    let count = bindings.iter().count();
    info!("headless novaos: registry holds {count} actions");
    assert!(
        bindings.get("novaos_toggle").is_some(),
        "the NOVA OS toggle must register headless - it is the verb the render \
         gate used to drop"
    );
    assert!(
        count > 30,
        "the full table is ~33 actions; {count} means a group is still gated out"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the NOVA OS verb registers headless",
        serde_json::json!({ "actions": count }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the whole action table registers headless",
        serde_json::json!({ "actions": count }),
    );
}
