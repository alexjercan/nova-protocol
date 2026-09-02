//! The [`NovaOsTerminal`] resource and its row/mode types, plus the accessors
//! and session lifecycle (boot reveal, app enter/exit, reset) the bevy layer
//! drives it through. Prompt editing lives in [`super::edit`], the rendered
//! content in [`super::view`].
//!
//! The resource is the terminal EMULATOR, not one shell. It holds one
//! [`ShellSession`] per [`ShellKind`] - each with its own prompt, scrollback,
//! history, command set and reveal state - and delegates every accessor to the
//! active one. Switching shells is therefore a field flip: no scrollback is
//! rebuilt, no reveal replays, and each shell comes back exactly as it was
//! left.

use std::collections::HashMap;

use bevy::prelude::*;

use super::view::{nova_os_boot_banner_rows, nova_os_welcome_rows};
use crate::{
    command::prelude::core_command_specs,
    commands::prelude::{command_shell_specs, CommandClass},
    shell::prelude::TerminalCommandSpec,
};

/// The most scrollback rows the terminal keeps. The UI spawns one `Text` entity
/// per row on every rebuild, so an unbounded scrollback is an unbounded entity
/// count for a session that never ends; the oldest rows are dropped past this.
pub const MAX_SCROLLBACK_ROWS: usize = 500;

/// The most command lines one shell's history keeps. Only a session reset ever
/// clears it, so an unbounded history is both an unbounded allocation and an
/// unusable Up-arrow: 200 repeats of `log` means 200 presses to reach anything
/// else.
pub(super) const MAX_HISTORY: usize = 200;

/// Which shell language the emulator is speaking.
///
/// The CRT is the terminal emulator and owns the casing, the glass and every
/// editing key; a shell is the LANGUAGE typed into it. Two exist: the
/// ship-computer's [`ShellKind::NovaOs`] and the game-level
/// [`ShellKind::Commands`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShellKind {
    /// The ship computer: the existing NOVA OS commands and apps. Requires a
    /// player ship and opens with the monitor key.
    #[default]
    NovaOs,
    /// The game-level command language: inspection, settings and armed cheats.
    /// Available over every surface and needs no ship.
    Commands,
}

impl ShellKind {
    /// Both shells, in switch order.
    pub const ALL: [ShellKind; 2] = [ShellKind::NovaOs, ShellKind::Commands];

    /// The prompt prefix this shell echoes a submitted line with.
    pub fn prompt_prefix(self) -> &'static str {
        match self {
            ShellKind::NovaOs => "nova> ",
            ShellKind::Commands => "cmd> ",
        }
    }

    /// The uppercase word the CRT header breadcrumb names this shell with.
    pub fn header_label(self) -> &'static str {
        match self {
            ShellKind::NovaOs => "SHELL",
            ShellKind::Commands => "COMMANDS",
        }
    }
}

/// One shell's whole editable state: the typed line and caret, its scrollback
/// and history, the command set it parses against, its parse/completion state
/// and its staged reveal queue.
///
/// Held twice by [`NovaOsTerminal`], once per [`ShellKind`], which is what lets
/// a shell switch preserve both transcripts.
#[derive(Debug, Clone)]
pub(super) struct ShellSession {
    pub(super) prompt: String,
    pub(super) cursor: usize,
    pub(super) scrollback: Vec<TerminalRow>,
    pub(super) history: Vec<String>,
    pub(super) history_cursor: Option<usize>,
    pub(super) completion_hint: Option<String>,
    pub(super) parse_status: TerminalParseStatus,
    /// Every command this shell parses, completes and documents against.
    pub(super) commands: Vec<TerminalCommandSpec>,
    /// Rows queued for the staggered reveal, drained one-by-one on real time.
    /// Empty except during a reveal.
    pub(super) pending_rows: Vec<TerminalRow>,
    /// Whether this shell's staged reveal has already played. Set on its first
    /// entry; cleared by a session reset so a fresh one re-reveals.
    pub(super) revealed: bool,
    /// The Tab-completion cycle stem: the text being completed. `None` when no
    /// cycle is active; reset on any prompt edit (PoC `resetCycle`).
    pub(super) cycle_stem: Option<String>,
    /// The current index into the match list for the active cycle stem.
    pub(super) cycle_index: usize,
    /// Completion candidates for the argument of an arg-bearing command, keyed
    /// by command name (`"ship repair" -> ["HULL-1", ...]`). The pure terminal
    /// cannot enumerate live ids, so the bevy layer injects them.
    pub(super) arg_completions: HashMap<&'static str, Vec<String>>,
}

impl ShellSession {
    /// A session parsing `commands`, opened on `welcome`.
    fn new(commands: Vec<TerminalCommandSpec>, welcome: Vec<TerminalRow>) -> Self {
        Self {
            prompt: String::new(),
            cursor: 0,
            scrollback: welcome,
            history: Vec::new(),
            history_cursor: None,
            completion_hint: Some("type help".to_string()),
            parse_status: TerminalParseStatus::Empty,
            commands,
            pending_rows: Vec::new(),
            revealed: false,
            cycle_stem: None,
            cycle_index: 0,
            arg_completions: HashMap::new(),
        }
    }
}

/// The CRT's terminal emulator: one [`ShellSession`] per [`ShellKind`], the
/// active shell, and the emulator-wide state (the NOVA OS app mode, the close
/// request, and the invocations waiting for the layers that can reach the
/// world).
///
/// `nova_os_ui` inserts this as a bevy `Resource`, drives it from the keyboard
/// systems (via the `pub` edit/submit/completion methods), and reads it back
/// through the accessors to render the terminal UI. Every accessor speaks for
/// the ACTIVE shell unless its name says otherwise.
#[derive(Resource, Debug, Clone)]
pub struct NovaOsTerminal {
    /// Which shell owns the prompt right now.
    active: ShellKind,
    nova_os: ShellSession,
    command: ShellSession,
    /// Bumped by every scrollback mutation AND by a shell switch. The UI
    /// rebuilds one `Text` entity per row, so it needs to know when the ROWS
    /// changed rather than when anything on the resource did - a caret move
    /// marks the whole resource changed and must not reach the row loop.
    ///
    /// Emulator-wide rather than per-shell on purpose: two sessions with equal
    /// per-shell counters would switch between each other invisibly.
    scrollback_revision: u64,
    pub(super) active_mode: TerminalMode,
    /// Set by the `exit`/`close` commands; the keyboard system consumes it to
    /// drive the animated close of the computer (mirrors the HTML PoC's `exit`).
    pub(super) pending_close: bool,
    /// How many [`NovaOsFlightLog`] entries had been seen the last time the NOVA
    /// OS closed. The boot banner's "N unread events" line counts entries
    /// appended since.
    ///
    /// [`NovaOsFlightLog`]: https://docs.rs/  "internal to nova_os_ui"
    pub(super) seen_events: usize,
    /// An arg-bearing NOVA OS gameplay command that [`Self::submit`] resolved
    /// and is waiting for the gameplay layer to apply.
    pub(super) pending_invocation: Option<NovaOsCommandInvocation>,
    /// A Command-shell command that [`Self::submit`] resolved and is waiting for
    /// the command dispatcher to run against the live game.
    pub(super) pending_command: Option<CommandInvocation>,
    /// A shell switch a submitted command asked for, applied once the line it
    /// was typed on has been cleared.
    pub(super) pending_shell: Option<ShellKind>,
    /// The shell Escape backs out to, set when a switch was a step INTO a
    /// shell rather than the way the CRT opened.
    ///
    /// Escape is a back gesture: `commands` typed at the NOVA OS prompt is one
    /// level down, so Escape climbs back to NOVA OS. A CRT opened straight into
    /// Commands with `:` has nothing underneath, so Escape closes it.
    back_out: Option<ShellKind>,
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

/// A resolved Command-shell command: what to run, what class it is, and the
/// argument words. Both front ends (the CRT and the process channel) produce
/// exactly this, and the one dispatcher runs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocation {
    /// The resolved command name (`"ammo infinite"`, `"graphics"`).
    pub name: &'static str,
    /// What the command is allowed to touch.
    pub class: CommandClass,
    /// The argument words past the command name.
    pub args: Vec<String>,
}

/// One rendered line of terminal scrollback: its semantic kind (which drives the
/// phosphor colour in the UI) and its text. `nova_os_ui`'s game-data bridges
/// build these directly, so the fields are public.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalRow {
    /// The semantic kind of the row, mapped to a phosphor colour by the UI.
    pub kind: TerminalRowKind,
    /// The row's text.
    pub text: String,
}

impl TerminalRow {
    /// A row of `kind` carrying `text`. Shorthand for the struct literal, which
    /// the row builders write hundreds of times.
    pub fn new(kind: TerminalRowKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }

    /// An ordinary output row.
    pub fn output(text: impl Into<String>) -> Self {
        Self::new(TerminalRowKind::Output, text)
    }

    /// A de-emphasised row.
    pub fn dim(text: impl Into<String>) -> Self {
        Self::new(TerminalRowKind::Dim, text)
    }

    /// An informational row (banners, section headers).
    pub fn info(text: impl Into<String>) -> Self {
        Self::new(TerminalRowKind::Info, text)
    }

    /// A warning row.
    pub fn warn(text: impl Into<String>) -> Self {
        Self::new(TerminalRowKind::Warn, text)
    }

    /// An error row.
    pub fn error(text: impl Into<String>) -> Self {
        Self::new(TerminalRowKind::Error, text)
    }
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
/// the bevy world itself. `nova_os_ui` builds this each submit from the flight
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
            active: ShellKind::NovaOs,
            nova_os: ShellSession::new(core_command_specs(), nova_os_welcome_rows()),
            // The Command shell's introduction is world-dependent (the live
            // scenario, the registry count, the cheat mark), so it is revealed
            // by the dispatcher on first entry rather than seeded here.
            command: ShellSession::new(command_shell_specs(), Vec::new()),
            scrollback_revision: 0,
            active_mode: TerminalMode::Prompt,
            pending_close: false,
            pending_shell: None,
            back_out: None,
            seen_events: 0,
            pending_invocation: None,
            pending_command: None,
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
    /// The active shell's session.
    pub(super) fn session(&self) -> &ShellSession {
        match self.active {
            ShellKind::NovaOs => &self.nova_os,
            ShellKind::Commands => &self.command,
        }
    }

    /// The active shell's session, mutably.
    pub(super) fn session_mut(&mut self) -> &mut ShellSession {
        match self.active {
            ShellKind::NovaOs => &mut self.nova_os,
            ShellKind::Commands => &mut self.command,
        }
    }

    /// Which shell language owns the prompt.
    pub fn active_shell(&self) -> ShellKind {
        self.active
    }

    /// The prompt prefix the active shell echoes with (`nova> ` / `cmd> `).
    pub fn prompt_prefix(&self) -> &'static str {
        self.active.prompt_prefix()
    }

    /// Switch to `shell`, keeping both transcripts, both histories and the
    /// emulator's open/pause state. A no-op when it is already active; returns
    /// whether the shell actually changed.
    ///
    /// Leaving NOVA OS while an app owns the screen returns to the NOVA OS
    /// PROMPT first, so coming back lands on a prompt rather than inside a tool
    /// whose keys the other shell was typing into.
    pub fn switch_shell(&mut self, shell: ShellKind) -> bool {
        if self.active == shell {
            return false;
        }
        self.exit_app();
        self.back_out = Some(self.active);
        self.active = shell;
        // The rows on screen are a different session's: the row loop keys on
        // this counter, so a switch has to move it.
        self.bump_scrollback_revision();
        self.session_mut().cycle_stem = None;
        self.refresh_parse();
        true
    }

    /// Enter a shell as the CRT's ground floor: Escape closes the computer
    /// rather than climbing back to whatever was active last time.
    pub fn open_shell(&mut self, shell: ShellKind) {
        let switched = self.switch_shell(shell);
        if !switched {
            self.exit_app();
        }
        self.back_out = None;
    }

    /// The shell Escape backs out to, if this shell was entered from another.
    pub fn back_out_shell(&self) -> Option<ShellKind> {
        self.back_out
    }

    /// Climb one level back to the shell this one was entered from. `false`
    /// when there is nothing underneath, which is the caller's cue to close the
    /// computer instead.
    pub fn back_out(&mut self) -> bool {
        let Some(shell) = self.back_out.take() else {
            return false;
        };
        self.switch_shell(shell);
        self.back_out = None;
        true
    }

    /// The rendered scrollback rows of the active shell, oldest first.
    pub fn scrollback(&self) -> &[TerminalRow] {
        &self.session().scrollback
    }

    /// A counter that changes exactly when the rendered rows change - a
    /// scrollback mutation or a shell switch. The UI keys its row rebuild on
    /// this so prompt edits and caret moves - which mark the whole resource
    /// changed - do not respawn every row.
    pub fn scrollback_revision(&self) -> u64 {
        self.scrollback_revision
    }

    /// Append rows to the active shell's scrollback (e.g. an objective
    /// completion announced while the prompt is open).
    pub fn extend_scrollback(&mut self, rows: impl IntoIterator<Item = TerminalRow>) {
        self.session_mut().scrollback.extend(rows);
        self.after_scrollback_change();
    }

    /// Append rows to a NAMED shell's scrollback, whichever is active.
    ///
    /// The gameplay bridges announce into the NOVA OS transcript, and an
    /// objective must not flip a player's `cmd>` transcript just because they
    /// had the other shell open.
    pub fn extend_shell_scrollback(
        &mut self,
        shell: ShellKind,
        rows: impl IntoIterator<Item = TerminalRow>,
    ) {
        let session = match shell {
            ShellKind::NovaOs => &mut self.nova_os,
            ShellKind::Commands => &mut self.command,
        };
        session.scrollback.extend(rows);
        let excess = session.scrollback.len().saturating_sub(MAX_SCROLLBACK_ROWS);
        if excess > 0 {
            session.scrollback.drain(..excess);
        }
        self.bump_scrollback_revision();
    }

    /// Append one row to the active shell's scrollback.
    pub(super) fn push_row(&mut self, row: TerminalRow) {
        self.session_mut().scrollback.push(row);
        self.after_scrollback_change();
    }

    /// Replace the active shell's whole scrollback (the `clear` command and a
    /// session reset).
    pub fn replace_scrollback(&mut self, rows: Vec<TerminalRow>) {
        self.session_mut().scrollback = rows;
        self.after_scrollback_change();
    }

    /// Bump the revision and drop the oldest rows past [`MAX_SCROLLBACK_ROWS`].
    /// Every scrollback mutation ends here, so neither the cap nor the revision
    /// can be bypassed by a new caller.
    fn after_scrollback_change(&mut self) {
        let session = self.session_mut();
        let excess = session.scrollback.len().saturating_sub(MAX_SCROLLBACK_ROWS);
        if excess > 0 {
            session.scrollback.drain(..excess);
        }
        self.bump_scrollback_revision();
    }

    fn bump_scrollback_revision(&mut self) {
        self.scrollback_revision = self.scrollback_revision.wrapping_add(1);
    }

    /// The current prompt text.
    pub fn prompt(&self) -> &str {
        &self.session().prompt
    }

    /// The prompt as the parser sees it. Every reader of the parse result - the
    /// status, the hint and the inline ghost - must strip the prompt the same way
    /// the parse did, or a leading space greens the prompt with no ghost.
    pub fn parsed_prompt(&self) -> &str {
        parsed_prompt(&self.session().prompt)
    }

    /// The caret's byte offset within the prompt.
    pub fn cursor(&self) -> usize {
        self.session().cursor
    }

    /// The prompt's parse status.
    pub fn parse_status(&self) -> TerminalParseStatus {
        self.session().parse_status
    }

    /// The current completion hint, if any.
    pub fn completion_hint(&self) -> Option<&str> {
        self.session().completion_hint.as_deref()
    }

    /// Which surface currently owns the screen (prompt or a launched app).
    pub fn active_mode(&self) -> TerminalMode {
        self.active_mode
    }

    /// The full command set of the active shell (for NOVA OS: the core builtins
    /// plus any registered apps and their subcommands).
    pub fn command_specs(&self) -> &[TerminalCommandSpec] {
        &self.session().commands
    }

    /// Replace the NOVA OS shell's mirrored command set and re-parse. The caller
    /// compares against [`Self::nova_os_command_specs`] first so this only fires
    /// (marking the resource changed) when the set actually changed.
    pub fn set_nova_os_commands(&mut self, commands: Vec<TerminalCommandSpec>) {
        self.nova_os.commands = commands;
        self.refresh_parse();
    }

    /// The NOVA OS shell's command set, whichever shell is active.
    pub fn nova_os_command_specs(&self) -> &[TerminalCommandSpec] {
        &self.nova_os.commands
    }

    /// The argument-completion candidates currently injected by the gameplay
    /// layer into the NOVA OS shell, keyed by command name. The caller compares
    /// against this before calling [`Self::merge_arg_completions`] so it only
    /// marks the resource changed when the live set actually changed.
    pub fn arg_completions(&self) -> &HashMap<&'static str, Vec<String>> {
        &self.nova_os.arg_completions
    }

    /// Merge arg-completion candidates for the given NOVA OS verbs, leaving
    /// other verbs' entries intact, so several gameplay-verb apps (`ship`,
    /// `map`) can each own their own verbs without clobbering the shared map.
    /// Only re-parses when a value actually changed.
    pub fn merge_arg_completions(
        &mut self,
        entries: impl IntoIterator<Item = (&'static str, Vec<String>)>,
    ) {
        let mut changed = false;
        for (name, candidates) in entries {
            match self.nova_os.arg_completions.get(name) {
                Some(existing) if *existing == candidates => {}
                _ => {
                    self.nova_os.arg_completions.insert(name, candidates);
                    changed = true;
                }
            }
        }
        if changed && self.active == ShellKind::NovaOs {
            self.refresh_parse();
        }
    }

    /// Merge argument-completion candidates into the COMMAND shell (live ship
    /// and section ids, action names, scenario ids). Same contract as
    /// [`Self::merge_arg_completions`].
    pub fn merge_command_arg_completions(
        &mut self,
        entries: impl IntoIterator<Item = (&'static str, Vec<String>)>,
    ) {
        let mut changed = false;
        for (name, candidates) in entries {
            match self.command.arg_completions.get(name) {
                Some(existing) if *existing == candidates => {}
                _ => {
                    self.command.arg_completions.insert(name, candidates);
                    changed = true;
                }
            }
        }
        if changed && self.active == ShellKind::Commands {
            self.refresh_parse();
        }
    }

    /// The Command shell's injected argument candidates, for the change compare.
    pub fn command_arg_completions(&self) -> &HashMap<&'static str, Vec<String>> {
        &self.command.arg_completions
    }

    /// Take the arg-bearing gameplay invocation queued by the last
    /// [`Self::submit`], if any. `nova_os_ui` calls this right after submit,
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

    /// Take the Command-shell invocation queued by the last [`Self::submit`].
    /// The command dispatcher drains this, runs it against the live game and
    /// appends the result rows.
    pub fn take_pending_command(&mut self) -> Option<CommandInvocation> {
        self.pending_command.take()
    }

    /// Whether the `exit`/`close` command has requested an animated close,
    /// clearing the request as it is read.
    pub fn take_pending_close(&mut self) -> bool {
        let pending = self.pending_close;
        self.pending_close = false;
        pending
    }

    /// Request the animated close of the computer, as `exit`/`close` do.
    pub fn request_close(&mut self) {
        self.pending_close = true;
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

    /// Whether the NOVA OS staggered boot banner has already played this
    /// session.
    pub fn is_booted(&self) -> bool {
        self.nova_os.revealed
    }

    /// Whether `shell`'s staged reveal has already played.
    pub fn is_revealed(&self, shell: ShellKind) -> bool {
        match shell {
            ShellKind::NovaOs => self.nova_os.revealed,
            ShellKind::Commands => self.command.revealed,
        }
    }

    /// Kick off the staggered boot banner on the NOVA OS shell: mark it
    /// revealed, clear its scrollback and queue `rows` for
    /// [`Self::reveal_next_boot_row`] to reveal one-by-one.
    pub fn begin_boot(&mut self, rows: Vec<TerminalRow>) {
        self.begin_reveal(ShellKind::NovaOs, rows);
    }

    /// Kick off `shell`'s staged reveal: mark it revealed, clear its scrollback
    /// and queue `rows`. The Command shell's introduction is revealed with this
    /// on its first entry, with the same timing and skip-on-input behaviour as
    /// the NOVA OS boot banner.
    pub fn begin_reveal(&mut self, shell: ShellKind, rows: Vec<TerminalRow>) {
        let session = match shell {
            ShellKind::NovaOs => &mut self.nova_os,
            ShellKind::Commands => &mut self.command,
        };
        session.revealed = true;
        session.scrollback = Vec::new();
        session.pending_rows = rows;
        self.bump_scrollback_revision();
    }

    /// Whether any reveal rows are still queued on the active shell.
    pub fn has_pending_boot_rows(&self) -> bool {
        !self.session().pending_rows.is_empty()
    }

    /// Reveal the next queued row into the active shell's scrollback. Returns
    /// whether a row was revealed (`false` when the queue is empty).
    pub fn reveal_next_boot_row(&mut self) -> bool {
        if self.session().pending_rows.is_empty() {
            return false;
        }
        let row = self.session_mut().pending_rows.remove(0);
        self.push_row(row);
        true
    }

    /// Reveal every queued row at once.
    ///
    /// The stagger is an ANIMATION, not a gate. A player who already knows the
    /// command they want should not have to wait out a reveal to type it, so
    /// the first deliberate key finishes the banner and then does its own job.
    /// Returns whether anything was still queued.
    pub fn finish_boot(&mut self) -> bool {
        if self.session().pending_rows.is_empty() {
            return false;
        }
        for row in std::mem::take(&mut self.session_mut().pending_rows) {
            self.push_row(row);
        }
        true
    }

    /// Hand the screen to the app with launch word `id`, the same transition
    /// [`Self::submit`] performs for an app launch word. Pairs with
    /// [`Self::exit_app`].
    pub fn enter_app(&mut self, id: &'static str) {
        self.active_mode = TerminalMode::App { id };
    }

    /// Reprint the NOVA OS boot banner instantly (PoC `clear` ->
    /// `printBanner(true)`), including the current unread-events line from
    /// `snapshot`.
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

    /// Reset the NOVA OS shell to a fresh session (a new ship): clear its
    /// prompt, scrollback and history and re-arm its staggered boot banner.
    ///
    /// The Command shell is deliberately untouched: it is a GAME-level shell, so
    /// a ship swap is not a reason to lose its transcript or history.
    pub fn reset_session(&mut self) {
        self.nova_os = ShellSession::new(
            std::mem::take(&mut self.nova_os.commands),
            nova_os_welcome_rows(),
        );
        self.active_mode = TerminalMode::Prompt;
        self.seen_events = 0;
        self.pending_invocation = None;
        self.bump_scrollback_revision();
        self.refresh_parse();
    }

    /// Re-arm the Command shell's introduction so the next entry reveals it
    /// against the new world. Called when a fresh scenario is loaded; the
    /// transcript above it is kept, exactly like a real shell's.
    pub fn rearm_command_intro(&mut self) {
        self.command.revealed = false;
    }
}
