//! The open save of one world: when it saves, the snapshot, and the writer.
//!
//! A save is wanted on the frame the world first arms with its player, on
//! every sector crossing, and when the player leaves. One write runs at a
//! time, off the main thread; the newest request waits behind it and is
//! snapshotted on the first frame the writer is idle. The snapshot is the
//! whole world in one frame: the ledger, every live sector, the player and
//! the canister counter.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::{
    ecs::{schedule::ScheduleConfigs, system::ScheduleSystem},
    prelude::*,
    tasks::{block_on, poll_once, IoTaskPool, Task},
};
use nova_assets::prelude::LoadedSectionPacks;
use nova_gameplay::prelude::{
    CargoCanisterIdAllocator, CargoCanisterRuntimeId, PlayerSpaceshipMarker, ShipCredits,
    UnsettledBody,
};
use nova_scenario::prelude::{freeze_ship, CurrentScenario, ResumedSpaceship};
use nova_ship::prelude::{CameraView, ResumedCameraView};
use nova_world::prelude::{
    snapshot_sectors, CurrentSector, FrozenSectors, SectorSnapshotError, WorldConfig,
};

use super::{
    check_saved_ids,
    transients::{freeze_transients, hold_resumed_transients},
    write_world, SavedIdFault, SavedPlayer, WorldFolder, WorldLock, WorldSaveHeader,
    WorldSaveState, WORLD_SAVE_FORMAT,
};
use crate::{NovaLayeredWorld, OpenWorldSession};

/// How many frames on which virtual time advanced a wanted save waits for
/// its bodies to settle before it fails. The bound `nova_world` gives a
/// retiring cell's unsettled bodies.
const SAVE_SETTLING_FRAMES_MAX: u32 = 600;

/// Why a save is wanted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveReason {
    /// The world first armed, or the player crossed into another sector.
    Crossing,
    /// The player is leaving the world.
    Leave,
}

/// What the last save did, for the status line and the leave overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldSaveStatus {
    /// A created world that has no save yet.
    Unsaved,
    /// The save of this generation is on disk.
    Saved {
        /// The generation written.
        generation: u64,
    },
    /// A write is running.
    Writing,
    /// The last save failed. The last good save is on disk unchanged.
    Failed(String),
    /// A wanted save waits for a body to settle.
    Waiting(String),
}

/// The world open in this game, its lock, and its save in progress.
///
/// Inserted by Create or Load before the open-world scenario loads. The menu
/// removes it when the player leaves, after the leave save or the player's
/// consent to leave without one.
///
/// One session saves one run of the world. Once the world it armed with
/// disarms (the player died, or the scenario ended) or arms again (a reload
/// from the seed), the session is spent and never writes again: what streams
/// after that is not the saved world, and saving it would overwrite the last
/// good save. Only a new session, from Create or Load, saves again.
#[derive(Resource, Debug)]
pub struct WorldSaveSession {
    folder: WorldFolder,
    /// Held, never read: dropping the session releases the world.
    _lock: WorldLock,
    name: String,
    seed: u32,
    /// The generation of the last good save, zero before the first.
    generation: u64,
    writer: Option<Task<Result<u64, String>>>,
    wanted: Option<SaveReason>,
    /// Frames on which virtual time advanced while `wanted` waited for a body
    /// to settle.
    settling_frames: u32,
    status: WorldSaveStatus,
    /// The world armed with this session's run.
    armed: bool,
    /// That run ended; nothing is written again.
    spent: bool,
}

impl WorldSaveSession {
    /// The session of a world Create just made: nothing is saved yet, and the
    /// first armed frame saves it.
    pub fn created(folder: WorldFolder, lock: WorldLock, name: String, seed: u32) -> Self {
        Self::new(folder, lock, name, seed, 0, WorldSaveStatus::Unsaved)
    }

    /// The session of a world Load opened at `header`'s generation.
    pub fn opened(folder: WorldFolder, lock: WorldLock, header: &WorldSaveHeader) -> Self {
        let status = WorldSaveStatus::Saved {
            generation: header.generation,
        };
        let name = header.name.clone();
        Self::new(folder, lock, name, header.seed, header.generation, status)
    }

    fn new(
        folder: WorldFolder,
        lock: WorldLock,
        name: String,
        seed: u32,
        generation: u64,
        status: WorldSaveStatus,
    ) -> Self {
        Self {
            folder,
            _lock: lock,
            name,
            seed,
            generation,
            writer: None,
            wanted: None,
            settling_frames: 0,
            status,
            armed: false,
            spent: false,
        }
    }

    /// What the last save did.
    pub fn status(&self) -> &WorldSaveStatus {
        &self.status
    }

    /// The world folder.
    pub fn folder(&self) -> &WorldFolder {
        &self.folder
    }

    /// The name the player gave the world.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Want the leave save. It replaces any save still waiting.
    pub fn request_leave(&mut self) {
        self.wanted = Some(SaveReason::Leave);
        self.settling_frames = 0;
        // A save from before the leave is not the leave save.
        if !matches!(self.status, WorldSaveStatus::Writing) {
            self.status = WorldSaveStatus::Waiting("saving before leaving".to_string());
        }
    }

    /// Never write again. A wanted save is dropped; a write in flight still
    /// finishes. "Load last save" calls it, so nothing is written before the
    /// world reopens from disk.
    pub fn stop_saving(&mut self) {
        self.spent = true;
        self.wanted = None;
        self.settling_frames = 0;
    }

    /// A write is in flight.
    pub fn is_writing(&self) -> bool {
        self.writer.is_some()
    }

    /// No write runs and no save is wanted. Idle is not saved: after
    /// [`Self::request_leave`], the leave save is on disk only when the
    /// session is idle AND the status is [`WorldSaveStatus::Saved`]. A leave
    /// save that cannot be taken ends [`WorldSaveStatus::Failed`].
    pub fn is_idle(&self) -> bool {
        self.writer.is_none() && self.wanted.is_none()
    }

    /// Drop the wanted save. A dropped leave save is a failure the player
    /// must see; a dropped crossing save keeps the status it had.
    fn drop_wanted(&mut self, reason: &str) {
        if self.wanted.take() == Some(SaveReason::Leave) {
            self.status = WorldSaveStatus::Failed(reason.to_string());
        }
        self.settling_frames = 0;
    }
}

/// The saved ledger a Load seeds the world with, taken on the frame the
/// world arms.
#[derive(Resource, Debug)]
pub struct ResumedWorld {
    /// The ledger the save holds.
    pub sectors: FrozenSectors,
}

/// Seed `world` with everything a Load resumes: the session, the saved
/// ledger, the player ship the scenario's player spawn thaws, the camera view
/// its controller opens at, the seed, the canister counter past every saved
/// id, and the saved transients, with the clocks held until they are back.
///
/// An armed world is disarmed first. The saved ledger is restored only on
/// the frame the world arms, and "Load last save" resumes over a live world
/// whose config would otherwise stay armed through the reload.
pub fn resume_world(
    world: &mut World,
    folder: WorldFolder,
    lock: WorldLock,
    header: &WorldSaveHeader,
    state: WorldSaveState,
) {
    world.remove_resource::<WorldConfig<NovaLayeredWorld>>();
    let WorldSaveState {
        player,
        sectors,
        canister_ids_next,
        transients,
        ..
    } = state;
    world.insert_resource(WorldSaveSession::opened(folder, lock, header));
    world.insert_resource(ResumedWorld { sectors });
    world.insert_resource(ResumedCameraView {
        view: player.view,
        steer: player.transform.rotation * player.view.steer_from_ship,
    });
    world.insert_resource(ResumedSpaceship {
        id: player.id,
        transform: player.transform,
        motion: player.motion,
        ship: player.ship,
    });
    world.insert_resource(OpenWorldSession { seed: header.seed });
    world
        .resource_mut::<CargoCanisterIdAllocator>()
        .resume_after(CargoCanisterRuntimeId(canister_ids_next));
    hold_resumed_transients(world, transients);
}

/// The save systems in their order: the request, then the writer poll, so a
/// waiting save is snapshotted on the frame the last write finishes. They
/// run while a session is open.
pub(crate) fn save_systems() -> ScheduleConfigs<ScheduleSystem> {
    (request_world_save, poll_world_writer, snapshot_world)
        .chain()
        .run_if(resource_exists::<WorldSaveSession>)
}

/// Seed the ledger `Cleanup` just cleared with the save's, on the frame the
/// world arms.
///
/// # Panics
///
/// When a [`ResumedWorld`] is still there on a frame the world was already
/// armed: it came too late, and the sectors already streamed from the seed.
pub(crate) fn restore_resumed_world(world: &mut World) {
    let Some(config) = world.get_resource_ref::<WorldConfig<NovaLayeredWorld>>() else {
        return;
    };
    assert!(
        config.is_added(),
        "nova_world_base: a ResumedWorld was inserted after the open world armed; a Load must \
         insert it before the scenario loads"
    );
    let ResumedWorld { sectors } = world
        .remove_resource::<ResumedWorld>()
        .expect("run only with a ResumedWorld");
    world.resource_mut::<FrozenSectors>().restore(sectors);
}

/// Want a save when the world first arms and when the player crosses into
/// another sector, and spend the session when its run ends.
///
/// A run ends when the config it armed with is removed or replaced, and when
/// the scenario reloads under it: the reload clears the ledger and the world
/// streams from the seed again, with the same config.
pub(crate) fn request_world_save(
    config: Option<Res<WorldConfig<NovaLayeredWorld>>>,
    scenario: Option<Res<CurrentScenario>>,
    current: Option<Res<CurrentSector>>,
    mut session: ResMut<WorldSaveSession>,
) {
    if session.armed && scenario.is_some_and(|scenario| scenario.is_changed()) {
        session.spent = true;
    }
    match config {
        Some(config) if session.armed && config.is_changed() => session.spent = true,
        Some(_) => session.armed = true,
        None if session.armed => session.spent = true,
        None => {}
    }
    if session.spent {
        return;
    }
    if current.is_some_and(|current| current.is_changed()) && session.wanted.is_none() {
        session.wanted = Some(SaveReason::Crossing);
    }
}

/// Take the snapshot of the wanted save and start its writer, when no writer
/// runs.
///
/// A spent session, no player ship (dead, or not spawned yet) or no armed
/// world drops the request: there is nothing to save, and the last good save
/// stays. A body that cannot freeze yet holds the request to a later frame,
/// for at most [`SAVE_SETTLING_FRAMES_MAX`] frames on which virtual time
/// advanced.
pub(crate) fn snapshot_world(world: &mut World) {
    // The chain's run condition is read once, before the request. A leave
    // that ends the session queues its removal, and the command flush before
    // this exclusive system can apply it in between.
    let Some(session) = world.get_resource::<WorldSaveSession>() else {
        return;
    };
    if session.writer.is_some() || session.wanted.is_none() {
        return;
    }
    if session.spent {
        world
            .resource_mut::<WorldSaveSession>()
            .drop_wanted("the world ended; the last save is kept");
        return;
    }
    let players: Vec<Entity> = world
        .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .iter(world)
        .collect();
    let ([player], true) = (
        players.as_slice(),
        world.contains_resource::<WorldConfig<NovaLayeredWorld>>(),
    ) else {
        world
            .resource_mut::<WorldSaveSession>()
            .drop_wanted("no player ship in an open world to save; the last save is kept");
        return;
    };
    let player = *player;
    // The camera first: a camera that is not ready waits before the sectors
    // are copied.
    let snapshot = CameraView::capture(world)
        .ok_or_else(|| SectorSnapshotError::Unsettled {
            label: "the player camera".to_string(),
            coord: world.resource::<CurrentSector>().0,
            reason: UnsettledBody {
                reason: "it is not ready",
            },
        })
        .and_then(|view| {
            let sectors = snapshot_sectors::<NovaLayeredWorld>(world)?;
            let transients = freeze_transients(world)?;
            freeze_ship(world, player)
                .map(|ship| (sectors, transients, ship, view))
                .map_err(|reason| SectorSnapshotError::Unsettled {
                    label: "the player ship".to_string(),
                    coord: world.resource::<CurrentSector>().0,
                    reason,
                })
        });
    let (sectors, transients, ship, view) = match snapshot {
        Ok(snapshot) => snapshot,
        Err(unsettled @ SectorSnapshotError::Unsettled { .. }) => {
            let advanced = world.resource::<Time<Virtual>>().delta() > Duration::ZERO;
            let mut session = world.resource_mut::<WorldSaveSession>();
            session.settling_frames += u32::from(advanced);
            if session.settling_frames > SAVE_SETTLING_FRAMES_MAX {
                let reason = format!("{unsettled}; it did not settle");
                session.wanted = None;
                session.settling_frames = 0;
                session.status = WorldSaveStatus::Failed(reason);
            } else {
                session.status = WorldSaveStatus::Waiting(unsettled.to_string());
            }
            return;
        }
        Err(
            unowned @ (SectorSnapshotError::UnownedBody { .. }
            | SectorSnapshotError::NoDurableId { .. }
            | SectorSnapshotError::DuplicateId { .. }
            | SectorSnapshotError::InvalidSavedState { .. }),
        ) => {
            let mut session = world.resource_mut::<WorldSaveSession>();
            session.wanted = None;
            session.settling_frames = 0;
            session.status = WorldSaveStatus::Failed(unowned.to_string());
            return;
        }
    };

    let body = world.entity(player);
    let mut transform = *body
        .get::<Transform>()
        .expect("a player ship has a transform");
    // The physics pose, not the interpolated transform that trails it.
    if let Some((position, rotation)) = body.get::<Position>().zip(body.get::<Rotation>()) {
        transform.translation = position.0;
        transform.rotation = rotation.0;
    }
    let motion = (
        body.get::<LinearVelocity>()
            .map_or(Vec3::ZERO, |linear| linear.0),
        body.get::<AngularVelocity>()
            .map_or(Vec3::ZERO, |angular| angular.0),
    );
    let credits = body
        .get::<ShipCredits>()
        .expect("a scenario ship has credits")
        .0;
    let id = body
        .get::<nova_events::prelude::EntityId>()
        .cloned()
        .expect("the open world's player ship is a scenario object with an id");
    let catalog = world
        .resource::<WorldConfig<NovaLayeredWorld>>()
        .generator
        .catalog()
        .0;
    let mods = world
        .resource::<LoadedSectionPacks>()
        .packs
        .iter()
        .map(|pack| pack.id.clone())
        .collect();
    let canister_ids_next = world
        .resource::<CargoCanisterIdAllocator>()
        .next_unminted()
        .0;
    let player_sector = world.resource::<CurrentSector>().0;
    let saved_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());

    let mut session = world.resource_mut::<WorldSaveSession>();
    let generation = session.generation + 1;
    let header = WorldSaveHeader {
        format: WORLD_SAVE_FORMAT,
        name: session.name.clone(),
        seed: session.seed,
        catalog,
        mods,
        game_version: env!("CARGO_PKG_VERSION").to_string(),
        saved_at_unix,
        generation,
        player_sector,
        credits,
    };
    let state = WorldSaveState {
        format: WORLD_SAVE_FORMAT,
        generation,
        player: SavedPlayer {
            id,
            transform,
            motion,
            ship,
            view,
        },
        sectors,
        canister_ids_next,
        transients,
    };
    if let Err(fault) = check_saved_ids(&state) {
        let error = match fault {
            SavedIdFault::Duplicate(id) => SectorSnapshotError::DuplicateId { id },
            fault => SectorSnapshotError::InvalidSavedState {
                reason: fault.to_string(),
            },
        };
        session.wanted = None;
        session.settling_frames = 0;
        session.status = WorldSaveStatus::Failed(error.to_string());
        return;
    }
    let folder = session.folder.clone();
    session.writer = Some(
        IoTaskPool::get()
            .spawn(async move { write_world(&folder, &header, &state).map(|()| generation) }),
    );
    session.wanted = None;
    session.settling_frames = 0;
    session.status = WorldSaveStatus::Writing;
}

/// Record what a finished writer did.
pub(crate) fn poll_world_writer(mut session: ResMut<WorldSaveSession>) {
    let Some(writer) = session.writer.as_mut() else {
        return;
    };
    let Some(written) = block_on(poll_once(writer)) else {
        return;
    };
    session.writer = None;
    match written {
        Ok(generation) => {
            session.generation = generation;
            session.status = WorldSaveStatus::Saved { generation };
        }
        Err(reason) => {
            error!("nova_world_base: the world save failed: {reason}");
            session.status = WorldSaveStatus::Failed(reason);
        }
    }
}
