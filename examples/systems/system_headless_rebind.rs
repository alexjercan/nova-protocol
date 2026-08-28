//! system_headless_rebind: spike 4 for `nova_channel` - rebind a key by wire.
//!
//! The ledger row this proves (task 20260820-174148, nova-channel.html):
//! "Rebind a key from Settings - the capture polls `ButtonInput`, which the
//! lanes already write". The whole flow is driven with the events a channel
//! client would send, headless: ESC to pause, click through Settings ->
//! Controls -> FLIGHT by widget `Name` (the body is a reconciler - entity ids
//! churn on every click, names are the only stable address), click the
//! `main_drive` desk chip to arm the capture, then press J and watch the
//! REGISTRY take the override - the whole keyboard column moves, W and Space
//! are both gone, and `overrides()` goes from empty to one row.
//!
//! Two rules of the capture that shape the beats, both from
//! `nova_menu/src/settings.rs`:
//!
//!   - the armed chip waits for `all_released()` before it will capture
//!     (`awaiting_release`, `settings.rs:646`), so the click that armed it
//!     must fully release and idle a frame before the key goes down;
//!   - Escape both cancels a capture AND toggles the pause overlay
//!     (`pause.rs:62`), so no beat here uses Escape past the first one.
//!
//! The config store is ISOLATED before the app builds: the settings save
//! debounce plus `flush_settings_on_exit` would otherwise write this run's
//! rebind into the developer's real `settings.ron` - the exact accident
//! `nova_menu/src/tests/support.rs` records.
//!
//! Run (no display needed):
//! ```text
//! cargo run --example system_headless_rebind --features debug
//! # look for: `headless rebind: PASS the registry took J for main_drive`.
//! ```

#[cfg(feature = "debug")]
use bevy::{prelude::*, window::PrimaryWindow};
#[cfg(feature = "debug")]
use nova_input::prelude::{InputBindings, InputSource};
#[cfg(feature = "debug")]
use nova_protocol::prelude::*;

#[cfg(not(feature = "debug"))]
fn main() {
    eprintln!("system_headless_rebind drives the app through the debug-only autopilot gestures;");
    eprintln!("run it with --features debug");
}

#[cfg(feature = "debug")]
fn main() -> bevy::app::AppExit {
    // BEFORE the app builds: `load_persisted_settings` runs at Startup and
    // would seed the table from the real store; the exit flush would write
    // back into it.
    std::env::set_var(
        "NOVA_CONFIG_ROOT",
        std::env::temp_dir().join("nova_channel_rebind_config"),
    );

    let mut app = editor_app(
        false,
        Some(StartupScenario::Id("shakedown_run".to_string())),
    );

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
    /// Frames since the current click target FIRST resolved - the gesture
    /// counter, distinct from `wait` (frames since the step began).
    phase: u32,
    started: std::time::Instant,
}

#[cfg(feature = "debug")]
impl Default for Spike {
    fn default() -> Self {
        Self {
            step: 0,
            wait: 0,
            phase: 0,
            started: std::time::Instant::now(),
        }
    }
}

#[cfg(feature = "debug")]
const DEADLINE_SECS: u64 = 180;

/// Drive one click on a named widget the way a person does: aim, wait until
/// the pointer is ACTUALLY over it, press, release. True once the release
/// fired.
///
/// The hover gate is the actionability check, and it is load-bearing: a
/// resolvable rect is NOT clickable yet. The scenario loading screen fades
/// out OVER the freshly opened pause overlay and eats every pick for about a
/// second, so a driver that presses on a frame count clicks the fade, not the
/// button. The press waits until the pick map says the aim landed on the
/// widget itself, re-aiming every frame while anything else - an occluder, a
/// reflow - keeps it off.
#[cfg(feature = "debug")]
fn click_named(world: &mut World, name: &str) -> bool {
    if ui_node_rect(world, name).is_none() {
        world.resource_mut::<Spike>().phase = 0;
        return false;
    }
    let phase = world.resource::<Spike>().phase;
    match phase {
        0 | 1 => {
            hover_named(name)(world);
            if phase == 1 && hovering(world, name) {
                world.resource_mut::<Spike>().phase = 2;
            } else {
                world.resource_mut::<Spike>().phase = 1;
                let wait = world.resource::<Spike>().wait;
                if wait % 60 == 59 {
                    info!(
                        "click: aim at {name} blocked; the pointer is over {:?}",
                        hovered(world)
                    );
                }
            }
        }
        2 => {
            press_mouse(MouseButton::Left)(world);
            world.resource_mut::<Spike>().phase = 3;
        }
        _ => {
            release_mouse(MouseButton::Left)(world);
            world.resource_mut::<Spike>().phase = 0;
            return true;
        }
    }
    false
}

/// Whether the window mouse pointer's picks include `name` - checked against
/// each hit AND its ancestors, because picking reports the deepest node (for
/// a labelled button, its text).
#[cfg(feature = "debug")]
fn hovering(world: &World, name: &str) -> bool {
    use bevy::picking::{hover::HoverMap, pointer::PointerId};
    let Some(hits) = world.resource::<HoverMap>().get(&PointerId::Mouse) else {
        return false;
    };
    hits.keys().any(|hit| {
        std::iter::successors(Some(*hit), |entity| {
            world.get::<ChildOf>(*entity).map(|child| child.parent())
        })
        .any(|entity| {
            world
                .get::<Name>(entity)
                .is_some_and(|named| named.as_str() == name)
        })
    })
}

/// What the window mouse pointer is over, named where possible - so a blocked
/// aim names its occluder.
#[cfg(feature = "debug")]
fn hovered(world: &World) -> Vec<String> {
    use bevy::picking::{hover::HoverMap, pointer::PointerId};
    let Some(hits) = world.resource::<HoverMap>().get(&PointerId::Mouse) else {
        return Vec::new();
    };
    hits.keys()
        .map(|hit| {
            std::iter::successors(Some(*hit), |entity| {
                world.get::<ChildOf>(*entity).map(|child| child.parent())
            })
            .find_map(|entity| world.get::<Name>(entity).map(|name| name.to_string()))
            .unwrap_or_else(|| "unnamed".to_string())
        })
        .collect()
}

#[cfg(feature = "debug")]
fn drive(world: &mut World) {
    let spike = world.resource::<Spike>();
    let (step, wait) = (spike.step, spike.wait);
    if spike.started.elapsed().as_secs() > DEADLINE_SECS {
        panic!("headless rebind: STALLED at step {step} after {DEADLINE_SECS}s");
    }

    let advance = |world: &mut World| {
        let mut spike = world.resource_mut::<Spike>();
        spike.step += 1;
        spike.wait = 0;
        spike.phase = 0;
    };
    let hold = |world: &mut World| world.resource_mut::<Spike>().wait += 1;
    let click = |world: &mut World, name: &str| {
        if click_named(world, name) {
            advance(world);
        } else {
            hold(world);
        }
    };

    match step {
        0 => {
            if *world.resource::<State<GameStates>>().get() == GameStates::Playing {
                assert!(
                    world.resource::<InputBindings>().overrides().is_empty(),
                    "the isolated store must start clean - a leftover override \
                     means NOVA_CONFIG_ROOT did not isolate"
                );
                advance(world);
            }
        }
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
                advance(world);
            } else {
                hold(world);
            }
        }
        // The walk in: every activation reconciles the settings body, so each
        // beat re-resolves the NEXT name from scratch.
        4 => click(world, "Pause Settings Button"),
        5 => click(world, "Settings Tab: Controls"),
        6 => click(world, "Controls Group: FLIGHT"),
        7 => {
            if wait == 0 {
                info!("headless rebind: FLIGHT rows are up; arming main_drive");
            }
            click(world, "Rebind: main_drive Desk");
        }
        // The armed chip waits for all_released(); give it clean frames with
        // nothing down before the key arrives.
        8 => {
            if wait < 3 {
                hold(world);
            } else {
                press_key(KeyCode::KeyJ)(world);
                advance(world);
            }
        }
        9 => {
            release_key(KeyCode::KeyJ)(world);
            advance(world);
        }
        10 => {
            let bindings = world.resource::<InputBindings>();
            let main_drive = bindings
                .get("main_drive")
                .expect("main_drive is a registry row");
            if main_drive.keyboard == vec![InputSource::Keyboard(KeyCode::KeyJ)] {
                assert!(
                    bindings.overrides().contains_key("main_drive"),
                    "the diff-against-defaults set must carry the rebind"
                );
                info!("headless rebind: PASS the registry took J for main_drive");
                info!(
                    "headless rebind: keyboard column is now {:?} (W and Space are gone)",
                    main_drive.keyboard
                );
                world.write_message(AppExit::Success);
                advance(world);
            } else if wait > 240 {
                panic!(
                    "headless rebind: J never landed; main_drive holds {:?}",
                    main_drive.keyboard
                );
            } else {
                hold(world);
            }
        }
        _ => {}
    }
}
