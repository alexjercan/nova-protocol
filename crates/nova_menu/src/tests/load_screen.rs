//! The Load picker: a saved world lists and loads, a refused one lists and
//! refuses to load.

use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use nova_assets::prelude::{ContentCatalogDigest, LoadedSectionPacks};
use nova_gameplay::prelude::*;
use nova_world_base::{
    prelude::{create_world, ResumedWorld, WorldSaveSession, WorldSaveStatus},
    test_support::{arm_save_fixture, WorldSaveTestPlugin},
};

use super::support::{app, dummy_scenarios};
use crate::{load_screen::SelectedWorldSlug, scenarios::NewGameScenario, world_setup::WorldsRoot};

/// A menu app entered the real way, with the Load picker's plugin wiring in
/// place.
fn menu() -> App {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();
    app
}

fn named(app: &mut App, name: &str) -> Entity {
    let mut names = app.world_mut().query::<(Entity, &Name)>();
    names
        .iter(app.world())
        .find(|(_, found)| found.as_str() == name)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("no entity named '{name}'"))
}

fn press(app: &mut App, name: &str) {
    let entity = named(app, name);
    app.world_mut().trigger(Activate { entity });
    app.update();
}

fn state(app: &App) -> GameStates {
    app.world().resource::<State<GameStates>>().get().clone()
}

/// Run `app` until its [`WorldSaveSession`] is idle and holds a good save, or
/// panic after a generous number of frames - the production save pipeline is
/// off-thread, so it can take more than one frame to land.
fn save_until_idle(app: &mut App) {
    for _ in 0..200 {
        app.update();
        let session = app.world().resource::<WorldSaveSession>();
        if session.is_idle() && matches!(session.status(), WorldSaveStatus::Saved { .. }) {
            return;
        }
        assert!(
            !matches!(session.status(), WorldSaveStatus::Failed(_)),
            "the fixture save failed: {:?}",
            session.status()
        );
    }
    panic!("the fixture world did not finish saving in time");
}

#[test]
fn load_lists_a_saved_world_and_a_refused_one_and_loads_only_the_saved_one() {
    let mut app = menu();
    let root = tempfile::tempdir().expect("a scratch worlds root");
    app.insert_resource(WorldsRoot(Some(root.path().to_path_buf())));

    // Write a real save through the production pipeline: the fixture arms an
    // open world (catalog digest 1) with a 300-credit player, and
    // `WorldSaveTestPlugin` runs the same save systems the game does.
    app.add_plugins(WorldSaveTestPlugin);
    arm_save_fixture(app.world_mut());
    let (good_folder, good_lock) =
        create_world(root.path(), "Good World").expect("an empty root takes a new world");
    app.world_mut().insert_resource(WorldSaveSession::created(
        good_folder,
        good_lock,
        "Good World".to_string(),
        42,
    ));
    save_until_idle(&mut app);
    // Release the lock the fixture session held: Load re-locks the same
    // folder, and a held lock would refuse it right back.
    app.world_mut().remove_resource::<WorldSaveSession>();

    // A second world with no header at all: refused as unreadable, the same
    // branch a catalog mismatch takes, without needing a second full save.
    let (_bad_folder, bad_lock) =
        create_world(root.path(), "Bad World").expect("the root takes a second world");
    drop(bad_lock);

    press(&mut app, "Load Button");

    let good_row = named(&mut app, "Load World Row: good-world");
    let bad_row = named(&mut app, "Load World Row: bad-world");
    assert!(app.world().get::<Name>(good_row).is_some());
    assert!(app.world().get::<Name>(bad_row).is_some());

    app.world_mut().trigger(Activate { entity: bad_row });
    app.update();
    let load_button = named(&mut app, "Load World Button");
    assert!(
        app.world()
            .entity(load_button)
            .contains::<InteractionDisabled>(),
        "a refused world's Load button is greyed"
    );

    app.world_mut().trigger(Activate { entity: good_row });
    app.update();
    let load_button = named(&mut app, "Load World Button");
    assert!(
        !app.world()
            .entity(load_button)
            .contains::<InteractionDisabled>(),
        "a saved world's Load button is not greyed"
    );

    press(&mut app, "Load World Button");

    assert_eq!(
        app.world().resource::<WorldSaveSession>().name(),
        "Good World"
    );
    assert!(app.world().get_resource::<ResumedWorld>().is_some());
    assert_eq!(*app.world().resource::<GameMode>(), GameMode::NewGame);
    assert_eq!(app.world().resource::<NewGameScenario>().0, None);
    assert_eq!(state(&app), GameStates::Playing);
}

/// Delete asks to confirm first, and only removes the world and its row on confirm.
#[test]
fn delete_asks_first_then_removes_the_world_and_its_row() {
    let mut app = menu();
    let root = tempfile::tempdir().expect("a scratch worlds root");
    app.insert_resource(WorldsRoot(Some(root.path().to_path_buf())));
    app.insert_resource(LoadedSectionPacks {
        packs: Vec::new(),
        digest: ContentCatalogDigest(0),
    });

    let (_keep_folder, keep_lock) =
        create_world(root.path(), "Keep World").expect("an empty root takes a new world");
    drop(keep_lock);
    let (_gone_folder, gone_lock) =
        create_world(root.path(), "Gone World").expect("the root takes a second world");
    drop(gone_lock);

    press(&mut app, "Load Button");

    let gone_row = named(&mut app, "Load World Row: gone-world");
    app.world_mut().trigger(Activate { entity: gone_row });
    app.update();

    press(&mut app, "Load World Delete Button");
    named(&mut app, "Load World Delete Prompt");

    press(&mut app, "Load World Delete Cancel Button");
    assert!(
        root.path().join("gone-world").exists(),
        "cancelling a delete must not touch the folder"
    );
    named(&mut app, "Load World Delete Button");

    press(&mut app, "Load World Delete Button");
    press(&mut app, "Load World Delete Confirm Button");

    assert!(
        !root.path().join("gone-world").exists(),
        "confirming a delete must remove the folder"
    );
    assert!(
        root.path().join("keep-world").exists(),
        "a delete must not touch another world's folder"
    );
    let mut names = app.world_mut().query::<&Name>();
    assert!(
        !names
            .iter(app.world())
            .any(|name| name.as_str() == "Load World Row: gone-world"),
        "the deleted world's row must not redraw"
    );
    assert_eq!(
        app.world().resource::<SelectedWorldSlug>().0.as_deref(),
        Some("keep-world")
    );
}
