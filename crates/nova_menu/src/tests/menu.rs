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
use nova_ui::widget::ButtonVariant;

#[cfg(not(target_arch = "wasm32"))]
use super::load_screen::{menu, menu_with_a_save, press};
use super::support::{
    app, cue_app, dummy_scenario, dummy_scenarios, dummy_start, enter_playing, entity_by_name,
    fixture_start, mods_app, observe_load_scenario, observe_unload_scenario, press_escape,
    take_cues, LoadedScenario, PlayedCues, Unloaded, TEST_BACKDROP_ID, TEST_ROLE_IDS,
    TEST_START_ID,
};
use crate::{
    menu_ui::{on_sandbox, resolve_new_game_scenario, NewGameStartFault},
    scenarios::NewGameScenario,
    widgets::button,
};

#[test]
fn a_menu_button_activation_clicks_and_a_bare_activation_does_not() {
    let mut app = cue_app();
    app.add_plugins(ButtonCuePlugin);
    // A real menu button and a bare
    // entity that merely gets Activate'd.
    let menu_button = app.world_mut().spawn(button("New Game")).id();
    let bare = app.world_mut().spawn_empty().id();
    app.update();

    app.world_mut().trigger(Activate { entity: bare });
    app.update();
    assert_eq!(
        app.world().resource::<PlayedCues>().0,
        0,
        "an activation on no button is silent"
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
                start: fixture_start(),
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
    app.add_plugins(ButtonCuePlugin);
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
    use bevy::picking::{
        hover::Hovered,
        pointer::{PointerLocation, PointerPressState},
    };

    let mut app = cue_app();
    app.add_plugins(ButtonCuePlugin);
    let entity = app.world_mut().spawn(button("Scenarios")).id();
    let pointer = app
        .world_mut()
        .spawn((PointerLocation::default(), PointerPressState::default()))
        .id();
    app.update();
    let _ = take_cues(&mut app);

    app.world_mut().entity_mut(entity).insert(Hovered(true));
    app.world_mut()
        .entity_mut(pointer)
        .insert(PointerLocation::default());
    app.update();
    assert_eq!(
        take_cues(&mut app),
        vec![Some(UiSfx::MenuFocus)],
        "arriving on a button ticks"
    );

    // The falling edge is silent: a sweep across a column of buttons must fire
    // once per button, not twice.
    app.world_mut().entity_mut(entity).insert(Hovered(false));
    app.world_mut()
        .entity_mut(pointer)
        .insert(PointerLocation::default());
    app.update();
    assert!(take_cues(&mut app).is_empty(), "leaving a button is silent");
}

#[test]
fn a_disabled_button_does_not_tick_under_the_cursor() {
    use bevy::{
        picking::{
            hover::Hovered,
            pointer::{PointerLocation, PointerPressState},
        },
        ui::InteractionDisabled,
    };

    let mut app = cue_app();
    app.add_plugins(ButtonCuePlugin);
    let entity = app
        .world_mut()
        .spawn((button("Continue"), InteractionDisabled))
        .id();
    let pointer = app
        .world_mut()
        .spawn((PointerLocation::default(), PointerPressState::default()))
        .id();
    app.update();
    let _ = take_cues(&mut app);

    app.world_mut().entity_mut(entity).insert(Hovered(true));
    app.world_mut()
        .entity_mut(pointer)
        .insert(PointerLocation::default());
    app.update();
    // A tick means "you are on something you can press". On a greyed button
    // that would be a lie, and the paint already tells the truth.
    assert!(take_cues(&mut app).is_empty());
}

/// The name of the one button entity, or `None` when the card draws none.
fn button_variant_of(app: &mut App, name: &str) -> Option<ButtonVariant> {
    let entity = entity_by_name(app, name)?;
    app.world().get::<ButtonVariant>(entity).copied()
}

/// One call-to-action is lit at a time: Continue when a save is there to
/// continue, else New Game. The slot is rebuilt at runtime, and New Game
/// follows it.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_main_menu_lights_one_primary_action() {
    let mut bare = menu();
    assert!(
        entity_by_name(&mut bare, "Continue Button").is_none(),
        "no save: no Continue"
    );
    assert_eq!(
        button_variant_of(&mut bare, "New Game Button"),
        Some(ButtonVariant::Primary),
        "no save: New Game is the lit action"
    );

    let root = tempfile::tempdir().expect("a scratch worlds root");
    let mut saved = menu_with_a_save(root.path(), "Only World");
    assert_eq!(
        button_variant_of(&mut saved, "Continue Button"),
        Some(ButtonVariant::Primary)
    );
    assert_eq!(
        button_variant_of(&mut saved, "New Game Button"),
        Some(ButtonVariant::Default),
        "a save to continue: New Game steps back"
    );

    // Deleting the last save rebuilds the slot: New Game lights up again.
    press(&mut saved, "Load Button");
    press(&mut saved, "Load World Row: only-world");
    press(&mut saved, "Load World Delete Button");
    press(&mut saved, "Load World Delete Confirm Button");
    assert!(entity_by_name(&mut saved, "Continue Button").is_none());
    assert_eq!(
        button_variant_of(&mut saved, "New Game Button"),
        Some(ButtonVariant::Primary),
        "the last save gone: New Game is lit again"
    );
}

/// The visibility of the named overlay root.
fn visibility_of(app: &mut App, name: &str) -> Visibility {
    let entity = entity_by_name(app, name).unwrap_or_else(|| panic!("no entity named {name}"));
    *app.world().get::<Visibility>(entity).expect("a UI node")
}

/// Escape answers for the Back button of the screen over the card, one level
/// at a time, and never leaves the menu.
#[test]
fn escape_closes_the_open_menu_screen_and_leaves_the_menu_up() {
    let mut app = mods_app();
    for (button, root) in [
        ("Settings Button", "Settings Panel Root"),
        ("Mods Button", "Mods Panel Root"),
        ("Scenarios Button", "Scenarios Panel Root"),
    ] {
        let opener = entity_by_name(&mut app, button).expect("the card has the button");
        app.world_mut().trigger(Activate { entity: opener });
        app.update();
        assert_ne!(
            visibility_of(&mut app, root),
            Visibility::Hidden,
            "{root} opens"
        );

        press_escape(&mut app);
        assert_eq!(
            visibility_of(&mut app, root),
            Visibility::Hidden,
            "{root} closes"
        );
        assert_eq!(
            *app.world().resource::<State<GameStates>>().get(),
            GameStates::MainMenu
        );
    }

    // Escape on the bare card does nothing at all.
    press_escape(&mut app);
    assert_eq!(
        *app.world().resource::<State<GameStates>>().get(),
        GameStates::MainMenu
    );
}

/// A chip waiting for a key owns that Escape: it disarms, and the Settings
/// screen it sits on stays open for the next one.
#[test]
fn escape_leaves_an_armed_rebind_to_the_chip() {
    use crate::settings::{PendingRebind, SettingsActiveTab, SettingsTabKind};

    let mut app = mods_app();
    let opener = entity_by_name(&mut app, "Settings Button").expect("Settings button");
    app.world_mut().trigger(Activate { entity: opener });
    app.world_mut().resource_mut::<SettingsActiveTab>().0 = SettingsTabKind::Controls;
    app.update();
    let chip = entity_by_name(&mut app, "Rebind: main_drive Desk").expect("a chip");
    app.world_mut().trigger(Activate { entity: chip });
    app.update();
    assert!(app.world().resource::<PendingRebind>().is_armed());

    press_escape(&mut app);
    assert!(
        !app.world().resource::<PendingRebind>().is_armed(),
        "the chip took its Escape"
    );
    assert_ne!(
        visibility_of(&mut app, "Settings Panel Root"),
        Visibility::Hidden,
        "and the screen stays open"
    );

    press_escape(&mut app);
    assert_eq!(
        visibility_of(&mut app, "Settings Panel Root"),
        Visibility::Hidden,
        "the next Escape closes it"
    );
}

/// An asked Delete is cancelled by Escape before the Load screen closes.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn escape_cancels_the_delete_confirmation_before_closing_load() {
    use crate::load_screen::WorldDeleteStep;

    let root = tempfile::tempdir().expect("a scratch worlds root");
    let mut app = menu_with_a_save(root.path(), "Doomed");
    press(&mut app, "Load Button");
    press(&mut app, "Load World Row: doomed");
    press(&mut app, "Load World Delete Button");
    assert!(matches!(
        *app.world().resource::<WorldDeleteStep>(),
        WorldDeleteStep::Confirm(_)
    ));

    press_escape(&mut app);
    assert!(
        matches!(
            *app.world().resource::<WorldDeleteStep>(),
            WorldDeleteStep::Idle
        ),
        "the first Escape withdraws the ask"
    );
    assert_ne!(
        visibility_of(&mut app, "Load Panel Root"),
        Visibility::Hidden,
        "and the Load screen stays open"
    );
    assert!(
        entity_by_name(&mut app, "Load World Row: doomed").is_some(),
        "and the world is still there"
    );

    press_escape(&mut app);
    assert_eq!(
        visibility_of(&mut app, "Load Panel Root"),
        Visibility::Hidden,
        "the next Escape closes Load"
    );
}

/// The card is a fixed 280 px column anchored to a corner. On a short window
/// (1280x720 at 150 percent UI scale is 853x480 logical) the buttons scroll
/// and the card stays inside the window.
#[test]
fn the_main_menu_card_stays_inside_a_small_window() {
    use bevy::{
        camera::{ComputedCameraValues, RenderTargetInfo},
        image::TextureAtlasLayout,
        math::UVec2,
        text::TextPlugin,
        ui::{ComputedNode, UiGlobalTransform, UiPlugin},
        window::PrimaryWindow,
    };

    const WIDTH: f32 = 853.0;
    const HEIGHT: f32 = 480.0;

    let mut app = app();
    app.insert_resource(dummy_scenarios());
    app.init_asset::<TextureAtlasLayout>();
    app.add_plugins((
        TransformPlugin,
        bevy::a11y::AccessibilityPlugin,
        bevy::input::InputPlugin,
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
        TextPlugin,
        UiPlugin,
    ));
    // The default UI camera is the one drawing to the primary window. The
    // menu spawns its own; no render plugin sizes it here, so size it by hand.
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();
    let mut cameras = app.world_mut().query::<&mut Camera>();
    for mut camera in cameras.iter_mut(app.world_mut()) {
        camera.computed = ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: UVec2::new(WIDTH as u32, HEIGHT as u32),
                scale_factor: 1.0,
            }),
            ..default()
        };
    }
    for _ in 0..6 {
        app.update();
    }

    let card = entity_by_name(&mut app, "Menu Panel").expect("the card");
    let size = app
        .world()
        .get::<ComputedNode>(card)
        .expect("laid out")
        .size;
    let centre = app
        .world()
        .get::<UiGlobalTransform>(card)
        .expect("positioned")
        .translation;
    let top = centre.y - size.y / 2.0;
    let bottom = centre.y + size.y / 2.0;
    assert!(size.y > 0.0, "the card was laid out: {size:?}");
    assert!(top >= 0.0, "the card top is on screen: {top}");
    assert!(bottom <= HEIGHT, "the card bottom is on screen: {bottom}");

    // The window really is too short for the column: the buttons scroll.
    let list = entity_by_name(&mut app, "Menu Buttons").expect("the button list");
    let list = app.world().get::<ComputedNode>(list).expect("laid out");
    assert!(
        list.content_size.y > list.size.y,
        "the list overflows its viewport: content {:?} in {:?}",
        list.content_size,
        list.size
    );
}
