//! Shell command language: the command spec the matcher reads, argument arity,
//! and the matcher/resolver plus typo suggestions that turn a typed line into a
//! resolved command or an error.
//!
//! Every command is a [`TerminalCommandSpec`], flattened from the
//! [`crate::commands::COMMAND_CATALOG`]; this module only parses a typed line
//! against a flat slice of specs.

/// How many whitespace-separated argument words a command accepts AFTER its
/// (possibly multi-word) name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandArity {
    /// Takes no arguments.
    None,
    /// Accepts `1..=max` argument words (e.g. `help [command]` is `UpTo(3)`).
    UpTo(usize),
    /// Accepts `min..=max` argument words, for a command whose arguments are
    /// not all optional (`ammo infinite <ship-id> <on|off>` is `Between(2, 2)`).
    Between(usize, usize),
}

impl CommandArity {
    /// Whether `count` argument words is acceptable for this arity.
    pub(crate) fn accepts(self, count: usize) -> bool {
        match self {
            CommandArity::None => count == 0,
            CommandArity::UpTo(max) => count <= max,
            CommandArity::Between(min, max) => count >= min && count <= max,
        }
    }

    /// Whether `count` argument words is MORE than this arity accepts, as
    /// opposed to fewer. The two read differently: an over-run past a command
    /// that owns subcommands named a subcommand that does not exist, while an
    /// under-run simply stopped early.
    pub(crate) fn overruns(self, count: usize) -> bool {
        count > self.most()
    }

    /// The most argument words this command takes. The word at this index in an
    /// over-long line is the first one the command did not ask for.
    pub(crate) fn most(self) -> usize {
        match self {
            CommandArity::None => 0,
            CommandArity::UpTo(max) | CommandArity::Between(_, max) => max,
        }
    }

    /// The message tail for an over-arity command: `takes no arguments` for
    /// `None`, `takes at most N argument(s)` otherwise.
    pub(crate) fn rejection(self) -> String {
        match self {
            CommandArity::None => "takes no arguments".to_string(),
            CommandArity::UpTo(max) => {
                let word = if max == 1 { "argument" } else { "arguments" };
                format!("takes at most {max} {word}")
            }
            CommandArity::Between(min, max) if min == max => {
                let word = if min == 1 { "argument" } else { "arguments" };
                format!("takes {min} {word}")
            }
            CommandArity::Between(min, max) => format!("takes {min} to {max} arguments"),
        }
    }
}

/// What one argument POSITION accepts. Tab completion reads this to answer
/// with values of the right kind instead of command names.
///
/// A closed set is answerable from the catalog alone. A live set is named by a
/// token, and the layer that owns the world fills it in through
/// [`CommandTerminal::merge_live_values`]: this crate stays a leaf, so it learns
/// that an argument is a `"ship"` without ever learning what a ship is.
///
/// [`CommandTerminal::merge_live_values`]: crate::terminal::CommandTerminal::merge_live_values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandArg {
    /// A closed set of words (`on`, `off`), complete from the catalog alone.
    Words(&'static [&'static str]),
    /// A set only the live world knows, published under this token.
    Live(&'static str),
    /// A free value - a number, a name nothing can enumerate.
    Free,
}

impl CommandArg {
    /// The words this position accepts from the catalog alone, if it is closed.
    pub fn words(self) -> &'static [&'static str] {
        match self {
            CommandArg::Words(words) => words,
            CommandArg::Live(_) | CommandArg::Free => &[],
        }
    }

    /// The live token this position reads its values from, if it has one.
    pub fn live_token(self) -> Option<&'static str> {
        match self {
            CommandArg::Live(token) => Some(token),
            CommandArg::Words(_) | CommandArg::Free => None,
        }
    }
}

/// A command as the matcher and the pure terminal see it: the (possibly
/// multi-word) name, a one-line summary for `help`/completion, and its argument
/// arity. Produced by flattening the catalog, so a subcommand carries its FULL
/// name (`"ammo refill"`).
#[derive(Debug, Clone, Copy)]
pub struct TerminalCommandSpec {
    /// The full command name, a word sequence (`help`, `ammo`, `ammo refill`).
    pub name: &'static str,
    /// One-line summary for `help` and completion.
    pub summary: &'static str,
    /// How many argument words the command accepts after its name.
    pub arity: CommandArity,
    /// The placeholder shown for this command's argument in a usage line
    /// (`"<label>"`, `"<section>"`), or `None` for a no-argument command or an
    /// arg-bearing command that named no argument (renders a generic `<arg>`).
    pub arg_hint: Option<&'static str>,
    /// What each argument position accepts, in order. Empty for a command that
    /// has not described its arguments; completion then offers nothing past
    /// the name.
    pub args: &'static [CommandArg],
}

/// Whether a word is a help flag (`help`, `-h`, `--help`), so any command can be
/// asked for its own usage with `<command> help`.
pub(crate) fn is_help_flag(word: &str) -> bool {
    matches!(word, "help" | "-h" | "--help")
}

/// Whether a word is a version flag (`version`, `-v`, `--version`), so any command
/// answers `<command> version` - the universal sub-verb, next to `help`.
pub(crate) fn is_version_flag(word: &str) -> bool {
    matches!(word, "version" | "-v" | "--version")
}

/// The registered names that are sub-commands of `name` (its word sequence plus
/// one more word), e.g. `ammo refill` is a sub-command of `ammo`. Drives the
/// did-you-mean on a bad argument and the `subcommands:` line in per-command help.
pub fn subcommands_of(name: &str, commands: &[TerminalCommandSpec]) -> Vec<&'static str> {
    let prefix = format!("{name} ");
    terminal_command_names(commands)
        .filter(|candidate| candidate.starts_with(&prefix))
        .collect()
}

/// The `(summary, arity, arg_hint)` of a registered command name - the fields the
/// per-command usage block renders.
pub(crate) fn command_meta(
    name: &str,
    commands: &[TerminalCommandSpec],
) -> Option<(&'static str, CommandArity, Option<&'static str>)> {
    commands
        .iter()
        .find(|command| command.name == name)
        .map(|command| (command.summary, command.arity, command.arg_hint))
}

/// The outcome of matching a command line against the registered commands.
/// `Run` carries the matched (possibly multi-word) name and its arguments;
/// the two error variants mirror the PoC's `takes no arguments` / `command not
/// found` paths.
pub enum ResolvedCommand {
    /// A resolved, arity-valid command. `args` holds the trailing argument
    /// words past the (possibly multi-word) command name - empty for the no-arg
    /// commands.
    Run {
        /// The matched command name.
        name: &'static str,
        /// The trailing argument words past the name.
        args: Vec<String>,
    },
    /// `<command> help` (or `-h`/`--help`): show that command's own usage.
    Usage {
        /// The command whose usage was asked for.
        name: &'static str,
    },
    /// `<command> version` (or `-v`/`--version`): show the version.
    Version,
    /// The whole input is a word-prefix of one or more registered names but is
    /// not a command itself (`ammo`, `cheats`). A shell answers by listing what
    /// it could have been rather than "command not found".
    Incomplete {
        /// The typed words, as a registered command's parent.
        name: &'static str,
    },
    /// Trailing words a command's arity does not accept.
    UnexpectedArguments {
        /// The matched command name.
        command: String,
        /// The arity it overran.
        arity: CommandArity,
        /// The trailing words past the command name that overran its arity - the
        /// offending input, so the error can name it (`ammo refill: unknown
        /// subcommand 'b'`).
        args: Vec<String>,
    },
    /// Nothing matched.
    Unknown {
        /// The offending first word.
        command: String,
        /// The nearest registered name, if one is close enough.
        suggestion: Option<&'static str>,
    },
}

/// Every command name known at the prompt, in registry order (core builtins
/// first, then their subcommands), for completion and
/// did-you-mean.
pub fn terminal_command_names(
    commands: &[TerminalCommandSpec],
) -> impl Iterator<Item = &'static str> + '_ {
    commands.iter().map(|command| command.name)
}

/// Whether the words of `name` are a leading prefix of `input_words` (so
/// `["ammo", "refill", "x"]` matches the name `"ammo refill"`).
fn command_name_matches(input_words: &[&str], name: &str) -> Option<usize> {
    let name_words: Vec<&str> = name.split_whitespace().collect();
    let is_prefix = input_words.len() >= name_words.len()
        && input_words
            .iter()
            .zip(&name_words)
            .all(|(input, expected)| input == expected);
    is_prefix.then_some(name_words.len())
}

/// Resolve a command line against the registered commands. Matches the LONGEST
/// command name that is a word-prefix of the input (so a multi-word name like
/// `ammo refill section` beats `ammo refill` on longest-match), then validates the trailing
/// words against that command's arity. There is no per-command special case:
/// multi-word names and argument-taking commands both fall out of this.
pub fn resolve_command(command_line: &str, commands: &[TerminalCommandSpec]) -> ResolvedCommand {
    let words: Vec<&str> = command_line.split_whitespace().collect();
    let Some(&first) = words.first() else {
        return ResolvedCommand::Unknown {
            command: String::new(),
            suggestion: None,
        };
    };
    let best = commands
        .iter()
        .filter_map(|spec| {
            command_name_matches(&words, spec.name).map(|name_words| (spec, name_words))
        })
        .max_by_key(|(_, name_words)| *name_words);
    let Some((spec, name_words)) = best else {
        // The words typed so far may be a registered command's PARENT (`ammo`
        // before `ammo refill`). Naming what it could have been beats a
        // not-found on a word the catalog does contain.
        if let Some(parent) = incomplete_parent(&words, commands) {
            return ResolvedCommand::Incomplete { name: parent };
        }
        return ResolvedCommand::Unknown {
            command: first.to_string(),
            suggestion: nearest_command(first, commands),
        };
    };
    let arg_count = words.len() - name_words;
    // `<command> help` / `<command> version` are universal sub-verbs, resolved
    // ahead of the arity check so they work even on a no-arg command.
    if arg_count == 1 && is_help_flag(words[name_words]) {
        return ResolvedCommand::Usage { name: spec.name };
    }
    if arg_count == 1 && is_version_flag(words[name_words]) {
        return ResolvedCommand::Version;
    }
    if !spec.arity.accepts(arg_count) {
        return ResolvedCommand::UnexpectedArguments {
            command: spec.name.to_string(),
            arity: spec.arity,
            args: words[name_words..]
                .iter()
                .map(|word| word.to_string())
                .collect(),
        };
    }
    ResolvedCommand::Run {
        name: spec.name,
        args: words[name_words..]
            .iter()
            .map(|word| word.to_string())
            .collect(),
    }
}

/// The registered name whose leading words are exactly `words`, when `words` is
/// a strict word-prefix of at least one command name and a command itself of
/// none (`ammo` against `ammo refill`).
///
/// The answer is borrowed OUT of the matching registered name, so it stays
/// `'static` and [`subcommands_of`] can list the children under it.
fn incomplete_parent(words: &[&str], commands: &[TerminalCommandSpec]) -> Option<&'static str> {
    commands.iter().find_map(|spec| {
        let name: &'static str = spec.name;
        let mut offsets = name.match_indices(' ').map(|(at, _)| at);
        // The byte index just past the `words.len()`-th word of the name.
        let end = offsets.nth(words.len() - 1)?;
        let parent = &name[..end];
        (parent.split_whitespace().eq(words.iter().copied())).then_some(parent)
    })
}

fn nearest_command(input: &str, commands: &[TerminalCommandSpec]) -> Option<&'static str> {
    terminal_command_names(commands)
        .map(|name| (name, levenshtein(input, name)))
        .filter(|(_, distance)| *distance <= 2)
        .min_by_key(|(_, distance)| *distance)
        .map(|(name, _)| name)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let mut previous: Vec<usize> = (0..=b.chars().count()).collect();
    let mut current = vec![0; previous.len()];
    for (i, ca) in a.chars().enumerate() {
        current[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let substitution = previous[j] + usize::from(ca != cb);
            let insertion = current[j] + 1;
            let deletion = previous[j + 1] + 1;
            current[j + 1] = substitution.min(insertion).min(deletion);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.chars().count()]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The parser accepts an argument-taking command and a multi-word name without
    /// breaking the argument-free commands.
    #[test]
    fn the_parser_supports_arguments_and_multi_word_names() {
        let specs = vec![
            TerminalCommandSpec {
                name: "ship view",
                summary: "",
                arity: CommandArity::None,
                arg_hint: None,
                args: &[],
            },
            TerminalCommandSpec {
                name: "ship",
                summary: "",
                arity: CommandArity::None,
                arg_hint: None,
                args: &[],
            },
            TerminalCommandSpec {
                name: "repair",
                summary: "",
                arity: CommandArity::UpTo(1),
                arg_hint: None,
                args: &[],
            },
            TerminalCommandSpec {
                name: "help",
                summary: "",
                arity: CommandArity::None,
                arg_hint: None,
                args: &[],
            },
        ];

        // A multi-word name resolves as its command, its 2-word name beating the
        // `ship` command on longest-match.
        assert!(matches!(
            resolve_command("ship view", &specs),
            ResolvedCommand::Run {
                name: "ship view",
                ..
            }
        ));
        // The `ship` command still resolves on its own.
        assert!(matches!(
            resolve_command("ship", &specs),
            ResolvedCommand::Run { name: "ship", .. }
        ));
        // An argument-taking command accepts its argument and CARRIES it out to
        // the gameplay layer (the arg words past the command name).
        assert!(matches!(
            resolve_command("repair thruster", &specs),
            ResolvedCommand::Run {
                name: "repair",
                args,
            } if args == ["thruster"]
        ));
        // ...and rejects more than its arity.
        assert!(matches!(
            resolve_command("repair a b", &specs),
            ResolvedCommand::UnexpectedArguments { command, .. } if command == "repair"
        ));
        // Argument-free commands are unaffected.
        assert!(matches!(
            resolve_command("help", &specs),
            ResolvedCommand::Run { name: "help", .. }
        ));
        assert!(matches!(
            resolve_command("help x", &specs),
            ResolvedCommand::UnexpectedArguments { command, .. } if command == "help"
        ));
        // The multi-word name rejects a trailing argument (arity none).
        assert!(matches!(
            resolve_command("ship view x", &specs),
            ResolvedCommand::UnexpectedArguments { command, .. } if command == "ship view"
        ));
    }
}

/// `CommandArity`, `CommandArg` and `TerminalCommandSpec`.
pub mod prelude {
    pub use super::{CommandArg, CommandArity, TerminalCommandSpec};
}
