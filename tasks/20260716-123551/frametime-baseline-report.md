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
  torpedoes/impacts are firing, so the particle toggle is idle. **The preset
  cannot be tuned from at-rest data alone** - it needs the combat-burst
  measurement (deferred, see Decisions).
- **The weak-hardware / web low end is fill-bound.** Software raster (lavapipe,
  the worst-case CPU floor) runs these scenes at **8-12 fps**, and there the
  ordering flips: `asteroid_field` is the *slowest* (126 ms) and `shakedown_run`
  the fastest (86 ms), tracking screen coverage/overdraw, not entity count.
- **Web/WebGPU numbers are deferred, not captured.** The harness compiles into
  the wasm build and logs its summary to the browser console, but capturing real
  WebGPU frame times needs a browser run (manual or headless-Chrome console
  scrape). Documented as the top follow-up. This is the one part of the task's
  ask that is explicitly not yet delivered.

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

A new env-gated capture plugin, `nova_debug::perf::nova_frametime`, drives the
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
- It does **not** yet drive combat (torpedoes in flight, turret fire, particle
  bursts on impact). Those need a scripted firing autopilot; the harness has the
  seam for it (the same input-hook mechanism as `AutopilotPlugin::input`) but the
  combat script is deferred (Decisions #2) so the burst cost is measured, not
  guessed.

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
nothing fires at rest. So at-rest data **cannot tune the preset fractions**.

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

## Web / WebGPU (deferred, with the mechanism in place)

Not captured. The same `nova_frametime` plugin compiles into the wasm/Trunk build
and, with no filesystem on web, logs its `nova perf: label=... mean=... p99=...`
summary line to the browser console (the JSON/CSV path is native-only). Capturing
real WebGPU numbers therefore needs a browser: either a manual `trunk serve` run
reading the console, or a headless-Chrome driver that loads the page with the
`NOVA_PERF_*` values baked in and scrapes the console line. That harness wiring
(passing env-equivalent config into the wasm build, and a Puppeteer/CDP scrape)
is the concrete follow-up. It is deferred here because (a) web verification on
this box is a known time sink (see `docs/LESSONS.md` on headless/iGPU runs) and
(b) the task's own rule is to document-and-defer rather than rush a noisy number.
The native results already say the interesting thing the web run needs to confirm:
the bottleneck is fill/overdraw on weak hardware and fixed CPU overhead on strong
hardware - both of which WebGPU-on-a-laptop will show more sharply than the 3060 Ti.

## Decisions

### 1. Frame-time capture harness (`20260716-123551`) - DONE

Shipped `crates/nova_debug/src/perf.rs` (the `nova_frametime` plugin, pure
unit-tested percentile stats), `examples/20_perf_baseline.rs` (boots any shipped
scenario by id, with a preset knob), and `scripts/perf-baseline.sh` (the sweep
driver). This is the reusable gate future perf work runs against. Numbers,
rigs and caveats are this report.

### 2. Combat-burst measurement - DEFER (flagged, not noise)

The at-rest baseline cannot see the cost the graphics preset exists to cut:
particle bursts from torpedoes/impacts and turret fire. Measuring it needs a
scripted firing autopilot (the `19_broadside` slice shows the shape; ~700 lines
of scenario driving). The harness already has the input-hook seam for it. This is
the **highest-value follow-up** and the prerequisite for tuning the
`GraphicsBudget` fractions - explicitly deferred, not dismissed.

### 3. No native at-rest optimization - DEFER (measured)

On discrete GPU no scene is near the frame budget at rest, and the cost is fixed
CPU/present overhead, not scene content or GPU fill. There is nothing here whose
optimization the numbers justify. Revisit only if the combat-burst run (Decision
#2) or the web run surfaces a real over-budget frame.

### 4. Graphics-preset fractions - HOLD (data insufficient by design)

The provisional `GraphicsBudget::for_quality` fractions stay as-is: at-rest data
cannot tune them (levers idle), and the place they *would* bite - fill on weak
hardware, particles during combat - is exactly what Decisions #2 and the web run
will measure. One structural hint already: on the fill-bound software floor the
preset's current levers (particles/scatter) help less than a **resolution /
render-scale** lever would, which weak-GPU/web builds may want more than particle
toggles. Noted for the preset owner; not acted on here.

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

`scripts/perf-baseline.sh gpu` / `scripts/perf-baseline.sh sw` wrap the sweep:
both stand up a throwaway Xvfb display (real GPU for `gpu`, forced lavapipe ICD
for `sw`) and set `BEVY_ASSET_ROOT`, so they reproduce this report's tables
directly. Pass `DISPLAY_OVERRIDE=:0` to run against the live desktop instead.
Per-run `<label>.json` and an aggregated `frametime.csv` land in the out dir. Full env table is documented in
`crates/nova_debug/src/perf.rs`. Raw results for this report are under
`tasks/20260716-123551/perf-results/{xgpu,sw}/`.

## Tooling added

| Tool | Where | Why |
|------|-------|-----|
| `nova_frametime` capture plugin | `crates/nova_debug/src/perf.rs` | env-gated whole-frame frame-time capture over the real app; writes JSON + CSV; pure, unit-tested percentile stats |
| `20_perf_baseline` example | `examples/20_perf_baseline.rs` | boots any shipped scenario by id under the harness, with a graphics-preset sweep knob |
| `perf-baseline.sh` sweep driver | `scripts/perf-baseline.sh` | builds once, runs scene x preset x renderer, aggregates the CSV |

## What was tried and rejected as a measurement rig

- **`:0` real desktop for absolutes.** The compositor vsync-clamps the median to
  60 Hz and desktop/agent contention dominates the tail (run-to-run mean swung
  40 fps ↔ 76 fps for identical config). Kept only as the source of the ~5-9 ms
  real-present floor; not used for the tables.
- **wgpu GL / llvmpipe for the software floor.** Adapter creation panics on this
  box (bevy 0.19 wgpu GL path). Switched to a forced software **Vulkan** ICD
  (lavapipe), which works and is the faster-to-init software path anyway.
