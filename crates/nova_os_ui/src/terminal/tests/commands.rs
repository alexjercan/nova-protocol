//! The read-only terminal commands and the prompt's parse feedback.

use super::*;

#[test]
fn terminal_log_command_prints_flight_log_rows() {
    let mut log = NovaOsFlightLog::default();
    log.entries.push(NovaOsFlightLogEntry {
        kind: NovaOsFlightLogEntryKind::Comms,
        objective_id: None,
        speaker: Some("Control".to_string()),
        message: "Hold course.".to_string(),
        icon: None,
    });
    log.entries.push(NovaOsFlightLogEntry {
        kind: NovaOsFlightLogEntryKind::ObjectivePosted,
        objective_id: Some("burn".to_string()),
        speaker: None,
        message: "Burn for Beacon 1".to_string(),
        icon: None,
    });
    log.entries.push(NovaOsFlightLogEntry {
        kind: NovaOsFlightLogEntryKind::ObjectiveCompleted,
        objective_id: Some("burn".to_string()),
        speaker: None,
        message: "Burn for Beacon 1".to_string(),
        icon: None,
    });
    let snapshot = terminal_snapshot_from_world(&log, &GameObjectives::default(), None, &[], 0);
    let mut terminal = NovaOsTerminal::default();

    type_text(&mut terminal, "log");
    terminal.submit(&snapshot);

    let printed: Vec<&str> = terminal
        .scrollback()
        .iter()
        .map(|row| row.text.as_str())
        .collect();
    // HTML-style numbered rows, no header.
    assert!(!printed.contains(&"Flight log:"));
    assert!(printed.contains(&"0001 COMMS CONTROL > Hold course."));
    assert!(printed.contains(&"0002 OBJ + Burn for Beacon 1"));
    assert!(printed.contains(&"0003 OBJ x Burn for Beacon 1"));
}

#[test]
fn terminal_objectives_command_prints_active_objectives() {
    let objectives = GameObjectives {
        objectives: vec![
            Objective::new("beacon", "Recover the beacon"),
            Objective::new("dock", "Dock at the relay"),
        ],
    };
    let snapshot =
        terminal_snapshot_from_world(&NovaOsFlightLog::default(), &objectives, None, &[], 0);
    let mut terminal = NovaOsTerminal::default();

    type_text(&mut terminal, "objectives");
    terminal.submit(&snapshot);

    let printed: Vec<&str> = terminal
        .scrollback()
        .iter()
        .map(|row| row.text.as_str())
        .collect();
    // HTML-style `OBJ + <message>` rows, no header.
    assert!(!printed.contains(&"Active objectives:"));
    assert!(printed.contains(&"OBJ + Recover the beacon"));
    assert!(printed.contains(&"OBJ + Dock at the relay"));

    let empty_snapshot = terminal_snapshot_from_world(
        &NovaOsFlightLog::default(),
        &GameObjectives::default(),
        None,
        &[],
        0,
    );
    let mut empty = NovaOsTerminal::default();
    type_text(&mut empty, "objectives");
    empty.submit(&empty_snapshot);
    assert_eq!(
        empty.scrollback().last().map(|row| row.text.as_str()),
        Some("No active objectives.")
    );
}

#[test]
fn terminal_objectives_command_reads_live_resource_updates() {
    let mut app = terminal_command_app();
    set_objectives(
        &mut app,
        vec![Objective::new("beacon", "Recover the beacon")],
    );

    submit_terminal_command(&mut app, "objectives");
    assert!(
        terminal_scrollback_texts(&app)
            .iter()
            .any(|row| row == "OBJ + Recover the beacon"),
        "first command submit reads the current objective resource"
    );

    set_objectives(&mut app, vec![Objective::new("dock", "Dock at the relay")]);
    submit_terminal_command(&mut app, "objectives");

    let printed = terminal_scrollback_texts(&app);
    assert!(
        printed.iter().any(|row| row == "OBJ + Dock at the relay"),
        "second command submit reads the changed objective resource"
    );
}

#[test]
fn ship_view_rows_format_section_status() {
    // `ship view` prints a column-aligned table of section CODES (labels), not
    // the freeform names. This exercises the row formatter directly.
    let sections = vec![
        ShipSectionStatus {
            code: "THR-1".to_string(),
            kind: SectionDamageClass::Thruster,
            health: Some(Health {
                current: 18.0,
                max: 100.0,
            }),
            inactive: false,
            zero_health: false,
            ammo: None,
        },
        ShipSectionStatus {
            code: "PDC-1".to_string(),
            kind: SectionDamageClass::Turret,
            health: Some(Health {
                current: 0.0,
                max: 60.0,
            }),
            inactive: true,
            zero_health: true,
            ammo: Some(SectionAmmo {
                rounds: 2,
                capacity: 6,
            }),
        },
    ];
    let printed: Vec<String> = terminal_ship_rows(Some("Rust Tally"), &sections)
        .into_iter()
        .map(|row| row.text)
        .collect();

    // Preamble + header.
    assert!(printed.iter().any(|r| r == "SHIP RUST TALLY"));
    assert!(printed.iter().any(|r| r == "Sections: 2"));
    let header = printed
        .iter()
        .find(|r| r.starts_with("KIND"))
        .expect("a KIND/LABEL/INFO header row");
    assert!(header.contains("LABEL") && header.contains("INFO"));

    // Rows show the CODE label (kind column is padded to the widest, "THRUSTER").
    let thruster = printed
        .iter()
        .find(|r| r.starts_with("THRUSTER"))
        .expect("thruster row");
    let turret = printed
        .iter()
        .find(|r| r.starts_with("TURRET"))
        .expect("turret row");
    assert_eq!(thruster, "THRUSTER  THR-1  18/100 HP  [critical]");
    assert_eq!(turret, "TURRET    PDC-1  0/60 HP; ammo 2/6  [neutralized]");

    // The freeform name is gone, and the separate `status:` sub-row is gone -
    // the status now rides the INFO column.
    assert!(!printed
        .iter()
        .any(|r| r.contains("engine") || r.contains("gun")));
    assert!(!printed
        .iter()
        .any(|r| r.trim_start().starts_with("status:")));

    // Columns line up: the LABEL token starts at the SAME character offset in
    // the header and every data row (fails if the padding is wrong).
    let label_col = header.find("LABEL").unwrap();
    assert_eq!(thruster.find("THR-1"), Some(label_col));
    assert_eq!(turret.find("PDC-1"), Some(label_col));
}

/// Register the `ship` app tree (launch word + `ship view` subcommand) into a
/// terminal test harness so `ship view` resolves without the full ship plugin.
#[test]
fn terminal_ship_command_reads_live_player_sections() {
    let mut app = terminal_command_app();
    register_ship_view_command(&mut app);
    let ship = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Name::new("Rust Tally"),
        ))
        .id();
    let thruster = app
        .world_mut()
        .spawn((
            SectionMarker,
            ThrusterSectionMarker,
            SectionDamageClass::Thruster,
            // The minted code is the LABEL the table must show - proving it is
            // threaded from the ECS `SectionCode` component to the scrollback.
            SectionCode("THR-1".to_string()),
            Health {
                current: 18.0,
                max: 100.0,
            },
            ChildOf(ship),
            Name::new("Port engine"),
        ))
        .id();
    // The turret carries NO SectionCode, so its label falls back to the kind.
    app.world_mut().spawn((
        SectionMarker,
        TurretSectionMarker,
        Health {
            current: 0.0,
            max: 60.0,
        },
        SectionInactiveMarker,
        HealthZeroMarker,
        SectionAmmo {
            rounds: 2,
            capacity: 6,
        },
        ChildOf(ship),
        Name::new("Bow gun"),
    ));

    submit_terminal_command(&mut app, "ship view");
    let printed = terminal_scrollback_texts(&app);
    assert!(printed.iter().any(|row| row == "SHIP RUST TALLY"));
    let header = printed
        .iter()
        .find(|r| r.starts_with("KIND"))
        .expect("a KIND/LABEL/INFO header row");

    // WIRING: the thruster row shows the code from its `SectionCode` component,
    // not the freeform name.
    let thruster_row = printed
        .iter()
        .find(|r| r.starts_with("THRUSTER"))
        .expect("thruster row");
    assert!(
        thruster_row.contains("THR-1")
            && thruster_row.contains("18/100 HP")
            && thruster_row.contains("[critical]"),
        "thruster row shows the live code label + status: {thruster_row:?}"
    );
    // The codeless turret falls back to the kind label.
    let turret_row = printed
        .iter()
        .find(|r| r.starts_with("TURRET"))
        .expect("turret row");
    assert!(
        turret_row.contains("0/60 HP; ammo 2/6") && turret_row.contains("[neutralized]"),
        "turret row: {turret_row:?}"
    );
    // Names are gone; columns line up under the header.
    assert!(!printed
        .iter()
        .any(|r| r.contains("engine") || r.contains("gun")));
    let label_col = header.find("LABEL").unwrap();
    assert_eq!(thruster_row.find("THR-1"), Some(label_col));

    app.world_mut().entity_mut(thruster).insert(Health {
        current: 80.0,
        max: 100.0,
    });
    submit_terminal_command(&mut app, "ship view");
    assert!(
        terminal_scrollback_texts(&app).iter().any(|row| {
            row.starts_with("THRUSTER") && row.contains("80/100 HP") && !row.contains("[critical]")
        }),
        "second command submit reads changed live section health (now nominal)"
    );
}

#[test]
fn terminal_ui_renders_prompt_hint_and_invalid_coloring() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<NovaOsTerminal>();
    spawn_nova_os_shell(&mut app);
    {
        let mut terminal = app.world_mut().resource_mut::<NovaOsTerminal>();
        terminal.insert_text("hlep");
    }

    app.world_mut()
        .run_system_once(rebuild_terminal_ui)
        .expect("terminal UI rebuild runs");

    let (prompt, prompt_color, prompt_node) = app
        .world_mut()
        .query_filtered::<(&Text, &TextColor, &Node), With<NovaOsTerminalPromptMarker>>()
        .single(app.world())
        .expect("one terminal prompt");
    assert_eq!(prompt.0, "hlep");
    assert_eq!(prompt_color.0, theme::semantic::THREAT);
    assert_eq!(
        prompt_node.flex_shrink, 0.0,
        "typed input must not collapse inside the prompt row"
    );
    // The terminal text carries no per-glyph shadow (crisp phosphor).
    assert!(
        app.world_mut()
            .query_filtered::<&TextShadow, With<NovaOsTerminalPromptMarker>>()
            .iter(app.world())
            .next()
            .is_none(),
        "terminal prompt text has no shadow/bloom glyph"
    );

    let (ghost, ghost_node) = app
        .world_mut()
        .query_filtered::<(&Text, &Node), With<NovaOsTerminalGhostMarker>>()
        .single(app.world())
        .expect("one terminal autocomplete ghost");
    assert_eq!(ghost.0, "");
    assert_eq!(
        ghost_node.flex_shrink, 0.0,
        "autocomplete ghost must not collapse the typed input"
    );
    assert_eq!(
        ghost_node.position_type,
        PositionType::Relative,
        "autocomplete ghost stays inline after the visible prompt text"
    );

    let (prefix, prefix_color) = app
        .world_mut()
        .query_filtered::<(&Text, &TextColor), With<NovaOsPromptPrefixMarker>>()
        .single(app.world())
        .expect("one terminal prompt prefix");
    assert_eq!(prefix.0, "nova>");
    assert_eq!(prefix_color.0, NOVA_OS_AMBER);

    let (hint, hint_color, hint_node) = app
        .world_mut()
        .query_filtered::<(&Text, &TextColor, &Node), With<NovaOsTerminalHintMarker>>()
        .single(app.world())
        .expect("one terminal hint");
    assert_eq!(hint.0, "did you mean help?");
    assert_eq!(hint_color.0, theme::semantic::THREAT);
    assert_eq!(
        hint_node.width,
        Val::Percent(100.0),
        "invalid-command suggestions live below the input line instead of stealing prompt width"
    );

    let input_wrap = app
        .world_mut()
        .query_filtered::<&Node, With<NovaOsPromptInputWrapMarker>>()
        .single(app.world())
        .expect("one prompt input wrap");
    assert_eq!(input_wrap.flex_grow, 1.0);
    assert_eq!(
        input_wrap.overflow,
        Overflow::clip_x(),
        "typed input owns the prompt lane and clips inside it"
    );
}
