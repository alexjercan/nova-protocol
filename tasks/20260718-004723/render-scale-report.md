# Render-scale lever: implementation + measured web win (task 20260718-004723)

Sprint v0.7.0, task `20260718-004723` (p42), descends from the frame-time
baseline `20260716-123551`. This is the user-facing report; the implementation
/ reflection log is `docs/2026-07-18-render-scale-lever.md`.

## What shipped

## TL;DR

- Added a real `render_scale` lever on the Low preset (0.7 = ~49% of the pixels,
  upscaled to the window); Medium/High untouched. Correct and complete
  (screenshots below prove world + HUD render fine, just softer).
- **Measure-first verdict: on the only web/WebGPU rig available (RTX 3060 Ti),
  0.7 buys ~0%** - that GPU is overhead-bound over WebGPU, not fill-bound, so the
  upscale pass roughly cancels the fill saved. Isolated cleanly (same Low tier,
  1.0 vs 0.7 vs 0.5): asteroid p50 30.7 -> 29.8, broadside 17.9 -> 18.2.
- The lever targets the weaker fill-bound hardware the Low preset exists for
  (iGPUs, phones), which this strong rig cannot represent; the baseline said as
  much. Kept at 0.7 (user decision) as an honest, readable, cheap-when-off knob -
  NOT sold as a measured web win. A real win only starts to appear at an
  aggressive, visibly soft 0.5.

## What shipped

A `render_scale` fraction on `GraphicsBudget` (`crates/nova_gameplay/src/settings.rs`),
defaulted per tier: High/Medium `1.0`, Low `0.7`. Below `1.0` the 3D world
renders into an offscreen `Image` sized `render_scale * window` and a blit
`Camera2d` (which also hosts the crisp, clickable HUD) upscales it to the window
(`crates/nova_scenario/src/render_scale.rs`). Native and web both honor it; only
Low drops resolution, per the user's direction. (The first cut baked the HUD into
the reduced image too; that broke UI clicks and was fixed in task
20260718-132638 - the HUD now stays on the window.)

## Correctness: it renders a real frame, not a black one

`examples/21_render_scale_shot.rs` captures the primary window (the real upscaled
frame) at a chosen preset. `tasks/20260718-004723/shots/`:

- `asteroid_field-high.png` - crisp full-resolution reference.
- `asteroid_field-low.png` - the SAME scene (gravity well, player ship,
  asteroids, full HUD - fps, objective, radar prompt, target indicators),
  the WORLD visibly softer from the 0.7 render upscaled to 1280x720 while the HUD
  stays crisp (it renders on the window camera, not into the reduced image), all
  complete and correctly composited. Low is faster because it draws fewer pixels,
  not because anything is missing. (Post-fix shots comparing the crisp-HUD/soft-
  world result are in `tasks/20260718-132638/shots/`.)

## Method

Same harness as the baseline (`crates/nova_perf`, `scripts/perf-web.sh`), plus a
new `render_scale` override (`NOVA_PERF_RENDER_SCALE` / `?render_scale=`) that
forces `GraphicsBudget::render_scale` while holding the rest of the tier fixed.
That is the key isolation: comparing the SAME tier at `1.0` vs a fraction makes
the delta *pure resolution*, separating the render-scale lever from Low's
existing particle/scatter/juice cuts. All web runs: 1280x720, WebGPU on the
RTX 3060 Ti under Xvfb, 300 captured frames / 150 warm-up.

Read `p50`/`min` over `mean`: the shared dev host was intermittently loaded by
parallel jobs during the sweep, which inflates the `mean` and the tail (`p99`,
`p999`) on some runs; the median and the floor are the robust signals.

## Results - web / WebGPU (the target)

Raw rows: `tasks/20260718-004723/perf-results/web-clean/web-frametime.txt`.

### Isolation: same tier (Low), resolution only - the clean signal

This is the measurement that matters: Low held fixed (particles off, scatter
0.5, juice off), varying ONLY `render_scale`. Adjacent, both warm.

| Scene | render_scale | p50 (ms) | min (ms) | vs 1.0 (p50) |
|-------|-------------:|---------:|---------:|-------------:|
| asteroid_field | 1.0 | 30.7 | 16.4 | - |
| asteroid_field | 0.7 | 29.8 | 16.3 | **-3%** |
| asteroid_field | 0.5 | 22.2* | 14.3 | -28%* |
| broadside | 1.0 | 17.9 | 11.3 | - |
| broadside | 0.7 | 18.2 | 13.4 | **+2%** |
| broadside | 0.5 | 22.9 | 12.7 | **+28%** |

Note the scene split at the aggressive `0.5`: asteroid_field drops (it has the
big translucent gravity-well sphere + dense-field overdraw - a genuine fill
component), while broadside gets *worse* (its at-rest frame is thoroughly
overhead-bound, so a smaller target only adds the extra-pass cost). The lever's
win is scene-dependent and, on this GPU, small-to-negative - not the general
fill win the software-raster floor implied.

That the frame time *responds* to `render_scale` at all (rs0.5's median clearly
below rs1.0's, and the images differ) is itself the proof the lever **engages on
the WebGPU build** - if it were a no-op on web the three Low rows would be
identical. Paired with the native screenshots, "it takes effect on both native
and web" is verified.

**Render-scale 0.7 has ~0% measured effect on web** (asteroid -3%, broadside
+2% - both within run-to-run noise). The tell is the `min` (the GPU floor, least
overhead variance): it barely moves with resolution (asteroid 16.4 -> 16.3 -> 14.3
across 1.0/0.7/0.5). A fill-bound frame would drop roughly with pixel count; this
one does not, so **the browser/WebGPU frame on this GPU is overhead-bound**
(draw submission, JS<->wasm boundary, present, and the fixed CPU/ECS work), not
fill-bound. The extra upscale pass the lever adds roughly cancels the little fill
it saves at 0.7. Only at the aggressive `0.5` (25% of the pixels) does a real
median/floor drop start to appear (asteroid `*` - mean was contention-polluted,
but p50 22.2 and min 14.3 are a genuine step down), and that image is visibly
soft.

`shakedown_run` (the vertical slice) fits the same pattern: High p50 30.0 /
min 16.6, Low(0.7) p50 16.9 / min 11.2 - a big tier drop, but that is the
particle/scatter/juice + warm-up story, not render-scale (which the isolation
above shows is ~0). No separate rs=1.0 isolation was run for it; the two isolated
scenes settle the render-scale question.

### Combat (broadside) - not captured this round

The three broadside `combat=1` web runs executed but scraped no `nova perf` line
within the harness's 150s window (the web combat driver did not surface a
capture this batch; every at-rest run did). Not re-chased: the at-rest isolation
already settles render-scale's contribution, and combat piles on CPU-bound work
(≈500 live bullets + collisions + AI) that render-scale does not touch, so it
would not overturn the ~neutral finding. The baseline's combat-web number
(42 ms) stands as the reference for that scene. Flagged as the one gap.

### Tier headline: High vs Low

Noisy and not the clean signal: the first run after each fresh wasm build eats
shader-compile + HTTP asset upload even past the 150-frame warm-up (asteroid-High
landed p50 41.7 / mean 49.5 with a fat tail; broadside-High, run later and warm,
was p50 28.7). Read the isolation table above, not these, for render-scale's
contribution. The full Low tier (particles + scatter + juice + render-scale)
lands broadside at p50 18.2 vs a warm-ish High 28.7, but that delta is dominated
by the non-render-scale cuts (and warm-up/contention), not the resolution lever.

## Results - software-raster floor (native fill sanity check)

Clean run (`tasks/20260718-004723/perf-results/sw-clean/`), lavapipe, 150 frames:

| Scene | High mean | Low mean | delta | High min | Low min |
|-------|----------:|---------:|------:|---------:|--------:|
| asteroid_field | 135.0 | 123.1 | -9% | 98.7 | 91.6 |
| broadside | 119.0 | 105.6 | -11% | 96.9 | 73.0 |

Low here bundles particles-off + scatter-0.5 + render-scale-0.7; the baseline
measured the particle/scatter half alone at -7%/-14%, so render-scale adds little
on top even on this fill-heavy software rig - the full-window blit is itself real
fragment cost on a CPU rasterizer, and much of the frame is fixed CPU/ECS work.

## Decision

**Ship the lever on Low at `render_scale = 0.7`; do NOT claim a measured web win
on the available hardware.**

The measure-first gate came back honest-but-inconvenient: on the only web/WebGPU
rig available (a discrete RTX 3060 Ti), render-scale at 0.7 buys ~nothing,
because that GPU has ample fill headroom and the browser frame is overhead-bound.
That is exactly the case where render-scale helps least. The baseline already
flagged this: "this is the strong-GPU case; a real laptop iGPU WebGPU target will
be worse" and more fill-bound. The Low preset exists for that weaker hardware
(iGPUs, phones), which we cannot stand up here - so the lever is aimed at a
target the measurement rig cannot represent.

Given (a) the user's explicit directive that Low should decrease resolution,
(b) that the lever is correct (screenshots) and costs nothing when off
(Medium/High are untouched, verified), and (c) that a conservative 0.7 stays
readable, the value stays at **0.7** (user decision). The honest framing in the
CHANGELOG and code is "reduced internal resolution on Low" - a real, cheap knob
for weak hardware - not "a big measured web win", which the data does not
support.

What the data DOES support, kept for the next time a weak-GPU rig exists:

- The `render_scale` perf override (`NOVA_PERF_RENDER_SCALE` / `?render_scale=`)
  isolates the lever cleanly - re-run the same Low tier at 1.0 vs a fraction.
- If a fill-bound rig shows headroom, `0.5` is where a real win begins here
  (asteroid p50 -28%, floor -13%), at a visibly softer cost. Retune from that
  rig's numbers, not this one's.

### Not pursued (and why)

- **More aggressive default (0.5) now:** rejected - on the measured HW the extra
  softness buys a win only because 0.5 finally clears the lever's fixed
  extra-pass overhead; without a fill-bound rig to confirm the tradeoff is worth
  it broadly, 0.7 is the safer readable default (user decision).
- **Canvas scale-factor (no extra pass) for web:** a plausibly cheaper web-only
  mechanism (browser CSS-upscales a smaller backing buffer, no blit camera), but
  it does not work on native (OS owns the window's physical size) and the user
  asked for one behavior on both. Noted as a future option if web overhead ever
  becomes the thing to shave.
