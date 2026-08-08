//! many_sections: ONE ship built out of N sections, spawned, held, torn down,
//! and spawned again.
//!
//! The scale sweep for the SECTION count. `many_bodies` scales the number of
//! independent physics bodies; this one scales the size of a single body's
//! structure, which is a different cost curve: one rigid body whose collider,
//! mass and center-of-mass are aggregated from N children, plus an integrity
//! graph over N nodes that every damage event walks.
//!
//! It makes a correctness claim alongside the number. Two, in fact:
//! - the aggregation SURVIVES the scale - the assembled root carries a finite,
//!   positive [`ComputedMass`] and a finite [`ComputedCenterOfMass`], so an
//!   aggregation that silently produced a NaN at N sections fails here rather
//!   than turning up later as a ship that will not fly;
//! - the structure comes BACK to baseline - every cycle unloads the scenario and
//!   asserts that not one [`SectionMarker`] and not one [`SpaceshipRootMarker`]
//!   survived (task 20260804-094006, DECISION.md D3 - an in-example assertion,
//!   not a probe check, because the subject is ECS entity counts).
//!
//! The script is enrolled in capture looping, so `--fps` measures spawn -> hold
//! -> teardown ACTIVITY rather than an idle tail.
//!
//! Count knob: `NOVA_STRESS_COUNT` overrides [`DEFAULT_COUNT`].
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example many_sections --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `many_sections: N sections up`,
//! #           `many_sections: teardown returned to baseline`,
//! #           `autopilot: cycle complete, no panic`
//!
//! # the frame-time sweep:
//! cargo run -p nova_probe_cli -- run many_sections --fps --release
//! ```

#[cfg(feature = "debug")]
use avian3d::prelude::{ComputedCenterOfMass, ComputedMass};
use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "many_sections")]
#[command(version = "1.0.0")]
#[command(about = "Scale sweep: one ship assembled out of N sections", long_about = None)]
struct Cli;

/// How many sections the ship carries when `NOVA_STRESS_COUNT` is unset.
///
/// Measured under the CI-shaped raster (lavapipe/llvmpipe, 1280x720, dev
/// profile, 20 warm-up + 120 captured frames):
///
/// | count | mean ms | mean fps |
/// | ----- | ------- | -------- |
/// | 100   | 76.3    | 13.1     |
/// | 250   | 130.7   | 7.7      |
/// | 400   | 176.0   | 5.7      |
///
/// 250 is the pick: the full 180 + 900 frame capture window fills in ~140 s,
/// comfortably inside probe's 585 s window-sized completion deadline, with the
/// structure clearly dominating the frame. It sits below `many_bodies`' default
/// on purpose - a section is a meshed, collider-bearing child of an aggregating
/// rigid body, so it costs several times what a lone asteroid does, which these
/// numbers bear out.
const DEFAULT_COUNT: usize = 250;

/// Env var that overrides [`DEFAULT_COUNT`]. Shared with the other `stress/`
/// sweeps so `probe run stress` scales all of them together.
const COUNT_ENV: &str = "NOVA_STRESS_COUNT";

/// The content id every filler section is built from. Hull rather than a
/// turret or thruster: the subject is structural aggregation at scale, and a
/// hull adds mass and collider without also adding a per-frame aiming or
/// thrust system that would put a different system's cost in the number.
const FILLER_SECTION: &str = "light_hull_section";

/// The section at the origin. A ship with no controller is not a ship the rest
/// of the stack recognizes, so slot 0 is always this one.
const ROOT_SECTION: &str = "basic_controller_section";

/// Grid pitch between adjacent section slots, in world units. Matches the
/// hand-authored ships (`torpedo_section.rs` mounts at `z = -1, 0, 1`), so the
/// lattice below is flush rather than gapped.
const SLOT_PITCH: f32 = 1.0;

/// The scenario id the ship loads under.
const SCENARIO_ID: &str = "stress_many_sections";

/// The first step's name, so `loop_from` restarts the cycle at the spawn
/// without repeating the string.
#[cfg(feature = "debug")]
const LOAD_STEP: &str = "assemble the ship";

/// How long the ship is held up before teardown. The steady-state stretch the
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
    let count = section_count();
    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            app.insert_resource(StructureSize(count));
            app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_structure);
        })
        .build();

    // Headless smoke-test harness: inert in a normal run. Under NOVA_AUTOPILOT
    // it assembles the ship, holds it, tears it down and asserts the world came
    // back to baseline - the correctness half of a run whose other half is a
    // frame-time number.
    #[cfg(feature = "debug")]
    {
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                // The ship is spawned by an OnStart handler, so waiting for the
                // full section count is also the gate a looped reload needs: the
                // old cycle's entities outlive the load replacing them.
                .step(LOAD_STEP)
                .enter(GameStates::Loading)
                .until(structure_is_up(count))
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
                    report_structure_up(world);
                })
                .add()
                .step("hold the ship")
                .until(elapsed(HOLD_SECS))
                .add()
                .step("tear the ship down")
                .on_enter(tear_the_structure_down)
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
                .on_loop(reassemble_the_ship),
        );
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        app.add_plugins(nova_probe::nova_timeline());
        app.add_plugins(nova_probe::nova_invariants());
        app.add_plugins(nova_probe::nova_frametime());
        app.add_plugins(nova_screenshot());
    }

    app.run()
}

/// How many sections this run assembles, so the script's gates and the scenario
/// builder agree without re-reading the environment.
#[derive(Resource, Clone, Copy)]
struct StructureSize(usize);

/// The section count for this run: `NOVA_STRESS_COUNT` if it parses, else
/// [`DEFAULT_COUNT`]. Read once, in `main`.
fn section_count() -> usize {
    std::env::var(COUNT_ENV)
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(DEFAULT_COUNT)
        // A ship is its controller at minimum: slot 0 is never optional.
        .max(1)
}

fn setup_structure(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    size: Res<StructureSize>,
) {
    commands.trigger(LoadScenario(structure_scenario(
        &game_assets,
        &sections,
        size.0,
    )));
}

/// The mount points for `count` sections: the `count` integer-lattice cells
/// closest to the origin, in Manhattan order.
///
/// Manhattan order rather than Euclidean is the load-bearing choice. Every cell
/// at L1 distance `d` has a face neighbour at `d - 1`, and all lower-`d` cells
/// are already placed, so the assembled hull is CONTIGUOUS at every count -
/// the integrity graph gets one connected structure to walk instead of a
/// cloud of drifting islands, which is what makes the number comparable
/// between counts. Purely a function of `(i, count)`, so a rerun at the same
/// knob builds the same ship.
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

/// The ship: a controller at the origin and `count - 1` hull sections filling
/// the lattice around it.
fn structure_scenario(
    game_assets: &GameAssets,
    sections: &GameSections,
    count: usize,
) -> ScenarioConfig {
    let specs: Vec<SectionSpec> = slot_positions(count)
        .into_iter()
        .enumerate()
        .map(|(i, position)| {
            let asset = if i == 0 { ROOT_SECTION } else { FILLER_SECTION };
            SectionSpec::new(format!("slot_{i}"), asset, position)
        })
        .collect();

    let ship = fixtures::ship(sections, SpaceshipController::None, &specs);

    ScenarioConfig {
        description: "One ship assembled out of a lattice of sections.".to_string(),
        // The scene lights itself: the engine spawns no light, so a scenario
        // that authors none renders black.
        events: fixtures::spawn_on_start(
            [
                vec![ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "structure".to_string(),
                        name: "Structure".to_string(),
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Spaceship(ship),
                }],
                ThreePointRig::around("lattice", Vec3::ZERO, 3.0).objects(),
            ]
            .concat(),
        ),
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Many Sections".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// How many section entities are alive right now. Scans entities rather than
/// caching a `QueryState`: the predicates and the `&mut World` step hooks both
/// need it, and it runs a handful of times per cycle, not per frame.
#[cfg(feature = "debug")]
fn live_sections(world: &World) -> usize {
    world
        .iter_entities()
        .filter(|entity| entity.contains::<SectionMarker>())
        .count()
}

/// How many ship roots are alive right now. Counted alongside the sections so
/// the baseline claim covers the aggregating body too, not only its children.
#[cfg(feature = "debug")]
fn live_roots(world: &World) -> usize {
    world
        .iter_entities()
        .filter(|entity| entity.contains::<SpaceshipRootMarker>())
        .count()
}

/// Advance once the whole structure is in the world.
#[cfg(feature = "debug")]
fn structure_is_up(count: usize) -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(move |world: &World| live_sections(world) >= count)
}

/// The aggregation claim, and the timeline marker.
///
/// A root that assembled N sections has to carry a finite, positive mass and a
/// finite center of mass. Panics rather than warning: a NaN here is the failure
/// mode that a frame-time graph cannot show at all, and it would otherwise only
/// surface much later as a ship that refuses to fly.
#[cfg(feature = "debug")]
fn report_structure_up(world: &mut World) {
    let live = live_sections(world);
    let mut roots =
        world.query_filtered::<(&ComputedMass, &ComputedCenterOfMass), With<SpaceshipRootMarker>>();
    let aggregate = roots
        .iter(world)
        .map(|(mass, com)| (mass.value(), com.0))
        .next()
        .expect("many_sections: the assembled ship has no aggregating root");
    let (mass, com) = aggregate;

    assert!(
        mass.is_finite() && mass > 0.0,
        "many_sections: {live} sections aggregated to a mass of {mass} - \
         aggregation did not survive the scale"
    );
    assert!(
        com.is_finite(),
        "many_sections: {live} sections aggregated to a center of mass of \
         {com} - aggregation did not survive the scale"
    );

    nova_probe::probe_marker(
        world,
        "stress: structure up",
        serde_json::json!({ "live": live, "mass": mass }),
    );
    info!("many_sections: {live} sections up, aggregate mass {mass}, com {com}");
}

/// Drop the whole scene. `UnloadScenario` guarantees every scenario-scoped
/// entity goes, which is exactly the claim the next step checks.
#[cfg(feature = "debug")]
fn tear_the_structure_down(world: &mut World) {
    nova_probe::probe_marker(world, "stress: teardown", serde_json::json!({}));
    world.trigger(UnloadScenario);
}

/// The correctness claim: the teardown left NOTHING behind, neither a section
/// nor the root that aggregated them. Panics rather than stalling a predicate,
/// so a leak fails the smoke test with the count that survived instead of
/// timing out anonymously.
#[cfg(feature = "debug")]
fn assert_back_to_baseline(world: &mut World) {
    let sections = live_sections(world);
    let roots = live_roots(world);
    nova_probe::probe_marker(
        world,
        "stress: baseline",
        serde_json::json!({ "sections": sections, "roots": roots }),
    );
    assert_eq!(
        (sections, roots),
        (0, 0),
        "many_sections: {sections} sections and {roots} ship roots survived \
         UnloadScenario - the structure did not return to baseline"
    );
    info!("many_sections: teardown returned to baseline");
}

/// Rebuild the ship for the next capture cycle, the way the loop point does.
#[cfg(feature = "debug")]
fn reassemble_the_ship(world: &mut World) {
    let (Some(game_assets), Some(sections), Some(size)) = (
        world.get_resource::<GameAssets>().cloned(),
        world.get_resource::<GameSections>().cloned(),
        world.get_resource::<StructureSize>().copied(),
    ) else {
        return;
    };
    nova_probe::capture_reload_begin(world);
    world.trigger(LoadScenario(structure_scenario(
        &game_assets,
        &sections,
        size.0,
    )));
}
