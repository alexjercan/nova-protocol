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

## Decisions (owner said implement 2026-11-19; agent calls, stated; map `MAP.md`, numbers `CENSUS-BEFORE.md`)
- Measure: a census over 240 real seeds per role
  (`crates/nova_authoring/tests/start_role_census.rs`) replaces the six
  bench plays. A bench play has no role flag and no jobs view, costs about
  14 USD, and no earlier New Game play reached a payout: it would measure
  the pilot. The census runs the game's own start pick, board draw, hull
  gate, pay and goal count. It models no combat, mining or dock skill.
- Before, measured: the first board has a row the hull can do and be paid
  for in 41% (industrial), 60% (transport), 83% (scavenger) and 88% (the
  three combat roles) of seeds. 90% of Recover draws and 86% of Neutralize
  draws post as a Procure (no wreck or hostile near a start), and kit or
  station stock fills about 5% of Procures. Goal row on the first board:
  military 92%, pirate 0%.
- A Recover or Neutralize job with no target in reach spawns its own: a
  wreck with a hold, or one raider of the local escort band, at a drawn
  leg. Neither falls back to Procure. Spawned like the other job bodies.
- Job mix per role, six mixes (weights Procure, Recover, Escort, Distress,
  Neutralize, Supply): Mining 4,1,0,2,0,5 (industrial); Salvage 1,6,1,2,1,1
  (scavenger); Trade 3,1,1,2,0,5 (transport); Bounty 1,1,2,1,7,0 (bounty
  hunter); Patrol 1,0,4,3,1,3 (military); Raid 1,4,0,1,5,1 (pirate).
  `Combat` is removed **(breaking)**.
- Starter board: every slot posts a job the start hull can do (a refused
  kind is drawn again), so each role sees three rows.
- Goals: line one is the role's own count, line two is "Stations owned 1".
  This swaps the order the owner set on 2026-10-12 (station first), because
  the pane shows the first unmet line and a station costs about 50,000 cr.
  Every count is 3. Needs owner sign-off.

  | Role | Goal line one | Counts |
  |---|---|---|
  | industrial | Ore deliveries 3 | handed-in Procure of a Raw item |
  | scavenger | Wrecks recovered 3 | handed-in Recover |
  | transport | Cargo runs 3 | handed-in Procure, Supply or Escort |
  | bounty hunter | Bounties handed in 3 | distinct Neutralize targets |
  | military | Patrols flown 3 | handed-in Distress, Supply, Escort; hail jobs |
  | pirate | Hulls taken 3 | trading hulls the player neutralized, any stance |

  `StationNetwork` is removed (one station stands in a 160 km window). The
  pirate line no longer needs a civilization turned Enemy first.
- Kits: credits 2,200 unarmed, 1,800 armed, 1,500 scavenger (its hull is
  worth twice the tug). Transport gains 10 hull plates. Every kit gains 6
  section panels, so a start hull can rebuild lost light hull in the field.
- Hulls, small changes only (looks are task 20261116-132144): the
  interceptor gains two thrusters and is the fastest armed start; the
  patrol picket gains one thruster and six hull sections and stays the
  lightest armed start.
- Not changed: Procure item pool, station funds, PDC rounds in kits, role
  descriptions (owner writes them), the unarmed starts stay unarmed.

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
