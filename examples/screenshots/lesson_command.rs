//! lesson_command: the two NOVA COMMANDS demonstrations - `command_open` (`:`
//! bringing the monitor up over flight, then Escape returning to the same
//! flight) and `command_prompt` (half a ship id typed at the `cmd>` prompt, Tab
//! completing it from the live world, and the answer printing under it).
//!
//! One producer, two sheets, because they are one session at one keyboard: the
//! first sheet raises the monitor over the flight and puts it away again, and
//! the second raises it once more and types at the prompt it opens on.
//!
//! ## Both are ACTION loops, and the camera never moves
//!
//! What is being demonstrated here is a KEY DOING SOMETHING - `:` dropping a
//! monitor over the flight, Escape putting it back, Tab turning half an id into
//! a whole one, Enter turning a line into a printed answer. That is the
//! definition of an action loop in `shared/lesson.rs`: the eye holds still and
//! the SCREEN moves. There is no [`LessonSweep`](lesson::LessonSweep) here at
//! all: the modal freezes the game (`PauseStates::Commands` holds the clocks
//! through `FreezeOwner::Interface`) and hands the window to a raster that does
//! not care where the world camera stands. Nothing behind the glass can drift,
//! so nothing has to be pinned - and `command_open` needs the flight on either
//! side of the modal to be the SAME flight, which a held camera and frozen
//! bodies give it for free.
//!
//! ## Why the range is the bare one
//!
//! `computer::interface_range`, not the rock hollow. The claim the prompt sheet
//! ends on is an id being completed, and this range has exactly one ship to
//! complete to - `player_ship` - so `ship pla` has one match and Tab takes it
//! whole. A range with traffic on it (`plot_raider`, `plot_tender`) would only
//! complete as far as the shared `pl` and the sheet would show a list rather
//! than the completion.
//!
//! ## What is typed and what is pressed
//!
//! `:` is read as the CHARACTER a player types, so it goes through
//! `type_text` and is the gesture whatever layout puts a colon on. Tab is an
//! EDIT the prompt reads off a `KeyboardInput` message
//! (`nova_interface/src/terminal/input.rs`), so it goes through
//! `press_edit_key(Key::Tab)`, which sends the release with the press; a Tab
//! that only moved the `ButtonInput<KeyCode>` table would complete nothing.
//! Escape is the button-table key the one back-out owner reads, so it is
//! `press_key` with its own `release_key` beat - a press left down would hold
//! Escape for the rest of the walk.
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
//!   cargo run --example lesson_command --features debug
//! ```

#[path = "shared/computer.rs"]
mod computer;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
use computer::interface_range;
#[cfg(feature = "debug")]
use computer::{press_escape, the_shell_is_closed, type_word};
#[cfg(feature = "debug")]
use lesson::{lesson_profile, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_command")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's NOVA COMMANDS demonstrations", long_about = None)]
struct Cli;

/// The sheet for "Opening NOVA COMMANDS".
#[cfg(feature = "debug")]
const OPEN_LESSON: &str = "command_open";
/// The sheet for "The prompt".
#[cfg(feature = "debug")]
const PROMPT_LESSON: &str = "command_prompt";

/// Cells of the open sheet that show the flight before `:` goes down.
///
/// The lesson's claim is that the monitor drops over the SAME flight it then
/// returns to, so the sheet carries the flight before as well as after.
#[cfg(feature = "debug")]
const OPEN_LEAD_CELLS: u32 = 3;

/// Cells the open sheet holds the raised monitor before Escape goes down.
#[cfg(feature = "debug")]
const OPEN_HOLD_CELLS: u32 = 4;

/// Cells the prompt sheet opens on before the first character is typed - the
/// introduction and an empty `cmd>` line, so the sheet carries the BEFORE as
/// well as the after.
#[cfg(feature = "debug")]
const PROMPT_LEAD_CELLS: u32 = 2;

/// Cells each typing beat holds before the next key goes down: long enough for
/// a line of green type to be read rather than glimpsed.
#[cfg(feature = "debug")]
const BEAT_CELLS: u32 = 3;

/// The half an id the sheet types, and the whole one Tab completes it to.
///
/// `plot_raider` and friends are not in this range, so the stem has ONE match
/// (see the module docs) and Tab lands on it in a single press.
#[cfg(feature = "debug")]
const ID_STEM: &str = "ship pla";
#[cfg(feature = "debug")]
const ID_WHOLE: &str = "ship player_ship";

/// The scrollback revision the last submitted line started from, so
/// [`the_shell_answered_since`] can tell this line's output from the last one's.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct AnswerBaseline(u64);

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    // NovaMenuPlugin explicitly: the `:` gesture lives in nova_menu, and
    // `with_game_plugins` turns the menu plugin off. `:` IS the subject of this
    // producer, so the plugin has to be here; it stands alone in a slim app,
    // and the boot-into-MainMenu handoff it normally owns is not taken (this
    // run goes Loading -> Playing).
    let mut app = AppBuilder::new()
        .with_game_plugins((custom_plugin, NovaMenuPlugin))
        .build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            lesson_profile(),
        ));
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(command_script());
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
    commands.trigger(LoadScenario(interface_range(&game_assets, &sections)));
}

/// Advance once the pause axis is at `state`.
#[cfg(feature = "debug")]
fn the_pause_state_is(
    state: PauseStates,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<State<PauseStates>>(move |pause| *pause.get() == state)
}

/// Advance once the modal's introduction has finished revealing itself, so the
/// sheet opens on a prompt that is ready to be typed at.
#[cfg(feature = "debug")]
fn the_introduction_has_landed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    use nova_protocol::nova_interface::nova_command::prelude::CommandTerminal;
    resource_where::<CommandTerminal>(|terminal| {
        terminal.is_revealed() && !terminal.has_pending_boot_rows()
    })
}

/// Mark the scrollback and submit the line, so [`the_shell_answered_since`]
/// answers for THIS line.
#[cfg(feature = "debug")]
fn submit_and_mark(world: &mut World) {
    use nova_protocol::nova_interface::nova_command::prelude::CommandTerminal;
    let before = world
        .get_resource::<CommandTerminal>()
        .map_or(0, |terminal| terminal.scrollback_revision());
    world.insert_resource(AnswerBaseline(before));
    press_edit_key(bevy::input::keyboard::Key::Enter)(world);
}

/// Advance once the shell has PRINTED something since [`submit_and_mark`] and
/// has no command left to run - the honest end of "run a command".
///
/// The revision, not the row count: an answer that scrolls the oldest rows off
/// the top would leave the count unchanged.
#[cfg(feature = "debug")]
fn the_shell_answered_since() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    use nova_protocol::nova_interface::nova_command::prelude::CommandTerminal;
    std::sync::Arc::new(|world: &World| {
        let before = world
            .get_resource::<AnswerBaseline>()
            .map_or(0, |mark| mark.0);
        world
            .get_resource::<CommandTerminal>()
            .is_some_and(|terminal| {
                terminal.scrollback_revision() > before && !terminal.has_pending_command()
            })
    })
}

/// Raise the monitor over the flight, put it away, then raise it again and type
/// at it.
#[cfg(feature = "debug")]
fn command_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the range")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle the range")
        .until(elapsed(2.0))
        .add()
        // OPENING NOVA COMMANDS: the instruments are up, because the cell
        // before the key has to look like flying rather than like a blank sky.
        // The status bar carries the build's own commit and the capture rig's
        // frame rate, and it draws OVER the monitor, so it goes with them.
        .step("raise the instruments and drop the status bar")
        .on_enter(|world: &mut World| {
            if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
                *hud = HudVisibility::On;
            }
            hide_status_bar(world);
        })
        .until(elapsed(0.5))
        .add()
        .step("open the sheet on the flight")
        .on_enter(|world: &mut World| sheet_start(world, OPEN_LESSON, LESSON_GRID))
        .until(frames(OPEN_LEAD_CELLS))
        .add()
        // Inside the recording, so the bloom is cells of the sheet rather than
        // something that happened before it.
        .step("type the colon")
        .on_enter(type_text(":"))
        .until(the_pause_state_is(PauseStates::Commands))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("hold the raised monitor")
        .until(frames(OPEN_HOLD_CELLS))
        .add()
        .step("press Escape")
        .on_enter(press_escape)
        .until(frames(1))
        .add()
        .step("let Escape up and hold the flight it returned to")
        .on_enter(release_key(KeyCode::Escape))
        .until(sheet_written(OPEN_LESSON))
        .deadline(60.0)
        .add()
        // A sheet that closed over a monitor still on the way out would show
        // an Escape that put nothing back, so this fails the run rather than
        // ships.
        .step("the flight is back")
        .until(and(
            the_pause_state_is(PauseStates::Unpaused),
            the_shell_is_closed(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // THE PROMPT: raised outside the recording, so the sheet opens on a
        // finished introduction and an empty line.
        .step("raise the monitor again")
        .on_enter(type_text(":"))
        .until(the_pause_state_is(PauseStates::Commands))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let the monitor come up and the introduction land")
        .until(and(nova_os_raster_open(), the_introduction_has_landed()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("open the prompt sheet on the empty line")
        .on_enter(|world: &mut World| sheet_start(world, PROMPT_LESSON, LESSON_GRID))
        .until(frames(PROMPT_LEAD_CELLS))
        .add()
        .step("type half an id and hold it")
        .on_enter(|world: &mut World| type_word(world, ID_STEM))
        .until(and(nova_os_command_line_reads(ID_STEM), frames(BEAT_CELLS)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Completion is an EDIT, so Tab goes down the keyboard-message path the
        // prompt actually reads (see the module docs).
        .step("Tab completes the id from the live world")
        .on_enter(press_edit_key(bevy::input::keyboard::Key::Tab))
        .until(and(
            nova_os_command_line_reads(ID_WHOLE),
            frames(BEAT_CELLS),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("run it and hold the answer")
        .on_enter(submit_and_mark)
        .until(sheet_written(PROMPT_LESSON))
        .deadline(60.0)
        .add()
        // A sheet that closed over a line nobody submitted would be twenty
        // cells of an answer it never shows, so the run ends on the shell's own
        // record that it printed one.
        .step("the shell answered")
        .until(the_shell_answered_since())
        .deadline(STEP_DEADLINE_SECS)
        .add()
}
