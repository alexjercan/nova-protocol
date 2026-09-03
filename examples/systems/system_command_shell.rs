//! system_command_shell: the `cmd>` shell driven end to end, off-screen.
//!
//! The NOVA OS range beside this one proves the terminal takes typing with no
//! renderer. This one proves the other half: that what is typed REACHES the
//! live world and comes back, and that the two ways into the computer land in
//! the shell they name.
//!
//! Both halves were live bugs. `:` opened the command shell and left it the
//! active shell for good, so Tab afterwards opened `cmd>` instead of NOVA OS;
//! and Escape out of a shell opened over flight unpaused a player who had been
//! paused. A shell that answers correctly but strands the player who opened it
//! is not working, so the range drives the whole gesture: open, run, complete,
//! close, and open the OTHER shell.
//!
//! Run (no display needed):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_command_shell --features debug
//! # look for: `command shell: PASS the shell answers and gives the surface back`.
//! ```

#[cfg(feature = "debug")]
use bevy::{input::keyboard::Key, prelude::*, window::PrimaryWindow};
#[cfg(feature = "debug")]
use nova_protocol::nova_os_ui::nova_os::prelude::{NovaOsTerminal, ShellKind};
#[cfg(feature = "debug")]
use nova_protocol::prelude::*;

#[cfg(not(feature = "debug"))]
fn main() {
    eprintln!("system_command_shell drives the app through the debug-only autopilot gestures;");
    eprintln!("run it with --features debug");
}

/// The id the shakedown scenario gives the player's hull. The range types it
/// half and lets completion finish it, so a rename breaks this range loudly
/// rather than leaving the completion untested.
#[cfg(feature = "debug")]
const PLAYER_ID: &str = "player_spaceship";

#[cfg(feature = "debug")]
fn main() -> bevy::app::AppExit {
    let mut app = editor_app(false, Some(StartupScenario::Id("first_shift".to_string())));

    // The virtual window, exactly as in `system_headless_novaos`: typing is
    // read off `KeyboardInput`, which carries the window it was typed into.
    app.world_mut().spawn((
        Window {
            resolution: (1280, 720).into(),
            ..default()
        },
        PrimaryWindow,
    ));

    app.add_plugins(
        nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            .step("command shell: reach Playing with no renderer")
            .until(state_is(GameStates::Playing))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            // `:` is read as the CHARACTER, so a layout where the colon is not
            // Shift+Semicolon opens the shell with the key that prints one.
            .step("command shell: `:` opens the computer")
            .on_enter(type_text(":"))
            .until(resource_where::<State<PauseStates>>(|pause| {
                *pause.get() == PauseStates::NovaOs
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: the shell it opened is `cmd>`")
            .on_enter(assert_command_shell_is_active)
            .add()
            .step("command shell: the intro drained")
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal.is_booted()
                    && !terminal.has_pending_boot_rows()
                    && terminal.is_revealed(ShellKind::Commands)
            }))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            // The round trip: the line is parsed here, run against the live
            // world by `nova_console`, and printed back into this transcript.
            .step("command shell: `ships` reaches the live world")
            .on_enter(type_text("ships"))
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal.prompt() == "ships"
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: the answer names the player's hull")
            .on_enter(press_edit_key(Key::Enter))
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal
                    .scrollback()
                    .iter()
                    .any(|row| row.text.contains(PLAYER_ID))
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: report the round trip")
            .on_enter(|world: &mut World| {
                nova_probe::probe_marker(
                    world,
                    "outcome: a typed command answers from the live world",
                    serde_json::json!({ "command": "ships" }),
                );
            })
            .add()
            // Completion asks the WORLD, not the catalog: `ship` takes a live
            // ship id, and the ids come from the ships that are actually there.
            .step("command shell: half an id at the prompt")
            .on_enter(type_text("ship play"))
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal.prompt() == "ship play"
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: Tab finishes it from the live world")
            .on_enter(press_edit_key(Key::Tab))
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal.prompt() == format!("ship {PLAYER_ID}")
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: report the completion")
            .on_enter(|world: &mut World| {
                nova_probe::probe_marker(
                    world,
                    "outcome: Tab completes an id only the live world knows",
                    serde_json::json!({ "completed": PLAYER_ID }),
                );
            })
            .add()
            // The shell is a surface OVER what was there, so closing it gives
            // that back. Opened from flight, Escape returns to flight.
            .step("command shell: Escape closes the computer")
            .on_enter(press_key(KeyCode::Escape))
            .until(resource_where::<State<PauseStates>>(|pause| {
                *pause.get() == PauseStates::Unpaused
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: release Escape")
            .on_enter(release_key(KeyCode::Escape))
            .on_enter(|world: &mut World| {
                nova_probe::probe_marker(
                    world,
                    "outcome: Escape gives back the surface the shell covered",
                    serde_json::json!({}),
                );
            })
            .add()
            // The second bug: `:` used to leave `cmd>` the active shell for the
            // rest of the run, so Tab reopened the shell the player had just
            // closed instead of the one it names.
            .step("command shell: Tab opens the computer again")
            .on_enter(press_key(KeyCode::Tab))
            .until(resource_where::<State<PauseStates>>(|pause| {
                *pause.get() == PauseStates::NovaOs
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: release Tab")
            .on_enter(release_key(KeyCode::Tab))
            .add()
            .step("command shell: and Tab opened NOVA OS")
            .on_enter(assert_nova_os_shell_is_active)
            .add(),
    );

    app.run()
}

/// `:` names the Command shell, and lands on its prompt.
#[cfg(feature = "debug")]
fn assert_command_shell_is_active(world: &mut World) {
    let shell = world.resource::<NovaOsTerminal>().active_shell();
    assert_eq!(
        shell,
        ShellKind::Commands,
        "`:` must open the command shell, not whichever shell was last active"
    );
    info!("command shell: `:` opened {}", shell.prompt_prefix());
    nova_probe::probe_marker(
        world,
        "outcome: `:` opens the command shell",
        serde_json::json!({ "shell": shell.prompt_prefix() }),
    );
}

/// Tab names NOVA OS, whatever shell the player was in last.
#[cfg(feature = "debug")]
fn assert_nova_os_shell_is_active(world: &mut World) {
    let shell = world.resource::<NovaOsTerminal>().active_shell();
    assert_eq!(
        shell,
        ShellKind::NovaOs,
        "Tab must open NOVA OS even after `:` has opened the command shell"
    );
    info!("command shell: PASS the shell answers and gives the surface back");
    nova_probe::probe_marker(
        world,
        "outcome: Tab opens NOVA OS after the command shell has been used",
        serde_json::json!({ "shell": shell.prompt_prefix() }),
    );
}
