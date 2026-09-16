//! lesson_start_dock: the two START HERE demonstrations that are about the
//! SCREEN rather than about the ship - `start_cinematic` (the HUD toggle
//! clearing every instrument) and `start_verbs` (the keybind dock, with the
//! engaged verb inverted).
//!
//! One producer, two sheets, because they are one cockpit: the same hull, the
//! same cruise, the same chip row. The screen is cleaned and put back, and
//! then a verb is engaged on the row that came back.
//!
//! ## Both are ACTION loops, and the camera is the game's own
//!
//! Neither lesson survives as a still. "The HUD toggle cycles two levels" is a
//! picture of one level with nothing saying there is another; "an engaged verb
//! inverts" is a picture of an inverted chip with nothing saying what inverted
//! it. So both were flipped to loops
//! (`crates/nova_authoring/src/base_content/lessons.rs`) and their alt text
//! rewritten to describe the footage.
//!
//! Nothing here poses a camera. Both subjects are drawn IN SCREEN SPACE over
//! whatever the player is looking at, and the player's own view is the follow
//! camera behind the hull - so the honest framing for a HUD lesson is the one
//! the game gives, and a posed eye would only make the shot less like the
//! screen the reader is being taught to read.
//!
//! ## Why the hull is pinned and its cruise is written
//!
//! Both sheets want a QUIET CRUISE that lasts exactly as long as the sheet
//! does, which is not a thing a real ship does. A ship that really cruises
//! leaves the hollow it was framed in, taking the composition with it; a ship
//! that really answers STOP reaches the rest deadband, the order disengages,
//! and the inverted chip goes out mid-sheet. So the hull is held at the origin
//! (`hollow::pin_player`) and its velocity is re-written every frame after the
//! pin ([`hold_cruise`]) - the device `lesson_start_scene` already ships for
//! the velocity sphere. The instruments then read a real cruise, the flight
//! computer holds a real STOP order against it, and neither runs out before
//! the twenty cells do.
//!
//! The price is one honest limit, worth naming: a hull pinned square can never
//! swing retrograde, so the flight readout holds at `AP STOP - ALIGN` and the
//! speed never falls. That is the first second after the key in any cockpit,
//! which is the second these twenty cells cover - but it is not the whole
//! maneuver, and the lesson that teaches the whole maneuver is `flight_stop`,
//! which flies it.
//!
//! The set is `hollow::duel_hollow`: the player, one hostile parked down the
//! nose, and the rock shell. The hostile is not the subject and is never
//! locked - it is there because the dock draws only the verbs the ship can use
//! RIGHT NOW, and a cockpit with nothing in front of it is a thinner row than
//! any player will ever see.
//!
//! ## What the dock does that the lesson did not ask for
//!
//! Engaging STOP inverts two chips, not one: `chip_state` makes CANCEL hot
//! whenever ANYTHING is engaged, and an unengaged ship has no CANCEL chip at
//! all. So the row does not merely invert, it GROWS - which is the other half
//! of the same lesson ("a verb the ship cannot use is off the row"), shot for
//! free. The alt text says so rather than pretending one chip changed.
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
//!   cargo run --example lesson_start_dock --features debug
//! ```

#[path = "shared/hollow.rs"]
mod hollow;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_start_dock")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's HUD-level and keybind-dock demonstrations", long_about = None)]
struct Cli;

/// The sheet for "Two HUD levels".
#[cfg(feature = "debug")]
const CINEMATIC_LESSON: &str = "start_cinematic";
/// The sheet for "The keybind dock".
#[cfg(feature = "debug")]
const VERBS_LESSON: &str = "start_verbs";

/// Cells the cinematic sheet holds the full HUD before the toggle.
///
/// Nearly half the sheet, because the BEFORE is half of this lesson: a reader
/// has to have read the instruments to see that the clean frame is the same
/// view with them gone.
#[cfg(feature = "debug")]
const HUD_LEAD_CELLS: u32 = 8;

/// Cells the verbs sheet holds the idle row before the verb is engaged.
#[cfg(feature = "debug")]
const DOCK_LEAD_CELLS: u32 = 6;

/// The cruise the instruments read, and the speed the flight computer is asked
/// to take off. A figure a new player meets on the training range.
#[cfg(feature = "debug")]
const CRUISE_SPEED: MetersPerSecond = MetersPerSecond(120.0);

/// How far OFF THE NOSE that cruise runs, in degrees, swung UP.
///
/// The chase camera stands directly astern, so a cruise straight down the nose
/// puts the velocity shell's cone behind the hull from here and the first
/// capture drew a "full HUD" frame with no sphere in it at all - the one
/// instrument the lesson names first. Swung up, the cone clears the deck and
/// reads against the starfield, and the claim the reader is checking (the
/// clean frame is this frame with the instruments gone) has something to lose.
#[cfg(feature = "debug")]
const CRUISE_OFF_NOSE_DEGREES: f32 = 55.0;

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
        app.add_plugins(start_dock_script());
        // Only under the script, like the other hollow producers: the set's
        // geometry is measured from a player at the origin, and a plain run is
        // the owner flying it.
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.add_systems(
                Update,
                (hollow::pin_player, hold_cruise)
                    .chain()
                    .run_if(resource_exists::<hollow::HoldStation>),
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

/// Write the hull's cruise every frame, AFTER the pin has zeroed it.
///
/// `hollow::pin_player` holds the hull on station by zeroing its velocity, so
/// the cruise the instruments read has to be re-stated on each frame. Ordered
/// after the pin, or the pin would erase it.
#[cfg(feature = "debug")]
fn hold_cruise(
    mut player: Query<
        &mut avian3d::prelude::LinearVelocity,
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let swing = CRUISE_OFF_NOSE_DEGREES.to_radians();
    // The nose is world -Z; pitch the cruise up off it, clear of the hull the
    // chase camera is looking straight through.
    let heading = Vec3::new(0.0, swing.sin(), -swing.cos());
    for mut velocity in &mut player {
        velocity.0 = heading * CRUISE_SPEED.to_engine();
    }
}

/// Advance once the HUD is at `level` - the resource the toggle writes, not a
/// count of what is on screen.
#[cfg(feature = "debug")]
fn the_hud_reads(
    level: HudVisibility,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<HudVisibility>(move |hud| *hud == level)
}

/// Advance once the dock's STOP chip is drawing INVERTED.
///
/// The chip's own rendered state, which the dock writes from what the ship is
/// doing (`chip_state` in `nova_hud/src/keybind_dock.rs`), so this is the
/// honest end of "the engaged verb inverts" rather than a guess at how long an
/// order takes to engage. The verb is found by NAME in `DOCK_VERBS`: the row's
/// order is the dock's business and would go stale the day a verb is added.
#[cfg(feature = "debug")]
fn the_stop_chip_is_hot() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(stop) = DOCK_VERBS.iter().position(|verb| *verb == "STOP") else {
            return false;
        };
        world
            .try_query::<(&DockChip, &DockChipState)>()
            .is_some_and(|mut chips| {
                chips
                    .iter(world)
                    .any(|(chip, state)| chip.0 == stop && *state == DockChipState::Hot)
            })
    })
}

/// Cruise with the instruments up, clear the screen, put it back, then engage
/// a verb on the row.
#[cfg(feature = "debug")]
fn start_dock_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
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
        .step("raise the instruments and hold the cruise")
        .on_enter(hollow::hud_instrument)
        .until(and(the_hud_reads(HudVisibility::On), frames(SETTLE_FRAMES)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // TWO HUD LEVELS: the full contextual screen, then the same view with
        // the toggle pressed inside the recording.
        .step("open the HUD sheet on the full screen")
        .on_enter(|world: &mut World| sheet_start(world, CINEMATIC_LESSON, LESSON_GRID))
        .until(frames(HUD_LEAD_CELLS))
        .add()
        .step("press the HUD toggle")
        .on_enter(press_action("hud_cinematic"))
        .until(frames(1))
        .add()
        .step("let the key up and hold the clean frame")
        .on_enter(release_action("hud_cinematic"))
        .until(sheet_written(CINEMATIC_LESSON))
        .deadline(60.0)
        .add()
        .step("the screen went to cinematic")
        .until(the_hud_reads(HudVisibility::Cinematic))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // THE KEYBIND DOCK: the same toggle brings the row back, because two
        // levels means one press round-trips.
        .step("press the toggle again")
        .on_enter(press_action("hud_cinematic"))
        .until(frames(1))
        .add()
        .step("let the key up and let the instruments come back")
        .on_enter(release_action("hud_cinematic"))
        .until(and(the_hud_reads(HudVisibility::On), frames(SETTLE_FRAMES)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("open the dock sheet on the idle row")
        .on_enter(|world: &mut World| sheet_start(world, VERBS_LESSON, LESSON_GRID))
        .until(frames(DOCK_LEAD_CELLS))
        .add()
        .step("engage the braking order")
        .on_enter(press_action("autopilot_stop"))
        .until(frames(1))
        .add()
        .step("let the key up and hold the inverted chip")
        .on_enter(release_action("autopilot_stop"))
        .until(and(the_stop_chip_is_hot(), sheet_written(VERBS_LESSON)))
        .deadline(60.0)
        .add()
        // A sheet that closed over a row nothing engaged would be twenty cells
        // of the lesson's other half only, so the run ends on the chip state.
        .step("the dock inverted the engaged verb")
        .until(the_stop_chip_is_hot())
        .deadline(STEP_DEADLINE_SECS)
        .add()
}
