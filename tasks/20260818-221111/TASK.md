# Distant bodies stop paying full price

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.11.0,performance,render,spike

Epic: `20260818-220812`. Owner, with their own caveat: "making use of LoD
somehow for asteroids and things in general; the maps are small though so maybe
not that big; maybe just for asteroids that are far away."

Treat the caveat as the brief. This is the LOWEST priority item in the epic and
it should not start until `PERF-HARNESS` says distance-independent cost is
actually on the ranked list. Maps are small; the win may not exist.

## The candidates, if it does

- Carved asteroids carry a remeshed `Mesh3d` at up to 26,000 triangles and a
  trimesh collider to match. A rock 200 units away does not need either at full
  rate.
- The distinction worth drawing is not only triangles: a distant rock does not
  need its carve field REMESHED promptly. Deferring the remesh by distance is
  cheaper to build than a mesh LoD chain and may capture most of the win, since
  the remesh - not the draw - is what costs.
- Greebles and the derived skin, if the profile implicates them.

## Do NOT

- Do not build a general LoD system on speculation. Measure first, then take
  the narrowest thing that pays.
- Do not let LoD change the SILHOUETTE at the distance a player fights at.
  Carving is a gameplay-visible state; a rock that looks uncarved because it is
  far away is a bug, not a saving.

## Done when

Either a measured win on a `PERF-HARNESS` case, or a written finding that the
maps are too small for it to pay - which is a legitimate close, and cheaper
than the system.
