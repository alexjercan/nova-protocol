# Add link-point snapping to the ship editor

- STATUS: IN_PROGRESS
- PRIORITY: 80
- TAGS: v0.11.0, ship, editor, content

## Goal

Let the editor assemble the semantic ship parts shipped in v0.10.0 by snapping
compatible authored link points. Link points remain the sole source of
structural adjacency.

## Completed prerequisites

- `20260812-130953` shipped the link-point schema, live components, mate
  derivation, integrity graph, lint foundation, and NOVA OS `MATES` overlay.
- `20260812-131842` shipped the Racer, CargoA, and CargoB semantic GLBs,
  prototypes, primitive colliders, tuned transforms, HP, and hand-authored
  gameplay link points. It also removed the cube compatibility path.
- Semantic parts stay hidden from the editor palette until this task makes
  their placement valid.

## Scope

- Extend `cut-obj-into-parts.py` and recipe manifests to emit generic
  link-point candidates from recipe conventions where practical. Recipe data
  can override generated candidates. Shipped gameplay sockets remain
  hand-authored in Rust.
- Replace the editor's fixed `normal * 1.0` placement assumption with
  link-point placement:
  - select a source point on the placement ghost;
  - find a compatible, unoccupied target point;
  - oppose normals and make positions coincident;
  - support quarter-turn roll around the mating normal;
  - reject occupied or ambiguous sockets.
- Use collider bounds only for picking, overlap checks, framing, and nearby
  socket candidate search. Bounds never create structural edges.
- Render the real part mesh in the placement ghost.
- Unhide semantic parts when the snapping path is available.
- Keep saves flat: prototype, position, and rotation. Re-derive mates at spawn
  and load.
- Reuse the NOVA OS `MATES` overlay to inspect the assembled graph.
- Integrate with the gallery picker from `20260812-131852`; this task owns
  placement behavior, not gallery layout or filtering.

## Out of scope

- New semantic ship assets or another shipped-content migration.
- Connected-component severing and debris bodies.
- Convex-hull colliders unless a selected part cannot use a primitive.
- Full-ship stamping, owned by `20260812-131901`.

## Definition of done

- Editor harness assembles a small connected ship through link-point snapping,
  including one rolled mate.
- Occupied and ambiguous sockets are rejected with visible feedback.
- Saved scenario reloads with the same derived mate graph.
- Runtime integrity sees the assembled ship as one connected structure.
- NOVA OS `MATES` shows the assembled graph.
- Recipe candidate generation and explicit overrides have focused tests.
- Content lint and focused editor/player-path probes pass.
