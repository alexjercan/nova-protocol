//! The main menu buttons: that an activation only counts as a click when it
//! really was one, that Sandbox sets the mode, and that the New Game start
//! loads the pick or the session-seeded start and refuses everything else.

use bevy::{
    prelude::*,
    ui_widgets::{observe, Activate},
};
use nova_gameplay::prelude::*;
use nova_hud::prelude::HudVisibility;
use nova_scenario::prelude::*;

use super::support::{
    app, cue_app, dummy_scenario, dummy_scenarios, dummy_start, enter_playing,
    observe_load_scenario, observe_unload_scenario, take_cues, LoadedScenario, PlayedCues,
    Unloaded, TEST_BACKDROP_ID, TEST_ROLE_IDS, TEST_START_ID,
};
use crate::{
    menu_ui::{on_sandbox, resolve_new_game_scenario, NewGameStartFault},
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

/// `start_new_game_scenario` loads the pick when one is set, else the
/// session-seeded start over the session Create or Load inserted. With no
/// session the start is refused: nothing loads and the app exits with an
/// error, the same refusal `--scenario` gives a session-seeded id.
#[test]
fn start_new_game_scenario_loads_the_pick_or_the_session_seeded_start() {
    let started = |pick: Option<&str>, session: bool| -> (Option<String>, Vec<AppExit>) {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        observe_load_scenario(&mut app);
        *app.world_mut().resource_mut::<GameMode>() = GameMode::NewGame;
        app.insert_resource(NewGameScenario(pick.map(|s| s.to_string())));
        if session {
            app.insert_resource(OpenWorldSession {
                seed: 7,
                role: TEST_ROLE_IDS[0].into(),
            });
        }
        enter_playing(&mut app);
        let loaded = app.world().resource::<LoadedScenario>().0.clone();
        let exits = app
            .world_mut()
            .resource_mut::<Messages<AppExit>>()
            .drain()
            .collect();
        (loaded, exits)
    };

    let (loaded, exits) = started(Some(TEST_BACKDROP_ID), false);
    assert_eq!(
        loaded.as_deref(),
        Some(TEST_BACKDROP_ID),
        "a registered pick loads that scenario, session or not"
    );
    assert!(exits.is_empty(), "{exits:?}");

    let (loaded, exits) = started(None, true);
    assert_eq!(
        loaded.as_deref(),
        Some(TEST_START_ID),
        "no pick loads the declared start over the session"
    );
    assert!(exits.is_empty(), "{exits:?}");

    let (loaded, exits) = started(None, false);
    assert_eq!(
        loaded, None,
        "the start takes the session's seed; without one nothing loads"
    );
    assert_eq!(exits, vec![AppExit::error()], "the refusal ends the app");
}

/// The Create gate: the declared start must take the typed seed, so a void or
/// fixed-seed start, a start nobody registered and no start at all are each
/// a named fault, never a fallback to another scenario. A pick must be
/// registered, and a session-seeded one needs the session.
#[test]
fn the_new_game_start_is_refused_unless_it_takes_the_session_seed() {
    let fixed = {
        let (key, mut config) = dummy_scenario("fixed");
        config.world = ScenarioWorldType::Generated(GeneratedWorldConfig {
            seed: Some(3),
            bounds: None,
        });
        (key, config)
    };
    let scenarios = GameScenarios(bevy::platform::collections::HashMap::from([
        dummy_start(TEST_START_ID),
        dummy_scenario("void"),
        fixed,
    ]));
    let start = |id: Option<&str>| NewGameStart(id.map(str::to_string));
    let pick = |id: Option<&str>| NewGameScenario(id.map(str::to_string));
    let resolve = |start: NewGameStart, pick: NewGameScenario, session: bool| {
        resolve_new_game_scenario(&scenarios, None, &start, &pick, session).map(|s| s.id)
    };

    assert_eq!(
        resolve(start(Some("void")), pick(None), true),
        Err(NewGameStartFault::StartTakesNoSession {
            id: "void".to_string(),
            world: ScenarioWorldType::Void,
        })
    );
    assert_eq!(
        resolve(start(Some("fixed")), pick(None), true),
        Err(NewGameStartFault::StartTakesNoSession {
            id: "fixed".to_string(),
            world: scenarios["fixed"].world.clone(),
        })
    );
    assert_eq!(
        resolve(start(Some("gone")), pick(None), true),
        Err(NewGameStartFault::StartNotRegistered("gone".to_string()))
    );
    assert_eq!(
        resolve(start(None), pick(None), true),
        Err(NewGameStartFault::NoStartDeclared)
    );
    assert_eq!(
        resolve(start(Some(TEST_START_ID)), pick(Some("gone")), true),
        Err(NewGameStartFault::PickNotRegistered("gone".to_string()))
    );
    assert_eq!(
        resolve(start(None), pick(Some(TEST_START_ID)), false),
        Err(NewGameStartFault::NoSession(TEST_START_ID.to_string()))
    );
    assert_eq!(
        resolve(start(Some(TEST_START_ID)), pick(None), true),
        Ok(TEST_START_ID.to_string())
    );
    assert_eq!(
        resolve(start(Some(TEST_START_ID)), pick(Some("fixed")), false),
        Ok("fixed".to_string()),
        "a fixed-seed pick needs no session"
    );

    // A Load loads the scenario its save names, over the pick and the
    // declared start; one the registry lacks is a named fault.
    let resumed = |id: &str, pick: NewGameScenario| {
        resolve_new_game_scenario(
            &scenarios,
            Some(&id.to_string()),
            &start(Some(TEST_START_ID)),
            &pick,
            true,
        )
        .map(|s| s.id)
    };
    assert_eq!(
        resumed("fixed", pick(Some(TEST_START_ID))),
        Ok("fixed".to_string()),
        "the saved scenario loads over the pick and the start"
    );
    assert_eq!(
        resumed("gone", pick(None)),
        Err(NewGameStartFault::ResumedNotRegistered("gone".to_string()))
    );
}

#[test]
fn a_back_button_pops_with_the_back_voice_and_an_ordinary_button_clicks() {
    use crate::widgets::back_button;

    let mut app = cue_app();
    app.add_observer(on_menu_button_activate);
    let settings = app.world_mut().spawn(button("Settings")).id();
    let back = app.world_mut().spawn(back_button("Back")).id();
    app.update();

    app.world_mut().trigger(Activate { entity: settings });
    app.world_mut().trigger(Activate { entity: back });
    app.update();

    // Going in and coming back out are the same gesture in two directions, and
    // the pair is what makes that legible - one voice for both would be a
    // button that says nothing about where it takes you.
    assert_eq!(
        take_cues(&mut app),
        vec![Some(UiSfx::MenuSelect), Some(UiSfx::MenuBack)],
    );
}

#[test]
fn focus_ticks_when_the_cursor_arrives_and_stays_quiet_when_it_leaves() {
    use bevy::picking::hover::Hovered;

    use crate::widgets::play_menu_focus_cue;

    let mut app = cue_app();
    app.add_systems(Update, play_menu_focus_cue);
    let entity = app.world_mut().spawn(button("Scenarios")).id();
    app.update();
    let _ = take_cues(&mut app);

    app.world_mut().entity_mut(entity).insert(Hovered(true));
    app.update();
    assert_eq!(
        take_cues(&mut app),
        vec![Some(UiSfx::MenuFocus)],
        "arriving on a button ticks"
    );

    // The falling edge is silent: a sweep across a column of buttons must fire
    // once per button, not twice.
    app.world_mut().entity_mut(entity).insert(Hovered(false));
    app.update();
    assert!(take_cues(&mut app).is_empty(), "leaving a button is silent");
}

#[test]
fn a_disabled_button_does_not_tick_under_the_cursor() {
    use bevy::{picking::hover::Hovered, ui::InteractionDisabled};

    use crate::widgets::play_menu_focus_cue;

    let mut app = cue_app();
    app.add_systems(Update, play_menu_focus_cue);
    let entity = app
        .world_mut()
        .spawn((button("Continue"), InteractionDisabled))
        .id();
    app.update();
    let _ = take_cues(&mut app);

    app.world_mut().entity_mut(entity).insert(Hovered(true));
    app.update();
    // A tick means "you are on something you can press". On a greyed button
    // that would be a lie, and the paint already tells the truth.
    assert!(take_cues(&mut app).is_empty());
}
