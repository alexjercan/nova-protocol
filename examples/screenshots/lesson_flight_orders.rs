//! lesson_flight_orders: the two FLIGHT demonstrations the flight computer
//! flies - `flight_goto` (the ship turning onto a mark and burning for it) and
//! `flight_orbit` (the ship settling into a circle around a planetoid).
//!
//! One producer, two sheets, and one set: "The ring" (`shared/ring.rs`), the
//! same planetoid, survey beacon and well the wiki's orbit figure is shot on.
//! Both verbs need a REAL gravity well to be worth a picture - GOTO flies a
//! leg out of one and ORBIT holds a ring inside it - and building a second
//! well beside the one this fleet already owns would be two bodies to keep in
//! step.
//!
//! ## Two ACTION loops, flown by the computer
//!
//! Neither sheet drives a key after the order: the whole claim is that the
//! COMPUTER flies the hull, so the cells have to be the maneuver itself. The
//! eye is `LessonChase`, which rides a fixed world offset from the moving
//! hull, and the two lessons ask opposite things of it:
//!
//! - GOTO is about the SHIP, so its eye is close, off the beam, and aimed at
//!   the hull: the turn onto the mark and the drive lighting fill the cell.
//! - ORBIT is about the ship's RELATIONSHIP to a body 1.8 km across. Its eye
//!   stands well outboard and aims PAST the hull at the planetoid, so the body,
//!   the holo ring drawn across it, the radius spoke and the ship on the end of
//!   that spoke are one picture - the framing the wiki figure settled on, for
//!   the same reason.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - fly the whole script, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the two sheets (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_flight_orders --features debug
//! ```

#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;
#[path = "shared/ring.rs"]
mod ring;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{chase_lesson_camera, lesson_profile, LessonChase, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_flight_orders")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's GOTO and ORBIT demonstrations", long_about = None)]
struct Cli;

/// The sheet for "GOTO a mark".
#[cfg(feature = "debug")]
const GOTO_LESSON: &str = "flight_goto";
/// The sheet for "ORBIT a mark".
#[cfg(feature = "debug")]
const ORBIT_LESSON: &str = "flight_orbit";

/// How far outboard of the ring the GOTO eye stands, how far it rises, and how
/// far back down the track it sits.
///
/// Outboard, so the planetoid is the BACKDROP: the beacon is over the well's
/// pole, so the leg is flown straight up out of the ring, and an eye that has
/// the body behind the ship sees the hull swing off the body and climb away
/// from it. Shot the other way round - the first cut of this producer put the
/// eye on a fixed world bearing - the leg plays out against black, on a hull
/// the ring's light rig is not pointing at.
#[cfg(feature = "debug")]
const GOTO_STANDOFF: f32 = 280.0;
/// How far above the ring the GOTO eye rides. Low: the leg climbs, so the room
/// in the cell is wanted above the ship, not below it.
#[cfg(feature = "debug")]
const GOTO_RISE: f32 = 50.0;
/// How far back down the track the GOTO eye sits, so the hull is three-quarters
/// on at the moment the order arrives.
#[cfg(feature = "debug")]
const GOTO_TRAIL: f32 = 140.0;

/// How far outboard of the ring the ORBIT eye stands, and how far it aims
/// inboard past the hull.
///
/// Both measured along the ship's own radial, which is why they are lengths
/// here and a position at the step: the ship is somewhere on a 3.2 km ring by
/// the time it holds, and where on the ring is the insertion's business, not
/// this file's.
#[cfg(feature = "debug")]
const ORBIT_STANDOFF: f32 = 450.0;
/// How far above the ring the ORBIT eye rides.
#[cfg(feature = "debug")]
const ORBIT_RISE: f32 = 150.0;
/// How far back down the track it sits, so the hull is seen three-quarters on
/// rather than end-on.
#[cfg(feature = "debug")]
const ORBIT_TRAIL: f32 = 100.0;
/// How far past the hull the eye aims: most of the way to the body, so the
/// planetoid takes the middle of the cell.
#[cfg(feature = "debug")]
const ORBIT_AIM: f32 = 1_200.0;

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
        app.add_plugins(flight_orders_script());
        // Nothing frozen and nothing posed by hand: both subjects are the
        // flight computer flying the hull. The chase is inert until a step
        // inserts its offset.
        app.add_systems(Update, chase_lesson_camera);
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    commands.trigger(LoadScenario(ring::the_ring(&game_assets, &ships)));
}

/// Stand the eye outboard of the ring, close, and aim it at the hull, with the
/// planetoid filling the black behind.
#[cfg(feature = "debug")]
fn frame_the_leg(world: &mut World) {
    let ship = ring::ship_position(world);
    let out = ship.get().normalize_or_zero();
    let track = ring::ship_heading(world);
    world.insert_resource(LessonChase::new(Meters3(
        out * GOTO_STANDOFF + Vec3::Y * GOTO_RISE - track * GOTO_TRAIL,
    )));
}

/// Stand the eye outboard of the ring and aim it past the hull at the body.
///
/// Read off the SHIP each time rather than written down, because the ring's
/// own geometry decides it: outboard is the ship's radial from the well, and
/// the trail is its track. A pair of constants would be a second copy of where
/// the insertion happened to leave it.
#[cfg(feature = "debug")]
fn frame_the_ring(world: &mut World) {
    let ship = ring::ship_position(world);
    let out = ship.get().normalize_or_zero();
    let track = ring::ship_heading(world);
    world.insert_resource(
        LessonChase::new(Meters3(
            out * ORBIT_STANDOFF + Vec3::Y * ORBIT_RISE - track * ORBIT_TRAIL,
        ))
        .looking(Meters3(-out * ORBIT_AIM)),
    );
}

/// Fly one leg and one insertion, and photograph a lesson out of each.
#[cfg(feature = "debug")]
fn flight_orders_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the ring")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("raise the instruments and take up the chase")
        .on_enter(|world: &mut World| {
            ring::hud_instrument(world);
            frame_the_leg(world);
        })
        .until(elapsed(1.0))
        .add()
        // GOTO A MARK: the order, then the swing onto the heading it picked.
        // The sheet opens WITH the order rather than before it, because the
        // first thing the computer does is the thing the lesson names.
        .step("give the travel computer the beacon")
        .on_enter(|world: &mut World| {
            ring::engage_goto(world);
            sheet_start(world, GOTO_LESSON, LESSON_GRID);
        })
        .until(sheet_written(GOTO_LESSON))
        .deadline(60.0)
        .add()
        // ORBIT A MARK: the same ship, the verb changed under it - which is
        // what a pilot pressing O over a held GOTO does.
        .step("change the verb to ORBIT")
        .on_enter(ring::engage_orbit)
        .until(ring::orbit_burning())
        .deadline(30.0)
        .add()
        .step("settle onto the ring")
        .until(and(
            ring::orbit_holding(),
            scenario_variable_is("orbit_stable", 1.0),
        ))
        .deadline(180.0)
        .add()
        .step("let the ring steady")
        .until(elapsed(ring::STEADY_SECS))
        .add()
        .step("record the ring")
        .on_enter(|world: &mut World| {
            frame_the_ring(world);
            sheet_start(world, ORBIT_LESSON, LESSON_GRID);
        })
        .until(sheet_written(ORBIT_LESSON))
        .deadline(60.0)
        .add()
        .step("park the chase")
        .on_enter(|world: &mut World| {
            world.remove_resource::<LessonChase>();
        })
        .add()
}
