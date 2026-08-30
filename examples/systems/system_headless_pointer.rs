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
//! cargo run --example system_headless_pointer --features debug
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

    app.init_resource::<Spike>();
    // The autopilot's own slot: after this frame's real input collection, so a
    // synthesized press is fresh (`just_pressed`) for everything in `Update`.
    app.add_systems(PreUpdate, drive.after(bevy::input::InputSystems));

    app.run()
}

/// The beat the run is on. One beat per frame at most; a beat that waits on
/// the world holds its step until the world answers or the deadline names it.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct Spike {
    step: usize,
    /// Frames the current step has waited (settle pacing between gestures).
    wait: u32,
    started: std::time::Instant,
}

#[cfg(feature = "debug")]
impl Default for Spike {
    fn default() -> Self {
        Self {
            step: 0,
            wait: 0,
            started: std::time::Instant::now(),
        }
    }
}

/// The whole run must fit here; a stall means the virtual window did NOT bring
/// the stack up, and the panic names the step that proves it.
#[cfg(feature = "debug")]
const DEADLINE_SECS: u64 = 180;

#[cfg(feature = "debug")]
fn drive(world: &mut World) {
    let spike = world.resource::<Spike>();
    let (step, wait) = (spike.step, spike.wait);
    if spike.started.elapsed().as_secs() > DEADLINE_SECS {
        panic!("headless pointer: STALLED at step {step} after {DEADLINE_SECS}s");
    }

    let advance = |world: &mut World| {
        let mut spike = world.resource_mut::<Spike>();
        spike.step += 1;
        spike.wait = 0;
    };
    let hold = |world: &mut World| world.resource_mut::<Spike>().wait += 1;

    match step {
        // The scenario comes up through the menu's own New Game door.
        0 => {
            if *world.resource::<State<GameStates>>().get() == GameStates::Playing {
                info!("headless pointer: reached Playing with no renderer");
                advance(world);
            }
        }
        // Let a full layout + picking pass run against the virtual window
        // before the first gesture reads anything back.
        1 => {
            if wait < 5 {
                hold(world);
            } else {
                press_key(KeyCode::Escape)(world);
                advance(world);
            }
        }
        2 => {
            release_key(KeyCode::Escape)(world);
            advance(world);
        }
        3 => {
            if *world.resource::<State<PauseStates>>().get() == PauseStates::Paused {
                info!("headless pointer: ESC opened the pause overlay");
                advance(world);
            } else {
                hold(world);
            }
        }
        // The layout proof: the overlay's widgets must resolve to real,
        // non-zero rects. This census is exactly what the channel's `ui`
        // block will report, so print it in that spirit.
        4 => {
            let Some(resume) = ui_node_rect(world, "Resume Button") else {
                hold(world);
                return;
            };
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
            assert!(
                resume.width() > 0.0 && resume.height() > 0.0,
                "the Resume button laid out without a box: {resume:?}"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the pause overlay lays out with no renderer",
                serde_json::json!({
                    "resume_w": resume.width(),
                    "resume_h": resume.height(),
                }),
            );
            hover_named("Resume Button")(world);
            advance(world);
        }
        // The click spans frames the way a player's does: picking reads the
        // move a frame after it is written, and `Activate` fires on
        // release-over. Two settle frames after the hover, then press, then
        // release on its own frame.
        5 => {
            if wait < 2 {
                hold(world);
            } else {
                press_mouse(MouseButton::Left)(world);
                advance(world);
            }
        }
        6 => {
            release_mouse(MouseButton::Left)(world);
            advance(world);
        }
        7 => {
            if *world.resource::<State<PauseStates>>().get() == PauseStates::Unpaused {
                info!("headless pointer: PASS clicked Resume with no renderer");
                nova_probe::probe_marker(
                    world,
                    "outcome: a wire click resumes the game",
                    serde_json::json!({}),
                );
                world.write_message(AppExit::Success);
                advance(world);
            } else {
                hold(world);
            }
        }
        _ => {}
    }
}
