# Phase B4: what the empty frame is made of

**The answer, up front. There is no floor.** The 16.74 ms an empty gallery was
measured at is 13.7 ms of Xvfb and 3.0 ms of game. Run the same scene, the same
binary, the same window size and the same pin against the real display and it
costs **3.02 ms - 331 FPS**. The whole difference sits in one place: bevy's
`render_system` blocking after the graph, and the block is a function of WINDOW
PIXELS and of nothing else in the frame.

Every absolute millisecond in `notes-ablation.md` and in the owner's hand-run is
therefore inflated by a per-pixel constant that has nothing to do with the game.
The RATIOS in that page survive - the private-material finding is untouched, and
so is `6b3bfc87` - but the floor it fitted, the `21.5 ms + 10.37 ms/ship` line,
and the "the floor alone is 100.4% of a 60 FPS budget" premise in
`20260818-220812/DECISIONS.md` D11 do not.

The 60 FPS question is not "can the floor reach 12 ms". It is 3.0 ms already.
The question is entirely the per-ship cost, and on real hardware ONE static hull
costs 24 ms.

## 1. The instrument

Three things landed in `nova_probe`, all env-gated, all inert in an ordinary run.

- **`capabilities/census.rs`** - one pass over the archetype graph at a fixed
  frame after `Playing`: total entities, entity count per COMPONENT, the
  archetypes those entities fall into, and mesh instances against DISTINCT mesh
  handles. `census.json` beside the frame-time artifacts. This is the "what ARE
  these 1,124 entities" instrument task `20260820-003401` asks for.
- **`capabilities/framecost.rs`** - a marker schedule between every pair of
  main-world schedules, a marker system between every pair of top-level
  `RenderSystems` sets, and a pair bracketing the `RenderGraph` schedule inside
  `render_system`. Plus bevy's `RenderDiagnosticsPlugin`, which nothing in the
  tree had added, for per-pass GPU timestamps.
- **`NOVA_PERF_PRESENT`** - forces the primary window's presentation mode.
  `AutoNoVsync` is only a REQUEST: wgpu falls back Immediate -> Mailbox -> Fifo
  silently, and bevy logs the fallback ONLY for an explicitly named mode. The
  probe's own docs claimed "vsync off" and could not prove it.

Two mechanical notes that cost time and are worth writing down:

- `MainScheduleOrder::insert_before` compares the label it is handed against the
  interned ones by dynamic equality, so handing it an already-interned label
  panics with `Expected ... to exist`. Rewrite `labels` wholesale instead.
- GPU timestamps need `wgpu::Features::TIMESTAMP_QUERY`, which is not in bevy's
  default feature set. `nova_core::RENDER_DIAG_ENV` gates the request in
  `render_plugin()`; asking for it costs a resolve pass and a readback, measured
  at about 3% of the frame, so it is never a default.

Subject: `wfc_ships --ships 0`. The example's `ships.max(1)` is gone, so the
empty stand - sky, photo rig, HUD, no subject - is reachable from the CLI. The
example also wires `nova_frametime()` when, and only when, `NOVA_PERF` is set,
so a capture can be pointed at it without changing what `probe run` grades.

Window 90 warm-up + 200-400 frames, `NOVA_PERF_MAX_DELTA=0.015625`,
`NOVA_PERF_RES=1280x720`, `NOVA_PERF_PRESENT=immediate`, one capture per
process, interleaved with a fresh reference before every arm.

**Fixed steps, every capture.** Xvfb: `min=0 max=1 mean=0.985-0.993`. Real
display: `min=0 max=1 mean=0.190`. The pin holds - one step per frame at most,
never a queue - and on the real display most frames are under 15.625 ms so most
run no step at all. Nothing here is fixed-step amplified.

## 2. The 16.74 ms, by name

Empty gallery, 1280x720. Medians over the reference captures of three sweeps
(7 captures Xvfb, 5 real display); the raw rows are in
`measurements/floor-arms.csv`, the full tables in
`measurements/floor-breakdown.txt`.

| item | Xvfb ms | real display ms |
|---|--:|--:|
| **frame** | **16.74** | **3.02** |
| main world, all schedules | 2.56 | 1.77 |
| render world, all phases | 14.09 | 2.66 |
| GPU, all timed passes | 4.62 | 0.31 |

The main world and the render world overlap - bevy runs the main world for frame
N while the render thread is on frame N-1 - so a frame costs about the longer of
the two. **The render world is the pacer in both.**

Inside the render world:

| render phase | Xvfb ms | real ms |
|---|--:|--:|
| **`Render`/submit+present** | **11.45** | **0.115** |
| `Render`/graph | 1.15 | 1.150 |
| `Prepare` | 1.22 | 0.873 |
| `PrepareMeshes` | 0.22 | 0.144 |
| `CreateViews` | 0.17 | 0.133 |
| `PrepareViews` | 0.10 | 0.074 |
| `ExtractCommands` | 0.09 | 0.056 |
| `Queue` | 0.07 | 0.048 |
| `Specialize` | 0.04 | 0.029 |
| `PhaseSort` | 0.02 | 0.017 |
| `PostCleanup` | 0.03 | 0.013 |
| `Cleanup` | 0.03 | 0.008 |

Inside the main world:

| main schedule | Xvfb ms | real ms |
|---|--:|--:|
| `PostUpdate` | 0.86 | 0.705 |
| `Update` | 0.68 | 0.549 |
| `PreUpdate` | 0.29 | 0.195 |
| `RunFixedMainLoop` | 0.52 | 0.120 |
| `StateTransition` | 0.10 | 0.065 |
| `Last` | 0.07 | 0.054 |
| `First` | 0.10 | 0.043 |
| `SpawnScene` | 0.05 | 0.037 |

On the GPU, real display, every pass the device timed:

| pass | ms |
|---|--:|
| `bloom` | 0.122 |
| `clustering` | 0.073 |
| `main_opaque_pass_3d` | 0.051 |
| `tonemapping` | 0.024 |
| `upscaling` | 0.020 |
| `bin_unpacking` | 0.007 |
| `early_mesh_preprocessing` | 0.007 |
| `ui` | 0.004 |
| `main_transparent_pass_2d` | 0.002 |
| **total** | **0.312** |

`main_opaque_pass_3d` shades 1,014,720 fragments - 1.10x the window - in
0.051 ms. That is the skybox, drawn as three vertices, and it is the ONLY 3D
draw in the frame.

## 3. Fill-bound, CPU-bound or fixed - decided by the sweep

Resolution sweep on the EMPTY scene, Xvfb, interleaved:

| window | pixels | frame ms | submit+present ms |
|---|--:|--:|--:|
| 160x90 | 1/64 | 16.76 | **1.45** |
| 640x360 | 1/4 | 16.72 | **5.05** |
| **1280x720** | 1 | **16.74** | **11.45** |
| 2560x1440 | 4 | 54.68 | **50.35** |

`submit+present` is linear in window pixels across two and a half decades. The
FRAME is not, because below 720p something else pins it - every phase of both
worlds inflates by about 2.8x at 160x90 while the frame stays at 16.76, which is
what a blocked process looks like from the inside.

Then the discriminator that separates "the GPU is drawing those pixels" from
"the window is presenting them" - the shipped `render_scale` knob, which renders
into a smaller offscreen target and blits it to the same window:

| arm | frame ms | submit+present ms |
|---|--:|--:|
| reference, 720p | 16.75 | 11.45 |
| `render_scale` 0.25 (renders 320x180) | 16.93 | **11.17** |
| `render_scale` 0.5 (renders 640x360) | 16.85 | **10.83** |

**1/16 of the shading, and the block does not move.** It is not the drawing. It
is the window, and on Xvfb - a software X server with no scanout - presenting a
window is a CPU-side image transfer of every pixel. At 720p that is 3.7 MB a
frame at about 320 MB/s, which is the number.

The real display settles it. Same binary, same 1280x720, same pin, same
Immediate presentation:

| host | frame ms | submit+present ms | mean FPS |
|---|--:|--:|--:|
| Xvfb | 16.74 | 11.45 | 59.7 |
| **`DISPLAY=:0`** | **3.02** | **0.115** | **331** |

**Verdict: the empty frame is CPU-bound in the render world, and it is 3.0 ms.**
GPU 0.31 ms of 3.02 is 10%; the largest single item is the CPU cost of walking
the render graph, at 1.15 ms. Fill is not binding anywhere near 720p and never
was - the 8% the old 11-ship sweep bought at 160x90 was 8% of the PRESENT, not
of the render.

## 4. What the 1,124 entities are

Census of the empty gallery: **1,176 entities in 586 archetypes, zero meshes,
zero sections, 15 mesh assets, 10 standard materials, 39 images.** (1,176 rather
than the 1,124 the earlier lane counted - the census is taken 90 frames after
`Playing`, and this tree is 5 commits further on.)

| what | entities | share |
|---|--:|--:|
| bevy resources, which are entities in 0.19 (`IsResource`) | 548 | 47% |
| observers (`Observer`, one component each) | 433 | 37% |
| HUD UI nodes (`Node` + `ComputedNode` + 18 more) | 161 | 14% |
| ... of which carry `Text` | 113 | |
| input bindings (`Binding`/`BindingOf`) | 14 | 1% |
| drawable transform entities (the three-point rig) | 3 | 0.3% |
| everything else - camera, sky, readout roots | ~17 | 1% |

**84% of the "1,124 entities" are not scene at all.** They are the resource
table and the observer registry, and nothing iterates either per frame. Of the
~195 that are real entities, 161 are HUD nodes. There is no scene in an empty
gallery; there is a HUD, four transforms, and bevy's own bookkeeping.

That is also why 0.705 ms of `PostUpdate` is the largest main-world item: it is
bevy UI layout and text over 161 nodes, plus transform propagation over four.

## 5. The ranked cut list

The epic's rule (`20260818-220812/TASK.md`, "What may be traded for frame rate")
splits presentation from physics and gameplay logic. Applied to the floor, the
list is short, because **the floor is 3.02 ms against a 16.67 ms budget - 18% -
and every item in it is under a millisecond.**

| # | candidate | measured | kind | verdict |
|---|---|--:|---|---|
| 1 | **Stop measuring on Xvfb** | **13.7 ms** | harness | Take it. It is not a game cost and it is 82% of the number this phase was sent to explain. |
| 2 | Render graph CPU walk (`Render`/graph) | 1.150 ms | presentation | Fixed cost of the node chain - skybox, opaque, bloom, tonemapping, upscaling, UI. Cutting a node cuts a fraction of it. Not worth it at 7% of a 60 FPS budget. |
| 3 | `Prepare` (view uniforms, bind groups, UI batching) | 0.873 ms | presentation | Same. It is where a LOADED frame's cost lives (see below), but on an empty one it is 5% of budget. |
| 4 | `PostUpdate` - UI layout over 161 HUD nodes + propagation | 0.705 ms | presentation | Measured directly: HUD on vs `HudVisibility::Cinematic`, 3 interleaved passes, p50 2.86 ms vs 2.80 ms. **Ratio 1.02, spread straddles 1.00 - the HUD costs nothing.** Do not touch it. |
| 5 | `Update` | 0.549 ms | gameplay logic | 3% of budget on an empty scene. Needs the owner and is not worth asking. |
| 6 | Bloom | 0.122 ms GPU | presentation | 39% of the empty frame's GPU and 0.7% of a 60 FPS budget. Take it only if it is free elsewhere. |

**Nothing in the floor is worth cutting.** The correct next lever is not on this
list at all, and it is where the epic's 1v1 target actually rides.

### Where the frame really goes, once a ship exists

Same instrument, real display, `wfc_ships --ships 1`:

| item | empty ms | one ship ms |
|---|--:|--:|
| frame | 3.02 | **26.01** |
| render world | 2.66 | 24.27 |
| ... `Prepare` | 0.873 | **11.92** |
| ... `Render`/graph | 1.150 | 5.89 |
| ... `PrepareMeshes` | 0.144 | **4.20** |
| ... `Render`/submit+present | 0.115 | 0.354 |
| main world | 1.77 | 10.49 |
| ... `PostUpdate` | 0.705 | 4.88 |
| ... `RunFixedMainLoop` | 0.120 | 2.22 |
| ... `Update` | 0.549 | 1.88 |
| GPU, all passes | 0.312 | 2.93 |

Census at one ship: 3,473 entities, 986 mesh instances over 120 distinct meshes,
74 standard materials. **`Prepare` plus `PrepareMeshes` is 16.1 ms of a 26 ms
frame** - per-instance buffer writes and bind groups, on the CPU, in the render
world. The GPU is 11% of the frame. That is the same axis `notes-ablation.md`
found and the crack-bucket fix moved; it is still the axis.

## 6. What this rules OUT, with the number

- **Vsync / presentation mode.** Xvfb, empty scene: Immediate 17.32 ms, Mailbox
  17.59 ms (logs its fallback to Immediate, so Immediate is genuinely available),
  Fifo 17.64 ms. **1.02x across all three.** On the real display Immediate reads
  331 FPS. Presentation MODE is not the floor; the presentation PATH was.
- **Fill / pixel shading.** `render_scale` 0.25 renders 1/16 the pixels and
  leaves the frame at 16.93 vs 16.75 ms. On the real display every timed GPU
  pass sums to 0.312 ms of a 3.02 ms frame.
- **Shadow maps.** The scene's one shadow-casting `DirectionalLight` emits **no
  `render/shadows` span at all** - the pass list has nine entries and shadows is
  not among them. With no casters there is no pass. 0.000 ms.
- **The NOVA OS CRT render-to-texture camera.** Exactly one
  `main_opaque_pass_3d` and one `upscaling` span in the whole frame. A second
  active camera would produce a second set. Confirmed inert.
- **The render-scale RTT + blit path.** Same evidence, plus the `render_scale`
  arms above reading 1.01x. At native quality the camera draws straight to the
  window.
- **`process_pipeline_queue_system` in steady state.** The whole
  `RenderSystems::Render` set is 1.265 ms on the real display, of which the
  render graph is 1.150 ms. Everything else in that set - the pipeline queue,
  the submit and the present together - is **0.115 ms**. Synchronous pipeline
  compilation costs nothing after warm-up, and must stay as it is.
- **The HUD.** 1.02x over three interleaved passes (item 4 above).
- **Fixed-step amplification.** Pinned in every capture; `mean=0.19` steps a
  frame on the real display, `mean=0.99` on Xvfb, never more than one.
- **Entity count.** 1,176 entities, of which 981 are the resource table and the
  observer registry. Per-entity work in an empty gallery is 0.705 ms of
  `PostUpdate` over 165 real entities.

## 7. Is 12 ms reachable? Is 9.5 ms?

Both are already met, by 4x and 3x respectively. **The measured floor is
3.02 ms**: 18% of a 60 FPS frame, 9% of a 30 FPS one. Neither target needs
anything cut, and neither is what stands between the game and 1v1 at 60.

What DOES stand there, measured on the same day, same box, same instrument:

| subject (real display, 1280x720) | frame ms | FPS |
|---|--:|--:|
| empty gallery | 3.02 | 331 |
| one static hull | 27.60 | 36 |
| two static hulls | 34.82 | 29 |
| three static hulls | 44.04 | 23 |

Two hulls posed and frozen - no AI, no weapons, no projectiles, less than a real
1v1 - is **34.8 ms, 29 FPS.** The gap to 16.67 ms is 18 ms, and 100% of it is
per-ship cost. The line is strongly sublinear (24.6 ms for the first hull, 7.2
for the second, 9.2 for the third), which the old Xvfb line could not see
because an 11 ms present sat on top of every reading; the `wfc_ships` camera
also pulls back as the row grows, so pixel coverage per hull falls. Whether the
frame tracks ships or sections is still the open question `notes-ablation.md`
left, and it should now be re-asked on a real display.

**What each target would cost:**

- ~12 ms floor: nothing. It is 3.02 ms.
- ~9.5 ms floor: nothing. It is 3.02 ms.
- **1v1 at 60 FPS: 18 ms off two hulls.** The lead is `Prepare` +
  `PrepareMeshes`, 16.1 ms of a 26 ms one-ship frame, on the CPU in the render
  world - per-instance buffer writes and bind groups over 986 mesh instances
  drawing through 120 distinct meshes. That is presentation, so it is takeable
  by the epic's own rule. It is also the same mechanism `0ee9cbb0` regressed and
  `6b3bfc87` half-fixed.

## 8. The rule this leaves behind

**A capture taken under Xvfb measures the X server as much as the game, and the
error is not a scale factor - it is an additive per-pixel constant that lands on
top of every arm.** At 720p it is 11.5 ms a frame; at 1440p it is 50. Any A/B
whose two arms share a window size still divides it out, which is why every
RATIO in `notes-ablation.md` stands. Any statement in milliseconds does not, and
neither does any statement about a BUDGET, because a budget is absolute.

Two consequences to act on:

1. Absolute frame-time numbers - the sweep baselines, the report's "flagged
   under 60 FPS" gate, D11's 100.4% - need re-measuring on a real device before
   they mean anything. The probe's own docs say frame numbers are comparable
   across runs; they are comparable across runs ON THE SAME PRESENTATION PATH.
2. The real display is not a controlled environment either. One interleaved
   sweep in this session caught its reference at 16.67 ms with `min=5.0` - a
   60 Hz composited path - while the arm beside it ran at 2.9 ms; three repeats
   afterwards read 3.03, 3.01 and 3.01. A capture host needs an offscreen or
   direct-scanout path that is neither, and until there is one, every absolute
   number needs repeats and a stated present mode.
