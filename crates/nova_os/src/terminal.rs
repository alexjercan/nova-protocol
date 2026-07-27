//! Terminal model: the [`NovaOsTerminal`] resource, its scrollback row types,
//! the boot/welcome/help content builders, and the prompt-render string
//! helpers. This is the pure command-prompt state the bevy UI in
//! `nova_gameplay` reads and drives.

use bevy::prelude::*;

use crate::{
    app::NovaOsAppCommand,
    shell::{
        command_meta, resolve_command, subcommands_of, terminal_command_names, CommandArity,
        ResolvedCommand, TERMINAL_COMMANDS,
    },
};

pub(crate) const NOVA_OS_PROMPT_PREFIX: &str = "nova> ";

/// The NOVA OS command prompt: the typed line, its parse state and completion
/// cycle, the rendered scrollback, the command history, and the boot-banner
/// queue. `nova_gameplay` inserts this as a bevy `Resource`, drives it from the
/// keyboard systems (via the `pub` edit/submit/completion methods), and reads it
/// back through the accessors to render the terminal UI.
#[derive(Resource, Debug, Clone)]
pub struct NovaOsTerminal {
    prompt: String,
    cursor: usize,
    scrollback: Vec<TerminalRow>,
    history: Vec<String>,
    history_cursor: Option<usize>,
    completion_hint: Option<String>,
    parse_status: TerminalParseStatus,
    active_mode: TerminalMode,
    /// Launch words mirrored from [`NovaOsAppRegistry`] so parsing/completion/help
    /// know the registered apps. Empty until [`sync_nova_os_app_commands`] fills
    /// it (and empty in the plain terminal-shell tests, which register no apps).
    app_commands: Vec<NovaOsAppCommand>,
    /// Set by the `exit` command; the keyboard system consumes it to drive the
    /// animated close of the computer (mirrors the HTML PoC's `exit`).
    pending_close: bool,
    /// Rows queued for the staggered boot banner, drained one-by-one by
    /// [`drain_nova_os_boot`] on real time. Empty except during a boot reveal.
    pending_rows: Vec<TerminalRow>,
    /// Whether the staggered boot banner has already played this session. Set on
    /// the first NOVA OS open; reset by [`Self::reset_session`] on ship teardown so
    /// a fresh ship re-boots.
    booted: bool,
    /// How many [`NovaOsFlightLog`] entries had been seen the last time the NOVA OS
    /// closed. The boot banner's "N unread events" line counts entries appended
    /// since (next to the log's own `seen_story` bookkeeping).
    seen_events: usize,
    /// The Tab-completion cycle stem: the text being completed. `None` when no
    /// cycle is active; reset on any prompt edit (PoC `resetCycle`). While a cycle
    /// runs, repeated Tab advances `cycle_index` through the matches for this stem.
    cycle_stem: Option<String>,
    /// The current index into the match list for the active [`Self::cycle_stem`].
    cycle_index: usize,
}

/// One rendered line of terminal scrollback: its semantic kind (which drives the
/// phosphor colour in the UI) and its text. `nova_gameplay`'s game-data bridges
/// build these directly, so the fields are public.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalRow {
    /// The semantic kind of the row, mapped to a phosphor colour by the UI.
    pub kind: TerminalRowKind,
    /// The row's text.
    pub text: String,
}

/// The semantic kind of a [`TerminalRow`], mirroring the HTML PoC's row classes.
/// The bevy UI maps each kind to a phosphor colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalRowKind {
    /// An echoed input line (`nova> ...`).
    Input,
    /// Ordinary command output.
    Output,
    /// De-emphasised text (diagnostics, hints, completion listings).
    Dim,
    /// Informational output (banners, section headers).
    Info,
    /// A warning (did-you-mean, unread hints).
    Warn,
    /// An error (unknown command, bad arguments).
    Error,
}

/// The parse state of the current prompt, driving the prompt colour, the inline
/// completion ghost and the hint line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalParseStatus {
    /// The prompt is empty.
    Empty,
    /// The prompt is a complete, arity-valid command.
    Valid,
    /// The prompt is a strict prefix of a longer command name.
    ValidPrefix,
    /// The prompt is not a command and is not a prefix of one.
    Invalid,
}

/// Which surface the NOVA OS screen is showing. `Prompt` is the command
/// terminal; `App` is a launched tool that has swallowed the terminal and owns
/// input until the user exits back to the prompt. The app id is `&'static str`
/// (an app's stable launch word) so the mode stays `Copy` and allocation-free;
/// the terminal scrollback is never touched while an app is active, so exiting
/// simply restores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalMode {
    /// The command prompt owns input.
    Prompt,
    /// A launched app owns the screen and input; `id` is its launch word.
    App {
        /// The active app's stable launch word / id.
        id: &'static str,
    },
}

/// Live game data resolved into terminal rows, passed into
/// [`NovaOsTerminal::submit`] so the pure model can serve `log`/`objectives`/
/// `ship`/`clear` without reaching into the bevy world itself. `nova_gameplay`
/// builds this each submit from the flight log, objectives and ship state.
#[derive(Debug, Clone, Default)]
pub struct TerminalCommandSnapshot {
    /// Rows for the `log` command (comms + mission events).
    pub log_rows: Vec<TerminalRow>,
    /// Rows for the `objectives` command (active objectives).
    pub objective_rows: Vec<TerminalRow>,
    /// Rows for the `ship` command (section status summary).
    pub ship_rows: Vec<TerminalRow>,
    /// Rows for the `map view` command (local-space contact list). Built by
    /// `nova_gameplay` from the same contact model the `map` app renders.
    pub map_rows: Vec<TerminalRow>,
    /// Flight-log entries appended since the NOVA OS last closed, for the boot
    /// banner's "N unread events" line (0 in the default snapshot).
    pub unread_events: usize,
    /// A short hook for the most recent unread event, appended to that line.
    pub unread_hook: Option<String>,
}

impl Default for NovaOsTerminal {
    fn default() -> Self {
        let mut terminal = Self {
            prompt: String::new(),
            cursor: 0,
            scrollback: nova_os_welcome_rows(),
            history: Vec::new(),
            history_cursor: None,
            completion_hint: Some("type help".to_string()),
            parse_status: TerminalParseStatus::Empty,
            active_mode: TerminalMode::Prompt,
            app_commands: Vec::new(),
            pending_close: false,
            pending_rows: Vec::new(),
            booted: false,
            seen_events: 0,
            cycle_stem: None,
            cycle_index: 0,
        };
        terminal.refresh_parse();
        terminal
    }
}

impl NovaOsTerminal {
    /// The rendered scrollback rows, oldest first.
    pub fn scrollback(&self) -> &[TerminalRow] {
        &self.scrollback
    }

    /// Append rows to the scrollback (e.g. an objective completion announced
    /// while the prompt is open).
    pub fn extend_scrollback(&mut self, rows: impl IntoIterator<Item = TerminalRow>) {
        self.scrollback.extend(rows);
    }

    /// The current prompt text.
    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    /// The caret's byte offset within the prompt.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// The prompt's parse status.
    pub fn parse_status(&self) -> TerminalParseStatus {
        self.parse_status
    }

    /// The current completion hint, if any.
    pub fn completion_hint(&self) -> Option<&str> {
        self.completion_hint.as_deref()
    }

    /// Which surface currently owns the screen (prompt or a launched app).
    pub fn active_mode(&self) -> TerminalMode {
        self.active_mode
    }

    /// The registered app launch words mirrored into the terminal.
    pub fn app_commands(&self) -> &[NovaOsAppCommand] {
        &self.app_commands
    }

    /// Replace the mirrored app launch words and re-parse. The caller compares
    /// against [`Self::app_commands`] first so this only fires (marking the
    /// resource changed) when the set actually changed.
    pub fn set_app_commands(&mut self, commands: Vec<NovaOsAppCommand>) {
        self.app_commands = commands;
        self.refresh_parse();
    }

    /// Whether the `exit` command has requested an animated close, clearing the
    /// request as it is read.
    pub fn take_pending_close(&mut self) -> bool {
        let pending = self.pending_close;
        self.pending_close = false;
        pending
    }

    /// How many flight-log entries had been seen the last time the computer
    /// closed (for the boot banner's unread count).
    pub fn seen_events(&self) -> usize {
        self.seen_events
    }

    /// Record how many flight-log entries have been seen (called on close).
    pub fn set_seen_events(&mut self, seen: usize) {
        self.seen_events = seen;
    }

    /// Whether the staggered boot banner has already played this session.
    pub fn is_booted(&self) -> bool {
        self.booted
    }

    /// Kick off the staggered boot banner: mark booted, clear the scrollback and
    /// queue `rows` for [`Self::reveal_next_boot_row`] to reveal one-by-one.
    pub fn begin_boot(&mut self, rows: Vec<TerminalRow>) {
        self.booted = true;
        self.scrollback.clear();
        self.pending_rows = rows;
    }

    /// Whether any boot-banner rows are still queued.
    pub fn has_pending_boot_rows(&self) -> bool {
        !self.pending_rows.is_empty()
    }

    /// Reveal the next queued boot-banner row into the scrollback. Returns
    /// whether a row was revealed (`false` when the queue is empty).
    pub fn reveal_next_boot_row(&mut self) -> bool {
        if self.pending_rows.is_empty() {
            return false;
        }
        let row = self.pending_rows.remove(0);
        self.scrollback.push(row);
        true
    }

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

        self.scrollback.push(TerminalRow {
            kind: TerminalRowKind::Input,
            text: format!("{NOVA_OS_PROMPT_PREFIX}{command_line}"),
        });
        self.history.push(command_line.clone());
        self.history_cursor = None;
        self.cycle_stem = None;

        // One matcher resolves built-ins AND registered app launch words as
        // (possibly multi-word) names with per-command arity - a launch leaves the
        // scrollback untouched (exit restores it) and hands the screen to the app.
        let outcome = match resolve_command(&command_line, &self.app_commands) {
            ResolvedCommand::App { id } => {
                self.scrollback.push(TerminalRow {
                    kind: TerminalRowKind::Info,
                    text: format!("launching {id} ..."),
                });
                self.active_mode = TerminalMode::App { id };
                TerminalSubmitOutcome::Launched
            }
            ResolvedCommand::Builtin { name } => match name {
                "help" => {
                    self.scrollback
                        .extend(terminal_help_rows(&self.app_commands));
                    TerminalSubmitOutcome::Ran
                }
                "clear" => {
                    self.reset_scrollback_to_welcome(snapshot);
                    TerminalSubmitOutcome::Ran
                }
                "log" => {
                    self.scrollback.extend(snapshot.log_rows.clone());
                    TerminalSubmitOutcome::Ran
                }
                "objectives" => {
                    self.scrollback.extend(snapshot.objective_rows.clone());
                    TerminalSubmitOutcome::Ran
                }
                "ship" => {
                    self.scrollback.extend(snapshot.ship_rows.clone());
                    TerminalSubmitOutcome::Ran
                }
                "map view" => {
                    self.scrollback.extend(snapshot.map_rows.clone());
                    TerminalSubmitOutcome::Ran
                }
                "version" => {
                    self.scrollback.extend(nova_os_version_rows());
                    TerminalSubmitOutcome::Ran
                }
                "exit" => {
                    self.pending_close = true;
                    TerminalSubmitOutcome::Ran
                }
                // Every built-in name in TERMINAL_COMMANDS is handled above.
                _ => TerminalSubmitOutcome::Ran,
            },
            ResolvedCommand::Usage { name } => {
                self.scrollback
                    .extend(command_help_rows(name, &self.app_commands));
                TerminalSubmitOutcome::Ran
            }
            ResolvedCommand::Version { .. } => {
                self.scrollback.extend(nova_os_version_rows());
                TerminalSubmitOutcome::Ran
            }
            ResolvedCommand::UnexpectedArguments { command, arity } => {
                // A bad argument shows WHY plus the command's usage, rather than a
                // bare "takes no arguments" - and names the sub-commands when the
                // command has them (e.g. `map v` -> `map view`).
                let subs = subcommands_of(&command, &self.app_commands);
                self.scrollback.push(TerminalRow {
                    kind: TerminalRowKind::Error,
                    text: if subs.is_empty() {
                        format!("{command} {}", arity.rejection())
                    } else {
                        format!("{command}: unknown sub-command")
                    },
                });
                self.scrollback
                    .extend(command_help_rows(&command, &self.app_commands));
                TerminalSubmitOutcome::Errored
            }
            ResolvedCommand::Unknown {
                command,
                suggestion,
            } => {
                // Two rows, matching the HTML PoC's `command not found` +
                // `did you mean ...?` wording.
                self.scrollback.push(TerminalRow {
                    kind: TerminalRowKind::Error,
                    text: format!("command not found: {command}"),
                });
                if let Some(suggestion) = suggestion {
                    self.scrollback.push(TerminalRow {
                        kind: TerminalRowKind::Warn,
                        text: format!("did you mean {suggestion}?"),
                    });
                }
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
            self.scrollback.push(TerminalRow {
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
        let mut matches: Vec<String> = terminal_command_names(&self.app_commands)
            .filter(|name| name.starts_with(stem))
            .map(|name| name.to_string())
            .collect();
        for name in terminal_command_names(&self.app_commands) {
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
        matches
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
        let trimmed = self.prompt.trim();
        if trimmed.is_empty() {
            self.parse_status = TerminalParseStatus::Empty;
            self.completion_hint = Some("type help".to_string());
            return;
        }
        match resolve_command(trimmed, &self.app_commands) {
            // A full, arity-valid command (built-in or app launch word), or a
            // `<command> help` usage request - all valid input.
            ResolvedCommand::App { .. }
            | ResolvedCommand::Builtin { .. }
            | ResolvedCommand::Usage { .. }
            | ResolvedCommand::Version { .. } => {
                self.parse_status = TerminalParseStatus::Valid;
                self.completion_hint = None;
            }
            // Trailing words that overrun a command's arity - unless the whole
            // input is still a prefix of a LONGER command name (e.g. `ship vi`
            // toward `ship view`), in which case it is a valid prefix, not an
            // error.
            ResolvedCommand::UnexpectedArguments { command, arity } => {
                if let Some(name) = self.command_name_starting_with(trimmed) {
                    self.parse_status = TerminalParseStatus::ValidPrefix;
                    self.completion_hint = Some(name.to_string());
                } else {
                    self.parse_status = TerminalParseStatus::Invalid;
                    self.completion_hint = Some(format!("{command} {}", arity.rejection()));
                }
            }
            ResolvedCommand::Unknown { suggestion, .. } => {
                if let Some(name) = self.command_name_starting_with(trimmed) {
                    self.parse_status = TerminalParseStatus::ValidPrefix;
                    self.completion_hint = Some(name.to_string());
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
    pub fn enter_app(&mut self, id: &'static str) {
        self.active_mode = TerminalMode::App { id };
    }

    /// Reprint the boot banner instantly (PoC `clear` -> `printBanner(true)`),
    /// including the current unread-events line from `snapshot`.
    fn reset_scrollback_to_welcome(&mut self, snapshot: &TerminalCommandSnapshot) {
        self.scrollback =
            nova_os_boot_banner_rows(snapshot.unread_events, snapshot.unread_hook.clone());
    }

    /// Return from an active app to the command terminal. The scrollback and
    /// prompt are untouched while an app runs, so this just flips the mode back;
    /// a no-op when already at the prompt. Drives both the Escape/close-control
    /// route and an app's own [`crate::app::NovaOsAppInputOutcome::Exit`].
    /// Returns whether an app was actually exited (so the caller can play the
    /// degauss coil only on a real app -> prompt transition).
    pub fn exit_app(&mut self) -> bool {
        if matches!(self.active_mode, TerminalMode::App { .. }) {
            self.active_mode = TerminalMode::Prompt;
            true
        } else {
            false
        }
    }

    /// Reset to a fresh session (a new ship): clear the prompt, scrollback and
    /// history and re-arm the staggered boot banner.
    pub fn reset_session(&mut self) {
        self.prompt.clear();
        self.cursor = 0;
        self.scrollback = nova_os_welcome_rows();
        self.history.clear();
        self.history_cursor = None;
        self.cycle_stem = None;
        self.active_mode = TerminalMode::Prompt;
        // A fresh ship is a fresh session: the next open re-runs the boot banner.
        self.pending_rows.clear();
        self.booted = false;
        self.seen_events = 0;
        self.refresh_parse();
    }

    fn set_history_cursor(&mut self, cursor: usize) {
        self.history_cursor = Some(cursor);
        self.cycle_stem = None;
        self.prompt = self.history[cursor].clone();
        self.cursor = self.prompt.len();
        self.refresh_parse();
    }
}

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

/// Build the `help` output: the built-in command table plus registered app
/// launch words, aligned in one column.
pub fn terminal_help_rows(app_commands: &[NovaOsAppCommand]) -> Vec<TerminalRow> {
    // App launch words share the aligned command column with the built-ins.
    let command_width = TERMINAL_COMMANDS
        .iter()
        .map(|command| command.name.len())
        .chain(app_commands.iter().map(|app| app.id.len()))
        .max()
        .unwrap_or(0);
    let builtins = TERMINAL_COMMANDS
        .iter()
        .map(|command| (command.name, command.summary));
    let apps = app_commands.iter().map(|app| (app.id, app.summary));
    std::iter::once(TerminalRow {
        kind: TerminalRowKind::Info,
        text: "Available commands:".to_string(),
    })
    .chain(
        builtins
            .chain(apps)
            .map(move |(name, summary)| TerminalRow {
                kind: TerminalRowKind::Output,
                text: format!("  {name:command_width$}  {summary}"),
            }),
    )
    .collect()
}

/// Per-command help (`<command> help`): a one-line summary, the usage syntax, any
/// sub-commands, and a version stamp - the shell-emulator touch. Works for every
/// registered command, built-in or app.
pub fn command_help_rows(name: &str, app_commands: &[NovaOsAppCommand]) -> Vec<TerminalRow> {
    let Some((summary, arity)) = command_meta(name, app_commands) else {
        return vec![TerminalRow {
            kind: TerminalRowKind::Error,
            text: format!("no help for '{name}'"),
        }];
    };
    let mut rows = vec![TerminalRow {
        kind: TerminalRowKind::Info,
        text: format!("{name} - {summary}"),
    }];
    rows.push(TerminalRow {
        kind: TerminalRowKind::Output,
        text: match arity {
            CommandArity::None => format!("usage: {name}"),
            CommandArity::UpTo(_) => format!("usage: {name} <arg>"),
        },
    });
    let subs = subcommands_of(name, app_commands);
    if !subs.is_empty() {
        rows.push(TerminalRow {
            kind: TerminalRowKind::Output,
            text: format!("subcommands: {}", subs.join(", ")),
        });
    }
    rows.push(TerminalRow {
        kind: TerminalRowKind::Dim,
        text: format!(
            "NOVA OS {} - try '{name} help' anytime",
            nova_os_version_label()
        ),
    });
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

/// The semantic result of a [`NovaOsTerminal::submit`], so the bevy layer can
/// pick the sound cue without the pure model knowing about audio (task
/// 20260726-214639).
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

/// The typed text left of the caret. The prompt line is rendered as three
/// inline pieces - `before` | caret | `after` - plus the dim ghost, so the fish
/// completion continues on the SAME line right after the typed text with a real
/// caret between them (no `|` glyph baked into the text, no leading space).
pub fn prompt_before_cursor(terminal: &NovaOsTerminal) -> String {
    // The edit methods keep `cursor` on a char boundary; assert it here since
    // this slice would panic otherwise and `cursor` is now reachable only
    // through the crate's own getters.
    debug_assert!(terminal.prompt.is_char_boundary(terminal.cursor));
    terminal.prompt[..terminal.cursor].to_string()
}

/// The typed text right of the caret (empty when the caret sits at the end).
pub fn prompt_after_cursor(terminal: &NovaOsTerminal) -> String {
    debug_assert!(terminal.prompt.is_char_boundary(terminal.cursor));
    terminal.prompt[terminal.cursor..].to_string()
}

/// The hint line shown under the prompt while the input is invalid (empty
/// otherwise).
pub fn prompt_hint_display(terminal: &NovaOsTerminal) -> String {
    if terminal.parse_status == TerminalParseStatus::Invalid {
        terminal.completion_hint.clone().unwrap_or_default()
    } else {
        String::new()
    }
}

/// The dim inline completion ghost: the suffix of the command name the prompt is
/// completing toward (empty unless the prompt is a valid prefix).
pub fn prompt_completion_ghost(terminal: &NovaOsTerminal) -> String {
    if terminal.parse_status != TerminalParseStatus::ValidPrefix {
        return String::new();
    }
    // On a valid prefix `completion_hint` holds the full command name the input is
    // completing toward (built-in or app launch word, single- or multi-word); the
    // ghost is the suffix past what has been typed.
    terminal
        .completion_hint
        .as_deref()
        .and_then(|name| name.strip_prefix(terminal.prompt.as_str()))
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::NovaOsAppCommand,
        shell::{CommandArity, TERMINAL_COMMANDS},
    };

    fn type_text(terminal: &mut NovaOsTerminal, text: &str) {
        terminal.insert_text(text);
    }
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
            terminal.scrollback.len() > nova_os_welcome_rows().len(),
            "help adds rows after the welcome block"
        );

        type_text(&mut terminal, "clear");
        terminal.submit(&TerminalCommandSnapshot::default());

        assert_eq!(terminal.scrollback, nova_os_welcome_rows());
        assert_eq!(terminal.prompt, "");
        assert_eq!(terminal.completion_hint.as_deref(), Some("type help"));
    }
    #[test]
    fn nova_os_help_rows_are_generated_from_registered_commands() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "help");
        terminal.submit(&TerminalCommandSnapshot::default());

        let help_rows = &terminal.scrollback[nova_os_welcome_rows().len() + 1..];
        assert_eq!(
            help_rows,
            &[
                TerminalRow {
                    kind: TerminalRowKind::Info,
                    text: "Available commands:".to_string()
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
                    text: "  ship        Print ship status summary".to_string()
                },
                TerminalRow {
                    kind: TerminalRowKind::Output,
                    text: "  map view    Print local-space contacts".to_string()
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
                    text: "  exit        Suspend the NOVA OS computer".to_string()
                }
            ]
        );
    }
    #[test]
    fn nova_os_prompt_renders_fish_style_completion_ghost() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "he");

        assert_eq!(terminal.parse_status, TerminalParseStatus::ValidPrefix);
        assert_eq!(prompt_before_cursor(&terminal), "he");
        assert_eq!(prompt_after_cursor(&terminal), "");
        assert_eq!(prompt_completion_ghost(&terminal), "lp");
        assert_eq!(prompt_hint_display(&terminal), "");

        type_text(&mut terminal, "zz");
        assert_eq!(prompt_completion_ghost(&terminal), "");
        assert_eq!(prompt_hint_display(&terminal), "did you mean help?");
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
        // Two HTML-style rows: the error line then the suggestion line.
        let rows: Vec<(TerminalRowKind, &str)> = terminal
            .scrollback
            .iter()
            .map(|row| (row.kind, row.text.as_str()))
            .collect();
        assert!(rows.contains(&(TerminalRowKind::Error, "command not found: hlep")));
        assert!(rows.contains(&(TerminalRowKind::Warn, "did you mean help?")));
    }
    #[test]
    fn terminal_rejects_unexpected_command_arguments() {
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "help garbage");
        assert_eq!(terminal.parse_status, TerminalParseStatus::Invalid);
        assert_eq!(
            terminal.completion_hint.as_deref(),
            Some("help takes no arguments")
        );
        terminal.submit(&TerminalCommandSnapshot::default());
        // The error line is printed, followed by the command's usage (so the
        // player sees how to use it); assert the error is present.
        assert!(
            terminal
                .scrollback
                .iter()
                .any(|row| row.text == "help takes no arguments"),
            "the rejection names the command and reason",
        );

        type_text(&mut terminal, "clear garbage");
        terminal.submit(&TerminalCommandSnapshot::default());
        assert!(
            !terminal.scrollback.is_empty(),
            "clear with unexpected arguments reports an error instead of clearing scrollback"
        );
        assert!(
            terminal
                .scrollback
                .iter()
                .any(|row| row.text == "clear takes no arguments"),
            "clear rejects its argument with a reason",
        );
    }
    #[test]
    fn nova_os_help_lists_html_command_set() {
        // `help` output lists exactly the executable set, in HTML order.
        let mut terminal = NovaOsTerminal::default();
        type_text(&mut terminal, "help");
        terminal.submit(&TerminalCommandSnapshot::default());
        let listed: Vec<String> = terminal
            .scrollback
            .iter()
            .filter_map(|row| {
                let trimmed = row.text.trim_start();
                TERMINAL_COMMANDS
                    .iter()
                    .map(|command| command.name)
                    .find(|name| trimmed.starts_with(name))
                    .filter(|name| trimmed.starts_with(&format!("{name} ")))
                    .map(str::to_string)
            })
            .collect();
        assert_eq!(
            listed,
            vec![
                "help",
                "log",
                "objectives",
                "ship",
                "map view",
                "clear",
                "version",
                "exit"
            ]
        );
    }
    #[test]
    fn nova_os_command_help_and_version() {
        let mut terminal = NovaOsTerminal::default();
        terminal.app_commands = vec![NovaOsAppCommand {
            id: "map",
            summary: "Open the local-space map",
            arity: crate::shell::CommandArity::None,
        }];

        // `<command> help` prints the command's summary, usage and sub-commands.
        type_text(&mut terminal, "map help");
        terminal.submit(&TerminalCommandSnapshot::default());
        let text = |t: &NovaOsTerminal| {
            t.scrollback
                .iter()
                .map(|row| row.text.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let out = text(&terminal);
        assert!(out.contains("map - Open the local-space map"), "{out}");
        assert!(out.contains("usage: map"), "{out}");
        assert!(out.contains("subcommands: map view"), "{out}");

        // A bad sub-command is a named error + usage, not "takes no arguments".
        type_text(&mut terminal, "map v");
        terminal.submit(&TerminalCommandSnapshot::default());
        assert!(
            terminal
                .scrollback
                .iter()
                .any(|row| row.text == "map: unknown sub-command"),
            "map v names the bad sub-command",
        );

        // `version` prints the version banner.
        type_text(&mut terminal, "version");
        terminal.submit(&TerminalCommandSnapshot::default());
        assert!(
            terminal
                .scrollback
                .iter()
                .any(|row| row.text.starts_with("NOVA OS v")),
            "version prints the NOVA OS version",
        );
    }
    #[test]
    fn nova_os_subcommand_completion_and_ghost() {
        let mut terminal = NovaOsTerminal::default();
        terminal.app_commands = vec![NovaOsAppCommand {
            id: "map",
            summary: "Open the local-space map",
            arity: crate::shell::CommandArity::None,
        }];

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
                .scrollback
                .iter()
                .any(|row| row.text.starts_with("NOVA OS v")),
            "map version prints the NOVA OS version",
        );
    }
    /// Tab lists ambiguous matches on the first press and cycles through them on
    /// repeats, resetting the cycle on any edit.
    #[test]
    fn nova_os_tab_cycles_ambiguous_completions() {
        let mut terminal = NovaOsTerminal::default();
        // Two app words sharing the `sh` stem with the `ship` built-in make the
        // stem ambiguous (three matches).
        terminal.app_commands = vec![
            NovaOsAppCommand {
                id: "shield",
                summary: "",
                arity: CommandArity::None,
            },
            NovaOsAppCommand {
                id: "shells",
                summary: "",
                arity: CommandArity::None,
            },
        ];
        terminal.insert_text("sh");

        // First Tab lists the matches and jumps to the first.
        assert!(terminal.complete());
        assert_eq!(
            terminal.scrollback.last().map(|row| row.text.as_str()),
            Some("ship   shield   shells"),
            "the first Tab on an ambiguous stem lists the matches",
        );
        assert_eq!(terminal.prompt, "ship");

        // Repeat presses cycle through the rest, then wrap.
        terminal.complete();
        assert_eq!(terminal.prompt, "shield");
        terminal.complete();
        assert_eq!(terminal.prompt, "shells");
        terminal.complete();
        assert_eq!(
            terminal.prompt, "ship",
            "cycling wraps back to the first match"
        );

        // The match list is printed once, not on every cycle press.
        let listings = terminal
            .scrollback
            .iter()
            .filter(|row| row.text == "ship   shield   shells")
            .count();
        assert_eq!(listings, 1);

        // Any edit resets the cycle (PoC `resetCycle`).
        terminal.insert_text("x");
        assert!(terminal.cycle_stem.is_none(), "editing resets the cycle");
    }
}
