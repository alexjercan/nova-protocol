# Scenario events and actions for jobs, credits, cargo and stance

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.18.0,scenario,modding

Planned 2026-11-11 from the v0.17.0 post-release review. Owner direction:
authored scenarios and the story must be able to drive jobs and the economy.

## Agent findings
- 46 scenario events (`crates/nova_scenario/src/events.rs:136+`): only
  `OnJobAccepted` touches jobs; no job completed, failed or handed-in event;
  no stance-changed, sector-enter or damage event (only Defeated, Destroyed,
  Neutralized).
- 56 actions (`crates/nova_scenario/src/actions/mod.rs:182+`): none give or
  take credits or items, change stance, post a job or spawn a wreck or
  encounter. `SetTradeProfile` (v0.17.0) is the only economy action.
- Queries cover Scenario, Entity, Elapsed and Speed only
  (`crates/nova_scenario/src/queries.rs:14-58`).
- The research note designed `OnSectorEnter`, `OnStandingChanged`,
  `AdjustStanding`, `GrantCredits`, `GiveCargo`, `OfferContract` and
  `SpawnEncounter` (`tasks/20260824-125938/RESEARCH.md:316-338`); none exist.
- Job completion handlers live in `crates/nova_world_base/src/jobs.rs:1080-1329`;
  boards in `post_station_boards` (`:233`); stance in the seeded graph
  (`tasks/20261012-100034`).
- Season one is OPEN at priority 0 (`tasks/20260908-161328`) and needs this
  vocabulary.

## Decisions (pending owner review)
- Events: `OnJobCompleted`, `OnJobFailed`, `OnJobHandedIn` with `id`;
  `OnStanceChanged` with `civilization` and `stance`; `OnSectorEntered` with
  the cell id. Payload names follow `OnJobAccepted`.
- Actions: `GrantCredits` (signed, refuses below zero and reports why),
  `GiveItems` / `TakeItems` by item id and count (refused when the hold cannot
  fit, fails loudly in the log and a `Refused` outcome), `SetStance`,
  `PostJob` (board id + job kind + target id, lint checks the ids),
  `SpawnWreck` (hull design id + drop list). No `SpawnEncounter` yet; that is
  the world-events direction.
- Queries: credits, item count, stance, current cell.
- Unknown item, civilization, board or hull id fails lint, then load.
  Unsupported action on a non-player target fails lint.
- Editor Events palette gets the new blocks in the same slice.

## Delivery
- Owners: `nova_scenario` events, actions, queries; `nova_events` macros;
  `nova_world_base` jobs and stance for the hooks; `nova_editor` palette;
  `nova_mod_format` lint. Dies: nothing. May break: scenario lint fixtures,
  editor palette tests, the webmod `the-ledger`, wiki scenario reference.
- Sequence: events -> queries -> actions -> lint -> editor palette -> one
  base scenario that uses them (a job-chain beat) -> docs and changelog.

## Verification
- Unit tests for each action's refusal rule and each event payload.
- `nova-probe` scenario: accept, complete and hand in a job; assert the three
  events in order and the credit delta from `GrantCredits`.
- Lint test: unknown ids fail.

## Done when
- A mod scenario can react to a job outcome, pay or charge the player, move
  cargo, change a stance and post a job without touching world code.
- Every new block appears in the editor Events palette and passes lint.
