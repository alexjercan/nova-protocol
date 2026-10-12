# Stations: hold canisters and section parts on destruction

- STATUS: CLOSED
- PRIORITY: 82
- TAGS: v0.16.0, stations

## User facts
- Epic `20261012-095958`. Decision 7 (2026-10-12), first stated 2026-09-28: a destroyed ship drops its remaining hold as canisters; a destroyed section drops a share of its recipe parts in the same drop. Standalone task; no station needed.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Canisters exist (`crates/nova_scenario/src/mining.rs`; `FrozenBodyType::Canister`). `OnDestroyed` is the only outcome of the open world (`assets/base/scenarios/open_world.content.ron:171-185`). Section destruction and detached pieces live in `crates/nova_gameplay/src/integrity/`.

- Orchestrator close 2026-10-13: landed in ae8be981e. New `crates/nova_scenario/src/wreck_cargo.rs` drops a destroyed root's hold plus collapse-peeled section parts as one `MinedOreDrop`; a section shot off a living ship drops `BuildRecipe::strip_parts` (floor per part) alone. Canisters take ids at birth, so no save format change. Item lint now refuses any item over one canister (jettison `Overweight` refusal deleted). Proofs: `nova_scenario --lib wreck_cargo` (two tests), `nova_world --features serde --lib a_destroyed_ships_cargo` (retire, RON round trip, thaw). `content lint` re-run: 0 errors. Owner review: drop reuses the mining ejector (2 m/s, about one canister per 5.5 s), drop does not inherit ship velocity, no live run.

## Delivery gate
- On ship destruction spawn canisters for the remaining hold; on section destruction spawn the recipe share as canisters. The share is a named constant in the recipe (`strip_yield`), not a magic number. Canister ids come from the existing allocator so they freeze and save.

## Verification
- Asserted example: destroy a loaded hull, count canisters and their stacks against the hold; destroy one section, the parts share matches the recipe times the share.

## Done when
- Both drops are permanent proofs and the frozen ledger keeps the canisters across retire and thaw.
