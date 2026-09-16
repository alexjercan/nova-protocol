//! lesson_combat_radar: the training handbook's demonstration for "Using the
//! radar" (`assets/base/training/combat_radar.webp`).
//!
//! The nav radar sweeps the hollow, marks a contact, and the sheet is recorded
//! with the mark standing on the HUD - which is the lesson: hold the gesture,
//! sweep, mark one.
//!
//! The mark is committed BEFORE the camera moves. Releasing the gesture stops
//! the retargeting and keeps the last candidate (`nova_ship`'s radar search),
//! so the sweep that makes the sheet close cannot swing the mark onto another
//! hull mid-record. The HUD stays up here, unlike the scene lessons: the
//! instrument IS the subject.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole script, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the sheet (staged under
//!   `NOVA_CAPTURE_DIR`). `scripts/capture-lesson-media.sh` runs this and
//!   packages the result into the base bundle.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_combat_radar --features debug
//! ```

#[path = "shared/hollow.rs"]
mod hollow;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{hold_lesson_camera, lesson_profile, sweep_lesson_camera, LessonSweep, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_combat_radar")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's radar demonstration", long_about = None)]
struct Cli;

/// The lesson this records for, and so the name of the tiled sheet.
#[cfg(feature = "debug")]
const LESSON: &str = "combat_radar";

/// What the sweep looks at: a point far down the player's own bearing, so the
/// camera sits over its shoulder and the ray it aims the radar along runs out
/// to where the contact is.
#[cfg(feature = "debug")]
const SWEEP_SUBJECT: Meters3 = Meters3::new(0.0, 6.0, -250.0);
/// How far the camera stands off that point. With the subject 250 m ahead this
/// leaves the camera about 220 m behind the player - four times the corvette's
/// 55 m bounding radius, so the hull reads whole in the cell instead of
/// running off its edge.
#[cfg(feature = "debug")]
const SWEEP_RANGE: Meters = Meters(460.0);
/// How far above the subject the camera rides. High enough to look DOWN on the
/// player rather than up its exhaust: from behind and level, a corvette under
/// power is one lit engine bell filling the lower corner.
#[cfg(feature = "debug")]
const SWEEP_HEIGHT: Meters = Meters(70.0);
/// Centred off the player's shoulder rather than straight down its spine, so
/// the hull does not stand in front of the contact it marked.
#[cfg(feature = "debug")]
const SWEEP_BEARING_DEGREES: f32 = 6.0;
/// Half the sweep's width. Tighter than a scene lesson's: the mark has to stay
/// where the eye found it. At this range it is still a 24 m swing either way.
#[cfg(feature = "debug")]
const SWEEP_ARC_DEGREES: f32 = 3.5;

/// The camera path this lesson records over. Built in one place because the
/// beats that AIM the radar pose from it too: the radar picks by the camera's
/// own look ray, so the view that latches the mark has to be the view that
/// records it.
#[cfg(feature = "debug")]
fn sweep() -> LessonSweep {
    LessonSweep::new(
        SWEEP_SUBJECT,
        SWEEP_RANGE,
        SWEEP_HEIGHT,
        SWEEP_BEARING_DEGREES,
        SWEEP_ARC_DEGREES,
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
        app.add_plugins(radar_lesson_script());
        app.add_systems(Update, sweep_lesson_camera);
        // Only under the script, like the other hollow producers: the set's
        // geometry - and so which body the sweep marks - is measured from a
        // player at the origin, and a plain run is the owner flying it.
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

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    // The QUIET set, not the fighting one: the lesson is that a sweep marks a
    // contact, and a battle in the frame puts wreckage and tracers over the
    // one bracket the cell has room to show.
    commands.trigger(LoadScenario(hollow::ordnance_hollow(&game_assets, &ships)));
}

/// Advance once the player has marked THE RAIDER.
///
/// Named rather than "marked anything": the camera is posed down the raider's
/// own bearing before the sweep opens, so the raider is the body nearest the
/// look ray, and a run that came back with a rock instead has a mark on
/// scenery in the one cell the lesson gets. Waiting on the mark - rather than
/// on a guessed second - is also what keeps an empty sweep from recording a
/// sheet with no bracket in it.
#[cfg(feature = "debug")]
fn the_raider_is_marked() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(mut locks) = world
            .try_query_filtered::<&TravelLock, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
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

/// Settle the hollow, sweep the radar, commit the mark, then record.
#[cfg(feature = "debug")]
fn radar_lesson_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the hollow")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle the hollow")
        .on_enter(|world| {
            hollow::hold_station(world);
            hollow::nudge_raider(world);
        })
        .until(elapsed(2.0))
        .add()
        // Frame FIRST: the radar picks by the camera's look ray, so the shot's
        // own view is what aims the sweep.
        .step("raise the instruments and frame the bearing")
        .on_enter(|world: &mut World| {
            hollow::hud_instrument(world);
            hold_lesson_camera(world, sweep());
        })
        .until(elapsed(0.5))
        .add()
        .step("sweep the nav radar")
        .on_enter(hollow::hold_radar)
        .until(the_raider_is_marked())
        .deadline(20.0)
        .add()
        // Commit: from here the mark is held and the camera is free to move.
        .step("commit the mark")
        .on_enter(hollow::release_radar)
        .until(elapsed(0.5))
        .add()
        .step("record the radar sheet")
        .on_enter(|world: &mut World| {
            world.insert_resource(sweep());
            sheet_start(world, LESSON, LESSON_GRID);
        })
        .until(sheet_written(LESSON))
        .deadline(60.0)
        .add()
}
