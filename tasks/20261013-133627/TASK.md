# Stations: a severed station stops taking damage

- STATUS: CLOSED
- PRIORITY: 98
- TAGS: v0.16.0, stations, bug

## User facts
- Owner report 2026-10-13: "when shooting a station after it got severed it stopped taking damage".
- Stations take damage since the station rework (ad4e89637): damage stops at the design's `collapse_threshold` share of the hull pool (0.5 for every station), so a station never breaks up.

## Agent findings
- Unverified hypothesis: the sever removes sections from the pool, the pool reaches the 0.5 floor, and every later hit is clamped to zero. If so the floor works as written but reads as a bug in play. Other candidates: the kept half loses `StationMarker`/AI or the hit routing after a sever (`crates/nova_ship/src/sections/integrity.rs` sever path, `crates/nova_gameplay/src/integrity/health.rs` `on_damage`).

- Reproduced 2026-10-13 (uncommitted test `a_severed_station_still_takes_damage_on_both_halves`, `crates/nova_ship/src/sections/integrity.rs:1991`): after a sever the kept half refuses every hit (pool 200 of pinned max 500, floor 250); the severed wreck takes damage normally. Cause: the floor rule itself. The station pool max is pinned (`integrity.rs:1075-1076`), the sever drops the wreck's sections from the current pool (`:703`, `:1068`), the floor stays `max * 0.5` (`:1107`), and `on_damage` refuses hits below it (`crates/nova_gameplay/src/integrity/health.rs:166`). Same cause, read not run: a floored station with turrets standing can never reach the dead state.
- Options put to the owner: A floor re-measured after a sever; B weapons and flight computers ignore the floor; A+B (recommended); C no hit cap, keep only never-collapse (needs a zero-section rule). No changelog entry: introduced and fixed since the last release.

- Owner decision 2026-10-13: stations work exactly like ships. Delete the station floor and hit cap (`HealthFloor`, the station branch in `aggregate_ship_health`, the cap in `on_damage`). A station below its `collapse_threshold` collapses, peels and is destroyed like a ship ("a station can be blown to pieces which is fine"). The higher station threshold (0.5) stays on purpose so stations collapse sooner. A sever that leaves the kept hull below its threshold collapses it, for ships and stations alike; prove ships have no variant of this bug. The dead state on last weapon or flight computer stays.
- Owner decision 2026-10-13: a destroyed station drops the player's stored items (`StationStorage`) as canisters with its stock.

- Orchestrator close 2026-10-13: landed in 1b60d4724. `HealthFloor` and the hit cap are deleted; stations run the ship collapse rule at `STATION_COLLAPSE_THRESHOLD` 0.5; a destroyed station drops its stock and the player's storage as canisters. Proofs in `crates/nova_ship/src/sections/integrity.rs` physics_tests (ship, station and dead station severed above threshold take hits on both halves; severed below threshold collapse and are destroyed; ships had no variant) and `a_destroyed_station_drops_the_players_storage_with_its_stock`. system_station_orbit and system_station_round_trip pass under lavapipe; `content lint` re-run: 0 errors. Balance for owner: one Serpent torpedo now destroys the waystation (3490 to 61 hp), so the orbit range uses a scripted hit. Unreproduced gap: a severed dock plus despawned root may leave `DockedShip` for one FixedUpdate and trip `InventoryShips::pair()` (`crates/nova_interface/src/inventory/app.rs:1312`); ships share it.

## Delivery gate
- Reproduce first (asserted example or unit test that severs a station and then hits it), keep the before artifact, name the cause, then fix. If the cause is the floor, stop for the owner on the floor rule before changing it.

## Verification
- After a sever, hits on standing sections reduce their health until the defined end state (dead station) is reached.

## Done when
- The reproduced failure has a permanent proof and passes.
