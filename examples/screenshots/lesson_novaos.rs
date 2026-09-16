//! lesson_novaos: the three NOVA OS demonstrations - `novaos_open` (the
//! monitor coming up over the flight), `novaos_view` (the schematic turning
//! and then snapping back to its framing) and `novaos_contacts` (the local map
//! with one contact picked).
//!
//! One producer, three frames, because they are one session at one machine:
//! the monitor is raised over the cockpit, the ship app is launched and turned
//! and reset, and the map app is opened and a contact cycled onto. Nothing is
//! staged twice, and each frame is recorded where the one before it left the
//! screen.
//!
//! The set is `hollow::duel_hollow`: the player, one hostile on station, and
//! the rock shell around them.
//!
//! ## Two action loops, and why neither one moves its camera
//!
//! Both loops are the ACTION kind (see `shared/lesson.rs`): what moves in the
//! cell is the SCREEN. `novaos_open` opens on the flight, takes the key press
//! inside the recording and lets the raster bloom on across the rest of the
//! sheet; `novaos_view` opens on the schematic at its home framing, turns it,
//! and then puts it back. A camera drifting under either one would be a second
//! moving thing arguing with the subject, and both wrap cleanly with the eye
//! held still.
//!
//! The bodies are FROZEN for the whole walk. The lesson's own sentence is that
//! the ship keeps flying while the screen is open, but the thing being
//! photographed is the screen, and an ambush going on behind the glass is
//! motion the sheet cannot hold: the loop would jump-cut on every wrap.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole session, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the two sheets and write the
//!   still (staged under `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_novaos --features debug
//! ```

#[cfg(feature = "debug")]
#[path = "shared/computer.rs"]
mod computer;
#[path = "shared/hollow.rs"]
mod hollow;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use computer::{press_enter, press_escape, type_word};
#[cfg(feature = "debug")]
use lesson::{lesson_profile, sweep_lesson_camera, LessonSweep, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_novaos")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's NOVA OS demonstrations", long_about = None)]
struct Cli;

/// The sheet for "Opening NOVA OS".
#[cfg(feature = "debug")]
const OPEN_LESSON: &str = "novaos_open";
/// The sheet for "Turning the model".
#[cfg(feature = "debug")]
const VIEW_LESSON: &str = "novaos_view";
/// The still for "Reading contacts".
#[cfg(feature = "debug")]
const CONTACTS_SHOT: &str = "novaos_contacts.png";

/// What the cockpit view looks at: a point far down the player's own bearing,
/// so the camera sits over its shoulder and the sheet opens on the view a
/// player reaches for the key from. The same device the radar lesson frames
/// with, and for the same reason.
#[cfg(feature = "debug")]
const COCKPIT_SUBJECT: Meters3 = Meters3::new(0.0, 6.0, -250.0);
/// How far the camera stands off that point - about 220 m behind the player,
/// four times the clad corvette's bounding radius, so the hull reads whole
/// under the monitor as it comes up.
#[cfg(feature = "debug")]
const COCKPIT_RANGE: Meters = Meters(460.0);
/// How far above the subject the camera rides: a look DOWN on the player
/// rather than up its exhaust.
#[cfg(feature = "debug")]
const COCKPIT_HEIGHT: Meters = Meters(70.0);
/// Centred off the player's shoulder rather than straight down its spine.
#[cfg(feature = "debug")]
const COCKPIT_BEARING_DEGREES: f32 = 6.0;

/// Cells of the open sheet that still have a clear screen.
///
/// The lesson's claim is what the KEY does, so the sheet has to carry the
/// before as well as the after: four cells of the flight, then the raster
/// blooming on and standing there.
#[cfg(feature = "debug")]
const OPEN_LEAD_CELLS: u32 = 4;

/// Cells of the view sheet before the model starts turning, and cells it turns
/// for.
///
/// The rest of the sheet is the reset and what the reset left. The reframe is
/// a SNAP, not an ease (`novaos_reframe` writes the home angles straight into
/// the orbit rig), so the cells after it have to be long enough to read as a
/// framing that came back rather than as a dropped frame.
#[cfg(feature = "debug")]
const VIEW_LEAD_CELLS: u32 = 2;
#[cfg(feature = "debug")]
const VIEW_TURN_CELLS: u32 = 12;

/// Where the camera stands for both recordings.
///
/// A [`LessonSweep`] with NO ARC: these are action loops, so the camera holds
/// (see `shared/lesson.rs`). The sweep type still drives it, because the
/// scenario camera eases back toward its own target and so the pose has to be
/// written every frame; a zero arc makes every frame of that path the same
/// place.
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
        app.add_plugins(novaos_script());
        // Only on a capture run, like the other posed hollow producers: a
        // plain `cargo run` is the owner flying the ambush with the computer
        // to hand, and freezing that would be a strange way to read a screen.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
        app.add_systems(Update, sweep_lesson_camera);
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.add_systems(
                Update,
                hollow::pin_player.run_if(resource_exists::<hollow::HoldStation>),
            );
        }
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    ships: Res<GameShipDesigns>,
) {
    commands.trigger(LoadScenario(hollow::duel_hollow(
        &game_assets,
        &sections,
        &ships,
    )));
}

/// Raise the monitor, turn the schematic, then read the map.
#[cfg(feature = "debug")]
fn novaos_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the hollow")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle the hollow on station")
        .on_enter(hollow::hold_station)
        .until(elapsed(2.0))
        .add()
        // OPENING NOVA OS: the instruments are up, because the cell before the
        // key press has to look like flying rather than like a blank sky.
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
        // Inside the recording, so the bloom is cells of the sheet rather than
        // something that happened before it.
        .step("press the NOVA OS key")
        .on_enter(press_action("novaos_toggle"))
        .until(frames(1))
        .add()
        .step("let the key up and let the raster bloom")
        .on_enter(release_action("novaos_toggle"))
        .until(sheet_written(OPEN_LESSON))
        .deadline(60.0)
        .add()
        // A sheet that closed before the monitor was flush would show a key
        // press that opened nothing, so this fails the run rather than ships.
        .step("the monitor is up")
        .until(nova_os_raster_open())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // TURNING THE MODEL, in the ship app.
        .step("type the ship command")
        .on_enter(|world: &mut World| type_word(world, "ship"))
        .until(nova_os_command_line_reads("ship"))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("launch the ship app")
        .on_enter(press_enter)
        .until(nova_os_app_owns_the_screen("ship"))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Owning the screen is not being STILL: the app's offscreen schematic
        // builds and settles over frames, which is render work.
        .step("settle the schematic")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("open the sheet on the home framing")
        .on_enter(|world: &mut World| sheet_start(world, VIEW_LESSON, LESSON_GRID))
        .until(frames(VIEW_LEAD_CELLS))
        .add()
        .step("turn the model")
        .on_enter(press_action("novaos_orbit_right"))
        .until(frames(VIEW_TURN_CELLS))
        .add()
        .step("let go and put the view back")
        .on_enter(|world: &mut World| {
            release_action("novaos_orbit_right")(world);
            press_action("novaos_reframe")(world);
        })
        .until(frames(1))
        .add()
        .step("let the reset key up and hold the framing")
        .on_enter(release_action("novaos_reframe"))
        .until(sheet_written(VIEW_LESSON))
        .deadline(60.0)
        .add()
        // READING CONTACTS, in the map app. Escape leaves the ship app for the
        // prompt, and the command line is typed in the same beat - the shell
        // takes a whole word in one frame, so there is nothing to wait between.
        .step("type the map command")
        .on_enter(|world: &mut World| {
            press_escape(world);
            type_word(world, "map");
        })
        .until(nova_os_command_line_reads("map"))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("launch the map app")
        .on_enter(press_enter)
        .until(nova_os_app_owns_the_screen("map"))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("settle the map")
        .until(frames(SETTLE_FRAMES))
        .add()
        // The still's whole subject: the map with a contact PICKED, which is
        // what puts its range and bearing under the plot.
        .step("cycle onto the contact")
        .on_enter(press_action("novaos_next"))
        .until(frames(1))
        .add()
        .step("let the cycle key up")
        .on_enter(release_action("novaos_next"))
        .until(frames(2))
        .add()
        .step("settle on the picked contact")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the contacts screen")
        .on_enter(|world: &mut World| shoot(world, CONTACTS_SHOT))
        .until(shot_written(CONTACTS_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("park the view")
        .on_enter(|world: &mut World| {
            world.remove_resource::<LessonSweep>();
        })
        .add()
}
