//! The Command shell's dispatcher: the executor behind the CRT prompt and the
//! process channel.
//!
//! `nova_os` owns the LANGUAGE - the catalog, the parser, the structured result
//! - and stays a leaf, so the terminal model can be tested without a game. This
//! crate owns the EXECUTION, and therefore sits above gameplay, scenario,
//! settings and menu. That split is the whole reason a command can read a live
//! ship without `nova_os` learning what a ship is.
//!
//! Two front ends arrive here and neither can drift from the other:
//!
//! - the CRT prompt, through [`NovaOsTerminal::take_pending_command`];
//! - the process channel, through [`CommandChannel`].
//!
//! Both were parsed by the same
//! [`resolve_command_line`](nova_os::prelude::resolve_command_line), both run
//! through [`dispatch::execute`], and both receive the same [`CommandResult`].
#![warn(missing_docs)]

pub mod dispatch;

mod cheats;
mod inspect;
mod lookup;
mod settings;
mod surface;

/// Glob-import surface: `use nova_console::prelude::*`.
pub mod prelude {
    pub use crate::{dispatch::execute, surface::world_line, ConsoleSystems, NovaConsolePlugin};
}

use bevy::prelude::*;
use nova_gameplay::{
    audio::prelude::{SoundBank, UiSfx, NOVA_OS_ERROR_VOLUME, NOVA_OS_OK_VOLUME},
    prelude::RunCheats,
};
use nova_os::prelude::*;
use nova_os_ui::terminal::prelude::{play_nova_os_cue, NovaOsMonitorSettings};

use crate::surface::world_line;

/// Where the dispatcher runs in a frame.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConsoleSystems {
    /// Reveal the Command shell's introduction and run whatever the prompt and
    /// the channel handed over.
    ///
    /// Ordered after [`NovaOsSystems::Input`](nova_os_ui::terminal::NovaOsSystems)
    /// produced the invocation and before
    /// [`NovaOsSystems::Simulate`](nova_os_ui::terminal::NovaOsSystems) drains
    /// the staged rows, so a command typed this frame has its answer on the
    /// screen this frame - the same promise the NOVA OS apps make.
    Dispatch,
}

/// Runs the Command shell: its introduction, and every command the prompt or
/// the channel submits.
///
/// Added by the assembly crate after the menu plugin, because a command may
/// write the same settings resources the settings UI writes.
pub struct NovaConsolePlugin;

impl Plugin for NovaConsolePlugin {
    fn build(&self, app: &mut App) {
        use nova_os_ui::terminal::NovaOsSystems;

        app.init_resource::<CommandChannel>();
        app.configure_sets(
            Update,
            ConsoleSystems::Dispatch
                .after(NovaOsSystems::Input)
                .before(NovaOsSystems::Simulate),
        );
        app.add_systems(Update, run_command_shell.in_set(ConsoleSystems::Dispatch));
    }
}

/// Reveal the introduction when it is due, then run what is queued.
///
/// Exclusive because a command may touch anything: it reads ships and sections,
/// writes settings resources, and `scenario load` triggers a whole reload.
fn run_command_shell(world: &mut World) {
    reveal_command_intro(world);
    run_pending_commands(world);
}

/// Stage the Command shell's introduction on first entry, and again after a
/// `clear` or a fresh scenario re-armed it.
///
/// The rows are built here rather than in `nova_os` because the `WORLD` row and
/// the cheat banner are live state. The staging, the timing and the
/// skip-on-input are the emulator's, and are shared with the NOVA OS welcome.
fn reveal_command_intro(world: &mut World) {
    let Some(terminal) = world.get_resource::<NovaOsTerminal>() else {
        return;
    };
    if terminal.active_shell() != ShellKind::Commands || terminal.is_revealed(ShellKind::Commands) {
        return;
    }
    let armed = world
        .get_resource::<RunCheats>()
        .is_some_and(|cheats| cheats.is_armed());
    let rows = command_intro_rows(&world_line(world), armed);
    world
        .resource_mut::<NovaOsTerminal>()
        .begin_reveal(ShellKind::Commands, rows);
}

/// Run every command the prompt and the channel have handed over.
fn run_pending_commands(world: &mut World) {
    let typed = world
        .get_resource_mut::<NovaOsTerminal>()
        .and_then(|mut terminal| terminal.take_pending_command());
    if let Some(invocation) = typed {
        let result = dispatch::execute(world, &invocation);
        answer_the_shell(world, &result);
    }

    let queued = world
        .get_resource_mut::<CommandChannel>()
        .map(|mut channel| channel.drain_pending())
        .unwrap_or_default();
    for (source, invocation) in queued {
        let result = dispatch::execute(world, &invocation);
        match source {
            CommandSource::Shell => answer_the_shell(world, &result),
            CommandSource::Channel { .. } => {
                if let Some(mut channel) = world.get_resource_mut::<CommandChannel>() {
                    channel.answer(source, result);
                }
            }
        }
    }
}

/// Put one answer on the screen: its rows, its shell control, and its cue.
fn answer_the_shell(world: &mut World, result: &CommandResult) {
    let Some(mut terminal) = world.get_resource_mut::<NovaOsTerminal>() else {
        return;
    };
    match result.command.as_str() {
        // `clear` restores the introduction rather than printing rows, so it
        // re-arms the reveal and lets the next frame stage it against the world
        // as it is NOW - which is the point of clearing after loading a
        // scenario.
        "clear" => {
            terminal.replace_scrollback(Vec::new());
            terminal.rearm_command_intro();
        }
        "close" => terminal.request_close(),
        _ => terminal.extend_scrollback(result.rows.clone()),
    }
    cue_the_answer(world, result.status);
}

/// The CRT's ok/error chirp for a dispatched command.
///
/// Cued here rather than at submit time because a command's answer lands after
/// the world has been touched: the note follows the outcome, not the keystroke.
fn cue_the_answer(world: &mut World, status: CommandStatus) {
    let (cue, volume) = match status {
        CommandStatus::Ok => (UiSfx::NovaOsOk, NOVA_OS_OK_VOLUME),
        // A refusal is as wrong a note as an error: the command was understood
        // and still did nothing.
        CommandStatus::Refused | CommandStatus::Error => (UiSfx::NovaOsError, NOVA_OS_ERROR_VOLUME),
    };
    let Some(bank) = world.get_resource::<SoundBank<UiSfx>>().cloned() else {
        return;
    };
    let settings = world
        .get_resource::<NovaOsMonitorSettings>()
        .copied()
        .unwrap_or_default();
    let mut queue = world.commands();
    play_nova_os_cue(&mut queue, &bank, &settings, cue, volume);
    world.flush();
}
