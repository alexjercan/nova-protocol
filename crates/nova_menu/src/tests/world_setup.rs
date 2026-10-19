//! The New Game modal: what a seed and a world name are, and what Next,
//! Create, Cancel and Randomize each do to the menu, the session, the worlds
//! folder and the state.

use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use nova_gameplay::prelude::*;
use nova_interface::prelude::InterfaceIcons;
use nova_scenario::prelude::{GameStartRoles, OpenWorldSession};
use nova_ui::widget::{TextFieldError, TextFieldValue};
use nova_world_base::{
    prelude::{create_world, start_session, WorldSaveSession},
    test_support::{fixture_loaded, fixture_styles},
};

use super::support::{
    app, dummy_scenarios, observe_load_scenario, LoadedScenario, TEST_ROLE_IDS, TEST_START_ID,
};
use crate::{
    preview::{MenuChartPreview, MenuHullPreview},
    scenarios::NewGameScenario,
    world_setup::{parse_world_seed, WorldSeedField, WorldSetupOverlay, WorldsRoot},
};

/// A menu app entered the real way, so the menu panel is built and the
/// menu-gated systems run.
fn menu() -> App {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();
    // After entry, so the menu's own backdrop load is not counted as a start.
    observe_load_scenario(&mut app);
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

fn overlays(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<(), With<WorldSetupOverlay>>()
        .iter(app.world())
        .count()
}

fn seed_text(app: &mut App) -> String {
    app.world_mut()
        .query_filtered::<&TextFieldValue, With<WorldSeedField>>()
        .single(app.world())
        .expect("the modal holds one seed field")
        .0
        .clone()
}

fn type_into(app: &mut App, field: &str, text: &str) {
    let field = named(app, field);
    app.world_mut()
        .entity_mut(field)
        .insert(TextFieldValue(text.to_string()));
    app.update();
}

fn type_seed(app: &mut App, text: &str) {
    type_into(app, "World Seed Field", text);
}

fn type_name(app: &mut App, text: &str) {
    type_into(app, "World Name Field", text);
}

/// Point the menu's worlds folder at a scratch one.
fn worlds_at(app: &mut App) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    app.insert_resource(WorldsRoot(Some(root.path().to_path_buf())));
    root
}

fn state(app: &App) -> GameStates {
    app.world().resource::<State<GameStates>>().get().clone()
}

#[test]
fn a_world_seed_is_decimal_digits_inside_u32() {
    assert_eq!(parse_world_seed("0"), Some(0));
    assert_eq!(parse_world_seed("4294967295"), Some(u32::MAX));
    assert_eq!(parse_world_seed(" 42 "), Some(42));
    for refused in ["", "   ", "4294967296", "-1", "+1", "12a", "1_000", "0x10"] {
        assert_eq!(parse_world_seed(refused), None, "'{refused}' is not a seed");
    }
}

/// New Game does not leave the menu: it opens the modal on a seed drawn
/// fresh each time it opens.
#[test]
fn new_game_opens_the_modal_with_a_fresh_seed_each_time() {
    let mut app = menu();

    press(&mut app, "New Game Button");
    assert_eq!(overlays(&mut app), 1);
    assert_ne!(
        state(&app),
        GameStates::Playing,
        "opening the modal starts nothing"
    );
    let first = seed_text(&mut app);
    assert!(
        parse_world_seed(&first).is_some(),
        "'{first}' is not a seed"
    );

    press(&mut app, "Cancel New Game Button");
    press(&mut app, "New Game Button");
    let second = seed_text(&mut app);
    assert_ne!(first, second, "a reopened modal drew no new seed");
}

#[test]
fn randomize_writes_a_new_seed_into_the_field() {
    let mut app = menu();
    press(&mut app, "New Game Button");
    let before = seed_text(&mut app);

    press(&mut app, "Randomize Seed Button");

    let after = seed_text(&mut app);
    assert_ne!(before, after);
    assert!(
        parse_world_seed(&after).is_some(),
        "'{after}' is not a seed"
    );
}

#[test]
fn a_bad_seed_is_refused_inline_and_greys_next_until_it_is_fixed() {
    let mut app = menu();
    press(&mut app, "New Game Button");
    let field = named(&mut app, "World Seed Field");
    let next = named(&mut app, "World Next Button");

    type_seed(&mut app, "4294967296");
    assert!(app.world().entity(field).contains::<TextFieldError>());
    assert!(app.world().entity(next).contains::<InteractionDisabled>());

    type_seed(&mut app, "7");
    assert!(!app.world().entity(field).contains::<TextFieldError>());
    assert!(!app.world().entity(next).contains::<InteractionDisabled>());
}

/// Create is the one way into the open world: the typed name becomes the
/// world's folder and its saving session, the typed seed and the selected
/// start role the session, the picker's override is cleared, and the
/// declared start loads.
#[test]
fn create_starts_the_declared_new_game_on_the_typed_seed_as_the_selected_role() {
    let mut app = menu();
    let root = worlds_at(&mut app);
    app.insert_resource(NewGameScenario(Some("practice_run".to_string())));
    press(&mut app, "New Game Button");
    type_name(&mut app, "Long Haul");
    type_seed(&mut app, "12345");
    press(&mut app, "World Next Button");
    press(
        &mut app,
        &format!("Start Role Button: {}", TEST_ROLE_IDS[1]),
    );

    press(&mut app, "Create World Button");

    let session = app.world().resource::<WorldSaveSession>();
    assert_eq!(session.name(), "Long Haul");
    assert_eq!(session.folder().path, root.path().join("long-haul"));
    assert!(root.path().join("long-haul/world.lock").exists());

    let expected = start_session(
        12_345,
        TEST_ROLE_IDS[1].into(),
        &fixture_loaded(),
        &fixture_styles(),
        app.world().resource::<GameItems>(),
    )
    .expect("the fixture seed has a living ally station near the origin");
    assert_eq!(
        app.world().get_resource::<OpenWorldSession>(),
        Some(&expected),
        "Create inserts the seed, the role and the seed's starting station"
    );
    assert_eq!(app.world().resource::<NewGameScenario>().0, None);
    assert_eq!(*app.world().resource::<GameMode>(), GameMode::NewGame);
    assert_eq!(state(&app), GameStates::Playing);
    assert_eq!(
        app.world().resource::<LoadedScenario>().0.as_deref(),
        Some(TEST_START_ID)
    );
    assert_eq!(overlays(&mut app), 0);
}

/// A player who types no name still gets a world: the field opens on a
/// default name, and Create saves the world under it.
#[test]
fn create_on_the_untouched_name_saves_the_world_as_new_world() {
    let mut app = menu();
    let root = worlds_at(&mut app);
    press(&mut app, "New Game Button");
    press(&mut app, "World Next Button");

    press(&mut app, "Create World Button");

    let session = app.world().resource::<WorldSaveSession>();
    assert_eq!(session.name(), "New World");
    assert_eq!(session.folder().path, root.path().join("new-world"));
    assert_eq!(state(&app), GameStates::Playing);
}

#[test]
fn cancel_closes_only_the_modal() {
    let mut app = menu();
    let before = state(&app);
    press(&mut app, "New Game Button");

    press(&mut app, "Cancel New Game Button");

    assert_eq!(overlays(&mut app), 0);
    assert_eq!(state(&app), before);
    assert!(app.world().get_resource::<OpenWorldSession>().is_none());
    assert_eq!(app.world().resource::<LoadedScenario>().0, None);
}

/// A name whose folder exists is refused under the field, back on the world
/// step: nothing starts, and the existing world's folder is left as it was.
#[test]
fn create_refuses_a_taken_world_name_inline_and_starts_nothing() {
    let mut app = menu();
    let root = worlds_at(&mut app);
    drop(create_world(root.path(), "Taken").unwrap());
    press(&mut app, "New Game Button");
    type_name(&mut app, "taken");
    type_seed(&mut app, "7");
    press(&mut app, "World Next Button");

    press(&mut app, "Create World Button");

    for (step, display) in [("World Step", Display::Flex), ("Role Step", Display::None)] {
        let step = named(&mut app, step);
        assert_eq!(app.world().get::<Node>(step).unwrap().display, display);
    }
    let field = named(&mut app, "World Name Field");
    let refusal = app
        .world()
        .get::<TextFieldError>(field)
        .expect("the name is refused under the field");
    assert!(refusal.0.contains("exists"), "{}", refusal.0);
    assert_ne!(state(&app), GameStates::Playing);
    assert!(app.world().get_resource::<WorldSaveSession>().is_none());
    assert!(app.world().get_resource::<OpenWorldSession>().is_none());
    assert_eq!(overlays(&mut app), 1, "the modal stays open");
    let files: Vec<_> = std::fs::read_dir(root.path().join("taken"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(files, ["world.lock"]);
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

/// The value texts of every `detail_row` named `name`, in spawn order.
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

/// Step 1 draws the typed seed's chart: the cells round its starting
/// station with the station marked and the start inside its cell, and a
/// caption counting the stations in reach; a seed with no start draws an
/// empty chart and says why. Step 2 draws each role row with its role's
/// icon, lists the role's milestones as the game states them, and shows no
/// hull view when the catalog lacks the hull.
#[test]
fn new_game_previews_the_seed_s_start_and_lists_the_role_s_milestones() {
    let mut app = menu();
    press(&mut app, "New Game Button");
    type_seed(&mut app, "42");
    app.update();

    let preview = named(&mut app, "Seed Preview");
    let chart = app
        .world()
        .get::<MenuChartPreview>(preview)
        .expect("the seed preview carries the seed's chart");
    let stations = chart
        .0
        .marks
        .iter()
        .filter(|mark| {
            matches!(
                mark.kind,
                nova_hud::prelude::ChartSceneMarkType::Station { .. }
            )
        })
        .count();
    assert!(stations >= 1, "seed 42 has a station in reach");
    assert!(!chart.0.cells.is_empty());
    assert!(chart.0.you.is_some(), "the start sits inside its cell");
    let plural = if stations == 1 { "station" } else { "stations" };
    assert_eq!(
        text_of(&mut app, "Seed Preview Caption"),
        format!(
            "The sectors round the start of seed 42: {stations} {plural} in reach. \
             Right-drag turns, wheel zooms."
        )
    );

    press(&mut app, "World Next Button");
    let roles = app.world().resource::<GameStartRoles>().clone();
    let mut drawn = Vec::new();
    for role in roles.iter() {
        let row = named(&mut app, &format!("Start Role Button: {}", role.id));
        let icon = app
            .world()
            .get::<Children>(row)
            .expect("a role row has children")[0];
        let image = app
            .world()
            .get::<ImageNode>(icon)
            .expect("a role row leads with its icon")
            .image
            .clone();
        assert_eq!(
            image,
            app.world().resource::<InterfaceIcons>().role(role.icon),
            "role '{}' draws its own icon",
            role.id
        );
        drawn.push(image);
    }
    assert_ne!(
        drawn[0], drawn[1],
        "two roles with two icons draw two masks"
    );

    press(
        &mut app,
        &format!("Start Role Button: {}", TEST_ROLE_IDS[0]),
    );
    assert_eq!(
        row_values(&mut app, "Start Role Details Milestone"),
        vec!["1. A goal".to_string()]
    );
    let mut hulls = app.world_mut().query::<&MenuHullPreview>();
    assert_eq!(
        hulls.iter(app.world()).count(),
        0,
        "the fixture catalog has no design for the role's hull"
    );
}
