//! system_world_sectors: a noise-gated world streams in and out of ONE live
//! scenario.
//!
//! The streamed-world direction rests on a lifetime that does not exist in
//! the game yet: a sector that is prepared off the frame, comes up, and goes
//! away again while the scenario it lives in never reloads. This range walks
//! that lifetime end to end on `nova_world` and the fixture in
//! `examples/shared/world_fixture/mod.rs` - bootstrap, arm, cross a boundary, come
//! back, abandon work, and unload - and asserts what survives each step.
//! Nothing here routes through `LoadScenario` except the one empty bootstrap,
//! and nothing routes through a scenario event action at all.
//!
//! It runs the FEATURED generator, not the uniform one: a cell filled from
//! three independent global noise fields is the harder claim, because two
//! neighbouring cells now have to agree about a sphere neither of them owns,
//! and a cell has to place a planetoid and a derelict ship beside its rocks
//! without any of them intersecting. The uniform generator is still asserted
//! where it is the sharper instrument - the visit-order claim compares both.
//! Each is its own `WorldConfig<G>` type, and the app installs only the
//! featured one: the uniform world is described here, never streamed.
//!
//! The observer is the scenario camera, which is also what a hand-run flies:
//! the range poses it, the playable `world_features` example lets a human fly
//! it, and both drive the same streaming systems.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the free-play bootstrap authors no objects` | the loaded scenario declares zero objects and zero handlers, and no scenario object entity, job or prepared result exists before streaming is armed |
//! | 2 | `outcome: a sector is the same sector in any visit order` | the 125 cells generated forward, backward and by stride give three identical canonical manifests, under both generators |
//! | 3 | `outcome: invalid sector geometry refuses before materialization` | a finite config whose coordinate conversion overflows returns an error rather than a partial description |
//! | 4 | `outcome: one feature sphere is one sphere from every cell that sees it` | every sphere two or more cells can see carries the same id, owner, centre, radius and strength in each of them, and exactly one cell owns it |
//! | 5 | `outcome: same-layer feature spheres never overlap` | no two accepted spheres of one layer claim the same ground anywhere in the window |
//! | 6 | `outcome: independent feature layers may overlap` | spheres of different layers do overlap, so blended places exist rather than a mosaic of single-purpose tiles |
//! | 7 | `outcome: every physical object stands clear inside its own sector` | every rock, planetoid and ship is inside its owning cell's inset and clears every other object in that cell by the placement margin |
//! | 8 | `outcome: arming the stream materializes the whole desired set` | exactly the desired 5x5x5 set is live, one root each, every root scenario-scoped and owning exactly the objects its manifest names |
//! | 9 | `outcome: every sector is requested and prepared before it is materialized` | arming started 125 jobs, never more at once than the task pool has threads, preparation overlapped where the pool has more than one, all 125 came back, all 125 were spawned from a prepared result, and none was discarded |
//! | 10 | `outcome: a featured sector owns real planetoids and inert derelict ships` | every planetoid a manifest names is a real `PlanetMarker` body and every derelict a `SpaceshipRootMarker` with no driver and neutral allegiance, each a child of the sector root whose manifest names it |
//! | 11 | `outcome: crossing one boundary retains the shared slab and swaps a face` | after a +X crossing 100 roots are the SAME entities, 25 are gone and 25 are new |
//! | 12 | `outcome: the return trip leaves no duplicate root` | coming back gives the original 125 cells, one root each, and the returned sectors hold the objects their manifests name |
//! | 13 | `outcome: work for an undesired sector never materializes` | a job and a prepared result for cells outside the desired set are both discarded, nothing is spawned from them, and the live set does not move |
//! | 14 | `outcome: replacing the world config retires the world it built` | swapping `WorldConfig` under a live session leaves no baseline root alive, discards the in-window job uncompleted and the in-window prepared result, and rebuilds the window once so both of those cells hold the NEW world's objects |
//! | 15 | `outcome: unloading the session removes every sector root` | `UnloadScenario` leaves zero sector roots, zero scenario object entities, zero pending jobs and zero prepared results |
//!
//! What this range does NOT claim: anything about a floating origin,
//! persistence, a measured frame budget, wall-clock preparation cost,
//! production density, or how a feature field should be tuned. The 32 km edge
//! and the 125-cell active window are the selected baseline; this range does
//! not prove either under production load.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_world_sectors --features debug
//! # look for: `world sectors: the return set is the original 125`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[path = "../shared/world_fixture/mod.rs"]
pub mod world_fixture;

#[cfg(feature = "debug")]
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;
use nova_world::prelude::*;
#[cfg(feature = "debug")]
use world_fixture::{
    featured_world_config, uniform_world_config, EXAMPLE_ACTIVE_RADIUS, FEATURE_HOME,
};
use world_fixture::{free_play_scenario, world_observer_plugin};

#[derive(Parser)]
#[command(name = "system_world_sectors")]
#[command(version = "1.0.0")]
#[command(
    about = "A noise-gated world streams in and out of one live scenario. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

/// The empty bootstrap this session runs inside.
const SCENARIO_ID: &str = "world_sectors_free_play";

/// How many roots the desired 5x5x5 set holds.
#[cfg(feature = "debug")]
const DESIRED_ROOTS: usize = 125;

/// How many roots a one-cell crossing keeps: the 5x5x4 slab both sets share.
#[cfg(feature = "debug")]
const RETAINED_ROOTS: usize = 100;

/// How many roots a one-cell crossing retires, and how many it adds: the 5x5
/// face left behind and the face entered.
#[cfg(feature = "debug")]
const SWAPPED_ROOTS: usize = 25;

/// How close the observer has to be to its commanded pose, in meters, before a
/// beat calls it parked.
///
/// A tolerance rather than an equality because the scripted pose is written
/// through a `Transform` in engine units and read back through a propagated
/// `GlobalTransform`. Far under the 32 km cell it decides, so it can never
/// move the answer.
#[cfg(feature = "debug")]
const PARKED_TOLERANCE: Meters = Meters(1.0);

/// How much slack the clearance claim allows against the generator's own
/// margin, in meters.
///
/// The range recomputes every clearance radius from the published constants
/// rather than asking the generator what it used, so the two arithmetics can
/// disagree in the last bits of an f32. One meter against a 500 m margin
/// cannot move the answer.
#[cfg(feature = "debug")]
const CLEARANCE_SLACK: Meters = Meters(1.0);

/// In-step seconds a beat gets to reach its world condition.
///
/// A backstop that names a hung beat, NOT a budget and not an assertion about
/// how long anything takes: a 125-cell window is 125 preparations, taken a
/// poolful at a time, and then 125 separate materialization frames, and a
/// featured cell's preparation also meshes any planetoid it owns.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 300.0;

/// The cell whose job is started and then left outside the desired set.
#[cfg(feature = "debug")]
const ABANDONED_JOB_CELL: SectorCoord = SectorCoord::new(9, 0, 0);

/// The cell whose prepared result is handed in and then left outside the
/// desired set.
#[cfg(feature = "debug")]
const ABANDONED_READY_CELL: SectorCoord = SectorCoord::new(0, 9, 0);

/// The cell whose job is still running when the session is unloaded.
#[cfg(feature = "debug")]
const SWEPT_JOB_CELL: SectorCoord = SectorCoord::new(0, 0, 9);

/// The cell whose prepared result is still waiting when the session is
/// unloaded.
#[cfg(feature = "debug")]
const SWEPT_READY_CELL: SectorCoord = SectorCoord::new(9, 9, 0);

/// How many pieces of work the abandonment beat hands in: one running job and
/// one prepared sector.
#[cfg(feature = "debug")]
const ABANDONED_WORK: usize = 2;

/// The cell whose job is running when the `WorldConfig` is replaced.
///
/// INSIDE the desired window around [`FEATURE_HOME`], unlike the abandoned
/// cells: a completion the window still wants is the one a coordinate-keyed
/// loop would accept, so this is the piece of work that would carry the old
/// seed into the new world. The old seed puts a planetoid here and
/// [`REPLACEMENT_SEED`] four rocks, so the two worlds name different objects.
#[cfg(feature = "debug")]
const REPLACED_JOB_CELL: SectorCoord = SectorCoord::new(0, 0, 0);

/// The cell whose prepared result is waiting when the `WorldConfig` is
/// replaced. Inside the desired window for the same reason. The old seed puts
/// one rock here and [`REPLACEMENT_SEED`] two.
#[cfg(feature = "debug")]
const REPLACED_READY_CELL: SectorCoord = SectorCoord::new(0, 0, 4);

/// The seed the world is replaced WITH, mid-session.
///
/// A different world and not a different dial: the claim is that nothing built
/// from the first seed survives into the second, and two configs that agreed
/// about every cell would not observe it. Chosen so both replaced cells hold
/// bodies under both seeds and name different objects under each.
#[cfg(feature = "debug")]
const REPLACEMENT_SEED: u32 = 20_260_925;

/// How many pieces of work the replacement beat hands in: one running job and
/// one prepared sector, both for cells the window still wants.
#[cfg(feature = "debug")]
const REPLACED_WORK: usize = 2;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new()
        .with_game_plugins((
            range_plugin,
            world_observer_plugin,
            NovaWorldPlugin::<NovaLayeredWorld>::default(),
        ))
        .build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(streaming_script());
    }

    app.run()
}

fn range_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), boot_free_play);
}

/// Boot into the empty bootstrap. Streaming is NOT armed here: the first claim
/// is about a session that has a camera and a sky and nothing else, and arming
/// it in the same breath would spawn 125 sectors before anything could look.
fn boot_free_play(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(free_play_scenario(
        &game_assets,
        SCENARIO_ID,
        "World Sectors Free Play",
    )));
}

/// The entity each cell's root had before the crossing, so the crossing can
/// name which roots SURVIVED rather than only counting them.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct CrossingBaseline(BTreeMap<SectorCoord, Entity>);

/// The live sector roots, by cell. Panics on a duplicate the same way the
/// streaming loop does - a range that quietly tolerated one would be asserting
/// against a world the game would refuse.
#[cfg(feature = "debug")]
fn live_roots(world: &mut World) -> BTreeMap<SectorCoord, Entity> {
    let mut live = BTreeMap::new();
    let mut query = world.query::<(Entity, &SectorRoot)>();
    for (entity, root) in query.iter(world) {
        assert!(
            live.insert(root.0, entity).is_none(),
            "world sectors: two live roots claim the cell {}",
            root.0
        );
    }
    live
}

/// How many scenario object entities exist, of any kind.
#[cfg(feature = "debug")]
fn scenario_objects(world: &World) -> usize {
    world
        .try_query_filtered::<&EntityId, With<ScenarioScopedMarker>>()
        .map_or(0, |mut query| query.iter(world).count())
}

/// Describe one cell, or fail the run naming the fault.
#[cfg(feature = "debug")]
fn describe<G: SectorGenerator>(coord: SectorCoord, config: &WorldConfig<G>) -> SectorDescription {
    generate_sector(config, coord).unwrap_or_else(|fault| panic!("world sectors: {fault}"))
}

/// Every cell of the armed window, described.
#[cfg(feature = "debug")]
fn describe_window<G: SectorGenerator>(
    centre: SectorCoord,
    config: &WorldConfig<G>,
) -> Vec<SectorDescription> {
    desired_sectors(centre, config.active_radius)
        .into_iter()
        .map(|coord| describe(coord, config))
        .collect()
}

/// Advance once the live root set is exactly the desired set around `centre`.
#[cfg(feature = "debug")]
fn sector_set_is(centre: SectorCoord) -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    Arc::new(move |world: &World| {
        let Some(mut query) = world.try_query::<&SectorRoot>() else {
            return false;
        };
        let live: BTreeSet<SectorCoord> = query.iter(world).map(|root| root.0).collect();
        live == desired_sectors(centre, EXAMPLE_ACTIVE_RADIUS)
    })
}

/// Advance once no sector root is left.
#[cfg(feature = "debug")]
fn sectors_are_gone() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    Arc::new(|world: &World| {
        world
            .try_query::<&SectorRoot>()
            .is_some_and(|mut query| query.iter(world).next().is_none())
    })
}

/// The cells that have a job running, by cell.
#[cfg(feature = "debug")]
fn pending_jobs(world: &mut World) -> BTreeSet<SectorCoord> {
    let mut query = world.query::<&SectorJob>();
    query.iter(world).map(|job| job.coord).collect()
}

/// The cells that are prepared and waiting for a frame.
#[cfg(feature = "debug")]
fn ready_cells(world: &World) -> BTreeSet<SectorCoord> {
    world.resource::<ReadySectors>().0.keys().copied().collect()
}

/// Advance once neither abandoned piece of work is anywhere in the world.
#[cfg(feature = "debug")]
fn abandoned_work_is_gone() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    Arc::new(|world: &World| {
        let job_gone = world
            .try_query::<&SectorJob>()
            .is_some_and(|mut query| query.iter(world).all(|job| job.coord != ABANDONED_JOB_CELL));
        let ready_gone = world
            .get_resource::<ReadySectors>()
            .is_some_and(|ready| !ready.0.contains_key(&ABANDONED_READY_CELL));
        job_gone && ready_gone
    })
}

/// Advance once nothing the previous `WorldConfig` built is left anywhere in
/// the world, and the new one has started building.
///
/// Reads ENTITIES, not coordinates: the replaced window wants the same 125
/// cells the old one did, so a coordinate test could not tell an old root from
/// the new root that replaced it, and both handed-in cells are re-requested
/// under the new config within a frame or two. What became of the handed-in
/// work is a question about CONTENT rather than presence, so the beat also
/// waits for the whole new window and `report_world_replacement` reads what
/// those two cells hold.
#[cfg(feature = "debug")]
fn replaced_world_is_gone() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    Arc::new(|world: &World| {
        let Some(replaced) = world.get_resource::<ReplacedWorld>() else {
            return false;
        };
        let Some(mut roots) = world.try_query::<(Entity, &SectorRoot)>() else {
            return false;
        };
        let mut standing = roots.iter(world).map(|(entity, _)| entity).peekable();
        let rebuilt = standing.peek().is_some();
        let old_roots_gone = standing.all(|entity| !replaced.roots.contains(&entity));
        let old_job_gone = world
            .try_query::<(Entity, &SectorJob)>()
            .is_some_and(|mut query| query.iter(world).all(|(entity, _)| entity != replaced.job));
        rebuilt && old_roots_gone && old_job_gone
    })
}

/// Advance once no job is running and nothing is waiting to be spawned.
#[cfg(feature = "debug")]
fn sector_work_is_gone() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    Arc::new(|world: &World| {
        let no_jobs = world
            .try_query::<&SectorJob>()
            .is_some_and(|mut query| query.iter(world).next().is_none());
        let nothing_ready = world
            .get_resource::<ReadySectors>()
            .is_some_and(|ready| ready.0.is_empty());
        no_jobs && nothing_ready
    })
}

/// Hand the streaming loop work for cells `coords` name, as a running job and
/// a prepared result. Counts the job as requested, the way
/// [`nova_world::request_sectors`] does, and returns the job's entity so a
/// caller can name THAT job again rather than whichever job holds the cell
/// next.
#[cfg(feature = "debug")]
fn hand_in_work(world: &mut World, job_cell: SectorCoord, ready_cell: SectorCoord) -> Entity {
    let config = world.resource::<WorldConfig<NovaLayeredWorld>>().clone();
    let job = world
        .spawn((
            Name::new(format!("Sector Job {job_cell}")),
            SectorJob::start(config.clone(), job_cell),
        ))
        .id();
    world.resource_mut::<SectorJobStats>().requested += 1;

    let prepared =
        prepare_sector(config, ready_cell).unwrap_or_else(|fault| panic!("world sectors: {fault}"));
    world
        .resource_mut::<ReadySectors>()
        .0
        .insert(ready_cell, prepared);
    job
}

/// Advance once the observer has reached its commanded pose.
///
/// Every crossing beat waits on THIS before it waits on the sector set: the
/// scripted pose lands a frame or two after it is commanded, and a set
/// assertion taken from the old pose would be asserting about the wrong
/// centre.
#[cfg(feature = "debug")]
fn observer_at(position: Meters3) -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    Arc::new(move |world: &World| {
        world
            .try_query_filtered::<&GlobalTransform, With<ScenarioCameraMarker>>()
            .and_then(|mut query| query.iter(world).next().copied())
            .is_some_and(|transform| {
                Meters3::from_engine(transform.translation()).distance(position) <= PARKED_TOLERANCE
            })
    })
}

/// Park the observer at `coord`'s centre, looking along +X so a crossing is
/// flown into rather than backed into.
#[cfg(feature = "debug")]
fn park_observer(coord: SectorCoord) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let edge = featured_world_config().sector_edge;
        pose_camera(
            world,
            coord.centre(edge),
            coord.offset(1, 0, 0).centre(edge),
        );
    }
}

#[cfg(feature = "debug")]
fn streaming_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let home = FEATURE_HOME;
    let across = home.offset(1, 0, 0);
    let edge = featured_world_config().sector_edge;

    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the free-play bootstrap")
        .enter(GameStates::Loading)
        .until(and(scenario_is_built(), scenario_camera_present()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("report the empty bootstrap")
        .on_enter(report_empty_bootstrap)
        .add()
        // Pure, so they need no world and no frames: the generator and the
        // feature field are the parts of the kit that can be asked the same
        // question several ways in one beat.
        .step("report the visit-order comparison")
        .on_enter(report_visit_order)
        .add()
        .step("report the feature field")
        .on_enter(report_feature_field)
        .add()
        .step("report the placement rule")
        .on_enter(report_clearance)
        .add()
        .step("park the observer in the feature home sector")
        .on_enter(park_observer(home))
        .until(observer_at(home.centre(edge)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("arm the stream")
        .on_enter(|world: &mut World| {
            world.insert_resource(featured_world_config());
        })
        .until(sector_set_is(home))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("report the initial sector set")
        .on_enter(report_initial_set)
        .add()
        .step("report the preparation lifetime")
        .on_enter(report_preparation)
        .add()
        .step("report the materialized places")
        .on_enter(report_places)
        .add()
        .step("cross the +X boundary")
        .on_enter(park_observer(across))
        .until(and(observer_at(across.centre(edge)), sector_set_is(across)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("report the crossing")
        .on_enter(report_crossing)
        .add()
        .step("return across the boundary")
        .on_enter(park_observer(home))
        .until(and(observer_at(home.centre(edge)), sector_set_is(home)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("report the return")
        .on_enter(report_return)
        .add()
        .step("abandon a job and a prepared sector")
        .on_enter(|world: &mut World| {
            let baseline = *world.resource::<SectorJobStats>();
            world.insert_resource(AbandonBaseline(baseline));
            hand_in_work(world, ABANDONED_JOB_CELL, ABANDONED_READY_CELL);
        })
        .until(abandoned_work_is_gone())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("report the abandoned work")
        .on_enter(report_abandoned_work)
        .add()
        // The config is swapped in the SAME beat the work is handed in, so the
        // only thing that can take either is `clear_sector_work` running in
        // `NovaWorldSystems::Cleanup`, ahead of the stages that would
        // otherwise accept them by coordinate. A beat runs in `PreUpdate`,
        // which is what satisfies nova_world's one ordering rule: a
        // `WorldConfig` writer must land ahead of `Cleanup`, and every stage
        // below refuses the frame if it did not.
        .step("replace the world under the live session")
        .on_enter(|world: &mut World| {
            let roots = live_roots(world).values().copied().collect::<BTreeSet<_>>();
            let stats = *world.resource::<SectorJobStats>();
            let job = hand_in_work(world, REPLACED_JOB_CELL, REPLACED_READY_CELL);
            world.insert_resource(ReplacedWorld { roots, job, stats });
            world.insert_resource(WorldConfig {
                seed: REPLACEMENT_SEED,
                ..featured_world_config()
            });
        })
        .until(and(replaced_world_is_gone(), sector_set_is(home)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("report the world replacement")
        .on_enter(report_world_replacement)
        .add()
        // The work is handed in and the session is killed in the SAME beat, so
        // the streaming stages - all gated on a live scenario - never run
        // again and `clear_sector_work` is the only thing left that can take
        // it. `report_unload` asserts the session really is gone, which is
        // what makes that the only reading.
        .step("unload the free-play session")
        .on_enter(|world: &mut World| {
            hand_in_work(world, SWEPT_JOB_CELL, SWEPT_READY_CELL);
            world.trigger(UnloadScenario);
        })
        .until(and(sectors_are_gone(), sector_work_is_gone()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("report the unload")
        .on_enter(report_unload)
        .add()
}

/// What the job counters read before the abandonment beat handed work in, so
/// the beat can name what IT changed rather than what the session totalled.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct AbandonBaseline(SectorJobStats);

/// Everything the OLD `WorldConfig` had on hand when it was replaced, named by
/// entity so the replacement can be judged on identity rather than on a cell
/// index the new world reuses.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct ReplacedWorld {
    /// Every live sector root the old config built.
    roots: BTreeSet<Entity>,
    /// The job that was running when the config changed.
    job: Entity,
    /// What the job counters read before the beat handed its work in.
    stats: SectorJobStats,
}

/// Claim 1: the session starts empty.
#[cfg(feature = "debug")]
fn report_empty_bootstrap(world: &mut World) {
    let scenario = world
        .resource::<CurrentScenario>()
        .0
        .clone()
        .expect("world sectors: the bootstrap must be the live scenario");
    assert_eq!(
        scenario.id, SCENARIO_ID,
        "world sectors: the run must boot into the free-play bootstrap"
    );
    assert!(
        scenario.events.is_empty(),
        "world sectors: the bootstrap must author no event handlers"
    );
    let objects = scenario_objects(world);
    assert_eq!(
        objects, 0,
        "world sectors: the bootstrap must spawn no scenario object, found {objects}"
    );
    assert!(
        live_roots(world).is_empty(),
        "world sectors: no sector may exist before the stream is armed"
    );
    assert!(
        pending_jobs(world).is_empty(),
        "world sectors: no sector may be preparing before the stream is armed"
    );
    assert!(
        ready_cells(world).is_empty(),
        "world sectors: no sector may be prepared before the stream is armed"
    );
    assert_eq!(
        *world.resource::<SectorJobStats>(),
        SectorJobStats::default(),
        "world sectors: the job lifetime must not have started before the stream is armed"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the free-play bootstrap authors no objects",
        serde_json::json!({ "scenario": SCENARIO_ID }),
    );
    info!("world sectors: the bootstrap is live with no authored object");
}

/// Claims 2 and 3: the generator is a function of the cell, not of the walk,
/// and it refuses what it cannot describe.
///
/// Both generators are compared, because they fail differently: the uniform
/// one could only lose determinism through its own seed stream, while the
/// featured one reads a global field and a thinning halo, and a halo resolved
/// in walk order rather than by rank is exactly the bug that would pass a
/// single-generator check.
#[cfg(feature = "debug")]
fn report_visit_order(world: &mut World) {
    let compared = assert_visit_order(&uniform_world_config(), SectorCoord::ORIGIN)
        + assert_visit_order(&featured_world_config(), FEATURE_HOME);

    nova_probe::probe_marker(
        world,
        "outcome: a sector is the same sector in any visit order",
        serde_json::json!({ "manifests": compared, "generators": 2 }),
    );
    info!(
        "world sectors: {compared} manifests describe identically in three walks, two generators"
    );

    // A quarter of `f32::MAX`, not `f32::MAX`: `WorldConfig::validate` refuses
    // an edge whose own origin window has no representable face, so the wider
    // one never reaches a coordinate conversion. This config is VALID - its
    // window arms - and it is the far CELL that cannot be placed in metres,
    // which is the case a worker has to catch.
    let overflow = WorldConfig {
        sector_edge: Meters(f32::MAX / 4.0),
        ..uniform_world_config()
    };
    overflow
        .validate()
        .expect("the overflow probe must use a config that arms");
    let fault = generate_sector(&overflow, SectorCoord::new(i32::MAX, 0, 0));
    assert!(
        matches!(fault, Err(SectorFault::InvalidGeometry { .. })),
        "world sectors: coordinate-to-meter overflow must refuse before materialization, got {fault:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: invalid sector geometry refuses before materialization",
        serde_json::json!({}),
    );
    info!("world sectors: invalid generated geometry refused before spawn");
}

/// Describe the window around `centre` forward, backward and by stride, fail
/// the run unless all three walks give the same manifests, and return how
/// many cells were compared.
///
/// Generic, and called once per generator: each generator is its own
/// `WorldConfig<G>` type, so one array of both configs cannot exist.
#[cfg(feature = "debug")]
fn assert_visit_order<G: SectorGenerator>(config: &WorldConfig<G>, centre: SectorCoord) -> usize {
    let generator = std::any::type_name::<G>();
    let cells: Vec<SectorCoord> = desired_sectors(centre, config.active_radius)
        .into_iter()
        .collect();

    let describe_all = |order: &[SectorCoord]| {
        let mut described: BTreeMap<SectorCoord, String> = BTreeMap::new();
        for coord in order {
            described.insert(*coord, describe(*coord, config).canonical());
        }
        described
    };

    let forward = describe_all(&cells);
    let backward = describe_all(&cells.iter().rev().copied().collect::<Vec<_>>());
    // A third walk that is neither the order the set iterates in nor its
    // reverse: two coprime strides visit every cell in an order no collection
    // here produces on its own.
    let strided: Vec<SectorCoord> = (0..cells.len())
        .map(|step| cells[step * 7 % cells.len()])
        .collect();
    let strided = describe_all(&strided);

    assert_eq!(
        forward.len(),
        DESIRED_ROOTS,
        "world sectors: the comparison must cover the whole desired set"
    );
    assert_eq!(
        forward, backward,
        "world sectors: reversing the walk must not change a single sector under {generator}"
    );
    assert_eq!(
        forward, strided,
        "world sectors: striding the walk must not change a single sector under {generator}"
    );
    forward.len()
}

/// Claims 4, 5 and 6: the feature field is one world, thinned within a layer
/// and free across layers.
///
/// The field is the base generator's, not the streamed world's: a manifest
/// carries bodies only, so each cell's spheres are asked of
/// `nova_world_base::sector_features` directly, with the same input the
/// generator was given.
#[cfg(feature = "debug")]
fn report_feature_field(world: &mut World) {
    let config = featured_world_config();

    // One sphere, however many cells can see it. Built as id -> every copy
    // handed out, so a disagreement names the sphere rather than the cell.
    let mut seen: BTreeMap<String, Vec<(SectorCoord, FeatureSphere)>> = BTreeMap::new();
    for coord in desired_sectors(FEATURE_HOME, config.active_radius) {
        let spheres = sector_features(config.input(coord))
            .unwrap_or_else(|fault| panic!("world sectors: {coord}: {fault}"));
        for sphere in spheres {
            seen.entry(sphere.id.clone())
                .or_default()
                .push((coord, sphere));
        }
    }
    assert!(
        !seen.is_empty(),
        "world sectors: the window around {FEATURE_HOME} must contain feature spheres, \
         or every claim below is vacuous"
    );

    let mut shared = 0;
    for (id, copies) in &seen {
        let (first_cell, first) = &copies[0];
        for (cell, other) in &copies[1..] {
            assert_eq!(
                first, other,
                "world sectors: sphere '{id}' reads differently in {cell} than in {first_cell}"
            );
        }
        if copies.len() > 1 {
            shared += 1;
        }
        let owners = copies
            .iter()
            .filter(|(cell, sphere)| sphere.owner == *cell)
            .count();
        assert!(
            owners <= 1,
            "world sectors: sphere '{id}' is owned by {owners} of the cells that see it"
        );
    }
    assert!(
        shared > 0,
        "world sectors: no sphere crosses a cell boundary, so the shared-sphere claim \
         proves nothing"
    );

    let spheres: Vec<&FeatureSphere> = seen.values().map(|copies| &copies[0].1).collect();
    let separation = |a: &FeatureSphere, b: &FeatureSphere| {
        a.centre.distance(b.centre).get() - (a.radius.get() + b.radius.get())
    };

    let mut same_layer_pairs = 0;
    let mut crossing_pairs = 0;
    for (index, a) in spheres.iter().enumerate() {
        for b in &spheres[index + 1..] {
            if a.layer == b.layer {
                same_layer_pairs += 1;
                assert!(
                    separation(a, b) >= 0.0,
                    "world sectors: {} spheres '{}' and '{}' overlap by {:.0} m; one layer \
                     must be thinned",
                    a.layer,
                    a.id,
                    b.id,
                    -separation(a, b)
                );
            } else if separation(a, b) < 0.0 {
                crossing_pairs += 1;
            }
        }
    }
    assert!(
        same_layer_pairs > 0,
        "world sectors: no two spheres share a layer here, so the thinning claim proves \
         nothing"
    );
    assert!(
        crossing_pairs > 0,
        "world sectors: no two layers overlap anywhere in the window, so the layers are \
         not independent in practice, whatever the rule says"
    );

    let owned = FeatureLayer::ALL.map(|layer| {
        spheres
            .iter()
            .filter(|sphere| sphere.layer == layer)
            .count()
    });
    nova_probe::probe_marker(
        world,
        "outcome: one feature sphere is one sphere from every cell that sees it",
        serde_json::json!({ "spheres": spheres.len(), "crossing_cells": shared }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: same-layer feature spheres never overlap",
        serde_json::json!({ "same_layer_pairs": same_layer_pairs }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: independent feature layers may overlap",
        serde_json::json!({ "overlapping_pairs": crossing_pairs }),
    );
    info!(
        "world sectors: {} spheres ({} asteroid, {} planet, {} derelict), {shared} of them \
         seen from more than one cell; {same_layer_pairs} same-layer pairs all clear, \
         {crossing_pairs} cross-layer pairs overlap",
        spheres.len(),
        owned[FeatureLayer::Asteroid.index()],
        owned[FeatureLayer::Planet.index()],
        owned[FeatureLayer::Derelict.index()],
    );
}

/// Claim 7: everything a cell places stands inside that cell and clear of
/// everything else in it.
///
/// The clearance radii are recomputed HERE from the published constants rather
/// than read back off the generator, so the claim is a second opinion about
/// the rule instead of a restatement of whatever the generator did. The inset
/// and the margin are the base generator's own `PLACEMENT_INSET` and
/// `CLEARANCE_MARGIN`: `nova_world` itself refuses only an overlap.
#[cfg(feature = "debug")]
fn report_clearance(world: &mut World) {
    let config = featured_world_config();
    let inset = config.sector_edge.get() * 0.5 * PLACEMENT_INSET;
    let described = describe_window(FEATURE_HOME, &config);

    let mut objects = 0;
    let mut pairs = 0;
    for description in &described {
        let cell_centre = description.coord().centre(config.sector_edge);
        let placed: Vec<(String, Meters3, Meters)> = description
            .asteroids()
            .iter()
            .map(|body| {
                (
                    body.id.clone(),
                    body.position,
                    // A rock's meshed surface reaches up to this multiple past
                    // its nominal radius, so that is what has to clear.
                    Meters(body.radius.get() * ASTEROID_GEOMETRIC_FACTOR_MAX),
                )
            })
            .chain(description.planets().iter().map(|planet| {
                (
                    planet.id.clone(),
                    planet.position,
                    planet.config.body_radius(),
                )
            }))
            .chain(
                description
                    .ships()
                    .iter()
                    .map(|ship| (ship.id.clone(), ship.position, SECTOR_SHIP_CLEARANCE)),
            )
            .collect();
        objects += placed.len();

        for (id, position, _) in &placed {
            let offset = (position.get() - cell_centre.get()).abs();
            assert!(
                offset.max_element() <= inset,
                "world sectors: '{id}' stands {:.0} m off the centre of {}, past the \
                 {inset:.0} m inset - it would be half in the neighbour",
                offset.max_element(),
                description.coord()
            );
        }
        for (index, (id, position, clearance)) in placed.iter().enumerate() {
            for (other_id, other_position, other_clearance) in &placed[index + 1..] {
                let gap = position.distance(*other_position).get()
                    - clearance.get()
                    - other_clearance.get();
                assert!(
                    gap + CLEARANCE_SLACK.get() >= CLEARANCE_MARGIN.get(),
                    "world sectors: '{id}' and '{other_id}' in {} keep only {gap:.0} m \
                     between their surfaces, under the {:.0} m the placement rule promises",
                    description.coord(),
                    CLEARANCE_MARGIN.get()
                );
                pairs += 1;
            }
        }
    }

    assert!(
        pairs > 0,
        "world sectors: no cell in the window holds two objects, so the clearance claim \
         proves nothing"
    );
    nova_probe::probe_marker(
        world,
        "outcome: every physical object stands clear inside its own sector",
        serde_json::json!({ "objects": objects, "pairs": pairs }),
    );
    info!(
        "world sectors: {objects} placed objects across {} cells, {pairs} same-cell pairs \
         all clear and inside the inset",
        described.len()
    );
}

/// Claim 8: arming brings up exactly the desired set, and each root owns
/// exactly what its manifest names.
#[cfg(feature = "debug")]
fn report_initial_set(world: &mut World) {
    let config = world.resource::<WorldConfig<NovaLayeredWorld>>().clone();
    let live = live_roots(world);
    assert_eq!(
        live.len(),
        DESIRED_ROOTS,
        "world sectors: arming must materialize {DESIRED_ROOTS} roots"
    );
    assert_eq!(
        live.keys().copied().collect::<BTreeSet<_>>(),
        desired_sectors(FEATURE_HOME, config.active_radius),
        "world sectors: the live set must be the desired set"
    );

    let mut objects = 0;
    for (coord, entity) in &live {
        assert!(
            world.get::<ScenarioScopedMarker>(*entity).is_some(),
            "world sectors: the root for {coord} must be scenario-scoped, or the \
             session sweep cannot reach it"
        );
        let expected = describe(*coord, &config).object_count();
        let children = world
            .get::<Children>(*entity)
            .map_or(0, |children| children.len());
        assert_eq!(
            children, expected,
            "world sectors: the root for {coord} must own the {expected} object(s) its \
             manifest names"
        );
        objects += expected;
    }
    assert!(
        objects > 0,
        "world sectors: the armed window must contain objects"
    );

    world.insert_resource(CrossingBaseline(live.clone()));
    nova_probe::probe_marker(
        world,
        "outcome: arming the stream materializes the whole desired set",
        serde_json::json!({ "roots": live.len(), "objects": objects }),
    );
    info!(
        "world sectors: {} roots live around {FEATURE_HOME}, holding {objects} objects",
        live.len()
    );
}

/// Claim 9: no sector was conjured inside the frame that needed it, and the
/// window never asked for more work than the machine can run.
///
/// The counters are session totals, so this reads them at the one point they
/// have an exact expected value: the desired set is live, nothing has left it,
/// and nothing else has ever been asked for. `peak_pending` carries two
/// claims. It must never exceed the job cap
/// [`nova_world::request_sectors`] enforces, which is what makes 125
/// desired cells cost a poolful of preparations rather than 125 of them at
/// once. And where the pool has more than one thread it must reach at least
/// two, which is the overlap a `generate-and-spawn` loop cannot produce. A
/// single-threaded pool - what wasm has - is allowed to peak at one, because
/// there the split buys the frame and the cancellation rather than
/// parallelism.
///
/// Nothing here is a timing assertion. It counts jobs, not milliseconds.
#[cfg(feature = "debug")]
fn report_preparation(world: &mut World) {
    let stats = *world.resource::<SectorJobStats>();
    let job_limit = bevy::tasks::AsyncComputeTaskPool::get().thread_num().max(1);
    assert_eq!(
        stats.requested, DESIRED_ROOTS,
        "world sectors: arming must request each desired cell exactly once, got {stats:?}"
    );
    assert!(
        stats.peak_pending <= job_limit,
        "world sectors: no more than {job_limit} preparation(s) may be in flight at once, \
         got {stats:?}"
    );
    assert!(
        job_limit == 1 || stats.peak_pending >= 2,
        "world sectors: a {job_limit}-thread pool must overlap at least two preparations, \
         got {stats:?}"
    );
    assert_eq!(
        stats.completed, DESIRED_ROOTS,
        "world sectors: every requested job must come back, got {stats:?}"
    );
    assert_eq!(
        stats.materialized, DESIRED_ROOTS,
        "world sectors: every live sector must have been spawned from a prepared result, \
         got {stats:?}"
    );
    assert_eq!(
        stats.discarded, 0,
        "world sectors: nothing left the desired set, so nothing may be discarded, \
         got {stats:?}"
    );
    assert!(
        pending_jobs(world).is_empty() && ready_cells(world).is_empty(),
        "world sectors: the desired set is live, so no job or prepared result may be left over"
    );

    nova_probe::probe_marker(
        world,
        "outcome: every sector is requested and prepared before it is materialized",
        serde_json::json!({
            "requested": stats.requested,
            "peak_pending": stats.peak_pending,
            "job_limit": job_limit,
            "completed": stats.completed,
            "materialized": stats.materialized,
        }),
    );
    info!(
        "world sectors: {} jobs requested, at most {} of {job_limit} in flight at once, \
         {} materialized",
        stats.requested, stats.peak_pending, stats.materialized
    );
}

/// Claim 10: a planetoid a manifest names is a real world, and a derelict it
/// names is a ship nobody is flying.
///
/// The counts alone would pass on a sector that spawned rocks named
/// `..._planet_0`. What is read here is the COMPONENTS the game's own object
/// factories insert: `PlanetMarker` for a world, `SpaceshipRootMarker` with
/// `SpaceshipController::None` and `Allegiance::Neutral` for a derelict -
/// and each of them under the root of the cell whose manifest names it. The
/// manifest carries no feature sphere, so which sphere placed a body is
/// proved in `nova_world_base`, not here.
#[cfg(feature = "debug")]
fn report_places(world: &mut World) {
    let config = world.resource::<WorldConfig<NovaLayeredWorld>>().clone();
    let live = live_roots(world);

    let mut planets = 0;
    let mut ships = 0;
    for (coord, root) in &live {
        let description = describe(*coord, &config);
        if description.planets().is_empty() && description.ships().is_empty() {
            continue;
        }
        let children: Vec<Entity> = world
            .get::<Children>(*root)
            .map_or_else(Vec::new, |children| children.iter().collect());
        let by_id: BTreeMap<String, Entity> = children
            .iter()
            .filter_map(|child| {
                world
                    .get::<EntityId>(*child)
                    .map(|id| (id.0.clone(), *child))
            })
            .collect();

        for planet in description.planets() {
            let entity = *by_id.get(&planet.id).unwrap_or_else(|| {
                panic!(
                    "world sectors: {coord} describes planetoid '{}' but spawned none",
                    planet.id
                )
            });
            assert!(
                world.get::<PlanetMarker>(entity).is_some(),
                "world sectors: '{}' must be a real planet body, not a big rock",
                planet.id
            );
            assert!(
                world.get::<AsteroidMarker>(entity).is_none(),
                "world sectors: '{}' must not also be an asteroid",
                planet.id
            );
            planets += 1;
        }

        for ship in description.ships() {
            let entity = *by_id.get(&ship.id).unwrap_or_else(|| {
                panic!(
                    "world sectors: {coord} describes derelict '{}' but spawned none",
                    ship.id
                )
            });
            assert!(
                world.get::<SpaceshipRootMarker>(entity).is_some(),
                "world sectors: '{}' must be a real ship root",
                ship.id
            );
            assert!(
                matches!(
                    world.get::<SpaceshipController>(entity),
                    Some(SpaceshipController::None)
                ),
                "world sectors: '{}' must have nobody aboard",
                ship.id
            );
            assert_eq!(
                world.get::<Allegiance>(entity).copied(),
                Some(Allegiance::Neutral),
                "world sectors: '{}' must be neutral, or the AI will shoot the furniture",
                ship.id
            );
            ships += 1;
        }
    }

    assert!(
        planets > 0,
        "world sectors: the window around {FEATURE_HOME} must own at least one planetoid, \
         or this claim proves nothing"
    );
    assert!(
        ships > 0,
        "world sectors: the window around {FEATURE_HOME} must own at least one derelict, \
         or this claim proves nothing"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a featured sector owns real planetoids and inert derelict ships",
        serde_json::json!({ "planetoids": planets, "ships": ships }),
    );
    info!("world sectors: {planets} planetoid(s) and {ships} inert derelict ship(s) are live");
}

/// Claim 11: a crossing moves the window, it does not rebuild it.
#[cfg(feature = "debug")]
fn report_crossing(world: &mut World) {
    let before = world.resource::<CrossingBaseline>().0.clone();
    let after = live_roots(world);

    let retained: Vec<SectorCoord> = after
        .iter()
        .filter(|(coord, entity)| before.get(coord) == Some(entity))
        .map(|(coord, _)| *coord)
        .collect();
    let retired: Vec<SectorCoord> = before
        .keys()
        .filter(|coord| !after.contains_key(coord))
        .copied()
        .collect();
    let added: Vec<SectorCoord> = after
        .keys()
        .filter(|coord| !before.contains_key(coord))
        .copied()
        .collect();

    assert_eq!(
        retained.len(),
        RETAINED_ROOTS,
        "world sectors: a one-cell crossing must keep the shared slab as the SAME \
         entities, kept {retained:?}"
    );
    assert_eq!(
        retired.len(),
        SWAPPED_ROOTS,
        "world sectors: a one-cell crossing must retire the face left behind, \
         retired {retired:?}"
    );
    assert_eq!(
        added.len(),
        SWAPPED_ROOTS,
        "world sectors: a one-cell crossing must add the face entered, added {added:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: crossing one boundary retains the shared slab and swaps a face",
        serde_json::json!({
            "retained": retained.len(),
            "retired": retired.len(),
            "added": added.len(),
        }),
    );
    info!(
        "world sectors: the crossing kept {}, retired {}, added {}",
        retained.len(),
        retired.len(),
        added.len()
    );
}

/// Claim 12: coming back is the same place, not a second copy of it.
#[cfg(feature = "debug")]
fn report_return(world: &mut World) {
    let config = world.resource::<WorldConfig<NovaLayeredWorld>>().clone();
    let live = live_roots(world);
    assert_eq!(
        live.keys().copied().collect::<BTreeSet<_>>(),
        desired_sectors(FEATURE_HOME, config.active_radius),
        "world sectors: the return must be the original set"
    );
    assert_eq!(
        live.len(),
        DESIRED_ROOTS,
        "world sectors: the return must hold one root per cell"
    );

    // The set being right is not enough: a sector that came back with someone
    // else's rocks would still count 125. Each returned root has to hold the
    // objects its own manifest names.
    for (coord, entity) in &live {
        let described = describe(*coord, &config)
            .object_ids()
            .into_iter()
            .collect::<BTreeSet<String>>();
        let children = world
            .get::<Children>(*entity)
            .map_or_else(Vec::new, |children| children.iter().collect());
        let spawned: BTreeSet<String> = children
            .iter()
            .filter_map(|child| world.get::<EntityId>(*child))
            .map(|id| id.0.clone())
            .collect();
        assert_eq!(
            spawned, described,
            "world sectors: the returned root for {coord} must hold the objects its \
             manifest names"
        );
    }

    nova_probe::probe_marker(
        world,
        "outcome: the return trip leaves no duplicate root",
        serde_json::json!({ "roots": live.len() }),
    );
    info!(
        "world sectors: the return set is the original {}",
        live.len()
    );
}

/// Claim 13: work the observer walked away from is dropped, not spawned.
///
/// The two pieces cover the two ways prepared work can go stale: a job still
/// running when its cell stops being wanted, and a result that finished and
/// then had nowhere to go. Which of the two paths took the job - cancelled by
/// [`nova_world::retire_sectors`] or dropped on arrival by
/// [`nova_world::collect_sector_jobs`] - depends on how fast the worker was
/// and is deliberately NOT asserted. What is asserted is that neither ever
/// became a sector.
#[cfg(feature = "debug")]
fn report_abandoned_work(world: &mut World) {
    let baseline = world.resource::<AbandonBaseline>().0;
    let stats = *world.resource::<SectorJobStats>();
    let config = world.resource::<WorldConfig<NovaLayeredWorld>>().clone();

    assert_eq!(
        stats.materialized, baseline.materialized,
        "world sectors: abandoned work must not spawn a sector, {baseline:?} -> {stats:?}"
    );
    assert_eq!(
        stats.discarded,
        baseline.discarded + ABANDONED_WORK,
        "world sectors: both abandoned pieces of work must be discarded, \
         {baseline:?} -> {stats:?}"
    );

    let live = live_roots(world);
    assert_eq!(
        live.keys().copied().collect::<BTreeSet<_>>(),
        desired_sectors(FEATURE_HOME, config.active_radius),
        "world sectors: abandoned work must not move the live set"
    );
    for cell in [ABANDONED_JOB_CELL, ABANDONED_READY_CELL] {
        assert!(
            !live.contains_key(&cell),
            "world sectors: {cell} was never desired and must never be live"
        );
    }

    nova_probe::probe_marker(
        world,
        "outcome: work for an undesired sector never materializes",
        serde_json::json!({
            "abandoned": ABANDONED_WORK,
            "materialized": stats.materialized,
            "roots": live.len(),
        }),
    );
    info!(
        "world sectors: {ABANDONED_WORK} abandoned pieces of work were discarded and \
         {} roots stand",
        live.len()
    );
}

/// Claim 14: replacing the `WorldConfig` replaces the WORLD, and nothing the
/// old one built or had in flight crosses over.
///
/// The hole this closes is that every piece of streaming state is keyed by
/// CELL and the replaced window wants the same 125 cells: a root, a running
/// job and a prepared payload built from the old seed all look exactly like
/// the new world's own work to a loop that only compares coordinates. So the
/// assertions read entities and contents, and the two configs are first shown
/// to disagree about both handed-in cells - a claim about a swap nobody could
/// observe would be no claim at all.
///
/// Both handed-in cells are inside the window and the observer does not move,
/// so neither [`nova_world::retire_sectors`] nor the stale-completion path in
/// [`nova_world::collect_sector_jobs`] can take them: only
/// `clear_sector_work` can. The counters name which path ran. The old job is
/// discarded without ever completing, and each of the 125 cells is requested,
/// completed and materialized exactly once under the new config.
#[cfg(feature = "debug")]
fn report_world_replacement(world: &mut World) {
    let replaced = world.resource::<ReplacedWorld>();
    let baseline_roots = replaced.roots.clone();
    let baseline = replaced.stats;
    let old_job = replaced.job;

    let old_config = featured_world_config();
    let new_config = world.resource::<WorldConfig<NovaLayeredWorld>>().clone();
    assert_eq!(
        new_config.seed, REPLACEMENT_SEED,
        "world sectors: the replacement beat must leave the new config in place"
    );
    let desired = desired_sectors(FEATURE_HOME, new_config.active_radius);
    let mut replaced_cells = BTreeMap::new();
    for cell in [REPLACED_JOB_CELL, REPLACED_READY_CELL] {
        assert!(
            desired.contains(&cell),
            "world sectors: {cell} must be inside the window around {FEATURE_HOME}, or a \
             stale-result discard could take its work instead of clear_sector_work"
        );
        let old_ids: BTreeSet<String> = describe(cell, &old_config)
            .object_ids()
            .into_iter()
            .collect();
        let new_ids: BTreeSet<String> = describe(cell, &new_config)
            .object_ids()
            .into_iter()
            .collect();
        assert!(
            !old_ids.is_empty() && !new_ids.is_empty(),
            "world sectors: {cell} must hold bodies under both seeds, old {old_ids:?} new \
             {new_ids:?}, or the replacement claim observes nothing there"
        );
        assert_ne!(
            old_ids, new_ids,
            "world sectors: the two configs must name different objects in {cell}, or a stale \
             payload there would look like the new world's own"
        );
        replaced_cells.insert(cell, (old_ids, new_ids));
    }

    let live = live_roots(world);
    let survivors: Vec<SectorCoord> = live
        .iter()
        .filter(|(_, entity)| baseline_roots.contains(entity))
        .map(|(coord, _)| *coord)
        .collect();
    assert!(
        survivors.is_empty(),
        "world sectors: no root the old config built may survive the swap, found {survivors:?}"
    );
    assert_eq!(
        live.keys().copied().collect::<BTreeSet<_>>(),
        desired,
        "world sectors: the new config must rebuild exactly the window around {FEATURE_HOME}"
    );

    let old_job_alive = world
        .get_entity(old_job)
        .is_ok_and(|entity| entity.contains::<SectorJob>());
    assert!(
        !old_job_alive,
        "world sectors: the job started under the old config must be dropped, not collected \
         into the new world"
    );
    for (cell, (old_ids, new_ids)) in &replaced_cells {
        let children = world
            .get::<Children>(live[cell])
            .map_or_else(Vec::new, |children| children.iter().collect());
        let spawned: BTreeSet<String> = children
            .iter()
            .filter_map(|child| world.get::<EntityId>(*child))
            .map(|id| id.0.clone())
            .collect();
        assert_eq!(
            &spawned, new_ids,
            "world sectors: {cell} must hold the new config's objects, not the {old_ids:?} \
             prepared before the swap"
        );
    }

    let stats = *world.resource::<SectorJobStats>();
    let expected = SectorJobStats {
        requested: baseline.requested + 1 + DESIRED_ROOTS,
        completed: baseline.completed + DESIRED_ROOTS,
        materialized: baseline.materialized + DESIRED_ROOTS,
        discarded: baseline.discarded + REPLACED_WORK,
        peak_pending: stats.peak_pending,
    };
    assert_eq!(
        stats, expected,
        "world sectors: the swap must discard exactly the {REPLACED_WORK} handed-in pieces of \
         work, the old job uncompleted, and build each of the {DESIRED_ROOTS} new cells once, \
         {baseline:?} -> {stats:?}"
    );

    nova_probe::probe_marker(
        world,
        "outcome: replacing the world config retires the world it built",
        serde_json::json!({
            "retired": baseline_roots.len(),
            "discarded": REPLACED_WORK,
            "survivors": 0,
            "rebuilt": live.len(),
        }),
    );
    info!(
        "world sectors: the swap retired {} roots and dropped {REPLACED_WORK} pieces of work, \
         and {} new roots stand",
        baseline_roots.len(),
        live.len()
    );
}

/// Claim 15: the session sweep is still the outer owner, and it reaches the
/// work as well as the world.
#[cfg(feature = "debug")]
fn report_unload(world: &mut World) {
    assert!(
        live_roots(world).is_empty(),
        "world sectors: UnloadScenario must leave no sector root"
    );
    let objects = scenario_objects(world);
    assert_eq!(
        objects, 0,
        "world sectors: UnloadScenario must leave no scenario object, found {objects}"
    );

    // The work handed in beside the unload trigger is NOT scenario-scoped, so
    // the scenario sweep cannot see it. Only `clear_sector_work` can, and it
    // is the only streaming stage that runs with no live session - which the
    // next assertion is what establishes.
    assert!(
        world.resource::<CurrentScenario>().0.is_none(),
        "world sectors: the session must be gone, or a streaming stage could have \
         taken the work instead of the sweep"
    );
    let jobs = pending_jobs(world);
    assert!(
        jobs.is_empty(),
        "world sectors: UnloadScenario must leave no preparing sector, found {jobs:?}"
    );
    let ready = ready_cells(world);
    assert!(
        ready.is_empty(),
        "world sectors: UnloadScenario must leave no prepared sector, found {ready:?}"
    );

    nova_probe::probe_marker(
        world,
        "outcome: unloading the session removes every sector root",
        serde_json::json!({ "roots": 0, "jobs": 0, "ready": 0 }),
    );
    info!("world sectors: the session sweep left nothing behind, work included");
}
