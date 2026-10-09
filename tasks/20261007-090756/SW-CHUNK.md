# SW-CHUNK: rock chunk freeze/thaw (section 4.5, T3 names) — Step 0 STOP

Owner: asteroid_carve.rs worker. Scope: `crates/nova_scenario/src/objects/asteroid_carve.rs`
(+ its tests) and the prelude export lines for the new items. No other file touched.
No edits made anywhere — this is a Step 0 report, as instructed.

## Claim

`thaw_rock_chunk(commands: &mut Commands, record: &FrozenRockChunk) -> Entity`, as named
in TRANSIENT-GATE.md section 12 ("T3 names"), **cannot** be implemented against that
signature. Rebuilding the chunk's mesh handle and its `AsteroidSurfaceMaterial` handle both
need `Assets<_>::add`, and no existing deferred-spawn or observer path reaches a standalone
chunk entity with those resources. `Commands` alone cannot do it.

The collider half is fine: `chunk_collider(&Mesh) -> Option<Collider>` is pure
(`crates/nova_gameplay/src/integrity/chunk.rs:276-296`, via `mesh_bounds` and `hull_points`,
both read-only on `&Mesh`). That part needs no resource and is not blocked.

## Evidence

**The live chunk path builds the mesh handle in a system that already holds `Assets<Mesh>`,
not in the chunk-spawn helper:**
- `spawn_carved_chunk(commands: &mut Commands, spawn: ChunkSpawn) -> Entity`
  (`crates/nova_gameplay/src/integrity/chunk.rs:241-263`) takes `ChunkSpawn.mesh: Handle<Mesh>`
  already built (`chunk.rs:217`) and `ChunkSpawn.collider: Collider` already built (`chunk.rs:227`).
  It never touches `Assets`.
- The handle is built by the caller: `throw_severed_pieces(commands: &mut Commands, meshes:
  &mut Assets<Mesh>, parent: &Parent, pieces: Vec<CarvedPiece>)`
  (`crates/nova_scenario/src/objects/asteroid_carve.rs:532-591`) calls
  `meshes.add(body.mesh)` at line 563, inline in the `ChunkSpawn` literal.
- Its own caller, `collect_asteroid_remeshes` (`asteroid_carve.rs:841-986`), is a system with
  `mut meshes: ResMut<Assets<Mesh>>` (line 843) — that's where the resource access lives
  today.

**The chunk's material is never built; it is a handle CLONE off the live parent rock, not an
`Assets::add`:**
- `Parent.material: Option<MeshMaterial3d<AsteroidSurfaceMaterial>>`
  (`asteroid_carve.rs:447`), populated at `collect_asteroid_remeshes` line 904 from
  `chunk_material.cloned()` (query field `Option<&MeshMaterial3d<AsteroidSurfaceMaterial>>`
  on the carved NODE, line 852).
  Doc at `asteroid_carve.rs:437-444`/`asteroid.rs:821-824`: "The material is built ONCE per
  body and the carve path re-applies the same handle to a remeshed rock and to every piece it
  throws."
  Applied to each spawned chunk at `throw_severed_pieces` line 575-577:
  `if let Some(material) = parent.material.clone() { commands.entity(spawned).insert(material); }`
  — a component insert of an already-live `Handle`, not a build.
- A resumed chunk has **no live parent rock to clone from** — the rock may be on a different
  sector, destroyed, or simply not the entity this save round-trips. There is nothing to
  clone a handle off of. The material has to be built from scratch, which needs
  `Assets<AsteroidSurfaceMaterial>::add` plus `AssetServer` (to resolve the texture) plus
  `asteroid_kind_look` (pure, no resource).

**The one observer that does build a fresh mesh handle and a fresh material from kind/
texture/seed is scoped to children of a live `AsteroidMarker` root, which a chunk is not:**
- `insert_asteroid_render` (`crates/nova_scenario/src/objects/asteroid.rs:787-858`), on
  `On<Add, AsteroidRenderMesh>`:
  - `q_render: Query<(&AsteroidRenderMesh, &ChildOf, Option<&ThawedCarvedMesh>)>` (line 793)
    — **requires `ChildOf`**. A standalone entity with no parent fails `.get(entity)` and the
    observer logs `error!` and returns with nothing inserted (lines 799-805) — headless, no
    mesh, no material. That is exactly the "placeholder art" / silent-drop outcome the task
    forbids.
  - `q_asteroid: Query<(&AsteroidTexture, &AsteroidKind, &AsteroidSeed), With<AsteroidMarker>>`
    (line 794) is queried on the **parent** (`ChildOf(asteroid)`), not on the entity itself.
    Even wrapping a chunk in a synthetic parent would require that parent to itself carry
    `AsteroidMarker` + the three components — fabricating a fake rock root for every resumed
    chunk, which is a new mechanism, not reuse of an existing one.
  - It is reached today only via `thaw_asteroid`'s own `entity.with_children(..)` spawn
    (`asteroid.rs:684-714`), which is how the child gets its `ChildOf` for free from Bevy's
    hierarchy — a rock's own render/collider node, not a severed piece.

**Durable key for the chunk's surface (kind, texture, seed): none exists yet, but the exact
field types are on the live asteroid root and can be copied as the design asks:**
- Root asteroid (`With<AsteroidMarker>`) carries, non-optional, at spawn
  (`asteroid.rs:351-358`) and at thaw (`asteroid.rs:670-674`):
  - `AsteroidKind(pub AsteroidKindId)` (`asteroid_kind.rs:112`)
  - `AsteroidTexture(pub AssetRef<Image>)` (`asteroid.rs:415`)
  - `AsteroidSeed(pub u32)` (`asteroid.rs:434`)
- `collect_asteroid_remeshes`'s `q_asteroid` query already resolves `*root`
  (`asteroid_carve.rs:854-862, 882`) but only for `(AsteroidRadius, BodyRadius,
  Option<EntityId>, Option<EntityTypeName>)` — it does not yet fetch Kind/Texture/Seed, but
  it is the same query on the same entity, so extending that tuple is in-scope, mechanical
  work, not a new system.
- So `RockChunkSurface { kind: AsteroidKindId, texture: AssetRef<Image>, seed: u32 }` (no
  `Option`s — every live rock has all three) is buildable and copyable onto a chunk at spawn
  in `throw_severed_pieces`/`collect_asteroid_remeshes`. This part is unblocked and matches
  item 1 of the approved plan.

## Stop condition hit

Per TRANSIENT-GATE.md section 12, "T3 names":
> `thaw_rock_chunk(...)` is approved only if it rebuilds the mesh, collider and material
> through existing deferred spawn or observer systems. If it must mutate `Assets<Mesh>` or
> materials, or read resource catalogs directly, stop: revise the signature to take those
> resources (or `&mut World`) and show its call graph before the API is edited.

It must mutate both `Assets<Mesh>` and `Assets<AsteroidSurfaceMaterial>`, and no existing
observer or deferred path reaches a parentless chunk. Stopping here, per Step 0's own
instruction, before touching `asteroid_carve.rs`.

## Call graphs

Before (live carve, for comparison):
```
collect_asteroid_remeshes (Commands, ResMut<Assets<Mesh>>, q_asteroid: AsteroidRadius/BodyRadius/.. on *root)
  -> throw_severed_pieces(commands, meshes, parent{material: cloned live handle}, pieces)
       -> spawn_carved_chunk(commands, ChunkSpawn{ mesh: meshes.add(body.mesh), collider: body.collider (pure) })
       -> commands.entity(spawned).insert(parent.material.clone())   // handle clone, no Assets::add
       -> commands.entity(spawned).insert(RadarOccluder)
```

Proposed (resumed chunk — needs resources `thaw_rock_chunk` does not have):
```
restore_resumed_transients (nova_world_base, has &mut World)
  -> thaw_rock_chunk(commands, record: &FrozenRockChunk)
       -> rebuild Mesh from record.mesh.{positions,normals,indices}        // pure, OK
       -> chunk_collider(&mesh)                                            // pure, OK
       -> meshes.add(mesh)              // BLOCKED: no Assets<Mesh> in scope
       -> asteroid_kind_look(&record.surface.kind)                         // pure, OK
       -> AsteroidSurfaceMaterialExt::new(image, &look, record.surface.seed) // needs AssetServer to resolve texture
       -> materials.add(..)             // BLOCKED: no Assets<AsteroidSurfaceMaterial> in scope
       -> commands.spawn(chunk bundle: CarvedChunkMarker, Mesh3d(handle), MeshMaterial3d(handle),
                          transform, LinearVelocity, AngularVelocity, ChunkGrace::resumed(..) | (RigidBody::Dynamic, collider, GravityAffected),
                          resumed_lifetime(record.lifetime), RadarOccluder)
```

## Options

**A — widen the signature to take the resources directly (recommended):**
```rust
pub fn thaw_rock_chunk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<AsteroidSurfaceMaterial>,
    asset_server: &AssetServer,
    record: &FrozenRockChunk,
) -> Entity
```
Caller (`restore_resumed_transients`, which already holds `&mut World` per section 3) does
`world.resource_scope` or an exclusive-system param pull to get `ResMut<Assets<Mesh>>` /
`ResMut<Assets<AsteroidSurfaceMaterial>>` / `Res<AssetServer>` and passes them through — the
same shape `insert_asteroid_render` already takes as system params, just handed down as
explicit args instead of a Bevy-injected observer. Keeps `thaw_rock_chunk` unit-testable with
a bare `Assets<Mesh>::default()` fixture, no full `App`/`World` needed, matching how this
crate's other pure-ish freeze/thaw helpers are tested.

**B — take `&mut World`:**
```rust
pub fn thaw_rock_chunk(world: &mut World, record: &FrozenRockChunk) -> Entity
```
Simpler call site (one argument from `restore_resumed_transients`), but the test for P-T7
then needs a full `World`/`App` fixture with `AssetPlugin`, `MaterialPlugin::<AsteroidSurfaceMaterial>`,
etc. registered just to call the function once, where option A can construct bare
`Assets<Mesh>`/`Assets<AsteroidSurfaceMaterial>` without a running app.

**Recommendation:** A. Narrower surface, keeps the unit test light, and mirrors the explicit
resource list `insert_asteroid_render` already needs rather than handing the whole `World`
to a leaf function that only touches three resources.

## Not done

No file under my ownership was edited. Items 1-6 of the instructions (the `RockChunkSurface`
component, `FrozenRockChunk`/`FrozenChunkMesh`, `freeze_rock_chunk`, `thaw_rock_chunk`, the
P-T7 test, and the three check commands) are blocked on this signature decision and are not
started.

## Unverified

- Whether `restore_resumed_transients` will want to pull these three resources via
  `SystemState`/`resource_scope` or whether it runs as its own exclusive system with them as
  params directly — that's the caller's shape, out of this worker's file ownership
  (`nova_world_base/src/save/transients.rs`, owned by another worker per the task brief).
- Exact `Mesh` reconstruction call (`Mesh::new(PrimitiveTopology::TriangleList, ..)` +
  `insert_attribute`/`insert_indices`) from `FrozenChunkMesh`'s three `Vec`s — not yet
  written, pending the signature decision above.

## Round 2

Owner approved Option A in TRANSIENT-GATE.md section 12 ("Rock chunk thaw", after this
file's step 0). Implemented items 1-6 against the approved signature. All edits are in
`crates/nova_scenario/src/objects/asteroid_carve.rs` and its own `mod tests`; the prelude
export lines in the same file were updated. No other file touched.

### 1. `RockChunkSurface` (asteroid_carve.rs:1018-1030)

```rust
#[derive(Component, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RockChunkSurface {
    kind: AsteroidKindId,
    texture: AssetRef<Image>,
    seed: u32,
}
```

Fields are private, as named in TRANSIENT-GATE.md section 4.5's own snippet — every reader
and writer (`freeze_rock_chunk`, `thaw_rock_chunk`, the `tests` submodule) lives in this one
module, so no accessor was needed.

Wired onto the live path:
- `Parent` (asteroid_carve.rs:450-457) gained a `surface: RockChunkSurface` field.
- `collect_asteroid_remeshes`'s `q_asteroid` query (asteroid_carve.rs:869-878) now also reads
  `&AsteroidKind, &AsteroidTexture, &AsteroidSeed` off the root, same entity it already
  resolved for `AsteroidRadius`/`BodyRadius` — not a new system, as SW-CHUNK's step 0
  evidence called out.
- The `Parent` literal (asteroid_carve.rs:898-925) now builds `surface` from those three.
- `throw_severed_pieces` (asteroid_carve.rs:584-589) inserts `parent.surface.clone()` on every
  spawned chunk, beside the existing material-clone and `RadarOccluder` insert.
- The two pre-existing `Parent` literals in `tests` (`a_severed_piece_carries_the_rock_it_left`,
  `a_severed_island_still_stops_radio`) were updated to carry a `surface` field too, via a new
  test helper `test_rock_chunk_surface()` (asteroid_carve.rs:1307-1313); both still pass.

### 2. `FrozenRockChunk` / `FrozenChunkMesh` (asteroid_carve.rs:1032-1060)

Exactly the shape named in TRANSIENT-GATE.md section 4.5 and the task brief. No `visibility`
field: grepped every `Visibility`/`Visibility::Hidden` write under `nova_gameplay/src/integrity/`
and `nova_scenario/src/objects/asteroid_carve.rs`/`chunk.rs` — the only hits are
`nova_scenario/src/mining.rs:757` (a mining-beam laser) and UI code, neither of which touches
`CarvedChunkMarker`. `spawn_carved_chunk` (`nova_gameplay/src/integrity/chunk.rs:248-262`)
never inserts a `Visibility`; `Mesh3d` carries no `#[require(Visibility)]` in this bevy version
either (checked `bevy_mesh-0.19.0/src/components.rs:102`), confirmed live by the new test: a
freshly thrown chunk has no `Visibility` component at all (`app.world().get::<Visibility>(chunk)`
is `None`). So there is no hidden/shown state on a chunk to lose, and none was added.

`serde` is gated behind the crate's own `#[cfg_attr(feature = "serde", ...)]`, matching
`FrozenRoundFlight` (`nova_gameplay/src/rounds.rs:190-191`) and `FrozenAsteroid`
(`nova_scenario/src/objects/asteroid.rs:443-444`).

### 3. `freeze_rock_chunk` (asteroid_carve.rs:1096-1163, including the private helper
`frozen_chunk_mesh` at 1062-1094)

```rust
pub fn freeze_rock_chunk(world: &World, entity: Entity) -> Result<FrozenRockChunk, TransientFreezeFault>
```

Reads `RockChunkSurface`, `Transform`, `LinearVelocity`, `AngularVelocity`, `Mesh3d`, and the
mesh out of `world.resource::<Assets<Mesh>>()`; panics by name on any gap, per the task's
instruction (not a save-time condition on a real chunk). `grace` is
`world.get::<ChunkGrace>(entity).map(ChunkGrace::remaining)` — `Some` in grace, `None` once
`land_carved_chunks` (`nova_gameplay/src/integrity/chunk.rs:348-375`) has removed it.

**`Unsettled` never fires today.** Grepped every production site that touches
`CarvedChunkMarker` (`asteroid_carve.rs:1113` query, `nova_world_base/src/save/transients.rs:235`,
`nova_gameplay/src/integrity/spew.rs:839,1119`, `nova_gameplay/src/integrity/chunk.rs`) — nothing
tags a chunk entity with an async `Task` or any other multi-frame marker the way `RoundRake`'s
non-empty `armed` set does for a raking slug. `ChunkGrace` is a plain countdown, not a process in
flight. So `freeze_rock_chunk` always returns `Ok`; the `Result` return type is kept only because
TRANSIENT-GATE.md section 12's "T3 names" decision says every T3 freeze returns one, for a
uniform caller. Reported here per this file's own earlier flag, rather than silently narrowing
the signature.

### 4. `thaw_rock_chunk` (asteroid_carve.rs:1210-1266, plus the private helper
`rebuild_chunk_mesh` at 1166-1187)

```rust
pub fn thaw_rock_chunk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<AsteroidSurfaceMaterial>,
    asset_server: &AssetServer,
    record: &FrozenRockChunk,
) -> Entity
```

Exactly the owner-approved signature. Call graph:

```
thaw_rock_chunk(commands, meshes, materials, asset_server, record)
  -> rebuild_chunk_mesh(&record.mesh)                       // pure; panics on bad lengths/empty/OOB index
  -> chunk_collider(&mesh)                                  // pure; panics (via unwrap_or_else) if None
  -> asteroid_kind_look(&record.surface.kind)                // pure; panics if unknown
  -> record.surface.texture.resolve(asset_server)            // same call insert_asteroid_render makes
  -> AsteroidSurfaceMaterialExt::new(image, &look, seed)      // same helper insert_asteroid_render uses
  -> spawn_carved_chunk(commands, ChunkSpawn { mesh: meshes.add(mesh), .. })
  -> commands.entity(entity).insert((surface.clone(), MeshMaterial3d(materials.add(material)), RadarOccluder))
  -> match record.grace {
       Some(remaining) => insert(ChunkGrace::resumed(collider, remaining)),
       None => remove::<ChunkGrace>() + insert((RigidBody::Dynamic, collider, GravityAffected)),
     }
```

No new helper was needed in `asteroid.rs`: `asteroid_kind_look` and
`AsteroidSurfaceMaterialExt::new` are already both `pub`, so `thaw_rock_chunk` calls them
directly, same as `insert_asteroid_render` does (`asteroid.rs:829-841`). Nothing in `asteroid.rs`
was touched.

`spawn_carved_chunk` is called as the task suggested: it hands back an undressed chunk already
carrying a *fresh* `ChunkGrace::new(..)`, `RigidBody::Kinematic` and a fresh
`TempEntity(CHUNK_LIFETIME_SECS)`. The grace is immediately overridden per the match above. The
`TempEntity`/lifetime is left alone deliberately — verified against
`nova_world_base/src/save/transients.rs:413-426` (`spawn_resumed`), which calls `thaw_round`
then unconditionally `.insert(resumed_lifetime(transient.lifetime))` afterward; the lifetime a
production thaw spawns with is always overwritten by the save collector, for rounds today and,
once that file's own `FrozenTransientType` grows a `RockChunk` arm (not in this worker's
ownership), for chunks the same way. This is what the task's "minus lifetime state: the save
collector inserts resumed_lifetime, as for rounds" meant — confirmed, not assumed.

An unknown kind panics (`asteroid_kind_look(..).unwrap_or_else(|| panic!(..))`), deliberately
different from `insert_asteroid_render`'s `error!`-and-return-silently branch
(`asteroid.rs:829-836`): the task's item 4 says "An unknown kind PANICS," and a save never keeps
an id the live game didn't already validate, so a gap here is corrupt/hand-edited data rather
than a live-authoring mistake a player can hit mid-game.

### 5. P-T7 `a_resumed_rock_chunk_has_its_mesh_material_and_grace` (asteroid_carve.rs:1677-1872)

Carves a real chunk through the real `throw_severed_pieces` (the exact function
`a_severed_piece_carries_the_rock_it_left` already exercises, not a reimplementation), freezes
it, RON round-trips the record, despawns the original, thaws it, and asserts:
- positions, normals, indices equal (via the private `frozen_chunk_mesh` helper applied to both
  the live and the thawed mesh);
- the collider is rebuilt with the same bounds — compared via
  `nova_gameplay::integrity::chunk::mesh_bounds` (a `pub fn`, already used by `chunk_collider`
  itself) on the live and thawed meshes, since `chunk_collider` is pure and the mesh data is
  already asserted equal, so this is checking the real pipeline input rather than re-deriving
  the claim from nothing;
- the material inputs equal the live surface's own kind look / seed / texture path (the live
  chunk in this test wears no material, by the same `material: None` headless path
  `a_severed_island_still_stops_radio` uses — so the comparison is against the SAME
  `RockChunkSurface` value the live chunk was thrown with, which is the thing that actually
  drives the material, rather than against a materialised `AsteroidSurfaceMaterial` that would
  need `AsteroidSurfaceUniform: PartialEq`, which it does not derive and which this worker is
  not permitted to add in `asteroid_surface.rs`, outside this file's ownership);
- visibility equal — both `None` (see item 2 above);
- grace remaining equal, in-grace case.

A second **landed-chunk case** reuses the same frozen record with `grace: None` (struct-update
syntax, same module so the private field is reachable) and asserts `RigidBody::Dynamic`, a real
`Collider` component, and no `ChunkGrace`.

`println!("a rock chunk record is {} RON bytes", ron.len())` logs the measurement, no assertion,
per the task. Real run (`--nocapture`, nothing else filtered):
```
a rock chunk record is 20772 RON bytes
test objects::asteroid_carve::tests::a_resumed_rock_chunk_has_its_mesh_material_and_grace ... ok
```
That number is for the smallest test fixture (a 16-cell island, one severed piece); a shipped
rock's field resolution goes up to 40 per axis (`FIELD_RESOLUTION_MAX`, asteroid_carve.rs:126),
so a real chunk's own mesh is unrelated to the grid resolution (it is the severed ISLAND's own
triangle count, already much smaller than the parent field) but will still vary per piece size —
this number is a floor, not a typical value, and nobody downstream asked for a typical one yet.

Mutation this test is designed to catch (not run — per this worker's brief, the main worker runs
it): deleting the `match record.grace { .. }` block's `None` arm (i.e. always re-inserting
`ChunkGrace::resumed` regardless of `record.grace`, or always leaving the fresh
`ChunkGrace::new(..)` that `spawn_carved_chunk` itself inserted) makes the landed-chunk case's
`assert!(app.world().get::<ChunkGrace>(landed).is_none())` fail, since the thawed landed chunk
would still carry a grace.

### 6. Check commands — real output

`cargo test -p nova_scenario --lib asteroid_carve` (`-j 8`):
```
running 7 tests
test objects::asteroid_carve::tests::a_rock_is_gridded_in_world_units_until_the_cap_binds ... ok
test objects::asteroid_carve::tests::a_severed_piece_carries_the_rock_it_left ... ok
test objects::asteroid_carve::tests::a_severed_island_still_stops_radio ... ok
test objects::asteroid_carve::tests::a_resumed_rock_chunk_has_its_mesh_material_and_grace ... ok
test objects::asteroid_carve::tests::the_seeded_field_reproduces_the_shipped_silhouette ... ok
test objects::asteroid_carve::tests::a_carved_rock_never_grows ... ok
test objects::asteroid_carve::tests::an_exhausted_authored_rock_removes_its_gravity_marker_and_id ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 463 filtered out
```

`cargo test -p nova_scenario --lib asteroid` (`-j 8`): `test result: ok. 44 passed; 0 failed; 0
ignored; 0 measured; 426 filtered out` (full list includes every `objects::asteroid*` and
related lint/spawn/loader test; all `ok`).

`cargo check -p nova_world_base --tests` (`-j 8`): clean —
```
Checking nova_world_base v0.15.0 (.../crates/nova_world_base)
Finished `dev` profile [optimized + debuginfo] target(s) in 25.07s
```
(one unrelated pre-existing warning: "the following packages contain code that will be rejected
by a future version of Rust: proc-macro-error2 v2.0.1" — a transitive dependency lint, not from
this change).

Ran `nix develop --command cargo fmt -p nova_scenario` after the edits (allowed by this
worker's brief); re-ran the `asteroid_carve` test target afterward to confirm formatting changed
nothing behaviourally — still 7/7 `ok`.

### Not done / out of scope

- `nova_world_base/src/save/transients.rs`'s `FrozenTransientType` still has only a `Round(..)`
  arm (confirmed at transients.rs:58-60, read-only) — adding `RockChunk(FrozenRockChunk)` there,
  and the `spawn_resumed`/`freeze_live_transients` wiring for it, is that file's owner's work,
  not touched here.
- `RockChunkSurface`, `FrozenRockChunk`, `FrozenChunkMesh`, `freeze_rock_chunk` and
  `thaw_rock_chunk` are exported from `asteroid_carve::prelude` (asteroid_carve.rs:85-95) so that
  owner can reach them without a direct module path.

### Unverified

- Whether a shipped (not test-fixture) rock's severed-piece mesh ever exceeds a size where the
  20772-byte-per-test-piece RON figure above becomes a save-size concern across "up to 24
  landings per frame" (TRANSIENT-GATE.md section 4.5's own "Unmeasured: save size" note) — this
  worker measured one small fixture piece only, as the task asked; a real-scenario measurement
  needs the real carve pipeline end to end, out of this worker's scope.
