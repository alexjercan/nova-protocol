//! lesson_novaos_prompt: the two NOVA OS demonstrations that are about the
//! PROMPT rather than about an app - `novaos_terminal` (Tab completion: the
//! ghost suffix, the candidate row, the cycle onto the next match, and the
//! live section code an argument wants) and `novaos_commands` (`help`
//! printing the registered command list).
//!
//! One producer, two sheets, because they are one session at one keyboard: the
//! monitor comes up once, the first sheet types at the prompt it opens on, and
//! the second is recorded on the line the first left behind.
//!
//! ## Both are ACTION loops, and the camera never moves
//!
//! What is being demonstrated here is a KEY DOING SOMETHING - Tab turning
//! `ship re` into `ship reload`, Tab again stepping onto `ship repair`, Enter
//! turning one word into a printed table. That is the definition of an action
//! loop in `shared/lesson.rs`: the eye holds still and the SCREEN moves. There
//! is no [`LessonSweep`](lesson::LessonSweep) here at all, not even a zero-arc
//! one, because NOVA OS freezes the game (`PauseStates::NovaOs` stops
//! `Time<Virtual>` and `Time<Physics>`) and hands the whole window to a raster
//! that does not care where the world camera stands. Nothing behind the glass
//! can drift, so nothing has to be pinned.
//!
//! Both sheets were authored as STILLS. A still of a prompt mid-line is a
//! picture of a ghost suffix with no account of where it came from or what
//! accepting it does, and a still of a printed table cannot say that the table
//! is what `help` ANSWERED - so both were flipped to loops
//! (`crates/nova_authoring/src/base_content/lessons.rs`) and their alt text
//! rewritten to describe the footage.
//!
//! ## Why the range is the bare one
//!
//! `computer::nova_os_range`, not the rock hollow. The claim the terminal
//! sheet ends on is a SUBCOMMAND being completed, and the ship this range
//! flies is the one whose sections give `ship` something to complete against -
//! a turret and a torpedo tube on its flanks as well as a spine. Nothing else
//! is in the scenario, which matters less here than it does for the map (see
//! `lesson_novaos_contacts`) but costs nothing either.
//!
//! ## What did not work
//!
//! Tab through `press_tab` (`shared/computer.rs`), which presses
//! `ButtonInput<KeyCode>::Tab`. That is the key the MONITOR toggles on, not
//! the one the terminal completes on: the terminal reads `logical_key` off a
//! `KeyboardInput` message (`nova_os_ui/src/terminal/input.rs`), so a press
//! that only moves the button table completes nothing, and `press_tab` never
//! releases - it would leave Tab held for the rest of the walk.
//! `press_edit_key(Key::Tab)` is the one that reaches the completer, and it
//! sends the release with the press.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole session, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the two sheets (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_novaos_prompt --features debug
//! ```

#[path = "shared/computer.rs"]
mod computer;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

#[cfg(feature = "debug")]
use bevy::input::keyboard::Key;
use bevy::prelude::*;
use clap::Parser;
use computer::nova_os_range;
#[cfg(feature = "debug")]
use computer::type_word;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_novaos_prompt")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's NOVA OS prompt demonstrations", long_about = None)]
struct Cli;

/// The sheet for "The prompt": completion.
#[cfg(feature = "debug")]
const TERMINAL_LESSON: &str = "novaos_terminal";
/// The sheet for "What you can type": `help` and its answer.
#[cfg(feature = "debug")]
const COMMANDS_LESSON: &str = "novaos_commands";

/// Cells each typing beat holds before the next key goes down.
///
/// A step's entry takes a frame of its own and `frames(n)` is polled from the
/// next one, so a beat of three is FOUR cells - four tenths of a second, which
/// is what a line of green type needs to be read rather than glimpsed. The
/// arithmetic is the whole shape of the sheet: a three-cell lead plus three
/// beats of four is fifteen, and the last act holds the remaining five.
///
/// The first cut ran five beats at this length and the sheet closed one Tab
/// early: the cycle the lesson is about never reached a cell. Count the cells
/// before adding a beat.
#[cfg(feature = "debug")]
const BEAT_CELLS: u32 = 3;

/// Cells the terminal sheet opens on before the first character is typed - the
/// boot report and an empty `nova>` line, so the sheet carries the BEFORE as
/// well as the after.
#[cfg(feature = "debug")]
const PROMPT_LEAD_CELLS: u32 = 2;

/// Cells the commands sheet holds the typed word before Enter.
///
/// Longer than [`BEAT_CELLS`], because this sheet has only two beats in it and
/// the reader has to see that `help` was TYPED before the table arrives.
#[cfg(feature = "debug")]
const HELP_HOLD_CELLS: u32 = 6;

/// Backspaces that clear whatever the terminal sheet left on the line.
///
/// `ship repair CTL-1` is seventeen characters and the extras are free - a
/// backspace on an empty line is a no-op in the editor model
/// (`nova_os/src/terminal/edit.rs`). Sixteen was the first figure, measured
/// against `ship repair` alone, and the completed argument put the line one
/// character past it: the beat then stalled on a prompt it could not empty.
/// Count against the LONGEST line the sheet can leave, with room to spare.
#[cfg(feature = "debug")]
const LINE_CLEAR: usize = 32;

/// The stem the sheet types, and the two subcommands Tab steps through.
///
/// `completion_matches` SORTS before it dedups, so `ship reload` is always the
/// first match and `ship repair` always the second - the cycle is the same on
/// every capture. The first Tab on an ambiguous stem also PRINTS the matches
/// as a dim row above the prompt, so the sheet shows the candidate list and
/// the jump in the same act.
#[cfg(feature = "debug")]
const SUBCOMMAND_STEM: &str = "ship re";
#[cfg(feature = "debug")]
const SUBCOMMAND_FIRST: &str = "ship reload";
#[cfg(feature = "debug")]
const SUBCOMMAND_SECOND: &str = "ship repair";

/// What the last act types before its Tab: the space that starts the ARGUMENT
/// word.
///
/// This is the half of the claim no command list can show - completion offers
/// the LIVE section codes of the ship you are flying, which the ship app
/// publishes as it spawns (`sync_ship_arg_completions`). The range's five
/// sections sort with `CTL-1` first, so the Tab lands there.
#[cfg(feature = "debug")]
const ARGUMENT_STEM: &str = "ship repair ";

/// What the second sheet types.
#[cfg(feature = "debug")]
const HELP_COMMAND: &str = "help";

/// What wipes the first sheet's leavings between the two recordings. The
/// lesson the second sheet carries names this command, so the walk uses the
/// thing it is about rather than reloading the range.
#[cfg(feature = "debug")]
const CLEAR_COMMAND: &str = "clear";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            lesson_profile(),
        ));
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(novaos_prompt_script());
        // The range is parked and the monitor is the subject, so nothing in it
        // may drift between the two sheets - but only on a capture run, so a
        // plain `cargo run` is the range as it flies.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(nova_os_range(&game_assets, &sections)));
}

/// Clear the command line back to an empty prompt.
#[cfg(feature = "debug")]
fn clear_the_line(world: &mut World) {
    for _ in 0..LINE_CLEAR {
        press_edit_key(Key::Backspace)(world);
    }
}

/// Advance once the prompt holds `ship repair` plus an argument - the section
/// code the completer filled in.
///
/// The CODE itself is not named: which one sorts first is the range's business
/// and would go stale the day a section is added to it. What the lesson claims
/// is that a live code arrives at all, so that is what this waits for.
#[cfg(feature = "debug")]
fn the_prompt_took_a_section_code() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate>
{
    resource_where::<nova_protocol::nova_os_ui::nova_os::prelude::NovaOsTerminal>(|terminal| {
        terminal
            .prompt()
            .strip_prefix(ARGUMENT_STEM)
            .is_some_and(|code| !code.is_empty())
    })
}

/// Raise the monitor, type at the prompt, then ask it what it knows.
#[cfg(feature = "debug")]
fn novaos_prompt_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    // Completion is an EDIT, so every Tab goes down the keyboard-message path
    // the terminal actually reads. See the module docs for what the button
    // table's Tab does instead.
    let tab = || press_edit_key(Key::Tab);

    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the range")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle the range")
        .until(elapsed(2.0))
        .add()
        // The status bar carries the build's own commit and the capture rig's
        // frame rate, and it draws OVER the monitor. Dropped before the key,
        // not after, so no recorded cell ever holds it.
        .step("drop the status bar and raise the monitor")
        .on_enter(|world: &mut World| {
            hide_status_bar(world);
            press_action("novaos_toggle")(world);
        })
        .until(frames(1))
        .add()
        .step("let the key up and let the raster bloom")
        .on_enter(release_action("novaos_toggle"))
        .until(nova_os_raster_open())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // THE PROMPT: a stem, the ghost it raises, Tab accepting it, then an
        // ambiguous stem and two Tabs stepping through the pair.
        .step("open the prompt sheet on the boot report")
        .on_enter(|world: &mut World| sheet_start(world, TERMINAL_LESSON, LESSON_GRID))
        .until(frames(PROMPT_LEAD_CELLS))
        .add()
        .step("type a stem two subcommands share, and hold the ghost suffix")
        .on_enter(|world: &mut World| type_word(world, SUBCOMMAND_STEM))
        .until(and(
            nova_os_command_line_reads(SUBCOMMAND_STEM),
            frames(BEAT_CELLS),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("Tab lists the matches and takes the first")
        .on_enter(tab())
        .until(and(
            nova_os_command_line_reads(SUBCOMMAND_FIRST),
            frames(BEAT_CELLS),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("Tab again steps onto the second match")
        .on_enter(tab())
        .until(and(
            nova_os_command_line_reads(SUBCOMMAND_SECOND),
            frames(BEAT_CELLS),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // The last act of the sheet: the space that starts the argument, and
        // a Tab that answers it with the ship's own section codes. The space
        // and the Tab go down in ONE beat because the cells are spent - the
        // space alone shows nothing, and the sheet closes itself at the grid's
        // own count, so this waits for the ack rather than for a number.
        .step("a space, then Tab for the live section codes")
        .on_enter(|world: &mut World| {
            type_word(world, " ");
            press_edit_key(Key::Tab)(world);
        })
        .until(sheet_written(TERMINAL_LESSON))
        .deadline(60.0)
        .add()
        // The honest end of the last act: a prompt that still read the bare
        // verb would mean the live codes never reached the completer, and the
        // sheet would be twenty cells of a claim it does not make.
        .step("the completion took a section code")
        .until(the_prompt_took_a_section_code())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // WHAT YOU CAN TYPE: the line is cleared and the scrollback wiped
        // OUTSIDE the sheet, so the second demonstration opens on the boot
        // report rather than on the first one's leavings.
        .step("clear the line")
        .on_enter(clear_the_line)
        .until(nova_os_command_line_reads(""))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("wipe the scrollback back to the boot report")
        .on_enter(|world: &mut World| {
            type_word(world, CLEAR_COMMAND);
            press_edit_key(Key::Enter)(world);
        })
        .until(nova_os_command_line_reads(""))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let the wiped screen settle")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("open the commands sheet and type help")
        .on_enter(|world: &mut World| {
            sheet_start(world, COMMANDS_LESSON, LESSON_GRID);
            type_word(world, HELP_COMMAND);
        })
        .until(and(
            nova_os_command_line_reads(HELP_COMMAND),
            frames(HELP_HOLD_CELLS),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("run it and hold the table it printed")
        .on_enter(press_edit_key(Key::Enter))
        .until(sheet_written(COMMANDS_LESSON))
        .deadline(60.0)
        .add()
        // The shell takes the line on submit, so an empty prompt is the honest
        // proof that `help` RAN - a sheet that closed over a still-typed word
        // would be twenty cells of a command nobody pressed Enter on.
        .step("the shell took the command")
        .until(nova_os_command_line_reads(""))
        .deadline(STEP_DEADLINE_SECS)
        .add()
}
