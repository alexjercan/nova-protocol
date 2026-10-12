# Stations: generator placement per civilization; dead stations

- STATUS: CLOSED
- PRIORITY: 86
- TAGS: v0.16.0, stations

## User facts
- Epic `20261012-095958`. Decision 2 (2026-10-12): the generator places at least one station per living civilization at the cluster nearest its centroid, more by size and advancement, with clearance; every extinct civilization leaves one dead station: lootable, strippable, no services, no admission. No authored base-world stations.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Civilizations on a 240 km jittered lattice, 320 km reach, `Living` 70% (`crates/nova_world_base/src/civilizations.rs:25-47`, `:83-88`); clusters on a 50 km lattice with a hull placed only beside a rock or planetoid of its own cluster (`crates/nova_world_base/src/clusters.rs:33-46`, `:93`). `SectorManifest { coord, asteroids, planets, ships }` is validated by `validate_manifest` (`crates/nova_world/src/generation.rs:295-304`).
- Landed: `SectorManifest.stations` (`SectorStation` carries the pinned `StationDesign` and a `StationConfig`); `nova_world_base::sector_stations` claims nodes per civilization (living `1 + floor(advancement * STATION_ADVANCEMENT_TIERS)`, extinct one dead), `clusters::station_pose` orbits a lone world at body radius + 500 m + clearance or stands at rest 500 m beside a rock of the same cell, a skipped station refuses the cell; dead = same design with `derelict: true` set in the planned `StationConfig`, consumed by `station_scenario_object`; `validate_manifest` refuses two stations of one stem and a design/spawn mismatch; base lint refuses a base bundle without every `GENERATED_STATION_DESIGN_IDS`; streaming spawns, holds (`PendingSectorStation`), freezes and thaws them like ships; proofs: `sector_stations::tests` seed sweep, `nova_world` refusal cases and station round trip, `system_generated_stations` live walk on seed 6.

- Orchestrator close 2026-10-12: commit 6fbd2b0c4 reviewed; content lint re-run by the orchestrator: 0 errors, 0 warnings. Planning in `crates/nova_world_base/src/sector_stations.rs` (count 1 + floor(advancement * 2), `TODO(20261012-100005)` single-design mapping), pose in `clusters.rs::station_pose`, streaming via `PendingSectorStation`, manifest validation for clearance and one per cluster, seed sweep over three seeds, live walk `system_generated_stations` (seed 6: reach, retire, thaw in place).

## Delivery gate
- Add `stations` to the manifest; placement is deterministic from the seed; the dead station's design is the civilization's design with services stripped, not a second catalogue. Lint and `validate_manifest` refuse a station outside clearance or two in one cluster.

## Verification
- A seed sweep (unit test) proves every living civilization has at least one station within reach and every extinct one has exactly one dead station; a probe walk finds the nearest station from spawn.

## Done when
- Stations appear in generated sectors, survive retire and thaw, and the sweep is a permanent test.
