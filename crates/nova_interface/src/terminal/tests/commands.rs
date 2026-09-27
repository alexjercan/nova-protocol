//! The prompt's paint: parse feedback, the prefix and the hint row.

use super::*;

#[test]
fn terminal_ui_renders_prompt_hint_and_invalid_coloring() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<CommandTerminal>();
    spawn_nova_os_shell(&mut app);
    {
        let mut terminal = app.world_mut().resource_mut::<CommandTerminal>();
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
    assert_eq!(prefix.0, "cmd>");
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
