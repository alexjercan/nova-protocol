# Mining: canisters drift to the open intake

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.19.0, mining

Planned 2026-11-16 from the owner review of the game-feel research doc
(claude.ai artifact Hq6GmMBXm2zAfVFwGnQg45).

## User facts
- "mining is a bit tedious and hard but maybe it's fine for now". Asks for
  ideas. Lower priority than survival, crafting and jobs.

## Decisions
- Owner 2026-11-16: mining itself is fun. The tedious part is picking up
  canisters. Canisters gravitate toward an open pickup point (cargo intake).
  This is the scope; the other ideas below stay candidates.

## Agent findings
- Beam: reach 100 m, pulse every 1.0 s, carve radius 1.5 cells; long beam
  160 m, 0.8 s (`assets/base/sections/base.content.ron:4223-4225,4360-4362`).
  Stats are provisional
  (`crates/nova_authoring/src/base_content/sections/mining_beam.rs:22`).
- Waits in the loop (`crates/nova_scenario/src/mining.rs:31-42,108`):
  the first pulse on a new rock only seeds the field and yields nothing; ore
  is held until the remesh lands; canisters drop one at a time and drift at
  0.2 u/s; the intake must come within 1 m (door opens at 40 m,
  `base.content.ron:3563`).
- Ore bids per 10 kg: Stone 3, Carbon 6, Ice 9, Iron 12.
- Owner-approved canister rules: 1000 kg cap, mixed-item canisters, partial
  pickup (`tasks/20261026-003724/TASK.md:10`).

## Candidate ideas (agent, not approved)
- The first pulse yields ore.
- A short tractor pull on canisters inside the intake door range.
- A scan pulse that shows rich veins, so the player picks a target.
- Ore worth more, or rich veins with rare ore.
- Mining feeds on-ship crafting (task 20261116-132140).

## Delivery
1. While the intake door is open, canisters in a defined range drift toward
   the intake. Range and pull speed are explicit content fields on the
   intake section, not hidden constants.
2. Keep the owner-approved canister rules (cap, mixed items, partial
   pickup).
3. Measure pickup time and cr per minute before and after on the
   `mine_and_sell` example.

## Verification
- ECS asserted example: a canister inside range reaches an open intake and
  is taken; with the door closed it does not move.

## Done when
- Canisters near an open intake come to it, and the measured pickup time
  per canister drops.
