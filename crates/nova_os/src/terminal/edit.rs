//! Prompt editing: typing and caret movement, submit, tab completion, history
//! recall and the parse refresh that keeps the prompt's status live.

use bevy::prelude::*;

use super::{
    state::{
        parsed_prompt, NovaOsCommandInvocation, TerminalParseStatus, TerminalRow, TerminalRowKind,
    },
    view::{command_help_rows, nova_os_version_rows, terminal_help_rows},
    NovaOsTerminal, TerminalCommandSnapshot, TerminalMode,
};
use crate::shell::{
    resolve_command, subcommands_of, terminal_command_names, CliOutput, CommandDispatch,
    ResolvedCommand,
};

const NOVA_OS_PROMPT_PREFIX: &str = "nova> ";

/// The most command lines the history keeps. Only `reset_session` ever clears
/// it, so an unbounded history is both an unbounded allocation and an unusable
/// Up-arrow: 200 repeats of `log` means 200 presses to reach anything else.
const MAX_HISTORY: usize = 200;

/// The semantic result of a [`NovaOsTerminal::submit`], so the bevy layer can
/// pick the sound cue without the pure model knowing about audio.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TerminalSubmitOutcome {
    /// An empty prompt line - no command, no cue.
    Empty,
    /// A command ran and produced output/state (help, clear, log, ...).
    Ran,
    /// A command failed (unknown, or arguments where none are allowed).
    Errored,
    /// An app launch word handed the screen to an app.
    Launched,
}

impl NovaOsTerminal {
    /// Insert typed text at the caret (control characters are filtered out).
    pub fn insert_text(&mut self, text: &str) {
        for ch in text.chars().filter(|ch| !ch.is_control()) {
            self.prompt.insert(self.cursor, ch);
            self.cursor += ch.len_utf8();
        }
        self.history_cursor = None;
        self.cycle_stem = None;
        self.refresh_parse();
    }

    /// Delete the character before the caret.
    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        if let Some((idx, _)) = self.prompt[..self.cursor].char_indices().last() {
            self.prompt.drain(idx..self.cursor);
            self.cursor = idx;
        }
        self.history_cursor = None;
        self.cycle_stem = None;
        self.refresh_parse();
    }

    /// Delete the character at the caret.
    pub fn delete(&mut self) {
        if self.cursor >= self.prompt.len() {
            return;
        }
        let end = self.prompt[self.cursor..]
            .char_indices()
            .nth(1)
            .map(|(offset, _)| self.cursor + offset)
            .unwrap_or(self.prompt.len());
        self.prompt.drain(self.cursor..end);
        self.history_cursor = None;
        self.cycle_stem = None;
        self.refresh_parse();
    }

    /// Move the caret one character left.
    pub fn move_cursor_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        if let Some((idx, _)) = self.prompt[..self.cursor].char_indices().last() {
            self.cursor = idx;
        }
    }

    /// Move the caret one character right.
    pub fn move_cursor_right(&mut self) {
        if self.cursor >= self.prompt.len() {
            return;
        }
        self.cursor = self.prompt[self.cursor..]
            .char_indices()
            .nth(1)
            .map(|(offset, _)| self.cursor + offset)
            .unwrap_or(self.prompt.len());
    }

    /// Run the current prompt line against `snapshot`, appending output to the
    /// scrollback and returning what kind of command ran.
    pub fn submit(&mut self, snapshot: &TerminalCommandSnapshot) -> TerminalSubmitOutcome {
        let command_line = self.prompt.trim().to_string();
        if command_line.is_empty() {
            self.reset_prompt();
            return TerminalSubmitOutcome::Empty;
        }

        self.push_row(TerminalRow {
            kind: TerminalRowKind::Input,
            text: format!("{NOVA_OS_PROMPT_PREFIX}{command_line}"),
        });
        self.push_history(command_line.clone());
        self.history_cursor = None;
        self.cycle_stem = None;

        // One matcher resolves every command - app launch words AND CLI commands -
        // as (possibly multi-word) names with per-command arity. Dispatch is
        // generic over the resolved command's `CommandDispatch`; there is no
        // per-command name special case. An app launch leaves the scrollback
        // untouched (exit restores it) and hands the screen to the app; a CLI
        // command performs its action against `snapshot`.
        let outcome = match resolve_command(&command_line, &self.commands) {
            ResolvedCommand::Run {
                name,
                dispatch: CommandDispatch::App,
                ..
            } => {
                self.push_row(TerminalRow {
                    kind: TerminalRowKind::Info,
                    text: format!("launching {name} ..."),
                });
                self.active_mode = TerminalMode::App { id: name };
                TerminalSubmitOutcome::Launched
            }
            ResolvedCommand::Run {
                name,
                dispatch: CommandDispatch::Gameplay,
                args,
            } => {
                // The pure terminal cannot reach the ECS: record the invocation
                // and let `nova_os_ui` apply it and append the result rows. The
                // echo row is already printed, so the result reads under it.
                self.pending_invocation = Some(NovaOsCommandInvocation { name, args });
                TerminalSubmitOutcome::Ran
            }
            ResolvedCommand::Run {
                name,
                dispatch: CommandDispatch::Cli(output),
                ..
            } => {
                match output {
                    CliOutput::Help => {
                        self.extend_scrollback(terminal_help_rows(&self.commands));
                    }
                    CliOutput::Version => self.extend_scrollback(nova_os_version_rows()),
                    CliOutput::Clear => self.reset_scrollback_to_welcome(snapshot),
                    CliOutput::Exit => self.pending_close = true,
                    // Snapshot commands (log/objectives/ship/map view) print the
                    // rows `nova_os_ui` placed under their name.
                    CliOutput::Snapshot => self.extend_scrollback(snapshot.output(name)),
                }
                TerminalSubmitOutcome::Ran
            }
            ResolvedCommand::Usage { name } => {
                self.extend_scrollback(command_help_rows(name, &self.commands));
                TerminalSubmitOutcome::Ran
            }
            ResolvedCommand::Version => {
                self.extend_scrollback(nova_os_version_rows());
                TerminalSubmitOutcome::Ran
            }
            ResolvedCommand::UnexpectedArguments {
                command,
                arity,
                args,
            } => {
                // A shell-style `command: reason` line, then the command's usage
                // block so the player sees how to invoke it. When the command owns
                // subcommands the first overrun word is a bad sub-command; name it
                // (`map: unknown subcommand 'v'`). Otherwise it took an argument it
                // does not accept (`help: takes no arguments`).
                let subs = subcommands_of(&command, &self.commands);
                self.push_row(TerminalRow {
                    kind: TerminalRowKind::Error,
                    text: if subs.is_empty() {
                        format!("{command}: {}", arity.rejection())
                    } else {
                        let bad = args.first().map(String::as_str).unwrap_or_default();
                        format!("{command}: unknown subcommand '{bad}'")
                    },
                });
                self.extend_scrollback(command_help_rows(&command, &self.commands));
                TerminalSubmitOutcome::Errored
            }
            ResolvedCommand::Unknown {
                command,
                suggestion,
            } => {
                // Shell-style not-found: the error line, the optional did-you-mean,
                // then a pointer back at `help` (a real shell points at its usage).
                self.push_row(TerminalRow {
                    kind: TerminalRowKind::Error,
                    text: format!("command not found: {command}"),
                });
                if let Some(suggestion) = suggestion {
                    self.push_row(TerminalRow {
                        kind: TerminalRowKind::Warn,
                        text: format!("did you mean {suggestion}?"),
                    });
                }
                self.push_row(TerminalRow {
                    kind: TerminalRowKind::Dim,
                    text: "Type 'help' for a list of commands.".to_string(),
                });
                TerminalSubmitOutcome::Errored
            }
        };

        self.reset_prompt();
        outcome
    }

    /// Tab completion that CYCLES through the matches instead of locking onto the
    /// common prefix (PoC `completeInput`): the first Tab on an ambiguous stem
    /// lists the matches into the scrollback and jumps to the first, and repeat
    /// presses cycle through them. The cycle is keyed on the original stem
    /// (`cycle_stem`) so it survives the prompt being rewritten to a match,
    /// and is reset by any prompt edit. Returns whether a match was applied (so the
    /// caller can play the autocomplete tick only when something happened).
    pub fn complete(&mut self) -> bool {
        // The stem is the original typed text; while cycling it is preserved so
        // each Tab re-matches against it rather than the completed value.
        let cycling = self.cycle_stem.is_some();
        let stem = self
            .cycle_stem
            .clone()
            .unwrap_or_else(|| self.prompt.clone());
        let matches = self.completion_matches(&stem);
        if matches.is_empty() {
            return false;
        }
        // The first Tab on an ambiguous stem lists the candidates (PoC prints the
        // match row before jumping to the first match).
        if matches.len() > 1 && !cycling {
            self.push_row(TerminalRow {
                kind: TerminalRowKind::Dim,
                text: matches.join("   "),
            });
        }
        let index = if cycling {
            (self.cycle_index + 1) % matches.len()
        } else {
            0
        };
        self.cycle_stem = Some(stem);
        self.cycle_index = index;
        self.prompt = matches[index].clone();
        self.cursor = self.prompt.len();
        self.refresh_parse();
        true
    }

    /// Completion candidates for `stem`: every command name it prefixes, plus the
    /// universal sub-verbs (`<command> help`, `<command> version`) once the player
    /// is past the command name. Drives Tab completion and the inline ghost, so
    /// both understand sub-commands (fish-style), not just top-level names.
    fn completion_matches(&self, stem: &str) -> Vec<String> {
        let mut matches: Vec<String> = terminal_command_names(&self.commands)
            .filter(|name| name.starts_with(stem))
            .map(|name| name.to_string())
            .collect();
        for name in terminal_command_names(&self.commands) {
            // Only offer sub-verbs once the player is past this command's name
            // (`<name> <partial>`), so top-level completion stays clean.
            let Some(partial) = stem.strip_prefix(&format!("{name} ")) else {
                continue;
            };
            // A leading `-` means the player is typing a flag, so complete the
            // flag forms; otherwise complete the word forms + real sub-commands.
            let verbs: &[&str] = if partial.starts_with('-') {
                &["-h", "--help", "-v", "--version"]
            } else {
                &["help", "version"]
            };
            for verb in verbs {
                let candidate = format!("{name} {verb}");
                if candidate.starts_with(stem) {
                    matches.push(candidate);
                }
            }
        }
        // Injected argument candidates for the arg-bearing gameplay verbs: once
        // the player is past the command name (`ship repair <partial>`), offer the
        // live section codes whose start matches the partial (case-insensitive, so
        // `hu` -> `HULL-3`).
        for (name, candidates) in &self.arg_completions {
            let Some(partial) = stem.strip_prefix(&format!("{name} ")) else {
                continue;
            };
            let partial_lower = partial.to_ascii_lowercase();
            for candidate in candidates {
                if candidate.to_ascii_lowercase().starts_with(&partial_lower) {
                    matches.push(format!("{name} {candidate}"));
                }
            }
        }
        // `arg_completions` is a `HashMap`, so the injected candidates arrive in a
        // per-process random order and Tab would cycle differently between runs.
        // Sorting is also what makes the dedup below a single pass.
        matches.sort_unstable();
        matches.dedup();
        matches
    }

    /// Record a submitted command line, skipping an immediate repeat and
    /// dropping the oldest entries past [`MAX_HISTORY`].
    fn push_history(&mut self, command_line: String) {
        if self.history.last() == Some(&command_line) {
            return;
        }
        self.history.push(command_line);
        let excess = self.history.len().saturating_sub(MAX_HISTORY);
        if excess > 0 {
            self.history.drain(..excess);
        }
    }

    /// Recall the previous command from history into the prompt.
    pub fn history_previous(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next = match self.history_cursor {
            Some(cursor) if cursor > 0 => cursor - 1,
            Some(cursor) => cursor,
            None => self.history.len() - 1,
        };
        self.set_history_cursor(next);
    }

    /// Advance to the next history entry, clearing the prompt past the end.
    pub fn history_next(&mut self) {
        let Some(cursor) = self.history_cursor else {
            return;
        };
        if cursor + 1 >= self.history.len() {
            self.history_cursor = None;
            self.cycle_stem = None;
            self.prompt.clear();
            self.cursor = 0;
            self.refresh_parse();
            return;
        }
        self.set_history_cursor(cursor + 1);
    }

    /// Re-evaluate the prompt's parse status and completion hint.
    pub fn refresh_parse(&mut self) {
        let trimmed = parsed_prompt(&self.prompt);
        if trimmed.is_empty() {
            self.parse_status = TerminalParseStatus::Empty;
            self.completion_hint = Some("type help".to_string());
            return;
        }
        match resolve_command(trimmed, &self.commands) {
            // A full, arity-valid command (app launch word or CLI command), or a
            // `<command> help` usage request - all valid input.
            ResolvedCommand::Run { .. }
            | ResolvedCommand::Usage { .. }
            | ResolvedCommand::Version => {
                self.parse_status = TerminalParseStatus::Valid;
                self.completion_hint = None;
            }
            // Trailing words that overrun a command's arity - unless the whole
            // input is still a prefix of a LONGER command name (e.g. `ship vi`
            // toward `ship view`), in which case it is a valid prefix, not an
            // error.
            ResolvedCommand::UnexpectedArguments { command, arity, .. } => {
                if let Some(name) = self.command_name_starting_with(trimmed) {
                    self.parse_status = TerminalParseStatus::ValidPrefix;
                    self.completion_hint = Some(name);
                } else {
                    self.parse_status = TerminalParseStatus::Invalid;
                    self.completion_hint = Some(format!("{command}: {}", arity.rejection()));
                }
            }
            ResolvedCommand::Unknown { suggestion, .. } => {
                if let Some(name) = self.command_name_starting_with(trimmed) {
                    self.parse_status = TerminalParseStatus::ValidPrefix;
                    self.completion_hint = Some(name);
                } else {
                    self.parse_status = TerminalParseStatus::Invalid;
                    self.completion_hint =
                        suggestion.map(|suggestion| format!("did you mean {suggestion}?"));
                }
            }
        }
    }

    /// The first command name (built-ins then app launch words) that has `stem` as
    /// a strict string prefix - the completion target while the player is still
    /// typing a command name.
    fn command_name_starting_with(&self, stem: &str) -> Option<String> {
        self.completion_matches(stem)
            .into_iter()
            .find(|name| name != stem)
    }

    /// Clear the prompt line and its completion cycle (leaving the scrollback and
    /// history intact).
    pub fn reset_prompt(&mut self) {
        self.prompt.clear();
        self.cursor = 0;
        self.cycle_stem = None;
        self.refresh_parse();
    }

    /// Hand the screen to the app with launch word `id`, the same transition
    /// [`Self::submit`] performs for an app launch word. Pairs with
    /// [`Self::exit_app`].
    fn set_history_cursor(&mut self, cursor: usize) {
        self.history_cursor = Some(cursor);
        self.cycle_stem = None;
        self.prompt = self.history[cursor].clone();
        self.cursor = self.prompt.len();
        self.refresh_parse();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::{
        fixtures::{app_spec, cli_spec, core_with, gameplay_spec, type_text},
        nova_os_welcome_rows, prompt_completion_ghost,
        state::MAX_SCROLLBACK_ROWS,
    };

    #[test]
    fn terminal_prompt_edits_and_navigates_history() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "help");
        terminal.move_cursor_left();
        terminal.backspace();
        type_text(&mut terminal, "ar");
        terminal.delete();
        assert_eq!(terminal.prompt, "hear");
        assert_eq!(terminal.cursor, 4);

        terminal.submit(&TerminalCommandSnapshot::default());
        type_text(&mut terminal, "clear");
        terminal.submit(&TerminalCommandSnapshot::default());
        terminal.history_previous();
        assert_eq!(terminal.prompt, "clear");
        terminal.history_previous();
        assert_eq!(terminal.prompt, "hear");
        terminal.history_next();
        assert_eq!(terminal.prompt, "clear");
    }
    #[test]
    fn nova_os_clear_restores_welcome_block() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "help");
        terminal.submit(&TerminalCommandSnapshot::default());
        assert!(
            terminal.scrollback().len() > nova_os_welcome_rows().len(),
            "help adds rows after the welcome block"
        );

        type_text(&mut terminal, "clear");
        terminal.submit(&TerminalCommandSnapshot::default());

        assert_eq!(terminal.scrollback(), nova_os_welcome_rows());
        assert_eq!(terminal.prompt, "");
        assert_eq!(terminal.completion_hint.as_deref(), Some("type help"));
    }
    #[test]
    fn terminal_unknown_command_suggests_nearest_match() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "hlep");

        assert_eq!(terminal.parse_status, TerminalParseStatus::Invalid);
        assert_eq!(
            terminal.completion_hint.as_deref(),
            Some("did you mean help?")
        );

        terminal.submit(&TerminalCommandSnapshot::default());
        // Shell-style rows: the error line, the suggestion, then a pointer at help.
        let rows: Vec<(TerminalRowKind, &str)> = terminal
            .scrollback()
            .iter()
            .map(|row| (row.kind, row.text.as_str()))
            .collect();
        assert!(rows.contains(&(TerminalRowKind::Error, "command not found: hlep")));
        assert!(rows.contains(&(TerminalRowKind::Warn, "did you mean help?")));
        assert!(rows.contains(&(TerminalRowKind::Dim, "Type 'help' for a list of commands.")));
    }

    #[test]
    fn terminal_unknown_command_without_suggestion_points_at_help() {
        // A command far from every builtin gets no did-you-mean, but still the
        // shell-style not-found line and the pointer at `help`.
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "xyzzy");
        terminal.submit(&TerminalCommandSnapshot::default());
        let rows: Vec<(TerminalRowKind, &str)> = terminal
            .scrollback()
            .iter()
            .map(|row| (row.kind, row.text.as_str()))
            .collect();
        assert!(rows.contains(&(TerminalRowKind::Error, "command not found: xyzzy")));
        assert!(
            !rows
                .iter()
                .any(|(_, text)| text.starts_with("did you mean")),
            "xyzzy is too far from any command to suggest one",
        );
        assert!(rows.contains(&(TerminalRowKind::Dim, "Type 'help' for a list of commands.")));
    }
    #[test]
    fn terminal_rejects_unexpected_command_arguments() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "help garbage");
        assert_eq!(terminal.parse_status, TerminalParseStatus::Invalid);
        assert_eq!(
            terminal.completion_hint.as_deref(),
            Some("help: takes no arguments")
        );
        terminal.submit(&TerminalCommandSnapshot::default());
        // A shell-style `command: reason` line, followed by the command's usage
        // block (so the player sees how to use it).
        assert!(
            terminal
                .scrollback()
                .iter()
                .any(|row| row.text == "help: takes no arguments"),
            "the rejection names the command and reason",
        );
        assert!(
            terminal
                .scrollback()
                .iter()
                .any(|row| row.text == "Usage: help"),
            "the rejection is followed by the command's usage",
        );

        type_text(&mut terminal, "clear garbage");
        terminal.submit(&TerminalCommandSnapshot::default());
        assert!(
            !terminal.scrollback().is_empty(),
            "clear with unexpected arguments reports an error instead of clearing scrollback"
        );
        assert!(
            terminal
                .scrollback()
                .iter()
                .any(|row| row.text == "clear: takes no arguments"),
            "clear rejects its argument with a reason",
        );
    }
    #[test]
    fn nova_os_subcommand_completion_and_ghost() {
        let mut terminal = NovaOsTerminal::default();
        terminal.set_commands(core_with([
            app_spec("map", "Open the local-space map"),
            cli_spec("map view", "Print local-space contacts"),
        ]));

        // A sub-command prefix is a VALID PREFIX (not a red error) and ghosts its
        // completion: `map h` -> `map help` (ghost `elp`), fish-style.
        type_text(&mut terminal, "map h");
        assert_eq!(terminal.parse_status, TerminalParseStatus::ValidPrefix);
        assert_eq!(prompt_completion_ghost(&terminal), "elp");

        // Tab completes sub-commands: `map v` completes to a `map v...` command.
        terminal.reset_prompt();
        type_text(&mut terminal, "map v");
        assert!(terminal.complete(), "Tab completes a sub-command");
        assert!(
            terminal.prompt() == "map view" || terminal.prompt() == "map version",
            "completed to a real sub-command: {}",
            terminal.prompt(),
        );

        // A leading `-` completes the FLAG forms, not a red error: `map -` is a
        // valid prefix that Tab-completes to a `map -...` flag.
        terminal.reset_prompt();
        type_text(&mut terminal, "map -");
        assert_eq!(terminal.parse_status, TerminalParseStatus::ValidPrefix);
        assert!(terminal.complete(), "Tab completes a flag");
        assert!(
            terminal.prompt().starts_with("map -"),
            "completed to a flag form: {}",
            terminal.prompt(),
        );

        // The universal `<command> version` sub-verb prints the version.
        terminal.reset_prompt();
        type_text(&mut terminal, "map version");
        terminal.submit(&TerminalCommandSnapshot::default());
        assert!(
            terminal
                .scrollback()
                .iter()
                .any(|row| row.text.starts_with("NOVA OS v")),
            "map version prints the NOVA OS version",
        );
    }
    /// An arg-bearing gameplay verb (`ship repair <id>`) does not print in the
    /// pure terminal: it queues an invocation carrying the parsed argument for the
    /// gameplay layer to apply. The echo row is still printed above it.
    #[test]
    fn nova_os_gameplay_verb_queues_invocation_with_args() {
        let mut terminal = NovaOsTerminal::default();
        terminal.set_commands(core_with([
            app_spec("ship", ""),
            gameplay_spec("ship repair"),
        ]));

        type_text(&mut terminal, "ship repair HULL-3");
        let outcome = terminal.submit(&TerminalCommandSnapshot::default());
        assert_eq!(outcome, TerminalSubmitOutcome::Ran);

        let invocation = terminal
            .take_pending_invocation()
            .expect("an arg-bearing gameplay verb queues an invocation");
        assert_eq!(invocation.name, "ship repair");
        assert_eq!(invocation.args, vec!["HULL-3".to_string()]);
        // Draining it once empties the queue.
        assert!(terminal.take_pending_invocation().is_none());
        // The echoed command line is in the scrollback (the result rows are the
        // gameplay layer's to append).
        assert!(terminal
            .scrollback()
            .iter()
            .any(|row| row.text == "nova> ship repair HULL-3"));
    }

    /// Tab completion of an arg-bearing verb's argument expands the injected live
    /// section codes, case-insensitively.
    #[test]
    fn nova_os_arg_completion_expands_injected_codes() {
        let mut terminal = NovaOsTerminal::default();
        terminal.set_commands(core_with([
            app_spec("ship", ""),
            gameplay_spec("ship repair"),
        ]));
        terminal.merge_arg_completions([(
            "ship repair",
            vec![
                "HULL-1".to_string(),
                "HULL-3".to_string(),
                "PDC-1".to_string(),
            ],
        )]);

        // A lowercase partial completes to the canonical code.
        type_text(&mut terminal, "ship repair pd");
        assert!(terminal.complete(), "Tab expands an injected section code");
        assert_eq!(terminal.prompt, "ship repair PDC-1");

        // An ambiguous partial lists its matches and jumps to the first.
        terminal.reset_prompt();
        type_text(&mut terminal, "ship repair hu");
        assert!(terminal.complete());
        assert!(
            terminal.prompt.starts_with("ship repair HULL-"),
            "completed to a HULL code: {}",
            terminal.prompt,
        );
    }

    /// Tab lists ambiguous matches on the first press and cycles through them on
    /// repeats, resetting the cycle on any edit.
    #[test]
    fn nova_os_tab_cycles_ambiguous_completions() {
        let mut terminal = NovaOsTerminal::default();
        // Three app words sharing the `sh` stem make the stem ambiguous.
        terminal.set_commands(core_with([
            app_spec("ship", ""),
            app_spec("shield", ""),
            app_spec("shells", ""),
        ]));
        terminal.insert_text("sh");

        // First Tab lists the matches (sorted) and jumps to the first.
        assert!(terminal.complete());
        assert_eq!(
            terminal.scrollback().last().map(|row| row.text.as_str()),
            Some("shells   shield   ship"),
            "the first Tab on an ambiguous stem lists the matches",
        );
        assert_eq!(terminal.prompt, "shells");

        // Repeat presses cycle through the rest, then wrap.
        terminal.complete();
        assert_eq!(terminal.prompt, "shield");
        terminal.complete();
        assert_eq!(terminal.prompt, "ship");
        terminal.complete();
        assert_eq!(
            terminal.prompt, "shells",
            "cycling wraps back to the first match"
        );

        // The match list is printed once, not on every cycle press.
        let listings = terminal
            .scrollback()
            .iter()
            .filter(|row| row.text == "shells   shield   ship")
            .count();
        assert_eq!(listings, 1);

        // Any edit resets the cycle (PoC `resetCycle`).
        terminal.insert_text("x");
        assert!(terminal.cycle_stem.is_none(), "editing resets the cycle");
    }

    /// Injected argument candidates come out of a `HashMap`, so without an
    /// explicit order the Tab cycle differs between processes. The matches are
    /// sorted and deduplicated, so the same stem always cycles the same way.
    #[test]
    fn nova_os_completion_matches_are_sorted_and_deduplicated() {
        let mut terminal = NovaOsTerminal::default();
        terminal.set_commands(core_with([
            app_spec("ship", ""),
            gameplay_spec("ship repair"),
        ]));
        terminal.merge_arg_completions([(
            "ship repair",
            vec![
                "PDC-1".to_string(),
                "HULL-3".to_string(),
                "HULL-1".to_string(),
                // A duplicate candidate must not become a duplicate Tab stop.
                "HULL-1".to_string(),
            ],
        )]);

        let matches = terminal.completion_matches("ship repair ");
        assert_eq!(
            matches,
            vec![
                "ship repair HULL-1".to_string(),
                "ship repair HULL-3".to_string(),
                "ship repair PDC-1".to_string(),
                // The universal sub-verbs are offered past the command name too.
                "ship repair help".to_string(),
                "ship repair version".to_string(),
            ],
        );
    }

    /// History is bounded and never records an immediate repeat, so Up-arrow
    /// stays usable after a long session of the same command.
    #[test]
    fn nova_os_history_is_bounded_and_skips_repeats() {
        let mut terminal = NovaOsTerminal::default();
        for _ in 0..3 {
            type_text(&mut terminal, "help");
            terminal.submit(&TerminalCommandSnapshot::default());
        }
        assert_eq!(
            terminal.history,
            vec!["help".to_string()],
            "a repeat of the last entry is not recorded again",
        );

        // Alternating lines are all distinct entries, so only the cap can trim.
        for index in 0..MAX_HISTORY + 20 {
            type_text(&mut terminal, &format!("help {index}"));
            terminal.submit(&TerminalCommandSnapshot::default());
        }
        assert_eq!(terminal.history.len(), MAX_HISTORY);
        assert_eq!(
            terminal.history.last().map(String::as_str),
            Some(format!("help {}", MAX_HISTORY + 19).as_str()),
            "the newest entry survives; the oldest are dropped",
        );
    }

    /// The scrollback is bounded, and its revision changes only when the ROWS
    /// change - the UI rebuilds one entity per row off that counter, so a caret
    /// move must not move it.
    #[test]
    fn nova_os_scrollback_is_bounded_and_revisioned() {
        let mut terminal = NovaOsTerminal::default();
        let before = terminal.scrollback_revision();
        terminal.insert_text("help");
        terminal.move_cursor_left();
        assert_eq!(
            terminal.scrollback_revision(),
            before,
            "prompt edits do not touch the scrollback",
        );

        terminal.submit(&TerminalCommandSnapshot::default());
        assert_ne!(terminal.scrollback_revision(), before);

        for index in 0..MAX_SCROLLBACK_ROWS {
            terminal.extend_scrollback([TerminalRow {
                kind: TerminalRowKind::Output,
                text: format!("row {index}"),
            }]);
        }
        assert_eq!(terminal.scrollback().len(), MAX_SCROLLBACK_ROWS);
        assert_eq!(
            terminal.scrollback().last().map(|row| row.text.as_str()),
            Some(format!("row {}", MAX_SCROLLBACK_ROWS - 1).as_str()),
            "the newest rows survive the cap",
        );
    }

    /// A leading space is stripped by the parser, so the ghost must strip it too
    /// (it greened the prompt with no completion behind it).
    #[test]
    fn nova_os_ghost_survives_a_leading_space() {
        let mut terminal = NovaOsTerminal::default();
        terminal.set_commands(core_with([app_spec("map", "Open the local-space map")]));
        type_text(&mut terminal, " ma");

        assert_eq!(terminal.parse_status, TerminalParseStatus::ValidPrefix);
        assert_eq!(prompt_completion_ghost(&terminal), "p");
    }
}
