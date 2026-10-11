# Simplify generated-world data structures

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog, world, refactor, api

Technical cleanup only: make generated rock, planet and ship records easier to read and harder to mix up. This changes no world content or player-visible behavior. Defer until a feature touches these data structures.

## User facts

- Generated asteroid, planet, and ship records should follow one predictable shape instead of mixing flattened primitives with nested configs.
- Prepared asteroid and planet names should be parallel and easy to read.
- The design must not move main-thread asset ownership onto worker jobs merely to make structs look alike.

## Decisions

- Prefer explicit worker-safe config payloads over duplicating an owning scenario config or hiding spawn defaults in materialization.
- Keep asset-backed fields on the main thread unless a measured design requires a worker-safe asset snapshot.
- Preserve fail-loud count, pairing, geometry, ID, ownership, and clearance checks.
- Replace obsolete interfaces directly. Do not keep aliases for unshipped world-generation types.
- Naming (automode selected the recommended reversible option, not an owner choice): rename `PreparedRock` to `PreparedSectorAsteroid` and `PreparedPlanetoid` to `PreparedSectorPlanet`. Keep the `geometry` and `surface` fields and the planetoid config ownership.

## Agent findings

- PR #113 made each generated record its id, its placement and the whole scenario spawn config: `SectorAsteroid` carries `AsteroidConfig`, `SectorPlanet` carries `PlanetConfig`, `SectorShip` carries `SpaceshipConfig` plus generator facts. The generator, not materialization, states controller, side, loot and stock.
- Prepared payloads travel with their records (`PreparedSectorAsteroid`, `PreparedSectorPlanet`); nothing pairs by vector position.
- `validate_manifest` refuses every runtime asset handle in a record and an asteroid lock signature that `AsteroidConfig::validate_lock_signature` refuses; the scenario lint applies the same lock-signature rule. The `generation` module doc states the record and ownership rule.
- The `generation` module doc states the naming rule: the pair for record `Sector<Name>` is `PreparedSector<Name>` with a `body` field and an owner-specific payload field (`geometry` for an asteroid, `surface` for a planet). See `report.md`.

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
