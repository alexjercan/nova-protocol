//! Opening and closing the computer, and the prompt's visible text state.

use super::*;

#[test]
fn tab_toggles_nova_os_state() {
    let mut app = toggle_app();
    assert_eq!(pause_state(&app), PauseStates::Unpaused);
    press_tab(&mut app);
    assert_eq!(
        pause_state(&app),
        PauseStates::NovaOs,
        "Tab from Unpaused opens the NOVA OS"
    );
    press_tab(&mut app);
    assert_eq!(
        pause_state(&app),
        PauseStates::NovaOs,
        "Tab inside the NOVA OS stays with NOVA OS so the terminal can autocomplete"
    );
}

/// Tab is inert with no ship on the field. `Playing` also covers the editor's
/// build mode, and a Tab there used to arm the freeze axis over a scene the
/// monitor cannot draw for: the press looked like it did nothing, then Play
/// opened straight into the NOVA OS.
#[test]
fn tab_does_not_open_the_nova_os_without_a_ship() {
    let mut app = toggle_app();
    for ship in app
        .world_mut()
        .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .iter(app.world())
        .collect::<Vec<_>>()
    {
        app.world_mut().despawn(ship);
    }

    press_tab(&mut app);

    assert_eq!(
        pause_state(&app),
        PauseStates::Unpaused,
        "no ship, no computer - and no stuck freeze state either"
    );
}

#[test]
fn tab_opens_nova_os_then_completes_terminal_command() {
    let mut app = toggle_app();
    init_terminal_input_resources(&mut app);
    app.add_systems(
        Update,
        handle_terminal_keyboard.run_if(in_state(GameStates::Playing)),
    );

    press_tab(&mut app);
    assert_eq!(pause_state(&app), PauseStates::NovaOs);
    press_text(&mut app, "he");
    press_tab(&mut app);

    let terminal = app.world().resource::<NovaOsTerminal>();
    assert_eq!(terminal.prompt(), "help");
    assert_eq!(terminal.cursor(), 4);
}

#[test]
fn terminal_ignores_text_typed_before_nova_os_opens() {
    let mut app = toggle_app();
    init_terminal_input_resources(&mut app);
    app.add_systems(
        Update,
        handle_terminal_keyboard.run_if(in_state(GameStates::Playing)),
    );

    press_text(&mut app, "flight");
    assert_eq!(
        app.world().resource::<NovaOsTerminal>().prompt(),
        "",
        "keyboard text typed during flight is drained but not inserted"
    );

    press_tab(&mut app);
    assert_eq!(pause_state(&app), PauseStates::NovaOs);
    assert_eq!(
        app.world().resource::<NovaOsTerminal>().prompt(),
        "",
        "opening the NOVA OS does not replay stale flight text into the prompt"
    );
}

#[test]
fn keyboard_input_updates_visible_prompt_text() {
    let mut app = toggle_app();
    init_terminal_input_resources(&mut app);
    app.add_systems(
        Update,
        (handle_terminal_keyboard, rebuild_terminal_ui)
            .chain()
            .run_if(in_state(GameStates::Playing)),
    );
    spawn_nova_os_shell(&mut app);

    press_tab(&mut app);
    assert_eq!(pause_state(&app), PauseStates::NovaOs);
    press_text(&mut app, "he");

    let prompt = app
        .world_mut()
        .query_filtered::<&Text, With<NovaOsTerminalPromptMarker>>()
        .single(app.world())
        .expect("one visible prompt text entity");
    assert_eq!(
        prompt.0, "he",
        "typed text left of the caret, no baked-in `|`"
    );

    let ghost = app
        .world_mut()
        .query_filtered::<&Text, With<NovaOsTerminalGhostMarker>>()
        .single(app.world())
        .expect("one visible ghost text entity");
    assert_eq!(
        ghost.0, "lp",
        "completion continues inline with no leading space (fish-style)"
    );
}

#[test]
fn nova_os_inline_completion_is_same_line_continuation() {
    // The ghost, the before-cursor and after-cursor prompt pieces must all
    // render with `NoWrap` so a completion never wraps below the input line
    // (the reported "completion appears below the line" bug).
    let mut terminal = NovaOsTerminal::default();
    type_text(&mut terminal, "hel");
    assert_eq!(prompt_before_cursor(&terminal), "hel");
    assert_eq!(prompt_after_cursor(&terminal), "");
    assert_eq!(
        prompt_completion_ghost(&terminal),
        "p",
        "ghost is the raw suffix, no leading space"
    );

    // With the caret moved into the middle, the block caret splits the typed
    // text: `he` renders left of it, `lp` (from a full `help`) to its right.
    type_text(&mut terminal, "p");
    terminal.move_cursor_left();
    terminal.move_cursor_left();
    assert_eq!(prompt_before_cursor(&terminal), "he");
    assert_eq!(prompt_after_cursor(&terminal), "lp");

    let mut app = toggle_app();
    init_terminal_input_resources(&mut app);
    app.add_systems(
        Update,
        (handle_terminal_keyboard, rebuild_terminal_ui)
            .chain()
            .run_if(in_state(GameStates::Playing)),
    );
    spawn_nova_os_shell(&mut app);
    press_tab(&mut app);
    press_text(&mut app, "hel");

    for marker_layout in app
        .world_mut()
        .query_filtered::<&TextLayout, Or<(
            With<NovaOsTerminalPromptMarker>,
            With<NovaOsTerminalPromptAfterMarker>,
            With<NovaOsTerminalGhostMarker>,
        )>>()
        .iter(app.world())
    {
        assert_eq!(
            marker_layout.linebreak,
            LineBreak::NoWrap,
            "prompt/ghost pieces must not wrap to a line below the input"
        );
    }
}

#[test]
fn nova_os_block_caret_is_absolute_and_tracks_measured_text_width() {
    // The block caret is ABSOLUTE (so it never advances the row and pushes the
    // ghost a cell right) and is positioned at the MEASURED rendered width of
    // the typed-before text - so it sits ON the first after-cursor /
    // completion-ghost letter regardless of the font's glyph advance (owner
    // playtest: "at the same position as the caret we have the first letter
    // that can be used"). `position_nova_os_block_caret` copies the before-text
    // node's `ComputedNode` width (converted to logical px) onto the caret.
    let mut app = toggle_app();
    init_terminal_input_resources(&mut app);
    app.add_systems(
        Update,
        (handle_terminal_keyboard, rebuild_terminal_ui)
            .chain()
            .run_if(in_state(GameStates::Playing)),
    );
    spawn_nova_os_shell(&mut app);
    press_tab(&mut app);
    press_text(&mut app, "hel");

    let (caret_entity, caret) = app
        .world_mut()
        .query_filtered::<(Entity, &Node), With<NovaOsTerminalCaretMarker>>()
        .single(app.world())
        .map(|(e, n)| (e, n.clone()))
        .expect("one block caret");
    assert_eq!(
        caret.position_type,
        PositionType::Absolute,
        "the caret is absolute so it never advances the row (would push the ghost right)"
    );

    // MinimalPlugins runs no UI layout, so stamp a known rendered width on the
    // before-text node (as the layout pass would) and a 2x scale factor to
    // prove the physical->logical conversion. The caret must copy that width.
    let before_entity = app
        .world_mut()
        .query_filtered::<Entity, With<NovaOsTerminalPromptMarker>>()
        .single(app.world())
        .expect("one before-cursor prompt text");
    // A width deliberately NOT a multiple of the old `chars * 0.6em` cell
    // (16 * 0.6 = 9.6px), so the asserted number itself proves the caret is
    // measure-derived rather than char-derived (review R2.1).
    app.world_mut()
        .entity_mut(before_entity)
        .insert(ComputedNode {
            size: Vec2::new(50.0, 18.0),
            inverse_scale_factor: 0.5,
            ..ComputedNode::DEFAULT
        });

    app.world_mut()
        .run_system_once(position_nova_os_block_caret)
        .expect("caret positioner runs");

    let caret_left = app
        .world()
        .entity(caret_entity)
        .get::<Node>()
        .expect("caret node")
        .left;
    assert_eq!(
        caret_left,
        Val::Px(25.0),
        "the caret lands on the MEASURED typed-text width (50.0 physical * 0.5 \
         scale = 25.0 logical px, not any `chars * 9.6` cell step)"
    );
}
/// The gamepad right-stick click (`RightThumb`) opens the NOVA OS too.
/// Narrowing the pad button away fails this.
#[test]
fn pad_opens_nova_os_and_requests_animated_close() {
    let mut app = toggle_app();
    app.init_resource::<ButtonInput<GamepadButton>>();
    assert_eq!(pause_state(&app), PauseStates::Unpaused);

    press_pad(&mut app);
    assert_eq!(
        pause_state(&app),
        PauseStates::NovaOs,
        "the right-stick click opens the NOVA OS"
    );
    press_pad(&mut app);
    assert_eq!(
        pause_state(&app),
        PauseStates::NovaOs,
        "the right-stick click keeps gameplay paused while close animation runs"
    );
    assert!(
        app.world().resource::<NovaOsCloseTransition>().closing,
        "the right-stick click requests the animated nova_os close"
    );
}

#[test]
fn tab_is_inert_while_the_pause_menu_owns_the_freeze() {
    let mut app = toggle_app();
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Paused);
    app.update();
    press_tab(&mut app);
    assert_eq!(
        pause_state(&app),
        PauseStates::Paused,
        "Tab does nothing while the pause menu is up"
    );
}
