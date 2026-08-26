//! The spawned shell tree: casing, layering and the persistent header.

use super::*;

/// The open NOVA OS is a modal: its monitor and backdrop must carry an explicit
/// `GlobalZIndex` above the HUD chrome (which carries none = 0), or the
/// top-right objectives panel and other flight HUD draw over it. Mirrors
/// nova_menu's overlay-z assertion. Fails before the fix (no `GlobalZIndex`).
#[test]
fn nova_os_renders_above_the_hud() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_observer(setup_nova_os);
    // setup_nova_os fires on the player ship's PlayerSpaceshipMarker add.
    app.world_mut()
        .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
    app.update();

    let backdrop_z = app
        .world_mut()
        .query_filtered::<&GlobalZIndex, With<NovaOsBackdropMarker>>()
        .single(app.world())
        .expect("the NOVA OS backdrop carries an explicit GlobalZIndex")
        .0;
    assert!(
        backdrop_z > 0,
        "the backdrop must stack above the HUD chrome (z = {backdrop_z})"
    );
    let monitor_zs: Vec<i32> = app
        .world_mut()
        .query_filtered::<&GlobalZIndex, With<NovaOsMonitorMarker>>()
        .iter(app.world())
        .map(|z| z.0)
        .collect();
    assert_eq!(
        monitor_zs.len(),
        1,
        "the shell spawns one NOVA OS monitor, not left/right panels"
    );
    assert!(
        monitor_zs[0] >= backdrop_z,
        "the monitor sits at or above the backdrop (monitor {}, backdrop {backdrop_z})",
        monitor_zs[0]
    );
    // Diagnostic NOVA OS-exempt chrome must out-rank the backdrop so the
    // deepened gray field cannot dim it.
    assert!(
        DRAWER_EXEMPT_Z > backdrop_z,
        "exempt chrome z ({DRAWER_EXEMPT_Z}) must beat the backdrop ({backdrop_z})"
    );
}

/// The shell builds one inset physical monitor with the CRT layers the
/// follow-up terminal tasks can fill, not two permanent side panels.
#[test]
fn nova_os_spawns_single_nova_os_monitor() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_observer(setup_nova_os);
    app.world_mut()
        .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
    app.update();

    let monitors: Vec<Node> = app
        .world_mut()
        .query_filtered::<&Node, With<NovaOsMonitorMarker>>()
        .iter(app.world())
        .cloned()
        .collect();
    assert_eq!(monitors.len(), 1);
    let monitor = &monitors[0];
    assert_eq!(monitor.position_type, PositionType::Absolute);
    assert_eq!(monitor.top, Val::Px(NOVA_OS_MONITOR_INSET_Y_PX));
    assert_eq!(monitor.bottom, Val::Px(NOVA_OS_MONITOR_INSET_Y_PX));
    assert_eq!(monitor.left, Val::Px(NOVA_OS_MONITOR_INSET_X_PX));
    assert_eq!(monitor.right, Val::Px(NOVA_OS_MONITOR_INSET_X_PX));
    let extra_roots = app
        .world_mut()
        .query_filtered::<(), (With<NovaOsRootMarker>, Without<NovaOsMonitorMarker>)>()
        .iter(app.world())
        .count();
    assert_eq!(extra_roots, 0, "there are no leftover side-panel roots");
    assert!(
        app.world_mut()
            .query_filtered::<(), With<NovaOsBezelMarker>>()
            .iter(app.world())
            .next()
            .is_some(),
        "monitor has a physical bezel"
    );
    assert!(
        app.world_mut()
            .query_filtered::<(), With<NovaOsScreenMarker>>()
            .iter(app.world())
            .next()
            .is_some(),
        "monitor has an inset phosphor screen"
    );
    // The CRT treatment is the render-to-texture sampling shader, not an
    // overlay node. Headless (no image/material assets) this rig falls back to
    // the terminal directly on the screen with no sampling surface; the
    // sampling surface is asserted by `nova_os_screen_samples_offscreen_image`
    // under the with-CRT harness.
    assert!(
        app.world_mut()
            .query_filtered::<(), With<NovaOsTerminalContentMarker>>()
            .iter(app.world())
            .next()
            .is_some(),
        "headless fallback renders the terminal directly on the screen"
    );
}

/// The casing + glass depth pass gives the monitor its
/// physical details: rounded casing/bezel/screen, the moulding seam, four
/// corner screws, the vent strip, and the chin bar carrying the brand plate
/// and a reserved (empty) controls slot.
#[test]
fn nova_os_monitor_has_physical_casing_details() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_observer(setup_nova_os);
    app.world_mut()
        .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
    app.update();

    // Rounded casing stack: asymmetric shell corners, rounded bezel + screen.
    let monitor = app
        .world_mut()
        .query_filtered::<&Node, With<NovaOsMonitorMarker>>()
        .single(app.world())
        .expect("one monitor")
        .clone();
    assert_eq!(
        monitor.border_radius.top_left,
        Val::Px(NOVA_OS_CASE_RADIUS_TOP_PX),
        "casing has the larger top corner radius"
    );
    assert_eq!(
        monitor.border_radius.bottom_left,
        Val::Px(NOVA_OS_CASE_RADIUS_BOTTOM_PX),
        "casing has the tighter bottom corner radius"
    );
    let bezel = app
        .world_mut()
        .query_filtered::<&Node, With<NovaOsBezelMarker>>()
        .single(app.world())
        .expect("one bezel")
        .clone();
    assert_eq!(
        bezel.border_radius.top_left,
        Val::Px(NOVA_OS_BEZEL_RADIUS_PX)
    );
    let screen = app
        .world_mut()
        .query_filtered::<&Node, With<NovaOsScreenMarker>>()
        .single(app.world())
        .expect("one screen")
        .clone();
    assert_eq!(
        screen.border_radius.top_left,
        Val::Px(NOVA_OS_SCREEN_RADIUS_PX)
    );

    // The screen edge is a dark recess line, NOT the old flat bright-phosphor
    // frame: the crisp glowing edge now comes from the shader's barrel-bowed
    // rim (feedback item 2 / DECISION.md). This pins the demotion so the flat
    // straight frame cannot silently return.
    let screen_border = app
        .world_mut()
        .query_filtered::<&BorderColor, With<NovaOsScreenMarker>>()
        .single(app.world())
        .expect("one screen");
    assert_eq!(
        screen_border.top,
        NOVA_OS_CASE_EDGE.with_alpha(0.85),
        "the screen edge is a dark recess line"
    );
    assert_ne!(
        screen_border.top,
        NOVA_OS_PHOSPHOR.with_alpha(0.52),
        "the flat bright-phosphor screen frame is gone"
    );

    // Four moulded corner screws.
    assert_eq!(
        app.world_mut()
            .query_filtered::<(), With<NovaOsScrewMarker>>()
            .iter(app.world())
            .count(),
        4,
        "four corner screws"
    );

    // Single-instance detail nodes: vent strip, moulding seam, chin, plate,
    // and the reserved controls row.
    for (count, expected, label) in [
        (
            app.world_mut()
                .query_filtered::<(), With<NovaOsVentMarker>>()
                .iter(app.world())
                .count(),
            1,
            "vent strip",
        ),
        (
            app.world_mut()
                .query_filtered::<(), With<NovaOsSeamMarker>>()
                .iter(app.world())
                .count(),
            1,
            "moulding seam",
        ),
        (
            app.world_mut()
                .query_filtered::<(), With<NovaOsChinMarker>>()
                .iter(app.world())
                .count(),
            1,
            "chin bar",
        ),
        (
            app.world_mut()
                .query_filtered::<(), With<NovaOsBrandPlateMarker>>()
                .iter(app.world())
                .count(),
            1,
            "brand plate",
        ),
        (
            app.world_mut()
                .query_filtered::<(), With<NovaOsControlsRowMarker>>()
                .iter(app.world())
                .count(),
            1,
            "reserved controls row",
        ),
    ] {
        assert_eq!(count, expected, "monitor has exactly one {label}");
    }

    // The brand plate carries the stamped wordmark + spec line.
    let plate_texts: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    assert!(
        plate_texts.iter().any(|t| t.contains("NOVACRT 9000")),
        "brand plate shows the NovaCRT 9000 wordmark"
    );
    assert!(
        plate_texts.iter().any(|t| t.contains("P22 GREEN PHOSPHOR")),
        "brand plate shows the phosphor spec line"
    );

    // The phosphor rim (glow + line) and the glass sheen trace the screen.
    assert_eq!(
        app.world_mut()
            .query_filtered::<(), With<NovaOsPhosphorRimMarker>>()
            .iter(app.world())
            .count(),
        2,
        "phosphor rim has a glow + line pair"
    );
    assert!(
        app.world_mut()
            .query_filtered::<(), With<NovaOsGlassMarker>>()
            .iter(app.world())
            .next()
            .is_some(),
        "screen has a glass sheen layer"
    );
}

#[test]
fn nova_os_matches_nova_os_terminal_poc_structure() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    spawn_nova_os_shell(&mut app);

    assert!(
        app.world_mut()
            .query_filtered::<(), With<NovaOsTopbarMarker>>()
            .iter(app.world())
            .next()
            .is_some(),
        "screen has the PoC topbar"
    );
    assert!(
        app.world_mut()
            .query_filtered::<(), With<NovaOsLampMarker>>()
            .iter(app.world())
            .next()
            .is_some(),
        "topbar has the lit status lamp"
    );
    assert!(
        app.world_mut()
            .query_filtered::<(), With<NovaOsStatusMarker>>()
            .iter(app.world())
            .next()
            .is_some(),
        "topbar has the right-side status text"
    );
    assert!(
        app.world_mut()
            .query_filtered::<(), With<NovaOsTerminalSurfaceMarker>>()
            .iter(app.world())
            .next()
            .is_some(),
        "screen has one terminal surface"
    );
    assert!(
        app.world_mut()
            .query_filtered::<(), With<NovaOsPromptRowMarker>>()
            .iter(app.world())
            .next()
            .is_some(),
        "terminal surface has the PoC prompt row"
    );
    assert!(
        app.world_mut()
            .query_filtered::<(), With<NovaOsPromptInputLineMarker>>()
            .iter(app.world())
            .next()
            .is_some(),
        "prompt strip has a dedicated input line"
    );
    assert!(
        app.world_mut()
            .query_filtered::<(), With<NovaOsPromptInputWrapMarker>>()
            .iter(app.world())
            .next()
            .is_some(),
        "prompt strip has a full-width input wrap like the HTML PoC"
    );
    assert!(
        app.world_mut()
            .query_filtered::<(), With<NovaOsFooterHintsMarker>>()
            .iter(app.world())
            .next()
            .is_some(),
        "screen has the PoC footer hint row"
    );

    let texts = all_texts(&mut app);
    for expected in [
        // The header brand shows the SHELL breadcrumb at the prompt (this
        // task); an open app swaps it for `APPS / <ID>`.
        format!("NOVA OS {} // SHELL", nova_os_version_label()),
        // The header carries the ship/link head plus a live FPS segment; it
        // spawns with a fixed-width `--` placeholder before the diagnostic has
        // a reading.
        "SHIP: SURVEY CUTTER     LINK: LOCAL     FPS:  --".to_string(),
        format!("NOVA OS {}", nova_os_version_label()),
        "POST ......... flight computer / ok".to_string(),
        "CORE ......... 64K static / ok".to_string(),
        "DISPLAY ...... green phosphor crt / warm".to_string(),
        "LINK ......... cockpit bus / local".to_string(),
        "Hint: type `help` and press Enter.".to_string(),
        "nova>".to_string(),
        // The footer lists the full current keybind set.
        "TAB: COMPLETE".to_string(),
        "UP/DN: HISTORY".to_string(),
        "PGUP/PGDN: SCROLL".to_string(),
        "ESC: CLOSE".to_string(),
    ] {
        assert!(
            texts.iter().any(|text| text == &expected),
            "missing PoC text: {expected}"
        );
    }
    assert!(
        !texts.iter().any(|text| text.contains("DRAWER PAUSED")),
        "the topbar should not repeat a useless paused label"
    );
    assert!(
        !texts
            .iter()
            .any(|text| text == "FLIGHT LOG" || text == "OBJECTIVES"),
        "NOVA OS no longer renders permanent side-panel headings inside the screen"
    );
}

#[test]
fn topbar_status_line_carries_a_live_fps_segment() {
    // The pure line builder appends the FPS segment after the ship/link head,
    // with a fixed-width `--` placeholder until the diagnostic reads. The FPS
    // is right-aligned to 3 chars so the topbar never reflows as the reading
    // changes digit count (owner playtest: 100 -> 99 must not shift).
    assert_eq!(
        nova_os_status_text("CERES QUEEN", Some(60)),
        "SHIP: CERES QUEEN     LINK: LOCAL     FPS:  60"
    );
    assert_eq!(
        nova_os_status_text("CERES QUEEN", None),
        "SHIP: CERES QUEEN     LINK: LOCAL     FPS:  --"
    );

    // Fixed width: the FPS segment is the SAME length across digit counts, so
    // 100 -> 99 does not change the rendered width (the reported bug).
    assert_eq!(nova_os_fps_segment(Some(100)).len(), 3);
    assert_eq!(nova_os_fps_segment(Some(99)).len(), 3);
    assert_eq!(nova_os_fps_segment(Some(9)).len(), 3);
    assert_eq!(nova_os_fps_segment(None).len(), 3);
    assert_eq!(nova_os_fps_segment(Some(99)), " 99");
    assert_eq!(nova_os_fps_segment(Some(100)), "100");

    // The live rewrite replaces only the FPS tail, preserving the head.
    let spawned = nova_os_status_text("CERES QUEEN", None);
    assert_eq!(
        topbar_line_with_fps(&spawned, Some(144)),
        "SHIP: CERES QUEEN     LINK: LOCAL     FPS: 144"
    );
    assert_eq!(
        topbar_line_with_fps("SHIP: CERES QUEEN     LINK: LOCAL     FPS: 144", None),
        "SHIP: CERES QUEEN     LINK: LOCAL     FPS:  --"
    );
    // A line missing the marker (older spawn) still gets an FPS segment.
    assert_eq!(
        topbar_line_with_fps("SHIP: CERES QUEEN     LINK: LOCAL", Some(30)),
        "SHIP: CERES QUEEN     LINK: LOCAL     FPS:  30"
    );
}

#[test]
fn nova_os_header_breadcrumb_tracks_the_active_surface() {
    // The terminal surface reads `// SHELL`; a launched app reads
    // `// APPS / <ID>` with the launch word upper-cased (owner-confirmed
    // wording, this task's DECISION.md).
    let ver = nova_os_version_label();
    assert_eq!(
        nova_os_header_breadcrumb(TerminalMode::Prompt),
        format!("NOVA OS {ver} // SHELL"),
    );
    assert_eq!(
        nova_os_header_breadcrumb(TerminalMode::App { id: "map" }),
        format!("NOVA OS {ver} // APPS / MAP"),
    );
    // The breadcrumb uses the launch word, not `title()` - a multi-word id
    // still upper-cases whole.
    assert_eq!(
        nova_os_header_breadcrumb(TerminalMode::App { id: "ship" }),
        format!("NOVA OS {ver} // APPS / SHIP"),
    );
}

#[test]
fn drive_topbar_fps_writes_the_smoothed_reading_onto_the_status_line() {
    use bevy::diagnostic::{
        Diagnostic, DiagnosticMeasurement, DiagnosticsStore, FrameTimeDiagnosticsPlugin,
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    spawn_nova_os_shell(&mut app);

    // Seed a DiagnosticsStore with an FPS reading, mirroring what
    // FrameTimeDiagnosticsPlugin publishes in production.
    let mut store = DiagnosticsStore::default();
    let mut fps = Diagnostic::new(FrameTimeDiagnosticsPlugin::FPS);
    fps.add_measurement(DiagnosticMeasurement {
        time: std::time::Instant::now(),
        value: 59.6,
    });
    store.add(fps);
    app.insert_resource(store);

    app.world_mut()
        .run_system_once(drive_nova_os_topbar_fps)
        .unwrap();

    let texts = all_texts(&mut app);
    assert!(
        texts
            .iter()
            .any(|text| text == "SHIP: SURVEY CUTTER     LINK: LOCAL     FPS:  60"),
        "the topbar shows the rounded smoothed FPS (fixed 3-wide) while the NOVA OS is open; got {texts:?}"
    );
}
