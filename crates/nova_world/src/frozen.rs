//! What a cell keeps while it is outside the active window.
//!
//! An off-window cell holds no entities and nothing in it is simulated. Every
//! persistent body that was in it - a rock, a ship, a held ship, a loose
//! canister, a severed wreck, the ore a spent rock still owed - is a typed
//! record in [`FrozenSectors`], and the cell's next materialization spawns
//! those records rather than what the generator would have placed.
//!
//! The one exception is a body whose owner reports it unsettled: it stays
//! live, and so does the root of a retiring cell that holds it, until it
//! settles and freezes. [`SettlingBodies`] bounds that wait.
//!
//! Session memory only. [`crate::clear_sector_work`] drops every record on the
//! frames it clears the world, and nothing is written to disk.
//!
//! The owner crates keep their bodies' private state private: each one exposes
//! a `freeze_*` and a `thaw_*` pair. This module decides WHEN a body freezes
//! and which cell keeps it, never what is inside it.

use std::collections::{BTreeMap, BTreeSet};

use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use nova_events::prelude::{EntityId, Meters, Meters3, ScenarioAddressableMarker};
use nova_gameplay::prelude::{CargoCanister, SpaceshipRootMarker, TempEntity, UnsettledBody};
use nova_scenario::prelude::{
    freeze_asteroid, freeze_ore_drop, freeze_ship, thaw_asteroid, thaw_ore_drop, thaw_ship,
    AsteroidMarker, FrozenAsteroid, FrozenOreDrop, FrozenShip, MinedOreDrop, PreparedAsteroid,
    ScenarioScopedMarker,
};
use nova_ship::prelude::{
    freeze_canister, freeze_wreck_fragment, thaw_canister, thaw_wreck_fragment, DockedShip,
    FrozenCanister, FrozenWreckFragment, ShipWreckFragmentMarker,
};

use crate::{
    desired_sectors, live_sectors,
    streaming::{assert_world_was_cleared, ClearedConfig},
    CurrentSector, PendingSectorShip, SectorCoord, SectorGenerator, SectorRoot, SectorShip,
    WorldConfig, WorldObserver,
};

/// The frozen cells of this session, keyed by cell.
///
/// A cell has at most one record. A [`FrozenSector::Visited`] record is
/// consumed when its cell materializes again; an [`FrozenSector::Arrivals`]
/// record is consumed when its cell first generates.
#[derive(Resource, Default, Debug)]
pub struct FrozenSectors(BTreeMap<SectorCoord, FrozenSector>);

impl FrozenSectors {
    /// The record a cell holds, if it holds one.
    pub fn get(&self, coord: SectorCoord) -> Option<&FrozenSector> {
        self.0.get(&coord)
    }

    /// How many cells hold a record.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no cell holds a record.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Take a cell's record, for the materialization that spawns it.
    pub(crate) fn take(&mut self, coord: SectorCoord) -> Option<FrozenSector> {
        self.0.remove(&coord)
    }

    /// Keep the bodies a retiring cell froze this frame.
    ///
    /// A cell retires over as many frames as its last unsettled body needs.
    /// Its first call creates the [`FrozenSector::Visited`] record, even with
    /// no bodies, and each later call extends it: a live root whose cell holds
    /// a record is still retiring.
    ///
    /// # Panics
    ///
    /// When the cell holds an [`FrozenSector::Arrivals`] record: a live
    /// cell's record was consumed when it materialized and a body bound for a
    /// live cell is adopted rather than frozen, so arrivals there are a lost
    /// body. And on two bodies with one id.
    pub(crate) fn visit(&mut self, coord: SectorCoord, bodies: Vec<FrozenBody>) {
        let record = self
            .0
            .entry(coord)
            .or_insert_with(|| FrozenSector::Visited(Vec::new()));
        let FrozenSector::Visited(kept) = record else {
            panic!(
                "nova_world: {coord} retired while it held {} frozen arrival(s)",
                record.bodies().len()
            );
        };
        kept.extend(bodies);
        require_unique_ids(coord, kept);
    }

    /// Keep a body that moved into an off-window cell nobody holds live, or
    /// whose root is retiring.
    ///
    /// A cell that was generated keeps it beside its other bodies; a cell
    /// that never was keeps it as an arrival, for its first generation to
    /// spawn beside the generated ones.
    ///
    /// # Panics
    ///
    /// When the body's id is already in the cell's record.
    pub(crate) fn arrive(&mut self, coord: SectorCoord, body: FrozenBody) {
        let record = self
            .0
            .entry(coord)
            .or_insert_with(|| FrozenSector::Arrivals(Vec::new()));
        let bodies = match record {
            FrozenSector::Visited(bodies) | FrozenSector::Arrivals(bodies) => bodies,
        };
        bodies.push(body);
        require_unique_ids(coord, bodies);
    }

    /// Drop every record. Returns how many cells held one.
    pub(crate) fn clear(&mut self) -> usize {
        let cells = self.0.len();
        self.0.clear();
        cells
    }
}

/// Refuse a record that names one id twice: thawing it would spawn two
/// bodies a range or an id lookup cannot tell apart.
fn require_unique_ids(coord: SectorCoord, bodies: &[FrozenBody]) {
    let mut seen = BTreeSet::new();
    for id in bodies.iter().filter_map(FrozenBody::id) {
        assert!(
            seen.insert(id),
            "nova_world: {coord} would freeze two bodies with the id '{id}'"
        );
    }
}

/// How many advancing frames a body may stay unsettled while its cell waits
/// to freeze it. Every owner's multi-frame process ends within a few frames;
/// one still running after this is stuck, and the cell would hold it live
/// outside the window for the rest of the session.
const SETTLING_FRAMES_MAX: u32 = 600;

/// Every persistent body that could not freeze on the last frame, keyed by
/// the body and the cell bound to freeze it, with how many consecutive
/// advancing frames it has waited.
///
/// A paused frame keeps a wait but does not count: nothing settles while
/// virtual time stands still. A wait not renewed on a frame is dropped, so a
/// body that froze, despawned or stopped being bound for a frozen cell starts
/// from zero if it waits again. Session memory: [`crate::clear_sector_work`]
/// drops every wait with the records.
#[derive(Resource, Default, Debug)]
pub(crate) struct SettlingBodies(BTreeMap<(Entity, SectorCoord), SettlingWait>);

#[derive(Debug)]
struct SettlingWait {
    frames: u32,
    renewed: bool,
}

impl SettlingBodies {
    /// Drop every wait not renewed since the last sweep. Runs once per frame,
    /// after every stage that freezes.
    pub(crate) fn sweep(&mut self) {
        self.0.retain(|_, wait| std::mem::take(&mut wait.renewed));
    }

    /// Drop every wait. Returns how many there were.
    pub(crate) fn clear(&mut self) -> usize {
        let waits = self.0.len();
        self.0.clear();
        waits
    }
}

/// Keep `entity` live another frame because `coord` cannot freeze it yet,
/// and count the frame if virtual time advanced.
///
/// # Panics
///
/// When the body has waited more than [`SETTLING_FRAMES_MAX`] advancing
/// frames: it is stuck, and the panic names it, its cell and the reason.
pub(crate) fn hold_unsettled(
    world: &mut World,
    entity: Entity,
    coord: SectorCoord,
    unsettled: UnsettledBody,
) {
    let advancing = !world.resource::<Time<Virtual>>().delta().is_zero();
    let mut settling = world.resource_mut::<SettlingBodies>();
    let wait = settling.0.entry((entity, coord)).or_insert(SettlingWait {
        frames: 0,
        renewed: false,
    });
    // Adopt keys a wait by the off-window cell a body stands in, and Retire
    // by the live cell whose root still holds it. Each key is asked once per
    // frame, so one body can count under both keys while it drifts out of a
    // retiring cell, and each count stays bounded.
    if advancing {
        wait.frames += 1;
    }
    wait.renewed = true;
    let frames = wait.frames;
    if frames > SETTLING_FRAMES_MAX {
        let label = world
            .get::<Name>(entity)
            .map_or_else(|| "<unnamed>".to_string(), ToString::to_string);
        panic!(
            "nova_world: '{label}' ({entity}) in {coord} stayed unsettled for {frames} advancing \
             frames, more than {SETTLING_FRAMES_MAX}: {unsettled}"
        );
    }
}

/// One off-window cell's record.
#[derive(Debug)]
pub enum FrozenSector {
    /// A cell that was generated and then retired. The bodies are the whole
    /// truth of it: the generator is asked again only for its planetoids,
    /// which never change.
    Visited(Vec<FrozenBody>),
    /// A cell never generated, holding bodies that moved into it while it was
    /// off-window. Its first generation spawns them beside the generated
    /// bodies and removes neither.
    Arrivals(Vec<FrozenBody>),
}

impl FrozenSector {
    /// Every body the record holds, in the order it froze them.
    pub fn bodies(&self) -> &[FrozenBody] {
        match self {
            Self::Visited(bodies) | Self::Arrivals(bodies) => bodies,
        }
    }

    /// Whether the cell was generated before it froze.
    pub fn is_visited(&self) -> bool {
        matches!(self, Self::Visited(_))
    }

    /// The seed and radius of each frozen rock, in record order: what a
    /// worker prepares the pristine geometry of before the main thread
    /// thaws them.
    pub(crate) fn rocks(&self) -> Vec<(u32, Meters)> {
        self.bodies()
            .iter()
            .filter_map(|body| match &body.body {
                FrozenBodyType::Asteroid(rock) => Some((rock.seed(), rock.radius())),
                _ => None,
            })
            .collect()
    }

    fn into_bodies(self) -> Vec<FrozenBody> {
        match self {
            Self::Visited(bodies) | Self::Arrivals(bodies) => bodies,
        }
    }
}

/// One persistent body, frozen.
#[derive(Debug)]
pub struct FrozenBody {
    id: Option<EntityId>,
    name: Option<Name>,
    /// World pose. A sector root stays at the origin with an identity
    /// transform, so a body's own transform is its world pose whether it was
    /// a root's child or top-level. A physics body's pose is its avian
    /// `Position` and `Rotation`, not its eased `Transform`.
    transform: Transform,
    visibility: Option<Visibility>,
    motion: Option<(LinearVelocity, AngularVelocity)>,
    body: FrozenBodyType,
}

impl FrozenBody {
    /// The body's scenario id. A severed wreck and a canister have none.
    pub fn id(&self) -> Option<&str> {
        self.id.as_ref().map(|id| id.0.as_str())
    }

    /// Where the body stood when it froze.
    pub fn transform(&self) -> Transform {
        self.transform
    }

    /// What the body is.
    pub fn body(&self) -> &FrozenBodyType {
        &self.body
    }
}

/// Every kind of body a cell can freeze, each in its owner crate's record.
#[derive(Debug)]
#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        clippy::large_enum_variant,
        reason = "Asteroid is the common frozen body; boxing it adds an allocation per rock only to shrink rare canister and ore records"
    )
)]
pub enum FrozenBodyType {
    /// A rock, with its carved field and the ore it still owes.
    Asteroid(FrozenAsteroid),
    /// A ship, with its hold, damage, surviving sections, ammunition and AI.
    Ship(Box<FrozenShip>),
    /// A ship held back because the observer overlapped it, still a manifest
    /// entry.
    PendingShip(Box<SectorShip>),
    /// A loose cargo canister with its own runtime id.
    Canister(FrozenCanister),
    /// Structure severed from a ship.
    WreckFragment(FrozenWreckFragment),
    /// The ore a spent rock still owed when it was mined out.
    OreDrop(FrozenOreDrop),
}

/// Freeze one persistent body.
///
/// # Errors
///
/// The owner's [`UnsettledBody`] while a multi-frame process runs on it. The
/// caller keeps the body live and asks again on a later frame.
///
/// # Panics
///
/// On a docked ship: freezing one half of a pair would release the dock and
/// leave its partner flying alone. And on an entity that is no persistent body
/// this module knows how to freeze.
pub(crate) fn freeze_body(world: &World, entity: Entity) -> Result<FrozenBody, UnsettledBody> {
    let body = world.entity(entity);
    if let Some(held) = body.get::<PendingSectorShip>() {
        let ship = held.ship().clone();
        return Ok(FrozenBody {
            id: Some(EntityId::new(ship.id.clone())),
            name: None,
            transform: Transform::IDENTITY,
            visibility: None,
            motion: None,
            body: FrozenBodyType::PendingShip(Box::new(ship)),
        });
    }
    let name = body.get::<Name>().cloned();
    let label = name
        .as_ref()
        .map_or_else(|| entity.to_string(), ToString::to_string);
    let Some(mut transform) = body.get::<Transform>().copied() else {
        panic!("nova_world: persistent body '{label}' has no transform to freeze");
    };
    // An interpolated body's Transform trails its physics pose by part of a
    // fixed step; avian's pose is where the body is.
    if let Some((position, rotation)) = body.get::<Position>().zip(body.get::<Rotation>()) {
        transform.translation = position.0;
        transform.rotation = rotation.0;
    }
    let frozen = if body.contains::<SpaceshipRootMarker>() {
        assert!(
            !body.contains::<DockedShip>(),
            "nova_world: ship '{label}' is docked and its cell is about to freeze it; a docked \
             pair cannot be split across the active window"
        );
        FrozenBodyType::Ship(Box::new(freeze_ship(world, entity)?))
    } else if body.contains::<AsteroidMarker>() {
        FrozenBodyType::Asteroid(freeze_asteroid(world, entity)?)
    } else if body.contains::<CargoCanister>() {
        FrozenBodyType::Canister(freeze_canister(world, entity)?)
    } else if body.contains::<ShipWreckFragmentMarker>() {
        FrozenBodyType::WreckFragment(freeze_wreck_fragment(world, entity)?)
    } else if body.contains::<MinedOreDrop>() {
        FrozenBodyType::OreDrop(freeze_ore_drop(world, entity))
    } else {
        panic!("nova_world: '{label}' is no persistent body a sector can freeze");
    };
    Ok(FrozenBody {
        id: body.get::<EntityId>().cloned(),
        name,
        transform,
        visibility: body.get::<Visibility>().copied(),
        motion: body
            .get::<LinearVelocity>()
            .copied()
            .zip(body.get::<AngularVelocity>().copied()),
        body: frozen,
    })
}

/// Spawn a record's bodies under `root`, each where and as it froze.
///
/// `rocks` is the pristine geometry of each frozen rock, prepared on the
/// cell's worker from [`FrozenSector::rocks`], in record order.
///
/// # Panics
///
/// When `rocks` does not hold exactly one geometry per frozen rock. A
/// record changes only while its cell is off-window, and every job for an
/// off-window cell is cancelled, so a mismatch is a job prepared against
/// another record.
pub(crate) fn thaw_record(
    commands: &mut Commands,
    root: Entity,
    coord: SectorCoord,
    record: FrozenSector,
    rocks: Vec<PreparedAsteroid>,
) {
    let mut rocks = rocks.into_iter();
    for frozen in record.into_bodies() {
        let FrozenBody {
            id,
            name,
            transform,
            visibility,
            motion,
            body,
        } = frozen;
        if let FrozenBodyType::PendingShip(ship) = body {
            commands.spawn((PendingSectorShip(*ship), ChildOf(root)));
            continue;
        }
        let mut entity = commands.spawn((
            ScenarioScopedMarker,
            transform,
            GlobalTransform::from(transform),
            ChildOf(root),
        ));
        if let Some(id) = id {
            entity.insert(id);
        }
        if let Some(name) = name {
            entity.insert(name);
        }
        match body {
            FrozenBodyType::Asteroid(rock) => {
                let Some(geometry) = rocks.next() else {
                    panic!("nova_world: {coord} thawed more frozen rocks than its job prepared");
                };
                thaw_asteroid(&mut entity, rock, geometry);
            }
            FrozenBodyType::Ship(ship) => thaw_ship(&mut entity, *ship),
            FrozenBodyType::Canister(canister) => {
                let linear = motion.map_or(Vec3::ZERO, |(linear, _)| linear.0);
                entity.insert(thaw_canister(canister, transform, linear));
            }
            FrozenBodyType::WreckFragment(fragment) => thaw_wreck_fragment(&mut entity, fragment),
            FrozenBodyType::OreDrop(drop) => {
                entity.insert(thaw_ore_drop(drop));
            }
            FrozenBodyType::PendingShip(_) => unreachable!("spawned above"),
        }
        // After the owner's thaw: a spawn bundle seeds the velocity and the
        // visibility it was authored with, and the body keeps the ones it
        // froze with.
        if let Some(motion) = motion {
            entity.insert(motion);
        }
        if let Some(visibility) = visibility {
            entity.insert(visibility);
        }
    }
    assert!(
        rocks.next().is_none(),
        "nova_world: {coord} was prepared with more rock geometry than it froze rocks"
    );
}

/// The persistent bodies adoption moves between cells, and a replaced world
/// takes with its roots when they stand top-level.
pub(crate) type PersistentBody = Or<(
    With<SpaceshipRootMarker>,
    With<AsteroidMarker>,
    With<CargoCanister>,
    With<ShipWreckFragmentMarker>,
    With<MinedOreDrop>,
)>;

/// Give every persistent body to the cell it stands in now.
///
/// A body is owned by the live root of its current cell. A retiring root
/// keeps the bodies it holds until it freezes them, and takes no new one:
/// a body that moves into its cell is handled as if the cell were not live.
/// One whose cell is desired but not live yet goes top-level, if the root
/// holding it is about to retire, and waits there for its cell. One whose
/// cell is off-window and not live is frozen into that cell's record and
/// despawned: no body is simulated outside the window. A body whose owner reports it unsettled
/// stays live and is asked again next frame, for a bounded number of
/// advancing frames.
///
/// The [`WorldObserver`] and every authored, addressable object stay
/// top-level and are never adopted: the world streams around the first, and a
/// scenario owns the second. A body parented to anything but a sector root
/// belongs to that parent.
///
/// A transient (a projectile, a chunk of debris) is not adopted. One that is
/// top-level outside the window is despawned.
///
/// # Panics
///
/// Through [`live_sectors`] on a duplicate root, through `freeze_body` on
/// a docked ship bound for an off-window cell, and on a body that stays
/// unsettled for too many advancing frames.
///
/// When a caller wrote [`WorldConfig`] after
/// [`crate::NovaWorldSystems::Cleanup`] had already gone.
pub fn adopt_moving_bodies<G: SectorGenerator>(world: &mut World) {
    let config = world.resource_ref::<WorldConfig<G>>();
    assert_world_was_cleared(config.last_changed(), world.resource::<ClearedConfig>());
    let (edge, radius) = (config.sector_edge, config.active_radius);
    let desired = desired_sectors(world.resource::<CurrentSector>().0, radius);
    let live = live_sectors(world.query::<(Entity, &SectorRoot)>().iter(world));
    let cells: BTreeMap<Entity, SectorCoord> =
        live.iter().map(|(coord, root)| (*root, *coord)).collect();
    // A live root whose cell holds a record is retiring. A docked ship it
    // took would reach `freeze_body`, which refuses to split the pair.
    let retiring: BTreeSet<SectorCoord> = live
        .keys()
        .copied()
        .filter(|coord| world.resource::<FrozenSectors>().get(*coord).is_some())
        .collect();

    // A physics body's cell is read from its avian `Position`, the pose it
    // freezes with: an interpolated `Transform` trails it by part of a fixed
    // step and can stand on the other side of a face.
    let bodies: Vec<(Entity, Vec3, Option<Entity>)> = world
        .query_filtered::<(Entity, &Transform, Option<&Position>, Option<&ChildOf>), (
            PersistentBody,
            Without<WorldObserver>,
            Without<ScenarioAddressableMarker>,
        )>()
        .iter(world)
        .map(|(entity, transform, position, parent)| {
            let translation = position.map_or(transform.translation, |position| position.0);
            (entity, translation, parent.map(ChildOf::parent))
        })
        .collect();
    for (entity, translation, parent) in bodies {
        let owner = match parent {
            None => None,
            Some(parent) => match cells.get(&parent) {
                Some(cell) => Some((parent, *cell)),
                None => continue,
            },
        };
        // Engine boundary: a pose counts world units, a cell meters.
        let coord = SectorCoord::containing(Meters3::from_engine(translation), edge);
        match live.get(&coord) {
            // Its own cell's body; a retiring root freezes it with the cell.
            Some(&root) if parent == Some(root) => {}
            Some(&root) if !retiring.contains(&coord) => {
                trace!("nova_world: {entity} moved into {coord}");
                world.entity_mut(entity).insert(ChildOf(root));
            }
            _ if desired.contains(&coord) => {
                if owner.is_some_and(|(_, cell)| !desired.contains(&cell)) {
                    trace!("nova_world: {entity} waits top-level for {coord} to materialize");
                    world.entity_mut(entity).remove::<ChildOf>();
                }
            }
            _ => match freeze_body(world, entity) {
                Ok(body) => {
                    debug!("nova_world: froze {entity} into off-window {coord}");
                    world.resource_mut::<FrozenSectors>().arrive(coord, body);
                    world.entity_mut(entity).despawn();
                }
                Err(unsettled) => {
                    debug!(
                        "nova_world: {entity} left the window into {coord} but cannot freeze \
                         yet: {unsettled}"
                    );
                    hold_unsettled(world, entity, coord, unsettled);
                }
            },
        }
    }

    let transients: Vec<Entity> = world
        .query_filtered::<(Entity, &Transform), (With<TempEntity>, Without<ChildOf>)>()
        .iter(world)
        .filter(|(_, transform)| {
            let position = Meters3::from_engine(transform.translation);
            !desired.contains(&SectorCoord::containing(position, edge))
        })
        .map(|(entity, _)| entity)
        .collect();
    for entity in transients {
        trace!("nova_world: despawning transient {entity} outside the window");
        world.entity_mut(entity).despawn();
    }
}
