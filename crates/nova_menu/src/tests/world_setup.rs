//! The New Game modal: what a seed is, and what Create, Cancel and Randomize
//! each do to the menu, the session and the state.

use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use nova_gameplay::prelude::*;
use nova_ui::widget::{TextFieldError, TextFieldValue};
use nova_world_base::prelude::OpenWorldSession;

use super::support::{app, dummy_scenarios, observe_load_scenario, LoadedScenario, TEST_START_ID};
use crate::{
    scenarios::NewGameScenario,
    world_setup::{parse_world_seed, WorldSeedField, WorldSetupOverlay},
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

fn type_seed(app: &mut App, text: &str) {
    let field = named(app, "World Seed Field");
    app.world_mut()
        .entity_mut(field)
        .insert(TextFieldValue(text.to_string()));
    app.update();
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
fn a_bad_seed_is_refused_inline_and_greys_create_until_it_is_fixed() {
    let mut app = menu();
    press(&mut app, "New Game Button");
    let field = named(&mut app, "World Seed Field");
    let create = named(&mut app, "Create World Button");

    type_seed(&mut app, "4294967296");
    assert!(app.world().entity(field).contains::<TextFieldError>());
    assert!(app.world().entity(create).contains::<InteractionDisabled>());

    type_seed(&mut app, "7");
    assert!(!app.world().entity(field).contains::<TextFieldError>());
    assert!(!app.world().entity(create).contains::<InteractionDisabled>());
}

/// Create is the one way into the open world: the typed seed becomes the
/// session, the picker's override is cleared, and the declared start loads.
#[test]
fn create_starts_the_declared_new_game_on_the_typed_seed() {
    let mut app = menu();
    app.insert_resource(NewGameScenario(Some("practice_run".to_string())));
    press(&mut app, "New Game Button");
    type_seed(&mut app, "12345");

    press(&mut app, "Create World Button");

    assert_eq!(
        app.world().get_resource::<OpenWorldSession>(),
        Some(&OpenWorldSession { seed: 12_345 })
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
