//! The [`NovaOsTerminal`] resource and its row/mode types, plus the accessors
//! and session lifecycle (boot reveal, app enter/exit, reset) the bevy layer
//! drives it through. Prompt editing lives in [`super::edit`], the rendered
//! content in [`super::view`].

use std::collections::HashMap;

use bevy::prelude::*;

use super::view::{nova_os_boot_banner_rows, nova_os_welcome_rows};
use crate::{command::prelude::core_command_specs, shell::prelude::TerminalCommandSpec};

/// The most scrollback rows the terminal keeps. The UI spawns one `Text` entity
/// per row on every rebuild, so an unbounded scrollback is an unbounded entity
/// count for a session that never ends; the oldest rows are dropped past this.
pub const MAX_SCROLLBACK_ROWS: usize = 500;

/// The NOVA OS command prompt: the typed line, its parse state and completion
/// cycle, the rendered scrollback, the command history, and the boot-banner
/// queue. `nova_gameplay` inserts this as a bevy `Resource`, drives it from the
/// keyboard systems (via the `pub` edit/submit/completion methods), and reads it
/// back through the accessors to render the terminal UI.
#[derive(Resource, Debug, Clone)]
pub struct NovaOsTerminal {
    pub(super) prompt: String,
    pub(super) cursor: usize,
    scrollback: Vec<TerminalRow>,
    /// Bumped by every scrollback mutation. The UI rebuilds one `Text` entity per
    /// row, so it needs to know when the ROWS changed rather than when anything on
    /// the resource did - a caret move marks the whole resource changed and must
    /// not reach the row loop.
    scrollback_revision: u64,
    pub(super) history: Vec<String>,
    pub(super) history_cursor: Option<usize>,
    pub(super) completion_hint: Option<String>,
    pub(super) parse_status: TerminalParseStatus,
    pub(super) active_mode: TerminalMode,
    /// Every command mirrored from the [`crate::command::NovaOsCommandRegistry`]
    /// so parsing/completion/help know the full set. Seeded with the core builtins
    /// by [`Default`]; the registered apps and their subcommands (e.g. `map` /
    /// `map view`) are folded in by `sync_nova_os_commands` in `nova_gameplay`.
    pub(super) commands: Vec<TerminalCommandSpec>,
    /// Set by the `exit` command; the keyboard system consumes it to drive the
    /// animated close of the computer (mirrors the HTML PoC's `exit`).
    pub(super) pending_close: bool,
    /// Rows queued for the staggered boot banner, drained one-by-one by
    /// [`drain_nova_os_boot`] on real time. Empty except during a boot reveal.
    pub(super) pending_rows: Vec<TerminalRow>,
    /// Whether the staggered boot banner has already played this session. Set on
    /// the first NOVA OS open; reset by [`Self::reset_session`] on ship teardown so
    /// a fresh ship re-boots.
    pub(super) booted: bool,
    /// How many [`NovaOsFlightLog`] entries had been seen the last time the NOVA OS
    /// closed. The boot banner's "N unread events" line counts entries appended
    /// since (next to the log's own `seen_story` bookkeeping).
    pub(super) seen_events: usize,
    /// The Tab-completion cycle stem: the text being completed. `None` when no
    /// cycle is active; reset on any prompt edit (PoC `resetCycle`). While a cycle
    /// runs, repeated Tab advances `cycle_index` through the matches for this stem.
    pub(super) cycle_stem: Option<String>,
    /// The current index into the match list for the active [`Self::cycle_stem`].
    pub(super) cycle_index: usize,
    /// An arg-bearing gameplay command
    /// ([`CommandDispatch::Gameplay`](crate::shell::CommandDispatch::Gameplay))
    /// that [`Self::submit`] resolved and is waiting for the gameplay layer to
    /// apply.
    /// `nova_gameplay` drains it with [`Self::take_pending_invocation`], runs the
    /// action against the live world, and appends the result rows. `None` except
    /// in the frame a `ship section/reload/repair <id>` line was submitted.
    pub(super) pending_invocation: Option<NovaOsCommandInvocation>,
    /// Completion candidates for the argument of an arg-bearing command, keyed by
    /// command name (`"ship repair" -> ["HULL-1", "PDC-1", ...]`). The pure
    /// terminal cannot enumerate live section codes, so `nova_gameplay` injects
    /// them via [`Self::merge_arg_completions`]; Tab and the ghost read them.
    pub(super) arg_completions: HashMap<&'static str, Vec<String>>,
}

/// A resolved arg-bearing gameplay command awaiting application by the gameplay
/// layer: the matched (possibly multi-word) command name and the argument words
/// the player typed after it. Produced by [`NovaOsTerminal::submit`] for a
/// [`CommandDispatch::Gameplay`](crate::shell::CommandDispatch::Gameplay)
/// command and drained with [`NovaOsTerminal::take_pending_invocation`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NovaOsCommandInvocation {
    /// The resolved command name (`"ship reload"`, `"ship section"`).
    pub name: &'static str,
    /// The argument words past the command name (the `<id>`).
    pub args: Vec<String>,
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
/// [`NovaOsTerminal::submit`] so the pure model can serve the snapshot-backed CLI
/// commands (`log`/`objectives`/`ship view`/`map view`/`clear`) without reaching into
/// the bevy world itself. `nova_gameplay` builds this each submit from the flight
/// log, objectives, ship state and contact model.
#[derive(Debug, Clone, Default)]
pub struct TerminalCommandSnapshot {
    /// Pre-built output rows keyed by command name, for every
    /// [`CliOutput::Snapshot`](crate::shell::CliOutput::Snapshot) command
    /// (`"log"`, `"objectives"`, `"ship view"`, `"map view"`). A command with no
    /// entry prints nothing.
    pub command_output: HashMap<&'static str, Vec<TerminalRow>>,
    /// Flight-log entries appended since the NOVA OS last closed, for the boot
    /// banner's "N unread events" line (0 in the default snapshot).
    pub unread_events: usize,
    /// A short hook for the most recent unread event, appended to that line.
    pub unread_hook: Option<String>,
}

impl TerminalCommandSnapshot {
    /// Store the pre-built output rows for the command named `command`, replacing
    /// any prior entry. Returns `self` for builder-style construction.
    pub fn with_output(mut self, command: &'static str, rows: Vec<TerminalRow>) -> Self {
        self.command_output.insert(command, rows);
        self
    }

    /// The rows for `command`, or an empty slice when the caller supplied none.
    pub(super) fn output(&self, command: &str) -> Vec<TerminalRow> {
        self.command_output
            .get(command)
            .cloned()
            .unwrap_or_default()
    }
}

impl Default for NovaOsTerminal {
    fn default() -> Self {
        let mut terminal = Self {
            prompt: String::new(),
            cursor: 0,
            scrollback: nova_os_welcome_rows(),
            scrollback_revision: 0,
            history: Vec::new(),
            history_cursor: None,
            completion_hint: Some("type help".to_string()),
            parse_status: TerminalParseStatus::Empty,
            active_mode: TerminalMode::Prompt,
            commands: core_command_specs(),
            pending_close: false,
            pending_rows: Vec::new(),
            booted: false,
            seen_events: 0,
            cycle_stem: None,
            cycle_index: 0,
            pending_invocation: None,
            arg_completions: HashMap::new(),
        };
        terminal.refresh_parse();
        terminal
    }
}

/// How the parser strips a raw prompt line. The single definition behind
/// [`NovaOsTerminal::parsed_prompt`]; `refresh_parse` calls it on the field
/// directly because it writes the parse result back in the same statement.
pub(super) fn parsed_prompt(prompt: &str) -> &str {
    prompt.trim()
}

impl NovaOsTerminal {
    /// The rendered scrollback rows, oldest first.
    pub fn scrollback(&self) -> &[TerminalRow] {
        &self.scrollback
    }

    /// A counter that changes exactly when the scrollback rows change. The UI
    /// keys its row rebuild on this so prompt edits and caret moves - which mark
    /// the whole resource changed - do not respawn every row.
    pub fn scrollback_revision(&self) -> u64 {
        self.scrollback_revision
    }

    /// Append rows to the scrollback (e.g. an objective completion announced
    /// while the prompt is open).
    pub fn extend_scrollback(&mut self, rows: impl IntoIterator<Item = TerminalRow>) {
        self.scrollback.extend(rows);
        self.after_scrollback_change();
    }

    /// Append one row to the scrollback.
    pub(super) fn push_row(&mut self, row: TerminalRow) {
        self.scrollback.push(row);
        self.after_scrollback_change();
    }

    /// Replace the whole scrollback (the `clear` command and a session reset).
    pub(super) fn replace_scrollback(&mut self, rows: Vec<TerminalRow>) {
        self.scrollback = rows;
        self.after_scrollback_change();
    }

    /// Bump the revision and drop the oldest rows past [`MAX_SCROLLBACK_ROWS`].
    /// Every scrollback mutation ends here, so neither the cap nor the revision
    /// can be bypassed by a new caller.
    fn after_scrollback_change(&mut self) {
        self.scrollback_revision = self.scrollback_revision.wrapping_add(1);
        let excess = self.scrollback.len().saturating_sub(MAX_SCROLLBACK_ROWS);
        if excess > 0 {
            self.scrollback.drain(..excess);
        }
    }

    /// The current prompt text.
    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    /// The prompt as the parser sees it. Every reader of the parse result - the
    /// status, the hint and the inline ghost - must strip the prompt the same way
    /// the parse did, or a leading space greens the prompt with no ghost.
    pub fn parsed_prompt(&self) -> &str {
        parsed_prompt(&self.prompt)
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

    /// The full command set mirrored into the terminal (core builtins plus any
    /// registered apps and their subcommands).
    pub fn command_specs(&self) -> &[TerminalCommandSpec] {
        &self.commands
    }

    /// Replace the mirrored command set and re-parse. The caller compares against
    /// [`Self::command_specs`] first so this only fires (marking the resource
    /// changed) when the set actually changed.
    pub fn set_commands(&mut self, commands: Vec<TerminalCommandSpec>) {
        self.commands = commands;
        self.refresh_parse();
    }

    /// The argument-completion candidates currently injected by the gameplay
    /// layer, keyed by command name. The caller compares against this before
    /// calling [`Self::merge_arg_completions`] so it only marks the resource
    /// changed when the live set actually changed.
    pub fn arg_completions(&self) -> &HashMap<&'static str, Vec<String>> {
        &self.arg_completions
    }

    /// Merge arg-completion candidates for the given verbs, leaving other verbs'
    /// entries intact, so several gameplay-verb apps (`ship`, `map`) can each own
    /// their own verbs without clobbering the shared map. Only re-parses when a
    /// value actually changed.
    pub fn merge_arg_completions(
        &mut self,
        entries: impl IntoIterator<Item = (&'static str, Vec<String>)>,
    ) {
        let mut changed = false;
        for (name, candidates) in entries {
            match self.arg_completions.get(name) {
                Some(existing) if *existing == candidates => {}
                _ => {
                    self.arg_completions.insert(name, candidates);
                    changed = true;
                }
            }
        }
        if changed {
            self.refresh_parse();
        }
    }

    /// Take the arg-bearing gameplay invocation queued by the last
    /// [`Self::submit`], if any. `nova_gameplay` calls this right after submit,
    /// applies the action against the live world, and appends the result rows via
    /// [`Self::extend_scrollback`].
    pub fn take_pending_invocation(&mut self) -> Option<NovaOsCommandInvocation> {
        self.pending_invocation.take()
    }

    /// Peek at the queued gameplay invocation without consuming it. Handlers for
    /// distinct verb families (`ship ...` vs `map ...`) share the single pending
    /// slot, so each peeks first and only [`Self::take_pending_invocation`]s the
    /// invocation whose name it owns - otherwise whichever handler runs first
    /// would swallow (and mis-handle) another app's verb.
    pub fn peek_pending_invocation(&self) -> Option<&NovaOsCommandInvocation> {
        self.pending_invocation.as_ref()
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
        self.replace_scrollback(Vec::new());
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
        self.push_row(row);
        true
    }

    /// Hand the screen to the app with launch word `id`, the same transition
    /// [`Self::submit`] performs for an app launch word. Pairs with
    /// [`Self::exit_app`].
    pub fn enter_app(&mut self, id: &'static str) {
        self.active_mode = TerminalMode::App { id };
    }

    /// Reprint the boot banner instantly (PoC `clear` -> `printBanner(true)`),
    /// including the current unread-events line from `snapshot`.
    pub(super) fn reset_scrollback_to_welcome(&mut self, snapshot: &TerminalCommandSnapshot) {
        self.replace_scrollback(nova_os_boot_banner_rows(
            snapshot.unread_events,
            snapshot.unread_hook.clone(),
        ));
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
        self.replace_scrollback(nova_os_welcome_rows());
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
}
