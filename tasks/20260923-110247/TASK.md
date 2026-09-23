# Normalize generated world object and prepared payload schemas

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, world, refactor, api

## User facts

- Generated asteroid, planet, and ship records should follow one predictable shape instead of mixing flattened primitives with nested configs.
- Prepared asteroid and planet names should be parallel and easy to read.
- The design must not move main-thread asset ownership onto worker jobs merely to make structs look alike.

## Decisions

- Prefer explicit worker-safe config payloads over duplicating an owning scenario config or hiding spawn defaults in materialization.
- Keep asset-backed fields on the main thread unless a measured design requires a worker-safe asset snapshot.
- Preserve fail-loud count, pairing, geometry, ID, ownership, and clearance checks.
- Replace obsolete interfaces directly. Do not keep aliases for unshipped world-generation types.

## Agent findings

- `SectorAsteroid` stores radius, raw kind, and seed directly; `SectorPlanet` stores `PlanetConfig`; the ship record stores design and yaw while materialization supplies controller and allegiance.
- `AsteroidConfig` contains an asset-backed texture, while `PlanetConfig` is worker-complete. Blindly storing both full configs would cross the worker/main-thread boundary incorrectly.
- `PreparedAsteroidGeometry` contains only worker-built geometry. `PreparedPlanet` carries its config and visual. `PreparedSector` also relies on positional agreement between description and prepared vectors.

## Delivery

- Define coherent worker-safe asteroid, planet, and ship payloads for `SectorDescription`.
- Make controller, allegiance, invulnerability, mass, lock signature, and other generated spawn policy explicit where they currently come from hidden materialization defaults.
- Decide whether prepared records should pair each description with its prepared payload instead of relying on parallel vector order.
- Normalize prepared type and field names without concealing the asteroid/planet ownership difference.
- Update canonical manifests, materialization, public preludes, docs, examples, and proofs.
- Remove superseded flattened fields and hidden reconstruction paths.

## Verification

- A malformed description cannot bypass finite geometry, ownership, unique ID, clearance, or known-content checks.
- Prepared asteroid and planet payloads cannot be paired with the wrong generated record or silently lose a vector tail.
- Worker jobs do not access `World`, `Assets`, commands, or main-thread-only catalogs.
- Materialization contains no undocumented gameplay defaults.
- Canonical same-seed manifests and the streamed-sector lifecycle remain deterministic.

## Done when

- Asteroid, planet, and ship generated records follow one documented schema rule.
- Prepared names and fields follow one documented naming rule.
- Main-thread and worker ownership is explicit in types and docs.
- All old unshipped schema paths are deleted.
- Focused `nova_world`, scenario-object, and streaming proofs pass.
