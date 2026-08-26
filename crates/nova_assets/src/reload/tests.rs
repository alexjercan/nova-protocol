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

/// The whole point of the cover: the read must not happen in the frame the
/// panel is spawned, or the freeze it hides lands before it is on screen.
#[test]
fn the_cover_goes_up_a_frame_before_the_read() {
    let mut app = app();
    app.add_systems(
        Update,
        (raise_reload_cover, reload_content, settle_reload).chain(),
    );
    app.world_mut()
        .resource_mut::<Messages<ReloadContent>>()
        .write(ReloadContent);

    app.update();
    assert_eq!(
        app.world().resource::<ContentReload>().phase,
        ReloadPhase::Covering,
        "the press raises the cover and reads nothing"
    );

    app.update();
    assert_ne!(
        app.world().resource::<ContentReload>().phase,
        ReloadPhase::Covering,
        "the frame after, the read goes out behind it"
    );
}

/// F5 twice is one reload. A second press must not restart the hold, or a
/// builder leaning on the key would hold the panel up for as long as they lean.
#[test]
fn a_second_press_under_the_cover_is_the_same_reload() {
    let mut app = app();
    app.add_systems(
        Update,
        (raise_reload_cover, reload_content, settle_reload).chain(),
    );
    app.world_mut()
        .resource_mut::<Messages<ReloadContent>>()
        .write(ReloadContent);
    app.update();
    let started = app.world().resource::<ContentReload>().started;

    app.world_mut()
        .resource_mut::<Messages<ReloadContent>>()
        .write(ReloadContent);
    app.update();

    assert_eq!(
        app.world().resource::<ContentReload>().started,
        started,
        "the reload in flight keeps its own start"
    );
}

/// The cover holds for the minimum dwell and comes down on a settled frame -
/// the frame after the merge, which is the long one it is there to hide.
#[test]
fn the_cover_holds_then_comes_down_on_a_settled_frame() {
    let mut app = app();
    app.add_systems(
        Update,
        (raise_reload_cover, reload_content, settle_reload).chain(),
    );
    app.world_mut()
        .resource_mut::<Messages<ReloadContent>>()
        .write(ReloadContent);
    app.update();
    app.update();
    assert!(
        app.world().get_resource::<ContentReload>().is_some(),
        "the cover must not flash away inside the minimum dwell"
    );

    std::thread::sleep(std::time::Duration::from_secs_f32(COVER_MIN_DWELL));
    // One update to pay the sleep as a long, unsettled frame, then a short one
    // that settles.
    app.update();
    app.update();

    assert!(
        app.world().get_resource::<ContentReload>().is_none(),
        "past the dwell, a settled frame takes the cover down"
    );
}

/// A file still out holds the cover past the dwell: the merge has not run yet,
/// and taking the panel down before it would show the freeze it exists to hide.
#[test]
fn a_file_still_out_holds_the_cover_past_the_dwell() {
    let mut app = app();
    // Started in the past, so only the pending file is holding the panel.
    app.insert_resource(ContentReload {
        started: -COVER_MIN_DWELL,
        phase: ReloadPhase::Reading,
        pending: 1,
    });
    app.add_systems(Update, settle_reload);

    app.update();
    assert!(
        app.world().get_resource::<ContentReload>().is_some(),
        "the dwell is spent, but the file is not back"
    );

    let id = app
        .world_mut()
        .resource_mut::<Assets<ContentAsset>>()
        .add(ContentAsset(Vec::new()))
        .id();
    app.world_mut()
        .resource_mut::<Messages<AssetEvent<ContentAsset>>>()
        .write(AssetEvent::Modified { id });
    app.update();

    assert!(
        app.world().get_resource::<ContentReload>().is_none(),
        "the last file back takes the cover down"
    );
}

/// A file that never comes back must not take the game with it. Unlike the
/// scenario screen's spawn gate - where a slow scene is WORKING - a read that
/// has not landed by the cap is a loader that failed, and there is nothing to
/// wait for.
#[test]
fn a_file_that_never_comes_back_still_gives_the_game_back() {
    let mut app = app();
    app.insert_resource(ContentReload {
        started: -COVER_MAX_DWELL,
        phase: ReloadPhase::Reading,
        pending: 1,
    });
    app.add_systems(Update, settle_reload);

    app.update();

    assert!(
        app.world().get_resource::<ContentReload>().is_none(),
        "the cap comes off even with a file still out"
    );
}
