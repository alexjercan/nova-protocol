//! Test fixtures shared by the terminal test modules.

use crate::{
    command::prelude::core_command_specs, commands::live, shell::prelude::*,
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
        args: &[],
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
        args: &[],
        dispatch: CommandDispatch::App,
    }
}

/// An arg-bearing gameplay command spec named `name`, taking one live section
/// id (an arg-bearing ship verb like `ship repair`).
pub(super) fn gameplay_spec(name: &'static str) -> TerminalCommandSpec {
    TerminalCommandSpec {
        name,
        summary: "",
        arity: CommandArity::UpTo(1),
        arg_hint: None,
        args: &[CommandArg::Live(live::SECTION)],
        dispatch: CommandDispatch::Gameplay,
    }
}

/// A terminal switched to the Command shell, with its introduction already
/// revealed so a test types against the same state a player would.
pub(super) fn command_shell() -> NovaOsTerminal {
    let mut terminal = NovaOsTerminal::default();
    terminal.switch_shell(crate::terminal::ShellKind::Commands);
    terminal
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
