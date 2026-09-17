//! lesson_readouts: the two demonstrations whose subject is a NUMBER on the
//! HUD - `start_units` (what the readouts are measured in) and
//! `combat_lock_ranges` (how far away a thing can be and still be locked).
//!
//! One producer, two stills, because both need the same one-off state: a hull
//! actually moving, a second hull drifting across the aim ray, and a radar
//! lock latched onto it. Getting a lock is four beats of walk; spending them
//! twice to photograph two framings of the same instrument would be the only
//! difference between two producers.
//!
//! ## Two framings, and why they are not the same picture
//!
//! `start_units` is shot CLOSE, over the player's own shoulder. Its claim is
//! what the numbers mean, so the frame belongs to the readouts: the speed chip
//! in `m/s` beside the velocity sphere, and the locked contact's distance and
//! closing speed under its bracket. The world is there for scale and nothing
//! else.
//!
//! `combat_lock_ranges` is shot WIDE, from inside the pocket and behind the
//! player, with all THREE hulls in it: the player's own stern for scale, the
//! raider with the bracket and its range on it, and the tender four hundred
//! meters further out carrying only its allegiance marker. Its claim is about
//! distance, so the frame has to have distance in it - a tight shot of one
//! bracket says nothing about how far the reticle reaches.
//!
//! Both are shot with the WEAPONS RAISED, which is not obvious for a Start
//! Here lesson. The reticle prints different things in the two stances: with
//! weapons lowered the bracket carries the GOTO affordance, and only the
//! combat lock carries `DST` and `CLS`. A units lesson needs a distance and a
//! speed on the screen, so the walk raises.
//!
//! ## What is written rather than flown
//!
//! The player is pinned on station (`hollow::pin_player`, as every hollow
//! producer does, because the radar picks by the aim ray and a drifting player
//! swings the raider off it) and its velocity is then written back
//! (`hold_drift`). A pinned hull reads `0.0 m/s`, and a units lesson whose
//! speed chip is zero teaches nothing about speeds. The raider's own drift is
//! `hollow::nudge_raider`, which is what gives the lock's closing-speed line
//! something to say.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole walk, exit
//!   clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the stills (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_readouts --features debug
//! ```

#[path = "shared/hollow.rs"]
mod hollow;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{sweep_lesson_camera, LessonSweep};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_readouts")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's units and lock-range demonstrations", long_about = None)]
struct Cli;

/// `start_units`: the readouts, close, with their units on them.
#[cfg(feature = "debug")]
const UNITS_SHOT: &str = "start_units.png";
/// `combat_lock_ranges`: the same lock, wide, with a second hull further out.
#[cfg(feature = "debug")]
const RANGES_SHOT: &str = "combat_lock_ranges.png";

/// How fast the player is written to be going.
///
/// A round number a reader can match to the chip, and inside the 150 m/s cap
/// Basic Training flies under, so the figure in the picture is one the game
/// would actually let a new player reach.
#[cfg(feature = "debug")]
const DRIFT_SPEED: MetersPerSecond = MetersPerSecond(120.0);

/// Where the close framing stands: a little off the player's starboard
/// shoulder and above it, looking down its own bearing.
///
/// The subject is a point 120 m ahead of the parked hull rather than the hull
/// itself, so the camera sits BEHIND the player and the raider it marks is on
/// the same ray - the radar picks by the camera's look ray, so the view that
/// takes the lock has to be the view that shows it.
#[cfg(feature = "debug")]
const UNITS_SUBJECT: Meters3 = Meters3::new(0.0, 4.0, -120.0);
#[cfg(feature = "debug")]
const UNITS_RANGE: Meters = Meters(260.0);
#[cfg(feature = "debug")]
const UNITS_HEIGHT: Meters = Meters(35.0);
#[cfg(feature = "debug")]
const UNITS_BEARING_DEGREES: f32 = 8.0;

/// Where the wide framing stands: 160 m behind the player's starboard quarter,
/// looking down the bisector of the two contacts.
///
/// Aimed BETWEEN them rather than at either, because the shot is the pair: the
/// raider 340 m out on the player's nose and the tender 740 m out, 380 m to
/// port and 300 m above it. From here they sit thirteen and eighteen degrees
/// off the axis, on opposite sides of one frame, with the player's own stern
/// under them for scale.
///
/// INSIDE the pocket, and that is the constraint the framing is built around:
/// the rock shell starts 480 m out (`hollow::shell`), so a camera far enough
/// back to hold both hulls from outside stands IN the shell and photographs
/// one rock. A stand-off of 540 m from a subject 390 m down the bisector puts
/// the lens 160 m from the origin, pointed out through the pocket, with the
/// shell where it belongs - the wall behind the subject.
#[cfg(feature = "debug")]
const RANGES_SUBJECT: Meters3 = Meters3::new(-110.0, 90.0, -375.0);
#[cfg(feature = "debug")]
const RANGES_RANGE: Meters = Meters(542.0);
#[cfg(feature = "debug")]
const RANGES_HEIGHT: Meters = Meters(-50.0);
#[cfg(feature = "debug")]
const RANGES_BEARING_DEGREES: f32 = 18.3;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(readouts_script());
        // The scenario camera eases back toward its own target every frame, so
        // a pose set once drifts off it. A zero-arc sweep is that pose written
        // again on every frame (the device `lesson_combat_radar` uses).
        app.add_systems(Update, sweep_lesson_camera);
        // Only under the script, like every other hollow producer: the set's
        // geometry is measured from a player at the origin, and a plain run is
        // the owner flying it.
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.add_systems(
                Update,
                (hollow::pin_player, hold_drift)
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

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    // The ORDNANCE set, for the one thing it has that the quiet sets do not: a
    // second hull, on a different bearing and four hundred meters further out
    // than the first, which is the whole of the wide shot.
    commands.trigger(LoadScenario(hollow::ordnance_hollow(&game_assets, &ships)));
}

/// Write the player's velocity back after the pin has zeroed it.
///
/// Chained after `pin_player` on purpose: the pin is what keeps the raider on
/// the aim ray, and it clears the velocity the speed chip reads. Straight down
/// the nose, because nothing here is about drift - `start_hud` owns that
/// claim, and a cone swung off the nose in this shot would be a second thing
/// to explain.
#[cfg(feature = "debug")]
fn hold_drift(
    mut player: Query<
        &mut avian3d::prelude::LinearVelocity,
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    for mut velocity in &mut player {
        velocity.0 = Vec3::NEG_Z * DRIFT_SPEED.to_engine();
    }
}

/// The close framing, as a camera path with no arc in it.
#[cfg(feature = "debug")]
fn units_view() -> LessonSweep {
    LessonSweep::new(
        UNITS_SUBJECT,
        UNITS_RANGE,
        UNITS_HEIGHT,
        UNITS_BEARING_DEGREES,
        0.0,
    )
}

/// The wide framing, likewise.
#[cfg(feature = "debug")]
fn ranges_view() -> LessonSweep {
    LessonSweep::new(
        RANGES_SUBJECT,
        RANGES_RANGE,
        RANGES_HEIGHT,
        RANGES_BEARING_DEGREES,
        0.0,
    )
}

/// Advance once the player's COMBAT slot holds THE RAIDER, rather than a rock.
///
/// The combat slot, not the travel one: the radar latches whichever slot the
/// stance current at the hold threshold names
/// (`nova_ship/src/input/targeting/state.rs`), and this walk holds the stance
/// up for the `DST`/`CLS` lines. A predicate that watched `TravelLock` would
/// wait out its deadline on a lock that had already landed.
#[cfg(feature = "debug")]
fn the_raider_is_marked() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(mut locks) = world
            .try_query_filtered::<&CombatLock, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
        else {
            return false;
        };
        locks.iter(world).any(|lock| {
            lock.0.is_some_and(|marked| {
                world
                    .get::<EntityId>(marked)
                    .is_some_and(|id| id.0 == hollow::RAIDER_ID)
            })
        })
    })
}

/// Settle the hollow, take one lock, and photograph it twice.
#[cfg(feature = "debug")]
fn readouts_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the hollow")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle the hollow and set the raider drifting")
        .on_enter(|world: &mut World| {
            hollow::hold_station(world);
            hollow::nudge_raider(world);
        })
        .until(elapsed(2.0))
        .add()
        // Frame FIRST: the radar picks by the camera's look ray, so the close
        // shot's own view is what aims the gesture.
        .step("raise the instruments and frame the bearing")
        .on_enter(|world: &mut World| {
            hollow::hud_instrument(world);
            world.insert_resource(units_view());
        })
        .until(elapsed(0.5))
        .add()
        // Weapons UP before the latch, and that is what the shot turns on: a
        // nav lock draws the GOTO affordance under the bracket, and a COMBAT
        // lock draws `DST <range>` and `CLS <closing speed>` beside it. Those
        // two lines ARE the units lesson, so the stance the walk takes is
        // decided by what the reticle prints rather than by the category the
        // lesson files under.
        // HELD, never released: `combat_stance` is bound to the right mouse
        // button and the weapons stay up only while it is down
        // (`nova_ship/src/input/bindings.rs`). A walk that let it up would
        // photograph a nav lock again, with the GOTO chip back under the
        // bracket.
        .step("raise the weapons, so the lock reads DST and CLS")
        .on_enter(hollow::raise_stance)
        .until(elapsed(0.5))
        .add()
        .step("hold the radar until it marks the raider")
        .on_enter(hollow::hold_radar)
        .until(the_raider_is_marked())
        .deadline(30.0)
        .add()
        // Releasing stops the RETARGETING and keeps the last candidate, so the
        // mark survives the camera move the wide shot needs.
        .step("release, and let the reticle settle")
        .on_enter(hollow::release_radar)
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the readouts")
        .on_enter(|world: &mut World| shoot(world, UNITS_SHOT))
        .until(shot_written(UNITS_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("stand off for the pair")
        .on_enter(|world: &mut World| {
            world.insert_resource(ranges_view());
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        // The mark is the subject of the wide shot too: a frame that lost it
        // on the way out is two hulls and no reticle, which is not the lesson.
        .step("the mark survived the move")
        .on_enter(|world: &mut World| {
            assert!(
                the_raider_is_marked()(world),
                "the lock was dropped between the two framings: the wide shot would carry no \
                 reticle at all. Check that nothing re-armed the radar search."
            );
        })
        .until(frames(1))
        .add()
        .step("capture the pair")
        .on_enter(|world: &mut World| shoot(world, RANGES_SHOT))
        .until(shot_written(RANGES_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
