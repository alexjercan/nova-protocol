# The torpedo's materials: the fourth sighting of one bug

`notes-pd-stress.md` traced the point-defence range at 26.06 ms and found the
top two systems were 60% of it:

| ms/f | share | system |
|--:|--:|---|
| 9.285 | 35.6% | `prepare_erased_assets<ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>>` |
| 6.327 | 24.3% | `prepare_material_bind_groups` |

Both are the distinct-asset law again: **the frame tracks DISTINCT ASSETS, not
instances.** Cracks per section (`6b3bfc87`), the plume rewritten per frame
(`8a26ae31`), placeholder art per entity (`cbc86980`), and now this.

**It is TWO bugs, not one, and the root fix does not collapse both.** They look
alike because both end in a private material per torpedo, but the reasons are
different and so are the fixes.

**STATUS: BOTH FIXED, MEASURED, AND THE PICTURE IS UNCHANGED.** Re-traced, the
two systems above are 64.0% of the frame before and **1.8% after**: the plume
prepare goes 8.576 ms to **0.008**, the bind-group prepare 5.804 to **0.262**.
Paired and pinned over seven pairs, the render world is **0.431x** (0.319-0.482)
and the least-contended frame **0.304x** (0.267-0.351). The whole frame is
0.785x, and no further, because **the render world stopped being the pacer**:
98.0% of the traced frame before, 48.5% after, with the main world now at 98.0%.
The scene's distinct drawn materials went **105 to 17** with MORE torpedoes in
the air. Every visual-gate pair sits inside the run-to-run noise of the
unchanged binary.

## 1. Why a torpedo gets a private source material

`insert_torpedo_controller_render`
(`crates/nova_ship/src/sections/torpedo_section/render.rs`) built the warhead
body with `MeshMaterial3d(materials.add(tint))` - one `StandardMaterial` per
LAUNCH.

The tint it puts in that material is
`TorpedoType::tint`, copied off `TorpedoTypeConfig` at launch. That is an
authored property of the ordnance TYPE - two shipped types, Lance and Serpent -
so **a per-instance asset was carrying a per-type value.** Nothing about a
torpedo's colour is per torpedo.

The old doc on `TorpedoTypeConfig::tint` said the quiet part out loud: "The
default torpedo body already builds its material per projectile, so a per-type
tint costs nothing." It was never free; the cost had just never been measured.

### What it multiplied

`SectionCracksMaterials` keys its bucket materials on `(source AssetId,
bucket)`. A source that is per instance cannot collapse, so **every torpedo also
minted its own crack-bucket materials** as point defence chewed on it. That is
why `forget_dead_sources` exists at all - its doc named the torpedo warhead as
the reason the registry would otherwise be a leak.

So one line of per-instance authoring produced two per-instance material
populations, one of them a multiple of the other.

### Fixing the root collapses ONE of the two items, not both

The exhaust plume is private for a different reason. Its material does not carry
an identity, it carries a per-frame VALUE - the throttle - and
`thruster_shader_update_system` wrote it into the material every frame.
`8a26ae31` added a read-before-write guard, which is a total win on a frozen
gallery and does nothing at all here: **a guided torpedo's thrust genuinely
moves every frame**, so the guard never fires. No amount of fixing the warhead
tint touches it.

## 2. What changed

Two fixes, both the established D1 pattern from `6b3bfc87`: quantise into shared
buckets, and SWAP rather than write.

**The warhead body** (`DefaultTorpedoRender`). The resource already held the
shared body mesh; it now also holds one material per TINT, keyed on the colour's
bit pattern the way `ExhaustMeshes` keys its flames. A salvo of one type is one
material. Held strong and never evicted: the set is bounded by the ordnance types
a mod authors, and keeping a tint alive is also what keeps its crack buckets from
being rebuilt on the next salvo.

**The exhaust plume** (`ExhaustMaterials`, new, beside `ExhaustMeshes`). The
throttle snaps to the nearest of `EXHAUST_PLUME_BUCKETS = 16` steps and a cone
SWAPS its `MeshMaterial3d` to the shared material for its `(shape, bucket)` pair.
Nothing is written into a built material at all, so the two named systems have
nothing to re-extract, re-upload or re-bind. The set is bounded by nozzle SHAPES
times buckets, however many drives are burning.

A cone's shape half is a `PlumeSpec` - emissive colour, shader falloff radius,
stretch - carried on the cone entity rather than re-derived from the config,
because a plume outlives its section: a severed drive keeps drawing its cone
while it tumbles away, and the swap still has to know what to swap to.

The two ENDS are exact. `bucket_input(0)` is 0.0 and
`bucket_input(EXHAUST_PLUME_BUCKETS - 1)` is 1.0, because
`system_thrust_and_plume` asserts the shader uniform reaches both to 1e-6 and it
reads whatever material the swap left behind.

**An instrument gap, closed.** The census's drawn-material count reads
`MeshMaterial3d<StandardMaterial>`, so it could not see an `ExtendedMaterial` -
which means the single largest material population in a torpedo fight was
invisible to the instrument built to count material populations. `census.json`
now carries `plume_material_assets` beside `cracks_material_assets`, and
`ThrusterPlumeMaterial` names the pair.

## 3. The visual gate

**N = 16, and the effect's own jitter is bigger than a step.**

The throttle reaches the picture through exactly one line
(`assets/shaders/thruster_exhaust.wgsl:48`):

```wgsl
var offset_amount = f * material.thruster_input * material.thruster_exhaust_height;
```

It stretches the cone along +Y and does nothing else - no colour, no opacity, no
width. `thruster_exhaust_height` is the config's `exhaust_max`: 1.0 for an outer
flame, 0.5 for the inner core, on both the base thruster and the torpedo's.

- One bucket step is `1/15` of that: **0.0667** local units outer, **0.0333**
  inner.
- Worst-case quantisation error is HALF a step: 0.0333 and 0.0167.
- Eight lines later, the same shader adds a per-frame noise wobble of up to
  `wobble_amp = 0.1` (line 56) to every vertex above the base, at any throttle,
  every frame.

**The wobble the effect deliberately puts on itself is 1.5x a whole outer bucket
step and 3x an inner one.** A quantisation smaller than the noise already in the
picture cannot be the thing a player sees.

### RMSE, and why one of the two shots is worthless

`screenshot_torpedo_run` on both binaries, real display, `DISPLAY=:0`,
960x1057. The script's beats land on slightly different frames run to run, so the
scene drifts a pixel or two - which means a single base-vs-fix number says
nothing until the run-to-run floor is measured beside it.

`wiki-combat-torpedo.png`, four base captures and three fix captures, every pair:

| set | pairs | RMSE range | median |
|---|--:|---|--:|
| within BASE | 6 | 0.268% - 1.025% | 0.891% |
| within FIX | 3 | 0.311% - 0.473% | 0.404% |
| **base vs fix** | **12** | **0.400% - 0.967%** | **0.549%** |

**Every cross-arm pair sits inside the within-base band, and the cross-arm median
is BELOW it.** Two runs of the unchanged binary differ more than a base run
differs from a fix run. The change is smaller than the scene's own drift.

`wiki-combat-aftermath.png` was captured and is **discarded**: two BASE runs
differ by **13.78%**, against 13.72% base-vs-fix. That shot is the debris field
after the fuze, and `detach_destroyed_body` draws its scatter from the global
unseeded `WyRand` - the residual `notes-pd-stress.md` named as the one
non-determinism left inside this subject. Base and fix agree to within a
thousandth of a number that is 25x the signal, so it measures nothing and is not
quoted as if it did.

### By eye

`plume-buckets-burn.png` (committed beside this note): two drives of one hull
under an AP GOTO burn at 857.6 m/s, continuous throttle left, 16 shared buckets
right, from `screenshot_flip_burn` on both binaries. Same flame length, same
white core, same bloom halo, same reach past the nozzle. The difference a reader
will notice is the ship's POSE - a frame of script drift - not its drives.

**Judgement: no stepping is visible, and I did not have to look for it twice.**
The cracks gate needed a careful look at a battered hull; this one does not,
because the throttle only moves the flame's LENGTH and the shader was already
jittering that length by more than a step.

The one thing a still cannot show is a ramp. Sixteen steps across a roll-on that
takes about a second is a step every ~4 frames, each moving the tip by 6.7% of
its stretch - about 2-3 px at 1080p on the drives above, under a bloom that is
wider than that. A torpedo does not ramp at all: it launches at full thrust and
tapers as it nears its cruise cap, so it sits in the top buckets for the whole
flight.

**The warhead tint gets no picture, and does not need one.** Sharing a material
by value does not change the value: `materials.add(tint)` is called with the
same `Color` it always was, once instead of once a launch.
`a_torpedo_flies_in_its_own_types_colour` reads the base colour back off the
material the observer actually attached and asserts it equals the type's tint,
for two types, and that two tints are two materials. There is nothing a
screenshot could add to that.

### Correctness, on the production path

- `system_thrust_and_plume`, fix binary, real display: all five invariants green,
  including `plume follows throttle` (uniform at 1.0, tolerance 1e-6) and
  `plume returns to idle` (0.0, 1e-6). Those two are what prove the bucket ends
  are exact THROUGH the swap, not just in a unit test.
- `stress_point_defense`, fix binary: 12 mounts up against 12 bays, peak 74
  inbound, 232 torpedoes shot down, peak 2389 rounds / 2567 colliders, the sky
  drained to zero, teardown returned to baseline.
- `nova_ship --lib`: 686 passed, 0 failed, including the four new ones.

## 4. The counts

Census inside the saturated window (`NOVA_PERF_CENSUS_FRAME=1000`), 12 mounts
against 12 bays, pinned, 1280x720, real display. Deterministic, so one run per
arm is the whole measurement.

| | base | fix |
|---|--:|--:|
| `Torpedo Controller` instances | 93 | **106** |
| ... its distinct materials | **93** | **7** |
| distinct drawn materials, whole scene | **105** | **17** |
| `StandardMaterial` assets | 112 | 18 |
| `SectionCracksMaterial` assets | 149 | 16 |
| `Thruster Exhaust` instances | 276 | 288 |
| mesh instances | 1800 | 1464 |
| distinct meshes | 17 | 17 |

**The fix arm has MORE torpedoes in the air and a sixth of the materials.** 93
warheads were 93 materials; 106 are 7 - a single shared source, and the seven of
its eight crack buckets a battery's fire had driven a live torpedo into. The
whole scene's distinct drawn materials went 105 to 17, and the crack registry,
which was 149 materials on the back of those 93 private sources, is 16.

The plume count is not in that table because the census could not see it on
either binary - both predate the count this lane added (section 2). It is exact
anyway from the code: `insert_thruster_shader` minted one material per cone and
the handle dies with the entity, so **base plume materials == plume cone
instances == 276**, every one of them written every frame. On the fix, every
plume in this range is a torpedo thruster off one literal config, which is two
specs (outer flame and inner core), so the ceiling is **2 x 16 = 32** and none of
them is ever written.
`a_throttle_sweep_mints_at_most_one_material_a_bucket` is the test that keeps it
there, and the traced pass (section 5a) is what confirms it on the real scene:
the system that prepares those materials falls from 8.576 ms a frame to 0.008.

## 5. The frame

### 5a. The two named systems, by name

The profiled pass, one run per arm, `--features debug,trace`, sliced to
ts 10.0-17.0 s - inside each arm's own saturated plateau. Tracing inflates every
span uniformly, so the SHARES are what this table is for.

| system | base ms/f | base share | fix ms/f | fix share |
|---|--:|--:|--:|--:|
| `prepare_erased_assets<ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>>` | **8.576** | **38.2%** | **0.008** | **0.1%** |
| `prepare_material_bind_groups` | **5.804** | **25.8%** | **0.262** | **1.7%** |
| `text_system` | 5.231 | 23.3% | 3.818 | 25.2% |
| `run_physics_schedule` | 3.774 | 16.8% | 2.955 | 19.5% |
| `schedule: RenderGraph` | 2.928 | 13.0% | 2.531 | 16.7% |
| `collect_collision_pairs<ProjectileHooks>` | 0.144 | 0.6% | 0.141 | 0.9% |
| **`sub app: RenderApp`** | **22.025** | **98.0%** | **7.358** | **48.5%** |
| **`main app`** | 18.372 | 81.8% | 14.854 | **98.0%** |
| `update` (the frame) | 23.472 | - | 15.887 | - |

**The two systems the brief named were 64.0% of the traced frame. They are now
1.8%.** The plume prepare is not reduced, it is GONE - 0.008 ms is the cost of a
pass over an asset collection nothing modified. `prepare_material_bind_groups`
keeps the 0.262 ms it takes to service every other material in the scene.

**And the PACER changed.** The render world was 98.0% of the frame and the main
world 81.8%; now the render world is 48.5% and the main world is 98.0%. From
here the frame is a MAIN-world problem, which is a different lane's.

### 5b. The clean pass, paired and interleaved

Seven pairs, `base, fix, base, fix ...`, one capture per process, pinned
`NOVA_PERF_MAX_DELTA=0.015625`, 1280x720, `present=immediate`, real display.
Set A (4 pairs) was taken while a sibling lane was building; set B (3 pairs) on
an idle box. **`fixed_steps max=1` in every capture** - the clamp never fired.

Medians over all seven pairs, with the ratio's own spread, and set B's median
beside it as the quiet-box check:

| statistic | base | fix | ratio (min-max) | set B only |
|---|--:|--:|---|--:|
| **`min_ms`** | 17.83 | **5.47** | **0.304 (0.267-0.351)** | 0.323 |
| **render world** | 30.22 | **13.33** | **0.431 (0.319-0.482)** | 0.434 |
| **`PrepareAssets`** | 13.92 | **1.30** | **0.087 (0.069-0.118)** | 0.086 |
| **`Prepare/BindGroups`** | 8.17 | **1.90** | **0.234 (0.144-0.243)** | 0.234 |
| `Prepare` | 10.60 | 5.26 | 0.496 (0.304-0.509) | 0.499 |
| `mean_ms` | 32.72 | 27.41 | 0.785 (0.500-0.855) | 0.808 |
| `p50_ms` | 30.91 | 25.38 | 0.764 (0.519-0.846) | 0.821 |
| `p99_ms` | 62.83 | 56.82 | 0.873 (0.576-0.942) | 0.886 |
| `Render/graph` | 4.01 | 4.31 | 1.065 (0.924-1.125) - **straddles** | 1.058 |
| main world | 18.87 | 25.25 | 1.338 (0.882-1.528) | 1.257 |
| `PostUpdate` | 9.13 | 13.17 | 1.475 (0.864-1.610) | 1.475 |
| `RunFixedMainLoop` | 6.26 | 8.53 | 1.317 (0.998-2.049) | 1.220 |

Three things to read off it.

**`Render/graph` straddles 1.00, and that is the control.** The same instances
draw through the same meshes; only their material COUNT moved. A draw phase that
had fallen would have meant something left the picture.

**The floor is a third of what it was.** `min_ms` is the statistic this box lands
cleanly - the base's is 17.5-18.9 across seven captures at loads from 0.7 to 8 -
and it goes to 4.9-6.2. Read with the caveat that the fix's cheapest frames can
be ones that ran NO fixed step (see below), which the base's never are.

**The whole frame moves less than its render half, because the render half
stopped being the pacer.** 0.785 on the mean against 0.431 on the render world.
What is left is the main world, and this lane made it more expensive, honestly:

- the fix arm carries **2365 peak rounds and 2565 colliders against the base's
  2036 and 2239** - 15% more bodies. A faster frame runs the `Update`-side
  point-defence chain more often per fixed tick, so more mounts hold their
  assignment and more rounds go up. `RunFixedMainLoop` 1.317 and `PostUpdate`
  1.475 are largely that.
- the fix ran **843-882 fixed steps in its 900-frame window** against the base's
  900, because some of its frames now arrive FASTER than the pinned 15.625 ms
  step and spend none. That is the ordinary "scene faster than the timestep"
  case `docs/performance.md` describes, not a stopped simulation - the capture
  refuses one of those outright, and none was refused.

So the frame ratio is measured on a scene the fix made HEAVIER, and is
conservative by that much.

## 6. What was ruled out, with the number

- **The census at its default frame is not comparable between arms.** At
  `DEFAULT_CENSUS_FRAME = 90` the two arms counted 24 and 12 torpedoes: the
  range's script beats do not land on the same frame, so the two censuses
  described different scenes. Every count above is at
  `NOVA_PERF_CENSUS_FRAME=1000`, inside the saturated hold on both.
- **`wiki-combat-aftermath.png` as a visual gate: 13.78% base-vs-base.** Section
  3.
- **Raising N above 16.** The step (0.0667) is already smaller than the shader's
  own per-frame wobble (0.1), so more buckets buy a difference nothing can
  resolve and cost bins.
- **Extending the read-before-write guard with a dead band.** It would skip some
  writes, and leave one material per instance - so `prepare_material_bind_groups`
  and the draw bins stay on the population, which is where 24.3% of the frame
  was. Skipping a write is not the same as not owning an asset.

## 7. Does the defect reach anything else?

Everything below is from the same saturated census, base arm.

- **Turret rounds: already fixed, and it shows.** `Bullet Projectile Render` is
  **1349 instances, 1 mesh, 1 material** - the largest population in the scene
  and the cheapest thing in it, through `DefaultProjectileRender`. This is what
  the torpedo should have looked like.
- **Debris: clean.** A destroyed section detaches and keeps the art it was
  already wearing, which is a shared bucket material. Nothing new is minted.
- **The blast shell: the SAME defect, small.** `insert_blast_radius_visual` mints
  one `StandardMaterial` per detonation and `animate_blast_radius_visual` writes
  it every frame to fade it - per-instance asset, per-frame write, exactly the
  shape this note is about. Peak here was **3 materials (base) / 1 (fix)**,
  because the shell lives 0.4 s and because this range shoots torpedoes down
  instead of letting them fuze. A campaign salvo arriving together would size it
  by the salvo. The same bucket treatment would work on it (quantise the fade),
  and at three materials it does not pay for itself yet.
- **The blast particle burst: the PLACEHOLDER-ART defect in another asset type.**
  `insert_particle_effect` builds a fresh 32768-particle `EffectAsset` from
  LITERALS per detonation when the bay authors none - the same "constant value,
  minted per entity" shape `cbc86980` fixed for meshes and materials. Not
  measured: nothing counts `EffectAsset`s. The bay's launch burst and the
  turret's muzzle burst are the same code but per SPAWNER and per MOUNT, so they
  are bounded by the ship rather than by its rate of fire and are fine.

## 8. Docs this invalidates, and did not update

This lane was told to stay out of `docs/`. Two lines there are now false and
belong to whoever lands next:

- `docs/sections.md:662` - "a torpedo warhead is tinted per LAUNCH, so eager
  buckets would cost eight ..." The reason for lazy buckets is still good; the
  example is not. A source that is per instance is now a MOD's, not the base
  game's.
- `docs/sections.md:817` - "... with a `StandardMaterial` built per LAUNCH from
  the ..." It is built per TINT.

Nothing on `/wiki/` or `/create/` changes: a player sees the same picture and a
mod author authors the same `tint` field, on the same terms.

## 9. How it was measured

Host: RTX 3060 Ti, i9-12900F, NixOS, `dev` profile, vulkan. `DISPLAY=:0`
throughout - never `xvfb-run`, including for the visual gate, because the plume
is a bloomed HDR emissive and a software rasteriser is not what a player sees.
All eighteen armed runs wore `WM_CLASS` class `nova-measure` and were moved to
i3 workspace 3 by an IPC `window::new` watcher matching on CLASS (`for_window`
is config-only on 4.25.1) - the watcher logged all eighteen moves. The
visual-gate captures are NOT armed (`screenshot_*` declares no frame-cost
claim), so they wear the ordinary class and ran on the desk; nothing about them
is a timing.

Two binaries, built explicitly with `--features debug` and copied out of
`target/` before anything else was built, so the profiled pass could not leave a
`debug,trace` binary at the path a timing run reads - the footgun that cost the
previous lane a seven-capture sweep. `base` is `0b606cb5`, `fix` is `0018e2c7`.
`NOVA_PERF_SHA` is set per arm because the capture reads the sha off the TREE,
not off the binary, and both arms otherwise claim whichever commit is checked
out.

**What the box was doing.** Three sibling lanes shared it. Set A (pairs 1-4,
18:21-18:39) ran against their builds: `ps` showed `rustc`, `rust-lld` and a
`wasm-bindgen` at 1389% CPU during pair 4's fix capture, and the 1-minute load
average ran 4.9-7.9. Set B (18:40-18:52) ran with nothing else on the box: load
0.7 before the first capture and 2.2-4.8 during them, all of which is the
capture itself. **The two sets agree** - `render` 0.431 against 0.434,
`PrepareAssets` 0.087 against 0.086, `Prepare/BindGroups` 0.234 against 0.234 -
so the contention widened the frame-total spread and left the phase ratios
alone. The traced pass ran at 18:51-18:52 on the same idle box.

The paired protocol is what makes set A usable at all: a fresh base capture
before every fix capture, so a run the box spoiled shows up as a moved
REFERENCE rather than as a result. Nothing was discarded on that basis; pair 4's
frame total is the widest ratio in the table (0.918) and its neighbour is the
narrowest (0.563), which is the spread being honest about a contended box.

Raw rows: `measurements/torpedo-materials-pairs.csv` (both sets, every
statistic and every phase), `-framecost.txt`, `-trace.txt` (the profiled pass),
`-census.csv`, `-gate.csv` (every RMSE pair).
