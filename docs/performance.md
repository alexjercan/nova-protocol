# Measuring performance

How a frame's cost is measured here, and the ways a measurement lies. The
harness that produces the numbers is the same `probe` front door
[Building and running](development.md) uses for correctness; this page is the
INSTRUMENT half - what the capture does, what a number is worth, and what has
to be true before one can be quoted.

Read [Run verification (probe)](development.md#run-verification-probe) first
for the run mechanics (the verbs, the run directory, the report). Everything
below assumes a run has happened.

## The capture

An env-gated capture plugin drives the real gameplay app to `Playing`, warms
up, records the wall-clock delta of every frame for a fixed window, and writes
percentile stats. It is inert unless `NOVA_PROBE` is set, so the whole fleet
carries it permanently. Probe runs it as a DEDICATED capture-only pass when the
program declares it (the correctness recorder flushes per entry on the frame
path - measurement and correctness never share a pass), the harness completion
protocol keeps the app alive until the window closes, and enrolled scenes (a
script `loop_from` point) reload + replay so the window measures activity -
reload intervals are excluded from the stats and reported as their own line.

Which runs get that pass is the PROGRAM's own answer, read back off its
contract. `NovaProbePlugin::default()` wires the capture, so a cataloged
example makes a frame-cost claim unless it says otherwise with
`without_frametime()` - which is what every `screenshots/` producer and the
`playable/` galleries do, since a posed still has no frame cost worth
defending. A program that wired no capture is inert, and its contract tells the
report the frame-time section is empty because the program makes no frame-cost
claim - not because a capture went missing.

What the report does with the numbers is REPORT them. The Performance section
leads with the worst frame, the mean, and what each comes to in FPS, flagged
when it is under 60; `checks.json` mirrors it under `frames`, carrying
`graded: false`. Nothing passes or fails on a frame-time number - whether a
scene is fast enough on this machine, in this build profile, is the reviewer's
call and always was.

## One capture cannot prove a tail moved

The worst frame is the number that matters - a stutter is a tail, and a mean
hides it - and it is also the least repeatable thing the capture produces. Two
captures of an UNCHANGED scene move it by tens of percent, while the mean and
the median of the same two windows barely move at all. So a claim about the
worst frame is made over a repeat SET, not over a run:

```sh
cargo run --features debug probe run wfc_arena --repeat 5
```

Each repeat is its own process, and each writes its own `frametime.csv` row
labelled `<subject>#<n>`. The report then reads them as a set:

- the reference is the MEDIAN of the repeats' means (and of their medians), so
  one bad capture cannot drag the band over itself;
- a repeat whose mean or median sits outside the band is DISCARDED - it met a
  different machine, or a different amount of scene;
- the tail is read only across what survives, as the median of the admitted
  **p99** values, printed with the spread of that group. The slowest single
  frame gets the same treatment and is printed beside it - as a reading, not
  as the number a claim is made on. It is one sample out of nine hundred and
  behaves like one; p99 is still a tail (the ninth-worst frame) and resolves
  roughly twice as small a change.

The spread is the point. It is the honest width of the number, and a claimed
improvement smaller than it has not been measured. Discarding is not grading:
`checks.json` carries the whole set under `repeats` with `graded: false`, and
a discarded repeat says something about the machine, never about the code.

Two things the band cannot do for you. It is derived from a REFERENCE HOST and
is a property of that machine, so re-derive it elsewhere. And it catches an
outlier, not a DRIFT: a set taken immediately after a build slides monotonically
down as the box recovers, the reference lands in the middle of the slide, and
the gate throws out both ends. Let the machine settle before a repeat set, and
treat a set the gate empties as "measure it again", never as a result.

### A refresh cap is a set's finding, not a window's

A run that named a presentation mode promising NOT to block on refresh can
still be paced by the display: the surface falls back, or a compositor holds
the swap chain, and every frame then waits for the same clock edge. What the
capture reports is the display's period, at a perfectly plausible-looking
number.

Each capture measures its own CLUSTER SHAPE - the frame time the window
collapsed onto, and the share of frames within 5% of it - and writes both
beside its stats (`cluster_ms` / `cluster_share` in `frametime.csv`, on the
summary line, and in the per-run JSON). A window that clusters at 0.60 or more
on a period of at least 4 ms logs a `SUSPECT reason=refresh_capped` line and
keeps its stats. The 4 ms floor is a display fact - nothing refreshes above
250 Hz - so a scene faster than that is cheap and steady, not capped.

Under a mode that MAY block on refresh (`fifo`, `autovsync`) the columns stay
empty: clustering there is the mode working, so it is not evidence of anything
and the honest record is that nothing was measured.

**A suspicion, because one window cannot settle it.** The check fires on
STEADINESS, and an optimisation makes frames steadier - so a threshold applied
per window preferentially accuses the FASTER arm of an A/B, which is a bias
pointed at the null and the worst direction for an instrument to be wrong in.

The repeat set is where the evidence exists, because a refresh period is a
CONSTANT. When two or more captures of a set are suspects and their cluster
medians agree to within 1%, the set measured the display: every capture is
discarded and the set reports no tail, the same shape as any other set the gate
empties. When the medians DISAGREE, no display did that, and the captures are
gated on their statistics like any others. `checks.json` carries the call under
`repeats.sets[].refresh_cap` (`refresh_capped`, `workload`, `unverifiable`,
`not_suspected` or `unmeasured`) with the per-capture shapes under it.

Agreement is a far tighter test than cluster membership, and the two numbers are
not interchangeable. The 5% band spans the SCATTER inside one window, which is
wide - a capped window is not a flat line. The 1% agreement spans the drift of a
period ACROSS windows, which for a crystal-derived clock is none: the 165 Hz
captures this was built from agree to under a tenth of a percent. Held at 5% the
discriminator would have come within 2.5 points of convicting the steady
workload it exists to acquit.

One lone suspect in a set reads `unverifiable`: there is no sibling to check
its period against, so whether it measured the display is UNMEASURED, and the
set is neither refused nor waved through. Repeat it to settle it.

## Was the window one scene?

A capture also records how many FIXED STEPS ran inside each frame, bucketed by
count in the per-run JSON (`fixed_steps`), on the summary line, and as a table
in the report's Performance section (`checks.json` mirrors it beside `frames`).
Bevy runs `RunFixedMainLoop` until the accumulated virtual time is spent,
capped by `Time<Virtual>::max_delta`, so a frame that overruns the timestep
hands its overrun to the next frame as extra steps.

Two readings matter, and no percentile shows either:

- **Frames that ran NO step.** In a scene slower than the timestep that means
  the simulation was STOPPED inside the window - a pause, a menu, a result
  screen. A window carrying them did not measure one scene, whatever its mean
  says. In a scene faster than the timestep it is ordinary and says nothing.
- **Frames at the top of the range.** When that count is
  `max_delta / timestep` the clamp is firing: those frames are discarding real
  time the world never simulates.

A stopped simulation is not merely reported, it is REFUSED. The capture reads
`Time<Virtual>` directly, and a frame that arrives paused (or at relative speed
zero) inside the warm-up or the window aborts the whole capture: it logs at
ERROR naming the phase and the frame, writes NO `frametime.csv` row and no
per-run JSON, and the `capture_simulated` check fails the run and lists every
refused capture. A refusal rather than a flag, because a stopped scene keeps
drawing at a steady cost - the mean and median it produces are exactly the shape
a validity gate admits.

So a scene that can REACH AN END needs a window that closes before it does. An
example declares its own with `NovaProbePlugin::frametime_window(warmup,
frames)`, sized from a measured run of that scene rather than guessed;
`wfc_arena` does, because its 4v4 is a match that can be won.

A bounded window is still not enough on its own, because **a scene can end
before its clock stops.** A fight whose losing side is gone is over while
`Time<Virtual>` still ticks and every environment gate still passes; what the
window measures then is the aftermath, at a fraction of the cost, in a row that
looks like any other. `wfc_arena` meets this by construction - its capture opens
on both teams having fired and connected, and a WIPE is what credits the last of
that damage, so the gate can open onto an empty arena.

`NovaProbePlugin::live_frametime(<predicate>)` is the second half. The predicate
is re-evaluated every warm-up and capture frame, and the first frame it fails
refuses the window with reason `scene_ended` - same ERROR line, same discarded
stats, same failing check as a stopped simulation. `wfc_arena` names "both teams
still have a ship flying". It says nothing about how MANY are left on purpose: a
four-on-one is a fight in progress, and refusing that would be a judgement about
workload rather than about the scene existing.

`NOVA_PROBE_MAX_DELTA=<secs>` forces the ceiling for a run, which is how a claim
about the fixed loop gets tested instead of argued. Capping it in a SHIPPING
build would trade a bounded tail for simulation time the world never runs, so
it stays a measurement knob.

**Pin it to one step before comparing two arms.** The loop AMPLIFIES whatever
it is handed. Write `B` for the per-frame base cost and `s` for the cost of one
fixed step; a frame of measured cost `F` runs `F / T` steps at Bevy's default
`Time<Fixed>` period `T`, so

```text
F = B + s * F / T      ->      F = B / (1 - s / T)
```

A frame is not slow because the fixed loop ran; the loop ran because the frame
was slow, and then charged it again. As `s` approaches `T` the denominator goes
to zero and the reading runs away, so two arms that differ by a little in `B`
can differ by a lot in `F` - and the difference is the amplifier, not the
change. Pinned to a single step per frame a capture reads `B` directly, and
arms become comparable.

## A capture under Xvfb measures the X server too

**Read this before quoting any absolute millisecond a probe run produced.** A
software X server has no scanout, so presenting a window is a CPU-side copy of
every pixel of it, and the render thread pays for that inside `render_system`
after the graph has already finished. On this project's host, at 1280x720, an
EMPTY scene costs 16.7 ms under `xvfb-run` and 3.0 ms against a real display -
same binary, same window, same pin, same `Immediate` presentation. The gap is
linear in window pixels (1.4 ms at 160x90, 11.5 ms at 720p, 50 ms at 1440p) and
does not move when `NOVA_PROBE_RENDER_SCALE` cuts the shading to a sixteenth, so
it is the window and not the drawing.

It is an ADDITIVE constant, not a scale factor. An A/B whose two arms share a
window size divides it out, so ratios and ablations under Xvfb stand. A budget,
an FPS gate and a "this scene costs N ms" claim do not.

Two knobs make it visible. `NOVA_PROBE_PRESENT=immediate` names the presentation
mode instead of requesting `AutoNoVsync` - bevy logs the fallback for a named
mode and says nothing for the auto ones, so this is how a run proves it was not
capped at refresh. `NOVA_PROBE_RENDER_DIAG=1` asks the renderer for GPU timestamp
queries and turns on the frame-cost report's per-pass GPU table (it costs a
resolve pass and a readback, about 3% of the frame, so it is never a default).

So an absolute number wants a REAL display, and the window does not have to be
in your way to get one. An armed run wears the `WM_CLASS` / app id
`nova_core::MEASURE_WINDOW_CLASS` and no other run does, so a window manager
can send it elsewhere on its own - on i3,
`for_window [class="nova-measure"] move container to workspace 3`. The class is
deliberately distinct from the normal one, so a placement rule can never catch
a hand-run somebody is playing.

## Where a frame's milliseconds went

Any armed capture also logs a `nova framecost:` line and, under it, three
tables: every main-world schedule, every top-level `RenderSystems` phase with
the render graph carved out of `Render` so the submit and the present are
visible on their own, and every render pass the device timed. Read them
together - the main world and the render world overlap under pipelined
rendering, so a frame costs about the longer of the two, and GPU far under both
says the device is not the constraint.

Beside it, `nova census:` counts the world once per capture: entities by
component, the archetypes they fall into, and mesh instances against DISTINCT
mesh handles. Instances and distinct always side by side - 12,572 instances over
681 meshes is a different story from the 12,572 alone. Distinct handles are what
a draw call bins on, so the pair is what separates "the scene is big" from "the
scene batches badly" (see
[Why cracks are QUANTISED](sections.md#why-cracks-are-quantised)).

## The fixed loop is single-threaded on purpose

`AppBuilder::assemble` puts `FixedFirst` through `FixedLast` on Bevy's
single-threaded executor, so a schedule table's fixed-loop rows are self time,
not fan-out. Those schedules run 64 times a second and are made of many small
systems; the multithreaded executor's per-schedule task fan-out costs more than
the parallelism buys. Matched at 650-750 dynamic bodies in a 1v1
`wfc_arena` fight, the per-step median measured 7.9 ms multithreaded against
6.1 single-threaded, with the capture's 1% low 27 fps against 48;
`stress_point_defense` at ~2,040 bodies measured 3.17 ms against 2.84.

Avian's `PhysicsSchedule` and `SubstepSchedule` are LEFT multithreaded. The same
switch applied to them moved no step metric and made the frame tail worse (p99
36.9 ms against 40.6): the solver's `par_for_each` passes are the one part of a
fixed step that does saturate threads. Re-measure before moving either boundary.

## The window, and the deadline sized to it

The capture window is the capture crate's full 180/900 baseline unless the
example declared one of its own, so probe numbers stay comparable with the
sweep's; your `NOVA_PROBE_WARMUP` / `NOVA_PROBE_FRAMES` override both. The
completion deadline is SIZED to the BASELINE window (not a flat 120s, and a
ceiling for any shorter one an example declares): probe sets
`NOVA_AUTOPILOT_DEADLINE` for the fps pass to `(warmup + frames) / ~2fps +
margin`, so a slow-but-progressing capture (a heavy scene in a dev build under
software rendering - the `stress_*` ranges are the case) completes instead of
tripping the hang detector; a genuine hang still fails at a window-appropriate
bound, and your own `NOVA_AUTOPILOT_DEADLINE` overrides it. Every example's `main`
returns `AppExit`, so a deadline expiry is a non-zero process exit the
`process_exit` check reports. See the crate docs for the full knob list
(`NOVA_PROBE_*`).

## Sweeping presets, renderers and the web

The perf sweep is the same front door: a preset matrix of the frame-time
capture, one labeled `frametime.csv` row per cell, release-built (dev-profile
frame numbers are not baselines):

```sh
cargo run --features debug probe run stress_bullets --release --preset high --preset low
cargo run --features debug probe run stress_bullets --release --render sw ...  # lavapipe floor
cargo run --features debug probe run stress_bullets --release --norender      # no renderer at all
cargo run --features debug probe run <scenario> --platform web   # web/WebGPU capture (scraped)
```

`--render` picks the BACKEND for every pass, the frame-time one included: `sw`
forces the lavapipe ICD and its short 20/120 window. `--norender` decides
whether anything draws at all, sets `NOVA_NORENDER` in the child, starts no
Xvfb, and is refused alongside `--render` - there is no backend to pick when
nothing draws. It keeps the 180/900 baseline window, because a headless run has
no fill cost to shorten around, and it is native only: a wasm run has no process
environment to set. Its rows name themselves in `frametime.csv` - `backend` and
`adapter` both read `unknown`, there being no adapter. It measures the main
schedule alone, so it CANNOT see a render-side panic; a speed option beside a
rendered run, never instead of one.

To measure a named SHIPPED scenario, use the `probe scenario` verb rather than
`run --scenario`: it launches the game binary itself and needs no example.
`run --scenario` only sets `NOVA_PROBE_SCENARIO`, which no cataloged example
reads on the native side; on `--platform web` it is load-bearing, because
`nova_perf_web` takes the scenario id from the URL.

Every capture records run metadata (wgpu backend + GPU adapter, resolution,
graphics preset, git SHA, host and - schema v3 - the BUILD PROFILE) so a
results file names its own renderer (pre-v3 files, like the v0.7.0
baseline, still load; their profile reads `unknown`). The report badges
each row `dev` or `release`: dev numbers are NOT baselines, and because the
capture is wired by default, the badge is what keeps ad-hoc dev captures from
being mistaken for comparable measurements. The web platform
builds the perf_web wasm app through Trunk, serves it from an embedded static
server, drives headless Chromium with the calibrated WebGPU flags, and
scrapes the summary line into a labeled CSV row (no fs in the browser).
Compare runs with `probe report <after> --baseline <before>` - signed deltas
per label - and `report` only accepts dirs probe itself produced
(`probe-run.json` is the gate).

## Profiled pass (where does the time go)

Per-system costs come from a SEPARATE traced run - tracing overhead inflates
frame times, so a profiled run RANKS systems while the clean capture owns the
FPS truth (never mix the two):

```sh
cargo run --features debug probe run system_scenario_grammar          # trace + report table
cargo run --features debug probe run system_scenario_grammar --samply # + flamegraph
```

The profiled pass builds with `--features debug,trace` (bevy's per-system
spans are compiled in only under `bevy/trace`), runs headless with
`TRACE_CHROME` into the run dir (plus the `RUST_LOG=bevy_ecs=info` override
that un-hides the spans from the game's log filter), and the report renders
the top-N table (`probe report <run-dir>` re-renders it). Open the raw
`trace.json` in https://ui.perfetto.dev for the full picture; `samply load`
opens the flamegraph in the Firefox Profiler (the samply run is skipped with
a note when samply is missing or blocked - sampling needs
`perf_event_paranoid <= 1` AND, on many-core hosts, enough perf ring-buffer
memory: an "mmap failed" means raising `perf_event_mlock_kb`, e.g.
`echo 16384 | sudo tee /proc/sys/kernel/perf_event_mlock_kb`). The samply
run builds with the dedicated `profiling` cargo profile (full DWARF in the
binary + frame pointers via RUSTFLAGS) so our frames symbolicate to real
names instead of raw addresses; frames inside the NVIDIA driver blob and
stripped system libraries stay hex - that is their stripping, not a build
problem. Load the profile right after recording: symbolication resolves
from the binary on disk, so a rebuild in between loses names.

**Expect the trace to be enormous.** It carries one span per system per frame
and grows at roughly 28 MB per second of traced gameplay, with nothing capping
it, so a long range leaves GIGABYTES in its run dir - the report prints the size
beside the table. That is deliberate: the raw file is the Perfetto artifact, and
a byte cap or a span filter would buy disk by truncating the deep dive it exists
for. The host reads it as a STREAM instead, at flat memory (about 70 MB peak,
whatever the file's size), so the cost is disk and disk only. It is a scratch
artifact: keep it while you are profiling, delete the run dir when you are not.

## Find it in the code

- The capture, its window and its knobs: `FrameTimePlugin`, `nova_frametime` -
  `crates/nova_probe/src/capabilities/frametime.rs`. The crate rustdoc
  (`cargo doc --open -p nova_probe`) carries the full `NOVA_PROBE_*` table,
  native env var against wasm query string.
- Where the milliseconds went: `FrameCostPlugin` -
  `crates/nova_probe/src/capabilities/framecost.rs`; the GPU half is gated on
  `nova_core::RENDER_DIAG_ENV`.
- What an armed run looks like from outside: `nova_core::PROBE_ENV` and
  `nova_core::MEASURE_WINDOW_CLASS` - `crates/nova_core/src/lib.rs`.
  `nova_probe` re-exports `PROBE_ENV`; it lives down there because the window
  builder needs it.
- What the scene contained: `CensusPlugin` -
  `crates/nova_probe/src/capabilities/census.rs`.
- Driving a repeat set, the presets, the web platform, the traced and samply
  passes: `crates/nova_probe_cli/src/native/`. Reading one back - the validity
  band and the refresh-cap discriminator - `read_repeats` and `RefreshCap` in
  `crates/nova_probe_cli/src/evaluation/frames.rs`; the stats and CSV/JSON
  schema both halves speak - `crates/nova_probe/src/stats.rs`.
- The bundle an example wires: `NovaProbePlugin` -
  `crates/nova_probe/src/capabilities/mod.rs`.
