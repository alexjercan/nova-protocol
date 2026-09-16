//! lesson_flight_well: the two FLIGHT demonstrations about the WELL -
//! `flight_gravity` (a coasting hull falling toward a planetoid with the
//! yellow pull shell standing off the blue velocity one) and `flight_arrival`
//! (a GOTO leg flipped retrograde on its braking ramp).
//!
//! One producer, two sheets, one flight, on the set the flight figures already
//! own: "the ring" (`shared/ring.rs`) - a 900 m ice world whose authored mass
//! puts a real 4.9 km sphere of influence around it, a survey beacon 7.6 km
//! over its pole, and the player parked on the 3.2 km ring. Both lessons need
//! that one body for opposite reasons. `flight_gravity` needs a pull strong
//! enough to fall down in the two seconds a sheet lasts, and `flight_arrival`
//! needs somewhere far enough away that the computer flies a full
//! align-burn-coast-FLIP-brake profile to reach it. Standing a second well
//! beside the one this fleet already keeps in step would be two bodies to tune
//! instead of one.
//!
//! ## Two ACTION loops, and why neither is a still
//!
//! Gravity is an ACCELERATION - that is the whole lesson - and an
//! acceleration cannot be photographed standing still. The cells have to be
//! the ship going faster with nobody's hand on the throttle, so the drive is
//! CUT before the sheet opens and stays out for every cell of it: the speed
//! chip climbing over a dead drive is the claim, and a plume in the corner
//! would hand the reader the wrong cause. The arrival is the same argument
//! with the hull instead of the number: "flip and brake" is two verbs, and a
//! frame of a flipped hull is only the second half of one of them.
//!
//! ## Falling sideways, on purpose
//!
//! The fall is entered from a short burn along the ring's track rather than
//! from rest. A ship dropped from rest falls straight down the radius, and
//! down the radius the two shells POINT THE SAME WAY - the yellow pull shell
//! hides inside the blue velocity one and the lesson's one picture is gone.
//! Sixty meters a second across the track is well under this ring's 137 m/s
//! circular speed, so the ship still falls; it just falls on a curve, with the
//! velocity shell lying along the track and the pull shell aimed at the body.
//! Two shells, two colours, two directions, in one cell.
//!
//! ## Where the two eyes stand
//!
//! Both are `shared/ring.rs`'s [`ring::LegCamera`], which is a rig on the
//! ship's TRACK rather than on a world bearing, because both subjects are a
//! hull whose attitude is about to stop matching its heading.
//!
//! - The fall is shot from BEHIND and above, aimed down the track: down the
//!   track is where the planetoid is, so the body grows into the cell over the
//!   two seconds and the reader can see what the ship is falling toward. From
//!   abeam the same fall is a hull on black with a glowing sphere around it.
//! - The brake is shot from AHEAD, the whole stand-off down the track, for the
//!   reason `loop_goto_arrival` writes out at length: at the top of this leg
//!   the body is a small disc directly BELOW the ship and everything else is
//!   empty sky, so an abeam bearing records a grey hull on black. Standing up
//!   the track aims the lens back past the hull at the world it climbed away
//!   from - and a braking ship fires its drive UP the track, so that is also
//!   the one bearing that is looking into the plume.
//!
//! ## What the brake sheet turned out to hold
//!
//! The sheet opens on `ring::player_retro_burning` - the flip over and the
//! drive lit against the track - because that is the one state the envelope is
//! always in at some point, while the swing before it is as long as the
//! computer decides it is. What it actually caught is better than that floor:
//! the computer trims its heading again a few cells in, so the chip drops from
//! `AP GOTO - BURN` to `AP GOTO - ALIGN` with a `FLIP` counter on the hull and
//! comes back to `BURN` before the sheet closes. The whole envelope the lesson
//! names is in the twenty cells, and none of it was scripted.
//!
//! Both wrap as a tutorial clip wraps, back to the start to do it again: a
//! fall is faster in its last cell than in its first, and a brake is slower.
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
//!   cargo run --example lesson_flight_well --features debug
//! ```

#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;
#[path = "shared/ring.rs"]
mod ring;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_flight_well")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's gravity-well and arrival demonstrations", long_about = None)]
struct Cli;

/// The sheet for "Gravity wells".
#[cfg(feature = "debug")]
const GRAVITY_LESSON: &str = "flight_gravity";
/// The sheet for "The arrival envelope".
#[cfg(feature = "debug")]
const ARRIVAL_LESSON: &str = "flight_arrival";

/// How much speed across the ring the drive builds before it is cut.
///
/// See the module docs: enough that the velocity shell and the pull shell
/// point in visibly different directions, and well under the 137 m/s circular
/// speed at this radius, so what follows is a fall and not an orbit.
#[cfg(feature = "debug")]
const TRACK_SPEED: MetersPerSecond = MetersPerSecond(60.0);

/// How fast the ship is falling when the sheet opens.
///
/// A speed rather than an altitude, because speed is what the readout in the
/// cell shows. On this body it is reached around 2.8 km out, which is the
/// altitude the framing wants as well: close enough that the planetoid is a
/// world filling the lower half of the cell, far enough that it is still a
/// body with a limb rather than ground.
#[cfg(feature = "debug")]
const FALL_SPEED: MetersPerSecond = MetersPerSecond(120.0);

/// The fall's eye, as `shared/ring.rs` bearings: onto the key light's side of
/// the hull, back down the track, up, and how far down the track it aims.
///
/// Behind, because the ship is falling toward the thing the lesson is about
/// and a camera behind it is looking at that thing too. The look-ahead is what
/// hands the planetoid the upper half of the cell: the lens aims down the track
/// instead of at the hull, so the body the ship is falling into gets the frame
/// and the ship sits under it.
///
/// The RISE is the part that cannot be traded away. Both shells are spheres
/// around the hull, and what a reader sees of one is its shaded cap: from an
/// eye level with the ship the two caps sit behind the hull and the cell has a
/// grey ship and a planet in it. Forty-five metres up is what puts both caps
/// clear of the hull's outline, and the second cut of this framing - dropped to
/// eighteen to stop the ship being clipped - proved it by losing them.
///
/// So the clipping is paid for with the STAND-OFF and the LEAD instead. The
/// first cut rode a 160 m stand-off with a 200 m lead, which put the hull
/// seventeen degrees off the lens axis and cut its lower half off at the bottom
/// of every cell; 190 and 130 bring that to ten degrees, for a hull nine tenths
/// the size and a whole ship in the frame.
#[cfg(feature = "debug")]
const FALL_SIDE: Meters = Meters(95.0);
/// How far back down the track the fall's eye rides.
#[cfg(feature = "debug")]
const FALL_BACK: Meters = Meters(190.0);
/// How far above the track the fall's eye rides.
#[cfg(feature = "debug")]
const FALL_RISE: Meters = Meters(45.0);
/// How far down the track the fall's eye aims past the hull.
#[cfg(feature = "debug")]
const FALL_LOOK_AHEAD: Meters = Meters(130.0);

/// The brake's eye: ahead of the ship, on the key light's side, slightly up.
///
/// The stand-off is nearly all `ahead`, with the side offset kept under a
/// third of it, for the reason the module docs give: up the track is the only
/// bearing on this leg with a world in the frame, and it is also the bearing
/// looking into a retro plume.
#[cfg(feature = "debug")]
const BRAKE_SIDE: Meters = Meters(46.0);
/// How far ahead down the track the brake's eye rides.
#[cfg(feature = "debug")]
const BRAKE_AHEAD: Meters = Meters(150.0);
/// How far above the track the brake's eye rides.
#[cfg(feature = "debug")]
const BRAKE_RISE: Meters = Meters(18.0);

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
        app.add_plugins(flight_well_script());
        // Nothing frozen and nothing posed by hand: one sheet is a body pulling
        // a ship and the other is the flight computer flying one. The leg rig
        // is inert until a step installs a bearing.
        app.add_systems(Update, ring::drive_leg_camera);
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    commands.trigger(LoadScenario(ring::the_ring(&game_assets, &ships)));
}

/// Advance once the player's hull is moving at least this fast.
///
/// Read off the hull's own velocity rather than counting seconds, because what
/// this body's pull does to this hull's mass is the well's arithmetic and the
/// ship catalog's, and a timed fall would re-shoot at a different altitude the
/// day either is retuned.
#[cfg(feature = "debug")]
fn speed_at_least(
    wanted: MetersPerSecond,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let Some(mut ships) = world.try_query_filtered::<
            &avian3d::prelude::LinearVelocity,
            (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
        >() else {
            return false;
        };
        // Engine boundary: avian publishes world units per second, and the
        // threshold above is authored in meters per second.
        let floor = wanted.to_engine();
        ships
            .iter(world)
            .any(|velocity| velocity.0.length() >= floor)
    })
}

/// Whether the player is inside a sphere of influence.
///
/// The yellow shell hides itself in flat space
/// (`nova_hud::velocity`'s `HudSelfDrivenVisibility`), so this is the
/// difference between the lesson's picture and the same frame without it.
#[cfg(feature = "debug")]
fn inside_a_well(world: &World) -> bool {
    world
        .try_query_filtered::<&DominantWell, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
        .is_some_and(|mut ships| ships.iter(world).next().is_some())
}

/// Fall down one well and fly one leg out of it, and photograph a lesson out of
/// each.
#[cfg(feature = "debug")]
fn flight_well_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the ring")
        .enter(GameStates::Loading)
        .until(and(player_ship_present(), scenario_camera_present()))
        .deadline(30.0)
        .add()
        .step("raise the instruments")
        .on_enter(ring::hud_instrument)
        .until(elapsed(1.0))
        .add()
        // GRAVITY WELLS. The burn is the run-up, not the subject: it buys the
        // cross-track component that keeps the two shells apart, and it is out
        // long before the sheet opens.
        .step("build a little speed across the ring")
        .on_enter(press_action("main_drive"))
        .until(speed_at_least(TRACK_SPEED))
        .deadline(60.0)
        .add()
        .step("cut the drive and take up the fall")
        .on_enter(|world: &mut World| {
            release_action("main_drive")(world);
            ring::chase(world, FALL_SIDE, FALL_BACK, FALL_RISE, FALL_LOOK_AHEAD);
        })
        .until(and(
            speed_at_least(FALL_SPEED),
            std::sync::Arc::new(inside_a_well),
        ))
        .deadline(120.0)
        .add()
        .step("record the fall")
        .on_enter(|world: &mut World| sheet_start(world, GRAVITY_LESSON, LESSON_GRID))
        .until(sheet_written(GRAVITY_LESSON))
        .deadline(60.0)
        .add()
        // A sheet shot outside the sphere of influence has no yellow shell in
        // it at all, and one shot with the drive lit is a picture of a burn.
        // Fail the run rather than ship either.
        .step("the fall was the well's doing")
        .on_enter(|world: &mut World| {
            assert!(
                inside_a_well(world),
                "the gravity sheet closed outside every sphere of influence: the pull shell hides \
                 itself in flat space, so the demonstration would be a ship with one plain \
                 velocity sphere around it. Check the planetoid still carries its authored mass."
            );
        })
        .until(frames(1))
        .add()
        // THE ARRIVAL ENVELOPE. The same hull, now handed to the computer, with
        // a destination far enough out to be worth a stopping plan.
        .step("give the travel computer the beacon")
        .on_enter(ring::engage_goto)
        .until(ring::player_burning())
        .deadline(60.0)
        .add()
        .step("ride the outbound burn and the coast")
        .until(ring::player_braking())
        .deadline(200.0)
        .add()
        .step("take the lead on the track")
        .on_enter(|world: &mut World| ring::lead(world, BRAKE_SIDE, BRAKE_AHEAD, BRAKE_RISE))
        .until(ring::player_retro_burning())
        .deadline(40.0)
        .add()
        .step("record the braking ramp")
        .on_enter(|world: &mut World| sheet_start(world, ARRIVAL_LESSON, LESSON_GRID))
        .until(sheet_written(ARRIVAL_LESSON))
        .deadline(60.0)
        .add()
        // A sheet that closed with the leg already over is a sheet of a parked
        // ship with a chip that no longer says BURN.
        .step("the ramp was still running")
        .on_enter(|world: &mut World| {
            assert!(
                ring::player_retro_burning()(world),
                "the arrival sheet closed with the brake already finished: the demonstration \
                 would show a hull coasting under a chip that has changed. Open it later on the \
                 leg, or stand the beacon further out."
            );
        })
        .until(frames(1))
        .add()
        .step("park the camera")
        .on_enter(|world: &mut World| {
            world.remove_resource::<ring::LegCamera>();
        })
        .add()
}
