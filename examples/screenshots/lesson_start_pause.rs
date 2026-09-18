//! lesson_start_pause: the handbook's "Pausing and retrying" still - the pause
//! overlay standing over a live flight, with every row the lesson names on it.
//!
//! Its own producer, and it has to be: the pause overlay lives in
//! `nova_menu`, and the hollow producers are built with
//! `AppBuilder::with_game_plugins`, which drops the menu
//! (`nova_core::AppBuilder::with_settings_store` says so in as many words). A
//! walk that pressed ESC in one of those sets would press it at nothing. So
//! this one boots the app the GAME BINARY runs, straight into a scenario - the
//! `--scenario <id>` path - and pauses that.
//!
//! The scenario is the TUTORIAL, for what the overlay draws over it rather
//! than for the flight: `Retry` is the one conditional row
//! (`reconcile_pause_overlay` draws it only over a live scenario) and it is
//! also the row the lesson spends a sentence on, so the still has to be taken
//! over a scenario that is really loaded. It is the first flight a reader of a
//! START HERE lesson will pause, too.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - reach the flight, pause it,
//!   exit clean, writing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the still (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_start_pause --features debug
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_start_pause")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's pause-overlay demonstration", long_about = None)]
struct Cli;

/// The still this shoots for: "Pausing and retrying".
#[cfg(feature = "debug")]
const PAUSE_SHOT: &str = "start_pause.png";

/// The flight the overlay stands over.
#[cfg(feature = "debug")]
const SCENARIO: &str = "tutorial";

/// The rows the lesson names, in the order the panel stacks them.
///
/// Asserted before the shot rather than trusted: a row that stopped being
/// drawn would otherwise ship as a picture of a shorter menu under a sentence
/// that still listed five.
#[cfg(feature = "debug")]
const PAUSE_ROWS: [&str; 5] = [
    "Resume Button",
    "Pause Retry Button",
    "Pause Settings Button",
    "Back To Menu Button",
    "Pause Exit Button",
];

/// How long the walk lets the tutorial run before it pauses it.
///
/// Long enough that the opening beats have played and the world behind the
/// panel is a flight rather than a load screen that just cleared.
#[cfg(feature = "debug")]
const FLY_SECS: f32 = 4.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The binary's `--scenario <id>`: the run lands in flight, with the menu
    // crate's pause overlay behind ESC.
    #[cfg(feature = "debug")]
    let startup = Some(StartupScenario::Id(SCENARIO.to_string()));
    #[cfg(not(feature = "debug"))]
    let startup = None;

    let mut app = editor_app(true, startup);

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(start_pause_script());
    }

    app.run()
}

/// Reach the tutorial, fly it for a moment, then pause it and shoot.
#[cfg(feature = "debug")]
fn start_pause_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the tutorial")
        .until(state_is(GameStates::Playing))
        .deadline(60.0)
        .add()
        .step("let the flight settle")
        .until(elapsed(FLY_SECS))
        .add()
        // ESC through the real `ButtonInput<KeyCode>` edge, which is what
        // `nova_menu::pause::toggle_pause` reads. Released on the next beat,
        // because the toggle answers the EDGE: a key left down is a key the
        // next press cannot produce an edge on.
        .step("pause the flight")
        .on_enter(press_key(KeyCode::Escape))
        .until(ui_node_present(PAUSE_ROWS[0]))
        .diagnose(ui_node_diagnosis(PAUSE_ROWS[0]))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let ESC go")
        .on_enter(release_key(KeyCode::Escape))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("the overlay carries every row the lesson names")
        .on_enter(|world: &mut World| {
            for row in PAUSE_ROWS {
                assert_named_visible(row)(world);
            }
        })
        .add()
        // The status bar goes, and nothing else does. It reads the frame rate
        // and `v<version>+<commit>`, and art the GAME SHIPS cannot carry a
        // build id - every re-capture would bake a different one into the
        // handbook. The rest of the HUD STAYS: the lesson's claim is that
        // pause holds a live flight, and a flight with its instruments taken
        // away is not the thing being held.
        .step("capture the pause overlay")
        .on_enter(|world: &mut World| {
            hide_status_bar(world);
            shoot(world, PAUSE_SHOT);
        })
        .until(shot_written(PAUSE_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
