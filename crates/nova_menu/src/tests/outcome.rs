//! The victory/defeat overlay: which entries it offers, that it freezes the sim
//! and frees the cursor the way the pause menu does, that ESC cannot raise the
//! pause overlay over it, and that it rebuilds when a switch is queued late.

use bevy::{
    prelude::*,
    ui_widgets::Activate,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::support::{
    all_text, app, app_with_outcome, clocks_paused, dummy_scenarios, enter_playing, find_named,
    pause_state, press_escape,
};
use crate::outcome::StartFailureOverlay;

/// Defeat with a queued lingering retry (the shakedown death shape):
/// DEFEAT banner, message, a Retry button that releases the lingering
/// switch (the same mechanism as the Enter key), and Main Menu.
#[test]
fn defeat_overlay_offers_retry_that_releases_the_lingering_switch() {
    let mut app = app_with_outcome();
    enter_playing(&mut app);

    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .next_scenario = Some(NextScenarioActionConfig {
        scenario_id: "retry_me".to_string(),
        linger: true,
        delay: None,
    });
    app.world_mut().resource_mut::<CurrentOutcome>().0 = Some(OutcomeActionConfig::new(
        ScenarioOutcomeKind::Defeat,
        "Your ship broke apart.",
    ));
    app.update();

    assert!(find_named(&mut app, "Outcome Overlay").is_some());
    let texts = all_text(&mut app);
    assert!(texts.iter().any(|t| t == "DEFEAT"), "banner: {texts:?}");
    assert!(texts.iter().any(|t| t == "Your ship broke apart."));
    assert!(texts.iter().any(|t| t == "Retry"), "retry label: {texts:?}");
    assert!(texts.iter().any(|t| t == "[Enter] Retry"), "key hint");

    let retry = find_named(&mut app, "Outcome Primary Button").expect("retry button");
    app.world_mut().trigger(Activate { entity: retry });
    app.update();

    let world = app.world().resource::<NovaEventWorld>();
    assert_eq!(
        world.next_scenario.as_ref().map(|next| next.linger),
        Some(false),
        "Retry releases the lingering switch"
    );
}

/// The timed overlay: auto_advance_secs releases the lingering chain after N REAL
/// seconds with no click - and an outcome WITHOUT it waits forever (delivery guard).
#[test]
fn auto_advance_releases_the_lingering_switch_after_real_seconds() {
    use core::time::Duration;

    let mut app = app_with_outcome();
    // The headless rig has no TimePlugin: provide the wall clock and
    // advance it by hand (the overlay pauses virtual time; the advance
    // clock must run on real time).
    app.insert_resource(Time::<Real>::default());
    let step = |app: &mut App| {
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_millis(200));
        app.update();
    };
    enter_playing(&mut app);

    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .next_scenario = Some(NextScenarioActionConfig {
        scenario_id: "next_up".to_string(),
        linger: true,
        delay: None,
    });
    app.world_mut().resource_mut::<CurrentOutcome>().0 = Some(OutcomeActionConfig {
        outcome: ScenarioOutcomeKind::Victory,
        message: Some("Onward.".to_string()),
        auto_advance_secs: Some(1.0),
    });
    step(&mut app);
    step(&mut app);
    assert_eq!(
        app.world()
            .resource::<NovaEventWorld>()
            .next_scenario
            .as_ref()
            .map(|next| next.linger),
        Some(true),
        "inside the window the overlay still waits"
    );

    for _ in 0..8 {
        step(&mut app);
    }
    assert_eq!(
        app.world()
            .resource::<NovaEventWorld>()
            .next_scenario
            .as_ref()
            .map(|next| next.linger),
        Some(false),
        "the timed banner released the chain by itself"
    );

    // Delivery guard: without auto_advance_secs nothing ever releases.
    let mut app = app_with_outcome();
    app.insert_resource(Time::<Real>::default());
    enter_playing(&mut app);
    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .next_scenario = Some(NextScenarioActionConfig {
        scenario_id: "next_up".to_string(),
        linger: true,
        delay: None,
    });
    app.world_mut().resource_mut::<CurrentOutcome>().0 = Some(OutcomeActionConfig::new(
        ScenarioOutcomeKind::Victory,
        "Take your time.",
    ));
    for _ in 0..12 {
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_millis(200));
        app.update();
    }
    assert_eq!(
        app.world()
            .resource::<NovaEventWorld>()
            .next_scenario
            .as_ref()
            .map(|next| next.linger),
        Some(true),
        "no auto_advance_secs: the overlay waits for the player"
    );
}

/// Victory with nothing queued (end of content): VICTORY banner, no
/// Continue/Retry, the hint points at the menu, and the Main Menu button
/// exits to MainMenu. Clearing the outcome (scenario teardown) despawns
/// the overlay.
#[test]
fn victory_overlay_without_a_queued_next_offers_only_the_menu() {
    let mut app = app_with_outcome();
    enter_playing(&mut app);

    app.world_mut().resource_mut::<CurrentOutcome>().0 = Some(OutcomeActionConfig {
        outcome: ScenarioOutcomeKind::Victory,
        message: None,
        auto_advance_secs: None,
    });
    app.update();

    assert!(find_named(&mut app, "Outcome Overlay").is_some());
    let texts = all_text(&mut app);
    assert!(texts.iter().any(|t| t == "VICTORY"), "banner: {texts:?}");
    assert!(
        find_named(&mut app, "Outcome Primary Button").is_none(),
        "nothing queued: no Continue/Retry"
    );
    assert!(texts.iter().any(|t| t == "[Enter] Main Menu"), "key hint");

    // Clearing the outcome (what scenario teardown does) removes the
    // overlay on the next frame.
    app.world_mut().resource_mut::<CurrentOutcome>().0 = None;
    app.update();
    assert!(
        find_named(&mut app, "Outcome Overlay").is_none(),
        "overlay follows the resource"
    );
}

/// The overlay's Main Menu button rides the same exit as the pause
/// overlay's Back button: lands in MainMenu (which is what tears the
/// scenario down and, with it, the outcome).
#[test]
fn outcome_menu_button_exits_to_main_menu() {
    let mut app = app_with_outcome();
    enter_playing(&mut app);

    app.world_mut().resource_mut::<CurrentOutcome>().0 = Some(OutcomeActionConfig {
        outcome: ScenarioOutcomeKind::Victory,
        message: None,
        auto_advance_secs: None,
    });
    app.update();

    let menu_button = find_named(&mut app, "Outcome Menu Button").expect("menu button");
    app.world_mut().trigger(Activate {
        entity: menu_button,
    });
    app.update();
    app.update();

    assert_eq!(
        *app.world().resource::<State<GameStates>>().get(),
        GameStates::MainMenu
    );
    assert!(
        find_named(&mut app, "Outcome Overlay").is_none(),
        "DespawnOnExit(Playing) takes the overlay with it"
    );
}

/// Review R1.1 regression: a live outcome (Victory keeps the player ship
/// alive) now holds the app in Paused of its own accord, and ESC is inert over it - so
/// no ESC cycle can strand the cursor by re-grabbing it. The cursor stays free the
/// whole time the overlay is up; only clearing the outcome (and a real ESC pause after)
/// re-grabs, which is the delivery guard.
#[test]
fn a_shown_outcome_keeps_the_cursor_free_and_esc_cannot_regrab_it() {
    let mut app = app_with_outcome();
    // A real window entity so the cursor systems have a target.
    let window = app
        .world_mut()
        .spawn((
            bevy::window::Window::default(),
            bevy::window::PrimaryWindow,
            CursorOptions::default(),
        ))
        .id();
    enter_playing(&mut app);
    // Victory: the ship survives, which is what armed the original bug.
    app.world_mut().spawn(PlayerSpaceshipMarker);
    app.world_mut().resource_mut::<CurrentOutcome>().0 = Some(OutcomeActionConfig {
        outcome: ScenarioOutcomeKind::Victory,
        message: None,
        auto_advance_secs: None,
    });
    app.update();
    app.update();

    // The outcome pause holds and ESC does nothing to it.
    press_escape(&mut app);
    assert_eq!(
        pause_state(&app),
        PauseStates::Paused,
        "ESC over a shown outcome must not unpause into the live sim"
    );
    let cursor = app.world().get::<CursorOptions>(window).unwrap();
    assert_eq!(
        cursor.grab_mode,
        CursorGrabMode::None,
        "the cursor stays free while the outcome overlay is up"
    );
    assert!(cursor.visible);

    // Delivery guard (release builds only - debug builds never grab):
    // clear the outcome (scenario teardown) and a real ESC pause cycle
    // re-grabs, proving the free-cursor assertion above is not vacuous.
    #[cfg(not(feature = "debug"))]
    {
        app.world_mut().resource_mut::<CurrentOutcome>().0 = None;
        app.update();
        app.update();
        press_escape(&mut app);
        press_escape(&mut app);
        let cursor = app.world().get::<CursorOptions>(window).unwrap();
        assert_eq!(
            cursor.grab_mode,
            CursorGrabMode::Locked,
            "delivery guard: without an outcome the pause cycle re-grabs"
        );
    }
}

/// A shown outcome freezes the sim the SAME way the pause menu does - it
/// enters `PauseStates::Paused` and both clocks stop - and clearing it (Continue/Retry
/// teardown) resumes. Run across the outcome variants
/// (`probe-the-adversarial-variant`): Victory keeps the player ship alive, the case
/// most likely to keep the sim visibly running behind the banner.
#[test]
fn a_shown_outcome_freezes_the_sim_like_the_pause_menu() {
    let cases = [
        (
            ScenarioOutcomeKind::Victory,
            Some(NextScenarioActionConfig {
                scenario_id: "next".to_string(),
                linger: true,
                delay: None,
            }),
        ),
        (
            ScenarioOutcomeKind::Defeat,
            Some(NextScenarioActionConfig {
                scenario_id: "retry".to_string(),
                linger: true,
                delay: None,
            }),
        ),
        (ScenarioOutcomeKind::Victory, None),
    ];
    for (kind, queued) in cases {
        let mut app = app_with_outcome();
        enter_playing(&mut app);
        app.world_mut().spawn(PlayerSpaceshipMarker);

        // Baseline: play is live, clocks run (delivery guard for the freeze).
        assert_eq!(
            pause_state(&app),
            PauseStates::Unpaused,
            "{kind:?}: starts live"
        );
        assert_eq!(clocks_paused(&app), (false, false), "{kind:?}: clocks run");

        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .next_scenario = queued;
        app.world_mut().resource_mut::<CurrentOutcome>().0 = Some(OutcomeActionConfig {
            outcome: kind,
            message: None,
            auto_advance_secs: None,
        });
        app.update();
        app.update();

        assert_eq!(
            pause_state(&app),
            PauseStates::Paused,
            "{kind:?}: outcome pauses"
        );
        assert_eq!(
            clocks_paused(&app),
            (true, true),
            "{kind:?}: both clocks freeze behind the overlay"
        );

        // Clearing the outcome (what Continue/Retry teardown does) resumes.
        app.world_mut().resource_mut::<CurrentOutcome>().0 = None;
        app.update();
        app.update();

        assert_eq!(
            pause_state(&app),
            PauseStates::Unpaused,
            "{kind:?}: clear unpauses"
        );
        assert_eq!(
            clocks_paused(&app),
            (false, false),
            "{kind:?}: both clocks resume for the next scenario"
        );
    }
}

/// The outcome pause must NOT stack the pause-menu panel under the outcome
/// overlay: `setup_pause_ui` skips while an outcome is set. Delivery guard:
/// a plain ESC pause (no outcome) DOES spawn the pause panel.
#[test]
fn the_outcome_pause_does_not_spawn_the_pause_menu_panel() {
    let mut app = app_with_outcome();
    enter_playing(&mut app);

    app.world_mut().resource_mut::<CurrentOutcome>().0 = Some(OutcomeActionConfig {
        outcome: ScenarioOutcomeKind::Victory,
        message: None,
        auto_advance_secs: None,
    });
    app.update();
    app.update();

    assert_eq!(pause_state(&app), PauseStates::Paused, "outcome paused");
    assert!(
        find_named(&mut app, "Outcome Overlay").is_some(),
        "the outcome overlay is the modal"
    );
    assert!(
        find_named(&mut app, "Pause Overlay").is_none(),
        "the pause-menu panel must not stack under the outcome"
    );

    // Delivery guard: without an outcome, a real ESC pause spawns the panel.
    app.world_mut().resource_mut::<CurrentOutcome>().0 = None;
    app.update();
    app.update();
    press_escape(&mut app);
    assert_eq!(pause_state(&app), PauseStates::Paused, "ESC paused");
    assert!(
        find_named(&mut app, "Pause Overlay").is_some(),
        "delivery guard: a plain pause DOES spawn the panel"
    );
}

/// Supersedes review R1.7's stack-order pin: the outcome frame and the pause menu are
/// now mutually exclusive rather than stacked. The outcome enters Paused of its own
/// accord and ESC is inert over it, so ESC can never raise the pause overlay on top of
/// a shown outcome - the case R1.7's z relation was guarding no longer occurs. The
/// outcome overlay keeps its explicit GlobalZIndex (above the HUD chrome).
#[test]
fn esc_over_a_shown_outcome_never_raises_the_pause_overlay() {
    let mut app = app_with_outcome();
    enter_playing(&mut app);
    app.world_mut().resource_mut::<CurrentOutcome>().0 = Some(OutcomeActionConfig {
        outcome: ScenarioOutcomeKind::Defeat,
        message: None,
        auto_advance_secs: None,
    });
    app.update();
    app.update();

    let before = pause_state(&app);
    assert_eq!(before, PauseStates::Paused, "the outcome pause is live");
    press_escape(&mut app);

    assert_eq!(
        pause_state(&app),
        before,
        "ESC must not toggle the outcome's own pause"
    );
    assert!(
        find_named(&mut app, "Pause Overlay").is_none(),
        "ESC must not stack the pause overlay over the outcome"
    );
    let outcome = find_named(&mut app, "Outcome Overlay").expect("the sole modal");
    let outcome_z = app
        .world()
        .get::<GlobalZIndex>(outcome)
        .expect("the outcome overlay carries an explicit GlobalZIndex")
        .0;
    assert!(
        outcome_z > 0,
        "the outcome overlay must stack above the HUD chrome (z = {outcome_z})"
    );
}

/// Review R1.3: a NextScenario queued by a LATER event than the Outcome
/// still reaches the overlay - the sync rebuilds when the queued-switch
/// snapshot goes stale, so the Continue button appears.
#[test]
fn outcome_overlay_rebuilds_when_a_switch_is_queued_later() {
    let mut app = app_with_outcome();
    enter_playing(&mut app);

    app.world_mut().resource_mut::<CurrentOutcome>().0 = Some(OutcomeActionConfig {
        outcome: ScenarioOutcomeKind::Victory,
        message: None,
        auto_advance_secs: None,
    });
    app.update();
    assert!(
        find_named(&mut app, "Outcome Primary Button").is_none(),
        "nothing queued yet: menu-only overlay"
    );

    // A later beat queues the next chapter, lingering.
    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .next_scenario = Some(NextScenarioActionConfig {
        scenario_id: "next_chapter".to_string(),
        linger: true,
        delay: None,
    });
    app.update();

    assert!(
        find_named(&mut app, "Outcome Primary Button").is_some(),
        "the overlay rebuilds and offers Continue"
    );
    let texts = all_text(&mut app);
    assert!(texts.iter().any(|t| t == "Continue"), "labels: {texts:?}");
}

/// A refusal report shows the FAILED TO START overlay in Playing, and
/// menu entry clears the stale report.
#[test]
fn start_failure_shows_the_overlay_and_menu_entry_clears_it() {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    app.update();
    enter_playing(&mut app);

    app.world_mut().resource_mut::<ScenarioStartFailure>().0 = Some(ScenarioStartFailureReport {
        scenario_name: "Broken Chapter".to_string(),
        messages: vec!["unknown section prototype 'ghost_hull'".to_string()],
    });
    app.update();

    let overlays = app
        .world_mut()
        .query_filtered::<(), With<StartFailureOverlay>>()
        .iter(app.world())
        .count();
    assert_eq!(overlays, 1, "the refusal modal spawns");
    let texts: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    assert!(
        texts.iter().any(|t| t.contains("Broken Chapter")),
        "the report names the scenario: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains("ghost_hull")),
        "the report carries the issue: {texts:?}"
    );

    // Menu entry despawns the overlay (state scoping) and clears the
    // resource so it cannot re-show next run.
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();
    assert!(
        app.world().resource::<ScenarioStartFailure>().0.is_none(),
        "menu entry clears the stale report"
    );
    let overlays = app
        .world_mut()
        .query_filtered::<(), With<StartFailureOverlay>>()
        .iter(app.world())
        .count();
    assert_eq!(overlays, 0, "the modal died with the Playing state");
}

/// A player ship spawning mid-flight (a Retry reloads the
/// scenario without a state transition) re-grabs the cursor - now in debug builds too,
/// since the observer's grab used to be compiled out under `feature = "debug"`.
#[test]
fn player_spawn_hides_cursor_while_flying() {
    let mut app = app();
    enter_playing(&mut app);
    let win = app
        .world_mut()
        .spawn((
            PrimaryWindow,
            CursorOptions {
                visible: true,
                grab_mode: CursorGrabMode::None,
                ..default()
            },
        ))
        .id();
    // A direct world spawn flushes the Add observer synchronously.
    app.world_mut().spawn(PlayerSpaceshipMarker);
    let cursor = app.world().get::<CursorOptions>(win).unwrap();
    assert!(!cursor.visible, "player spawn grabs the cursor in flight");
    assert_eq!(cursor.grab_mode, CursorGrabMode::Locked);
}

/// The spawn regrab yields to the pause overlay: a ship spawning while paused
/// must not steal the cursor the pause menu freed.
#[test]
fn player_spawn_yields_to_pause() {
    let mut app = app();
    enter_playing(&mut app);
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Paused);
    app.update();
    let win = app
        .world_mut()
        .spawn((
            PrimaryWindow,
            CursorOptions {
                visible: true,
                grab_mode: CursorGrabMode::None,
                ..default()
            },
        ))
        .id();
    app.world_mut().spawn(PlayerSpaceshipMarker);
    let cursor = app.world().get::<CursorOptions>(win).unwrap();
    assert!(cursor.visible, "paused: player spawn must not re-grab");
}
