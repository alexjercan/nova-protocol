//! stress_bullets: a thousand turret rounds in the sky at once, then none.
//!
//! The projectile lifecycle at a scale no fight will ever reach. A battery of
//! [`TURRET_MOUNTS`] mounts fires into an EMPTY sky, so nothing is consumed by
//! impacts and the population is the spawn/expire cycle alone: every round is a
//! body with a sensor collider, a render mesh and a `TempEntity` lifetime, and
//! the range holds [`BULLETS_IN_FLIGHT`] of them at once.
//!
//! What it claims, beside the frame time:
//!
//! - the battery is WHOLE - exactly the mounts it was built with, so a run that
//!   quietly lost half its guns cannot report an easy number;
//! - the sky actually FILLED - the peak live-round count clears
//!   [`BULLETS_IN_FLIGHT`], so a dud trigger (safety never went hot, the edge
//!   got eaten) fails loudly instead of measuring an empty scene;
//! - the volley CLEARS - once the trigger is released every round retires on its
//!   own lifetime, and the count is exactly zero before anything is torn down;
//! - the teardown leaves NOTHING - not a round, not a section, not a root.
//!
//! The drain is asserted BEFORE the teardown, on purpose: a round retires on its
//! own `TempEntity(projectile_lifetime)`, so "the volley cleared" is a claim
//! about the projectile lifecycle itself. The teardown also takes every round,
//! so asserting only after the unload would hide a stuck round rather than
//! prove it drained.
//!
//! The script is enrolled in capture looping, so a frame capture measures
//! fill -> hold -> drain -> teardown ACTIVITY rather than an idle tail.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example stress_bullets --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `stress_bullets: N mounts up`,
//! #           `stress_bullets: peak N rounds in the sky`,
//! #           `stress_bullets: teardown returned to baseline`,
//! #           `autopilot: cycle complete, no panic`
//!
//! # with the frame-time capture armed:
//! cargo run --features debug probe run stress_bullets
//! ```

use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "stress_bullets")]
#[command(version = "1.0.0")]
#[command(about = "Stress range: a thousand turret rounds in flight at once. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// ABSURD SCALE, and deliberately so: this number must never reflect real
/// content. A fight is decided by a handful of mounts; a thousand rounds in the
/// air at once is a load the code has to survive, not a ship anyone will fly.
/// Read it as a ceiling the lifecycle is proven under, never as a balance
/// figure.
#[cfg(feature = "debug")]
const BULLETS_IN_FLIGHT: usize = 1000;

/// ABSURD SCALE (see [`BULLETS_IN_FLIGHT`]): eight mounts on one small hull.
///
/// The shared PDC fires 100 rounds/s with a 2 s round lifetime, so one
/// mount saturates at ~200 rounds in an empty sky and eight clear the thousand
/// with headroom - the range reaches its scale in a couple of seconds instead
/// of spending the whole completion budget climbing to it.
const TURRET_MOUNTS: usize = 8;

/// The sections the hull carries: one spine cell per mount (the flight
/// computer takes the middle one) plus the mounts themselves. The exact-count
/// invariant reads this.
#[cfg(feature = "debug")]
const SECTIONS_ON_THE_HULL: usize = 2 * TURRET_MOUNTS;

/// The turret content id every mount is built from.
const MOUNT_SECTION: &str = "pdc_kinetic_turret_section";

/// Where a mount's centre sits above the spine cell it stands on: half a cell
/// out to that cell's top face, then a quarter for the PDC's own base plate.
const MOUNT_SEAT: f32 = 0.75;

/// The scenario id the range loads under.
const SCENARIO_ID: &str = "stress_bullets";

/// The first step's name, so `loop_from` restarts the cycle at the spawn
/// without repeating the string.
#[cfg(feature = "debug")]
const LOAD_STEP: &str = "spawn the battery";

/// How long the sky is held full once the scale is reached: the steady-state
/// stretch a frame capture is actually interested in.
#[cfg(feature = "debug")]
const HOLD_SECS: f32 = 2.0;

/// Settle window between the unload and the baseline assertion: `UnloadScenario`
/// despawns on the next command flush, and a couple of frames under llvmpipe is
/// generous for that.
#[cfg(feature = "debug")]
const TEARDOWN_SETTLE_SECS: f32 = 0.5;

/// Deadlines, in the clamped sim seconds every gate here counts in. Generous
/// for llvmpipe, where a frame of this scene is hundreds of milliseconds, and
/// still summing well under the 120 s harness completion backstop.
#[cfg(feature = "debug")]
const SPAWN_DEADLINE_SECS: f32 = 45.0;
#[cfg(feature = "debug")]
const HOT_DEADLINE_SECS: f32 = 15.0;
#[cfg(feature = "debug")]
const FILL_DEADLINE_SECS: f32 = 30.0;
/// Comfortably above the 2 s round lifetime, which is the true bound: once the
/// trigger is released nothing new is fired, so every round expires.
#[cfg(feature = "debug")]
const DRAIN_DEADLINE_SECS: f32 = 20.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new()
        .with_game_plugins(|app: &mut App| {
            app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
        })
        .build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<HeldInput>();
        app.init_resource::<PeakRounds>();
        app.add_systems(Update, track_peak_rounds);
        // The picture is taken at the range's PEAK. `nova_screenshot`
        // appends its beat to whatever it is handed, and the beats after
        // this call drain and tear the range down - a shot behind them
        // photographs an empty world.
        app.add_plugins(
            nova_screenshot(
                nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                    .input(hold_inputs)
                    // The battery is spawned by an OnStart handler, so waiting for
                    // the full section count is also the gate a looped reload
                    // needs: the old cycle's entities outlive the load replacing
                    // them.
                    .step(LOAD_STEP)
                    .enter(GameStates::Loading)
                    .until(the_battery_is_up())
                    .deadline(SPAWN_DEADLINE_SECS)
                    .add()
                    // The scene is live again: close the reload interval so a frame
                    // capture excludes it. A no-op on the first cycle.
                    .step("close the reload interval")
                    .on_enter(nova_probe::capture_reload_end)
                    .on_enter(assert_the_battery_is_whole)
                    .add()
                    // Raise the stance and WAIT for the safety to go hot. Pressing
                    // fire while cold latches nothing, forever: the safety denies a
                    // press that arrives cold, and a held button produces no fresh
                    // edge once hot.
                    .step("raise the weapons")
                    .on_enter(|world: &mut World| world.resource_mut::<HeldInput>().combat = true)
                    .until(weapons_are_hot())
                    .deadline(HOT_DEADLINE_SECS)
                    .add()
                    .step("fill the sky")
                    .on_enter(|world: &mut World| world.resource_mut::<HeldInput>().fire = true)
                    .until(the_sky_is_full())
                    .deadline(FILL_DEADLINE_SECS)
                    .add()
                    .step("hold the saturation")
                    .until(elapsed(HOLD_SECS))
                    .add()
                    .step("assert the sky filled")
                    .on_enter(assert_the_sky_filled)
                    .add(),
            )
            .step("let the volley drain")
            .on_enter(cease_fire)
            .until(no_rounds_in_flight())
            .deadline(DRAIN_DEADLINE_SECS)
            .add()
            .step("assert the volley drained")
            .on_enter(assert_the_volley_drained)
            .add()
            .step("tear the range down")
            .on_enter(tear_the_range_down)
            .until(elapsed(TEARDOWN_SETTLE_SECS))
            .add()
            .step("check the world came back to baseline")
            .on_enter(assert_back_to_baseline)
            .add()
            .loop_from(LOAD_STEP)
            .on_loop(respawn_the_range),
        );
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        app.add_plugins(nova_probe::NovaProbePlugin::default());
    }

    app.run()
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(range_scenario(&game_assets, &sections)));
}

/// The battery: a controller, a hull block, and [`TURRET_MOUNTS`] mounts in a
/// contiguous row, every one of them bound to the same trigger.
///
/// The row shares faces with its neighbours and with the controller, so the
/// integrity graph gets one connected structure - a cloud of islands would
/// spend the run coming apart instead of firing.
fn battery(sections: &GameSections) -> SpaceshipConfig {
    // A spine of unit cells along X, one per mount, with the flight computer
    // taking the middle cell; every mount stands on its own cell's top face.
    // The mounts used to sit in a bare row with nothing under them, chained
    // side to side through the old unit-cube turret's six sockets. The shared
    // PDC bolts down by its base plate and by nothing else, so a row like that
    // mates nothing - each mount needs a cell of its own to stand on.
    let cell = |i: usize| i as f32 - (TURRET_MOUNTS as f32) / 2.0;
    let computer = TURRET_MOUNTS / 2;
    let mut specs = vec![SectionSpec::new(
        "controller",
        "basic_controller_section",
        Vec3::new(cell(computer), 0.0, 0.0),
    )];
    specs.extend((0..TURRET_MOUNTS).filter(|&i| i != computer).map(|i| {
        SectionSpec::new(
            format!("spine_{i}"),
            "reinforced_hull_section",
            Vec3::new(cell(i), 0.0, 0.0),
        )
    }));
    specs.extend((0..TURRET_MOUNTS).map(|i| {
        SectionSpec::new(
            mount_slot(i),
            MOUNT_SECTION,
            Vec3::new(cell(i), MOUNT_SEAT, 0.0),
        )
    }));

    fixtures::ship(
        sections,
        SpaceshipController::Player(PlayerControllerConfig {
            // Every mount on the shipped fire binding: the battery is one
            // trigger, not a loadout.
            input_mapping: (0..TURRET_MOUNTS)
                .map(|i| {
                    (
                        mount_slot(i),
                        vec![
                            MouseButton::Left.into(),
                            GamepadButton::RightTrigger2.into(),
                        ],
                    )
                })
                .collect::<HashMap<_, _>>(),
            speed_cap: None,
            // Saturation harness: the magazine is not the subject.
            infinite_ammo: true,
        }),
        &specs,
    )
}

/// The per-ship slot id of mount `i`, shared by the layout and the bindings.
fn mount_slot(i: usize) -> String {
    format!("mount_{i}")
}

/// The range: the battery alone in an empty sky.
///
/// Nothing to shoot AT, deliberately. A target field would consume rounds on
/// impact, and the population would then measure the impact path rather than
/// the spawn/expire cycle this range exists to hold at scale.
fn range_scenario(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let objects = vec![ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "battery".to_string(),
            name: "Battery".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(battery(sections)),
    }];

    ScenarioConfig {
        description: "A battery holding a thousand rounds in an empty sky.".to_string(),
        hidden: true,
        // The scene lights itself: the engine spawns no light, so a scenario
        // that authors none renders black.
        events: fixtures::spawn_on_start(
            [
                objects,
                ThreePointRig::around("lights", Vec3::ZERO, 12.0).objects(),
            ]
            .concat(),
        ),
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Stress: Bullets".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The inputs the script holds down. A resource re-applied every frame rather
/// than a one-shot press: `ButtonInput` is a live map the game's own systems
/// read every frame, and the weapons safety derives `WeaponsHot` from the HELD
/// combat input.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct HeldInput {
    /// RMB - the combat stance the safety gates firing on.
    combat: bool,
    /// LMB - the trigger.
    fire: bool,
}

/// Re-press the held inputs for the frame. Wired as the script's `input` hook,
/// NOT as an `Update` system: that slot runs in `PreUpdate` after
/// `InputSystems`, the only place a synthesized press is still `just_pressed`
/// when the game's `Update` input systems read it.
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

/// The high-water mark of rounds in flight this cycle, so the scale claim is
/// about the PEAK rather than about whatever happened to be alive on the one
/// frame an assertion looked.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct PeakRounds(usize);

/// Track the high-water mark. A cheap count per frame, which is what it takes
/// to see a peak a per-step sample would miss between frames.
#[cfg(feature = "debug")]
fn track_peak_rounds(
    q_rounds: Query<(), With<TurretBulletProjectileMarker>>,
    mut peak: ResMut<PeakRounds>,
) {
    peak.0 = peak.0.max(q_rounds.iter().count());
}

/// How many turret rounds are alive right now.
#[cfg(feature = "debug")]
fn live_rounds(world: &World) -> usize {
    world
        .iter_entities()
        .filter(|entity| entity.contains::<TurretBulletProjectileMarker>())
        .count()
}

/// Live sections and ship roots - what the teardown claim covers beside the
/// rounds.
#[cfg(feature = "debug")]
fn live_structure(world: &World) -> (usize, usize) {
    let sections = world
        .iter_entities()
        .filter(|entity| entity.contains::<SectionMarker>())
        .count();
    let roots = world
        .iter_entities()
        .filter(|entity| entity.contains::<SpaceshipRootMarker>())
        .count();
    (sections, roots)
}

/// Advance once the whole battery is in the world.
#[cfg(feature = "debug")]
fn the_battery_is_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| live_structure(world).0 >= SECTIONS_ON_THE_HULL)
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

/// The sky holds the range's whole scale at once.
#[cfg(feature = "debug")]
fn the_sky_is_full() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| live_rounds(world) >= BULLETS_IN_FLIGHT)
}

/// Every round has expired.
#[cfg(feature = "debug")]
fn no_rounds_in_flight() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| live_rounds(world) == 0)
}

/// The battery is EXACTLY what it was built with. A range that lost mounts
/// would still fill a sky, only slower, and report a number for a scene nobody
/// authored.
#[cfg(feature = "debug")]
fn assert_the_battery_is_whole(world: &mut World) {
    let (sections, roots) = live_structure(world);
    let mounts = world
        .iter_entities()
        .filter(|entity| entity.contains::<TurretSectionMarker>())
        .count();
    assert_eq!(
        (sections, roots, mounts),
        (SECTIONS_ON_THE_HULL, 1, TURRET_MOUNTS),
        "stress_bullets: the range must stand up as exactly one hull of \
         {SECTIONS_ON_THE_HULL} sections carrying {TURRET_MOUNTS} mounts"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the battery assembled every mount",
        serde_json::json!({ "sections": sections, "mounts": mounts }),
    );
    info!("stress_bullets: {mounts} mounts up on {sections} sections");
}

/// The scale claim. Panics rather than warning: a run that fired nothing would
/// otherwise report an honest-looking frame time for a scene that did none of
/// the work the range exists to measure.
#[cfg(feature = "debug")]
fn assert_the_sky_filled(world: &mut World) {
    let peak = world.resource::<PeakRounds>().0;
    let live = live_rounds(world);
    assert!(
        peak >= BULLETS_IN_FLIGHT,
        "stress_bullets: peaked at only {peak} rounds in flight (want >= \
         {BULLETS_IN_FLIGHT}) - the battery never reached the range's scale"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a thousand rounds in the sky at once",
        serde_json::json!({ "peak": peak, "live": live }),
    );
    info!("stress_bullets: peak {peak} rounds in the sky, {live} live now");
}

/// RELEASE the trigger, do not merely stop re-pressing it. `ButtonInput` only
/// yields a fresh `just_pressed` on a not-pressed -> pressed edge, and nothing
/// else releases a synthesized button: leaving it latched would make the next
/// cycle's press a no-op, so a looped run would fire once and then measure a
/// silent scene forever.
#[cfg(feature = "debug")]
fn cease_fire(world: &mut World) {
    world.resource_mut::<HeldInput>().fire = false;
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
}

/// The lifecycle claim: the sky came back to EXACTLY zero on the rounds' own
/// lifetimes, before anything was torn down.
#[cfg(feature = "debug")]
fn assert_the_volley_drained(world: &mut World) {
    let live = live_rounds(world);
    assert_eq!(
        live, 0,
        "stress_bullets: {live} rounds still in flight after the drain gate \
         opened - a round outlived its own lifetime"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the volley drained to nothing",
        serde_json::json!({ "live": live }),
    );
    info!("stress_bullets: the volley drained to zero");
}

/// Drop the whole scene. `UnloadScenario` guarantees every scenario-scoped
/// entity goes, which is exactly the claim the next step checks.
#[cfg(feature = "debug")]
fn tear_the_range_down(world: &mut World) {
    nova_probe::probe_marker(world, "stress: teardown", serde_json::json!({}));
    world.trigger(UnloadScenario);
}

/// The teardown claim: nothing survived. Panics rather than stalling a
/// predicate, so a leak fails with the counts that survived instead of timing
/// out anonymously.
#[cfg(feature = "debug")]
fn assert_back_to_baseline(world: &mut World) {
    let rounds = live_rounds(world);
    let (sections, roots) = live_structure(world);
    assert_eq!(
        (rounds, sections, roots),
        (0, 0, 0),
        "stress_bullets: {rounds} rounds, {sections} sections and {roots} ship \
         roots survived UnloadScenario - the range did not return to baseline"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the teardown left nothing behind",
        serde_json::json!({ "rounds": rounds, "sections": sections, "roots": roots }),
    );
    info!("stress_bullets: teardown returned to baseline");
}

/// Rebuild the range for the next capture cycle, the way the loop point does.
/// The peak resets with it: the scale claim is per-cycle, so a later cycle that
/// fired nothing must not pass on an earlier one's high-water mark.
#[cfg(feature = "debug")]
fn respawn_the_range(world: &mut World) {
    let (Some(game_assets), Some(sections)) = (
        world.get_resource::<GameAssets>().cloned(),
        world.get_resource::<GameSections>().cloned(),
    ) else {
        return;
    };
    world.resource_mut::<PeakRounds>().0 = 0;
    nova_probe::capture_reload_begin(world);
    world.trigger(LoadScenario(range_scenario(&game_assets, &sections)));
}
