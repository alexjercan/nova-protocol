# Mesh the rock's surface, not its volume

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.11.0,performance,asteroid,spike

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
