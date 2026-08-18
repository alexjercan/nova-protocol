# Bake scenario work into the loading screen, stop computing on first hit

- STATUS: OPEN
- PRIORITY: 78
- TAGS: v0.11.0,performance,scenario

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
- Section mesh solidify (`mesh/solidify.rs`) - the ship roster is known at
  load; the fields are not.
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

## Done when

- The audit is written down with a per-item verdict, in this task.
- Asteroid seeding no longer happens on first hit in a shipped scenario.
- Load time measured before and after, and inside budget.
