# Stations: generator placement per civilization; dead stations

- STATUS: OPEN
- PRIORITY: 86
- TAGS: v0.16.0,stations

## User facts
- Epic `20261012-095958`. Decision 2 (2026-10-12): the generator places at least one station per living civilization at the cluster nearest its centroid, more by size and advancement, with clearance; every extinct civilization leaves one dead station: lootable, strippable, no services, no admission. No authored base-world stations.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Civilizations on a 240 km jittered lattice, 320 km reach, `Living` 70% (`crates/nova_world_base/src/civilizations.rs:25-47`, `:83-88`); clusters on a 50 km lattice with a hull placed only beside a rock or planetoid of its own cluster (`crates/nova_world_base/src/clusters.rs:33-46`, `:93`). `SectorManifest { coord, asteroids, planets, ships }` is validated by `validate_manifest` (`crates/nova_world/src/generation.rs:295-304`).

## Delivery gate
- Add `stations` to the manifest; placement is deterministic from the seed; the dead station's design is the civilization's design with services stripped, not a second catalogue. Lint and `validate_manifest` refuse a station outside clearance or two in one cluster.

## Verification
- A seed sweep (unit test) proves every living civilization has at least one station within reach and every extinct one has exactly one dead station; a probe walk finds the nearest station from spawn.

## Done when
- Stations appear in generated sectors, survive retire and thaw, and the sweep is a permanent test.
