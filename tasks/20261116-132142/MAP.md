# MAP: world encounters (code as of cc24e1238)

Read-only map. Every claim has `path:line`. "Not verified" marks a claim
that was not read in code. "Inference" marks a conclusion from the cited
lines. Paths are from the repository root.

Short names used below:

- `wb/` = `crates/nova_world_base/src/`
- `w/` = `crates/nova_world/src/`
- `gp/` = `crates/nova_gameplay/src/`
- `sc/` = `crates/nova_scenario/src/`
- `ui/` = `crates/nova_interface/src/`
- `hud/` = `crates/nova_hud/src/`
- `ship/` = `crates/nova_ship/src/`

## A. World layout and streaming

- A cell is 32 km on each edge. The live window is radius 2: 5x5x5 = 125
  cells, a 160 km cube. `wb/lib.rs:200`, `wb/lib.rs:206`,
  `w/lib.rs:256`.
- The world streams only while a `ScenarioWorldType::Generated` scenario is
  live with one player ship. `wb/lib.rs:226-229`.
- Each live cell has one root entity `SectorRoot(SectorCoord)`.
  `w/streaming.rs:70`, spawned at `w/streaming.rs:288-299`.
- System stages: Cleanup, Observe, Request, Collect, Materialize, Adopt,
  Retire. `w/lib.rs:669-689`. At most one cell is materialized per frame
  (`w/lib.rs` plugin build `:758-840`).
- A cell is either `Fresh` (built from the seed, plus arrivals) or `Thawed`
  (built from its frozen record). `w/streaming.rs:262-270`. So a body that
  was in a cell when it retired comes back from the record, not the seed.
- Ownership rule: a body belongs to the cell it stands in NOW.
  `w/lib.rs:104-141`. `adopt_moving_bodies` re-parents a body under the
  live root of its cell by avian `Position`, and freezes it into
  `FrozenSectors` when its cell is off-window. `w/frozen.rs:1174-1293`.
- Adoption skips `WorldObserver`, `PlayerFleetMarker` and
  `ScenarioAddressableMarker`. `w/frozen.rs:1195-1200`.
- Persistent body kinds: ship root, asteroid, cargo canister, wreck
  fragment, mined ore drop. `w/frozen.rs:1132-1141`. `freeze_body` panics on
  any other entity. `w/frozen.rs:627-721`.
- Frozen record kinds: Asteroid, Ship, Station (with docked hulls),
  PendingShip, PendingStation, Canister, WreckFragment, OreDrop.
  `w/frozen.rs:490-520`. Each record keeps `frozen_at`. `w/frozen.rs:384-399`.
- A generated ship is held back (`PendingSectorShip`) while the observer
  overlaps its clearance sphere. `w/streaming.rs:87`. Ship clearance cap is
  400 m. `w/generation.rs:555`.
- The canonical ship spawn is `spawn_sector_ship`: `base_scenario_object` +
  the `GeneratedBodyId` component + `spaceship_scenario_object` +
  `ChildOf(root)`; a derelict adds `DerelictShipMarker`.
  `w/streaming.rs:505-541`.
- Cluster generator: anchors on a 50 km lattice, jitter 0.4 (anchors at
  least 30 km apart), cluster extent at most 8 km.
  `wb/clusters.rs:106`, `:123`, `:132`. Doc: "a 160 km window holds a few
  clusters and long runs of open space". `wb/clusters.rs:101-105`.
- Cluster types: AsteroidRich, RockOnly, PlanetHeavy, DerelictField, or no
  cluster. `gp/chart.rs:41-51`, weights `wb/clusters.rs:1180-1199`.
- Background scatter chance is at most 0.35 per cell.
  `wb/clusters.rs:264`, `:1203-1205`.
- "A 125-cell window holds about one station." Each living station has 2
  traffic hulls at 2 to 4 km. `wb/clusters.rs:238-242`, `:251`.
- A living civilization keeps 1 to 3 stations. Centroids are at least
  160 km apart. `wb/sector_stations.rs:1-29`, `:18-19`, `:52`.
- Generated hulls: living civilization gives an intact AI ship, extinct
  gives a derelict wreck (`generate_wreck`). `wb/sector_ships.rs:260-268`.
  Intact AI: patrol radius 1500 m, stops 30 to 60 s, leash 5000 m, engage
  delay 8 s. `wb/sector_ships.rs:69`, `:79`, `:82`, `:91`.
- Speed: GOTO has no speed cap for the player. "A longer leg just means a
  higher peak speed." `web/src/wiki/flight-autopilot.md:54`. Only AI
  haulers wear `FlightCruiseCap`. `ship/flight/state.rs:94-101`.
- Flight time between places: NOT VERIFIED in code. The research note gives
  rest-to-rest `2 * sqrt(D / a)` at 5 g: 100 km in 90 s.
  `tasks/20260824-125938/RESEARCH.md:222-230`. By that formula 30 to 50 km
  is 50 to 64 s. The main-drive acceleration of each hull was not read.
- Inference: the player crosses a 32 km cell in under a minute and meets a
  cluster about once per 50 km. Open space between clusters has only
  background rocks.

## B. Seeds and clocks

- World seed: `WorldConfig.seed: u32`. `w/lib.rs:433-445`. New Game draws
  it from the global RNG. `crates/nova_menu/src/world_setup.rs:243-248`.
- `OpenWorldSession {seed, role, start}` holds it for the session.
  `sc/objects/start_role.rs:45-52`.
- Hash tools: `Fnv32` `gp/hash.rs:40`, `Fnv64` `gp/hash.rs:81`,
  `SeedStream` `gp/hash.rs:130-162` (`next_u32`, `unit`, `signed`).
- Per-purpose streams in generation: `SectorGenerationInput::stream(purpose)`.
  `w/lib.rs:369`.
- Real seeded use 1 (jobs): `Fnv64(seed, "station_jobs", station id, slot,
  posting)` -> `WyRand::from_seed`. `gp/jobs.rs:2423-2440`.
- Real seeded use 2 (job bodies): `Fnv32(seed, aspect, job id)`.
  `wb/jobs.rs:782-788`.
- `bevy_rand`: one `GlobalRng`, `EntropyPlugin::<WyRand>`, seeded by
  `NOVA_SEED` when set. `gp/plugin.rs:96-101`. Real use: the New Game seed
  draw above.
- `WorldClock { play_seconds: f64 }`. `gp/clock.rs:30`. It advances on
  `Time<Virtual>`. `gp/clock.rs:66`. It is saved. `wb/save/mod.rs:324-327`.
- Clock holders: `FreezeOwner {PauseMenu, Interface, ScenarioLoad,
  WorldResume, DebugHud}`. `gp/freeze.rs:27-47`. There is no "docked"
  owner. The clock runs while docked unless Tab is open.
- The scenario has its own pause-frozen clock (`scenario_elapsed`) and keyed
  timers. `sc/world.rs:213`, `:677-696`. Both are saved.
  `sc/resume.rs:35-36`, `:46-48`.
- There is NO per-cell visit count and NO day counter. The chart keeps only
  the first entry time of a cell. `gp/chart.rs:127-133`, `:195-198`.
  The `hash(sector_seed, visit_count, world_day)` idea exists only in
  `tasks/20260824-125938/RESEARCH.md:206-207`.

## C. Player travel state

All on the player ship root unless stated.

| Fact | Where |
| - | - |
| Is the player | `PlayerSpaceshipMarker` `gp/markers.rs:68` |
| Fleet escorts | `PlayerFleetMarker` `gp/markers.rs:80` |
| Docked | `DockedShip` `ship/sections/docking_section/connection.rs:200`; `DockingConnection` `:154`; `StationDock` `:224` |
| Current cell | `CurrentSector` `w/streaming.rs:142`; `PlayerHull.cell` `wb/jobs.rs:161-222` |
| Cell crossing | `SectorCrossed {from,to}` message `w/streaming.rs:153-158`; not written on the arming frame |
| Weapons hot | `WeaponsHot(bool)`: stance raised or combat lock. `ship/input/targeting/state.rs:181-191` |
| Combat lock | `CombatLock(Option<Entity>)` `ship/input/targeting/state.rs:106` |
| Hostiles near | `ThreatContacts.entries` `ship/input/targeting/state.rs:193-200` |
| Neutralized | `NeutralizedMarker` `gp/integrity/neutralize.rs:72` |
| SOS running | `ShipRescueCall` `ship/sections/rescue_call.rs:47` |
| Repair running | `ShipRepairJobs` `ship/sections/repair_jobs.rs:71` |
| Craft running | `ShipCraftJob` `ship/sections/craft_job.rs:61` |
| Autopilot | `Autopilot {action, phase}` `ship/flight/state.rs:328-332`; `AutopilotAction` (Stop, Goto, GotoPos, Orbit, Match..) `:404` |
| GOTO distance, ETA | `ManeuverTelemetry` `ship/flight/state.rs:590` |
| Speed | avian `LinearVelocity` (not verified by a read of one use) |
| One summary | `HudSituations {maneuver, combat_lock, weapons_hot, firing, docked}` `hud/situation.rs:46-57` (HUD crate only) |

- Nearest station: `find_rescue_station(world, from)` returns the nearest
  living non-Enemy station in the 3x3x3 civilization block. It needs
  `&mut World`. `wb/rescue.rs:101-132`, `:30`.
- No "distance to nearest body" value exists as a resource. Not found by
  grep. A director must compute it from `GeneratedBodyId` +
  `GlobalTransform` queries, as `spawn_job_bodies` does.
  `wb/jobs.rs:778-782`.

## D. Jobs in space

Shape:

- `JobId {station, source}` `gp/jobs.rs:248-253`.
- `JobSourceType {Slot{slot,posting}, Visit{visit,index}, Scenario{key}}`
  `gp/jobs.rs:258-284`.
- `Job {id, kind, target, leg_m, threat, reward}` `gp/jobs.rs:1206-1227`.
- `JobReward {base_cr, goods_cr, range_cr, threat_cr}` `gp/jobs.rs:1233`;
  `job_reward(..)` `gp/jobs.rs:1299-1336`.
- `JobType`: Procure, Recover, Escort, Distress, Neutralize, Supply.
  `gp/jobs.rs:1019-1082`.
- `JobPostings` (per-station slot postings) `gp/jobs.rs:487`. `Jobs`
  (held, done, completed, legs, dropped) `gp/jobs.rs:1834-1840`.
- `JobBoard(Vec<Job>)` is a component on the station root.
  `gp/jobs.rs:1700`.
- Events: `JobAccepted` `:1721`, `JobDone` `:1731`, `JobHandedIn` `:1740`,
  `JobDropped` `:1752` in `gp/jobs.rs`.

Rules:

- Accept and hand-in are docked only. Doc `gp/jobs.rs:33-36`.
  `plan_job_accept` returns `NotDocked` with no docked id and
  `NotPostedHere` at another station. `gp/jobs.rs:2627-2637`.
- Why docked only: the accept reads the job off the docked station's
  `JobBoard`. `ui/jobs.rs:1860-1876`. The board is drawn only docked.
  `ui/jobs.rs:5-9`.
- `JobAccepted` -> `spawn_job_bodies` puts hauler, raiders or caller in the
  world. `wb/jobs.rs:729-978`, registered `wb/lib.rs:257`.
- Done rules, one observer each: recover on undock, escort on order
  complete, distress on dock with the caller, supply on dock with goods,
  neutralize on kill. `wb/jobs.rs:1306`, `:1362`, `:1439`, `:1471`, `:1517`.
- Hand-in: docked at `Job.target`. The station pays from its own
  `ShipCredits` (`checked_sub`). `gp/jobs.rs:2820-2873`. Credits move on
  both sides with cause `JobReward`. `ui/jobs.rs:1945-1950`.
- Expiry: an unheld offer expires after the price table window. A held job
  never expires. `gp/jobs.rs:16-21`, `:433-435`.
- Drop: free from the pane (`ui/jobs.rs:35-42`). The world drops a held job
  when its poster, hauler, wreck or caller is lost, or the hauler order
  fails. `wb/jobs.rs:1225-1299`.
- Drop cleanup despawns only LIVE bodies. `wb/jobs.rs:1288-1293`.
  `FrozenSectors` has no remove-by-id call (`w/frozen.rs:67-110`, writers
  are `visit` `:135` and `arrive` `:161`, both `pub(crate)`).

What assumes a station poster (each breaks for a job with no station):

1. `Job::validate` refuses `target != id.station`. `gp/jobs.rs:1534-1539`.
2. Visit and Scenario sources must be procurements. `gp/jobs.rs:1506-1519`.
3. `JobId::spawned_body` names bodies `<station stem>_job_<slot>_<posting>_
   <part>` in the station's origin cell, and is for slot ids only.
   `gp/jobs.rs:353-364`.
4. `spawn_job_bodies` panics when the poster is not a live station, and
   parents bodies to the poster's cell root. `wb/jobs.rs:764-775`.
5. The hull civilization comes from the poster's `TraderMarket`.
   `wb/jobs.rs:146-155`, `:800`.
6. `leg_direction` keeps the mark inside the poster's cell.
   `wb/jobs.rs:1009-1053`; legs are capped at 16 km for that reason
   `gp/jobs.rs:144-148` (constant block `:101-166`).
7. `post_job_done_notice` panics when the poster is not on the chart.
   `hud/hud_notice.rs:539-559`.
8. The pane names the hand-in from the chart station record, with the raw
   id as fallback. `ui/jobs.rs:311-316`, `:343-351`.
9. The map marks a procurement in `job.target.origin`.
   `ui/map/chart.rs:186-191`.
10. Scenario job events carry `station = job.id.station`.
    `sc/loader/bridges.rs:32-40`.
11. `drop_on_loss` keys on the poster body. `wb/jobs.rs:1225-1266`.
12. Save: `Jobs::validate` runs each `Job::validate` on Load
    (`gp/jobs.rs:2050-2099`, `wb/save/mod.rs:662-665`), so points 1 to 3
    also refuse a save.

Not broken: goal counters read `Jobs::completed()` by kind only.
`wb/goal.rs:185-214`. HUD chips come from `mark_held_job_targets`, keyed by
body id, not station. `wb/jobs.rs:1565-1626`. The paid notice needs only the
code and reward. `hud/hud_notice.rs:564-568`.

Smallest change to offer and accept undocked (facts, not a design):

- The accept path needs a `Job` value and a pass of `plan_job_accept`.
  A second accept command that takes the `Job` from another source than
  `JobBoard` would bypass `ui/jobs.rs:1860-1876` only.
- `plan_job_accept` needs a non-docked branch or a new planner.
  `gp/jobs.rs:2634-2637`.
- A fourth `JobSourceType` variant is needed for an id with no slot, with
  its own `spawned_body` naming. `gp/jobs.rs:258-284`, `:353-364`.
- `JobId.station` is not optional. A stationless job needs either a real
  station in `station` (pay later at a station) or a type change that
  touches every item in the list above.
- "Your jobs" already lists held jobs undocked. `ui/jobs.rs:13-17`.

Paying without a station:

- The one balance writer is `ShipInventoryReports::change_credits(credits,
  current_cr, cause)`. `gp/inventory.rs:835-851`.
- One-sided payments exist: scenario `GrantCredits` (cause `Scenario`)
  `sc/actions/hold.rs:100-157`; the SOS bill (cause `Rescue`)
  `wb/rescue.rs:316-324`.
- `CreditChangeCauseType` values: Trade, Take, Rebuild, Swap, Add, ShipBuy,
  EscortHire, StationBuy, JobReward, JobRefresh, Settle, Craft, Cheat,
  Scenario, Rescue. `crates/nova_events/src/lib.rs:969-1004`.
- The payout moment (count-up, `Payout` cue) lives in the Jobs pane.
  `ui/jobs.rs:1951`, `:1977`; cue `crates/nova_sound/src/lib.rs:180-182`.
  The HUD line `JOB-n PAID 1,460 CR` is posted silent because the pane
  plays the sound. `hud/hud_notice.rs:9-13`, `:564-568`.

## E. Comms, prompts, input

Comms panel:

- `StoryLine {speaker, text, dwell, icon, accent}`; `StoryFeed(Vec)`.
  `hud/comms_panel.rs:48-62`, `:71`. Dwell 8 s default, 3 to 30 s.
  `hud/comms_panel.rs:77-82`.
- One-way. No choice, no key, no callback. `hud/comms_panel.rs:1-22`.
- One writer: the scenario story log is copied to `StoryFeed`.
  `sc/world.rs:296-314`, `push_narrative_cue` `sc/world.rs:599`. The action
  is `NarrativeCue`. `sc/actions/mission.rs:53`, `sc/actions/mod.rs:223`.
- The story log is saved with the scenario. `sc/resume.rs:42-43`.
- Open world uses it for "Station Control" lines.
  `crates/nova_authoring/src/base_content/scenarios/open_world.rs:44`,
  `:132-167`.

HUD notice:

- `HudNotice { text, cue }`, `new` (with cue) and `silent`.
  `hud/hud_notice.rs:99-125`. One slot, no queue; a new text replaces the
  shown one. Dwell 4 s on virtual time. `hud/hud_notice.rs:42-45`, `:78`.
- Writers: dock, undock, job done, job paid, supply refused, repair, SOS,
  craft, ammo. `hud/hud_notice.rs:5-41`. Sector crossing posts
  `SECTOR (x, y, z) - FIRST VISIT`. `ui/sector_notice.rs:40-60`.
- Inference: a sector-crossing line and an encounter line on the same frame
  fight for the one slot.

In-flight prompts with a key:

- Keybind dock: a row of chips from `FlightVerbHints`. Core group
  (INTERFACE, DRIVE, STOP, RCS, RADAR), situational group (GOTO, ORBIT,
  CANCEL, DOCK, HELM, MINE), weapon group. A chip draws only while its verb
  applies. `hud/keybind_dock.rs:9-23`.
- Anchored cues: a keycap chip on the thing to act on (ORBIT on a well,
  GOTO on the lock). `hud/keybind_dock.rs:24-26`.
- Cinematic skip prompt: `CinematicPrompt { skip_action }`.
  `hud/cinematic_prompt.rs:46-51`.
- `hud/docking_sight.rs` and `hud/pickup_sight.rs` exist. Not read.
- A scenario can pulse a dock chip by verb name (`HintEmphasis`).
  `hud/keybind_dock.rs:28-32`.

How a rebindable flight action is added (from commit 2436502f1, which
added keys 1, 2, 3; commit 27916e36d removed all three):

1. `ActionBinding::new(id, "FLIGHT", label).keyboard([..])` in
   `flight_bindings()`. `ship/input/bindings.rs:33-145`. Flight actions are
   reserved against every section key mapping.
2. `#[derive(InputAction)]` type and an `Action::<T>` entry with
   `bindings.bundle(id)` in `flight_input_rig`.
   `ship/input/player/flight_rig.rs` (see the diff of 2436502f1).
3. An `On<Start<T>>` observer, gated on pause and
   `PlayerControlSuspended`, registered in `ship/input/player/mod.rs`.
4. A `VerbHint` field in `FlightVerbHints` (`ship/input/player/hints.rs:71`)
   and a dock chip (`hud/keybind_dock.rs`).
5. Docs and proof: `web/src/wiki/keybinds.md`, `web/src/wiki/hud.md`,
   `examples/screenshots/screenshot_key_hints.rs` (file lists of both
   commits).

Existing actions an accept or decline could reuse:

| Action | Key | Today | Where |
| - | - | - | - |
| `scenario_advance` | Enter, DPadDown | Flight context. Does nothing mid-scenario with no queued scenario and no outcome (`AdvanceDecision::Ignore`). | `sc/loader/lifecycle.rs:709-714`, `:760-786` |
| `dock` | D | Acts on the travel lock. Distress and supply jobs are already answered by a dock. | `ship/input/bindings.rs:64`, `wb/jobs.rs:1439`, `:1471` |
| `autopilot_goto` | G | Fly to the nav lock. | `ship/input/bindings.rs:46` |
| `interface_toggle` | Tab | Opens the interface; holds the clock. | `ui/bindings.rs:50-53`, `gp/freeze.rs:27-47` |
| `radar_hold` | Ctrl | Lock gesture. | `ship/input/bindings.rs:74` |

Digits 1 to 3 are unbound in flight after 27916e36d
(`ship/input/bindings.rs`, 10 lines removed).

## F. Spawning bodies in open space

- Canonical pattern: section A, `w/streaming.rs:505-541`. Job bodies copy
  it. `wb/jobs.rs:834-966`.
- Hull build: `ShipLayoutRequest {seed, civilization, role, advancement,
  size, spare_drive}` -> `generate_ship(parts, request)` ->
  `ship_stock(parts, items, design, role, condition, advancement, draw)`.
  `wb/ship_layout.rs:178-192`, `:462`, `wb/sector_ships.rs:476-484`,
  used at `wb/jobs.rs:802-833`.
- `generate_ship` returns `Result`; `ship_stock` returns `Option`. The job
  code panics on both. `wb/jobs.rs:818-831`.
- `parts` come from `config.generator.parts()`. `wb/jobs.rs:797`,
  `wb/layered.rs:154`.
- A wreck: `generate_wreck(parts, request)`. `wb/ship_layout.rs:541`.
  Derelict caller: `SectorShipConditionType::Derelict`,
  `SpaceshipController::None`, `Allegiance::Neutral`, `lootable: true`,
  `DerelictShipMarker`. `wb/jobs.rs:936-966`.
- Hostile: `AIControllerConfig { leash, engage_range, .. }`,
  `Allegiance::Enemy`, `AIBehaviorState::Idle`. `wb/jobs.rs:834-875`.
  Config fields (patrol, leash, engage_delay, engage_range, non_combatant):
  `sc/objects/spaceship.rs:104`.
- Hauler: Industrial, `non_combatant: true`, `Allegiance::Player`.
  `wb/jobs.rs:879-915`. Helm order: `ShipHelmOrder::new(key,
  ShipOrderDirective::Move{..})` + `ShipOrderHelmAuthority` +
  `FlightCruiseCap`. `wb/jobs.rs:1088-1200`.
- Without a station the civilization must come from the field:
  `CivilizationField::in_reach(position)`. `wb/civilizations.rs:399`.
  `plan_ship` already does this for sector hulls.
  `wb/sector_ships.rs:189-352`.
- Clearance: job spawns try 8 seeded directions, 2 km clear of standing
  generated bodies, 500 m from the station. `wb/jobs.rs:123-131`,
  `:1009-1053`.
- Ids: every persistent body needs a unique `GeneratedBodyId {origin, stem,
  index}`; the stem must be snake_case. `crates/nova_events/src/
  generated.rs:189-218`. `origin` is identity, not the current cell.
- Duplicate ids: `spawn_job_bodies` panics when a body or `<id>/wreck/..`
  already lives. `wb/jobs.rs:750-763`. The snapshot refuses `DuplicateId`
  and `NoDurableId`. `w/frozen.rs:1328-1335`.
- Parent: the cell root (`ChildOf(root)`). A top-level non-addressable
  persistent body is adopted into its cell later.
  `w/frozen.rs:1226-1229`.
- Do NOT add `ScenarioAddressableMarker`: it stops adoption and freezing,
  and the save refuses. `sc/actions/mod.rs:131-139`,
  `w/frozen.rs:1415-1432`.
- Freeze and thaw keep: sections, inventory, health, marks, neutralized,
  AI state, repair jobs, rescue call, craft job.
  `sc/objects/spaceship.rs:650-686`. AI keeps behavior, patrol, grace,
  threat, evade, leash. `ship/input/ai/frozen.rs:27-34`.
- Freeze and thaw DROP: `AIPrey`, helm orders, cruise cap, entity
  references. The owner must re-issue them, as `fly_escort_legs` does.
  `wb/jobs.rs:38-41`, `ship/input/ai/acquisition.rs:27-38`.
- Cleanup: despawn by id match over live bodies. `wb/jobs.rs:1288-1293`,
  `ui/jobs.rs:1977-1996`. A frozen copy is not removed (section D).

## G. Scenario engine as a possible owner

- Events: `OnStart`, `OnResume`, `OnUpdate` (every frame), `OnTimerEnd`,
  `OnSectorEntered`, `OnDocked`, `OnJob*`, `OnDestroyed` and more.
  `sc/events.rs:163-353`.
- `OnSectorEntered`: once per crossing, fields `sector, x, y, z, from`;
  none for the start cell or a Load. `sc/events.rs:331-335`. Fired by
  `w/streaming.rs:642-647`.
- `TimerStart {key, seconds}` / `TimerCancel`. `sc/actions/timer.rs:8-17`,
  `:50-57`, `sc/actions/mod.rs:619`. Timers run on the scenario clock and
  are saved. `sc/resume.rs:46-48`.
- `PostJob {station, key, item, count, reward_cr}`: procurement only, on a
  station bound from the firing event (`$id` under `OnDocked`). No author
  can write a generated station id. `sc/actions/jobs.rs:16-51`,
  `sc/actions/mod.rs:514`.
- `SpawnScenarioObject` `sc/actions/mod.rs:264`. Its bodies get
  `ScenarioAddressableMarker` (`sc/actions/spawn.rs:407`, `:470`). In a
  saved world a persistent addressable body makes the save FAIL
  (`UnownedBody`). `wb/save/mod.rs:35-38`, `w/frozen.rs:1415-1432`,
  `wb/save/session.rs:505-516`. "A scenario that saves spawns only the
  player."
- `GrantCredits` needs an addressable ship id. `sc/actions/hold.rs:100-157`.
- `NarrativeCue` posts a comms line. `sc/actions/mod.rs:223`.
- The open world scenario is built in Rust:
  `crates/nova_authoring/src/base_content/scenarios/open_world.rs:61-177`,
  generated to `assets/base/scenarios/open_world.content.ron` (268 lines).
  Seven `once: true` handlers. `open_world.rs:90-167`.
- Handlers may only be appended: a save names spent `once` handlers by
  index. `open_world.rs:117-119`, `sc/resume.rs:49-52`.
- Saved scenario state: elapsed, variables, objectives, story, readouts,
  timers, spent handlers, generated marks. `sc/resume.rs:31-55`.

Facts for each path:

- Scenario content path: has timers, sector events, comms, credits and
  saved variables today. It cannot spawn a saved ship, cannot post a
  non-procurement job, and has no random draw action (not found by grep;
  not verified across all actions).
- New content kind path: a Rust director in `wb/` can use
  `spawn_job_bodies`-style spawns, the seed, `WorldClock` and the cell
  root. It needs its own save field, lint and merge wiring (section H).
- A hybrid is possible: the director raises a `GameEvent` and the scenario
  answers with comms. No encounter event exists yet.
- Runtime handler injection (`RESEARCH.md:308` "the handlers it injects")
  was NOT found in code. Not verified.

## H. Adding a content kind (checklist from Recipe and Consumable)

Sources: `git show --stat c876b6454` (Recipe), `2436502f1` (Consumable),
`27916e36d`, `9594b0c52`.

1. Type + serde + faults fn + `Game<Kind>s` resource in `nova_gameplay`:
   `gp/consumable.rs:69`, `:78`, `:119`; `gp/crafting.rs:46`, `:124`,
   `:153`. Export in `gp/lib.rs:42`, `:132`.
2. `Content::<Kind>` variant, kind word and id accessor:
   `crates/nova_modding/src/lib.rs:190`, `:200`, `:228-229`, `:250`.
3. Builder: `crates/nova_authoring/src/base_content/consumables.rs:12-24`;
   field and call in `base_content/mod.rs:22`, `:41`, `:60`.
4. Generation: `crates/nova_authoring/src/generation.rs:198-209`, file
   entry `:301-302`.
5. Generated RON: `assets/base/consumables/base.content.ron`; bundle list
   `assets/base/base.bundle.ron:25`. Never hand-edit.
6. Merge + resource insert: `crates/nova_assets/src/merge.rs:207-210`,
   `:811`, `:1114-1116`, `:1264-1271`.
7. Digest: `content_catalog_digest` covers every `Content` item by kind
   and id. `crates/nova_assets/src/merge.rs:996-1037`. Any base content
   change refuses older saves at the header. `wb/save/mod.rs:23-25`.
8. Pack lint in `crates/nova_assets/src/items.rs:48-49`, `:223-240`.
9. Authoring lint walk: `crates/nova_authoring/src/lint_walk.rs:72`,
   `:147-181`, `:1114-1136`.
10. World generator input when the world reads it: `NovaLayeredWorld::
    from_loaded(..)` `wb/lib.rs:524`, `wb/layered.rs:93`, `:170`.
11. Editor: `crates/nova_editor/src/asset/blank.rs:122`,
    `asset/verbs.rs:62`, `glyph.rs:153`, `inspect/asset.rs:210`, `:229`,
    `project.rs:82`, `project/package.rs:252-254`, `:417-420`.
12. Test support: `gp/test_support.rs`; `wb/test_support.rs`;
    `crates/nova_core/tests/world_resume_refusal.rs`;
    `crates/nova_authoring/tests/generated_*.rs` (file lists).
13. Example mod: `assets/mods/example/example.content.ron` (Recipe at
    `:908`; touched by c876b6454).
14. Docs: `web/src/create/mod-files.md:7`, `:304-340`,
    `web/src/create/objects.md`, `web/src/docs-manifest.js`,
    `docs/concept-index.md`, `CHANGELOG.md`.
15. Proof: `examples/systems/system_ship_craft.rs`,
    `examples/playable/craft_sandbox.rs`, `examples/playable/README.md`,
    `Cargo.toml` example entries, slugs in
    `crates/nova_probe_cli/tests/catalog_drift.rs`.
16. Save, when the kind has running state: `wb/save/mod.rs`
    (`check_saved_jobs` `:694`), `wb/save/tests.rs`.

## I. Save

- `WorldSaveState` fields: format, generation, player, sectors,
  canister_ids_next, transients, scenario, chart, jobs, postings,
  station_visits, clock, stances, goal, start. `wb/save/mod.rs:296-336`.
- Strict: `deny_unknown_fields`, one version `v0.18.0`, catalog digest in
  the header, nothing migrated. `wb/save/mod.rs:18-29`, `:109`, `:298`.
  A new field is a format break under the same version string.
- Capture and restore of jobs: `wb/save/session.rs:566-568`, `:373-375`.
- Validation chain on Load: `wb/save/mod.rs:652-695` (ids, clock, start,
  chart, jobs, postings, visits, markets, stances, goal, fleet, controls,
  items, stock, rcs, ship jobs, scenario id).
- Job bodies are saved as ordinary frozen cell bodies. `wb/jobs.rs:30-34`.
  The save freezes every live cell into the ledger. `w/frozen.rs:1445-1457`.
- Running timed things are saved on the ship: `repairs`, `rescue`, `craft`,
  `consumable` in `FrozenShipState`. `sc/objects/spaceship.rs:650-686`.
- The save waits while a body is unsettled and FAILS for an unowned body,
  an unsaved dock, a missing or duplicate id.
  `wb/save/session.rs:495-516`.
- Owner rule on record: timed player jobs are saved and resume.
  SOS: "a timed rescue call on the gameplay clock"
  `tasks/20261116-132138/TASK.md` (Decisions); craft: "One timed craft job
  per ship on the gameplay clock, saved and resumed"
  `tasks/20261116-132140/TASK.md` (First slice). Code doc:
  `ship/sections/rescue_call.rs:1-17`.

What a live encounter needs to survive a Load (inference from the above):

- Its bodies: non-addressable, persistent kind, unique `GeneratedBodyId`,
  under a cell root. Then the ledger carries them with no new code.
- Its record (which encounter, state, timer, linked body ids, offer state):
  a new field in `WorldSaveState` or inside `Jobs`. Either is a format
  break. It must hold ids, never entities.
- A validate step in the chain, with a clock check like
  `postings.validate(clock)`. `wb/save/mod.rs:666-669`.
- A re-issue system for what a thaw drops (prey, helm orders).
- The director's own "next roll" state, or a pure function of
  `(seed, clock, cell)` so nothing needs saving.

## J. Map and markers

- In-flight chip: `ObjectiveMarkerTarget { label }` on any entity grows a
  gold HUD chip with label and distance, edge-clamped as a direction cue.
  `gp/objectives.rs:47-56`.
- Jobs use it: `JOB-n <verb>` and `JOB-n Hand in`. It panics when the body
  already wears a foreign label. `wb/jobs.rs:1565-1626`.
- Scenario marks on generated bodies live in `GeneratedMarks` and are
  saved. `sc/marks.rs:22-33`, `sc/resume.rs:53-54`.
- Map contacts: every ship root, objective marker and terrain body in the
  live window. Kinds: OwnShip, Ally, Hostile, Objective, Terrain, Neutral,
  Planet. `ui/map/contacts.rs:30-48`, `:180-222`. No sensor-range filter
  found by grep in that file.
- A marked ship or rock is one Objective contact that keeps its icon and
  carries the label. `ui/map/contacts.rs:9-14`. "Touch this module when
  adding a kind of thing the map can show." `ui/map/contacts.rs:16`.
- Chart marks: Cell, Station, Wreck, Job. `ui/map/chart.rs:88-100`. A job
  is marked in the cell of its located body, else its target's cell.
  `ui/map/chart.rs:186-191`.
- Allegiance triangles: on-screen ships only. `hud/allegiance_markers.rs:
  1-10`. Edge arrows: lock, candidates, hostile torpedoes.
  `hud/edge_indicators.rs:1-10`.
- Beacons: `BeaconMarker` grows a cyan chip with distance.
  `gp/beacon.rs:14-24`, `hud/beacon_chips.rs:1-12`. A beacon is a scenario
  object kind (`sc/actions/spawn.rs:129`). Inference: it is not a
  persistent body kind, so it is neither saved nor a save blocker. Not
  verified.

Owner decisions on record, `tasks/20260925-190131/TASK.md` (CLOSED):

- Line 11: "Correction: ship spawning/placement stays as it is in the PR
  #96/#99/#101/#103 stack. The suggestion to add separately placed live
  encounter groups or tune their spawn density was wrong. Do not add new
  ships, candidate slots, or cluster frequency in this task unless the
  owner changes this decision."
- Line 20: "Ship counts, positions, and clearance are unchanged; add no new
  candidate slots or density draw. ... This replaces the earlier proposal
  for separate live encounter groups."
- Line 26: "Use existing allegiance-colored HUD/map contacts and visible
  combat to signal nearby fights. ... Do not add encounter-zone markers or
  entry/outcome messages in the first pass. Inspect rendered flight;
  reconsider only if real fights are easy to miss."
- These were scoped to that task ("in this task", "first pass"). The new
  task reverses them only if the owner says so.

Research notes, `tasks/20260824-125938/RESEARCH.md` (nothing accepted):

- `:206-207`: "**Encounter seed** = `hash(sector_seed, visit_count,
  world_day)`, so what is flying there changes between visits while the
  place does not."
- `:272`: "chokepoint the encounter director spawns at".
- `:308`: "`Encounter` | a weighted, conditioned spawn recipe: where it may
  fire, filters on world state, the handlers it injects | encounter
  director".
- `:335`: proposed action `SpawnEncounter`. `:342`: "A `Contract` or
  `Encounter` may name a scenario id."

## K. Proof tooling

- Skill: `.claude/skills/nova-probe/SKILL.md`. Reuse an affected example
  first. `examples/systems/` for asserted behavior, `examples/playable/`
  for hand play. Register examples in `Cargo.toml`; put an
  `outcome: <slug>` marker beside each assertion and register the slug in
  `crates/nova_probe_cli/tests/catalog_drift.rs`. Never assert timing.
- Run form: `nix develop --command cargo run --features dev probe run
  <name> --correctness-only` (same file).
- A flow that accepts a job and flies: `examples/systems/
  system_job_distress.rs:1-33`, built on `examples/systems/shared/
  located_jobs.rs` (2746 lines). It boots `editor_app(true, None)`, starts
  New Game on a pinned seed, undocks, accepts, parks beside the caller,
  docks and hands in. Walk head: `located_jobs.rs:2417-2470`.
- It sandboxes the profile with `CONFIG_ROOT_ENV`.
  `system_job_distress.rs:56-68`.
- Sibling ranges: `system_job_escort.rs`, `system_job_recover.rs`,
  `system_job_supply.rs`, `system_job_neutralize.rs`,
  `system_station_jobs.rs`, `system_scenario_jobs.rs`,
  `system_world_resume.rs`, `system_world_sectors.rs` (directory listing).
- Harness API (`crates/nova_autopilot/src/`): `.step(name)` `autopilot.rs:
  195`, `.on_enter(fn)` `:380`, `.each(fn)` `:394`, `.until(pred)` `:415`,
  `.deadline(secs)` `:428`, `.click_named(..)` `:234`; inputs `press_key`
  `input.rs:74`, `release_key` `:81`, `press_mouse` `:166`; predicates
  `elapsed` `predicate.rs:64`, `frames` `:101`, `ui_node_present` `:249`,
  `and`/`or`/`not` `:430-446`.
- "Fly to" in a walk is a direct insert: `Autopilot::engage(
  AutopilotAction::Goto { target })`. `located_jobs.rs:1382`, `:1598`.
  Parking is a teleport helper. `located_jobs.rs:662-687`.
- Playable: `examples/playable/job_board_sandbox.rs`, index
  `examples/playable/README.md`.
- Name clash: `examples/playable/world_encounters.rs` already exists. It
  proves the allegiance and retaliation rules on a staged cast.
  `examples/playable/world_encounters.rs:1-40`.

## L. Things to see

- Planets: six `PlanetType` values (BarrenRock, DustWorld, IceWorld,
  Volcanic, Greenhouse, Temperate). `sc/objects/planet_type.rs:169-183`.
  Planetoid radius 500 to 1600 m. `wb/clusters.rs:147`. PlanetHeavy
  clusters place 2 to 4. `gp/chart.rs:47-48`.
- Rocks: radius 25 to 120 m nominal. `wb/clusters.rs:137`.
- Derelict fields: 2 or 3 hulls. `wb/clusters.rs:221`.
- VFX modules that use hanabi: `gp/integrity/spew.rs`,
  `gp/integrity/pyre.rs`, `sc/mining.rs`, torpedo, turret and railgun
  render modules (file listing only; effects not read).
- Do AI ships fight each other? Yes, across sides. `relation(Player,
  Enemy)` is Hostile both ways and AI picks targets from the relation
  model. `gp/relations.rs:64-72`, `ship/input/ai/acquisition.rs:19-25`,
  `:342`.
- Do two hostile civilizations fight each other? No. `relation(Enemy,
  Enemy)` is `Own`. `gp/relations.rs:67`. A Neutral fights only the one
  ship that hit it (`RetaliationTarget`). `gp/relations.rs:74-83`.
- So a visible AI fight needs an Ally-side ship and an Enemy-side ship in
  sensor range. Sides come from the civilization stance.
  `wb/sector_ships.rs:320-333`.
- Sounds: `UiSfx` is fixed interface cues (ObjectiveNew, CommsLine,
  HudNotice, Payout..). `crates/nova_sound/src/lib.rs:139-183`. Files in
  `assets/sounds/`. World sounds are authored on section and object
  configs. `crates/nova_sound/src/lib.rs:139-141`. There is no alarm or
  distress cue in the list.

## Risks

(1) Director that spawns bodies near a flying player

- Speed: no speed cap (section A). A body spawned 5 km ahead of a ship at
  1 to 2 km/s is passed in seconds. Spawn distance and trigger must read
  velocity or autopilot state, or fire only when slow.
- Pop-in: a spawn inside view range is seen appearing. Job spawns hide
  this by spawning while docked. `wb/jobs.rs:764-775`.
- Clearance: reuse `leg_direction`-style tries. It warns and takes the
  clearest when all 8 fail. `wb/jobs.rs:1009-1053`.
- Ids: a time-based draw needs a new unique stem and index per spawn.
  Reuse after a drop collides with frozen wrecks (`<id>/wreck/`) and makes
  the save refuse. `wb/jobs.rs:15-24`, `:750-763`.
- Determinism: no visit count or day exists (section B). A draw keyed on
  `(seed, cell)` alone repeats on every visit. A draw keyed on the clock
  needs a quantized bucket and a saved "fired" set, or it re-fires after
  Load.
- Density rule on record: line 11 and 20 of the 20260925 task say no new
  ships or density draws (section J). Owner must lift it.
- Sides: Enemy does not fight Enemy (section L). An ambush on a neutral
  hauler needs the hauler on the Player side or `AIPrey`, as escort does.
  `wb/jobs.rs:879-924`.
- Frame order: spawn after `NovaWorldSystems::Retire` like the job systems
  (`wb/lib.rs:286-348`), or adoption sees a half-built body.
- Cost: `generate_ship` runs a layout search on the main thread at spawn.
  Its cost was not measured here.
- Cleanup: an unanswered encounter leaves bodies in the ledger forever.
  There is no ledger remove (section D).

(2) Job accepted undocked and paid in space

- Twelve station assumptions (section D list). Three panic at run time
  (items 4, 5, 7) and three refuse a save (items 1 to 3).
- Who pays: the station's balance gates the hand-in today
  (`StationCannotPay`, `gp/jobs.rs:2770-2798`). A one-sided grant has no
  such gate and mints credits; `Scenario` and `Rescue` causes set the
  precedent.
- Payout feel lives in the Jobs pane (count-up, `Payout` cue). A space
  payout needs its own cue path; the HUD paid line is silent by design.
- Stance merit is recorded from the paying station's market civilization.
  `ui/jobs.rs:1964-1965`. A stationless job needs another source.
- Input: no free prompt-with-choice surface exists. Comms is one-way; the
  notice slot is one line. Enter is free in flight (section E), D already
  answers a caller.
- Owner UI rule on record (memory, not code): state by counts, dimming and
  disabled buttons, no prose refusals. An accept prompt must fit it.
- The Tab pane holds the clock. An offer timer on `WorldClock` pauses under
  Tab; one on real time does not.

(3) Saving a live encounter

- Format: any new saved field breaks `v0.18.0` saves of other builds
  (strict, no migration). The base content change already does this via
  the digest when a new content kind ships.
- Bodies and record can disagree: bodies are in the ledger per cell, the
  record is global. Load must refuse or repair a record whose body id is
  in no cell, as `check_saved_visits` does for visits.
  `wb/save/mod.rs:670-674`.
- Entities are not saved. Prey, helm orders and cruise caps need a
  re-issue pass on thaw and Load.
- A save can be asked mid-spawn: the snapshot waits while unsettled (600
  frame cap `w/frozen.rs:198`) and fails on a top-level addressable body.
- A pending offer (shown, not yet answered) is state too. Either save it
  or define that a Load clears it and the director may re-roll.
- Scenario-side state (timers, variables, spent handlers) is saved by
  index and key. Adding handlers anywhere but the end breaks old saves.
  `open_world.rs:117-119`.
