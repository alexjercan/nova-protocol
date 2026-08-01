//! Test fixtures shared by the terminal test modules.

use crate::{
    command::core_command_specs,
    shell::{CliOutput, CommandArity, CommandDispatch, TerminalCommandSpec},
    terminal::NovaOsTerminal,
};

pub(super) fn type_text(terminal: &mut NovaOsTerminal, text: &str) {
    terminal.insert_text(text);
}

/// A no-arg CLI command spec named `name` (a stand-in app subcommand).
pub(super) fn cli_spec(name: &'static str, summary: &'static str) -> TerminalCommandSpec {
    TerminalCommandSpec {
        name,
        summary,
        arity: CommandArity::None,
        arg_hint: None,
        dispatch: CommandDispatch::Cli(CliOutput::Snapshot),
    }
}

/// A no-arg app command spec named `name`.
pub(super) fn app_spec(name: &'static str, summary: &'static str) -> TerminalCommandSpec {
    TerminalCommandSpec {
        name,
        summary,
        arity: CommandArity::None,
        arg_hint: None,
        dispatch: CommandDispatch::App,
    }
}

/// An arg-bearing gameplay command spec named `name` taking one argument (an
/// arg-bearing ship verb like `ship repair`).
pub(super) fn gameplay_spec(name: &'static str) -> TerminalCommandSpec {
    TerminalCommandSpec {
        name,
        summary: "",
        arity: CommandArity::UpTo(1),
        arg_hint: None,
        dispatch: CommandDispatch::Gameplay,
    }
}

/// The core command set plus `extra`, as the terminal would see it once an app
/// registered its tree.
pub(super) fn core_with(
    extra: impl IntoIterator<Item = TerminalCommandSpec>,
) -> Vec<TerminalCommandSpec> {
    let mut specs = core_command_specs();
    specs.extend(extra);
    specs
}
