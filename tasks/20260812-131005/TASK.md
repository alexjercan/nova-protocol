# Add ship parts and link-point editor snapping

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.11.0, ship, editor, content

## Goal

Add authorable ship parts that use the authoritative link-point graph, then let
the editor assemble them by snapping compatible sockets.

Depends on `20260812-130953`, which owns the link-point schema, live component,
mate derivation, integrity graph, lint foundation, and NOVA OS graph overlay.

## Scope

- Generate final part GLBs from the approved recipes in
  `scripts/part-recipes/` and ship them under `assets/base/gltf/parts/`.
- Add part section prototypes with primitive colliders, tuned origins, render
  transforms, HP values, and authored link points.
- Extend `cut-obj-into-parts.py` and recipe manifests to emit link-point
  candidates from pack conventions where practical. Authors can override the
  generated candidates in recipe data.
- Add editor part placement:
  - select a source link point on the placement ghost;
  - find a compatible target link point;
  - oppose normals and make positions coincident;
  - support quarter-turn roll around the mating normal;
  - reject occupied or ambiguous target sockets.
- Replace the editor's fixed `normal * 1.0` cube placement assumption.
- Use collider bounds only for picking, overlap checks, framing, and nearby
  socket candidate search. Bounds do not create structural edges.
- Render the real part mesh in the placement ghost.
- Add part categories and filters to the editor palette as needed for usable
  assembly.
- Keep saves flat: section prototype, position, and rotation. Mates are
  re-derived from link points at spawn and load.
- Reuse the NOVA OS link overlay to inspect assembled part graphs.

## Relationship to parts migration

This task adds the part catalog and assembly path. Task `20260812-131842` owns
the breaking content migration that replaces existing racer/cargo cube ships,
deletes cube assets and prototypes, fixes fallout, and refreshes screenshots.

## Out of scope

- Replacing all shipped cube ships.
- Deleting cube prototypes or assets.
- Connected-component severing and debris bodies.
- Convex-hull colliders unless a chosen part cannot use an acceptable primitive
  collider; escalate that case separately.

## Definition of done

- At least the approved racer seven-part set ships as authorable prototypes.
- Import/recipe output includes validated link-point candidates and round-trips.
- Editor harness assembles a small connected ship through link-point snapping,
  including one rotated mate.
- Saved scenario round-trips and reloads with the same derived mate graph.
- Runtime integrity sees the assembled ship as one connected structure.
- NOVA OS `MATES` overlay shows the assembled graph.
- Content lint and focused editor/player-path checks pass.
