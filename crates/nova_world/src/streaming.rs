//! The job lifetime: which cells the observer wants, how they are prepared
//! off the frame, how one becomes entities, and how it is taken away again.
//!
//! Routine sector traffic never touches `LoadScenario` and never runs a
//! scenario event action. It calls the same object factories the loader calls
//! (`base_scenario_object`, `asteroid_scenario_object_prepared`,
//! `planet_scenario_object_prepared`, `spaceship_scenario_object`,
//! `station_scenario_object`), which is
//! what makes a streamed rock the same rock a scenario spawns - with one
//! deliberate difference: a streamed entity is scenario-SCOPED, so unloading
//! takes it, but never scenario-ADDRESSABLE, because its id came from a
//! coordinate and no author wrote it.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use bevy::{
    ecs::change_detection::{CheckChangeTicks, Tick},
    prelude::*,
    tasks::{block_on, poll_once, AsyncComputeTaskPool, Task},
};
use nova_events::prelude::{GeneratedBodyId, Meters, Meters3, ScenarioAddressableMarker};
use nova_gameplay::prelude::{
    ClusterType, DerelictShipMarker, IntegrityEnvelope, PlayerFleetMarker, TempEntity,
    UnsettledBody,
};
use nova_scenario::prelude::{
    asteroid_scenario_object_prepared, base_scenario_object, planet_scenario_object_prepared,
    resolve_ship_design, spaceship_scenario_object, station_scenario_object, AsteroidKindId,
    BaseScenarioObjectConfig, GameShipDesigns, PendingParkedHull, PendingParkedHulls, PlanetMarker,
    ShipDesignSource, StationMarker,
};
use nova_ship::prelude::GameSections;

use crate::{
    bodies_clear,
    frozen::{
        freeze_body, hold_unsettled, thaw_record, DockPolicy, FrozenBody, FrozenSector,
        FrozenSectors, PersistentBody, SettlingBodies,
    },
    generation::{prepare_cell, PreparedBodies, PreparedSectorAsteroid, PreparedSectorPlanet},
    PreparedSector, SectorCoord, SectorFault, SectorGenerator, SectorShip, SectorShipConditionType,
    SectorStation, WorldClock, WorldConfig,
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

/// The type of every cluster with a body in a live cell, as its
/// generator described it. On the cell's [`SectorRoot`], so the player's
/// chart reads it when the player enters the cell without planning the cell
/// again.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct SectorClusterTypes(pub Vec<ClusterType>);

/// A generated ship held back because the observer's body overlapped its
/// clearance sphere when its cell materialized.
///
/// A child of the cell's [`SectorRoot`], so retiring the cell takes it with
/// the root, and it carries the manifest entry unchanged:
/// [`materialize_pending_bodies`] spawns exactly that ship where the manifest
/// put it once the observer is clear. It has no body of its own.
#[derive(Component, Debug)]
pub struct PendingSectorShip(pub(crate) SectorShip);

impl PendingSectorShip {
    /// The manifest entry it holds.
    pub fn ship(&self) -> &SectorShip {
        &self.0
    }
}

/// A generated station held back the way a [`PendingSectorShip`] is: the
/// observer's body overlapped its clearance sphere when its cell
/// materialized, and [`materialize_pending_bodies`] spawns its entry
/// unchanged once the observer is clear.
#[derive(Component, Debug)]
pub struct PendingSectorStation(pub(crate) SectorStation);

impl PendingSectorStation {
    /// The manifest entry it holds.
    pub fn station(&self) -> &SectorStation {
        &self.0
    }
}

/// Where the [`WorldObserver`]'s body stands and how far it reaches, as a
/// held ship's overlap test reads it.
#[derive(Clone, Copy, Debug)]
pub struct ObserverBody {
    /// The observer's position.
    pub position: Meters3,
    /// How far its body reaches from that position.
    pub reach: Meters,
}

impl ObserverBody {
    /// Read from the observer's transform and its published
    /// [`IntegrityEnvelope`]. An observer that publishes none, such as a
    /// free-fly camera, has no body to overlap and reaches nothing.
    pub fn read(transform: &GlobalTransform, envelope: Option<&IntegrityEnvelope>) -> Self {
        Self {
            position: Meters3::from_engine(transform.translation()),
            reach: envelope.map_or(Meters::ZERO, |envelope| Meters::from_engine(envelope.0)),
        }
    }

    /// Whether the body reaches into the clearance sphere of `clearance`
    /// around `position`.
    fn overlaps(self, position: Meters3, clearance: Meters) -> bool {
        !bodies_clear(self.position, self.reach, position, clearance, Meters::ZERO)
    }
}

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

/// Spawn one prepared sector: the sector root, then its bodies as the worker
/// resolved them against the cell's frozen record - a fresh cell's rocks,
/// planetoids, ships and arrivals in that order, or a visited cell's
/// planetoids and then its record alone. Returns the root.
///
/// The worker decided what the cell spawns; this transcribes it. See
/// [`PreparedBodies`] for the rule.
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
/// into the streamed world too, and `adopt_moving_bodies` reads the marker's
/// absence as "a body this crate freezes". The marker lives in `nova_events`
/// beside `EntityId`, so `nova_ship` enforces the same rule when it resolves
/// an authored well target of an orbit directive or an Orbit helm order - a
/// streamed planetoid carries a `GravityWell` and would otherwise be orbitable
/// by name. A nearest-well target reaches it on purpose: it names no id.
///
/// Every body ALSO wears its typed `GeneratedBodyId` beside the `EntityId`
/// string, here and at thaw. That is the seam a scenario reaches a generated
/// body through - an objective marker bound from an event payload - without
/// the addressable marker: `nova_scenario::generated_entity`.
///
/// A ship whose clearance sphere `observer` overlaps is not spawned: it is
/// held as a [`PendingSectorShip`] child of the root, for
/// [`materialize_pending_ships`] to spawn once the observer is clear.
///
/// # Panics
///
/// On [`SectorFault::InvalidShipDesign`] when a ship's inline design does
/// not resolve whole against `sections`. The spawn's resolver skips a broken
/// section and flies the rest, so every design is resolved strictly here
/// first. The catalog is a main-thread resource, so this is the first place
/// the design CAN be resolved, and it is resolved before the root is spawned.
pub fn materialize_sector(
    commands: &mut Commands,
    prepared: PreparedSector,
    sections: &GameSections,
    observer: ObserverBody,
    clock: &WorldClock,
) -> Entity {
    let PreparedSector {
        coord,
        cluster_types,
        planets,
        bodies,
    } = prepared;
    let (rocks, ships, stations, frozen) = match bodies {
        PreparedBodies::Fresh {
            rocks,
            ships,
            stations,
            arrivals,
        } => (rocks, ships, stations, arrivals),
        PreparedBodies::Thawed(frozen) => (Vec::new(), Vec::new(), Vec::new(), frozen),
    };
    for ship in &ships {
        require_resolved(&ship.id, &ship.spawn.design, sections);
    }
    for station in &stations {
        require_resolved(
            &station.id,
            &ShipDesignSource::Inline(station.design().hull.clone()),
            sections,
        );
        for hull in &station.hulls {
            require_resolved(
                &station.id,
                &ShipDesignSource::Inline(hull.design.clone()),
                sections,
            );
        }
    }

    let objects = planets.len() + rocks.len() + ships.len() + stations.len() + frozen.len();
    let root = commands
        .spawn((
            base_scenario_object(&BaseScenarioObjectConfig {
                id: coord.slug(),
                name: format!("Sector {coord}"),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            }),
            SectorRoot(coord),
            SectorClusterTypes(cluster_types),
        ))
        .id();

    for PreparedSectorAsteroid { body, geometry } in rocks {
        let mut entity = commands.spawn((
            base_scenario_object(&BaseScenarioObjectConfig {
                id: body.id.to_string(),
                name: generated_asteroid_name(&body.config.kind),
                position: body.position,
                rotation: Quat::IDENTITY,
            }),
            body.id.clone(),
            ChildOf(root),
        ));
        let seed = body.seed();
        asteroid_scenario_object_prepared(&mut entity, body.config, seed, geometry);
    }

    for PreparedSectorPlanet { body, surface } in planets {
        let mut entity = commands.spawn((
            base_scenario_object(&BaseScenarioObjectConfig {
                id: body.id.to_string(),
                name: body.id.to_string(),
                position: body.position,
                rotation: Quat::IDENTITY,
            }),
            body.id.clone(),
            ChildOf(root),
        ));
        planet_scenario_object_prepared(&mut entity, surface);
    }

    for ship in ships {
        if observer.overlaps(ship.position, ship.clearance) {
            debug!(
                "nova_world: holding ship '{}' in {coord}: the observer overlaps its {:.0} m \
                 clearance",
                ship.id,
                ship.clearance.get()
            );
            commands.spawn((PendingSectorShip(ship), ChildOf(root)));
        } else {
            spawn_sector_ship(commands, root, ship);
        }
    }

    for station in stations {
        if observer.overlaps(station.position, station.clearance) {
            debug!(
                "nova_world: holding station '{}' in {coord}: the observer overlaps its {:.0} m \
                 clearance",
                station.id,
                station.clearance.get()
            );
            commands.spawn((PendingSectorStation(station), ChildOf(root)));
        } else {
            spawn_sector_station(commands, root, station);
        }
    }

    if !frozen.is_empty() {
        trace!(
            "nova_world: thawing {} frozen body(ies) into {coord}",
            frozen.len()
        );
        thaw_record(commands, root, frozen, clock);
    }

    trace!("nova_world: materialized {coord} with {objects} object(s)");
    root
}

/// Refuse a body `id` whose inline design does not resolve whole against
/// `sections`: a ship's design, or a station's hull.
///
/// # Panics
///
/// On [`SectorFault::InvalidShipDesign`] with every resolver error.
fn require_resolved(id: &GeneratedBodyId, design: &ShipDesignSource, sections: &GameSections) {
    let (_, errors) = resolve_ship_design(design, &GameShipDesigns::default(), sections);
    if !errors.is_empty() {
        panic!(
            "nova_world: {}",
            SectorFault::InvalidShipDesign {
                id: id.to_string(),
                errors,
            }
        );
    }
}

/// Spawn one generated station under `root`, where and as its manifest
/// entry says: the entry's `spawn` and `hulls` are the whole station, and
/// this adds only the pose, the identity and a living station's market. The
/// hulls for
/// sale park at its ports and the escorts for hire coast beside it
/// ([`PendingParkedHulls`]). A dead station is the same
/// design spawned `derelict`: the factory marks it, so its hull spawns
/// inactive and lootable, and no service is offered on it.
fn spawn_sector_station(commands: &mut Commands, root: Entity, station: SectorStation) {
    let design = station.design().clone();
    let SectorStation {
        id,
        position,
        rotation,
        clearance: _,
        civilization,
        spawn,
        hulls,
        market,
    } = station;
    let name = generated_station_name(&civilization.name(), spawn.derelict);
    let mut entity = commands.spawn((
        base_scenario_object(&BaseScenarioObjectConfig {
            id: id.to_string(),
            name,
            position,
            rotation,
        }),
        id,
        ChildOf(root),
    ));
    station_scenario_object(&mut entity, &design, &spawn);
    if let Some(market) = market {
        entity.insert(market);
    }
    if !hulls.is_empty() {
        entity.insert(PendingParkedHulls(
            hulls.into_iter().map(PendingParkedHull::Offer).collect(),
        ));
    }
}

/// The name a generated station of the civilization named `civilization`
/// carries, living or `dead`.
fn generated_station_name(civilization: &str, dead: bool) -> String {
    if dead {
        format!("{civilization} {DEAD_STATION_SUFFIX}")
    } else {
        format!("{civilization} {LIVING_STATION_SUFFIX}")
    }
}

/// The name a generated asteroid of `kind` carries, as `Rock asteroid`, so
/// the target inset names what the rock is. Its id stays in `EntityId` and
/// `GeneratedBodyId`.
fn generated_asteroid_name(kind: &AsteroidKindId) -> String {
    let words: Vec<String> = kind
        .as_str()
        .split(['_', '-'])
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect();
    format!("{} asteroid", words.join(" "))
}

const LIVING_STATION_SUFFIX: &str = "station";
const DEAD_STATION_SUFFIX: &str = "dead station";

/// The name a generated station named `name` carries once it is dead: `None`
/// when it already carries a dead station's name.
///
/// # Panics
///
/// On a name neither [`generated_station_name`] nor the station buy
/// ([`OWNED_STATION_NAME`](nova_ship::prelude::OWNED_STATION_NAME)) wrote: a generated station is named only there,
/// so another name is a station this module did not spawn.
fn dead_station_name(name: &str) -> Option<String> {
    if name.ends_with(&format!(" {DEAD_STATION_SUFFIX}")) {
        return None;
    }
    let Some(civilization) = name.strip_suffix(&format!(" {LIVING_STATION_SUFFIX}")) else {
        panic!("nova_world: generated station '{name}' does not carry a generated station name");
    };
    Some(generated_station_name(civilization, true))
}

/// Rename a generated station that has just died, as one generated dead is
/// named, so the chart and the target readout name what it is now. A
/// station spawned or thawed dead already carries that name. An authored
/// scenario station has no [`GeneratedBodyId`] and keeps its author's name.
pub(crate) fn rename_dead_generated_station(
    add: On<Add, DerelictShipMarker>,
    mut q_station: Query<&mut Name, (With<StationMarker>, With<GeneratedBodyId>)>,
) {
    let Ok(mut name) = q_station.get_mut(add.entity) else {
        return;
    };
    if let Some(dead) = dead_station_name(name.as_str()) {
        name.set(dead);
    }
}

/// Spawn one generated ship under `root`, where and as its manifest entry
/// says: the entry's `spawn` config is the whole ship, and this adds only
/// the pose, the identity, an intact ship's market and, for a derelict,
/// [`DerelictShipMarker`] in the same bundle as the root, so the section
/// spawn sees it and every section but hull and docking port spawns
/// inactive.
fn spawn_sector_ship(commands: &mut Commands, root: Entity, ship: SectorShip) {
    let SectorShip {
        id,
        position,
        rotation,
        clearance: _,
        condition,
        civilization,
        role,
        spawn,
        market,
    } = ship;
    let name = match condition {
        SectorShipConditionType::Intact => format!("{} {}", civilization.name(), role.label()),
        SectorShipConditionType::Derelict => {
            format!("{} derelict, former {}", civilization.name(), role.label())
        }
    };
    let ship = (
        base_scenario_object(&BaseScenarioObjectConfig {
            id: id.to_string(),
            name,
            position,
            rotation,
        }),
        id,
        spaceship_scenario_object(spawn),
        ChildOf(root),
    );
    let mut entity = match condition {
        SectorShipConditionType::Intact => commands.spawn(ship),
        SectorShipConditionType::Derelict => commands.spawn((ship, DerelictShipMarker)),
    };
    if let Some(market) = market {
        entity.insert(market);
    }
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
pub fn live_sectors<'a>(
    roots: impl IntoIterator<Item = (Entity, &'a SectorRoot)>,
) -> BTreeMap<SectorCoord, Entity> {
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
    assert_world_was_cleared(config.last_changed(), &cleared);
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
    /// `record` is the cell's frozen record, if it holds one. The worker
    /// resolves the cell against it (see [`PreparedBodies`]) and the payload
    /// carries the record's bodies, so the materialization only drops the
    /// ledger's copy. A record changes only while its cell is off-window, and
    /// [`retire_sectors`] cancels every job for an off-window cell, so the
    /// record a job was started from is the one its cell comes back from.
    ///
    /// The config is MOVED into the task rather than read from the resource
    /// when it finishes: a job answers the question it was asked, and a dial
    /// changed mid-flight must not silently re-aim work already in the air.
    /// The other half of that rule is [`clear_sector_work`], which drops every
    /// job in the air on the frame the config changes - a job that answers the
    /// old question must not be ACCEPTED under the new one either.
    pub fn start<G: SectorGenerator>(
        config: WorldConfig<G>,
        coord: SectorCoord,
        record: Option<Arc<FrozenSector>>,
    ) -> Self {
        let task =
            AsyncComputeTaskPool::get().spawn(async move { prepare_cell(config, coord, record) });
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
pub(crate) fn assert_world_was_cleared(config_changed: Tick, cleared: &ClearedConfig) {
    assert!(
        config_changed == cleared.0,
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
    frozen: Res<FrozenSectors>,
    mut stats: ResMut<SectorJobStats>,
) {
    assert_world_was_cleared(config.last_changed(), &cleared);
    let running: BTreeSet<SectorCoord> = jobs.iter().map(|job| job.coord).collect();
    let openings = job_limit().saturating_sub(running.len());
    if openings == 0 {
        return;
    }

    let live = live_sectors(roots);
    let centre = current.0;
    let mut missing: Vec<SectorCoord> = desired_sectors(centre, config.active_radius)
        .into_iter()
        .filter(|coord| {
            !live.contains_key(coord) && !running.contains(coord) && !ready.0.contains_key(coord)
        })
        .collect();
    missing.sort_by_key(|coord| nearest_first(centre, *coord));

    for coord in missing.into_iter().take(openings) {
        trace!("nova_world: requesting {coord}");
        commands.spawn((
            Name::new(format!("Sector Job {coord}")),
            SectorJob::start(config.clone(), coord, frozen.shared(coord)),
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
    assert_world_was_cleared(config.last_changed(), &cleared);
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
            trace!("nova_world: dropping the finished job for {coord}, it is no longer desired");
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
/// The cell's [`FrozenSectors`] record, if any, is consumed here: a visited
/// cell comes back as it was frozen, and arrivals join a freshly generated one.
///
/// # Panics
///
/// Through [`live_sectors`] on a duplicate root, through
/// [`materialize_sector`] on a ship design the loaded sections do not resolve,
/// and on [`SectorFault::AbsentObserver`] unless there is exactly one
/// [`WorldObserver`].
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
    mut frozen: ResMut<FrozenSectors>,
    mut stats: ResMut<SectorJobStats>,
    sections: Res<GameSections>,
    clock: Res<WorldClock>,
    observer: Query<(&GlobalTransform, Option<&IntegrityEnvelope>), With<WorldObserver>>,
) {
    assert_world_was_cleared(config.last_changed(), &cleared);
    let desired = desired_sectors(current.0, config.active_radius);
    let live = live_sectors(roots);
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

    let coord = prepared.coord();
    let Ok((transform, envelope)) = observer.single() else {
        panic!("nova_world: {}", SectorFault::AbsentObserver);
    };
    let observer = ObserverBody::read(transform, envelope);
    // The job carried the record's bodies; the ledger's copy is spent.
    frozen.consume(coord);
    materialize_sector(&mut commands, prepared, &sections, observer, &clock);
    stats.materialized += 1;

    if desired
        .iter()
        .all(|wanted| *wanted == coord || live.contains_key(wanted))
    {
        debug!(
            "nova_world: all {} desired sector(s) around {centre} are live; app-lifetime job \
             totals: {} requested, {} completed, {} materialized, {} discarded, peak {} pending",
            desired.len(),
            stats.requested,
            stats.completed,
            stats.materialized,
            stats.discarded,
            stats.peak_pending
        );
    }
}

/// Spawn every held ship and station whose clearance sphere the observer
/// has left, each on its own, where its manifest entry puts it, under the
/// root that holds it.
///
/// A held body the observer still overlaps waits another frame. Nothing
/// moves or drops it: the manifest decided the body, and the observer only
/// decides when it appears.
///
/// # Panics
///
/// On [`SectorFault::AbsentObserver`] unless there is exactly one
/// [`WorldObserver`].
pub fn materialize_pending_bodies(
    mut commands: Commands,
    ships: Query<(Entity, &PendingSectorShip, &ChildOf)>,
    stations: Query<(Entity, &PendingSectorStation, &ChildOf)>,
    observer: Query<(&GlobalTransform, Option<&IntegrityEnvelope>), With<WorldObserver>>,
) {
    if ships.is_empty() && stations.is_empty() {
        return;
    }
    let Ok((transform, envelope)) = observer.single() else {
        panic!("nova_world: {}", SectorFault::AbsentObserver);
    };
    let observer = ObserverBody::read(transform, envelope);
    for (entity, held, child_of) in &ships {
        if observer.overlaps(held.0.position, held.0.clearance) {
            continue;
        }
        debug!(
            "nova_world: the observer cleared held ship '{}'; spawning it",
            held.0.id
        );
        commands.entity(entity).despawn();
        spawn_sector_ship(&mut commands, child_of.parent(), held.0.clone());
    }
    for (entity, held, child_of) in &stations {
        if observer.overlaps(held.0.position, held.0.clearance) {
            continue;
        }
        debug!(
            "nova_world: the observer cleared held station '{}'; spawning it",
            held.0.id
        );
        commands.entity(entity).despawn();
        spawn_sector_station(&mut commands, child_of.parent(), held.0.clone());
    }
}

/// Take back everything outside the desired set: live roots, running jobs, and
/// prepared sectors nobody asked for any more.
///
/// A retiring root's persistent bodies freeze into a
/// [`FrozenSector::Visited`] record and despawn, so the cell comes back as it
/// was left. Its planetoids are not frozen - the description spawns them
/// again unchanged - and its transients despawn with it. A body whose owner
/// reports it unsettled stays live under the root, and the root stays live
/// until the last one freezes; each later frame freezes the ones that settled
/// into the same record. A live root whose cell holds a record keeps retiring
/// even when the cell is desired again: the record already holds part of the
/// cell, and the cell comes back whole from it once the root is gone.
///
/// # Panics
///
/// Through [`live_sectors`] on a duplicate root, on a root child that is
/// neither a planetoid, a transient nor a persistent body a sector can freeze,
/// or a docked ship: retiring it would lose it. And on a body that stays
/// unsettled for too many advancing frames.
///
/// When a caller wrote [`WorldConfig`] after
/// [`crate::NovaWorldSystems::Cleanup`] had already gone, so the world the old
/// config built was never retired.
pub fn retire_sectors<G: SectorGenerator>(world: &mut World) {
    let config = world.resource_ref::<WorldConfig<G>>();
    assert_world_was_cleared(config.last_changed(), world.resource::<ClearedConfig>());
    let radius = config.active_radius;
    let centre = world.resource::<CurrentSector>().0;
    let desired = desired_sectors(centre, radius);

    let live = live_sectors(world.query::<(Entity, &SectorRoot)>().iter(world));
    let mut retired = 0_usize;
    let mut held = 0_usize;
    for (coord, root) in live {
        // A live cell's record was consumed when it materialized, so a live
        // root whose cell holds one is part-way through retiring.
        let retiring = world.resource::<FrozenSectors>().get(coord).is_some();
        if desired.contains(&coord) && !retiring {
            continue;
        }
        let (frozen, unsettled) = freeze_sector_bodies(world, coord, root, DockPolicy::Refuse);
        trace!(
            "nova_world: retiring {coord}, freezing {} body(ies)",
            frozen.len()
        );
        let mut bodies = Vec::with_capacity(frozen.len());
        for (entity, body) in frozen {
            world.entity_mut(entity).despawn();
            bodies.push(body);
        }
        world.resource_mut::<FrozenSectors>().visit(coord, bodies);
        if unsettled.is_empty() {
            world.entity_mut(root).despawn();
            retired += 1;
            continue;
        }
        held += 1;
        for (entity, reason) in unsettled {
            debug!("nova_world: holding {entity} live in retiring {coord}: {reason}");
            hold_unsettled(world, entity, coord, reason);
        }
    }
    world.resource_mut::<SettlingBodies>().sweep();

    let unwanted: Vec<(Entity, SectorCoord)> = world
        .query::<(Entity, &SectorJob)>()
        .iter(world)
        .filter(|(_, job)| !desired.contains(&job.coord))
        .map(|(entity, job)| (entity, job.coord))
        .collect();
    let cancelled = unwanted.len();
    for (entity, coord) in unwanted {
        trace!("nova_world: cancelling the job for {coord}");
        world.entity_mut(entity).despawn();
    }

    let mut dropped = 0_usize;
    world.resource_mut::<ReadySectors>().0.retain(|coord, _| {
        let wanted = desired.contains(coord);
        if !wanted {
            trace!("nova_world: dropping the prepared sector {coord}");
            dropped += 1;
        }
        wanted
    });
    world.resource_mut::<SectorJobStats>().discarded += cancelled + dropped;

    if retired + held + cancelled + dropped > 0 {
        debug!(
            "nova_world: around {centre}, retired {retired} sector(s), held {held} with \
             unsettled bodies, cancelled {cancelled} job(s) and dropped {dropped} prepared \
             sector(s)"
        );
    }
}

/// Freeze every settled persistent body a root holds, in child order.
/// Returns the frozen bodies with their entities, and each body its owner
/// reports unsettled. A retiring root refuses a docked ship; a save keeps one
/// undocked.
pub(crate) fn freeze_sector_bodies(
    world: &World,
    coord: SectorCoord,
    root: Entity,
    docked: DockPolicy,
) -> (Vec<(Entity, FrozenBody)>, Vec<(Entity, UnsettledBody)>) {
    let mut frozen = Vec::new();
    let mut unsettled = Vec::new();
    let Some(children) = world.get::<Children>(root) else {
        return (frozen, unsettled);
    };
    let clock = world.resource::<WorldClock>();
    for &child in children {
        let entity = world.entity(child);
        // The description spawns a planetoid again unchanged, and a transient
        // is never kept.
        if entity.contains::<PlanetMarker>() || entity.contains::<TempEntity>() {
            continue;
        }
        match freeze_body(world, child, docked, clock) {
            Ok(body) => {
                trace!("nova_world: freezing {child} with {coord}");
                frozen.push((child, body));
            }
            Err(reason) => unsettled.push((child, reason)),
        }
    }
    (frozen, unsettled)
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
///   That case is the only one that also takes the LIVE ROOTS and every
///   persistent body standing top-level outside them, waiting for a cell to
///   materialize: they describe a world nobody configured any more, and no
///   other system would ever retire them, because the desired set names the
///   same coordinates either way. Adoption would give a waiting body to the
///   new world's cell. The player's fleet ([`PlayerFleetMarker`]) stays: it
///   belongs to the player's record, and a Load thaws it with the player
///   before the world arms.
///
/// Every [`FrozenSectors`] record, the [`WorldClock`] and every
/// unsettled-body wait go on each of those frames too: a record is the state
/// of the world that froze it, and the session is the most it outlives.
///
/// It runs before the streaming stages, so a world that is replaced and
/// re-armed in one frame cannot spawn the old world's prepared sectors into
/// the new one.
#[expect(
    private_interfaces,
    reason = "ClearedConfig is the crate's own record of what Cleanup saw, deliberately not a \
              caller-readable epoch; SettlingBodies is the crate's own count of unsettled waits"
)]
pub fn clear_sector_work<G: SectorGenerator>(
    mut commands: Commands,
    config: Option<Res<WorldConfig<G>>>,
    roots: Query<Entity, With<SectorRoot>>,
    loose: Query<
        Entity,
        (
            PersistentBody,
            Without<ChildOf>,
            Without<WorldObserver>,
            Without<PlayerFleetMarker>,
            Without<ScenarioAddressableMarker>,
        ),
    >,
    jobs: Query<Entity, With<SectorJob>>,
    mut ready: ResMut<ReadySectors>,
    mut frozen: ResMut<FrozenSectors>,
    mut clock: ResMut<WorldClock>,
    mut settling: ResMut<SettlingBodies>,
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
    let (retiring, abandoned) = if world_replaced {
        (roots.iter().len(), loose.iter().len())
    } else {
        (0, 0)
    };
    // Every frame this runs on, the session or the world that froze a record
    // is gone: an unload, a replaced scenario, a Retry, a swapped config. The
    // clock goes with the records; a Load restores both after this.
    let forgotten = frozen.clear();
    if *clock != WorldClock::default() {
        *clock = WorldClock::default();
    }
    let waits = settling.clear();
    if jobs.is_empty()
        && ready.0.is_empty()
        && retiring == 0
        && abandoned == 0
        && forgotten == 0
        && waits == 0
    {
        return;
    }
    debug!(
        "nova_world: the world on hand is gone, dropping {} job(s), {} prepared sector(s), \
         {forgotten} frozen sector(s) and {waits} unsettled wait(s), retiring {retiring} live \
         sector(s) and despawning {abandoned} top-level body(ies)",
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
        for entity in &loose {
            commands.entity(entity).try_despawn();
        }
    }
}

#[cfg(test)]
mod generated_name_tests {
    use nova_ship::prelude::OWNED_STATION_NAME;

    use super::*;

    /// A generated living station's name turns into the dead one's, a bought
    /// station's too, a dead name stays, and a name the generator never
    /// writes is refused.
    #[test]
    fn a_dead_generated_station_takes_the_dead_station_name() {
        let living = generated_station_name("Vessa", false);
        assert_eq!(living, "Vessa station");
        assert_eq!(
            dead_station_name(&living).as_deref(),
            Some("Vessa dead station")
        );
        assert_eq!(
            dead_station_name(OWNED_STATION_NAME).as_deref(),
            Some("Owned dead station")
        );
        assert_eq!(
            dead_station_name(&generated_station_name("Vessa", true)),
            None
        );
        assert!(std::panic::catch_unwind(|| dead_station_name("Outpost 9")).is_err());
    }

    /// A generated asteroid is named by its kind, one capitalised word per
    /// part of the id. `deep_ice` is no shipped kind and does not load; it
    /// shows how a multi-word id reads.
    #[test]
    fn a_generated_asteroid_is_named_by_its_kind() {
        for (kind, name) in [
            ("rock", "Rock asteroid"),
            ("metal", "Metal asteroid"),
            ("ice", "Ice asteroid"),
            ("carbon", "Carbon asteroid"),
            ("deep_ice", "Deep Ice asteroid"),
        ] {
            assert_eq!(generated_asteroid_name(&AsteroidKindId::from(kind)), name);
        }
    }
}
