# Define streamed-world mutation and save contract

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,world,design

## User facts
- The open-world and station/inventory spikes must decide persistence together (`20260824-125938`, `20260824-125943`); do not assume a world save runtime exists.
- Free play should support meaningful mining, salvage, station state and physical ship upgrades, rather than silently restoring consumed things after sector reload.

## Agent findings
- Current `OpenWorldSession` stores a seed; retired cells regenerate pristine content (`crates/nova_world_base/src/lib.rs:18-22,91`). Streamed entities are scenario-scoped but not authored-addressable (`crates/nova_world/src/streaming.rs:114-122`). No player inventory, ship-mutation save or sector tombstone system exists.
- Historical `WorldState`, `ShipSource::Persistent`, `SectionModification` and scenario-per-sector sketches in `tasks/20260824-125938/RESEARCH.md` are not implemented and do not decide this format. Platform key-value settings storage is not proof of save-slot capacity/atomicity.

## Proposed design work, not implementation approval
- Decide stable identities and ownership for harvested/carved rocks, derelicts, moving bodies, repaired/refitted ships, and stations across retirement, Retry, app restart, build changes and mod changes. Compare seed-only session, session-only sparse deltas, and durable sparse save; one consequence for each.
- Define failure policy for incomplete writes, invalid content IDs, changed eligible designs, missing mods and version mismatch. Decide save points and web storage constraints. One shared decision record must appear in both parent tasks.
- Name the minimum delivery slice that demonstrates claim-once and changed-ship return without a full ECS dump; do not add a schema before owner review.

## Verification to design
- Same seed and visit order variations reproduce pristine cells. A claimed body cannot reward twice after retire/revisit and restart if the chosen tier promises it. Distinct changes survive or refuse per the selected format. An interrupted write cannot silently replace a prior valid save. Focused pure validation plus deterministic ECS and probe flows; no exit-zero-only proof.

## Done when
- One owner-reviewed world/ship/station lifetime and save policy, with error rules, content mismatch, platform bounds and cross-task sign-off, is ready for an implementation gate; unapproved alternatives remain explicit.
