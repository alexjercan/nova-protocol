//! system_headless_pointer: spike 1 for `nova_channel` - the virtual window.
//!
//! Boots the EXACT app `--norender --scenario shakedown_run` runs - backendless
//! wgpu, no winit, no display server - then spawns the one thing that run lacks:
//! an ordinary `Window` + `PrimaryWindow` ENTITY. Nothing renders it; it is a
//! size record. The claim under test (task 20260820-174148, nova-channel.html)
//! is that this record alone brings the whole GUI interaction stack back up
//! with no GPU behind it:
//!
//!   - cameras resolve `target_info` from `Window::physical_size`, so `bevy_ui`
//!     lays out against a real viewport instead of collapsing to 0 x 0;
//!   - the layout is therefore a truthful census: a widget's rect says where a
//!     player WOULD see it, so "is this button reachable" is answerable;
//!   - `bevy_picking`'s UI backend hit-tests real pointer events against those
//!     rects, so a click is a gesture through the full pipeline - move, press,
//!     release-over, `Activate` observer - never a bypassed observer trigger.
//!
//! The proof is a played beat, not an inspection: reach Playing, press ESC (the
//! real key, polled by `toggle_pause`), wait for the pause overlay to LAY OUT,
//! resolve `Resume Button` by `Name`, click its centre with real pointer
//! events, and require the game to resume. Every step that a missing layout or
//! a dead picking backend would break is on that path.
//!
//! Run (no display needed - that is the point):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_headless_pointer --features debug
//! # look for: `headless pointer: pause census` naming the overlay's widgets,
//! # then `headless pointer: PASS clicked Resume with no renderer`.
//! ```

#[cfg(feature = "debug")]
use bevy::{prelude::*, window::PrimaryWindow};
#[cfg(feature = "debug")]
use nova_protocol::prelude::*;

#[cfg(not(feature = "debug"))]
fn main() {
    eprintln!("system_headless_pointer drives the app through the debug-only autopilot gestures;");
    eprintln!("run it with --features debug");
}

#[cfg(feature = "debug")]
fn main() -> bevy::app::AppExit {
    let mut app = editor_app(
        false,
        Some(StartupScenario::Id("shakedown_run".to_string())),
    );

    // The virtual window: the size record every headless-blocked reader needs.
    // Spawned as a plain entity because that is all `bevy_render`'s camera
    // sizing, `bevy_ui`'s layout and `bevy_picking`'s backend ever read - the
    // OS window behind it belongs to winit, which this app does not run.
    app.world_mut().spawn((
        Window {
            resolution: (1280, 720).into(),
            ..default()
        },
        PrimaryWindow,
    ));

    app.add_plugins(
        nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            // The scenario comes up through the menu's own New Game door.
            .step("headless pointer: reach Playing with no renderer")
            .until(state_is(GameStates::Playing))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("headless pointer: ESC opens the pause overlay")
            .on_enter(press_key(KeyCode::Escape))
            .until(the_game_is(PauseStates::Paused))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless pointer: release ESC")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            // The layout proof. `ui_node_present` waits for a BOX, so this is
            // the claim itself - with no renderer the overlay would lay out to
            // nothing - and the census below states what it found.
            .step("headless pointer: the pause overlay laid out")
            .until(ui_node_present("Resume Button"))
            .diagnose(ui_node_diagnosis("Resume Button"))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless pointer: census the overlay")
            .on_enter(census_the_overlay)
            .add()
            // The click spans beats the way a player's does: picking reads the
            // move a frame after it is written, and `Activate` fires on
            // release-over. Its last beat waits on the GAME, so a click that
            // the overlay swallowed is a stall named here.
            .click_named(
                "headless pointer: the wire click resumes the game",
                "Resume Button",
                the_game_is(PauseStates::Unpaused),
                BEAT_DEADLINE_SECS,
            )
            .step("headless pointer: record the pass")
            .on_enter(|world: &mut World| {
                info!("headless pointer: PASS clicked Resume with no renderer");
                nova_probe::probe_marker(
                    world,
                    "outcome: a wire click resumes the game",
                    serde_json::json!({}),
                );
            })
            .add(),
    );

    app.run()
}

/// Advance once the pause machine holds `state`.
#[cfg(feature = "debug")]
fn the_game_is(
    state: PauseStates,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<State<PauseStates>>(move |pause| *pause.get() == state)
}

/// Print where the overlay's widgets landed, and state that Resume has a box.
///
/// Exactly what the channel's `ui` block will report, so it is printed in that
/// spirit rather than as a bare assertion.
#[cfg(feature = "debug")]
fn census_the_overlay(world: &mut World) {
    let resume = ui_node_rect(world, "Resume Button").expect("the beat before waited for its box");
    info!("headless pointer: pause census");
    for name in [
        "Pause Overlay",
        "Pause Panel",
        "Pause Title",
        "Resume Button",
        "Pause Settings Button",
    ] {
        match ui_node_rect(world, name) {
            Some(rect) => info!(
                "  {name}: centre ({:.0}, {:.0}) size {:.0} x {:.0}",
                rect.center().x,
                rect.center().y,
                rect.width(),
                rect.height()
            ),
            None => info!("  {name}: not laid out"),
        }
    }
    nova_probe::probe_marker(
        world,
        "outcome: the pause overlay lays out with no renderer",
        serde_json::json!({
            "resume_w": resume.width(),
            "resume_h": resume.height(),
        }),
    );
}
