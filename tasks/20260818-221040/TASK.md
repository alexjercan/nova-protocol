# Bake scenario work into the loading screen, stop computing on first hit

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.11.0, performance, scenario

Epic: `20260818-220812`. Owner: "if we can bake certain things while the
scenario is loading we do that instead of lazy computation (e.g I am pretty
sure asteroids are doing some lazy work, so no more of that)".

They are. A rock's carve field is seeded on FIRST HIT, mid-fight, at 12.7 ms.
The seed input is the rock's seed and radius - both known the moment the
scenario is authored. There is no reason for that cost to land in a frame the
player is flying.

## The audit

Sweep for work whose input is fully known at load but is deferred to first use.
Known and suspected:

- Asteroid field seeding (`asteroid_carve.rs`) - confirmed.
- ~~Section mesh solidify~~ - WRONG, struck 2026-08-19. `mesh/solidify.rs` does
  not exist and sections never carve.
- Collider construction from meshes generally: `convex_hull_from_mesh` and the
  trimesh QBVH builds.
- Anything that lazily fills a cache keyed on an asset that the scenario
  already declares.

Then classify each: bake at load, offload (`PERF-OFFLOAD`), or leave lazy with
a reason recorded. "Leave lazy" is a legitimate outcome when the input really
is not known until it happens - the point is that the choice is made, not
defaulted into.

## The constraint that makes this non-trivial

Baking moves cost into the loading screen, which is only a win if the loading
screen does not become the new complaint. Budget it: measure load time before
and after, and if a bake pushes load past what is tolerable, it belongs in
`PERF-OFFLOAD` or in `PERF-PRELOAD` instead.

A scenario that scatters 20 rocks pays 20 seeds at load whether or not the
player shoots any of them. Consider baking only what a scenario is likely to
need, or baking progressively behind the loading screen's own progress bar.

## RE-AIMED 2026-08-19 by the measured ranking

`20260819-123928/NOTES.md` changes both what this task is for and what gates it.

**The item it was written for is already off the main thread.** Asteroid seeding
no longer costs a frame - the whole carve path is 0.12 ms/frame under sustained
fire, on `AsyncComputeTaskPool`. Seeding still has LATENCY (how long a rock wears
its placeholder), which is worth baking away, but it is no longer the frame-rate
argument this task opened with. Three tasks in this epic have now been ranked
against costs that moved out from under them; this is the fourth, caught in time.

**The measurement found a better first item, and it is a textbook one.**
`PipelineCache::process_pipeline_queue_system` spikes to **68.09 ms mid-run** -
shader compilation for something the warm-up never touched. The input is fully
known at load: a scenario declares its materials. And
`synchronous_pipeline_compilation = true` is kept DELIBERATELY (it fixes an
exit-time SIGSEGV), so this is a chosen block on the main thread, mid-fight,
outside a loading screen. That is exactly what the epic's rule forbids.

Pre-warming pipelines during load is the fix, and it goes first.

## Owner decision - the load-time budget is NOT a gate

Owner, 2026-08-19: "it's fine if loading takes a lot of time, we can address that
when it becomes a problem, for now let's move expensive/lazy things to loading
screen."

This overrides the constraint section above. Bake first, and do not stop to
prove the loading screen stayed inside a budget. A load cost is a one-off the
player is already braced for; a mid-fight stutter is not.

One thing still owed, and it is cheap: **load time has never been measured at
all.** The frame table deliberately excludes it behind a 180-frame warm-up, and
the worst spans of every profiled run are load-time already - cubemap decode
~280 ms, `RenderPlugin` build ~230 ms, first `GpuImage` extract ~115 ms. Take a
single baseline before baking, so that when load DOES become a problem there is
a before number to compare against. That is a measurement, not a gate.

## RE-SCOPED 2026-08-21 by the perf check

`20260819-173219` phase B2, and the rendered explainer
`tasks/20260818-220812/perf-check-2026-08-21.html`. **Two of the three audit items
are already DONE by other tasks and are struck. One item remains, and it is not
yet measured in this tree.**

- ~~Asteroid field seeding~~ - STRUCK. It runs on `AsyncComputeTaskPool`, one job
  at a time per rock, and the rock keeps drawing the surface it had until the job
  lands (`asteroid_carve.rs` module doc). The whole carve path is 0.12 ms a frame
  under sustained fire. Baking it would buy LATENCY - how long a rock wears its
  placeholder - and cost the resident grid, 140 KB on an arena rock and 275 KB on
  the largest, times every rock a scenario scatters whether or not it is ever
  shot. The drop is deliberate, not an oversight.
- ~~Collider construction, and the scenario spawn burst generally~~ - STRUCK.
  `state_to_world` chunks the queue under `SPAWN_DRAIN_BUDGET` (3 ms, one command
  per check so an object stays atomic), the scenario SCRIPT is gated on
  `scenario_has_settled` so no handler sees a half-built world, and the loading
  panel is held up by that same gate with a 0.6 s floor and a 50 ms settled test.
  Landed by `20260816-122158` and `20260816-112353`, both CLOSED.
- **Pipeline pre-warm - the only live item, and it needs a number before it is
  ranked.** `synchronous_pipeline_compilation: true` is unchanged and deliberate
  (`nova_core/src/lib.rs`, task `20260805-111329`: an async compile task holding a
  device reference at teardown SIGSEGVs one run in five). So a first-draw shader
  compile IS a chosen main-thread block, which is what this task exists to remove.

  **But 68.09 ms is an old figure from a different tree and nothing in the current
  suite can see the spike.** D19 ruled it out headless for the trivial reason that
  there is no render sub-app there. The arena's capture opens 11.6 s in, past every
  first draw. Measuring it needs a RENDERED capture that spans first draw - which
  no current subject provides.

  Ranking a fix against 68.09 ms would be the sixth time this epic ranked against a
  cost that had already moved. **Measure first.**

Load time, the one cheap thing this task still owed: `wfc_arena` reaches
`Playing` about **1.1 s** after its first log line (boot assets 0.27 s, asset
loading to 0.72 s, scenario built and Playing at 1.12 s), consistent across eight
captures. That is an example building its own scene, not a shipped scenario load,
so it is a first data point rather than the baseline the task asked for.

## Done when

- ~~The audit is written down with a per-item verdict, in this task.~~ DONE,
  2026-08-22, below.
- ~~Asteroid seeding no longer happens on first hit in a shipped scenario.~~
  Satisfied DIFFERENTLY to how it is written, and the difference is recorded:
  seeding still happens on first hit, but on `AsyncComputeTaskPool` behind a
  placeholder, so it does not cost the frame this task was opened about. Baking
  it would buy latency at the price of a resident grid per scattered rock.
- ~~Load time measured before and after, and inside budget.~~ Measured; there is
  no "after" because nothing was baked. `wfc_arena` reaches `Playing` in ~1.1 s.
- **REMAINS**: the four Tier 1 items below. They are the same shape as this task
  - work whose input is known in advance, paid at first use - but they are
  CACHING and SCHEDULING, not baking, and the largest of them is a mid-mission
  spawn rather than anything to do with load.

## MEASURED 2026-08-22: the pipeline item is CLOSED, and the audit found four others

### Pipeline pre-warm: the spike is not real in this tree

`probe scenario broadside`, `NOVA_PROBE_WARMUP=0`, 636 frames, real GPU, all
eight checks PASS. `PipelineCache::process_pipeline_queue_system` over 630 calls:
total 235.71 ms, **p50 2.3 microseconds**, max 63.93 ms.

Where the cost sits, anchored to the trace clock (validated to 2.4 ms against a
log line nested in a known span; `Playing` at t=1.557 s, the panel's guaranteed
minimum dwell ends at t=2.157 s):

| | ms | covered |
|---|--:|---|
| under the boot screen | 98.5 | yes |
| under the scenario panel | 125.4 | yes |
| at or past the panel edge | 10.4 | upper bound |
| the other 621 calls | ~1.4 | - |

**95% is provably inside a loading screen, and there are ZERO calls over 1 ms
after t=2.79 s** - across ~14 s and 551 frames of gameplay. The 68.09 ms figure
this item was ranked on was never mid-run: the worst call here is 63.93 ms at
t=0.56 s, during boot, before a scenario exists.

Checked for the obvious false negative: **the run fought.** 64,692
`damage_cracks` spans, 55,852 `SectionCracksMaterial`, 41,904 torpedo, 61,741
hanabi, 6 `CarveSpew`. The three biggest first-draw candidates were exercised
and cost no compile.

**Pre-warming would move at most 10.4 ms, once per session. Not worth doing.**
`synchronous_pipeline_compilation: true` stays, and it now has a number behind
it rather than only the SIGSEGV argument. Arm B was not run: with arm A clean
for the mid-fight question the A/B answers nothing, at the price of a known
1-in-5 SIGSEGV.

One instrument note, because it is the third time this has bitten: the framecost
`Render/submit+present` row was the designed detector and it was **unusable
under Xvfb** - 8-9 ms sustained, ~50% of the frame, because that row carries
Xvfb's `present_frames`. Its 0.115 ms clean baseline was taken on a real display.
The trace fallback carried the measurement.

### What the audit found instead, and it is not baking

Four items in this task's scope that ARE real, ranked by what they buy:

1. **A mid-mission ship spawn is unbudgeted, and it is the largest confirmed
   violation of the epic's rule.** `state_to_world` chunks under
   `SPAWN_DRAIN_BUDGET`, but the budget is checked AFTER each command and one
   command is atomic - so a Spaceship spawn runs `spawn_ship_skin` whole, at a
   measured 20.77 ms, in a frame the player is flying. No panel covers it:
   `spawn_scenario_load_screen` raises only on `LoadScenario`, never on a script
   spawn. Shipped scenarios that do this mid-mission: `lifeline` 7 ships,
   `shakedown_run` 6 objects, `final_tally` 2, `broadside` 2, `menu_duel` 2. All
   `ScatterObjects` are `OnStart`-only, so no asteroid is involved.
2. **A blast mints a fresh `EffectAsset` per detonation** - `EffectAsset::new(32768, ..)`
   plus a full `ExprWriter` graph rebuild, every time
   (`torpedo_section/render.rs:447`). hanabi's `ShaderCache` is keyed on
   generated WGSL so nothing recompiles, but the 32k-particle buffer is
   re-allocated per blast and a salvo holds several at once. Build it once into
   a resource.
3. **A muzzle `EffectAsset` per barrel** (`turret_section/render.rs:401`), N
   byte-identical 32k assets for N barrels. Dedupe to one handle.
4. **NOVA OS ship and map scenes are re-minted every time the player opens the
   tab** (`nova_os_ui/src/ship/scene.rs`, `map/scene.rs`), torn down on close.
   Mid-flight, and `MAP_RING_RADII` is a `const`.

Struck as decided-lazy, with reasons: the carve trimesh and the severed-piece
hull (input genuinely does not exist until the shot, both already on a worker);
the velocity HUD's 32,768-triangle octahedron (fires at player spawn, i.e.
inside the load already); per-crate, per-beacon and per-rock materials (all
`OnStart`, so a dedupe is a batching win, not a stutter win); asteroid mesh and
hull construction (load-only in shipped content).

### Two claims this task carried that were stale

- `NOVA_PERF_WARMUP` / `NOVA_PERF_FRAMES` / `NOVA_PERF_MAX_DELTA` are not read
  by anything. `probe_param` builds `NOVA_PROBE_<NAME>`. Corrected in
  `20260819-173219/NOTES.md`.
- `sample_scenario_queries` is NOT ungated: it carries
  `run_if(scenario_reads_an_entity_query)` and no shipped scenario reads one.
