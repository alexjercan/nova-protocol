# World first: addressable generated bodies

- STATUS: OPEN
- PRIORITY: 93
- TAGS: v0.16.0,world-first

## User facts
- Epic `20261012-095526`. A harness and a job must be able to name a generated body (a derelict, a hull, a rock) so handlers, objective markers and filters reach it.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Streamed entities get `ScenarioScopedMarker` and an `EntityId` but deliberately not `ScenarioAddressableMarker` (`crates/nova_world/src/streaming.rs:176-182`). Generated ships already carry syllable names (`crates/nova_world/src/generation.rs:135-165`) and frozen bodies carry `id` and `name` (`crates/nova_world/src/frozen.rs:343-356`).

## Delivery gate
- Propose the naming bridge: a stable generated id (sector coord plus manifest index) that a handler may reference by a typed id, and the lint rule that refuses an authored reference to a generated id (they exist only at runtime). Stop for the owner on the id shape before code.

## Verification
- A probe harness attaches an objective marker to a generated derelict, leaves the sector, returns, and the marker is on the same body after thaw.

## Done when
- Handlers and the jobs strand can address a generated body by typed id across retire and thaw.
