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
//! The ledger lives in memory while the world runs. [`crate::clear_sector_work`]
//! drops every record on the frames it clears the world. A saved world writes
//! it out whole: [`snapshot_sectors`] adds the live cells to a copy of it
//! without despawning anything, and [`FrozenSectors::restore`] seeds a fresh
//! world's ledger from one read back. Every record is plain data for that
//! reason; a collider or a mesh is rebuilt at thaw, never kept.
//!
//! The owner crates keep their bodies' private state private: each one exposes
//! a `freeze_*` and a `thaw_*` pair. This module decides WHEN a body freezes
//! and which cell keeps it, never what is inside it.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use nova_events::prelude::{EntityId, GeneratedBodyId, Meters3, ScenarioAddressableMarker};
use nova_gameplay::prelude::{
    CargoCanister, PlayerFleetMarker, SpaceshipRootMarker, TempEntity, UnsettledBody, WorldClock,
};
use nova_scenario::prelude::{
    freeze_asteroid, freeze_ore_drop, freeze_ship, freeze_station, prepare_frozen_asteroid,
    thaw_asteroid, thaw_ore_drop, thaw_ship, thaw_station, AsteroidMarker, FrozenAsteroid,
    FrozenOreDrop, FrozenShip, FrozenStation, MinedOreDrop, PreparedAsteroid, ScenarioScopedMarker,
    StationMarker,
};
use nova_ship::prelude::{
    freeze_canister, freeze_wreck_fragment, thaw_canister, thaw_wreck_fragment, DockedShip,
    DockingConnection, FrozenCanister, FrozenWreckFragment, ParkedHull, ShipWreckFragmentMarker,
    StationDocks,
};

use crate::{
    desired_sectors, live_sectors,
    streaming::{assert_world_was_cleared, freeze_sector_bodies, ClearedConfig},
    CurrentSector, PendingSectorShip, PendingSectorStation, SectorCoord, SectorGenerator,
    SectorRoot, SectorShip, SectorStation, WorldConfig, WorldObserver,
};

/// The frozen cells of this world, keyed by cell.
///
/// A cell has at most one record. A [`FrozenSector::Visited`] record is
/// consumed when its cell materializes again; an [`FrozenSector::Arrivals`]
/// record is consumed when its cell first generates.
///
/// Each record is shared, so a save copies the ledger without copying its
/// bodies, and a cell's job reads its record off the main thread. Changing a
/// record a save or a job still holds copies it first.
#[derive(Resource, Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct FrozenSectors(BTreeMap<SectorCoord, Arc<FrozenSector>>);

impl FrozenSectors {
    /// The record a cell holds, if it holds one.
    pub fn get(&self, coord: SectorCoord) -> Option<&FrozenSector> {
        self.0.get(&coord).map(Arc::as_ref)
    }

    /// The record a cell holds, shared, for the job that prepares the cell.
    pub(crate) fn shared(&self, coord: SectorCoord) -> Option<Arc<FrozenSector>> {
        self.0.get(&coord).cloned()
    }

    /// Every cell's record, in cell order.
    pub fn iter(&self) -> impl Iterator<Item = (SectorCoord, &FrozenSector)> {
        self.0
            .iter()
            .map(|(coord, record)| (*coord, record.as_ref()))
    }

    /// Seed this world's empty ledger with a saved world's records, before
    /// any cell of it materializes.
    ///
    /// # Panics
    ///
    /// When the ledger already holds a record: a saved world replaces no
    /// world, and merging two would thaw bodies neither of them froze.
    pub fn restore(&mut self, saved: FrozenSectors) {
        assert!(
            self.0.is_empty(),
            "nova_world: restoring a saved ledger over {} live record(s)",
            self.0.len()
        );
        self.0 = saved.0;
    }

    /// How many cells hold a record.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no cell holds a record.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Drop a cell's record on the frame the cell materializes: the job that
    /// prepared the cell took the record's bodies with it. Returns how many
    /// bodies the record held.
    pub(crate) fn consume(&mut self, coord: SectorCoord) -> usize {
        self.0
            .remove(&coord)
            .map_or(0, |record| record.bodies().len())
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
        let record = Arc::make_mut(
            self.0
                .entry(coord)
                .or_insert_with(|| Arc::new(FrozenSector::Visited(Vec::new()))),
        );
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
        let record = Arc::make_mut(
            self.0
                .entry(coord)
                .or_insert_with(|| Arc::new(FrozenSector::Arrivals(Vec::new()))),
        );
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
            .get::<EntityId>(entity)
            .map(|id| id.0.clone())
            .or_else(|| world.get::<Name>(entity).map(ToString::to_string))
            .unwrap_or_else(|| "<unnamed>".to_string());
        panic!(
            "nova_world: '{label}' ({entity}) in {coord} stayed unsettled for {frames} advancing \
             frames, more than {SETTLING_FRAMES_MAX}: {unsettled}"
        );
    }
}

/// One off-window cell's record.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
}

/// One frozen body with what a worker prepared for it: a rock's geometry,
/// pristine and, for a carved one, the surface its field meshes to; a held
/// ship's or station's manifest entry, regenerated.
///
/// Built only by [`PreparedFrozenBody::prepare`], so a rock always travels
/// with its geometry, a held body always with its entry, and nothing else
/// carries either.
#[derive(Debug)]
pub enum PreparedFrozenBody {
    /// A body that thaws where it froze.
    Body(Box<(FrozenBody, Option<PreparedAsteroid>)>),
    /// A ship still held back, to spawn once the observer is clear of it.
    HeldShip(Box<SectorShip>),
    /// A station still held back, to spawn once the observer is clear of it.
    HeldStation(Box<SectorStation>),
}

impl PreparedFrozenBody {
    /// Prepare `body` of `coord` for its thaw, `ships` and `stations` being
    /// the cell's regenerated ships and stations. PURE, for the cell's
    /// worker.
    ///
    /// # Panics
    ///
    /// When a held ship's or station's id names none of `ships` or
    /// `stations`. The generator is pinned by the catalog digest the save
    /// carries, so the cell cannot regenerate without it: an assembly fault,
    /// not a runtime condition.
    pub fn prepare(
        coord: SectorCoord,
        body: FrozenBody,
        ships: &[SectorShip],
        stations: &[SectorStation],
    ) -> Self {
        match &body.body {
            FrozenBodyType::PendingShip { id } => {
                let Some(ship) = ships.iter().find(|ship| ship.id.to_string() == *id) else {
                    panic!(
                        "nova_world: {coord} froze the held ship '{id}' but regenerates no ship \
                         of that id"
                    );
                };
                Self::HeldShip(Box::new(ship.clone()))
            }
            FrozenBodyType::PendingStation { id } => {
                let Some(station) = stations
                    .iter()
                    .find(|station| station.id.to_string() == *id)
                else {
                    panic!(
                        "nova_world: {coord} froze the held station '{id}' but regenerates no \
                         station of that id"
                    );
                };
                Self::HeldStation(Box::new(station.clone()))
            }
            FrozenBodyType::Asteroid(rock) => {
                let geometry = prepare_frozen_asteroid(rock);
                Self::Body(Box::new((body, Some(geometry))))
            }
            _ => Self::Body(Box::new((body, None))),
        }
    }
}

/// One persistent body, frozen.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FrozenBody {
    id: Option<EntityId>,
    name: Option<String>,
    /// World pose. A sector root stays at the origin with an identity
    /// transform, so a body's own transform is its world pose whether it was
    /// a root's child or top-level. A physics body's pose is its avian
    /// `Position` and `Rotation`, not its eased `Transform`.
    transform: Transform,
    #[cfg_attr(feature = "serde", serde(with = "visibility_serde"))]
    visibility: Option<Visibility>,
    /// Linear and angular velocity, in engine units.
    motion: Option<(Vec3, Vec3)>,
    body: FrozenBodyType,
    /// The [`WorldClock`] reading when the body froze, in play seconds.
    frozen_at: f64,
}

/// [`Visibility`] has no serde form of its own; a record writes its variant
/// name.
#[cfg(feature = "serde")]
mod visibility_serde {
    use bevy::prelude::Visibility;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    #[serde(remote = "Visibility")]
    enum VisibilityDef {
        Inherited,
        Hidden,
        Visible,
    }

    #[derive(Serialize, Deserialize)]
    struct Authored(#[serde(with = "VisibilityDef")] Visibility);

    pub(super) fn serialize<S: Serializer>(
        visibility: &Option<Visibility>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        visibility.map(Authored).serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Visibility>, D::Error> {
        Ok(Option::<Authored>::deserialize(deserializer)?.map(|Authored(visibility)| visibility))
    }
}

impl FrozenBody {
    /// The body's id: a scenario id, a generated id, or the id a wreck's
    /// sever minted. A canister has none, nor a wreck whose sever could not
    /// mint one.
    pub fn id(&self) -> Option<&str> {
        self.id.as_ref().map(|id| id.0.as_str())
    }

    /// Where the body stood when it froze.
    pub fn transform(&self) -> Transform {
        self.transform
    }

    /// The linear and angular velocity the body froze with, in engine
    /// units; `None` for a body that had no physics motion.
    pub fn motion(&self) -> Option<(Vec3, Vec3)> {
        self.motion
    }

    /// What the body is.
    pub fn body(&self) -> &FrozenBodyType {
        &self.body
    }

    /// The play seconds the world clock read when the body froze.
    pub fn frozen_at(&self) -> f64 {
        self.frozen_at
    }
}

/// On a body thawed from a frozen record: how many play seconds it stood
/// frozen. A body the generator spawned fresh has none.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ThawedAfter {
    /// Play seconds between the body's freeze and its thaw.
    pub play_seconds: f64,
}

/// Every kind of body a cell can freeze, each in its owner crate's record.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// A station: its design id, its hull as a ship record, and its storage.
    Station(Box<FrozenStation>),
    /// A ship held back because the observer overlapped it, by the id of
    /// its manifest entry. The cell regenerates the entry on its worker:
    /// the catalog digest a save carries pins the generator, so the same id
    /// names the same ship.
    PendingShip {
        /// The held ship's generated id.
        id: String,
    },
    /// A station held back because the observer overlapped it, by the id of
    /// its manifest entry, regenerated like a held ship's.
    PendingStation {
        /// The held station's generated id.
        id: String,
    },
    /// A loose cargo canister with its own runtime id.
    Canister(FrozenCanister),
    /// Structure severed from a ship.
    WreckFragment(FrozenWreckFragment),
    /// The ore a spent rock still owed when it was mined out.
    OreDrop(FrozenOreDrop),
}

/// What freezing a docked ship means to the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockPolicy {
    /// The body is about to despawn: a docked ship is a fault, because
    /// freezing one half of a pair would release the dock and leave its
    /// partner flying alone.
    Refuse,
    /// The body stays live and a save copies it: a docked ship is kept
    /// without its dock, and a saved world thaws the pair undocked where it
    /// stood.
    Undock,
}

/// Freeze one persistent body.
///
/// # Errors
///
/// The owner's [`UnsettledBody`] while a multi-frame process runs on it. The
/// caller keeps the body live and asks again on a later frame.
///
/// The record is stamped with `clock`, so its thaw knows how much play
/// passed while it stood frozen.
///
/// # Panics
///
/// On a docked ship under [`DockPolicy::Refuse`]. And on an entity that is no
/// persistent body this module knows how to freeze.
pub fn freeze_body(
    world: &World,
    entity: Entity,
    docked: DockPolicy,
    clock: &WorldClock,
) -> Result<FrozenBody, UnsettledBody> {
    let frozen_at = clock.play_seconds;
    let body = world.entity(entity);
    if let Some(held) = body.get::<PendingSectorShip>() {
        let id = held.ship().id.to_string();
        return Ok(FrozenBody {
            id: Some(EntityId::new(id.clone())),
            name: None,
            transform: Transform::IDENTITY,
            visibility: None,
            motion: None,
            body: FrozenBodyType::PendingShip { id },
            frozen_at,
        });
    }
    if let Some(held) = body.get::<PendingSectorStation>() {
        let id = held.station().id.to_string();
        return Ok(FrozenBody {
            id: Some(EntityId::new(id.clone())),
            name: None,
            transform: Transform::IDENTITY,
            visibility: None,
            motion: None,
            body: FrozenBodyType::PendingStation { id },
            frozen_at,
        });
    }
    let name = body.get::<Name>().map(ToString::to_string);
    let label = body
        .get::<EntityId>()
        .map(|id| id.0.clone())
        .or_else(|| name.clone())
        .unwrap_or_else(|| entity.to_string());
    let Some(mut transform) = body.get::<Transform>().copied() else {
        panic!("nova_world: persistent body '{label}' has no transform to freeze");
    };
    // An interpolated body's Transform trails its physics pose by part of a
    // fixed step; avian's pose is where the body is.
    if let Some((position, rotation)) = body.get::<Position>().zip(body.get::<Rotation>()) {
        transform.translation = position.0;
        transform.rotation = rotation.0;
    }
    // A station root also carries `SpaceshipRootMarker`, so its own marker
    // decides first.
    let frozen = if body.contains::<StationMarker>() {
        assert!(
            docked == DockPolicy::Undock || only_parked_docks(world, entity),
            "nova_world: station '{label}' has a ship docked and its cell is about to freeze \
             it; a docked assembly cannot be split across the active window"
        );
        FrozenBodyType::Station(Box::new(freeze_station(world, entity)?))
    } else if body.contains::<SpaceshipRootMarker>() {
        assert!(
            docked == DockPolicy::Undock || !body.contains::<DockedShip>(),
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
            .map(|linear| linear.0)
            .zip(body.get::<AngularVelocity>().map(|angular| angular.0)),
        body: frozen,
        frozen_at,
    })
}

/// Whether every ship docked at `station` is a hull parked there, which
/// freezes inside the station's record.
fn only_parked_docks(world: &World, station: Entity) -> bool {
    world.get::<StationDocks>(station).is_none_or(|docks| {
        docks.iter().all(|connection| {
            world
                .get::<DockingConnection>(connection)
                .is_some_and(|connection| {
                    [connection.first_ship, connection.second_ship]
                        .into_iter()
                        .filter(|ship| *ship != station)
                        .all(|ship| {
                            world
                                .get::<ParkedHull>(ship)
                                .is_some_and(|parked| parked.0 == station)
                        })
                })
        })
    })
}

/// Spawn frozen `bodies` under `root`, each where and as it froze, with the
/// geometry the cell's worker prepared for it. Each thawed body carries
/// [`ThawedAfter`]: the play seconds between its freeze and `clock`, for its
/// owner's catch-up rule. A held ship or station regenerates from its entry
/// unchanged and carries none.
pub fn thaw_record(
    commands: &mut Commands,
    root: Entity,
    bodies: Vec<PreparedFrozenBody>,
    clock: &WorldClock,
) {
    for prepared in bodies {
        let (
            FrozenBody {
                id,
                name,
                transform,
                visibility,
                motion,
                body,
                frozen_at,
            },
            geometry,
        ) = match prepared {
            PreparedFrozenBody::HeldShip(ship) => {
                commands.spawn((PendingSectorShip(*ship), ChildOf(root)));
                continue;
            }
            PreparedFrozenBody::HeldStation(station) => {
                commands.spawn((PendingSectorStation(*station), ChildOf(root)));
                continue;
            }
            PreparedFrozenBody::Body(prepared) => *prepared,
        };
        let mut entity = commands.spawn((
            ScenarioScopedMarker,
            transform,
            GlobalTransform::from(transform),
            ChildOf(root),
            ThawedAfter {
                play_seconds: clock.elapsed_since(frozen_at),
            },
        ));
        if let Some(id) = id {
            // A generated body thaws with its typed id beside the string, as
            // it spawned. A transient's id - a canister's, a wreck's - is not
            // generated and gets none.
            if let Ok(generated) = id.0.parse::<GeneratedBodyId>() {
                entity.insert(generated);
            }
            entity.insert(id);
        }
        if let Some(name) = name {
            entity.insert(Name::new(name));
        }
        match body {
            FrozenBodyType::Asteroid(rock) => {
                let geometry =
                    geometry.expect("PreparedFrozenBody::prepare pairs every rock with geometry");
                thaw_asteroid(&mut entity, rock, geometry);
            }
            FrozenBodyType::Ship(ship) => thaw_ship(&mut entity, *ship),
            FrozenBodyType::Station(station) => thaw_station(&mut entity, *station),
            FrozenBodyType::Canister(canister) => {
                let linear = motion.map_or(Vec3::ZERO, |(linear, _)| linear);
                entity.insert(thaw_canister(canister, transform, linear));
            }
            FrozenBodyType::WreckFragment(fragment) => thaw_wreck_fragment(&mut entity, fragment),
            FrozenBodyType::OreDrop(drop) => {
                entity.insert(thaw_ore_drop(drop));
            }
            FrozenBodyType::PendingShip { .. } | FrozenBodyType::PendingStation { .. } => {
                unreachable!("PreparedFrozenBody::prepare resolves a held body to its entry")
            }
        }
        // After the owner's thaw: a spawn bundle seeds the velocity and the
        // visibility it was authored with, and the body keeps the ones it
        // froze with.
        if let Some((linear, angular)) = motion {
            entity.insert((LinearVelocity(linear), AngularVelocity(angular)));
        }
        if let Some(visibility) = visibility {
            entity.insert(visibility);
        }
    }
}

/// The persistent bodies adoption moves between cells, and a replaced world
/// takes with its roots when they stand top-level. A hull parked at a
/// station is none: it freezes and despawns with the station.
pub(crate) type PersistentBody = (
    Or<(
        With<SpaceshipRootMarker>,
        With<AsteroidMarker>,
        With<CargoCanister>,
        With<ShipWreckFragmentMarker>,
        With<MinedOreDrop>,
    )>,
    Without<ParkedHull>,
);

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
/// The [`WorldObserver`], every escort of the player's fleet
/// ([`PlayerFleetMarker`]) and every authored, addressable object stay
/// top-level and are never adopted: the world streams around the first, the
/// fleet travels with it, and a scenario owns the last. A body parented to
/// anything but a sector root belongs to that parent.
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
            Without<PlayerFleetMarker>,
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
            _ => match freeze_body(
                world,
                entity,
                DockPolicy::Refuse,
                world.resource::<WorldClock>(),
            ) {
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

/// Why [`snapshot_sectors`] could not copy the world.
#[derive(Debug)]
pub enum SectorSnapshotError {
    /// A body is in a multi-frame process its owner cannot freeze yet. Ask
    /// again on a later frame.
    Unsettled {
        /// The body's name, or its entity.
        label: String,
        /// The cell that holds it.
        coord: SectorCoord,
        /// The owner's reason.
        reason: UnsettledBody,
    },
    /// A persistent body that no cell owns and that no save can place: an
    /// authored, addressable one other than the observer, or one parented to
    /// something that is not a sector root. A fleet escort is neither: the
    /// save keeps it with the player. A save refuses rather than drop it.
    UnownedBody {
        /// The body's name, or its entity.
        label: String,
    },
    /// A saved transient points at a live body with no `EntityId`, so no
    /// Load could find it again. The save collector of the game returns
    /// it; a sector snapshot never does.
    NoDurableId {
        /// The transient and the reference.
        label: String,
    },
    /// Two saved bodies have one id, so a Load could not tell them apart.
    /// The save collector of the game returns it; a sector snapshot never
    /// does.
    DuplicateId {
        /// The id.
        id: EntityId,
    },
    /// The saved state breaks an id rule other than a duplicate body id: an id
    /// with a `/` that is not a minted wreck id, two canisters with one id, a
    /// canister id at or past the saved next canister id, or a reference that
    /// names nothing the save keeps. The save collector of the game returns
    /// it.
    InvalidSavedState {
        /// What is wrong.
        reason: String,
    },
}

impl std::fmt::Display for SectorSnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsettled {
                label,
                coord,
                reason,
            } => write!(f, "'{label}' in {coord} cannot freeze yet: {reason}"),
            Self::UnownedBody { label } => write!(
                f,
                "'{label}' is a persistent body that no sector owns, so no save can keep it"
            ),
            Self::NoDurableId { label } => {
                write!(f, "{label} has no durable id, so no save can keep it")
            }
            Self::DuplicateId { id } => write!(
                f,
                "two saved bodies have the id '{}', so no Load could tell them apart",
                id.0
            ),
            Self::InvalidSavedState { reason } => f.write_str(reason),
        }
    }
}

/// The whole streamed world as it would freeze now, without despawning
/// anything: the ledger, every live cell's bodies as a visited record, and
/// every body waiting top-level for its cell as an arrival there.
///
/// A docked ship is kept without its dock ([`DockPolicy::Undock`]), so a
/// saved world thaws the pair undocked where it stood. The
/// [`WorldObserver`] and the player's fleet ([`PlayerFleetMarker`]) are not
/// part of it: the world streams around the observer, the fleet travels with
/// it, and their owner saves both.
///
/// # Errors
///
/// [`SectorSnapshotError::Unsettled`] while any body is mid-process, and
/// [`SectorSnapshotError::UnownedBody`] for a persistent body no record could
/// hold.
///
/// # Panics
///
/// Without a [`WorldConfig`]: there is no world to copy.
pub fn snapshot_sectors<G: SectorGenerator>(
    world: &mut World,
) -> Result<FrozenSectors, SectorSnapshotError> {
    let edge = world.resource::<WorldConfig<G>>().sector_edge;
    let label = |world: &World, entity: Entity| {
        world
            .get::<EntityId>(entity)
            .map(|id| id.0.clone())
            .or_else(|| world.get::<Name>(entity).map(ToString::to_string))
            .unwrap_or_else(|| entity.to_string())
    };

    let unowned: Option<Entity> = world
        .query_filtered::<(Entity, Option<&ChildOf>, Has<ScenarioAddressableMarker>), (
            PersistentBody,
            Without<WorldObserver>,
            Without<PlayerFleetMarker>,
        )>()
        .iter(world)
        .find(|(_, parent, addressable)| {
            *addressable
                || parent
                    .is_some_and(|parent| !world.entity(parent.parent()).contains::<SectorRoot>())
        })
        .map(|(entity, ..)| entity);
    if let Some(entity) = unowned {
        return Err(SectorSnapshotError::UnownedBody {
            label: label(world, entity),
        });
    }

    let mut ledger = world.resource::<FrozenSectors>().clone();
    let roots = live_sectors(world.query::<(Entity, &SectorRoot)>().iter(world));
    for (coord, root) in roots {
        let (frozen, unsettled) = freeze_sector_bodies(world, coord, root, DockPolicy::Undock);
        if let Some((entity, reason)) = unsettled.into_iter().next() {
            return Err(SectorSnapshotError::Unsettled {
                label: label(world, entity),
                coord,
                reason,
            });
        }
        ledger.visit(coord, frozen.into_iter().map(|(_, body)| body).collect());
    }

    let waiting: Vec<(Entity, Vec3)> = world
        .query_filtered::<(Entity, &Transform, Option<&Position>), (
            PersistentBody,
            Without<ChildOf>,
            Without<WorldObserver>,
            Without<PlayerFleetMarker>,
            Without<ScenarioAddressableMarker>,
        )>()
        .iter(world)
        .map(|(entity, transform, position)| {
            (
                entity,
                position.map_or(transform.translation, |position| position.0),
            )
        })
        .collect();
    for (entity, translation) in waiting {
        // Engine boundary: a pose counts world units, a cell meters.
        let coord = SectorCoord::containing(Meters3::from_engine(translation), edge);
        match freeze_body(
            world,
            entity,
            DockPolicy::Undock,
            world.resource::<WorldClock>(),
        ) {
            Ok(body) => ledger.arrive(coord, body),
            Err(reason) => {
                return Err(SectorSnapshotError::Unsettled {
                    label: label(world, entity),
                    coord,
                    reason,
                })
            }
        }
    }
    Ok(ledger)
}
