//! The Command shell's language: the curated catalog, its metadata, the parse
//! entry point both front ends share, and the structured result they both
//! receive.
//!
//! This is NOT the scenario action vocabulary. `EventActionConfig`
//! (`nova_scenario::actions`) stays an authoring and implementation enum; a
//! command exists here because it was deliberately added to
//! [`COMMAND_CATALOG`], and adding a scenario action does not add a command.
//!
//! Everything in this module is pure: it parses a line against the catalog and
//! answers whatever can be answered from the catalog alone (`help`, `commands`,
//! usage, and every parse error). Anything that has to look at the live game is
//! handed back as a [`CommandInvocation`] for the dispatcher above to run, so
//! the CRT and the process channel go through one parser and one result shape.

use std::sync::OnceLock;

use bevy::prelude::Resource;

use crate::{
    shell::{
        resolve_command, subcommands_of, CommandArg, CommandArity, CommandDispatch,
        ResolvedCommand, TerminalCommandSpec,
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

    /// Parse the word `commands <class>` takes.
    pub fn parse(word: &str) -> Option<Self> {
        let word = word.to_ascii_lowercase();
        CommandClass::ALL
            .into_iter()
            .find(|class| class.label() == word)
    }
}

/// The live-value tokens the catalog names in a [`CommandArg::Live`] argument.
///
/// The token is all this crate knows: it says an argument is a ship without
/// knowing what a ship is. The executor - which does own the world - publishes
/// the values under the same token with
/// [`NovaOsTerminal::merge_live_values`](crate::terminal::NovaOsTerminal::merge_live_values),
/// and Tab then completes them.
pub mod live {
    /// Live ship ids.
    pub const SHIP: &str = "ship";
    /// Live section ids, across every ship: a section is addressed by its ship
    /// AND its own id, and the terminal cannot know which ship was typed, so
    /// the set it completes from is the union.
    pub const SECTION: &str = "section";
    /// Registered scenario ids.
    pub const SCENARIO: &str = "scenario";
    /// The live scenario's variable names.
    pub const VARIABLE: &str = "variable";
    /// Registered input action names.
    pub const ACTION: &str = "action";
    /// The input sources `bind` accepts.
    pub const SOURCE: &str = "source";
    /// Command names, for `help <command>`. Published by the terminal itself
    /// from its own command set, so it needs no executor.
    pub const COMMAND: &str = "command";
    /// The labels the MAP app's live contact list is showing.
    pub const CONTACT: &str = "contact";

    /// Every token, so [`noun`] can be pinned against the list by test.
    pub const ALL: &[&str] = &[
        SHIP, SECTION, SCENARIO, VARIABLE, ACTION, SOURCE, COMMAND, CONTACT,
    ];

    /// What the player is being ASKED for, as `help` phrases it. Tab is what
    /// turns the noun into the actual set.
    pub fn noun(token: &str) -> &'static str {
        match token {
            SHIP => "a live ship id",
            SECTION => "a section id on that ship",
            SCENARIO => "a scenario id",
            VARIABLE => "a scenario variable name",
            ACTION => "an input action name",
            SOURCE => "a key, mouse button or pad button",
            COMMAND => "a command name",
            CONTACT => "a contact label on the map",
            _ => UNNAMED,
        }
    }

    /// The answer [`noun`] gives for a token nobody phrased. Pinned by test.
    pub const UNNAMED: &str = "a value the world knows";
}

/// The class words `commands [class]` accepts. Pinned against
/// [`CommandClass::ALL`] by test, so a new class cannot go uncompleted.
const CLASS_WORDS: &[&str] = &["utility", "readonly", "setting", "cheat"];

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
    /// What each argument position accepts, in order. Tab completion reads it,
    /// so an argument declared here completes against the live world.
    pub args: &'static [CommandArg],
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
        args: &[CommandArg::Live(live::COMMAND)],
        examples: &["help", "help ammo infinite"],
    },
    CommandSpec {
        name: "commands",
        usage: "commands [class]",
        summary: "List every command, or one class",
        class: CommandClass::Utility,
        arity: CommandArity::Between(0, 1),
        arg_hint: Some("[class]"),
        args: &[CommandArg::Words(CLASS_WORDS)],
        examples: &["commands", "commands cheat"],
    },
    CommandSpec {
        name: "clear",
        usage: "clear",
        summary: "Restore this shell's introduction",
        class: CommandClass::Utility,
        arity: CommandArity::None,
        arg_hint: None,
        args: &[],
        examples: &["clear"],
    },
    CommandSpec {
        name: "close",
        usage: "close",
        summary: "Close the terminal and return to what was underneath",
        class: CommandClass::Utility,
        arity: CommandArity::None,
        arg_hint: None,
        args: &[],
        examples: &["close"],
    },
    CommandSpec {
        name: "scenario",
        usage: "scenario",
        summary: "Show the current scenario, state and outcome",
        class: CommandClass::ReadOnly,
        arity: CommandArity::None,
        arg_hint: None,
        args: &[],
        examples: &["scenario"],
    },
    CommandSpec {
        name: "scenario load",
        usage: "scenario load <id>",
        summary: "Abandon this attempt and load a fresh scenario",
        class: CommandClass::Utility,
        arity: CommandArity::Between(1, 1),
        arg_hint: Some("<id>"),
        args: &[CommandArg::Live(live::SCENARIO)],
        examples: &["scenario load shakedown_run"],
    },
    CommandSpec {
        name: "status",
        usage: "status",
        summary: "Show a compact run and world summary",
        class: CommandClass::ReadOnly,
        arity: CommandArity::None,
        arg_hint: None,
        args: &[],
        examples: &["status"],
    },
    CommandSpec {
        name: "ships",
        usage: "ships",
        summary: "List live ships by id",
        class: CommandClass::ReadOnly,
        arity: CommandArity::None,
        arg_hint: None,
        args: &[],
        examples: &["ships"],
    },
    CommandSpec {
        name: "ship",
        usage: "ship <id>",
        summary: "Inspect one ship",
        class: CommandClass::ReadOnly,
        arity: CommandArity::Between(1, 1),
        arg_hint: Some("<id>"),
        args: &[CommandArg::Live(live::SHIP)],
        examples: &["ship player_spaceship"],
    },
    CommandSpec {
        name: "sections",
        usage: "sections <ship-id>",
        summary: "List a ship's sections",
        class: CommandClass::ReadOnly,
        arity: CommandArity::Between(1, 1),
        arg_hint: Some("<ship-id>"),
        args: &[CommandArg::Live(live::SHIP)],
        examples: &["sections player_spaceship"],
    },
    CommandSpec {
        name: "section",
        usage: "section <ship-id> <section-id>",
        summary: "Inspect one section of one ship",
        class: CommandClass::ReadOnly,
        // A section id is unique to its ship, not to the field: both cargoa
        // hulls carry `turret_port`. The ship is part of the address.
        arity: CommandArity::Between(2, 2),
        arg_hint: Some("<ship-id> <section-id>"),
        args: &[
            CommandArg::Live(live::SHIP),
            CommandArg::Live(live::SECTION),
        ],
        examples: &["section player_spaceship turret_port"],
    },
    CommandSpec {
        name: "objectives",
        usage: "objectives",
        summary: "List current objectives and completion state",
        class: CommandClass::ReadOnly,
        arity: CommandArity::None,
        arg_hint: None,
        args: &[],
        examples: &["objectives"],
    },
    CommandSpec {
        name: "variables",
        usage: "variables",
        summary: "List scenario variables",
        class: CommandClass::ReadOnly,
        arity: CommandArity::None,
        arg_hint: None,
        args: &[],
        examples: &["variables"],
    },
    CommandSpec {
        name: "variable",
        usage: "variable <name>",
        summary: "Read one scenario variable",
        class: CommandClass::ReadOnly,
        arity: CommandArity::Between(1, 1),
        arg_hint: Some("<name>"),
        args: &[CommandArg::Live(live::VARIABLE)],
        examples: &["variable gates_cleared"],
    },
    CommandSpec {
        name: "bindings",
        usage: "bindings [action]",
        summary: "List input actions, or inspect one",
        class: CommandClass::ReadOnly,
        arity: CommandArity::Between(0, 1),
        arg_hint: Some("[action]"),
        args: &[CommandArg::Live(live::ACTION)],
        examples: &["bindings", "bindings novaos_toggle"],
    },
    CommandSpec {
        name: "settings",
        usage: "settings",
        summary: "Show all current settings",
        class: CommandClass::ReadOnly,
        arity: CommandArity::None,
        arg_hint: None,
        args: &[],
        examples: &["settings"],
    },
    CommandSpec {
        name: "cheats status",
        usage: "cheats status",
        summary: "Report arming and run-mark state",
        class: CommandClass::ReadOnly,
        arity: CommandArity::None,
        arg_hint: None,
        args: &[],
        examples: &["cheats status"],
    },
    CommandSpec {
        name: "cheats enable",
        usage: "cheats enable",
        summary: "Arm cheats and mark this run, one way",
        class: CommandClass::Cheat,
        arity: CommandArity::None,
        arg_hint: None,
        args: &[],
        examples: &["cheats enable"],
    },
    CommandSpec {
        name: "graphics",
        usage: "graphics [low|medium|high]",
        summary: "Print or change the graphics quality preset",
        class: CommandClass::Setting,
        arity: CommandArity::Between(0, 1),
        arg_hint: Some("[low|medium|high]"),
        args: &[CommandArg::Words(&["low", "medium", "high"])],
        examples: &["graphics", "graphics low"],
    },
    CommandSpec {
        name: "volume",
        usage: "volume [master|music|world|interface [0..1]]",
        summary: "Print or change a mixer channel's volume",
        class: CommandClass::Setting,
        arity: CommandArity::Between(0, 2),
        arg_hint: Some("[channel [0..1]]"),
        args: &[
            CommandArg::Words(&["master", "music", "world", "interface"]),
            CommandArg::Free,
        ],
        examples: &["volume", "volume master", "volume world 0.4"],
    },
    CommandSpec {
        name: "window",
        usage: "window [windowed|borderless]",
        summary: "Print or change the window mode",
        class: CommandClass::Setting,
        arity: CommandArity::Between(0, 1),
        arg_hint: Some("[windowed|borderless]"),
        args: &[CommandArg::Words(&["windowed", "borderless"])],
        examples: &["window", "window borderless"],
    },
    CommandSpec {
        name: "bind",
        usage: "bind <action> <source>",
        summary: "Rebind one input action",
        class: CommandClass::Setting,
        arity: CommandArity::Between(2, 2),
        arg_hint: Some("<action> <source>"),
        args: &[
            CommandArg::Live(live::ACTION),
            CommandArg::Live(live::SOURCE),
        ],
        examples: &["bind novaos_toggle F1"],
    },
    CommandSpec {
        name: "bind reset",
        usage: "bind reset <action>",
        summary: "Restore one action's registered default",
        class: CommandClass::Setting,
        arity: CommandArity::Between(1, 1),
        arg_hint: Some("<action>"),
        args: &[CommandArg::Live(live::ACTION)],
        examples: &["bind reset novaos_toggle"],
    },
    CommandSpec {
        name: "ammo infinite",
        usage: "ammo infinite <ship-id> <on|off>",
        summary: "Enable or disable unlimited ammunition on one ship",
        class: CommandClass::Cheat,
        arity: CommandArity::Between(2, 2),
        arg_hint: Some("<ship-id> <on|off>"),
        args: &[
            CommandArg::Live(live::SHIP),
            CommandArg::Words(&["on", "off"]),
        ],
        examples: &[
            "ammo infinite player_spaceship on",
            "ammo infinite player_spaceship off",
        ],
    },
    CommandSpec {
        name: "ammo refill",
        usage: "ammo refill <ship-id>",
        summary: "Refill every finite magazine on one ship",
        class: CommandClass::Cheat,
        arity: CommandArity::Between(1, 1),
        arg_hint: Some("<ship-id>"),
        args: &[CommandArg::Live(live::SHIP)],
        examples: &["ammo refill player_spaceship"],
    },
    CommandSpec {
        name: "ammo refill section",
        usage: "ammo refill section <ship-id> <section-id>",
        summary: "Refill one finite magazine",
        class: CommandClass::Cheat,
        arity: CommandArity::Between(2, 2),
        arg_hint: Some("<ship-id> <section-id>"),
        args: &[
            CommandArg::Live(live::SHIP),
            CommandArg::Live(live::SECTION),
        ],
        examples: &["ammo refill section player_spaceship turret_port"],
    },
    CommandSpec {
        name: "speed-cap",
        usage: "speed-cap <ship-id> <m/s|off>",
        summary: "Change or remove a ship's manual speed cap, in metres per second",
        class: CommandClass::Cheat,
        arity: CommandArity::Between(2, 2),
        arg_hint: Some("<ship-id> <m/s|off>"),
        args: &[CommandArg::Live(live::SHIP), CommandArg::Words(&["off"])],
        examples: &[
            "speed-cap player_spaceship 400",
            "speed-cap player_spaceship off",
        ],
    },
];

/// The catalog as the shared matcher sees it - one [`TerminalCommandSpec`] per
/// catalog row, every one dispatching to the command layer. Built once and
/// lent out: every caller reads the same slice, none copies it.
pub fn command_shell_specs() -> &'static [TerminalCommandSpec] {
    static SPECS: OnceLock<Vec<TerminalCommandSpec>> = OnceLock::new();
    SPECS.get_or_init(|| {
        COMMAND_CATALOG
            .iter()
            .map(|spec| TerminalCommandSpec {
                name: spec.name,
                summary: spec.summary,
                arity: spec.arity,
                arg_hint: spec.arg_hint,
                args: spec.args,
                dispatch: CommandDispatch::Command,
            })
            .collect()
    })
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
            // An OVER-run past a command that owns subcommands is a bad
            // subcommand and is named as one. An UNDER-run is not: a bare
            // `bind` typed nothing, and `unknown subcommand ''` blames the
            // player for a word they never wrote.
            let subs = subcommands_of(&command, specs);
            // Past what the command accepts, the next word is a sub-command
            // that does not exist (`ammo: unknown subcommand 'x'`). An arity
            // miss that is NOT an overrun is just an arity miss, whether or not
            // the command owns sub-commands - a bare `bind` is not a bad
            // sub-command.
            let overrun = arity
                .overruns(args.len())
                .then(|| args[arity.most()].as_str());
            let headline = match overrun {
                Some(bad) if !subs.is_empty() => format!("{command}: unknown subcommand '{bad}'"),
                _ => format!("{command}: {}", arity.rejection()),
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

/// The `help` answer: the whole catalog, or one command's block.
///
/// Bare `help` LISTS. A shell whose help only says "type a command" teaches
/// nothing, and the catalog is the one thing a player arriving at `cmd>` does
/// not have: every command, grouped by what it is allowed to touch, with the
/// keys that drive the prompt under it.
fn help_result(args: &[String]) -> CommandResult {
    // A command name can be several words, so the arguments are the NAME, not
    // a name and its own arguments.
    let command = args.join(" ");
    if command.is_empty() {
        let mut rows = vec![
            TerminalRow::info(format!(
                "{} commands in {} classes. Usage: <command> [arguments]",
                command_registry_count(),
                CommandClass::ALL.len(),
            )),
            TerminalRow::dim(String::new()),
        ];
        rows.extend(command_list_rows(None));
        rows.extend(shell_key_rows());
        return CommandResult::ok(
            "help",
            CommandClass::Utility,
            format!("listed {} commands", command_registry_count()),
        )
        .with_rows(rows);
    }
    usage_result(&command)
}

/// What drives the prompt itself, printed under every catalog listing. These
/// are the emulator's keys, not commands, so they have no catalog row to be
/// read off - and a player who cannot find them types blind.
fn shell_key_rows() -> Vec<TerminalRow> {
    vec![
        TerminalRow::dim(String::new()),
        TerminalRow::info("PROMPT"),
        TerminalRow::output("  Tab            complete a command, or the argument under the caret"),
        TerminalRow::output("  Up / Down      walk this shell's history"),
        TerminalRow::output("  Ctrl+A / E     caret to the start / end of the line"),
        TerminalRow::output("  Ctrl+U / K     cut to the start / end of the line"),
        TerminalRow::output("  Esc            close the terminal and return to what was under it"),
        TerminalRow::dim("Type 'help <command>' for one command's details."),
        TerminalRow::dim(
            "Cheats are refused until `cheats enable` arms them, and arming marks the run.",
        ),
    ]
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
    rows.extend(
        spec.args
            .iter()
            .enumerate()
            .map(|(at, arg)| TerminalRow::dim(format!("  {}. {}", at + 1, arg_line(*arg)))),
    );
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

/// What one argument position accepts, as `help <command>` says it. A live
/// token is printed as the noun it names, because that is what a player is
/// being asked for; Tab is what turns the noun into the actual set.
fn arg_line(arg: CommandArg) -> String {
    match arg {
        CommandArg::Words(words) => format!("one of {}", words.join(", ")),
        CommandArg::Live(token) => format!("{} (Tab lists it)", live::noun(token)),
        CommandArg::Free => "a value".to_string(),
    }
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
    .and_rows([TerminalRow::dim("Type 'help <command>' for details.")])
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
        if !rows.is_empty() {
            rows.push(TerminalRow::dim(String::new()));
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

    /// A declared argument is what Tab completes and what `help` describes, so
    /// a command that takes arguments and declares none silently loses both.
    #[test]
    fn every_argument_position_is_declared() {
        for spec in COMMAND_CATALOG {
            let takes = match spec.arity {
                CommandArity::None => 0,
                CommandArity::UpTo(max) | CommandArity::Between(_, max) => max,
            };
            assert_eq!(
                spec.args.is_empty(),
                takes == 0,
                "{} takes {takes} arguments and declares {}",
                spec.name,
                spec.args.len(),
            );
            // Fewer is legal: `help [command]` takes up to three WORDS and they
            // are one argument, a command name.
            assert!(
                spec.args.len() <= takes,
                "{} declares more arguments than it accepts",
                spec.name,
            );
        }
    }

    /// Every live token a catalog argument names is phrased for `help`. The
    /// fallback exists so a token cannot panic; a shipped token using it is a
    /// command asking the player for "a value the world knows".
    #[test]
    fn every_live_token_is_phrased_for_help() {
        for token in live::ALL {
            assert_ne!(live::noun(token), live::UNNAMED, "{token} has no noun");
        }
        for spec in COMMAND_CATALOG {
            for arg in spec.args {
                if let CommandArg::Live(token) = arg {
                    assert!(
                        live::ALL.contains(token),
                        "{} names the unlisted token '{token}'",
                        spec.name,
                    );
                }
            }
        }
    }

    /// The class words `commands <class>` completes from are the classes, so a
    /// new class cannot go uncompleted.
    #[test]
    fn the_class_words_are_the_classes() {
        let words: Vec<&str> = CommandClass::ALL
            .iter()
            .map(|class| class.label())
            .collect();
        assert_eq!(CLASS_WORDS, words.as_slice());
    }

    /// A bare command that owns subcommands is an UNDER-run, not a bad
    /// subcommand: `bind` used to print `bind: unknown subcommand ''`.
    #[test]
    fn an_arity_miss_reads_as_an_arity_miss_not_a_bad_subcommand() {
        let specs = command_shell_specs();
        let headline = |line: &str| {
            let CommandOutcome::Answer(result) = resolve_command_line(line, &specs) else {
                panic!("{line} is answered by the catalog");
            };
            result.detail
        };
        assert_eq!(headline("bind"), "bind: takes 2 arguments");
        // The bad word is the first one PAST what the command accepts, not the
        // first argument: `ammo refill <ship>` takes one, so the second is the
        // sub-command that does not exist.
        assert_eq!(
            headline("ammo refill cargoa nope"),
            "ammo refill: unknown subcommand 'nope'",
        );
    }

    /// `help` alone has to teach the shell, not just name it: the catalog, and
    /// the prompt keys that are not commands and so have no catalog row.
    #[test]
    fn bare_help_lists_the_catalog_and_the_prompt_keys() {
        let specs = command_shell_specs();
        let CommandOutcome::Answer(result) = resolve_command_line("help", &specs) else {
            panic!("help is answered by the catalog");
        };
        let text: Vec<&str> = result.rows.iter().map(|row| row.text.as_str()).collect();
        for spec in COMMAND_CATALOG {
            assert!(
                text.iter().any(|row| row.contains(spec.usage)),
                "help does not list {}",
                spec.name,
            );
        }
        for class in CommandClass::ALL {
            assert!(
                text.iter()
                    .any(|row| row.starts_with(&class.label().to_uppercase())),
                "help does not head the {} class",
                class.label(),
            );
        }
        assert!(text.iter().any(|row| row.contains("Tab")), "help names Tab");
    }

    /// `help <command>` says what each argument POSITION accepts, in order, so
    /// the player knows a section is addressed by its ship.
    #[test]
    fn one_commands_help_describes_each_argument() {
        let rows = usage_rows("section");
        let text: Vec<&str> = rows.iter().map(|row| row.text.as_str()).collect();
        assert!(
            text.iter().any(|row| row.contains(live::noun(live::SHIP))),
            "the first argument is a ship: {text:?}",
        );
        assert!(
            text.iter()
                .any(|row| row.contains(live::noun(live::SECTION))),
            "the second is a section on it: {text:?}",
        );
    }

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
            invoke("ammo refill section player_spaceship PDC-1").name,
            "ammo refill section"
        );
        assert_eq!(invoke("ammo refill player_spaceship").name, "ammo refill");
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

/// The channel line one queued command came in on.
///
/// The CRT is not a source here: a typed command reaches the dispatcher through
/// [`NovaOsTerminal::take_pending_command`](crate::terminal::NovaOsTerminal::take_pending_command)
/// and its answer goes straight to the scrollback. This queue exists for the
/// callers whose answer has to be addressed back to a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSource {
    /// The channel line's sequence, echoed back on the acknowledgement.
    pub seq: u64,
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

    /// Take every answer waiting to be acknowledged. The front end's half.
    pub fn drain_answers(&mut self) -> Vec<(CommandSource, CommandResult)> {
        std::mem::take(&mut self.answers)
    }

    /// Whether anything is waiting to run.
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }
}
