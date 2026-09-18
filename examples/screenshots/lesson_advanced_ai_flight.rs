//! lesson_advanced_ai_flight: the handbook's "How an enemy flies" loop - one
//! hostile flying the real engage maneuver around the player's hull.
//!
//! An ACTION loop (see `shared/lesson.rs`), and the only one in the set whose
//! action is another ship's AUTOPILOT rather than a gesture: the camera stands
//! still and what moves in the two seconds is the raider itself, running in
//! down its bearing with its nose already on the target. Nothing here is
//! posed. The maneuver is `nova_ship`'s own (`input/ai/maneuver.rs`), which is
//! the lesson's whole claim.
//!
//! ## The CLOSE rather than the circle, and why
//!
//! The lesson names both halves of the maneuver - close while outside the
//! band, circle once inside - and a two-second sheet can only carry one of
//! them. It carries the close, because the circle has no motion in it at this
//! scale: the orbit term is allowed a quarter of the hull's authority
//! (`AI_ORBIT_AUTHORITY_RESERVE`), so twenty cells of a settled fight is
//! twenty cells of a hull that looks parked. The run-in is the same computer
//! doing the same job at a hundred metres a second. Measured, not assumed: the
//! first cut of this sheet recorded the settled band, and cell one and cell
//! twenty are the same picture.
//!
//! ## The one authored number, and why it is authored
//!
//! `hollow::HUNTER_STANDOFF` asks this raider for 140 m of clearance instead
//! of the engine's kilometre. Not for the framing of the close but to MAKE the
//! close reach: a ship that wants a kilometre of clearance stops a kilometre
//! out, and the gap this sheet opens at is 500 m, which it would never see.
//! `standoff_clearance` is the supported knob for this, so what is recorded is
//! the production maneuver with an authored clearance rather than a posed copy
//! of it.
//!
//! ## Why the camera stands where it does
//!
//! Square ACROSS the run-in, on the perpendicular through the midpoint of the
//! pair. The subject is a distance being eaten, and a camera looking down the
//! approach sees that distance foreshortened to nothing - the same reason
//! `lesson_combat_torpedoes` stands across its torpedo's run rather than
//! behind it. From here the raider crosses the frame toward a player that
//! stays put in it.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - fly the set, exit clean,
//!   recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the sheet (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_advanced_ai_flight --features debug
//! ```

#[path = "shared/hollow.rs"]
mod hollow;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, sweep_lesson_camera, LessonSweep, LESSON_GRID, LESSON_SECS};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_advanced_ai_flight")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's enemy-flight demonstration", long_about = None)]
struct Cli;

/// The lesson this records for, and so the name of the tiled sheet.
#[cfg(feature = "debug")]
const LESSON: &str = "advanced_ai_flight";

/// What the camera looks at: a point on the run-in line, biased toward the
/// raider's end of it.
///
/// A point between the two hulls, because the shot IS the pair - aimed at the
/// player the raider enters at the frame edge, aimed at the raider the player
/// leaves it. Biased rather than halfway, because the two ends are not worth
/// the same: the raider is the SUBJECT and must never be clipped, and the
/// player is an anchor that reads perfectly well sitting in a corner. Dead
/// centre put the raider half off the top edge for the opening cells.
#[cfg(feature = "debug")]
const SUBJECT: Meters3 = Meters3::new(79.0, 28.0, -278.0);
/// How far the eye stands off that midpoint.
///
/// 560 m against the 500 m the pair is apart.
///
/// The pair separates DIAGONALLY on this screen, so the angle that has to fit
/// is the frame's short one: the raider sits 210 m off the aim point, which
/// is twenty-one degrees from here, inside the vertical half-angle. Close
/// enough that the raider is still a HULL rather than a dot - the same shot
/// taken from 700 m was a blob that grew into a slightly larger blob.
#[cfg(feature = "debug")]
const EYE_RANGE: Meters = Meters(560.0);
/// How far above it the eye rides: a downward look, so the run-in is seen
/// against the rock field rather than against open space.
#[cfg(feature = "debug")]
const EYE_HEIGHT: Meters = Meters(150.0);
/// The bearing the eye stands on: square across the run-in.
///
/// The raider comes in on `hollow::HUNTER_START`'s bearing, and this is that
/// bearing turned ninety degrees - the atan2 of the perpendicular, written
/// out because the sweep takes an angle rather than a direction.
#[cfg(feature = "debug")]
const EYE_BEARING_DEGREES: f32 = 74.1;

/// How long the walk lets the maneuver run before it opens the sheet.
///
/// A ceiling, not a wait: the beat ends on `the_hunter_has_closed`, and this
/// is only how long the run will hold for a raider that never gets there.
#[cfg(feature = "debug")]
const CLOSE_DEADLINE_SECS: f32 = 45.0;

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
        app.add_plugins(ai_flight_script());
        // NOTHING is frozen, and nothing may be: the subject of this sheet is
        // a ship under its own flight computer, and `freeze_bodies` would
        // record twenty cells of it holding still.
        app.add_systems(Update, sweep_lesson_camera);
        // The player is pinned so the circle has a fixed centre - the camera
        // is aimed at a point, and a drifting centre would walk the whole
        // geometry out of the frame. Only under the script, like every other
        // hollow producer.
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
    commands.trigger(LoadScenario(hollow::hunter_hollow(
        &game_assets,
        &sections,
        &ships,
    )));
}

/// The framing this lesson's loop holds: an eye that does not move.
#[cfg(feature = "debug")]
fn view() -> LessonSweep {
    LessonSweep::new(SUBJECT, EYE_RANGE, EYE_HEIGHT, EYE_BEARING_DEGREES, 0.0)
}

/// Load the pocket, let the hostile close, then record the band it holds.
#[cfg(feature = "debug")]
fn ai_flight_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the hunter hollow")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("hold the centre and clear the screen")
        .on_enter(|world: &mut World| {
            hollow::hold_station(world);
            // Cinematic, because the instruments belong to the ship the
            // lesson is NOT about: every chip on the screen would be the
            // player's, and the subject is the other hull.
            hollow::hud_cinematic(world);
            world.insert_resource(view());
        })
        .until(elapsed(1.0))
        .add()
        // The gate, not a guess: the maneuver takes as long as it takes, and
        // a sheet opened early is twenty cells of a dot on the far wall.
        .step("let the hostile close to its band")
        .until(hollow::the_hunter_has_closed())
        .diagnose(hollow::hunter_diagnosis)
        .deadline(CLOSE_DEADLINE_SECS)
        .add()
        // BOTH, and the clock is the half that matters: `sheet_written` holds
        // the instant it is asked on a run with nothing recording, so a wait on
        // the sheet alone ends this beat one frame in and the assertion below
        // reads a run-in that has had no time to break off. The sheet's own
        // length is what the recorded lesson shows.
        .step("record the last of the run-in")
        .on_enter(|world: &mut World| sheet_start(world, LESSON, LESSON_GRID))
        .until(and(sheet_written(LESSON), elapsed(LESSON_SECS)))
        .deadline(60.0)
        .add()
        // The gap only shrinks on a run-in, so a raider outside the gate at
        // the last cell is one that broke off mid-sheet - and that ships as a
        // demonstration of an empty pocket rather than as an error.
        .step("the hostile was still closing at the last cell")
        .on_enter(|world: &mut World| {
            assert!(
                hollow::the_hunter_has_closed()(world),
                "the hostile pulled away while the sheet was recording: {}",
                hollow::hunter_diagnosis(world)
            );
        })
        .until(frames(1))
        .add()
}
