# World encounters: seeded events and job offers in space

- STATUS: OPEN
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
