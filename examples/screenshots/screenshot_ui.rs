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
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive menu -> editor, reach
//!   Playing, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_REEL=1`: also capture each beat's PNG (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/reel NOVA_AUTOPILOT=1 NOVA_REEL=1 \
//!   cargo run --example screenshot_ui --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_ui --features debug
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

/// Seconds a step may sit before it is called a stall. Sized with headroom for a
/// slow software-rendered CI GPU (llvmpipe), where every beat costs more
/// wall-clock than on a real GPU. An expiry is an error exit naming the step,
/// which is the point: a walk that never reaches the menu or the editor now
/// fails loudly instead of idling out a fixed window.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 30.0;

/// Frames a beat settles before its shot. The long settles matter ONLY for the
/// capture path (`NOVA_REEL`): the scene/UI must be still and the PNG must land
/// before we navigate away. The smoke path (`NOVA_AUTOPILOT` alone) captures
/// nothing, so it drives straight through on minimal waits - just enough frames
/// for the next button to spawn and the state transition to apply. That keeps
/// the menu -> editor -> Playing walk short in FRAMES, which is what a slow
/// software-rendered CI GPU (llvmpipe) needs (task 20260716).
#[cfg(feature = "debug")]
struct Settle {
    scene: u32,
    after_capture: u32,
    after_nav: u32,
}

#[cfg(feature = "debug")]
impl Settle {
    fn new(capturing: bool) -> Self {
        if capturing {
            Self {
                scene: 90,
                after_capture: 20,
                after_nav: 30,
            }
        } else {
            Self {
                scene: 6,
                after_capture: 0,
                after_nav: 6,
            }
        }
    }
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same app the game/binary runs (main menu over the ambience backdrop).
    let mut app = editor_app(true);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            // Turn command errors (despawned-entity targets on the menu/editor
            // teardown) into panics so the run fails loudly on them (as 12 does).
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // Clean frames at a known 16:9 size: force the window resolution and drop
        // the dev overlays. The HUD chrome is re-hidden right before each capture
        // (entering the editor re-raises it).
        app.add_systems(Startup, (force_resolution, hide_dev_overlays));
        let capturing = std::env::var_os(nova_protocol::nova_debug::harness::REEL_ENV).is_some();
        let settle = Settle::new(capturing);
        // The menu -> editor walk a player would do, one step per beat: the
        // navigation steps wait on the STATE rather than on a frame count, and
        // the settles are the frames the scene/PNG need.
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                .step("reach the main menu")
                .enter(GameStates::Loading)
                .until(state_is(GameStates::MainMenu))
                .deadline(STEP_DEADLINE_SECS)
                .add()
                .step("settle the menu and its ambience backdrop")
                .until(frames(settle.scene))
                .add()
                // Hide the HUD first, and let the PNG land BEFORE navigating
                // away: clicking Sandbox in the same frame captured a black
                // mid-teardown frame.
                .step("capture the main menu")
                .on_enter(move |world| {
                    if capturing {
                        hide_hud(world);
                        capture_window(world, "tutorial-menu.png");
                        info!("ui capture: tutorial-menu.png");
                    }
                })
                .until(frames(settle.after_capture))
                .add()
                .step("leave for the editor")
                .on_enter(|world| click_button(world, "Sandbox Button"))
                .until(and(state_is(GameStates::Playing), frames(settle.after_nav)))
                .deadline(STEP_DEADLINE_SECS)
                .add()
                .step("build a ship in the editor")
                .on_enter(|world| click_button(world, "Create New Spaceship Button V2"))
                .until(frames(settle.scene))
                .add()
                // The last step's hold is the only thing giving `save_to_disk`
                // room to land: `capture_window` spawns a bare `Screenshot`, so
                // nothing registers it as a completion collector and the driver
                // reports done the moment this step ends.
                .step("capture the editor with the built ship")
                .on_enter(move |world| {
                    if capturing {
                        hide_hud(world);
                        capture_window(world, "feature-editor.png");
                        info!("ui capture: feature-editor.png");
                    }
                })
                .until(frames(settle.after_capture))
                .add(),
        );
    }

    app.run()
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
        *hud = HudVisibility::Cinematic;
    }
}

/// Activate the named button through the widget path a click takes.
#[cfg(feature = "debug")]
fn click_button(world: &mut World, name: &str) {
    let button = {
        let mut names = world.query::<(Entity, &Name)>();
        names
            .iter(world)
            .find(|(_, n)| n.as_str() == name)
            .map(|(entity, _)| entity)
    };
    if let Some(button) = button {
        world.trigger(bevy::ui_widgets::Activate { entity: button });
    }
}
