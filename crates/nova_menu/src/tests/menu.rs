//! The main menu buttons: that an activation only counts as a click when it
//! really was one, and that New Game and Sandbox set the mode, resolve the
//! scenario override and fall back past a bad declaration.

use bevy::{
    prelude::*,
    ui_widgets::{observe, Activate},
};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::support::{
    app, cue_app, dummy_scenarios, enter_playing, observe_load_scenario, observe_unload_scenario,
    LoadedScenario, PlayedCues, Unloaded, TEST_BACKDROP_ID, TEST_START_ID,
};
use crate::{
    menu_ui::{on_new_game, on_sandbox, setup_menu_ui},
    scenarios::NewGameScenario,
    widgets::{button, on_menu_button_activate},
};

#[test]
fn a_menu_button_activation_clicks_and_a_bare_activation_does_not() {
    let mut app = cue_app();
    app.add_observer(on_menu_button_activate);
    // A real menu button (carries MenuSfxButton via `button()`) and a bare
    // entity that merely gets Activate'd.
    let menu_button = app.world_mut().spawn(button("New Game")).id();
    let bare = app.world_mut().spawn_empty().id();
    app.update();

    app.world_mut().trigger(Activate { entity: bare });
    app.update();
    assert_eq!(
        app.world().resource::<PlayedCues>().0,
        0,
        "a non-MenuSfxButton activation is silent"
    );

    app.world_mut().trigger(Activate {
        entity: menu_button,
    });
    app.update();
    assert_eq!(
        app.world().resource::<PlayedCues>().0,
        1,
        "pressing a menu button clicks once"
    );
}

#[test]
fn new_game_button_sets_mode_and_hands_off_to_playing() {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    observe_load_scenario(&mut app);
    let button = app.world_mut().spawn(observe(on_new_game)).id();
    app.update();

    app.world_mut().trigger(Activate { entity: button });
    app.update();

    assert_eq!(*app.world().resource::<GameMode>(), GameMode::NewGame);
    assert_eq!(
        *app.world().resource::<State<GameStates>>().get(),
        GameStates::Playing
    );
    // Delivery guard: the handoff must actually load the scenario, not just
    // flip states.
    assert_eq!(
        app.world().resource::<LoadedScenario>().0.as_deref(),
        Some(TEST_START_ID)
    );
}

#[test]
fn sandbox_button_sets_mode_and_loads_no_scenario() {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    let button = app.world_mut().spawn(observe(on_sandbox)).id();
    // Enter the menu first (the real path), so leaving it exercises the
    // uniform OnExit teardown.
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();
    // Observers registered after entry, so the menu's own ambience load
    // does not count against the "loads nothing" assertion below.
    observe_load_scenario(&mut app);
    observe_unload_scenario(&mut app);

    app.world_mut().trigger(Activate { entity: button });
    app.update();

    assert_eq!(*app.world().resource::<GameMode>(), GameMode::Sandbox);
    assert_eq!(
        *app.world().resource::<State<GameStates>>().get(),
        GameStates::Playing
    );
    // The editor owns the Sandbox path; the menu must not load anything,
    // and it must tear the ambience backdrop down (the editor does not).
    assert_eq!(app.world().resource::<LoadedScenario>().0, None);
    assert!(app.world().resource::<Unloaded>().0);
    // Leaving the menu restores the HUD level.
    assert_eq!(*app.world().resource::<HudVisibility>(), HudVisibility::On);
}

/// Review R1.3: exercise the REAL New Game button, not just the handler fn.
/// Builds the actual menu UI headless, finds the button by Name, and clicks
/// it - so dropping the observe(on_new_game) wiring from setup_menu_ui fails
/// this test.
#[test]
fn real_new_game_button_is_wired() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = app();
    app.insert_resource(dummy_scenarios());
    observe_load_scenario(&mut app);
    app.world_mut()
        .run_system_once(setup_menu_ui)
        .expect("setup_menu_ui runs headless");
    app.update();

    let button = {
        let mut names = app.world_mut().query::<(Entity, &Name)>();
        names
            .iter(app.world())
            .find(|(_, name)| name.as_str() == "New Game Button")
            .map(|(entity, _)| entity)
            .expect("the menu spawns a 'New Game Button'")
    };
    app.world_mut().trigger(Activate { entity: button });
    app.update();

    assert_eq!(*app.world().resource::<GameMode>(), GameMode::NewGame);
    assert_eq!(
        *app.world().resource::<State<GameStates>>().get(),
        GameStates::Playing
    );
    assert_eq!(
        app.world().resource::<LoadedScenario>().0.as_deref(),
        Some(TEST_START_ID)
    );
}

/// `start_new_game_scenario` reads the override: `Some(existing)` loads that
/// scenario, `None` loads the canned start, and `Some(missing)` falls back
/// to the canned start rather than panicking.
#[test]
fn start_new_game_scenario_honors_the_override_and_falls_back() {
    let loaded_for = |pick: Option<&str>| -> Option<String> {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        observe_load_scenario(&mut app);
        *app.world_mut().resource_mut::<GameMode>() = GameMode::NewGame;
        app.insert_resource(NewGameScenario(pick.map(|s| s.to_string())));
        enter_playing(&mut app);
        app.world().resource::<LoadedScenario>().0.clone()
    };

    assert_eq!(
        loaded_for(Some(TEST_BACKDROP_ID)).as_deref(),
        Some(TEST_BACKDROP_ID),
        "an existing override loads that scenario"
    );
    assert_eq!(
        loaded_for(None).as_deref(),
        Some(TEST_START_ID),
        "no override loads the canned New Game start"
    );
    assert_eq!(
        loaded_for(Some("no-such-scenario")).as_deref(),
        Some(TEST_START_ID),
        "a missing override falls back to the canned start"
    );
}

/// The rest of the New Game fallback chain: a missing or absent base declaration falls
/// back to the first LISTED scenario, and an empty registry loads nothing without
/// panicking.
#[test]
fn start_new_game_scenario_falls_back_past_a_bad_declaration() {
    let loaded_with = |start: Option<&str>, scenarios: GameScenarios| -> Option<String> {
        let mut app = app();
        app.insert_resource(scenarios);
        app.insert_resource(NewGameStart(start.map(|s| s.to_string())));
        observe_load_scenario(&mut app);
        *app.world_mut().resource_mut::<GameMode>() = GameMode::NewGame;
        app.insert_resource(NewGameScenario(None));
        enter_playing(&mut app);
        app.world().resource::<LoadedScenario>().0.clone()
    };

    // Both dummy fixtures share the display name, so the LISTED order
    // tiebreaks on id: TEST_START_ID ("story_start") sorts first.
    assert_eq!(
        loaded_with(Some("gone-with-a-mod"), dummy_scenarios()).as_deref(),
        Some(TEST_START_ID),
        "an unregistered base declaration falls back to the first listed scenario"
    );
    assert_eq!(
        loaded_with(None, dummy_scenarios()).as_deref(),
        Some(TEST_START_ID),
        "no declaration at all falls back to the first listed scenario"
    );
    assert_eq!(
        loaded_with(Some("gone"), GameScenarios::default()),
        None,
        "an empty registry loads nothing - and must not panic"
    );
}

/// New Game clears any override the picker left, so it always starts the
/// main story even after the player used the Scenarios picker.
#[test]
fn new_game_button_clears_the_scenario_override() {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    app.insert_resource(NewGameScenario(Some("practice_run".to_string())));
    let button = app.world_mut().spawn(observe(on_new_game)).id();
    app.update();

    app.world_mut().trigger(Activate { entity: button });
    app.update();

    assert_eq!(
        app.world().resource::<NewGameScenario>().0,
        None,
        "New Game resets the picker override to None"
    );
}
