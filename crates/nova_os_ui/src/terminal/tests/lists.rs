//! The combined flight log and the objective row lists.

use super::*;

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
fn nova_os_keeps_log_objective_state_without_visible_panes() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    spawn_nova_os_shell(&mut app);

    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<NovaOsFlightLogListMarker>>()
            .iter(app.world())
            .count(),
        0,
        "the PoC-style monitor does not show a permanent Flight Log pane"
    );
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<NovaOsObjectivesListMarker>>()
            .iter(app.world())
            .count(),
        0,
        "the PoC-style monitor does not show a permanent Objectives pane"
    );

    let mut data_app = objectives_app();
    spawn_flight_log_list(&mut data_app);
    spawn_objectives_list(&mut data_app);
    set_objectives(
        &mut data_app,
        vec![Objective::new("b1", "Strip 3 salvage crates")],
    );
    push_story_line(&mut data_app, "Okono", "Telemetry is dirty but readable.");
    data_app.update();

    assert_eq!(
        flight_log_texts(&mut data_app),
        vec![
            "COMMS OKONO > Telemetry is dirty but readable.".to_string(),
            "OBJ + Strip 3 salvage crates".to_string(),
        ],
        "backing log/objective logic still derives rows for terminal commands"
    );
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
fn nova_os_objectives_section_uses_styled_rows() {
    let mut app = objectives_app();
    set_objectives(
        &mut app,
        vec![
            Objective::new("b1", "Burn for Beacon 1"),
            Objective::new("b2", "Dock at the relay"),
        ],
    );
    let list = spawn_objectives_list(&mut app);
    app.update();

    let rows = row_entities(&mut app);
    let direct_text_children = app
        .world()
        .entity(list)
        .get::<Children>()
        .expect("list children")
        .iter()
        .filter(|child| app.world().entity(*child).contains::<Text>())
        .count();
    assert_eq!(
        direct_text_children, 0,
        "objectives render as row nodes, not direct bare Text children"
    );
    let row_ids: Vec<String> = rows
        .iter()
        .map(|&row| {
            app.world()
                .entity(row)
                .get::<NovaOsObjectiveId>()
                .expect("row id")
                .0
                .clone()
        })
        .collect();
    assert_eq!(
        row_ids,
        vec!["b1".to_string(), "b2".to_string()],
        "the objectives section renders one styled row per active objective"
    );
    for &row in &rows {
        assert_eq!(
            *app.world()
                .entity(row)
                .get::<NovaOsObjectiveRowStatus>()
                .expect("row status"),
            NovaOsObjectiveRowStatus::Active
        );
        assert!(
            app.world().entity(row).get::<BackgroundColor>().is_some(),
            "styled rows carry a fill"
        );
        assert!(
            app.world().entity(row).get::<BorderColor>().is_some(),
            "styled rows carry a border"
        );
        let has_glyph = app
            .world()
            .entity(row)
            .get::<Children>()
            .expect("row children")
            .iter()
            .any(|child| {
                app.world()
                    .entity(child)
                    .contains::<NovaOsObjectiveGlyphMarker>()
            });
        assert!(has_glyph, "styled rows carry a status glyph");
    }
    assert_eq!(row_text(&app, rows[0]), "Burn for Beacon 1");
}

#[test]
fn nova_os_monitor_has_combined_flight_log_stream() {
    let mut app = objectives_app();
    let list = spawn_flight_log_list(&mut app);
    app.update();

    assert!(
        app.world().entity(list).get::<Children>().is_some(),
        "the monitor owns one stream container with an empty row"
    );
    let empty = app
        .world_mut()
        .query_filtered::<Entity, With<NovaOsFlightLogEmptyMarker>>()
        .single(app.world())
        .expect("combined log empty state");
    assert!(
        app.world().entity(empty).get::<BackgroundColor>().is_some(),
        "combined log empty state carries nova_os chrome fill"
    );
}

#[test]
fn nova_os_combined_log_renders_story_feed_rows() {
    let mut app = objectives_app();
    spawn_flight_log_list(&mut app);
    app.update();

    push_story_line(&mut app, "Okono", "Strip it clean.");
    app.update();

    assert_eq!(
        flight_log_texts(&mut app),
        vec!["COMMS OKONO > Strip it clean.".to_string()],
        "story feed lines append as comms rows in the combined stream"
    );
    let icon = app
        .world_mut()
        .query_filtered::<&NovaOsFlightLogIconMarker, With<NovaOsFlightLogRowMarker>>()
        .single(app.world())
        .expect("comms row has an icon marker");
    assert_eq!(icon.kind, NovaOsFlightLogIconKind::Fallback);
}

#[test]
fn nova_os_combined_log_records_objective_events_once() {
    let mut app = objectives_app();
    spawn_flight_log_list(&mut app);
    app.update();

    set_objectives(&mut app, vec![Objective::new("b1", "Burn for Beacon 1")]);
    app.update();
    set_objectives(&mut app, vec![Objective::new("b1", "Recovered: 1/3")]);
    app.update();
    set_objectives(&mut app, Vec::new());
    app.update();

    assert_eq!(
        flight_log_texts(&mut app),
        vec![
            "OBJ + Recovered: 1/3".to_string(),
            "OBJ x Recovered: 1/3".to_string(),
        ],
        "an objective text update edits the posted row rather than appending a duplicate"
    );
}

#[test]
fn nova_os_combined_log_interleaves_comms_and_objective_rows() {
    let mut app = objectives_app();
    spawn_flight_log_list(&mut app);
    app.update();

    push_story_line(&mut app, "Okono", "First transmission.");
    app.update();
    set_objectives(&mut app, vec![Objective::new("b1", "Burn for Beacon 1")]);
    app.update();
    push_story_line(&mut app, "Relay", "Telemetry locked.");
    app.update();
    set_objectives(&mut app, Vec::new());
    app.update();

    assert_eq!(
        flight_log_texts(&mut app),
        vec![
            "COMMS OKONO > First transmission.".to_string(),
            "OBJ + Burn for Beacon 1".to_string(),
            "COMMS RELAY > Telemetry locked.".to_string(),
            "OBJ x Burn for Beacon 1".to_string(),
        ],
        "comms and objective rows share one chronological stream"
    );
}

#[test]
fn nova_os_monitor_shows_only_active_objectives() {
    let mut app = objectives_app();
    set_objectives(
        &mut app,
        vec![
            Objective::new("b1", "Burn for Beacon 1"),
            Objective::new("b2", "Dock at the relay"),
        ],
    );
    spawn_objectives_list(&mut app);
    app.update();

    set_objectives(&mut app, vec![Objective::new("b2", "Dock at the relay")]);
    app.update();

    let rows = row_entities(&mut app);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        *app.world()
            .entity(rows[0])
            .get::<NovaOsObjectiveRowStatus>()
            .expect("row status"),
        NovaOsObjectiveRowStatus::Active
    );
    assert_eq!(row_text(&app, rows[0]), "Dock at the relay");
    assert!(
        app.world_mut()
            .query_filtered::<(), With<NovaOsObjectiveStrikeMarker>>()
            .iter(app.world())
            .next()
            .is_none(),
        "completed objectives are not duplicated as struck-through right-panel rows"
    );
}

#[test]
fn nova_os_final_objective_moves_to_flight_log_only() {
    let mut app = objectives_app();
    set_objectives(&mut app, vec![Objective::new("b1", "Burn for Beacon 1")]);
    spawn_objectives_list(&mut app);
    spawn_flight_log_list(&mut app);
    app.update();

    set_objectives(&mut app, Vec::new());
    app.update();

    assert!(row_entities(&mut app).is_empty());
    assert!(
        app.world_mut()
            .query_filtered::<Entity, With<NovaOsObjectiveEmptyMarker>>()
            .iter(app.world())
            .next()
            .is_some(),
        "the monitor returns to its no-active-objectives empty state"
    );
    assert_eq!(
        flight_log_texts(&mut app),
        vec![
            "OBJ + Burn for Beacon 1".to_string(),
            "OBJ x Burn for Beacon 1".to_string(),
        ],
        "the completed objective remains only in the left Flight Log"
    );
}

#[test]
fn terminal_commands_clear_on_nova_os_teardown() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<NovaOsFlightLog>();
    app.init_resource::<NovaOsTerminal>();
    app.add_observer(setup_nova_os);
    app.add_observer(remove_nova_os);

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

#[test]
fn nova_os_objectives_empty_state_is_styled() {
    let mut app = objectives_app();
    spawn_objectives_list(&mut app);
    app.update();

    let empty = app
        .world_mut()
        .query_filtered::<Entity, With<NovaOsObjectiveEmptyMarker>>()
        .single(app.world())
        .expect("styled empty row");
    assert!(
        app.world().entity(empty).get::<BackgroundColor>().is_some(),
        "empty state carries nova_os chrome fill"
    );
    assert!(
        app.world().entity(empty).get::<BorderColor>().is_some(),
        "empty state carries nova_os chrome border"
    );
}

#[test]
fn nova_os_objectives_rebuild_replaces_stale_rows() {
    let mut app = objectives_app();
    set_objectives(&mut app, vec![Objective::new("b1", "Burn")]);
    spawn_objectives_list(&mut app);
    app.update();
    let first_rows = row_entities(&mut app);
    assert_eq!(first_rows.len(), 1);

    set_objectives(&mut app, vec![Objective::new("b1", "Recovered: 1/3")]);
    app.update();

    let rows = row_entities(&mut app);
    assert_eq!(rows.len(), 1, "old row entity was replaced");
    assert_ne!(rows[0], first_rows[0], "rebuild despawns stale rows");
    assert_eq!(row_text(&app, rows[0]), "Recovered: 1/3");
}
