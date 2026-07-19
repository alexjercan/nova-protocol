//! screenshot_ui: capture the UI/state-dependent web screenshots that a
//! scenario reel cannot make - the main menu and the sandbox editor - by driving
//! the shipped app (`editor_app`, menu + editor) through an autopilot script.
//!
//! The pure-3D scene shots live in `screenshot_reel`; this example covers the
//! shots that need a real game state and UI up: `tutorial-menu.png` (main menu
//! over the ambience backdrop) and `feature-editor.png` (the sandbox editor with
//! a ship built from sections). It steps the same way a player would - reach the
//! menu, click Sandbox, create a ship - settling before each capture.
//!
//! Two run modes, both under the autopilot (`BCS_AUTOPILOT`):
//! - `BCS_AUTOPILOT=1` alone: the smoke path - drive menu -> editor, reach
//!   Playing, exit clean, capturing nothing.
//! - `BCS_AUTOPILOT=1 BCS_REEL=1`: also capture each beat's PNG (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/reel BCS_AUTOPILOT=1 BCS_REEL=1 \
//!   cargo run --example screenshot_ui --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example screenshot_ui --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_ui")]
#[command(version = "1.0.0")]
#[command(about = "Capture the menu/editor web screenshots via the shipped app", long_about = None)]
struct Cli;

/// Seconds the autopilot holds its window - long enough to reach the menu, click
/// into the editor, build a ship, and settle + capture each beat. Sized with
/// headroom for a slow software-rendered CI GPU (llvmpipe), where every beat
/// costs more wall-clock than on a real GPU; the smoke path's short frame waits
/// (see `ui_capture_script`) keep the menu -> editor -> Playing walk well inside
/// this on such a box.
#[cfg(feature = "debug")]
const UI_AUTOPILOT_SECS: f32 = 20.0;

fn main() {
    let _ = Cli::parse();

    // The same app the game/binary runs (main menu over the ambience backdrop).
    let mut app = editor_app(true);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("BCS_AUTOPILOT").is_some() {
            // Turn command errors (despawned-entity targets on the menu/editor
            // teardown) into panics so the run fails loudly on them (as 12 does).
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.init_resource::<UiCapture>();
        // Clean frames at a known 16:9 size: force the window resolution and drop
        // the dev overlays. The HUD chrome is re-hidden right before each capture
        // (entering the editor re-raises it).
        app.add_systems(Startup, (force_resolution, hide_dev_overlays));
        app.add_plugins(
            AutopilotPlugin::<GameStates>::new()
                .hold(GameStates::Loading, UI_AUTOPILOT_SECS)
                .input(ui_capture_script),
        );
    }

    app.run();
}

/// Force the window to 1920x1080 (the 16:9 the web figures use) at startup.
#[cfg(feature = "debug")]
fn force_resolution(mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>) {
    if let Ok(mut window) = windows.single_mut() {
        window.resolution.set(1920.0, 1080.0);
        window.resizable = false;
    }
}

/// Set the HUD to its clean tier so the fps/version bar is out of shot. Called
/// right before each capture because entering the editor re-raises the HUD.
#[cfg(feature = "debug")]
fn hide_hud(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::None;
    }
}

/// Frame-paced beat tracker (the editor needs a few frames between actions, and
/// each capture needs the scene/UI to settle first).
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct UiCapture {
    stage: u32,
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

/// Drive the menu -> editor beats and capture each, the way a player would. Runs
/// every autopilot frame; each stage fires once and then waits where the scene or
/// editor needs to settle before the shot. Captures only when `BCS_REEL` is set,
/// so the plain autopilot smoke run drives the same path without writing files.
#[cfg(feature = "debug")]
fn ui_capture_script(world: &mut World, _elapsed: f32) {
    use bevy::ui_widgets::Activate;

    let capturing = std::env::var_os("BCS_REEL").is_some();

    // Frames to let a beat settle before its shot. The long settles matter ONLY
    // for the capture path (`BCS_REEL`): the scene/UI must be still and the PNG
    // must land before we navigate away. The smoke path (`BCS_AUTOPILOT` alone)
    // captures nothing, so it drives straight through on minimal waits - just
    // enough frames for the next button to spawn and the state transition to
    // apply. That keeps the menu -> editor -> Playing walk short in FRAMES, so it
    // fits the fixed-seconds autopilot window even on a slow software-rendered CI
    // GPU (llvmpipe), where the capture-sized 90/20/30-frame settles overran the
    // window and the run never left MainMenu (task 20260716).
    let settle_scene = if capturing { 90 } else { 6 };
    let after_capture = if capturing { 20 } else { 0 };
    let after_nav = if capturing { 30 } else { 6 };

    let mut state = world.remove_resource::<UiCapture>().unwrap();
    if state.wait > 0 {
        state.wait -= 1;
        world.insert_resource(state);
        return;
    }

    match *world.resource::<State<GameStates>>().get() {
        GameStates::Loading => {}
        GameStates::MainMenu => match state.stage {
            // Settle the menu + ambience backdrop before the shot.
            0 => {
                state.stage = 1;
                state.wait = settle_scene;
            }
            // Capture the menu. Hide the HUD first; wait for the PNG to land
            // BEFORE navigating away (clicking Sandbox in the same frame captured
            // a black mid-teardown frame).
            1 => {
                if capturing {
                    hide_hud(world);
                    capture_window(world, "tutorial-menu.png");
                    info!("ui capture: tutorial-menu.png");
                }
                state.stage = 2;
                state.wait = after_capture;
            }
            // Now leave for the editor.
            2 => {
                if let Some(button) = button_by_name(world, "Sandbox Button") {
                    world.trigger(Activate { entity: button });
                }
                state.stage = 3;
                state.wait = after_nav;
            }
            _ => {}
        },
        GameStates::Playing => match state.stage {
            // In the editor: create a ship, then let the preview spawn + settle.
            3 => {
                if let Some(button) = button_by_name(world, "Create New Spaceship Button V2") {
                    world.trigger(Activate { entity: button });
                }
                state.stage = 4;
                state.wait = settle_scene;
            }
            // Capture the editor with the built ship.
            4 => {
                if capturing {
                    hide_hud(world);
                    capture_window(world, "feature-editor.png");
                    info!("ui capture: feature-editor.png");
                }
                state.stage = 5;
            }
            _ => {}
        },
    }

    world.insert_resource(state);
}
