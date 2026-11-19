# World encounters: seeded events and job offers in space

- STATUS: CLOSED
- PRIORITY: 75
- TAGS: v0.19.0, world, jobs

Planned 2026-11-16 from the owner review of the game-feel research doc
(claude.ai artifact Hq6GmMBXm2zAfVFwGnQg45).

## User facts
- Long flights and empty space are intended. The world stays almost empty.
- Fill the waits with random events from time to time: things to see or
  find, planet activity, fights, job offers, distress signals.
- Jobs can appear in the world, not only at a station. The player accepts or
  declines and does the job there and then.
- Owner call 2026-11-16: this reverses "Do not add new ships, candidate
  slots, or cluster frequency" and "no encounter-zone markers"
  (`tasks/20260925-190131/TASK.md:11,20,26`) for encounters.

## Agent findings
- No random or timed event system and no encounter director exist. The world
  is a fixed seeded layout (`crates/nova_world_base/src/clusters.rs:19-30`),
  patrol ships per cluster slot (`sector_ships.rs:11-81`) and station visits
  (`station_visits.rs:1-30`).
- Open-world scenario handles only OnStart, OnResume, OnDestroyed and job
  events (`assets/base/scenarios/open_world.content.ron:13-251`).
- Scenario vocabulary to build on: OnTimerEnd, OnSectorEntered,
  OnJobAccepted/Done/HandedIn/Dropped, OnStanceChanged; actions TimerStart,
  SpawnScenarioObject, PostJob (Procure only, station-bound), GrantCredits,
  GiveItems, SetStance, PatrolShip, MoveShipTo, SetAllegiance, PlaySound
  (`crates/nova_scenario/src/events.rs`, `actions/mod.rs`).
- Job accept is docked only (`crates/nova_gameplay/src/jobs.rs:33-36`).
- Prior research: an encounter director seeded by
  `hash(sector_seed, visit_count, world_day)`
  (`tasks/20260824-125938/RESEARCH.md:206,308`).

## Open decisions (owner)
- Rate: how often an encounter fires, so space stays mostly empty.
- First encounter set for v0.19.0 (agent ideas, not approved: distress
  signal, ambush, derelict find, trader hail with a job, planet activity).
- How an in-space offer reaches the player (comms hail, HUD prompt).
- Save rule for a live encounter.

## Decisions (owner said implement 2026-11-18; agent calls, stated; map `MAP.md`)
- Owner of the logic: a director in `nova_world_base`, not the scenario
  engine. A scenario-spawned ship cannot be saved (`MAP.md` G); encounter
  bodies are spawned like job bodies: unique `GeneratedBodyId`, under a
  cell root, kept by the ledger.
- Content: a new `Encounter` kind. `EncounterDesign { id, weight, player,
  kind }`; `player` is `Any` or `Armed`; `kind` is a closed set:
  `Hail { job: Distress | Supply, line }`, `Ambush`, `Derelict`. Lint:
  weight above zero, a non-empty short line. An empty set is valid: no
  encounter fires. **(breaking)** only as a new kind; mods need no change.
- First set (base): `distress_call` (Hail, Distress), `supply_call` (Hail,
  Supply), `ambush` (Armed player only), `derelict_find`. Planet activity
  and a convoy under attack wait for a later slice.
- Rate: one encounter per 300 to 900 s of eligible travel, drawn per
  encounter. Travel counts only while the player is undocked, faster than
  50 m/s, more than 10 km from every station, not in combat, not
  neutralized, with no SOS call, no live encounter and no hail offer or
  hail job. One live encounter at a time.
- Determinism: the draw is `Fnv64(seed, "encounter", fired)`; `fired` and
  the travel seconds are saved, so a Load neither repeats nor skips one.
- Placement: ahead on the player's velocity, `clamp(speed x 30 s, 6 km,
  40 km)` out and 2 km aside, in a live cell, 2 km clear of standing
  bodies and 10 km from stations; no place found means try again later.
- Hull sizes: ambush raiders use the escort raider band of the place's
  advancement (count and budget); callers and wrecks are civilian hulls of
  the civilization in reach.
- In-space job: the hail offers a real `Job` posted by the caller hull.
  `JobId.station` becomes `JobId.poster`; a new source `Hail { serial }`
  allows Distress and Supply only. Range is the metres to the caller at
  the hail; pay follows `job_reward`.
- How the offer reaches the player: a comms line from the caller, a HUD
  notice, a `HAIL` chip with distance on the caller, and one row on the
  Jobs pane Board while undocked, with Accept and decline (X). No new
  key. Docking with the caller while the offer stands accepts it.
- Hand-in rule: a hail job pays when it is done (on the dock with the
  caller), in space, one-sided, cause `JobReward`, with the `Payout` cue
  and the `PAID` notice. No station hand-in. No stance merit yet.
- Offer end: accepted, declined, 300 s unanswered, or the caller lost or
  out of the live window. The hull stays in the world as a derelict.
- Other signals: an ambush has no message (hostile contacts show as they
  do now); a derelict find wears a `SIGNAL` chip until the player comes
  within 1 km.
- Save: `WorldSaveState` gains the encounter state (`fired`, travel
  seconds, the pending offer, the live record by body ids). Bodies ride
  the ledger. Prey and chips are re-issued after a thaw or a Load.
- Hand testing: a debug command fires a named encounter at once.

## Delivery
1. Encounter director seeded through `bevy_rand`, with content-authored
   encounter definitions (explicit fields, known IDs, lint then load).
2. In-space job offer: accept or decline without docking; hand-in rule
   defined per job kind.
3. First encounter set, then wiki and changelog.

## Verification
- Unit tests for encounter selection with a fixed seed.
- nova-probe flow: a seeded run fires a known encounter and the player
  accepts its job in space.
- A playable example.

## Done when
- Travel between stations sometimes meets a seeded encounter, at least one
  encounter offers a job in space, and the rate keeps space mostly empty.

## Delivery record (2026-11-19; task CLOSED)
- Landed: `Encounter` content kind, lint, editor support and base set
  (`assets/base/encounters/`); pure rules
  (`crates/nova_gameplay/src/encounter.rs`); the director
  (`crates/nova_world_base/src/encounters.rs`); saved encounter state;
  undocked offer row on the Jobs pane, `HAIL` and `SIGNAL` chips, comms
  line, `PAID` notice; debug command `encounter <id>`; `system_encounters`
  (probe flow) and `encounter_sandbox`; wiki, creator pages, changelog.
- Rules added in delivery (agent calls):
  - In combat means weapons hot or any threat contact. A due draw with no
    place tries again every 5 s. A draw the hull cannot take is spent.
  - A hail job fires no scenario job events; they follow station board and
    `PostJob` jobs only. The Take a job chip clears on a board job.
  - A body of the live encounter is off the station job field until the
    record ends.
  - A destroyed body leaves the live record at once; the record ends with
    its last body. A save with a held hail job and no live record is
    refused.
  - A held hail job whose caller is frozen outside the live window stays
    held until the player drops it; no encounter fires meanwhile.
  - A HUD chip whose target is on screen near an edge stays whole in view
    (all chip families).
- Review (independent, read-only): 2 high, 1 medium, 6 low. Fixed: a board
  Recover job on an encounter hull panicked on accept; every save failed
  while an ambush had one raider destroyed and one standing; a hail pay
  spent the open-world first-delivery line and bonus; orphaned hail job on
  Load; draw test pinned to literal values; a refused command spends no
  draw; `HailAnswerType`. Not changed: the frozen-caller rule and the chip
  clamp, above.
- Verified: targeted unit, ECS and save tests (the two high defects have
  tests that failed before the fix); workspace check; live under Xvfb and
  lavapipe: `system_encounters` through the probe OK (seed fires
  `base/distress_call`, accepted in space, pay 589 cr exact),
  `encounter_sandbox` at 1280x720 and 1024x768, `system_station_jobs`,
  `system_job_distress`; frames read (chips, comms line, offer row at both
  widths, held job, `PAID` notice, ambush marker).
- Not verified: hand play; a Supply hail live (ECS test only); the chip
  clamp in a frame (unit test only); Load mid ambush live; player death,
  SOS rescue or hull buy with a hail job held; audio; `npm run ci`,
  `mdbook build`, workspace tests and clippy.
- Open: the caller says its drive is dead but its plumes are lit; the
  contact caret draws over a chip label above a small distant hull; a
  single raider can neutralize an idle hull fast at 1.5 km.
- Later: planet activity, a convoy under attack, stance merit for a hail
  job, scenario events for hail jobs.

## Owner revision after play (2026-11-19)
- Owner: the Jobs pane always shows the split of two lists, docked or
  not. "Board" is renamed "Available"; it holds what the player can take
  now (the docked station's jobs, a standing hail offer). An offer is
  never under "Your jobs" until taken; Accept and X work with no dock;
  the dock with the caller takes it.
- Landed: both panels always draw; a hail offer is the first Available
  row, also while docked at a station; an empty Available keeps its
  column heads. A held Supply hail gains Hand in while docked with its
  caller (before, it needed an undock and a new dock).
- Cause of the owner report: undocked with no offer the pane drew "Your
  jobs" alone. The offer with Accept while docked with the caller was not
  reproduced: the dock-alone walk step and its ECS tests pass on the
  unchanged dock path. Likely the sandbox: a new hail fired while still
  docked with an earlier, paid caller.
- Verified: pane and hail tests; `encounter_sandbox` at 1280x720 and
  1024x768 (three pays: Accept in space, dock alone, short Supply then
  Hand in), `system_encounters` through the probe, `system_station_jobs`,
  `job_board_sandbox`; frames read. Not verified: hand play; an offer
  listed with station jobs in a live frame (ECS test only).
