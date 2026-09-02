//! What the terminal RENDERS: the welcome/boot/version/help row builders and
//! the three-piece prompt-line strings (before caret, after caret, ghost).

use super::{
    state::{TerminalParseStatus, TerminalRow, TerminalRowKind},
    NovaOsTerminal,
};
use crate::shell::{command_meta, subcommands_of, CommandArity, TerminalCommandSpec};

/// The static welcome block: the version line, the PoC's POST/CORE/DISPLAY/LINK
/// diagnostic rows, then the help hint. The dynamic "N unread events" line is
/// appended separately by [`nova_os_boot_banner_rows`] because it depends on the
/// live flight log.
pub fn nova_os_welcome_rows() -> Vec<TerminalRow> {
    vec![
        TerminalRow {
            kind: TerminalRowKind::Info,
            text: format!("NOVA OS {}", nova_os_version_label()),
        },
        TerminalRow {
            kind: TerminalRowKind::Dim,
            text: "POST ......... flight computer / ok".to_string(),
        },
        TerminalRow {
            kind: TerminalRowKind::Dim,
            text: "CORE ......... 64K static / ok".to_string(),
        },
        TerminalRow {
            kind: TerminalRowKind::Dim,
            text: "DISPLAY ...... green phosphor crt / warm".to_string(),
        },
        TerminalRow {
            kind: TerminalRowKind::Dim,
            text: "LINK ......... cockpit bus / local".to_string(),
        },
        TerminalRow {
            kind: TerminalRowKind::Warn,
            text: "Hint: type `help` and press Enter.".to_string(),
        },
    ]
}

/// The full boot banner: the welcome block plus the "N unread events" line when
/// there is anything unread (PoC's `output` array). This is what the staggered
/// boot reveals row-by-row and what `clear` reprints instantly.
pub fn nova_os_boot_banner_rows(unread: usize, hook: Option<String>) -> Vec<TerminalRow> {
    let mut rows = nova_os_welcome_rows();
    if unread > 0 {
        rows.push(nova_os_unread_events_row(unread, hook));
    }
    rows
}

/// The unread-events hint line: "N unread events. <hook> - try `log`." (PoC's
/// last banner row). `hook` is a short lead-in for the most recent unread event.
fn nova_os_unread_events_row(unread: usize, hook: Option<String>) -> TerminalRow {
    let noun = if unread == 1 { "event" } else { "events" };
    let text = match hook {
        Some(hook) if !hook.is_empty() => {
            format!("{unread} unread {noun}. {hook} - try `log`.")
        }
        _ => format!("{unread} unread {noun} - try `log`."),
    };
    TerminalRow {
        kind: TerminalRowKind::Dim,
        text,
    }
}

/// The NOVA OS version label (`v<app-version>`), shown in the welcome banner and
/// on the monitor's brand plate.
pub fn nova_os_version_label() -> String {
    format!("v{}", nova_info::APP_VERSION)
}

/// Build the `help` output in a shell shape: a `Usage:` synopsis, a `Commands:`
/// header, every registered command (core builtins, then apps and their
/// subcommands) aligned in one column, and a pointer to per-command help.
pub fn terminal_help_rows(commands: &[TerminalCommandSpec]) -> Vec<TerminalRow> {
    let command_width = commands
        .iter()
        .map(|command| command.name.len())
        .max()
        .unwrap_or(0);
    let mut rows = vec![
        TerminalRow {
            kind: TerminalRowKind::Info,
            text: "Usage: <command> [arguments]".to_string(),
        },
        TerminalRow {
            kind: TerminalRowKind::Info,
            text: "Commands:".to_string(),
        },
    ];
    rows.extend(commands.iter().map(|command| TerminalRow {
        kind: TerminalRowKind::Output,
        text: format!("  {:command_width$}  {}", command.name, command.summary),
    }));
    rows.push(TerminalRow {
        kind: TerminalRowKind::Dim,
        text: "Type '<command> help' for details.".to_string(),
    });
    rows
}

/// The argument fragment a usage line appends after the command name: nothing for
/// a no-argument command, ` [subcommand]` when the command owns subcommands, or
/// ` <hint>` for an argument-taking command (its named placeholder, falling back
/// to `<arg>`).
fn usage_arg_fragment(
    arity: CommandArity,
    arg_hint: Option<&str>,
    has_subcommands: bool,
) -> String {
    if has_subcommands {
        return " [subcommand]".to_string();
    }
    match arity {
        CommandArity::None => String::new(),
        CommandArity::UpTo(_) | CommandArity::Between(_, _) => {
            format!(" {}", arg_hint.unwrap_or("<arg>"))
        }
    }
}

/// Per-command help (`<command> help`) in a shell shape: a `{name} - {summary}`
/// title, a capital `Usage:` line naming the real argument (or `[subcommand]`),
/// and an aligned `Subcommands:` section when the command owns any. Works for
/// every registered command, CLI or app.
pub fn command_help_rows(name: &str, commands: &[TerminalCommandSpec]) -> Vec<TerminalRow> {
    let Some((summary, arity, arg_hint)) = command_meta(name, commands) else {
        return vec![TerminalRow {
            kind: TerminalRowKind::Error,
            text: format!("no help for '{name}'"),
        }];
    };
    let subs = subcommands_of(name, commands);
    let mut rows = vec![
        TerminalRow {
            kind: TerminalRowKind::Info,
            text: format!("{name} - {summary}"),
        },
        TerminalRow {
            kind: TerminalRowKind::Output,
            text: format!(
                "Usage: {name}{}",
                usage_arg_fragment(arity, arg_hint, !subs.is_empty())
            ),
        },
    ];
    if !subs.is_empty() {
        rows.push(TerminalRow {
            kind: TerminalRowKind::Output,
            text: "Subcommands:".to_string(),
        });
        let sub_width = subs.iter().map(|sub| sub.len()).max().unwrap_or(0);
        rows.extend(subs.iter().map(|sub| {
            let summary = command_meta(sub, commands)
                .map(|(summary, _, _)| summary)
                .unwrap_or("");
            TerminalRow {
                kind: TerminalRowKind::Output,
                text: format!("  {sub:sub_width$}  {summary}"),
            }
        }));
    }
    rows
}

/// The `version` command output: the version plus a little flavor.
pub fn nova_os_version_rows() -> Vec<TerminalRow> {
    vec![
        TerminalRow {
            kind: TerminalRowKind::Info,
            text: format!("NOVA OS {}", nova_os_version_label()),
        },
        TerminalRow {
            kind: TerminalRowKind::Dim,
            text: "cockpit link nominal - (c) Nova Dynamics, all reactors reserved".to_string(),
        },
    ]
}

/// The typed text left of the caret. The prompt line is rendered as three
/// inline pieces - `before` | caret | `after` - plus the dim ghost, so the fish
/// completion continues on the SAME line right after the typed text with a real
/// caret between them (no `|` glyph baked into the text, no leading space).
pub fn prompt_before_cursor(terminal: &NovaOsTerminal) -> String {
    // The edit methods keep `cursor` on a char boundary; assert it here since
    // this slice would panic otherwise and `cursor` is now reachable only
    // through the crate's own getters.
    debug_assert!(terminal.prompt().is_char_boundary(terminal.cursor()));
    terminal.prompt()[..terminal.cursor()].to_string()
}

/// The typed text right of the caret (empty when the caret sits at the end).
pub fn prompt_after_cursor(terminal: &NovaOsTerminal) -> String {
    debug_assert!(terminal.prompt().is_char_boundary(terminal.cursor()));
    terminal.prompt()[terminal.cursor()..].to_string()
}

/// The hint line shown under the prompt while the input is invalid (empty
/// otherwise).
pub fn prompt_hint_display(terminal: &NovaOsTerminal) -> String {
    if terminal.parse_status() == TerminalParseStatus::Invalid {
        terminal.completion_hint().unwrap_or_default().to_string()
    } else {
        String::new()
    }
}

/// The dim inline completion ghost: the suffix of the command name the prompt is
/// completing toward (empty unless the prompt is a valid prefix).
pub fn prompt_completion_ghost(terminal: &NovaOsTerminal) -> String {
    if terminal.parse_status() != TerminalParseStatus::ValidPrefix {
        return String::new();
    }
    // On a valid prefix `completion_hint` holds the full command name the input is
    // completing toward (built-in or app launch word, single- or multi-word); the
    // ghost is the suffix past what has been typed. Stripping the PARSED prompt,
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
    use crate::{
        command::core_command_specs,
        shell::CommandDispatch,
        terminal::{
            fixtures::{app_spec, cli_spec, core_with, type_text},
            TerminalCommandSnapshot,
        },
    };

    #[test]
    fn nova_os_help_rows_are_generated_from_registered_commands() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "help");
        terminal.submit(&TerminalCommandSnapshot::default());

        // The default terminal carries only the core builtins; app commands (and
        // their subcommands like `map view`) appear once their tree is registered.
        let help_rows = &terminal.scrollback()[nova_os_welcome_rows().len() + 1..];
        assert_eq!(
            help_rows,
            &[
                // Shell shape: a Usage synopsis, a Commands header, the aligned
                // command list, then a pointer to per-command help.
                TerminalRow {
                    kind: TerminalRowKind::Info,
                    text: "Usage: <command> [arguments]".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Info,
                    text: "Commands:".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Output,
                    text: "  help        Show this command list".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Output,
                    text: "  log         Print comms and mission events".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Output,
                    text: "  objectives  Print active objectives".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Output,
                    text: "  clear       Clear terminal scrollback".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Output,
                    text: "  version     Print the NOVA OS version".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Output,
                    text: "  commands    Enter the game command shell".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Output,
                    text: "  exit        Suspend the NOVA OS computer".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Dim,
                    text: "Type '<command> help' for details.".to_string()
                }
            ]
        );
    }

    #[test]
    fn nova_os_help_lists_registered_app_subcommands() {
        // With an app tree registered, `help` lists the app AND its subcommand,
        // grouped after the core builtins.
        let mut terminal = NovaOsTerminal::default();
        terminal.set_nova_os_commands(core_with([
            app_spec("map", "Open the local-space map"),
            cli_spec("map view", "Print local-space contacts"),
        ]));
        type_text(&mut terminal, "help");
        terminal.submit(&TerminalCommandSnapshot::default());
        let listed: Vec<String> = terminal
            .scrollback()
            .iter()
            .filter_map(|row| {
                let trimmed = row.text.trim_start();
                // A help row for `map view` also has `map ` as a prefix, so pick
                // the LONGEST matching command name.
                terminal
                    .command_specs()
                    .iter()
                    .map(|spec| spec.name)
                    .filter(|name| trimmed.starts_with(&format!("{name} ")))
                    .max_by_key(|name| name.len())
                    .map(str::to_string)
            })
            .collect();
        assert_eq!(
            listed,
            vec![
                "help",
                "log",
                "objectives",
                "clear",
                "version",
                "commands",
                "exit",
                "map",
                "map view",
            ]
        );
    }
    #[test]
    fn nova_os_prompt_renders_fish_style_completion_ghost() {
        let mut terminal = NovaOsTerminal::default();
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
    #[test]
    fn nova_os_help_lists_core_command_set() {
        // `help` on the default terminal lists exactly the core builtins, in order.
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "help");
        terminal.submit(&TerminalCommandSnapshot::default());
        let listed: Vec<String> = terminal
            .scrollback()
            .iter()
            .filter_map(|row| {
                let trimmed = row.text.trim_start();
                core_command_specs()
                    .iter()
                    .map(|command| command.name)
                    .find(|name| trimmed.starts_with(&format!("{name} ")))
                    .map(str::to_string)
            })
            .collect();
        assert_eq!(
            listed,
            vec![
                "help",
                "log",
                "objectives",
                "clear",
                "version",
                "commands",
                "exit",
            ]
        );
    }
    #[test]
    fn nova_os_command_help_and_version() {
        let mut terminal = NovaOsTerminal::default();
        terminal.set_nova_os_commands(core_with([
            app_spec("map", "Open the local-space map"),
            cli_spec("map view", "Print local-space contacts"),
        ]));

        // `<command> help` prints the command's summary, usage and sub-commands.
        type_text(&mut terminal, "map help");
        terminal.submit(&TerminalCommandSnapshot::default());
        let text = |t: &NovaOsTerminal| {
            t.scrollback()
                .iter()
                .map(|row| row.text.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let out = text(&terminal);
        assert!(out.contains("map - Open the local-space map"), "{out}");
        // Shell shape: a capital Usage line marking the subcommand slot, then an
        // aligned Subcommands section carrying each child's summary.
        assert!(out.contains("Usage: map [subcommand]"), "{out}");
        assert!(out.contains("Subcommands:"), "{out}");
        assert!(
            terminal
                .scrollback()
                .iter()
                .any(|row| row.text == "  map view  Print local-space contacts"),
            "{out}",
        );

        // A bad sub-command is a named error + usage, not "takes no arguments".
        type_text(&mut terminal, "map v");
        terminal.submit(&TerminalCommandSnapshot::default());
        assert!(
            terminal
                .scrollback()
                .iter()
                .any(|row| row.text == "map: unknown subcommand 'v'"),
            "map v names the bad sub-command",
        );

        // `version` prints the version banner.
        type_text(&mut terminal, "version");
        terminal.submit(&TerminalCommandSnapshot::default());
        assert!(
            terminal
                .scrollback()
                .iter()
                .any(|row| row.text.starts_with("NOVA OS v")),
            "version prints the NOVA OS version",
        );
    }

    /// A command's usage line names its real argument from the registered
    /// `arg_hint` (`map goto <label>`), and a no-argument command shows a bare
    /// `Usage: <name>`. An arg-bearing command that named no hint falls back to a
    /// generic `<arg>` so nothing regresses.
    #[test]
    fn nova_os_command_help_names_the_real_argument() {
        let mut terminal = NovaOsTerminal::default();
        // `map goto` names its argument; `map view` is a no-arg leaf; the bare
        // `spin` gameplay verb declares an arity but no hint (fallback path).
        let goto = TerminalCommandSpec {
            name: "map goto",
            summary: "Fly the ship to a contact label",
            arity: CommandArity::UpTo(1),
            arg_hint: Some("<label>"),
            dispatch: CommandDispatch::Gameplay,
        };
        let spin = TerminalCommandSpec {
            name: "spin",
            summary: "Spin up a subsystem",
            arity: CommandArity::UpTo(1),
            arg_hint: None,
            dispatch: CommandDispatch::Gameplay,
        };
        terminal.set_nova_os_commands(core_with([
            app_spec("map", "Open the local-space map"),
            cli_spec("map view", "Print local-space contacts"),
            goto,
            spin,
        ]));
        let usage_of = |terminal: &mut NovaOsTerminal, line: &str| -> Vec<String> {
            type_text(terminal, line);
            terminal.submit(&TerminalCommandSnapshot::default());
            let rows = terminal
                .scrollback()
                .iter()
                .map(|row| row.text.clone())
                .collect::<Vec<_>>();
            terminal.reset_scrollback_to_welcome(&TerminalCommandSnapshot::default());
            rows
        };

        assert!(
            usage_of(&mut terminal, "map goto help")
                .contains(&"Usage: map goto <label>".to_string()),
            "an arg-bearing command names its argument from the hint",
        );
        assert!(
            usage_of(&mut terminal, "help help").contains(&"Usage: help".to_string()),
            "a no-argument command shows a bare usage line",
        );
        assert!(
            usage_of(&mut terminal, "spin help").contains(&"Usage: spin <arg>".to_string()),
            "an arg command with no hint falls back to a generic <arg>",
        );
    }
}
