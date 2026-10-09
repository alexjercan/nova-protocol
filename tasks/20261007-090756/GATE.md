# Implementation gate: resumable New Game worlds

Base: `resumable-worlds` at 655100c29 (frozen-sectors PR #110). Read-only
discovery only. No code is changed. Paths are relative to `crates/`.

## 1. End state

- New Game asks for a world name and a seed. Create makes a native world
  folder, locks it, and plays. Load lists the world folders, shows each
  refusal reason, and resumes the selected world.
- A resumed world restores the player ship (pose, motion, sections, damage,
  plates, ammo, reloads, hold, credits, bindings), every frozen off-window
  cell, and every live cell in the 125-cell window, as it was at the last good
  save. Unvisited cells generate from the seed, as now.
- The game takes a save on each sector crossing (`CurrentSector` change) and
  when the player leaves the world (Back to Menu, Exit, window close). It
  serializes and writes off the main thread. Leaving waits visibly for the
  writer. Crossing saves never block play.
- A write is atomic: new state file, fsync, rename, header commit, dir fsync.
  A failed or refused write never replaces the last good save, and it is
  visible.
- Death writes nothing. The last good save stays.

What dies:
- "Nothing is saved" text and the session-only promise:
  `nova_menu/src/world_setup.rs:1-7,124-128`, `nova_world_base/src/lib.rs:27-31`,
  `nova_world/src/frozen.rs:12-14`, `docs/architecture.md:26`, and wiki pages.
- `FrozenAsteroid.collider`, `FrozenAsteroid.drawn_mesh`, `FrozenDrawnMesh`
  (`nova_scenario/src/objects/asteroid.rs:417-473,656-688`) and
  `FrozenFixture.collider` (`nova_ship/src/sections/frozen.rs:47-62`). Thaw
  rebuilds them, in session too. There is one path, not a disk path beside a
  memory path.
- bevy's own window-close exit (`WindowPlugin::close_when_requested`, default
  `true`). The game handles the close request.

What may break:
- A revisited carved rock now remeshes on the cell worker. Before, it reused
  the frozen handle. The cell needs more worker time, and no more main-thread
  time.
- The main-thread snapshot cost grows with the number of bodies in the 125
  live cells. This is unmeasured (section 10, P7).
- Examples, probes and lessons that insert `OpenWorldSession` directly stay
  session-only. Nothing writes for them.

What must fail loudly: see the matrix in section 9.

## 2. Owner decisions (options, consequence, recommendation)

D1 Docked at save time. `freeze_body` panics on `DockedShip`
(`nova_world/src/frozen.rs:361-366`). The player can dock to a streamed ship
(`nova_ship/src/sections/docking_section/connection.rs:176-249`).
- a. Save the pair and form the dock again on load with
  `DockingConnectionRequest`. This keeps the dock, but the request derives the
  anchors from the current transforms and admission can refuse it. There is
  no instant re-dock API.
- b. Save both ships at their poses with no dock. On load they stand
  port-to-port and undocked. All data is kept, the dock link is not, and the
  player docks again.
- c. Defer the save until undock. Leaving while docked is then blocked or
  loses progress.
- Recommend b. The changelog states the policy.

D2 Unsettled bodies at a leave save. The pause menu holds virtual time, and
nothing settles while it is held (`frozen.rs:142-160`).
- a. Behind the blocking overlay, release virtual time (player input off)
  until the snapshot succeeds, at most 600 advancing frames, the same bound
  as `SETTLING_FRAMES_MAX`. After that, show a visible failure.
- b. Leave with the last good save and report "not saved".
- Recommend a.

D3 Retry in a saved world (pause Retry and the death state).
- a. Retry loads the last good save from disk. Progress since that save is
  discarded, and nothing is written first.
- b. Retry restarts the world from the seed. This throws the saved world
  away.
- Recommend a, with the label "Load last save" for a saved world.

D4 Web.
- a. No saves on web. Load and the name field are hidden, New Game stays a
  session, and a menu line says that saves need the desktop build.
- b. localStorage. Carved rocks are up to 275 KB each, the quota is about
  5 MB, and multi-key commits are not atomic.
- Recommend a.

D5 Save root.
- a. `dirs::data_dir()/nova-protocol/worlds/`, or
  `$NOVA_CONFIG_ROOT/worlds/` when that override is set (tests and probes
  already set it, `nova_assets/src/storage.rs:103-130`).
- b. `config_dir/nova-protocol/worlds/`, next to the settings.
- Recommend a.

D6 Version refusal.
- a. Refuse a world on format version or catalog digest mismatch. One const,
  `WORLD_SAVE_FORMAT`, is bumped when the save layout OR the generator output
  for a seed changes. The game version is stored for display only.
- b. Also refuse on any game version change. Every release then refuses every
  world.
- Recommend a.

D7 Encoding.
- a. RON (already a dependency). f32 values round-trip exactly. A carved rock
  is about 3x its binary size.
- b. Add postcard. It is compact and binary, but it is a new dependency.
- Recommend a. Measure the size in P7.

D8 First write.
- a. Create writes the first save as soon as the world arms with its player,
  so a new world exists on disk before the first crossing.
- b. The first write happens at the first crossing or on leave. A crash
  before that leaves an empty, locked-name folder.
- Recommend a.

D9 World names. 1-32 chars from `[A-Za-z0-9 _-]`, trimmed. The folder is the
lowercase slug. A taken slug refuses Create, and nothing is overwritten. This
slice has no Delete: the player removes the folder by hand, and the docs say
so. Recommend as written.

## 3. Existing code (anchors)

| Concern | Path:line |
| --- | --- |
| Create modal, `on_create_world` inserts `OpenWorldSession`, `GameMode::NewGame`, `Playing` | `nova_menu/src/world_setup.rs:62-169,175-198,216-235` |
| Main menu buttons, Exit, New Game scenario pick | `nova_menu/src/menu_ui.rs:47-136,457-459,474` |
| Pause Retry / Back to Menu | `nova_menu/src/pause.rs:606-620,627-634` |
| Outcome Main Menu / Retry | `nova_menu/src/outcome.rs:39-199,204` |
| Settings flush on `AppExit` in `Last` (precedent, races the window close) | `nova_menu/src/settings_store.rs:415,625-643` |
| Window plugin, no `close_when_requested` override | `nova_core/src/lib.rs:698-737` |
| `OpenWorldSession`, `sync_open_world`, disarm on zero players (death) | `nova_world_base/src/lib.rs:123-132,146-158,186-273` |
| `NovaWorldSystems` Cleanup..Retire chain | `nova_world/src/lib.rs:737,832-871` |
| `CurrentSector`, `track_current_sector` (no crossing event) | `nova_world/src/streaming.rs:118,494-517` |
| `SectorJob::start` reads the record at request time | `nova_world/src/streaming.rs:543-575,728` |
| `retire_sectors`, `freeze_sector_bodies`, `clear_sector_work` | `nova_world/src/streaming.rs:937-1007,1013-1037,1080-1140` |
| `FrozenSectors`, `FrozenBody`, `freeze_body`, `thaw_record`, `adopt_moving_bodies` | `nova_world/src/frozen.rs:48-130,222-330,335-395,403-470,508-590` |
| `prepare_cell` prepares frozen rock geometry on the worker | `nova_world/src/generation.rs:854-890` |
| `FrozenAsteroid`, `freeze_asteroid`, `thaw_asteroid` | `nova_scenario/src/objects/asteroid.rs:417-473,482,603-700` |
| Carved surface and collider build | `nova_scenario/src/objects/asteroid_carve.rs:640-690` |
| `FrozenShip`, `ThawingShip`, `freeze_ship`, `thaw_ship`, `insert_spaceship_sections` | `nova_scenario/src/objects/spaceship.rs:523-549,560-610,675-710,736-1115` |
| Scenario object spawn (adds `ScenarioAddressableMarker`) | `nova_scenario/src/actions/spawn.rs:146-223` |
| Open-world bootstrap, player id `"player"` | `nova_authoring/src/base_content/scenarios/open_world.rs:3-66` |
| `FrozenSection`, `FrozenFixture`, `freeze_section` | `nova_ship/src/sections/frozen.rs:46-130,183-290` |
| `plate_collider(shape.volume())`, `decor_collider(size)` | `nova_ship/src/sections/shell_skin.rs:1144-1150`, `skin_decor.rs:614-621` |
| `CargoCanisterIdAllocator` (process lifetime, never reset) | `nova_gameplay/src/inventory.rs:623-648` |
| `AssetRef` serde refuses `Handle` | `nova_gameplay/src/asset_ref.rs:117-134` |
| `ContentCatalogDigest`, `LoadedSectionPacks`, `EnabledMods` | `nova_assets/src/merge.rs:655-700`, `mod_set.rs:69` |
| `write_atomic` (file fsync, no dir fsync), config root | `nova_assets/src/storage.rs:103-130,217-249` |

## 4. Proposed types and functions

These are new or changed. `serde` derives are mechanical and gated the same
way as the existing ones (`cfg_attr(feature = "serde", ...)`).

nova_world (`frozen.rs`):
```rust
#[derive(Resource, Default, Debug, Clone, Serialize, Deserialize)]
pub struct FrozenSectors(BTreeMap<SectorCoord, Arc<FrozenSector>>); // was by value
// take(): Arc::unwrap_or_clone; visit/arrive: Arc::make_mut (copy only if a save holds it)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrozenBody {
    id: Option<EntityId>,
    name: Option<String>,            // was Option<Name>
    transform: Transform,            // engine units; exact round trip
    visibility: Option<Visibility>,
    motion: Option<(Vec3, Vec3)>,    // was (LinearVelocity, AngularVelocity)
    body: FrozenBodyType,
}

/// The whole streamed world as it would freeze now, without despawning.
pub fn snapshot_sectors(world: &World) -> Result<FrozenSectors, SectorSnapshotError>;
pub enum SectorSnapshotError {
    /// Try again next frame.
    Unsettled { label: String, coord: SectorCoord, reason: UnsettledBody },
    /// Never omitted: the save refuses.
    UnownedBody { label: String },
}

impl FrozenSectors {
    /// Seed an empty ledger from a save. Panics when it is not empty.
    pub fn restore(&mut self, saved: FrozenSectors);
}
```
`snapshot_sectors` clones the ledger Arcs. For each live root it runs
`visit(coord, ...)` over its children (the `freeze_sector_bodies` filter). For
each top-level persistent body outside a root (not the observer, not
addressable) it runs `arrive(cell, ...)`. A docked ship is frozen undocked
(D1b) through `freeze_body_with(world, entity, DockPolicy)`, where
`freeze_body` = `DockPolicy::Refuse`.

nova_scenario:
```rust
// asteroid.rs: FrozenAsteroid loses `collider`, `drawn_mesh`; FrozenDrawnMesh deleted.
// thaw_asteroid takes geometry that is already carved when the rock was carved:
pub fn prepare_frozen_asteroid(rock: &FrozenAsteroid) -> PreparedAsteroid; // worker-side
/// The ship the open world's player spawn thaws instead of building fresh.
#[derive(Resource)]
pub struct ResumedSpaceship {
    pub id: EntityId,
    pub transform: Transform,
    pub motion: (Vec3, Vec3),
    pub ship: FrozenShip,
}
```
`spawn.rs` Spaceship arm: when `ResumedSpaceship.id` equals the object id,
take it and spawn `thaw_ship` with the saved pose and motion (keeping
`ScenarioAddressableMarker`). Otherwise spawn fresh, as now.
`sync_open_world` panics if the player arms while a `ResumedSpaceship` is
still unconsumed.

nova_ship: `FrozenFixture.collider` is deleted. A plate rebuilds it with
`plate_collider(shape.volume())`. `FrozenFixtureKind::Decor` gains
`collider_size: Vec3` and uses `decor_collider`.

nova_gameplay:
```rust
impl CargoCanisterIdAllocator {
    pub fn next_unminted(&self) -> CargoCanisterRuntimeId;
    /// Never mint at or below a saved world's ids. Takes the max.
    pub fn resume_after(&mut self, saved_next: CargoCanisterRuntimeId);
}
```

nova_assets: `write_atomic` also fsyncs the parent dir after the rename
(unix). `pub fn worlds_root() -> PathBuf` (D5).

nova_world_base (new `save/` module, the owner of the format and the session):
```rust
pub const WORLD_SAVE_FORMAT: u32 = 1;

#[derive(Serialize, Deserialize)] #[serde(deny_unknown_fields)]
pub struct WorldSaveHeader {
    pub format: u32, pub name: String, pub seed: u32,
    pub catalog: u64, pub mods: Vec<String>, pub game_version: String,
    pub saved_at_unix: u64, pub generation: u64,
    pub player_sector: SectorCoord, pub credits: u32,
}
#[derive(Serialize, Deserialize)] #[serde(deny_unknown_fields)]
pub struct WorldSaveState {
    pub format: u32, pub generation: u64,
    pub player: SavedPlayer, pub sectors: FrozenSectors,
    pub canister_ids_next: u64,
}
pub struct SavedPlayer { pub id: EntityId, pub transform: Transform, pub motion: (Vec3, Vec3), pub ship: FrozenShip }

pub struct WorldFolder { pub path: PathBuf, pub slug: String }
pub struct WorldLock(std::fs::File);          // File::try_lock, held for the session
pub struct WorldListing { pub folder: WorldFolder, pub header: Result<WorldSaveHeader, WorldRefusal> }
pub enum WorldRefusal { InvalidName(String), NameTaken, Locked, Unreadable(String),
    Format { found: u32 }, Catalog { saved: u64, loaded: u64, saved_mods: Vec<String> }, Io(String) }

pub fn list_worlds(root: &Path, loaded: &CatalogIdentity) -> Vec<WorldListing>;
pub fn create_world(root: &Path, name: &str) -> Result<(WorldFolder, WorldLock), WorldRefusal>;
pub fn open_world(root: &Path, slug: &str, loaded: &CatalogIdentity)
    -> Result<(WorldFolder, WorldLock, WorldSaveHeader, WorldSaveState), WorldRefusal>;
pub fn write_world(folder: &WorldFolder, header: &WorldSaveHeader, state: &WorldSaveState)
    -> Result<(), String>;                    // runs on IoTaskPool

#[derive(Resource)] pub struct WorldSaveSession { /* folder, lock, name, seed, generation,
    writer: Option<Task<Result<u64, String>>>, wanted: Option<SaveReason>, status: WorldSaveStatus */ }
pub enum SaveReason { Crossing, Leave }
pub enum WorldSaveStatus { Saved { generation: u64 }, Writing, Failed(String), Waiting(String) }
#[derive(Resource)] pub struct ResumedWorld { pub sectors: FrozenSectors }   // consumed once
```
Disk layout: `worlds/<slug>/world.ron` (the header, small, read by the list),
`worlds/<slug>/state.<generation>.ron`, and `worlds/<slug>/world.lock`.

Commit order for one write:
1. Write `state.<g+1>.ron` with `write_atomic`.
2. Write `world.ron` with `write_atomic` (generation `g+1`).
3. Remove `state.<g>.ron`.

A crash between steps leaves the old header pointing at a state file that
still exists. Open removes orphan `state.*.ron` files and `.*.tmp` files that
the header does not name, and nothing else.

nova_menu: Name field on the Create modal. A Load screen
(`list_detail_screen`, as Scenarios does). A save status line (in-game,
top-right). A leave overlay ("Saving world..."; on failure: Try again / Leave
without saving). A Retry label and handler for saved worlds (D3).
`on_window_close_requested` with `close_when_requested: false` in
`nova_core/src/lib.rs:711-736`.

## 5. Call graph

Before:
```
Create -> OpenWorldSession{seed} -> Playing -> LoadScenario(open_world) -> spawn player
Update: sync_open_world -> WorldConfig -> Cleanup -> Observe(CurrentSector) -> Request(SectorJob: record?)
        -> Collect -> Materialize(take record | generate) -> Adopt(freeze off-window) -> Retire(freeze, despawn)
Back to Menu / Exit / close -> state change / AppExit (nothing written)
```
After:
```
Create(name) -> create_world -> WorldSaveSession -> OpenWorldSession -> Playing -> LoadScenario
Load(slug)   -> open_world (refuse | header+state) -> WorldSaveSession + ResumedWorld + ResumedSpaceship
             -> allocator.resume_after -> OpenWorldSession -> Playing -> LoadScenario
                -> spawn.rs: player id matches ResumedSpaceship -> thaw_ship at saved pose
Update: sync_open_world -> Cleanup -> restore_resumed_world (FrozenSectors::restore) -> Observe -> ...Retire
        -> request_world_save (CurrentSector changed | first arm | leave)
        -> snapshot_world (exclusive): player alive & freeze_ship ok & snapshot_sectors ok
             -> IoTaskPool: RON + write_world      (Unsettled -> retry next frame)
        -> poll_world_writer -> WorldSaveStatus
Leave(menu|exit|close) -> pause input, release virtual time (D2) -> final snapshot -> wait writer (overlay)
        -> Saved: transition / AppExit, drop lock | Failed: Try again | Leave without saving
Death -> player gone -> no snapshot; disarm clears the world; Retry = open_world again (D3)
```

## 6. Save lifecycle and semantics

- Triggers:
  - First arm with the player (D8).
  - `CurrentSector` changed.
  - Leave.
  If a write is in flight, the request is kept, and only the newest one is
  snapshotted when the writer is idle (coalesce). The snapshot is taken in
  the frame the writer becomes idle.
- A snapshot is consistent: the ledger, the live cells, the player and the
  allocator, all in one exclusive system in one frame, after
  `NovaWorldSystems::Retire`.
- The player is excluded from the ledger (`WorldObserver`) and saved with
  `freeze_ship` plus the avian pose. Not saved, and reset on load: target
  lock, autopilot order, camera mode, HUD/TAB state, projectiles and other
  transients, and the scenario clock. These are listed in the docs.
- No player (death, or not spawned yet): no snapshot, and the request is
  dropped.
- Load seeding: `restore_resumed_world` runs in `Update` after
  `NovaWorldSystems::Cleanup` and before `Observe`, on the first armed frame.
  Cleanup clears the ledger on that same frame, because `CurrentScenario`
  changed (`streaming.rs:1080`, scout finding).

## 7. Cross-plugin ordering

- `restore_resumed_world`: `.after(NovaWorldSystems::Cleanup).before(NovaWorldSystems::Observe)`.
- `request_world_save`, `snapshot_world`, `poll_world_writer`: chained `.after(NovaWorldSystems::Retire)` in `Update`.
- All are owned by `NovaWorldBasePlugin`, which is already after `NovaScenarioPlugin` (`nova_core/src/lib.rs:421-430`).
- nova_menu reads `WorldSaveSession` and drives leave and transitions (menu is downstream of world_base).
- The window-close handler runs in `Update`, so `AppExit` is written before `Last`. This also closes the settings-flush race (`settings_store.rs:631`).

## 8. IDs, content, UI, docs, callers

- IDs: no new content IDs, and no generated RON changes. The player id
  `"player"` (`open_world.rs:29`) is the resume key. Canister ids resume past
  the saved counter.
- Callers to update:
  - `FrozenSectors` users in `nova_world/src/tests/frozen.rs`.
  - `examples/systems/system_world_sectors.rs`.
  - `FrozenAsteroid` and `FrozenFixture` field users.
  - `nova_menu/src/tests/world_setup.rs` (Create now needs a name).
  - `screenshot_menu.rs` and `loop_world_start.rs`, if they drive Create.
- Docs: `docs/architecture.md` (`nova_world`, `nova_world_base`, `nova_menu`
  rows), the `nova_world_base` crate doc, `frozen.rs` module doc, the
  `world_setup` doc, `web/src/wiki/interface.md` and the pages that say the
  world is not saved, plus a new wiki section "Saved worlds" (folder, refusals,
  what resets). Changelog entries under Gameplay & Flight, Interface & HUD,
  and Web & Platform (web has no saves).

## 9. Failure and refusal matrix

| Case | Behavior |
| --- | --- |
| Name empty or invalid | Create is disabled, with the reason in the modal |
| Name slug exists | Refused "a world named X exists"; nothing is touched |
| Folder or lock IO error at Create | Refused, with the error; no session starts |
| Header missing or unreadable | Listed "unreadable: <err>"; Load is disabled |
| `format != WORLD_SAVE_FORMAT` | Listed "save format N; this build reads M"; Load is disabled |
| Catalog digest mismatch | Listed "content changed"; shows the saved mods; Load is disabled |
| World locked by another game | Listed and refused "open in another game" |
| State file missing or corrupt, or its generation is not the header's | Load refuses with the reason; nothing is written |
| Crossing write fails | Status line "SAVE FAILED: <err>" stays; last good files untouched; the next trigger retries |
| Leave write fails | Overlay with the error: Try again / Leave without saving (explicit consent) |
| Body unsettled at snapshot | Wait; a leave releases virtual time (D2); more than 600 advancing frames is a visible failure |
| Persistent body owned by no root and not the player | Save refused, with the body named; never omitted |
| Docked pair | Saved undocked at its poses (D1b) |
| Crash or kill mid-write | Header plus the old state survive; orphans are swept at next open |
| Player dead | No write; last good save stays; Retry loads it |
| Exit while a writer is in flight | Overlay waits for it, then the final save |
| Window close in the menu or with no saved world | Exits at once, as now |
| Unknown field or enum in the file | serde error, so Unreadable (strict, `deny_unknown_fields`) |
| Hand-edited unknown design or kind with a matching digest | Existing `SectorFault` panic at materialize (loud) |

## 10. Proofs (proposed; each needs approval)

- P1 unit (nova_world_base): header refusals for format, catalog and mods;
  name and slug rules; create on a taken slug; second `open_world` refused
  while locked.
- P2 unit: `write_world` then `open_world` round-trips. A failed second write
  (read-only dir) leaves the first readable, and orphan states are swept.
- P3 ECS (nova_world): a live window with a carved rock, a looted ship, a
  canister, owed ore, and a top-level waiting body. Run `snapshot_sectors`,
  RON round trip, then `restore` in a fresh app. The re-serialized RON equals
  the first, and the materialized bodies match (carved collider rebuilt).
- P4 ECS (nova_scenario): a `ResumedSpaceship` thaws the player with
  `PlayerSpaceshipMarker`, `ScenarioAddressableMarker`, the saved pose,
  inventory, credits, section health and missing sections.
- P5 ECS: the allocator resumes past the saved counter; no reused id.
- P6 probe: New Game named, mine and loot, cross a sector, leave. A fresh app
  Loads it, and the state is asserted (canister ids, stock, carved volume,
  player pose and credits).
- P7 measurement: snapshot main-thread time and file size on the 125-cell
  window, matched repeats against `master` with no save. Reported, not
  asserted.
- P8 rendered: Create modal with name, Load list with one valid and one
  refused world, the status line, and the leave overlay (Xvfb, lavapipe
  frames inspected).

## 11. Slices

1. Format: serde derives, drop-and-derive colliders and meshes, Arc ledger,
   `snapshot_sectors`, `restore`, allocator. P3, P5.
2. Disk: `save/` format, list, create, open, write, lock, dir fsync. P1, P2.
3. Session: triggers, writer, resume seeding, player seam, death and Retry.
   P4, P6.
4. UI: Create name, Load screen, status line, leave overlay, window close,
   web gating. P8.
5. Docs, changelog, P7, and an independent review.
