# SW-PIECE: step 0 result for 4.6 (detached section pieces) - STOP

Worker scope: `crates/nova_ship/src/sections/frozen_piece.rs` (new) + its
tests, the `mod`/prelude lines for it in `sections/mod.rs`, and
`damage_cracks.rs`'s `mark_wreck_cracks` only.

No edit was made. Step 0 (TRANSIENT-GATE.md 4.6) finds the gate's own stop
condition true: `thaw_detached_piece(commands: &mut Commands, ...)` cannot
rebuild every `FrozenArtType`. Reporting the signature and stopping, per the
task's explicit instruction.

## Step 0 checklist

### Can `Commands` alone rebuild every `FrozenArtType`? No.

- **`Group`, `Light`, `Exhaust`, `Decor`: yes, `Commands` alone is enough.**
  - `Light`: a `PointLight` insert, nothing else.
  - `Exhaust`: inserting `ThrusterExhaustConfig` is enough; its own observer
    `insert_thruster_shader` (`crates/nova_ship/src/sections/thruster_section.rs:814-822`)
    pulls `ResMut<Assets<Mesh>>`, `ResMut<ExhaustMeshes>`, `ResMut<ExhaustMaterials>`
    and `ResMut<Assets<ThrusterPlumeMaterial>>` itself. The thaw does not
    touch those resources directly.
  - `Decor`: inserting `ShipDecorMarker(AssetRef::Path(asset))` is enough;
    its own observer `dress_skin_decor` (`crates/nova_ship/src/sections/skin_decor.rs:639-652`)
    resolves the ref with its own `Res<AssetServer>` and inserts
    `WorldAssetRoot` itself. (This contradicts the gate's one-line claim
    "Decor gets `WorldAssetRoot` only" at thaw - the real path gets
    `ShipDecorMarker`, and the observer derives `WorldAssetRoot`. Noted, not
    a stop by itself.)

- **`Scene`: no.** The design has thaw do
  `WorldAssetRoot(asset_server.load(asset))` directly
  (TRANSIENT-GATE.md:448-449), the same as `insert_hull_section_render`
  does with a resolved `AssetRef`
  (`crates/nova_ship/src/sections/hull_section.rs:105-118`). That needs
  `&AssetServer`, which `Commands` does not give.

- **`Placeholder`: no.** Thaw needs the shared mesh/material handles on
  `PlaceholderArt` (`crates/nova_ship/src/sections/placeholder_art.rs:54-75`),
  a resource, not reachable through `Commands`.

- **`SkinPlate`: no, for two independent reasons.**
  1. It needs real resources: `hang_surfaces`
     (`crates/nova_ship/src/sections/shell_skin.rs:1092-1110`) takes
     `assets: &mut SkinAssets, meshes: &mut Assets<Mesh>, materials: &mut
     Assets<StandardMaterial>` beside `&mut Commands`. The style lookup
     also needs `Res<GameStyles>::get_style(&str)`
     (`crates/nova_ship/src/sections/skin_style.rs:559-566`) to turn the
     saved style id back into a `&ShipStyleConfig` for `hang_surfaces`'s
     `style` argument.
  2. `hang_surfaces` is a private `fn` in `shell_skin.rs`
     (`crates/nova_ship/src/sections/shell_skin.rs:1092`), a file this
     worker does not own. Calling it from `frozen_piece.rs` needs its
     owner to raise its visibility (`pub(crate)` or similar) first.

This matches the rock-chunk precedent exactly: the owner already approved
`thaw_rock_chunk` taking explicit `&mut Assets<Mesh>`, `&mut
Assets<AsteroidSurfaceMaterial>`, `&AssetServer`
(TRANSIENT-GATE.md:906-923) instead of bare `Commands`, with `spawn_resumed`
borrowing them once through a `SystemState` over the whole loop
(TRANSIENT-GATE.md:916-917; `crates/nova_world_base/src/save/transients.rs:414-432`
is that same loop today, for rounds only).

### Proposed signature (option A shape, STOP - do not implement without approval)

```rust
// nova_ship/src/sections/frozen_piece.rs
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

`spawn_resumed` (`nova_world_base/src/save/transients.rs:414`, not owned by
this worker) would need to borrow `AssetServer`, `PlaceholderArt`,
`GameStyles`, `SkinAssets`, `Assets<Mesh>`, `Assets<StandardMaterial>`
through one `SystemState` over its loop, the same way its owner already
plans for `thaw_rock_chunk`'s resources.

Call graph, before (today, art dies with the section) and after (proposed):

```
Before:
  detach_destroyed_body (explode.rs)
    -> spawn piece root, reparent children, strip Collider, despawn source
       (no capture, no thaw: art rides the existing entities as-is)

After:
  capture:
    freeze_detached_piece(world: &World, piece: Entity)
        -> Result<FrozenDetachedPiece, TransientFreezeFault>
      walks the piece tree, classifies each node into FrozenArtType
      (reads AssetServer::get_path, PlaceholderArt handles, ShipSkinMarker,
       ShipDecorMarker, ThrusterExhaustConfig, RailgunChargeGlowMarker
       through `world.resource::<_>()` / `world.get::<_>()`, read-only)

  thaw (spawn_resumed, not owned here):
    SystemState<(Commands, Res<AssetServer>, Res<PlaceholderArt>,
                 Option<Res<GameStyles>>, ResMut<SkinAssets>,
                 ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>)>
      -> thaw_detached_piece(&mut commands, &asset_server, &placeholder,
                              styles.as_deref(), &mut skin_assets,
                              &mut meshes, &mut materials, record)
           -> spawns root with ChunkGrace::resumed(..) or plain Collider,
              spawns nodes in parent order, Scene nodes also get
              ResumedScenePoses for apply_resumed_scene_poses to consume
              on WorldInstanceReady
```

### Other step-0 answers (reported regardless of the stop)

- **Drop the `lifetime: SavedLifetime` param.** Precedent confirms the T1
  pattern holds: `thaw_round(commands: &mut Commands, round: &FrozenRound,
  owner: Entity) -> Entity` (`crates/nova_ship/src/sections/frozen_rounds.rs:104`)
  already takes no lifetime param, and `spawn_resumed` inserts
  `resumed_lifetime(transient.lifetime)` on the returned entity right after
  calling it (`crates/nova_world_base/src/save/transients.rs:420-425`).
  `thaw_detached_piece` should match: `(commands: &mut Commands, ..., record:
  &FrozenDetachedPiece) -> Entity`, with the lifetime applied by the same
  caller the same way.
- **`ChunkGrace::resumed` takes the piece collider from
  `SectionCollider::to_collider()`: yes, confirmed by type.**
  `ChunkGrace::resumed(collider: Collider, remaining: f32) -> Self`
  (`crates/nova_gameplay/src/integrity/chunk.rs:202-207`, already exists,
  per the task brief) takes an avian `Collider`, and
  `SectionCollider::to_collider(self) -> Collider`
  (`crates/nova_ship/src/sections/base_section.rs:91`) produces exactly
  that type from the stamped `DetachedPieceSource.collider` field. No gap.
- **Does `freeze_detached_piece` need `&World` access to `AssetServer` for
  `get_path`: no extra gap.** `world: &World` already supports
  `world.resource::<AssetServer>().get_path(handle)` for the `Scene` case,
  and `world.get::<PlaceholderArt>()`-style resource reads for the
  `Placeholder` match. The proposed `freeze_detached_piece(world: &World,
  piece: Entity) -> Result<FrozenDetachedPiece, TransientFreezeFault>`
  signature (T3 names, TRANSIENT-GATE.md:891) is sufficient for capture.
  Capture is read-only, so this is the one half of 4.6 that is NOT blocked
  by the resource problem below - only the thaw half is.

## What is needed before this worker can proceed

1. Owner decision on the `thaw_detached_piece` signature above (or an
   alternative shape).
2. A visibility change to `hang_surfaces`
   (`crates/nova_ship/src/sections/shell_skin.rs:1092`) by its owner, since
   this worker does not have that file.
3. Confirmation that `spawn_resumed`'s owner (`nova_world_base`, also not
   this worker's file) can supply the six-resource `SystemState` the new
   signature needs, consistent with what it already must build for
   `thaw_rock_chunk`.

No type, fn, or test from 4.6/P-T10 was added. `mark_wreck_cracks` was not
started: its only caller is inside the blocked `Scene` thaw path
(`apply_resumed_scene_poses`), so writing it now would be code with no
caller to prove it against.

## Round 2

### Claim

The approved design (4.6, 9/P-T10, 12) is implemented and tested in
`crates/nova_ship/src/sections/frozen_piece.rs` (new, 1492 lines). The two
owner amendments mid-round are both in: `FrozenDetachedPiece::validate` and
a local `visibility_serde` fix. `sections/mod.rs` carries only the three
approved lines. `damage_cracks.rs` and `shell_skin.rs` are unchanged this
round (both landed in a prior round of this same task).

### Owner amendments this round

1. **`FrozenDetachedPiece::validate(&self) -> Result<(), String>`**
   (`frozen_piece.rs:74-137`), approved via TRANSIENT-GATE.md section 12
   "Transient integration (owner, 2026-10-09)". Rejects a non-finite
   translation/rotation/linear/angular/center_of_mass, a non-finite or
   negative grace, a node whose `parent` index is not less than its own
   index, a non-finite node transform, and a non-finite light value. Each
   error names the fault in plain words (example:
   `"a detached piece's node 4 names the parent 7, which comes after it"`).
   No new test for it, per the amendment; `run_case` does call it on every
   freshly frozen record (`frozen_piece.rs:1130`) as a cheap sanity check,
   not a dedicated proof.
2. **Visibility serde fix**, reported by the owner mid-round:
   `FrozenArtNode.visibility: Visibility` has no serde impl of its own.
   Fixed with a local `visibility_serde` module (`frozen_piece.rs:158-185`),
   a remote-derive shadow enum copied from `nova_world/src/frozen.rs`'s
   `visibility_serde` pattern (`crates/nova_world/src/frozen.rs:325-355`)
   and adapted for a plain (non-`Option`) field - the field now carries
   `#[cfg_attr(feature = "serde", serde(with = "visibility_serde"))]`
   (`frozen_piece.rs:153`). `nova_world` was not touched. Verified green at
   the time: `cargo check -p nova_ship --features serde` and
   `cargo check -p nova_ship` (both via `nix develop`,
   `CARGO_TARGET_DIR=./target`, `-j 8`), reconfirmed again at the end of
   this round (see Verification below).
3. **Code comments name no task, report or section number.** Checked by
   re-reading every doc comment added this round; all give the reason in
   plain words.

### Evidence - signatures as written

```rust
// frozen_piece.rs:44
pub struct FrozenDetachedPiece { /* private fields */ }

// frozen_piece.rs:61
pub fn style(&self) -> Option<&str>

// frozen_piece.rs:74
pub fn validate(&self) -> Result<(), String>

// frozen_piece.rs:145
pub struct FrozenArtNode { /* private fields */ }

// frozen_piece.rs:190
pub enum FrozenArtType { Group, Scene { asset, cracked, poses }, Placeholder(PlaceholderArtType),
                          SkinPlate(ShellShape), Decor { asset }, Exhaust(ThrusterExhaustConfig),
                          Light { color, intensity, range, radius } }

// frozen_piece.rs:238
pub enum PlaceholderArtType { Body, ControllerBody, Window, Barrel, Nozzle, TurretPlate }

// frozen_piece.rs:260
pub struct DetachedPieceSource { collider: SectionCollider, style: Option<String> }

// frozen_piece.rs:274
pub(crate) fn stamp_detached_piece_source(
    add: On<Add, DetachedPieceMarker>,
    q_marker: Query<&DetachedPieceMarker>,
    q_stamped: Query<&DetachedPieceSource>,
    q_collider: Query<&SectionCollider>,
    q_child_of: Query<&ChildOf>,
    q_style: Query<&ShipStyle>,
    mut commands: Commands,
)

// frozen_piece.rs:333
pub fn freeze_detached_piece(world: &World, piece: Entity) -> Result<FrozenDetachedPiece, TransientFreezeFault>

// frozen_piece.rs:625
pub(crate) fn apply_resumed_scene_poses(
    ready: On<WorldInstanceReady>,
    q_pending: Query<&ResumedScenePoses>,
    q_children: Query<&Children>,
    q_name: Query<&Name>,
    mut q_transform: Query<&mut Transform>,
    q_mesh: Query<(), With<MeshMaterial3d<StandardMaterial>>>,
    mut commands: Commands,
)

// frozen_piece.rs:658
pub(crate) struct ResumedScenePoses(Vec<(Vec<String>, Transform)>, bool);

// frozen_piece.rs:672
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

Matches the Round 1 proposed signature exactly, including dropping the
`lifetime` parameter.

### Call graph (as built)

```
Live detach (unchanged, explode.rs - not owned here):
  detach_destroyed_body
    -> spawn piece root (DetachedPieceMarker(source), ...)
    -> On<Add, DetachedPieceMarker> fires: stamp_detached_piece_source
         reads SectionCollider + ShipStyle (ancestor walk) off `source`,
         which is still alive -> inserts DetachedPieceSource on the piece
    -> reparent source's children onto the piece, stripping Collider
    -> despawn source

Save:
  freeze_detached_piece(world, piece)
    -> reads Name/Transform/LinearVelocity/AngularVelocity/CenterOfMass/
       DetachedPieceSource (panics if any is missing - a stamped piece
       always has them) and ChunkGrace::remaining() if present
    -> collect_art_nodes walks Children depth-first, parent before child,
       skipping ParticleEffect and SkinSurfaceMarker
         -> classify_art_node per child: ShipDecorMarker -> Decor,
            ShipSkinMarker -> SkinPlate, ThrusterExhaustConfig -> Exhaust,
            PointLight -> Light, WorldAssetRoot -> Scene (collect_scene_poses
            + any_descendant_cracked), Mesh3d+MeshMaterial3d -> Placeholder
            (classify_placeholder matches against PlaceholderArt handles),
            else -> Group
    -> Result<FrozenDetachedPiece, TransientFreezeFault> (Unsettled is never
       returned - a detached piece has no multi-frame process left)

Resume (caller not owned here, matches thaw_round's pattern):
  SystemState<(Commands, Res<AssetServer>, Res<PlaceholderArt>,
               Option<Res<GameStyles>>, ResMut<SkinAssets>,
               ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>)>
    -> thaw_detached_piece(...) -> spawns root (ChunkGrace::resumed or a
         plain Collider + GravityAffected), then each node in parent order:
           Scene -> WorldAssetRoot(asset_server.load(asset)) + ResumedScenePoses
           Placeholder -> Mesh3d + MeshMaterial3d from PlaceholderArt
           SkinPlate -> hang_surfaces(.., bare node, no ShipSkinMarker)
           Decor -> ShipDecorMarker(AssetRef::Path(asset))
           Exhaust -> the config directly
           Light -> a PointLight
    -> caller inserts resumed_lifetime(saved) on the returned entity
    -> later, for a Scene node: On<WorldInstanceReady> fires:
         apply_resumed_scene_poses writes every saved pose back by name
         path, burns wreck cracks on every mesh descendant if `cracked`,
         removes ResumedScenePoses
```

### Registration (sections/mod.rs, my three lines only)

- `pub mod frozen_piece;`
- `frozen_piece::prelude::*,` in the crate's `sections::prelude`
- `app.add_observer(frozen_piece::stamp_detached_piece_source);` and
  `app.add_observer(frozen_piece::apply_resumed_scene_poses);` inside
  `SpaceshipSectionPlugin::build`, unconditional (not render-gated), with a
  one-line comment saying why in plain words.

Confirmed by `git diff -- crates/nova_ship/src/sections/mod.rs`: only these
lines changed; no other worker's lines were touched.

### Test: P-T10, `a_detached_piece_resumes_as_the_art_it_wore`

One `#[test]` (`frozen_piece.rs:1041`) looping over 9 cases, one per
`SectionKind` (hull, thruster, controller, turret, railgun, docking -
`PlaceholderArt`; cargo_intake, mining, torpedo - a real scene). Each case:

- Builds a `ship` with `ShipStyle(Some("raider"))` and a `source` section
  child carrying `SectionMarker`, `Health`, `SectionCollider` and its
  avian `Collider` - the state that must NOT survive the detach.
- Gives `source` one art-root child (the kind's placeholder mesh/material,
  or a real `WorldAssetRoot` loaded from the shipped asset tree), one real
  skin plate (`plate_body`) and one real decoration (`decor_body`), through
  a real `ShipSkinPlugin { render: true }` so `dress_skin_plate` and
  `dress_skin_decor` dress them for real.
- Runs the app so the source's own scene (for the 3 scene cases) spawns for
  real, then poses a REAL named door node off its authored rest
  (`intake_slat_l0`, `stow_lid_right`, `door_petal_0` - all read directly
  out of the shipped `.glb` files, not invented names) - the state the test
  later proves survives the freeze/thaw round trip.
- Detaches by hand (spawn the piece, reparent the 3 fixtures off `source`
  stripping their `Collider`, despawn `source`) in the same order
  `detach_destroyed_body` uses, so the real `stamp_detached_piece_source`
  observer fires and is checked BEFORE the freeze.
- Freezes, validates, round-trips through RON behind `feature = "serde"`,
  printing the byte size (no assertion on it).
- Thaws through the real `thaw_detached_piece` via a `SystemState` (the
  same resource set `spawn_resumed` would build), inserts
  `resumed_lifetime`, then runs the app so a real `WorldInstanceReady`
  fires on the thawed scene node and `apply_resumed_scene_poses` writes
  the door's pose back.
- Asserts: node count and classification; the scene's own asset path; the
  door's pose differs from rest before the freeze and is written back,
  bit-for-bit, after the real ready event; plate surface count matches and
  at least one surface reads the dyed "raider" colour (floor is never
  dressed by design, so not every surface is checked); the decor resolves
  the same asset path through a real `WorldAssetRoot`; grace and lifetime
  both survive within float tolerance; and that neither the thawed root nor
  any of its descendants carries `Health`, `SectionFixture`,
  `SectionMarker`, a `Collider`, `TorpedoSectionSpawnerFireState`, or (by
  component name, since the real type is private to `turret_section`) a
  `TurretJointMarker`.

**Real asset loading, headless, no window or GPU.** The owner's constraint
was to load the real authored scene through the `AssetServer`, with the
real spawner plugin raising `WorldInstanceReady` itself, and to stop and
report if that cannot be done headless. It can: `bevy_gltf`'s own test
suite proves the minimal plugin set
(`bevy_gltf-0.19.1/src/loader/mod.rs:2124-2141`, its `test_app`) -
`AssetPlugin`, `WorldSerializationPlugin`, `MeshPlugin`, `GltfPlugin` - with
no `ImagePlugin`, `PbrPlugin` or `RenderPlugin`. `real_app()`
(`frozen_piece.rs:813-847`) copies that set, adds `TransformPlugin` and the
real `ShipSkinPlugin`, points `AssetPlugin.file_path` at `../../assets`
(the real shipped tree, relative to `crates/nova_ship`), and sets
`meta_check: Never` since the test does not ship `.meta` sidecars. The three
scene assets are real, shipped files:
`assets/base/gltf/intake_accordion_3x2x1.glb`,
`assets/base/gltf/mining_beam_compact.glb`, `assets/base/gltf/bay_tube.glb`
(the torpedo case authors a `render_mesh` to force Scene art; a bare
`TorpedoSectionConfig::default()` wears a placeholder).

### Test output

```
cargo test -p nova_ship --lib --features serde frozen_piece   (nix develop, CARGO_TARGET_DIR=./target, -j 8)

running 1 test
case hull: frozen piece RON is 836 bytes
case thruster: frozen piece RON is 842 bytes
case controller: frozen piece RON is 852 bytes
case turret: frozen piece RON is 845 bytes
case railgun: frozen piece RON is 839 bytes
case docking: frozen piece RON is 839 bytes
case cargo_intake: frozen piece RON is 4913 bytes
case mining: frozen piece RON is 2618 bytes
case torpedo: frozen piece RON is 3348 bytes
test sections::frozen_piece::tests::a_detached_piece_resumes_as_the_art_it_wore ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1113 filtered out
```

```
cargo check -p nova_ship --tests                               -> Finished, clean
cargo check -p nova_ship --features serde --tests               -> Finished, clean
cargo test -p nova_ship --lib --features serde shell_skin       -> 23 passed; 0 failed
cargo test -p nova_ship --lib --features serde damage_cracks    -> 13 passed; 0 failed
cargo check -p nova_world_base --tests                          -> SKIPPED (owner instruction: the
                                                                     main worker's nova_world_base
                                                                     edit does not compile yet)
```

All runs via `nix develop --command ... cargo ...`, `CARGO_TARGET_DIR=./target`, `-j 8`. No staging,
commit, or push. `cargo fmt -p nova_ship` run once; `git diff --stat` confirms
only `frozen_piece.rs` and my three `sections/mod.rs` lines changed shape.

### Named mutations (not run - each should break P-T10)

1. **Drop `poses` on thaw** (skip inserting `ResumedScenePoses`, or insert it
   empty) in `thaw_detached_piece`'s `Scene` arm (`frozen_piece.rs:~722-726`).
   Breaks the door-pose assertion on the `cargo_intake`, `mining` and
   `torpedo` cases: the resumed door stays at the glb's authored rest
   instead of the posed-open transform the test set before freezing.
2. **Drop the stamp**: make `stamp_detached_piece_source` a no-op (or skip
   registering it). Breaks every case: `freeze_detached_piece` panics
   immediately (`piece {piece:?} carries no DetachedPieceSource`) before any
   assertion runs.
3. **Decor via `WorldAssetRoot` directly**: change `thaw_detached_piece`'s
   `Decor` arm to insert `WorldAssetRoot(asset_server.load(asset))` instead
   of `ShipDecorMarker(AssetRef::Path(asset))`. Breaks nothing about the
   FINAL state (the test only checks the end result, which would still
   carry `WorldAssetRoot`) but breaks the "decor resolves through
   `dress_skin_decor`" design point; to make this mutation provable, the
   test would need to additionally assert `ShipDecorMarker` is present
   immediately after `thaw_detached_piece` returns, before any `app.update`.
   Flagging this as a real gap rather than silently calling the mutation
   proven - see Unverified.
4. **Reinstate `ShipSkinMarker` on a thawed plate** (insert it alongside
   calling `hang_surfaces`). Breaks nothing visible either, for the same
   reason as (3): the test tells the plate apart from the art/decor
   children by elimination, not by the marker's absence. Also flagged in
   Unverified.
5. **Classify `cracked` as always `true`** (hardcode it in
   `classify_art_node`'s `WorldAssetRoot` arm instead of calling
   `any_descendant_cracked`). Not caught by P-T10: no case in this test
   ever marks a scene descendant with `SectionCracksMaterial`, so every
   case's real `cracked` value is already `false` and a hardcoded `true`
   would silently diverge. Named in Not done below.

### Not done

- No case exercises `cracked: true` (a scene that was already showing
  section cracks when it froze). Mutation 5 above is not caught by this
  test as a result.
- Mutations 3 and 4 are not actually falsified by this test's assertions
  (see above) - the test proves the END state is correct, not that it was
  reached through the approved intermediate path (`ShipDecorMarker` then
  `dress_skin_decor`, and a bare node for a plate). A tighter proof would
  assert the intermediate state (right after `thaw_detached_piece` returns,
  before the first `app.update()`).
- The `turret` case's "no `TurretJointMarker`" check is real code (a string
  match against every component name on the thawed tree) but the fixture
  never had a `TurretJointMarker` to begin with - this test's turret
  fixture is a placeholder-art stand-in, not the real jointed
  `turret_section` builder output, so the check cannot catch a regression
  that reinstates one; it only proves `thaw_detached_piece` itself never
  emits that name.
- No case covers `FrozenArtType::Light` end to end (a railgun's real charge
  glow uses a private `RailgunChargeGlowMarker` this worker cannot
  construct outside `railgun_section`); `classify_art_node`'s `PointLight`
  branch and `thaw_detached_piece`'s `Light` arm are exercised only by
  `cargo check`, not by this test.
- `FrozenDetachedPiece::validate` has no dedicated test, per the owner's
  amendment; `run_case` calls it once per case as a smoke check only.

### Unverified

- Whether `ShipDecorMarker`/bare-node-for-`SkinPlate` is load-bearing for
  any OTHER reason than style/dress timing (e.g. save-file size, or a
  future observer keyed on the marker's absence) was not re-derived this
  round; Round 1's reading of section 4.6's own text is the basis for both
  choices.
- The two mid-round owner amendments (the `validate` addition and the
  `visibility_serde` fix) are included in this report as instructed; no
  further owner communication happened this round beyond the test-design
  question (whether to exercise live `WorldInstanceReady` or scope the test
  down - owner chose the former, with constraints, which this round's test
  follows).

## Round 3

### Claim

P-T10 (`a_detached_piece_resumes_as_the_art_it_wore`) is green for all 9
cases. Two bugs found in my own test fixture (not in owned production code)
are fixed: a railgun instant-fire default and a headless glb-material gap
that made the real `cracked` assertion on scene art unprovable as written.
Per owner instruction, the real-glb cases now assert `cracked` as what is
real (always `false`, headless), and a new standalone test,
`a_real_section_crack_pipeline_marks_an_in_memory_scene_mesh`
(`frozen_piece.rs:1929-2156`), proves the real crack pipeline end to end
through an in-memory scene mesh instead. `sections/mod.rs` and all other
owned files are unchanged this round; only `frozen_piece.rs`'s `mod tests`
was edited.

### Bug 1: railgun charge glow never observed charging

**Evidence.** `RailgunSectionConfig::default().charge_seconds == 0.0`
(`railgun_section/mod.rs:194-198`, documented as an instant-fire test rig).
My P-T10 railgun case set `RailgunCharge::Charging { elapsed: 0.0 }` by hand
on the default config; `RailgunCharge::progress` returns `1.0` whenever
`charge_seconds <= 0.0`, so the real `charge_and_fire_railgun` system fired
and reset to `Ready` within the same `app.update()`, before
`drive_railgun_charge_glow` (an `Update`-schedule system) ever lit the glow.

**Fix.** The railgun case now authors `charge_seconds: 5.0` instead of the
default (`frozen_piece.rs:1020`), with a comment citing the source line.
This is a test-fixture change only; `railgun_section`'s own code is untouched
and correct for its own documented purpose.

### Bug 2: a headless app can never crack a glb-sourced scene mesh

**Evidence - root cause.** The cargo_intake case's `cracked` assertion read
`left: false, right: true` even after 70.0 damage and 15 real
`app.update()`s. Traced through vendored source (read only, not edited):

- `bevy_gltf`'s own node-spawning code inserts `Mesh3d` and transform/name
  components on a glb's mesh nodes but not a material - marked
  `// TODO: could add the \`GltfMaterial\` here`
  (`bevy_gltf-0.19.0/src/loader/mod.rs:1660-1743`). The material only
  arrives through an `extension.on_spawn_mesh_and_material(...)` hook
  (same file, line 1743).
- That hook is implemented by a private `struct GltfExtensionHandlerPbr;`
  (`bevy_pbr-0.19.1/src/gltf.rs:102`, no `pub`), registered only by
  `pub(crate) fn add_gltf(app: &mut App)` (same file, lines 13-28).
- `add_gltf` is called only from inside `PbrPlugin::build`
  (`bevy_pbr-0.19.1/src/lib.rs:255-258`), which needs a render world - out
  of scope per the owner's explicit "do not add a render-world plugin."

So a headless app's glb-sourced scene mesh never carries
`MeshMaterial3d<StandardMaterial>`. Without that component,
`mark_section_meshes` (`damage_cracks.rs`) never tracks the mesh, so no
amount of pre-kill damage can mark it cracked - the real `cracked: false`
was the correct live state all along, not a bug in owned production code.

**Fix, per owner instruction (option 1 timeboxed, option 2 added).** Option
1 (one test-app line) does not exist: the fix point is a private type behind
a full render plugin. So, kept the real glb cases' freeze-side assertion
honest (`frozen_piece.rs:1443-1485`, now `assert!(!*cracked, ...)` with the
chain above cited inline) and the thaw-side assertion honest the same way
(`frozen_piece.rs:1797-1845`, now asserts zero `SectionCracksMaterial`
meshes instead of the old `expect_cracked` branch, since the same gap means
`apply_resumed_scene_poses` has nothing to hand `mark_wreck_cracks` either).
Added option 2: `a_real_section_crack_pipeline_marks_an_in_memory_scene_mesh`
(`frozen_piece.rs:1929-2156`), which builds a scene mesh node in memory with
a real `MeshMaterial3d<StandardMaterial>` (what a glb's mesh would carry if
`PbrPlugin` had run) inside a hand-built `WorldAsset`, inserted via a
path-backed handle so `freeze_detached_piece`'s `classify_art_node` can save
it, then drives it through the real pipeline:

- Spawns a real section (`section_body`), lets `fit_damage_effects` arm
  `DamageCracks` on it for real.
- Confirms the pristine mesh is already tracked: `SectionCracks` present
  right after `WorldInstanceReady` spawns it (`damage_cracks.rs:431-441`,
  the bucket-0 branch).
- Real `HealthApplyDamage` (70.0) -> `mesh_wears_cracks` reads true.
- Real lethal damage -> real kill -> real `detach_destroyed_body` ->
  `freeze_detached_piece` on the real piece -> asserts
  `FrozenArtType::Scene { cracked: true, .. }`.
- Real `thaw_detached_piece` -> asserts the queued `ResumedScenePoses`
  carries `cracked: true` (its fields are private but visible from `mod
  tests`, a descendant module, per Rust privacy rules).
- Real `app.update()`s -> real `WorldInstanceReady` fires ->
  `apply_resumed_scene_poses` runs -> asserts `mesh_wears_cracks` reads true
  on the thawed mesh.

This required one addition to `real_app()`: `app.register_type::<
MeshMaterial3d<StandardMaterial>>()` (`frozen_piece.rs:827`).
`bevy_world_serialization`'s spawner copies a `WorldAsset`'s components by
reflection (`AppTypeRegistry`/`ReflectComponent`); without this registration
the in-memory test's hand-built material component would never copy across
into the real world on spawn. This registration does not, and cannot, fix
the real glb cases - the component is simply never present on their source
data to begin with.

**Disclosure, as instructed.** The file-loaded scene crack path
(`WorldAssetRoot` loaded from a real shipped `.glb`, then damaged, killed,
frozen, thawed, and checked for `cracked`/`mark_wreck_cracks`) is not, and
cannot be, exercised headless in this test suite. Reason: `bevy_gltf` only
attaches `MeshMaterial3d<StandardMaterial>` through a private extension
handler that only `PbrPlugin::build` registers
(`bevy_pbr-0.19.1/src/lib.rs:255-258`), and a render-world plugin is out of
scope here. The real crack pipeline is proven instead through an in-memory
`WorldAsset` carrying a hand-built material component
(`a_real_section_crack_pipeline_marks_an_in_memory_scene_mesh`), which
exercises every system in the real path except the glb loader's own
material-attachment step.

### Test output

```
cargo test -p nova_ship --lib --features serde frozen_piece -- --nocapture   (nix develop)

running 2 tests
case hull: frozen piece RON is 952 bytes
test sections::frozen_piece::tests::a_real_section_crack_pipeline_marks_an_in_memory_scene_mesh ... ok
case thruster: frozen piece RON is 1649 bytes
case controller: frozen piece RON is 1181 bytes
case turret: frozen piece RON is 2364 bytes
case railgun: frozen piece RON is 1223 bytes
case docking: frozen piece RON is 950 bytes
case cargo_intake: frozen piece RON is 5019 bytes
case mining: frozen piece RON is 2716 bytes
case torpedo: frozen piece RON is 3784 bytes
test sections::frozen_piece::tests::a_detached_piece_resumes_as_the_art_it_wore ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1113 filtered out; finished in 0.62s
```

```
cargo test -p nova_ship --lib --features serde damage_cracks   -> 13 passed; 0 failed
cargo check -p nova_ship --tests                                -> Finished, clean, no warnings
```

All runs via `nix develop --command cargo ...`. `cargo fmt -p nova_ship` run
once after the above verified green; re-ran `frozen_piece` tests after
formatting to confirm no regression (still 2 passed). No staging, commit, or
push. `git status --short` shows only `frozen_piece.rs` changed.

### Named mutations (not run)

1. **Revert either `cracked` assertion to `assert_eq!(*cracked,
   expect_cracked, ...)`.** Breaks `cargo_intake`, `mining`, and `torpedo`
   immediately with the same `left: false, right: true` failure this round
   fixed - proving the fix is load-bearing against regression back to the
   unprovable form.
2. **Remove `app.register_type::<MeshMaterial3d<StandardMaterial>>()` from
   `real_app()`.** Breaks
   `a_real_section_crack_pipeline_marks_an_in_memory_scene_mesh` at the
   "must carry the real material `WorldInstanceReady` copied" assertion: the
   reflection-based spawner would have nothing to copy the component with.
3. **Revert `charge_seconds: 5.0` to the default on the railgun case.**
   Reintroduces the instant-fire race; the glow assertion flips back to
   "Hidden" intermittently (deterministic here, since `ManualDuration` fixes
   the delta, but the underlying race is real).

### Not done

- The in-memory crack proof does not, and cannot, exercise `bevy_gltf`'s own
  material-attachment code - see Disclosure above. This is a structural
  headless-testing limit, not a deferred task.
- Mining and torpedo cases were not previously reached by any run before
  this round's fix (cargo_intake panicked first); this round is the first
  confirmation either case passes at all.

### Unverified

- Whether a future `bevy_gltf`/`bevy_pbr` version exposes a public hook for
  the material-attachment extension (making option 1 possible later) was
  not checked beyond the currently vendored `0.19.0`/`0.19.1` sources.

### Main worker note after round 3

The standalone test `a_real_section_crack_pipeline_marks_an_in_memory_scene_mesh`
was not approved. The main worker folded it into P-T10 as the final step,
`run_in_memory_scene_crack_case()`, called from
`a_detached_piece_resumes_as_the_art_it_wore`.
