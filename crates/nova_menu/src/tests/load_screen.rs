//! The Load picker: a saved world lists and loads, a refused one lists and
//! refuses to load.

use std::{future, thread, time::Duration};

use bevy::{prelude::*, tasks::IoTaskPool, ui::InteractionDisabled, ui_widgets::Activate};
use nova_assets::prelude::{ContentCatalogDigest, LoadedSectionPacks};
use nova_gameplay::prelude::*;
use nova_world_base::{
    prelude::{create_world, ResumedWorld, WorldRefusal, WorldSaveSession, WorldSaveStatus},
    test_support::{arm_save_fixture, WorldSaveTestPlugin},
};

use super::support::{app, dummy_scenarios};
use crate::{
    load_screen::{SelectedWorldSlug, WorldChecks},
    scenarios::NewGameScenario,
    world_setup::WorldsRoot,
};

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

/// Run `app` until [`WorldChecks`] empties, or panic after a generous number
/// of frames - the checks run off the main thread on the `IoTaskPool`, so
/// more than one frame can pass before they land.
fn checks_settle(app: &mut App) {
    for _ in 0..200 {
        app.update();
        if app.world().resource::<WorldChecks>().0.is_empty() {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("the world checks did not settle in time");
}

/// Whether any entity in `app` is named `name`.
fn has_named(app: &mut App, name: &str) -> bool {
    let mut names = app.world_mut().query::<&Name>();
    names.iter(app.world()).any(|found| found.as_str() == name)
}

/// The text of the one [`Text`] entity named `name`.
fn text_of(app: &mut App, name: &str) -> String {
    let entity = named(app, name);
    app.world()
        .get::<Text>(entity)
        .expect("a Text component")
        .0
        .clone()
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
    checks_settle(&mut app);

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

/// Load greys a world while its saved state is checked, then refuses one
/// whose state holds an item the loaded catalog lacks, with the reason, and
/// never starts it.
#[test]
fn load_greys_a_world_while_its_check_runs_and_refuses_one_holding_an_unknown_item() {
    let mut app = menu();
    let root = tempfile::tempdir().expect("a scratch worlds root");
    app.insert_resource(WorldsRoot(Some(root.path().to_path_buf())));

    app.add_plugins(WorldSaveTestPlugin);
    let player = arm_save_fixture(app.world_mut());
    // The fixture player holds no stock; give it one item so the loaded
    // catalog can go on to lack it.
    let hull_plate = ItemDesignId::from(ITEM_HULL_PLATE);
    let items = app.world().resource::<GameItems>().clone();
    let mut inventory = ShipInventory::new(&items, 10_000, std::iter::empty());
    inventory.add(&items, &hull_plate, 1);
    app.world_mut().entity_mut(player).insert(inventory);

    let (folder, lock) =
        create_world(root.path(), "Holder World").expect("an empty root takes a new world");
    app.world_mut().insert_resource(WorldSaveSession::created(
        folder,
        lock,
        "Holder World".to_string(),
        42,
    ));
    save_until_idle(&mut app);
    app.world_mut().remove_resource::<WorldSaveSession>();

    // The catalog the Load list checks against now lacks HullPlate.
    app.insert_resource(GameItems::new(
        nova_gameplay::test_support::test_items()
            .iter()
            .filter(|design| design.id != hull_plate)
            .cloned(),
    ));

    // Part A: a check still running greys the row and the Load button, and
    // Load does nothing while it is pending. The override replaces the real
    // check BEFORE any `app.update()` runs, so it cannot race `poll_world_checks`
    // finding the real one first.
    let load_screen_button = named(&mut app, "Load Button");
    app.world_mut().trigger(Activate {
        entity: load_screen_button,
    });
    app.world_mut().resource_mut::<WorldChecks>().0.insert(
        "holder-world".to_string(),
        IoTaskPool::get().spawn(future::pending::<Result<(), WorldRefusal>>()),
    );
    app.update();
    let row = named(&mut app, "Load World Row: holder-world");
    app.world_mut().trigger(Activate { entity: row });
    app.update();
    assert!(
        has_named(&mut app, "Load World Row Checking"),
        "a world still being checked shows the Checking label on its row"
    );
    let load_button = named(&mut app, "Load World Button");
    assert!(
        app.world()
            .entity(load_button)
            .contains::<InteractionDisabled>(),
        "a world still being checked greys the Load button"
    );
    press(&mut app, "Load World Button");
    assert_eq!(
        state(&app),
        GameStates::MainMenu,
        "Load does nothing while the check is pending"
    );
    assert!(
        app.world().get_resource::<ResumedWorld>().is_none(),
        "Load does not start a world whose check is still pending"
    );

    // Part B: the real check lands and refuses the unknown item; the row and
    // details show the reason and Load stays greyed.
    press(&mut app, "Load Button");
    checks_settle(&mut app);
    let row = named(&mut app, "Load World Row: holder-world");
    app.world_mut().trigger(Activate { entity: row });
    app.update();
    let reason = "unknown item 'HullPlate' held by the player ship";
    assert!(
        text_of(&mut app, "Load World Row Refusal").contains(reason),
        "the row names the unknown item"
    );
    assert!(
        text_of(&mut app, "Load World Details Refusal").contains(reason),
        "the details pane names the unknown item"
    );
    let load_button = named(&mut app, "Load World Button");
    assert!(
        app.world()
            .entity(load_button)
            .contains::<InteractionDisabled>(),
        "a refused world's Load button stays greyed"
    );
    assert!(
        !has_named(&mut app, "Load World Row Checking"),
        "a settled check shows no Checking label"
    );

    // Part C: the catalog has the item again, so Load is no longer refused.
    app.insert_resource(nova_gameplay::test_support::test_items());
    press(&mut app, "Load Button");
    checks_settle(&mut app);
    let row = named(&mut app, "Load World Row: holder-world");
    app.world_mut().trigger(Activate { entity: row });
    app.update();
    let load_button = named(&mut app, "Load World Button");
    assert!(
        !app.world()
            .entity(load_button)
            .contains::<InteractionDisabled>(),
        "once the catalog has the item again, Load is not disabled"
    );

    // Part D: the catalog loses the item again after the list settled. Load
    // checks again as it opens, so the stale enabled row does not start the
    // world and shows the refusal instead.
    app.insert_resource(GameItems::new(
        nova_gameplay::test_support::test_items()
            .iter()
            .filter(|design| design.id != hull_plate)
            .cloned(),
    ));
    press(&mut app, "Load World Button");
    assert_eq!(
        state(&app),
        GameStates::MainMenu,
        "Load refuses a world whose item left the catalog after the check"
    );
    assert!(
        app.world().get_resource::<ResumedWorld>().is_none(),
        "Load does not start a world whose item left the catalog after the check"
    );
    assert!(
        text_of(&mut app, "Load World Row Refusal").contains(reason),
        "the row names the unknown item after a refused Load"
    );
    assert!(
        text_of(&mut app, "Load World Details Refusal").contains(reason),
        "the details pane names the unknown item after a refused Load"
    );
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
