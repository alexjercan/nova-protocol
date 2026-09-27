//! lesson_interface: the two TAB interface demonstrations that are about the
//! interface as a whole and its Map pane - `interface_open` (the interface
//! coming up over the cockpit on its Map pane, then closing back to flight) and
//! `interface_map` (the Map pane turning over local space, one hostile picked
//! with its range and bearing in the panel beside it, then GOTO set on it).
//!
//! One producer, two sheets, because they are one session at one keyboard: the
//! first sheet opens the interface and closes it again, and the second opens it
//! once more and is recorded on the pane the first left behind (TAB reopens the
//! last pane, and the first open shows Map).
//!
//! The set is `computer::interface_plot_range`: the player, one hostile close
//! in, a friendly tender to port and a second hostile further out. The rock
//! hollow the flight lessons are shot in would put forty-eight asteroids on the
//! Map pane, each one a contact drawn at its own projected size, and the ship
//! the sheet picks would vanish under a field of white discs. The map plots
//! EVERYTHING the scenario carries, so the range the map is read on carries the
//! traffic and nothing else - and the cockpit sheet is shot on the same range,
//! because one scenario cannot be swapped for another inside a session.
//!
//! ## Two action loops, and why neither one moves its camera
//!
//! Both loops are the ACTION kind (see `shared/lesson.rs`): what moves in the
//! cell is the INTERFACE. `interface_open` opens on the flight, takes the key
//! press inside the recording, holds the Map pane and takes the key press that
//! puts the flight back; `interface_map` opens on the pane, turns it, picks a
//! contact and sets GOTO. A camera drifting under either one would be a second
//! moving thing arguing with the subject.
//!
//! The bodies are FROZEN for the whole walk, and the flight camera is posed by a
//! [`LessonSweep`](lesson::LessonSweep) with no arc: the scenario camera eases
//! back toward its own target, so the pose is written every frame, and a zero
//! arc makes every frame of that path the same place. Opening the interface
//! freezes the world anyway (`PauseStates::Interface`), so the pose only has
//! to hold for the flight cells on either side of it.
//!
//! ## Why the map walk presses `viewer_next` twice
//!
//! The first press from no selection lands on the own ship, whose panel reads
//! "That is you"; the second steps onto the next contact. The contact list has
//! no sort (`MapContacts::collect` is query order), and the two hostiles have
//! come up in either order between runs, so the sheet is about "a hostile", not
//! about one named ship. The walk does not read the selection (`MapRuntime` is
//! crate-private): it asserts on what the pane DRAWS, a `HOSTILE` panel with
//! range and bearing and then the `GOTO SET:` note, so a walk that landed on the
//! friendly tender stalls on a named beat instead of shipping the wrong
//! footage. The note, not the ship's `Autopilot`: on the smoke run the range's
//! hull handed the computer back within a frame of the order ("maneuver
//! complete" in the `nova_ship` log), so the component was gone before a beat
//! could read it.
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
//!   cargo run --example lesson_interface --features debug
//! ```

#[path = "shared/computer.rs"]
mod computer;
#[path = "shared/hollow.rs"]
mod hollow;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
use computer::interface_plot_range;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, sweep_lesson_camera, LessonSweep, LESSON_GRID};
#[cfg(feature = "debug")]
use nova_protocol::nova_interface::pane::InterfacePaneType;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_interface")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's interface and Map pane demonstrations", long_about = None)]
struct Cli;

/// The sheet for "Opening the interface".
#[cfg(feature = "debug")]
const OPEN_LESSON: &str = "interface_open";
/// The sheet for "The Map pane".
#[cfg(feature = "debug")]
const MAP_LESSON: &str = "interface_map";

/// What the cockpit view looks at: a point far down the player's own bearing,
/// so the camera sits over its shoulder and the sheet opens on the view a
/// player reaches for the key from. The same device the radar lesson frames
/// with, and for the same reason.
#[cfg(feature = "debug")]
const COCKPIT_SUBJECT: Meters3 = Meters3::new(0.0, 6.0, -250.0);
/// How far the camera stands off that point - about 220 m behind the player,
/// four times the clad corvette's bounding radius, so the hull reads whole
/// before the interface covers it.
#[cfg(feature = "debug")]
const COCKPIT_RANGE: Meters = Meters(460.0);
/// How far above the subject the camera rides: a look DOWN on the player
/// rather than up its exhaust.
#[cfg(feature = "debug")]
const COCKPIT_HEIGHT: Meters = Meters(70.0);
/// Centred off the player's shoulder rather than straight down its spine.
#[cfg(feature = "debug")]
const COCKPIT_BEARING_DEGREES: f32 = 6.0;

/// Cells of the open sheet that show the flight before the key goes down.
///
/// The lesson's claim is what the KEY does, so the sheet carries the before as
/// well as the after.
#[cfg(feature = "debug")]
const OPEN_LEAD_CELLS: u32 = 3;
/// Cells the open sheet holds the Map pane before the key goes down again.
///
/// The sheet is twenty cells: the lead, the two key beats and this hold leave
/// the last handful of cells on the flight the interface closed back to.
#[cfg(feature = "debug")]
const OPEN_HOLD_CELLS: u32 = 6;

/// Cells of the map sheet before the pane starts turning.
#[cfg(feature = "debug")]
const MAP_LEAD_CELLS: u32 = 1;
/// Cells the turn key is held for.
///
/// The viewer turns on `Time<Real>`, and an armed capture pins that clock at
/// one cell per frame, so this is a fixed stretch of real time and a fixed
/// angle on every capture. The angle itself lives in the crate-private map
/// orbit, so the walk holds for the time rather than reading the angle.
#[cfg(feature = "debug")]
const MAP_TURN_CELLS: u32 = 3;
/// Cells the picked hostile's panel holds before GOTO is pressed. The map
/// EASES its framing onto a new selection, so the plot is still sliding for
/// the first of them.
#[cfg(feature = "debug")]
const MAP_PICK_CELLS: u32 = 3;

/// Where the camera stands for the flight cells of the open sheet.
///
/// A [`LessonSweep`] with NO ARC: these are action loops, so the camera holds
/// (see `shared/lesson.rs`).
#[cfg(feature = "debug")]
fn cockpit() -> LessonSweep {
    LessonSweep::new(
        COCKPIT_SUBJECT,
        COCKPIT_RANGE,
        COCKPIT_HEIGHT,
        COCKPIT_BEARING_DEGREES,
        0.0,
    )
}

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
        app.add_plugins(interface_script());
        // Only on a capture run, like the other posed producers: a plain
        // `cargo run` is the owner flying the range with the interface to
        // hand, and freezing that would be a strange way to read a screen.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
        app.add_systems(Update, sweep_lesson_camera);
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(interface_plot_range(&game_assets, &sections)));
}

/// Advance once the interface is open on `pane`.
#[cfg(feature = "debug")]
fn the_interface_shows(
    pane: InterfacePaneType,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            && world
                .get_resource::<InterfacePaneType>()
                .is_some_and(|shown| *shown == pane)
    })
}

/// Advance once the flight has the screen back and the clocks are running.
#[cfg(feature = "debug")]
fn the_flight_is_back() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<State<PauseStates>>(|pause| *pause.get() == PauseStates::Unpaused)
}

/// Advance once any line of on-screen text satisfies `test`.
///
/// The pane's own panel, read the way a player reads it: the selection and
/// the GOTO note live in crate-private state, and the text they draw is the
/// one place both are observable.
#[cfg(feature = "debug")]
fn the_screen_reads(
    test: fn(&str) -> bool,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .try_query::<&Text>()
            .is_some_and(|mut texts| texts.iter(world).any(|text| test(&text.0)))
    })
}

/// The kind line the contact panel draws for a picked hostile.
#[cfg(feature = "debug")]
fn is_the_hostile_kind(line: &str) -> bool {
    line == "HOSTILE"
}

/// The bearing line the contact panel draws for a picked contact that is not
/// the own ship. The range line fills with it.
#[cfg(feature = "debug")]
fn is_a_bearing(line: &str) -> bool {
    line.starts_with("Bearing ") && line != "Bearing ---"
}

/// The note the pane flashes once GOTO is set on a contact.
#[cfg(feature = "debug")]
fn is_the_goto_note(line: &str) -> bool {
    line.starts_with("GOTO SET: ")
}

/// Open the interface over the flight, hold the map, close it, then read the
/// map and set GOTO.
#[cfg(feature = "debug")]
fn interface_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the plot range")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle the range")
        .until(elapsed(2.0))
        .add()
        // OPENING THE INTERFACE: the instruments are up, because the cell
        // before the key press has to look like flying rather than like a
        // blank sky.
        .step("raise the instruments and frame the cockpit")
        .on_enter(|world: &mut World| {
            hollow::hud_instrument(world);
            world.insert_resource(cockpit());
        })
        .until(elapsed(0.5))
        .add()
        .step("open the sheet on the flight")
        .on_enter(|world: &mut World| sheet_start(world, OPEN_LESSON, LESSON_GRID))
        .until(frames(OPEN_LEAD_CELLS))
        .add()
        // Inside the recording, so the interface coming up is cells of the
        // sheet rather than something that happened before it.
        .step("press the interface key")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        // A sheet that closed before the pane was up would show a key press
        // that opened nothing, so this waits on the pane rather than on a
        // frame count alone.
        .step("let the key up and hold the map pane")
        .on_enter(release_action("interface_toggle"))
        .until(and(
            the_interface_shows(InterfacePaneType::Map),
            frames(OPEN_HOLD_CELLS),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("press the interface key again")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("let the key up and hold the flight")
        .on_enter(release_action("interface_toggle"))
        .until(sheet_written(OPEN_LESSON))
        .deadline(60.0)
        .add()
        .step("the flight is back")
        .until(the_flight_is_back())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // THE MAP PANE: opened outside the recording, so the sheet starts on a
        // pane that has already built its scene.
        .step("open the interface on the map again")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("let the key up and wait for the map")
        .on_enter(release_action("interface_toggle"))
        .until(the_interface_shows(InterfacePaneType::Map))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // A shown pane is not a STILL one: its offscreen scene builds and
        // settles over frames, which is render work.
        .step("settle the map")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("open the sheet on the map")
        .on_enter(|world: &mut World| sheet_start(world, MAP_LESSON, LESSON_GRID))
        .until(frames(MAP_LEAD_CELLS))
        .add()
        .step("turn the map")
        .on_enter(press_action("viewer_orbit_right"))
        .until(frames(MAP_TURN_CELLS))
        .add()
        // The first press lands on the own ship (see the module docs).
        .step("let go and step past the own ship")
        .on_enter(|world: &mut World| {
            release_action("viewer_orbit_right")(world);
            press_action("viewer_next")(world);
        })
        .until(frames(1))
        .add()
        .step("let the select key up")
        .on_enter(release_action("viewer_next"))
        .until(frames(1))
        .add()
        .step("step onto the next contact")
        .on_enter(press_action("viewer_next"))
        .until(frames(1))
        .add()
        .step("let the select key up and hold the hostile's panel")
        .on_enter(release_action("viewer_next"))
        .until(and(
            and(
                the_screen_reads(is_the_hostile_kind),
                the_screen_reads(is_a_bearing),
            ),
            frames(MAP_PICK_CELLS),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("press GOTO")
        .on_enter(press_action("map_goto"))
        .until(frames(1))
        .add()
        .step("let the key up and hold the GOTO note")
        .on_enter(release_action("map_goto"))
        .until(sheet_written(MAP_LESSON))
        .deadline(60.0)
        .add()
        // A sheet that closed over a GOTO that never engaged would be twenty
        // cells of a claim it does not make, so the run ends on the pane's own
        // note.
        .step("the pane reports GOTO set")
        .until(the_screen_reads(is_the_goto_note))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("park the view")
        .on_enter(|world: &mut World| {
            world.remove_resource::<LessonSweep>();
        })
        .add()
}
