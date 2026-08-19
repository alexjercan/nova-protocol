# Mesh the rock's surface, not its volume

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: archive,wontdo

Epic: `20260818-220812`. Owner's own observation: the asteroid does a `count^3`
split "but we only need the surface, not the entire volume".

## The claim

`SignedField` scans every cell of a `count^3` grid to seed, to measure
`solid_volume`, and to mesh. Only cells NEAR the zero crossing can ever change
sign or emit a triangle. At 64^3 that is 262,144 cells against a surface shell
of a few thousand - roughly a square-cube ratio, so the win is large and it
grows with the cap.

`pristine_field` already does a near/far shortcut ("the shell is about a third
of the grid"), which proves the surface is cheap to bound. This task takes that
from a shortcut to the representation.

## Approach

Narrow-band / sparse field: store and walk only cells within a band of the
surface, expanding the band where a carve moves the crossing. Standard
technique; the constraint here is that carving only ever REMOVES material, so
the band can only move inward and never has to be rebuilt from nothing.

Cuts the seed, the remesh AND the volume measurement, so it compounds with
`PERF-OFFLOAD` rather than competing with it.

## Do NOT

- Do not use this to raise resolution. Coarseness is the ART direction; a finer
  grid makes a smoother rock, not a better one. The win here is spending less
  for the SAME look, and the look must be verified unchanged.
- Do not land it before `PERF-REGRESSION` and `PERF-HARNESS`. This is the
  largest change in the epic and it needs a measurement harness pointed at it
  before it starts, or it is another correct-and-slow landing.

## Done when

- Identical silhouettes against the current output - `carve_asteroids` crater
  radii and the pristine-silhouette test both hold.
- Measured seed / remesh / volume cost at the shipped sizes, before and after.
- The `count^3` claim in the `FIELD_RESOLUTION_MAX` doc is rewritten to
  whatever is then true, because that doc is currently the best description of
  the cost model in the tree.

## CLOSED 2026-08-19 - measured, and it does not buy a frame

Owner: "it is an interesting idea, but sure, not sure how complex it would get
so let's say we can close it."

The measurement that settles it is `20260819-123928/NOTES.md`. The claim in this
task is still TRUE of the algorithm - `SignedField` really does walk `count^3`
cells - but the cost it was ranked on has moved out from under it:

- Seed and remesh run on `AsyncComputeTaskPool`, not in a frame.
- `FIELD_RESOLUTION_MAX` is 40, not 64, so the headline square-cube argument was
  written against 262,144 cells and the game ships 64,000.
- The whole carve path costs **0.12 ms/frame** under sustained fire.
- The one carve spike that IS a frame is the APPLY step,
  `collect_asteroid_remeshes` at 19.24 ms. A narrower band produces the same
  mesh and the same collider, so it would not shrink that by a byte.

This was also the largest change in the epic, against a case measured at 42 ms
worst - while `wfc_arena` 4v4 sits at 295 ms owned by rendering, the avian
solver and the projectile broad phase. Wrong end of the board.

## What was real and is NOT preserved by closing

A narrow band would still cut how LONG a remesh takes on the worker - which is
how long a shot rock wears its placeholder - and how much memory a field holds.
That is a latency and footprint argument, and nobody has complained about
either. If a rock is ever seen wearing its placeholder too long, this is the
technique, and the reasoning above is why it was not done for frame rate.
