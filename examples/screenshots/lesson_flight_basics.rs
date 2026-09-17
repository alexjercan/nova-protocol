//! lesson_flight_basics: the three FLIGHT demonstrations that are the ship
//! itself moving - `flight_momentum` (the drive goes out and the speed does
//! not), `flight_stop` (the computer turns the ship around and brakes) and
//! `flight_rcs` (the small thrusters push the hull sideways).
//!
//! One producer, three sheets, because they are one flight: nudge the parked
//! hull with the small thrusters, then build a cruise, cut the drive, and hand
//! what is left to STOP. Each sheet is recorded in its place in that flight,
//! so nothing is staged twice and no sheet shows a state the one before it did
//! not leave behind.
//!
//! The RCS beat comes FIRST, and that is not the handbook's order. It is the
//! only beat that needs a hull which is genuinely still and still inside the
//! hollow's pocket, and the ship is both of those exactly once - before it has
//! flown anywhere. Run last, it plays out wherever the braking order happened
//! to leave the ship, which the first cut of this producer showed is out the
//! far side of the rock field with nothing in frame to move against.
//!
//! ## These are ACTION loops, shot from a chase
//!
//! All three (see `shared/lesson.rs`) are the ACTION kind: the motion in the
//! cell is the SHIP, not the camera. That is the claim in each case - a loop
//! that moved the camera instead would be a picture of a ship, and the lesson
//! is about what the ship does. They wrap the way a tutorial clip wraps, back
//! to the start to do it again.
//!
//! The eye is `LessonChase`, which holds a fixed WORLD offset from the hull,
//! and not the player's own camera: that one rides close astern, where a
//! corvette is a drive bell filling the cell and a flip is invisible because
//! the camera flips with it. Each lesson picks its own offset, because each
//! one has a different thing to show - a plume going out, a hull turning end
//! for end, a hull sliding sideways - and the offset is what decides whether
//! that motion is across the screen or straight down the lens.
//!
//! The rocks are why the set is the standard shell rather than the solo one:
//! momentum and a braking order are both READ off the background going past,
//! and a thinned field 1600 m out does not move in two seconds.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - fly the whole script, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the three sheets (staged
//!   under `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_flight_basics --features debug
//! ```

#[path = "shared/hollow.rs"]
mod hollow;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{lesson_chase_plugin, lesson_profile, LessonChase, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_flight_basics")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's momentum, STOP and RCS demonstrations", long_about = None)]
struct Cli;

/// The sheet for "You keep your speed".
#[cfg(feature = "debug")]
const MOMENTUM_LESSON: &str = "flight_momentum";
/// The sheet for "The STOP order".
#[cfg(feature = "debug")]
const STOP_LESSON: &str = "flight_stop";
/// The sheet for "Using the RCS thrusters".
#[cfg(feature = "debug")]
const RCS_LESSON: &str = "flight_rcs";

/// How long the drive is held before the momentum sheet opens.
///
/// Long enough that the speed readout is a cruise rather than a crawl, short
/// enough that the corvette does not cross the whole hollow while the sheet
/// and the braking order that follows it play out. Held for 2.5 s the ship
/// passes 200 m/s, and at that speed the rock field is a blur with a rock in
/// front of the subject in half the cells.
#[cfg(feature = "debug")]
const CRUISE_BURN_SECS: f32 = 1.8;

/// Cells of the momentum sheet that still have the drive lit.
///
/// The lesson's claim is what happens WHEN THE DRIVE GOES OUT, so the sheet
/// has to carry the before as well as the after: five cells of plume, then
/// fifteen of the same speed with the tail dark.
#[cfg(feature = "debug")]
const MOMENTUM_LEAD_CELLS: u32 = 5;

/// How long the braking order is left to run before its sheet opens.
///
/// The order does more in four seconds than a two-second sheet can hold, so
/// the sheet has to pick a half. It picks the SECOND one: the nose arriving on
/// retrograde, the drive lighting against the track, and the speed readout
/// falling. The first half is one long slow turn with nothing else happening,
/// and it ends the sheet mid-swing, which on a loop reads as the ship snapping
/// back rather than as a maneuver.
#[cfg(feature = "debug")]
const STOP_TURN_SECS: f32 = 1.5;

/// Cells of the RCS sheet before the thrusters fire, for the same reason.
#[cfg(feature = "debug")]
const RCS_LEAD_CELLS: u32 = 3;

/// Where the eye rides for the momentum sheet: off the starboard quarter and
/// a little above, about 195 m out.
///
/// Behind the beam on purpose. The lesson turns on the DRIVE going out, so the
/// tail has to be in shot; from dead abeam the plume is a sliver, and from
/// dead astern the hull is nothing but the bell.
#[cfg(feature = "debug")]
const MOMENTUM_EYE: Meters3 = Meters3::new(105.0, 28.0, 80.0);

/// Where the eye rides for the STOP sheet: square on the starboard beam.
///
/// The braking order turns the hull end for end, and a beam eye is the one
/// place that whole turn is seen at full width - the same reason the HUD
/// lesson stands off the beam to show a drift angle.
#[cfg(feature = "debug")]
const STOP_EYE: Meters3 = Meters3::new(125.0, 22.0, 0.0);

/// Where the eye rides for the RCS sheet: high above the hull, looking steeply
/// down on it.
///
/// The only framing that carries this lesson. The small thrusters move a
/// corvette about a metre a second, so over a two-second sheet the hull does
/// not visibly go anywhere, and what the cells actually carry is the velocity
/// shell turning violet in the direction of the push. That push is to
/// STARBOARD: from the starboard beam it comes at the lens, from the port beam
/// the hull hides it, and from either quarter it is a smear beside the drive.
/// From overhead the hull lies across the cell with its nose up the screen and
/// the violet cone out to the right of it - the lesson's own sentence,
/// sideways without turning, drawn.
#[cfg(feature = "debug")]
const RCS_EYE: Meters3 = Meters3::new(25.0, 140.0, 85.0);

/// The mouse motion the RCS beat feeds the ship each frame, in the pixels a
/// real mouse would report.
///
/// Sideways only (+X is starboard), and large enough to saturate the binding's
/// own gain: `on_rcs_aim` clamps the delta to one, so what this picks is not
/// the strength of the push but whether the push is at full deflection. It is,
/// because a half-hearted nudge does not read in a 960 px cell.
#[cfg(feature = "debug")]
const RCS_PUSH_PIXELS: Vec2 = Vec2::new(14.0, 0.0);

/// Present while the RCS beat is holding the mouse over.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct RcsPush;

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
        app.add_plugins(flight_basics_script());
        // NOTHING is frozen: the subject of all three lessons is the hull
        // actually moving. The chase is inert until a step inserts its offset,
        // and the mouse push until the RCS beat holds it over.
        app.add_plugins(lesson_chase_plugin);
        app.add_systems(Update, push_rcs.run_if(resource_exists::<RcsPush>));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    commands.trigger(LoadScenario(hollow::flight_hollow(&game_assets, &ships)));
}

/// Report the mouse as still moving, one frame at a time.
///
/// `on_rcs_aim` is DELTA-driven and `decay_player_rcs_intent` fades the intent
/// the moment the mouse stops, which is the production feel: force follows
/// motion. So a held push is not one message, it is a message on every frame
/// the push lasts - the same shape as the drift the HUD lesson writes.
#[cfg(feature = "debug")]
fn push_rcs(mut motion: MessageWriter<bevy::input::mouse::MouseMotion>) {
    motion.write(bevy::input::mouse::MouseMotion {
        delta: RCS_PUSH_PIXELS,
    });
}

/// Fly one flight and photograph three lessons out of it.
#[cfg(feature = "debug")]
fn flight_basics_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the hollow")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("raise the instruments and take up the chase")
        .on_enter(|world: &mut World| {
            hollow::hud_instrument(world);
            world.insert_resource(LessonChase::new(RCS_EYE));
        })
        .until(elapsed(1.0))
        .add()
        // USING THE RCS THRUSTERS, on the hull before it has flown anywhere.
        .step("open the RCS sheet on the parked hull")
        .on_enter(|world: &mut World| sheet_start(world, RCS_LESSON, LESSON_GRID))
        .until(frames(RCS_LEAD_CELLS))
        .add()
        .step("hold the modifier and push the mouse over")
        .on_enter(|world: &mut World| {
            press_action("rcs_modifier")(world);
            world.insert_resource(RcsPush);
        })
        .until(sheet_written(RCS_LESSON))
        .deadline(60.0)
        .add()
        .step("let the modifier up")
        .on_enter(|world: &mut World| {
            release_action("rcs_modifier")(world);
            world.remove_resource::<RcsPush>();
        })
        .until(elapsed(0.5))
        .add()
        // YOU KEEP YOUR SPEED.
        .step("build a cruise")
        .on_enter(|world: &mut World| {
            world.insert_resource(LessonChase::new(MOMENTUM_EYE));
            press_action("main_drive")(world);
        })
        .until(elapsed(CRUISE_BURN_SECS))
        .add()
        .step("open the momentum sheet on the lit drive")
        .on_enter(|world: &mut World| sheet_start(world, MOMENTUM_LESSON, LESSON_GRID))
        .until(frames(MOMENTUM_LEAD_CELLS))
        .add()
        .step("cut the drive and keep recording")
        .on_enter(release_action("main_drive"))
        .until(sheet_written(MOMENTUM_LESSON))
        .deadline(60.0)
        .add()
        // THE STOP ORDER.
        .step("give the STOP order")
        .on_enter(|world: &mut World| {
            world.insert_resource(LessonChase::new(STOP_EYE));
            press_action("autopilot_stop")(world);
        })
        .until(frames(1))
        .add()
        .step("let the key up and let the hull come round")
        .on_enter(release_action("autopilot_stop"))
        .until(elapsed(STOP_TURN_SECS))
        .add()
        .step("record the braking burn")
        .on_enter(|world: &mut World| sheet_start(world, STOP_LESSON, LESSON_GRID))
        .until(sheet_written(STOP_LESSON))
        .deadline(60.0)
        .add()
        .step("park the chase")
        .on_enter(|world: &mut World| {
            world.remove_resource::<LessonChase>();
        })
        .add()
}
