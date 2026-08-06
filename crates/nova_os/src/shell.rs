//! Shell command language: the unified command spec, argument arity, and the
//! matcher/resolver plus typo suggestions that turn a typed line into a resolved
//! command (an app launch or a CLI action) or an error.
//!
//! There is one command concept here - every command is a [`TerminalCommandSpec`],
//! whether it launches an app (`map`) or prints to the terminal (`log`,
//! `map view`). The rich authoring form and the registry that produces these
//! specs live in [`crate::command`]; this module only parses a typed line against
//! a flat slice of specs.

/// How many whitespace-separated argument words a command accepts AFTER its
/// (possibly multi-word) name. All current commands are `None`; the parser
/// carries the richer arity so an argument-taking command (e.g. `repair <part>`)
/// can be added without touching the matcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandArity {
    /// Takes no arguments.
    None,
    /// Accepts `1..=max` argument words (e.g. `ship repair <id>` is `UpTo(1)`).
    UpTo(usize),
}

impl CommandArity {
    /// Whether `count` argument words is acceptable for this arity.
    pub(crate) fn accepts(self, count: usize) -> bool {
        match self {
            CommandArity::None => count == 0,
            CommandArity::UpTo(max) => count <= max,
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
        }
    }
}

/// The in-terminal CLI action a [`CommandDispatch::Cli`] command performs when it
/// runs. Every variant is produced by the pure terminal itself; `Snapshot` reads
/// its rows from the [`crate::terminal::TerminalCommandSnapshot`] by the command's
/// name key, the rest are built in-terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliOutput {
    /// Print the `help` command list.
    Help,
    /// Print the NOVA OS version banner.
    Version,
    /// Clear the scrollback back to the welcome banner.
    Clear,
    /// Request the animated close of the computer.
    Exit,
    /// Extend the scrollback with the rows `nova_gameplay` placed in the snapshot
    /// under this command's name (e.g. `log`, `ship view`, `map view`).
    Snapshot,
}

/// What running a command DOES: launch its app, or perform a CLI action. This is
/// the generic dispatch tag `submit` matches on - there is no per-command name
/// special case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandDispatch {
    /// Hand the screen to the app whose id is this command's name (`map`).
    App,
    /// Print to the terminal (the CLI builtins and app subcommands like `map view`).
    Cli(CliOutput),
    /// Hand the parsed invocation (name + argument words) to the gameplay layer,
    /// which applies it against the live world and appends the result rows. The
    /// pure terminal only records the invocation (it cannot reach the ECS
    /// itself) - this is the seam the arg-bearing ship verbs (`ship
    /// section/reload/repair <id>`) dispatch through, and the same seam a
    /// future queued/over-time,
    /// resource-costed action model plugs into. See
    /// [`crate::terminal::NovaOsTerminal::take_pending_invocation`].
    Gameplay,
}

/// A command as the matcher and the pure terminal see it: the (possibly
/// multi-word) name, a one-line summary for `help`/completion, its argument arity
/// and how it dispatches. Produced by flattening the [`crate::command`] registry
/// tree, so a subcommand carries its FULL name (`"map view"`).
#[derive(Debug, Clone, Copy)]
pub struct TerminalCommandSpec {
    /// The full command name, a word sequence (`help`, `map`, `map view`).
    pub name: &'static str,
    /// One-line summary for `help` and completion.
    pub summary: &'static str,
    /// How many argument words the command accepts after its name.
    pub arity: CommandArity,
    /// The placeholder shown for this command's argument in a usage line
    /// (`"<label>"`, `"<section>"`), or `None` for a no-argument command or an
    /// arg-bearing command that named no argument (renders a generic `<arg>`).
    pub arg_hint: Option<&'static str>,
    /// Whether it launches an app or performs a CLI action.
    pub dispatch: CommandDispatch,
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
/// one more word), e.g. `map view` is a sub-command of `map`. Drives the
/// did-you-mean on a bad argument and the `subcommands:` line in per-command help.
pub(crate) fn subcommands_of(name: &str, commands: &[TerminalCommandSpec]) -> Vec<&'static str> {
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
/// `Run` carries the matched (possibly multi-word) name and how to dispatch it;
/// the two error variants mirror the PoC's `takes no arguments` / `command not
/// found` paths.
pub(crate) enum ResolvedCommand {
    /// A resolved, arity-valid command: dispatch it by `dispatch`. `args` holds
    /// the trailing argument words past the (possibly multi-word) command name -
    /// empty for the no-arg commands, the `<id>` for an arg-bearing gameplay verb.
    Run {
        name: &'static str,
        dispatch: CommandDispatch,
        args: Vec<String>,
    },
    /// `<command> help` (or `-h`/`--help`): show that command's own usage.
    Usage { name: &'static str },
    /// `<command> version` (or `-v`/`--version`): show the NOVA OS version.
    Version,
    UnexpectedArguments {
        command: String,
        arity: CommandArity,
        /// The trailing words past the command name that overran its arity - the
        /// offending input, so the error can name it (`map: unknown subcommand
        /// 'v'`).
        args: Vec<String>,
    },
    Unknown {
        command: String,
        suggestion: Option<&'static str>,
    },
}

/// Every command name known at the prompt, in registry order (core builtins
/// first, then registered apps and their subcommands), for completion and
/// did-you-mean.
pub(crate) fn terminal_command_names(
    commands: &[TerminalCommandSpec],
) -> impl Iterator<Item = &'static str> + '_ {
    commands.iter().map(|command| command.name)
}

/// Whether the words of `name` are a leading prefix of `input_words` (so
/// `["ship", "view", "x"]` matches the name `"ship view"`).
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
/// `map view` beats the `map` app on longest-match), then validates the trailing
/// words against that command's arity. There is no per-command special case:
/// multi-word names and argument-taking commands both fall out of this.
pub(crate) fn resolve_command(
    command_line: &str,
    commands: &[TerminalCommandSpec],
) -> ResolvedCommand {
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
        dispatch: spec.dispatch,
        args: words[name_words..]
            .iter()
            .map(|word| word.to_string())
            .collect(),
    }
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
    use crate::command::core_command_specs;

    /// A `map` app spec plus its `map view` CLI subcommand, mirroring what the
    /// map plugin registers - used to prove app-vs-subcommand resolution without
    /// pulling in `nova_gameplay`.
    fn core_with_map() -> Vec<TerminalCommandSpec> {
        let mut specs = core_command_specs();
        specs.push(TerminalCommandSpec {
            name: "map",
            summary: "Open the local-space map",
            arity: CommandArity::None,
            arg_hint: None,
            dispatch: CommandDispatch::App,
        });
        specs.push(TerminalCommandSpec {
            name: "map view",
            summary: "Print local-space contacts",
            arity: CommandArity::None,
            arg_hint: None,
            dispatch: CommandDispatch::Cli(CliOutput::Snapshot),
        });
        specs
    }

    #[test]
    fn nova_os_core_commands_are_the_html_builtins() {
        // The core (app-less) command set + order mirrors the HTML PoC minus the
        // app subcommands, which now live with their apps. `map view` is NOT core
        // anymore: it belongs to the `map` command tree.
        let registered: Vec<&str> = core_command_specs()
            .iter()
            .map(|command| command.name)
            .collect();
        assert_eq!(
            registered,
            vec!["help", "log", "objectives", "clear", "version", "exit"]
        );

        for name in ["help", "clear", "log", "objectives", "version", "exit"] {
            assert!(
                matches!(
                    resolve_command(name, &core_command_specs()),
                    ResolvedCommand::Run { name: matched, .. } if matched == name
                ),
                "{name} resolves to its core command",
            );
        }
        // `ship`/`ship view`, `map`/`map view` are unknown until their own app
        // command trees are registered by the owning gameplay plugin.
        for planned in ["map", "map view", "ship", "ship view"] {
            assert!(
                matches!(
                    resolve_command(planned, &core_command_specs()),
                    ResolvedCommand::Unknown { .. }
                ),
                "{planned} stays deferred to its own registration"
            );
        }
    }

    #[test]
    fn nova_os_map_launches_app_and_map_view_prints() {
        let specs = core_with_map();
        // Bare `map` launches the app; `map view` resolves to the CLI subcommand
        // via longest-match.
        assert!(matches!(
            resolve_command("map", &specs),
            ResolvedCommand::Run {
                name: "map",
                dispatch: CommandDispatch::App,
                ..
            }
        ));
        assert!(matches!(
            resolve_command("map view", &specs),
            ResolvedCommand::Run {
                name: "map view",
                dispatch: CommandDispatch::Cli(CliOutput::Snapshot),
                ..
            }
        ));
        // `<command> help` shows a command's own usage, for an app or a CLI
        // command, ahead of the arity check.
        assert!(matches!(
            resolve_command("log help", &specs),
            ResolvedCommand::Usage { name: "log" }
        ));
        assert!(matches!(
            resolve_command("map -h", &specs),
            ResolvedCommand::Usage { name: "map" }
        ));
        // A non-help bad argument to a no-arg command still rejects (submit turns
        // it into a usage hint that names the sub-commands, e.g. `map view`).
        assert!(matches!(
            resolve_command("map v", &specs),
            ResolvedCommand::UnexpectedArguments { command, .. } if command == "map"
        ));
        assert_eq!(subcommands_of("map", &specs), vec!["map view"]);
        // `ship` left the core builtins to become a gameplay-registered app tree
        // (like `map`), so it is Unknown against the bare core set.
        assert!(matches!(
            resolve_command("ship", &core_command_specs()),
            ResolvedCommand::Unknown { command, .. } if command == "ship"
        ));
    }

    #[test]
    fn nova_os_typo_of_an_app_word_is_suggested() {
        // Did-you-mean covers app launch words too, not just the core builtins.
        let specs = core_with_map();
        assert_eq!(
            nearest_command("nap", &specs),
            Some("map"),
            "a typo of a registered app word suggests that app word",
        );
        assert_eq!(
            nearest_command("nap", &core_command_specs()),
            None,
            "without the app registered there is no near command to suggest",
        );
    }

    /// The parser accepts an argument-taking command and a multi-word name without
    /// breaking the argument-free commands.
    #[test]
    fn nova_os_parser_supports_arguments_and_multiword() {
        let specs = vec![
            TerminalCommandSpec {
                name: "ship view",
                summary: "",
                arity: CommandArity::None,
                arg_hint: None,
                dispatch: CommandDispatch::App,
            },
            TerminalCommandSpec {
                name: "ship",
                summary: "",
                arity: CommandArity::None,
                arg_hint: None,
                dispatch: CommandDispatch::Cli(CliOutput::Snapshot),
            },
            TerminalCommandSpec {
                name: "repair",
                summary: "",
                arity: CommandArity::UpTo(1),
                arg_hint: None,
                dispatch: CommandDispatch::Gameplay,
            },
            TerminalCommandSpec {
                name: "help",
                summary: "",
                arity: CommandArity::None,
                arg_hint: None,
                dispatch: CommandDispatch::Cli(CliOutput::Help),
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
                dispatch: CommandDispatch::Gameplay,
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
