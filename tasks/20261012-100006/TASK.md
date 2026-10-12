# Stations: hold canisters and section parts on destruction

- STATUS: OPEN
- PRIORITY: 82
- TAGS: v0.16.0,stations

## User facts
- Epic `20261012-095958`. Decision 7 (2026-10-12), first stated 2026-09-28: a destroyed ship drops its remaining hold as canisters; a destroyed section drops a share of its recipe parts in the same drop. Standalone task; no station needed.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Canisters exist (`crates/nova_scenario/src/mining.rs`; `FrozenBodyType::Canister`). `OnDestroyed` is the only outcome of the open world (`assets/base/scenarios/open_world.content.ron:171-185`). Section destruction and detached pieces live in `crates/nova_gameplay/src/integrity/`.

## Delivery gate
- On ship destruction spawn canisters for the remaining hold; on section destruction spawn the recipe share as canisters. The share is a named constant in the recipe (`strip_yield`), not a magic number. Canister ids come from the existing allocator so they freeze and save.

## Verification
- Asserted example: destroy a loaded hull, count canisters and their stacks against the hold; destroy one section, the parts share matches the recipe times the share.

## Done when
- Both drops are permanent proofs and the frozen ledger keeps the canisters across retire and thaw.
