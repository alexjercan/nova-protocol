//! The pause axis: the ESC toggle and both clocks, overlay spawn/despawn,
//! retry and back-to-menu, and the NOVA OS as a third variant of the same
//! freeze rather than a separate one.

use bevy::{
    prelude::*,
    state::app::StatesPlugin,
    ui_widgets::Activate,
    window::{CursorGrabMode, CursorOptions},
};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::support::{
    app, clocks_paused, cue_app, dummy_scenario, dummy_scenarios, enter_playing, find_named,
    mods_app, observe_load_scenario, pause_state, press_escape, LoadedScenario, PlayedCues,
    TEST_BACKDROP_ID,
};
use crate::{
    mods::ModsPanel, pause::toggle_pause, scenarios::ScenariosPanel, settings::SettingsPanel,
};

#[test]
fn the_escape_pause_toggle_blips_on_both_directions() {
    let mut app = cue_app();
    app.add_plugins(StatesPlugin);
    app.init_state::<PauseStates>();
    app.init_resource::<ButtonInput<KeyCode>>();
    // Bare (no in_state run condition): drive the toggle directly.
    app.add_systems(Update, toggle_pause);

    let tap_escape = |app: &mut App| {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::Escape);
        keys.clear();
        app.update();
    };

    tap_escape(&mut app); // open
    assert_eq!(pause_state(&app), PauseStates::Paused);
    tap_escape(&mut app); // close
    assert_eq!(pause_state(&app), PauseStates::Unpaused);

    assert_eq!(
        app.world().resource::<PlayedCues>().0,
        2,
        "ESC open and ESC close each blip once"
    );
}

/// Delivery-guarded per press: ESC pauses, freezes both clocks, and a
/// second press resumes and unfreezes.
#[test]
fn escape_toggles_pause_and_both_clocks() {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    enter_playing(&mut app);
    assert_eq!(pause_state(&app), PauseStates::Unpaused);
    assert_eq!(clocks_paused(&app), (false, false));

    press_escape(&mut app);
    assert_eq!(pause_state(&app), PauseStates::Paused);
    assert_eq!(clocks_paused(&app), (true, true), "both clocks freeze");

    press_escape(&mut app);
    assert_eq!(pause_state(&app), PauseStates::Unpaused);
    assert_eq!(clocks_paused(&app), (false, false), "both clocks resume");
}

/// The pause overlay spawns with its buttons and despawns on resume.
#[test]
fn pause_overlay_spawns_and_despawns() {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    enter_playing(&mut app);
    press_escape(&mut app);

    let find = |app: &mut App, name: &str| {
        let mut q = app.world_mut().query::<(Entity, &Name)>();
        q.iter(app.world())
            .find(|(_, n)| n.as_str() == name)
            .map(|(e, _)| e)
    };
    assert!(find(&mut app, "Resume Button").is_some());
    let back = find(&mut app, "Back To Menu Button").expect("back button exists");

    // Resume via the real button, then the overlay must be gone.
    let resume = find(&mut app, "Resume Button").unwrap();
    app.world_mut().trigger(Activate { entity: resume });
    app.update();
    app.update();
    assert_eq!(pause_state(&app), PauseStates::Unpaused);
    assert!(
        find(&mut app, "Pause Overlay").is_none(),
        "overlay despawns"
    );
    // The back button entity died with the overlay.
    assert!(app.world().get_entity(back).is_err());
}

/// Opening the Tab NOVA OS reuses the pause freeze: entering `PauseStates::NovaOs`
/// freezes both clocks and frees the cursor, exactly like `Paused` - but WITHOUT
/// spawning the pause menu. Deleting the `OnEnter(NovaOs)` wiring leaves the clocks
/// running, so this fails without the mechanism (`would-it-fail-without-it`).
#[test]
fn entering_nova_os_freezes_clocks_frees_cursor_and_shows_no_pause_menu() {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    // A window whose cursor starts grabbed (flight state), so freeing it is
    // observable.
    let window = app
        .world_mut()
        .spawn((
            bevy::window::Window::default(),
            bevy::window::PrimaryWindow,
            CursorOptions {
                grab_mode: CursorGrabMode::Locked,
                visible: false,
                ..default()
            },
        ))
        .id();
    enter_playing(&mut app);
    assert_eq!(clocks_paused(&app), (false, false));

    // The NOVA OS opens by driving the shared freeze axis to NovaOs (what
    // nova_gameplay's `toggle_nova_os` does).
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::NovaOs);
    app.update();

    assert_eq!(
        clocks_paused(&app),
        (true, true),
        "opening the NOVA OS freezes both clocks"
    );
    let cursor = app.world().get::<CursorOptions>(window).unwrap();
    assert_eq!(
        cursor.grab_mode,
        CursorGrabMode::None,
        "the NOVA OS frees the cursor"
    );
    assert!(cursor.visible, "the NOVA OS shows the cursor");

    // The NOVA OS is NOT the pause menu: no pause overlay spawns.
    let mut q = app.world_mut().query::<(&Name,)>();
    let has_pause_overlay = q
        .iter(app.world())
        .any(|(n,)| n.as_str() == "Pause Overlay");
    assert!(
        !has_pause_overlay,
        "opening the NOVA OS must not spawn the pause menu overlay"
    );
}

/// ESC while the Tab NOVA OS is open belongs to the NOVA OS, not the pause menu, so the
/// menu toggle does not unpause or stack its own overlay.
#[test]
fn escape_does_not_menu_toggle_the_nova_os() {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    enter_playing(&mut app);
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::NovaOs);
    app.update();
    assert_eq!(pause_state(&app), PauseStates::NovaOs);
    assert_eq!(clocks_paused(&app), (true, true));

    press_escape(&mut app);
    assert_eq!(
        pause_state(&app),
        PauseStates::NovaOs,
        "the NOVA OS owns ESC so it can animate closed before gameplay resumes"
    );
    assert_eq!(
        clocks_paused(&app),
        (true, true),
        "the NOVA OS remains frozen until its close animation completes"
    );
    let has_pause_overlay = app
        .world_mut()
        .query::<(&Name,)>()
        .iter(app.world())
        .any(|(n,)| n.as_str() == "Pause Overlay");
    assert!(
        !has_pause_overlay,
        "ESC over the NOVA OS must not spawn the pause menu overlay"
    );
}

/// Retry needs something to reload: over a live scenario the pause
/// overlay offers it, in the editor's build mode (CurrentScenario is
/// None there) it does not. The Resume button pins that the overlay
/// itself spawned in both rigs.
#[test]
fn pause_overlay_offers_retry_only_over_a_live_scenario() {
    // A paused rig with the given loader state (the editor's build mode
    // holds a CurrentScenario of None; scenario play holds Some).
    let paused_app = |current: CurrentScenario| {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        app.insert_resource(current);
        enter_playing(&mut app);
        press_escape(&mut app);
        app
    };

    let mut editor_shape = paused_app(CurrentScenario(None));
    assert!(find_named(&mut editor_shape, "Resume Button").is_some());
    assert!(
        find_named(&mut editor_shape, "Pause Retry Button").is_none(),
        "no scenario loaded: nothing to retry"
    );

    let mut scenario_shape = paused_app(CurrentScenario(Some(dummy_scenario("live_run").1)));
    assert!(find_named(&mut scenario_shape, "Resume Button").is_some());
    assert!(
        find_named(&mut scenario_shape, "Pause Retry Button").is_some(),
        "a live scenario earns the Retry button"
    );
}

/// The Retry button reloads the CURRENT scenario (the same config the
/// loader holds) and unpauses: overlay gone, both clocks running.
#[test]
fn pause_retry_reloads_the_current_scenario_and_unpauses() {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    app.insert_resource(CurrentScenario(Some(dummy_scenario("live_run").1)));
    observe_load_scenario(&mut app);
    enter_playing(&mut app);
    press_escape(&mut app);
    assert_eq!(clocks_paused(&app), (true, true));

    let retry = find_named(&mut app, "Pause Retry Button").expect("retry button");
    app.world_mut().trigger(Activate { entity: retry });
    app.update();
    app.update();

    assert_eq!(
        app.world().resource::<LoadedScenario>().0.as_deref(),
        Some("live_run"),
        "Retry re-triggers LoadScenario with the live config"
    );
    assert_eq!(pause_state(&app), PauseStates::Unpaused);
    assert_eq!(clocks_paused(&app), (false, false), "both clocks resume");
    assert!(
        find_named(&mut app, "Pause Overlay").is_none(),
        "the overlay despawns with the pause state"
    );
}

/// Back to Main Menu from a paused game: lands in MainMenu, unpaused,
/// clocks running, and the ambience backdrop load fired (which is what
/// tears the gameplay scenario down).
#[test]
fn back_to_menu_unpauses_and_reloads_the_ambience() {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    observe_load_scenario(&mut app);
    enter_playing(&mut app);
    press_escape(&mut app);
    assert_eq!(
        clocks_paused(&app),
        (true, true),
        "paused before backing out"
    );

    let back = {
        let mut q = app.world_mut().query::<(Entity, &Name)>();
        q.iter(app.world())
            .find(|(_, n)| n.as_str() == "Back To Menu Button")
            .map(|(e, _)| e)
            .expect("back button exists")
    };
    app.world_mut().trigger(Activate { entity: back });
    app.update();
    app.update();

    assert_eq!(
        *app.world().resource::<State<GameStates>>().get(),
        GameStates::MainMenu
    );
    assert_eq!(pause_state(&app), PauseStates::Unpaused);
    assert_eq!(clocks_paused(&app), (false, false));
    assert_eq!(
        app.world().resource::<LoadedScenario>().0.as_deref(),
        Some(TEST_BACKDROP_ID)
    );
}

/// Review R1.1: both full-screen overlay roots must carry an explicit
/// GlobalZIndex above the default 0, so they stack over the bottom-right
/// menu card deterministically (sibling z-order otherwise falls back to
/// Entity ordering, whose ids the despawned ambience scene recycles). The
/// RENDERED order is only visually verifiable; this pins the component.
#[test]
fn overlay_roots_carry_an_explicit_z_index() {
    let mut app = mods_app();
    let mods_root = {
        let mut q = app.world_mut().query_filtered::<Entity, With<ModsPanel>>();
        q.single(app.world()).expect("one mods panel root")
    };
    let settings_root = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<SettingsPanel>>();
        q.single(app.world()).expect("one settings panel root")
    };
    let scenarios_root = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<ScenariosPanel>>();
        q.single(app.world()).expect("one scenarios panel root")
    };
    for (name, root) in [
        ("mods", mods_root),
        ("settings", settings_root),
        ("scenarios", scenarios_root),
    ] {
        let z = app
            .world()
            .get::<GlobalZIndex>(root)
            .unwrap_or_else(|| panic!("the {name} overlay root carries a GlobalZIndex"));
        assert!(
            z.0 > 0,
            "the {name} overlay must stack above the menu card (z = {})",
            z.0
        );
    }
}
