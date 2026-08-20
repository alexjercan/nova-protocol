# Phase B5: the operator's desk, and where `Prepare` goes

Two jobs. The first was meant to be a protocol question and turned into three
harness defects, one of which explains an anomaly `notes-floor.md` could not
account for. The second reaches the answer the epic was sent for.

**The headline of each, up front.**

- **Job A: YES. A hidden workspace measures the same as a visible one** - median
  ratio 0.97-0.99 over 14 paired captures, spread straddling 1.00 - **but only
  after a one-line harness fix.** Before it, a capture whose window did not have
  focus was paced by bevy at exactly 60 Hz. That is the 16.67 ms
  `notes-floor.md` blamed on a compositor.
- **Job B: the frame tracks DISTINCT MESH ASSETS, not mesh instances.**
  `R^2 = 0.996` against distinct meshes, `0.91` against instances, and the
  marginal cost per instance falls 4.9x across three hulls while the marginal
  cost per distinct mesh is flat. **That is the whole sublinearity**: hull one
  introduces 120 meshes and 74 materials, hull two re-uses that catalog and adds
  75, hull three adds 47. A hull costs what it INTRODUCES, not what it draws.

About 87 captures, all on `DISPLAY=:0`, all at a verified 1280x720, all with
`NOVA_PERF_MAX_DELTA=0.015625` holding (`fixed_steps max=1` in every row).

---

## 1. Job A: can the operator keep their desk

### 1.1 The class, and the line that does NOT work

`xprop` on a run armed with `NOVA_PERF`:

```
WM_CLASS(STRING) = "", "nova-measure"
```

Two strings: instance first, class second. Bevy's `Window::name` lands in the
**class**, and the instance is EMPTY. So i3's criterion is `class`, and
`[instance="nova-measure"]` matches nothing.

**The line in the brief is a parse error on i3 4.25.1.** `for_window` is a
CONFIG directive; the command parser rejects it:

```
$ i3-msg 'for_window [class="nova-measure"] move container to workspace 3'
ERROR: Expected one of these tokens: <end>, '[', 'move', 'exec', ...
```

Two forms that do work:

**Durable (one line of config, what the owner should have):**

```
# ~/.config/i3/config
for_window [class="nova-measure"] move container to workspace 3
```

**Session-scoped (no config edit; what these measurements used):** subscribe to
the IPC window event and move by `con_id`. `scripts`-free, three lines:

```bash
i3-msg -t subscribe -m '["window"]' | while read -r line; do
  [ "$(jq -r '.change' <<<"$line")" = new ] || continue
  [ "$(jq -r '.container.window_properties.class' <<<"$line")" = nova-measure ] || continue
  i3-msg "[con_id=$(jq -r '.container.id' <<<"$line")] move container to workspace 3"
done
```

The `for_window` form applies at map time and has no race. The watcher form
acts on `window::new`, which i3 emits as it manages the window, so the window
can be on the operator's workspace for a few milliseconds. Neither steals
focus: `move container to workspace 3` without a following `workspace 3` leaves
the operator where they are, **verified** - focused workspace before and after
every one of 28 placed captures was unchanged.

**No `floating enable` is needed, and no `resize set`.** Once the capture sizes
the window before winit creates it (section 2.1), winit's `WM_NORMAL_HINTS`
carry `min == max == 1280x720` and i3 auto-floats it (`floating: "auto_on"`) at
exactly that size. `resize set 1280 720` in the rule is REFUSED by i3 for the
same reason, and `floating disable` mid-run leaves the geometry alone. The size
hints, not the rule, are what protect the pixel count.

### 1.2 The measurement, and what had to be fixed first

Arm **V**: workspace 3 focused, window mapped. Arm **H**: the operator's
workspace focused, window on 3 and UNMAPPED (`xprop` reads
`WM_STATE: Withdrawn`). Same scene, same binary, same 1280x720, interleaved
H,V,H,V.

**First attempt, before the fix:**

| arm | empty gallery |
|---|--:|
| V (mapped) | 3.37 ms, 296 FPS |
| H (unmapped) | **16.66 and 16.68 ms, `mean_fps=60.0`** |

That is not the window being hidden. It is
`WinitSettings::game()`, which the capture set and whose own comment claimed it
"keeps the loop running flat out even when the window is unfocused". It does the
opposite: `unfocused_mode: reactive_low_power(1/60)`. The event loop, not the
frame, was deciding the number.

**After `WinitSettings::continuous()`** (`6d208511`), paired and interleaved:

| subject | pairs | statistic | H median | V median | H/V median (min-max) |
|---|--:|---|--:|--:|--:|
| empty gallery | 4 | mean | 3.161 | 3.368 | 0.929 (0.86-1.15) |
| empty gallery | 4 | min | 2.614 | 2.593 | **0.993 (0.91-1.08)** |
| one hull | 10 | mean | 24.867 | 24.655 | 0.970 (0.64-1.34) |
| one hull | 10 | min | 7.463 | 7.399 | **0.984 (0.92-1.13)** |

**Verdict: they agree.** Every spread straddles 1.00. The mean's spread is wide
because this box is the operator's desk and its load moves under the
measurement (section 4); `min_ms` - the least-contended frame in a window - is
reproducible to a few percent and it agrees to within 2%.

**The protocol is adopted.** Captures go to workspace 3, the operator keeps
workspaces 1 and 2, and nothing about the numbers changes.

---

## 2. The validity gates

Three refusals, all in the shape `notes-floor.md` established for
`ABORT_SIMULATION_STOPPED`: log at ERROR, name the reason, **write no stats**.
Each of these produces a steady and entirely plausible number for something
that is not the game, which is exactly why none can be left for a reader to
notice.

### 2.1 `ABORT_WINDOW_SIZE` - and the defect that motivated it

**Every real-display capture in this epic ran at a window size it did not
report.** `NOVA_PERF_RES` was applied in a `Startup` system, but winit freezes
`WM_NORMAL_HINTS` to the CURRENT size the moment `resizable` goes false, so the
resize is one a reparenting WM simply refuses - while `Window` goes on
reporting the size that was asked for. Measured today: a run that reported
1280x720 was a **1024x768** window when floated, and **960x1057** when i3 tiled
it beside another window.

`notes-floor.md` has the fingerprint in it: "`main_opaque_pass_3d` shades
1,014,720 fragments - 1.10x the window". 1,014,720 is 960 x 1057 exactly. That
page has been corrected.

The fix is not the gate, it is that the capture now sizes the window at plugin
BUILD time, before winit creates it, so hints, surface and `Window` agree by
construction. `wfc_ships` also stops running `force_capture_resolution`
(1920x1080) under a measured run, which was ambiguous with the probe's own
write in the same schedule.

The gate then catches whatever is left, and it fired for real: two captures
whose window landed on the operator's workspace (the placement watcher had been
killed) were refused with

```
ABORTED reason=window_size phase=capture frame=37 - the primary window is
1920x1080 and the capture is configured for 1280x720. Frame cost is a function
of window PIXELS ... No stats were written.
```

### 2.2 `ABORT_UPDATE_THROTTLED`

Refuses any `WinitSettings` that is not `Continuous` on both focused and
unfocused. The capture sets `continuous()` itself; the check exists because the
probe does not own the app - an example inserting its own `WinitSettings` after
the plugin wins, and the setting that loses is invisible in the numbers. It
just makes them 60.

### 2.3 `ABORT_REFRESH_CAPPED`

Arms only when the run named a mode that promises NOT to block on refresh
(`immediate`, `mailbox`, `autonovsync`). It measures how tightly the window
collapsed onto one value: the share of frame deltas within +/-5% of the median.
A workload produces a distribution; a period produces a spike.

**Calibrated against measurement, not chosen.** On this 165 Hz output:

| capture | median | mean_fps | clustered share |
|---|--:|--:|--:|
| `present=fifo`, empty gallery | 6.059 ms | 165.0 | **0.790** |
| `present=fifo`, empty gallery | 6.042 ms | 165.0 | **0.760** |
| `present=immediate`, 35 captures, 0-3 hulls | - | - | **0.03-0.44** |

Threshold **0.60**, with a 4 ms floor below which no display refreshes. Note
what the calibration changed: my first guess was 0.90, and a REAL capped window
fails it - its own minimum ran 23% under the period. A synthetic cap cleaner
than the real thing would have passed a threshold the real thing fails, so the
unit test builds its samples to the measured shape.

The live trip could not be exercised on this host: `immediate` and `mailbox`
both come back uncapped (243-344 FPS), and `fifo` is the operator asking for the
cap, so the gate correctly stands down. The predicate is unit-tested on
measured-shape data and the `clustered` figure is logged on every capture, so
the gate's input is auditable on any row.

### 2.4 One instrument fix that changes a published number

`RenderPlugin` chains `PrepareAssets` between `ExtractCommands` and
`PrepareMeshes` **without putting it in the top-level chain**, so `framecost`'s
boundary marker was ambiguous with it and its time landed in whichever
neighbour won the scheduler. Split out:

| `notes-floor.md`, one hull | corrected |
|---|---|
| `PrepareMeshes` **4.20 ms** | `PrepareAssets` **4.30 ms**, `PrepareMeshes` **0.30 ms** |

The brief's "`Prepare` + `PrepareMeshes` is 16.1 ms" is right in total and
wrong in the name of its second item. `Prepare` also now reports its six
sub-sets; anchoring those markers `.in_set(Prepare)` is load-bearing, because
without it the scheduler ran the first one at the top of the whole `Render`
schedule and charged `Prepare/Resources` with more time than all of `Prepare`.

---

## 3. Do the gallery figures survive? YES.

`notes-floor.md`'s four figures are cited in `20260818-220812/TASK.md` as the
release's definition of done and in `DECISIONS.md` D11 and D12. They were taken
with two defects present: the window was 960x1057 rather than 1280x720, and the
camera was ORBITING (`IdleOrbit` stood down for `NOVA_CAPTURE` only, not for a
frame-time capture).

Re-measured from scratch - orbit off, window verified 1280x720, continuous
update mode, capture held until the row is BUILT, interleaved against a fresh
one-hull reference before every arm, 3 passes, 24 captures:

| ships | `notes-floor.md` | re-measured | delta |
|--:|--:|--:|--:|
| 0 | 3.02 ms | **3.45 ms** | +14% |
| 1 | 27.60 ms | **28.41 ms** | +3% |
| 2 | 34.82 ms | **40.71 ms** | +17% |
| 3 | 44.04 ms | **43.33 ms** | -2% |

**They stand.** Every one is inside this box's own run-to-run spread, and the
shape - 3 ms empty, ~28 for one hull, ~40 for two, ~43 for three - is
unchanged. D11 and D12 need no retraction and neither does the release's DoD.

**Why the orbit did not bite them.** I expected it to, and said so before
measuring. It does not: paired, one hull is ~28 ms with the orbit off against
the 27.6 ms the floor lane measured with it on. The orbit's own cost is below
this box's noise. It is still fixed, because a capture whose subject moves is
not reproducible - but it changed no number, and the 9.9-vs-30.1 ms pair I
first blamed on it was something else (section 4).

**1v1 still misses 60 FPS by about 24 ms, and 100% of that is per-ship.**

---

## 4. What actually made consecutive captures disagree

Not the orbit, and not the CPU clock - the clock boosts to 4.9-5.1 GHz in slow
runs and fast ones alike. Two things:

1. **The window opened before the scene existed.** The capture's warm-up is
   counted in FRAMES from `Playing`, and a loading-screen frame costs 1.8 ms,
   so the declared 90 of them are 0.16 s. A 50-frame `framecost` series through
   one run reads `1.9, 1.9, 11.0, 33.9, 35.4, ...`: the window was opening on a
   row that was still spawning. `wfc_ships` now holds the capture until the
   drawable count has stopped moving for 90 frames
   (`FrameTimePlugin::ready_when`), and a 1200-frame series after that is
   stationary at 23.7-35.8 ms with no drift over 37 seconds.
2. **The box is the operator's desk.** Even gated, whole runs land 2-3x apart,
   with EVERY row scaling together - main world, render world, each phase. That
   is not the scene. It is what a browser on the other workspace does to a
   single-threaded render world.

**The consequence for every measurement after this one:** on this host, an
absolute millisecond is worth what an interleaved pair says it is worth, and
two statistics survive contention where the mean does not:

- **`min_ms`**, the least-contended frame in a window: 7.190-8.524 ms across 15
  separate one-hull captures.
- **Phase SHARES.** Across a 2.9x whole-process slowdown, `Prepare` read 48.2%
  and 45.3% of the frame, `Render/graph` 22.0% and 24.2%, `PostUpdate` 19.1%
  and 19.0%. The decomposition is stable even when the total is not.

---

## 5. Where the 16.1 ms goes, named

One hull, 1280x720, real display, median over 15 captures. Indented rows are
inside the row above them.

| item | s=0 | **s=1** | s=2 | s=3 | share @ s=1 |
|---|--:|--:|--:|--:|--:|
| **frame** | 3.45 | **28.41** | 40.71 | 43.33 | 100% |
| render world | 3.04 | 27.13 | 39.22 | 42.05 | 95.5% |
| &nbsp;&nbsp;**`Prepare`** | 1.07 | **13.66** | 19.86 | 20.19 | **47.9%** |
| &nbsp;&nbsp;&nbsp;&nbsp;`Prepare/WritePhaseBuffers` | 0.04 | **5.69** | 8.88 | 8.83 | 19.9% |
| &nbsp;&nbsp;&nbsp;&nbsp;`Prepare/BindGroups` | 0.29 | **5.45** | 8.14 | 8.78 | 19.1% |
| &nbsp;&nbsp;&nbsp;&nbsp;`Prepare/Resources` | 0.48 | 1.67 | 1.84 | 1.78 | 6.0% |
| &nbsp;&nbsp;**`PrepareAssets`** | 0.12 | **4.30** | 7.39 | 8.80 | **15.1%** |
| &nbsp;&nbsp;`Render/graph` | 1.21 | 6.17 | 8.04 | 7.94 | 22.2% |
| &nbsp;&nbsp;`PrepareMeshes` | 0.06 | 0.30 | 0.38 | 1.13 | 1.3% |
| &nbsp;&nbsp;`Render/submit+present` | 0.13 | 0.40 | 0.35 | 0.38 | 1.4% |
| main world | 2.18 | 11.39 | 14.10 | 16.45 | 40.4% |
| &nbsp;&nbsp;`PostUpdate` | 0.90 | 5.14 | 6.41 | 7.74 | 18.3% |

**The 16.1 ms is three items, not two:** `Prepare/WritePhaseBuffers` 5.69,
`Prepare/BindGroups` 5.45, `PrepareAssets` 4.30. Together **15.4 ms of a
28.4 ms frame, 54%.** `PrepareMeshes` - which the brief named as half the
target - is **0.30 ms**, one percent, and is not a lever.

## 6. The sublinearity, answered

The census is deterministic and noise-free, so this part needs no repeats:

| ships | mesh instances | distinct meshes | material assets | frame ms |
|--:|--:|--:|--:|--:|
| 0 | 0 | 0 | 10 | 3.45 |
| 1 | 986 | 120 | 74 | 28.41 |
| 2 | 2,211 | 195 | 126 | 40.71 |
| 3 | 3,423 | 242 | 152 | 43.33 |
| **marginal, hull 1 / 2 / 3** | +986 / +1225 / +1212 | **+120 / +75 / +47** | +64 / +52 / +26 | +24.96 / +12.30 / +2.62 |

**Instances are LINEAR in ship count. Distinct assets are not.** Fitting the
frame against each predictor over all four points:

| predictor | fit | R^2 | marginal cost per unit, hull by hull | drift |
|---|---|--:|---|--:|
| **distinct meshes** | `5.25 + 0.170 ms each` | **0.974** | 0.208 / 0.164 / 0.056 | 3.7x |
| material assets | `3.17 + 0.285 ms each` | 0.963 | 0.390 / 0.237 / 0.101 | 3.9x |
| mesh instances | `10.35 + 0.0113 ms each` | 0.843 | 0.0253 / 0.0100 / 0.0022 | **11.7x** |
| entity count | `4.93 + 0.0047 ms each` | 0.843 | - | 11.7x |

On `min_ms`, where the contention is out of the way, it is not close:

| predictor | R^2 | marginal, hull by hull | drift |
|---|--:|---|--:|
| **distinct meshes** | **0.996** | 0.0437 / 0.0442 / 0.0280 | **1.58x** |
| material assets | 0.992 | 0.0820 / 0.0637 / 0.0506 | 1.62x |
| mesh instances | 0.910 | 0.00532 / 0.00271 / 0.00109 | 4.90x |

**A hull costs what it INTRODUCES to the frame, not what it draws.** The first
hull is 3x the marginal one because it stands up the whole catalog: 120 distinct
meshes and 64 new materials. The second hull draws 1,225 more instances - MORE
than the first hull's 986 - and costs half as much, because 62% of its meshes
are ones the first hull already brought. The third adds 47 and costs 2.6 ms.

So the intercept D12 called "a fixed cost that appears the moment there is ONE
hull" is real and now has a name: **it is the per-frame cost of the distinct
meshes and materials a hull introduces, and it is paid every frame, on a scene
where nothing changes.**

That last clause is the finding. 120 distinct meshes at 0.17 ms each is 20 ms
of per-frame work proportional to how many DIFFERENT assets are on screen, in a
frozen gallery where no asset, transform or visibility changes between frames.
Per-instance work is not the problem: at the margin an instance costs 2.2
microseconds and a distinct mesh costs 56 - **25x**.

---

## 7. The same 16.1 ms, by SYSTEM

`--features debug,trace` (bevy's per-system spans + `trace_chrome`), one hull,
steady-state slice of the timeline, 199 frames. Tracing inflates everything
uniformly, so read the SHARES; the ordering and the ratios are what this pass
is for.

| ms/frame | system | lands in |
|--:|---|---|
| 1.741 | `gpu_preprocess::prepare_preprocess_bind_groups` | `Prepare/BindGroups` |
| 1.466 | `material::prepare_material_bind_groups` | `Prepare/BindGroups` |
| 1.322 | `write_binned_instance_buffers<Opaque3d, MeshPipeline>` | `Prepare/WritePhaseBuffers` |
| **1.107** | **`prepare_erased_assets<ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>>`** | **`PrepareAssets`** |
| 0.382 | `write_binned_instance_buffers<Shadow, MeshPipeline>` | `Prepare/WritePhaseBuffers` |
| 0.367 | `render::mesh::prepare_mesh_bind_groups` | `Prepare/BindGroups` |
| 0.295 | `cluster::gpu::prepare_clustering_bind_groups` | `Prepare/BindGroups` |
| 0.262 | `prepare_mesh_view_bind_groups` | `Prepare/BindGroups` |
| 0.261 | `prepare_clusters_for_gpu_clustering` | `Prepare/Resources` |
| 0.142 | `write_indirect_parameters_buffers` | `Prepare/WritePhaseBuffers` |
| 0.136 | `render::light::queue_shadows` | `Queue` |
| 0.120 | `render::light::specialize_shadows` | `Specialize` |
| 0.114 | `write_batched_instance_buffers<MeshPipeline>` | `Prepare/WritePhaseBuffers` |

The fourth row is not bevy's cost. It is ours, and it is a bug.

### 7.1 `thruster_shader_update_system` re-uploaded every exhaust material, every frame

```rust
let Some(mut material) = materials.get_mut(&**material) else { ... };
material.extension.thruster_input = *input * damage;
```

`Assets::get_mut` marks an asset MODIFIED whether or not the value moves, and a
modified material is re-extracted, re-uploaded and has its bind group rebuilt
in the render world that frame. On a frozen gallery with nothing burning, the
value written is the same number every frame - so the whole exhaust material
set was being re-prepared every frame for no visible reason.

`notes-ablation.md` measured this arm at ratio 0.867 (`ABL_NOTHRUST`, 13% of an
11-ship frame) and could name the system but not the mechanism. This is the
mechanism, and the fix is to read first and write only on a change.

**LANDED, `8a26ae31`.** Paired interleaved, two binaries alternated, 8 pairs,
one hull, real display:

| statistic | base median | fixed median | fixed/base median (min-max) |
|---|--:|--:|--:|
| `PrepareAssets` | 3.112 ms | 0.607 ms | **0.175 (0.11-0.66)** |
| `min_ms` | 7.909 ms | 5.708 ms | **0.733 (0.61-1.02)** |
| `mean_ms` | 23.303 ms | 16.428 ms | 0.685 (0.34-1.92) - straddles, do not quote |

**82% off `PrepareAssets` and 27% off the least-contended frame, from one
guard.** The mean's spread straddles 1.00 and measured nothing; the other two
do not. It is presentation by the epic's rule, and it is not even a trade -
writing the same value is a no-op in everything except change detection.

---

## 8. The ranked list

The 1v1 target is 16.67 ms. Two hulls measured **40.71 ms** before the thruster
fix, so the gap is about 24 ms and all of it is per-ship.

| # | lever | expected | kind | state |
|---|---|--:|---|---|
| 1 | **Exhaust material written every frame** | 27% of the frame at one hull; grows with drive count | presentation | **DONE** (`8a26ae31`) |
| 2 | **Fewer DISTINCT MESHES per hull** | **0.170 ms per distinct mesh per frame.** 120 at one hull, 195 at two. Halving the catalog a hull instantiates is worth ~10 ms at one hull and ~16 at two | presentation | the lead |
| 3 | **Fewer DISTINCT MATERIALS per hull** | 0.285 ms each; 74 at one hull, 126 at two. `prepare_material_bind_groups` alone is 1.47 ms/frame. The private per-section material is the known source (`notes-ablation.md`, ratio 0.520) | presentation | second |
| 4 | The `Shadow` phase is prepared and never drawn | ~0.64 ms/frame of CPU (`write_binned_instance_buffers<Shadow>` 0.382, `queue_shadows` 0.136, `specialize_shadows` 0.120) against **no `render/shadows` GPU pass at all**, at one hull as well as at zero | presentation | small, confirm first |
| 5 | `prepare_preprocess_bind_groups`, 1.741 ms/frame | bevy-internal; only reachable by cutting views or phases, so it rides on 4 | presentation | not directly actionable |

**2 and 3 are the same finding stated twice**, and section 6 is the argument
for both: a hull costs what it INTRODUCES. The corollary is unusual and worth
saying plainly to whoever picks this up - **drawing the same ship twice is
nearly free, and drawing two different ships is not.** A fleet of identical
hulls is cheap. The gallery is expensive because every hull is a fresh roll.

Nothing on this list changes what the game DOES, so none of it needs the owner
under `20260818-220812/TASK.md`. Item 2 is the only one that touches art
authoring rather than code, and that is where it should be argued.

---

## 9. What this rules OUT, with the number

- **Hidden-workspace measurement.** H/V median 0.984-0.993 on `min_ms` over 14
  paired captures, every spread straddling 1.00. Section 1.
- **The idle orbit as a cost.** One hull reads 28.41 ms with the camera frozen
  against the 27.60 ms `notes-floor.md` measured with it turning. Below this
  box's noise. Fixed anyway, for reproducibility, not for speed.
- **CPU frequency as the cause of run-to-run variance.** Peak clock was
  4894-5060 MHz across four runs whose means spanned 9.9 to 26.8 ms.
- **Fill and the GPU.** Every timed GPU pass at one hull sums to **1.815 ms of
  a 15.28 ms frame (12%)**, and the largest is `main_opaque_pass_3d` at
  0.727 ms.
- **The shadow PASS.** No `render/shadows` GPU span exists at one hull, two or
  three - the pass list has ten entries and shadows is not among them. But this
  does NOT rule out shadows as `notes-floor.md` claimed on the empty gallery:
  the CPU-side `Shadow` phase runs, and costs ~0.64 ms/frame preparing a phase
  that draws nothing. That is item 4 above.
- **Present.** `Render/submit+present` is 0.40 ms at one hull, 1.4% of the
  frame. `immediate` reads 243-344 FPS on the empty gallery, `mailbox` the
  same; only `fifo` caps, at exactly 165.0 FPS.
- **`PrepareMeshes`.** 0.30 ms, 1.3% of a one-hull frame. The brief named it as
  half the target; it was `PrepareAssets` wearing its label (section 2.4).
- **Mesh instances, and entity count, as the cost driver.** `R^2` 0.91 against
  `min_ms` where distinct meshes give 0.996, and the marginal cost per instance
  falls 4.9x across three hulls where the marginal cost per distinct mesh moves
  1.58x. At the margin an instance costs 2.2 us and a distinct mesh 56 us.
- **Fixed-step amplification.** `fixed_steps max=1` in every one of ~87
  captures, `NOVA_PERF_MAX_DELTA=0.015625` pinned throughout.
- **The window size, now.** Every capture in this page logs its own
  `window=1280x720 logical, 1280x720 physical`, and one that did not was
  refused (section 2.1).

## 10. What is left open

- **Is the per-distinct-mesh cost really per MESH, or per BIN?** Bins are keyed
  by mesh and material together, and both scale together across this sweep, so
  the four ship counts cannot separate them. The discriminator is a scene with
  many meshes sharing one material, or one mesh under many materials.
  `wfc_ships --bare` (skin off, same structure) is one arm of it and is already
  wired.
- **The box.** On the operator's desk a whole run lands 2-3x apart from the one
  before it, with every phase scaling together. Interleave, quote `min_ms` and
  phase SHARES, and do not quote a mean whose spread straddles 1.00.

## 11. One trap left standing, deliberately

`nova_debug::force_capture_resolution` forces 1920x1080 at `Startup` and is
added unconditionally by about thirty examples. Under a measured run that is a
second writer for the window the capture owns, and it is now REFUSED rather
than silently obeyed. Two examples wire both and are fixed here (`wfc_ships`,
`wfc_arena`, both verified capturing at `window=1280x720 logical, 1280x720
physical`); the rest wire `.without_frametime()` and are unaffected.

It is not fixed at the source, and the reason is a dependency edge.
`force_capture_resolution` lives in `nova_debug`, which cannot see
`nova_core::PERF_ENV`, and `CONVENTIONS.md` forbids reaching a constant by
adding an edge - the constant would have to move DOWN to `nova_autopilot`
beside `CAPTURE_ENV`, which is a change to three crates and is not this lane's
to make. **The gate is what makes it safe to leave**: a future example that
wires both now fails loudly on its first warm-up frame instead of filing a
1920x1080 reading as 720p, which is exactly the failure this whole page exists
to close.
