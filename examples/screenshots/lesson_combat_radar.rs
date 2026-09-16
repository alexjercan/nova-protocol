//! lesson_combat_radar: the training handbook's demonstration for "Using the
//! radar" (`assets/base/training/combat_radar.webp`).
//!
//! An ACTION loop (see `shared/lesson.rs`): the lock is acquired INSIDE the
//! recording. The sheet opens on an unmarked contact, the gesture goes down,
//! the acquisition dwell charges, the bracket lands, and the rest of the cells
//! hold it - which is the lesson, in the order a player does it.
//!
//! So the camera does NOT move here. A pose loop needs a sweep to make its
//! last cell hand back to its first; this one has something happening in it
//! already, and a camera drifting under an acquisition is one moving thing too
//! many - it also risks walking the look ray off the candidate, which restarts
//! the dwell (`nova_ship`'s radar search). A still camera wraps cleanly on its
//! own. The HUD stays up, unlike the scene lessons: the instrument IS the
//! subject.
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
use lesson::{lesson_profile, sweep_lesson_camera, LessonSweep, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_combat_radar")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's radar demonstration", long_about = None)]
struct Cli;

/// The lesson this records for, and so the name of the tiled sheet.
#[cfg(feature = "debug")]
const LESSON: &str = "combat_radar";

/// What the view looks at: a point far down the player's own bearing, so the
/// camera sits over its shoulder and the ray it aims the radar along runs out
/// to where the contact is.
#[cfg(feature = "debug")]
const VIEW_SUBJECT: Meters3 = Meters3::new(0.0, 6.0, -250.0);
/// How far the camera stands off that point. With the subject 250 m ahead this
/// leaves the camera about 220 m behind the player - four times the corvette's
/// 55 m bounding radius, so the hull reads whole in the cell instead of
/// running off its edge.
#[cfg(feature = "debug")]
const VIEW_RANGE: Meters = Meters(460.0);
/// How far above the subject the camera rides. High enough to look DOWN on the
/// player rather than up its exhaust: from behind and level, a corvette under
/// power is one lit engine bell filling the lower corner.
#[cfg(feature = "debug")]
const VIEW_HEIGHT: Meters = Meters(70.0);
/// Centred off the player's shoulder rather than straight down its spine, so
/// the hull does not stand in front of the contact it marks.
#[cfg(feature = "debug")]
const VIEW_BEARING_DEGREES: f32 = 6.0;
/// How many cells run before the gesture goes down.
///
/// The sheet has to open on the state BEFORE the act, or a player sees the
/// answer and never sees the question. Three cells is a third of a second: long
/// enough to read as "no mark yet", short enough to leave the whole
/// acquisition and a held result inside twenty.
#[cfg(feature = "debug")]
const LEAD_IN_CELLS: u32 = 3;

/// Where the camera stands for the whole recording.
///
/// A [`LessonSweep`] with NO ARC: this is an action loop, so the camera holds
/// (see the module docs). The sweep type still drives it, because the camera
/// has to be written EVERY frame to stay put - a zero arc makes every frame of
/// that path the same place. It is also what aims the radar: the search picks
/// by the camera's own look ray, so the view that latches the mark is the view
/// that records it.
#[cfg(feature = "debug")]
fn view() -> LessonSweep {
    LessonSweep::new(
        VIEW_SUBJECT,
        VIEW_RANGE,
        VIEW_HEIGHT,
        VIEW_BEARING_DEGREES,
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
        app.add_plugins(radar_lesson_script());
        // Nothing in the WORLD moves under an action loop. The only thing that
        // changes between the first cell and the last is the instrument, so
        // the sheet wraps on a scene that is exactly where it started.
        //
        // `freeze_bodies` pins the hulls and the rocks. `sweep_lesson_camera`
        // pins the VIEW: the scenario camera eases back toward its own target
        // every frame, so a pose set once drifts a cell at a time, and a
        // zero-arc sweep is the same pose written again on each of them.
        app.add_systems(Update, (freeze_bodies, sweep_lesson_camera));
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
        .on_enter(hollow::hold_station)
        .until(elapsed(2.0))
        .add()
        // Frame FIRST: the radar picks by the camera's look ray, so the shot's
        // own view is what aims the gesture.
        .step("raise the instruments and frame the bearing")
        .on_enter(|world: &mut World| {
            hollow::hud_instrument(world);
            world.insert_resource(view());
        })
        .until(elapsed(0.5))
        .add()
        // The recording opens BEFORE the gesture: these cells are the state a
        // player starts from, and without them the sheet only ever shows the
        // answer.
        .step("open the sheet on an unmarked contact")
        .on_enter(|world: &mut World| {
            sheet_start(world, LESSON, LESSON_GRID);
        })
        .until(frames(LEAD_IN_CELLS))
        .add()
        // Inside the recording, so the dwell ring charging and the bracket
        // landing are cells of the sheet rather than something that happened
        // before it.
        .step("hold the radar on the contact")
        .on_enter(hollow::hold_radar)
        .until(the_raider_is_marked())
        .deadline(30.0)
        .add()
        // Releasing stops the RETARGETING and keeps the last candidate, so the
        // remaining cells hold the mark rather than hunting for another.
        .step("release, and hold the mark for the rest of the sheet")
        .on_enter(hollow::release_radar)
        .until(sheet_written(LESSON))
        .deadline(60.0)
        .add()
        // The sheet is twenty cells long and the acquisition dwell is about
        // 0.7 s of it; a scene that made the dwell restart would tile a sheet
        // with no bracket in it at all, and that must fail the run rather than
        // ship.
        .step("the sheet caught the mark")
        .on_enter(|world: &mut World| {
            assert!(
                the_raider_is_marked()(world),
                "the sheet closed before the radar committed: the lesson would show a gesture \
                 that never lands. Give the acquisition more cells (LEAD_IN_CELLS) or check \
                 that the view still holds the raider."
            );
        })
        .until(frames(1))
        .add()
}
