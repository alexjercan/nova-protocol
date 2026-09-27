//! The Command shell's dispatcher: the executor behind the CRT prompt and the
//! process channel.
//!
//! `nova_command` owns the LANGUAGE (the catalog, the parser, the structured
//! result) and stays a leaf, so the terminal model can be tested without a
//! game. This crate owns the EXECUTION, and therefore sits above gameplay,
//! scenario, settings and menu. That split is the whole reason a command can
//! read a live ship without `nova_command` learning what a ship is.
//!
//! Two front ends arrive here and neither can drift from the other:
//!
//! - the CRT prompt, through [`CommandTerminal::take_pending_command`];
//! - the process channel, through [`CommandChannel`].
//!
//! Both were parsed by the same
//! [`resolve_command_line`](nova_command::prelude::resolve_command_line), both run
//! through [`dispatch::execute`], and both receive the same [`CommandResult`].
#![warn(missing_docs)]

pub mod dispatch;

mod cheats;
mod completion;
mod inspect;
mod lookup;
mod settings;
mod surface;

/// Glob-import surface: `use nova_console::prelude::*`.
pub mod prelude {
    pub use crate::{dispatch::execute, surface::world_line, ConsoleSystems, NovaConsolePlugin};
}

use bevy::prelude::*;
use nova_command::prelude::*;
use nova_gameplay::{
    audio::prelude::{SoundBank, UiSfx, NOVA_OS_ERROR_VOLUME, NOVA_OS_OK_VOLUME},
    prelude::{PauseStates, RunCheats},
};
use nova_interface::terminal::prelude::{play_nova_os_cue, CommandsSystems, NovaOsMonitorSettings};

use crate::{completion::publish_live_values, surface::world_line};

/// Where the dispatcher runs in a frame.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConsoleSystems {
    /// Reveal the Command shell's introduction and run whatever the prompt and
    /// the channel handed over.
    ///
    /// Ordered after [`CommandsSystems::Input`](nova_interface::terminal::CommandsSystems)
    /// produced the invocation and before
    /// [`CommandsSystems::Simulate`](nova_interface::terminal::CommandsSystems)
    /// drains the staged rows, so a command typed this frame has its answer on
    /// the screen this frame.
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
        app.init_resource::<CommandChannel>();
        app.configure_sets(
            Update,
            ConsoleSystems::Dispatch
                .after(CommandsSystems::Input)
                .before(CommandsSystems::Simulate),
        );
        app.add_systems(
            Update,
            (
                // Exclusive and a sync point, so it runs only on the frames that
                // actually have something to run.
                run_command_shell.run_if(command_shell_has_work),
                // Only while the CRT is up: nothing can Tab at a shell that is
                // not on screen.
                publish_live_values.run_if(in_state(PauseStates::Commands)),
            )
                .in_set(ConsoleSystems::Dispatch),
        );
    }
}

/// Whether anything is waiting for the dispatcher this frame.
///
/// Peeked through `Deref`: taking the queue to look at it would flag
/// [`CommandTerminal`] as changed every frame and defeat the CRT's own
/// change-detection gates.
fn command_shell_has_work(
    terminal: Option<Res<CommandTerminal>>,
    pause: Option<Res<State<PauseStates>>>,
    next: Option<Res<NextState<PauseStates>>>,
    channel: Option<Res<CommandChannel>>,
) -> bool {
    let from_prompt = terminal.is_some_and(|terminal| {
        terminal.has_pending_command()
            || (!terminal.is_revealed()
                && (pause.is_some_and(|state| *state.get() == PauseStates::Commands)
                    || next.is_some_and(|next| {
                        matches!(*next, NextState::Pending(PauseStates::Commands))
                    })))
    });
    from_prompt || channel.is_some_and(|channel| channel.has_pending())
}

/// Reveal the introduction when it is due, then run what is queued.
///
/// Exclusive because a command may touch anything: it reads ships and sections,
/// writes settings resources, and `scenario load` triggers a whole reload.
fn run_command_shell(world: &mut World) {
    reveal_command_intro(world);
    run_pending_commands(world);
}

/// Stage the command modal's introduction when it opens unrevealed: on first
/// entry, and again after a `clear` or a fresh scenario re-armed it.
///
/// The rows are built here rather than in `nova_command` because the `WORLD` row and
/// the cheat banner are live state. The staging, the timing and the
/// skip-on-input are the terminal model's.
fn reveal_command_intro(world: &mut World) {
    let Some(terminal) = world.get_resource::<CommandTerminal>() else {
        return;
    };
    let opening = world
        .get_resource::<State<PauseStates>>()
        .is_some_and(|state| *state.get() == PauseStates::Commands)
        || world
            .get_resource::<NextState<PauseStates>>()
            .is_some_and(|next| matches!(*next, NextState::Pending(PauseStates::Commands)));
    if terminal.is_revealed() || !opening {
        return;
    }
    let armed = world
        .get_resource::<RunCheats>()
        .is_some_and(|cheats| cheats.is_armed());
    let rows = command_intro_rows(&world_line(world), armed);
    world.resource_mut::<CommandTerminal>().begin_reveal(rows);
}

/// Run every command the prompt and the channel have handed over.
///
/// The prompt's queue is drained one at a time and only while it holds
/// something: a bare `take` every frame would take `CommandTerminal` mutably and
/// flag it as changed with nothing in hand.
fn run_pending_commands(world: &mut World) {
    while world
        .get_resource::<CommandTerminal>()
        .is_some_and(CommandTerminal::has_pending_command)
    {
        let Some(invocation) = world
            .resource_mut::<CommandTerminal>()
            .take_pending_command()
        else {
            break;
        };
        let result = dispatch::execute(world, &invocation);
        answer_the_shell(world, &result);
    }

    let queued = world
        .get_resource_mut::<CommandChannel>()
        .map(|mut channel| channel.drain_pending())
        .unwrap_or_default();
    for (source, invocation) in queued {
        let result = dispatch::execute(world, &invocation);
        if let Some(mut channel) = world.get_resource_mut::<CommandChannel>() {
            channel.answer(source, result);
        }
    }
}

/// Put one answer on the screen: its rows, and its cue.
///
/// Shell control (`clear`, `close`) never arrives here: the emulator owns the
/// screen and acts on those at submit time.
fn answer_the_shell(world: &mut World, result: &CommandResult) {
    let Some(mut terminal) = world.get_resource_mut::<CommandTerminal>() else {
        return;
    };
    terminal.extend_scrollback(result.rows.clone());
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
