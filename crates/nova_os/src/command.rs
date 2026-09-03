//! The unified command model: the rich [`TerminalCommand`] authoring form, the
//! [`NovaOsCommandRegistry`] that holds the command tree, and the flattening into
//! the [`TerminalCommandSpec`]s the matcher and the pure terminal consume.
//!
//! One concept, [`TerminalCommand`], describes every command the NOVA OS knows.
//! A command either launches an app ([`CommandBody::App`], e.g. `map`) or performs
//! a CLI action ([`CommandBody::Cli`], e.g. `log`, `map view`), and may own
//! subcommands. The core builtins are seeded by [`core_terminal_commands`]; a
//! gameplay plugin registers its own tree (the `map` app plus its `view`
//! subcommand) with [`NovaOsCommandRegistry::register`], so an app's launch word,
//! its CLI subcommands and its runtime are declared together in one place.

use bevy::prelude::*;
use nova_input::prelude::InputBindings;

use crate::{
    app::prelude::*,
    shell::prelude::*,
    terminal::{ShellKind, TerminalMode},
};

/// What a [`TerminalCommand`] does when it runs: launch its app (owning the app
/// runtime) or perform a CLI action.
pub enum CommandBody {
    /// A CLI command that prints to the terminal (`help`, `log`, `map view`, ...).
    Cli(CliOutput),
    /// An app command: running it hands the screen to this runtime, whose id is
    /// the command's name.
    App(Box<dyn NovaOsAppRuntime>),
    /// A command handled by the gameplay layer: the terminal records the parsed
    /// invocation and the gameplay layer applies it against the live world and
    /// appends the result rows (`ship section/reload/repair <id>`). See
    /// [`CommandDispatch::Gameplay`].
    Gameplay,
}

impl CommandBody {
    /// The flat dispatch tag the matcher stores for this body.
    fn dispatch(&self) -> CommandDispatch {
        match self {
            CommandBody::Cli(output) => CommandDispatch::Cli(*output),
            CommandBody::App(_) => CommandDispatch::App,
            CommandBody::Gameplay => CommandDispatch::Gameplay,
        }
    }
}

/// A NOVA OS command: its full name (a subcommand carries its whole name, e.g.
/// `"map view"`), a one-line summary, its argument arity, what it does
/// ([`CommandBody`]) and any nested subcommands. This is the authoring form held
/// by [`NovaOsCommandRegistry`]; the matcher and terminal see the flattened
/// [`TerminalCommandSpec`]s from [`NovaOsCommandRegistry::specs`].
pub struct TerminalCommand {
    /// The full command name (`"help"`, `"map"`, `"map view"`).
    pub name: &'static str,
    /// One-line summary for `help` and completion.
    pub summary: &'static str,
    /// How many argument words the command accepts after its name.
    pub arity: CommandArity,
    /// The placeholder shown for this command's argument in a usage line
    /// (`"<label>"`, `"<section>"`). `None` for a no-argument command, or for an
    /// arg-bearing command that has not named its argument yet (the renderer then
    /// falls back to a generic `<arg>`). Set at the registration site with
    /// [`Self::with_arg_hint`].
    pub arg_hint: Option<&'static str>,
    /// What each argument position accepts, in order. Drives Tab completion,
    /// so a verb that names a live token here completes against the world.
    /// Set with [`Self::with_args`].
    pub args: &'static [CommandArg],
    /// What running the command does.
    pub body: CommandBody,
    /// Nested subcommands, each carrying its own FULL name.
    pub subcommands: Vec<TerminalCommand>,
}

impl TerminalCommand {
    /// A no-argument CLI command with no subcommands.
    pub fn cli(name: &'static str, summary: &'static str, output: CliOutput) -> Self {
        Self {
            name,
            summary,
            arity: CommandArity::None,
            arg_hint: None,
            args: &[],
            body: CommandBody::Cli(output),
            subcommands: Vec::new(),
        }
    }

    /// A no-argument app command with no subcommands. Its launch word is `name`
    /// and running it spawns `runtime`.
    pub fn app(name: &'static str, summary: &'static str, runtime: impl NovaOsAppRuntime) -> Self {
        Self {
            name,
            summary,
            arity: CommandArity::None,
            arg_hint: None,
            args: &[],
            body: CommandBody::App(Box::new(runtime)),
            subcommands: Vec::new(),
        }
    }

    /// A gameplay-handled command taking `arity` argument words with no
    /// subcommands. The gameplay layer receives the parsed args and appends the
    /// result rows (`ship reload <id>`); use it for the arg-bearing ship verbs.
    /// Name the argument for its usage line with [`Self::with_arg_hint`].
    pub fn gameplay(name: &'static str, summary: &'static str, arity: CommandArity) -> Self {
        Self {
            name,
            summary,
            arity,
            arg_hint: None,
            args: &[],
            body: CommandBody::Gameplay,
            subcommands: Vec::new(),
        }
    }

    /// Name the argument shown in this command's usage line (`"<label>"`,
    /// `"<section>"`), returning `self` for builder-style chaining. The hint is
    /// the placeholder a shell prints after the command name, e.g.
    /// `Usage: map goto <label>`.
    pub fn with_arg_hint(mut self, arg_hint: &'static str) -> Self {
        self.arg_hint = Some(arg_hint);
        self
    }

    /// Say what each argument position accepts, so Tab completes values rather
    /// than command names. A [`CommandArg::Live`] token is filled in by
    /// whoever owns the world, through
    /// [`NovaOsTerminal::merge_live_values`](crate::terminal::NovaOsTerminal::merge_live_values).
    pub fn with_args(mut self, args: &'static [CommandArg]) -> Self {
        self.args = args;
        self
    }

    /// Attach a subcommand, returning `self` for builder-style chaining. The
    /// subcommand's name must extend this command's name by one word
    /// (`"map"` -> `"map view"`); debug builds assert it.
    pub fn with_subcommand(mut self, subcommand: TerminalCommand) -> Self {
        debug_assert!(
            subcommand
                .name
                .strip_prefix(self.name)
                .and_then(|rest| rest.strip_prefix(' '))
                .is_some_and(|word| !word.is_empty() && !word.contains(' ')),
            "subcommand `{}` must extend `{}` by exactly one word",
            subcommand.name,
            self.name,
        );
        self.subcommands.push(subcommand);
        self
    }

    /// Append this command and all its subcommands (depth-first) to `out` as flat
    /// specs.
    fn flatten_into(&self, out: &mut Vec<TerminalCommandSpec>) {
        out.push(TerminalCommandSpec {
            name: self.name,
            summary: self.summary,
            arity: self.arity,
            arg_hint: self.arg_hint,
            args: self.args,
            dispatch: self.body.dispatch(),
        });
        for subcommand in &self.subcommands {
            subcommand.flatten_into(out);
        }
    }

    /// The app runtime registered at `id` within this command's subtree, if any.
    fn app_runtime(&self, id: &str) -> Option<&dyn NovaOsAppRuntime> {
        if self.name == id {
            if let CommandBody::App(runtime) = &self.body {
                return Some(runtime.as_ref());
            }
        }
        self.subcommands
            .iter()
            .find_map(|subcommand| subcommand.app_runtime(id))
    }
}

/// The core (app-less) NOVA OS builtins, in HTML-PoC order. Apps and their CLI
/// subcommands (like `map` / `map view`) are NOT here - they are registered with
/// their app by the owning gameplay plugin.
pub fn core_terminal_commands() -> Vec<TerminalCommand> {
    vec![
        TerminalCommand::cli("help", "Show this command list", CliOutput::Help),
        TerminalCommand::cli("log", "Print comms and mission events", CliOutput::Snapshot),
        TerminalCommand::cli("objectives", "Print active objectives", CliOutput::Snapshot),
        TerminalCommand::cli("clear", "Clear terminal scrollback", CliOutput::Clear),
        TerminalCommand::cli("version", "Print the NOVA OS version", CliOutput::Version),
        TerminalCommand::cli(
            "commands",
            "Enter the game command shell",
            CliOutput::EnterCommands,
        ),
        TerminalCommand::cli("exit", "Suspend the NOVA OS computer", CliOutput::Exit),
    ]
}

/// The flattened specs of the core builtins - the command set the pure terminal
/// seeds itself with (before any app registers). Shared with the terminal's
/// `Default` and the shell tests so there is one source of the core set.
pub fn core_command_specs() -> Vec<TerminalCommandSpec> {
    let mut specs = Vec::new();
    for command in core_terminal_commands() {
        command.flatten_into(&mut specs);
    }
    specs
}

/// The registry of every NOVA OS command. Seeded with the core builtins; gameplay
/// plugins register their own command trees (an app plus its subcommands). The
/// terminal mirrors [`Self::specs`] in for parsing/completion/help, and the app-UI
/// systems look a runtime up by id via [`Self::app_runtime`].
#[derive(Resource)]
pub struct NovaOsCommandRegistry {
    roots: Vec<TerminalCommand>,
}

impl Default for NovaOsCommandRegistry {
    fn default() -> Self {
        Self {
            roots: core_terminal_commands(),
        }
    }
}

impl NovaOsCommandRegistry {
    /// Register a command tree (the seam apps plug into). The `map`/`ship viewer`
    /// plugins register their launch word, subcommands and app runtime here.
    pub fn register(&mut self, command: TerminalCommand) {
        self.roots.push(command);
    }

    /// Every command flattened to a spec, in registration order (core builtins
    /// first, then each registered tree depth-first). This is what the terminal
    /// mirrors in.
    pub fn specs(&self) -> Vec<TerminalCommandSpec> {
        let mut specs = Vec::new();
        for command in &self.roots {
            command.flatten_into(&mut specs);
        }
        specs
    }

    /// The registered app runtime whose launch word is `id`, if any.
    pub fn app_runtime(&self, id: &str) -> Option<&dyn NovaOsAppRuntime> {
        self.roots
            .iter()
            .find_map(|command| command.app_runtime(id))
    }
}

/// The footer hint set for the active surface (PoC `HINTS` map): the terminal set
/// at the prompt, or the running app's own [`NovaOsAppRuntime::hints`] while an
/// app owns the screen.
pub fn nova_os_footer_hints(
    shell: ShellKind,
    mode: TerminalMode,
    registry: &NovaOsCommandRegistry,
    bindings: &InputBindings,
) -> Vec<String> {
    match (shell, mode) {
        (ShellKind::Commands, _) => command_shell_hints(bindings),
        (ShellKind::NovaOs, TerminalMode::Prompt) => terminal_hints(bindings),
        (ShellKind::NovaOs, TerminalMode::App { id }) => registry
            .app_runtime(id)
            .map(|app| app.hints(bindings))
            .unwrap_or_else(|| terminal_hints(bindings)),
    }
}

#[cfg(test)]
mod tests {
    use bevy::input::keyboard::Key;

    use super::*;

    struct StubApp;
    impl NovaOsAppRuntime for StubApp {
        fn id(&self) -> &'static str {
            "map"
        }
        fn title(&self) -> &'static str {
            "MAP"
        }
        fn spawn_body(&self, _body: &mut ChildSpawnerCommands, _font: Handle<Font>) {}
        fn handle_key(&self, _key: &Key) -> NovaOsAppInputOutcome {
            NovaOsAppInputOutcome::Continue
        }
    }

    #[test]
    fn registered_app_tree_flattens_and_resolves_its_runtime() {
        let mut registry = NovaOsCommandRegistry::default();
        registry.register(
            TerminalCommand::app("map", "Open the local-space map", StubApp).with_subcommand(
                TerminalCommand::cli(
                    "map view",
                    "Print local-space contacts",
                    CliOutput::Snapshot,
                ),
            ),
        );

        let names: Vec<&str> = registry.specs().iter().map(|spec| spec.name).collect();
        // Core builtins first, then the map tree (parent then subcommand).
        assert_eq!(
            names,
            vec![
                "help",
                "log",
                "objectives",
                "clear",
                "version",
                "commands",
                "exit",
                "map",
                "map view",
            ]
        );
        // The `map` spec launches an app; `map view` is a CLI snapshot command.
        let map = registry
            .specs()
            .into_iter()
            .find(|s| s.name == "map")
            .unwrap();
        assert!(matches!(map.dispatch, CommandDispatch::App));
        let view = registry
            .specs()
            .into_iter()
            .find(|s| s.name == "map view")
            .unwrap();
        assert!(matches!(
            view.dispatch,
            CommandDispatch::Cli(CliOutput::Snapshot)
        ));
        // The app runtime is looked up by its launch word; the subcommand is not
        // an app.
        assert!(registry.app_runtime("map").is_some());
        assert!(registry.app_runtime("map view").is_none());
        assert!(registry.app_runtime("ship").is_none());
    }
}

/// `NovaOsCommandRegistry`, `TerminalCommand` and its `CommandBody`, the core
/// command and spec builders, and `nova_os_footer_hints`.
pub mod prelude {
    pub use super::{
        core_command_specs, core_terminal_commands, nova_os_footer_hints, CommandBody,
        NovaOsCommandRegistry, TerminalCommand,
    };
}
