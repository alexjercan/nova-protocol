//! The saved world on disk: its format, its folder, its lock, and the
//! refusals a folder can give.
//!
//! One world is one folder under the worlds root, named by its slug:
//!
//! - `world.ron`: the [`WorldSaveHeader`]. Small, so the Load list reads only
//!   this file.
//! - `state.<generation>.ron`: the [`WorldSaveState`] the header names.
//! - `world.lock`: held by the one game that has the world open.
//!
//! [`write_world`] writes the new state, then the header, then removes the old
//! state. Every file write is atomic, so a crash between two steps leaves the
//! old header naming an old state that still exists. [`open_world`] removes
//! the state and temp files the header does not name.
//!
//! The format is strict: an unknown field, a format other than
//! [`WORLD_SAVE_FORMAT`] or another catalog refuses the world. Nothing is
//! migrated. Native only: the web build has no saved worlds.

use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs::{File, OpenOptions, TryLockError},
    io::ErrorKind,
    path::{Path, PathBuf},
};

use bevy::prelude::*;
use nova_assets::{prelude::LoadedSectionPacks, storage::write_atomic};
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::{
    CargoCanisterRuntimeId, SavedBodyRef, SavedOwner, SavedSectionRef, SavedTargetRef,
};
use nova_scenario::prelude::FrozenShip;
use nova_ship::prelude::{CameraView, SavedTorpedoTarget};
use nova_world::prelude::{FrozenBody, FrozenBodyType, FrozenSectors, SectorCoord};
use serde::{Deserialize, Serialize};

mod session;
#[cfg(test)]
mod tests;
mod transients;

pub(crate) use session::{restore_resumed_world, save_systems};
pub use session::{resume_world, ResumedWorld, SaveReason, WorldSaveSession, WorldSaveStatus};
pub(crate) use transients::{restore_resumed_transients, ResumedTransients};
pub use transients::{
    FrozenTransient, FrozenTransientType, WorldResumeProgress, WorldResumeRefused,
    WORLD_RESUME_SECONDS_MAX,
};

/// The save layout this build reads and writes.
///
/// Bump it with any change to the save layout, or to what the generator
/// builds from a seed: an old save holds only the sectors the player changed
/// and regenerates the rest, so another generator would change it silently.
pub const WORLD_SAVE_FORMAT: u32 = 1;

/// The most characters a world name has, after trimming.
const WORLD_NAME_MAX: usize = 32;

/// The header file every world folder holds.
const HEADER_FILE: &str = "world.ron";

/// The lock file the game that has a world open holds.
const LOCK_FILE: &str = "world.lock";

/// The small part of a save: what the Load list shows and checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSaveHeader {
    /// [`WORLD_SAVE_FORMAT`] when it was written.
    pub format: u32,
    /// The name the player gave the world.
    pub name: String,
    /// The world seed every pristine sector is generated from.
    pub seed: u32,
    /// The digest of the catalog the world was saved with.
    pub catalog: u64,
    /// The mods of that catalog, in merge order, for the refusal text.
    pub mods: Vec<String>,
    /// The game version that wrote it. Shown, never checked.
    pub game_version: String,
    /// When it was written, in seconds since the Unix epoch.
    pub saved_at_unix: u64,
    /// The generation of the state file this header names.
    pub generation: u64,
    /// The sector the player was in.
    pub player_sector: SectorCoord,
    /// The player's credits.
    pub credits: u32,
}

/// The whole world as it was saved: the player and every changed sector.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSaveState {
    /// [`WORLD_SAVE_FORMAT`] when it was written.
    pub format: u32,
    /// The generation the header names this state by.
    pub generation: u64,
    /// The player ship.
    pub player: SavedPlayer,
    /// The frozen ledger with every live sector frozen into it.
    pub sectors: FrozenSectors,
    /// The first cargo canister id the world had not minted. Every saved
    /// canister id is below it.
    pub canister_ids_next: u64,
    /// Every combat transient in flight in the window.
    pub transients: Vec<FrozenTransient>,
}

/// The player ship as it was saved.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedPlayer {
    /// The scenario id the player ship is spawned by.
    pub id: EntityId,
    /// Its pose, in engine units.
    pub transform: Transform,
    /// Its linear and angular velocity, in engine units.
    pub motion: (Vec3, Vec3),
    /// The ship itself.
    pub ship: FrozenShip,
    /// The player's camera, relative to the ship.
    pub view: CameraView,
}

/// One world folder under the worlds root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldFolder {
    /// The folder.
    pub path: PathBuf,
    /// The folder name: the lowercase form of the world name.
    pub slug: String,
}

/// The lock on one world folder, held while the world is open. Dropping it
/// releases the world.
#[derive(Debug)]
pub struct WorldLock(
    #[expect(
        dead_code,
        reason = "held for its lock, never read; dropping it unlocks"
    )]
    File,
);

/// One folder the Load list found, with its header or the reason it cannot
/// load.
#[derive(Debug)]
pub struct WorldListing {
    /// The folder.
    pub folder: WorldFolder,
    /// The header, or why the world cannot load.
    pub header: Result<WorldSaveHeader, WorldRefusal>,
}

/// Why a world cannot be created, listed or opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldRefusal {
    /// The name or folder name breaks the naming rule.
    InvalidName(String),
    /// A world with the same folder name exists.
    NameTaken,
    /// Another game has the world open.
    Locked,
    /// A save file is missing, corrupt, or names another generation.
    Unreadable(String),
    /// The save has another layout than this build.
    Format {
        /// The format the save was written with.
        found: u32,
    },
    /// The save was written with another catalog.
    Catalog {
        /// The digest it was saved with.
        saved: u64,
        /// The digest loaded now.
        loaded: u64,
        /// The mods it was saved with.
        saved_mods: Vec<String>,
    },
    /// A file operation failed.
    Io(String),
    /// The world was opened but did not come back, so the Load was refused.
    /// Nothing was written.
    Unrestored(String),
}

impl std::fmt::Display for WorldRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidName(reason) => f.write_str(reason),
            Self::NameTaken => f.write_str("a world with this name exists"),
            Self::Locked => f.write_str("open in another game"),
            Self::Unreadable(reason) => write!(f, "unreadable: {reason}"),
            Self::Format { found } => {
                write!(
                    f,
                    "save format {found}; this build reads {WORLD_SAVE_FORMAT}"
                )
            }
            Self::Catalog { saved_mods, .. } => {
                write!(f, "content changed; saved with {}", saved_mods.join(", "))
            }
            Self::Io(reason) | Self::Unrestored(reason) => f.write_str(reason),
        }
    }
}

/// Every world folder under `root`, in folder-name order, each with its
/// header checked against the `loaded` catalog.
///
/// A missing root is no worlds. A root that cannot be read is an error.
pub fn list_worlds(
    root: &Path,
    loaded: &LoadedSectionPacks,
) -> Result<Vec<WorldListing>, WorldRefusal> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(WorldRefusal::Io(format!(
                "cannot read {}: {e}",
                root.display()
            )))
        }
    };
    let mut listings = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|e| WorldRefusal::Io(format!("cannot read {}: {e}", root.display())))?;
        if !entry.path().is_dir() {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().into_owned();
        let header = check_slug(&slug).and_then(|()| read_header(&entry.path(), loaded));
        listings.push(WorldListing {
            folder: WorldFolder {
                path: entry.path(),
                slug,
            },
            header,
        });
    }
    listings.sort_by(|a, b| a.folder.slug.cmp(&b.folder.slug));
    Ok(listings)
}

/// Make the folder of a new world named `name` under `root` and lock it.
///
/// Refuses a name that breaks the rule and a folder name that exists, and
/// touches nothing then. The folder holds no save until the first
/// [`write_world`].
pub fn create_world(root: &Path, name: &str) -> Result<(WorldFolder, WorldLock), WorldRefusal> {
    let slug = world_slug(name)?;
    let io = |e: std::io::Error| WorldRefusal::Io(format!("cannot create world {slug}: {e}"));
    std::fs::create_dir_all(root).map_err(io)?;
    let path = root.join(&slug);
    match std::fs::create_dir(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::AlreadyExists => return Err(WorldRefusal::NameTaken),
        Err(e) => return Err(io(e)),
    }
    // The new folder's entry is only durable once the root is synced.
    #[cfg(unix)]
    File::open(root)
        .and_then(|dir| dir.sync_all())
        .map_err(io)?;
    let lock = lock_world(&path)?;
    Ok((WorldFolder { path, slug }, lock))
}

/// Lock the world `slug` under `root` and read its save.
///
/// Refuses a world another game has open, a header of another format or
/// catalog, and a state file that is missing, corrupt, of another generation
/// or holding a camera view or a transient that cannot open, and writes
/// nothing then. On success it removes the state and temp files the header
/// does not name.
pub fn open_world(
    root: &Path,
    slug: &str,
    loaded: &LoadedSectionPacks,
) -> Result<(WorldFolder, WorldLock, WorldSaveHeader, WorldSaveState), WorldRefusal> {
    check_slug(slug)?;
    let path = root.join(slug);
    let lock = lock_world(&path)?;
    let header = read_header(&path, loaded)?;
    let state_name = state_file(header.generation);
    let text = std::fs::read_to_string(path.join(&state_name))
        .map_err(|e| WorldRefusal::Unreadable(format!("{state_name}: {e}")))?;
    let state: WorldSaveState =
        ron::from_str(&text).map_err(|e| WorldRefusal::Unreadable(format!("{state_name}: {e}")))?;
    if state.format != header.format || state.generation != header.generation {
        return Err(WorldRefusal::Unreadable(format!(
            "{state_name} is format {} generation {}; {HEADER_FILE} names format {} generation {}",
            state.format, state.generation, header.format, header.generation
        )));
    }
    state
        .player
        .view
        .validate()
        .map_err(|e| WorldRefusal::Unreadable(format!("{state_name}: camera view: {e}")))?;
    for (index, transient) in state.transients.iter().enumerate() {
        transient.validate().map_err(|e| {
            WorldRefusal::Unreadable(format!("{state_name}: transient {index}: {e}"))
        })?;
    }
    check_saved_ids(&state)
        .map_err(|fault| WorldRefusal::Unreadable(format!("{state_name}: {fault}")))?;
    sweep_orphans(&path, &state_name);
    Ok((
        WorldFolder {
            path,
            slug: slug.to_string(),
        },
        lock,
        header,
        state,
    ))
}

/// Why the ids of a saved state cannot open.
#[derive(Debug)]
enum SavedIdFault {
    /// Two saved bodies have this id: the player and a ledger body, or two
    /// bodies of different cells.
    Duplicate(EntityId),
    /// An id with a `/` that is not a minted wreck id
    /// `<base>/wreck/<section>`, repeated for a wreck of a wreck.
    Malformed(EntityId),
    /// Two saved canisters have this id.
    DuplicateCanister(CargoCanisterRuntimeId),
    /// A saved canister id at or past the saved next canister id: a resumed
    /// allocator would mint it again.
    CanisterUnminted {
        /// The saved canister's id.
        id: CargoCanisterRuntimeId,
        /// The saved next canister id.
        next: u64,
    },
    /// A saved reference that names nothing the save keeps.
    Dangling(String),
}

impl std::fmt::Display for SavedIdFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Duplicate(id) => write!(f, "two saved bodies have the id '{}'", id.0),
            Self::Malformed(id) => write!(
                f,
                "the id '{}' has a '/' but is not a minted wreck id",
                id.0
            ),
            Self::DuplicateCanister(id) => {
                write!(f, "two saved canisters have the id {}", id.0)
            }
            Self::CanisterUnminted { id, next } => write!(
                f,
                "the saved canister {} was never minted: the next canister id is {next}",
                id.0
            ),
            Self::Dangling(reason) => f.write_str(reason),
        }
    }
}

/// Every saved body id is well formed and names one body, every saved
/// canister id names one canister below the saved next canister id, every
/// saved reference names a body, bay or canister the save keeps, and a
/// torpedo that tracks a transient names another saved transient.
///
/// The ledger refuses a duplicate inside one cell when it freezes; this also
/// covers ids across cells and the player. A save checks before it writes,
/// and a Load checks before it opens.
fn check_saved_ids(state: &WorldSaveState) -> Result<(), SavedIdFault> {
    let mut ids = BTreeSet::new();
    let mut ships: BTreeMap<&str, &FrozenShip> =
        BTreeMap::from([(state.player.id.0.as_str(), &state.player.ship)]);
    let mut canisters = HashSet::new();
    let bodies: Vec<&FrozenBody> = state
        .sectors
        .iter()
        .flat_map(|(_, record)| record.bodies().iter())
        .collect();
    for body in &bodies {
        match (body.id(), body.body()) {
            (Some(id), FrozenBodyType::Ship(ship)) => {
                ships.insert(id, ship);
            }
            (_, FrozenBodyType::Canister(canister)) => {
                let id = canister.id();
                if id.0 >= state.canister_ids_next {
                    return Err(SavedIdFault::CanisterUnminted {
                        id,
                        next: state.canister_ids_next,
                    });
                }
                if !canisters.insert(id) {
                    return Err(SavedIdFault::DuplicateCanister(id));
                }
            }
            _ => {}
        }
    }
    let ids_saved = std::iter::once(state.player.id.0.as_str())
        .chain(bodies.iter().filter_map(|body| body.id()));
    for id in ids_saved {
        if id.contains('/')
            && id
                .split("/wreck/")
                .any(|part| part.is_empty() || part.contains('/'))
        {
            return Err(SavedIdFault::Malformed(EntityId::new(id)));
        }
        if !ids.insert(id) {
            return Err(SavedIdFault::Duplicate(EntityId::new(id)));
        }
    }
    let dangling = |index: usize, what: String| {
        Err(SavedIdFault::Dangling(format!("transient {index}: {what}")))
    };
    for (index, transient) in state.transients.iter().enumerate() {
        let (owner, section, target) = match &transient.body {
            FrozenTransientType::Round(round) => (Some(&round.flight.owner), None, None),
            FrozenTransientType::Torpedo(torpedo) => (
                Some(&torpedo.owner),
                torpedo.section.as_ref(),
                match &torpedo.target {
                    SavedTorpedoTarget::Tracking { target, .. } => Some(target),
                    _ => None,
                },
            ),
            FrozenTransientType::ShedFixture(_)
            | FrozenTransientType::RockChunk(_)
            | FrozenTransientType::DetachedPiece(_) => (None, None, None),
        };
        // A shooter that died before the save is `Gone`, which names no body.
        if let Some(SavedOwner::Ship(id)) = owner {
            if !ids.contains(id.0.as_str()) {
                return dangling(index, format!("its owner '{}' is not saved", id.0));
            }
        }
        if let Some(SavedSectionRef { ship, section }) = section {
            if !ships
                .get(ship.0.as_str())
                .is_some_and(|frozen| frozen.has_section(&section.0))
            {
                return dangling(
                    index,
                    format!(
                        "its bay '{}' of the ship '{}' is not saved",
                        section.0, ship.0
                    ),
                );
            }
        }
        match target {
            None => {}
            Some(SavedTargetRef::Body(SavedBodyRef(id))) => {
                if !ids.contains(id.0.as_str()) {
                    return dangling(index, format!("its target '{}' is not saved", id.0));
                }
            }
            Some(SavedTargetRef::Canister(id)) => {
                if !canisters.contains(id) {
                    return dangling(index, format!("its target canister {} is not saved", id.0));
                }
            }
            Some(&SavedTargetRef::Transient(other))
                if other == index || other >= state.transients.len() =>
            {
                return dangling(
                    index,
                    format!(
                        "its target transient {other} is not another of the {} saved",
                        state.transients.len()
                    ),
                );
            }
            Some(SavedTargetRef::Transient(_)) => {}
        }
    }
    Ok(())
}

/// Write `state` and then `header` into `folder`, and remove the state the
/// previous generation wrote.
///
/// Run off the main thread: it encodes and syncs the whole world. On an error
/// the files of the last good save are unchanged. A leftover previous state
/// is not an error: the next [`open_world`] removes it.
///
/// # Panics
///
/// When `header` and `state` name different generations.
pub fn write_world(
    folder: &WorldFolder,
    header: &WorldSaveHeader,
    state: &WorldSaveState,
) -> Result<(), String> {
    assert_eq!(
        header.generation, state.generation,
        "write_world: header generation {} and state generation {} differ",
        header.generation, state.generation
    );
    let state_ron = ron::to_string(state).map_err(|e| format!("cannot encode the world: {e}"))?;
    let header_ron = ron::ser::to_string_pretty(header, ron::ser::PrettyConfig::default())
        .map_err(|e| format!("cannot encode the header: {e}"))?;
    let state_name = state_file(state.generation);
    write_atomic(&folder.path.join(&state_name), state_ron.as_bytes())
        .map_err(|e| format!("cannot write {state_name}: {e}"))?;
    write_atomic(&folder.path.join(HEADER_FILE), header_ron.as_bytes())
        .map_err(|e| format!("cannot write {HEADER_FILE}: {e}"))?;
    if let Some(previous) = state.generation.checked_sub(1) {
        let previous = folder.path.join(state_file(previous));
        match std::fs::remove_file(&previous) {
            Err(e) if e.kind() != ErrorKind::NotFound => {
                warn!("write_world: cannot remove {}: {e}", previous.display());
            }
            _ => {}
        }
    }
    Ok(())
}

/// The folder name of the world name `name`: trimmed, lowercase, spaces as
/// `-`.
///
/// # Errors
///
/// [`WorldRefusal::InvalidName`] with the reason, for a name that is empty
/// after trimming, longer than 32 characters, or holds a character other
/// than a letter, a digit, a space, `_` or `-`.
pub fn world_slug(name: &str) -> Result<String, WorldRefusal> {
    let name = name.trim();
    if name.is_empty() {
        return Err(WorldRefusal::InvalidName("name the world".to_string()));
    }
    if name.chars().count() > WORLD_NAME_MAX {
        return Err(WorldRefusal::InvalidName(format!(
            "a name has at most {WORLD_NAME_MAX} characters"
        )));
    }
    if let Some(bad) = name
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-')))
    {
        return Err(WorldRefusal::InvalidName(format!(
            "'{bad}' is not allowed; use letters, digits, space, _ and -"
        )));
    }
    Ok(name.to_ascii_lowercase().replace(' ', "-"))
}

/// Refuse a folder name no world name gives, so a slug cannot leave the root.
fn check_slug(slug: &str) -> Result<(), WorldRefusal> {
    match world_slug(slug) {
        Ok(same) if same == slug => Ok(()),
        _ => Err(WorldRefusal::InvalidName(format!(
            "'{slug}' is not a world folder name"
        ))),
    }
}

/// Lock the world folder `path`, making its lock file when it has none.
fn lock_world(path: &Path) -> Result<WorldLock, WorldRefusal> {
    let lock_path = path.join(LOCK_FILE);
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .map_err(|e| WorldRefusal::Io(format!("cannot open {}: {e}", lock_path.display())))?;
    match file.try_lock() {
        Ok(()) => Ok(WorldLock(file)),
        Err(TryLockError::WouldBlock) => Err(WorldRefusal::Locked),
        Err(TryLockError::Error(e)) => Err(WorldRefusal::Io(format!(
            "cannot lock {}: {e}",
            lock_path.display()
        ))),
    }
}

/// Only the format of a header, read before the whole header so a header of
/// another layout refuses as [`WorldRefusal::Format`], not as unreadable.
#[derive(Deserialize)]
struct HeaderFormat {
    format: u32,
}

/// Read the header of the world folder `path` and check it against the
/// `loaded` catalog.
fn read_header(path: &Path, loaded: &LoadedSectionPacks) -> Result<WorldSaveHeader, WorldRefusal> {
    let text = std::fs::read_to_string(path.join(HEADER_FILE))
        .map_err(|e| WorldRefusal::Unreadable(format!("{HEADER_FILE}: {e}")))?;
    let HeaderFormat { format } = ron::from_str(&text)
        .map_err(|e| WorldRefusal::Unreadable(format!("{HEADER_FILE}: {e}")))?;
    if format != WORLD_SAVE_FORMAT {
        return Err(WorldRefusal::Format { found: format });
    }
    let header: WorldSaveHeader = ron::from_str(&text)
        .map_err(|e| WorldRefusal::Unreadable(format!("{HEADER_FILE}: {e}")))?;
    if header.catalog != loaded.digest.0 {
        return Err(WorldRefusal::Catalog {
            saved: header.catalog,
            loaded: loaded.digest.0,
            saved_mods: header.mods,
        });
    }
    Ok(header)
}

/// The name of the state file of `generation`.
fn state_file(generation: u64) -> String {
    format!("state.{generation}.ron")
}

/// Remove the state files other than `kept` and the temp files an
/// interrupted write left in the locked world folder `path`.
fn sweep_orphans(path: &Path, kept: &str) {
    let Ok(entries) = std::fs::read_dir(path) else {
        warn!(
            "open_world: cannot read {} to remove old saves",
            path.display()
        );
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let state = name.starts_with("state.") && name.ends_with(".ron") && name != kept;
        let temp = name.starts_with('.') && name.ends_with(".tmp");
        if !(state || temp) {
            continue;
        }
        if let Err(e) = std::fs::remove_file(entry.path()) {
            warn!("open_world: cannot remove {}: {e}", entry.path().display());
        }
    }
}
