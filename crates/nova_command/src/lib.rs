//! The game command language and the `NOVA COMMANDS` terminal model, factored
//! out of the bevy UI in `nova_interface`.
//!
//! `nova_interface` owns the CRT modal (the casing, the terminal nodes, the
//! keyboard systems and the plugin). This crate owns everything under it: the
//! [`terminal::CommandTerminal`] prompt state and its edit/submit/completion
//! behaviour ([`terminal`]), and the command matcher and typo suggestions
//! ([`shell`]).
//!
//! The crate also owns the GAME-level command language ([`commands`]): its
//! curated catalog, the parse entry point the CRT and the process channel
//! share, and the structured result they both receive. The catalog is metadata
//! only - running a command needs the live game, so the dispatcher lives above
//! this crate and this one stays free of gameplay, scenario and menu deps.
//!
//! The split keeps the dependency graph acyclic (`nova_interface ->
//! nova_command`) and `nova_ui` free of any terminal dependency. The crate
//! depends on `bevy` on purpose: the [`terminal::CommandTerminal`] resource
//! speaks bevy types.
#![warn(missing_docs)]

pub mod commands;
pub mod shell;
pub mod terminal;

/// The public command surface `nova_interface` imports as one glob.
pub mod prelude {
    pub use crate::{
        commands::{
            command_intro_rows, command_list_rows, command_registry_count, command_shell_hints,
            command_shell_specs, command_spec, live, resolve_command_line, usage_rows,
            CommandChannel, CommandClass, CommandError, CommandOutcome, CommandResult,
            CommandSource, CommandSpec, CommandStatus, COMMAND_CATALOG, COMMAND_SHELL_HINTS,
        },
        shell::{CommandArg, CommandArity},
        terminal::{
            nova_os_version_label, prompt_after_cursor, prompt_before_cursor,
            prompt_completion_ghost, prompt_hint_display, CommandInvocation, CommandTerminal,
            TerminalParseStatus, TerminalRow, TerminalRowKind, TerminalSubmitOutcome,
        },
    };
}
