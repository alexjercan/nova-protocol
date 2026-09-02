//! NOVA OS logic: the ship-computer's terminal model, shell command language and
//! app runtime, factored out of the bevy UI in `nova_os_ui`.
//!
//! `nova_os_ui` owns the NOVA OS bevy UI (the CRT casing, the terminal nodes,
//! the keyboard systems and the plugin) and the game-data bridges that turn live
//! objectives / flight-log / ship state into [`terminal::TerminalRow`]s. This crate owns
//! everything under those bridges: the [`terminal::NovaOsTerminal`] prompt state and its
//! edit/submit/completion behaviour ([`terminal`]), the command matcher and typo
//! suggestions ([`shell`]), and the [`app::NovaOsAppRuntime`] app seam ([`app`]).
//!
//! The crate also owns the GAME-level command language ([`commands`]): its
//! curated catalog, the parse entry point the CRT and the process channel
//! share, and the structured result they both receive. The catalog is metadata
//! only - running a command needs the live game, so the dispatcher lives above
//! this crate and this one stays free of gameplay, scenario and menu deps.
//!
//! The split keeps the dependency graph acyclic (`nova_os_ui -> nova_os`) and
//! `nova_ui` free of any NOVA OS dependency. The crate depends on `bevy` on
//! purpose: the [`terminal::NovaOsTerminal`] resource, the
//! [`app::NovaOsAppRuntime`] trait's `spawn_body`/`handle_key`, and the
//! `Handle<Font>` row styling all speak bevy types.
#![warn(missing_docs)]

pub mod app;
pub mod command;
pub mod commands;
pub mod shell;
pub mod terminal;

/// The public NOVA OS logic surface `nova_os_ui` imports as one glob.
pub mod prelude {
    pub use crate::{
        app::{
            command_shell_hints, terminal_hints, NovaOsAppInputOutcome, NovaOsAppRuntime,
            COMMAND_SHELL_HINTS, NOVA_OS_TERMINAL_HINTS,
        },
        command::{nova_os_footer_hints, CommandBody, NovaOsCommandRegistry, TerminalCommand},
        commands::{
            command_intro_rows, command_list_rows, command_registry_count, command_shell_specs,
            command_spec, resolve_command_line, usage_rows, CommandChannel, CommandClass,
            CommandOutcome, CommandResult, CommandSource, CommandSpec, CommandStatus,
            COMMAND_CATALOG,
        },
        shell::{CliOutput, CommandArity},
        terminal::{
            nova_os_boot_banner_rows, nova_os_version_label, nova_os_welcome_rows,
            prompt_after_cursor, prompt_before_cursor, prompt_completion_ghost,
            prompt_hint_display, terminal_help_rows, CommandInvocation, NovaOsCommandInvocation,
            NovaOsTerminal, ShellKind, TerminalCommandSnapshot, TerminalMode, TerminalParseStatus,
            TerminalRow, TerminalRowKind, TerminalSubmitOutcome,
        },
    };
}
