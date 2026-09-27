//! Prompt editing: typing and caret movement, submit, tab completion, history
//! recall and the parse refresh that keeps the prompt's status live.

use super::{
    state::{parsed_prompt, TerminalParseStatus, TerminalRow, TerminalRowKind, MAX_HISTORY},
    CommandTerminal,
};
use crate::{
    commands::{
        live,
        prelude::{resolve_command_line, CommandError, CommandOutcome, CommandStatus},
    },
    shell::{resolve_command, terminal_command_names},
};

/// The semantic result of a [`CommandTerminal::submit`], so the bevy layer can
/// pick the sound cue without the pure model knowing about audio.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TerminalSubmitOutcome {
    /// An empty prompt line - no command, no cue.
    Empty,
    /// A command ran and produced output/state (help, clear, log, ...).
    Ran,
    /// A command failed (unknown, or arguments where none are allowed).
    Errored,
    /// A command was parsed and handed to the dispatcher. Its
    /// rows and its ok/error cue belong to the dispatcher's result, one system
    /// later in the same frame - the submit itself decided nothing.
    Dispatched,
}

impl CommandTerminal {
    /// Insert typed text at the caret (control characters are filtered out).
    pub fn insert_text(&mut self, text: &str) {
        for ch in text.chars().filter(|ch| !ch.is_control()) {
            self.prompt.insert(self.cursor, ch);
            self.cursor += ch.len_utf8();
        }
        self.after_edit();
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
        self.after_edit();
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
        self.after_edit();
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

    /// Put the caret at the start of the line (Home, Ctrl+A).
    pub fn move_cursor_to_start(&mut self) {
        self.cursor = 0;
    }

    /// Put the caret at the end of the line (End, Ctrl+E).
    pub fn move_cursor_to_end(&mut self) {
        self.cursor = self.prompt.len();
    }

    /// Cut everything before the caret (Ctrl+U).
    ///
    /// Nothing is kept to paste back. A kill ring is a second clipboard for a
    /// prompt that is one line long, and the line it cut is one Up-arrow away
    /// in the history the moment it was submitted.
    pub fn kill_to_start(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.prompt.drain(..self.cursor);
        self.cursor = 0;
        self.after_edit();
    }

    /// Cut everything from the caret to the end of the line (Ctrl+K).
    pub fn kill_to_end(&mut self) {
        if self.cursor >= self.prompt.len() {
            return;
        }
        self.prompt.truncate(self.cursor);
        self.after_edit();
    }

    /// What every prompt mutation ends with: the history recall and the
    /// completion cycle both describe a line the player has now changed.
    fn after_edit(&mut self) {
        self.history_cursor = None;
        self.cycle_stem = None;
        self.refresh_parse();
    }

    /// Run the current prompt line, appending output to the scrollback and
    /// returning what kind of command ran.
    ///
    /// One parser, one dispatcher. Anything answerable from the catalog is
    /// answered here; everything else is queued for the dispatcher, which is
    /// the same seam the process channel pushes into.
    pub fn submit(&mut self) -> TerminalSubmitOutcome {
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
        self.history_cursor = None;
        self.cycle_stem = None;

        let outcome = match resolve_command_line(&command_line, self.command_specs()) {
            CommandOutcome::Answer(result) => {
                let errored = result.status != CommandStatus::Ok;
                self.extend_scrollback(result.rows);
                if errored {
                    TerminalSubmitOutcome::Errored
                } else {
                    TerminalSubmitOutcome::Ran
                }
            }
            // Shell control never reaches the dispatcher: the terminal owns
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

    /// Completion candidates for `stem`: every command name it prefixes, the
    /// universal sub-verbs (`<command> help`, `<command> version`), and - once
    /// the player is past the name - the values the ARGUMENT under the caret
    /// accepts. Drives Tab completion and the inline ghost, so both understand
    /// sub-commands (fish-style) and arguments, not just top-level names.
    fn completion_matches(&self, stem: &str) -> Vec<String> {
        let mut matches: Vec<String> = terminal_command_names(self.command_specs())
            .filter(|name| name.starts_with(stem))
            .map(|name| name.to_string())
            .collect();
        for name in terminal_command_names(self.command_specs()) {
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
        let Some(spec) = terminal_command_names(self.command_specs())
            .filter(|name| stem.starts_with(&format!("{name} ")))
            .max_by_key(|name| name.split_whitespace().count())
            .and_then(|name| self.command_specs().iter().find(|spec| spec.name == name))
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
        let trimmed = parsed_prompt(&self.prompt).to_string();
        if trimmed.is_empty() {
            self.parse_status = TerminalParseStatus::Empty;
            self.completion_hint = Some("type help".to_string());
            return;
        }
        let commands = self.command_specs();
        let resolved = resolve_command(&trimmed, commands);
        let (status, hint) = match CommandError::from_resolved(&resolved, commands) {
            // A full, arity-valid command, or a `<command> help` / `<command> version` request - all valid input.
            None => (TerminalParseStatus::Valid, None),
            // A line that did not resolve - unless it is still a prefix of a
            // LONGER command name (`ammo re` toward `ammo refill`, or a parent
            // word on the way to a real command), in which case it is a valid
            // prefix and the completion is the hint, not an error.
            //
            // The hint is the error's OWN one-line form, so what the caret says
            // while typing and what Enter prints cannot drift apart.
            Some(error) => match self.command_name_starting_with(&trimmed) {
                Some(name) => (TerminalParseStatus::ValidPrefix, Some(name)),
                None => (TerminalParseStatus::Invalid, error.hint),
            },
        };
        self.parse_status = status;
        self.completion_hint = hint;
    }

    /// The first command name that has `stem` as
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

    /// Put the history entry at `cursor` on the prompt.
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
    use crate::{
        commands::{live, prelude::CommandClass},
        terminal::{
            fixtures::type_text,
            prompt_completion_ghost,
            state::{MAX_HISTORY, MAX_SCROLLBACK_ROWS},
        },
    };

    #[test]
    fn terminal_prompt_edits_and_navigates_history() {
        let mut terminal = CommandTerminal::default();
        type_text(&mut terminal, "help");
        terminal.move_cursor_left();
        terminal.backspace();
        type_text(&mut terminal, "ar");
        terminal.delete();
        assert_eq!(terminal.prompt(), "hear");
        assert_eq!(terminal.cursor(), 4);

        terminal.submit();
        type_text(&mut terminal, "clear");
        terminal.submit();
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
        let mut terminal = CommandTerminal::default();
        type_text(&mut terminal, "section block_picket hull");

        terminal.move_cursor_to_start();
        assert_eq!(terminal.cursor(), 0);
        terminal.move_cursor_to_end();
        assert_eq!(terminal.cursor(), "section block_picket hull".len());

        terminal.move_cursor_to_start();
        type_text(&mut terminal, "x");
        terminal.kill_to_start();
        assert_eq!(
            terminal.prompt(),
            "section block_picket hull",
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
        let mut terminal = CommandTerminal::default();
        type_text(&mut terminal, "log");

        terminal.kill_to_end();
        assert_eq!(terminal.prompt(), "log", "the caret is already at the end");
        terminal.move_cursor_to_start();
        terminal.kill_to_start();
        assert_eq!(terminal.prompt(), "log", "and now at the start");
    }
    #[test]
    fn terminal_unknown_command_suggests_nearest_match() {
        let mut terminal = CommandTerminal::default();
        type_text(&mut terminal, "hlep");

        assert_eq!(terminal.parse_status(), TerminalParseStatus::Invalid);
        assert_eq!(terminal.completion_hint(), Some("did you mean help?"));

        terminal.submit();
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
        let mut terminal = CommandTerminal::default();
        type_text(&mut terminal, "xyzzy");
        terminal.submit();
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
        let mut terminal = CommandTerminal::default();
        type_text(&mut terminal, "status garbage");
        assert_eq!(terminal.parse_status(), TerminalParseStatus::Invalid);
        assert_eq!(
            terminal.completion_hint(),
            Some("status: takes no arguments")
        );
        terminal.submit();
        // A shell-style `command: reason` line, followed by the command's usage
        // block (so the player sees how to use it).
        assert!(
            terminal
                .scrollback()
                .iter()
                .any(|row| row.text == "status: takes no arguments"),
            "the rejection names the command and reason",
        );
        assert!(
            terminal
                .scrollback()
                .iter()
                .any(|row| row.text == "Usage: status"),
            "the rejection is followed by the command's usage",
        );

        type_text(&mut terminal, "clear garbage");
        terminal.submit();
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

    /// The prompt hint is the error's own sentence, so a bad sub-command is
    /// named under the caret and not only after Enter.
    #[test]
    fn the_prompt_hint_names_a_bad_subcommand_before_enter() {
        let mut terminal = CommandTerminal::default();
        type_text(&mut terminal, "ammo refill a b c");
        assert_eq!(terminal.parse_status(), TerminalParseStatus::Invalid);
        assert_eq!(
            terminal.completion_hint(),
            Some("ammo refill: unknown subcommand 'b'"),
        );
    }

    /// Tab lists ambiguous matches on the first press and cycles through them on
    /// repeats, resetting the cycle on any edit.
    #[test]
    fn tab_cycles_ambiguous_completions() {
        let mut terminal = CommandTerminal::default();
        // Three catalog names share the `se` stem.
        terminal.insert_text("se");

        // First Tab lists the matches (sorted) and jumps to the first.
        assert!(terminal.complete());
        assert_eq!(
            terminal.scrollback().last().map(|row| row.text.as_str()),
            Some("section   sections   settings"),
            "the first Tab on an ambiguous stem lists the matches",
        );
        assert_eq!(terminal.prompt(), "section");

        // Repeat presses cycle through the rest, then wrap.
        terminal.complete();
        assert_eq!(terminal.prompt(), "sections");
        terminal.complete();
        assert_eq!(terminal.prompt(), "settings");
        terminal.complete();
        assert_eq!(
            terminal.prompt(),
            "section",
            "cycling wraps back to the first match"
        );

        // The match list is printed once, not on every cycle press.
        let listings = terminal
            .scrollback()
            .iter()
            .filter(|row| row.text == "section   sections   settings")
            .count();
        assert_eq!(listings, 1);

        // Any edit resets the cycle (PoC `resetCycle`).
        terminal.insert_text("x");
        assert!(terminal.cycle_stem.is_none(), "editing resets the cycle");
    }

    /// Live argument values come out of a `HashMap`, so without an explicit
    /// order the Tab cycle differs between processes. The matches are sorted and
    /// deduplicated, so the same stem always cycles the same way.
    #[test]
    fn completion_matches_are_sorted_and_deduplicated() {
        let mut terminal = CommandTerminal::default();
        terminal.merge_live_values([(
            live::SHIP,
            vec![
                "block_picket".to_string(),
                "block_gunship".to_string(),
                // A duplicate candidate must not become a duplicate Tab stop.
                "block_gunship".to_string(),
            ],
        )]);

        let matches = terminal.completion_matches("sections ");
        assert_eq!(
            matches,
            vec![
                "sections block_gunship".to_string(),
                "sections block_picket".to_string(),
                // The universal sub-verbs are offered past the command name too.
                "sections help".to_string(),
                "sections version".to_string(),
            ],
        );
    }

    /// Completion follows the CARET, not the command: the second argument
    /// completes from the second argument's set, and a live set scoped to the
    /// ship already typed offers that ship's sections alone.
    #[test]
    fn the_argument_under_the_caret_is_what_completes() {
        let mut terminal = CommandTerminal::default();
        terminal.merge_live_values([
            (
                live::SHIP.to_string(),
                vec!["block_gunship".to_string(), "block_picket".to_string()],
            ),
            (
                format!("{}:block_gunship", live::SECTION),
                vec!["hull_front".to_string(), "pdc_aft_port".to_string()],
            ),
            (
                format!("{}:block_picket", live::SECTION),
                vec!["pdc_aft_port".to_string()],
            ),
        ]);

        // First position: the ships.
        type_text(&mut terminal, "section block_p");
        assert!(terminal.complete());
        assert_eq!(terminal.prompt(), "section block_picket");

        // Second position: only THAT ship's sections, and the settled first
        // argument is kept.
        terminal.reset_prompt();
        type_text(&mut terminal, "section block_gunship hu");
        assert!(terminal.complete());
        assert_eq!(terminal.prompt(), "section block_gunship hull_front");

        // The picket carries no `hull_front`, so nothing completes there.
        terminal.reset_prompt();
        type_text(&mut terminal, "section block_picket hu");
        assert!(!terminal.complete());
    }

    /// A closed argument set lives in the catalog, so it completes with no
    /// world at all.
    #[test]
    fn a_catalog_argument_completes_without_a_world() {
        let mut terminal = CommandTerminal::default();
        type_text(&mut terminal, "graphics me");
        assert!(terminal.complete());
        assert_eq!(terminal.prompt(), "graphics medium");
    }

    /// `help <TAB>` names commands, and a command name is several words, so it
    /// is matched against the whole tail rather than the last word.
    #[test]
    fn help_completes_whole_multi_word_command_names() {
        let mut terminal = CommandTerminal::default();
        type_text(&mut terminal, "help ammo refill s");
        assert!(terminal.complete());
        assert_eq!(terminal.prompt(), "help ammo refill section");
    }

    /// `clear` and `close` control the SCREEN, which only the emulator has, so
    /// they never reach the dispatcher.
    #[test]
    fn shell_control_is_answered_by_the_emulator_not_the_dispatcher() {
        let mut terminal = CommandTerminal::default();
        terminal.extend_scrollback(vec![TerminalRow {
            kind: TerminalRowKind::Output,
            text: "old".to_string(),
        }]);

        type_text(&mut terminal, "clear");
        assert_eq!(terminal.submit(), TerminalSubmitOutcome::Ran,);
        assert!(!terminal.has_pending_command(), "clear is not dispatched");
        assert!(terminal.scrollback().is_empty(), "the transcript is gone");
        assert!(
            !terminal.is_revealed(),
            "the introduction is re-armed against the world as it is now",
        );

        type_text(&mut terminal, "close");
        assert_eq!(terminal.submit(), TerminalSubmitOutcome::Ran,);
        assert!(!terminal.has_pending_command(), "close is not dispatched");
        assert!(terminal.has_pending_close());
    }

    /// Two commands submitted before the dispatcher runs both reach it: the
    /// process channel can stage two Enters on one tick.
    #[test]
    fn two_submits_in_one_frame_both_reach_the_dispatcher() {
        let mut terminal = CommandTerminal::default();
        for line in ["ships", "status"] {
            type_text(&mut terminal, line);
            terminal.submit();
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
    fn history_is_bounded_and_skips_repeats() {
        let mut terminal = CommandTerminal::default();
        for _ in 0..3 {
            type_text(&mut terminal, "help");
            terminal.submit();
        }
        assert_eq!(
            terminal.history,
            vec!["help".to_string()],
            "a repeat of the last entry is not recorded again",
        );

        // Alternating lines are all distinct entries, so only the cap can trim.
        for index in 0..MAX_HISTORY + 20 {
            type_text(&mut terminal, &format!("help {index}"));
            terminal.submit();
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
    fn scrollback_is_bounded_and_revisioned() {
        let mut terminal = CommandTerminal::default();
        let before = terminal.scrollback_revision();
        terminal.insert_text("help");
        terminal.move_cursor_left();
        assert_eq!(
            terminal.scrollback_revision(),
            before,
            "prompt edits do not touch the scrollback",
        );

        terminal.submit();
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

    /// The Command shell answers catalog questions itself and queues everything
    /// that needs the live game, with the class the catalog documented.
    #[test]
    fn help_is_answered_and_world_commands_are_queued() {
        let mut terminal = CommandTerminal::default();

        type_text(&mut terminal, "help");
        assert_eq!(terminal.submit(), TerminalSubmitOutcome::Ran,);
        assert!(
            terminal.take_pending_command().is_none(),
            "help is answered from the catalog; the dispatcher never sees it",
        );
        assert!(terminal
            .scrollback()
            .iter()
            .any(|row| row.text.contains("commands in")));

        type_text(&mut terminal, "ammo infinite player_ship on");
        assert_eq!(terminal.submit(), TerminalSubmitOutcome::Dispatched,);
        let queued = terminal
            .take_pending_command()
            .expect("queued for dispatch");
        assert_eq!(queued.name, "ammo infinite");
        assert_eq!(queued.class, CommandClass::Cheat);
        assert_eq!(queued.args, ["player_ship", "on"]);

        // A typo is an error in the shell, and nothing reaches the dispatcher.
        type_text(&mut terminal, "ammoo");
        assert_eq!(terminal.submit(), TerminalSubmitOutcome::Errored,);
        assert!(terminal.take_pending_command().is_none());
    }

    /// The Command shell completes against its own catalog, so a half-typed
    /// cheat ghosts toward the real command and Tab finishes it.
    #[test]
    fn completion_runs_against_the_catalog() {
        let mut terminal = CommandTerminal::default();
        type_text(&mut terminal, "cheats en");
        assert_eq!(terminal.parse_status(), TerminalParseStatus::ValidPrefix);
        assert_eq!(prompt_completion_ghost(&terminal), "able");
        assert!(terminal.complete());
        assert_eq!(terminal.prompt(), "cheats enable");

        // `log` is a catalog word now that the flight log prints here.
        terminal.reset_prompt();
        type_text(&mut terminal, "log");
        assert_eq!(terminal.parse_status(), TerminalParseStatus::Valid);
    }
}
