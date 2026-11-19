# MAP: start roles, start hulls, job mix, goals

Read-only map at `6d0f0bc69` (master), written 2026-11-19. Nothing was built
or run. Every number is read from code or content, or computed from them by
a formula given here.

Labels: `[computed]` = arithmetic on read values. `[earlier]` = measured by an
earlier task, not measured again. `[not verified]` = not read or not
derivable from static reading. `[opinion]` = a judgement, not a fact.

Stale facts in `TASK.md` "Agent findings": the kits (now 3,000 PdcRound on
four roles and 3 RepairKit on all six, `TASK.md:22-27`), and "10k-40k worth"
(`TASK.md:34`; that range is station credits, see E).

## A. Roles now

Source: builder `crates/nova_authoring/src/base_content/start_roles.rs:44-160`
(`role()` helper `:169-206`). Generated file:
`assets/base/roles/base.content.ron`. Type:
`crates/nova_scenario/src/objects/start_role.rs:200-219`.

| id | name | hull | credits | stock | jobs | goal title | goal kind |
|---|---|---|---|---|---|---|---|
| industrial | Industrial | mining_tug | 2,200 | HullPlate 15, Rations 25, RepairKit 3 | Trade | Ore to market | OreDeliveries(jobs 6) |
| scavenger | Scavenger | salvage_hauler | 1,800 | HullPlate 10, PdcRound 3000, RepairKit 3 | Salvage | A salvage run | Recoveries(jobs 6) |
| transport | Transport | light_freighter | 2,000 | Rations 50, WaterIce 50, RepairKit 3 | Trade | A station network | StationNetwork(stations 4) |
| bounty_hunter | Bounty Hunter | interceptor | 1,600 | HullPlate 10, PdcRound 3000, RepairKit 3 | Combat | A kill list | KillList(hulls 6) |
| military | Military | patrol_picket | 1,000 | HullPlate 10, PdcRound 3000, RepairKit 3, Torpedo 2 | Combat | Patrol record | Patrols(jobs 6) |
| pirate | Pirate | raider | 1,800 | HullPlate 10, PdcRound 3000, RepairKit 3 | Combat | An outlaw's name | Outlaw(hulls 6) |

Kit worth at reference ask and kit mass `[computed]` from
`assets/base/items/base.content.ron`:

| role | credits + items at ask | kit mass | hold | hold used |
|---|---|---|---|---|
| industrial | 2,200 + 600 + 200 + 360 = 3,360 | 260 kg | 2,100 kg | 12% |
| scavenger | 1,800 + 400 + 300 + 360 = 2,860 | 760 kg | 5,400 kg | 14% |
| transport | 2,000 + 400 + 600 + 360 = 3,360 | 660 kg | 3,700 kg | 18% |
| bounty_hunter | 1,600 + 400 + 300 + 360 = 2,660 | 760 kg | 3,000 kg | 25% |
| military | 1,000 + 400 + 300 + 360 + 800 = 2,860 | 1,060 kg | 1,800 kg | 59% |
| pirate | 1,800 + 400 + 300 + 360 = 2,860 | 760 kg | 2,900 kg | 26% |

The kit worth rule is a builder comment, not an owner number:
`start_roles.rs:1-27` (3,000 cr unarmed; 2,000 to 2,200 cr armed without
rounds; rounds 300 cr; kits 360 cr).

A role has no start station, stance, hostility, price or scenario field.
Other fields: `icon` (Drill, Claw, Container, Star, Shield, Blades), `about`,
`lore`. The about texts are the agent's; the owner rewrites them (H row 17).

Who reads each field at runtime:

| field | reader | path:line |
|---|---|---|
| hull, kit.stock, kit.credits | `SpawnStartRole` | `crates/nova_scenario/src/actions/spawn.rs:312-361` |
| hull | parked own hull | `crates/nova_scenario/src/objects/parked_hulls.rs:411-412` |
| hull sections | weapon key bindings | `crates/nova_scenario/src/objects/start_role.rs:128-158` |
| jobs (mix) | board draw; no session means Trade; unknown role panics | `crates/nova_world_base/src/jobs.rs:316-323` |
| goal | goal tracker | `crates/nova_world_base/src/goal.rs:194-197` |
| id, name, icon | New Game role list | `crates/nova_menu/src/world_setup.rs:454-460` |
| name, about, credits, lore, goal.title, hull | New Game role card | `crates/nova_menu/src/world_setup.rs:804-929` |
| name, icon | Load list | `crates/nova_menu/src/load_screen.rs:943-982` |
| id | save header; Load refuses an unknown role | `crates/nova_world_base/src/save/mod.rs:131-132`, `:2208` |

Lint (`crates/nova_scenario/src/lint/start_role.rs:28-70`): unknown hull, no
cargo intake, intake in the dock approach, a section above advancement 0
(`:139-161`), unknown kit item, kit over the hold, goal count 0 (`:74-88`).

## B. Start hulls

Builders: `crates/nova_authoring/src/base_content/ships/block.rs` (mining_tug
`:136-180`, salvage_hauler `:190-245`, light_freighter `:333-355`, interceptor
`:390-431`, raider `:437-485`, patrol_picket `:491-547`). Section counts are
counted from `assets/base/ships/base.content.ron`.

| hull | sections | light hull | controller | thruster | PDC | torpedo bay | mining beam | intake | dock port |
|---|---|---|---|---|---|---|---|---|---|
| mining_tug | 28 | 21 | 1 | 3 | 0 | 0 | 1 | 1 | 1 |
| salvage_hauler | 65 | 54 | 2 | 5 | 1 | 0 | 1 | 1 | 1 |
| light_freighter | 46 | 37 | 2 | 5 | 0 | 0 | 0 | 1 | 1 |
| interceptor | 38 | 30 | 1 | 3 | 2 | 0 | 0 | 1 | 1 |
| raider | 38 | 29 | 1 | 4 | 2 | 0 | 0 | 1 | 1 |
| patrol_picket | 27 | 18 | 1 | 2 | 2 | 1 | 0 | 1 | 2 |

Section facts (`assets/base/sections/base.content.ron`). All eight parts are
advancement 0.0. Worth = recipe credits + parts at ask
(`BuildRecipe::value_cr`, `crates/nova_gameplay/src/inventory.rs:1707-1714`).

| part | line | health | mass (cells) | recipe | worth `[computed]` |
|---|---|---|---|---|---|
| light_hull_section | 106-117 | 60 | 1 | 80 cr + SectionPanel 2 | 120 |
| basic_thruster | 521-558 | 70 | 1 | 60 cr + DriveBell 1 + SectionPanel 2 | 260 |
| basic_controller | 1193-1295 | 100 | 1 | 100 cr + FlightCore 1 + SectionPanel 2 | 540 |
| pdc_kinetic_turret | 1410-1601 | 130 | 0.125 | 100 cr + ActuatorPack 1 + SectionPanel 2 + TurretBarrel 1 + TurretBody 1 | 480 |
| torpedo_section | 2216-2421 | 100 | 2 | 150 cr + ActuatorPack 1 + FrameSpar 1 + SectionPanel 4 + TorpedoTube 1 | 610 |
| docking_port | 3330-3440 | 90 | 1 | 80 cr + ActuatorPack 1 + DockingCollar 1 + SectionPanel 2 | 400 |
| cargo_intake | 3561-3574 | 90 | 6 | 60 cr + ActuatorPack 2 + FrameSpar 1 + SectionPanel 4 | 360 |
| mining_beam | 4094-4225 | 90 | 1 | 100 cr + ActuatorPack 1 + CapacitorBank 1 + SectionPanel 2 | 460 |

Formulas:

- Mass = collider volume at density 1; default collider is a unit cube
  (`crates/nova_ship/src/sections/ship_measure.rs:29-41`,
  `crates/nova_ship/src/sections/base_section.rs:84-90`).
- Acceleration = 64 x thrusters x magnitude / mass, in cells/s^2; x10 for
  m/s^2. Magnitude is an impulse per 64 Hz tick and is 1.0
  (`crates/nova_ship/src/sections/thruster_section.rs:41`, `:482-500`). It
  assumes every thruster pushes forward `[not verified]`.
- Hold = 100 kg per Hull-kind section
  (`crates/nova_scenario/src/objects/ship_design.rs:413`, `:422-433`).
- Hull price = sum of section worth
  (`crates/nova_ship/src/sections/hull_buy.rs:110-120`).
- Health = sum of section health. No collapse rule was read `[not verified]`.

| hull | mass | accel m/s^2 | thrust per cell (floor 0.06) | health sum | hold kg | hull worth cr |
|---|---|---|---|---|---|---|
| mining_tug | 33.0 | 58 | 0.091 | 1,840 | 2,100 | 5,060 |
| salvage_hauler | 69.1 | 46 | 0.072 | 4,190 | 5,400 | 10,560 |
| light_freighter | 51.0 | 63 | 0.098 | 2,950 | 3,700 | 7,580 |
| interceptor | 41.25 | 47 | 0.073 | 2,550 | 3,000 | 6,640 |
| raider | 41.25 | 62 | 0.097 | 2,560 | 2,900 | 6,780 |
| patrol_picket | 31.25 | 41 | 0.064 | 1,950 | 1,800 | 5,950 |

All `[computed]`. The floor is `MIN_THRUST_PER_CELL`
(`ship_measure.rs:23`). `examples/systems/system_start_roles.rs:721-738` logs
the true hull and kit worth per role; it was not run. An older note gives a
spread of 9,980 to 23,920 cr (`tasks/20261012-100010/TASK.md:59`); the part
recipes changed after it, so it is stale.

Not derived: turn rate (controller `max_torque 9760`, `steering_lag 0.5`,
`sections/base.content.ron:1294-1295`; needs inertia), any speed cap (none
found for a player hull; `FlightCruiseCap` is for convoy hulls,
`crates/nova_ship/src/flight/state.rs:100`), whether cargo adds mass.

Weapons:

| fact | value | source |
|---|---|---|
| PDC fire rate, damage | 100 rounds/s x 4 = 400 raw damage/s per mount | `sections/base.content.ron:1576`, `:1589-1592` |
| PDC nominal range | 1,000 units/s x 2 s = 20 km | same block |
| PDC magazine | 500 rounds = 5 s of fire | `:1597-1601` |
| PDC reload | 200 rounds per 3 s with no shot, from the hold | `:1597-1601`, `crates/nova_ship/src/sections/ammo.rs:13-22` |
| Fire time, 2-PDC hull | (1,000 + 3,000) / 200 = 20 s `[computed]` | kit + magazine |
| Fire time, salvage_hauler | (500 + 3,000) / 100 = 35 s `[computed]` | kit + magazine |
| Torpedo bay | 1 shot/s, blast 750 in 300 units, magazine 6, reload 1 per 10 s | `:2389-2421` |
| Military torpedoes | 6 in the bay if a magazine starts full, + 2 in the kit | `ammo.rs:13-22` |
| Mining beam | reach 100 m, 1 pulse/s, cut radius 1.5 field cells, 1 ore per corner cut | `:4223-4225`, `crates/nova_scenario/src/mining.rs:17`, `:157-167` |

Hit rate, damage falloff and time to kill are `[not verified]`. On paper one
PDC mount removes a 60 HP light hull section in 0.15 s of hits `[computed]`.

Repair and rebuild:

| lever | rule | source |
|---|---|---|
| HullPlate | +20 HP per plate, 4 s per plate, in flight | `crates/nova_gameplay/src/inventory.rs:1572`, `:1578` |
| RepairKit | +80 HP over 10 s, most damaged section first | `assets/base/consumables/base.content.ron`, `crates/nova_authoring/src/base_content/consumables.rs:14-23` |
| Field rebuild | recipe parts from the hold, no credits, 5 s per part, min 20 s | `inventory.rs:1581-1593`, `:2064-2069` |
| Ship recipes | 100 PdcRound = IronOre 1 + StoneOre 1 (8 s); HullPlate = IronOre 2; RepairKit = HullPlate 2 + SectionPanel 1 | `crates/nova_authoring/src/base_content/recipes.rs:44-80` |
| Kit repair total | industrial 15 x 20 + 240 = 540 HP; armed roles 200 + 240 = 440 HP; transport 240 HP `[computed]` | kits |

No kit holds a recipe part, so no start hull can field-rebuild a lost
section from its kit `[computed]`.

A start station (advancement at most 0.15,
`crates/nova_world_base/src/civilizations.rs:61-67`) offers Rebuild, Refit,
Trade, Storage and Jobs. Craft opens at 0.25
(`crates/nova_world_base/src/station_layout.rs:146-161`). It keeps at least
4,000 PDC rounds when its civilization knows the PDC (`:136`). Refit adds a
part at a free socket for its recipe
(`crates/nova_gameplay/src/markers.rs:185-188`). So an unarmed start can add
a PDC for 480 cr at ask `[computed]`, if the hull has a free socket
`[not verified]`. The reinforced hull opens at 0.3 and the railgun at 0.5
(`sections/base.content.ron:4-16`, `:2847-2860`): neither is on sale at a
start. Hull buy at advancement 0.0 `[earlier]`: median 5,360 (Scavenger),
5,480 (Civilian), 11,340 (Industrial), 13,500 (Armored)
(`tasks/20261116-132141/MAP.md` section H).

## C. Job mix and the board

Constants in `crates/nova_gameplay/src/jobs.rs`:

| fact | value | line |
|---|---|---|
| slots per station | 3 | 107 |
| mix weights, order Procure, Recover, Escort, Distress, Neutralize, Supply | Trade 4,1,4,1,1,1 (12); Salvage 1,5,1,2,1,2 (12); Combat 1,1,1,3,4,3 (13) | 1030-1036 |
| draw | per-slot WyRand: seed, station, slot, posting | 2497-2519 |
| fallback to Procure | Recover with no wreck, Neutralize with no hostile, Supply with no pool | 2376-2381 |
| hull gate | Procure: hold. Supply: hold and clamp. Recover, Distress: clamp. Escort, Neutralize: weapon | 2829-2864 |
| starter board | hides a refused job; one fitting offer is assured | 2818-2822, 2428-2439 |
| reward | base x A + fee + goods + range x A + threat; A = 1 + 2 x advancement | 1348-1385 |
| bases | Procure 0, Recover 100, Escort 200, Distress 200, Supply 200, Neutralize 300 | 132-140 |
| fee | 50 to 250 cr | 125-127 |
| range, threat | 30 cr per km x A; 300 cr per weapon | 143, 146 |
| legs | 6 to 16 km; escort 10 to 16 km | 154, 160 |
| escort raiders below 0.25 | 1 raider, budget 40 sections, 1 turret | 183-213 |
| paid refresh | 100 cr | 3109 |

Offer life: 1,200 s, escort 3,600 s
(`assets/base/market/base.content.ron:26-27`). A slot renews when its job is
handed in, on a paid refresh, or on expiry
(`crates/nova_world_base/src/jobs.rs:534-571`).

Hull gate at a start: every start hull has a docking port and a hold above
the largest Procure stack (8 x 80 kg = 640 kg `[computed]`). So the only
refusals are Escort and Neutralize for the two unarmed hulls.

No hostile stands near a start. A hostile is an intact hull with
`Allegiance::Enemy` in the 27-cell cube
(`crates/nova_world_base/src/jobs.rs:656-717`). The Neutralize example
plants one, because "generated hostiles stand with their own Enemy
civilization, so no scanned seed put one in the cube of a living friendly
station near the spawn" (`examples/systems/shared/located_jobs.rs:14-17`).
So at a start board a Neutralize draw posts a Procure. Wreck presence near a
start is `[not verified]`.

Effective kind share per drawn slot at a start (no hostile) `[computed]`:

| mix | Procure | Recover | Escort | Distress | Supply | Neutralize |
|---|---|---|---|---|---|---|
| Trade, wreck in cube | 5/12 | 1/12 | 4/12 | 1/12 | 1/12 | 0 |
| Trade, no wreck | 6/12 | 0 | 4/12 | 1/12 | 1/12 | 0 |
| Salvage, wreck in cube | 2/12 | 5/12 | 1/12 | 2/12 | 2/12 | 0 |
| Salvage, no wreck | 7/12 | 0 | 1/12 | 2/12 | 2/12 | 0 |
| Combat, wreck in cube | 5/13 | 1/13 | 1/13 | 3/13 | 3/13 | 0 |
| Combat, no wreck | 6/13 | 0 | 1/13 | 3/13 | 3/13 | 0 |

Pay at advancement 0.0, min / median / max
(`tasks/20261116-132141/PAY-TABLE.md` "After" table; synthetic legs, no play
time measured): Procure 62 / 294 / 5,050; Recover 212 / 670 / 2,627; Escort
930 / 1,373 / 1,810; Distress 430 / 690 / 930; Supply 460 / 890 / 2,250;
Neutralize 413 / 1,073 / 2,788. At 0.15: Procure same; Recover 269 / 942 /
4,369; Escort 1,080 / 1,544 / 2,014; Distress 544 / 850 / 1,134; Supply 574
/ 1,050 / 2,418; Neutralize 522 / 1,409 / 3,895.

Per role at the starter board, advancement 0.0, no hostile `[computed]`.
"Per row" is the share-weighted mean of the kind medians over fitting rows.

| role | rows shown of 3 | fitting share of draws | cr per fitting row, wreck / no wreck | goal rows per draw |
|---|---|---|---|---|
| industrial | 2.0 | 8/12 | 465 / 418 | Raw Procure only: 0 to 6/12 by cell type |
| transport | 2.0 | 8/12 | 465 / 418 | Procure 5/12 or 6/12; Escort 4/12 does not fit |
| scavenger | 3.0 | 12/12 | 706 / 549 | Recover 5/12; 0 with no wreck |
| bounty_hunter | 3.0 | 13/13 | 635 / 606 | 0 (no hostile) |
| military | 3.0 | 13/13 | 635 / 606 | 7/13 |
| pirate | 3.0 | 13/13 | 635 / 606 | 0 (no job kind counts) |

A Procure or Supply reward includes its goods at 1.5 x ask
(`jobs.rs:1348-1385`); the player supplies the stack. The Procure pool does
not exclude items the poster stocks or the kit holds (`jobs.rs:2329-2356`).

Procure pool by cell (`wanted_categories`, `jobs.rs:2221-2253`; categories
from `assets/base/items/base.content.ron`) `[computed]`:

| cell | categories | items | Raw share |
|---|---|---|---|
| no cluster | Raw, Food, Parts, Repair | 20 | 20% |
| AsteroidRich, RockOnly | Raw | 4 | 100% |
| PlanetHeavy | Food, Raw | 5 | 80% |
| DerelictField | Parts, Repair | 15 | 0% |

## D. First payout per role, on paper

Assumptions: the player starts docked at the poster
(`crates/nova_world_base/src/start.rs:154-173`). Flight is an ideal
flip-and-burn, t = 2 x sqrt(d / a), full thrust, no turn time
`[computed]`. Manual docking time is unknown: two earlier bench plays never
completed a redock in 711 s and 819 s of game time (G). So time is given as
flight seconds plus a docking count.

One-way flight seconds `[computed]`:

| hull | 6 km | 12 km | 16 km |
|---|---|---|---|
| mining_tug | 20 | 29 | 33 |
| salvage_hauler | 23 | 32 | 37 |
| light_freighter | 20 | 28 | 32 |
| interceptor | 23 | 32 | 37 |
| raider | 20 | 28 | 32 |
| patrol_picket | 24 | 34 | 40 |

The spread between hulls is at most 8 s per leg. Hull thrust does not
decide the first payout `[opinion]`.

| role | most likely first job | steps | flight | docks | combat | goods | reward median | main risk |
|---|---|---|---|---|---|---|---|---|
| industrial | Procure (5/8 of rows) | Accept. Hand in from the kit (Rations, HullPlate, RepairKit) or buy at the poster. Else cut ore within 8 km and redock | 0 to 2 x 23 s | 0 or 1 | none | kit, or stack at ask | 294 | item not in kit or stock; only a Raw item counts for the goal |
| transport | Procure (5/8 of rows) | Accept. Hand in from the kit (Rations, WaterIce, RepairKit) or buy at the poster | 0, or a trip to another station | 0 or more | none | kit, or stack at ask | 294 | item not stocked; no mining beam |
| scavenger | Recover (5/12) if a wreck stands | Fly to the wreck, dock, take the stack, undock, fly back, dock | 2 legs; leg read off the wreck, inside a 96 km cube | 2 | none posted | none; keeps the stack | 670 | no wreck: 7/12 rows are Procure |
| bounty_hunter | Procure (5/13), Distress (3/13) or Supply (3/13) | Distress: fly 6 to 16 km, dock with the caller, fly back, dock | 2 x 23 to 37 s | 2 | none | Supply: stack at ask | 294 / 690 / 890 | no bounty posts |
| military | same board as bounty_hunter | same | 2 x 24 to 40 s | 2 | none | same | 294 / 690 / 890 | 1,000 cr; hold 59% full |
| pirate | same board as bounty_hunter | same | 2 x 20 to 32 s | 2 | none | same | 294 / 690 / 890 | no job matches the role |

Escort (1/13 on Combat, 1/12 on Salvage): rendezvous at 1 km, hauler at
100 m/s over 10 to 16 km = 100 to 160 s, one raider with one turret at 75%
of the leg, engage range 3 km, 1 dock, median 1,373
(`crates/nova_world_base/src/jobs.rs:106-140`).

A hand-in adds 50 cr once (`FIRST_DELIVERY_BONUS_CR`,
`crates/nova_authoring/src/base_content/scenarios/open_world.rs:46`).

Whether a kit item can be handed in the moment a Procure is accepted was
derived from `plan_job_hand_in` (`jobs.rs:3050-3103`), not run.

## E. Goals

System: `crates/nova_world_base/src/goal.rs`. Line one is shared and in code:
"Stations owned" 1 (`:34-37`). Line two is the role's kind (`:61-174`).
Progress is derived each frame and not saved; `GoalRecord.met_at` is stamped
once and saved (`:182-230`). A met goal is "an optional milestone, not an
ending" (`:20-21`): nothing else happens.

| kind | counts | label | line |
|---|---|---|---|
| OreDeliveries | handed-in Procure of a `Raw` item | Ore deliveries | 70-91 |
| Recoveries | handed-in Recover | Wrecks recovered | 92-107 |
| StationNetwork | distinct posters of handed-in Procure or Escort | Stations served | 108-123 |
| KillList | distinct hulls of handed-in Neutralize | Bounties handed in | 124-142 |
| Patrols | handed-in Distress, Supply or Escort | Patrols flown | 143-156 |
| Outlaw | 1 civilization turned Enemy by the player; hulls taken while it is Enemy | Civilizations turned Enemy; Hulls taken while Enemy | 157-168 |

"Own one station": price = 2 x (sections + stock + credits); only an Ally
station sells; "An owned station earns nothing"
(`crates/nova_ship/src/sections/station_buy.rs:3-4`, `:14`, `:55-82`). At
advancement 0.0 `[earlier]`: 48,256 to 60,824 cr, median 54,536
(`tasks/20261116-132141/MAP.md:420`). At the per-row medians of C that is
77 (scavenger) to 117 (unarmed) hand-ins `[computed]`.

A station at advancement 0 holds 10,000 cr
(`crates/nova_world_base/src/station_layout.rs:142-144`, `:599-604`). A
hand-in is refused when the poster cannot pay (`jobs.rs:3087-3092`). That is
14 to 24 median jobs `[computed]`. Credits refill only on a thaw
(`crates/nova_world_base/src/economy.rs:98-130`).

UI: the role card lists only the goal title
(`crates/nova_menu/src/world_setup.rs:792-825`). The Jobs pane row shows the
title and the first unmet line (`crates/nova_interface/src/jobs.rs:1555-1600`;
`crates/nova_gameplay/src/open_objectives.rs:72-75`). The first unmet line
is "Stations owned 0 / 1", so the flavour count is not shown until a station
is owned `[computed]`. The Map reads the same resources
(`crates/nova_interface/src/map/chart.rs:1378-1495`; its layout was not
read). Save: `WorldSaveState.goal` (`crates/nova_world_base/src/save/mod.rs:331-332`).

Hail jobs: an encounter hail is a Distress or Supply job. `pay_hail` calls
`jobs.complete` (`crates/nova_world_base/src/encounters.rs:925-950`), and
the goal reads completed jobs by kind. So a hail counts for Patrols only.

Per role:

| role | counts | does not count | mismatch |
|---|---|---|---|
| industrial | Procure of StoneOre, IronOre, WaterIce, CarbonOre | ore sold at a market; Procure of any other item; all other kinds | The about text says "Cut ore from rocks and sell it"; selling counts nothing. Raw share is 0% in a derelict-field cell |
| scavenger | Recover | stripping or looting a wreck with no job | Needs a wreck in the cube per job |
| transport | distinct stations by Procure or Escort | repeat jobs at one station; trade with no job | Escort (4/12 of the mix) does not fit the hull. About one station per 160 km window (`crates/nova_world_base/src/clusters.rs:238-242`) |
| bounty_hunter | distinct Neutralize targets | escort raiders, ambush raiders, any kill with no job | Neutralize does not post near a start |
| military | Distress, Supply, Escort, and hail jobs | Neutralize, Procure, Recover | 7/13 of draws count. The top mix weight (Neutralize 4) does not count |
| pirate | kills of intact generated hulls of a civilization the player turned Enemy | every job; escort and ambush raiders (no `TraderMarket`, `crates/nova_world_base/src/jobs.rs:1059-1096`); hulls of a seeded Enemy (`crates/nova_gameplay/src/civilization.rs:514-538`) | The Combat mix feeds nothing |

Pirate cost in kills: 3 kills in 3,600 s step a civilization down one stance
(`civilization.rs:128`, `:133`). Neutral needs 3, Ally needs 6. The flipping
kill does not count (`crates/nova_world_base/src/stance.rs:84-91`). Then 6
more: 9 to 12 kills of trading hulls `[computed]`. A step out of Enemy
zeroes the count (`civilization.rs:371-374`); Enemy decays after 7,200 s
(`:147`). The nearest such hulls are the start civilization's own traffic,
2 to 4 km from the start station (`clusters.rs:242-251`). Three kills there
end the Ally stance, and only an Ally station sells itself. So the two goal
lines work against each other at the start `[computed]`.

Jobs to the first flavour step (count 1 of 6), if the player takes only
counting rows `[computed]`: military 1 job; scavenger 1 with a wreck;
industrial 1 in a Raw cell; transport 1; bounty_hunter none near a start;
pirate 3 to 6 kills before the first counted hull. Minutes are
`[not verified]`.

## F. Other role levers

| lever | state now | source |
|---|---|---|
| start station | nearest-to-origin station of a living Ally civilization in the 3x3x3 lattice block; same rule for every role | `crates/nova_world_base/src/start.rs:1-12`, `:154-173` |
| stance at start | seeded graph, 25% Ally and 25% Enemy edges; no role term | `crates/nova_world_base/src/civilizations.rs:96-100` |
| pirate hostile-on-start | none; no role-keyed stance rule exists | `start_roles.rs:44-160`; H row 5 |
| merit | 3 hand-ins at a civilization's stations step it up | `crates/nova_gameplay/src/civilization.rs:136` |
| fine to settle | 1,000 to 10,000 cr | `crates/nova_world_base/src/stance.rs:55` |
| scenario lines | role-agnostic: "Take a job", first JobDone, first hand-in (+50 cr), job lost | `crates/nova_authoring/src/base_content/scenarios/open_world.rs:60-164` |
| tutorial | separate entry; flies `training_picket`, not a start hull | `ships/block.rs:553-594` |
| death | `OnDestroyed` -> Defeat; Load last save or Main Menu | `open_world.rs:22-26`, `:101-110` |
| save points | first arm, each sector crossing, leave | `crates/nova_world_base/src/save/session.rs:1-3`; `web/src/wiki/getting-started.md:59` |
| SOS rescue | 60 s, 200 cr + rebuild recipes, pays what the balance has, never refuses; nearest Ally or Neutral station | `crates/nova_gameplay/src/rescue.rs:25-29`, `:119-150`; `crates/nova_world_base/src/rescue.rs:1-11` |
| encounters | gap 300 to 900 s of travel above 50 m/s; hail offer 300 s | `crates/nova_gameplay/src/encounter.rs:58-64`; `crates/nova_world_base/src/encounters.rs:74-93` |
| encounter weights | distress 3, supply 2, derelict find 3 for any hull; ambush 2 for an armed hull only | `crates/nova_authoring/src/base_content/encounters.rs:11-47` |
| ambush size | escort raider count and budget at the local advancement | `crates/nova_world_base/src/encounters.rs:526-545` |
| repair kit, field rebuild | see B | |

So an unarmed role never meets an ambush. An armed role draws one in 2 of
10 encounters `[computed]`.

## G. Measurement options

Collected by a helper agent from files; nothing was run.

Bench in practice:

- `bench play --session new-game --world-seed <u32> --seed <u64> --agent pi
  --goal ... --ticks --turns --deadline --out`
  (`crates/nova_bench/src/cli.rs:129-169`). 60 ticks = 1 sim second (`:83-91`).
- `baseline` is refused for New Game (`cli.rs:244-246`). Only `pi` (LLM) or
  `cmd:<argv>` can play it. No scripted pilot ships.
- No role flag. The opening prompt says "Leave the start role it selects
  first" (`crates/nova_bench/src/manual.rs:87-88`). Role buttons are named
  targets (`crates/nova_menu/src/world_setup.rs:491`); a `--goal` text could
  ask for a click. That is `[not verified]` in play.
- The pilot has no jobs block and no jobs page
  (`crates/nova_bench/src/manual.md:16-17`). It clicks `JobAccept-<i>` and
  `JobHandIn-<i>` (`crates/nova_interface/src/jobs.rs:125-127`) and cannot
  read a job title or reward.
- `score.json` has no credits, jobs or role
  (`crates/nova_bench/src/score.rs:35-75`). Credits come from `audit.jsonl`;
  job lines from `game.log`. A Defeat ends the run, so deaths are 0 or 1.

Earlier runs:

- `bench-runs/` holds one run, the tutorial
  (`bench-runs/34dd96176/tutorial/pi-gpt-5.6-sol-medium-20260921-080202`).
- `~/Videos/nova-bench-*` does not exist.
- Two New Game plays exist, both on the first role (industrial):
  seed 22, 711 s game, 1,136 s wall, 154 turns, 10.82 USD, one job accepted,
  no hand-in; seed 7, 819 s game, 998 s wall, 6.27 USD, gave up on the redock
  (`tasks/20261026-003723/TASK.md:29-30`;
  `tasks/20261026-003723/evidence/visits-20261102/bench-seed22/`).
- No play has reached a payout. No other role has been played.

Cost: a play without `--record` needs no GPU. Stepping ran at 1.8x real
time. A 15-minute play is about 24 min wall and about 14 USD `[computed
from the seed 22 audit]`. Six roles, one sample each: about 2.5 h and about
85 USD. One game at a time.

Cheaper options in the tree:

| example | role, seed | gives | no flight? |
|---|---|---|---|
| `system_start_roles` | all six, seed 10 | hull cr, kit cr, starter jobs each hull accepts (`examples/systems/system_start_roles.rs:360`, `:721-738`) | job stamped done in place |
| `system_station_jobs` | transport, seed 10 | paid procurement | berth teleport |
| `system_job_recover` / `_distress` / `_supply` / `_neutralize` | scavenger or bounty_hunter, pinned seeds | paid hand-in per kind | teleports; hostile planted |
| `system_job_escort`, `system_job_ammo_economics` | military, seed 37 | real flight and fight, play seconds, ammo ledger | fails on lavapipe at 1.6 fps (`tasks/20261111-122540/TASK.md:137`) |
| `system_encounters` | bounty_hunter, seed 17 | paid hail job | travel clock written forward |
| `system_role_goals` | bounty_hunter, seed 10 | goal lines; six jobs injected, 100,000,000 cr | no flight |
| `job_board_sandbox` | `NOVA_SANDBOX_ROLE` | starter board by hull fit | hand play |

Industrial and pirate have no job walk. No real-content board census
exists: no file in `crates/nova_authoring/tests/` calls `station_job_offers`.

| option | evidence | build work | wall per role | risk |
|---|---|---|---|---|
| A. Pure board census per role: real items, each start hull's fit, N seeds, real field | share of fitting rows, goal rows, pay spread, "no bounty" rate | one test or one-off in `nova_authoring` | seconds | static; permanent proof needs owner approval |
| B. `system_start_roles` as is | hull cr, kit cr, accepted starter jobs, seed 10 | none | one run for six roles; unmeasured | one seed; no payout |
| C. Bench New Game, `pi`, role named in `--goal` | credits at 15 min, first hand-in, death 0 or 1, stall point | none; role click unverified | about 24 min, about 14 USD | pilot skill hides role difficulty; n = 1 |
| D. Bench + `--start-role`, role in score, jobs block, jobs page | as C, role verified, jobs readable | changes in `nova_bench`, snapshot, manual | as C | still pilot-bound |
| E. Bench with a scripted `cmd:` pilot | deterministic time to payout, 1.8x real time, no LLM cost | large: lock, GOTO, manual dock, pane clicks | about 8 min | the script becomes the measure |
| F. Per-role autopilot walk on `located_jobs` | reward per kind per role; real fight on escort | one example per role | unmeasured; escort needs the GPU | teleports hide travel |

Recommendation: A and B first. They are the cheapest evidence that answers
C and E of this map with real seeds, in seconds to minutes, at no LLM cost.
Then C on the two roles that A ranks easiest and hardest, about 48 min and
about 28 USD. The task asks for a bench table per role
(`TASK.md:44-45`); with no role flag, no jobs view and no payout in any
earlier play, six bench plays today would mostly measure the pilot
`[opinion]`. Option D is the smallest change that makes the task's own
metric readable.

## H. Prior decisions and owner statements

Collected by a helper agent. I re-read rows 1, 14, 15, 17 and the task
lines; the other quotes are as the helper reported them.
O = owner, D = decided on the owner-reviewed design page, A = agent call,
C = code only.

| # | src | statement | path:line |
|---|---|---|---|
| 1 | O | "The journey of each start role is very different: some are very easy, some are too hard and feel meh." "The start ships are not fun and need tuning." "Role goals are 'kind of meh'." | `tasks/20261116-132143/TASK.md:11-14` |
| 2 | O | "getting started is hard, the learning curve is steep ... Intended challenge must remain" ; keep docking manual | `tasks/20261025-214714/TASK.md:8`, `:11` |
| 3 | D | Six start roles, "each a small hull and kit at inner-ring advancement; roles are content" | `tasks/20261012-095526/DESIGN.html:906` |
| 4 | O | "the six roles are approved"; a role has no side; the kit is checked by the stock lint | `tasks/20261012-100010/TASK.md:36` |
| 5 | D | "relations start as for any other role ... a pirate is a playstyle, not a side." Rejected: draw bias, all-Enemy start | `tasks/20261012-095526/DESIGN.html:908` |
| 6 | O | "one start hull per role that makes sense for that role" | `tasks/20261012-100010/TASK.md:37` |
| 7 | O | patrol_picket: "Grow it to about 10,000 cr of sections. It stays the lightest armed start." | `tasks/20261012-100010/TASK.md:42` |
| 8 | O | Every open-world ship has a cargo intake; lint refuses one without | `tasks/20261012-100010/TASK.md:43` |
| 9 | O | Start docked at "the closest station of an allied civilization"; it is "the only one that posts the class-specific starter jobs" | `tasks/20261012-100008/TASK.md:37-40` |
| 10 | O | "Jobs should fit the start role the player chose" | `tasks/20261012-100022/TASK.md:25` |
| 11 | O | "The goal is an easy early progression from jobs." | `tasks/20261012-100022/TASK.md:103-106` |
| 12 | A | Starter board lists only jobs the current hull can do; no timer, no radius | `tasks/20261012-100022/TASK.md:145-152` |
| 13 | O | "The current job kinds are a proof of concept; the owner has plans for them, do not get attached." | `tasks/20261012-100022/TASK.md:124-125` |
| 14 | O | "Goals must be balanced across roles. 'Own 4 stations' against 'own 2' is not acceptable. One owned station is the shared first milestone for every role ... After it each role gets one simple flavour goal ... Keep them simple." | `tasks/20261012-100030/TASK.md:114-119` |
| 15 | O | "The `3 / 8` charter counts read as nonsense to the owner. Show the current objectives (held jobs, the next milestone) instead of goal counts" | `tasks/20261012-100030/TASK.md:109-111` |
| 16 | O | A charter is "a finite optional milestone ... never an ending"; "No world refresh" | `tasks/20261012-100033/TASK.md:41-42`; `tasks/20261012-100030/TASK.md:9` |
| 17 | O | "Role descriptions are bad; the owner rewrites them. Do not spend time on them." Restated: "Owner writes the descriptions." | `tasks/20261012-100030/TASK.md:112-113`; `tasks/20261116-132143/TASK.md:48` |
| 18 | O | New Game keeps two panels: world preview, then ship details | `tasks/20261012-100030/TASK.md:126-128` |
| 19 | D | "Keep Load last save; OnDestroyed stays the only run-ending event; no player Neutralized state, no rebuy, no tow in v1." | `tasks/20261012-095526/DESIGN.html:918` |
| 20 | O+A | SOS: timed call, 200 cr + rebuild price, credits stop at zero | `tasks/20261116-132138/TASK.md:43-57` |
| 21 | O | "rebuilding in the field is now wanted"; parts from the hold, no credits | `tasks/20261116-132137/TASK.md:18-20`, `:71-75` |
| 22 | O | Stations keep 4,000 PDC rounds; start kits carry 3,000; craft ammo from ore. No regen, no cheaper rounds | `tasks/20261116-132139/TASK.md:24-29` |
| 23 | O | "Keep only the repair kit." | `tasks/20261116-132140/TASK.md:93-95` |
| 24 | O | One scenario objective "Take a job". "No forced pane switch, no tutorial, no auto-accept, no reward" | `tasks/20261026-003807/TASK.md:28` |
| 25 | O | "Agrees that jobs are boring because rewards are flat." | `tasks/20261116-132141/TASK.md:11` |
| 26 | O | No sentences in UI; counts, dimming and disabled buttons carry state | memory `ui-imply-state-no-prose.md` |
| 27 | O | Camera clean and minimal; real pain is "fragility after section loss, ammo, flat jobs, start roles" | memory `game-feel-owner-preferences.md` |
| 28 | C | Start hulls use advancement-0 parts only; lint refuses more | `ships/block.rs:9-11` |
| 29 | A | Kit worth 3,000 / 2,000-2,200 cr convention | `start_roles.rs:4-8` |

Reversed or superseded:

| what | status | path:line |
|---|---|---|
| Goal reached "flips the world" | reversed by owner: "No world refresh" | `tasks/20261012-100030/TASK.md:8-9` |
| Decision 13 goals (mining empire, own yard, Line Warship, outlive a civilization) | superseded by row 14; landed `a86ceea6e` | `tasks/20261012-095526/DESIGN.html:928`; `tasks/20261012-100033/TASK.md:58-60` |
| Pirate goal "Turn a civilization Enemy and outlive it" | replaced by `Outlaw { hulls: 6 }`; the count was an agent's start number | `tasks/20261012-100033/TASK.md:40`; `tasks/20261012-100035/RESEARCH.md:605-608` |
| Owner gate: class + timer + radius | "Owner's alternative, not taken"; hull gate shipped; reopens if the job kinds are reworked | `tasks/20261012-100022/TASK.md:153-157` |
| `FITS YOUR ROLE` badge | dropped by owner: "the class is not a fixed trait" | `tasks/20261012-100022/TASK.md:101-102` |
| Goal chip on the HUD | dropped from scope by owner | `tasks/20261012-100030/TASK.md:159-160` |
| Rebuild only docked | reversed by row 21 | `tasks/20261012-095526/DESIGN.html:887` |
| "No crafting tree" | reversed by owner 2026-11-16 | `tasks/20261116-132140/TASK.md:17-20` |
| 1,000 PDC rounds in kits | superseded by row 22; landed `adf9031b8` | `tasks/20261025-214714/TASK.md:38` |
| patrol_picket with one nose PDC | superseded by row 7; later `cb15b60a4` added the bay and removed the interceptor bay and raider railgun | `tasks/20261012-100010/TASK.md:30` |

Open, no owner decision found: whether "own one station" stays line one
(it conflicts with "one reachable first step", `TASK.md:48`); new goal kinds
and counts; the pirate job mix; equal or spread start credits; parts above
advancement 0 on a start hull; the planned job-kind rework (row 13).

## I. Change checklist

| change | touch | then |
|---|---|---|
| kit, credits, mix, goal count | `crates/nova_authoring/src/base_content/start_roles.rs` | `content -- gen`, lint; `assets/base/roles/base.content.ron` regenerates |
| hull sections | `crates/nova_authoring/src/base_content/ships/block.rs` | gen, lint; ship tests `ships/mod.rs:311`, `:359` |
| part stats, recipes, prices | `base_content/sections/*.rs`, `recipes.rs`, items builder, `market.rs` | gen, lint |
| mix weights, pay, gate | `crates/nova_gameplay/src/jobs.rs:1030-1036`, `:1348-1385`, `:2829-2864` | Rust only; no digest change |
| goal kinds | `crates/nova_scenario/src/objects/start_role.rs:243-278`, `crates/nova_world_base/src/goal.rs:61-174`, lint `:74-88` | every `match` on `RoleGoalType` |
| new role field | `StartRole` struct `:200-219`, lint, editor forms (`crates/nova_editor/src/inspect/asset.rs:315`, `:409`) | mod format break: mark **(breaking)** |

Tests and fixtures that pin role or goal values:

- `crates/nova_scenario/src/lint/start_role.rs:187-284` (goal kinds and counts).
- `crates/nova_world_base/src/goal.rs:343-540` (kinds with 6 / 6 / 4 / 6 / 6 / 6).
- `crates/nova_world_base/src/stance.rs:774` (`Outlaw { hulls: 6 }`).
- `crates/nova_world_base/src/test_support.rs:46-60`,
  `crates/nova_menu/src/tests/support.rs:69`,
  `crates/nova_editor/src/asset/tests.rs:1373-1382`.
- `crates/nova_gameplay/src/jobs.rs:3297-3430` (mix weights over 60 seeds).
- `crates/nova_authoring/tests/content_ron_parity.rs`, `content_lint_gate.rs`.

Examples that name roles, hulls or goals: `examples/systems/system_start_roles.rs`
(asserts kit credits `:606`), `system_role_goals.rs`, `system_station_jobs.rs`,
`system_job_*.rs` with `shared/located_jobs.rs` (pinned seeds `:150-159`),
`system_encounters.rs`, `system_open_world.rs`;
`examples/playable/start_role_sandbox.rs`, `job_board_sandbox.rs`,
`mine_and_sell.rs`, `first_shift_ships.rs`, `shared/open_world_stage.rs`,
`README.md`. A hull or kit change can move a pinned seed's board.

Docs: `web/src/create/start-roles.md:88-91` (goal counts), `:140-160` (kit
rule and table); `web/src/create/ships.md:190-196` (hull text);
`web/src/wiki/interface.md:277-303` (board, pay, hull gate);
`web/src/wiki/sections.md:46-77` (part stats);
`web/src/wiki/getting-started.md` (New Game, saves); `CHANGELOG.md`
`[Unreleased]` (`:16`; kit entries at `:27`, `:37`).

Saves:

- Header: `role: StartRoleDesignId` and `catalog: u64`
  (`crates/nova_world_base/src/save/mod.rs:121-136`). A Load refuses another
  catalog digest or an unknown role. The digest covers every content item
  (`crates/nova_assets/src/merge.rs:1002-1036`, as read by the earlier map).
  So any kit, hull, part or goal content change refuses every existing save.
- State: `jobs`, `postings`, `stances`, `goal`, `start`, `encounters`
  (`save/mod.rs:317-338`). All are strict; nothing migrates.
- `WORLD_SAVE_VERSION = "v0.18.0"` (`save/mod.rs:109`). `CHANGELOG.md`
  already says a v0.18.0 save no longer loads, so no released save is lost.
  Development saves of this cycle break.
- A mix weight or pay constant is Rust: it does not move the digest. Saved
  offers keep the reward they were drawn with.

## J. Findings

Ordered by hurt to the first 15 minutes. Facts first; opinion is marked.

1. **Bounty hunter has no bounty to take.** Neutralize is the top weight of
   its mix (4/13, `jobs.rs:1030-1036`). It needs an Enemy hull in the
   27-cell cube, and none stands near a start
   (`examples/systems/shared/located_jobs.rs:14-17`). The slot posts a
   Procure (`jobs.rs:2376-2381`). Its goal counts only Neutralize
   (`goal.rs:124-142`). Not measured over seeds.
2. **Pirate has no job that fits it.** No job kind feeds Outlaw
   (`goal.rs:157-168`). Escort and ambush raiders do not count
   (`crates/nova_world_base/src/jobs.rs:1059-1096`). It needs 9 to 12 kills
   of trading hulls, and the nearest are its Ally start civilization's
   (`civilization.rs:128-147`; `clusters.rs:242-251`). Its board is the
   military board.
3. **Unarmed roles lose a third of the board and the best pay.** Escort is
   4/12 of the Trade mix and needs a weapon (`jobs.rs:2829-2864`). The
   starter board hides it. Per fitting row: 418 to 465 cr against 549 to
   706 (scavenger) and 606 to 635 (combat) `[computed]`.
4. **The shown goal is "Stations owned 0 / 1" for every role.** About
   48,000 to 61,000 cr `[earlier]`, 77 to 117 median hand-ins `[computed]`.
   The flavour count is hidden until then
   (`open_objectives.rs:72-75`). A start station holds 10,000 cr and
   refuses a hand-in it cannot pay (`station_layout.rs:142-144`;
   `jobs.rs:3087-3092`).
5. **Three roles are one role.** Bounty hunter, military and pirate share
   the Combat mix and the same kit but credits and 2 torpedoes. Interceptor
   and raider have 38 sections, mass 41.25 and 2 PDCs each; the raider has
   one more thruster (62 against 47 m/s^2) and 200 cr more `[computed]`.
6. **Military starts poorest and weakest, with the easiest goal.** 1,000
   cr, 1,950 HP, 41 m/s^2 (0.064 against the 0.06 floor), hold 59% full
   `[computed]`. 7/13 of its draws count, and hail jobs count too.
7. **Industrial's goal does not count mining and selling.** Only a Raw
   Procure counts (`goal.rs:70-91`). The Raw share of Procure is 0% in a
   derelict-field cell and 20% with no cluster (`jobs.rs:2221-2253`). The
   tug is the weakest hull: 1,840 HP `[computed]`.
8. **Transport's goal needs 4 stations and its hull cannot take Escort.**
   About one station per 160 km window (`clusters.rs:238-242`). Its kit has
   no HullPlate: 240 HP of repair against 440 to 540 `[computed]`.
9. **Armed starts are fragile and short of fire time.** 400 raw damage/s
   per PDC against 60 HP sections; 20 s of fire for a 2-PDC hull from the
   whole kit; 5 s per magazine `[computed]`. Only armed hulls draw an
   ambush (`base_content/encounters.rs:11-47`). No kit can field-rebuild a
   lost section.
10. **Scavenger is the strongest start on paper.** 65 sections, 4,190 HP,
    5,400 kg hold, worth 10,560 cr (twice the tug), a PDC and a mining
    beam, no refused job, 706 cr per row `[computed]`. With no wreck in the
    cube 7/12 of its rows are Procure.

Hull thrust differs by 41 to 63 m/s^2 but changes a 12 km leg by at most
6 s `[computed]`. Docking count and job fit decide the first payout, not
the drive `[opinion]`.

## Not verified

- Wreck and hostile frequency near a start over real seeds (needs option A).
- Whether a Procure can be handed in at once from the kit or the poster's
  stock; derived from code, not run.
- Manual docking time; GOTO margins; turn rate; any speed cap; cargo mass.
- Hit rate, falloff, time to kill; collapse rule.
- Hull and kit worth as the game computes them (`system_start_roles` logs
  them); free refit sockets on the unarmed hulls.
- Ore per pulse in play; market prices at a real start station.
- Station price and hull buy price at the current content (both `[earlier]`).
- Whether an Enemy station refuses docking.
- The Map goal layout; the `--goal` role click in the bench.
- H quotes other than rows 1, 14, 15, 17.
