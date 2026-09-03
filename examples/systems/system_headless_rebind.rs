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
//!     (`awaiting_release`, `settings.rs:646`), so the key beat holds until
//!     the registry ITSELF has taken J rather than assuming one frame of
//!     quiet was enough;
//!   - Escape both cancels a capture AND toggles the pause overlay
//!     (`pause.rs:62`), so no beat here uses Escape past the first one.
//!
//! The store is INERT here, and the first beat asserts it. `NOVA_AUTOPILOT` is
//! what makes it so (`SettingsStorePlugin::from_env`), and it has to hold in
//! both directions: a live store would seed the table from the developer's real
//! `settings.ron` - so this run would not be starting from the defaults it
//! claims - and the save debounce plus `flush_settings_on_exit` would write
//! this run's rebind back into it, the exact accident
//! `nova_menu/src/tests/support.rs` records.
//!
//! Run (no display needed):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_headless_rebind --features debug
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

    app.add_plugins(
        nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            .step("headless rebind: reach Playing with no renderer")
            .until(state_is(GameStates::Playing))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("headless rebind: the settings store is inert")
            .on_enter(assert_the_store_is_inert)
            .add()
            .step("headless rebind: ESC opens the pause overlay")
            .on_enter(press_key(KeyCode::Escape))
            .until(resource_where::<State<PauseStates>>(|pause| {
                *pause.get() == PauseStates::Paused
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless rebind: release ESC")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            // The walk in. Every activation reconciles the settings body, so
            // each click re-resolves the NEXT name from scratch - and each aim
            // holds until the pick map says it landed, because the scenario
            // loading screen fades out OVER the fresh overlay and eats picks
            // for about a second.
            .click_named(
                "headless rebind: open Settings",
                "Pause Settings Button",
                ui_node_present("Settings Tab: Controls"),
                BEAT_DEADLINE_SECS,
            )
            .click_named(
                "headless rebind: open Controls",
                "Settings Tab: Controls",
                ui_node_present("Controls Group: FLIGHT"),
                BEAT_DEADLINE_SECS,
            )
            .click_named(
                "headless rebind: open FLIGHT",
                "Controls Group: FLIGHT",
                ui_node_present("Rebind: main_drive Desk"),
                BEAT_DEADLINE_SECS,
            )
            .click_named(
                "headless rebind: arm main_drive",
                "Rebind: main_drive Desk",
                pointer_released(),
                BEAT_DEADLINE_SECS,
            )
            // J goes down and stays down until the REGISTRY has taken it. The
            // armed chip captures only once everything is released, so the
            // beat that presses cannot also be the beat that reads the result.
            .step("headless rebind: press J")
            .on_enter(press_key(KeyCode::KeyJ))
            .add()
            .step("headless rebind: release J")
            .on_enter(release_key(KeyCode::KeyJ))
            .add()
            .step("headless rebind: the registry took J for main_drive")
            .until(main_drive_is_bound_to(KeyCode::KeyJ))
            .diagnose(|world: &World| format!("main_drive holds {:?}", keyboard_column(world)))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless rebind: record the pass")
            .on_enter(record_the_rebind)
            .add(),
    );

    app.run()
}

/// The keyboard column `main_drive` currently binds, empty when the row is not
/// registered.
#[cfg(feature = "debug")]
fn keyboard_column(world: &World) -> Vec<InputSource> {
    world
        .get_resource::<InputBindings>()
        .and_then(|bindings| bindings.get("main_drive"))
        .map(|row| row.keyboard.clone())
        .unwrap_or_default()
}

/// Advance once `main_drive`'s whole keyboard column is `key` - not "contains",
/// because the rebind REPLACES the column and W and Space going away is half
/// of what this range proves.
#[cfg(feature = "debug")]
fn main_drive_is_bound_to(
    key: KeyCode,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        keyboard_column(world) == vec![InputSource::Keyboard(key)]
    })
}

/// The gate itself, not its consequence: under `NOVA_AUTOPILOT` the store is
/// inert, so nothing was loaded and nothing will be written.
///
/// The empty override set is the consequence and is asserted with it, but on
/// its own it proves nothing - an inert store leaves it empty whatever is in
/// the developer's `settings.ron`, so a broken gate would read as a pass.
#[cfg(feature = "debug")]
fn assert_the_store_is_inert(world: &mut World) {
    let live = world.resource::<SettingsStoreLive>().0;
    assert!(
        !live,
        "a scripted run must carry an inert settings store - a live one starts \
         from the developer's own keybinds and ends by overwriting them"
    );
    assert!(
        world.resource::<InputBindings>().overrides().is_empty(),
        "an inert store leaves the table on its defaults"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the rebind run's settings store is inert",
        serde_json::json!({ "live": live }),
    );
}

/// State what the registry now holds, and that the diff-against-defaults set
/// carries it.
#[cfg(feature = "debug")]
fn record_the_rebind(world: &mut World) {
    assert!(
        world
            .resource::<InputBindings>()
            .overrides()
            .contains_key("main_drive"),
        "the diff-against-defaults set must carry the rebind"
    );
    let column = format!("{:?}", keyboard_column(world));
    info!("headless rebind: PASS the registry took J for main_drive");
    info!("headless rebind: keyboard column is now {column} (W and Space are gone)");
    nova_probe::probe_marker(
        world,
        "outcome: the registry takes a wire rebind",
        serde_json::json!({ "main_drive": column }),
    );
}
