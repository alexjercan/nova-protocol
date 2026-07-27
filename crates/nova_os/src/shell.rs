//! Shell command language: the built-in command table, argument arity, and
//! the matcher/resolver plus typo suggestions that turn a typed line into a
//! resolved built-in, app launch, or error.

use crate::app::NovaOsAppCommand;

/// How many whitespace-separated argument words a command accepts AFTER its
/// (possibly multi-word) name. All current built-ins and apps are `None`; the
/// parser carries the richer arity so the `map`/`ship viewer` app tasks can add
/// argument-taking commands (e.g. `repair <part>`) without touching the matcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandArity {
    /// Takes no arguments.
    None,
    /// Accepts `1..=max` argument words. No production command registers this
    /// yet - this task ships the parser capability, the app tasks consume it - so
    /// it is unused outside `#[cfg(test)]` (mirrors `NovaOsAppRegistry::register`).
    #[allow(dead_code)]
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

#[derive(Debug, Clone, Copy)]
pub(crate) struct TerminalCommand {
    /// The command name, a word sequence (all current built-ins are single
    /// words; the matcher supports multi-word names like `ship view`).
    pub(crate) name: &'static str,
    pub(crate) summary: &'static str,
    pub(crate) arity: CommandArity,
}

pub(crate) const TERMINAL_COMMANDS: &[TerminalCommand] = &[
    TerminalCommand {
        name: "help",
        summary: "Show this command list",
        arity: CommandArity::None,
    },
    TerminalCommand {
        name: "log",
        summary: "Print comms and mission events",
        arity: CommandArity::None,
    },
    TerminalCommand {
        name: "objectives",
        summary: "Print active objectives",
        arity: CommandArity::None,
    },
    TerminalCommand {
        name: "ship",
        summary: "Print ship status summary",
        arity: CommandArity::None,
    },
    // `map view` is the CLI counterpart of the `map` app: a two-word built-in
    // that prints the local-space contact list inline. It resolves ahead of the
    // one-word `map` app launch word via the longest-prefix rule, exactly like
    // `ship view` beats `ship`.
    TerminalCommand {
        name: "map view",
        summary: "Print local-space contacts",
        arity: CommandArity::None,
    },
    TerminalCommand {
        name: "clear",
        summary: "Clear terminal scrollback",
        arity: CommandArity::None,
    },
    TerminalCommand {
        name: "version",
        summary: "Print the NOVA OS version",
        arity: CommandArity::None,
    },
    TerminalCommand {
        name: "exit",
        summary: "Suspend the NOVA OS computer",
        arity: CommandArity::None,
    },
];

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
pub(crate) fn subcommands_of<'a>(
    name: &str,
    app_commands: &'a [NovaOsAppCommand],
) -> Vec<&'static str> {
    let prefix = format!("{name} ");
    terminal_command_names(app_commands)
        .filter(|candidate| candidate.starts_with(&prefix))
        .collect()
}

/// The `(summary, arity)` of a registered command name, built-in or app.
pub(crate) fn command_meta(
    name: &str,
    app_commands: &[NovaOsAppCommand],
) -> Option<(&'static str, CommandArity)> {
    TERMINAL_COMMANDS
        .iter()
        .find(|command| command.name == name)
        .map(|command| (command.summary, command.arity))
        .or_else(|| {
            app_commands
                .iter()
                .find(|app| app.id == name)
                .map(|app| (app.summary, app.arity))
        })
}

/// The outcome of matching a command line against the built-ins and registered
/// apps. `App`/`Builtin` carry the matched (possibly multi-word) name; the two
/// error variants mirror the PoC's `takes no arguments` / `command not found`
/// paths.
pub(crate) enum ResolvedCommand {
    App {
        id: &'static str,
    },
    Builtin {
        name: &'static str,
    },
    /// `<command> help` (or `-h`/`--help`): show that command's own usage.
    Usage {
        name: &'static str,
    },
    /// `<command> version` (or `-v`/`--version`): show the NOVA OS version.
    Version {
        name: &'static str,
    },
    UnexpectedArguments {
        command: String,
        arity: CommandArity,
    },
    Unknown {
        command: String,
        suggestion: Option<&'static str>,
    },
}

/// Every command name known at the prompt, built-ins first then app launch words,
/// in the fixed order used for completion and did-you-mean.
pub(crate) fn terminal_command_names(
    app_commands: &[NovaOsAppCommand],
) -> impl Iterator<Item = &'static str> + '_ {
    TERMINAL_COMMANDS
        .iter()
        .map(|command| command.name)
        .chain(app_commands.iter().map(|app| app.id))
}

/// Every command as `(name, arity, is_app)`, built-ins first then apps.
pub(crate) fn terminal_command_specs(
    app_commands: &[NovaOsAppCommand],
) -> impl Iterator<Item = (&'static str, CommandArity, bool)> + '_ {
    TERMINAL_COMMANDS
        .iter()
        .map(|command| (command.name, command.arity, false))
        .chain(app_commands.iter().map(|app| (app.id, app.arity, true)))
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

/// Resolve a command line against the built-ins and registered apps. Matches the
/// LONGEST command name that is a word-prefix of the input (so a multi-word
/// launch word like `ship view` beats the `ship` built-in), then validates the
/// trailing words against that command's arity. There is no per-command special
/// case: multi-word names and argument-taking commands both fall out of this.
pub(crate) fn resolve_command(
    command_line: &str,
    app_commands: &[NovaOsAppCommand],
) -> ResolvedCommand {
    let words: Vec<&str> = command_line.split_whitespace().collect();
    let Some(&first) = words.first() else {
        return ResolvedCommand::Unknown {
            command: String::new(),
            suggestion: None,
        };
    };
    let best = terminal_command_specs(app_commands)
        .filter_map(|(name, arity, is_app)| {
            command_name_matches(&words, name).map(|name_words| (name, arity, is_app, name_words))
        })
        .max_by_key(|(_, _, _, name_words)| *name_words);
    let Some((name, arity, is_app, name_words)) = best else {
        return ResolvedCommand::Unknown {
            command: first.to_string(),
            suggestion: nearest_command(first, app_commands),
        };
    };
    let arg_count = words.len() - name_words;
    // `<command> help` / `<command> version` are universal sub-verbs, resolved
    // ahead of the arity check so they work even on a no-arg command.
    if arg_count == 1 && is_help_flag(words[name_words]) {
        return ResolvedCommand::Usage { name };
    }
    if arg_count == 1 && is_version_flag(words[name_words]) {
        return ResolvedCommand::Version { name };
    }
    if !arity.accepts(arg_count) {
        return ResolvedCommand::UnexpectedArguments {
            command: name.to_string(),
            arity,
        };
    }
    if is_app {
        ResolvedCommand::App { id: name }
    } else {
        ResolvedCommand::Builtin { name }
    }
}

fn nearest_command(input: &str, app_commands: &[NovaOsAppCommand]) -> Option<&'static str> {
    // Typo suggestions cover app launch words too, so a mistyped `map` gets a
    // did-you-mean the same way a mistyped builtin does.
    TERMINAL_COMMANDS
        .iter()
        .map(|command| command.name)
        .chain(app_commands.iter().map(|app| app.id))
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
    use crate::app::NovaOsAppCommand;
    #[test]
    fn nova_os_registered_commands_match_html_set() {
        // The executable set + order mirror the HTML PoC. `map view` is the one
        // addition beyond the PoC set: the CLI counterpart of the `map` app (its
        // own launch word `map` is registered by the app, not the built-in table).
        let registered: Vec<&str> = TERMINAL_COMMANDS
            .iter()
            .map(|command| command.name)
            .collect();
        assert_eq!(
            registered,
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

        for name in [
            "help",
            "clear",
            "log",
            "objectives",
            "ship",
            "map view",
            "version",
            "exit",
        ] {
            assert!(
                matches!(
                    resolve_command(name, &[]),
                    ResolvedCommand::Builtin { name: matched } if matched == name
                ),
                "{name} resolves to its built-in",
            );
        }
        // `reload`, `repair` are single unknown words until their own tasks
        // register them. Bare `map` is also unknown UNTIL the `map` app registers
        // its launch word (with no app registered here, `map` alone matches
        // nothing - `map view` needs the second word).
        for planned in ["map", "reload", "repair"] {
            assert!(
                matches!(
                    resolve_command(planned, &[]),
                    ResolvedCommand::Unknown { .. }
                ),
                "{planned} stays deferred to its own task"
            );
        }
        // With the `map` app registered, `map` resolves to the app while
        // `map view` still resolves to the two-word built-in (longest-match).
        let map_app = [NovaOsAppCommand {
            id: "map",
            summary: "",
            arity: CommandArity::None,
        }];
        assert!(matches!(
            resolve_command("map", &map_app),
            ResolvedCommand::App { id: "map" }
        ));
        assert!(matches!(
            resolve_command("map view", &map_app),
            ResolvedCommand::Builtin { name: "map view" }
        ));
        // `<command> help` (and -h/--help) shows a command's own usage, for a
        // built-in or an app, ahead of the arity check.
        assert!(matches!(
            resolve_command("ship help", &map_app),
            ResolvedCommand::Usage { name: "ship" }
        ));
        assert!(matches!(
            resolve_command("map -h", &map_app),
            ResolvedCommand::Usage { name: "map" }
        ));
        // A non-help bad argument to a no-arg command still rejects (submit turns
        // it into a usage hint that names the sub-commands, e.g. `map view`).
        assert!(matches!(
            resolve_command("map v", &map_app),
            ResolvedCommand::UnexpectedArguments { command, .. } if command == "map"
        ));
        assert_eq!(subcommands_of("map", &map_app), vec!["map view"]);
        // `ship viewer` is no longer a hardcoded special-case: with no app
        // registered it is just the `ship` built-in with an unexpected argument.
        assert!(matches!(
            resolve_command("ship viewer", &[]),
            ResolvedCommand::UnexpectedArguments { command, .. } if command == "ship"
        ));
    }
    #[test]
    fn nova_os_typo_of_an_app_word_is_suggested() {
        // Did-you-mean covers app launch words, not just builtins (finding 3).
        let apps = [NovaOsAppCommand {
            id: "sample",
            summary: "",
            arity: CommandArity::None,
        }];
        assert_eq!(
            nearest_command("sanple", &apps),
            Some("sample"),
            "a typo of a registered app word suggests that app word",
        );
        assert_eq!(
            nearest_command("sanple", &[]),
            None,
            "without the app registered there is no near builtin to suggest",
        );
    }
    /// The parser accepts an argument-taking command registration and a multi-word
    /// launch word, without breaking the argument-free built-ins.
    #[test]
    fn nova_os_parser_supports_arguments_and_multiword() {
        let apps = [
            NovaOsAppCommand {
                id: "ship view",
                summary: "",
                arity: CommandArity::None,
            },
            NovaOsAppCommand {
                id: "repair",
                summary: "",
                arity: CommandArity::UpTo(1),
            },
        ];

        // A multi-word launch word resolves as the app, its 2-word name beating the
        // `ship` built-in on longest-match.
        assert!(matches!(
            resolve_command("ship view", &apps),
            ResolvedCommand::App { id: "ship view" }
        ));
        // The `ship` built-in still resolves on its own.
        assert!(matches!(
            resolve_command("ship", &apps),
            ResolvedCommand::Builtin { name: "ship" }
        ));
        // An argument-taking command accepts its argument...
        assert!(matches!(
            resolve_command("repair thruster", &apps),
            ResolvedCommand::App { id: "repair" }
        ));
        // ...and rejects more than its arity.
        assert!(matches!(
            resolve_command("repair a b", &apps),
            ResolvedCommand::UnexpectedArguments { command, .. } if command == "repair"
        ));
        // Argument-free built-ins are unaffected.
        assert!(matches!(
            resolve_command("help", &apps),
            ResolvedCommand::Builtin { name: "help" }
        ));
        assert!(matches!(
            resolve_command("help x", &apps),
            ResolvedCommand::UnexpectedArguments { command, .. } if command == "help"
        ));
        // The multi-word launch word rejects a trailing argument (arity none).
        assert!(matches!(
            resolve_command("ship view x", &apps),
            ResolvedCommand::UnexpectedArguments { command, .. } if command == "ship view"
        ));
    }
}
