//! The job lifetime: which cells the observer wants, how they are prepared
//! off the frame, how one becomes entities, and how it is taken away again.
//!
//! Routine sector traffic never touches `LoadScenario` and never runs a
//! scenario event action. It calls the same object factories the loader calls
//! (`base_scenario_object`, `asteroid_scenario_object_prepared`,
//! `planet_scenario_object_prepared`, `spaceship_scenario_object`), which is
//! what makes a streamed rock the same rock a scenario spawns - with one
//! deliberate difference: a streamed entity is scenario-SCOPED, so unloading
//! takes it, but never scenario-ADDRESSABLE, because its id came from a
//! coordinate and no author wrote it.

use std::collections::{BTreeMap, BTreeSet};

use bevy::{
    ecs::change_detection::{CheckChangeTicks, Tick},
    prelude::*,
    tasks::{block_on, poll_once, AsyncComputeTaskPool, Task},
};
use nova_assets::prelude::GameAssets;
use nova_events::prelude::Meters3;
use nova_gameplay::prelude::{Allegiance, AssetRef};
use nova_scenario::prelude::{
    asteroid_scenario_object_prepared, base_scenario_object, planet_scenario_object_prepared,
    spaceship_scenario_object, AsteroidConfig, BaseScenarioObjectConfig, GameShipDesigns,
    ShipDesignSource, SpaceshipConfig, SpaceshipController,
};

use crate::{
    prepare_sector, PreparedSector, SectorCoord, SectorFault, SectorGenerator, WorldConfig,
};

/// Marks the entity the desired set is centred on.
///
/// A component rather than a camera query, because who the world streams
/// around is the CALLER's decision: a free-fly observer in an example, the
/// player's ship in the game, and a scripted eye in a range are all the same
/// question asked of different entities. Exactly one entity may carry it;
/// [`track_current_sector`] refuses any other count rather than guessing a
/// centre and silently moving the world.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct WorldObserver;

/// Marks a sector root and names the cell it owns.
///
/// The narrow cleanup scope. The root's children are the sector's objects, so
/// despawning it retires exactly one sector; the `ScenarioScopedMarker` it
/// also carries keeps the session sweep able to take it.
#[derive(Component, Clone, Copy, Debug)]
pub struct SectorRoot(pub SectorCoord);

/// Which cell the [`WorldObserver`] is in. Written by
/// [`track_current_sector`], and the centre every desired-set decision in the
/// job lifetime is taken against.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CurrentSector(pub SectorCoord);

/// The cells the observer wants live: the cube of side `2 * radius + 1`
/// centred on `centre`.
///
/// Built whole, and built again by three stages of every frame, so the cost is
/// cubic in `radius`.
///
/// # Panics
///
/// [`SectorFault::Config`] on a window above
/// [`crate::ACTIVE_WINDOW_SECTORS_MAX`], refused HERE and not only in
/// [`WorldConfig::validate`]. This is the allocation, and the resource can be
/// swapped by a caller's own system between the stage that validates a change
/// and the stage that reads it, so a bound that lived only in the config would
/// be a bound the allocation never saw.
///
/// And on a window that runs off the `i32` grid. `SectorCoord::containing`
/// converts with an `as` cast, which SATURATES, so a far enough observer
/// stands in cell `i32::MAX` and the offsets around it would panic in a debug
/// build and WRAP to the far side of the world in a release one. Refused
/// rather than clipped: a window quietly missing the half nobody could
/// represent is a world that thins out for a reason no reader can see.
pub fn desired_sectors(centre: SectorCoord, radius: i32) -> BTreeSet<SectorCoord> {
    if let Err(fault) = crate::window_cells(radius) {
        panic!("nova_world: {fault}");
    }
    let representable =
        |axis: i32| axis.checked_add(radius).is_some() && axis.checked_sub(radius).is_some();
    assert!(
        representable(centre.x) && representable(centre.y) && representable(centre.z),
        "nova_world: a window of radius {radius} around {centre} runs off the i32 sector grid"
    );
    let mut desired = BTreeSet::new();
    for x in -radius..=radius {
        for y in -radius..=radius {
            for z in -radius..=radius {
                desired.insert(centre.offset(x, y, z));
            }
        }
    }
    desired
}

/// Spawn one prepared sector: the sector root, and one child per object the
/// description names - rocks, then planetoids, then ships. Returns the root.
///
/// The root is an OWNERSHIP node and not a pose - it stays at the world origin
/// and its children carry world positions. `base_scenario_object` seeds a
/// `GlobalTransform` from the entity's own `Transform`, and avian's
/// `transform_to_position` reads that seed in `FixedPostUpdate`, a schedule
/// ahead of bevy's `TransformSystems::Propagate`. A root posed at the sector
/// centre would therefore hand every child body a global position equal to its
/// LOCAL offset, and a rigid body's `Position` is authoritative from then on.
///
/// SCOPED but not ADDRESSABLE. `base_scenario_object` gives each entity a
/// `ScenarioScopedMarker`, which is what makes `UnloadScenario` sweep the
/// streamed world, and an `EntityId` the range reads to check a cell against
/// its manifest. It does NOT add `ScenarioAddressableMarker`, which the
/// authored spawn seam grants: a cell id is derived from a coordinate, so a
/// scenario that happened to author an object named `sector_0_0_0` would
/// otherwise have every despawn and objective-marker action for that id reach
/// into the streamed world too. The marker lives in `nova_events` beside
/// `EntityId`, so `nova_ship` enforces the same rule when it resolves an
/// authored well target of an orbit directive or an Orbit helm order - a
/// streamed planetoid carries a `GravityWell` and would otherwise be orbitable
/// by name. A nearest-well target reaches it on purpose: it names no id.
///
/// # Panics
///
/// When `prepared` carries a different number of rocks than asteroids or a
/// different number of surfaces than planetoids - zipping a short list would
/// silently spawn a sector missing its tail - and on
/// [`SectorFault::UnknownShip`] when a ship names a design the loaded catalog
/// does not hold. The catalog is a main-thread resource, so this is
/// the first place the id CAN be checked, and it is checked before the root is
/// spawned.
pub fn materialize_sector(
    commands: &mut Commands,
    prepared: PreparedSector,
    texture: &AssetRef<Image>,
    designs: &GameShipDesigns,
) -> Entity {
    let PreparedSector {
        description,
        asteroids,
        planets,
    } = prepared;
    let coord = description.coord;
    assert_eq!(
        description.asteroids.len(),
        asteroids.len(),
        "nova_world: {coord} was prepared with {} rocks for {} asteroids",
        asteroids.len(),
        description.asteroids.len()
    );
    assert_eq!(
        description.planets.len(),
        planets.len(),
        "nova_world: {coord} was prepared with {} surfaces for {} planetoids",
        planets.len(),
        description.planets.len()
    );
    for ship in &description.ships {
        assert!(
            designs.get_design(&ship.design).is_some(),
            "nova_world: {}",
            SectorFault::UnknownShip {
                id: ship.id.clone(),
                design: ship.design.clone(),
            }
        );
    }

    let objects = description.object_count();
    let root = commands
        .spawn((
            base_scenario_object(&BaseScenarioObjectConfig {
                id: coord.slug(),
                name: format!("Sector {coord}"),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            }),
            SectorRoot(coord),
        ))
        .id();

    for (body, rock) in description.asteroids.into_iter().zip(asteroids) {
        let mut entity = commands.spawn((
            base_scenario_object(&BaseScenarioObjectConfig {
                id: body.id.clone(),
                name: body.id.clone(),
                position: body.position,
                rotation: Quat::IDENTITY,
            }),
            ChildOf(root),
        ));
        asteroid_scenario_object_prepared(
            &mut entity,
            AsteroidConfig {
                radius: body.radius,
                texture: texture.clone(),
                kind: body.kind.clone(),
                destroy_sound: None,
                mass: body.mass,
                lock_signature: None,
                seed: Some(body.seed),
            },
            body.seed,
            rock,
        );
    }

    for (planet, surface) in description.planets.into_iter().zip(planets) {
        let mut entity = commands.spawn((
            base_scenario_object(&BaseScenarioObjectConfig {
                id: planet.id.clone(),
                name: planet.id.clone(),
                position: planet.position,
                rotation: Quat::IDENTITY,
            }),
            ChildOf(root),
        ));
        planet_scenario_object_prepared(&mut entity, surface);
    }

    for ship in description.ships {
        commands.spawn((
            base_scenario_object(&BaseScenarioObjectConfig {
                id: ship.id.clone(),
                name: ship.id.clone(),
                position: ship.position,
                rotation: Quat::from_rotation_y(ship.yaw),
            }),
            spaceship_scenario_object(SpaceshipConfig {
                design: ShipDesignSource::Prototype {
                    id: ship.design.clone(),
                    section_patches: BTreeMap::new(),
                },
                // Nobody aboard and nobody's side: a generated ship is scenery
                // with a hull, and an AI that shot it would be shooting the
                // furniture. The allegiance is inserted beside the bundle for
                // the same reason the scenario loader does it - the controller
                // marker's requirement default would otherwise decide.
                controller: SpaceshipController::None,
                allegiance: Some(Allegiance::Neutral),
                ..default()
            }),
            Allegiance::Neutral,
            ChildOf(root),
        ));
    }

    debug!("nova_world: materialized {coord} with {objects} object(s)");
    root
}

/// The live sector roots, keyed by cell.
///
/// # Panics
///
/// [`SectorFault::DuplicateRoot`] when two roots claim one cell. The live set
/// is read from the world every frame rather than mirrored in a resource, so
/// this is the only place the two could ever disagree - and a duplicate means
/// a retirement was missed, which is the leak the whole nested-ownership rule
/// exists to prevent.
pub fn live_sectors(roots: &Query<(Entity, &SectorRoot)>) -> BTreeMap<SectorCoord, Entity> {
    let mut live = BTreeMap::new();
    for (entity, root) in roots {
        if live.insert(root.0, entity).is_some() {
            panic!(
                "nova_world: {}",
                SectorFault::DuplicateRoot { coord: root.0 }
            );
        }
    }
    live
}

/// Follow the observer: write which cell the [`WorldObserver`] stands in, and
/// refuse a config the rest of the frame cannot afford.
///
/// The first stage a newly armed [`WorldConfig`] reaches, which is why the
/// validation is HERE as well as inside the generator.
/// [`crate::generate_sector`] refuses on a WORKER, one job too late for the
/// dials whose cost the main thread pays first: [`desired_sectors`] enumerates
/// the whole window three times a frame, starting in the very next stage. Only
/// a changed config is re-read, so an armed world costs one generator check
/// per swap and nothing per frame.
///
/// # Panics
///
/// [`SectorFault::AbsentObserver`] unless there is exactly one
/// [`WorldObserver`]. Streaming around a guessed centre would move the world
/// without saying so. And on whatever [`WorldConfig::validate`] refuses, on
/// the frame the config is armed or replaced.
///
/// When a caller wrote [`WorldConfig`] after
/// [`crate::NovaWorldSystems::Cleanup`] had already gone, so the world the old
/// config built was never retired.
#[expect(
    private_interfaces,
    reason = "ClearedConfig is the crate's own record of what Cleanup saw, deliberately not a \
              caller-readable epoch"
)]
pub fn track_current_sector<G: SectorGenerator>(
    mut commands: Commands,
    config: Res<WorldConfig<G>>,
    cleared: Res<ClearedConfig>,
    observer: Query<&GlobalTransform, With<WorldObserver>>,
    current: Option<ResMut<CurrentSector>>,
) {
    assert_world_was_cleared(&config, &cleared);
    if config.is_changed() {
        if let Err(fault) = config.validate() {
            panic!("nova_world: {fault}");
        }
    }
    let Ok(transform) = observer.single() else {
        panic!("nova_world: {}", SectorFault::AbsentObserver);
    };
    // Engine boundary: a bevy transform counts world units, a cell is measured
    // in meters.
    let position = Meters3::from_engine(transform.translation());
    let coord = SectorCoord::containing(position, config.sector_edge);
    match current {
        Some(mut current) => {
            if current.0 != coord {
                debug!("nova_world: the observer crossed into {coord}");
                current.0 = coord;
            }
        }
        None => {
            commands.insert_resource(CurrentSector(coord));
        }
    }
}

/// One cell's preparation, running on the entity that owns it.
///
/// An entity rather than a resource entry so a job is a thing the world can be
/// asked about, and so dropping it is how it is cancelled: `Task`'s own `Drop`
/// cancels the work, and despawning the entity is what [`retire_sectors`] and
/// [`clear_sector_work`] both do.
#[derive(Component)]
pub struct SectorJob {
    /// The cell being prepared.
    pub coord: SectorCoord,
    /// The running preparation. Private: a job is started one way, through
    /// [`SectorJob::start`], so a task can never disagree with the coordinate
    /// beside it.
    task: Task<Result<PreparedSector, SectorFault>>,
}

impl SectorJob {
    /// Start one cell's preparation on `AsyncComputeTaskPool`.
    ///
    /// The config is MOVED into the task rather than read from the resource
    /// when it finishes: a job answers the question it was asked, and a dial
    /// changed mid-flight must not silently re-aim work already in the air.
    /// The other half of that rule is [`clear_sector_work`], which drops every
    /// job in the air on the frame the config changes - a job that answers the
    /// old question must not be ACCEPTED under the new one either.
    pub fn start<G: SectorGenerator>(config: WorldConfig<G>, coord: SectorCoord) -> Self {
        let task = AsyncComputeTaskPool::get().spawn(async move { prepare_sector(config, coord) });
        Self { coord, task }
    }
}

/// Sectors that finished preparing and are waiting for a frame to be spawned
/// in.
///
/// A `BTreeMap`, so the set a frame picks its one materialization out of is
/// ordered before it is ranked and which worker finished first cannot reach
/// the decision.
#[derive(Resource, Default, Debug)]
pub struct ReadySectors(pub BTreeMap<SectorCoord, PreparedSector>);

/// The [`WorldConfig`] version [`clear_sector_work`] last cleared the world
/// for.
///
/// Crate-private and not a generation counter a caller can read: it exists
/// only so a stage can tell "the config Cleanup acted on" from "a config that
/// arrived after Cleanup had already gone".
#[derive(Resource, Default, Debug)]
pub(crate) struct ClearedConfig(Tick);

impl ClearedConfig {
    /// Clamp the recorded tick with bevy's periodic tick sweep.
    ///
    /// A `Tick` kept in a plain field is invisible to
    /// `World::check_change_ticks`, which clamps every tick it DOES know about
    /// once the world has run about half of `u32::MAX` systems. On a long
    /// session the config's own change tick would be clamped and this copy
    /// would not, and the two would stop comparing equal - a refusal hours
    /// into a run with nothing wrong. Clamping both against the same present
    /// tick keeps equal ticks equal.
    pub(crate) fn check_tick(&mut self, check: CheckChangeTicks) {
        self.0.check_tick(check);
    }
}

/// Refuse a frame whose [`WorldConfig`] arrived after `Cleanup` had gone.
///
/// A run condition is evaluated BEFORE its set, so `Cleanup` decides whether
/// to retire the old world at the top of the frame. A writer that lands after
/// that decision leaves the old roots, jobs and prepared payloads on hand
/// while the new config is already in force, and every stage below matches
/// work by COORDINATE alone - a job a worker computed from the old config
/// would be collected and spawned into the new world, and a surviving old root
/// would suppress re-requesting its cell. The next frame does clean up, which
/// is what makes this a silent one-frame mixed world rather than a crash.
///
/// So it is a refusal, and the contract is one line: write [`WorldConfig`]
/// ahead of [`crate::NovaWorldSystems::Cleanup`]. Anything in `PreUpdate` or
/// in a state-transition schedule is already ahead of it; an `Update` writer
/// must say so with `.before(NovaWorldSystems::Cleanup)`.
///
/// # Panics
///
/// When the config changed after `Cleanup` ran this frame.
pub(crate) fn assert_world_was_cleared<G: SectorGenerator>(
    config: &Res<WorldConfig<G>>,
    cleared: &ClearedConfig,
) {
    assert!(
        config.last_changed() == cleared.0,
        "nova_world: the WorldConfig changed after NovaWorldSystems::Cleanup ran, so the world \
         the previous one built was never retired and this frame would stream two worlds at \
         once; order every WorldConfig writer .before(NovaWorldSystems::Cleanup)"
    );
}

/// What the job lifetime has done since the app started.
///
/// Session-lifetime totals, not a live gauge: they are how a reader can tell
/// that a live sector was REQUESTED and PREPARED first rather than conjured
/// inside the frame that needed it. Never reset, including across an unload,
/// because the evidence is the point.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectorJobStats {
    /// Jobs started.
    pub requested: usize,
    /// Jobs that came back with a result.
    pub completed: usize,
    /// Prepared sectors spawned into the world.
    pub materialized: usize,
    /// Jobs and prepared sectors dropped without being spawned: cancelled,
    /// finished after nobody wanted them, or swept by an unload.
    pub discarded: usize,
    /// The most jobs in flight at once, sampled before each poll. Above one it
    /// is the direct evidence that preparation outlives the frame that asked
    /// for it.
    pub peak_pending: usize,
}

/// How many preparations the machine can actually be running. The `max(1)`
/// covers a single-threaded pool, which is what wasm has.
pub(crate) fn job_limit() -> usize {
    AsyncComputeTaskPool::get().thread_num().max(1)
}

/// Rank a cell by how far it is from the observer's cell, the coordinate
/// itself breaking ties.
///
/// `i128` because the difference of two `i32` cell indices squared is outside
/// `i32` and the world grid is unbounded.
fn nearest_first(centre: SectorCoord, coord: SectorCoord) -> (i128, SectorCoord) {
    let x = i128::from(coord.x) - i128::from(centre.x);
    let y = i128::from(coord.y) - i128::from(centre.y);
    let z = i128::from(coord.z) - i128::from(centre.z);
    (x * x + y * y + z * z, coord)
}

/// Start jobs for the desired cells that are not already live, running or
/// prepared - NEAREST FIRST, and never more at once than the task pool has
/// threads.
///
/// A window this size asks for far more work than the cap allows, and the
/// excess is simply NOT REQUESTED - not queued, not deferred, no job entity.
/// Nothing carries it, so a cell that stopped being desired while the window
/// filled was never work anybody has to cancel, and the frame a slot opens
/// asks for whichever cell is nearest BY THEN rather than whichever one was
/// nearest when the observer was somewhere else.
///
/// # Panics
///
/// Through [`live_sectors`] on a duplicate root.
///
/// When a caller wrote [`WorldConfig`] after
/// [`crate::NovaWorldSystems::Cleanup`] had already gone, so the world the old
/// config built was never retired.
#[expect(
    private_interfaces,
    reason = "ClearedConfig is the crate's own record of what Cleanup saw, deliberately not a \
              caller-readable epoch"
)]
pub fn request_sectors<G: SectorGenerator>(
    mut commands: Commands,
    config: Res<WorldConfig<G>>,
    cleared: Res<ClearedConfig>,
    current: Res<CurrentSector>,
    roots: Query<(Entity, &SectorRoot)>,
    jobs: Query<&SectorJob>,
    ready: Res<ReadySectors>,
    mut stats: ResMut<SectorJobStats>,
) {
    assert_world_was_cleared(&config, &cleared);
    let running: BTreeSet<SectorCoord> = jobs.iter().map(|job| job.coord).collect();
    let openings = job_limit().saturating_sub(running.len());
    if openings == 0 {
        return;
    }

    let live = live_sectors(&roots);
    let centre = current.0;
    let mut missing: Vec<SectorCoord> = desired_sectors(centre, config.active_radius)
        .into_iter()
        .filter(|coord| {
            !live.contains_key(coord) && !running.contains(coord) && !ready.0.contains_key(coord)
        })
        .collect();
    missing.sort_by_key(|coord| nearest_first(centre, *coord));

    for coord in missing.into_iter().take(openings) {
        debug!("nova_world: requesting {coord}");
        commands.spawn((
            Name::new(format!("Sector Job {coord}")),
            SectorJob::start(config.clone(), coord),
        ));
        stats.requested += 1;
    }
}

/// Take in what the workers finished, WITHOUT waiting on anything.
///
/// A completion for a cell that is no longer desired is dropped here rather
/// than kept: the observer moved while the job was in the air, and spawning it
/// now would put a sector behind the player that the next frame has to retire.
///
/// # Panics
///
/// On any [`SectorFault`] a worker returns. The fault crosses back to the main
/// thread as a value and fails HERE, where it can name the cell, rather than
/// poisoning a pool thread.
///
/// When a caller wrote [`WorldConfig`] after
/// [`crate::NovaWorldSystems::Cleanup`] had already gone, so the world the old
/// config built was never retired.
#[expect(
    private_interfaces,
    reason = "ClearedConfig is the crate's own record of what Cleanup saw, deliberately not a \
              caller-readable epoch"
)]
pub fn collect_sector_jobs<G: SectorGenerator>(
    mut commands: Commands,
    config: Res<WorldConfig<G>>,
    cleared: Res<ClearedConfig>,
    current: Res<CurrentSector>,
    mut jobs: Query<(Entity, &mut SectorJob)>,
    mut ready: ResMut<ReadySectors>,
    mut stats: ResMut<SectorJobStats>,
) {
    assert_world_was_cleared(&config, &cleared);
    stats.peak_pending = stats.peak_pending.max(jobs.iter().len());
    let desired = desired_sectors(current.0, config.active_radius);

    for (entity, mut job) in &mut jobs {
        let Some(result) = block_on(poll_once(&mut job.task)) else {
            continue;
        };
        let coord = job.coord;
        commands.entity(entity).despawn();
        stats.completed += 1;

        let prepared = result.unwrap_or_else(|fault| panic!("nova_world: {fault}"));
        if !desired.contains(&coord) {
            debug!("nova_world: dropping the finished job for {coord}, it is no longer desired");
            stats.discarded += 1;
            continue;
        }
        // A `BTreeMap` insert, so the order the world hands jobs back in
        // cannot reach the decision the next system takes.
        ready.0.insert(coord, prepared);
    }
}

/// Spawn AT MOST ONE prepared sector, NEAREST desired cell first.
///
/// One a frame is the whole frame policy: the expensive half already happened
/// on a worker, and what is left is a command batch per body that the main
/// thread has to own. Nearest first - the same order [`request_sectors`] asks
/// in - rather than first-prepared-first, so the world fills out from where
/// the observer stands instead of from the low corner of the window.
///
/// # Panics
///
/// Through [`live_sectors`] on a duplicate root, and through
/// [`materialize_sector`] on a ship the catalog does not hold.
///
/// When a caller wrote [`WorldConfig`] after
/// [`crate::NovaWorldSystems::Cleanup`] had already gone, so the world the old
/// config built was never retired.
#[expect(
    private_interfaces,
    reason = "ClearedConfig is the crate's own record of what Cleanup saw, deliberately not a \
              caller-readable epoch"
)]
pub fn materialize_ready_sector<G: SectorGenerator>(
    mut commands: Commands,
    config: Res<WorldConfig<G>>,
    cleared: Res<ClearedConfig>,
    current: Res<CurrentSector>,
    roots: Query<(Entity, &SectorRoot)>,
    mut ready: ResMut<ReadySectors>,
    mut stats: ResMut<SectorJobStats>,
    game_assets: Res<GameAssets>,
    designs: Res<GameShipDesigns>,
) {
    assert_world_was_cleared(&config, &cleared);
    let desired = desired_sectors(current.0, config.active_radius);
    let live = live_sectors(&roots);
    let centre = current.0;
    // Skipping a cell that is already live is what keeps the second root that
    // `live_sectors` panics on from ever being spawned. It costs a lookup and
    // it is the only thing standing between a hand-fed prepared sector and a
    // duplicate.
    let next = ready
        .0
        .keys()
        .copied()
        .filter(|coord| desired.contains(coord) && !live.contains_key(coord))
        .min_by_key(|coord| nearest_first(centre, *coord));
    let Some(prepared) = next.and_then(|coord| ready.0.remove(&coord)) else {
        return;
    };

    let texture: AssetRef<Image> = game_assets.asteroid_texture.clone().into();
    materialize_sector(&mut commands, prepared, &texture, &designs);
    stats.materialized += 1;
}

/// Take back everything outside the desired set: live roots, running jobs, and
/// prepared sectors nobody asked for any more.
///
/// # Panics
///
/// Through [`live_sectors`] on a duplicate root.
///
/// When a caller wrote [`WorldConfig`] after
/// [`crate::NovaWorldSystems::Cleanup`] had already gone, so the world the old
/// config built was never retired.
#[expect(
    private_interfaces,
    reason = "ClearedConfig is the crate's own record of what Cleanup saw, deliberately not a \
              caller-readable epoch"
)]
pub fn retire_sectors<G: SectorGenerator>(
    mut commands: Commands,
    config: Res<WorldConfig<G>>,
    cleared: Res<ClearedConfig>,
    current: Res<CurrentSector>,
    roots: Query<(Entity, &SectorRoot)>,
    jobs: Query<(Entity, &SectorJob)>,
    mut ready: ResMut<ReadySectors>,
    mut stats: ResMut<SectorJobStats>,
) {
    assert_world_was_cleared(&config, &cleared);
    let desired = desired_sectors(current.0, config.active_radius);

    for (coord, entity) in &live_sectors(&roots) {
        if !desired.contains(coord) {
            debug!("nova_world: retiring {coord}");
            commands.entity(*entity).despawn();
        }
    }

    for (entity, job) in &jobs {
        if !desired.contains(&job.coord) {
            debug!("nova_world: cancelling the job for {}", job.coord);
            commands.entity(entity).despawn();
            stats.discarded += 1;
        }
    }

    ready.0.retain(|coord, _| {
        let wanted = desired.contains(coord);
        if !wanted {
            debug!("nova_world: dropping the prepared sector {coord}");
            stats.discarded += 1;
        }
        wanted
    });
}

/// Drop the work the session no longer owns, and the world a replaced
/// [`WorldConfig`] no longer describes.
///
/// Sector roots are scenario objects and the scenario sweep takes them when a
/// SESSION ends. A pending [`SectorJob`] and a prepared [`ReadySectors`]
/// payload are NOT, so nothing else can: without this an unloaded session
/// would leave workers running, and the next session would materialize sectors
/// the previous one asked for.
///
/// Runs on every way the work on hand stops belonging to the world that asked
/// for it, which is why the plugin's condition is not just
/// `not(scenario_is_live)`:
///
/// - `LoadScenario` over a LIVE scenario swaps the two inside one observer
///   call, so there is no frame where liveness is false to catch it and the
///   condition reads `CurrentScenario` changing as well.
/// - the [`WorldConfig`] itself is inserted, replaced or removed. A job
///   carries the config it was STARTED with and everything on hand is keyed by
///   coordinate alone, so an old seed, edge or generator value would otherwise
///   materialize into the new world looking like the new world's own cell.
///   That case is the only one that also takes the LIVE ROOTS: they describe a
///   world nobody configured any more, and no other system would ever retire
///   them, because the desired set names the same coordinates either way.
///
/// It runs before the streaming stages, so a world that is replaced and
/// re-armed in one frame cannot spawn the old world's prepared sectors into
/// the new one.
#[expect(
    private_interfaces,
    reason = "ClearedConfig is the crate's own record of what Cleanup saw, deliberately not a \
              caller-readable epoch"
)]
pub fn clear_sector_work<G: SectorGenerator>(
    mut commands: Commands,
    config: Option<Res<WorldConfig<G>>>,
    roots: Query<Entity, With<SectorRoot>>,
    jobs: Query<Entity, With<SectorJob>>,
    mut ready: ResMut<ReadySectors>,
    mut stats: ResMut<SectorJobStats>,
    mut cleared: ResMut<ClearedConfig>,
) {
    // Absent OR new this frame. The two are the same event seen from either
    // side of a swap, and on the arming frame there is nothing live to take.
    let world_replaced = config.as_ref().is_none_or(|config| config.is_changed());
    // Recorded BEFORE the nothing-to-do return below: every stage under
    // `assert_world_was_cleared` reads this to tell a config Cleanup saw from
    // one that arrived behind its back, and a quiet frame is still a frame
    // Cleanup saw.
    if let Some(config) = config {
        cleared.0 = config.last_changed();
    }
    let retiring = if world_replaced {
        roots.iter().len()
    } else {
        0
    };
    if jobs.is_empty() && ready.0.is_empty() && retiring == 0 {
        return;
    }
    debug!(
        "nova_world: the world on hand is gone, dropping {} job(s) and {} prepared sector(s) \
         and retiring {retiring} live sector(s)",
        jobs.iter().len(),
        ready.0.len()
    );
    stats.discarded += jobs.iter().len() + ready.0.len();
    for entity in &jobs {
        commands.entity(entity).despawn();
    }
    ready.0.clear();
    if world_replaced {
        for entity in &roots {
            // try_despawn: a scenario replaced on the same frame the config is
            // has already queued the same root through the scoped sweep, and
            // the probe's clean pass fails a run that warns on the second one.
            commands.entity(entity).try_despawn();
        }
    }
}
