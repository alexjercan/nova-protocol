//! The combined flight-log model and the modal's scroll viewports.

use super::*;

/// The logged entries as kind, speaker and message.
fn flight_log_entries(app: &App) -> Vec<(NovaOsFlightLogEntryKind, Option<String>, String)> {
    app.world()
        .resource::<NovaOsFlightLog>()
        .entries
        .iter()
        .map(|entry| (entry.kind, entry.speaker.clone(), entry.message.clone()))
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
        "wheel down from the top scrolls the modal panel down"
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
        flight_log_entries(&app),
        vec![(
            NovaOsFlightLogEntryKind::Comms,
            Some("Alpha".to_string()),
            "Strip it clean.".to_string(),
        )],
        "story feed lines append as comms entries in the combined log"
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
        flight_log_entries(&app),
        vec![
            (
                NovaOsFlightLogEntryKind::ObjectivePosted,
                None,
                "Recovered: 1/3".to_string(),
            ),
            (
                NovaOsFlightLogEntryKind::ObjectiveCompleted,
                None,
                "Recovered: 1/3".to_string(),
            ),
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
        flight_log_entries(&app),
        vec![
            (
                NovaOsFlightLogEntryKind::Comms,
                Some("Alpha".to_string()),
                "First transmission.".to_string(),
            ),
            (
                NovaOsFlightLogEntryKind::ObjectivePosted,
                None,
                "Burn for Beacon 1".to_string(),
            ),
            (
                NovaOsFlightLogEntryKind::Comms,
                Some("Relay".to_string()),
                "Telemetry locked.".to_string(),
            ),
            (
                NovaOsFlightLogEntryKind::ObjectiveCompleted,
                None,
                "Burn for Beacon 1".to_string(),
            ),
        ],
        "comms and objective entries share one chronological stream"
    );
}

#[test]
fn the_flight_log_clears_when_the_player_ship_goes() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<NovaOsFlightLog>();
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
    app.world_mut()
        .entity_mut(player)
        .remove::<PlayerSpaceshipMarker>();
    app.update();

    let log = app.world().resource::<NovaOsFlightLog>();
    assert!(
        log.entries.is_empty() && log.previous_active.is_empty() && log.seen_story == 0,
        "losing the player ship clears the ship-scoped flight log"
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
    assert!(flight_log_entries(&app).is_empty(), "nothing yet");

    let target = app.world_mut().spawn_empty().id();
    for reason in [
        CombatLockDrop::TargetGone,
        CombatLockDrop::OutOfRange,
        CombatLockDrop::AllegianceFlip,
        CombatLockDrop::Occluded,
    ] {
        app.world_mut()
            .write_message(CombatLockDropped { target, reason });
    }
    app.update();

    let entries = flight_log_entries(&app);
    assert_eq!(entries.len(), 4, "one line per drop: {entries:?}");
    assert!(
        entries.iter().all(|(kind, speaker, _)| {
            *kind == NovaOsFlightLogEntryKind::System && speaker.is_none()
        }),
        "the ship reports these, nobody says them: {entries:?}"
    );
    let lines: Vec<&str> = entries
        .iter()
        .map(|(_, _, message)| message.as_str())
        .collect();
    assert!(lines[0].contains("target is gone"), "{}", lines[0]);
    assert!(lines[1].contains("out of lock range"), "{}", lines[1]);
    assert!(lines[2].contains("no longer hostile"), "{}", lines[2]);
    assert!(lines[3].contains("behind cover"), "{}", lines[3]);

    app.update();
    assert_eq!(
        flight_log_entries(&app).len(),
        4,
        "a drained message does not log again"
    );
}
