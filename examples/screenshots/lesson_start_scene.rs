//! lesson_start_scene: the two START HERE demonstrations shot in the world -
//! `start_hud` (the instruments drawn around the ship) and `start_camera` (the
//! camera turning without the ship).
//!
//! One producer, two frames, because they are one set: a corvette held on
//! station in the rock hollow, shot once with the HUD up and once as a sweep
//! with the screen clean.
//!
//! The velocity the sphere draws is WRITTEN, not flown (`hold_drift`), and it
//! is deliberately off the nose: the lesson's claim is that the sphere points
//! where you are going and the nose does not, so a shot of a ship flying
//! straight ahead would illustrate the opposite. The hull is frozen because
//! both framings are posed, and a frozen hull integrates nothing - including
//! the drift the white cone needs (the same device as
//! `screenshot_hud_shell`).
//!
//! `start_camera` is a POSE loop (see `shared/lesson.rs`): the ship holds, the
//! CAMERA moves, on one period of a wide sine arc so the last cell hands back
//! to the first.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole script, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the still and tile the sheet
//!   (staged under `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_start_scene --features debug
//! ```

#[path = "shared/hollow.rs"]
mod hollow;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, sweep_lesson_camera, LessonSweep, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_start_scene")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's HUD still and camera sweep", long_about = None)]
struct Cli;

/// The still this shoots for: "Reading the HUD".
#[cfg(feature = "debug")]
const HUD_SHOT: &str = "start_hud.png";

/// The sheet this tiles: "Looking around".
#[cfg(feature = "debug")]
const CAMERA_LESSON: &str = "start_camera";

/// How fast the ship is drifting for the HUD shot. Fast enough that the speed
/// chip reads a real cruise rather than a rounding error, slow enough to be a
/// figure a new player meets on the training range.
#[cfg(feature = "debug")]
const DRIFT_SPEED: MetersPerSecond = MetersPerSecond(120.0);

/// How far OFF THE NOSE that drift runs, in degrees.
///
/// The whole point of the shot. The hull noses down world -Z; the drift is
/// swung off it by this much, so the shell's cone sits away from the nose and
/// the picture says what the lesson says. Straight down the nose and the cone
/// hides behind the hull, which reads as "the sphere is where you are
/// pointing" - the opposite claim.
///
/// The swing is VERTICAL, and that is a framing decision, not a physical one.
/// An angle only reads on screen when the camera is square to the plane it is
/// swung in: swung to port it is seen end-on from the beam and foreshortens to
/// almost nothing, and the camera that would show it stands overhead, looking
/// down at a ship. Swung up, the beam camera this shot already wants sees the
/// whole angle.
#[cfg(feature = "debug")]
const DRIFT_OFF_NOSE_DEGREES: f32 = 38.0;

/// Where the HUD shot stands: square on the ship's starboard beam, close, and
/// barely above it.
///
/// Square, because the drift is swung in the vertical plane and only a beam
/// camera sees that angle at its full width (see [`DRIFT_OFF_NOSE_DEGREES`]).
/// Close, because the shell is a soft glow rather than a drawn wireframe: at
/// 300 m it is a smudge beside the hull, and at 200 m it is a third of the
/// frame. The bearing puts the nose across the screen, so the cone above it
/// reads as OFF the nose rather than as part of the drive.
#[cfg(feature = "debug")]
const HUD_EYE: Meters3 = Meters3::new(200.0, 22.0, 0.0);

/// What the sweep looks at, and what the HUD shot looks at: the parked hull,
/// which `hollow::pin_player` holds at the hollow's origin.
#[cfg(feature = "debug")]
const SUBJECT: Meters3 = Meters3::ZERO;

/// How far the sweep's camera stands off the hull. About 2.7 times the clad
/// corvette's 54.7 m bounding radius, which holds the whole ship in a cell.
#[cfg(feature = "debug")]
const SWEEP_RANGE: Meters = Meters(155.0);
/// How far above the hull the sweep rides: a slight downward look, so the deck
/// reads.
#[cfg(feature = "debug")]
const SWEEP_HEIGHT: Meters = Meters(45.0);
/// The bearing the sweep is centred on: the ship's starboard quarter.
#[cfg(feature = "debug")]
const SWEEP_BEARING_DEGREES: f32 = 40.0;
/// Half the sweep's width.
///
/// WIDE, unlike the pose loops whose subject is the ship: this lesson's subject
/// IS the camera move, so the arc has to read as looking around rather than as
/// parallax. Thirty degrees either way is a 155 m swing at this range - the
/// hull turns right through three-quarters to bow-on and back.
#[cfg(feature = "debug")]
const SWEEP_ARC_DEGREES: f32 = 30.0;

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
        app.add_plugins(start_scene_script());
        // Both framings are posed, so nothing in the set may drift between
        // them - but only on a capture run, so a plain `cargo run` keeps its
        // physics and the hollow really does float.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
        app.add_systems(Update, sweep_lesson_camera);
        // Only under the script, like the other hollow producers: the set's
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
    // The SOLO set: both lessons point at the player's own hull - the glow on
    // its shell, then the camera going round it - so a second ship parked in
    // frame is something for the eye to go to instead.
    commands.trigger(LoadScenario(hollow::solo_hollow(&game_assets, &ships)));
}

/// Write the ship's cruise every frame, AFTER the pin has zeroed it.
///
/// `hollow::pin_player` holds the hull on station by zeroing its velocity, and
/// a frozen body integrates nothing anyway, so the drift the velocity sphere
/// draws has to be re-stated on each frame. Ordered after the pin, or the pin
/// would erase it.
#[cfg(feature = "debug")]
fn hold_drift(
    mut player: Query<
        &mut avian3d::prelude::LinearVelocity,
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let swing = DRIFT_OFF_NOSE_DEGREES.to_radians();
    // The nose is world -Z; pitch the drift up off it, which is the swing the
    // beam camera sees square on.
    let heading = Vec3::new(0.0, swing.sin(), -swing.cos());
    for mut velocity in &mut player {
        velocity.0 = heading * DRIFT_SPEED.to_engine();
    }
}

/// The sweep this lesson's loop rides.
#[cfg(feature = "debug")]
fn view() -> LessonSweep {
    LessonSweep::new(
        SUBJECT,
        SWEEP_RANGE,
        SWEEP_HEIGHT,
        SWEEP_BEARING_DEGREES,
        SWEEP_ARC_DEGREES,
    )
}

/// Settle the hollow, shoot the instruments, then sweep the clean screen.
#[cfg(feature = "debug")]
fn start_scene_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
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
        // READING THE HUD: the instruments are the subject, so the HUD is up
        // and the camera stands off the beam where the whole sphere fits.
        .step("raise the instruments and frame the beam")
        .on_enter(|world: &mut World| {
            hollow::hud_instrument(world);
            pose_camera(world, HUD_EYE, SUBJECT);
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the instruments")
        .on_enter(|world: &mut World| shoot(world, HUD_SHOT))
        .until(shot_written(HUD_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // LOOKING AROUND: the screen is clean, because what moves here is the
        // camera and nothing else. One period of the arc IS the sheet.
        .step("clear the screen and open the sweep")
        .on_enter(|world: &mut World| {
            hollow::hud_cinematic(world);
            world.insert_resource(view());
            sheet_start(world, CAMERA_LESSON, LESSON_GRID);
        })
        .until(sheet_written(CAMERA_LESSON))
        .deadline(60.0)
        .add()
        .step("park the sweep")
        .on_enter(|world: &mut World| {
            world.remove_resource::<LessonSweep>();
        })
        .add()
}
