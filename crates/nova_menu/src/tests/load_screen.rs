//! The Load picker: a saved world lists and loads, a refused one lists and
//! refuses to load.

use std::{future, path::Path, thread, time::Duration};

use bevy::{prelude::*, tasks::IoTaskPool, ui::InteractionDisabled, ui_widgets::Activate};
use nova_assets::prelude::{ContentCatalogDigest, LoadedSectionPacks};
use nova_events::prelude::SectorCoord;
use nova_gameplay::prelude::*;
use nova_interface::prelude::InterfaceIcons;
use nova_scenario::prelude::{GameStartRoles, OpenWorldSession};
use nova_world_base::{
    prelude::{create_world, ResumedWorld, WorldRefusal, WorldSaveSession, WorldSaveStatus},
    test_support::{arm_save_fixture, WorldSaveTestPlugin, FIXTURE_ROLE_ID, FIXTURE_SCENARIO_ID},
};

use super::support::{app, dummy_scenarios, dummy_start};
use crate::{
    load_screen::{SelectedWorldSlug, WorldChecks, WorldOpen},
    preview::MenuChartPreview,
    scenarios::NewGameScenario,
    world_setup::WorldsRoot,
};

/// A menu app entered the real way, with the Load picker's plugin wiring in
/// place.
fn menu() -> App {
    let mut app = app();
    // The fixture saves under its own scenario id; a Load looks it up.
    let mut scenarios = dummy_scenarios();
    scenarios.0.extend([dummy_start(FIXTURE_SCENARIO_ID)]);
    app.insert_resource(scenarios);
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
    let session = app.world().resource::<WorldSaveSession>();
    panic!(
        "the fixture world did not finish saving in time: idle {}, status {:?}, state {:?}",
        session.is_idle(),
        session.status(),
        state(app)
    );
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

/// Press the Load World button and run `app` until its [`WorldOpen`] lands
/// and the frame after it applies the state it asked for, or panic after a
/// generous number of frames - the open runs off the main thread on the
/// `IoTaskPool`, so more than one frame can pass before the world resumes or
/// the refusal shows.
fn press_load_world(app: &mut App) {
    press(app, "Load World Button");
    for _ in 0..200 {
        if app.world().resource::<WorldOpen>().0.is_none() {
            app.update();
            return;
        }
        thread::sleep(Duration::from_millis(5));
        app.update();
    }
    panic!("the world open did not land in time");
}

/// Whether any entity in `app` is named `name`.
fn has_named(app: &mut App, name: &str) -> bool {
    let mut names = app.world_mut().query::<&Name>();
    names.iter(app.world()).any(|found| found.as_str() == name)
}

/// The value texts of every `detail_row` named `name`, in spawn order: the
/// last [`Text`] child of each.
fn row_values(app: &mut App, name: &str) -> Vec<String> {
    let mut rows = app.world_mut().query::<(&Name, &Children)>();
    let children: Vec<Vec<Entity>> = rows
        .iter(app.world())
        .filter(|(found, _)| found.as_str() == name)
        .map(|(_, children)| children.iter().collect())
        .collect();
    children
        .iter()
        .map(|children| {
            children
                .iter()
                .rev()
                .find_map(|child| app.world().get::<Text>(*child).map(|text| text.0.clone()))
                .expect("a detail row holds a value text")
        })
        .collect()
}

/// Save the fixture world `name` under `root` through the production
/// pipeline, as a player flying "Fixture hull" for 90 s with one trade goal
/// open, and release its lock so Load can take it.
fn save_fixture_world(app: &mut App, root: &Path, name: &str) {
    if !app.is_plugin_added::<WorldSaveTestPlugin>() {
        app.add_plugins(WorldSaveTestPlugin);
    }
    let player = arm_save_fixture(app.world_mut());
    app.world_mut()
        .entity_mut(player)
        .insert(Name::new("Fixture hull"));
    app.world_mut().resource_mut::<WorldClock>().play_seconds = 90.0;
    let start = app.world().resource::<OpenWorldSession>().start.id.clone();
    {
        let mut chart = app.world_mut().resource_mut::<Chart>();
        chart.visit(SectorCoord::ORIGIN, vec![ClusterType::RockOnly], 10.0);
        chart.find_station(&start, false, false, None, 45.0);
    }
    app.world_mut().insert_resource(GoalProgress {
        title: "A station network".to_string(),
        lines: vec![GoalLine::new("Trade jobs", 1, 3)],
    });
    let (folder, lock) = create_world(root, name).expect("the root takes a new world");
    app.world_mut().insert_resource(WorldSaveSession::created(
        folder,
        lock,
        name.to_string(),
        42,
        FIXTURE_ROLE_ID.into(),
    ));
    save_until_idle(app);
    app.world_mut().remove_resource::<WorldSaveSession>();
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
        FIXTURE_ROLE_ID.into(),
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

    press_load_world(&mut app);

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
    let hull_plate = ItemDesignId::from(ItemRoleType::HullPlate);
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
        FIXTURE_ROLE_ID.into(),
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
        has_named(&mut app, "Load World Row Invalid"),
        "the row wears the Invalid badge"
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
    press_load_world(&mut app);
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
        has_named(&mut app, "Load World Row Invalid"),
        "the row wears the Invalid badge after a refused Load"
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

/// A world's row draws its start role's icon and reads its play time, how
/// long ago it was saved and how many objectives it holds open; its details
/// draw the save's chart, name the hull in a FLEET block that lists only the
/// fleet counts the save holds, and list the open objectives with their
/// counts. A role a mod reload dropped keeps the icon slot empty and the
/// row names the role by id.
#[test]
fn a_load_row_reads_play_time_save_age_and_open_count_and_its_details_the_fleet_and_objectives() {
    let mut app = menu();
    let root = tempfile::tempdir().expect("a scratch worlds root");
    app.insert_resource(WorldsRoot(Some(root.path().to_path_buf())));
    save_fixture_world(&mut app, root.path(), "Good World");

    press(&mut app, "Load Button");
    checks_settle(&mut app);

    assert_eq!(text_of(&mut app, "Load World Row Play Time"), "0:01:30");
    assert_eq!(text_of(&mut app, "Load World Row Saved"), "just now");
    assert_eq!(text_of(&mut app, "Load World Row Open"), "1 open");
    let role_icon = app
        .world()
        .resource::<GameStartRoles>()
        .get_role(&FIXTURE_ROLE_ID.into())
        .expect("the fixture save's role is loaded")
        .icon;
    let icon = named(&mut app, "Load World Row Icon");
    assert_eq!(
        app.world()
            .get::<ImageNode>(icon)
            .expect("the row draws its role icon")
            .image,
        app.world().resource::<InterfaceIcons>().role(role_icon)
    );

    let row = named(&mut app, "Load World Row: good-world");
    app.world_mut().trigger(Activate { entity: row });
    app.update();

    let preview = named(&mut app, "Load World Details Preview");
    let chart = app
        .world()
        .get::<MenuChartPreview>(preview)
        .expect("the details preview carries the save's chart")
        .0
        .clone();
    assert!(
        chart.marks.iter().any(|mark| matches!(
            mark.kind,
            nova_hud::prelude::ChartSceneMarkType::Station { .. }
        )),
        "the saved chart marks the starting station: {:?}",
        chart.marks
    );
    assert_eq!(
        row_values(&mut app, "Load World Details Line"),
        vec![
            "42".to_string(),
            chart.centre.to_string(),
            "300 cr".to_string(),
            "0:01:30".to_string(),
            format!("{} (just now)", text_of_saved(&mut app)),
            nova_info::prelude::APP_VERSION.to_string(),
        ]
    );
    assert_eq!(
        row_values(&mut app, "Load World Details Fleet Ship"),
        vec!["Fixture hull".to_string()]
    );
    assert!(!has_named(&mut app, "Load World Details Fleet Owned Ships"));
    assert!(!has_named(&mut app, "Load World Details Fleet Escorts"));
    assert!(!has_named(
        &mut app,
        "Load World Details Fleet Owned Stations"
    ));
    assert_eq!(
        text_of(&mut app, "Load World Details Objective Title"),
        "A station network"
    );
    assert_eq!(
        text_of(&mut app, "Load World Details Objective Counts"),
        "Trade jobs 1 / 3"
    );
    assert!(!has_named(&mut app, "Load World Details No Objectives"));

    app.world_mut().resource_mut::<GameStartRoles>().0.clear();
    app.world_mut()
        .resource_mut::<crate::load_screen::WorldListings>()
        .set_changed();
    app.update();
    let slot = named(&mut app, "Load World Row Icon Slot");
    assert!(
        app.world().get::<Children>(slot).is_none(),
        "a dropped role draws no icon"
    );
    assert_eq!(text_of(&mut app, "Load World Row Status"), FIXTURE_ROLE_ID);
}

/// The `Saved` line's UTC stamp, read back from the selected world's header
/// so the test does not re-derive the save's clock.
fn text_of_saved(app: &mut App) -> String {
    let listings = app.world().resource::<crate::load_screen::WorldListings>();
    let header = listings
        .0
        .as_ref()
        .expect("the root listed")
        .iter()
        .find_map(|listing| listing.header.as_ref().ok())
        .expect("one readable save");
    crate::load_screen::utc_stamp(header.saved_at_unix)
}

/// The menu heads its buttons with Continue when the worlds folder holds a
/// readable save, naming the newest one and its age, and Continue resumes
/// that world exactly like Load does; with no save there is no Continue.
#[test]
fn continue_names_the_newest_save_and_resumes_it() {
    let mut empty = menu();
    assert!(
        !has_named(&mut empty, "Continue Button"),
        "an empty worlds folder offers no Continue"
    );
    drop(empty);

    let root = tempfile::tempdir().expect("a scratch worlds root");
    // The fixture arms one player per app, so the older save comes from an
    // app of its own.
    let mut older = menu();
    older.insert_resource(WorldsRoot(Some(root.path().to_path_buf())));
    save_fixture_world(&mut older, root.path(), "Older World");
    drop(older);
    thread::sleep(Duration::from_millis(1_100));
    let mut app = menu();
    app.insert_resource(WorldsRoot(Some(root.path().to_path_buf())));
    save_fixture_world(&mut app, root.path(), "Newest World");
    // The menu panel is built on entry, so leave and re-enter the menu the
    // way a player coming back from a world does.
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::Loading);
    app.update();
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();

    assert_eq!(
        text_of(&mut app, "Continue Caption"),
        "Newest World, saved just now"
    );
    press(&mut app, "Continue Button");
    for _ in 0..200 {
        if app.world().resource::<WorldOpen>().0.is_none() {
            break;
        }
        thread::sleep(Duration::from_millis(5));
        app.update();
    }
    app.update();

    assert_eq!(
        app.world().resource::<WorldSaveSession>().name(),
        "Newest World"
    );
    assert_eq!(
        app.world().resource::<SelectedWorldSlug>().0.as_deref(),
        Some("newest-world")
    );
    assert!(app.world().get_resource::<ResumedWorld>().is_some());
    assert_eq!(state(&app), GameStates::Playing);
}
