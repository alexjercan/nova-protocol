# What the 120 distinct meshes actually are

**The headline: the owner is right and the cladding is innocent.** `hull-01.glb`
is ONE mesh drawn 136 times. The 120 is **53% placeholder art that nothing
caches** - a fresh `Cuboid`, `Cylinder`, `Cone` and exhaust flame minted per
SECTION ENTITY - and the cladding cache the lane brief suspected works exactly
as its own documentation claims: a whole hull touches **10** shapes, and a
second hull adds **one**.

Measured with a new `census` origin breakdown (`crates/nova_probe/src/capabilities/census.rs`),
`wfc_ships`, real display, 1280x720, `NOVA_PERF_MAX_DELTA=0.015625`. The census
is deterministic, so one run per ship count is the whole measurement.

---

## 1. The breakdown, by origin and by piece

One hull, `--ships 1`, industrial style. **120 distinct meshes, 986 instances.**

| origin | distinct meshes | distinct materials | instances |
|---|--:|--:|--:|
| **placeholder art (per-entity, uncached)** | **64** | **40 + 24 exhaust** | 64 |
| cladding | 30 | 3 | 498 |
| greeble (authored `.glb`) | 21 | 21 | 256 |
| section art (authored `.glb`) | 5 | 5 | 168 |
| **total** | **120** | **69 + 24** | **986** |

Named piece by piece, which is where the defect is legible - **every row in the
first block has one distinct mesh per instance**:

| piece | meshes | materials | instances | shared? |
|---|--:|--:|--:|---|
| `Thruster Section Body (A)` (barrel) | 12 | 12 | 12 | **no** |
| `Thruster Section Body (B)` (nozzle) | 12 | 12 | 12 | **no** |
| `Thruster Exhaust` (outer flame) | 12 | 12* | 12 | **no** |
| `Thruster Exhaust` (inner core) | 12 | 12* | 12 | **no** |
| `Render Turret Joint` (base plate) | 8 | 8 | 8 | **no** |
| `Controller Section Body (A)` | 4 | 4 | 4 | **no** |
| `Controller Section Window (B)` | 4 | 4 | 4 | **no** |
| `Skin Surface` (cladding) | 30 | 3 | 498 | yes - 10 shapes x 3 roles |
| greeble `Mesh.industrial_shadow` | 9 | 9 | 117 | yes |
| greeble `Mesh.industrial_steel` | 8 | 8 | 67 | yes |
| greeble `Mesh.industrial_hazard` | 3 | 3 | 64 | yes |
| greeble `Mesh.industrial_placard` | 1 | 1 | 8 | yes |
| `Cube.003.Material` - **`hull-01.glb`** | **1** | 1 | **136** | yes |
| `Cube.Material` - torpedo bay | 1 | 1 | 8 | yes |
| `Cylinder*.Material*` - turret glbs | 3 | 3 | 24 | yes |

`*` the exhaust materials are
`ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>`, a different asset
type from the 69 the census counts. Their count is read off the component
tally: of 986 drawables, 754 carry `MeshMaterial3d<StandardMaterial>` and 208
carry the cracks material, leaving exactly **24** - the twelve nozzles' outer
and inner flames, one material each by construction. So the drawn material
total at one hull is **93**, not 69.

**64 of 120 meshes and 64 of 93 materials exist only because five render
observers call `meshes.add(...)` and `materials.add(...)` inside the branch that
runs once per entity.** The values are identical - four mesh shapes and five
colours across the whole set.

The raw census buckets in `measurements/mesh-count-origins.csv` split this block
in two, by whether the drawable IS the render child or hangs under it:
`section-fallback-cuboid` 52 and `section-art` 17, of which 12 are the exhaust's
inner cores. The tables above regroup those 12 with their outer flames, which is
the line the DEFECT falls on.

### Why the placeholder branch fires at all

`hull_section.rs`'s fallback is NOT the one that fires (question 3 below): the
hull prototypes author `hull-01.glb`. Two catalog prototypes author no render
mesh at all, and those are the ones a WFC hull is mostly made of -

- `basic_thruster_section` - `render_mesh: None`
  (`crates/nova_authoring/src/base_content/sections/standard.rs:355`)
- `basic_controller_section` - `render_mesh: None` (same file, `:391`)

plus every turret's structural joint, whose base plate is a primitive by design
(`crates/nova_ship/src/sections/turret_section/render.rs:315`).

---

## 2. The `ShellShape` key-space hypothesis: REFUTED

The brief's hypothesis was that `ShellShape` is a struct of eight digits over
five heights (5^8 = 390,625), so two cells rarely share a shape and one hull
mints ~40 shapes x 3 surfaces = ~120 meshes.

Measured, from `census.json`'s `skin_shapes`:

| hulls | plates | **distinct shapes** | cladding meshes |
|--:|--:|--:|--:|
| 1 | 166 | **10** | 30 |
| 2 | 366 | **11** | 33 |
| 3 | 576 | **14** | 42 |

Two hulls share **all ten** of the first hull's shapes and add exactly one
(`shell_4444_4444`, the full block). Three hulls share all eleven and add three.
The ten a single hull wears:

```
shell_0002_0011  shell_0022_0121  shell_0044_0242  shell_0222_1221
shell_0244_1342  shell_0442_2431  shell_0444_2442  shell_2222_2222
shell_2224_2233  shell_2244_2343
```

The key space is large and IRRELEVANT, exactly as `shell_shape.rs`'s module doc
says: "a whole ship touches a couple of dozen of them". The boundary samples are
read off structure and quantised, and a hull built on a cell grid only ever
produces a handful of boundary readings. `SkinAssets` then holds one mesh set
per shape, and 166 plates draw 498 instances through 30 meshes - **17 instances
per mesh, the best re-use ratio of any origin except `hull-01.glb` itself.**

Cladding is **25% of one hull's distinct meshes and 3 of its 93 materials.**
Nothing here needs fixing, and coarsening `SAMPLE_HEIGHTS` (the brief's
candidate 3) would change how the skin LOOKS to buy at most a few meshes. Drop
it.

---

## 3. Does `hull_section.rs:107` fire for WFC hulls? NO

`reinforced_hull_section` and `scavenger_hull_section` both author
`render_mesh: Some(meshes.hull)`, so the hull takes the `WorldAssetRoot` path
and `hull-01.glb`'s single primitive is shared by **136 instances** on one hull.
The census confirms it: the `section-fallback-cuboid` bucket contains no
`Hull Section Body` row at all.

The defect the brief spotted is real, it is just in the other three files. The
same `meshes.add(...)` / `materials.add(...)` per-entity branch is copied into
`thruster_section.rs`, `controller_section.rs`, `torpedo_section/render.rs` and
`turret_section/render.rs`, and for the two prototypes that author no art it
runs on every section of every hull.

---

## 4. Is `ShellSurface::Floor` built and drawn? YES, and nothing exposes it

`ShellSurface::Floor` says of itself "Never seen - it is against the section it
clads", and it is built unconditionally
(`shell_shape.rs`: "The bolt face. The whole cell floor, always"). Measured, it
is **10 of the 30 cladding meshes and 166 of the 498 cladding instances** on one
hull.

Four checks on whether anything ever exposes it, all negative:

1. **Backface culling.** The floor's normal is `Vec3::NEG_Y` in the tile frame,
   which points INTO the section. `StandardMaterial` culls back faces, so from
   any viewpoint outside the hull its triangles are discarded after the vertex
   stage. It is never rasterised - but it is still a distinct mesh, so it is
   still extracted, prepared, bound and written every frame.
2. **Section death.** A destroyed section takes its plates with it - the plates
   are its children and it is despawned recursively
   (`shell_skin.rs`, `destroying_a_section_takes_the_plates_clad_to_it`). A
   detached wreck carries its cladding out still bolted on, at any attitude.
3. **Plate death.** A plate is a `SectionFixture`, not an `ExplodableEntity`;
   `despawn_dead_fixtures` despawns it. It never tumbles free with an underside
   to show.
4. **The artist already treats it as invisible.** No shipped style dresses it:
   all five styles author `StyleSurfaceConfig` for `Top` and `Wall` only, and
   `assets/base/styles/base.content.ron` contains five `surface: Top` and five
   `surface: Wall` and no `Floor`.

**Not landed here** - see the ranking. It is the only candidate that changes
what is rasterised, and it is worth 8% of the mesh count against the 53% the
placeholder defect is worth.

---

## 5. What the frame costs now, and the health warning on the brief's numbers

**The premise of this lane's brief no longer holds on current master.** The
brief priced a distinct mesh at 0.170 ms/frame against a 28.4 ms one-hull frame.
Measured today on `d9e95127`, same scene, same census (986 instances / 120
meshes / 74 standard materials), 1280x720, real display:

| item | `notes-prepare.md` (one hull) | today |
|---|--:|--:|
| `mean_ms` | 28.41 | **5.44** |
| `min_ms` | 7.19-8.52 | **3.02** |
| `Prepare` | 13.66 (47.9%) | **1.66 (30.9%)** |
| `PrepareAssets` | 4.30 (15.1%) | **0.28 (5.3%)** |
| `Render/graph` | 6.17 (22.2%) | 2.23 (41.6%) |

The exhaust-material change (`8a26ae31`) and whatever else has landed since took
the per-frame asset work out. **A distinct mesh no longer costs 0.170 ms**, and
any fix ranked on that price is over-valued by roughly 5x. The counts below are
therefore reported as COUNTS, with the measured frame delta beside them.

---

## 6. Ranked fixes

All three are PRESENTATION under `20260818-220812/TASK.md`. None changes what a
player can shoot off, what damage looks like, or any simulated value.

| # | fix | distinct meshes | distinct materials | instances | kind | state |
|---|---|--:|--:|--:|---|---|
| 1 | **Share the placeholder art and exhaust flames** | **-57** | **-35** | 0 | presentation | **LANDED** |
| 2 | Do not build `ShellSurface::Floor` | -10 | -1 | -166 | presentation | recommended |
| 3 | Merge a hull's skin into one mesh | -29 | -2 | -497 | presentation | do not do |
| 4 | Coarsen `SAMPLE_HEIGHTS` | ~-3 | 0 | 0 | presentation | drop |

(Fix 1's own placeholder set is 5 meshes and 5 materials, so 64 per-entity
assets become 7 drawn ones: -57 net at one hull, -159 at three.)

**1 is the whole finding.** Four render observers minted an asset per entity for
values that are constant across the game. Nothing about the pixels changes - the
meshes and materials are built from the same literals they always were, once
instead of once per section.

**2** is safe on the evidence in section 4 and worth 10 meshes, 166 instances and
166 ENTITIES per hull (5% of the world). It is left for the owner because it is
the only candidate that removes drawn geometry, and its value collapsed with the
frame cost: at today's prices it is a fraction of a millisecond.

**3** (the brief's candidate 2) is the biggest count on paper and should NOT be
taken. A plate is individually destructible, individually damage-graded and
individually re-dressable by style; one mesh per hull would cost all three, and
plate-level destruction is gameplay, not presentation.

**4** would change how the skin looks to buy about three meshes. The measurement
in section 2 kills it.

---

## 7. The before and after, on the paired protocol

Two binaries alternated `base,fix,base,fix`, 8 pairs each at one and three
hulls, real display, 1280x720, `NOVA_PERF_MAX_DELTA=0.015625`, `fixed_steps
max=1` throughout. `min_ms` and phase shares are what survive this box's
contention; a spread that straddles 1.00 measured nothing and is marked.

### Counts (deterministic - one run per point is the whole measurement)

| ships | distinct meshes | distinct materials | mesh instances |
|--:|---|---|---|
| 1 | 120 -> **63** (-48%) | 69 -> **34** (-51%) | 986 -> 986 |
| 2 | 195 -> **74** (-62%) | 121 -> **42** (-65%) | 2211 -> 2211 |
| 3 | 242 -> **83** (-66%) | 147 -> **42** (-71%) | 3423 -> 3423 |

**Instances are unchanged to the entity.** Nothing left the picture; the same
draws go through a shared catalog. The marginal hull now introduces 11 and 9
meshes where it introduced 75 and 47.

### Frame time, 8 pairs

| ships | statistic | base median | fix median | fix/base median (min-max) |
|--:|---|--:|--:|---|
| 1 | `min_ms` | 2.852 | 2.589 | **0.899 (0.86-0.95)** |
| 1 | `mean_ms` | 4.363 | 3.794 | 0.856 (0.56-1.50) - straddles |
| 1 | `p50_ms` | 3.505 | 3.214 | 0.913 (0.57-1.25) - straddles |
| 3 | `mean_ms` | 7.810 | 4.662 | **0.631 (0.46-0.87)** |
| 3 | `p50_ms` | 5.817 | 4.096 | **0.701 (0.47-0.87)** |
| 3 | `min_ms` | 3.860 | 3.181 | **0.824 (0.77-0.89)** |

**37% off a three-hull frame and 10% off the least-contended one-hull frame.**
At one hull only `min_ms` is clean: the whole frame is 4 ms and the box's own
load swamps the mean.

### Where it came from, three hulls, paired

| phase | base median | fix median | ratio (min-max) |
|---|--:|--:|---|
| `Prepare/WritePhaseBuffers` | 1.083 | 0.407 | **0.379 (0.27-0.48)** |
| `Prepare/BindGroups` | 0.560 | 0.264 | **0.456 (0.32-0.62)** |
| `Prepare` | 2.486 | 1.321 | **0.538 (0.37-0.66)** |
| `Render/graph` | 3.056 | 1.839 | **0.599 (0.41-0.78)** |
| `PrepareAssets` | 0.418 | 0.302 | **0.715 (0.51-0.89)** |

Every one is clean. This is the per-BIN model behaving exactly as it should:
the two phases that do work per distinct mesh and material fall hardest, and
they fall by about what the counts did.

### The picture is unchanged

`wfc_ships --ships 1 --seed 7` captured under `NOVA_CAPTURE` on both binaries:
`wfc-ships-row.png` differs by **RMSE 0.0013** (0.13%) and `wfc-ships-bare.png`
by 0.0071, which is bloom and tonemapping noise between two runs. Opened and
compared side by side: identical. Expected - the shared assets are built from
the same literals the per-entity ones were.

---

## 8. What landed

`crates/nova_ship/src/sections/placeholder_art.rs` (new): one `PlaceholderArt`
`FromWorld` resource holding the five meshes and five materials every
un-authored section body in the game wears - the same pattern
`turret_section`'s `DefaultProjectileRender` already used. Wired into the
fallback branch of `hull_section`, `thruster_section`, `controller_section`,
`torpedo_section/render` and `turret_section/render`.

`ExhaustMeshes` in `thruster_section.rs`: the flame mesh keyed by
`(geometry, hx, hz, height)` with the floats as bit patterns. The MATERIALS stay
per nozzle deliberately - `thruster_shader_update_system` writes the throttle
into the material, and two drives burn at two rates, so sharing them would be a
visible regression rather than a saving.

`crates/nova_probe/src/capabilities/census.rs`: the origin and piece breakdowns
that produced every table above, plus the drawn-material count (the census read
only `MeshMaterial3d<StandardMaterial>` and every section mesh has that removed
by `damage_cracks`), the `SectionCracksMaterial` asset total, and the skin's
plate count and shape ID LIST - a list rather than a count, because the one/two
hull question is an intersection and a count cannot answer it.

Tests: `nova_ship --lib` 682 passed, 0 failed, including
`nozzles_of_one_size_share_one_flame_mesh` and
`the_placeholder_set_is_built_once`.
