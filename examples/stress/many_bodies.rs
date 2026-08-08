//! many_bodies: N asteroids under physics, gravity and render, spawned, held,
//! torn down, and spawned again.
//!
//! The scale sweep for the BODY count. A shipped scenario measures the game as
//! authored; this one measures what happens when the same systems are handed an
//! order of magnitude more work than any scene ships with - broad-phase pairs,
//! gravity well x affected-body pairs, and one renderable per rock.
//!
//! It makes a correctness claim alongside the number: the swarm has to come
//! BACK to baseline. Every cycle unloads the scenario and asserts that not one
//! `AsteroidMarker` survived, so a leak that a frame-time graph would only show
//! as slow drift fails the run instead (task 20260804-094006, DECISION.md D3 -
//! this is an in-example assertion, not a probe check, because the subject is
//! ECS entity counts rather than scenario variables).
//!
//! The script is enrolled in capture looping, so `--fps` measures spawn -> hold
//! -> teardown ACTIVITY rather than an idle tail.
//!
//! Count knob: `NOVA_STRESS_COUNT` overrides [`DEFAULT_COUNT`].
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example many_bodies --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `many_bodies: N asteroids up`,
//! #           `many_bodies: teardown returned to baseline`,
//! #           `autopilot: cycle complete, no panic`
//!
//! # the frame-time sweep:
//! cargo run -p nova_probe_cli -- run many_bodies --fps --release
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "many_bodies")]
#[command(version = "1.0.0")]
#[command(about = "Scale sweep: N asteroids under physics, gravity and render", long_about = None)]
struct Cli;

/// How many asteroids the swarm holds when `NOVA_STRESS_COUNT` is unset.
///
/// Measured under the CI-shaped raster (lavapipe/llvmpipe, 1280x720, dev
/// profile, 20 warm-up + 120 captured frames):
///
/// | count | mean ms | mean fps |
/// | ----- | ------- | -------- |
/// | 200   | 36.7    | 27.2     |
/// | 400   | 38.7    | 25.9     |
/// | 800   | 165.1   | 6.1      |
///
/// 400 is the pick: the swarm already dominates the frame, and the full
/// 180 + 900 frame capture window still fills in ~45 s, an order of magnitude
/// inside probe's 585 s window-sized completion deadline. 800 fills it in
/// ~180 s, which works but spends most of the CI budget on one example; the
/// knob is there for anyone who wants that on purpose.
const DEFAULT_COUNT: usize = 400;

/// Env var that overrides [`DEFAULT_COUNT`]. Shared with the other `stress/`
/// sweeps so `probe run stress` scales all of them together.
const COUNT_ENV: &str = "NOVA_STRESS_COUNT";

/// One rock in this many carries a gravity well. Gravity is O(wells x affected
/// bodies), so a swarm of pure colliders would leave that half of the sweep
/// untouched; a well every eighth rock keeps the pair count quadratic in the
/// knob without turning the scene into a single gravitational blender.
const GRAVITY_EVERY: usize = 8;

/// Mass parameter (mu) on a well-bearing rock. Weak on purpose: the sweep
/// measures the COST of the gravity pass, and rocks that visibly clump would
/// change the broad-phase distribution between cycles and make the number
/// unrepeatable. 135 is a ~23u SOI - reach follows strength, so a weak well
/// is also a small one, which if anything makes the sweep conservative.
const WELL_MASS: f32 = 135.0;

/// Radius of every rock. Small relative to [`SHELL_RADIUS`], so even the
/// densest swarm the knob is meant for stays a field of separate bodies rather
/// than one interpenetrating mass.
const ROCK_RADIUS: f32 = 2.0;

/// Radius of the shell the swarm sits on. Fixed rather than count-scaled - see
/// [`rock_position`] for why that is the whole point of the sweep. At the
/// default count the rocks sit ~19 units apart on this shell, an order of
/// magnitude clear of [`ROCK_RADIUS`].
const SHELL_RADIUS: f32 = 120.0;

/// The scenario id the swarm loads under.
const SCENARIO_ID: &str = "stress_many_bodies";

/// The first step's name, so `loop_from` restarts the cycle at the spawn
/// without repeating the string.
#[cfg(feature = "debug")]
const LOAD_STEP: &str = "spawn the swarm";

/// How long the swarm is held up before teardown. The steady-state stretch the
/// frame-time capture is actually interested in.
#[cfg(feature = "debug")]
const HOLD_SECS: f32 = 3.0;

/// Settle window between the unload and the baseline assertion: `UnloadScenario`
/// despawns on the next command flush, and a couple of frames under llvmpipe is
/// generous for that.
#[cfg(feature = "debug")]
const TEARDOWN_SETTLE_SECS: f32 = 0.5;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let count = swarm_count();
    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            app.insert_resource(SwarmSize(count));
            app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_swarm);
        })
        .build();

    // Headless smoke-test harness: inert in a normal run. Under NOVA_AUTOPILOT
    // it spawns the swarm, holds it, tears it down and asserts the world came
    // back to baseline - the correctness half of a run whose other half is a
    // frame-time number.
    #[cfg(feature = "debug")]
    {
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                // The swarm is spawned by an OnStart handler, so waiting for
                // the full count is also the gate a looped reload needs: the
                // old cycle's entities outlive the load replacing them.
                .step(LOAD_STEP)
                .enter(GameStates::Loading)
                .until(swarm_is_up(count))
                .deadline(60.0)
                .add()
                // The scene is live again: close the reload interval so the
                // capture excludes it. A no-op on the first cycle.
                //
                // ONE closure doing both jobs, not two `on_enter` calls:
                // `on_enter` REPLACES the hook rather than appending, so a
                // second call silently drops the first. Splitting these left
                // the reload gate latched open for the rest of the run, and
                // every frame after the first loop was excluded from the
                // capture - the fps window could never fill.
                .step("close the reload interval")
                .on_enter(|world: &mut World| {
                    nova_probe::capture_reload_end(world);
                    report_swarm_up(world);
                })
                .add()
                .step("hold the swarm")
                .until(elapsed(HOLD_SECS))
                .add()
                .step("tear the swarm down")
                .on_enter(tear_the_swarm_down)
                .until(elapsed(TEARDOWN_SETTLE_SECS))
                .add()
                .step("check the world came back to baseline")
                .on_enter(assert_back_to_baseline)
                .add()
                // Enrolled in capture looping (task 20260720-000616): when a
                // frame capture outlives the script, the cycle replays, so the
                // capture measures spawn/hold/teardown activity rather than an
                // idle tail.
                .loop_from(LOAD_STEP)
                .on_loop(respawn_the_swarm),
        );
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        app.add_plugins(nova_probe::nova_timeline());
        app.add_plugins(nova_probe::nova_invariants());
        app.add_plugins(nova_probe::nova_frametime());
        app.add_plugins(nova_screenshot());
    }

    app.run()
}

/// How many asteroids this run spawns, so the script's gates and the scenario
/// builder agree without re-reading the environment.
#[derive(Resource, Clone, Copy)]
struct SwarmSize(usize);

/// The swarm size for this run: `NOVA_STRESS_COUNT` if it parses, else
/// [`DEFAULT_COUNT`]. Read once, in `main`.
fn swarm_count() -> usize {
    std::env::var(COUNT_ENV)
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(DEFAULT_COUNT)
}

fn setup_swarm(mut commands: Commands, game_assets: Res<GameAssets>, size: Res<SwarmSize>) {
    commands.trigger(LoadScenario(swarm_scenario(&game_assets, size.0)));
}

/// Where rock `i` of `count` sits: a Fibonacci-sphere shell of FIXED radius,
/// which spreads the swarm evenly over the sphere for any count (a lattice
/// would leave the broad-phase distribution lumpy at counts that are not
/// perfect cubes). Purely a function of `(i, count)`, so a rerun at the same
/// knob builds the same scene and the frame-time numbers stay comparable.
///
/// The radius is a CONSTANT, deliberately. It first grew with the cube root of
/// the count, to hold the spacing between rocks fixed - which sounds like the
/// careful choice and is in fact the bug: constant density means the number of
/// rocks inside the view frustum and inside each broad-phase cell barely moves,
/// so the measured cost saturates. Under llvmpipe the knob did nothing at all
/// past a few hundred (400 and 800 rocks both measured ~40 ms/frame). Pinning
/// the radius makes density rise with the count, which is what puts the swarm
/// size back into the number the sweep exists to report.
fn rock_position(i: usize, count: usize) -> Vec3 {
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    let y = 1.0 - (i as f32 / (count.max(2) - 1) as f32) * 2.0;
    let ring = (1.0 - y * y).max(0.0).sqrt();
    let theta = golden * i as f32;
    Vec3::new(theta.cos() * ring, y, theta.sin() * ring) * SHELL_RADIUS
}

/// The swarm: `count` asteroids on a shell, every [`GRAVITY_EVERY`]th one a
/// gravity well.
fn swarm_scenario(game_assets: &GameAssets, count: usize) -> ScenarioConfig {
    let rocks = (0..count)
        .map(|i| {
            let mut rock = fixtures::asteroid(
                game_assets,
                &format!("rock_{i}"),
                &format!("Rock {i}"),
                rock_position(i, count),
                ROCK_RADIUS,
                // High enough that nothing in the sweep destroys a rock: the
                // count is the measurement, so it must not drift mid-window.
                100_000.0,
                // Unsigned: nothing here radar-locks.
                None,
            );
            if i % GRAVITY_EVERY == 0 {
                // The one field `fixtures::asteroid` does not take, patched at
                // its single caller rather than widened into the shared
                // builder's signature (DECISION.md D2's reasoning).
                if let ScenarioObjectKind::Asteroid(config) = &mut rock.kind {
                    config.mass = Some(WELL_MASS);
                }
            }
            rock
        })
        .collect();

    ScenarioConfig {
        description: "A swarm of asteroids under physics, gravity and render.".to_string(),
        // The scene lights itself: the engine spawns no light, so a scenario
        // that authors none renders black.
        events: fixtures::spawn_on_start(
            [
                rocks,
                ThreePointRig::around("swarm", Vec3::ZERO, 10.0).objects(),
            ]
            .concat(),
        ),
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Many Bodies".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// How many asteroids are alive right now. Scans entities rather than caching a
/// `QueryState`: the predicates and the `&mut World` step hooks both need it,
/// and it runs a handful of times per cycle, not per frame.
#[cfg(feature = "debug")]
fn live_asteroids(world: &World) -> usize {
    world
        .iter_entities()
        .filter(|entity| entity.contains::<AsteroidMarker>())
        .count()
}

/// Advance once the whole swarm is in the world.
#[cfg(feature = "debug")]
fn swarm_is_up(count: usize) -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(move |world: &World| live_asteroids(world) >= count)
}

/// Log the live count, and push it as a timeline marker so an armed run can see
/// the swarm rise and fall in the same JSONL stream as the frame times.
#[cfg(feature = "debug")]
fn report_swarm_up(world: &mut World) {
    let live = live_asteroids(world);
    nova_probe::probe_marker(
        world,
        "stress: swarm up",
        serde_json::json!({ "live": live }),
    );
    info!("many_bodies: {live} asteroids up");
}

/// Drop the whole scene. `UnloadScenario` guarantees every scenario-scoped
/// entity goes, which is exactly the claim the next step checks.
#[cfg(feature = "debug")]
fn tear_the_swarm_down(world: &mut World) {
    nova_probe::probe_marker(world, "stress: teardown", serde_json::json!({}));
    world.trigger(UnloadScenario);
}

/// The correctness claim: the teardown left NOTHING behind. Panics rather than
/// stalling a predicate, so a leak fails the smoke test with the count that
/// survived instead of timing out anonymously.
#[cfg(feature = "debug")]
fn assert_back_to_baseline(world: &mut World) {
    let live = live_asteroids(world);
    nova_probe::probe_marker(
        world,
        "stress: baseline",
        serde_json::json!({ "live": live }),
    );
    assert_eq!(
        live, 0,
        "many_bodies: {live} asteroids survived UnloadScenario - the swarm did \
         not return to baseline"
    );
    info!("many_bodies: teardown returned to baseline");
}

/// Rebuild the swarm for the next capture cycle, the way the loop point does.
#[cfg(feature = "debug")]
fn respawn_the_swarm(world: &mut World) {
    let (Some(game_assets), Some(size)) = (
        world.get_resource::<GameAssets>().cloned(),
        world.get_resource::<SwarmSize>().copied(),
    ) else {
        return;
    };
    nova_probe::capture_reload_begin(world);
    world.trigger(LoadScenario(swarm_scenario(&game_assets, size.0)));
}
