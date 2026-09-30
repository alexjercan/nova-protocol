# Decide what survives leaving and restarting the open world

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, world, design

## Goal

Decide which player, ship, and world changes survive leaving a streamed sector, Retry, and restarting the game. Compare session-only state with durable saves before choosing a schema. This is research, not authorization to add save slots.

The open-world and item-loop work must share the decision. Player inventory exists on live ships, but the lifetime of changed streamed ships, mined rocks, encounters, station state, and future upgrades remains unsettled. M5 trade/mining is being delivered before saves; its session-only limits must be stated rather than implying durable stock, depletion or claim-once loot.

## Agent findings
- `OpenWorldSession` stores a seed; retired cells regenerate pristine content (`crates/nova_world_base/src/lib.rs:18-22,91`). Streamed entities are scenario-scoped but not authored-addressable (`crates/nova_world/src/streaming.rs`). Live ships carry `ShipInventory` (`crates/nova_gameplay/src/inventory.rs`), but there is no ship-mutation save or sector tombstone system.
- Historical `WorldState`, `ShipSource::Persistent`, `SectionModification` and scenario-per-sector sketches in `tasks/20260824-125938/RESEARCH.md` are not implemented and do not decide this format. Platform key-value settings storage is not proof of save-slot capacity/atomicity.

## Proposed design work, not implementation approval
- First decide which changes must survive sector unload, Retry, and restart: harvested/carved rocks, looted derelicts, encounters, moving bodies, player/other ship inventories, repaired/refitted ships, and stations. Include each M5 transition: ore-source depletion, mined canister spawn/pickup/destruction, queued jettison cargo ownership, destroyed-ship remaining cargo, market inventory/prices, credits and completed buy/sell. Compare seed-only session, session-only sparse deltas, and durable sparse saves, with one consequence for each.
- Then decide stable identities and ownership for the selected tier across build changes and mod changes, before designing a format. Distinguish authored/streamed ore bodies and claimed yields, ship/root inventory, loose versus queued canisters, destroyed sources and station market state. Prevent both duplicate reward after retire/revisit and stock loss or duplication during unload/save; define what happens to a queued ejection if its intake is destroyed. Coordinate future mod-item IDs with `tasks/20260930-100831/TASK.md` and player refits with `tasks/20260930-100908/TASK.md`; neither task authorizes a save schema.
- Define failure policy for incomplete writes, invalid content IDs, changed eligible designs, missing mods and version mismatch. Decide save points and web storage constraints. One shared decision record must appear in both parent tasks.
- Name the minimum delivery slice that demonstrates claim-once and changed-ship return without a full ECS dump; do not add a schema before owner review.

## Verification to design
- Same seed and visit order variations reproduce pristine cells. A claimed ore body or cargo source cannot reward twice after retire/revisit and restart if the chosen tier promises it. Transfer, jettison queue, loose canister, mining, credits and market stock conserve their owned quantities across unload/save; destroyed stock is lost exactly once. Distinct changes survive or refuse per the selected format. An interrupted write cannot silently replace a prior valid save. Focused pure validation plus deterministic ECS and probe flows; no exit-zero-only proof.

## Done when
- One owner-reviewed world/ship/station lifetime and save policy, with error rules, content mismatch, platform bounds and cross-task sign-off, is ready for an implementation gate; unapproved alternatives remain explicit.
