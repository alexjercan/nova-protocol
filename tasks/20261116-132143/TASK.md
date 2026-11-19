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

## Delivery record (2026-11-19; OPEN until the owner signs off the goals)
- Landed (`54e9fe1f2`, `ddf5877a3`, then the review fixes):
  - Recover and Neutralize jobs with no target in reach post with their
    own wreck or raider, spawned on Accept under the poster's cell root.
  - Six mixes; a starter board of three rows the start hull can do; the
    Accept command checks the hull as the pane does.
  - Role goals as in the table above; the transport title is "A freight
    line".
  - Kits, credits and the two hulls as decided. Interceptor: five drives,
    77.6 m/s2, hold 2,800 kg. Patrol picket: three drives, 51.5 m/s2,
    2,320 HP.
  - `NOVA_SANDBOX_ROLE` on `job_board_sandbox` and `start_role_sandbox`.
- Changed from the Decisions during tuning (agent calls):
  - Final mixes (Procure, Recover, Escort, Distress, Neutralize, Supply):
    Mining 5,1,0,2,0,4; Salvage 1,6,1,2,1,1; Trade 3,1,1,2,0,5; Bounty
    2,2,2,2,4,0; Patrol 2,0,4,1,3,2; Raid 1,4,0,1,5,1.
  - A Raw procurement asks a lot of four (8 to 32 units), on every board.
    The reward formula is unchanged.
  - Two own-target rows of one kind on a board never share a leg. Own
    raider leg 8 to 16 km, own wreck leg 6 to 16 km.
  - A hull taken is a ship (never a station) with a market, neutralized
    within 30 s of a hit by the player or a fleet escort. A stance offence
    uses the same rule (before: any old hit).
  - A recovery needs hold room for its stack. A dropped or renewed job
    retires its bodies from a frozen sector too.
- Evidence: `CENSUS-BEFORE.md`, `CENSUS-AFTER.md` (240 seeds). Median
  gain at 15 min 3,280 to 4,532 cr for the six roles (before 0 to 2,504);
  median first payout 177 to 241 s; goal 1 or 2 of 3 for five roles;
  at most 1.7% of seeds with no job (before up to 58.8%).
- Not met: the pirate goal. The board posts no row for it; a pirate that
  only hunts takes three hulls in about 270 paper seconds, one that only
  works the board takes none. Owner call: keep 3, raise the count, or put
  a row for it on the Raid board.
- Model limits: paper time; 60 s a hostile; mining from beam yield; no
  deaths, no dock skill, no trader weapons or escorts. No bench play and
  no hand play was done ("on the bench" in Done when is judged by the
  census).
- Review (independent, read-only): 0 high, 1 medium, 5 low, 5 suspicions.
  Fixed: goal and dropped-job records leaked into a New Game; hull-taken
  rule too wide; starter-board wait after the renewal loop; thruster
  widget text; census accept path. Suspicions found real and fixed:
  frozen job bodies outlived a drop; a board could name a wreck retired on
  the same pass; Recover passed the hull gate with no hold room. Not real:
  leash centre after thaw; a stuck job on death, SOS or hull buy.
- Verified: filtered unit and ECS tests (defects have tests that failed
  first); workspace check; content gen and lint; 13 probe walks under Xvfb
  and lavapipe after the retune, 6 again after the fixes; frames read
  (six first boards, six goal lines, spawned job chips, both hulls).
- Not verified: hand play; a drop with a frozen cell live; the SOS rescue
  bill against an undone Recover stack; saves written between the two
  commits may hold orphan job bodies; workspace tests and clippy.
- Open for the owner: goal order and counts (sign-off); role `about` and
  `lore` texts (industrial says "sell it", transport says "buy ... sell";
  the goals count job hand-ins); the pirate goal call above.
- Seen, not changed: a station-stocked Procure can be filled by buying
  from the same station; "Recover 43 Rations" beside "Recover 2 Water
  ice"; job route reads "X station > X station" when poster and hand-in
  match; map labels crowd in one cell at default zoom; a red range label
  overlaps a JOB chip.
