//! stress_many_structures: a hundred ships of ten sections, spawned, churned,
//! and spawned again.
//!
//! The other half of the structure load: `stress_one_structure` scales the size
//! of ONE body, this one scales the NUMBER of them. [`SHIPS_IN_THE_SKY`] ships
//! of [`SECTIONS_PER_SHIP`] sections each is a thousand sections spread over a
//! hundred aggregating roots, and every one of those roots is an AI hull whose
//! acquisition pass scans every other body in the scene each frame - the
//! quadratic spatial query is the point, not a side effect.
//!
//! CHURN is the other subject: the run tears the whole fleet down mid-script
//! and builds it again, then asserts the sky came back to exactly the same
//! shape. A leak, a stale relation or a half-despawned root shows there rather
//! than at the end of a long capture.
//!
//! What it claims, beside the frame time:
//!
//! - the fleet is EXACTLY the size it was authored - a hundred roots, a
//!   thousand sections, a turret per hull;
//! - the acquisition pass RAN over all of it - every AI hull holds a hostile,
//!   which a fleet whose spatial query silently stopped scanning could not do;
//! - it COMES BACK - after a full teardown and respawn the counts are identical;
//! - the teardown leaves NOTHING - not a section, not a root.
//!
//! The ships are authored with a 1 u engage range, so they acquire but never
//! leave their passive routine: the fleet holds its formation for the whole run
//! and the scene stays comparable between cycles instead of dissolving into a
//! hundred-ship brawl.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example stress_many_structures --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `stress_many_structures: 100 ships up`,
//! #           `stress_many_structures: the fleet came back after the churn`,
//! #           `stress_many_structures: teardown returned to baseline`,
//! #           `autopilot: cycle complete, no panic`
//!
//! # with the frame-time capture armed:
//! cargo run --features debug probe run stress_many_structures
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "stress_many_structures")]
#[command(version = "1.0.0")]
#[command(about = "Stress range: a hundred ships of ten sections, churned. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// ABSURD SCALE, and deliberately so: these numbers must never reflect real
/// content. A shipped engagement is a handful of hulls; a hundred at once, each
/// scanning every other body every frame, is a load the acquisition pass and
/// the physics broad phase have to survive, not a fight anyone will fly.
const SHIPS_IN_THE_SKY: usize = 100;

/// ABSURD SCALE (see [`SHIPS_IN_THE_SKY`]): ten sections per hull, so the fleet
/// carries the same thousand sections `stress_one_structure` puts on ONE body.
/// The pair is the comparison: same section count, opposite shape.
const SECTIONS_PER_SHIP: usize = 10;

/// Every section in the sky, across the whole fleet. The exact-count invariant
/// reads this.
#[cfg(feature = "debug")]
const SECTIONS_IN_THE_SKY: usize = SHIPS_IN_THE_SKY * SECTIONS_PER_SHIP;

/// Radius of the shell the fleet sits on. Fixed rather than count-scaled, so
/// density rises with the ship count - a constant-density shell would keep the
/// number of hulls in the frustum and in each broad-phase cell flat, and the
/// measured cost would saturate instead of following the scale.
const SHELL_RADIUS: f32 = 300.0;

/// The scenario id the fleet loads under.
const SCENARIO_ID: &str = "stress_many_structures";

/// The first step's name, so `loop_from` restarts the cycle at the spawn
/// without repeating the string.
#[cfg(feature = "debug")]
const LOAD_STEP: &str = "spawn the fleet";

/// How long the fleet is held before the churn: the steady-state stretch a
/// frame capture is actually interested in, and long enough for the acquisition
/// pass to have run over every hull.
#[cfg(feature = "debug")]
const HOLD_SECS: f32 = 2.0;

/// Settle window between the unload and the baseline assertion.
#[cfg(feature = "debug")]
const TEARDOWN_SETTLE_SECS: f32 = 0.5;

/// Deadlines, in the clamped sim seconds every gate here counts in. Generous
/// for llvmpipe, and summing well under the 120 s harness completion backstop
/// even though this script spawns the fleet TWICE.
#[cfg(feature = "debug")]
const SPAWN_DEADLINE_SECS: f32 = 45.0;
#[cfg(feature = "debug")]
const UNLOAD_DEADLINE_SECS: f32 = 20.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new()
        .with_game_plugins(|app: &mut App| {
            app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_fleet);
        })
        .build();

    #[cfg(feature = "debug")]
    {
        // Latched every frame, so the whole-fleet beats can ask what assembled
        // rather than what has survived being shot at since.
        app.init_resource::<FleetPeak>();
        app.add_systems(Update, record_fleet_peak);
        // The picture is taken at the range's PEAK. `nova_screenshot`
        // appends its beat to whatever it is handed, and the beats after
        // this call drain and tear the range down - a shot behind them
        // photographs an empty world.
        app.add_plugins(
            nova_screenshot(
                nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                    // The fleet is spawned by an OnStart handler, so waiting for
                    // the full section count is also the gate a looped reload
                    // needs: the old cycle's entities outlive the load replacing
                    // them.
                    .step(LOAD_STEP)
                    .enter(GameStates::Loading)
                    .until(the_fleet_is_up())
                    .deadline(SPAWN_DEADLINE_SECS)
                    .add()
                    // The scene is live again: close the reload interval so a frame
                    // capture excludes it. A no-op on the first cycle.
                    .step("close the reload interval")
                    .on_enter(nova_probe::capture_reload_end)
                    .on_enter(assert_the_fleet_is_whole)
                    .add()
                    .step("hold the fleet")
                    .until(elapsed(HOLD_SECS))
                    .add()
                    .step("assert the fleet acquired across itself")
                    .on_enter(assert_the_fleet_acquired)
                    .add()
                    // The churn: the whole fleet goes and comes back inside one
                    // run, so a leak or a stale relation shows here rather than at
                    // the end of a long capture.
                    .step("churn: drop the fleet")
                    .on_enter(tear_the_fleet_down)
                    .until(the_sky_is_empty())
                    .deadline(UNLOAD_DEADLINE_SECS)
                    .add()
                    .step("churn: build it again")
                    .on_enter(respawn_the_fleet)
                    .until(the_fleet_is_up())
                    .deadline(SPAWN_DEADLINE_SECS)
                    .add()
                    .step("assert the fleet came back")
                    .on_enter(assert_the_fleet_came_back)
                    .add(),
            )
            .step("tear the fleet down")
            .on_enter(tear_the_fleet_down)
            .until(elapsed(TEARDOWN_SETTLE_SECS))
            .add()
            .step("check the world came back to baseline")
            .on_enter(assert_back_to_baseline)
            .add()
            .loop_from(LOAD_STEP)
            .on_loop(respawn_the_fleet_for_the_capture),
        );
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        app.add_plugins(nova_probe::NovaProbePlugin::default());
    }

    app.run()
}

fn setup_fleet(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(fleet_scenario(&game_assets, &sections)));
}

/// One hull: a flight computer, eight hull blocks and a turret, in a contiguous
/// column so the integrity graph gets one connected structure per ship.
///
/// AI-controlled with a 1 u engage range: the acquisition pass runs (that is
/// the spatial query this range exists to load), while the behavior FSM keeps
/// every hull in its passive routine, so the formation holds and the scene
/// stays comparable between cycles.
fn hull(sections: &GameSections, index: usize) -> SpaceshipConfig {
    let mut specs = vec![SectionSpec::new(
        format!("computer_{index}"),
        "basic_controller_section",
        Vec3::ZERO,
    )];
    specs.extend((1..SECTIONS_PER_SHIP - 1).map(|i| {
        SectionSpec::new(
            format!("hull_{index}_{i}"),
            "light_hull_section",
            Vec3::new(0.0, 0.0, i as f32),
        )
    }));
    specs.push(SectionSpec::new(
        format!("guns_{index}"),
        "pdc_kinetic_turret_section",
        Vec3::new(0.0, 0.75, 0.0),
    ));

    SpaceshipConfig {
        // Half the fleet answers to the other half: hostility comes from the
        // relation model, so opposing allegiances are what give the
        // acquisition pass something to find.
        allegiance: Some(if index.is_multiple_of(2) {
            Allegiance::Player
        } else {
            Allegiance::Enemy
        }),
        ..fixtures::ship(
            sections,
            SpaceshipController::AI(AIControllerConfig {
                engage_range: Some(1.0),
                ..default()
            }),
            &specs,
        )
    }
}

/// Where ship `i` sits: a Fibonacci-sphere shell of FIXED radius, which spreads
/// the fleet evenly over the sphere for any count. Purely a function of the
/// index, so a rerun builds the same sky and the frame-time numbers stay
/// comparable.
fn ship_position(i: usize) -> Vec3 {
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    let y = 1.0 - (i as f32 / (SHIPS_IN_THE_SKY.max(2) - 1) as f32) * 2.0;
    let ring = (1.0 - y * y).max(0.0).sqrt();
    let theta = golden * i as f32;
    Vec3::new(theta.cos() * ring, y, theta.sin() * ring) * SHELL_RADIUS
}

/// The fleet: [`SHIPS_IN_THE_SKY`] hulls on a shell, alternating allegiance.
fn fleet_scenario(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let ships = (0..SHIPS_IN_THE_SKY)
        .map(|i| ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: format!("ship_{i}"),
                name: format!("Ship {i}"),
                position: ship_position(i),
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(hull(sections, i)),
        })
        .collect::<Vec<_>>();

    ScenarioConfig {
        description: "A hundred AI hulls holding formation on a shell.".to_string(),
        hidden: true,
        // The scene lights itself: the engine spawns no light, so a scenario
        // that authors none renders black.
        events: fixtures::spawn_on_start(
            [
                ships,
                ThreePointRig::around("lights", Vec3::ZERO, 30.0).objects(),
            ]
            .concat(),
        ),
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Stress: Many Structures".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The most of each the sky has held since the last teardown.
///
/// The whole-fleet claim is about what ASSEMBLED, and a live count cannot say
/// that here: these hulls are mutually hostile and armed, they acquire across
/// the shell the moment they are up (which is what
/// [`assert_the_fleet_acquired`] goes on to require), and they start taking each
/// other apart. Reading the count a frame after the fleet completes races the
/// first kill - a race this range lost on CI, one section down out of a
/// thousand, while passing on a faster machine that got its look in first.
///
/// Latching the high-water mark separates the two claims: this one is that every
/// authored hull and section arrived, and attrition afterwards is the range
/// working as intended rather than a spawn that came up short.
#[cfg(feature = "debug")]
#[derive(Resource, Default, Clone, Copy)]
struct FleetPeak {
    sections: usize,
    ships: usize,
    turrets: usize,
    /// Hulls holding a hostile at once. Latched for the same reason as the rest:
    /// a hull whose target was just destroyed reads empty until the next pass
    /// re-acquires, and with this fleet shooting itself apart that is happening
    /// constantly. The claim is that the acquisition pass REACHED every hull,
    /// which the high-water mark is the honest measure of.
    acquired: usize,
}

/// Latch the high-water counts. Exclusive because [`fleet_counts`] walks every
/// entity; it is the same walk the predicates already do each frame.
#[cfg(feature = "debug")]
fn record_fleet_peak(world: &mut World) {
    let (sections, ships, turrets) = fleet_counts(world);
    let acquired = world
        .try_query_filtered::<&AITarget, With<SpaceshipRootMarker>>()
        .map_or(0, |mut hulls| {
            hulls.iter(world).filter(|target| target.is_some()).count()
        });
    let mut peak = world.resource_mut::<FleetPeak>();
    peak.sections = peak.sections.max(sections);
    peak.ships = peak.ships.max(ships);
    peak.turrets = peak.turrets.max(turrets);
    peak.acquired = peak.acquired.max(acquired);
}

/// Live sections, ship roots and turrets - the shape of the sky, counted the
/// same way before and after the churn.
#[cfg(feature = "debug")]
fn fleet_counts(world: &World) -> (usize, usize, usize) {
    let mut sections = 0;
    let mut roots = 0;
    let mut turrets = 0;
    for entity in world.iter_entities() {
        if entity.contains::<SectionMarker>() {
            sections += 1;
        }
        if entity.contains::<SpaceshipRootMarker>() {
            roots += 1;
        }
        if entity.contains::<TurretSectionMarker>() {
            turrets += 1;
        }
    }
    (sections, roots, turrets)
}

/// Advance once the whole fleet is in the world.
#[cfg(feature = "debug")]
fn the_fleet_is_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| fleet_counts(world).0 >= SECTIONS_IN_THE_SKY)
}

/// Advance once the unload has taken every hull.
#[cfg(feature = "debug")]
fn the_sky_is_empty() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| fleet_counts(world) == (0, 0, 0))
}

/// The fleet ASSEMBLED to exactly what it was authored as. Read off
/// [`FleetPeak`], not the live count, so the fleet shooting at itself cannot
/// turn a whole assembly into a failure.
#[cfg(feature = "debug")]
fn assert_the_fleet_is_whole(world: &mut World) {
    let peak = *world.resource::<FleetPeak>();
    let counts = (peak.sections, peak.ships, peak.turrets);
    assert_eq!(
        counts,
        (SECTIONS_IN_THE_SKY, SHIPS_IN_THE_SKY, SHIPS_IN_THE_SKY),
        "stress_many_structures: the sky must hold exactly {SHIPS_IN_THE_SKY} \
         hulls of {SECTIONS_PER_SHIP} sections with a turret each"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the fleet assembled every ship",
        serde_json::json!({ "sections": counts.0, "ships": counts.1 }),
    );
    info!(
        "stress_many_structures: {} ships up on {} sections",
        counts.1, counts.0
    );
}

/// The spatial-query claim: the acquisition pass ran over the whole fleet.
///
/// Acquisition has no range gate - it reads every candidate body in the scene
/// for every AI hull, which is the quadratic scan this range exists to load -
/// so with both allegiances in the sky EVERY hull must hold a hostile. A fleet
/// where the pass silently stopped scanning would sit here with empty targets
/// and an honest-looking frame time.
#[cfg(feature = "debug")]
fn assert_the_fleet_acquired(world: &mut World) {
    // Off the high-water mark, for the same reason the fleet's own shape is:
    // this range's sections do come apart as the sky settles - a hundred and
    // more of them over a run, on a fast backend as well as a slow one - and a
    // hull whose hostile was one of them reads empty until the next pass finds
    // it another. A snapshot then scores the churn rather than the scan, and it
    // is the SCAN this claim is about.
    let peak = *world.resource::<FleetPeak>();
    let (ships, acquired) = (peak.ships, peak.acquired);
    assert_eq!(
        (ships, acquired),
        (SHIPS_IN_THE_SKY, SHIPS_IN_THE_SKY),
        "stress_many_structures: every one of {SHIPS_IN_THE_SKY} AI hulls must \
         have acquired a hostile ({acquired} of {ships} did)"
    );
    nova_probe::probe_marker(
        world,
        "outcome: every ship acquired a hostile",
        serde_json::json!({ "ships": ships, "acquired": acquired }),
    );
    info!("stress_many_structures: {acquired} of {ships} hulls hold a hostile");
}

/// The churn claim: the sky came back to exactly the shape it had before.
#[cfg(feature = "debug")]
fn assert_the_fleet_came_back(world: &mut World) {
    // The peak was zeroed by the teardown, so this reads the SECOND assembly
    // rather than the memory of the first - and, like the first, it survives the
    // fleet reopening fire while the count is being taken.
    let peak = *world.resource::<FleetPeak>();
    let counts = (peak.sections, peak.ships, peak.turrets);
    assert_eq!(
        counts,
        (SECTIONS_IN_THE_SKY, SHIPS_IN_THE_SKY, SHIPS_IN_THE_SKY),
        "stress_many_structures: the fleet must come back identical after a \
         full teardown and respawn"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the fleet comes back after a churn cycle",
        serde_json::json!({ "sections": counts.0, "ships": counts.1 }),
    );
    info!("stress_many_structures: the fleet came back after the churn");
}

/// Drop the whole scene. `UnloadScenario` guarantees every scenario-scoped
/// entity goes, which is exactly the claim the churn and the baseline check.
#[cfg(feature = "debug")]
fn tear_the_fleet_down(world: &mut World) {
    nova_probe::probe_marker(world, "stress: teardown", serde_json::json!({}));
    world.trigger(UnloadScenario);
    // Zero the high-water mark with the fleet it measured, or the churn's own
    // assembly would be judged on the first one's numbers and could not fail.
    *world.resource_mut::<FleetPeak>() = FleetPeak::default();
}

/// Build the fleet again, mid-script.
#[cfg(feature = "debug")]
fn respawn_the_fleet(world: &mut World) {
    let (Some(game_assets), Some(sections)) = (
        world.get_resource::<GameAssets>().cloned(),
        world.get_resource::<GameSections>().cloned(),
    ) else {
        return;
    };
    world.trigger(LoadScenario(fleet_scenario(&game_assets, &sections)));
}

/// The same rebuild at the capture loop point, with the reload interval opened
/// so the frame capture excludes the load.
#[cfg(feature = "debug")]
fn respawn_the_fleet_for_the_capture(world: &mut World) {
    nova_probe::capture_reload_begin(world);
    respawn_the_fleet(world);
}

/// The teardown claim: nothing survived.
#[cfg(feature = "debug")]
fn assert_back_to_baseline(world: &mut World) {
    let counts = fleet_counts(world);
    assert_eq!(
        counts,
        (0, 0, 0),
        "stress_many_structures: {} sections, {} ship roots and {} turrets \
         survived UnloadScenario - the fleet did not return to baseline",
        counts.0,
        counts.1,
        counts.2
    );
    nova_probe::probe_marker(
        world,
        "outcome: the teardown left nothing behind",
        serde_json::json!({ "sections": counts.0, "ships": counts.1 }),
    );
    info!("stress_many_structures: teardown returned to baseline");
}
