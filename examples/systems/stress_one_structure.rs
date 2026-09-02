//! stress_one_structure: one ship built out of a thousand sections.
//!
//! The STRUCTURE at a scale no ship will ever reach: a single rigid body whose
//! collider, mass and center of mass are aggregated from
//! [`SECTIONS_ON_THE_HULL`] children, with a health node on every one of them
//! and a derived skin clad over the whole surface. `stress_many_structures`
//! scales the number of bodies; this one scales the size of ONE.
//!
//! What it claims, beside the frame time:
//!
//! - the hull is EXACTLY the size it was authored - a thousand sections, no
//!   more, no fewer, so a spawn that silently dropped a third of them cannot
//!   report an easy number;
//! - it is ONE BODY - a single aggregating root, carrying a finite, positive
//!   mass and a finite center of mass, so an aggregation that produced a NaN at
//!   this scale fails here rather than turning up later as a ship that will not
//!   fly;
//! - the HEALTH GRAPH covers it - every section carries the health node the
//!   damage pipeline walks;
//! - the SKIN clads it - the derivation reads a thousand cells and lays plates
//!   over them;
//! - the teardown leaves NOTHING - not a section, not a plate, not a root.
//!
//! The script is enrolled in capture looping, so a frame capture measures
//! spawn -> hold -> teardown ACTIVITY rather than an idle tail.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example stress_one_structure --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `stress_one_structure: 1000 sections up`,
//! #           `stress_one_structure: teardown returned to baseline`,
//! #           `autopilot: cycle complete, no panic`
//!
//! # with the frame-time capture armed:
//! cargo run --features debug probe run stress_one_structure
//! ```

#[cfg(feature = "debug")]
use avian3d::prelude::{ComputedCenterOfMass, ComputedMass};
use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "stress_one_structure")]
#[command(version = "1.0.0")]
#[command(about = "Stress range: one ship assembled out of a thousand sections. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// ABSURD SCALE, and deliberately so: this number must never reflect real
/// content. A corvette is a dozen sections and a capital a few dozen; a
/// thousand on one hull is a load the aggregation, the integrity graph and the
/// skin derivation have to survive, not a ship anyone will fly. Read it as a
/// ceiling the structure is proven under, never as a design figure.
const SECTIONS_ON_THE_HULL: usize = 1000;

/// The content id every filler section is built from. Hull rather than a turret
/// or a drive: the subject is structural aggregation at scale, and a hull adds
/// mass, collider and a health node without also adding a per-frame aiming or
/// thrust system that would put another system's cost in the number.
const FILLER_SECTION: &str = "light_hull_section";

/// The section at the origin. A ship with no flight computer is not a ship the
/// rest of the stack recognizes, so slot 0 is always this one.
const ROOT_SECTION: &str = "basic_controller_section";

/// Grid pitch between adjacent section slots, in engine world units (build-grid
/// cells, not a distance). Matches the
/// hand-authored ships, so the lattice below is flush rather than gapped.
const SLOT_PITCH: f32 = 1.0;

/// The scenario id the structure loads under.
const SCENARIO_ID: &str = "stress_one_structure";

/// The first step's name, so `loop_from` restarts the cycle at the spawn
/// without repeating the string.
#[cfg(feature = "debug")]
const LOAD_STEP: &str = "assemble the hull";

/// How long the hull is held up before teardown: the steady-state stretch a
/// frame capture is actually interested in.
#[cfg(feature = "debug")]
const HOLD_SECS: f32 = 2.0;

/// Settle window between the unload and the baseline assertion.
#[cfg(feature = "debug")]
const TEARDOWN_SETTLE_SECS: f32 = 0.5;

/// Deadlines, in the clamped sim seconds every gate here counts in. The spawn
/// one is the generous one on purpose: a thousand meshed, collider-bearing
/// children plus a skin derivation over all of them is the slowest single
/// moment in this catalog under llvmpipe.
#[cfg(feature = "debug")]
const SPAWN_DEADLINE_SECS: f32 = 60.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new()
        .with_game_plugins(|app: &mut App| {
            app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_structure);
        })
        .build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(
            // The picture is taken while the hull STANDS. `nova_screenshot`
            // appends its beat to whatever it is handed, and the beats after
            // this call tear the range down - a shot behind them photographs
            // an empty world.
            nova_screenshot(
                nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                    // The hull is spawned by an OnStart handler, so waiting for
                    // the full section count is also the gate a looped reload
                    // needs: the old cycle's entities outlive the load
                    // replacing them.
                    .step(LOAD_STEP)
                    .enter(GameStates::Loading)
                    .until(the_hull_is_up())
                    .deadline(SPAWN_DEADLINE_SECS)
                    .add()
                    // The scene is live again: close the reload interval so a
                    // frame capture excludes it. A no-op on the first cycle.
                    .step("close the reload interval")
                    .on_enter(nova_probe::capture_reload_end)
                    .on_enter(assert_the_hull_stood_up)
                    .add()
                    .step("hold the hull")
                    .until(elapsed(HOLD_SECS))
                    .add(),
            )
            .step("tear the hull down")
            .on_enter(tear_the_hull_down)
            .until(elapsed(TEARDOWN_SETTLE_SECS))
            .add()
            .step("check the world came back to baseline")
            .on_enter(assert_back_to_baseline)
            .add()
            .loop_from(LOAD_STEP)
            .on_loop(reassemble_the_hull),
        );
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        app.add_plugins(nova_probe::NovaProbePlugin::default());
    }

    app.run()
}

fn setup_structure(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
) {
    commands.trigger(LoadScenario(structure_scenario(&game_assets, &sections)));
}

/// The mount points for the hull: the `count` integer-lattice cells closest to
/// the origin, in Manhattan order.
///
/// Manhattan order rather than Euclidean is the load-bearing choice. Every cell
/// at L1 distance `d` has a face neighbour at `d - 1`, and all lower-`d` cells
/// are already placed, so the assembled hull is CONTIGUOUS at every count - the
/// integrity graph gets one connected structure to walk instead of a cloud of
/// drifting islands. Purely a function of the count, so a rerun builds the same
/// ship.
fn slot_positions(count: usize) -> Vec<Vec3> {
    // The half-width that certainly contains `count` cells: the L1 ball of
    // radius r holds more than r^3 cells, so a cube of this side is ample.
    let half = (count as f32).cbrt().ceil() as i32 + 1;
    let mut cells: Vec<IVec3> = (-half..=half)
        .flat_map(|x| {
            (-half..=half).flat_map(move |y| (-half..=half).map(move |z| IVec3::new(x, y, z)))
        })
        .collect();
    // Ties broken on the coordinates themselves, so the order is total and the
    // same ship comes back on every run.
    cells.sort_by_key(|c| (c.x.abs() + c.y.abs() + c.z.abs(), c.x, c.y, c.z));
    cells
        .into_iter()
        .take(count)
        .map(|c| c.as_vec3() * SLOT_PITCH)
        .collect()
}

/// The hull: a flight computer at the origin and hull sections filling the
/// lattice around it, clad in a derived skin.
fn structure_scenario(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let specs: Vec<SectionSpec> = slot_positions(SECTIONS_ON_THE_HULL)
        .into_iter()
        .enumerate()
        .map(|(i, position)| {
            let asset = if i == 0 { ROOT_SECTION } else { FILLER_SECTION };
            SectionSpec::new(format!("slot_{i}"), asset, position)
        })
        .collect();

    let mut ship = fixtures::ship(sections, SpaceshipController::None, &specs);
    // The skin is part of the load: the derivation reads the whole structure as
    // unit cells and lays a plate over every exposed face, which is the cost
    // this range exists to hold at a thousand cells.
    if let ShipSource::Inline(hull) = &mut ship.hull {
        hull.skin = true;
    }

    ScenarioConfig {
        description: "One ship assembled out of a thousand sections.".to_string(),
        hidden: true,
        // The scene lights itself: the engine spawns no light, so a scenario
        // that authors none renders black.
        events: fixtures::spawn_on_start(
            [
                vec![ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "structure".to_string(),
                        name: "Structure".to_string(),
                        position: Meters3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Spaceship(ship),
                }],
                ThreePointRig::around("lights", Meters3::ZERO, 15.0).objects(),
            ]
            .concat(),
        ),
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Stress: One Structure".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// How many section entities are alive right now.
#[cfg(feature = "debug")]
fn live_sections(world: &World) -> usize {
    world
        .iter_entities()
        .filter(|entity| entity.contains::<SectionMarker>())
        .count()
}

/// How many of those sections carry the health node the damage pipeline walks.
#[cfg(feature = "debug")]
fn live_health_nodes(world: &World) -> usize {
    world
        .iter_entities()
        .filter(|entity| entity.contains::<SectionMarker>() && entity.contains::<Health>())
        .count()
}

/// How many ship roots are alive right now.
#[cfg(feature = "debug")]
fn live_roots(world: &World) -> usize {
    world
        .iter_entities()
        .filter(|entity| entity.contains::<SpaceshipRootMarker>())
        .count()
}

/// How many derived skin plates are on the stage.
#[cfg(feature = "debug")]
fn live_plates(world: &World) -> usize {
    world
        .iter_entities()
        .filter(|entity| entity.contains::<ShipSkinMarker>())
        .count()
}

/// Advance once the whole structure is in the world.
#[cfg(feature = "debug")]
fn the_hull_is_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| live_sections(world) >= SECTIONS_ON_THE_HULL)
}

/// The whole claim, made where the spawn has just completed: exact size, one
/// aggregating body with sane mass properties, a health node per section, and a
/// skin over the lot.
#[cfg(feature = "debug")]
fn assert_the_hull_stood_up(world: &mut World) {
    let sections = live_sections(world);
    let roots = live_roots(world);
    assert_eq!(
        (sections, roots),
        (SECTIONS_ON_THE_HULL, 1),
        "stress_one_structure: the range must stand up as exactly one root \
         carrying {SECTIONS_ON_THE_HULL} sections"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the hull assembled every section",
        serde_json::json!({ "sections": sections, "roots": roots }),
    );

    let mut bodies =
        world.query_filtered::<(&ComputedMass, &ComputedCenterOfMass), With<SpaceshipRootMarker>>();
    let aggregate: Vec<(f32, Vec3)> = bodies
        .iter(world)
        .map(|(mass, com)| (mass.value(), com.0))
        .collect();
    assert_eq!(
        aggregate.len(),
        1,
        "stress_one_structure: a thousand sections must aggregate into ONE \
         body, not {}",
        aggregate.len()
    );
    let (mass, com) = aggregate[0];
    assert!(
        mass.is_finite() && mass > 0.0,
        "stress_one_structure: {sections} sections aggregated to a mass of \
         {mass} - aggregation did not survive the scale"
    );
    assert!(
        com.is_finite(),
        "stress_one_structure: {sections} sections aggregated to a center of \
         mass of {com} - aggregation did not survive the scale"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the structure aggregates to one body",
        serde_json::json!({ "mass": mass }),
    );

    let health_nodes = live_health_nodes(world);
    assert_eq!(
        health_nodes, SECTIONS_ON_THE_HULL,
        "stress_one_structure: the damage pipeline walks a health node per \
         section, and {health_nodes} of {SECTIONS_ON_THE_HULL} carry one"
    );
    nova_probe::probe_marker(
        world,
        "outcome: every section carries a health node",
        serde_json::json!({ "health_nodes": health_nodes }),
    );

    let plates = live_plates(world);
    assert!(
        plates > 0,
        "stress_one_structure: the hull is authored clad, so the derivation \
         must lay plates over it"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the skin clad the whole hull",
        serde_json::json!({ "plates": plates }),
    );
    info!(
        "stress_one_structure: {sections} sections up, mass {mass}, com {com}, \
         {plates} plates"
    );
}

/// Drop the whole scene. `UnloadScenario` guarantees every scenario-scoped
/// entity goes, which is exactly the claim the next step checks.
#[cfg(feature = "debug")]
fn tear_the_hull_down(world: &mut World) {
    nova_probe::probe_marker(world, "stress: teardown", serde_json::json!({}));
    world.trigger(UnloadScenario);
}

/// The teardown claim: the plates went with the structure that derived them.
#[cfg(feature = "debug")]
fn assert_back_to_baseline(world: &mut World) {
    let sections = live_sections(world);
    let roots = live_roots(world);
    let plates = live_plates(world);
    assert_eq!(
        (sections, roots, plates),
        (0, 0, 0),
        "stress_one_structure: {sections} sections, {roots} ship roots and \
         {plates} plates survived UnloadScenario - the hull did not return to \
         baseline"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the teardown left nothing behind",
        serde_json::json!({ "sections": sections, "roots": roots, "plates": plates }),
    );
    info!("stress_one_structure: teardown returned to baseline");
}

/// Rebuild the hull for the next capture cycle, the way the loop point does.
#[cfg(feature = "debug")]
fn reassemble_the_hull(world: &mut World) {
    let (Some(game_assets), Some(sections)) = (
        world.get_resource::<GameAssets>().cloned(),
        world.get_resource::<GameSections>().cloned(),
    ) else {
        return;
    };
    nova_probe::capture_reload_begin(world);
    world.trigger(LoadScenario(structure_scenario(&game_assets, &sections)));
}
