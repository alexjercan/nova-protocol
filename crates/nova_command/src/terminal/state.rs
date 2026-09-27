//! The [`CommandTerminal`] resource and its row types, plus the accessors and
//! session lifecycle (the introduction reveal, the close request) the bevy
//! layer drives it through. Prompt editing lives in [`super::edit`], the
//! rendered content in [`super::view`].

use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;

use crate::{
    commands::prelude::{command_shell_specs, CommandClass},
    shell::prelude::TerminalCommandSpec,
};

/// The most scrollback rows the terminal keeps. The UI spawns one `Text` entity
/// per row on every rebuild, so an unbounded scrollback is an unbounded entity
/// count for a session that never ends; the oldest rows are dropped past this.
pub const MAX_SCROLLBACK_ROWS: usize = 500;

/// The most command lines the history keeps. Only a new process clears it, so
/// an unbounded history is both an unbounded allocation and an unusable
/// Up-arrow: 200 repeats of `log` means 200 presses to reach anything else.
pub(super) const MAX_HISTORY: usize = 200;

/// The prefix a submitted line is echoed with.
const PROMPT_PREFIX: &str = "cmd> ";

/// The `NOVA COMMANDS` terminal: the typed line and caret, the scrollback and
/// history, the parse and completion state, the staged introduction, the
/// close request, and the invocations waiting for the layers that can reach
/// the world.
///
/// `nova_interface` inserts this as a bevy `Resource`, drives it from the
/// keyboard systems (via the `pub` edit/submit/completion methods), and reads
/// it back through the accessors to render the CRT. The process channel
/// shares the parser and the dispatcher, not this resource.
#[derive(Resource, Debug, Clone)]
pub struct CommandTerminal {
    pub(super) prompt: String,
    pub(super) cursor: usize,
    scrollback: Vec<TerminalRow>,
    pub(super) history: Vec<String>,
    pub(super) history_cursor: Option<usize>,
    pub(super) completion_hint: Option<String>,
    pub(super) parse_status: TerminalParseStatus,
    /// Rows queued for the staggered reveal, drained one-by-one on real time.
    /// Empty except during a reveal.
    pending_rows: Vec<TerminalRow>,
    /// Whether the introduction has already played. Cleared by
    /// [`Self::rearm_command_intro`] so a new world re-reveals it.
    revealed: bool,
    /// The Tab-completion cycle stem: the text being completed. `None` when no
    /// cycle is active; reset on any prompt edit (PoC `resetCycle`).
    pub(super) cycle_stem: Option<String>,
    /// The current index into the match list for the active cycle stem.
    pub(super) cycle_index: usize,
    /// Bumped by every scrollback mutation. The UI rebuilds one `Text` entity
    /// per row, so it needs to know when the ROWS changed rather than when
    /// anything on the resource did - a caret move marks the whole resource
    /// changed and must not reach the row loop.
    scrollback_revision: u64,
    /// Set by the `close` command; the keyboard system consumes it to drive the
    /// animated close of the modal.
    pub(super) pending_close: bool,
    /// The commands [`Self::submit`] resolved and handed on for the dispatcher
    /// to run against the live game, oldest first.
    ///
    /// A QUEUE, not a slot: the process channel can stage two Enters on one
    /// tick, and a slot dropped the first of them silently.
    pub(super) pending_commands: VecDeque<CommandInvocation>,
    /// Values only the live world knows, keyed by the token a
    /// [`CommandArg::Live`] argument names (`"ship" -> ["player_spaceship"]`).
    ///
    /// A key may be QUALIFIED by the argument before it
    /// (`"section:block_gunship"`),
    /// which is how `section <ship> <TAB>` offers that ship's sections and not
    /// every ship's. Completion tries the qualified key first and falls back to
    /// the bare token.
    ///
    /// [`CommandArg::Live`]: crate::shell::CommandArg::Live
    pub(super) live_values: HashMap<String, Vec<String>>,
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
/// phosphor colour in the UI) and its text. `nova_interface`'s game-data bridges
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
    /// An echoed input line (`cmd> ...`).
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

impl Default for CommandTerminal {
    fn default() -> Self {
        let mut terminal = Self {
            prompt: String::new(),
            cursor: 0,
            // The introduction is world-dependent (the live scenario, the
            // registry count, the cheat mark), so the dispatcher reveals it on
            // first entry rather than seeding it here.
            scrollback: Vec::new(),
            history: Vec::new(),
            history_cursor: None,
            completion_hint: Some("type help".to_string()),
            parse_status: TerminalParseStatus::Empty,
            pending_rows: Vec::new(),
            revealed: false,
            cycle_stem: None,
            cycle_index: 0,
            scrollback_revision: 0,
            pending_close: false,
            pending_commands: VecDeque::new(),
            live_values: HashMap::new(),
        };
        terminal.seed_command_name_values();
        terminal.refresh_parse();
        terminal
    }
}

/// How the parser strips a raw prompt line. The single definition behind
/// [`CommandTerminal::parsed_prompt`]; `refresh_parse` calls it on the field
/// directly because it writes the parse result back in the same statement.
pub(super) fn parsed_prompt(prompt: &str) -> &str {
    prompt.trim()
}

impl CommandTerminal {
    /// The prompt prefix a submitted line is echoed with (`cmd> `).
    pub fn prompt_prefix(&self) -> &'static str {
        PROMPT_PREFIX
    }

    /// The rendered scrollback rows, oldest first.
    pub fn scrollback(&self) -> &[TerminalRow] {
        &self.scrollback
    }

    /// A counter that changes exactly when the rendered rows change. The UI
    /// keys its row rebuild on this so prompt edits and caret moves - which
    /// mark the whole resource changed - do not respawn every row.
    pub fn scrollback_revision(&self) -> u64 {
        self.scrollback_revision
    }

    /// Append rows to the scrollback (a dispatcher result).
    pub fn extend_scrollback(&mut self, rows: impl IntoIterator<Item = TerminalRow>) {
        self.scrollback.extend(rows);
        self.after_scrollback_change();
    }

    /// Append one row to the scrollback.
    pub(super) fn push_row(&mut self, row: TerminalRow) {
        self.scrollback.push(row);
        self.after_scrollback_change();
    }

    /// Replace the whole scrollback (the `clear` command).
    pub fn replace_scrollback(&mut self, rows: Vec<TerminalRow>) {
        self.scrollback = rows;
        self.after_scrollback_change();
    }

    /// Bump the revision and drop the oldest rows past [`MAX_SCROLLBACK_ROWS`].
    /// Every scrollback mutation ends here, so neither the cap nor the revision
    /// can be bypassed by a new caller.
    fn after_scrollback_change(&mut self) {
        let excess = self.scrollback.len().saturating_sub(MAX_SCROLLBACK_ROWS);
        if excess > 0 {
            self.scrollback.drain(..excess);
        }
        self.scrollback_revision = self.scrollback_revision.wrapping_add(1);
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

    /// Every command the terminal parses, completes and documents against: the
    /// command catalog.
    pub fn command_specs(&self) -> &'static [TerminalCommandSpec] {
        command_shell_specs()
    }

    /// The live values currently published for completion, keyed by the token a
    /// [`CommandArg::Live`] argument names. A caller compares against this
    /// before [`Self::merge_live_values`] so it only marks the resource changed
    /// when the live set actually changed.
    ///
    /// [`CommandArg::Live`]: crate::shell::CommandArg::Live
    pub fn live_values(&self) -> &HashMap<String, Vec<String>> {
        &self.live_values
    }

    /// Publish the values for one or more live tokens, leaving every other
    /// token intact. Only re-parses on a real change.
    pub fn merge_live_values(
        &mut self,
        entries: impl IntoIterator<Item = (impl Into<String>, Vec<String>)>,
    ) {
        let mut changed = false;
        for (token, candidates) in entries {
            let token = token.into();
            match self.live_values.get(&token) {
                Some(existing) if *existing == candidates => {}
                _ => {
                    self.live_values.insert(token, candidates);
                    changed = true;
                }
            }
        }
        if changed {
            self.refresh_parse();
        }
    }

    /// Publish the catalog's command names under
    /// [`live::COMMAND`](crate::commands::live::COMMAND), so `help <TAB>` names
    /// the catalog it is asking about. Needs no world, so the terminal fills it
    /// itself.
    fn seed_command_name_values(&mut self) {
        let mut names: Vec<String> = self
            .command_specs()
            .iter()
            .map(|spec| spec.name.to_string())
            .collect();
        names.sort_unstable();
        names.dedup();
        self.live_values
            .insert(crate::commands::live::COMMAND.to_string(), names);
    }

    /// Take the oldest invocation waiting for the dispatcher. The dispatcher
    /// drains this, runs it against the live game and appends the result rows.
    pub fn take_pending_command(&mut self) -> Option<CommandInvocation> {
        self.pending_commands.pop_front()
    }

    /// Whether any invocation is waiting for the dispatcher.
    ///
    /// Read through the immutable `Deref` so an idle frame does not mark the
    /// resource changed: every paint gate keys on that, and taking through
    /// `ResMut` on an empty queue rebuilt the whole terminal UI 60 times a
    /// second.
    pub fn has_pending_command(&self) -> bool {
        !self.pending_commands.is_empty()
    }

    /// Whether the `close` command has requested an animated close, clearing
    /// the request as it is read.
    pub fn take_pending_close(&mut self) -> bool {
        let pending = self.pending_close;
        self.pending_close = false;
        pending
    }

    /// Whether an animated close is requested, without clearing it. Same
    /// change-detection reason as [`Self::has_pending_command`].
    pub fn has_pending_close(&self) -> bool {
        self.pending_close
    }

    /// Request the animated close of the modal, as `close` does.
    pub fn request_close(&mut self) {
        self.pending_close = true;
    }

    /// Whether the introduction has already played.
    pub fn is_revealed(&self) -> bool {
        self.revealed
    }

    /// Kick off the staged introduction: mark it revealed, clear the
    /// scrollback and queue `rows` for [`Self::reveal_next_boot_row`] to reveal
    /// one-by-one.
    pub fn begin_reveal(&mut self, rows: Vec<TerminalRow>) {
        self.revealed = true;
        self.scrollback = Vec::new();
        self.pending_rows = rows;
        self.scrollback_revision = self.scrollback_revision.wrapping_add(1);
    }

    /// Whether any reveal rows are still queued.
    pub fn has_pending_boot_rows(&self) -> bool {
        !self.pending_rows.is_empty()
    }

    /// Reveal the next queued row into the scrollback. Returns whether a row
    /// was revealed (`false` when the queue is empty).
    pub fn reveal_next_boot_row(&mut self) -> bool {
        if self.pending_rows.is_empty() {
            return false;
        }
        let row = self.pending_rows.remove(0);
        self.push_row(row);
        true
    }

    /// Reveal every queued row at once.
    ///
    /// The stagger is an ANIMATION, not a gate. A player who already knows the
    /// command they want should not have to wait out a reveal to type it, so
    /// the first deliberate key finishes the introduction and then does its
    /// own job. Returns whether anything was still queued.
    pub fn finish_boot(&mut self) -> bool {
        if self.pending_rows.is_empty() {
            return false;
        }
        for row in std::mem::take(&mut self.pending_rows) {
            self.push_row(row);
        }
        true
    }

    /// Re-arm the introduction so the next entry reveals it against the new
    /// world. Called when a fresh scenario is loaded; the transcript above it
    /// is kept, exactly like a real shell's.
    pub fn rearm_command_intro(&mut self) {
        self.revealed = false;
    }
}
