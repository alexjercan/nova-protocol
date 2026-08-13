# Replace distance-based integrity with authoritative link-point mates

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.10.0, ship, integrity, nova-os

## Goal

Replace the unit-cube `distance == 1.0` integrity rule with explicit link-point
mates. Link-point edges become the sole source of ship structural adjacency.

This is the engine and content migration gate for parts. Parts themselves and
editor snapping remain next-sprint work.

## Design

- Add `LinkPoint { id, position, normal }` to section authoring data.
- Add serde-defaulted `link_points: Vec<LinkPoint>` to `BaseSectionConfig`.
- Snapshot resolved points onto live sections as `SectionLinkPoints`.
- Transform positions and normals from section-local into ship-root space.
- Two points mate when:
  - their positions coincide within the authored epsilon; and
  - their normalized normals oppose within the angular tolerance.
- Link-point `id` identifies a socket for diagnostics and UI. It is not a
  compatibility class.
- Normalize mate edges, reject ambiguous one-to-many mates, and build symmetric
  `ConnectedTo` lists from the resulting edge set.
- Remove distance-based and AABB-derived structural adjacency. AABB remains for
  overlap lint, collider representation, schematic framing, and future broad
  phase searches.

## Scope

- Link-point schema, validation, live snapshot component, and prelude exports.
- Pure mate derivation and normalized graph construction.
- Replace `build_integrity_relations` center-distance adjacency.
- Seed all existing structural section prototypes with face-center link points.
- Fix base content, examples, and bundled mods that fail under strict graph
  validation. No compatibility fallback.
- Ship lint uses the same mate derivation and requires a connected graph for a
  multi-section ship.
- Lint errors:
  - duplicate or empty link-point ids within one section;
  - non-finite positions or normals;
  - zero normals;
  - ambiguous one-to-many mates;
  - disconnected multi-section ships.
- NOVA OS ship visualizer consumes `SectionLinkPoints` and gains a minimal,
  default-off `MATES` overlay that draws structural edges. Collider AABBs still
  size and frame sections.

## Out of scope

- Part assets and part-specific section prototypes.
- Editor link-point snapping and rotation UX.
- Replacing existing cube ships with parts ships.
- Connected-component severing after section destruction.
- Convex-hull colliders.

## Definition of done

- Exact parity test: every existing cube ship produces the same normalized edge
  set under old center-distance adjacency and new link-point mates.
- Unit tests pin coincidence epsilon, opposed-normal tolerance, transforms,
  ambiguity rejection, edge normalization, and graph connectivity.
- Runtime integration proves `ConnectedTo` is built only from mates.
- Existing base content, examples, and bundled mods lint and load after explicit
  link-point migration.
- NOVA OS `MATES` overlay renders the live structural graph without changing
  default ship-view presentation.
- No ship/scenario save-format change. Section schema changes only through the
  serde-defaulted catalog field.
