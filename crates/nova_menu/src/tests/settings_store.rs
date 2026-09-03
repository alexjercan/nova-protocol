//! `SettingsStorePlugin` on its own: the seam that makes a setting apply in an
//! app that has no settings panel to edit it in.
//!
//! Most examples supply their own game plugins and never build a menu, so the
//! rig here is deliberately menuless - a bare `App` with the store plugin and
//! nothing else. If the plugin needs a neighbour to load, these fail.
//!
//! Both directions are driven through the plugin's own access and its own
//! `root`, never through the process environment: a fixture that moves
//! `NOVA_CONFIG_ROOT` or reads `NOVA_CAPTURE` is one whose result depends on
//! what is running beside it.

use bevy::prelude::*;
use nova_gameplay::prelude::{GraphicsQuality, MasterVolume};
use nova_input::prelude::{MousePath, MouseSensitivity};

use crate::{
    settings_store::{
        allow_settings_saves, PersistedSettings, SettingsStoreAccess, SettingsStorePlugin, KEY,
        SETTINGS_SAVE_DEBOUNCE_FRAMES,
    },
    tests::support::scratch_store,
};

/// A store of this test's own, with `look` pushed to the top of its range and
/// the master volume off default, so a loaded value cannot be mistaken for the
/// default one.
fn store_a_played_in_settings(root: &std::path::Path) -> PersistedSettings {
    let saved = PersistedSettings {
        master_volume: 0.42,
        mouse_look_sensitivity: MousePath::Look.range().raw(300.0),
        graphics_quality: GraphicsQuality::Low,
        ..Default::default()
    };
    nova_assets::persist::save_to(&nova_assets::storage::NativeStorage::at(root), KEY, &saved);
    saved
}

/// A menuless app storing at `root`: the store plugin and nothing else, which
/// is what an example built with `with_game_plugins` gets.
fn menuless_app(access: SettingsStoreAccess, root: &std::path::Path) -> App {
    let mut app = App::new();
    app.add_plugins(SettingsStorePlugin {
        access,
        root: Some(root.to_path_buf()),
    });
    app
}

/// Read the store back off disk, whatever the app did or did not write.
fn on_disk(root: &std::path::Path) -> PersistedSettings {
    nova_assets::persist::load_from(&nova_assets::storage::NativeStorage::at(root), KEY)
        .expect("the store this test wrote is still there")
}

/// Move a setting, then run past the debounce and out through an exit, which
/// is every chance the plugin has to write.
fn edit_and_give_it_every_chance_to_save(app: &mut App) {
    app.world_mut().insert_resource(MasterVolume(0.99));
    for _ in 0..SETTINGS_SAVE_DEBOUNCE_FRAMES + 2 {
        app.update();
    }
    app.world_mut().write_message(AppExit::Success);
    app.update();
    app.update();
}

/// The point of the whole plugin: an app with no menu still boots on the
/// player's saved settings.
///
/// Before the store was split out of `NovaMenuPlugin`, every example flew on
/// the DEFAULTS - a mouse sensitivity the player had already rejected, and
/// keybinds they had already moved - because the menu was what loaded them and
/// an example never has one.
#[test]
fn a_menuless_app_boots_on_the_saved_settings() {
    let root = scratch_store("live");
    let _ = std::fs::remove_dir_all(&root);
    let saved = store_a_played_in_settings(&root);

    let mut app = menuless_app(SettingsStoreAccess::Read, &root);
    app.update();

    let sensitivity = *app.world().resource::<MouseSensitivity>();
    assert!(
        (sensitivity.look - saved.mouse_look_sensitivity).abs() < 1e-9,
        "the saved look sensitivity is the one the app flies on (got {})",
        sensitivity.look
    );
    assert!(
        (app.world().resource::<MasterVolume>().0 - 0.42).abs() < 1e-6,
        "the rest of the settings load with it"
    );
    assert_eq!(
        *app.world().resource::<GraphicsQuality>(),
        GraphicsQuality::Low,
        "including the ones an example never had before"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A scripted run reads nothing and writes nothing.
///
/// A capture or a probe sweep has to produce the same frames and the same
/// numbers on any machine, so the developer's own graphics preset must not
/// decide what a screenshot shows. The write direction is the sharper edge: a
/// screenshot pass that saves is a screenshot pass that rewrites the settings
/// of whoever launched it.
///
/// Driven through the plugin's own access rather than the environment:
/// `from_env` reads process-wide state that a parallel fixture in this binary
/// would also see.
#[test]
fn an_inert_store_neither_loads_nor_saves() {
    let root = scratch_store("inert");
    let _ = std::fs::remove_dir_all(&root);
    let saved = store_a_played_in_settings(&root);

    let mut app = menuless_app(SettingsStoreAccess::Inert, &root);
    app.update();

    assert_eq!(
        *app.world().resource::<MouseSensitivity>(),
        MouseSensitivity::default(),
        "a scripted run flies on the defaults, whatever is in the store"
    );
    assert_eq!(
        *app.world().resource::<GraphicsQuality>(),
        GraphicsQuality::default(),
        "so a capture is the same picture on every machine"
    );

    edit_and_give_it_every_chance_to_save(&mut app);
    assert!(
        (on_disk(&root).master_volume - saved.master_volume).abs() < 1e-6,
        "the run left the player's store exactly as it found it (got {})",
        on_disk(&root).master_volume
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// The split itself: an app with no settings panel reads the player's file and
/// cannot write it.
///
/// This is what a `with_game_plugins` example gets. Before the split it got
/// both directions, so a bench that poked `GraphicsQuality` on a keypress
/// saved that tier into the player's own `settings.ron`.
#[test]
fn a_reading_store_never_writes_what_the_app_changes() {
    let root = scratch_store("read_only");
    let _ = std::fs::remove_dir_all(&root);
    let saved = store_a_played_in_settings(&root);

    let mut app = menuless_app(SettingsStoreAccess::Read, &root);
    app.update();
    assert!(
        (app.world().resource::<MasterVolume>().0 - saved.master_volume).abs() < 1e-6,
        "the reading direction is the one every app keeps"
    );

    edit_and_give_it_every_chance_to_save(&mut app);
    assert!(
        (on_disk(&root).master_volume - saved.master_volume).abs() < 1e-6,
        "an app with nowhere to ask for a setting to be kept does not keep one \
         (got {})",
        on_disk(&root).master_volume
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// The write direction is granted after the fact, which is how the menu
/// upgrades the store `AppBuilder` already gave the app.
#[test]
fn a_settings_panel_grants_the_write_direction() {
    let root = scratch_store("granted");
    let _ = std::fs::remove_dir_all(&root);
    store_a_played_in_settings(&root);

    let mut app = menuless_app(SettingsStoreAccess::Read, &root);
    allow_settings_saves(&mut app);
    app.update();

    edit_and_give_it_every_chance_to_save(&mut app);
    assert!(
        (on_disk(&root).master_volume - 0.99).abs() < 1e-6,
        "a granted store keeps what the player moved (got {})",
        on_disk(&root).master_volume
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// An inert store is not upgradable: a scripted run that happens to build a
/// menu must not start saving over the settings of whoever launched it.
#[test]
fn a_scripted_run_that_builds_a_menu_stays_inert() {
    let root = scratch_store("inert_menu");
    let _ = std::fs::remove_dir_all(&root);

    let mut app = menuless_app(SettingsStoreAccess::Inert, &root);
    allow_settings_saves(&mut app);

    assert_eq!(
        *app.world().resource::<SettingsStoreAccess>(),
        SettingsStoreAccess::Inert,
        "the grant raises a READING store, never an inert one"
    );

    let _ = std::fs::remove_dir_all(&root);
}
