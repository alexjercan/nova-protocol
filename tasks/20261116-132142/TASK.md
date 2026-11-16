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
