//! system_headless_replay: spike 3 for `nova_channel` - the seed and the clock.
//!
//! The design record (task 20260820-174148, nova-channel.html) names the two
//! determinism gaps between "a driven run" and "a replayable run": the gameplay
//! RNG seeds from the OS, and a free-running headless app takes its frame
//! delta from the wall clock. This range closes both the way the channel
//! will - `NOVA_SEED` (the new knob in `nova_gameplay::settings`) plus bevy's
//! own `TimeUpdateStrategy::ManualDuration`, the step clock - and prints a
//! digest that two runs can be diffed on:
//!
//!   - boot `--norender --scenario shakedown_run` with a pinned 1/64 s frame;
//!   - play until the SCENARIO's own clock reaches [`REPLAY_FRAMES`] ticks
//!     (8 simulated seconds - anchored on the world's clock, because when an
//!     outside observer first notices Playing races the scenario start by a
//!     frame);
//!   - capture the probe's own world snapshot, drop the two stamps that are
//!     not world state (`frame` counts the variable loading phase, `t_real`
//!     is the wall), and print its hash.
//!
//! Two runs with one seed must print one digest; a different seed must not.
//! The runner asserts that, not this range - a single process cannot see its
//! twin.
//!
//! Run (no display needed):
//! ```text
//! NOVA_SEED=42 cargo run --example system_headless_replay --features debug
//! # look for: `headless replay: digest <hex>` - compare across runs.
//! ```

#[cfg(feature = "debug")]
use bevy::{prelude::*, time::TimeUpdateStrategy, window::PrimaryWindow};
#[cfg(feature = "debug")]
use nova_protocol::prelude::*;

#[cfg(not(feature = "debug"))]
fn main() {
    eprintln!("system_headless_replay boots through the debug feature set;");
    eprintln!("run it with --features debug");
}

/// Scenario ticks of play before the digest: 8 simulated seconds, enough for
/// the scenario's opening spawns and some physics to diverge if anything is
/// unseeded.
#[cfg(feature = "debug")]
const REPLAY_FRAMES: u32 = 512;

#[cfg(feature = "debug")]
fn main() -> bevy::app::AppExit {
    match nova_protocol::nova_gameplay::prelude::seed_from_env() {
        Some(seed) => info!("headless replay: seeded with {seed}"),
        None => warn!("headless replay: NOVA_SEED unset - two runs will not agree"),
    }

    let mut app = editor_app(
        false,
        Some(StartupScenario::Id("shakedown_run".to_string())),
    );

    // The step clock: every frame advances the world by exactly one fixed
    // step's worth of time, however long the wall took. This is the channel's
    // clock model; here it is what makes frame counts mean the same instant
    // in both runs. (The first manual update is dt 0 - bevy warms the clock -
    // which is fine: both runs pay the same zero.)
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_micros(15_625),
        ));

    // The virtual window, as in the sibling spikes - not load-bearing for the
    // digest, but the channel always spawns it, so the replay proof runs the
    // assembly the channel will.
    app.world_mut().spawn((
        Window {
            resolution: (1280, 720).into(),
            ..default()
        },
        PrimaryWindow,
    ));

    app.init_resource::<Replay>();
    app.add_systems(PreUpdate, drive.after(bevy::input::InputSystems));

    app.run()
}

#[cfg(feature = "debug")]
#[derive(Resource)]
struct Replay {
    played: Option<u32>,
    started: std::time::Instant,
}

#[cfg(feature = "debug")]
impl Default for Replay {
    fn default() -> Self {
        Self {
            played: None,
            started: std::time::Instant::now(),
        }
    }
}

#[cfg(feature = "debug")]
const DEADLINE_SECS: u64 = 180;

#[cfg(feature = "debug")]
fn drive(world: &mut World) {
    if world.resource::<Replay>().started.elapsed().as_secs() > DEADLINE_SECS {
        panic!("headless replay: STALLED after {DEADLINE_SECS}s");
    }

    let played = match world.resource::<Replay>().played {
        None => {
            if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
                return;
            }
            0
        }
        Some(played) => played + 1,
    };
    world.resource_mut::<Replay>().played = Some(played);

    // Keyed to the SCENARIO clock, not to this driver's frame count: when this
    // observer first sees Playing races the scenario's own start by a frame
    // (the one-tick `elapsed` jitter the first digest run surfaced), so the
    // digest fires on the first frame the world's own clock reaches 8
    // simulated seconds - an exact multiple of the 2^-6 s tick, so both runs
    // capture the identical instant. The channel has no such race - its step
    // clock owns time from boot - but an outside observer diffing two runs
    // must anchor on the world's clock.
    let mut snapshot = nova_probe::capture_snapshot(world, "headless-replay");
    let elapsed = snapshot
        .get("elapsed")
        .and_then(serde_json::Value::as_f64)
        .expect("the snapshot stamps the scenario clock");
    if elapsed < f64::from(REPLAY_FRAMES) * 0.015_625 {
        return;
    }
    let object = snapshot
        .as_object_mut()
        .expect("the snapshot is one JSON object");
    // Not world state: `frame` counts the load phase (IO-variable), `t_real`
    // is the wall clock. Everything else is what a replay must reproduce.
    object.remove("frame");
    object.remove("t_real");
    let serialized = snapshot.to_string();

    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    serialized.hash(&mut hasher);
    let digest = format!("{:016x}", hasher.finish());
    info!(
        "headless replay: digest {digest} over {} bytes at played frame {played}",
        serialized.len()
    );
    info!("headless replay: state {serialized}");
    nova_probe::probe_marker(
        world,
        "outcome: the replay digest is recorded",
        serde_json::json!({ "digest": digest, "bytes": serialized.len() }),
    );

    // The knob proof, separate from the digest: shakedown's first 8 seconds
    // never consume the global entropy (the belt scatter runs on scenario-
    // authored seeds, and the RNG's consumers - turret spread, debris - are
    // combat-time), so the world digest alone cannot show the seed arriving.
    // One draw from the stream the turrets will play from can: same seed,
    // same draw; different seed, different draw.
    {
        use bevy_rand::prelude::{GlobalRng, WyRand};
        use rand::RngExt;
        let mut query = world.query_filtered::<&mut WyRand, With<GlobalRng>>();
        let draw = {
            let mut rng = query
                .single_mut(world)
                .expect("the entropy plugin owns one global stream");
            format!("{:016x}", rng.random_range(0..u64::MAX))
        };
        info!("headless replay: entropy draw {draw}");
        nova_probe::probe_marker(
            world,
            "outcome: the entropy draw is recorded",
            serde_json::json!({ "draw": draw }),
        );
    }
    world.write_message(AppExit::Success);
}
