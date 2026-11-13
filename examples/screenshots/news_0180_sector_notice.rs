//! news_0180_sector_notice: the v0.18.0 news sector still - the flight HUD of
//! a fresh world on the frame after the ship crossed into a cell its chart
//! had not recorded, with the notice `SECTOR (x, y, z) - FIRST VISIT`.
//!
//! The world starts the way the Create World modal starts it, on
//! [`WORLD_SEED`] as [`ROLE`], with no save folder. Once the window has
//! streamed in and the start has docked the player, the walk lets the clamp
//! go and stages the crossing: it puts the ship [`STANDOFF`] short of the +X
//! face of its cell, nose on +X, waits for the `UNDOCKED` notice to leave the
//! slot, and gives the ship [`CROSSING_SPEED`] along +X. Nobody flies the
//! ship. The crossing, the chart read and the notice are the game's own: the
//! walk waits for the HUD notice to read the line for the cell past the face
//! and shoots [`SHOT`].
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the walk with its asserts,
//!   writing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write `sector-first-visit.png`
//!   (staged under `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0180-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0180_sector_notice --features debug
//! # look for: `sector notice: PASS the HUD posted 'SECTOR (1, 0, 0) - FIRST VISIT'`
//! ```

#[path = "../playable/shared/open_world_stage.rs"]
mod open_world_stage;

use std::sync::Arc;

use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::harness::Predicate, prelude::*};
use open_world_stage::{sector_edge, the_player, window_settled, OpenWorldStartPlugin};

#[derive(Parser)]
#[command(name = "news_0180_sector_notice")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.18.0 sector still: the FIRST VISIT notice after a staged crossing",
    long_about = None
)]
struct Cli {
    /// The world seed.
    #[arg(long, default_value_t = WORLD_SEED)]
    seed: u32,
}

/// The seed the still plays: the v0.17.0 New Game still's world.
const WORLD_SEED: u32 = 2;

/// The start role the still flies.
const ROLE: &str = "military";

/// How far short of the cell face the walk puts the ship.
const STANDOFF: Meters = Meters(200.0);

/// The speed the walk gives the ship toward the face, in meters per second.
const CROSSING_SPEED: f32 = 80.0;

/// How long the ship stands at rest before the run: the `UNDOCKED` notice
/// dwells four seconds and fades, and must not share the slot with the
/// crossing.
const NOTICE_CLEAR_SECS: f32 = 5.0;

/// Frames between the line and the still. The notice dwells four seconds of
/// play time, and a software rasterizer spends up to a quarter second of it
/// on each frame of an open world, so the wait is the layout pass and no
/// more.
const NOTICE_LAYOUT_FRAMES: u32 = 3;

/// The still this producer writes.
const SHOT: &str = "sector-first-visit.png";

/// Real seconds the world gets to stream in and dock the start on a software
/// rasterizer.
const SETUP_SECS: f32 = 900.0;

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = editor_app(true, None);
    app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
        bevy::ecs::error::panic,
    ));
    app.add_plugins(OpenWorldStartPlugin {
        seed: cli.seed,
        role: ROLE.to_string(),
    });
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    app.add_plugins(sector_notice_script());
    app.run()
}

fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
    Arc::new(check)
}

/// The start has streamed in and docked the player.
#[derive(Resource)]
struct StartReady;

/// The line the crossing must post, recorded when the walk stages it.
#[derive(Resource)]
struct ExpectedLine(String);

/// The window has settled and exactly one player ship is docked.
fn start_is_ready(world: &mut World) -> bool {
    if !window_settled(world) {
        return false;
    }
    the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
}

/// The player ship flies free.
fn player_is_free(world: &World) -> bool {
    world
        .try_query_filtered::<Has<DockedShip>, With<PlayerSpaceshipMarker>>()
        .is_some_and(|mut players| players.iter(world).all(|docked| !docked))
}

/// Put the ship [`STANDOFF`] short of the +X face of its cell, nose on +X, at
/// rest, and record the line the cell past the face must post.
fn stand_short_of_the_face(world: &mut World) {
    let player = the_player(world).expect("sector notice: one player ship stands");
    let edge = sector_edge(world).expect("sector notice: the world is armed");
    let at = Meters3::from_engine(
        world
            .get::<Position>(player)
            .expect("sector notice: the player has a position")
            .0,
    );
    let home = SectorCoord::containing(at, edge);
    let face = home.centre(edge).x() + Meters(edge.get() * 0.5);
    let start = Meters3::new(face.get() - STANDOFF.get(), at.y().get(), at.z().get());
    let beyond = SectorCoord::containing(
        Meters3::new(face.get() + STANDOFF.get(), at.y().get(), at.z().get()),
        edge,
    );
    assert_ne!(home, beyond, "sector notice: the face must part two cells");
    assert!(
        !world.resource::<Chart>().has_visited(beyond),
        "sector notice: the chart already holds {beyond}"
    );
    world.entity_mut(player).insert((
        Position(start.to_engine()),
        Rotation(Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::X)),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
    ));
    world.insert_resource(ExpectedLine(format!("SECTOR {beyond} - FIRST VISIT")));
    info!("sector notice: staged {STANDOFF:?} short of the face between {home} and {beyond}");
}

/// Give the ship [`CROSSING_SPEED`] along +X.
fn run_at_the_face(world: &mut World) {
    let player = the_player(world).expect("sector notice: one player ship stands");
    world.entity_mut(player).insert(LinearVelocity(
        MetersPerSecond3::new(CROSSING_SPEED, 0.0, 0.0).to_engine(),
    ));
}

/// The HUD notice reads the recorded line.
fn notice_reads_the_line(world: &World) -> bool {
    let Some(expected) = world.get_resource::<ExpectedLine>() else {
        return false;
    };
    world
        .try_query_filtered::<&Text, With<HudNoticeText>>()
        .is_some_and(|mut lines| lines.iter(world).any(|text| text.0 == expected.0))
}

/// The walk: into the world, off the clamp, across the face, then the still.
fn sector_notice_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("the window streams in and the start docks")
        // `window_settled` needs the world mutably, which a predicate never
        // has, so each frame records it.
        .each(|world: &mut World, _, _| {
            if start_is_ready(world) {
                world.insert_resource(StartReady);
            }
        })
        .until(when(|world| world.contains_resource::<StartReady>()))
        .deadline(SETUP_SECS)
        .add()
        .step("drop the overlays and let the clamp go")
        .on_enter(|world: &mut World| {
            hide_dev_overlays(world);
            hide_status_bar(world);
            let player = the_player(world).expect("sector notice: one player ship stands");
            world.trigger(DockingReleaseRequest { entity: player });
        })
        .until(when(player_is_free))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("stand short of the cell face")
        .on_enter(stand_short_of_the_face)
        .until(elapsed(NOTICE_CLEAR_SECS))
        .add()
        .step("run at the face")
        .on_enter(run_at_the_face)
        .until(when(notice_reads_the_line))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("settle the notice")
        .on_enter(|world: &mut World| {
            let line = world.resource::<ExpectedLine>().0.clone();
            info!("sector notice: PASS the HUD posted '{line}'");
        })
        .until(frames(NOTICE_LAYOUT_FRAMES))
        .add()
        .step("shoot the notice")
        .on_enter(|world: &mut World| shoot(world, SHOT))
        .until(shot_written(SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
