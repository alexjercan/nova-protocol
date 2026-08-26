//! The restart's wiring, not its filesystem: what a press asks for, what the
//! answer does to the two states, and what a replaced file wakes.

use bevy::{asset::AssetPlugin, ecs::system::RunSystemOnce, state::app::StatesPlugin};
use nova_modding::prelude::{ContentAsset, NovaModdingPlugin};

use super::*;

/// An app with the asset server, the content asset type, the installed set and
/// the two states the restart drives.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), StatesPlugin));
    app.add_plugins(NovaModdingPlugin);
    app.init_resource::<DownloadedMods>();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_state::<GameAssetsStates>();
    app.init_state::<GameStates>();
    app.add_message::<ReloadContent>();
    app
}

#[test]
fn the_reload_key_asks_for_one_reload() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(RELOAD_KEY);

    app.world_mut()
        .run_system_once(request_reload_on_key)
        .expect("the key system runs");

    let asked = app
        .world_mut()
        .resource_mut::<Messages<ReloadContent>>()
        .drain()
        .count();
    assert_eq!(asked, 1, "one press, one reload");
}

#[test]
fn no_press_asks_for_nothing() {
    let mut app = app();

    app.world_mut()
        .run_system_once(request_reload_on_key)
        .expect("the key system runs");

    let asked = app
        .world_mut()
        .resource_mut::<Messages<ReloadContent>>()
        .drain()
        .count();
    assert_eq!(asked, 0);
}

/// The reload is a RESTART: both states go back to loading, which is the boot
/// path - the loading screen at `OnEnter(Loading)`, the whole merge again at
/// `OnEnter(Processing)`, and the main menu at the end of it.
#[test]
fn a_reload_puts_the_game_back_through_the_boot_load() {
    let mut app = app();
    app.insert_state(GameAssetsStates::Loaded);
    app.insert_state(GameStates::MainMenu);
    app.add_systems(Update, restart_for_content);
    app.update();
    app.world_mut()
        .resource_mut::<Messages<ReloadContent>>()
        .write(ReloadContent);

    // One update to run the system, one for the transitions it queued.
    app.update();
    app.update();

    assert_eq!(
        *app.world().resource::<State<GameAssetsStates>>().get(),
        GameAssetsStates::Loading,
        "the content is read again behind the boot loading screen"
    );
    assert_eq!(
        *app.world().resource::<State<GameStates>>().get(),
        GameStates::Loading,
        "and the game is back where the boot hook can hand it to the menu"
    );
}

/// Nothing asked, nothing moves. The system runs every frame in the main menu,
/// and a menu that reloaded itself on its own would never stay up.
#[test]
fn an_unasked_frame_leaves_the_game_where_it_is() {
    let mut app = app();
    app.insert_state(GameAssetsStates::Loaded);
    app.insert_state(GameStates::MainMenu);
    app.add_systems(Update, restart_for_content);

    app.update();
    app.update();

    assert_eq!(
        *app.world().resource::<State<GameAssetsStates>>().get(),
        GameAssetsStates::Loaded
    );
    assert_eq!(
        *app.world().resource::<State<GameStates>>().get(),
        GameStates::MainMenu
    );
}

/// The merge is gated on the INSTALLED SET, and re-saving a mod the index
/// already names changes nothing about the set. The replaced file is the only
/// signal there is.
#[test]
fn a_replaced_content_file_wakes_the_merge() {
    let mut app = app();
    let id = app
        .world_mut()
        .resource_mut::<Assets<ContentAsset>>()
        .add(ContentAsset(Vec::new()))
        .id();
    app.world_mut()
        .resource_mut::<Messages<AssetEvent<ContentAsset>>>()
        .write(AssetEvent::Modified { id });
    // Two updates so the change tick the fixture's own inserts left is spent:
    // the assertion is about the event, not about the setup.
    app.update();
    app.update();

    app.world_mut()
        .run_system_once(remerge_on_replaced_content)
        .expect("the remerge system runs");

    assert!(
        app.world().resource_ref::<DownloadedMods>().is_changed(),
        "a file that came back changed rebuilds the registries"
    );
}
