# Start roles: even difficulty, fun start ships, better goals

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.19.0, roles, balance

Planned 2026-11-16 from the owner review of the game-feel research doc
(claude.ai artifact Hq6GmMBXm2zAfVFwGnQg45).

## User facts
- The journey of each start role is very different: some are very easy,
  some are too hard and feel meh.
- The start ships are not fun and need tuning.
- Role goals are "kind of meh".
- Ship looks are task 20261116-132144.

## Agent findings
- Roles (`assets/base/roles/base.content.ron:1-135`):

  | Role | Hull | Credits | Kit | Job mix | Goal |
  |---|---|---|---|---|---|
  | industrial | mining_tug | 2200 | 15 plates, 25 rations | Trade | 6 ore Procures |
  | scavenger | salvage_hauler | 1800 | 10 plates, 1000 PDC | Salvage | 6 Recoveries |
  | transport | light_freighter | 2000 | 50 rations, 50 ice | Trade | 4 distinct stations |
  | bounty_hunter | interceptor | 1600 | 10 plates, 1000 PDC | Combat | KillList 6 |
  | military | patrol_picket | 1000 | 10 plates, 1000 PDC, 2 torpedoes | Combat | Patrols 6 |
  | pirate | raider | 1800 | 10 plates, 1000 PDC | Combat | Outlaw 6 |

- Transport starts with 0 plates. No role carries RailSlug.
- Pirate: the Combat job mix posts Distress and Neutralize jobs, which the
  Outlaw goal does not count (`crates/nova_scenario/src/objects/start_role.rs:220-250`).
  Agent observation, unverified in play.
- Every goal opens with "own one station" (`crates/nova_world_base/src/goal.rs:4-8`),
  at 2x a 10k-40k worth. A met goal is a milestone, not an ending.
- v0.17 audit: "getting started is hard ... not yet very fun"
  (`tasks/20261025-214714/TASK.md:8,38`). Owner wants goals balanced across
  roles with one simple flavour goal each, and writes role descriptions
  (`tasks/20261012-100030/TASK.md:112-119`).
- Start hulls are hand-placed blocks in
  `crates/nova_authoring/src/base_content/ships/block.rs:136-491`,
  advancement-0 parts only.

## Delivery
1. Measure: a nova-bench run of the first 15 minutes per role. Record time to
   first payout, deaths, credits at 15 min, and where the run stalled.
2. Retune kits, credits, hull sections and job mix per role through the
   builders.
3. Rework role goals: one reachable first step for every role, one flavour
   goal each. Owner writes the descriptions.

## Verification
- Bench evidence table per role, before and after.

## Done when
- Every role reaches its first payout and first goal step in a comparable
  time on the bench, and the owner signs off the goals.
