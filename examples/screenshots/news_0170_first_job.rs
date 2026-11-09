//! news_0170_first_job: the v0.17.0 news New Game still - a fresh world's
//! flight HUD, docked at the starting station, with the held `Take a job`
//! objective chip and the interface key under it.
//!
//! The world starts the way the Create World modal starts it, on
//! [`WORLD_SEED`] as [`ROLE`], with no save folder. Once the window has
//! streamed in and the start has docked the player, the walk drops the dev
//! overlays and the status bar, checks that the open-world scenario posted
//! the `take_a_job` objective and holds its chip, waits for the chip to lay
//! out with its label, and shoots [`SHOT`].
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the walk with its asserts,
//!   writing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write `news-0170-first-job.png`
//!   (staged under `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0170-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0170_first_job --features debug
//! ```

#[path = "../playable/shared/open_world_stage.rs"]
mod open_world_stage;

use std::sync::Arc;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::harness::Predicate, prelude::*};
use open_world_stage::{window_settled, OpenWorldStartPlugin};

#[derive(Parser)]
#[command(name = "news_0170_first_job")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.17.0 New Game still: the docked start with the held Take a job chip",
    long_about = None
)]
struct Cli {
    /// The world seed.
    #[arg(long, default_value_t = WORLD_SEED)]
    seed: u32,
}

/// The seed the still plays: the v0.16.0 release lead's world.
const WORLD_SEED: u32 = 2;

/// The start role the still flies.
const ROLE: &str = "military";

/// The objective a fresh open world posts and holds.
const TAKE_A_JOB: &str = "take_a_job";
/// The chip label the objective draws: the objective text in capitals.
const TAKE_A_JOB_LABEL: &str = "TAKE A JOB";

/// The still this producer writes.
const SHOT: &str = "news-0170-first-job.png";

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
    app.add_plugins(first_job_script());
    app.run()
}

fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
    Arc::new(check)
}

/// The start has streamed in and docked the player.
#[derive(Resource)]
struct StartReady;

/// The window has settled and exactly one player ship is docked.
fn start_is_ready(world: &mut World) -> bool {
    if !window_settled(world) {
        return false;
    }
    let mut players = world.query_filtered::<Entity, With<PlayerSpaceshipMarker>>();
    let players: Vec<Entity> = players.iter(world).collect();
    matches!(players.as_slice(), [player] if world.get::<DockedShip>(*player).is_some())
}

/// A laid-out objective chip reads [`TAKE_A_JOB_LABEL`].
fn chip_shows_take_a_job(world: &World) -> bool {
    ui_node_present("ObjectiveStackChip")(world)
        && world
            .try_query::<(&Name, &Text)>()
            .is_some_and(|mut labels| {
                labels.iter(world).any(|(name, text)| {
                    name.as_str() == "ObjectiveStackLabel" && text.0.contains(TAKE_A_JOB_LABEL)
                })
            })
}

/// Panic unless the scenario posted the objective and holds its chip.
fn assert_the_held_objective(world: &mut World) {
    assert!(
        world.resource::<ObjectiveChipHolds>().contains(TAKE_A_JOB),
        "first job: the '{TAKE_A_JOB}' chip is not held"
    );
    info!("first job: PASS the fresh world holds the '{TAKE_A_JOB}' chip");
}

/// The walk: into the world, then the still.
fn first_job_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
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
        .step("drop the overlays and the status bar")
        .on_enter(|world: &mut World| {
            hide_dev_overlays(world);
            hide_status_bar(world);
            assert_the_held_objective(world);
        })
        .until(and(frames(SETTLE_FRAMES), when(chip_shows_take_a_job)))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("shoot the held chip")
        .on_enter(|world: &mut World| shoot(world, SHOT))
        .until(shot_written(SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
