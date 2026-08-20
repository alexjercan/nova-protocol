# Distant bodies stop paying full price

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: archive,wontdo

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

## CLOSED wontdo, 2026-08-20 - the harness answered the gate question, and the answer is no

This task gated itself: do not start "until `PERF-HARNESS` says
distance-independent cost is actually on the ranked list". The ablation
(`tasks/20260819-173219/notes-ablation.md`) ran that list, and every axis LoD
attacks was measured NOT to bind.

| axis LoD would reduce | arm that tested it | result |
|---|---|---|
| pixels / fill | 160x90, 1/64 the pixels | 0.917 - **8.3%** of the frame |
| pixels, other direction | 2560x1440, 4x the pixels | 1.275 |
| mesh instances | `ABL_NOCLAD`, minus 11,660 entities | did not bind |
| colliders and broad phase | `ABL_FREEZE`, every body static | did not bind |

What DOES bind is **material count**, because a render bin is keyed on the
material. That is the axis LoD cannot touch: a hull 200 units away still carries
its ~35 section materials whether it is drawn at 26,000 triangles or 200. The
2x that this epic actually found (`6b3bfc87`) came from collapsing 2,046
materials to 288 - not from drawing less geometry.

**The one real win in the candidate list is already captured more cheaply.**
Dressing geometry - rocks, derelicts, the planetoid - is 86% of all vertices for
under 10% of instances, and removing it measures 0.90. That 10% is worth having,
but as a STATIC reduction in the phase 3 batch, not as a distance-varying system
with a silhouette-popping failure mode the task's own "Do NOT" section warns
about.

So this closes under its own "Done when" clause: *"a written finding that the
maps are too small for it to pay - which is a legitimate close, and cheaper than
the system."* The maps are small, exactly as the owner caveated when filing it.

**Reopen if**: a case appears where distance genuinely varies - a large open map,
or a fleet engagement where most hulls are far - AND material sharing has already
landed, so triangles are the binding axis rather than bins.
