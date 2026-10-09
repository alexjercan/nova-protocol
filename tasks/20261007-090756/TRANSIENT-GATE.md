# Gate: save and restore in-flight transients (rev 2)

Rev 2 answers the owner's refusal of rev 1:
- Detached section pieces are saved and restored (section 4.6), not
  excluded.
- A Load that cannot fill its window ends in a bounded, visible refusal
  (section 3).
- Owner and ref fidelity and real launch cues have their own proofs
  (P-T4b, P-T8).

Sources added since rev 1: SCOUT-PIECES.md and SCOUT-ART.md.

Status: proposal. Nothing is edited. Automode chose the policy (save and
restore live-window projectiles and detached physical parts with pose,
motion, ownership and remaining lifetime). That is not owner sign-off; this
gate asks for it.

Sources: SCOUT-TRANSIENTS.md, SCOUT-IDS.md, and the reads cited below.

## 1. End state

- A save holds every live-window combat transient that a Load can rebuild
  exactly. A Load rebuilds it with the same pose, velocity, owner, damage and
  remaining lifetime, and it expires on its own clock.
- A transient the save cannot rebuild exactly makes the save wait (bounded,
  then a visible failure) or is a declared exclusion that the owner approves
  here. Nothing is partly restored. Nothing is dropped without a decision.
- Off-window transients stay despawned at retirement (`frozen.rs:670-682`).
  A sector record never holds a transient.
- Visual-only effects are not saved (D-T6).

What dies: the claim "save/load does not resume an in-flight shot"
(SCOUT-TRANSIENTS.md section "fidelity edges"). It is not in durable docs yet.
Launch cues on `On<Add, marker>` die (D-T2).

What may break:
- probe runs that count launch cues per `On<Add>`;
- tests that spawn a bare `TurretBulletProjectileMarker` or
  `TorpedoProjectileMarker` and expect a muzzle flash.

What must fail loudly: see section 9.

## 2. Kinds in scope

| Kind | Spawn | Body | Lifetime | Live-entity refs |
|---|---|---|---|---|
| Turret round | `turret_section/firing.rs:335-395` | none, swept (`rounds.rs:247-315`) | `TempEntity(config.projectile_lifetime)` | `ProjectileOwner`, `TurretSectionPartOf`, `TurretSectionMuzzleEntity`, `RoundBitten` ring |
| Railgun slug | `railgun_section/firing.rs:189-220` | none, swept | `TempEntity(config.slug_lifetime)` | `ProjectileOwner`, `RoundBitten`, `RoundRake.armed/charged` |
| Torpedo | `torpedo_section/bay.rs:358-559` | dynamic, child collider sections | `TempEntity(config.projectile_lifetime)` | `ProjectileOwner`, `TorpedoSectionPartOf`, `TorpedoSectionSpawnerEntity`, `TorpedoTargetEntity` |
| Blast volume | `damage.rs:545-567` | static sensor | `BlastTicksLeft(8)` + `TempEntity(1 s)` | pending pairs by `Entity` (`damage.rs:583-615`) |
| Shed fixture | `fixture.rs:303-355` | kinematic, then dynamic | `TempEntity(12 s)`, `ChunkGrace` | `ShedFixtureMarker(section)` |
| Rock chunk | `asteroid_carve.rs:559-590`, `chunk.rs:232-254` | kinematic, then dynamic | `TempEntity(30 s)`, `ChunkGrace` | none; mesh and material are runtime handles |
| Detached section piece | `explode.rs:420-477` | kinematic, then dynamic | `TempEntity(30 s)`, `ChunkGrace` | `DetachedPieceMarker(source)`; reparented section art |

Visual-only (D-T6): impact sparks (`impact_spark.rs:175-190`), damage sparks
(`damage_sparks.rs:181-193`), pyre particles (`pyre.rs:977-1018`), transient
lights (`transient_light.rs:153-204`), torpedo render blasts
(`torpedo_section/render.rs:551-605`), railgun wake emitters
(`railgun_section/wake.rs:385-435`), hanabi carve chips (`spew.rs`).

## 3. Restore point (D-T1)

Facts (SCOUT-IDS.md section 9):
- `restore_resumed_world` restores the ledger on the arming frame
  (`session.rs:252-265`).
- `materialize_ready_sector` spawns at most one sector per Update
  (`streaming.rs:814-850`).
- `ScenarioLoadGate` releases the physics and virtual clocks before the
  window is full (`loader/gate.rs:82-106`).
- No signal says that every desired sector is live.

So today, sectors that materialize early simulate while later ones are still
frozen. A transient whose owner or target sits in a later sector has no
entity to point at.

Proposal (a):
- `resume_world` holds the clocks under a new `FreezeOwner::WorldResume`.
- A new exclusive system `restore_resumed_transients` runs after
  materialization. When every desired coordinate has a live root and its
  spawn commands have applied, it spawns the saved transients, then releases
  the hold. This happens once per Load.
- Readiness (owner, T1 integration):
  - the exact desired coordinate set of the armed window is live;
  - this frame's spawn commands are applied, because the restore runs after
    `Retire`;
  - every saved owner resolves.

  `nova_world_base` counts the window with the existing
  `desired_sectors` and `live_sectors`. No new `nova_world` function is
  added, because the radius lives in the generic `WorldConfig<G>`.
- The records wait in the crate-private `ResumedTransients { transients,
  started }` until restore or refusal.
- While `ResumedTransients` exists, `freeze_transients` returns
  `Unsettled("the world is still resuming")`. A crossing or first-arm save
  then cannot write `transients: []` over the good save. The settling
  counter does not advance under the hold. The race has its own proof,
  P-T12.
- An owner that does not resolve yet keeps the Load waiting up to the
  bound. If held saved ships cannot be built under the hold, I stop and
  ask.
- Checked: no saved owner is a held ship. A ship that fired had a body, so
  it freezes as `FrozenBodyType::Ship`. Only a manifest ship that never
  spawned freezes as `PendingShip` (`nova_world/src/frozen.rs:430`). Its
  record thaws as a body (`frozen.rs:536`), not through
  `materialize_pending_ships`. A live owner outside the desired window
  cannot be saved. `adopt_moving_bodies` keeps top-level only the bodies
  bound for a desired cell (`frozen.rs:640-646`). It freezes the others,
  or holds them unsettled, and the snapshot waits on those
  (`frozen.rs:782-789`, `frozen.rs:808-812`). A Load uses the saved
  `CurrentSector`, so the desired set is the same, and every saved owner
  thaws in it.

Consequence: a Load shows the loading screen a few frames longer. Every
resumed body and transient then starts on the same tick. This also removes
the inter-sector skew that exists today.

Unverified: that streaming preparation and materialization run while the
virtual clock is held. They are Update systems with async tasks; the proof
P-T1 observes it.

Option (b): spawn on the arming frame. Refs to owners in later sectors
cannot resolve, so we would have to fail or drop. Rejected.

Bound and refusal (owner condition):
- The hold counts `Time<Real>`, not frames. The virtual clock is held, and
  preparation runs on worker threads, so frames say nothing about progress.
  The bound is `WORLD_RESUME_SECONDS_MAX = 240.0` (owner, rev 2 review).
  The hosted runner fills about 125 cells at about 1.8 fps, which can take
  more than 69 s with steady progress. 240 s is the same order as the slow
  world-training deadline. No claim is made that a machine loads in 30 s.
- Visible progress: the new resource
  `WorldResumeProgress { live: usize, desired: usize }` (nova_world_base)
  exists while the hold does. `restore_resumed_transients` updates it each
  frame. The scenario Loading screen (`nova_core/src/loading_screen.rs`):
  - stays up while the resource exists, before the dwell and the cap, as
    it does for `is_settling` and `ScenarioPreload` (`loading_screen.rs:432-436`);
  - shows the line `RESTORING SECTORS <live> / <desired>` in the panel
    column.
- Past the bound, `restore_resumed_transients`:
  1. releases `FreezeOwner::WorldResume`, so the clocks never stay frozen;
  2. spawns no transient, so nothing is half-restored;
  3. inserts the new resource
     `WorldResumeRefused { slug: String, reason: String }`. The reason names
     every desired sector that is not live.
- The same refusal ends a Load whose saved ref has no live match at restore
  (section 9).
- A new nova_menu system, `refuse_resumed_world`, answers it with the
  teardown that "Leave without saving" does today (`leave.rs`
  `on_leave_without_saving`):
  - remove `WorldSaveSession`, which drops `world.lock`;
  - set `GameStates::MainMenu` and `PauseStates::Unpaused`;
  - open the Load panel and write the reason on that world's row, as the
    refusal from `on_load_world` does.
- Nothing is written, so the last good save stays as it was. A second Load
  of that world can try again.

## 4. Records

New required field `WorldSaveState.transients: Vec<FrozenTransient>`
(`save/mod.rs:85-97`). This format is unshipped, so there is no version bump
or migration.

Ownership follows the frozen-body pattern: each owning crate defines its
record and its `freeze_*` and `thaw_*`; `nova_world_base::save` collects and
dispatches.

```rust
// nova_world_base/src/save/transients.rs (new module)
pub struct FrozenTransient { lifetime: SavedLifetime, body: FrozenTransientType }
pub enum FrozenTransientType {
    Round(FrozenRound),          // nova_ship (turret or railgun art)
    Torpedo(Box<FrozenTorpedo>), // nova_ship
    ShedFixture(FrozenShedFixture), // nova_ship
    RockChunk(FrozenRockChunk),  // nova_scenario
    DetachedPiece(Box<FrozenDetachedPiece>), // nova_ship
}
pub(crate) fn freeze_transients(world: &mut World) -> Result<Vec<FrozenTransient>, SectorSnapshotError>;
pub(crate) fn restore_resumed_transients(world: &mut World);

// nova_gameplay/src/lifetime.rs
pub struct SavedLifetime { total: f32, remaining: f32 }
impl SavedLifetime { pub fn of(world: &World, entity: Entity) -> Option<Self>; }
pub fn resumed_lifetime(saved: SavedLifetime) -> impl Bundle; // (TempEntity(total), TempEntityState(elapsed = total - remaining))
```

`on_insert_temp_entity` (`lifetime.rs:91-110`) leaves an existing
`TempEntityState` alone. A re-insert of `TempEntity` then no longer resets
the timer. Verified (SCOUT-TRIGGERS.md 4): no caller re-inserts `TempEntity`
on an entity that has one. `railgun_section/wake.rs:435` is a first insert.

### 4.1 Rounds

```rust
// nova_gameplay/src/rounds.rs
pub struct FrozenRoundFlight {
    translation: Vec3, rotation: Quat, // easing `end` when set, else Transform
    velocity: Vec3,
    damage: ProjectileDamage,
    allegiance: Option<Allegiance>,
    owner: SavedOwner,
    rake_radius: Option<f32>,
}
pub fn freeze_round_flight(world: &World, entity: Entity) -> Result<FrozenRoundFlight, UnsettledBody>;
pub fn thaw_round_flight(flight: &FrozenRoundFlight, owner: Entity) -> impl Bundle;
#[derive(Component)] pub struct ResumedRound; // first sweep step only

// nova_ship/src/sections/frozen_rounds.rs (new)
pub struct FrozenRound { flight: FrozenRoundFlight, source: RoundSourceType }
pub enum RoundSourceType { Turret { render_mesh: Option<AssetRef<WorldAsset>> }, Railgun }
```

- Pose: a round eases between fixed ticks (`turret_section/firing.rs:362-385`).
  The raw tick pose is `TranslationEasingState.end` and
  `RotationEasingState.end` when set, and otherwise `Transform`. The thaw
  seeds both easing `start` values with that pose.
- `RoundBitten` (`rounds.rs:59-80`) holds collider `Entity`s. Fixtures have no
  durable id (SCOUT-IDS.md 1), so the ring cannot be saved. Rule: the sweep
  casts only forward (`rounds.rs:49-55`), so the only remembered colliders
  that still matter are those the tip overlaps now. A `ResumedRound` treats
  each collider its first step's cast meets at distance zero as already
  bitten, without damage, and then the marker is removed. Unverified: that
  the cast reports origin overlaps. If it does not, the fallback is an
  explicit `shape_intersections` at the tip on the first step. Either way the
  proof P-T3 observes it.
- `RoundRake` (`rounds.rs:119-179`) reaches backward, so a forgotten entry is
  charged twice (`rounds.rs:107-112`). A slug with a non-empty `armed` set is
  `UnsettledBody("a raking slug is mid-body")`, and the save waits through
  the existing bound (`session.rs:347-360`). A slug with an empty set saves
  with `rake_radius` and a fresh rake.
- `TurretSectionPartOf` and `TurretSectionMuzzleEntity` feed only the launch
  cues and probe diagnostics (SCOUT-IDS.md 7). A thawed round carries
  neither (see D-T2). The probe reader at `snapshot.rs:1388` reads them
  optionally; at implementation I check it and stop if it needs them.

### 4.2 Torpedoes

```rust
// nova_ship/src/sections/torpedo_section/bay.rs
pub(crate) struct TorpedoLaunch<'a> {
    config: &'a TorpedoSectionConfig, owner: Entity, section: Option<Entity>, spawner: Option<Entity>,
    translation: Vec3, rotation: Quat, linear: Vec3, angular: Vec3,
    allegiance: Option<Allegiance>, arming: TorpedoArming, cold: Option<TorpedoColdLaunch>,
    steering: Vec3, weave: TorpedoWeave, lifetime: SavedLifetime,
}
pub(crate) fn spawn_torpedo(commands: &mut Commands, launch: TorpedoLaunch) -> Entity;

// nova_ship/src/sections/torpedo_section/frozen.rs (new)
pub struct FrozenTorpedo {
    config: TorpedoSectionConfig, // inline: a torpedo outlives its bay (mod.rs:686-692)
    owner: SavedOwner, section: Option<SavedSectionRef>,
    translation: Vec3, rotation: Quat, linear: Vec3, angular: Vec3,
    allegiance: Option<Allegiance>, arming: TorpedoArming, cold: Option<TorpedoColdLaunch>,
    target: SavedTorpedoTarget, steering: Vec3, weave: TorpedoWeave, ignited: bool,
}
pub enum SavedTorpedoTarget { Unchosen, DumbFire, Tracking { body: SavedBodyRef, last: Option<Vec3> }, Frozen(Vec3) } // D-T7 widens `body` to ship, rock, canister, wreck and transient index
```

- `spawn_torpedo` is the bay launch bundle (`bay.rs:358-559`), extracted. Fire
  and thaw both call it, so there is one bundle.
- Weave phase comes from the entity index (`bay.rs:552-559`). The saved weave
  keeps its phase, so a thaw does not re-derive it.
- `TorpedoArming` and `TorpedoWeave` gain serde derives; their fields stay
  private.
- Target:
  - `Unchosen`: no `TorpedoTargetChosen` yet; targeting decides on the next
    pass, as live.
  - `DumbFire`: `TorpedoTargetChosen` only.
  - `Tracking`: `TorpedoTargetEntity` resolved from a durable body ref, plus
    the last `TorpedoTargetPosition` if it was set.
  - `Frozen`: the target died before the save, so position only
    (`projectile.rs:18-54`).
- `section`: `TorpedoSectionPartOf` feeds blast art (SCOUT-IDS.md 6). A live
  bay resolves by (ship id, section id). A dead bay saves `None`. This matches
  the live state: a dead section gives degraded blast art.
- `spawner`: launch cues only, so a thaw gives `None` (D-T2).
- A shot-down torpedo (`TorpedoShotDownMarker`, `bay.rs:1325-1360`) is in a
  despawn already queued. It is `UnsettledBody`, and the save waits one frame.
- A torpedo in its cold launch keeps `cold.remaining`. Its child colliders
  thaw with `ColliderDisabled` as `ignite_cold_torpedoes` expects
  (`projectile.rs:85-124`).

### 4.3 Blast volumes (D-T4)

A blast resolves on `CollisionStart` (`damage.rs:596-615`). A re-spawned
sensor raises `CollisionStart` again for every body it overlaps, which is
double damage.

Proposal: a live `NovaBlast` is `UnsettledBody("a blast is resolving")`. The
save waits at most 8 fixed ticks per blast, and the existing bound ends a
continuous barrage as a visible failure. No blast record.

### 4.4 Shed fixtures

```rust
// nova_ship/src/sections/frozen.rs
pub struct FrozenShedFixture { fixture: FrozenFixture, style: ShipStyleId?, translation: Vec3, rotation: Quat, linear: Vec3, angular: Vec3, grace: Option<f32> }
pub fn freeze_shed_fixture(world: &World, entity: Entity) -> FrozenShedFixture;
pub fn thaw_shed_fixture(commands: &mut Commands, record: &FrozenShedFixture) -> Entity;
```

- Rebuild uses `frozen_plate_body` and `frozen_decor_body`
  (`shell_skin.rs:666-681`, `skin_decor.rs:596-618`).
- Plate colour comes from the ancestor `ShipStyle` (`skin_style.rs:72-106`).
  A shed plate has no ancestor, so the record names the style.
- `freeze_fixture` refuses `HealthZeroMarker` (`frozen.rs:104-112`), so a
  shed fixture needs its own entry that skips that check.
- `ShedFixtureMarker(section)` is attribution only (`fixture.rs:180-190`). A
  thaw writes `Entity::PLACEHOLDER`, which reads as "the section is gone", as
  it is.
- Open: the style id type. If no durable style key exists, I stop and ask.

### 4.5 Rock chunks

```rust
// nova_scenario/src/objects/asteroid_carve.rs
#[derive(Component)] pub struct RockChunkSurface { kind: AsteroidKindId?, texture: ..., seed: u64 } // copied at spawn
pub struct FrozenRockChunk { surface: RockChunkSurface, mesh: FrozenChunkMesh, translation: Vec3, rotation: Quat, scale: Vec3, linear: Vec3, angular: Vec3, grace: Option<f32> }
pub struct FrozenChunkMesh { positions: Vec<[f32; 3]>, normals: Vec<[f32; 3]>, indices: Vec<u32> }
```

- The chunk mesh is built from its own island field, and the field is then
  dropped (`asteroid_carve.rs:495-520`). The record copies the mesh
  attributes out of `Assets<Mesh>`.
- The collider is rebuilt by `chunk_collider` from that mesh.
- The material is rebuilt the way an asteroid thaw builds it
  (`asteroid.rs:787-843`), from the chunk's `RockChunkSurface`.
- `RadarOccluder` is re-inserted as at spawn (`asteroid_carve.rs:580-588`).
- Verified (SCOUT-TRIGGERS.md 2): chunk meshes use
  `RenderAssetUsages::default()` (`mesh/builder.rs:309-312`), so the main
  world keeps the attributes. The material reads its texture through the
  extension, not the UVs (`asteroid.rs:815-819`).
- Unmeasured: save size. A 30 s chunk lifetime and up to 24 landings per frame
  can mean many meshes. P-T7 measures it.

### 4.6 Detached section pieces (D-T5)

Facts (SCOUT-PIECES.md, SCOUT-ART.md):
- A piece is a new root (`explode.rs:420-447`). The dead section's direct
  children are reparented onto it, and every descendant `Collider` is
  stripped (`explode.rs:450-477`). Section gameplay components stay on
  some nodes: fixture `Health`, turret joint controllers, the torpedo
  spawner cooldown (SCOUT-ART.md 10).
- No art-only builder exists. `section_body` builds a live section.
- The durable inputs die with the source: `SectionBuildConfig` stays on the
  despawned section, and `ShipStyle` stays on the ship root.
- The art nodes keep runtime handles only:
  - `WorldAssetRoot(Handle<WorldAsset>)`;
  - placeholder mesh and material handles;
  - skin surface meshes;
  - effect handles.

Design: a generic art-node snapshot. The piece is saved as its root's
physics state plus an ordered list of nodes. Each node keeps one durable
art record that a thaw rebuilds with existing builders. Gameplay leftovers
are never saved. An art node that the capture cannot classify panics with
its name and component list, so nothing is dropped silently.

```rust
// nova_ship/src/sections/frozen_piece.rs (new)
pub struct FrozenDetachedPiece {
    name: String,
    translation: Vec3, rotation: Quat, linear: Vec3, angular: Vec3,
    center_of_mass: Vec3,
    collider: SectionCollider,        // serde today (base_section.rs:54-55)
    grace: Option<f32>,               // ChunkGrace remaining, None once landed
    style: Option<String>,            // the ship's ShipStyle id, stamped at detach
    nodes: Vec<FrozenArtNode>,        // parents before children
}
pub struct FrozenArtNode {
    parent: Option<u32>,              // index into nodes; None = the piece root
    name: Option<String>,
    transform: Transform,
    visibility: Visibility,
    art: FrozenArtType,
}
pub enum FrozenArtType {
    Group,                                                   // transform only (turret joint, spawner, wrapper parent)
    Scene { asset: String, cracked: bool, poses: Vec<(Vec<String>, Transform)> },
    Placeholder(PlaceholderArtType),
    SkinPlate(ShellShape),                                   // surfaces re-hung from shape + piece style
    Decor { asset: String },                                 // ShipDecorMarker's AssetRef path
    Exhaust(ThrusterExhaustConfig),                          // outer cone; inner cone is its observer's child
    Light { color: Color, intensity: f32, range: f32, radius: f32 },
}
pub enum PlaceholderArtType { Body, ControllerBody, Window, Barrel, Nozzle, TurretPlate }

// at detach (nova_ship): copy the inputs that die with the source
#[derive(Component)] pub struct DetachedPieceSource { collider: SectionCollider, style: Option<String> }
fn stamp_detached_piece_source(add: On<Add, DetachedPieceMarker>, ..); // reads the source while it is still alive

pub fn freeze_detached_piece(world: &World, piece: Entity) -> FrozenDetachedPiece;
pub fn thaw_detached_piece(commands: &mut Commands, record: &FrozenDetachedPiece, lifetime: SavedLifetime) -> Entity;

// nova_gameplay/src/integrity/chunk.rs
impl ChunkGrace { pub fn resumed(collider: Collider, remaining: f32) -> Self; }

// nova_ship/src/sections/damage_cracks.rs
pub(crate) fn mark_wreck_cracks(commands: &mut Commands, mesh: Entity); // PendingSectionCracks { section: Entity::PLACEHOLDER }

// nova_ship: re-apply captured scene node poses once the scene exists
#[derive(Component)] struct ResumedScenePoses(Vec<(Vec<String>, Transform)>, bool /* cracked */);
fn apply_resumed_scene_poses(ready: On<WorldInstanceReady>, ..);
```

Capture, per node (walk from the root, depth first):
- A `WorldAssetRoot` wrapper gives `Scene`:
  - `asset` is `AssetServer::get_path(handle)`. A handle with no path (an
    in-memory `AssetRef::Handle`) panics, because it cannot be saved.
  - `poses` holds every scene descendant's local `Transform`, keyed by its
    `Name` path from the wrapper. Same-named siblings get an index suffix.
    This keeps the animated doors, arms and covers
    (`section_animation.rs:443-491`) and needs no track state.
  - `cracked` is true when any descendant wears `SectionCracksMaterial`
    (`damage_cracks.rs:363-425`).
  - The scene's own descendants get no node records, because the asset
    re-derives them.
- A placeholder node gives `Placeholder`, identified by comparing its mesh
  and material handles with `PlaceholderArt`'s (`placeholder_art.rs:54-108`).
  A handle that matches none panics.
- A `ShipSkinMarker(shape)` plate gives `SkinPlate`. Its `SkinSurfaceMarker`
  children are skipped, because the thaw re-hangs them from the shape and
  `style`.
- A `ShipDecorMarker(asset)` node gives `Decor`, with its `WorldAssetRoot`
  on the same entity. `Health`, `SectionFixture` and `DecorColliderSize`
  are not saved.
- A `ThrusterExhaustConfig` node gives `Exhaust`, and its inner cone is
  skipped. A detached plume settles to the zero bucket either way
  (`thruster_section.rs:641-696`).
- The `RailgunChargeGlowMarker` `PointLight` gives `Light`, with its frozen
  charge pose and visibility.
- Turret joints, the torpedo body and spawner, and plain parents give
  `Group`. This keeps the hinge pose that `sync_turret_joint_rotation` last
  wrote. Their controllers are not saved.
- A `ParticleEffect` child (turret muzzle, torpedo launch) is not saved. A
  piece never fires, and its spawners do not emit on start. This is D-T6
  (visual only). Verified (SCOUT-TRIGGERS.md 3): both piece-child spawners
  use `emit_on_start(false)`. The emit-on-start effects (detonation, pyre)
  are standalone entities, not piece children.

Thaw:
- The root gets the piece bundle of `explode.rs:425-447`:
  - `DetachedPieceMarker(Entity::PLACEHOLDER)`, the same dead-source
    reading as D-T3;
  - `ChunkGrace::resumed(collider.to_collider(), remaining)` while in grace;
  - otherwise a `Collider`, `RigidBody::Dynamic` and gravity, as
    `land_carved_chunks` leaves it.
- The nodes spawn in order under their parent index:
  - `Scene` gets `WorldAssetRoot(asset_server.load(asset))` and
    `ResumedScenePoses`. On `WorldInstanceReady`, it writes the saved
    poses and, if `cracked`, calls `mark_wreck_cracks` on each mesh. The
    resolver gives a missing section bucket 7 (`damage_cracks.rs:376-382`),
    which is what the piece showed.
  - `Placeholder` takes the shared handles from `PlaceholderArt`.
  - `SkinPlate` calls `hang_surfaces(.., shape, style config)`
    (`shell_skin.rs:1092`) on a bare node, without `ShipSkinMarker` (whose
    dressing needs a ship root).
  - `Decor` gets `WorldAssetRoot` only.
  - `Exhaust` gets its config, and its observer builds both cones.
  - `Light` gets a `PointLight`.
- No `Health`, `SectionFixture`, collider, turret controller or cooldown is
  inserted, so nothing on a piece can take damage, fire, or die again.

Stamp at detach: `stamp_detached_piece_source` observes
`Add<DetachedPieceMarker>`. The piece spawn is queued before the source
despawn (`explode.rs:470-477`), so in that flush the source still exists.
The observer copies:
- the section's `SectionCollider` from `SectionBuildConfig`. `None` is the
  authored unit-cube default (`base_section.rs:365-374`), so it is stamped
  as `SectionCollider::default()`;
- the `ShipStyle` of its ship root.

Verified (SCOUT-TRIGGERS.md 1): the spawn is queued before the reparent and
the despawn (`nova_gameplay/src/integrity/explode.rs:420-477`). Whole-ship
death does not detach (`explode.rs:356`). P-T10 asserts the stamp.

Fail loudly at capture (programming errors, panic with the node name and its
components, same policy as `freeze_body`):
- a piece without `DetachedPieceSource`;
- an unclassified art node;
- a scene handle without a path;
- a placeholder handle that matches no `PlaceholderArt` entry.

## 5. Durable refs (D-T3)

Placement (owner, T1 prep): the three ref types live in the lowest crate
that the records use, `nova_gameplay/src/saved_refs.rs`, and are exported
through its prelude. The rounds (nova_gameplay) and torpedo (nova_ship)
records name them. Resolution and refusal stay in nova_world_base.

```rust
// nova_gameplay/src/saved_refs.rs
pub enum SavedOwner { Ship(EntityId), Gone }
pub struct SavedBodyRef(EntityId);
pub struct SavedSectionRef { ship: EntityId, section: EntityId }

// nova_gameplay/src/saved_refs.rs (owner, T1 prep): one fault for every freeze
pub enum TransientFreezeFault { Unsettled(UnsettledBody), NoDurableId { label: String } }
impl SavedOwner { pub fn of(world: &World, owner: Entity) -> Result<SavedOwner, TransientFreezeFault>; }
// Ship(id) live with an id, Gone despawned, NoDurableId live without an id.
// Every transient freeze returns Result<_, TransientFreezeFault>.

// nova_world/src/frozen.rs
SectorSnapshotError::NoDurableId { label: String } // the Failed path, last good save kept

// nova_world_base/src/save/transients.rs
fn live_by_id(world: &mut World, id: &EntityId) -> Result<Entity, TransientRefFault>; // panics never; errors on 0 or >1 match
```

- Ship and rock ids are unique across the live window (SCOUT-IDS.md 1:
  `generation.rs:508-510,587-593`, `lib.rs:347-355`, player id `player`).
  Section ids are unique only within a ship, so a section ref pairs the two.
- No id index exists (SCOUT-IDS.md 2). `live_by_id` scans `EntityId` once per
  restore. Restore is a single pass, so there is no new index resource.
- `Gone`: the owner died before the save. The live entity holds a handle to
  a despawned entity, and every reader handles that (SCOUT-IDS.md 6 and 7):
  - arming reads the launcher as absent;
  - the owner filter matches nothing;
  - party uses the copied allegiance.

  Proposal (a): thaw writes `ProjectileOwner(Entity::PLACEHOLDER)`, the same
  "no such entity" state. Option (b): make the owner optional. But
  `ProjectileOwner` absence already means "authorless round", which is
  consumed by its first collider (`rounds.rs:1185-1189`). That changes
  behaviour, so it is rejected.

## 6. Launch cues (D-T2)

Four observers fire on `On<Add, marker>`:
- `on_projectile_marker_effect` (`turret_section/render.rs:15-75`);
- `on_turret_fire_play_sfx` (`ship_audio/combat.rs:242-267`);
- `on_torpedo_launch_effect` (`torpedo_section/render.rs:807-844`);
- `on_torpedo_launch_play_sfx` (`ship_audio/combat.rs:290-344`).

A thawed round with its refs replays the muzzle flash and sound. Without its
refs, the flash observer logs `error!` (`render.rs:51-56`,
`torpedo_section/render.rs:836-841`), which fails a clean probe pass.

- (a) The fire paths trigger `RoundFired { round: Entity }` and
  `TorpedoLaunched { torpedo: Entity }`. The four observers move to them. A
  thaw triggers neither. A launch cue then belongs to the launch, not to the
  entity existing.
- (b) A `ResumedProjectile` marker that the four observers filter out.

Recommend (a). (b) keeps a cue path that means "spawned" while it claims
"fired".

## 7. Snapshot integration

`snapshot_world` (`session.rs:304-380`) calls `freeze_transients` after
`snapshot_sectors` and before `freeze_ship`. Each `UnsettledBody` from it maps
to `SectorSnapshotError::Unsettled` and uses the same settling counter, so no
second wait path is added.

The transient query is every top-level `TempEntity` in the desired window,
classified by marker:
- `TurretBulletProjectileMarker` and `RailgunSlugProjectileMarker` give
  `Round`;
- `TorpedoProjectileMarker` gives `Torpedo`;
- `ShedFixtureMarker` gives `ShedFixture`;
- `CarvedChunkMarker` gives `RockChunk`;
- `DetachedPieceMarker` gives `DetachedPiece` (section 4.6);
- `NovaBlast` gives `Unsettled`;
- a visual-only marker from section 2 is skipped (D-T6);
- any other top-level `TempEntity` body with a `RigidBody` or `RoundVelocity`
  panics with its name. A new transient kind cannot be dropped silently.

## 8. Call graph

Before:
```
snapshot_world -> snapshot_sectors -> freeze_ship -> writer
resume_world -> ResumedWorld -> restore_resumed_world (arming frame) -> streaming thaws -> ScenarioLoadGate releases clocks
fire paths -> spawn(marker, ..) -> On<Add, marker> -> flash + sfx
```
After:
```
snapshot_world -> snapshot_sectors -> freeze_transients -> freeze_ship -> writer
resume_world -> hold(FreezeOwner::WorldResume) -> restore_resumed_world -> streaming thaws
  -> restore_resumed_transients [exact desired set live, owners resolve] -> thaw_* -> release(WorldResume)
fire paths -> spawn_torpedo / spawn round -> trigger RoundFired | TorpedoLaunched -> flash + sfx
```

## 9. Fail loudly

| Case | Result |
|---|---|
| Live owner, target or bay has no `EntityId`, or more than one live match | save `Failed("<label>: <ref> has no durable id")`, last good save kept |
| Ref missing at restore (saved `Ship(id)`, no live match) | `WorldResumeRefused` (section 3): clocks released, session dropped, reason on the Load row |
| Desired sector not live within `WORLD_RESUME_SECONDS_MAX` real seconds | `WorldResumeRefused`, naming the sectors |
| Detached piece: unclassified art node, scene handle without a path, unknown placeholder, missing `DetachedPieceSource` | panic naming the node and its components (section 4.6) |
| Raking slug with an armed body, a live blast, a shot-down torpedo | `Unsettled`, then the existing bound and visible `Failed` |
| Unknown top-level transient body | panic naming it (programming error, same policy as `freeze_body`, `frozen.rs:466`) |
| Non-finite pose, velocity or lifetime, or remaining > total | Load refusal `Unreadable` |
| Chunk mesh with no main-world data | panic at freeze; if it is reachable at all, I stop and ask (section 4.5) |

## 10. Proofs

All are ECS tests with asserted state unless named. Real Load path means
`open_world` then `resume_world`, as in the existing session tests.

- P-T1 `a_load_holds_the_world_until_every_saved_sector_is_live`: clocks
  hold until the exact desired sector set is live; transients spawn on that frame; the
  first fixed tick runs after.
- P-T2 `a_resumed_round_flies_on_and_expires_on_its_saved_lifetime`: pose,
  velocity, damage, owner and `remaining` round-trip through RON; it despawns
  at `remaining`, not `total`.
- P-T3 `a_round_resumed_inside_a_plate_does_not_bite_it_again`: the round
  saved mid-plate deals exactly the authored damage once, the same as
  `a_round_deals_its_authored_damage_once_per_crossing`.
- P-T4 `a_resumed_round_never_hits_the_ship_that_fired_it`: the owner filter
  holds after the thaw.
- P-T4b `a_resumed_shot_still_belongs_to_its_shooter`:
  - A thawed round and torpedo resolve `ProjectileOwner` to the live
    shooter that has the saved id.
  - The hit's attribution and `projectile_party` relation name that
    shooter, the same as an unsaved shot.
  - A torpedo saved after its owner died resolves to the placeholder, and
    its arming reads the launcher as gone.
- P-T5 `a_resumed_torpedo_keeps_its_target_arming_and_cold_launch`: the
  tracking target resolves, the arming state is equal, cold `remaining` is
  equal, the weave is equal, and detonation is attributed to the owner.
- P-T6 `a_save_waits_out_a_live_blast_and_a_raking_slug`: status is Waiting,
  then Saved after the blast retires; no blast is in the record.
- P-T7 `a_resumed_rock_chunk_has_its_mesh_material_and_grace`: vertex data is
  equal and grace remaining is equal. Also log the record size for a
  carve_asteroids save (measurement, no assertion).
- P-T8 `a_resumed_projectile_plays_no_launch_cue`: with the real
  turret and torpedo firing systems and the real cue observers, one fire
  gives exactly one muzzle flash, launch effect and fire sound. A thaw of
  that round and torpedo gives none, and no `error!` is logged.
- P-T10 `a_detached_piece_resumes_as_the_art_it_wore`, one case per
  section kind (Hull, Thruster, Controller, Turret, Torpedo, Railgun,
  Docking, CargoIntake, Mining) from the fixture catalog with skin and
  decor:
  - destroy the section and capture the piece;
  - RON round-trip it, then thaw it;
  - assert the thawed tree has the same node names, local transforms,
    visibility, art records, scene asset paths, skin surface count and
    style materials;
  - assert no node carries `Health`, `SectionFixture`, `Collider`,
    `TurretJointMarker`, `TorpedoSectionSpawnerFireState` or a section
    marker;
  - assert grace remaining and lifetime are equal, and the
    `DetachedPieceSource` stamp is present before the source despawns.
- P-T11 `a_load_that_cannot_fill_its_window_is_refused_and_releases_the_clocks`
  (the test steps `Time<Real>` past 240 s with
  `TimeUpdateStrategy::ManualDuration`; no bound override exists). It also
  asserts that the Loading screen shows `RESTORING SECTORS 0 / 1` while it
  holds:
  a desired sector that never prepares, plus a saved owner id with no live
  ship, each end past the bound with:
  - the clocks released;
  - no `WorldSaveSession`, and the lock file free;
  - `GameStates::MainMenu`;
  - the reason on the Load row;
  - the save files unchanged.
- P-T12 `a_load_saves_nothing_until_its_transients_are_back`: a Load with a
  saved round wants a save on its arming frame. The status stays Waiting
  and no file changes while the hold lasts. After the restore, the next
  write holds the round.
- P-T4b also covers `SavedOwner::of`: a live owner with an id gives
  `Ship(id)`, a despawned owner gives `Gone`, and a live owner without an
  id gives `NoDurableId`, which is a visible Failed save that keeps the
  last good save.
- P-T9 two-process: extend `system_world_resume` (P6) to save with a turret
  round, a torpedo and a detached piece in flight. Phase 2 asserts each
  exists with the saved owner, pose and remaining lifetime, then expires.
  It captures the frame before the leave and the first frame after the
  Load (P8 style), and compares them by eye and by a pixel-difference
  figure.

Mutations, each expected to fail its proof:
- drop the `ResumedRound` seeding (P-T3);
- thaw `TempEntity(total)` without state (P-T2);
- skip the clock hold (P-T1);
- leave the cue on `On<Add>` (P-T8);
- skip `poses` on thaw (P-T10, on the Torpedo, CargoIntake and Mining doors);
- skip the clock release on refusal (P-T11).

## 11. Phasing

- T1: restore point, lifetime, rounds, blast wait, cue events (P-T1 to P-T4,
  P-T6, P-T8).
- T2: torpedoes (P-T5). Before T2 (owner): code-backed options for a
  torpedo tracking an id-less severed wreck or carved chunk (for example,
  keep the last target position under explicit Frozen guidance, or refuse).
  No silent retargeting or data loss, and a proof of that real-play case.
- T3: shed fixtures, rock chunks, detached pieces (P-T7, P-T10).
- T4: the P-T9 extension and P-T11.

The camera gate rev 3 lands first, since its owner condition is a stable
slice.

## 12. Decisions

- D-T1 restore point: (a) hold the clocks until the window is live, with
  a 240 s real-time bound, visible progress and a visible refusal.
  Approved by the owner on the rev 2 review.
- D-T2 launch cues: (a) explicit fired events. Recommend (a).
- D-T3 gone owner: (a) placeholder handle. Recommend (a).
- D-T4 blast and raking slug: save waits. Recommend wait.
- D-T5 detached section pieces: the generic art-node snapshot (section 4.6).
  Rev 1's exclusion is withdrawn.
- D-T6 visual-only effects not saved. Recommend yes.
- D-T7 torpedo target with no `EntityId` (owner, 2026-10-08): option (c).
  Evidence is in SCOUT-IDLESS.md.
  - Each kind keeps a key the save already has:
    - `Ship` and `Rock` keep their `EntityId`;
    - a canister keeps its `CargoCanisterRuntimeId`;
    - a saved transient (torpedo, chunk, piece, fixture) keeps its index in
      `WorldSaveState.transients`.
  - A persistent severed wreck gets a stable `EntityId` when it is severed:
    the source ship id plus its lowest section id, which is disjoint per
    wreck. `FrozenBody.id` keeps it, and `live_by_id` resolves it.
  - The sever path refuses a collision, an ambiguous id or a missing source
    id. It never overwrites one.
  - Proofs:
    - the index stays stable across a RON round-trip;
    - the restore order holds;
    - every target is in the saved live window;
    - torpedo-on-torpedo works;
    - a moving wreck works.
  - T2 does not land with dangling debris refs. T2 and T3 land as one
    integrated slice, or T2 waits for T3.
  - Stop and amend the gate before the wreck-id part if any of these is
    true: section ids can collide, a source id can be missing in valid play,
    or the id breaks an existing namespace or catalog rule.
- D-T7 amendment (owner approved 2026-10-08, after scout-wreckid). The stop
  condition "a source id can be missing in valid play" is true for the rule
  as approved:
  - A wreck root is a sever source too (`nova_ship/src/sections/integrity.rs:501-521`).
  - Today it is spawned with no `EntityId` and no link to its ship
    (`integrity.rs:628-646`).

  Section ids are unique per ship. `resolve_and_mate` refuses a duplicate
  (`nova_world_base/src/ship_layout.rs:2257-2261`). Every section gets one
  (`nova_scenario/src/objects/spaceship.rs:913-916`). No `EntityId` format
  rule exists (`nova_events/src/lib.rs:67-78`). `FrozenBody` carries any id
  with no format change (`nova_world/src/frozen.rs:470,521`).

  Amended rule:
  - Mint once on every sever, ship to wreck and wreck to wreck, in
    `sever_disconnected_structures` (`integrity.rs:628-646`). Never
    recompute it.
  - The id is `"{source}/wreck/{section}"`:
    - `source` is the severed root's `EntityId`. A ship always has one, and
      a wreck gets one at its own birth.
    - `section` is the least section `EntityId` of the new fragment, by its
      inner `String`.
    - No generator or authored id uses `/`, so the namespace is disjoint
      from both.
  - Sibling fragments of one sever hold disjoint sections, so their ids
    differ. A later sever of the same ship uses sections that are no longer
    in an earlier wreck.
  - A severed root with no `EntityId` mints nothing. The fragment stays
    id-less, and `error!` names it. A save with a torpedo tracking it then
    fails visibly (`NoDurableId`) and keeps the last good save. A minted id
    that a live body already has is handled the same way, so no id is
    overwritten.
  - Proofs:
    - a ship severs into two wrecks with distinct ids;
    - one wreck severs again, and the grandchild id derives from the
      wreck's id;
    - a moving wreck's id survives a save and Load;
    - a torpedo tracking it resolves to it after the Load.
  Owner conditions on the approval:
  - The mint checks collisions against the persisted and frozen ids
    (the `FrozenSectors` ledger) as well as the live ids.
  - The Load validates the ids and refuses duplicates. A live-only check
    does not cover unloaded cells.
  - Stop and report before implementing the mint if either is true:
    - an authored or mod id can use the `/wreck/` namespace;
    - a frozen collision cannot be detected safely.
- D-T7 second amendment (approved 2026-10-08 by the owner's automode, not
  personal sign-off; reversible, unshipped-format work only). scout-idspace
  found both stop conditions true:
  - `EntityId` has no character rule (`nova_events/src/lib.rs:67-78`).
  - The mint in `nova_ship` cannot see `FrozenSectors`, because
    `nova_world` depends on `nova_ship` (`nova_world/Cargo.toml:30`).

  Approved:
  - (A) `/` is reserved in an `EntityId`. The authoring lint and the content
    load refuse a scenario object id or a section id that contains `/`, and
    the error names the field. Check every path that authors or generates
    an id, and every mod load path. CHANGELOG entry: **(breaking)** for
    mod ids.
  - (B) The mint checks the live ids only. A hit logs `error!`, and the
    fragment stays id-less. `nova_world_base` checks the ids across the
    ledger, the live window and the transients:
    - at the snapshot, a duplicate gives a visible Failed that keeps the
      last good save;
    - at `open_world`, a duplicate or malformed id is refused as
      `Unreadable` (fail closed).
  - Stop for explicit user approval if this needs a migration or a
    destructive change to a shipped format.
- D-T8 refusal surface (owner, 2026-10-08, after SW-REFUSE.md):
  1. On refusal, after the lock is dropped, `refuse_resumed_world` re-reads
     the list from disk as `on_load_screen` does, then writes the reason on
     the row. If that read fails, the row list shows the real I/O error.
  2. The row error is the new `WorldRefusal::Unrestored(String)`, not
     `Io`.
  3. The private `LoadingResumeLineMarker` goes on a Text line in the
     panel column. Its text is cleared when there is no progress.
  4. P-T11 lives in `nova_core/tests/world_resume_refusal.rs`, with the
     production plugins and the dev-only `nova_world_base` fixture. There
     is no `nova_menu` -> `nova_core` dependency. P-T11 proves:
     - New Game -> Retry with an empty initial list;
     - the bounded refusal;
     - the clocks and the lock released;
     - the save intact;
     - the row reason and the progress line visible.

     Stop with options if the rig cannot reach the production path.
- D-T7 id-rule gate, revision 3 (proposed 2026-10-08, after SCOUT-SLASH.md).
  The owner rejected revision 2: a lint finding on a catalog design does
  not stop a scenario that spawns it, and the spawn skips the bad section
  (`nova_scenario/src/objects/spaceship.rs:848-855`). Every used design must
  fail at load. Duplicate section ids are refused too (owner).
  - One pure check, NEW in `nova_scenario/src/objects/ship_design.rs`:
    `pub fn section_id_errors(design: &ShipDesign) -> Vec<ShipDesignError>`.
    It gives NEW `ShipDesignError::ReservedSectionId(SectionId)` for an id
    that contains `/`, and NEW `ShipDesignError::DuplicateSectionId(SectionId)`
    for an id that is used twice. `resolve_ship_design` calls it first and
    returns the empty design with those errors, so no caller gets a partial
    ship.
  - Load owner: the existing bundle gate of `register_bundles`
    (`nova_assets/src/merge.rs:150-271`), which today refuses sections only.
    `section_errors` (`merge.rs:617-650`) also runs `section_id_errors` on
    every `Content::Ship` of the bundle, before any merge. The result is the
    same as for a bad section:
    - base content: `FatalAssetFailure`, nothing is published;
    - a mod: safe mode quarantines it and its dependents, and the reason is
      recorded;
    - a mod in `Playing`: `FatalAssetFailure`.
    No registry holds such a design. A scenario that names it gets
    `UnknownDesign`, an Error, so `on_load_scenario` refuses the scenario.
  - Inline designs (scenario objects, spawn actions, scatter templates):
    `check_design_sections` and `check_object_prototypes`
    (`nova_scenario/src/lint/ship.rs:26-95`, `lint/scenario.rs:772-783`)
    turn the two errors into Error findings. `on_load_scenario` refuses the
    scenario, base or mod, and spawns nothing.
  - Object ids: a `walk_names` `Names::NewObject` check in `lint_scenario`
    gives an Error that names the field. `on_load_scenario` refuses the
    scenario.
  - Spawn (`insert_spaceship_sections`): the two errors cannot get past the
    gate there. If one does, the spawn panics with the entity and the error.
    It does not spawn a partial ship.
  - Generated ids: no change. `resolve_and_mate` already refuses duplicates,
    and no generated format holds `/` (SCOUT-SLASH.md 2).
  - New tests:
    - nova_scenario lint: `a_slash_in_an_object_or_section_id_is_an_error`
      and `a_repeated_section_id_is_an_error`;
    - nova_assets merge:
      `a_design_with_a_bad_section_id_is_refused_before_any_registry`
      (base gives `FatalAssetFailure`; a mod is quarantined and its design is
      absent from `GameShipDesigns`);
    - nova_scenario lifecycle:
      `a_scenario_with_a_bad_inline_design_spawns_no_ship` (no ship root and
      no section).
  - If base content already repeats a section id, the worker stops and
    reports the ids before any content edit.
  - Approved by the owner 2026-10-08, with the spawn-time panic. Conditions:
    - verify catalog designs are refused at bundle registration (base
      fatal, mod quarantined); inline designs and object ids fail the
      scenario load; a scenario that names a quarantined design cannot
      spawn it;
    - the runtime guard refuses before any ship root or section is
      inserted; add a focused bypass proof if practical;
    - stop before any content edit if base content collides.

    `SavedTargetRef`, the torpedo pair, the two-phase restore, the wreck
    mint and the save/open id checks (revision 2 items 1-3, 5, 6) proceed
    after this gate, with the identity and round-trip tests named there.
  - Guard placement (scout-shiproot, main worker check):
    - Scenario ships (spawn action, scatter copies, resumed ships): the
      root is spawned by a `Commands` closure (`nova_scenario/src/actions/spawn.rs:188-191`),
      and the ship is built later in a queued `World` closure
      (`spawn.rs:206-216`, `spawn_scenario_spaceship`). The Spaceship arm
      moves the root spawn into that `World` closure. The closure runs
      `section_id_errors` on the design (inline: the config; prototype: the
      `GameShipDesigns` entry) and panics before it spawns the root.
    - Sector ships: `require_resolved` (`nova_world/src/streaming.rs:300-315`)
      resolves before `spawn_sector_ship` and panics on
      `SectorFault::InvalidShipDesign`. The new resolver errors reach it, so
      no change is needed there.
    - Base content: no collision (scout-shiproot; `every_block_ship_names_each_section_once`,
      `nova_authoring/src/base_content/ships/block.rs:788-800`, covers the
      catalog). UI theme ids such as `base/phosphor` are a catalog key, not
      an `EntityId`, so they are out of scope.
- T3 names (owner, 2026-10-08):
  - Every T3 freeze returns `Result<_, TransientFreezeFault>`:
    `freeze_shed_fixture`, `freeze_rock_chunk`, `freeze_detached_piece`.
  - `pub fn freeze_rock_chunk(world: &World, entity: Entity) -> Result<FrozenRockChunk, TransientFreezeFault>`
    in `nova_scenario/src/objects/asteroid_carve.rs`.
  - `pub fn thaw_rock_chunk(commands: &mut Commands, record: &FrozenRockChunk) -> Entity`
    is approved only if it rebuilds the mesh, collider and material through
    existing deferred spawn or observer systems. If it must mutate
    `Assets<Mesh>` or materials, or read resource catalogs directly, stop:
    revise the signature to take those resources (or `&mut World`) and
    show its call graph before the API is edited. No placeholder art.
  - New test `a_resumed_shed_fixture_keeps_its_art_style_and_grace`
    (nova_ship): a real plate and a real decor fixture shed off a styled
    ship; freeze, RON round-trip, thaw; assert the collider, the style
    material, the grace remaining, the pose, the placeholder
    `ShedFixtureMarker`, and no `Health`.
- Rock chunk thaw (owner, 2026-10-08, after SW-CHUNK.md step 0): option A.
  ```rust
  pub fn thaw_rock_chunk(
      commands: &mut Commands,
      meshes: &mut Assets<Mesh>,
      materials: &mut Assets<AsteroidSurfaceMaterial>,
      asset_server: &AssetServer,
      record: &FrozenRockChunk,
  ) -> Entity
  ```
  - `spawn_resumed` borrows these through one `SystemState` over the whole
    spawn loop and applies it once, so the indices and refs stay stable.
  - The thaw rebuilds the real mesh and collider. It builds the material
    from the saved kind, texture and seed with the same helpers as
    `insert_asteroid_render`.
  - P-T7 compares the rebuilt geometry, the material inputs and the
    visibility. Missing or invalid mesh or surface data fails loudly; it
    never draws nothing. No placeholder art.
- Save and open id checks (owner, 2026-10-09): approved, private in
  `nova_world_base/src/save`:
  `enum SavedIdFault { Duplicate(EntityId), Malformed(EntityId), Dangling(String) }`
  with `Display`, and `fn check_saved_ids(state: &WorldSaveState) -> Result<(), SavedIdFault>`.
  - Snapshot: `Duplicate(id)` gives `SectorSnapshotError::DuplicateId { id }`.
    `Malformed` and `Dangling` give a visible Failed through the NEW
    `SectorSnapshotError::InvalidSavedState { reason: String }`, not a
    panic. Nothing is written; the last good save is kept.
  - Open: every fault gives `WorldRefusal::Unreadable` with the state file
    name and the precise reason.
  - `SavedOwner::Gone` and a `Frozen(position)` target are valid, never
    refused.
  - Tests cover duplicates across cells and against the player, a
    malformed recursive wreck id, a dangling owner, body and section ref,
    a transient ref to itself and out of range, and no write on a refused
    snapshot.
- Detached piece thaw (owner, 2026-10-09, after SW-PIECE.md step 0):
  ```rust
  pub fn thaw_detached_piece(
      commands: &mut Commands,
      asset_server: &AssetServer,
      placeholder: &PlaceholderArt,
      styles: Option<&GameStyles>,
      skin_assets: &mut SkinAssets,
      meshes: &mut Assets<Mesh>,
      materials: &mut Assets<StandardMaterial>,
      record: &FrozenDetachedPiece,
  ) -> Entity
  ```
  - No `lifetime` argument. `spawn_resumed` inserts `resumed_lifetime`
    after the spawn, as for rounds.
  - `freeze_detached_piece` returns `Result<_, TransientFreezeFault>`.
  - `hang_surfaces` (`shell_skin.rs`) becomes `pub(crate)`.
  - A Decor thaw inserts `ShipDecorMarker(AssetRef::Path(asset))`; the live
    `dress_skin_decor` observer adds `WorldAssetRoot`. A re-save then
    classifies the node as Decor again.
  - Style: `style: None` keeps the live unstyled look. A saved
    `Some(id)` must be in the pinned `GameStyles`. The Load prevalidates it
    in `resolve_all`, before any transient spawns, and refuses with the
    Unrestored path. The thaw panics on a miss, because prevalidation makes
    a miss a programming error. It never wears none in place of a saved
    style. `FrozenDetachedPiece` gets `pub fn style(&self) -> Option<&str>`
    for that check.
  - New test `a_load_whose_piece_wears_an_unknown_style_is_refused`
    (nova_world_base save tests): the refusal names the style, no transient
    spawns, and the clocks release.
  - One `SystemState` borrow and one apply for every thaw in
    `spawn_resumed`. P-T10 checks the real scene and skin assets.
- T2/T3 worker deviations (owner, 2026-10-09, after SW-TORPEDO.md and
  SW-SHED.md):
  - Keep `pub(crate) struct TorpedoLaunchConfig(TorpedoSectionConfig)` on
    every live torpedo, so a destroyed bay cannot erase the config a freeze
    needs.
  - `FrozenTorpedo` gains `pub controller_health: f32` and
    `pub thruster_health: f32`, the current `Health` of the two parts. The
    thaw writes them after `spawn_torpedo`. The Load validates them in
    `FrozenTransient::validate`: finite and in `(0, max]` of the authored
    projectile health. A corrupt value refuses the open (`Unreadable`); it is
    never clamped. P-T5 asserts both.
  - Keep `a_resumed_torpedo_with_a_frozen_target_keeps_its_last_known_position`.
    A `Frozen` thaw inserts `TorpedoTargetChosen`, as every live lock does
    (`intent.rs:248-251`), so no post-Load retargeting.
  - `FrozenShedFixture.grace` is deleted. The shed fixture's `TempEntity`
    remaining time is its `FrozenTransient.lifetime`, and the test checks it.
  - `resolve_all` prevalidates an explicit `FrozenShedFixture.style` the same
    way as a piece style. A miss refuses the Load visibly; `None` stays
    unstyled. The refusal test gets a shed case.
- Transient integration (owner, 2026-10-09):
  - Private `struct ResolvedRefs { owner: Entity, section: Option<Entity>, target: Option<Entity> }`
    per transient from `resolve_all`. `target` holds Body and Canister refs;
    `Transient(i)` resolves in phase 2 from the spawned list, with no tick
    between the spawn and the target link.
  - A missing owner, Body, Canister or bay-section ref waits, bounded by the
    240 s hold, then the Load is refused. Ambiguity refuses at once. An
    out-of-range or self `Transient(i)` is refused at open.
  - Render resources are optional borrows. Before phase 1 the restore checks
    the resources the record set needs, and a missing one refuses the Load
    (Unrestored) with its name. Nothing spawns and nothing is written.
    Invalid file data is Unreadable at open; a missing runtime resource is
    Unrestored.
  - New accessors `FrozenShip::has_section(&self, section: &str) -> bool`
    and `FrozenCanister::id(&self) -> CargoCanisterRuntimeId`, for the
    dangling-ref checks in `check_saved_ids`.
  - `pub fn validate(&self) -> Result<(), String>` on `FrozenRockChunk` and
    `FrozenDetachedPiece`, called from `FrozenTransient::validate`: finite
    poses and grace, equal mesh attribute lengths, indices in range, a
    non-empty mesh, and piece parents before children.
- Torpedo part at zero (owner, 2026-10-09, after SW-TORPEDO.md round 2):
  option (a). `freeze_torpedo` returns `Unsettled` while either part has
  `Health.current <= 0` or `HealthZeroMarker`, until the destruction
  finishes. On-disk validation stays `(0, max]`. Proof: P-T5 asserts the
  freeze is Unsettled in the window before the marker lands, and a save
  test proves the save waits and then succeeds, and keeps the last good
  save meanwhile.

### Dead target link (owner, 2026-10-09)

A torpedo whose `TorpedoTargetEntity` names a despawned entity is not
Unsettled. The pause gate (`nova_ship` `configure_pause_gating`) stops
`update_target_position` while a leave save settles, so a wait for that
pass can reach the bound. `freeze_torpedo`'s `target` closure returns
`Result<Option<SavedTargetRef>, TransientFreezeFault>`; `None` means the
target is despawned. The freeze then saves the state the dropped link
leaves: `Frozen(TorpedoTargetPosition)`, else `DumbFire`. Guidance does not
run under pause. Proof: `a_torpedo_tracking_a_torpedo_resumes_on_it`
(the reload keeps `Frozen(last)`, the live pose, and no link) and
`leaving_a_saved_world_waits_for_the_leave_save_with_input_still_paused`
(the real paused leave completes). Mutation 16.

### Correction to 4 (lifetime re-insert)

The section 4 sentence "no caller re-inserts `TempEntity`" holds for rounds,
torpedoes and shed fixtures only. A rock chunk thaw goes through
`spawn_carved_chunk`, which inserts `TempEntity(CHUNK_LIFETIME_SECS)`, and
`spawn_resumed` then inserts `resumed_lifetime`. The second insert writes
`TempEntityState` in the same bundle, so the saved remaining time wins.
This is traced in code (REVIEW-T23.md 2, SW-CHUNK.md); no test restores a
chunk through `spawn_resumed`.

### Placeholder art identity (owner, 2026-10-09)

Section cracks swap a placeholder node's StandardMaterial away and can drop
its `SectionCracks` (orphan), so a freeze could not tell the art apart and
panicked. `placeholder_art.rs` owns `PlaceholderArtType` and
`PlaceholderArtMarker(pub PlaceholderArtType)`. `PlaceholderArt::bundle(art)`
stamps the marker at every spawn site and at the thaw.
`FrozenArtType::Placeholder { art, cracked }`. `cracked` (and the Scene
`cracked`) comes from `damage_cracks::mesh_wears_cracks`: a cracked
material, or a pending or graded mark whose section is gone or past the
pristine bucket. The thaw calls `mark_wreck_cracks` when cracked. An
unmarked mesh panics by node. `classify_placeholder` and
`placeholder_handles` are deleted. Proof: P-T10 with the real kill path,
pre-grade window, orphaned state, real thaw, re-freeze. Mutations 17-20.

### Ambiguity proof (owner, 2026-10-09)

`a_resumed_shot_still_belongs_to_its_shooter` covers the two ambiguity
refusals. Two live ships with the saved owner id, and two live sections of
the owner with the saved bay id, are each refused by name on the first
restore frame. Nothing spawns, the clocks run, and the save files are
unchanged. The section case adds its record to the held list after open,
because the fixture player has no saved section. Mutations A1 and A2.
