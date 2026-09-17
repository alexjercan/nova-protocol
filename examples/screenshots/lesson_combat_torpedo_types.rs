//! lesson_combat_torpedo_types: the training handbook's demonstration for
//! "Serpent and Lance" (`assets/base/training/combat_torpedo_types.webp`).
//!
//! An ACTION loop (see `shared/lesson.rs`) of a salvo already in the air. Two
//! boats of the same class fire in the same beat on the same bearing, one with
//! the shipped Serpent bay and one with the Lance bay swapped into it
//! (`hollow::torpedo_types_hollow`), and the sheet is cut out of the middle of
//! the run-in.
//!
//! ## Why the launch is not in it
//!
//! The two types differ in a weave and a cruise cap, and neither is visible at
//! the tube: both rounds leave cold on the ejector at the same speed and both
//! are still straight while the drive catches. The difference is something the
//! run ACCUMULATES - the corkscrew has to have swung, and the Lance has to have
//! spent seconds being faster - so the sheet opens after the run has been
//! going long enough to have built it. The launch itself is the neighbouring
//! lesson's frame (`lesson_combat_torpedoes`), shot on the same bays.
//!
//! ## Why the camera rides with them
//!
//! Two seconds of cruise is about 660 m. A camera holding still has to stand
//! nearly half a kilometre off to keep that much run in the lens, and at that
//! range a torpedo is thirty pixels and its weave is a wobble. Riding the pair
//! at a hundred meters puts both rounds big in frame, the swing reads at its
//! true size against the frame, and the rock shell streaming past behind them
//! is what says they are moving at all.
//!
//! The ride solves in `PostUpdate`, in `CameraAuthoritySystems::Solve`, for the
//! reason written on `shared/lesson.rs`'s own chase: the projectiles are moved
//! by the fixed step, and a camera solved in `Update` samples them a variable
//! fraction of a frame stale, which reads as the camera falling behind and
//! snapping back.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - fly the whole salvo, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the sheet (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_combat_torpedo_types --features debug
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
#[command(name = "lesson_combat_torpedo_types")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's Serpent-and-Lance demonstration", long_about = None)]
struct Cli;

/// The lesson this records for, and so the name of the tiled sheet.
#[cfg(feature = "debug")]
const LESSON: &str = "combat_torpedo_types";

/// The two torpedoes by the name their type carries in flight
/// (`TorpedoType::name`), which is also the name the lesson calls them.
#[cfg(feature = "debug")]
const WEAVING: &str = "Serpent";
/// The straight one.
#[cfg(feature = "debug")]
const STRAIGHT: &str = "Lance";

/// Seconds of run-in to let go by before the sheet opens.
///
/// Cut to put the sheet on ONE clean dip of the corkscrew. Measured on this
/// set, the Serpent crosses under the Lance at about four seconds, reaches 45 m
/// under it at five, and is back level at six - one swing, beginning and ending
/// near the line, inside the two seconds a sheet holds. Both rounds are on
/// their drives at cruise by then, the Lance's lead has opened to 47 m and
/// grows to 83 m by the last cell, and neither is within a kilometre of its
/// fuze.
#[cfg(feature = "debug")]
const RUN_SETTLE_SECS: f32 = 4.0;

/// How far the riding camera stands off the pair, square across their run.
///
/// Sized on the WEAVE, which is the smallest thing the frame has to show. The
/// Serpent's swing measures about fifteen meters off its guidance line, so a
/// camera that wants that read as a curve rather than as a wobble can afford a
/// frame only a couple of hundred meters wide - which at 16:9 and the game's
/// lens is this standoff. It is also close enough to make a ten-meter round
/// read as a torpedo, and wide enough to hold the swing AND the lead the Lance
/// opens by the last cell: a closer cut lost the Serpent over the top edge.
#[cfg(feature = "debug")]
const PAIR_STANDOFF: Meters = Meters(160.0);
/// How far above the run the camera rides.
///
/// Almost nothing, which is deliberate. The Serpent's corkscrew swings 90 m
/// TOWARD and away from an abeam camera as well as up and down it, and a
/// camera tilted down turns that depth into travel UP AND DOWN THE CELL: the
/// round dives under the frame at the near end of its swing and comes back at
/// the far end, which is a picture of a torpedo diving rather than of one
/// weaving. Level, the depth swing only changes how big the round is, and the
/// height it shows in the cell is the height it actually has.
#[cfg(feature = "debug")]
const PAIR_RISE: Meters = Meters(4.0);
/// How far BELOW the round it rides the camera aims.
///
/// The weave is not centred on the round that flies the line. Measured over
/// this window the Serpent runs between 11 and 45 m under the Lance and never
/// crosses above it, so an aim on the Lance itself puts the whole swing in the
/// bottom half of the cell and drops the deep end of it off the edge. Aimed
/// here, the two tracks sit either side of the middle.
#[cfg(feature = "debug")]
const AIM_DROP: Meters = Meters(22.0);
/// How far back along the run the camera aims from the round it rides.
///
/// The Lance holds the guidance line and the Serpent falls back off it, so the
/// frame is not symmetric: aiming at the Lance itself spends half the cell on
/// the empty run in front of it. Aimed this far behind, the straight round
/// sits forward of centre and the rest of the cell is the ground the weave
/// is measured over.
#[cfg(feature = "debug")]
const TRAIL_ROOM: Meters = Meters(33.0);

/// Ride beside the salvo: inert until a step inserts it.
#[derive(Resource, Clone, Copy)]
#[cfg(feature = "debug")]
struct PairRide {
    /// Standoff square across the run.
    standoff: Meters,
    /// Height above it.
    rise: Meters,
}

/// Where the STRAIGHT round is.
///
/// The frame is cut from the Lance's own reference, not from the midpoint of
/// the pair, because a corkscrew is only a corkscrew against something that is
/// not corkscrewing. Ridden off the midpoint the camera takes half the weave
/// out of the picture by following it, and what is left is two rounds drifting
/// apart; ridden off the round that flies the guidance line, the line itself
/// holds still in the cell and everything the Serpent does - the swing, and
/// falling back out of it - happens against it.
///
/// The local `Transform`, for the reason [`ride_beside_the_pair`] is registered
/// where it is: a projectile's world pose for THIS frame is its own transform,
/// and its `GlobalTransform` is still last frame's until `Propagate` runs.
#[cfg(feature = "debug")]
fn the_straight_round(world: &mut World) -> Option<Meters3> {
    let mut rounds =
        world.query_filtered::<(&Transform, &TorpedoType), With<TorpedoProjectileMarker>>();
    rounds
        .iter(world)
        .find(|(_, kind)| kind.name == STRAIGHT)
        .map(|(transform, _)| Meters3::from_engine(transform.translation))
}

/// Which way the camera stands off the run: square across the LAUNCH LINE.
///
/// Off the authored line rather than off the rounds' own headings. A weaving
/// torpedo's heading swings by a fifth of a radian several times a second, so a
/// basis derived from it turns the whole frame at weave rate - the rocks roll,
/// the pair slides, and the one thing the sheet has to show, a track curving
/// against a track that does not, is smeared across a camera doing its own
/// curving. The launch line does not move, so the frame does not either, and
/// everything that swings in it is the ordnance.
#[cfg(feature = "debug")]
fn across_the_run() -> Vec3 {
    let run = (hollow::TYPES_TARGET - hollow::TYPES_POSITION)
        .get()
        .normalize_or_zero();
    run.cross(Vec3::Y).normalize_or_zero()
}

/// Pose the camera abeam of the salvo, looking at it.
#[cfg(feature = "debug")]
fn ride_beside_the_pair(world: &mut World) {
    let Some(ride) = world.get_resource::<PairRide>().copied() else {
        return;
    };
    let Some(lead) = the_straight_round(world) else {
        return;
    };
    // Aimed BEHIND the round it rides, so the straight track sits forward in
    // the cell and the room the frame has is where the Serpent actually is -
    // swinging, and falling further back every cell.
    let run = (hollow::TYPES_TARGET - hollow::TYPES_POSITION)
        .get()
        .normalize_or_zero();
    let at = lead - Meters3(Vec3::Y * AIM_DROP.get() + run * TRAIL_ROOM.get());
    let eye = at + Meters3(across_the_run() * ride.standoff.get() + Vec3::Y * ride.rise.get());
    pose_camera(world, eye, at);
}

/// Register the ride where a camera that follows moving bodies has to run.
#[cfg(feature = "debug")]
fn pair_ride_plugin(app: &mut App) {
    app.add_systems(
        PostUpdate,
        ride_beside_the_pair.in_set(CameraAuthoritySystems::Solve),
    );
}

/// Print where each round is along the run and how far off the guidance line
/// it has swung, so the framing constants above can be cut from measurements
/// rather than guessed.
#[cfg(feature = "debug")]
fn report_the_run(world: &World) -> String {
    let Some(mut rounds) =
        world.try_query_filtered::<(&Transform, &TorpedoType), With<TorpedoProjectileMarker>>()
    else {
        return "the run has no torpedoes in it".to_string();
    };
    let run = (hollow::TYPES_TARGET - hollow::TYPES_POSITION)
        .get()
        .normalize_or_zero();
    let across = across_the_run();
    let mut lines = Vec::new();
    for (transform, kind) in rounds.iter(world) {
        let at = Meters3::from_engine(transform.translation);
        let from_launch = (at - hollow::TYPES_POSITION).get();
        lines.push(format!(
            "{}: {:.0} m along, {:.0} m abeam, {:.0} m up",
            kind.name,
            from_launch.dot(run),
            from_launch.dot(across),
            from_launch.y
        ));
    }
    lines.join("; ")
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
        app.add_plugins(pair_ride_plugin);
        app.add_plugins(torpedo_types_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    commands.trigger(LoadScenario(hollow::torpedo_types_hollow(
        &game_assets,
        &ships,
    )));
}

/// Settle the pocket, fire both boats together, let the run build, then cut the
/// sheet out of the middle of it.
#[cfg(feature = "debug")]
fn torpedo_types_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the two-type hollow")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle the two-type hollow")
        .until(elapsed(1.0))
        .add()
        // The camera has left the player's ship before the first round is in
        // the air, so its instruments would be chrome over somebody else's
        // ordnance - the same call the other ordnance frames make.
        .step("clear the screen")
        .on_enter(hollow::hud_cinematic)
        .until(elapsed(0.5))
        .add()
        // BOTH boats in the SAME beat. A salvo that left one tube a beat
        // before the other is a picture of two rounds at different points of
        // one run, which is the one reading this frame must not give.
        .step("fire both boats together")
        .on_enter(|world: &mut World| {
            hollow::loose_torpedoes_from(world, hollow::WEAVER_ID);
            hollow::loose_torpedoes_from(world, hollow::STRAIGHT_ID);
        })
        .until(hollow::torpedo_salvo_in_flight(
            hollow::EXPECTED_TYPE_TORPEDO_COUNT,
        ))
        .deadline(6.0)
        .add()
        .step("commit them both to the raider")
        .on_enter(|world: &mut World| {
            hollow::commit_torpedo_salvo(world, hollow::EXPECTED_TYPE_TORPEDO_COUNT);
            world.insert_resource(PairRide {
                standoff: PAIR_STANDOFF,
                rise: PAIR_RISE,
            });
        })
        .until(frames(1))
        .add()
        .step("let the run build the difference")
        .until(elapsed(RUN_SETTLE_SECS))
        .diagnose(report_the_run)
        .add()
        .step("cut the sheet out of the middle of the run")
        .on_enter(|world: &mut World| sheet_start(world, LESSON, LESSON_GRID))
        .until(sheet_written(LESSON))
        .deadline(60.0)
        .add()
        // Both rounds have to have survived the whole sheet. One that died on
        // a rock leaves a demonstration of a single torpedo, which ships as a
        // picture of the wrong thing rather than as an error.
        .step("the sheet caught both rounds")
        .on_enter(|world: &mut World| {
            let mut rounds = world.query_filtered::<&TorpedoType, With<TorpedoProjectileMarker>>();
            let names: Vec<String> = rounds.iter(world).map(|kind| kind.name.clone()).collect();
            assert!(
                names.iter().any(|name| name == WEAVING)
                    && names.iter().any(|name| name == STRAIGHT),
                "the sheet has to close on both types still flying, and it closed on {names:?}"
            );
        })
        .until(frames(1))
        .diagnose(report_the_run)
        .add()
}
