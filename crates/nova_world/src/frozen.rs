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

use avian3d::prelude::{AngularVelocity, FixedJoint, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use nova_events::prelude::{EntityId, GeneratedBodyId, Meters3, ScenarioAddressableMarker};
use nova_gameplay::prelude::{
    CargoCanister, PlayerFleetMarker, SpaceshipRootMarker, TempEntity, UnsettledBody, WorldClock,
};
use nova_scenario::prelude::{
    echo_restored_dock, freeze_asteroid, freeze_ore_drop, freeze_ship, freeze_station,
    prepare_frozen_asteroid, thaw_asteroid, thaw_ore_drop, thaw_ship, thaw_station, AsteroidMarker,
    FrozenAsteroid, FrozenOreDrop, FrozenShip, FrozenStation, MinedOreDrop, PreparedAsteroid,
    ScenarioScopedMarker, StationMarker,
};
use nova_ship::prelude::{
    freeze_canister, freeze_wreck_fragment, restore_docking_connection, thaw_canister,
    thaw_wreck_fragment, DockedHelmType, DockedShip, DockingConnection, FrozenCanister,
    FrozenWreckFragment, ParkedHull, SavedJointFrame, ShipWreckFragmentMarker, StationDock,
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
    for id in with_docked(bodies).filter_map(FrozenBody::id) {
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

    /// Every body the record holds, each station followed by the ships that
    /// froze docked at it ([`FrozenDockedShip`]).
    pub fn each_body(&self) -> impl Iterator<Item = &FrozenBody> {
        with_docked(self.bodies())
    }
}

/// `bodies`, each station followed by the ships that froze docked at it.
fn with_docked(bodies: &[FrozenBody]) -> impl Iterator<Item = &FrozenBody> {
    bodies.iter().flat_map(|body| {
        std::iter::once(body).chain(body.docked().iter().map(FrozenDockedShip::body))
    })
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

    /// The ships that froze docked at this body, a station; none for every
    /// other body.
    pub fn docked(&self) -> &[FrozenDockedShip] {
        match &self.body {
            FrozenBodyType::Station(_, docked) => docked,
            _ => &[],
        }
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
    /// A station: its design id, its hull as a ship record, and its storage,
    /// with the ships docked at it that freeze inside its record.
    Station(Box<FrozenStation>, Vec<FrozenDockedShip>),
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

/// A ship docked at a station, frozen inside the station's record. The pair
/// freezes, saves and thaws as one, so their dock never meets the edge of the
/// active window and never splits across two records. Only a ship
/// [`station_dock`] names freezes here.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FrozenDockedShip {
    /// The ship as a plain body record: its id, name, pose, motion and ship.
    body: FrozenBody,
    dock: FrozenStationDock,
}

impl FrozenDockedShip {
    /// The ship's body record.
    pub fn body(&self) -> &FrozenBody {
        &self.body
    }

    /// The dock that holds it to its station.
    pub fn dock(&self) -> &FrozenStationDock {
        &self.dock
    }
}

/// The dock of a [`FrozenDockedShip`]: the connection the capture built, by
/// the two ports' section ids, with the fixed joint's local frames and the
/// helm. A thaw rebuilds it with `restore_docking_connection`, never with a
/// new capture.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FrozenStationDock {
    /// Whether the station is the connection's first ship and the joint's
    /// first body.
    station_first: bool,
    ship_port: EntityId,
    station_port: EntityId,
    /// The joint frame on the ship, in its body frame.
    ship_frame: SavedJointFrame,
    /// The joint frame on the station, in its body frame.
    station_frame: SavedJointFrame,
    helm: FrozenDockHelm,
}

impl FrozenStationDock {
    /// The section id of the ship's port.
    pub fn ship_port(&self) -> &EntityId {
        &self.ship_port
    }

    /// The section id of the station's port.
    pub fn station_port(&self) -> &EntityId {
        &self.station_port
    }

    /// Who flies the pair.
    pub fn helm(&self) -> FrozenDockHelm {
        self.helm
    }
}

/// The helm of a [`FrozenStationDock`]. Only the player holds a helm, and
/// the player's dock is saved beside the observer, so a frozen dock has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FrozenDockHelm {
    /// Neither hull flies the pair: the station is dead.
    Neutral,
    /// The living station owns the pair.
    Station,
}

/// What freezing a docked ship means to the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockPolicy {
    /// The body is about to despawn: a station's parked hulls and the ships
    /// [`station_dock`] names freeze inside the station's record, and any
    /// other docked ship is a fault, because freezing one half of a pair
    /// would release the dock and leave its partner flying alone.
    Refuse,
    /// The body stays live and a save copies it: a dock with the
    /// [`WorldObserver`] stays live and the observer's owner saves it beside
    /// the observer, so the copy freezes the partner as a plain body; a
    /// station's parked hulls and the ships [`station_dock`] names freeze
    /// inside the station's record; every other dock is refused before the
    /// copy (see [`snapshot_sectors`]).
    Copy,
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
/// On a ship [`station_dock`] names: it freezes inside its station's record.
/// On any other docked ship under [`DockPolicy::Refuse`]. On a docked ship
/// under [`DockPolicy::Copy`] whose dock [`snapshot_sectors`] did not already
/// confirm kept. On a station with a dock its record cannot keep. And on an
/// entity that is no persistent body this module knows how to freeze.
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
    let label = body_label(world, entity);
    // A station root also carries `SpaceshipRootMarker`, so its own marker
    // decides first.
    let frozen = if body.contains::<StationMarker>() {
        let docks_kept = world.get::<StationDocks>(entity).is_none_or(|docks| {
            docks.iter().all(|connection| {
                world
                    .get::<DockingConnection>(connection)
                    .is_some_and(|connection| {
                        [connection.first_ship, connection.second_ship]
                            .into_iter()
                            .filter(|ship| *ship != entity)
                            .all(|ship| {
                                parked_at(world, ship, entity)
                                    || station_dock(world, ship) == Some(entity)
                                    || (docked == DockPolicy::Copy
                                        && world.entity(ship).contains::<WorldObserver>())
                            })
                    })
            })
        });
        assert!(
            docks_kept,
            "nova_world: station '{label}' has a ship docked and its cell is about to freeze \
             it; a docked assembly cannot be split across the active window"
        );
        FrozenBodyType::Station(
            Box::new(freeze_station(world, entity)?),
            freeze_station_docks(world, entity, frozen_at)?,
        )
    } else if body.contains::<SpaceshipRootMarker>() {
        assert!(
            station_dock(world, entity).is_none(),
            "nova_world: ship '{label}' is docked at a station and freezes inside the \
             station's record; freezing it alone would split the pair"
        );
        let dock_kept = match docked {
            DockPolicy::Refuse => !body.contains::<DockedShip>(),
            DockPolicy::Copy => body.get::<DockedShip>().is_none_or(|docked_ship| {
                world
                    .get::<DockingConnection>(docked_ship.connection)
                    .is_some_and(|connection| kept_dock(world, connection))
            }),
        };
        assert!(
            dock_kept,
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
    Ok(frozen_body(world, entity, frozen, frozen_at))
}

/// A body's id, name or entity, for a fault.
fn body_label(world: &World, entity: Entity) -> String {
    world
        .get::<EntityId>(entity)
        .map(|id| id.0.clone())
        .or_else(|| world.get::<Name>(entity).map(ToString::to_string))
        .unwrap_or_else(|| entity.to_string())
}

/// `entity`'s record around its owner's `frozen` record: its id, name, pose,
/// visibility and motion.
///
/// # Panics
///
/// On a body with no transform.
fn frozen_body(
    world: &World,
    entity: Entity,
    frozen: FrozenBodyType,
    frozen_at: f64,
) -> FrozenBody {
    let body = world.entity(entity);
    let Some(mut transform) = body.get::<Transform>().copied() else {
        panic!(
            "nova_world: persistent body '{}' has no transform to freeze",
            body_label(world, entity)
        );
    };
    // An interpolated body's Transform trails its physics pose by part of a
    // fixed step; avian's pose is where the body is.
    if let Some((position, rotation)) = body.get::<Position>().zip(body.get::<Rotation>()) {
        transform.translation = position.0;
        transform.rotation = rotation.0;
    }
    FrozenBody {
        id: body.get::<EntityId>().cloned(),
        name: body.get::<Name>().map(ToString::to_string),
        transform,
        visibility: body.get::<Visibility>().copied(),
        motion: body
            .get::<LinearVelocity>()
            .map(|linear| linear.0)
            .zip(body.get::<AngularVelocity>().map(|angular| angular.0)),
        body: frozen,
        frozen_at,
    }
}

/// Whether `ship` is a hull parked at `station`.
fn parked_at(world: &World, ship: Entity, station: Entity) -> bool {
    world
        .get::<ParkedHull>(ship)
        .is_some_and(|parked| parked.0 == station)
}

/// The station `ship` is docked at, when the pair freezes and saves as one
/// record ([`FrozenDockedShip`]): `ship` is a plain hull with a durable id,
/// docked at a station with one, through two ports with section ids. A hull
/// parked at the station, the observer, a fleet escort and an authored,
/// addressable ship are none: each has its own owner. Any other dock is
/// refused rather than dropped.
pub(crate) fn station_dock(world: &World, ship: Entity) -> Option<Entity> {
    let body = world.entity(ship);
    if body.contains::<StationMarker>()
        || body.contains::<ParkedHull>()
        || body.contains::<WorldObserver>()
        || body.contains::<PlayerFleetMarker>()
        || body.contains::<ScenarioAddressableMarker>()
        || !body.contains::<EntityId>()
    {
        return None;
    }
    let connection = body.get::<DockedShip>()?.connection;
    let StationDock(station) = *world.get::<StationDock>(connection)?;
    let record = world.get::<DockingConnection>(connection)?;
    let station_root = world.entity(station);
    (station_root.contains::<EntityId>()
        && !station_root.contains::<ScenarioAddressableMarker>()
        && world.get::<EntityId>(record.first_section).is_some()
        && world.get::<EntityId>(record.second_section).is_some())
    .then_some(station)
}

/// Every ship docked at `station` that freezes inside its record
/// ([`station_dock`]), with its connection, in the order they docked.
fn station_visitors(world: &World, station: Entity) -> Vec<(Entity, Entity)> {
    let Some(docks) = world.get::<StationDocks>(station) else {
        return Vec::new();
    };
    docks
        .iter()
        .filter_map(|connection| {
            let record = world.get::<DockingConnection>(connection)?;
            let ship = if record.first_ship == station {
                record.second_ship
            } else {
                record.first_ship
            };
            (station_dock(world, ship) == Some(station)).then_some((ship, connection))
        })
        .collect()
}

/// Freeze every ship docked at `station` that freezes inside its record.
///
/// # Errors
///
/// [`UnsettledBody`] while a docked ship is mid-process ([`freeze_ship`]),
/// or while it is not yet under its station's parent: adoption moves it to
/// its station's cell on its next pass.
///
/// # Panics
///
/// On a dock whose helm the player holds: the player's dock is saved beside
/// the observer and never reaches a station record.
fn freeze_station_docks(
    world: &World,
    station: Entity,
    frozen_at: f64,
) -> Result<Vec<FrozenDockedShip>, UnsettledBody> {
    let parent = |entity: Entity| world.get::<ChildOf>(entity).map(ChildOf::parent);
    station_visitors(world, station)
        .into_iter()
        .map(|(ship, connection)| {
            if parent(ship) != parent(station) {
                return Err(UnsettledBody {
                    reason: "a ship docked at the station is not in the station's cell yet",
                });
            }
            let record = world
                .get::<DockingConnection>(connection)
                .expect("station_visitors names live connections");
            let joint = world
                .get::<FixedJoint>(connection)
                .expect("a docking connection holds its fixed joint");
            let station_first = record.first_ship == station;
            let (station_port, ship_port, station_frame, ship_frame) = if station_first {
                (
                    record.first_section,
                    record.second_section,
                    &joint.frame1,
                    &joint.frame2,
                )
            } else {
                (
                    record.second_section,
                    record.first_section,
                    &joint.frame2,
                    &joint.frame1,
                )
            };
            let helm = match record.helm {
                DockedHelmType::Neutral => FrozenDockHelm::Neutral,
                DockedHelmType::Station(hub) => {
                    assert_eq!(hub, station, "a station helm is the station's own");
                    FrozenDockHelm::Station
                }
                DockedHelmType::Held(holder) => panic!(
                    "nova_world: {holder} holds the helm of a dock at station '{}', and only \
                     the player's dock has a held helm",
                    body_label(world, station)
                ),
            };
            let port = |section: Entity| {
                world
                    .get::<EntityId>(section)
                    .cloned()
                    .expect("station_dock checks both section ids")
            };
            let ship_record = FrozenBodyType::Ship(Box::new(freeze_ship(world, ship)?));
            Ok(FrozenDockedShip {
                body: frozen_body(world, ship, ship_record, frozen_at),
                dock: FrozenStationDock {
                    station_first,
                    ship_port: port(ship_port),
                    station_port: port(station_port),
                    ship_frame: SavedJointFrame::of(ship_frame),
                    station_frame: SavedJointFrame::of(station_frame),
                    helm,
                },
            })
        })
        .collect()
}

/// Despawn `entity`, which [`freeze_body`] froze, with every ship that froze
/// docked at it inside its record.
pub(crate) fn despawn_frozen(world: &mut World, entity: Entity) {
    for (ship, _) in station_visitors(world, entity) {
        world.entity_mut(ship).despawn();
    }
    world.entity_mut(entity).despawn();
}

/// Whether a save keeps `connection`'s dock: one endpoint carries
/// [`WorldObserver`] and its owner saves the dock beside it, one endpoint is
/// a station and the other is a hull parked there, which freezes inside the
/// station's record, or the dock is one [`station_dock`] names, which
/// freezes inside the station's record as a [`FrozenDockedShip`]. Every
/// other dock is refused rather than dropped (see
/// [`SectorSnapshotError::UnsavedDock`]).
fn kept_dock(world: &World, connection: &DockingConnection) -> bool {
    let [first, second] = [connection.first_ship, connection.second_ship];
    if world.entity(first).contains::<WorldObserver>()
        || world.entity(second).contains::<WorldObserver>()
    {
        return true;
    }
    [(first, second), (second, first)]
        .into_iter()
        .any(|(station, ship)| {
            parked_at(world, ship, station) || station_dock(world, ship) == Some(station)
        })
}

/// Spawn frozen `bodies` under `root`, each where and as it froze, with the
/// geometry the cell's worker prepared for it. Each thawed body carries
/// [`ThawedAfter`]: the play seconds between its freeze and `clock`, for its
/// owner's catch-up rule. A held ship or station regenerates from its entry
/// unchanged and carries none. A station's docked ships spawn beside it and
/// dock again in the same command flush.
pub fn thaw_record(
    commands: &mut Commands,
    root: Entity,
    bodies: Vec<PreparedFrozenBody>,
    clock: &WorldClock,
) {
    for prepared in bodies {
        let (body, geometry) = match prepared {
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
        thaw_body(commands, root, body, geometry, clock);
    }
}

/// Spawn one frozen body under `root`, and a station's docked ships beside
/// it, docked again by [`restore_station_docks`] in the same command flush.
fn thaw_body(
    commands: &mut Commands,
    root: Entity,
    frozen: FrozenBody,
    geometry: Option<PreparedAsteroid>,
    clock: &WorldClock,
) -> Entity {
    let FrozenBody {
        id,
        name,
        transform,
        visibility,
        motion,
        body,
        frozen_at,
    } = frozen;
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
    let mut docked = Vec::new();
    match body {
        FrozenBodyType::Asteroid(rock) => {
            let geometry =
                geometry.expect("PreparedFrozenBody::prepare pairs every rock with geometry");
            thaw_asteroid(&mut entity, rock, geometry);
        }
        FrozenBodyType::Ship(ship) => thaw_ship(&mut entity, *ship),
        FrozenBodyType::Station(station, ships) => {
            thaw_station(&mut entity, *station);
            docked = ships;
        }
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
    let station = entity.id();
    if !docked.is_empty() {
        let docks: Vec<_> = docked
            .into_iter()
            .map(|FrozenDockedShip { body, dock }| {
                assert!(
                    matches!(body.body, FrozenBodyType::Ship(_)),
                    "nova_world: a station record holds a docked body that is no ship"
                );
                (thaw_body(commands, root, body, None, clock), dock)
            })
            .collect();
        // Queued after both hulls' spawns: each spawn's section observer
        // applies in the flush after that spawn, so every port is live here,
        // and no system - a parked-hull capture or a physics step - runs
        // between the spawns and the dock.
        commands.queue(move |world: &mut World| restore_station_docks(world, station, docks));
    }
    station
}

/// Dock the ships thawed beside `station` again, each with its frozen
/// connection: the same ports, helm and joint frames, through
/// `restore_docking_connection`, never a new capture. It fires no
/// `OnDocked`: the scenario tracker echoes the dock as already reported.
///
/// # Panics
///
/// On a frozen port that is no live, free docking port of its hull, and on
/// a docked ship that is docked already. A Load checks the record before it
/// thaws; a thaw of a record this session froze cannot fail them.
fn restore_station_docks(
    world: &mut World,
    station: Entity,
    docks: Vec<(Entity, FrozenStationDock)>,
) {
    for (ship, dock) in docks {
        let station_port = live_port(world, station, &dock.station_port);
        let ship_port = live_port(world, ship, &dock.ship_port);
        let helm = match dock.helm {
            FrozenDockHelm::Neutral => DockedHelmType::Neutral,
            FrozenDockHelm::Station => DockedHelmType::Station(station),
        };
        let ends = [
            (station, station_port, dock.station_frame.frame()),
            (ship, ship_port, dock.ship_frame.frame()),
        ];
        let [first, second] = if dock.station_first {
            ends
        } else {
            [ends[1], ends[0]]
        };
        let connection = restore_docking_connection(
            world,
            DockingConnection {
                first_ship: first.0,
                first_section: first.1,
                second_ship: second.0,
                second_section: second.1,
                helm,
                measurement_fault: false,
            },
            [first.2, second.2],
        );
        echo_restored_dock(world, connection);
    }
}

/// The one section of `hull` with the id `id`.
///
/// # Panics
///
/// When `hull` holds no section, or more than one, with that id.
fn live_port(world: &mut World, hull: Entity, id: &EntityId) -> Entity {
    let matches: Vec<Entity> = world
        .query::<(Entity, &EntityId, &ChildOf)>()
        .iter(world)
        .filter(|(_, live, parent)| live.0 == id.0 && parent.parent() == hull)
        .map(|(entity, ..)| entity)
        .collect();
    let [port] = matches[..] else {
        panic!(
            "nova_world: '{}' holds {} sections with the frozen dock port id '{}'",
            body_label(world, hull),
            matches.len(),
            id.0
        );
    };
    port
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
/// anything but a sector root belongs to that parent. A ship docked at a
/// station ([`station_dock`]) takes its station's parent, wherever its own
/// pose stands, and freezes with the station.
///
/// A transient (a projectile, a chunk of debris) is not adopted. One that is
/// top-level outside the window is despawned.
///
/// # Panics
///
/// Through [`live_sectors`] on a duplicate root, through `freeze_body` on
/// a docked ship bound for an off-window cell whose dock no station record
/// keeps, and on a body that stays unsettled for too many advancing frames.
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
    // A ship docked at a station belongs to the station's cell, wherever its
    // own pose stands: the pair freezes as one record.
    let (visitors, bodies): (Vec<_>, Vec<_>) = bodies
        .into_iter()
        .partition(|(entity, ..)| station_dock(world, *entity).is_some());
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
                    despawn_frozen(world, entity);
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

    for (ship, ..) in visitors {
        // Despawned with its station, which froze off-window.
        if world.get_entity(ship).is_err() {
            continue;
        }
        let station = station_dock(world, ship).expect("adoption moves no docked ship");
        let (own, wanted) = (
            world.get::<ChildOf>(ship).map(ChildOf::parent),
            world.get::<ChildOf>(station).map(ChildOf::parent),
        );
        match wanted {
            Some(root) if own != wanted => {
                trace!("nova_world: {ship} follows its station into {root}");
                world.entity_mut(ship).insert(ChildOf(root));
            }
            None if own.is_some() => {
                trace!("nova_world: {ship} waits top-level with its station");
                world.entity_mut(ship).remove::<ChildOf>();
            }
            _ => {}
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
    /// Two live hulls are docked and neither is the observer, a station's
    /// parked hull nor a ship that freezes inside its station's record
    /// (`station_dock`), so no save keeps their dock; the save refuses rather
    /// than drop it.
    UnsavedDock {
        /// The first ship's name, or its entity.
        first: String,
        /// The second ship's name, or its entity.
        second: String,
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
            Self::UnsavedDock { first, second } => write!(
                f,
                "'{first}' is docked with '{second}', and a save keeps only the player's dock \
                 and a station's docks"
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
/// A saved dock stays docked ([`DockPolicy::Copy`]): the player's dock is
/// saved separately beside the [`WorldObserver`] and rebuilt on Load, and a
/// station's parked hulls and the ships [`station_dock`] names freeze inside
/// the station's record. Any other dock cannot be kept, so the save refuses
/// rather than drop it. The
/// [`WorldObserver`] and the player's fleet ([`PlayerFleetMarker`]) are not
/// part of it: the world streams around the observer, the fleet travels with
/// it, and their owner saves both.
///
/// # Errors
///
/// [`SectorSnapshotError::Unsettled`] while any body is mid-process,
/// [`SectorSnapshotError::UnownedBody`] for a persistent body no record could
/// hold, and [`SectorSnapshotError::UnsavedDock`] for a dock no save can
/// keep.
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

    let mut connections = world.query::<&DockingConnection>();
    if let Some(connection) = connections
        .iter(world)
        .find(|connection| !kept_dock(world, connection))
    {
        return Err(SectorSnapshotError::UnsavedDock {
            first: label(world, connection.first_ship),
            second: label(world, connection.second_ship),
        });
    }

    let mut ledger = world.resource::<FrozenSectors>().clone();
    let roots = live_sectors(world.query::<(Entity, &SectorRoot)>().iter(world));
    for (coord, root) in roots {
        let (frozen, unsettled) = freeze_sector_bodies(world, coord, root, DockPolicy::Copy);
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
        // It freezes inside its station's record.
        if station_dock(world, entity).is_some() {
            continue;
        }
        // Engine boundary: a pose counts world units, a cell meters.
        let coord = SectorCoord::containing(Meters3::from_engine(translation), edge);
        match freeze_body(
            world,
            entity,
            DockPolicy::Copy,
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
