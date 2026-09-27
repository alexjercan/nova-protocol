//! screenshot_command_shell: the `:` NOVA COMMANDS modal with a command answered
//! and an inline-completion ghost on it (`wiki-command-shell.png`).
//!
//! It boots the one-ship range from `shared/computer.rs`, opens the shell over
//! flight with `:` and types through the real keyboard path, so a modal that
//! stopped reading the keyboard fails the run instead of shooting an empty
//! prompt.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - open the shell, run the
//!   command script, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNG (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_command_shell --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_command_shell --features debug
//! # look for: `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

// The range and the keyboard path, shared with the interface walks.
#[path = "shared/computer.rs"]
mod computer;
use computer::interface_range;
#[cfg(feature = "debug")]
use computer::{run_command, type_char, type_word};

#[derive(Parser)]
#[command(name = "screenshot_command_shell")]
#[command(version = "1.0.0")]
#[command(about = "Capture the `:` NOVA COMMANDS modal with output and a completion ghost. Autopilot-only: a scripted command walk", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    // NovaMenuPlugin explicitly: the `:` gesture lives in nova_menu, and
    // `with_game_plugins` turns the menu plugin off. It stands alone in a slim
    // app; the boot-into-MainMenu handoff it normally owns is not taken (this
    // run goes Loading -> Playing).
    let mut app = AppBuilder::new()
        .with_game_plugins((custom_plugin, NovaMenuPlugin))
        .build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                // Wait for the ship to EXIST: the shell is opened over live
                // flight, so a beat fired before the scenario spawned would
                // open it over nothing.
                .step("load the interface range")
                .enter(GameStates::Loading)
                .until(player_ship_present())
                .deadline(30.0)
                .add()
                // `:` is read as a TYPED CHARACTER, not a key code, so the walk
                // sends it the way a player's layout does.
                .step("open the command shell over flight")
                .on_enter(|world| type_char(world, ":"))
                .until(nova_os_raster_open())
                .deadline(STEP_DEADLINE_SECS)
                .add()
                // Run `help` then `ships` so command-output formatting is on
                // screen, one command that lists verbs and one that reads the
                // world.
                .step("run the help command")
                .on_enter(|world| run_command(world, "help"))
                .until(computer::the_shell_answered())
                .deadline(STEP_DEADLINE_SECS)
                .add()
                .step("run the ships command")
                .on_enter(|world| run_command(world, "ships"))
                .until(computer::the_shell_answered())
                .deadline(STEP_DEADLINE_SECS)
                .add()
                // Leave a valid prefix in the input to show the inline
                // completion ghost.
                .step("leave an inline-completion prefix")
                .on_enter(|world| type_word(world, "lo"))
                .until(nova_os_command_line_reads("lo"))
                .deadline(STEP_DEADLINE_SECS)
                .add()
                // The ghost completion is PAINTED off that prefix, so the shot
                // still owes the renderer its stillness.
                .step("settle the completion ghost")
                .until(frames(SETTLE_FRAMES))
                .add()
                // The last step holds until the PNG is on disk, so the driver
                // cannot report done out from under a pending write.
                .step("capture the command shell")
                .on_enter(move |world| shoot(world, "wiki-command-shell.png"))
                .until(shot_written("wiki-command-shell.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add(),
        );
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(interface_range(&game_assets, &sections)));
}
