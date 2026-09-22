//! The job lifetime: which cells the observer wants, how they are prepared
//! off the frame, how one becomes entities, and how it is taken away again.
//!
//! Routine sector traffic never touches `LoadScenario` and never runs a
//! scenario event action. It calls the same object factories the loader calls
//! (`base_scenario_object`, `asteroid_scenario_object_prepared`,
//! `planet_scenario_object_prepared`, `spaceship_scenario_object`), which is
//! what makes a streamed rock the same rock a scenario spawns.

use std::collections::{BTreeMap, BTreeSet};

use bevy::{
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
    prepare_sector, FeatureLayer, FeatureSphere, PreparedSector, SectorCoord, SectorFault,
    WorldConfig,
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

/// The feature spheres that reach this sector, on its root. DIAGNOSTIC: the
/// generator already used them, and this is how a caller draws the field and
/// how a range reads one sphere from two cells.
#[derive(Component, Clone, Debug)]
pub struct SectorFeatureSpheres(pub Vec<FeatureSphere>);

/// The combined per-layer influence at this sector's centre, on its root.
/// DIAGNOSTIC, indexed by [`FeatureLayer::index`].
#[derive(Component, Clone, Copy, Debug)]
pub struct SectorStrengths(pub [f32; FeatureLayer::COUNT]);

/// Which cell the [`WorldObserver`] is in. Written by
/// [`track_current_sector`], and the centre every desired-set decision in the
/// job lifetime is taken against.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CurrentSector(pub SectorCoord);

/// The cells the observer wants live: the cube of side `2 * radius + 1`
/// centred on `centre`.
pub fn desired_sectors(centre: SectorCoord, radius: i32) -> BTreeSet<SectorCoord> {
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
/// description names - rocks, then planetoids, then moored hulls. Returns the
/// root.
///
/// The root is an OWNERSHIP node and not a pose - it stays at the world origin
/// and its children carry world positions. `base_scenario_object` seeds a
/// `GlobalTransform` from the entity's own `Transform`, and avian's
/// `transform_to_position` reads that seed in `FixedPostUpdate`, a schedule
/// ahead of bevy's `TransformSystems::Propagate`. A root posed at the sector
/// centre would therefore hand every child body a global position equal to its
/// LOCAL offset, and a rigid body's `Position` is authoritative from then on.
///
/// # Panics
///
/// When `prepared` carries a different number of rocks than asteroids or a
/// different number of surfaces than planetoids - zipping a short list would
/// silently spawn a sector missing its tail - and on
/// [`SectorFault::UnknownShip`] when a moored hull names a design the loaded
/// catalog does not hold. The catalog is a main-thread resource, so this is
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
        asteroid_geometry,
        planet_surfaces,
    } = prepared;
    let coord = description.coord;
    assert_eq!(
        description.asteroids.len(),
        asteroid_geometry.len(),
        "nova_world: {coord} was prepared with {} rocks for {} asteroids",
        asteroid_geometry.len(),
        description.asteroids.len()
    );
    assert_eq!(
        description.planets.len(),
        planet_surfaces.len(),
        "nova_world: {coord} was prepared with {} surfaces for {} planetoids",
        planet_surfaces.len(),
        description.planets.len()
    );
    for hull in &description.anchorages {
        assert!(
            designs.get_design(&hull.design).is_some(),
            "nova_world: {}",
            SectorFault::UnknownShip {
                id: hull.id.clone(),
                design: hull.design.clone(),
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
            SectorStrengths(description.strengths),
            SectorFeatureSpheres(description.features),
        ))
        .id();

    for (body, rock) in description.asteroids.into_iter().zip(asteroid_geometry) {
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
                mass: None,
                invulnerable: true,
                lock_signature: None,
                seed: Some(body.seed),
            },
            body.seed,
            rock,
        );
    }

    for (planet, surface) in description.planets.into_iter().zip(planet_surfaces) {
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

    for hull in description.anchorages {
        commands.spawn((
            base_scenario_object(&BaseScenarioObjectConfig {
                id: hull.id.clone(),
                name: hull.id.clone(),
                position: hull.position,
                rotation: Quat::from_rotation_y(hull.yaw),
            }),
            spaceship_scenario_object(SpaceshipConfig {
                design: ShipDesignSource::Prototype {
                    id: hull.design.clone(),
                    section_patches: BTreeMap::new(),
                },
                // Nobody aboard and nobody's side: a moored hull is scenery
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

/// Follow the observer: write which cell the [`WorldObserver`] stands in.
///
/// # Panics
///
/// [`SectorFault::AbsentObserver`] unless there is exactly one
/// [`WorldObserver`]. Streaming around a guessed centre would move the world
/// without saying so.
pub fn track_current_sector(
    mut commands: Commands,
    config: Res<WorldConfig>,
    observer: Query<&GlobalTransform, With<WorldObserver>>,
    current: Option<ResMut<CurrentSector>>,
) {
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
    pub fn start(config: WorldConfig, coord: SectorCoord) -> Self {
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
pub fn request_sectors(
    mut commands: Commands,
    config: Res<WorldConfig>,
    current: Res<CurrentSector>,
    roots: Query<(Entity, &SectorRoot)>,
    jobs: Query<&SectorJob>,
    ready: Res<ReadySectors>,
    mut stats: ResMut<SectorJobStats>,
) {
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
pub fn collect_sector_jobs(
    mut commands: Commands,
    config: Res<WorldConfig>,
    current: Res<CurrentSector>,
    mut jobs: Query<(Entity, &mut SectorJob)>,
    mut ready: ResMut<ReadySectors>,
    mut stats: ResMut<SectorJobStats>,
) {
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
/// [`materialize_sector`] on a moored hull the catalog does not hold.
pub fn materialize_ready_sector(
    mut commands: Commands,
    config: Res<WorldConfig>,
    current: Res<CurrentSector>,
    roots: Query<(Entity, &SectorRoot)>,
    mut ready: ResMut<ReadySectors>,
    mut stats: ResMut<SectorJobStats>,
    game_assets: Res<GameAssets>,
    designs: Res<GameShipDesigns>,
) {
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
pub fn retire_sectors(
    mut commands: Commands,
    config: Res<WorldConfig>,
    current: Res<CurrentSector>,
    roots: Query<(Entity, &SectorRoot)>,
    jobs: Query<(Entity, &SectorJob)>,
    mut ready: ResMut<ReadySectors>,
    mut stats: ResMut<SectorJobStats>,
) {
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

/// Drop the work the session no longer owns.
///
/// Sector roots are scenario objects and the scenario sweep takes them. A
/// pending [`SectorJob`] and a prepared [`ReadySectors`] payload are NOT, so
/// nothing else can: without this an unloaded session would leave workers
/// running, and the next session would materialize sectors the previous one
/// asked for.
///
/// Runs on both ways a session ends, which is why the plugin's condition is
/// not just `not(scenario_is_live)`: `LoadScenario` over a LIVE scenario swaps
/// the two inside one observer call, so there is no frame where liveness is
/// false to catch it. It runs before the streaming stages, so a session that
/// is replaced and re-armed in one frame cannot spawn the old session's
/// prepared sectors into the new one.
pub fn clear_sector_work(
    mut commands: Commands,
    jobs: Query<Entity, With<SectorJob>>,
    mut ready: ResMut<ReadySectors>,
    mut stats: ResMut<SectorJobStats>,
) {
    if jobs.is_empty() && ready.0.is_empty() {
        return;
    }
    debug!(
        "nova_world: the session is gone, dropping {} job(s) and {} prepared sector(s)",
        jobs.iter().len(),
        ready.0.len()
    );
    stats.discarded += jobs.iter().len() + ready.0.len();
    for entity in &jobs {
        commands.entity(entity).despawn();
    }
    ready.0.clear();
}
