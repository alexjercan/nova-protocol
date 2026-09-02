//! Launching, driving and exiting NOVA OS apps.

use super::*;

#[test]
fn nova_os_exit_closes_computer() {
    // `exit` requests the same animated close as Esc/Start: it flips the
    // shared close transition (which `drive_nova_os_slide` then eases shut).
    let mut app = terminal_command_app();
    assert_eq!(pause_state(&app), PauseStates::NovaOs);
    assert!(!app.world().resource::<NovaOsCloseTransition>().closing);

    submit_terminal_command(&mut app, "exit");

    assert!(
        app.world().resource::<NovaOsCloseTransition>().closing,
        "exit requests the animated close of the computer"
    );
}

/// `drive_nova_os_slide` now drives the single monitor's visibility and
/// openness while retaining the real-time transition used by the old panels.
#[test]
fn slide_drives_single_monitor_openness() {
    use std::time::Duration;

    let mut app = App::new();
    // Disable the real TimePlugin so its per-frame clock update cannot
    // overwrite the deltas we advance by hand; drive_nova_os_slide reads
    // Time<Real>, which we own here.
    app.add_plugins(MinimalPlugins.build().disable::<bevy::time::TimePlugin>());
    app.insert_resource(Time::<Real>::default());
    app.add_plugins(StatesPlugin);
    app.init_state::<PauseStates>();
    app.init_resource::<NovaOsCloseTransition>();
    app.add_systems(Update, drive_nova_os_slide);

    let backdrop = app
        .world_mut()
        .spawn((
            NovaOsBackdropMarker,
            BackgroundColor(theme::semantic::BACKDROP.with_alpha(0.0)),
            Visibility::Hidden,
        ))
        .id();
    let _ = backdrop;
    let monitor = app
        .world_mut()
        .spawn((
            NovaOsRootMarker,
            NovaOsMonitorMarker,
            NovaOsOpenness(0.0),
            Visibility::Hidden,
            Node::default(),
        ))
        .id();

    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::NovaOs);
    app.update();
    for _ in 0..4 {
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_millis(30));
        app.update();
    }

    let openness = app.world().get::<NovaOsOpenness>(monitor).unwrap().0;
    assert!(
        openness > 0.0 && openness <= 1.0,
        "monitor openness advances toward visible (openness {openness})"
    );
    assert_eq!(
        *app.world().get::<Visibility>(monitor).unwrap(),
        Visibility::Visible
    );

    app.world_mut()
        .resource_mut::<NovaOsCloseTransition>()
        .closing = true;
    for _ in 0..8 {
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_millis(30));
        app.update();
    }
    app.update();

    assert_eq!(
        pause_state(&app),
        PauseStates::Unpaused,
        "gameplay resumes only after the NOVA OS close animation finishes"
    );
    assert_eq!(
        *app.world().get::<Visibility>(monitor).unwrap(),
        Visibility::Hidden
    );
}

// --- NOVA OS app runtime lifecycle ---

/// A test-only sample app registered into the registry to exercise the app
/// runtime without waiting for the real `map` app. It renders one body row and
/// exits on its own `q` key, so a test can prove the app owns input.
#[test]
fn terminal_command_launches_registered_app() {
    let mut app = app_runtime_app();
    submit_terminal_command(&mut app, "sample");

    let terminal = app.world().resource::<NovaOsTerminal>();
    assert_eq!(
        terminal.active_mode(),
        TerminalMode::App { id: "sample" },
        "submitting a registered app word enters app mode",
    );
    assert!(
        terminal
            .scrollback()
            .iter()
            .any(|row| row.text.contains("launching sample")),
        "launch prints a status row into the scrollback",
    );
    assert_eq!(
        pause_state(&app),
        PauseStates::NovaOs,
        "the computer stays open while an app runs",
    );
}

#[test]
fn nova_os_app_launch_word_rejects_arguments() {
    let mut app = app_runtime_app();
    submit_terminal_command(&mut app, "sample foo");

    let terminal = app.world().resource::<NovaOsTerminal>();
    assert_eq!(
        terminal.active_mode(),
        TerminalMode::Prompt,
        "an app word with arguments does not launch",
    );
    assert!(
        terminal
            .scrollback()
            .iter()
            .any(|row| row.text == "sample: takes no arguments"),
        "the argument rejection is reported",
    );
}

#[test]
fn nova_os_launch_keystroke_does_not_bleed_into_the_app() {
    // The Enter that submits `enterapp` must not reach the app it launches -
    // `EnterExitApp` exits on Enter, so a bleed would close it on the same
    // frame it opened.
    let mut app = app_runtime_app();
    submit_terminal_command(&mut app, "enterapp");
    assert_eq!(
        app.world().resource::<NovaOsTerminal>().active_mode(),
        TerminalMode::App { id: "enterapp" },
        "the launching Enter did not bleed through to exit the app",
    );

    // A SUBSEQUENT Enter does reach the app (it is genuinely Enter-sensitive).
    press_key(&mut app, KeyCode::Enter, Key::Enter, None);
    assert_eq!(
        app.world().resource::<NovaOsTerminal>().active_mode(),
        TerminalMode::Prompt,
        "a later Enter reaches the app and exits it",
    );
}

#[test]
fn nova_os_app_close_restores_terminal_state() {
    let mut app = app_runtime_app();
    // Build some scrollback before launching so we can prove it survives.
    submit_terminal_command(&mut app, "help");
    let before = terminal_scrollback_texts(&app);
    submit_terminal_command(&mut app, "sample");
    assert!(matches!(
        app.world().resource::<NovaOsTerminal>().active_mode(),
        TerminalMode::App { .. }
    ));

    // Escape exits the app back to the terminal, NOT the NOVA OS.
    press_escape(&mut app);

    let terminal = app.world().resource::<NovaOsTerminal>();
    assert_eq!(
        terminal.active_mode(),
        TerminalMode::Prompt,
        "Escape from app mode returns to the terminal",
    );
    assert!(
        !app.world().resource::<NovaOsCloseTransition>().closing,
        "exiting the app does not request a computer close",
    );
    assert_eq!(
        pause_state(&app),
        PauseStates::NovaOs,
        "the computer stays open after the app exits",
    );
    assert_eq!(terminal.prompt(), "", "the prompt is restored empty");
    for row in &before {
        assert!(
            terminal.scrollback().iter().any(|r| &r.text == row),
            "pre-app scrollback row preserved after the app: {row}",
        );
    }
}

#[test]
fn nova_os_app_mode_owns_input_and_escape_exits_app() {
    let mut app = app_runtime_app();
    submit_terminal_command(&mut app, "sample");
    assert!(matches!(
        app.world().resource::<NovaOsTerminal>().active_mode(),
        TerminalMode::App { .. }
    ));

    // Typing while the app owns the screen does not reach the terminal prompt.
    press_text(&mut app, "x");
    assert_eq!(
        app.world().resource::<NovaOsTerminal>().prompt(),
        "",
        "app mode owns input: typing does not edit the terminal prompt",
    );

    // The app's own key drives its exit back to the terminal.
    press_key(
        &mut app,
        KeyCode::KeyQ,
        Key::Character("q".into()),
        Some("q"),
    );
    assert_eq!(
        app.world().resource::<NovaOsTerminal>().active_mode(),
        TerminalMode::Prompt,
        "an app-owned key exits the app",
    );
    assert_eq!(
        pause_state(&app),
        PauseStates::NovaOs,
        "exiting the app keeps the computer open",
    );

    // Back at the prompt, Escape now closes the whole computer.
    press_escape(&mut app);
    assert!(
        app.world().resource::<NovaOsCloseTransition>().closing,
        "from terminal mode Escape requests the computer close",
    );
}

#[test]
fn nova_os_app_state_resets_on_teardown() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<NovaOsFlightLog>();
    app.init_resource::<NovaOsTerminal>();
    app.add_systems(Update, ensure_nova_os_spawned);
    app.add_observer(reset_nova_os_for_new_ship);

    let player = app
        .world_mut()
        .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
        .id();
    app.update();

    // An app is running when the player ship goes away.
    app.world_mut()
        .resource_mut::<NovaOsTerminal>()
        .enter_app("sample");

    app.world_mut()
        .entity_mut(player)
        .remove::<PlayerSpaceshipMarker>();
    app.update();

    let terminal = app.world().resource::<NovaOsTerminal>();
    assert_eq!(
        terminal.active_mode(),
        TerminalMode::Prompt,
        "teardown clears stale app state back to the terminal",
    );
    assert_eq!(
        terminal.scrollback(),
        nova_os_welcome_rows(),
        "teardown restores the welcome screen",
    );
}

#[test]
fn nova_os_app_ui_spawns_chrome_and_close_button_exits() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.insert_state(GameStates::Playing);
    app.init_state::<PauseStates>();
    app.init_resource::<NovaOsFlightLog>();
    app.init_resource::<NovaOsTerminal>();
    app.init_resource::<NovaOsDegauss>();
    let mut registry = NovaOsCommandRegistry::default();
    registry.register(TerminalCommand::app(
        "sample",
        "Test-only lifecycle app",
        SampleApp,
    ));
    app.insert_resource(registry);
    app.add_systems(Update, ensure_nova_os_spawned);
    app.add_observer(reset_nova_os_for_new_ship);
    app.add_systems(
        Update,
        sync_nova_os_app_ui.run_if(in_state(PauseStates::NovaOs)),
    );

    app.world_mut()
        .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::NovaOs);
    app.update();

    // Launch, then let the UI reconcile.
    app.world_mut()
        .resource_mut::<NovaOsTerminal>()
        .enter_app("sample");
    app.update();

    let app_roots = app
        .world_mut()
        .query_filtered::<Entity, With<NovaOsAppRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(app_roots, 1, "launch spawns exactly one app root");
    let close = app
        .world_mut()
        .query_filtered::<Entity, With<NovaOsAppCloseMarker>>()
        .iter(app.world())
        .next()
        .expect("the header has a close control");
    let content_visibility = app
        .world_mut()
        .query_filtered::<&Visibility, With<NovaOsTerminalContentMarker>>()
        .iter(app.world())
        .next()
        .copied();
    assert_eq!(
        content_visibility,
        Some(Visibility::Hidden),
        "the terminal content is hidden while an app owns the screen",
    );

    // The header close control returns to the terminal, the same route as Escape.
    app.world_mut().trigger(Activate { entity: close });
    app.update();

    assert_eq!(
        app.world().resource::<NovaOsTerminal>().active_mode(),
        TerminalMode::Prompt,
        "the header close control exits the app",
    );
    let app_roots_after = app
        .world_mut()
        .query_filtered::<Entity, With<NovaOsAppRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(app_roots_after, 0, "exiting despawns the app root");
    let content_after = app
        .world_mut()
        .query_filtered::<&Visibility, With<NovaOsTerminalContentMarker>>()
        .iter(app.world())
        .next()
        .copied();
    assert_eq!(
        content_after,
        Some(Visibility::Inherited),
        "exiting reveals the terminal content again",
    );
}

/// The persistent header tracks the active surface: `reconcile_nova_os_header`
/// swaps the brand breadcrumb (`// SHELL` <-> `// APPS / <ID>`) and shows the
/// close control only while an app owns the screen (DoD item 2 + 4).
#[test]
fn header_reconciles_breadcrumb_and_close_control_across_the_swap() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.insert_state(GameStates::Playing);
    app.init_state::<PauseStates>();
    app.init_resource::<NovaOsFlightLog>();
    app.init_resource::<NovaOsTerminal>();
    app.init_resource::<NovaOsDegauss>();
    let mut registry = NovaOsCommandRegistry::default();
    registry.register(TerminalCommand::app(
        "sample",
        "Test-only lifecycle app",
        SampleApp,
    ));
    app.insert_resource(registry);
    app.add_systems(Update, ensure_nova_os_spawned);
    app.add_observer(reset_nova_os_for_new_ship);
    app.add_systems(
        Update,
        (
            sync_nova_os_app_ui.run_if(in_state(PauseStates::NovaOs)),
            reconcile_nova_os_header
                .run_if(resource_changed::<NovaOsTerminal>.or_else(nova_os_header_just_spawned)),
        )
            .chain(),
    );

    app.world_mut()
        .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::NovaOs);
    app.update();

    let ver = nova_os_version_label();
    let brand_text = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&Text, With<NovaOsBrandMarker>>()
            .iter(app.world())
            .next()
            .map(|text| text.0.clone())
    };
    let close_visibility = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&Visibility, With<NovaOsAppCloseMarker>>()
            .iter(app.world())
            .next()
            .copied()
    };

    // At the prompt: SHELL breadcrumb, close control hidden.
    assert_eq!(
        brand_text(&mut app),
        Some(format!("NOVA OS {ver} // SHELL"))
    );
    assert_eq!(close_visibility(&mut app), Some(Visibility::Hidden));

    // Open the app: breadcrumb swaps to the APPS path, close control shows.
    app.world_mut()
        .resource_mut::<NovaOsTerminal>()
        .enter_app("sample");
    app.update();
    assert_eq!(
        brand_text(&mut app),
        Some(format!("NOVA OS {ver} // APPS / SAMPLE")),
        "opening an app swaps the header breadcrumb to its APPS path",
    );
    assert_eq!(
        close_visibility(&mut app),
        Some(Visibility::Inherited),
        "the header close control shows while an app owns the screen",
    );

    // Exit back to the prompt: breadcrumb + close control revert.
    assert!(app.world_mut().resource_mut::<NovaOsTerminal>().exit_app());
    app.update();
    assert_eq!(
        brand_text(&mut app),
        Some(format!("NOVA OS {ver} // SHELL")),
        "exiting restores the SHELL breadcrumb",
    );
    assert_eq!(
        close_visibility(&mut app),
        Some(Visibility::Hidden),
        "exiting hides the header close control again",
    );
}

// --- NOVA OS terminal UX parity ---

/// The first open types the boot banner row-by-row on real time and reports
/// the unread-events count; `clear` reprints it instantly.
#[test]
fn nova_os_boot_banner_staggers_and_counts_unread() {
    use std::time::Duration;

    let mut app = App::new();
    // Own the real clock so the staggered reveal is deterministic (same rig as
    // `slide_drives_single_monitor_openness`).
    app.add_plugins(MinimalPlugins.build().disable::<bevy::time::TimePlugin>());
    app.insert_resource(Time::<Real>::default());
    app.add_plugins(StatesPlugin);
    app.init_state::<PauseStates>();
    app.init_resource::<NovaOsTerminal>();

    let mut log = NovaOsFlightLog::default();
    for i in 0..4 {
        log.entries.push(NovaOsFlightLogEntry {
            kind: NovaOsFlightLogEntryKind::Comms,
            objective_id: None,
            speaker: Some("SYS".to_string()),
            message: format!("event {i}"),
            icon: None,
        });
    }
    app.insert_resource(log);
    app.add_systems(OnEnter(PauseStates::NovaOs), begin_nova_os_boot);
    app.add_systems(Update, drain_nova_os_boot);

    // Open the computer for the first time: the banner is QUEUED, not printed.
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::NovaOs);
    app.update();
    let full = nova_os_boot_banner_rows(4, Some("event 3".to_string())).len();
    assert!(
        app.world()
            .resource::<NovaOsTerminal>()
            .scrollback()
            .is_empty(),
        "the boot banner is staggered, not printed instantly",
    );

    // A few 130 ms ticks reveal a FEW rows, not all of them at once.
    for _ in 0..3 {
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_millis(130));
        app.update();
    }
    let partway = app.world().resource::<NovaOsTerminal>().scrollback().len();
    assert!(
        partway >= 1 && partway < full,
        "rows reveal gradually (revealed {partway} of {full})",
    );

    // Draining fully lands the whole banner including the unread-events line.
    for _ in 0..12 {
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_millis(130));
        app.update();
    }
    let terminal = app.world().resource::<NovaOsTerminal>();
    assert_eq!(terminal.scrollback().len(), full);
    assert!(
        !terminal.has_pending_boot_rows(),
        "the reveal queue drains dry"
    );
    assert!(
        terminal
            .scrollback()
            .iter()
            .any(|row| row.text.contains("4 unread events")),
        "the boot banner reports the unread-events count",
    );

    // `clear` reprints the banner instantly (no staggered queue), still with
    // the current unread count from the snapshot.
    let mut terminal = app.world_mut().resource_mut::<NovaOsTerminal>();
    terminal.insert_text("clear");
    terminal.submit(&TerminalCommandSnapshot {
        unread_events: 2,
        unread_hook: Some("Torpedo bay is down".to_string()),
        ..Default::default()
    });
    assert!(
        !terminal.has_pending_boot_rows(),
        "clear reprints instantly rather than re-staggering",
    );
    assert!(
        terminal
            .scrollback()
            .iter()
            .any(|row| row.text.contains("2 unread events")),
        "clear reprints the current unread-events line",
    );
}

/// PageUp/PageDown page the scrollback viewport, clamped to its content.
#[test]
fn nova_os_page_keys_scroll_scrollback() {
    let mut app = terminal_command_app();
    let scrollback = app
        .world_mut()
        .spawn((
            NovaOsTerminalScrollbackMarker,
            ScrollPosition(Vec2::new(0.0, 100.0)),
            ComputedNode {
                size: Vec2::new(100.0, 100.0),
                content_size: Vec2::new(100.0, 400.0),
                scrollbar_size: Vec2::ZERO,
                ..default()
            },
        ))
        .id();

    // PageUp pages toward the top.
    press_key(&mut app, KeyCode::PageUp, Key::PageUp, None);
    let after_up = app
        .world()
        .entity(scrollback)
        .get::<ScrollPosition>()
        .unwrap()
        .0
        .y;
    assert!(
        after_up < 100.0,
        "PageUp pages the scrollback toward the top (y {after_up})",
    );

    // PageDown pages back toward the bottom.
    press_key(&mut app, KeyCode::PageDown, Key::PageDown, None);
    let after_down = app
        .world()
        .entity(scrollback)
        .get::<ScrollPosition>()
        .unwrap()
        .0
        .y;
    assert!(
        after_down > after_up,
        "PageDown pages the scrollback toward the bottom (y {after_down})",
    );
    assert!(
        after_down <= 300.0,
        "the page offset clamps to the content bottom (max 300, got {after_down})",
    );
}

/// A test-only app that overrides `hints` to a distinct footer set.
#[test]
fn nova_os_footer_hints_follow_active_surface() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.init_state::<PauseStates>();
    app.init_resource::<NovaOsTerminal>();
    let mut registry = NovaOsCommandRegistry::default();
    registry.register(TerminalCommand::app(
        "hintsapp",
        "Test-only app with its own footer hints",
        HintsApp,
    ));
    app.insert_resource(registry);
    app.register_input_actions(crate::bindings::novaos_bindings());
    app.add_systems(Update, rebuild_nova_os_footer_hints);

    let footer = app.world_mut().spawn(NovaOsFooterHintsMarker).id();

    // At the prompt the footer shows the terminal hint set.
    app.update();
    let prompt_hints = terminal_hints(app.world().resource::<InputBindings>());
    assert_eq!(
        footer_hint_texts(&app, footer),
        prompt_hints,
        "the terminal surface shows the terminal hints",
    );
    assert_eq!(
        prompt_hints.first().map(String::as_str),
        Some("TAB: COMPLETE"),
        "the completion hint names the key `novaos_toggle` holds",
    );

    // Entering the app swaps the footer to the app's own hints.
    app.world_mut()
        .resource_mut::<NovaOsTerminal>()
        .enter_app("hintsapp");
    app.update();
    assert_eq!(
        footer_hint_texts(&app, footer),
        vec![
            "1/2/3: DO A THING".to_string(),
            "ESC: BACK TO TERMINAL".to_string(),
            "SHIFT+ESC: CLOSE".to_string(),
        ],
        "an active app swaps the footer to its own hints",
    );

    // Backing out to the prompt restores the terminal hints.
    app.world_mut().resource_mut::<NovaOsTerminal>().exit_app();
    app.update();
    assert_eq!(
        footer_hint_texts(&app, footer),
        prompt_hints,
        "returning to the prompt restores the terminal hints",
    );

    // A rebind is the third trigger: the surface never changed, but the key the
    // footer names did.
    app.world_mut().resource_mut::<InputBindings>().rebind(
        "novaos_toggle",
        BindingSpec {
            keyboard: vec![InputSource::Keyboard(KeyCode::Backslash)],
            gamepad: Vec::new(),
        },
    );
    app.update();
    assert_eq!(
        footer_hint_texts(&app, footer).first().map(String::as_str),
        Some("\\: COMPLETE"),
        "the footer follows the move without leaving the prompt",
    );
}

/// Press a `<modifier>+<key>` chord via `ButtonInput`, mirroring `press_tab`.
/// Ctrl+C exits a running app back to the terminal; Shift+Esc closes the whole
/// computer from inside an app.
#[test]
fn nova_os_app_exit_chords() {
    let mut app = app_runtime_app();
    submit_terminal_command(&mut app, "sample");
    assert!(matches!(
        app.world().resource::<NovaOsTerminal>().active_mode(),
        TerminalMode::App { .. }
    ));

    // Ctrl+C backs out to the terminal without closing the computer.
    press_chord(&mut app, KeyCode::ControlLeft, KeyCode::KeyC);
    assert_eq!(
        app.world().resource::<NovaOsTerminal>().active_mode(),
        TerminalMode::Prompt,
        "Ctrl+C exits the app back to the terminal",
    );
    assert!(
        !app.world().resource::<NovaOsCloseTransition>().closing,
        "Ctrl+C does not close the computer",
    );
    assert_eq!(pause_state(&app), PauseStates::NovaOs);

    // Relaunch, then Shift+Esc closes the computer from INSIDE the app.
    submit_terminal_command(&mut app, "sample");
    assert!(matches!(
        app.world().resource::<NovaOsTerminal>().active_mode(),
        TerminalMode::App { .. }
    ));
    press_chord(&mut app, KeyCode::ShiftLeft, KeyCode::Escape);
    assert!(
        app.world().resource::<NovaOsCloseTransition>().closing,
        "Shift+Esc closes the computer from inside an app",
    );
}

/// An objective flipping while the computer is open announces itself into the
/// live scrollback (PoC `checkObjectives`), without needing a `log` command.
#[test]
fn nova_os_objective_flip_announces_in_open_terminal() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.init_state::<PauseStates>();
    app.init_resource::<NovaOsTerminal>();
    app.init_resource::<NovaOsFlightLog>();
    app.init_resource::<GameObjectives>();
    app.init_resource::<StoryFeed>();
    app.add_systems(
        Update,
        (sync_nova_os_logs, announce_objectives_in_terminal)
            .chain()
            .run_if(resource_changed::<GameObjectives>),
    );

    // Open the computer and post an objective.
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::NovaOs);
    app.world_mut()
        .resource_mut::<GameObjectives>()
        .objectives
        .push(Objective::new("burn", "Burn for Beacon 1"));
    app.update();
    assert!(
        !app.world()
            .resource::<NovaOsTerminal>()
            .scrollback()
            .iter()
            .any(|row| row.text.contains("OBJ x")),
        "posting an objective does not yet announce a completion",
    );

    // Complete it: the OBJ x line lands in the live scrollback.
    app.world_mut()
        .resource_mut::<GameObjectives>()
        .objectives
        .clear();
    app.update();
    assert!(
        app.world()
            .resource::<NovaOsTerminal>()
            .scrollback()
            .iter()
            .any(|row| row.text == "OBJ x Burn for Beacon 1"),
        "an objective completing while open announces into the scrollback",
    );
}

// CRT screen->image mapping
//
// The math rig. `forward_nova_os_pointer` has to place the forwarded pointer
// on exactly the image texel the shader DISPLAYS under the cursor. The
// reference it is measured against is a hand transcription of the WGSL
// fragment's own sample-UV chain, living in `nova_os_pointer_rig` so the
// live-tree click tests measure against the same independent definition.

/// The viewer contexts follow what owns the monitor, and the per-app one
/// follows WHICH app. This is what makes `map_goto` and `ship_mates` sharing
/// `G` legal - one of them is live at a time and the other cannot hear the
/// key.
///
/// `Viewer` is deliberately down at the prompt: there the keyboard is typing a
/// command, not naming actions, and `W` is a character.
#[test]
fn the_viewer_contexts_follow_the_app_that_owns_the_monitor() {
    use nova_input::prelude::{ActionContext, ActiveContexts};

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.init_state::<PauseStates>();
    app.init_resource::<NovaOsTerminal>();
    app.register_input_actions(crate::bindings::novaos_bindings());
    app.add_systems(Update, crate::terminal::sync_nova_os_contexts);

    let live = |app: &App, context: ActionContext| {
        app.world().resource::<ActiveContexts>().is_live(context)
    };

    app.update();
    assert!(
        !live(&app, ActionContext::Viewer),
        "with the monitor shut nothing in it is listening"
    );

    // The monitor is open, but at the prompt.
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::NovaOs);
    app.update();
    assert!(
        !live(&app, ActionContext::Viewer),
        "at the prompt the keyboard is typing, not naming actions"
    );

    app.world_mut()
        .resource_mut::<NovaOsTerminal>()
        .enter_app("ship");
    app.update();
    assert!(live(&app, ActionContext::Viewer), "the shared verbs answer");
    assert!(live(&app, ActionContext::ViewerApp("ship")));
    assert!(
        !live(&app, ActionContext::ViewerApp("map")),
        "one app owns the screen, so the other app's verbs stay quiet"
    );

    app.world_mut()
        .resource_mut::<NovaOsTerminal>()
        .enter_app("map");
    app.update();
    assert!(live(&app, ActionContext::ViewerApp("map")));
    assert!(!live(&app, ActionContext::ViewerApp("ship")));

    // Closing the monitor lowers everything, app still entered or not.
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Unpaused);
    app.update();
    assert!(!live(&app, ActionContext::Viewer));
    assert!(!live(&app, ActionContext::ViewerApp("map")));
}
