//! Prompt editing: typing and caret movement, submit, tab completion, history
//! recall and the parse refresh that keeps the prompt's status live.
//!
//! Every method here works on the ACTIVE shell's [`ShellSession`], so one
//! editor serves both shell languages: the CRT owns the keys, and which
//! vocabulary they are typed into is a field on the emulator.

use bevy::prelude::*;

use super::{
    state::{
        parsed_prompt, CommandInvocation, NovaOsCommandInvocation, ShellKind, TerminalParseStatus,
        TerminalRow, TerminalRowKind, MAX_HISTORY,
    },
    view::{command_help_rows, nova_os_version_rows, terminal_help_rows},
    NovaOsTerminal, TerminalCommandSnapshot, TerminalMode,
};
use crate::{
    commands::{
        live,
        prelude::{resolve_command_line, CommandOutcome, CommandStatus},
    },
    shell::{
        resolve_command, subcommands_of, terminal_command_names, CliOutput, CommandDispatch,
        ResolvedCommand,
    },
};

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
    /// A Command-shell command was parsed and handed to the dispatcher. Its
    /// rows and its ok/error cue belong to the dispatcher's result, one system
    /// later in the same frame - the submit itself decided nothing.
    Dispatched,
}

impl NovaOsTerminal {
    /// Insert typed text at the caret (control characters are filtered out).
    pub fn insert_text(&mut self, text: &str) {
        let session = self.session_mut();
        for ch in text.chars().filter(|ch| !ch.is_control()) {
            session.prompt.insert(session.cursor, ch);
            session.cursor += ch.len_utf8();
        }
        self.after_edit();
    }

    /// Delete the character before the caret.
    pub fn backspace(&mut self) {
        let session = self.session_mut();
        if session.cursor == 0 {
            return;
        }
        if let Some((idx, _)) = session.prompt[..session.cursor].char_indices().last() {
            session.prompt.drain(idx..session.cursor);
            session.cursor = idx;
        }
        self.after_edit();
    }

    /// Delete the character at the caret.
    pub fn delete(&mut self) {
        let session = self.session_mut();
        if session.cursor >= session.prompt.len() {
            return;
        }
        let end = session.prompt[session.cursor..]
            .char_indices()
            .nth(1)
            .map(|(offset, _)| session.cursor + offset)
            .unwrap_or(session.prompt.len());
        session.prompt.drain(session.cursor..end);
        self.after_edit();
    }

    /// Move the caret one character left.
    pub fn move_cursor_left(&mut self) {
        let session = self.session_mut();
        if session.cursor == 0 {
            return;
        }
        if let Some((idx, _)) = session.prompt[..session.cursor].char_indices().last() {
            session.cursor = idx;
        }
    }

    /// Move the caret one character right.
    pub fn move_cursor_right(&mut self) {
        let session = self.session_mut();
        if session.cursor >= session.prompt.len() {
            return;
        }
        session.cursor = session.prompt[session.cursor..]
            .char_indices()
            .nth(1)
            .map(|(offset, _)| session.cursor + offset)
            .unwrap_or(session.prompt.len());
    }

    /// Put the caret at the start of the line (Home, Ctrl+A).
    pub fn move_cursor_to_start(&mut self) {
        self.session_mut().cursor = 0;
    }

    /// Put the caret at the end of the line (End, Ctrl+E).
    pub fn move_cursor_to_end(&mut self) {
        let session = self.session_mut();
        session.cursor = session.prompt.len();
    }

    /// Cut everything before the caret (Ctrl+U).
    ///
    /// Nothing is kept to paste back. A kill ring is a second clipboard for a
    /// prompt that is one line long, and the line it cut is one Up-arrow away
    /// in the history the moment it was submitted.
    pub fn kill_to_start(&mut self) {
        let session = self.session_mut();
        if session.cursor == 0 {
            return;
        }
        session.prompt.drain(..session.cursor);
        session.cursor = 0;
        self.after_edit();
    }

    /// Cut everything from the caret to the end of the line (Ctrl+K).
    pub fn kill_to_end(&mut self) {
        let session = self.session_mut();
        if session.cursor >= session.prompt.len() {
            return;
        }
        session.prompt.truncate(session.cursor);
        self.after_edit();
    }

    /// What every prompt mutation ends with: the history recall and the
    /// completion cycle both describe a line the player has now changed.
    fn after_edit(&mut self) {
        let session = self.session_mut();
        session.history_cursor = None;
        session.cycle_stem = None;
        self.refresh_parse();
    }

    /// Run the current prompt line against `snapshot`, appending output to the
    /// scrollback and returning what kind of command ran.
    ///
    /// The echo, the history and the prompt reset are the EMULATOR's, shared by
    /// both shells; only the middle - what the line means - is per-language.
    pub fn submit(&mut self, snapshot: &TerminalCommandSnapshot) -> TerminalSubmitOutcome {
        let command_line = self.prompt().trim().to_string();
        if command_line.is_empty() {
            self.reset_prompt();
            return TerminalSubmitOutcome::Empty;
        }

        let prefix = self.prompt_prefix();
        self.push_row(TerminalRow {
            kind: TerminalRowKind::Input,
            text: format!("{prefix}{command_line}"),
        });
        self.push_history(command_line.clone());
        let session = self.session_mut();
        session.history_cursor = None;
        session.cycle_stem = None;

        let outcome = match self.active_shell() {
            ShellKind::NovaOs => self.submit_nova_os(&command_line, snapshot),
            ShellKind::Commands => self.submit_command(&command_line),
        };

        self.reset_prompt();
        // The switch lands AFTER the reset so `commands` clears the line it was
        // typed on, not the line waiting in the shell it opens.
        if let Some(shell) = self.pending_shell.take() {
            self.switch_shell(shell);
        }
        outcome
    }

    /// The Command shell: one parser, one dispatcher. Anything answerable from
    /// the catalog is answered here; everything else is queued for the
    /// dispatcher, which is the same seam the process channel pushes into.
    fn submit_command(&mut self, command_line: &str) -> TerminalSubmitOutcome {
        let specs = self.session().commands.clone();
        match resolve_command_line(command_line, &specs) {
            CommandOutcome::Answer(result) => {
                let errored = result.status != CommandStatus::Ok;
                self.extend_scrollback(result.rows);
                if errored {
                    TerminalSubmitOutcome::Errored
                } else {
                    TerminalSubmitOutcome::Ran
                }
            }
            // Shell control never reaches the dispatcher: the emulator owns
            // the screen, so it acts here and the channel refuses these two
            // outright rather than acknowledging a screen it does not have.
            CommandOutcome::Invoke(invocation) if invocation.name == "clear" => {
                self.replace_scrollback(Vec::new());
                // The introduction is re-staged against the world as it is NOW,
                // which is the point of clearing after loading a scenario.
                self.rearm_command_intro();
                TerminalSubmitOutcome::Ran
            }
            CommandOutcome::Invoke(invocation) if invocation.name == "close" => {
                self.pending_close = true;
                TerminalSubmitOutcome::Ran
            }
            CommandOutcome::Invoke(invocation) => {
                self.pending_commands.push_back(invocation);
                TerminalSubmitOutcome::Dispatched
            }
        }
    }

    /// The NOVA OS shell: one matcher resolves every command - app launch words
    /// AND CLI commands - as (possibly multi-word) names with per-command arity.
    /// Dispatch is generic over the resolved command's `CommandDispatch`; there
    /// is no per-command name special case. An app launch leaves the scrollback
    /// untouched (exit restores it) and hands the screen to the app; a CLI
    /// command performs its action against `snapshot`.
    fn submit_nova_os(
        &mut self,
        command_line: &str,
        snapshot: &TerminalCommandSnapshot,
    ) -> TerminalSubmitOutcome {
        let commands = self.session().commands.clone();
        match resolve_command(command_line, &commands) {
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
                dispatch: CommandDispatch::Command,
                args,
            } => {
                // The Command catalog is not registered on this shell, so this
                // is unreachable in practice; queueing it keeps the two shells'
                // dispatch honest if one ever shares a spec with the other.
                if let Some(spec) = crate::commands::command_spec(name) {
                    self.pending_commands.push_back(CommandInvocation {
                        name,
                        class: spec.class,
                        args,
                    });
                }
                TerminalSubmitOutcome::Dispatched
            }
            ResolvedCommand::Run {
                name,
                dispatch: CommandDispatch::Cli(output),
                ..
            } => {
                match output {
                    CliOutput::Help => {
                        self.extend_scrollback(terminal_help_rows(&commands));
                    }
                    CliOutput::Version => self.extend_scrollback(nova_os_version_rows()),
                    CliOutput::Clear => self.reset_scrollback_to_welcome(snapshot),
                    CliOutput::Exit => self.pending_close = true,
                    // The CRT stays open across this: the switch keeps the
                    // freeze and the animation, and Escape climbs back here.
                    CliOutput::EnterCommands => self.pending_shell = Some(ShellKind::Commands),
                    // Snapshot commands (log/objectives/ship/map view) print the
                    // rows `nova_os_ui` placed under their name.
                    CliOutput::Snapshot => self.extend_scrollback(snapshot.output(name)),
                }
                TerminalSubmitOutcome::Ran
            }
            ResolvedCommand::Usage { name } => {
                self.extend_scrollback(command_help_rows(name, &commands));
                TerminalSubmitOutcome::Ran
            }
            ResolvedCommand::Version => {
                self.extend_scrollback(nova_os_version_rows());
                TerminalSubmitOutcome::Ran
            }
            ResolvedCommand::Incomplete { name } => {
                self.push_row(TerminalRow {
                    kind: TerminalRowKind::Error,
                    text: format!("{name}: incomplete command"),
                });
                self.extend_scrollback(command_help_rows(name, &commands));
                TerminalSubmitOutcome::Errored
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
                let subs = subcommands_of(&command, &commands);
                let overrun = arity
                    .overruns(args.len())
                    .then(|| args[arity.most()].as_str());
                let text = match overrun {
                    Some(bad) if !subs.is_empty() => {
                        format!("{command}: unknown subcommand '{bad}'")
                    }
                    _ => format!("{command}: {}", arity.rejection()),
                };
                self.push_row(TerminalRow {
                    kind: TerminalRowKind::Error,
                    text,
                });
                self.extend_scrollback(command_help_rows(&command, &commands));
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
        }
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
        let cycling = self.session().cycle_stem.is_some();
        let stem = self
            .session()
            .cycle_stem
            .clone()
            .unwrap_or_else(|| self.session().prompt.clone());
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
            (self.session().cycle_index + 1) % matches.len()
        } else {
            0
        };
        let session = self.session_mut();
        session.cycle_stem = Some(stem);
        session.cycle_index = index;
        session.prompt = matches[index].clone();
        session.cursor = session.prompt.len();
        self.refresh_parse();
        true
    }

    /// Completion candidates for `stem`: every command name it prefixes, the
    /// universal sub-verbs (`<command> help`, `<command> version`), and - once
    /// the player is past the name - the values the ARGUMENT under the caret
    /// accepts. Drives Tab completion and the inline ghost, so both understand
    /// sub-commands (fish-style) and arguments, not just top-level names.
    fn completion_matches(&self, stem: &str) -> Vec<String> {
        let session = self.session();
        let mut matches: Vec<String> = terminal_command_names(&session.commands)
            .filter(|name| name.starts_with(stem))
            .map(|name| name.to_string())
            .collect();
        for name in terminal_command_names(&session.commands) {
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
        matches.extend(self.argument_matches(stem));
        // The live values live in a `HashMap`, so candidates arrive in a
        // per-process random order and Tab would cycle differently between runs.
        // Sorting is also what makes the dedup below a single pass.
        matches.sort_unstable();
        matches.dedup();
        matches
    }

    /// Completions for the argument position the caret is in.
    ///
    /// The command is resolved by the same longest-name rule the parser uses,
    /// so `ammo refill section <TAB>` completes the section verb's first
    /// argument rather than `ammo refill`'s. Which POSITION is being typed
    /// follows the caret: a stem ending in a space starts a new word, anything
    /// else is still editing the last one.
    fn argument_matches(&self, stem: &str) -> Vec<String> {
        let session = self.session();
        let Some(spec) = terminal_command_names(&session.commands)
            .filter(|name| stem.starts_with(&format!("{name} ")))
            .max_by_key(|name| name.split_whitespace().count())
            .and_then(|name| session.commands.iter().find(|spec| spec.name == name))
        else {
            return Vec::new();
        };
        let tail = &stem[spec.name.len() + 1..];
        let typed: Vec<&str> = tail.split_whitespace().collect();
        // A trailing space means the player has finished a word and started the
        // next one; anything else means they are still editing the last.
        let starting_a_word = tail.is_empty() || tail.ends_with(char::is_whitespace);
        // A command NAME is several words, so a command that asks for one takes
        // the whole tail as its single argument: `help ammo r<TAB>` reaches
        // `help ammo refill`. Every other argument is one word in its position.
        let names_a_command =
            spec.args.first().and_then(|arg| arg.live_token()) == Some(live::COMMAND);
        let at = match (names_a_command, starting_a_word) {
            (true, _) => 0,
            (false, true) => typed.len(),
            (false, false) => typed.len() - 1,
        };
        let Some(arg) = spec.args.get(at) else {
            return Vec::new();
        };
        let partial = match (names_a_command, starting_a_word) {
            (true, _) => tail.trim_start(),
            (false, true) => "",
            (false, false) => typed[typed.len() - 1],
        };
        let settled: String = typed[..at.min(typed.len())]
            .iter()
            .map(|word| format!("{word} "))
            .collect();
        // A live set may be scoped by the argument before it, so
        // `section <ship> <TAB>` offers that ship's sections rather than the
        // union across the field. An unqualified set is the fallback.
        let live = arg
            .live_token()
            .and_then(|token| {
                let qualified = (at > 0).then(|| format!("{token}:{}", typed[at - 1]));
                qualified
                    .and_then(|key| self.live_values.get(&key))
                    .or_else(|| self.live_values.get(token))
            })
            .map(Vec::as_slice)
            .unwrap_or_default();
        let partial_lower = partial.to_ascii_lowercase();
        arg.words()
            .iter()
            .map(|word| (*word).to_string())
            .chain(live.iter().cloned())
            // Case-insensitive, so `hu` reaches `HULL-3`.
            .filter(|candidate| candidate.to_ascii_lowercase().starts_with(&partial_lower))
            .map(|candidate| format!("{} {settled}{candidate}", spec.name))
            .collect()
    }

    /// Record a submitted command line, skipping an immediate repeat and
    /// dropping the oldest entries past [`MAX_HISTORY`].
    fn push_history(&mut self, command_line: String) {
        let session = self.session_mut();
        if session.history.last() == Some(&command_line) {
            return;
        }
        session.history.push(command_line);
        let excess = session.history.len().saturating_sub(MAX_HISTORY);
        if excess > 0 {
            session.history.drain(..excess);
        }
    }

    /// Recall the previous command from history into the prompt.
    pub fn history_previous(&mut self) {
        let session = self.session();
        if session.history.is_empty() {
            return;
        }
        let next = match session.history_cursor {
            Some(cursor) if cursor > 0 => cursor - 1,
            Some(cursor) => cursor,
            None => session.history.len() - 1,
        };
        self.set_history_cursor(next);
    }

    /// Advance to the next history entry, clearing the prompt past the end.
    pub fn history_next(&mut self) {
        let Some(cursor) = self.session().history_cursor else {
            return;
        };
        if cursor + 1 >= self.session().history.len() {
            let session = self.session_mut();
            session.history_cursor = None;
            session.cycle_stem = None;
            session.prompt.clear();
            session.cursor = 0;
            self.refresh_parse();
            return;
        }
        self.set_history_cursor(cursor + 1);
    }

    /// Re-evaluate the prompt's parse status and completion hint.
    pub fn refresh_parse(&mut self) {
        let trimmed = parsed_prompt(&self.session().prompt).to_string();
        if trimmed.is_empty() {
            let session = self.session_mut();
            session.parse_status = TerminalParseStatus::Empty;
            session.completion_hint = Some("type help".to_string());
            return;
        }
        let commands = self.session().commands.clone();
        let (status, hint) = match resolve_command(&trimmed, &commands) {
            // A full, arity-valid command (app launch word or CLI command), or a
            // `<command> help` usage request - all valid input.
            ResolvedCommand::Run { .. }
            | ResolvedCommand::Usage { .. }
            | ResolvedCommand::Version => (TerminalParseStatus::Valid, None),
            // Trailing words that overrun a command's arity - unless the whole
            // input is still a prefix of a LONGER command name (e.g. `ship vi`
            // toward `ship view`), in which case it is a valid prefix, not an
            // error.
            ResolvedCommand::UnexpectedArguments { command, arity, .. } => {
                match self.command_name_starting_with(&trimmed) {
                    Some(name) => (TerminalParseStatus::ValidPrefix, Some(name)),
                    None => (
                        TerminalParseStatus::Invalid,
                        Some(format!("{command}: {}", arity.rejection())),
                    ),
                }
            }
            // A parent word on the way to a real command is a prefix, not an
            // error, exactly like a half-typed name.
            ResolvedCommand::Incomplete { name } => {
                match self.command_name_starting_with(&trimmed) {
                    Some(completion) => (TerminalParseStatus::ValidPrefix, Some(completion)),
                    None => (
                        TerminalParseStatus::Invalid,
                        Some(format!("{name}: incomplete command")),
                    ),
                }
            }
            ResolvedCommand::Unknown { suggestion, .. } => {
                match self.command_name_starting_with(&trimmed) {
                    Some(name) => (TerminalParseStatus::ValidPrefix, Some(name)),
                    None => (
                        TerminalParseStatus::Invalid,
                        suggestion.map(|suggestion| format!("did you mean {suggestion}?")),
                    ),
                }
            }
        };
        let session = self.session_mut();
        session.parse_status = status;
        session.completion_hint = hint;
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
        let session = self.session_mut();
        session.prompt.clear();
        session.cursor = 0;
        session.cycle_stem = None;
        self.refresh_parse();
    }

    /// Put the history entry at `cursor` on the prompt.
    fn set_history_cursor(&mut self, cursor: usize) {
        let session = self.session_mut();
        session.history_cursor = Some(cursor);
        session.cycle_stem = None;
        session.prompt = session.history[cursor].clone();
        session.cursor = session.prompt.len();
        self.refresh_parse();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::{live, prelude::CommandClass},
        terminal::{
            fixtures::{app_spec, cli_spec, command_shell, core_with, gameplay_spec, type_text},
            nova_os_welcome_rows, prompt_completion_ghost,
            state::{MAX_HISTORY, MAX_SCROLLBACK_ROWS},
        },
    };

    #[test]
    fn terminal_prompt_edits_and_navigates_history() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "help");
        terminal.move_cursor_left();
        terminal.backspace();
        type_text(&mut terminal, "ar");
        terminal.delete();
        assert_eq!(terminal.prompt(), "hear");
        assert_eq!(terminal.cursor(), 4);

        terminal.submit(&TerminalCommandSnapshot::default());
        type_text(&mut terminal, "clear");
        terminal.submit(&TerminalCommandSnapshot::default());
        terminal.history_previous();
        assert_eq!(terminal.prompt(), "clear");
        terminal.history_previous();
        assert_eq!(terminal.prompt(), "hear");
        terminal.history_next();
        assert_eq!(terminal.prompt(), "clear");
    }
    /// The caret jumps a typo does not need a walk to reach: Home / End and
    /// their Ctrl+A / Ctrl+E chords are the same two moves, and a kill takes
    /// the half of the line the caret is not on.
    #[test]
    fn the_prompt_jumps_and_kills_by_the_line_not_the_character() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "map goto beacon");

        terminal.move_cursor_to_start();
        assert_eq!(terminal.cursor(), 0);
        terminal.move_cursor_to_end();
        assert_eq!(terminal.cursor(), "map goto beacon".len());

        terminal.move_cursor_to_start();
        type_text(&mut terminal, "x");
        terminal.kill_to_start();
        assert_eq!(
            terminal.prompt(),
            "map goto beacon",
            "the typo alone is cut"
        );
        assert_eq!(terminal.cursor(), 0);

        type_text(&mut terminal, "run ");
        terminal.kill_to_end();
        assert_eq!(terminal.prompt(), "run ", "and the rest of the line goes");
        assert_eq!(terminal.cursor(), "run ".len());
    }

    /// A kill on a line with nothing to cut on that side leaves the line alone
    /// rather than clearing it.
    #[test]
    fn a_kill_with_nothing_on_that_side_of_the_caret_does_nothing() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "log");

        terminal.kill_to_end();
        assert_eq!(terminal.prompt(), "log", "the caret is already at the end");
        terminal.move_cursor_to_start();
        terminal.kill_to_start();
        assert_eq!(terminal.prompt(), "log", "and now at the start");
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
        assert_eq!(terminal.prompt(), "");
        assert_eq!(terminal.completion_hint(), Some("type help"));
    }
    #[test]
    fn terminal_unknown_command_suggests_nearest_match() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "hlep");

        assert_eq!(terminal.parse_status(), TerminalParseStatus::Invalid);
        assert_eq!(terminal.completion_hint(), Some("did you mean help?"));

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
        assert_eq!(terminal.parse_status(), TerminalParseStatus::Invalid);
        assert_eq!(terminal.completion_hint(), Some("help: takes no arguments"));
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
        terminal.set_nova_os_commands(core_with([
            app_spec("map", "Open the local-space map"),
            cli_spec("map view", "Print local-space contacts"),
        ]));

        // A sub-command prefix is a VALID PREFIX (not a red error) and ghosts its
        // completion: `map h` -> `map help` (ghost `elp`), fish-style.
        type_text(&mut terminal, "map h");
        assert_eq!(terminal.parse_status(), TerminalParseStatus::ValidPrefix);
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
        assert_eq!(terminal.parse_status(), TerminalParseStatus::ValidPrefix);
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
        terminal.set_nova_os_commands(core_with([
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

    /// Tab completion of an arg-bearing verb's argument expands the published
    /// live section codes, case-insensitively.
    #[test]
    fn nova_os_arg_completion_expands_injected_codes() {
        let mut terminal = NovaOsTerminal::default();
        terminal.set_nova_os_commands(core_with([
            app_spec("ship", ""),
            gameplay_spec("ship repair"),
        ]));
        terminal.merge_live_values([(
            live::SECTION,
            vec![
                "HULL-1".to_string(),
                "HULL-3".to_string(),
                "PDC-1".to_string(),
            ],
        )]);

        // A lowercase partial completes to the canonical code.
        type_text(&mut terminal, "ship repair pd");
        assert!(terminal.complete(), "Tab expands an injected section code");
        assert_eq!(terminal.prompt(), "ship repair PDC-1");

        // An ambiguous partial lists its matches and jumps to the first.
        terminal.reset_prompt();
        type_text(&mut terminal, "ship repair hu");
        assert!(terminal.complete());
        assert!(
            terminal.prompt().starts_with("ship repair HULL-"),
            "completed to a HULL code: {}",
            terminal.prompt(),
        );
    }

    /// Tab lists ambiguous matches on the first press and cycles through them on
    /// repeats, resetting the cycle on any edit.
    #[test]
    fn nova_os_tab_cycles_ambiguous_completions() {
        let mut terminal = NovaOsTerminal::default();
        // Three app words sharing the `sh` stem make the stem ambiguous.
        terminal.set_nova_os_commands(core_with([
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
        assert_eq!(terminal.prompt(), "shells");

        // Repeat presses cycle through the rest, then wrap.
        terminal.complete();
        assert_eq!(terminal.prompt(), "shield");
        terminal.complete();
        assert_eq!(terminal.prompt(), "ship");
        terminal.complete();
        assert_eq!(
            terminal.prompt(),
            "shells",
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
        assert!(
            terminal.session().cycle_stem.is_none(),
            "editing resets the cycle"
        );
    }

    /// Live argument values come out of a `HashMap`, so without an explicit
    /// order the Tab cycle differs between processes. The matches are sorted and
    /// deduplicated, so the same stem always cycles the same way.
    #[test]
    fn nova_os_completion_matches_are_sorted_and_deduplicated() {
        let mut terminal = NovaOsTerminal::default();
        terminal.set_nova_os_commands(core_with([
            app_spec("ship", ""),
            gameplay_spec("ship repair"),
        ]));
        terminal.merge_live_values([(
            live::SECTION,
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

    /// Completion follows the CARET, not the command: the second argument
    /// completes from the second argument's set, and a live set scoped to the
    /// ship already typed offers that ship's sections alone.
    #[test]
    fn the_argument_under_the_caret_is_what_completes() {
        let mut terminal = NovaOsTerminal::default();
        terminal.open_shell(ShellKind::Commands);
        terminal.merge_live_values([
            (
                live::SHIP.to_string(),
                vec!["cargoa".to_string(), "cargoa_raider".to_string()],
            ),
            (
                format!("{}:cargoa", live::SECTION),
                vec!["hull_front".to_string(), "turret_port".to_string()],
            ),
            (
                format!("{}:cargoa_raider", live::SECTION),
                vec!["turret_port".to_string()],
            ),
        ]);

        // First position: the ships.
        type_text(&mut terminal, "section cargoa_r");
        assert!(terminal.complete());
        assert_eq!(terminal.prompt(), "section cargoa_raider");

        // Second position: only THAT ship's sections, and the settled first
        // argument is kept.
        terminal.reset_prompt();
        type_text(&mut terminal, "section cargoa hu");
        assert!(terminal.complete());
        assert_eq!(terminal.prompt(), "section cargoa hull_front");

        // The raider carries no `hull_front`, so nothing completes there.
        terminal.reset_prompt();
        type_text(&mut terminal, "section cargoa_raider hu");
        assert!(!terminal.complete());
    }

    /// A closed argument set lives in the catalog, so it completes with no
    /// world at all.
    #[test]
    fn a_catalog_argument_completes_without_a_world() {
        let mut terminal = NovaOsTerminal::default();
        terminal.open_shell(ShellKind::Commands);
        type_text(&mut terminal, "graphics me");
        assert!(terminal.complete());
        assert_eq!(terminal.prompt(), "graphics medium");
    }

    /// `help <TAB>` names commands, and a command name is several words, so it
    /// is matched against the whole tail rather than the last word.
    #[test]
    fn help_completes_whole_multi_word_command_names() {
        let mut terminal = NovaOsTerminal::default();
        terminal.open_shell(ShellKind::Commands);
        type_text(&mut terminal, "help ammo refill s");
        assert!(terminal.complete());
        assert_eq!(terminal.prompt(), "help ammo refill section");
    }

    /// `clear` and `close` control the SCREEN, which only the emulator has, so
    /// they never reach the dispatcher.
    #[test]
    fn shell_control_is_answered_by_the_emulator_not_the_dispatcher() {
        let mut terminal = NovaOsTerminal::default();
        terminal.open_shell(ShellKind::Commands);
        terminal.extend_scrollback(vec![TerminalRow {
            kind: TerminalRowKind::Output,
            text: "old".to_string(),
        }]);

        type_text(&mut terminal, "clear");
        assert_eq!(
            terminal.submit(&TerminalCommandSnapshot::default()),
            TerminalSubmitOutcome::Ran,
        );
        assert!(!terminal.has_pending_command(), "clear is not dispatched");
        assert!(terminal.scrollback().is_empty(), "the transcript is gone");
        assert!(
            !terminal.is_revealed(ShellKind::Commands),
            "the introduction is re-armed against the world as it is now",
        );

        type_text(&mut terminal, "close");
        assert_eq!(
            terminal.submit(&TerminalCommandSnapshot::default()),
            TerminalSubmitOutcome::Ran,
        );
        assert!(!terminal.has_pending_command(), "close is not dispatched");
        assert!(terminal.has_pending_close());
    }

    /// Two commands submitted before the dispatcher runs both reach it: the
    /// process channel can stage two Enters on one tick.
    #[test]
    fn two_submits_in_one_frame_both_reach_the_dispatcher() {
        let mut terminal = NovaOsTerminal::default();
        terminal.open_shell(ShellKind::Commands);
        for line in ["ships", "status"] {
            type_text(&mut terminal, line);
            terminal.submit(&TerminalCommandSnapshot::default());
        }
        assert_eq!(
            terminal.take_pending_command().map(|it| it.name),
            Some("ships")
        );
        assert_eq!(
            terminal.take_pending_command().map(|it| it.name),
            Some("status")
        );
        assert!(terminal.take_pending_command().is_none());
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
            terminal.session().history,
            vec!["help".to_string()],
            "a repeat of the last entry is not recorded again",
        );

        // Alternating lines are all distinct entries, so only the cap can trim.
        for index in 0..MAX_HISTORY + 20 {
            type_text(&mut terminal, &format!("help {index}"));
            terminal.submit(&TerminalCommandSnapshot::default());
        }
        assert_eq!(terminal.session().history.len(), MAX_HISTORY);
        assert_eq!(
            terminal.session().history.last().map(String::as_str),
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
        terminal.set_nova_os_commands(core_with([app_spec("map", "Open the local-space map")]));
        type_text(&mut terminal, " ma");

        assert_eq!(terminal.parse_status(), TerminalParseStatus::ValidPrefix);
        assert_eq!(prompt_completion_ghost(&terminal), "p");
    }

    /// The two shells share one editor and one CRT but nothing else: each keeps
    /// its own prompt, scrollback, history and command set, and switching back
    /// finds the session exactly as it was left.
    #[test]
    fn the_two_shells_keep_separate_transcripts_and_histories() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "help");
        terminal.submit(&TerminalCommandSnapshot::default());
        let nova_os_rows = terminal.scrollback().len();

        assert!(terminal.switch_shell(ShellKind::Commands));
        assert_eq!(terminal.active_shell(), ShellKind::Commands);
        assert!(
            terminal.scrollback().is_empty(),
            "the Command shell opens on its own (still unrevealed) transcript",
        );
        assert_eq!(terminal.prompt_prefix(), "cmd> ");

        type_text(&mut terminal, "status");
        terminal.submit(&TerminalCommandSnapshot::default());
        assert!(terminal
            .scrollback()
            .iter()
            .any(|row| row.text == "cmd> status"));
        assert_eq!(
            terminal
                .take_pending_command()
                .expect("a world-facing command is queued for the dispatcher")
                .name,
            "status",
        );

        assert!(terminal.switch_shell(ShellKind::NovaOs));
        assert_eq!(
            terminal.scrollback().len(),
            nova_os_rows,
            "the NOVA OS transcript came back untouched",
        );
        terminal.history_previous();
        assert_eq!(
            terminal.prompt(),
            "help",
            "each shell recalls its OWN history, not the other's",
        );
        assert_eq!(terminal.prompt_prefix(), "nova> ");
    }

    /// Switching shells is a field flip, not a reboot: the reveal state and the
    /// pending-close request survive it, so the CRT neither replays its
    /// animation nor loses a close the player asked for.
    #[test]
    fn switching_shells_replays_no_reveal_and_keeps_the_close_request() {
        let mut terminal = NovaOsTerminal::default();
        terminal.begin_boot(vec![TerminalRow::info("BOOT")]);
        terminal.finish_boot();
        assert!(terminal.is_revealed(ShellKind::NovaOs));

        terminal.switch_shell(ShellKind::Commands);
        assert!(!terminal.is_revealed(ShellKind::Commands));
        terminal.begin_reveal(ShellKind::Commands, vec![TerminalRow::info("INTRO")]);
        terminal.finish_boot();

        terminal.switch_shell(ShellKind::NovaOs);
        assert!(
            terminal.is_revealed(ShellKind::NovaOs) && terminal.is_revealed(ShellKind::Commands),
            "coming back does not re-arm either reveal",
        );
        assert!(
            !terminal.has_pending_boot_rows(),
            "nothing is queued to replay",
        );

        terminal.request_close();
        terminal.switch_shell(ShellKind::Commands);
        assert!(
            terminal.take_pending_close(),
            "the close request belongs to the emulator, not to one shell",
        );
    }

    /// A shell switch must move the row-rebuild counter: the two sessions carry
    /// different rows, and a UI keyed on a per-shell counter would paint the
    /// old shell's transcript.
    #[test]
    fn a_shell_switch_moves_the_scrollback_revision() {
        let mut terminal = NovaOsTerminal::default();
        let before = terminal.scrollback_revision();
        terminal.switch_shell(ShellKind::Commands);
        assert_ne!(terminal.scrollback_revision(), before);
        let switched = terminal.scrollback_revision();
        assert!(
            !terminal.switch_shell(ShellKind::Commands),
            "switching to the active shell is a no-op",
        );
        assert_eq!(terminal.scrollback_revision(), switched);
    }

    /// Leaving the NOVA OS shell while an app owns the screen returns to its
    /// prompt first, so coming back lands on a prompt rather than inside a tool
    /// the other shell was typing into.
    #[test]
    fn leaving_nova_os_from_an_app_returns_to_its_prompt() {
        let mut terminal = NovaOsTerminal::default();
        terminal.set_nova_os_commands(core_with([app_spec("map", "Open the local-space map")]));
        type_text(&mut terminal, "map");
        assert_eq!(
            terminal.submit(&TerminalCommandSnapshot::default()),
            TerminalSubmitOutcome::Launched
        );
        assert_eq!(terminal.active_mode(), TerminalMode::App { id: "map" });

        terminal.switch_shell(ShellKind::Commands);
        assert_eq!(terminal.active_mode(), TerminalMode::Prompt);
    }

    /// The Command shell answers catalog questions itself and queues everything
    /// that needs the live game, with the class the catalog documented.
    #[test]
    fn the_command_shell_answers_help_and_queues_world_commands() {
        let mut terminal = command_shell();

        type_text(&mut terminal, "help");
        assert_eq!(
            terminal.submit(&TerminalCommandSnapshot::default()),
            TerminalSubmitOutcome::Ran,
        );
        assert!(
            terminal.take_pending_command().is_none(),
            "help is answered from the catalog; the dispatcher never sees it",
        );
        assert!(terminal
            .scrollback()
            .iter()
            .any(|row| row.text.contains("commands in")));

        type_text(&mut terminal, "ammo infinite player_ship on");
        assert_eq!(
            terminal.submit(&TerminalCommandSnapshot::default()),
            TerminalSubmitOutcome::Dispatched,
        );
        let queued = terminal
            .take_pending_command()
            .expect("queued for dispatch");
        assert_eq!(queued.name, "ammo infinite");
        assert_eq!(queued.class, CommandClass::Cheat);
        assert_eq!(queued.args, ["player_ship", "on"]);

        // A typo is an error in the shell, and nothing reaches the dispatcher.
        type_text(&mut terminal, "ammoo");
        assert_eq!(
            terminal.submit(&TerminalCommandSnapshot::default()),
            TerminalSubmitOutcome::Errored,
        );
        assert!(terminal.take_pending_command().is_none());
    }

    /// The Command shell completes against its own catalog, so a half-typed
    /// cheat ghosts toward the real command and Tab finishes it.
    #[test]
    fn the_command_shell_completes_against_its_own_catalog() {
        let mut terminal = command_shell();
        type_text(&mut terminal, "cheats en");
        assert_eq!(terminal.parse_status(), TerminalParseStatus::ValidPrefix);
        assert_eq!(prompt_completion_ghost(&terminal), "able");
        assert!(terminal.complete());
        assert_eq!(terminal.prompt(), "cheats enable");

        // A NOVA OS word is not a Command-shell word: the vocabularies are per
        // shell, not one merged namespace.
        terminal.reset_prompt();
        type_text(&mut terminal, "objectives");
        assert_eq!(terminal.parse_status(), TerminalParseStatus::Valid);
        terminal.reset_prompt();
        type_text(&mut terminal, "log");
        assert_eq!(terminal.parse_status(), TerminalParseStatus::Invalid);
    }

    /// A new ship is a new NOVA OS session, and nothing else: the game-level
    /// Command transcript and history outlive the ship they were typed near.
    #[test]
    fn a_ship_reset_leaves_the_command_shell_alone() {
        let mut terminal = command_shell();
        type_text(&mut terminal, "status");
        terminal.submit(&TerminalCommandSnapshot::default());
        let rows = terminal.scrollback().len();

        terminal.reset_session();
        assert_eq!(terminal.active_shell(), ShellKind::Commands);
        assert_eq!(terminal.scrollback().len(), rows);
        terminal.history_previous();
        assert_eq!(terminal.prompt(), "status");

        terminal.switch_shell(ShellKind::NovaOs);
        assert_eq!(terminal.scrollback(), nova_os_welcome_rows());
        assert!(!terminal.is_booted(), "a fresh ship re-boots the NOVA OS");
    }
    /// `commands` is a step DOWN from the NOVA OS prompt, so Escape climbs back
    /// to it; a CRT opened straight into the Command shell has nothing
    /// underneath and Escape means close.
    #[test]
    fn commands_enters_the_command_shell_and_escape_climbs_back() {
        let mut terminal = NovaOsTerminal::default();
        terminal.insert_text("commands");
        assert_eq!(
            terminal.submit(&TerminalCommandSnapshot::default()),
            TerminalSubmitOutcome::Ran,
        );
        assert_eq!(terminal.active_shell(), ShellKind::Commands);
        assert_eq!(terminal.prompt_prefix(), "cmd> ");
        assert_eq!(terminal.back_out_shell(), Some(ShellKind::NovaOs));

        assert!(terminal.back_out());
        assert_eq!(terminal.active_shell(), ShellKind::NovaOs);
        assert_eq!(terminal.back_out_shell(), None);
        assert!(
            !terminal.back_out(),
            "the ground floor has nothing to climb back to",
        );
    }

    /// The switch clears the line it was typed on and leaves the line waiting in
    /// the shell it opens.
    #[test]
    fn entering_a_shell_clears_only_the_line_it_was_typed_on() {
        let mut terminal = NovaOsTerminal::default();
        terminal.switch_shell(ShellKind::Commands);
        terminal.insert_text("stat");
        terminal.switch_shell(ShellKind::NovaOs);

        terminal.insert_text("commands");
        terminal.submit(&TerminalCommandSnapshot::default());
        assert_eq!(terminal.active_shell(), ShellKind::Commands);
        assert_eq!(terminal.prompt(), "stat");

        terminal.switch_shell(ShellKind::NovaOs);
        assert_eq!(terminal.prompt(), "", "the NOVA OS line was submitted");
    }

    /// `open_shell` is the direct `:` open: it enters the shell with no level
    /// underneath, whether or not that shell was already active.
    #[test]
    fn opening_a_shell_directly_leaves_nothing_to_back_out_to() {
        let mut terminal = NovaOsTerminal::default();
        terminal.switch_shell(ShellKind::Commands);
        assert_eq!(terminal.back_out_shell(), Some(ShellKind::NovaOs));

        terminal.open_shell(ShellKind::Commands);
        assert_eq!(terminal.active_shell(), ShellKind::Commands);
        assert_eq!(terminal.back_out_shell(), None);
    }
}
