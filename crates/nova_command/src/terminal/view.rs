//! What the terminal RENDERS: the version label and the three-piece
//! prompt-line strings (before caret, after caret, ghost).

use super::{state::TerminalParseStatus, CommandTerminal};

/// The version label (`v<app-version>`) on the monitor's brand plate.
pub fn nova_os_version_label() -> String {
    format!("v{}", nova_info::APP_VERSION)
}

/// The typed text left of the caret. The prompt line is rendered as three
/// inline pieces - `before` | caret | `after` - plus the dim ghost, so the fish
/// completion continues on the SAME line right after the typed text with a real
/// caret between them (no `|` glyph baked into the text, no leading space).
pub fn prompt_before_cursor(terminal: &CommandTerminal) -> String {
    // The edit methods keep `cursor` on a char boundary; assert it here since
    // this slice would panic otherwise and `cursor` is now reachable only
    // through the crate's own getters.
    debug_assert!(terminal.prompt().is_char_boundary(terminal.cursor()));
    terminal.prompt()[..terminal.cursor()].to_string()
}

/// The typed text right of the caret (empty when the caret sits at the end).
pub fn prompt_after_cursor(terminal: &CommandTerminal) -> String {
    debug_assert!(terminal.prompt().is_char_boundary(terminal.cursor()));
    terminal.prompt()[terminal.cursor()..].to_string()
}

/// The hint line shown under the prompt while the input is invalid (empty
/// otherwise).
pub fn prompt_hint_display(terminal: &CommandTerminal) -> String {
    if terminal.parse_status() == TerminalParseStatus::Invalid {
        terminal.completion_hint().unwrap_or_default().to_string()
    } else {
        String::new()
    }
}

/// The dim inline completion ghost: the suffix of the command name the prompt is
/// completing toward (empty unless the prompt is a valid prefix).
pub fn prompt_completion_ghost(terminal: &CommandTerminal) -> String {
    if terminal.parse_status() != TerminalParseStatus::ValidPrefix {
        return String::new();
    }
    // On a valid prefix `completion_hint` holds the full command name the input is
    // completing toward (single- or multi-word); the ghost is the suffix past
    // what has been typed. Stripping the PARSED prompt,
    // not the raw one, is what keeps a leading space from greening the prompt
    // with no ghost behind it.
    terminal
        .completion_hint()
        .and_then(|name| name.strip_prefix(terminal.parsed_prompt()))
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::fixtures::type_text;

    #[test]
    fn the_prompt_renders_a_fish_style_completion_ghost() {
        let mut terminal = CommandTerminal::default();
        type_text(&mut terminal, "he");

        assert_eq!(terminal.parse_status(), TerminalParseStatus::ValidPrefix);
        assert_eq!(prompt_before_cursor(&terminal), "he");
        assert_eq!(prompt_after_cursor(&terminal), "");
        assert_eq!(prompt_completion_ghost(&terminal), "lp");
        assert_eq!(prompt_hint_display(&terminal), "");

        type_text(&mut terminal, "zz");
        assert_eq!(prompt_completion_ghost(&terminal), "");
        assert_eq!(prompt_hint_display(&terminal), "did you mean help?");
    }
}
