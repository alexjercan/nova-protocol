//! many_projectiles: a turret ship firing into a field of N asteroids, held
//! under saturation, drained, torn down, and spawned again.
//!
//! The scale sweep for PROJECTILE churn. `many_bodies` and `many_sections`
//! scale a static population; this one scales the rate at which entities are
//! created and destroyed. The turret fires 100 rounds/s with a 5 s round
//! lifetime, so the steady state is several hundred sensor-collider bodies in
//! flight at once, each testing against the target field, each despawning on
//! impact or expiry - collision pairs, impact particles and despawn churn all
//! at saturation rather than at rest.
//!
//! It makes a correctness claim alongside the number. Two, in fact:
//! - the run actually SATURATED - the peak live-round count has to clear
//!   [`MIN_PEAK_ROUNDS`], so a silently dud turret (safety never went hot, the
//!   trigger edge got eaten) fails loudly instead of reporting a fast frame
//!   time for a scene that fired nothing;
//! - the churn comes BACK to zero - once the trigger is released every round
//!   must drain, and after the unload not one round, rock or ship root may
//!   survive (task 20260804-094006, DECISION.md D3 - an in-example assertion,
//!   not a probe check, because the subject is ECS entity counts).
//!
//! The drain is asserted BEFORE the teardown, on purpose: rounds are not
//! scenario-scoped entities. They carry `TempEntity(projectile_lifetime)` and
//! outlive an `UnloadScenario` until that expires, so "the volley cleared"
//! is a claim about the projectile lifecycle itself, which unloading the
//! scene would hide rather than prove.
//!
//! The script is enrolled in capture looping, so `--fps` measures spawn ->
//! saturate -> drain -> teardown ACTIVITY rather than an idle tail.
//!
//! Count knob: `NOVA_STRESS_COUNT` overrides [`DEFAULT_COUNT`] (the number of
//! target rocks; the round count follows from the fire rate).
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example many_projectiles --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `many_projectiles: N targets up`,
//! #           `many_projectiles: peak N rounds in flight`,
//! #           `many_projectiles: teardown returned to baseline`,
//! #           `autopilot: cycle complete, no panic`
//!
//! # the frame-time sweep:
//! cargo run -p nova_probe_cli -- run many_projectiles --fps --release
//! ```

use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "many_projectiles")]
#[command(version = "1.0.0")]
#[command(about = "Scale sweep: turret saturation into a field of N targets", long_about = None)]
struct Cli;

/// How many target rocks the field holds when `NOVA_STRESS_COUNT` is unset.
///
/// Measured under the CI-shaped raster (lavapipe/llvmpipe, 1280x720, dev
/// profile, 20 warm-up + 120 captured frames):
///
/// | count | mean ms | mean fps | peak rounds |
/// | ----- | ------- | -------- | ----------- |
/// | 60    | 64.2    | 15.6     | 307         |
/// | 120   | 58.0    | 17.2     | 89          |
/// | 240   | 779.8   | 1.3      | 84          |
///
/// 120 is the pick: the full 180 + 900 frame capture window fills in ~65 s,
/// well inside probe's 585 s window-sized completion deadline, and the field is
/// dense enough that the turret stream is hitting something most of the time -
/// which is the point, since a round that never collides exercises neither the
/// impact particles nor the despawn path. That density shows up in the peak
/// column: at 60 targets the stream mostly MISSES and rounds pile up to their
/// lifetime ceiling, while from 120 on they are being consumed on impact.
///
/// Note the cliff at 240. Software raster falls to 1.3 fps there, below the
/// 2 fps floor probe sizes its completion deadline against, so a run at that
/// count will TIME OUT on a CI-shaped box rather than merely being slow. The
/// knob is still there for anyone measuring on real hardware; the default is
/// chosen to stay well clear of it.
const DEFAULT_COUNT: usize = 120;

/// Env var that overrides [`DEFAULT_COUNT`]. Shared with the other `stress/`
/// sweeps so `probe run stress` scales all of them together.
const COUNT_ENV: &str = "NOVA_STRESS_COUNT";

/// Radius of the target shell. Close enough that the turret stream reaches it
/// well inside the 5 s round lifetime (rounds leave the barrel at ~100 u/s), so
/// the steady state is rounds IMPACTING rather than rounds expiring in transit.
const SHELL_RADIUS: f32 = 90.0;

/// Radius of every target rock.
const ROCK_RADIUS: f32 = 4.0;

/// Health of every target rock. High enough that the saturation cannot destroy
/// one: the target count is part of the measured scene, so it must not drift
/// mid-window or the frame times stop being comparable between cycles.
const ROCK_HEALTH: f32 = 1.0e9;

/// The ship's turret section id, and the key the input mapping binds to it.
const GUNS_SLOT: &str = "guns";

/// The lowest peak round count that still counts as SATURATION.
///
/// The turret fires 100 rounds/s and rounds live 5 s, so an unobstructed stream
/// would pile up toward 500 in flight; at the default field density impacts
/// consume them far sooner, and the measured peak is ~85 (see the table on
/// [`DEFAULT_COUNT`]). 50 is a floor that a genuinely firing turret clears
/// inside the first second and a dud one never leaves 0 for - it is a dud
/// detector, not a performance bound, so it deliberately sits below the
/// steady state rather than tracking it. It held at every count measured,
/// including the 1.3 fps one.
#[cfg(feature = "debug")]
const MIN_PEAK_ROUNDS: usize = 50;

/// The scenario id the range loads under.
const SCENARIO_ID: &str = "stress_many_projectiles";

/// The first step's name, so `loop_from` restarts the cycle at the spawn
/// without repeating the string.
#[cfg(feature = "debug")]
const LOAD_STEP: &str = "spawn the range";

/// How long the trigger is held down. The steady-state stretch the frame-time
/// capture is actually interested in.
#[cfg(feature = "debug")]
const SATURATE_SECS: f32 = 3.0;

/// Settle window between the unload and the baseline assertion: `UnloadScenario`
/// despawns on the next command flush, and a couple of frames under llvmpipe is
/// generous for that.
#[cfg(feature = "debug")]
const TEARDOWN_SETTLE_SECS: f32 = 0.5;

/// How long the drain step waits for the last round to clear. Comfortably above
/// the 5 s round lifetime, which is the true bound: once the trigger is
/// released nothing new is fired, so every round either hits or expires.
#[cfg(feature = "debug")]
const DRAIN_DEADLINE_SECS: f32 = 15.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let count = target_count();
    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            app.insert_resource(FieldSize(count));
            app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
        })
        .build();

    // Headless smoke-test harness: inert in a normal run. Under NOVA_AUTOPILOT
    // it spawns the range, saturates it, drains it, tears it down and asserts
    // the world came back to baseline - the correctness half of a run whose
    // other half is a frame-time number.
    #[cfg(feature = "debug")]
    {
        app.init_resource::<HeldInput>();
        app.init_resource::<PeakRounds>();
        app.add_systems(Update, track_peak_rounds);
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                .input(hold_inputs)
                // The field is spawned by an OnStart handler, so waiting for
                // the full target count is also the gate a looped reload needs:
                // the old cycle's entities outlive the load replacing them.
                .step(LOAD_STEP)
                .enter(GameStates::Loading)
                .until(field_is_up(count))
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
                    report_field_up(world);
                })
                .add()
                // Raise the stance and WAIT for the safety to actually go hot.
                // Pressing fire while cold latches nothing, forever: the safety
                // denies a press that arrives cold, and a held key produces no
                // fresh edge once hot (the lesson `torpedo_section` paid for in
                // task 20260712-211352).
                .step("raise the weapons")
                .on_enter(|world: &mut World| world.resource_mut::<HeldInput>().combat = true)
                .until(weapons_are_hot())
                .deadline(15.0)
                .add()
                .step("saturate the field")
                .on_enter(|world: &mut World| world.resource_mut::<HeldInput>().fire = true)
                .until(elapsed(SATURATE_SECS))
                .add()
                .step("let the volley drain")
                .on_enter(cease_fire)
                .until(no_rounds_in_flight())
                .deadline(DRAIN_DEADLINE_SECS)
                .add()
                .step("assert the range saturated and drained")
                .on_enter(assert_saturated_and_drained)
                .add()
                .step("tear the range down")
                .on_enter(tear_the_range_down)
                .until(elapsed(TEARDOWN_SETTLE_SECS))
                .add()
                .step("check the world came back to baseline")
                .on_enter(assert_back_to_baseline)
                .add()
                // Enrolled in capture looping (task 20260720-000616): when a
                // frame capture outlives the script, the cycle replays, so the
                // capture measures spawn/saturate/drain activity rather than an
                // idle tail.
                .loop_from(LOAD_STEP)
                .on_loop(respawn_the_range),
        );
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        app.add_plugins(nova_probe::nova_timeline());
        app.add_plugins(nova_probe::nova_invariants());
        app.add_plugins(nova_probe::nova_frametime());
        app.add_plugins(nova_screenshot());
    }

    app.run()
}

/// How many targets this run spawns, so the script's gates and the scenario
/// builder agree without re-reading the environment.
#[derive(Resource, Clone, Copy)]
struct FieldSize(usize);

/// The target count for this run: `NOVA_STRESS_COUNT` if it parses, else
/// [`DEFAULT_COUNT`]. Read once, in `main`.
fn target_count() -> usize {
    std::env::var(COUNT_ENV)
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(DEFAULT_COUNT)
        // A range with nothing to shoot at measures the wrong thing.
        .max(1)
}

fn setup_range(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    size: Res<FieldSize>,
) {
    commands.trigger(LoadScenario(range_scenario(
        &game_assets,
        &sections,
        size.0,
    )));
}

/// The ship: a controller so it holds attitude, a hull, the turret under test,
/// and a torpedo bay.
///
/// The bay is CARRIED but not bound to a key. It belongs here - the category's
/// subject is a saturating weapons ship, and the bay's mass, collider and
/// integrity node are part of that cost - but firing it would make the run
/// unbounded: a torpedo carries a 100 s lifetime against the turret round's
/// 5 s, so a single torpedo that missed would hold the drain gate open long
/// past any deadline worth setting. The turret alone saturates at 100
/// rounds/s, which is the churn this example exists to measure.
fn range_ship(sections: &GameSections) -> SpaceshipConfig {
    fixtures::ship(
        sections,
        SpaceshipController::Player(PlayerControllerConfig {
            // The shipped fire bindings: the base scenarios map turrets to
            // LMB / RightTrigger2.
            input_mapping: HashMap::from([(
                GUNS_SLOT.to_string(),
                vec![
                    MouseButton::Left.into(),
                    GamepadButton::RightTrigger2.into(),
                ],
            )]),
            speed_cap: None,
            // Saturation harness: the magazine is not the subject.
            infinite_ammo: true,
            lock_refire_secs: None,
        }),
        &[
            SectionSpec::new("controller", "basic_controller_section", Vec3::ZERO),
            SectionSpec::new("hull", "reinforced_hull_section", Vec3::new(0.0, 0.0, 1.0)),
            SectionSpec::new(
                GUNS_SLOT,
                "better_turret_section",
                Vec3::new(0.0, 1.0, -1.0),
            ),
            SectionSpec::new("torpedo", "torpedo_section", Vec3::new(0.0, 0.0, -1.0)),
        ],
    )
}

/// Where target `i` of `count` sits: a Fibonacci-sphere shell of fixed radius,
/// which spreads the field evenly over the sphere for any count. Purely a
/// function of `(i, count)`, so a rerun at the same knob builds the same range
/// and the frame-time numbers stay comparable.
fn target_position(i: usize, count: usize) -> Vec3 {
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    let y = 1.0 - (i as f32 / (count.max(2) - 1) as f32) * 2.0;
    let ring = (1.0 - y * y).max(0.0).sqrt();
    let theta = golden * i as f32;
    Vec3::new(theta.cos() * ring, y, theta.sin() * ring) * SHELL_RADIUS
}

/// The range: the turret ship at the origin inside a shell of `count` targets.
fn range_scenario(
    game_assets: &GameAssets,
    sections: &GameSections,
    count: usize,
) -> ScenarioConfig {
    let mut objects = vec![ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "range_ship".to_string(),
            name: "Range Ship".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(range_ship(sections)),
    }];
    objects.extend((0..count).map(|i| {
        fixtures::asteroid(
            game_assets,
            &format!("target_{i}"),
            &format!("Target {i}"),
            target_position(i, count),
            ROCK_RADIUS,
            ROCK_HEALTH,
            // Unsigned: the stream is fired over open sights, never radar-locked.
            None,
        )
    }));

    ScenarioConfig {
        id: SCENARIO_ID.to_string(),
        name: "Many Projectiles".to_string(),
        description: "A turret ship saturating a field of asteroid targets.".to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        // The scene lights itself: the engine spawns no light, so a scenario
        // that authors none renders black.
        events: fixtures::spawn_on_start(
            [
                objects,
                ThreePointRig::around("field", Vec3::ZERO, 10.0).objects(),
            ]
            .concat(),
        ),
        ..Default::default()
    }
}

/// The inputs the script holds down. A resource re-applied every frame rather
/// than a one-shot press: `ButtonInput` is a live map the game's own systems
/// read every frame, and the weapons safety derives `WeaponsHot` from the HELD
/// combat input, so a press applied once on a step's entry would be a press,
/// not a hold.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct HeldInput {
    /// RMB - the combat stance the safety gates firing on.
    combat: bool,
    /// LMB - the trigger.
    fire: bool,
}

/// Re-press the held inputs for the frame.
///
/// Wired as the script's `input` hook, NOT as an `Update` system: that slot
/// runs in `PreUpdate` after `InputSystems`, which is the only place a
/// synthesized press is still `just_pressed` when the game's `Update` input
/// systems read it. As an unordered `Update` system it races
/// `SpaceshipInputSystems` and the trigger edge is cleared before the weapon
/// sees it (the failure `torpedo_section` documents).
#[cfg(feature = "debug")]
fn hold_inputs(world: &mut World, _elapsed: f32) {
    let held = world.resource::<HeldInput>();
    let (combat, fire) = (held.combat, held.fire);
    if combat {
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
    }
    if fire {
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
    }
}

/// The high-water mark of rounds in flight this cycle, so the saturation claim
/// is about the PEAK rather than about whatever happened to be alive on the one
/// frame an assertion looked.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct PeakRounds(usize);

/// How many turret rounds are alive right now.
#[cfg(feature = "debug")]
fn live_rounds(world: &World) -> usize {
    world
        .iter_entities()
        .filter(|entity| entity.contains::<TurretBulletProjectileMarker>())
        .count()
}

/// How many target rocks are alive right now.
#[cfg(feature = "debug")]
fn live_targets(world: &World) -> usize {
    world
        .iter_entities()
        .filter(|entity| entity.contains::<AsteroidMarker>())
        .count()
}

/// Every entity the teardown claim covers: rounds, targets and ship roots.
#[cfg(feature = "debug")]
fn live_counts(world: &World) -> (usize, usize, usize) {
    let roots = world
        .iter_entities()
        .filter(|entity| entity.contains::<SpaceshipRootMarker>())
        .count();
    (live_rounds(world), live_targets(world), roots)
}

/// Track the high-water mark. A cheap query per frame, which is what it takes
/// to see a peak that a per-step sample would miss between frames.
#[cfg(feature = "debug")]
fn track_peak_rounds(
    q_rounds: Query<(), With<TurretBulletProjectileMarker>>,
    mut peak: ResMut<PeakRounds>,
) {
    peak.0 = peak.0.max(q_rounds.iter().count());
}

/// Advance once the whole target field is in the world.
#[cfg(feature = "debug")]
fn field_is_up(count: usize) -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(move |world: &World| live_targets(world) >= count)
}

/// The player ship's weapons safety has gone hot.
#[cfg(feature = "debug")]
fn weapons_are_hot() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&WeaponsHot, With<PlayerSpaceshipMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|hot| hot.0))
    })
}

/// Every round has hit or expired.
#[cfg(feature = "debug")]
fn no_rounds_in_flight() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| live_rounds(world) == 0)
}

/// Log the live count, and push it as a timeline marker so an armed run can see
/// the field rise and fall in the same JSONL stream as the frame times.
#[cfg(feature = "debug")]
fn report_field_up(world: &mut World) {
    let targets = live_targets(world);
    nova_probe::probe_marker(
        world,
        "stress: field up",
        serde_json::json!({ "targets": targets }),
    );
    info!("many_projectiles: {targets} targets up");
}

/// RELEASE the trigger, do not merely stop re-pressing it. `ButtonInput` only
/// yields a fresh `just_pressed` on a not-pressed -> pressed edge, and nothing
/// else releases a synthesized button: leaving it latched would make the next
/// cycle's press a no-op, so the looped run would fire exactly once and then
/// measure a silent scene forever.
#[cfg(feature = "debug")]
fn cease_fire(world: &mut World) {
    world.resource_mut::<HeldInput>().fire = false;
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
}

/// The saturation claim. Panics rather than warning: a run that fired nothing
/// would otherwise report an honest-looking frame time for a scene that did
/// none of the work the example exists to measure.
#[cfg(feature = "debug")]
fn assert_saturated_and_drained(world: &mut World) {
    let peak = world.resource::<PeakRounds>().0;
    let live = live_rounds(world);
    nova_probe::probe_marker(
        world,
        "stress: volley drained",
        serde_json::json!({ "peak": peak, "live": live }),
    );
    assert!(
        peak >= MIN_PEAK_ROUNDS,
        "many_projectiles: peaked at only {peak} rounds in flight (want >= \
         {MIN_PEAK_ROUNDS}) - the turret never saturated, so the frame time \
         measures an idle scene"
    );
    assert_eq!(
        live, 0,
        "many_projectiles: {live} rounds still in flight after the drain gate \
         opened"
    );
    info!("many_projectiles: peak {peak} rounds in flight, volley drained");
}

/// Drop the whole scene. `UnloadScenario` guarantees every scenario-scoped
/// entity goes, which is exactly the claim the next step checks.
#[cfg(feature = "debug")]
fn tear_the_range_down(world: &mut World) {
    nova_probe::probe_marker(world, "stress: teardown", serde_json::json!({}));
    world.trigger(UnloadScenario);
}

/// The correctness claim: the teardown left NOTHING behind. Panics rather than
/// stalling a predicate, so a leak fails the smoke test with the counts that
/// survived instead of timing out anonymously.
#[cfg(feature = "debug")]
fn assert_back_to_baseline(world: &mut World) {
    let (rounds, targets, roots) = live_counts(world);
    nova_probe::probe_marker(
        world,
        "stress: baseline",
        serde_json::json!({ "rounds": rounds, "targets": targets, "roots": roots }),
    );
    assert_eq!(
        (rounds, targets, roots),
        (0, 0, 0),
        "many_projectiles: {rounds} rounds, {targets} targets and {roots} ship \
         roots survived UnloadScenario - the range did not return to baseline"
    );
    info!("many_projectiles: teardown returned to baseline");
}

/// Rebuild the range for the next capture cycle, the way the loop point does.
/// The peak resets with it: the saturation claim is per-cycle, so a later cycle
/// that fired nothing must not pass on an earlier one's high-water mark.
#[cfg(feature = "debug")]
fn respawn_the_range(world: &mut World) {
    let (Some(game_assets), Some(sections), Some(size)) = (
        world.get_resource::<GameAssets>().cloned(),
        world.get_resource::<GameSections>().cloned(),
        world.get_resource::<FieldSize>().copied(),
    ) else {
        return;
    };
    world.resource_mut::<PeakRounds>().0 = 0;
    nova_probe::capture_reload_begin(world);
    world.trigger(LoadScenario(range_scenario(
        &game_assets,
        &sections,
        size.0,
    )));
}
