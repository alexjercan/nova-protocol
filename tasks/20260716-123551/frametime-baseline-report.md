# Gameplay frame-time baseline: heavy scenes, native + web

Sprint v0.7.0, branch `perf-baseline-frametime`, task `20260716-123551` (p40).
Descends from spike `20260716-122954` (v0.7.0 scope) and strand 2 of
`docs/plans/20260716-v0.7.0-plan.md`.

This is the **measurement half only**. Per the task and the user's direction, it
produces the baseline numbers and the tooling to reproduce them; it does NOT
apply optimizations. Fixes are decided together, from these numbers, under the
same measure-first gate the v0.6.0 modding-perf work used
(`tasks/20260714-083331/modding-perf-report.md`).

## TL;DR

- Built a whole-frame capture harness (`nova_frametime`, an env-gated plugin)
  plus the `20_perf_baseline` example and a sweep script. It drives the **real
  gameplay app**, records the wall-clock delta of every frame, and writes
  percentile stats. Numbers below are `1280x720`, vsync off, at rest.
- **On discrete GPU (RTX 3060 Ti), no heavy scene is close to the 16.6 ms / 60
  fps budget at rest.** The true per-frame cost, measured through a real
  swapchain, is ~**5-9 ms** on the fast frames. There is no native rendering
  problem to fix here yet.
- **The at-rest frame cost is fixed-overhead / CPU-bound, not scene-bound.** On
  the discrete GPU all three scenes land within a hair of each other
  (~19-21 ms mean on the Xvfb rig, which adds a fixed software-present cost);
  the *heaviest* authored scene (`shakedown_run`) is not the slowest. Scene
  content only starts to dominate when you remove the GPU (software raster).
- **The graphics preset barely moves the at-rest number** (Low vs High: -13% on
  `asteroid_field`, -7% on `broadside`, ~0% on `shakedown_run`). Its two levers
  (particles, scatter-density) do little at rest: the authored scenes use
  hand-placed objects (not the procedural scatter `scaled_count` thins), and no
  torpedoes/impacts are firing, so the particle toggle is idle.
- **Combat burst (now measured): combat costs real frame time; particles are a
  measurable slice of it.** Driving sustained fire on `broadside` (496 bullets in
  flight + muzzle particles + AI return fire, combatants kept alive) raises
  saturated frame time to ~**29 ms** (High) vs ~**19 ms** at rest (+54%), and the
  particle toggle accounts for ~**11%** of the combat frame (High 29.4 ms vs Low
  26.0 ms - up from ~7% at rest). So the preset's `particles` lever earns its
  keep during combat, not at rest. Caveat: combat cost is volatile (bullet count
  ramps, AI engage/evade, ammo reloads) and this box is a contended shared host,
  so treat these as order-of-magnitude, not 3-sig-fig.
- **The weak-hardware / web low end is fill-bound.** Software raster (lavapipe,
  the worst-case CPU floor) runs these scenes at **8-12 fps**, and there the
  ordering flips: `asteroid_field` is the *slowest* (126 ms) and `shakedown_run`
  the fastest (86 ms), tracking screen coverage/overdraw, not entity count.
- **Web/WebGPU (now captured): ~2x native, uniformly over budget.** The same
  harness compiled into the wasm/Trunk build and driven by headless Chromium
  (real WebGPU on the NVIDIA GPU, `backend: BrowserWebGpu`) runs the three scenes
  at **34-39 ms (26-29 fps)** at rest and `broadside` combat at **42 ms (24
  fps)** - all well over the 16.6 ms budget. Web is the constrained target the
  preset exists for, and unlike the discrete-GPU native path it has little
  headroom, so combat and preset cuts bite harder here.

## Why a frame-time baseline (and why now)

"Improve performance" had no target. v0.6.0 benchmarked the modding *dispatch*
layer (CPU, criterion microbench) but never measured a whole rendered gameplay
frame. Three things now depend on real numbers:

1. The **graphics preset** (`GraphicsQuality` Low/Medium/High, task
   `20260525-133013` + `20260711-180511`). Its derived `GraphicsBudget`
   fractions (`particles`, `scatter_density`) are explicitly *provisional
   pending this baseline* - see the comment on `GraphicsBudget::for_quality`
   in `crates/nova_gameplay/src/settings.rs:118`.
2. The **low-end / web target**. Web is the constrained platform; the preset
   exists to make the game playable there.
3. Any future optimization needs a **gate**: a reproducible before/after so a
   change is justified by a measured win, not a plausible story.

## What "heavy scene" means here

The three shipped scenarios the task names
(`assets/base/scenarios/*.content.ron`):

| Scenario | Role | Authored weight |
|----------|------|-----------------|
| `asteroid_field` | dense asteroid scatter + gravity well | 20 scattered asteroids, gravity rock, player ship |
| `broadside` | chapter-two combat slice | hostile corvettes, turrets, torpedo bays, particles |
| `shakedown_run` | the vertical slice | the largest authored scenario (1279 lines), multi-stage |

## Method

A new env-gated capture plugin, `nova_perf::nova_frametime`, drives the
**real gameplay app** (the exact `AppBuilder` plugin stack the binary runs:
render + ECS + avian physics + gravity + hanabi particles + HUD) to `Playing`,
then:

1. Forces the primary window to a fixed size (**1280x720**) with **vsync off**
   (`PresentMode::AutoNoVsync`) and **continuous updates**
   (`WinitSettings::game`), so the loop is not pinned to a refresh or throttled
   when unfocused.
2. Discards a **warm-up** window (300 frames here) to shed shader-compile,
   asset-upload and first-frame spikes.
3. Records the **wall-clock delta of every frame** (`Time<Real>`) for a fixed
   capture window (600 frames on GPU, 120 on the slow software rig), computes
   percentile stats (nearest-rank, so every `pXX` is a real observed frame),
   writes `<label>.json` + a row in `frametime.csv`, and exits.

The "1% low fps" figure is `1000 / p99_ms` - the standard stutter-floor number.

### What this captures, and what it does not

- It measures the **loaded scene at rest**: everything spawned, physics and
  gravity integrating, the full render graph running, the camera framing the
  scene. The honest steady-state cost of *being in* the scene.
- With `NOVA_PERF_COMBAT=1` it also measures an **active combat burst**: the
  harness's `drive` hook runs `combat_burst_driver` every frame - it raises the
  player's weapons and holds the turret trigger (the proven headless fire chain:
  RMB "Combat Mode" -> wait for `WeaponsHot` -> hold LMB "Turret"), and tops up
  every combatant's `Health` so the burst is sustained and no kill advances the
  scenario mid-capture. AI hostiles engage on their own and add return fire and
  torpedo blasts. Verified firing (496 projectiles + live muzzle particles over
  240 frames in a trace run), so the burst cost is measured, not guessed.

### The measurement rigs (and their honest caveats)

Capturing clean GPU frame times on this box is constrained: it is a **live,
shared developer desktop** (a logged-in user, Firefox, rust-analyzer, and sibling
background agent jobs all competing for CPU), so absolute numbers carry noise.
Three vantage points, each with a different bias:

| Rig | What it is | Bias |
|-----|-----------|------|
| **`:0` real swapchain** | the actual desktop display, real windowed present | 60 Hz **compositor vsync** clamps the median to ~17 ms and desktop contention inflates the tail; but the *fast frames* (~5-9 ms min) are the truest per-frame cost, since present is a real GPU flip with no software copy |
| **Xvfb + NVIDIA** (`xgpu`) | RTX 3060 Ti rendering into a headless Xvfb window (Vulkan WSI); no compositor, no visible window | compositor-free and repeatable, so good for **relative** comparison and stall detection, but the Xvfb software **present-copy adds a fixed ~10 ms/frame**, so absolute means (~19-21 ms) are inflated |
| **Software raster** (`sw`) | lavapipe/llvmpipe (CPU) via a forced software Vulkan ICD, under Xvfb | no GPU at all: pure CPU + software raster, the **worst-case floor**. Not a browser-WebGPU stand-in (a real weak GPU sits between this and the 3060 Ti) but it brackets the low end and isolates fill cost |
| **Web / WebGPU** (`web`) | the wasm build in Chromium, real WebGPU on the RTX 3060 Ti (`BrowserWebGpu`), under Xvfb | the shipped web backend; adds the browser/wasm/WebGPU-API overhead on top of the GPU, so ~2x the native Xvfb figure. The actual constrained target, not a proxy |

Reading them together separates the axes: the ~5-9 ms `:0` floor is the real
native cost; the flat ~20 ms Xvfb means (± scene) show the GPU path is
overhead/CPU-bound not scene-bound; the 86-126 ms software numbers show what the
*rendering* costs once the GPU is removed.

## Results

### Native, discrete GPU - Xvfb + RTX 3060 Ti (1280x720, vsync off, 600 frames)

| Scenario | Preset | mean (ms) | p50 | p95 | p99 | min | max | mean fps | 1% low fps |
|----------|--------|----------:|----:|----:|----:|----:|----:|---------:|-----------:|
| asteroid_field | High | 19.27 | 18.46 | 24.11 | 29.53 | 16.94 | 32.93 | 51.9 | 33.9 |
| asteroid_field | Low  | 16.73 | 15.74 | 24.63 | 28.98 | 11.77 | 36.33 | 59.8 | 34.5 |
| broadside      | High | 21.28 | 18.64 | 28.19 | 38.91 | 17.06 | 75.17 | 47.0 | 25.7 |
| broadside      | Low  | 19.75 | 19.15 | 23.45 | 26.14 | 15.91 | 29.86 | 50.6 | 38.3 |
| shakedown_run  | High | 20.75 | 20.56 | 21.90 | 23.98 | 19.67 | 31.08 | 48.2 | 41.7 |
| shakedown_run  | Low  | 20.51 | 20.47 | 22.52 | 25.77 | 15.84 | 31.83 | 48.8 | 38.8 |

Read this as **relative** (the Xvfb present-copy inflates every mean by a roughly
fixed ~10 ms; subtract it and the real-window figures match the `:0` ~5-9 ms fast
frames). Three things:

1. **Flat across scenes.** High-preset means span 19.3-21.3 ms - a ~10% spread
   across scenes whose authored content differs by an order of magnitude. The
   floor (min) is ~17-20 ms for all of them. A per-frame cost this insensitive to
   scene content is **fixed overhead** (CPU main-thread systems + present), not
   render/fill or entity count.
2. **Resolution-insensitive too.** A side check (same scene at 640x480 vs
   1920x1080) moved the mean by less than run-to-run noise - confirming the GPU
   is not fill-bound here.
3. **The one visible tail** is `broadside-high` (max 75 ms, p99 39 ms): an
   occasional hitch, plausibly asset streaming or a particle-system init, that
   the Low run does not show. Worth a glance but a single-frame event, not a
   sustained cost.

### Graphics-preset delta (Low vs High, same rig)

| Scenario | High mean | Low mean | delta |
|----------|----------:|---------:|------:|
| asteroid_field | 19.27 | 16.73 | **-13%** |
| broadside      | 21.28 | 19.75 | **-7%** |
| shakedown_run  | 20.75 | 20.51 | **~0%** |

The preset does little **at rest** because its two levers are mostly idle there:
`scatter_density` only thins *procedural* scatter via `GraphicsBudget::scaled_count`,
and these scenarios place asteroids with authored `SpawnScenarioObject` actions
that never call it; `particles` only matters when something is emitting, and
nothing fires at rest. So at-rest data alone cannot tune the preset fractions -
which is what the combat burst below is for.

### Combat burst - `broadside`, Xvfb + RTX 3060 Ti, saturated fire

The `combat_burst_driver` holds the player's turret trigger and keeps combatants
alive; captured after a long warm-up so the bullet population (5 s lifetime, ~100
rounds/s) and AI engagement (5 s grace) have saturated. Representative clean run
(load ~5):

| broadside | mean (ms) | p50 | p95 | p99 | min | mean fps | vs rest |
|-----------|----------:|----:|----:|----:|----:|---------:|--------:|
| at rest, High | 19.09 | 17.94 | 24.53 | 26.56 | 17.07 | 52.4 | - |
| **combat, High** | **29.37** | 28.82 | 33.73 | 37.55 | 26.02 | 34.0 | **+54%** |
| **combat, Low**  | **26.04** | 25.28 | 34.01 | 41.23 | 18.66 | 38.4 | +36% |

Findings:

1. **Combat costs real frame time.** Saturated fire raises the mean ~54% (19 ->
   29 ms) and, tellingly, lifts the *floor* (min 17 -> 26 ms): unlike the at-rest
   scene, combat has no slack frames - every frame carries ~500 live bullets +
   collision + muzzle particles + AI return fire.
2. **Particles are ~11% of the combat frame** (High 29.4 vs Low 26.0 ms) - up
   from ~7% at rest. So the preset's `particles` toggle earns its keep *during
   combat*. The other ~36% (rest -> combat-Low, particles off) is the
   non-particle combat work (bullets, collisions, AI) the preset does not touch.
3. **Combat is volatile.** Across repeat runs combat-High ranged ~20-38 ms clean
   (and to ~80 ms under CPU contention): bullet count ramps and decays, ammo
   reloads (500-round mag, 3 s), and AI cycles engage/evade, so the instantaneous
   cost swings. Combat's signature vs rest is a **fatter tail** (higher p95/p99)
   more than a higher median - the stutter a player feels in a firefight. Treat
   the table as representative, not precise (contended shared host - see rigs).

### Software-raster floor - lavapipe (CPU), 1280x720, 120 frames

| Scenario | Preset | mean (ms) | p50 | p99 | min | mean fps | 1% low fps |
|----------|--------|----------:|----:|----:|----:|---------:|-----------:|
| asteroid_field | High | 126.55 | 125.44 | 164.26 | 96.69 | 7.9 | 6.1 |
| asteroid_field | Low  | 117.87 | 117.94 | 151.34 | 88.61 | 8.5 | 6.6 |
| broadside      | High | 115.05 | 111.45 | 166.71 | 82.75 | 8.7 | 6.0 |
| broadside      | Low  |  98.89 |  98.25 | 133.27 | 72.38 | 10.1 | 7.5 |
| shakedown_run  | High |  86.55 |  84.27 | 121.92 | 54.85 | 11.6 | 8.2 |
| shakedown_run  | Low  |  85.71 |  85.01 | 128.02 | 55.90 | 11.7 | 7.8 |

Two findings:

1. **Ordering flips.** With the GPU gone, cost tracks **pixels shaded / overdraw**,
   not entity count: `asteroid_field` (big near-camera rock + dense field filling
   the frame) is the *slowest* at 126 ms; the "heaviest" `shakedown_run` frames
   more empty space and is the *fastest* at 86 ms. This is the fill cost the
   discrete GPU hides.
2. **The preset earns more here** where fill matters (`broadside` -14%), but
   still modestly, and still nothing at rest for `shakedown_run`.

Since software raster (all pixel work on CPU) is ~6x the GPU path while the GPU
path is scene-flat, the render/fill work the CPU does in software (~100 ms) is
exactly what the discrete GPU absorbs into a few ms - confirming the ~20 ms GPU
figure is **not** raster-bound.

### Web / WebGPU - Chromium (real WebGPU on the NVIDIA GPU), 1280x720, 300 frames

The same `perf_web` binary compiled into the wasm/Trunk build and driven by
headless Chromium under Xvfb. The wasm has no filesystem, so it logs its
`nova perf: label=...` line to the browser console and the driver scrapes it; on
wasm the config comes from the URL query string (`?perf=1&scenario=...`) instead
of env vars. The adapter is genuine WebGPU on the discrete GPU
(`AdapterInfo { name: "NVIDIA GeForce RTX 3060 Ti", backend: BrowserWebGpu }` -
saved in `perf-results/web/webgpu-adapter.txt`), the exact backend the shipped
site uses.

| Scene (High) | mean (ms) | p50 | p95 | p99 | min | mean fps | 1% low fps |
|--------------|----------:|----:|----:|----:|----:|---------:|-----------:|
| asteroid_field | 38.76 | 37.50 | 59.50 | 68.90 | 15.80 | 25.8 | 14.5 |
| broadside | 34.04 | 33.60 | 48.00 | 57.00 | 17.60 | 29.4 | 17.5 |
| shakedown_run | 35.64 | 36.60 | 54.10 | 56.90 | 14.70 | 28.1 | 17.6 |
| broadside + combat | 42.24 | 42.20 | 57.30 | 60.40 | 23.70 | 23.7 | 16.6 |

Findings:

1. **~2x native, uniformly over budget.** All three scenes sit at **34-39 ms
   (26-29 fps)** at rest on the *same GPU* that renders them at ~19-21 ms
   natively. The browser/wasm layer (JS<->wasm boundary, browser present, wgpu
   over the WebGPU API) roughly doubles the frame - and this is the strong-GPU
   case; a real laptop iGPU WebGPU target will be worse. Web is genuinely the
   constrained platform the graphics preset exists for.
2. **Flat across scenes, like native.** 34-39 ms regardless of authored content -
   web is overhead-bound too, not scene-content-bound.
3. **Combat bites harder on web.** `broadside` combat is **42 ms (24 fps)** vs
   34 ms at rest (+24%), and the floor climbs (min 17.6 -> 23.7 ms). Unlike the
   discrete-GPU native path, web has little headroom to absorb the combat load -
   so this is exactly where the preset's particle/scatter cuts (and any future
   render-scale lever) should be aimed.

Reproduce with `scripts/perf-web.sh <scenario>` (`QUALITY=`, `COMBAT=1`). Raw
scraped rows are in `perf-results/web/frametime.csv` (hand-recorded from the
console, since wasm cannot write files).

## Decisions

### 1. Frame-time capture harness (`20260716-123551`) - DONE

Shipped `crates/nova_perf/src/lib.rs` (the `nova_frametime` plugin, pure
unit-tested percentile stats), `examples/20_perf_baseline.rs` (boots any shipped
scenario by id, with a preset knob), and `scripts/perf-baseline.sh` (the sweep
driver). This is the reusable gate future perf work runs against. Numbers,
rigs and caveats are this report.

### 2. Combat-burst + web/WebGPU measurement - DONE

Both follow-ups are now delivered (the `combat_burst_driver` hook and the
`perf_web` wasm binary + `perf-web.sh`). Combat raises the frame ~54% saturated
and shows particles at ~11% of the combat frame; web/WebGPU runs the scenes at
~2x native, over budget. See the Combat-burst and Web results above. These close
the two gaps the first cut of this report flagged.

### 3. No native at-rest optimization - DEFER (measured)

On discrete GPU no scene is near the frame budget at rest, and the cost is fixed
CPU/present overhead, not scene content or GPU fill. There is nothing here whose
optimization the numbers justify. Combat *does* cost real frame time, but even
saturated combat (~29 ms) only dips a discrete GPU to ~34 fps - uncomfortable but
not the emergency. **The real over-budget target is web** (26-29 fps at rest,
24 fps in combat), so any optimization effort should be measured against the web
number, not the native one.

### 4. Graphics-preset fractions - HOLD, but now with a direction

The provisional `GraphicsBudget::for_quality` fractions still should not be
hand-tuned blind, but the data now points somewhere concrete:

- **`particles` is validated as a combat lever** (~11% of the combat frame),
  wasted at rest. Keep it; there is no reason to soften it on Low.
- **`scatter_density` is unproven for the shipped scenes** - they use authored,
  not procedural, scatter, so the multiplier never fires. Either wire authored
  scatter through `scaled_count` or stop advertising scatter-thinning as a lever.
- **The missing lever is fill / render-scale.** Both the software floor and the
  web numbers are fill/overhead-bound with little headroom; a resolution or
  render-scale drop would buy more there than particle/scatter toggles. Strongest
  candidate for a new Low-preset lever aimed at the web target.

Directional only - actual fraction changes are a separate task, decided from
these numbers with the user.

## Reproducing

Build once, then sweep every heavy scene x preset. The prebuilt binary needs
`BEVY_ASSET_ROOT` pointed at the repo (Bevy otherwise resolves `assets/` beside
the executable):

```bash
# Native discrete GPU into a headless Xvfb window (no compositor, no visible
# window, no screen hijack) - the rig this report's GPU table used:
Xvfb :95 -screen 0 1280x720x24 & 
NOVA_PERF=1 NOVA_PERF_SCENARIO=asteroid_field NOVA_PERF_QUALITY=high \
  NOVA_PERF_LABEL=asteroid_field-high NOVA_PERF_OUT=./perf-results/xgpu \
  NOVA_PERF_WARMUP=300 NOVA_PERF_FRAMES=600 BEVY_ASSET_ROOT="$PWD" DISPLAY=:95 \
  cargo run --release --example 20_perf_baseline --features debug

# Software-raster floor (forced lavapipe ICD):
ICD=/run/opengl-driver/share/vulkan/icd.d/lvp_icd.x86_64.json
VK_ICD_FILENAMES=$ICD VK_DRIVER_FILES=$ICD WGPU_BACKEND=vulkan \
  NOVA_PERF=1 NOVA_PERF_SCENARIO=asteroid_field NOVA_PERF_QUALITY=high \
  NOVA_PERF_LABEL=asteroid_field-high NOVA_PERF_OUT=./perf-results/sw \
  NOVA_PERF_WARMUP=20 NOVA_PERF_FRAMES=120 BEVY_ASSET_ROOT="$PWD" DISPLAY=:95 \
  cargo run --release --example 20_perf_baseline --features debug
```

Combat and web:

```bash
# Combat burst (native): NOVA_PERF_COMBAT=1 + a combat scenario, long warm-up so
# the bullet population and AI engagement saturate before the capture window.
NOVA_PERF=1 NOVA_PERF_COMBAT=1 NOVA_PERF_SCENARIO=broadside NOVA_PERF_QUALITY=high \
  NOVA_PERF_LABEL=broadside-combat-high NOVA_PERF_WARMUP=450 NOVA_PERF_FRAMES=300 \
  BEVY_ASSET_ROOT="$PWD" DISPLAY=:95 \
  cargo run --release --example 20_perf_baseline --features debug

# Web / WebGPU: builds the perf_web wasm, serves it, drives Chromium (WebGPU on
# the GPU under Xvfb), scrapes the console line. QUALITY= and COMBAT=1 supported.
scripts/perf-web.sh broadside            # QUALITY=high COMBAT=1 scripts/perf-web.sh broadside
```

`scripts/perf-baseline.sh gpu` / `sw` wrap the native sweep: both stand up a
throwaway Xvfb display (real GPU for `gpu`, forced lavapipe ICD for `sw`) and set
`BEVY_ASSET_ROOT`, reproducing this report's tables. Pass `DISPLAY_OVERRIDE=:0`
for the live desktop. Full env/URL param table is in `crates/nova_perf/src/lib.rs`.
Raw results are under `tasks/20260716-123551/perf-results/{xgpu,sw,combat,web}/`.

## Tooling added

| Tool | Where | Why |
|------|-------|-----|
| `nova_frametime` capture plugin | `crates/nova_perf/src/lib.rs` | env/URL-gated whole-frame capture over the real app; a `drive` hook for active-scene drivers; writes JSON + CSV (native) / console line (web); pure, unit-tested percentile stats |
| `combat_burst_driver` | `crates/nova_perf/src/lib.rs` | a `PerfDriver` that holds player fire and keeps combatants alive, so a capture measures a sustained combat burst |
| `20_perf_baseline` example | `examples/20_perf_baseline.rs` | boots any shipped scenario by id under the harness, with preset + `NOVA_PERF_COMBAT` knobs |
| `perf_web` bin + `perf.html` | `crates/nova_perf/src/bin/perf_web.rs`, `perf.html` | the same harness built to wasm by Trunk (config from the URL query), for the web/WebGPU capture |
| `perf-baseline.sh` / `perf-web.sh` | `scripts/` | native sweep (scene x preset x renderer) and the web capture (Trunk build -> serve -> headless Chromium -> console scrape) |

## What was tried and rejected as a measurement rig

- **`:0` real desktop for absolutes.** The compositor vsync-clamps the median to
  60 Hz and desktop/agent contention dominates the tail (run-to-run mean swung
  40 fps ↔ 76 fps for identical config). Kept only as the source of the ~5-9 ms
  real-present floor; not used for the tables.
- **wgpu GL / llvmpipe for the software floor.** Adapter creation panics on this
  box (bevy 0.19 wgpu GL path). Switched to a forced software **Vulkan** ICD
  (lavapipe), which works and is the faster-to-init software path anyway.
- **Headless Chromium (`--headless=new`) for WebGPU.** No `navigator.gpu` even
  with `--enable-unsafe-webgpu`. WebGPU needs the GPU process + Vulkan + a real
  (http/localhost) origin; the working recipe is Chromium **under Xvfb** with
  `--enable-features=Vulkan,WebGPU --use-angle=vulkan --ignore-gpu-blocklist`
  serving over `http://localhost`, which yields a real NVIDIA `BrowserWebGpu`
  adapter. `data:` URLs and headless mode both silently disable WebGPU here.
- **Trunk's bundled `wasm-opt`** rejects rustc's default bulk-memory ops; the
  perf build sets `data-wasm-opt="0"` (rustc `--release` already optimizes, so
  this only forgoes extra size shrinking - noted so the web mean is read as
  "unopt-wasm", a small pessimism vs the shipped, wasm-opt'd game).
