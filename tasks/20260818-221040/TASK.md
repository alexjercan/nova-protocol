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

## Done when

- The audit is written down with a per-item verdict, in this task.
- Asteroid seeding no longer happens on first hit in a shipped scenario.
- Load time measured before and after, and inside budget.
