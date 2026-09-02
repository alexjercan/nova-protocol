//! The combined flight-log model and the NOVA OS scroll viewports.

use super::*;

/// The log entries formatted as the `log` terminal command prints them.
fn flight_log_entry_texts(app: &App) -> Vec<String> {
    app.world()
        .resource::<NovaOsFlightLog>()
        .entries
        .iter()
        .map(nova_os_flight_log_text)
        .collect()
}

#[test]
fn nova_os_terminal_scrollback_lives_in_scrollable_viewport() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    spawn_nova_os_shell(&mut app);

    let list = app
        .world_mut()
        .query_filtered::<Entity, With<NovaOsTerminalScrollbackMarker>>()
        .single(app.world())
        .expect("terminal scrollback viewport");

    assert_scrollable_viewport(&app, list, "terminal scrollback viewport");
}

#[test]
fn nova_os_wheel_scrolls_viewports_and_clamps_at_top() {
    use bevy::input::mouse::{MouseScrollUnit, MouseWheel};

    let scroll_after = |start_y: f32, wheel_y: f32| -> f32 {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut().init_resource::<Messages<MouseWheel>>();
        app.world_mut().spawn((
            NovaOsScrollViewportMarker,
            ScrollPosition(Vec2::new(0.0, start_y)),
        ));
        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: wheel_y,
            window: Entity::PLACEHOLDER,
            phase: TouchPhase::Moved,
        });
        app.world_mut()
            .run_system_once(scroll_nova_os_panels)
            .expect("nova_os scroll system runs");
        app.world_mut()
            .query::<&ScrollPosition>()
            .single(app.world())
            .expect("one scroll position")
            .0
            .y
    };

    assert!(
        scroll_after(0.0, -1.0) > 0.0,
        "wheel down from the top scrolls the NOVA OS panel down"
    );
    assert_eq!(
        scroll_after(12.0, 1.0),
        0.0,
        "wheel up clamps at the top instead of going negative"
    );
}

#[test]
fn nova_os_wheel_scroll_clamps_at_content_bottom() {
    use bevy::input::mouse::{MouseScrollUnit, MouseWheel};

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.world_mut().init_resource::<Messages<MouseWheel>>();
    let viewport = app
        .world_mut()
        .spawn((
            NovaOsScrollViewportMarker,
            ScrollPosition(Vec2::new(0.0, 95.0)),
            ComputedNode {
                size: Vec2::new(100.0, 100.0),
                content_size: Vec2::new(100.0, 200.0),
                scrollbar_size: Vec2::ZERO,
                ..default()
            },
        ))
        .id();

    app.world_mut().write_message(MouseWheel {
        unit: MouseScrollUnit::Line,
        x: 0.0,
        y: -1.0,
        window: Entity::PLACEHOLDER,
        phase: TouchPhase::Moved,
    });
    app.world_mut()
        .run_system_once(scroll_nova_os_panels)
        .expect("nova_os scroll system runs");

    assert_eq!(
        app.world()
            .entity(viewport)
            .get::<ScrollPosition>()
            .unwrap()
            .0
            .y,
        100.0,
        "stored nova_os scroll offset clamps to the content bottom"
    );
}

#[test]
fn nova_os_wheel_scrolls_only_hovered_viewport_when_one_is_hovered() {
    use bevy::input::mouse::{MouseScrollUnit, MouseWheel};

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.world_mut().init_resource::<Messages<MouseWheel>>();
    let hovered = app
        .world_mut()
        .spawn((
            NovaOsScrollViewportMarker,
            Hovered(true),
            ScrollPosition(Vec2::ZERO),
        ))
        .id();
    let not_hovered = app
        .world_mut()
        .spawn((
            NovaOsScrollViewportMarker,
            Hovered(false),
            ScrollPosition(Vec2::ZERO),
        ))
        .id();

    app.world_mut().write_message(MouseWheel {
        unit: MouseScrollUnit::Line,
        x: 0.0,
        y: -1.0,
        window: Entity::PLACEHOLDER,
        phase: TouchPhase::Moved,
    });
    app.world_mut()
        .run_system_once(scroll_nova_os_panels)
        .expect("nova_os scroll system runs");

    let hovered_y = app
        .world()
        .entity(hovered)
        .get::<ScrollPosition>()
        .unwrap()
        .0
        .y;
    let not_hovered_y = app
        .world()
        .entity(not_hovered)
        .get::<ScrollPosition>()
        .unwrap()
        .0
        .y;
    assert!(
        hovered_y > 0.0,
        "the hovered viewport receives the wheel scroll"
    );
    assert_eq!(
        not_hovered_y, 0.0,
        "a non-hovered viewport does not scroll when another nova_os viewport is hovered"
    );
}

#[test]
fn flight_log_records_story_feed_comms() {
    let mut app = objectives_app();
    push_story_line(&mut app, "Alpha", "Strip it clean.");
    app.update();

    assert_eq!(
        flight_log_entry_texts(&app),
        vec!["COMMS ALPHA > Strip it clean.".to_string()],
        "story feed lines append as comms entries in the combined log"
    );
    assert_eq!(
        app.world().resource::<NovaOsFlightLog>().entries[0].kind,
        NovaOsFlightLogEntryKind::Comms
    );
}

#[test]
fn flight_log_records_objective_events_once() {
    let mut app = objectives_app();
    set_objectives(&mut app, vec![Objective::new("b1", "Burn for Beacon 1")]);
    app.update();
    set_objectives(&mut app, vec![Objective::new("b1", "Recovered: 1/3")]);
    app.update();
    set_objectives(&mut app, Vec::new());
    app.update();

    assert_eq!(
        flight_log_entry_texts(&app),
        vec![
            "OBJ + Recovered: 1/3".to_string(),
            "OBJ x Recovered: 1/3".to_string(),
        ],
        "an objective text update edits the posted entry rather than appending a duplicate"
    );
}

#[test]
fn flight_log_interleaves_comms_and_objective_entries() {
    let mut app = objectives_app();
    push_story_line(&mut app, "Alpha", "First transmission.");
    app.update();
    set_objectives(&mut app, vec![Objective::new("b1", "Burn for Beacon 1")]);
    app.update();
    push_story_line(&mut app, "Relay", "Telemetry locked.");
    app.update();
    set_objectives(&mut app, Vec::new());
    app.update();

    assert_eq!(
        flight_log_entry_texts(&app),
        vec![
            "COMMS ALPHA > First transmission.".to_string(),
            "OBJ + Burn for Beacon 1".to_string(),
            "COMMS RELAY > Telemetry locked.".to_string(),
            "OBJ x Burn for Beacon 1".to_string(),
        ],
        "comms and objective entries share one chronological stream"
    );
}

#[test]
fn terminal_commands_clear_on_nova_os_teardown() {
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
    {
        let mut log = app.world_mut().resource_mut::<NovaOsFlightLog>();
        log.entries.push(NovaOsFlightLogEntry {
            kind: NovaOsFlightLogEntryKind::ObjectiveCompleted,
            objective_id: Some("b1".to_string()),
            speaker: None,
            message: "Burn for Beacon 1".to_string(),
            icon: None,
        });
        log.previous_active = vec![Objective::new("b2", "Dock at the relay")];
        log.seen_story = 1;
    }
    {
        let mut terminal = app.world_mut().resource_mut::<NovaOsTerminal>();
        type_text(&mut terminal, "log");
        terminal.submit(&TerminalCommandSnapshot::default().with_output(
            "log",
            vec![TerminalRow {
                kind: TerminalRowKind::Output,
                text: "OBJ x Burn for Beacon 1".to_string(),
            }],
        ));
        assert!(
            terminal
                .scrollback()
                .iter()
                .any(|row| row.text.contains("Burn for Beacon 1")),
            "delivery guard: terminal contains scenario output before teardown"
        );
    }

    app.world_mut()
        .entity_mut(player)
        .remove::<PlayerSpaceshipMarker>();
    app.update();

    let log = app.world().resource::<NovaOsFlightLog>();
    assert!(
        log.entries.is_empty() && log.previous_active.is_empty() && log.seen_story == 0,
        "nova_os teardown clears the retained left-panel log"
    );
    assert_eq!(
        app.world().resource::<NovaOsTerminal>().scrollback(),
        nova_os_welcome_rows(),
        "nova_os teardown clears printed command output before the next player ship"
    );
}

/// Every drop branch reaches the log the player reads it in, each saying which
/// branch it was. The reason is the whole point: an intended 30 s decay and a
/// target that died read as the same disappearing lock without it.
#[test]
fn a_dropped_combat_lock_says_why_in_the_flight_log() {
    use nova_ship::prelude::{CombatLockDrop, CombatLockDropped};

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<NovaOsFlightLog>();
    app.add_message::<CombatLockDropped>();
    app.add_systems(Update, log_combat_lock_drops);
    app.update();
    assert!(flight_log_entry_texts(&app).is_empty(), "nothing yet");

    let target = app.world_mut().spawn_empty().id();
    for (reason, idle_secs) in [
        (CombatLockDrop::TargetGone, 0.0),
        (CombatLockDrop::OutOfRange, 1.0),
        (CombatLockDrop::AllegianceFlip, 2.0),
        (CombatLockDrop::IdleDecay, 30.0),
    ] {
        app.world_mut().write_message(CombatLockDropped {
            target,
            reason,
            idle_secs,
        });
    }
    app.update();

    let lines = flight_log_entry_texts(&app);
    assert_eq!(lines.len(), 4, "one line per drop: {lines:?}");
    assert!(
        lines.iter().all(|line| line.starts_with("SYS ! ")),
        "the ship reports these, nobody says them: {lines:?}"
    );
    assert!(lines[0].contains("target is gone"), "{}", lines[0]);
    assert!(lines[1].contains("out of lock range"), "{}", lines[1]);
    assert!(lines[2].contains("no longer hostile"), "{}", lines[2]);
    assert!(
        lines[3].contains("30 s without combat"),
        "the decay names the clock that ran out: {}",
        lines[3]
    );

    app.update();
    assert_eq!(
        flight_log_entry_texts(&app).len(),
        4,
        "a drained message does not log again"
    );
}
