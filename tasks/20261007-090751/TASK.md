# Map shipped features and player-experience priorities for v0.16.0

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.16.0,planning

## User facts
- Before scheduling polish or new systems, list all shipped game features and classify core player experience versus side features, with current maturity and friction. Focus v0.16.0 on making existing play enjoyable and reliable.
- Do not equate a compiled feature with a complete player journey.

## Delivery
- Inventory shipped features from code, content, in-game UI, docs, and current player-flow evidence. Include New Game/onboarding, travel/autopilot/world, combat, mining/canisters, inventory/trade/repair, scenarios/story, input/accessibility, mods, and menus. Distinguish available mechanics from complete journeys and non-shipped ideas.
- For each feature record: core/secondary/supporting role, player goal, entry/UI, current maturity (unavailable / prototype / usable with gaps / proven), specific evidence, top friction, nearest proof, and cost/risk of improvement. Mark untested claims unverified.
- Rank a short v0.16.0 polish shortlist by player impact and verified friction; exclude broad additions unless they unblock the core loop. Include old bug tasks only when still reproducible; the earlier docking issue was fixed and the owner cannot reproduce it now.

## Feature map (static audit, 2026-10-14 at `dd42046a9`)

Sources: `git log` since 2026-10-07, CLOSED v0.16.0 task notes, CHANGELOG `[Unreleased]`, code. No build or run in this pass.

Maturity: **unavailable**; **prototype** (code exists, no proof of the flow); **usable with gaps** (proof of the mechanic, gaps named); **proven** (an asserted example or test with a recorded pass for the mechanic). Proven means the mechanic, not fun or clarity. Player-run evidence: only `bench-runs/34dd96176/tutorial/*/score.json` (2026-09-21, tutorial, defeat, 9 of 10 objectives). `~/Videos/nova-bench-*` does not exist; `~/Videos/Recording` holds two unlabeled 2026-10-09 clips. Every claim about player perception is **unverified**.

### Mechanics

| Feature | Role | Player goal | Entry and UI | Maturity | Evidence | Top friction | Nearest proof | Cost, risk |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| New Game create (name, seed) | core | start a world | Menu > New Game modal | proven | `crates/nova_menu/src/world_setup.rs:98,324`; `resolve_new_game_scenario` `crates/nova_menu/src/menu_ui.rs:637`; live runs reached Playing `tasks/20261012-095952/TASK.md:18` | modal copy names derelicts only, no stations (`world_setup.rs:160-163`); harness has no objective (`crates/nova_authoring/src/base_content/scenarios/open_world.rs:7-10`) | `system_open_world` | copy XS, low |
| World save, Load, Load last save | core | keep and resume a world | auto save on arm, sector crossing, leave; Load screen; DEFEAT and pause offer Load last save | proven (desktop) | `crates/nova_world_base/src/save/session.rs:3-8`; format 8 `save/mod.rs:75`; `crates/nova_menu/src/load_screen.rs:182`; `leave.rs:52`; rendered `tasks/20261007-090756/p6-frames/*.png`; round trip Load passed twice `tasks/20261012-095958/TASK.md:33`; death writer 40/40 `tasks/20261007-090722/RESEARCH.md` s8 | rendered native death-to-Load flow missing; web keeps no saves; format 8 refuses all older saves | `system_world_resume`, `system_station_round_trip` | proof S, low |
| Flight, camera, rebinding | core | fly and aim | keys, mouse, Settings | usable with gaps | `crates/nova_ship/src/flight/manual.rs`; rebind `system_headless_rebind` | Alt or RMB mid-motion may keep turning the hull (`crates/nova_ship/src/camera/mode.rs:132-181`), unverified, not reproduced | ECS repro on `mode.rs:613` rig | S, low |
| Navigation: locks, GOTO, ORBIT, Map | core | reach a target | lock keys, dock verbs, TAB Map | usable with gaps | `crates/nova_autopilot/src/autopilot.rs`; curved GOTO path (CHANGELOG Interface); `system_flight_legs`, `system_headless_map_goto` | refused GOTO/ORBIT is `debug!` only (`crates/nova_ship/src/input/player/flight_rig.rs:424-500`); long trips unverified | flown New Game trip to a station | S, low |
| Sector streaming, freeze on leave | core | explore a persistent world | fly | proven | `crates/nova_world/src/streaming.rs`; `system_world_sectors`; retire and thaw 0.00 m `tasks/20261012-095958/TASK.md:33` | repetition and empty spawns unverified (50 km lattice, `crates/nova_world_base/src/clusters.rs:9-11`) | three-seed spawn frames (RESEARCH P-C) | proof S |
| Mining, ore canisters | core | earn ore | lock rock, mining key (no HUD chip) | usable with gaps | `crates/nova_scenario/src/mining.rs:452`; refusal cue 7/7 voices `RESEARCH.md` s8; `OnMined` `system_hold_events` | no MINE chip in dock (`crates/nova_hud/src/keybind_dock.rs:87-97`); refusal heard by nobody | New Game mine-pickup-sell bench run (P-B) | chip S |
| Cargo intake, jettison, inventory, ammo | core | collect and carry | intake door, TAB Inventory | usable with gaps | `crates/nova_gameplay/src/inventory.rs:754,1174`; door fix 14 tests `tasks/20261009-184234/TASK.md:23-24` | flown pickup never recorded; quantity field has no bench target | P-B | proof S |
| Trade and credits | core | sell ore, buy parts and ammo | docked, TAB Inventory Buy/Sell | usable with gaps | `inventory.rs:839,905`; refusals named `crates/nova_interface/src/inventory/app.rs:1711-1762`; `OnTraded` `system_hold_events` | trader status only visible after dock; no New Game sale recorded | P-B | proof S |
| Station storage | secondary | stash items | docked at living station, Inventory Storage column | proven | `inventory.rs:971`; `system_station_storage` (retire, thaw, save, load) `tasks/20261012-100005/TASK.md:13`; round trip pass | storage cap and use case unclear to a player, unverified | `system_station_storage` | low |
| Plate repair | core | heal sections anywhere | TAB Ship | usable with gaps | `inventory.rs:1243`; `crates/nova_interface/src/ship/sections.rs:453-470` | no in-flight hull readout tells the player to repair (`crates/nova_ship/src/ship_audio/cues.rs:275`) | combat capture with hull deltas | see readout |
| Section rebuild | core | recover a lost section | docked at station with Rebuild, TAB Ship lost list | proven (fixture loss) | `inventory.rs:1403`; `crates/nova_interface/src/ship/rebuild.rs:164-178`; `system_station_rebuild`; round trip passed twice `tasks/20261012-095958/TASK.md:33` | lost list shows only when docked; Refit and Strip absent | round trip | low |
| Generated stations, docking admission, helm | core hub | find a safe service point | Map contacts, DOCK verb, `STATION HELM` chip | proven (teleported approach) | `crates/nova_world_base/src/sector_stations.rs:66-126`; `crates/nova_ship/src/sections/docking_section/admission.rs:46`; refusal lines `crates/nova_ship/src/input/player/hints.rs:109`; `system_generated_stations` seed 6; `system_station_docking` flown | distance from spawn to the nearest station unknown (centroids 160 km apart, `sector_stations.rs:18-19`), unverified; nothing points a new player at one | flown spawn-to-dock run | proof M |
| Chart layer | supporting | remember where things are | Map `C`, footer Chart button | proven | `crates/nova_gameplay/src/chart.rs:107`; `crates/nova_interface/src/map/chart.rs`; live pass `tasks/20261012-100007/TASK.md:25` | panel text low contrast, raw item ids; wreck marks unproven live | `system_world_chart` | S, low |
| Wreck cargo | secondary | loot a kill | canisters from destroyed ship or section | usable with gaps | `crates/nova_scenario/src/wreck_cargo.rs:61`; unit and serde tests only `tasks/20261012-100006/TASK.md:14` | no live run; drop at about one canister per 5.5 s (owner review) | live kill-and-pickup example | S |
| Weapons, point defense, AI fights | core | survive and win fights | weapon keys, locks, HUD | usable with gaps | `crates/nova_ship/src/sections/`; `system_ai_combat`, `system_turret_gunnery`, `stress_point_defense` | no player integrity readout; neutralized player drifts with no outcome (`RESEARCH.md` s9.2 B) | combat capture with hull deltas | readout M |
| Station damage, AI, collapse | secondary | fight or defend a station | lock and fire; docked guns hold | usable with gaps | `crates/nova_ship/src/input/ai/passive.rs`; `crates/nova_ship/src/input/point_defense/assignment.rs`; sever tests and orbit range pass `tasks/20261013-133627/TASK.md:20` | one torpedo destroys the waystation (3490 to 61 hp); no turret-firing capture; no player-vs-station fight | player fight capture | tuning S, risk mid |
| Hull skin, station parts look | supporting | read ships and stations | visual | proven (screenshots reviewed) | `18a4f9ae8`, `53c5a186a`; reviews `tasks/20261013-133631/TASK.md:24`, `tasks/20261013-133637/TASK.md:26` | small chamfer sliver, unclad gunship turret side; gunship armoured intake no longer places | `screenshot_station_showcase` | low |
| HUD, TAB panes, training | supporting | learn and read state | HUD dock, TAB, Basic Training | usable with gaps | `crates/nova_hud/src/keybind_dock.rs:87-97`; `system_training_journey`; bench tutorial 9/10 | New Game does not point at training or TAB (`crates/nova_menu/src/menu_ui.rs:531`); dock hides refused verbs | P-C frames | S-M |
| Scenarios, story, harness on world | secondary | play authored chapters | Scenarios picker | proven | world block `bd1838e75`; resume `tasks/20261012-095956/TASK.md:17`; `system_world_bounds` | handler-spawned persistent ships make the save refuse (`crates/nova_world/src/frozen.rs:795-832`) | `system_world_resume` | M, blocks jobs |
| Mods: items, stations, recipes as content | supporting | extend the game | RON bundles, lint | proven | items `crates/nova_assets/src/items.rs`; `crates/nova_scenario/src/lint/station.rs`; lint 0 errors per close notes | many breaking format changes this release; `skin` defaults `Clad` | content lint | low |
| Sandbox, ship editor | supporting | build ships and stations | Menu > Sandbox | usable with gaps | `system_ship_editor`; station mark in progress (task `20261013-133646`) | station preview drew as a unit cell (`tasks/20261012-095959/TASK.md:18`) | `system_sandbox_station` | in flight |
| Audio, VFX | supporting | feel actions | ambient | prototype for clarity | `crates/nova_gameplay/src/audio/mod.rs`; sidecar voices only | nobody listened to new cues | listening pass | S |
| Settings, desktop, web | supporting | configure and play anywhere | Settings | usable with gaps | `crates/nova_assets/src/persist.rs`; pause-unfocused `b0c36d4dc`; `system_settings_persist` | web New Game is session-only | per-platform smoke | low |
| Gamepad, mobile pad | supporting | non-keyboard play | input source | prototype | `crates/nova_input/src/source.rs`; backlog tasks | not playable end to end | none in v0.16.0 | out of scope |

### Journeys (mechanics joined into a player outcome)

| Journey | Status | Evidence | Gap |
| --- | --- | --- | --- |
| First session: Create, spawn, learn, find a goal | unverified | no recorded New Game player run | no objective, copy and HUD do not name stations, mining or TAB |
| Mine, pick up, sell | unverified | each step proven alone | no New Game run with deltas (P-B) |
| Lose a section, dock, rebuild, save, load | proven with fixtures | `system_station_round_trip` passed twice | loss and parts are fixture; approach teleported; no `seed: None`, combat loss or orbiting pose |
| Fight, kill, loot the wreck | unverified | unit and serde tests | no live run |
| Die, Load last save | usable with gaps | focused tests, 40/40 writer regression | rendered native flow missing |
| Leave, return, restart | proven | round trip, `system_world_resume`, p6 frames | none for the mechanic |
| Explore and chart | usable with gaps | `system_world_chart` pass | wreck marks unproven live |
| Fight a station | unverified | sever tests, scripted orbit hit | no player fight |

## v0.16.0 polish shortlist (ranked)

| # | Item | Verified friction | Cost | Supports | Proof |
| --- | --- | --- | --- | --- | --- |
| 1 | Record one flown New Game journey: spawn, find station, dock, trade, rebuild, die, Load last save | code-backed: every station proof teleports; no player run since 2026-09-21 | M, no code | `100010` start role spawn, `100023` job board | bench run with audit deltas and frames |
| 2 | Point a new player at a station: fix New Game copy, show the nearest living station on the Map or chart | code-backed copy (`world_setup.rs:160-163`); distance unverified until item 1 | S | `100010`, `100031` discovery log | three-seed spawn frames |
| 3 | In-flight player integrity readout and a neutralized-player outcome | code-backed (`cues.rs:275`) | M | `100012` swap, rebuild loop | rendered combat capture with hull deltas |
| 4 | Camera mode switch zeroes the outgoing hull rate | code-backed chain, not reproduced | S | combat incl. stations | ECS repro, then fix |
| 5 | In-flight verb cues: MINE chip and refusal reasons for DOCK, GOTO, ORBIT | code-backed (`keybind_dock.rs:87-97`, `flight_rig.rs:424-500`); stations already name dock refusals | S-M | `100023`, `100024` | rendered chip frames, refusal tests |
| 6 | Chart panel legibility: contrast, item display names; prove wreck marks live | close note `tasks/20261012-100007/TASK.md:25` | S | `100031` | `system_world_chart` frame |
| 7 | Station combat tuning: waystation torpedo one-shot, armed stock, no leash; record a player fight | close notes `133627:20`, `095958:31` | S tuning, M proof | `100019` strip, `100024` neutralize | player-vs-station capture |
| 8 | Allow handler-spawned persistent ships in the world save | debt `crates/nova_world/src/frozen.rs:795-832`, `tasks/20261012-095526/TASK.md:28` | M | unblocks `100024` escort and distress, `100016` escort | world save round trip with a spawned ship |

- Supports: `100010`, `100012`, `100016`, `100019`, `100023`, `100024`, `100031`.
- Defers until the shortlist lands: `100011` curve, `100013` add section, `100014` ship catalogue, `100017` own station, `100021` tow, economy `100025`-`100029`, charter goals `100033`, stance `100034`-`100036`.
- Not core loop, owner call: greebles `20261013-133640` (95), `20261013-133643` (94) rank above this task by priority but polish presentation only.

## Open owner questions and recent friction

Owner questions from close notes:
- `20261013-133637:26`: `LinkPoint.skin` defaults to `Clad` and may be omitted, against the explicit-fields rule. Accept?
- `20261013-133631:24`: gunship armoured intake no longer places (Panel takes Flat only). Leave it?
- `20261012-095958:31`: agent values: collapse 0.5, armed station 1000 PdcRound and 20000 cr, no AI leash, sever keeps station velocity.
- `20261013-133627:20`: one Serpent torpedo destroys the waystation; the orbit range now uses a scripted hit.
- `20261012-100005:15`: `STATION_STOCK_SHARE` 0.5..1.0; dead stations keep full stock as loot.
- `20261012-100006:14`: wreck drop reuses the mining ejector (2 m/s, about one canister per 5.5 s).
- `20261013-133646` (open): sandbox station services, stock, cap and credits chosen by the agent.

Friction and debt:
- Save refuses handler-spawned persistent ships (`frozen.rs:795-832`); blocks jobs that spawn ships.
- Unreproduced: a severed dock with a despawned root may leave `DockedShip` one tick and trip `InventoryShips::pair()` (`crates/nova_interface/src/inventory/app.rs:1289`).
- Chart: no permanent proof for the Retry clear.
- Stations: turrets small in close shots, no turret-firing screenshot, wiki part images are placeholders, gallery labels say "NEW".
- Link-graph lint says `ship '<id>'` for a station hull (`20261012-095959:18`).
- Ship generator clearance treats a `Bare` face as clad (`20261013-133637:26`).
- Generated bodies: despawn, camera and ship lookups still refuse them (`20261012-095955:17`).
- Gravity integration gains about 1.5 % binding energy per orbit (`20261012-100000:22`).
- Stale comment "Nothing repairs a hull today" (`crates/nova_ship/src/ship_audio/cues.rs:264,773`).
- Unreproduced fixture risk: `docking_warship_tender` empty mining mapping (`tasks/20261007-090722/RESEARCH.md` s8).

## Done when
- Owner can review the full feature matrix and evidence-backed priority order before implementation tasks are committed to a sprint. No gameplay implementation or release promise follows from this audit.
