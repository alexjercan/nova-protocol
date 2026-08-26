//! The reload's wiring, not its filesystem: what a press asks for, and what a
//! replaced file wakes.

use bevy::{asset::AssetPlugin, ecs::system::RunSystemOnce, state::app::StatesPlugin};
use nova_modding::prelude::{ContentAsset, NovaModdingPlugin};

use super::*;

/// An app with the asset server, the content asset type and the installed set -
/// the three things the reload reads.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), StatesPlugin));
    app.add_plugins(NovaModdingPlugin);
    app.init_resource::<DownloadedMods>();
    app.init_resource::<ButtonInput<KeyCode>>();
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
