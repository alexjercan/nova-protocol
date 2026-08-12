# Link-points on parts: catalog data, import seeding, editor snap, mate edges

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,ship,editor

Goal: implement link-points ("sockets") on parts so parts know how to attach
to each other - the authored counterpart to derived AABB-touch adjacency.

Context:
- Design: tasks/20260812-100246/SPIKE.md R2 D6.
- Owner direction 2026-08-12: parts should carry link-points; editor snapping
  builds ships from parts (blocks-collection style palette).

Scope:
- Catalog data: link_points: Vec<LinkPoint{id, position, normal}> on
  BaseSectionConfig, serde-defaulted (older content keeps loading).
- Import seeding: cut-obj-into-parts.py / blocks import emit link-point
  candidates from the pack conventions (half-unit bboxes -> face centers on
  the half grid; naming filters faces) into the manifest -> content builders.
- Editor snap: ghost part snaps to nearest target link-point, normals
  opposed, points coincident, quarter-turn roll; footprint-offset fallback
  where a part has no link-points.
- Mate edges: link-point mates become a ConnectedTo edge source, unioned
  with derived AABB-touch (union seam from the graph-adjacency task).
- Saves stay flat (positions + rotations); mates re-derived at spawn.

Depends: graph-adjacency task (the union seam). Editor palette work can
start independently.

DoD:
- Schema + seeding + snap + mate derivation, each with tests.
- Editor harness proof: assemble a small ship from parts via snapping in the
  editor example; saved scenario round-trips.
