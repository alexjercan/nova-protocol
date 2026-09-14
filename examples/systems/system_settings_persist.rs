//! system_settings_persist: a setting a player changes survives the app.
//!
//! The composed path nothing else covers: the real Settings UI writes the
//! store, a NEW app reads it back, the value reaches the live resources AND
//! the widgets, and a rebound key drives its verb in gameplay. The focused
//! tests underneath this each own one link - `settings_store`'s serde
//! round-trip, `system_headless_rebind`'s one-app capture,
//! `system_headless_drag`'s slider - and none of them can see the JOIN,
//! because the join only exists across two app lifetimes.
//!
//! Three apps, in one process, in order:
//!
//!   1. EDIT. Through the pause Settings panel: rebind `main_drive` to J,
//!      set the master volume to 0.4, choose the `Low` graphics preset. Then
//!      wait for the normal save path - the debounce, not a hand-called
//!      `save_settings` - and read the file back.
//!   2. RELAUNCH. A second app on the same root. The live resources, the
//!      widgets and the flight rig all have to carry the saved values, and
//!      the ship has to burn on J and NOT on W. Then Reset Defaults, and the
//!      persisted keybind override has to go while the volume and the preset
//!      stay.
//!   3. DEFAULTS. A third app on a hand-authored store that predates most of
//!      the fields. Every field it omits has to load on its serde default.
//!
//! The store is the point, so this range cannot use the one every other
//! scripted range gets. `SettingsStorePlugin::from_env` makes the store INERT
//! under `NOVA_AUTOPILOT` (`harness_env_active`), which is right everywhere
//! else and is exactly what a persistence range cannot use: an inert store
//! neither loads nor saves. So the app is built through
//! `AppBuilder::with_settings_store` with an explicit
//! `SettingsStoreAccess::ReadWrite` on a TEMPORARY root of its own. The root
//! is what keeps the developer's real `settings.ron` out of it - the accident
//! `nova_menu/src/tests/support.rs` records - and 0.4, `Low` and J are
//! FIXTURES, not proposed defaults.
//!
//! Headless, because three `App`s live and die in one process and a winit
//! event loop does not. The settings body is a reconciler, so every click
//! re-resolves the next widget by `Name` (entity ids churn), exactly as
//! `system_headless_rebind` does.
//!
//! The run recorder is the last app's: a second recorder on one path
//! TRUNCATES the first one's timeline. The two earlier apps write
//! `timeline-edit.jsonl` and `timeline-relaunch.jsonl` beside it, so every
//! marker this range emits is still on disk under the run.
//!
//! Run (no display needed):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_settings_persist --features debug
//! # look for: `settings persist: PASS ...` once per phase.
//! ```

#[cfg(feature = "debug")]
use bevy::{prelude::*, ui_widgets::SliderValue, window::PrimaryWindow};
use clap::Parser;
#[cfg(feature = "debug")]
use nova_input::prelude::{BindingSpec, InputBindings, InputSource};
#[cfg(feature = "debug")]
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_ui::prelude::UiSkin;

#[derive(Parser)]
#[command(name = "system_settings_persist")]
#[command(version = "1.0.0")]
#[command(
    about = "Change settings through the real UI and prove they survive the app. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

#[cfg(not(feature = "debug"))]
fn main() {
    let _ = Cli::parse();
    eprintln!("system_settings_persist drives the app through the debug-only autopilot gestures;");
    eprintln!("run it with --features debug");
}

/// The master volume this run leaves behind. A FIXTURE - the shipped default
/// is 1.0 and stays 1.0.
#[cfg(feature = "debug")]
const FIXTURE_VOLUME: f32 = 0.4;

/// The key `main_drive` is moved onto. A FIXTURE - the shipped column is
/// W / Space and stays W / Space.
#[cfg(feature = "debug")]
const FIXTURE_KEY: KeyCode = KeyCode::KeyJ;

/// How far a volume read may sit off its fixture. The track is clicked at a
/// computed fraction of its own box, so what is left is f32 rounding.
#[cfg(feature = "debug")]
const VOLUME_EPSILON: f32 = 1e-3;

/// The widgets this range drives, by `Name`. Every one is resolved fresh on
/// the frame it is clicked: the settings body is rebuilt on each activation.
#[cfg(feature = "debug")]
const MASTER_TRACK: &str = "Master Volume Slider Track";
#[cfg(feature = "debug")]
const REBIND_CHIP: &str = "Rebind: main_drive Desk";
#[cfg(feature = "debug")]
const LOW_PRESET: &str = "Graphics Low";

#[cfg(feature = "debug")]
fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    let edited = temp_root("edited");
    let authored = temp_root("authored");
    let _ = std::fs::remove_dir_all(&edited);
    let _ = std::fs::remove_dir_all(&authored);

    finished("the editing app", editing_app(&edited).run());
    finished("the relaunched app", relaunch_app(&edited).run());
    author_an_older_partial_store(&authored);
    let exit = defaults_app(&authored).run();

    let _ = std::fs::remove_dir_all(&edited);
    let _ = std::fs::remove_dir_all(&authored);
    exit
}

/// A phase that did not exit cleanly is a FAILED range, not a quieter one: the
/// phases after it read the store the failed one was supposed to write, and
/// would otherwise report on an empty file.
#[cfg(feature = "debug")]
fn finished(phase: &str, exit: bevy::app::AppExit) {
    assert!(
        exit.is_success(),
        "{phase} did not exit cleanly ({exit:?}); the phases after it read what it wrote"
    );
}

/// A store root of this run's own, under the system temp dir.
///
/// Per-process, so two copies of this range on one box do not read each
/// other's file, and never the player's own store.
#[cfg(feature = "debug")]
fn temp_root(phase: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "nova_settings_persist_{}_{phase}",
        std::process::id()
    ))
}

/// The SHIPPED app - the one `editor_app(false, ..)` builds - on an explicit
/// store.
///
/// `with_settings_store` is the only difference, and it is the whole point:
/// see the module docs on why `from_env` cannot be used here.
#[cfg(feature = "debug")]
fn shipped_app_on(root: &std::path::Path) -> App {
    let mut app = AppBuilder::headless()
        .with_settings_store(SettingsStorePlugin {
            access: SettingsStoreAccess::ReadWrite,
            root: Some(root.to_path_buf()),
        })
        .with_startup_scenario(Some(StartupScenario::Id("tutorial".to_string())))
        .build();
    // Headless has no winit, so the UI needs a window to lay out in - the same
    // stand-in `system_headless_rebind` spawns.
    app.world_mut().spawn((
        Window {
            resolution: (1280, 720).into(),
            ..default()
        },
        PrimaryWindow,
    ));
    app
}

/// A marker sink for a phase that is not the last one.
///
/// [`ProbeTimeline::create`] TRUNCATES, so three apps pointed at the run's one
/// timeline would leave only the last one's markers on disk. Each earlier
/// phase gets a sibling file in the same run directory instead. Off the probe
/// there is no path to derive, and the recorder stays inert.
#[cfg(feature = "debug")]
fn phase_timeline(phase: &str) -> nova_probe::prelude::RunRecorderPlugin {
    let recorder = nova_probe::prelude::nova_timeline();
    match nova_probe::prelude::probe_param(nova_probe::prelude::TIMELINE_PARAM) {
        Some(path) => {
            let path = std::path::PathBuf::from(path);
            recorder.out(path.with_file_name(format!("timeline-{phase}.jsonl")))
        }
        None => recorder,
    }
}

// ---------------------------------------------------------------- phase 1 --

/// EDIT: move three settings through the real panel and let the store write.
#[cfg(feature = "debug")]
fn editing_app(root: &std::path::Path) -> App {
    let mut app = shipped_app_on(root);
    app.add_plugins(phase_timeline("edit"));
    let root = root.to_path_buf();
    let saved = root.clone();
    app.add_plugins(
        nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            .step("settings persist: reach Playing on an isolated store")
            .until(state_is(GameStates::Playing))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("settings persist: the store is writable and ours")
            .on_enter(move |world: &mut World| assert_the_store_is_ours(world, &root))
            .add()
            .step("settings persist: ESC opens the pause overlay")
            .on_enter(press_key(KeyCode::Escape))
            .until(resource_where::<State<PauseStates>>(|pause| {
                *pause.get() == PauseStates::Paused
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("settings persist: release ESC")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "settings persist: open Settings",
                "Pause Settings Button",
                ui_node_present("Settings Tab: Controls"),
                BEAT_DEADLINE_SECS,
            )
            .click_named(
                "settings persist: open Controls",
                "Settings Tab: Controls",
                ui_node_present("Controls Group: FLIGHT"),
                BEAT_DEADLINE_SECS,
            )
            .click_named(
                "settings persist: open FLIGHT",
                "Controls Group: FLIGHT",
                ui_node_present(REBIND_CHIP),
                BEAT_DEADLINE_SECS,
            )
            .click_named(
                "settings persist: arm main_drive",
                REBIND_CHIP,
                pointer_released(),
                BEAT_DEADLINE_SECS,
            )
            // The armed chip waits for `all_released()` before it captures, so
            // the beat that presses cannot be the beat that reads the result.
            .step("settings persist: press J")
            .on_enter(press_key(FIXTURE_KEY))
            .add()
            .step("settings persist: release J")
            .on_enter(release_key(FIXTURE_KEY))
            .add()
            .step("settings persist: the table took J")
            .until(main_drive_is_bound_to(FIXTURE_KEY))
            .diagnose(|world: &World| format!("main_drive holds {:?}", keyboard_column(world)))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .click_named(
                "settings persist: open Audio",
                "Settings Tab: Audio",
                ui_node_present(MASTER_TRACK),
                BEAT_DEADLINE_SECS,
            )
            .step("settings persist: set the master volume to 0.4")
            .on_enter(press_the_track_at(MASTER_TRACK, FIXTURE_VOLUME))
            .until(master_volume_is(FIXTURE_VOLUME))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("settings persist: release the track")
            .on_enter(release_mouse(MouseButton::Left))
            .add()
            .click_named(
                "settings persist: open Graphics",
                "Settings Tab: Graphics",
                ui_node_present(LOW_PRESET),
                BEAT_DEADLINE_SECS,
            )
            .click_named(
                "settings persist: choose the Low preset",
                LOW_PRESET,
                resource_where::<GraphicsQuality>(|quality| *quality == GraphicsQuality::Low),
                BEAT_DEADLINE_SECS,
            )
            // The DEBOUNCE, not a hand-called save: waiting on the file is what
            // makes this the path a player's edit actually takes.
            .step("settings persist: the normal save path writes the store")
            .until(the_store_carries_the_fixtures(saved.clone()))
            .diagnose(move |_: &World| format!("the store holds {:?}", stored(&saved)))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("settings persist: inspect the persisted value")
            .on_enter(assert_the_store_was_written)
            .add(),
    );
    app
}

/// The gate itself: this app writes, and it writes THERE.
///
/// Both halves matter. An inert store would leave every later phase reading an
/// empty file and reporting defaults as if they were saved values; a store on
/// the default root would be the developer's own `settings.ron`.
#[cfg(feature = "debug")]
fn assert_the_store_is_ours(world: &mut World, root: &std::path::Path) {
    let access = *world.resource::<SettingsStoreAccess>();
    assert_eq!(
        access,
        SettingsStoreAccess::ReadWrite,
        "this range has to WRITE; `SettingsStorePlugin::from_env` would have \
         made the store inert under NOVA_AUTOPILOT"
    );
    let live = world.resource::<SettingsStoreRoot>().clone();
    assert_eq!(
        live,
        SettingsStoreRoot(Some(root.to_path_buf())),
        "the store must be this run's own temporary root, never the player's"
    );
    assert_eq!(
        load_settings(&live),
        None,
        "the phase that WRITES the fixtures must start from an empty store, or \
         it proves nothing about having written them"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the run writes an isolated settings store",
        serde_json::json!({ "access": format!("{access:?}"), "root": root }),
    );
}

/// What is on disk right now, or `None` when nothing has been written.
#[cfg(feature = "debug")]
fn stored(root: &std::path::Path) -> Option<PersistedSettings> {
    load_settings(&SettingsStoreRoot(Some(root.to_path_buf())))
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

/// Advance once `main_drive`'s whole keyboard column is `key` - not
/// "contains": the rebind REPLACES the column, and W going away is half of
/// what the relaunch then has to see.
#[cfg(feature = "debug")]
fn main_drive_is_bound_to(
    key: KeyCode,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        keyboard_column(world) == vec![InputSource::Keyboard(key)]
    })
}

/// The live master volume, to the fixture's tolerance.
#[cfg(feature = "debug")]
fn master_volume_is(want: f32) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<MasterVolume>()
            .is_some_and(|volume| (volume.0 - want).abs() < VOLUME_EPSILON)
    })
}

/// Press the pointer on a slider track at the position that MEANS `value`.
///
/// `TrackClick::Snap` takes the value from where the press landed inside the
/// track's own box, and nova's tracks carry no `SliderThumb` and no
/// `SliderPrecision`, so the fraction of the box IS the fraction of the range.
/// Aiming at a computed fraction is how a run reaches an exact figure through
/// a real pointer instead of writing the resource behind the widget's back.
#[cfg(feature = "debug")]
fn press_the_track_at(track: &'static str, value: f32) -> impl Fn(&mut World) {
    move |world: &mut World| {
        let rect = ui_node_rect(world, track)
            .unwrap_or_else(|| panic!("`{track}` has not laid out, so it cannot be aimed at"));
        let at = Vec2::new(rect.min.x + rect.width() * value, rect.center().y);
        click_at(at, MouseButton::Left)(world);
    }
}

/// Advance once the file on disk carries all three fixtures.
///
/// A CONDITION on the artifact, not a frame count: the save is debounced
/// (`SETTINGS_SAVE_DEBOUNCE_FRAMES`), and how many frames that takes is the
/// store's business.
#[cfg(feature = "debug")]
fn the_store_carries_the_fixtures(
    root: std::path::PathBuf,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |_: &World| {
        stored(&root).is_some_and(|saved| {
            (saved.master_volume - FIXTURE_VOLUME).abs() < VOLUME_EPSILON
                && saved.graphics_quality == GraphicsQuality::Low
                && saved.keybinds.get("main_drive")
                    == Some(&BindingSpec {
                        keyboard: vec![InputSource::Keyboard(FIXTURE_KEY)],
                        gamepad: live_gamepad_column(&saved),
                    })
        })
    })
}

/// The gamepad column the rebind KEPT. A desk chip writes its own device's
/// column and leaves the other one alone, so the saved spec still carries the
/// shipped trigger - and a predicate that ignored it would pass over a rebind
/// that had wiped it.
#[cfg(feature = "debug")]
fn live_gamepad_column(saved: &PersistedSettings) -> Vec<InputSource> {
    saved
        .keybinds
        .get("main_drive")
        .map(|spec| spec.gamepad.clone())
        .unwrap_or_default()
}

/// State what the store now holds. The waiting was done by the beat before
/// this one; what is left is to say it, on the record.
#[cfg(feature = "debug")]
fn assert_the_store_was_written(world: &mut World) {
    let root = world.resource::<SettingsStoreRoot>().clone();
    let saved = load_settings(&root).expect("the beat before this one waited for the file");
    assert!(
        (saved.master_volume - FIXTURE_VOLUME).abs() < VOLUME_EPSILON,
        "the store must carry the volume the slider was clicked to ({} vs {FIXTURE_VOLUME})",
        saved.master_volume
    );
    assert_eq!(
        saved.graphics_quality,
        GraphicsQuality::Low,
        "the store must carry the preset the panel selected"
    );
    let spec = saved
        .keybinds
        .get("main_drive")
        .expect("only the CHANGED rows are saved, and main_drive is one");
    assert_eq!(
        spec.keyboard,
        vec![InputSource::Keyboard(FIXTURE_KEY)],
        "the store must carry the rebound column"
    );
    assert!(
        !spec.gamepad.is_empty(),
        "a desk rebind writes its own device's column only; the pad trigger \
         must still be in the saved spec"
    );
    assert_eq!(
        saved.keybinds.len(),
        1,
        "only the row the run moved is saved, so a default the game later moves \
         still reaches a player who never touched it"
    );
    info!("settings persist: PASS the settings UI wrote {saved:?}");
    nova_probe::probe_marker(
        world,
        "outcome: the settings UI writes the store",
        serde_json::json!({
            "master_volume": saved.master_volume,
            "graphics_quality": format!("{:?}", saved.graphics_quality),
            "keybinds": saved.keybinds.keys().collect::<Vec<_>>(),
        }),
    );
}

// ---------------------------------------------------------------- phase 2 --

/// RELAUNCH: a second app on the same root has to come up ON the saved values,
/// fly on the rebound key, and let a reset take the override back off disk.
#[cfg(feature = "debug")]
fn relaunch_app(root: &std::path::Path) -> App {
    let mut app = shipped_app_on(root);
    app.add_plugins(phase_timeline("relaunch"));
    let root = root.to_path_buf();
    app.add_plugins(
        nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            .step("settings persist: the relaunch reaches Playing")
            .until(state_is(GameStates::Playing))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("settings persist: the relaunch loaded the saved settings")
            .on_enter(assert_the_relaunch_loaded_the_store)
            .add()
            // GAMEPLAY, not the table: the rebind is only worth anything if the
            // rig the ship flies on was built from it.
            .step("settings persist: hold the rebound key")
            .on_enter(press_key(FIXTURE_KEY))
            .until(player_burn_is(|burn| burn > 0.0))
            .diagnose(|world: &World| format!("the player's burn is {:?}", burn_reading(world)))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("settings persist: release the rebound key")
            .on_enter(release_key(FIXTURE_KEY))
            .until(player_burn_is(|burn| burn == 0.0))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("settings persist: hold the key the rebind replaced")
            .on_enter(press_key(KeyCode::KeyW))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("settings persist: the rebound key is the one that flies")
            .on_enter(assert_only_the_rebound_key_burns)
            .add()
            .step("settings persist: release the replaced key")
            .on_enter(release_key(KeyCode::KeyW))
            .add()
            .step("settings persist: ESC opens the pause overlay")
            .on_enter(press_key(KeyCode::Escape))
            .until(resource_where::<State<PauseStates>>(|pause| {
                *pause.get() == PauseStates::Paused
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("settings persist: release ESC")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "settings persist: reopen Settings",
                "Pause Settings Button",
                ui_node_present(MASTER_TRACK),
                BEAT_DEADLINE_SECS,
            )
            .step("settings persist: the reopened Audio tab shows the saved volume")
            .on_enter(assert_the_track_shows_the_saved_volume)
            .add()
            .click_named(
                "settings persist: reopen Graphics",
                "Settings Tab: Graphics",
                ui_node_present(LOW_PRESET),
                BEAT_DEADLINE_SECS,
            )
            .step("settings persist: the reopened Graphics tab shows Low")
            .on_enter(assert_the_preset_row_shows_low)
            .add()
            .click_named(
                "settings persist: reopen Controls",
                "Settings Tab: Controls",
                ui_node_present("Controls Group: FLIGHT"),
                BEAT_DEADLINE_SECS,
            )
            .click_named(
                "settings persist: reopen FLIGHT",
                "Controls Group: FLIGHT",
                ui_node_present(REBIND_CHIP),
                BEAT_DEADLINE_SECS,
            )
            .step("settings persist: the reopened chip shows the rebound key")
            .on_enter(assert_the_chip_shows_the_rebound_key)
            .add()
            // The only override this run holds is `main_drive`, so Reset
            // Defaults is a one-row reset - and the two settings beside it are
            // what says so.
            .click_named(
                "settings persist: reset the binding",
                "Reset Bindings",
                main_drive_is_on_its_default(),
                BEAT_DEADLINE_SECS,
            )
            .step("settings persist: the reset reaches the file")
            .until(the_store_has_dropped_the_keybind(root.clone()))
            .diagnose(move |_: &World| format!("the store holds {:?}", stored(&root)))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("settings persist: only the keybind was reset")
            .on_enter(assert_only_the_keybind_was_reset)
            .add(),
    );
    app
}

/// The relaunch starts ON the saved values - in the resources the game reads
/// and in the table the rig is built from.
#[cfg(feature = "debug")]
fn assert_the_relaunch_loaded_the_store(world: &mut World) {
    let volume = world.resource::<MasterVolume>().0;
    assert!(
        (volume - FIXTURE_VOLUME).abs() < VOLUME_EPSILON,
        "a new app must come up on the SAVED master volume ({volume} vs {FIXTURE_VOLUME})"
    );
    assert_eq!(
        *world.resource::<GraphicsQuality>(),
        GraphicsQuality::Low,
        "a new app must come up on the saved graphics preset"
    );
    assert_eq!(
        keyboard_column(world),
        vec![InputSource::Keyboard(FIXTURE_KEY)],
        "a new app must come up on the saved keybind"
    );
    info!("settings persist: PASS the relaunch loaded volume {volume} and Low");
    nova_probe::probe_marker(
        world,
        "outcome: a new app comes up on the saved settings",
        serde_json::json!({ "master_volume": volume, "main_drive": format!("{:?}", keyboard_column(world)) }),
    );
}

/// The player ship's commanded burn, `None` when no player ship is up.
#[cfg(feature = "debug")]
fn player_burn(world: &mut World) -> Option<f32> {
    world
        .try_query_filtered::<&FlightIntent, With<PlayerSpaceshipMarker>>()?
        .iter(world)
        .map(|intent| intent.burn)
        .next()
}

/// The same reading through a shared `&World`, for a predicate and a
/// diagnostic.
#[cfg(feature = "debug")]
fn burn_reading(world: &World) -> Option<f32> {
    world
        .try_query_filtered::<&FlightIntent, With<PlayerSpaceshipMarker>>()
        .and_then(|mut query| query.iter(world).map(|intent| intent.burn).next())
}

/// Advance once the player's burn satisfies `want`.
#[cfg(feature = "debug")]
fn player_burn_is(
    want: fn(f32) -> bool,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| burn_reading(world).is_some_and(want))
}

/// W is dead and J is what flies.
///
/// The negative half is the one that cannot be faked: a rig still carrying the
/// shipped column would burn on W, and a run that only pressed J could not
/// tell a loaded rebind from a rig that answers to everything.
#[cfg(feature = "debug")]
fn assert_only_the_rebound_key_burns(world: &mut World) {
    let burn = player_burn(world).expect("the tutorial puts a player ship up");
    assert_eq!(
        burn, 0.0,
        "W must be dead after the rebind: the flight rig is built from the \
         table the store loaded, and the rebind REPLACED the column"
    );
    info!("settings persist: PASS the saved rebind is what the flight rig flies on");
    nova_probe::probe_marker(
        world,
        "outcome: the rebound key drives the verb in gameplay",
        serde_json::json!({ "burn_on_w": burn, "main_drive": format!("{:?}", keyboard_column(world)) }),
    );
}

/// The slider widget - what the player LOOKS at - carries the saved value, not
/// just the resource behind it.
#[cfg(feature = "debug")]
fn assert_the_track_shows_the_saved_volume(world: &mut World) {
    let widget = named_slider_value(world, MASTER_TRACK)
        .expect("the Audio tab's master track carries bevy's SliderValue");
    assert!(
        (widget - FIXTURE_VOLUME).abs() < VOLUME_EPSILON,
        "the reopened track must sit on the saved volume ({widget} vs {FIXTURE_VOLUME})"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the relaunched settings panel shows the saved values",
        serde_json::json!({ "master_track": widget }),
    );
}

/// The `Low` chip wears the selection, so the row agrees with the resource.
#[cfg(feature = "debug")]
fn assert_the_preset_row_shows_low(world: &mut World) {
    let selected = world
        .query_filtered::<&Name, With<nova_ui::widget::Selected>>()
        .iter(world)
        .any(|name| name.as_str() == LOW_PRESET);
    assert!(
        selected,
        "the reopened Graphics row must show `{LOW_PRESET}` as the selection"
    );
}

/// The chip in the reopened Controls tab reads the rebound key.
#[cfg(feature = "debug")]
fn assert_the_chip_shows_the_rebound_key(world: &mut World) {
    let want = world
        .resource::<InputBindings>()
        .get("main_drive")
        .map(|row| row.keyboard_display())
        .expect("main_drive is a registered flight action");
    let faces = chip_faces(world, REBIND_CHIP);
    assert!(
        faces.iter().any(|face| want.contains(face.as_str())),
        "the reopened `{REBIND_CHIP}` must read `{want}`, and it shows {faces:?}"
    );
}

/// What one rebind chip DRAWS: its keycap names and its plain-text fallbacks.
///
/// Read off the widget subtree rather than off the table, so this says what is
/// on the screen instead of restating the resource the assertion beside it
/// already covers.
#[cfg(feature = "debug")]
fn chip_faces(world: &mut World, chip: &str) -> Vec<String> {
    let Some(root) = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, name)| name.as_str() == chip)
        .map(|(entity, _)| entity)
    else {
        return Vec::new();
    };
    let mut faces = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(cap) = world
            .get::<Name>(entity)
            .and_then(|name| name.as_str().strip_prefix("Keycap: ").map(str::to_string))
        {
            faces.push(cap);
        }
        if let Some(text) = world.get::<Text>(entity) {
            faces.push(text.0.clone());
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    faces
}

/// The track's own `SliderValue`, which bevy writes and nova's `ValueChange`
/// observer reads.
#[cfg(feature = "debug")]
fn named_slider_value(world: &mut World, track: &str) -> Option<f32> {
    world
        .try_query::<(&Name, &SliderValue)>()?
        .iter(world)
        .find(|(name, _)| name.as_str() == track)
        .map(|(_, value)| value.0)
}

/// Advance once `main_drive` is back on the column it shipped with.
#[cfg(feature = "debug")]
fn main_drive_is_on_its_default() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<InputBindings>()
            .is_some_and(|bindings| !bindings.overrides().contains_key("main_drive"))
    })
}

/// Advance once the FILE has dropped the override - not merely the resource.
#[cfg(feature = "debug")]
fn the_store_has_dropped_the_keybind(
    root: std::path::PathBuf,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |_: &World| {
        stored(&root).is_some_and(|saved| !saved.keybinds.contains_key("main_drive"))
    })
}

/// The reset took the keybind off disk and left everything else where it was.
#[cfg(feature = "debug")]
fn assert_only_the_keybind_was_reset(world: &mut World) {
    let root = world.resource::<SettingsStoreRoot>().clone();
    let saved = load_settings(&root).expect("the beat before this one waited for the file");
    assert!(
        saved.keybinds.is_empty(),
        "the reset must take the override OFF DISK, not only out of the live \
         table; the store still holds {:?}",
        saved.keybinds
    );
    assert!(
        (saved.master_volume - FIXTURE_VOLUME).abs() < VOLUME_EPSILON,
        "a keybind reset must not touch the saved volume ({} vs {FIXTURE_VOLUME})",
        saved.master_volume
    );
    assert_eq!(
        saved.graphics_quality,
        GraphicsQuality::Low,
        "a keybind reset must not touch the saved graphics preset"
    );
    assert_eq!(
        keyboard_column(world),
        world
            .resource::<InputBindings>()
            .get("main_drive")
            .map(|row| row.keyboard.clone())
            .unwrap_or_default(),
        "the live table and the row it answers with must agree after a reset"
    );
    info!("settings persist: PASS the reset cleared the persisted keybind alone");
    nova_probe::probe_marker(
        world,
        "outcome: a reset clears the persisted keybind and nothing else",
        serde_json::json!({
            "keybinds": saved.keybinds.len(),
            "master_volume": saved.master_volume,
            "graphics_quality": format!("{:?}", saved.graphics_quality),
        }),
    );
}

// ---------------------------------------------------------------- phase 3 --

/// A store from before most of the fields existed, written by hand.
///
/// Authored rather than produced by an older build, because that is the file
/// the claim is about: a player's `settings.ron` from a release that had two
/// of these fields. Every omitted field has to arrive on its `#[serde(default)]`.
#[cfg(feature = "debug")]
fn author_an_older_partial_store(root: &std::path::Path) {
    std::fs::create_dir_all(root).expect("the temporary store root must be creatable");
    std::fs::write(
        root.join("settings.ron"),
        b"(master_volume: 0.25, graphics_quality: Low)",
    )
    .expect("the older store must be writable");
}

/// What the hand-authored store above actually names.
#[cfg(feature = "debug")]
const AUTHORED_VOLUME: f32 = 0.25;

/// DEFAULTS: a third app on the older file.
#[cfg(feature = "debug")]
fn defaults_app(root: &std::path::Path) -> App {
    let mut app = shipped_app_on(root);
    // The LAST phase owns the run's timeline, the contract and the invariant
    // stream - see the module docs.
    app.add_plugins(nova_probe::NovaProbePlugin::default());
    app.add_plugins(
        nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            .step("settings persist: the older store reaches Playing")
            .until(state_is(GameStates::Playing))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("settings persist: the omitted fields load on their defaults")
            .on_enter(assert_the_older_store_defaults)
            .add(),
    );
    app
}

/// The two authored fields arrived, and every field the file does not mention
/// is on the value a fresh install starts with.
#[cfg(feature = "debug")]
fn assert_the_older_store_defaults(world: &mut World) {
    let volume = world.resource::<MasterVolume>().0;
    assert!(
        (volume - AUTHORED_VOLUME).abs() < VOLUME_EPSILON,
        "the authored field must load ({volume} vs {AUTHORED_VOLUME})"
    );
    assert_eq!(
        *world.resource::<GraphicsQuality>(),
        GraphicsQuality::Low,
        "the other authored field must load"
    );
    let defaults = PersistedSettings::default();
    let interface = world.resource::<InterfaceVolume>().0;
    assert!(
        (interface - defaults.interface_volume).abs() < VOLUME_EPSILON,
        "a bus the older file never had must load on its serde default \
         ({interface} vs {})",
        defaults.interface_volume
    );
    assert_eq!(
        *world.resource::<UiSkin>(),
        defaults.ui_skin,
        "a setting the older file never had must load on its serde default"
    );
    assert!(
        world.resource::<InputBindings>().overrides().is_empty(),
        "an older file with no keybinds block leaves every row on its default"
    );
    info!("settings persist: PASS the older partial store loaded on its defaults");
    nova_probe::probe_marker(
        world,
        "outcome: an older partial store loads its omitted fields on defaults",
        serde_json::json!({
            "master_volume": volume,
            "interface_volume": interface,
            "ui_skin": format!("{:?}", *world.resource::<UiSkin>()),
        }),
    );
}
