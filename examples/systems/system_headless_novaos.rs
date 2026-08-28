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
//! cargo run --example system_headless_novaos --features debug
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
    let mut app = editor_app(
        false,
        Some(StartupScenario::Id("shakedown_run".to_string())),
    );

    // The virtual window, exactly as in `system_headless_pointer`.
    app.world_mut().spawn((
        Window {
            resolution: (1280, 720).into(),
            ..default()
        },
        PrimaryWindow,
    ));

    app.init_resource::<Spike>();
    app.add_systems(PreUpdate, drive.after(bevy::input::InputSystems));

    app.run()
}

#[cfg(feature = "debug")]
#[derive(Resource)]
struct Spike {
    step: usize,
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

#[cfg(feature = "debug")]
const DEADLINE_SECS: u64 = 180;

#[cfg(feature = "debug")]
fn drive(world: &mut World) {
    let spike = world.resource::<Spike>();
    let (step, wait) = (spike.step, spike.wait);
    if spike.started.elapsed().as_secs() > DEADLINE_SECS {
        panic!("headless novaos: STALLED at step {step} after {DEADLINE_SECS}s");
    }

    let advance = |world: &mut World| {
        let mut spike = world.resource_mut::<Spike>();
        spike.step += 1;
        spike.wait = 0;
    };
    let hold = |world: &mut World| world.resource_mut::<Spike>().wait += 1;

    match step {
        0 => {
            if *world.resource::<State<GameStates>>().get() == GameStates::Playing {
                // The registry proof: with the gate removed, the verbs the
                // channel must advertise all exist off-screen too.
                let bindings = world.resource::<InputBindings>();
                let count = bindings.iter().count();
                info!("headless novaos: registry holds {count} actions");
                assert!(
                    bindings.get("novaos_toggle").is_some(),
                    "the NOVA OS toggle must register headless - it is the verb \
                     the render gate used to drop"
                );
                assert!(
                    count > 30,
                    "the full table is ~33 actions; {count} means a group is \
                     still gated out"
                );
                advance(world);
            }
        }
        // Tab, held across the enhanced-input read so the edge cannot fall
        // between two collectors, then released.
        1 => {
            if wait < 5 {
                hold(world);
            } else {
                press_key(KeyCode::Tab)(world);
                advance(world);
            }
        }
        2 => {
            if wait < 2 {
                hold(world);
            } else {
                release_key(KeyCode::Tab)(world);
                advance(world);
            }
        }
        3 => {
            if *world.resource::<State<PauseStates>>().get() == PauseStates::NovaOs {
                info!("headless novaos: Tab opened the monitor");
                advance(world);
            } else {
                hold(world);
            }
        }
        // The boot banner reveals on real time; the prompt is only worth
        // typing at once the reveal has drained.
        4 => {
            let terminal = world.resource::<NovaOsTerminal>();
            if terminal.is_booted() && !terminal.has_pending_boot_rows() {
                type_text("map")(world);
                advance(world);
            } else {
                hold(world);
            }
        }
        // The model proof: the characters landed in the RESOURCE, which is
        // the terminal's backend - the CRT texture nobody drew is just its
        // projection.
        5 => {
            let terminal = world.resource::<NovaOsTerminal>();
            if terminal.prompt() == "map" {
                info!("headless novaos: the prompt holds {:?}", terminal.prompt());
                press_edit_key(Key::Enter)(world);
                advance(world);
            } else if wait > 120 {
                panic!(
                    "headless novaos: typed `map` but the prompt holds {:?}",
                    terminal.prompt()
                );
            } else {
                hold(world);
            }
        }
        6 => {
            let terminal = world.resource::<NovaOsTerminal>();
            if terminal.active_mode() == (TerminalMode::App { id: "map" }) {
                info!("headless novaos: PASS typed into NOVA OS with no renderer");
                world.write_message(AppExit::Success);
                advance(world);
            } else {
                hold(world);
            }
        }
        _ => {}
    }
}
