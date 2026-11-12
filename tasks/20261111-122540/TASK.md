# Scenario events and actions for jobs, credits, cargo and stance

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.18.0, scenario, modding

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

## Decisions (owner said continue 2026-11-12; agent calls, stated)
Map with path:line evidence in `MAP.md`; its thirteen spec corrections
replace the planned vocabulary where the code cannot carry it.
- Payload access: one new expression factor `event.<field>`, legal in
  filters, so a handler can tell events apart by payload.
- Events: `OnJobDone` (work finished, located kinds only), `OnJobHandedIn`,
  `OnJobDropped` (`cause`: `Lost` or `Abandoned`). No "completed" or "failed":
  the code has neither state. Payload `id`, `kind`, `station`, `reward_cr`,
  `post_key` (the `PostJob` key, empty for a generated job; `key` is taken
  by the timer payload); `OnJobAccepted` gains
  the same fields. `OnStanceChanged` (`civilization`, `from`, `to`), raised
  at the four step sites and by `SetStance`; a provocation raises none.
  `OnSectorEntered` (`sector` slug, `x`, `y`, `z`, `from`), bridged in
  `nova_world`.
- Lint: a `Void` scenario that uses a job, stance or sector event, `PostJob`
  or `SetStance` is an error, because it can never fire there. Breaking for
  an outside mod that does so.
- Queries: entity credits, entity item count, current sector. No stance
  query: a civilization has no authored id; read `event.to`.
- Refusals: `error!` plus a new `OnActionRefused` (`action`, `id`, `reason`).
- Actions: `GrantCredits` (ship, signed amount, cause `Scenario`; refused
  below zero or past `u32`), `GiveItems` / `TakeItems` (ship, item, count;
  refused past the hold or the held count; any spawned ship), `SetStance`
  (a bound body with a market names its civilization; applied in
  `nova_world_base`), `PostJob` (bound generated station, `key`, item, count,
  reward; Procure only; a new `JobSourceType::Scenario`; a key already
  posted, held or completed at that station is refused). `WORLD_SAVE_VERSION`
  stays: the variant is additive for old saves.
- No `SpawnWreck`: `SpawnScenarioObject(Spaceship)` with `lootable: true`
  does it; the docs get the recipe.
- Editor: stock arms for each action, a query choice list.
- Proof: unit tests per refusal rule and payload; an asserted example on
  the located-jobs harness for the job chain (`probe scenario` proves a
  boot, not a goal); lint tests for unknown ids and the `Void` rule.
- `open_world` gains its handlers at the end only (saves index spent
  handlers).

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

## Delivered (2026-11-12)
| Commit | Change |
| --- | --- |
| 1a96926e7 | Expression factor `event.<field>` (RON `Event("field")`) |
| 71b058987 | `OnJobDone`, `OnJobHandedIn`, `OnJobDropped`; job payload on `OnJobAccepted` |
| fc145bbf1 | `OnStanceChanged`, `OnSectorEntered` |
| 9dbbd8333 | Job payload field `post_key` |
| ecca3fb41 | Queries `scenario.sector`, `entity("id").credits`, `entity("id").items("Item")` |
| ed920c703 | `GrantCredits`, `OnActionRefused` |
| eac20ba4a | `GiveItems`, `TakeItems`; lootable-wreck recipe in the docs |
| d44de960e | `SetStance((body: "$id", stance: Enemy))` |
| 851fa246a | `PostJob((station: "$id", key: "ore_run", item: "IronOre", count: 4, reward_cr: 250))` |
| 8cc3e5795 | `open_world`: three appended handlers; first hand-in grants 50 cr |
| f8b6bd8b3 | `examples/systems/system_scenario_jobs.rs` |
| 55b42325a | Scenario docs, creator reference, changelog |
| 9d4281011, 59e2d7361 | Out of lane: `system_ship_audio` compiles again; ammo economics header names the bonus |

Refusal reasons on `OnActionRefused`: `unknown_ship`, `insufficient_credits`,
`credit_overflow`, `hold_full`, `short_items`, `unknown_body`,
`no_civilization`, `unknown_station`, `no_job_board`, `duplicate_job`.

### Evidence
- `system_scenario_jobs` (lavapipe, two runs, same result): beat 1 job chain
  in order, balance 1800 -> 2197 cr (reward 322 + base 50 + fixture 25);
  beat 2 `PostJob` key `walk` posted, accepted, handed in, second post
  refused `duplicate_job`; beat 3 `TakeItems` of 1000 Rations refused
  `short_items`, hold unchanged; beat 4 stance Ally -> Enemy -> Ally.
- Reruns with the base 50 cr grant: `system_job_recover`,
  `system_station_jobs`, `system_job_distress`, `system_job_neutralize`
  (lavapipe), `system_job_escort` and `system_job_ammo_economics` (hardware
  GPU). All print their PASS lines and `cycle complete, no panic`.
- Unit tests per refusal rule, payload and lint rule pass in `nova_scenario`,
  `nova_gameplay`, `nova_world_base`, `nova_interface`, `nova_editor`.
  `content lint`: 0 errors, 0 warnings, 17 scenarios.
- Editor: the palette is built from `EventConfig::ALL` and
  `ActionChoice::ALL` (`crates/nova_editor/src/event/blocks.rs:170`);
  `the_palette_covers_every_kind_in_the_tables` passes.

### Deviations
- `GiveItems` and `TakeItems` raise no hold event; a scenario reads the
  change with the items query.
- `SetStance` clears offences, merits, provocation and the outlaw count. A
  write of the seeded stance drops the record.
- `duplicate_job` also covers a key the player holds, handed in or dropped.
- The station binding under `OnDocked` is `$id`.
- A v0.17.0 save loads unchanged; `JobPostings.scenario` is omitted when
  empty.

### Limits
- No editor frame was captured with the new blocks; palette cover is by
  test only.
- No hand-played run. The base handlers were seen only through the walks.
- `system_job_escort` fails under lavapipe (the hauler dies at 1.6 fps); it
  passes on the hardware GPU. Not caused by this task.
- `system_world_resume` panics at line 1190: the walk picks `bounty_hunter`,
  which lost its torpedo bay in `cb15b60a4` (v0.16.0). Not fixed.
- `system_ship_audio` needs `ALSA_CONFIG_PATH=/dev/null` on this host; with
  the empty value the audio device stays and the walk fails its last claim.
- `system_job_escort` and `system_job_ammo_economics` ran on the NVIDIA ICD,
  a deviation from the lavapipe rule.
