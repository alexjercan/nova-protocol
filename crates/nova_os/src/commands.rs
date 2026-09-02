//! The Command shell's language: the curated catalog, its metadata, the parse
//! entry point both front ends share, and the structured result they both
//! receive.
//!
//! This is NOT the scenario action vocabulary. [`EventActionConfig`] stays an
//! authoring and implementation enum; a command exists here because it was
//! deliberately added to [`COMMAND_CATALOG`], and adding a scenario action does
//! not add a command.
//!
//! Everything in this module is pure: it parses a line against the catalog and
//! answers whatever can be answered from the catalog alone (`help`, `commands`,
//! usage, and every parse error). Anything that has to look at the live game is
//! handed back as a [`CommandInvocation`] for the dispatcher above to run, so
//! the CRT and the process channel go through one parser and one result shape.
//!
//! [`EventActionConfig`]: https://docs.rs/ "nova_scenario::actions::EventActionConfig"

use std::sync::OnceLock;

use bevy::prelude::Resource;

use crate::{
    shell::{
        resolve_command, subcommands_of, CommandArity, CommandDispatch, ResolvedCommand,
        TerminalCommandSpec,
    },
    terminal::{CommandInvocation, TerminalRow, TerminalRowKind},
};

/// What a command is allowed to touch. The class is the whole permission model:
/// it decides whether arming is required, whether the run is marked, and which
/// section of the reference documents the command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandClass {
    /// Controls the shell itself, or abandons one scenario to load another.
    Utility,
    /// Observes state and never mutates it.
    ReadOnly,
    /// Changes the same persisted player settings as the settings UI. Never
    /// marks the run.
    Setting,
    /// Changes the live world. Requires arming, and arming marks the run.
    Cheat,
}

impl CommandClass {
    /// The classes, in catalog order.
    pub const ALL: [CommandClass; 4] = [
        CommandClass::Utility,
        CommandClass::ReadOnly,
        CommandClass::Setting,
        CommandClass::Cheat,
    ];

    /// The lowercase word the shell prints and `commands <class>` accepts.
    pub fn label(self) -> &'static str {
        match self {
            CommandClass::Utility => "utility",
            CommandClass::ReadOnly => "readonly",
            CommandClass::Setting => "setting",
            CommandClass::Cheat => "cheat",
        }
    }

    /// One line describing what the class may do, for `commands`.
    pub fn summary(self) -> &'static str {
        match self {
            CommandClass::Utility => "shell control and scenario loading",
            CommandClass::ReadOnly => "observe state; never mutates",
            CommandClass::Setting => "change persisted player settings",
            CommandClass::Cheat => "change the live world; needs arming",
        }
    }

    /// Whether running this class marks the run as cheated.
    pub fn marks_run(self) -> bool {
        matches!(self, CommandClass::Cheat)
    }

    /// Parse the word `commands <class>` takes.
    pub fn parse(word: &str) -> Option<Self> {
        let word = word.to_ascii_lowercase();
        CommandClass::ALL
            .into_iter()
            .find(|class| class.label() == word)
    }
}

/// One command's single source of metadata: what it is called, how it is typed,
/// what it does, what it may touch, and how it is exercised.
///
/// Parsing, completion, `help`, `commands` and the shipped reference all read
/// this one value, so a command cannot be documented in one place and parsed
/// differently in another.
#[derive(Debug, Clone, Copy)]
pub struct CommandSpec {
    /// The full command name, a word sequence (`ammo refill section`).
    pub name: &'static str,
    /// The whole typed form, arguments included.
    pub usage: &'static str,
    /// One-line summary for `help` and `commands`.
    pub summary: &'static str,
    /// What the command may touch.
    pub class: CommandClass,
    /// How many argument words follow the name.
    pub arity: CommandArity,
    /// The argument placeholder, for the shared usage renderer.
    pub arg_hint: Option<&'static str>,
    /// Worked examples printed by `help <command>`.
    pub examples: &'static [&'static str],
}

/// The whole Command-shell vocabulary. Deliberately closed: a scenario action
/// becoming a command is an edit HERE, made on purpose.
pub const COMMAND_CATALOG: &[CommandSpec] = &[
    CommandSpec {
        name: "help",
        usage: "help [command]",
        summary: "Show basic usage, or one command's details",
        class: CommandClass::Utility,
        // Up to THREE words, because a command name can be three words:
        // `help ammo refill section` has to reach the command it names.
        arity: CommandArity::UpTo(3),
        arg_hint: Some("[command]"),
        examples: &["help", "help ammo infinite"],
    },
    CommandSpec {
        name: "commands",
        usage: "commands [class]",
        summary: "List every command, or one class",
        class: CommandClass::Utility,
        arity: CommandArity::Between(0, 1),
        arg_hint: Some("[class]"),
        examples: &["commands", "commands cheat"],
    },
    CommandSpec {
        name: "clear",
        usage: "clear",
        summary: "Restore this shell's introduction",
        class: CommandClass::Utility,
        arity: CommandArity::None,
        arg_hint: None,
        examples: &["clear"],
    },
    CommandSpec {
        name: "close",
        usage: "close",
        summary: "Close the terminal and return to what was underneath",
        class: CommandClass::Utility,
        arity: CommandArity::None,
        arg_hint: None,
        examples: &["close"],
    },
    CommandSpec {
        name: "scenario",
        usage: "scenario",
        summary: "Show the current scenario, state and outcome",
        class: CommandClass::ReadOnly,
        arity: CommandArity::None,
        arg_hint: None,
        examples: &["scenario"],
    },
    CommandSpec {
        name: "scenario load",
        usage: "scenario load <id>",
        summary: "Abandon this attempt and load a fresh scenario",
        class: CommandClass::Utility,
        arity: CommandArity::Between(1, 1),
        arg_hint: Some("<id>"),
        examples: &["scenario load shakedown_run"],
    },
    CommandSpec {
        name: "status",
        usage: "status",
        summary: "Show a compact run and world summary",
        class: CommandClass::ReadOnly,
        arity: CommandArity::None,
        arg_hint: None,
        examples: &["status"],
    },
    CommandSpec {
        name: "ships",
        usage: "ships",
        summary: "List live ships by id",
        class: CommandClass::ReadOnly,
        arity: CommandArity::None,
        arg_hint: None,
        examples: &["ships"],
    },
    CommandSpec {
        name: "ship",
        usage: "ship <id>",
        summary: "Inspect one ship",
        class: CommandClass::ReadOnly,
        arity: CommandArity::Between(1, 1),
        arg_hint: Some("<id>"),
        examples: &["ship player_ship"],
    },
    CommandSpec {
        name: "sections",
        usage: "sections <ship-id>",
        summary: "List a ship's sections",
        class: CommandClass::ReadOnly,
        arity: CommandArity::Between(1, 1),
        arg_hint: Some("<ship-id>"),
        examples: &["sections player_ship"],
    },
    CommandSpec {
        name: "section",
        usage: "section <id>",
        summary: "Inspect one section",
        class: CommandClass::ReadOnly,
        arity: CommandArity::Between(1, 1),
        arg_hint: Some("<id>"),
        examples: &["section player_turret_1"],
    },
    CommandSpec {
        name: "objectives",
        usage: "objectives",
        summary: "List current objectives and completion state",
        class: CommandClass::ReadOnly,
        arity: CommandArity::None,
        arg_hint: None,
        examples: &["objectives"],
    },
    CommandSpec {
        name: "variables",
        usage: "variables",
        summary: "List scenario variables",
        class: CommandClass::ReadOnly,
        arity: CommandArity::None,
        arg_hint: None,
        examples: &["variables"],
    },
    CommandSpec {
        name: "variable",
        usage: "variable <name>",
        summary: "Read one scenario variable",
        class: CommandClass::ReadOnly,
        arity: CommandArity::Between(1, 1),
        arg_hint: Some("<name>"),
        examples: &["variable gates_cleared"],
    },
    CommandSpec {
        name: "bindings",
        usage: "bindings [action]",
        summary: "List input actions, or inspect one",
        class: CommandClass::ReadOnly,
        arity: CommandArity::Between(0, 1),
        arg_hint: Some("[action]"),
        examples: &["bindings", "bindings novaos_toggle"],
    },
    CommandSpec {
        name: "settings",
        usage: "settings",
        summary: "Show all current settings",
        class: CommandClass::ReadOnly,
        arity: CommandArity::None,
        arg_hint: None,
        examples: &["settings"],
    },
    CommandSpec {
        name: "cheats status",
        usage: "cheats status",
        summary: "Report arming and run-mark state",
        class: CommandClass::ReadOnly,
        arity: CommandArity::None,
        arg_hint: None,
        examples: &["cheats status"],
    },
    CommandSpec {
        name: "cheats enable",
        usage: "cheats enable",
        summary: "Arm cheats and mark this run, one way",
        class: CommandClass::Cheat,
        arity: CommandArity::None,
        arg_hint: None,
        examples: &["cheats enable"],
    },
    CommandSpec {
        name: "graphics",
        usage: "graphics [low|medium|high]",
        summary: "Print or change the graphics quality preset",
        class: CommandClass::Setting,
        arity: CommandArity::Between(0, 1),
        arg_hint: Some("[low|medium|high]"),
        examples: &["graphics", "graphics low"],
    },
    CommandSpec {
        name: "volume",
        usage: "volume [master|music|world|interface [0..1]]",
        summary: "Print or change a mixer channel's volume",
        class: CommandClass::Setting,
        arity: CommandArity::Between(0, 2),
        arg_hint: Some("[channel [0..1]]"),
        examples: &["volume", "volume master", "volume world 0.4"],
    },
    CommandSpec {
        name: "window",
        usage: "window [windowed|borderless]",
        summary: "Print or change the window mode",
        class: CommandClass::Setting,
        arity: CommandArity::Between(0, 1),
        arg_hint: Some("[windowed|borderless]"),
        examples: &["window", "window borderless"],
    },
    CommandSpec {
        name: "bind",
        usage: "bind <action> <source>",
        summary: "Rebind one input action",
        class: CommandClass::Setting,
        arity: CommandArity::Between(2, 2),
        arg_hint: Some("<action> <source>"),
        examples: &["bind novaos_toggle F1"],
    },
    CommandSpec {
        name: "bind reset",
        usage: "bind reset <action>",
        summary: "Restore one action's registered default",
        class: CommandClass::Setting,
        arity: CommandArity::Between(1, 1),
        arg_hint: Some("<action>"),
        examples: &["bind reset novaos_toggle"],
    },
    CommandSpec {
        name: "ammo infinite",
        usage: "ammo infinite <ship-id> <on|off>",
        summary: "Enable or disable unlimited ammunition on one ship",
        class: CommandClass::Cheat,
        arity: CommandArity::Between(2, 2),
        arg_hint: Some("<ship-id> <on|off>"),
        examples: &[
            "ammo infinite player_ship on",
            "ammo infinite player_ship off",
        ],
    },
    CommandSpec {
        name: "ammo refill",
        usage: "ammo refill <ship-id>",
        summary: "Refill every finite magazine on one ship",
        class: CommandClass::Cheat,
        arity: CommandArity::Between(1, 1),
        arg_hint: Some("<ship-id>"),
        examples: &["ammo refill player_ship"],
    },
    CommandSpec {
        name: "ammo refill section",
        usage: "ammo refill section <section-id>",
        summary: "Refill one finite magazine",
        class: CommandClass::Cheat,
        arity: CommandArity::Between(1, 1),
        arg_hint: Some("<section-id>"),
        examples: &["ammo refill section player_turret_1"],
    },
    CommandSpec {
        name: "speed-cap",
        usage: "speed-cap <ship-id> <m/s|off>",
        summary: "Change or remove a ship's manual speed cap, in metres per second",
        class: CommandClass::Cheat,
        arity: CommandArity::Between(2, 2),
        arg_hint: Some("<ship-id> <m/s|off>"),
        examples: &["speed-cap player_ship 400", "speed-cap player_ship off"],
    },
];

/// The catalog as the shared matcher sees it - one [`TerminalCommandSpec`] per
/// catalog row, every one dispatching to the command layer. Built once.
pub fn command_shell_specs() -> Vec<TerminalCommandSpec> {
    static SPECS: OnceLock<Vec<TerminalCommandSpec>> = OnceLock::new();
    SPECS
        .get_or_init(|| {
            COMMAND_CATALOG
                .iter()
                .map(|spec| TerminalCommandSpec {
                    name: spec.name,
                    summary: spec.summary,
                    arity: spec.arity,
                    arg_hint: spec.arg_hint,
                    dispatch: CommandDispatch::Command,
                })
                .collect()
        })
        .clone()
}

/// How many commands the catalog holds - the number the shell's introduction
/// reports. Computed, never written down twice.
pub fn command_registry_count() -> usize {
    COMMAND_CATALOG.len()
}

/// One catalog row by name.
pub fn command_spec(name: &str) -> Option<&'static CommandSpec> {
    COMMAND_CATALOG.iter().find(|spec| spec.name == name)
}

/// How a command ended, for both the shell's colouring and the channel's ack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStatus {
    /// The command ran and did what it says.
    Ok,
    /// The command exists and was understood, but is not permitted right now
    /// (an unarmed cheat, a surface that has no such state).
    Refused,
    /// The input was wrong: an unknown command, bad arguments, an id that does
    /// not resolve.
    Error,
}

impl CommandStatus {
    /// The lowercase word the channel ack carries.
    pub fn label(self) -> &'static str {
        match self {
            CommandStatus::Ok => "ok",
            CommandStatus::Refused => "refused",
            CommandStatus::Error => "error",
        }
    }
}

/// What running one command produced. The CRT prints [`Self::rows`]; the channel
/// acks [`Self::command`], [`Self::class`] and [`Self::status`] with
/// [`Self::detail`] as the one-line answer. Both front ends receive exactly
/// this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    /// The resolved command name, or the offending word when nothing resolved.
    pub command: String,
    /// The class of the resolved command; `None` when nothing resolved.
    pub class: Option<CommandClass>,
    /// How it ended.
    pub status: CommandStatus,
    /// The rows a shell prints.
    pub rows: Vec<TerminalRow>,
    /// One line summarising the outcome, for a caller with no screen.
    pub detail: String,
}

impl CommandResult {
    /// A successful result whose rows are `rows` and whose ack line is `detail`.
    pub fn ok(name: impl Into<String>, class: CommandClass, detail: impl Into<String>) -> Self {
        Self {
            command: name.into(),
            class: Some(class),
            status: CommandStatus::Ok,
            rows: Vec::new(),
            detail: detail.into(),
        }
    }

    /// A refusal: understood, not permitted. The detail is printed as a warning
    /// row so a player is told why.
    pub fn refused(
        name: impl Into<String>,
        class: CommandClass,
        detail: impl Into<String>,
    ) -> Self {
        let detail = detail.into();
        Self {
            command: name.into(),
            class: Some(class),
            status: CommandStatus::Refused,
            rows: vec![TerminalRow::warn(detail.clone())],
            detail,
        }
    }

    /// A failure: bad input. The detail is printed as an error row.
    pub fn error(
        name: impl Into<String>,
        class: Option<CommandClass>,
        detail: impl Into<String>,
    ) -> Self {
        let detail = detail.into();
        Self {
            command: name.into(),
            class,
            status: CommandStatus::Error,
            rows: vec![TerminalRow::error(detail.clone())],
            detail,
        }
    }

    /// Replace the rows, keeping the status and ack line.
    #[must_use]
    pub fn with_rows(mut self, rows: Vec<TerminalRow>) -> Self {
        self.rows = rows;
        self
    }

    /// Append rows.
    #[must_use]
    pub fn and_rows(mut self, rows: impl IntoIterator<Item = TerminalRow>) -> Self {
        self.rows.extend(rows);
        self
    }
}

/// What a parsed command line turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome {
    /// Answered from the catalog alone - help, a listing, or a parse error. No
    /// world was needed and none will be touched.
    Answer(Box<CommandResult>),
    /// A real command: hand it to the dispatcher, which runs it against the
    /// live game.
    Invoke(CommandInvocation),
}

/// Parse one command line against `specs` (the Command catalog as the matcher
/// sees it). This is the ONE entry point both the CRT prompt and the process
/// channel use, so the two cannot drift.
pub fn resolve_command_line(line: &str, specs: &[TerminalCommandSpec]) -> CommandOutcome {
    let line = line.trim();
    if line.is_empty() {
        return CommandOutcome::Answer(Box::new(CommandResult::error(
            "",
            None,
            "empty command line",
        )));
    }
    match resolve_command(line, specs) {
        ResolvedCommand::Run { name, args, .. } => {
            let Some(spec) = command_spec(name) else {
                return CommandOutcome::Answer(Box::new(CommandResult::error(
                    name,
                    None,
                    format!("{name}: no catalog entry"),
                )));
            };
            // `help` and `commands` are answers about the catalog, so they are
            // served here rather than costing the dispatcher a round trip.
            match name {
                "help" => CommandOutcome::Answer(Box::new(help_result(&args))),
                "commands" => CommandOutcome::Answer(Box::new(commands_result(args.first()))),
                _ => CommandOutcome::Invoke(CommandInvocation {
                    name: spec.name,
                    class: spec.class,
                    args,
                }),
            }
        }
        ResolvedCommand::Usage { name } => CommandOutcome::Answer(Box::new(usage_result(name))),
        ResolvedCommand::Version => CommandOutcome::Answer(Box::new(
            CommandResult::ok("version", CommandClass::Utility, version_line())
                .with_rows(vec![TerminalRow::info(version_line())]),
        )),
        ResolvedCommand::Incomplete { name } => {
            let subs = subcommands_of(name, specs);
            let mut rows = vec![TerminalRow::error(format!("{name}: incomplete command"))];
            rows.push(TerminalRow::output("Subcommands:"));
            rows.extend(subs.iter().map(|sub| {
                let summary = command_spec(sub).map(|spec| spec.summary).unwrap_or("");
                TerminalRow::output(format!("  {sub}  {summary}"))
            }));
            CommandOutcome::Answer(Box::new(CommandResult {
                command: name.to_string(),
                class: None,
                status: CommandStatus::Error,
                rows,
                detail: format!("{name}: incomplete command"),
            }))
        }
        ResolvedCommand::UnexpectedArguments {
            command,
            arity,
            args,
        } => {
            let subs = subcommands_of(&command, specs);
            let headline = if subs.is_empty() {
                format!("{command}: {}", arity.rejection())
            } else {
                let bad = args.first().map(String::as_str).unwrap_or_default();
                format!("{command}: unknown subcommand '{bad}'")
            };
            let mut result = CommandResult::error(
                command.clone(),
                command_spec(&command).map(|spec| spec.class),
                headline,
            );
            result.rows.extend(usage_rows(&command));
            CommandOutcome::Answer(Box::new(result))
        }
        ResolvedCommand::Unknown {
            command,
            suggestion,
        } => {
            let mut result = CommandResult::error(
                command.clone(),
                None,
                format!("command not found: {command}"),
            );
            if let Some(suggestion) = suggestion {
                result
                    .rows
                    .push(TerminalRow::warn(format!("did you mean {suggestion}?")));
            }
            result
                .rows
                .push(TerminalRow::dim("Type 'help' for a list of commands."));
            CommandOutcome::Answer(Box::new(result))
        }
    }
}

fn version_line() -> String {
    format!("NOVA OS v{} // COMMANDS", nova_info::APP_VERSION)
}

/// The `help` answer: bare usage, or one command's block.
fn help_result(args: &[String]) -> CommandResult {
    // A command name can be several words, so the arguments are the NAME, not
    // a name and its own arguments.
    let command = args.join(" ");
    if command.is_empty() {
        return CommandResult::ok("help", CommandClass::Utility, "shell usage").with_rows(vec![
            TerminalRow::info("Usage: <command> [arguments]"),
            TerminalRow::output("  help [command]     this text, or one command's details"),
            TerminalRow::output("  commands [class]   list every command, or one class"),
            TerminalRow::dim(format!(
                "{} commands in {} classes: {}",
                command_registry_count(),
                CommandClass::ALL.len(),
                CommandClass::ALL
                    .iter()
                    .map(|class| class.label())
                    .collect::<Vec<_>>()
                    .join(", "),
            )),
            TerminalRow::dim("Cheats are refused until `cheats enable` arms them."),
            TerminalRow::dim("`close` leaves the terminal; `clear` restores this introduction."),
        ]);
    }
    usage_result(&command)
}

/// `help <command>` / `<command> help`: the full block for one command.
fn usage_result(name: &str) -> CommandResult {
    match command_spec(name) {
        Some(spec) => {
            CommandResult::ok(spec.name, spec.class, spec.summary).with_rows(usage_rows(name))
        }
        None => CommandResult::error(name, None, format!("no help for '{name}'")),
    }
}

/// One command's help block: title, usage, class, arguments and examples.
pub fn usage_rows(name: &str) -> Vec<TerminalRow> {
    let Some(spec) = command_spec(name) else {
        return vec![TerminalRow::error(format!("no help for '{name}'"))];
    };
    let mut rows = vec![
        TerminalRow::info(format!("{} - {}", spec.name, spec.summary)),
        TerminalRow::output(format!("Usage: {}", spec.usage)),
        TerminalRow::output(format!(
            "Class: {} ({})",
            spec.class.label(),
            spec.class.summary()
        )),
        TerminalRow::output(format!("Arguments: {}", arity_line(spec.arity))),
    ];
    if !spec.examples.is_empty() {
        rows.push(TerminalRow::output("Examples:"));
        rows.extend(
            spec.examples
                .iter()
                .map(|example| TerminalRow::dim(format!("  {example}"))),
        );
    }
    if spec.class == CommandClass::Cheat && spec.name != "cheats enable" {
        rows.push(TerminalRow::warn(
            "Refused until `cheats enable` arms cheats; arming marks the run.",
        ));
    }
    rows
}

/// How many argument words a command takes, in words.
fn arity_line(arity: CommandArity) -> String {
    match arity {
        CommandArity::None => "none".to_string(),
        CommandArity::UpTo(max) => format!("up to {max}"),
        CommandArity::Between(min, max) if min == max => {
            let word = if min == 1 { "word" } else { "words" };
            format!("exactly {min} {word}")
        }
        CommandArity::Between(min, max) => format!("{min} to {max} words"),
    }
}

/// `commands [class]`: the catalog, aligned, whole or filtered.
fn commands_result(class: Option<&String>) -> CommandResult {
    let filter = match class {
        None => None,
        Some(word) => match CommandClass::parse(word) {
            Some(class) => Some(class),
            None => {
                return CommandResult::error(
                    "commands",
                    Some(CommandClass::Utility),
                    format!(
                        "commands: no class named '{word}' ({})",
                        CommandClass::ALL
                            .iter()
                            .map(|class| class.label())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                )
            }
        },
    };
    CommandResult::ok(
        "commands",
        CommandClass::Utility,
        match filter {
            Some(class) => format!("listed the {} commands", class.label()),
            None => format!("listed {} commands", command_registry_count()),
        },
    )
    .with_rows(command_list_rows(filter))
}

/// The catalog listing, grouped by class in [`CommandClass::ALL`] order.
pub fn command_list_rows(filter: Option<CommandClass>) -> Vec<TerminalRow> {
    let width = COMMAND_CATALOG
        .iter()
        .filter(|spec| filter.is_none_or(|class| spec.class == class))
        .map(|spec| spec.usage.len())
        .max()
        .unwrap_or(0);
    let mut rows = Vec::new();
    for class in CommandClass::ALL {
        if filter.is_some_and(|wanted| wanted != class) {
            continue;
        }
        let mut listed = COMMAND_CATALOG
            .iter()
            .filter(|spec| spec.class == class)
            .peekable();
        if listed.peek().is_none() {
            continue;
        }
        rows.push(TerminalRow::info(format!(
            "{} - {}",
            class.label().to_uppercase(),
            class.summary()
        )));
        rows.extend(
            listed.map(|spec| {
                TerminalRow::output(format!("  {:width$}  {}", spec.usage, spec.summary))
            }),
        );
    }
    rows.push(TerminalRow::dim("Type 'help <command>' for details."));
    rows
}

/// The Command shell's staged introduction, revealed row-by-row on first entry
/// and reprinted whole by `clear`.
///
/// `world` describes the live context (`shakedown_run / paused`, `main menu /
/// idle`, `ship editor / paused`, `no scenario / idle`); `armed` is whether the
/// player has run `cheats enable` in this run.
pub fn command_intro_rows(world: &str, armed: bool) -> Vec<TerminalRow> {
    let cheats = if armed {
        "enabled / run marked"
    } else {
        "disabled / run clean"
    };
    vec![
        TerminalRow::info(version_line()),
        TerminalRow::dim("POST ......... command shell / ok"),
        TerminalRow::dim("CORE ......... local game runtime / attached"),
        TerminalRow::dim(format!(
            "REGISTRY ..... {} commands / ready",
            command_registry_count()
        )),
        TerminalRow::dim(format!("WORLD ........ {world}")),
        TerminalRow::new(
            if armed {
                TerminalRowKind::Warn
            } else {
                TerminalRowKind::Dim
            },
            format!("CHEATS ....... {cheats}"),
        ),
        TerminalRow::warn("Hint: type `help` and press Enter."),
    ]
}

#[cfg(test)]
mod tests {
    /// Every catalog example has to be a line the parser accepts, or the help
    /// text teaches something that does not work. `help ammo infinite` was
    /// exactly that: a two-word name arriving as two arguments.
    #[test]
    fn every_documented_example_parses() {
        let specs = super::command_shell_specs();
        for spec in super::COMMAND_CATALOG {
            for example in spec.examples {
                let outcome = super::resolve_command_line(example, &specs);
                let bad = match &outcome {
                    super::CommandOutcome::Answer(result) => {
                        result.status == super::CommandStatus::Error
                    }
                    super::CommandOutcome::Invoke(_) => false,
                };
                assert!(!bad, "`{example}` (from {}) does not parse", spec.name);
            }
        }
    }

    /// `help` names a command, and a command name can be three words.
    #[test]
    fn help_reaches_a_multi_word_command() {
        let specs = super::command_shell_specs();
        let super::CommandOutcome::Answer(result) =
            super::resolve_command_line("help ammo refill section", &specs)
        else {
            panic!("help is answered by the catalog");
        };
        assert_eq!(result.status, super::CommandStatus::Ok);
        assert_eq!(result.command, "ammo refill section");
    }

    use super::*;

    /// Every catalog row is reachable by the parser under its own name, and the
    /// classes agree between the catalog and what the parser hands the
    /// dispatcher. A command that parses to a different class than it documents
    /// would be armed (or not) by the wrong rule.
    #[test]
    fn every_catalogued_command_parses_back_to_its_own_class() {
        let specs = command_shell_specs();
        for spec in COMMAND_CATALOG {
            // The usage line with its placeholders stripped down to the name
            // plus one dummy word per required argument.
            let args = match spec.arity {
                CommandArity::None | CommandArity::UpTo(_) => 0,
                CommandArity::Between(min, _) => min,
            };
            let mut line = spec.name.to_string();
            for index in 0..args {
                line.push_str(&format!(" arg{index}"));
            }
            match resolve_command_line(&line, &specs) {
                CommandOutcome::Invoke(invocation) => {
                    assert_eq!(invocation.name, spec.name);
                    assert_eq!(invocation.class, spec.class, "{}", spec.name);
                }
                // `help` and `commands` answer from the catalog itself.
                CommandOutcome::Answer(result) => assert!(
                    matches!(spec.name, "help" | "commands"),
                    "{} answered instead of dispatching: {result:?}",
                    spec.name,
                ),
            }
        }
    }

    /// Help and completion derive from the catalog, so every command has a
    /// usage block naming its class - there is no command the shell can run and
    /// cannot document.
    #[test]
    fn every_command_documents_its_usage_class_and_arguments() {
        for spec in COMMAND_CATALOG {
            let rows = usage_rows(spec.name);
            let text: Vec<&str> = rows.iter().map(|row| row.text.as_str()).collect();
            assert!(
                text.iter().any(|row| row.contains(spec.usage)),
                "{} prints its usage",
                spec.name
            );
            assert!(
                text.iter()
                    .any(|row| row.starts_with("Class: ") && row.contains(spec.class.label())),
                "{} names its class",
                spec.name
            );
            assert!(
                text.iter().any(|row| row.starts_with("Arguments: ")),
                "{} names its arguments",
                spec.name
            );
            assert!(
                !spec.examples.is_empty(),
                "{} carries at least one example",
                spec.name
            );
            assert!(
                spec.usage.starts_with(spec.name),
                "{} usage line starts with its own name",
                spec.name
            );
        }
        // The listing covers the whole catalog and nothing else.
        let listed = command_list_rows(None)
            .into_iter()
            .filter(|row| row.text.starts_with("  ") && !row.text.starts_with("  Type"))
            .count();
        assert_eq!(listed, command_registry_count());
    }

    /// A cheat is refused before arming, and the refusal is part of the
    /// documented block rather than a surprise at the prompt.
    #[test]
    fn cheat_help_warns_that_arming_is_required() {
        let rows = usage_rows("ammo infinite");
        assert!(
            rows.iter().any(|row| row.text.contains("cheats enable")),
            "a cheat's help says what arms it",
        );
        // Arming itself must not tell the player to arm first.
        let arming = usage_rows("cheats enable");
        assert!(
            !arming.iter().any(|row| row.text.contains("Refused until")),
            "`cheats enable` is the arming act, not a gated cheat",
        );
    }

    /// The longest registered name wins, so the three-word ammo command is not
    /// swallowed by the two-word one, and `bind reset` is not a rebind of an
    /// action called `reset`.
    #[test]
    fn multiword_commands_resolve_to_the_longest_match() {
        let specs = command_shell_specs();
        let invoke = |line: &str| match resolve_command_line(line, &specs) {
            CommandOutcome::Invoke(invocation) => invocation,
            other => panic!("{line} did not dispatch: {other:?}"),
        };
        assert_eq!(
            invoke("ammo refill section PDC-1").name,
            "ammo refill section"
        );
        assert_eq!(invoke("ammo refill player_ship").name, "ammo refill");
        assert_eq!(invoke("bind reset novaos_toggle").name, "bind reset");
        let bind = invoke("bind novaos_toggle F1");
        assert_eq!(bind.name, "bind");
        assert_eq!(bind.args, ["novaos_toggle", "F1"]);
    }

    /// An incomplete parent word lists what it could have been instead of
    /// dead-ending on "command not found".
    #[test]
    fn an_incomplete_command_lists_its_subcommands() {
        let specs = command_shell_specs();
        let CommandOutcome::Answer(result) = resolve_command_line("ammo", &specs) else {
            panic!("`ammo` is not a runnable command");
        };
        assert_eq!(result.status, CommandStatus::Error);
        let text: Vec<&str> = result.rows.iter().map(|row| row.text.as_str()).collect();
        assert!(text.iter().any(|row| row.contains("incomplete command")));
        assert!(text.iter().any(|row| row.contains("ammo infinite")));
        assert!(text.iter().any(|row| row.contains("ammo refill section")));
    }

    /// `commands cheat` lists the cheats and nothing else; an unknown class
    /// names the ones that exist rather than printing an empty list.
    #[test]
    fn commands_filters_by_class_and_refuses_an_unknown_one() {
        let specs = command_shell_specs();
        let CommandOutcome::Answer(cheats) = resolve_command_line("commands cheat", &specs) else {
            panic!("`commands` answers from the catalog");
        };
        assert_eq!(cheats.status, CommandStatus::Ok);
        for row in &cheats.rows {
            let trimmed = row.text.trim_start();
            if let Some(spec) = COMMAND_CATALOG
                .iter()
                .find(|spec| trimmed.starts_with(spec.usage))
            {
                assert_eq!(spec.class, CommandClass::Cheat, "{}", spec.name);
            }
        }
        let CommandOutcome::Answer(bad) = resolve_command_line("commands nonsense", &specs) else {
            panic!("an unknown class is answered, not dispatched");
        };
        assert_eq!(bad.status, CommandStatus::Error);
        assert!(bad.detail.contains("cheat"), "{}", bad.detail);
    }

    /// The introduction reports the computed registry count and the live cheat
    /// mark rather than a written-down number.
    #[test]
    fn the_intro_reports_the_computed_registry_and_cheat_state() {
        let clean = command_intro_rows("shakedown_run / paused", false);
        let text = |rows: &[TerminalRow]| {
            rows.iter()
                .map(|row| row.text.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let clean_text = text(&clean);
        assert!(clean_text.contains(&format!(
            "REGISTRY ..... {} commands / ready",
            COMMAND_CATALOG.len()
        )));
        assert!(clean_text.contains("WORLD ........ shakedown_run / paused"));
        assert!(clean_text.contains("CHEATS ....... disabled / run clean"));
        assert!(
            clean_text.contains(&format!("NOVA OS v{}", nova_info::APP_VERSION)),
            "the header reads the build version, never a literal",
        );

        let armed = command_intro_rows("main menu / idle", true);
        assert!(text(&armed).contains("CHEATS ....... enabled / run marked"));
        assert_eq!(
            armed
                .iter()
                .find(|row| row.text.starts_with("CHEATS"))
                .map(|row| row.kind),
            Some(TerminalRowKind::Warn),
            "an armed run says so in amber",
        );
    }
}

/// `CommandClass`, the catalog and its spec, the shared parse entry point and
/// the structured result both front ends receive.
pub mod prelude {
    pub use super::{
        command_intro_rows, command_list_rows, command_registry_count, command_shell_specs,
        command_spec, resolve_command_line, usage_rows, CommandChannel, CommandClass,
        CommandOutcome, CommandResult, CommandSource, CommandSpec, CommandStatus, COMMAND_CATALOG,
    };
}

/// Who asked for a command to run.
///
/// The dispatcher answers both the same way; only the delivery of the answer
/// differs - the shell prints its rows, the channel acknowledges by sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSource {
    /// The CRT's Command shell. The answer goes to that shell's scrollback.
    Shell,
    /// The process channel. The answer becomes an acknowledgement carrying this
    /// sequence number.
    Channel {
        /// The channel line's sequence, echoed back on the acknowledgement.
        seq: u64,
    },
}

/// One command waiting for the dispatcher, and one answer waiting for its
/// caller.
///
/// The queue exists so the process channel can drive the same dispatcher as the
/// CRT without either front end depending on the other, or on the gameplay
/// crates the dispatcher needs. `nova_os` owns the language; whoever owns the
/// world drains this.
#[derive(Resource, Debug, Default)]
pub struct CommandChannel {
    pending: Vec<(CommandSource, CommandInvocation)>,
    answers: Vec<(CommandSource, CommandResult)>,
}

impl CommandChannel {
    /// Queue an invocation for the dispatcher.
    pub fn submit(&mut self, source: CommandSource, invocation: CommandInvocation) {
        self.pending.push((source, invocation));
    }

    /// Queue an answer the dispatcher never had to run - a parse refusal, or a
    /// `help` the catalog answered on its own.
    pub fn answer(&mut self, source: CommandSource, result: CommandResult) {
        self.answers.push((source, result));
    }

    /// Take everything waiting to run. The dispatcher's half.
    pub fn drain_pending(&mut self) -> Vec<(CommandSource, CommandInvocation)> {
        std::mem::take(&mut self.pending)
    }

    /// Take every answer for one source. The front end's half: the channel asks
    /// for its own without consuming the shell's.
    pub fn drain_answers_for(
        &mut self,
        wanted: impl Fn(CommandSource) -> bool,
    ) -> Vec<(CommandSource, CommandResult)> {
        let mut taken = Vec::new();
        self.answers.retain(|(source, result)| {
            if wanted(*source) {
                taken.push((*source, result.clone()));
                false
            } else {
                true
            }
        });
        taken
    }

    /// Whether anything is waiting to run.
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }
}
