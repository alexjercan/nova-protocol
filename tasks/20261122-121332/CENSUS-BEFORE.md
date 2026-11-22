# Start-role census

Written by `crates/nova_authoring/tests/start_role_census.rs`. Do not edit the tables by hand: run the tool again.

Seeds 1 to 240: 240 measured, 0 refused by `starting_station`. 5 postings of each of the 3 slots, so 15 board rows a seed and role.

## What is real and what is modelled

Real, called and not copied:

- Base content from the Rust builders: items, sections, ship designs, start roles, the price table, trade profiles, and the base ship part packs.
- The world generator (`NovaLayeredWorld`) and the start pick (`starting_station`).
- The board pass `post_station_boards`, run as a system: station order, the 27-cell cube gate, the field rule (`FieldBodies`), the bodies other boards name, the draw (`station_job_offers`, `SlotDraw`), the target a Recover or a Neutralize names for itself with none left in the field (its leg, its threat and the hold of its wreck, `job_wreck`), the starter board's redraw for the hull, rewards and renewals.
- The hull read (`PlayerHull::fit`), the accept gate the Jobs pane and the accept command share (`plan_board_accept`: the hull gate, then the accept rules), the hand-in (`plan_job_hand_in`), trades (`plan_item_trade`, `price`), and the goal count (`goal_progress`). The gate is told whether a field target still stands from the mirror, by the pane's rule: the wreck carries the whole stack, the hull is not neutralized. A job's own target is not mirrored; the gate does not read it.
- Hull numbers: `resolve_ship_design`, `ShipMeasure`, `hull_price_cr`, `cargo_capacity_g`, item reference prices.

Modelled:

- No game runs. Each cell of the 125-cell window around the player's spawn is mirrored from its real manifest: a root, each station (services, market, dead or living) and each ship (hold, side, derelict or intact, live weapon sections). Rocks, planets and parked sale hulls are not mirrored; the board pass reads none of them.
- Streaming: one cell a frame, nearest to the spawn cell first. The game prepares cells on worker threads, so its order can differ between cells at equal distance.
- A row counts as fitting when the hull gate passes. The Jobs pane lists every row and offers Accept on those (`board_listing` in `crates/nova_interface/src/jobs.rs`), stated here and not called: this crate does not depend on the interface crate.
- A spawned target is not mirrored: no accept runs, so its wreck and its raider never stand. The row carries the leg, the threat and the stack the board posted.
- Postings 2 to 5 of a slot are window renewals: the play clock steps past the longer offer window (3601 s; Procure and the rest stand 1200 s, Escort 3600 s), no job taken, so every board of the window renews, the start board whole. Kit, credits and station stock stay as at the start for every posting.
- Visit postings, scenario postings, hails and encounters do not exist here. No stance changes. No restock.
- Goal: a row counts when `goal_progress` over that one job alone moves the role's own line, the first. Stations owned is 0. For a goal that reads hulls taken, a Neutralize of a hull that stood in the field counts as one hull taken: every field hostile is an intact generated ship and carries a market. A job's own raider carries none and does not count. The cross-role goal count of such a role is a hunt-only paper run: from the start station to the nearest trading hull of the window (an intact ship with a market, at its manifest position), then the nearest left, one flip and burn and one fight each, no dock, no job. The run that counts keeps its dock: 3 hulls of one civilization inside the offence window step it one stance down and an Enemy station refuses the dock, so that run passes over a hull of the start station's civilization once the next one would step it to Enemy from its seeded stance. The free run, which takes the nearest hull whatever its side, is listed beside it. A hunt earns nothing, so it is an upper bound beside the greedy run, not a part of it. The split run joins the two: the first two hulls of the run that keeps its dock, one flip and burn back to the start station, then the greedy run on what is left of the horizon; its goal count is 2. Traffic moves after the manifest, the steps a kill passes to a neighbour civilization are not applied, and a trading hull's own weapons and escorts are not read.

## Paper time

`t(d) = 2 * sqrt(d / a)`: flip and burn over `d` metres from rest to rest, `a` the hull's acceleration. `a = thrust / (mass * tick) * 10 m`, thrust and mass from `ShipMeasure` over the start hull's sections, tick 0.015625 s. Assumptions: every drive pushes along the flight axis, cargo adds no mass, no speed limit, no turn time.

One dock costs 60 s (`DOCK_ALLOWANCE_S`, an assumption, not a measurement).

| kind | paper seconds from accept to payout |
|---|---|
| Procure | 0 when the kit holds the stack or the station sells it and the player can pay and stow it; else, for a hull with a mining beam, a stack of an ore a rock of the station's cell yields and the free hold takes: 2 t(rock) + cut + 1 dock; else not doable (trade elsewhere is not modelled) |
| Supply | 2 t(leg) + 2 docks, with the stack from the kit or the station; else not doable |
| Recover | 2 t(leg) + 2 docks; not doable when the stack does not fit the free hold |
| Distress | 2 t(leg) + 2 docks |
| Escort | leg / 100 m/s (the hauler's cruise, `HAULER_CRUISE`) + raiders x fight + t(leg) + 1 dock |
| Neutralize | 2 t(leg) + fight + 1 dock |

Fight: 60 s a hostile hull (`FIGHT_S_PER_HOSTILE`, an assumption, not a measurement). Content does not give it: a start-band raider's health over two point-defence mounts' raw damage rate is a few seconds, and the measured ammunition ledger (582 and 1,971 rounds for one raider) is 3 to 10 s of trigger time, so closing and tracking bound a fight, and no content field states those. A hull with no weapon section is refused such rows by the hull gate. No losses, no hull damage, no deaths and no ammunition spent are modelled.

Mining: rock is the metres from the start station to the nearest asteroid of its cell whose kind yields the ore (`ore_for_asteroid_kind`), less the rock's authored radius and the beam's reach. The cut is 30 s (`MINING_SETUP_S`, an assumption: stop, open the emitter, aim, and the first pulse, which takes nothing) plus one pulse interval for every pulse, each pulse yielding half the carve sphere's volume in cells as items (one item a field corner; the beam meets a surface). The rendered rock is larger than its authored radius, so the flight is an upper bound. A hull with no beam cannot fill such a row.

Net pay is the reward less the goods bought at the station's ask; a stack from the kit costs 0. A recovered stack stays with the player and is not in net pay.

## Greedy 15-minute model

The player docks once at the start (60 s). Then, until 900 paper seconds: from the current board take the row with the best net pay per paper second among the rows that fit the hull, pass the accept rule, can be done, pay more than their goods, end inside the horizon and that the station has credits for. A row that costs 0 s goes before any timed row, the larger net pay first. The job is accepted, done, and handed in through the real plans; goods are bought through the real trade plan and leave the station's stock; the station's credits pay the reward. The handed-in slot renews on the next board pass, the other slots stand. A Recover empties its stack off the wreck and the player sells the stack to the station at its bid when the market takes it; that sale is listed apart. A Neutralize marks its hull neutralized. A spawned target has no body here: its stack goes to the player and its raider counts as neutralized. A run stops at 100 jobs. Escort and Neutralize income is listed apart because the model gives them a flat fight time and no losses.

## Start stations

The start is the same for every role of a seed.

| fact | value |
|---|---|
| seeds measured | 240 |
| cluster type of the start station's market | asteroid-rich 58.3%, derelict-field 22.9%, planet-heavy 2.9%, rock-only 15.8% |
| civilization advancement (the draw's), min / median / max | 0.000 / 0.010 / 0.024 |
| station design advancement, min / median / max | 0.000 / 0.010 / 0.024 |
| station credits, min / median / max | 10001 / 10295 / 10732 |
| station stock stacks, min / median / max | 15 / 15 / 16 |
| station stock at reference ask cr, min / median / max | 3248 / 3508 / 4036 |
| stations that offer Trade | 100.0% |
| lawful civilizations | 48.8% |
| player spawns in the start station's own cell | 95.0% |
| stations in the 125-cell window, min / median / max | 1 / 1 / 1 |
| of them living with a Jobs board | 1 / 1 / 1 |
| ships in the 125-cell window, min / median / max | 1 / 10 / 38 |

## industrial (Industrial)

Hull `mining_tug`, job mix Mining, goal "Ore to market" ([(OreDeliveries { jobs: 3 }, 3000), (StationsOwned { stations: 1 }, 10000)]).

| start worth | cr |
|---|---|
| hull (every section's recipe at reference ask) | 5060 |
| kit stock at reference ask | 1280 |
| credits | 2200 |
| total | 8540 |

| hull | value |
|---|---|
| sections | 28 |
| mass, cells | 33.0 |
| thrust, impulse per tick | 3.00 |
| acceleration, m/s2 | 58.2 |
| t(10 km), s | 26.2 |
| health sum | 1840 |
| hold, kg | 2100 |
| kit mass, kg | 320 |
| docking sections | 1 |
| turrets / torpedo bays / railguns / mining beams | 0 / 0 / 0 / 1 |
| PDC rounds in the kit | 0 |
| PDC rounds in the magazines | 0 |
| PDC rounds a second, every turret | 0.0 |
| fire seconds, kit and magazines | - |

| first board | value |
|---|---|
| rows that fit, mean of 3 | 3.00 |
| boards with 0 / 1 / 2 / 3 rows that fit | 0.0% / 0.0% / 0.0% / 100.0% |
| cells live when the board posts, min / median / max | 27 / 27 / 77 |
| seeds with a wreck the draw can name | 12.9% |
| wrecks when any, median; nearest wreck m, median | 2; 47700 |
| seeds with a hostile the draw can name | 16.7% |
| hostiles when any, median; nearest hostile m, median | 2; 49200 |
| seeds with a wreck in the cube, named by another board or not | 12.9% |
| seeds with a hostile in the cube, named by another board or not | 16.7% |

| kind share | value |
|---|---|
| drawn, 5 postings | Procure 41.4%, Recover 8.8%, Escort 0.0%, Distress 16.4%, Neutralize 0.0%, Supply 33.4% |
| posted, first board | Procure 41.5%, Recover 9.6%, Escort 0.0%, Distress 15.7%, Neutralize 0.0%, Supply 33.2% |
| posted, 5 postings | Procure 41.4%, Recover 8.8%, Escort 0.0%, Distress 16.4%, Neutralize 0.0%, Supply 33.4% |
| posted and fit, first board | Procure 41.5%, Recover 9.6%, Escort 0.0%, Distress 15.7%, Neutralize 0.0%, Supply 33.2% |
| posted and fit, 5 postings | Procure 41.4%, Recover 8.8%, Escort 0.0%, Distress 16.4%, Neutralize 0.0%, Supply 33.4% |

| draw outcome, 5 postings | value |
|---|---|
| Recover rows with a spawned target | 278 of 315 (88.3%) |
| Neutralize rows with a spawned target | 0 of 0 (-) |
| rows the starter redraw posted as another kind | 0 of 3600 (0.0%) |

Rows by posted kind, 5 postings. Reward, cost, net pay, paper seconds, leg and threat are min / median / max; cost, net pay and paper seconds over the rows that fit and can be done.

| kind | posted | fit | doable | pay | goal | reward cr | goods cost cr | net pay cr | paper s | leg m | threat |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Procure | 1489 | 1489 | 1051 | 1051 | 1139 | 98 / 460 / 5000 | 0 / 0 / 424 | 98 / 392 / 1018 | 0 / 113 / 135 | 0 / 0 / 0 | 0 / 0 / 0 |
| Recover | 315 | 315 | 315 | 315 | 0 | 360 / 740 / 3329 | 0 / 0 / 0 | 360 / 740 / 3329 | 145 / 177 / 270 | 2300 / 12000 / 81400 | 0 / 0 / 0 |
| Escort | 0 | 0 | 0 | 0 | 0 | - | - | - | - | - | - |
| Distress | 592 | 592 | 592 | 592 | 0 | 430 / 697 / 951 | 0 / 0 / 0 | 430 / 697 / 951 | 161 / 177 / 186 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| Neutralize | 0 | 0 | 0 | 0 | 0 | - | - | - | - | - | - |
| Supply | 1204 | 1204 | 817 | 817 | 0 | 462 / 914 / 2333 | 0 / 0 / 512 | 447 / 838 / 1441 | 161 / 177 / 186 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| all | 3600 | 3600 | 2775 | 2775 | 1139 | 98 / 710 / 5000 | 0 / 0 / 512 | 98 / 668 / 3329 | 0 / 167 / 270 | 0 / 6000 / 81400 | 0 / 0 / 0 |

| goods of the rows that fit, 5 postings | Procure | Supply |
|---|---|---|
| rows | 1489 | 1204 |
| kit holds the stack | 3.8% | 36.5% |
| station stocks the stack | 4.7% | 55.3% |
| filled from the kit | 3.8% | 36.5% |
| bought at the station | 2.9% | 31.3% |
| cut with the hull's beam | 63.9% | 0.0% |
| not to be had | 29.4% | 32.1% |
| stack at the station's ask cr, min / median / max | 16 / 128 / 2240 | 4 / 124 / 1048 |

Recover rows that fit: 315; stack has room in the free hold: 100.0%; the stack at the station's bid, min / median / max: 5 / 150 / 2600 cr (kept by the player, not in net pay).

| first board, the rows that pay | value |
|---|---|
| seeds with a row that pays | 99.2% |
| best row by net pay per paper minute: net pay cr, median | 776 |
| best row: paper minutes, median | 2.8 |
| best row costs 0 s (a docked Procure) | 6.3% |
| best row kind | Procure 35.7%, Recover 13.9%, Escort 0.0%, Distress 14.7%, Neutralize 0.0%, Supply 35.7% |
| quickest row: paper seconds, median | 117 |
| quickest row: seconds to first payout with the start dock, median | 177 |
| quickest row: net pay cr, median | 532 |

| goal, rows that fit | value |
|---|---|
| the goal asks for | 3 |
| seeds whose first board has a goal row | 64.2% |
| seeds whose first board has a goal row that pays | 59.2% |
| goal rows that pay, of the goal rows that fit, 5 postings | 952 of 1139 (83.6%) |
| seeds with no goal row in 5 postings | 22.9% |
| goal rows per 15 board rows, lower quartile / median / upper quartile | 3 / 5 / 7 |
| goal count with every fitting row of the 15 handed in, quartiles | 3 / 5 / 7 |

| greedy 15-minute model | value |
|---|---|
| credits at the start | 2200 |
| credits at 15 min, quartiles | 5094 / 5721 / 6210 |
| gain cr at 15 min, quartiles | 2894 / 3521 / 4010 |
| gain cr at 15 min, min / median / max | 0 / 3521 / 7864 |
| jobs handed in, quartiles | 4 / 5 / 5 |
| seeds with no job handed in | 0.8% |
| first payout, paper s, quartiles | 179 / 227 / 242 |
| last payout or stop, paper s, quartiles | 746 / 823 / 869 |
| goal count at 15 min of 3, quartiles | 0 / 1 / 2 |
| net job pay by kind | Procure 26.4%, Recover 15.9%, Escort 0.0%, Distress 20.5%, Neutralize 0.0%, Supply 37.2% |
| Escort and Neutralize share of net job pay | 0.0% |
| docked 0 s rows: share of jobs / of net job pay | 3.6% / 1.9% |
| sale of recovered stacks cr, quartiles | 0 / 0 / 180 |
| why the run stopped | goods not to be had or no hold room 25.4%, next row ends past the horizon 74.6% |

## scavenger (Scavenger)

Hull `salvage_hauler`, job mix Salvage, goal "A salvage run" ([(Recoveries { jobs: 3 }, 3000), (StationsOwned { stations: 1 }, 10000)]).

| start worth | cr |
|---|---|
| hull (every section's recipe at reference ask) | 10560 |
| kit stock at reference ask | 1180 |
| credits | 1500 |
| total | 13240 |

| hull | value |
|---|---|
| sections | 65 |
| mass, cells | 69.1 |
| thrust, impulse per tick | 5.00 |
| acceleration, m/s2 | 46.3 |
| t(10 km), s | 29.4 |
| health sum | 4190 |
| hold, kg | 5400 |
| kit mass, kg | 820 |
| docking sections | 1 |
| turrets / torpedo bays / railguns / mining beams | 1 / 0 / 0 / 1 |
| PDC rounds in the kit | 3000 |
| PDC rounds in the magazines | 500 |
| PDC rounds a second, every turret | 100.0 |
| fire seconds, kit and magazines | 35 |

| first board | value |
|---|---|
| rows that fit, mean of 3 | 3.00 |
| boards with 0 / 1 / 2 / 3 rows that fit | 0.0% / 0.0% / 0.0% / 100.0% |
| cells live when the board posts, min / median / max | 27 / 27 / 77 |
| seeds with a wreck the draw can name | 12.9% |
| wrecks when any, median; nearest wreck m, median | 2; 47700 |
| seeds with a hostile the draw can name | 16.7% |
| hostiles when any, median; nearest hostile m, median | 2; 49200 |
| seeds with a wreck in the cube, named by another board or not | 12.9% |
| seeds with a hostile in the cube, named by another board or not | 16.7% |

| kind share | value |
|---|---|
| drawn, 5 postings | Procure 7.8%, Recover 50.7%, Escort 8.1%, Distress 16.5%, Neutralize 8.3%, Supply 8.7% |
| posted, first board | Procure 8.3%, Recover 51.5%, Escort 6.9%, Distress 16.4%, Neutralize 7.6%, Supply 9.2% |
| posted, 5 postings | Procure 7.8%, Recover 50.7%, Escort 8.1%, Distress 16.5%, Neutralize 8.3%, Supply 8.7% |
| posted and fit, first board | Procure 8.3%, Recover 51.5%, Escort 6.9%, Distress 16.4%, Neutralize 7.6%, Supply 9.2% |
| posted and fit, 5 postings | Procure 7.8%, Recover 50.7%, Escort 8.1%, Distress 16.5%, Neutralize 8.3%, Supply 8.7% |

| draw outcome, 5 postings | value |
|---|---|
| Recover rows with a spawned target | 1642 of 1825 (90.0%) |
| Neutralize rows with a spawned target | 245 of 299 (81.9%) |
| rows the starter redraw posted as another kind | 0 of 3600 (0.0%) |

Rows by posted kind, 5 postings. Reward, cost, net pay, paper seconds, leg and threat are min / median / max; cost, net pay and paper seconds over the rows that fit and can be done.

| kind | posted | fit | doable | pay | goal | reward cr | goods cost cr | net pay cr | paper s | leg m | threat |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Procure | 279 | 279 | 200 | 200 | 0 | 98 / 460 / 3800 | 0 / 0 / 364 | 98 / 394 / 1018 | 0 / 116 / 140 | 0 / 0 / 0 | 0 / 0 / 0 |
| Recover | 1825 | 1825 | 1825 | 1825 | 1825 | 242 / 740 / 3379 | 0 / 0 / 0 | 242 / 740 / 3379 | 143 / 184 / 288 | 1500 / 12000 / 81400 | 0 / 0 / 0 |
| Escort | 292 | 292 | 292 | 292 | 0 | 955 / 1393 / 1828 | 0 / 0 / 0 | 955 / 1393 / 1828 | 249 / 295 / 317 | 10000 / 14000 / 16000 | 1 / 1 / 1 |
| Distress | 593 | 593 | 593 | 593 | 0 | 430 / 697 / 963 | 0 / 0 / 0 | 430 / 697 / 963 | 166 / 184 / 194 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| Neutralize | 299 | 299 | 299 | 299 | 0 | 434 / 1160 / 3744 | 0 / 0 / 0 | 434 / 1160 / 3744 | 140 / 184 / 294 | 1200 / 12000 / 87100 | 0 / 1 / 2 |
| Supply | 312 | 312 | 214 | 214 | 0 | 463 / 902 / 2297 | 0 / 48 / 496 | 442 / 834 / 1441 | 166 / 184 / 194 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| all | 3600 | 3600 | 3423 | 3423 | 1825 | 98 / 775 / 3800 | 0 / 0 / 496 | 98 / 770 / 3744 | 0 / 184 / 317 | 0 / 12000 / 87100 | 0 / 0 / 2 |

| goods of the rows that fit, 5 postings | Procure | Supply |
|---|---|---|
| rows | 279 | 312 |
| kit holds the stack | 2.5% | 22.1% |
| station stocks the stack | 2.9% | 55.4% |
| filled from the kit | 2.5% | 22.1% |
| bought at the station | 1.4% | 46.5% |
| cut with the hull's beam | 67.7% | 0.0% |
| not to be had | 28.3% | 31.4% |
| stack at the station's ask cr, min / median / max | 16 / 140 / 1602 | 9 / 124 / 1048 |

Recover rows that fit: 1825; stack has room in the free hold: 100.0%; the stack at the station's bid, min / median / max: 4 / 150 / 6578 cr (kept by the player, not in net pay).

| first board, the rows that pay | value |
|---|---|
| seeds with a row that pays | 100.0% |
| best row by net pay per paper minute: net pay cr, median | 980 |
| best row: paper minutes, median | 3.2 |
| best row costs 0 s (a docked Procure) | 0.8% |
| best row kind | Procure 5.8%, Recover 46.2%, Escort 9.6%, Distress 10.8%, Neutralize 17.9%, Supply 9.6% |
| quickest row: paper seconds, median | 173 |
| quickest row: seconds to first payout with the start dock, median | 233 |
| quickest row: net pay cr, median | 621 |

| goal, rows that fit | value |
|---|---|
| the goal asks for | 3 |
| seeds whose first board has a goal row | 87.9% |
| seeds whose first board has a goal row that pays | 87.9% |
| goal rows that pay, of the goal rows that fit, 5 postings | 1825 of 1825 (100.0%) |
| seeds with no goal row in 5 postings | 0.0% |
| goal rows per 15 board rows, lower quartile / median / upper quartile | 6 / 8 / 9 |
| goal count with every fitting row of the 15 handed in, quartiles | 6 / 8 / 9 |

| greedy 15-minute model | value |
|---|---|
| credits at the start | 1500 |
| credits at 15 min, quartiles | 5122 / 5710 / 6458 |
| gain cr at 15 min, quartiles | 3622 / 4210 / 4958 |
| gain cr at 15 min, min / median / max | 2666 / 4210 / 9462 |
| jobs handed in, quartiles | 4 / 4 / 4 |
| seeds with no job handed in | 0.0% |
| first payout, paper s, quartiles | 239 / 250 / 254 |
| last payout or stop, paper s, quartiles | 780 / 808 / 856 |
| goal count at 15 min of 3, quartiles | 1 / 2 / 3 |
| net job pay by kind | Procure 3.4%, Recover 51.5%, Escort 12.4%, Distress 12.3%, Neutralize 13.7%, Supply 6.8% |
| Escort and Neutralize share of net job pay | 26.1% |
| docked 0 s rows: share of jobs / of net job pay | 0.7% / 0.3% |
| sale of recovered stacks cr, quartiles | 208 / 418 / 826 |
| why the run stopped | goods not to be had or no hold room 0.8%, next row ends past the horizon 99.2% |

## transport (Transport)

Hull `light_freighter`, job mix Trade, goal "A freight line" ([(CargoRuns { jobs: 3 }, 3000), (StationsOwned { stations: 1 }, 10000)]).

| start worth | cr |
|---|---|
| hull (every section's recipe at reference ask) | 7580 |
| kit stock at reference ask | 1880 |
| credits | 2200 |
| total | 11660 |

| hull | value |
|---|---|
| sections | 46 |
| mass, cells | 51.0 |
| thrust, impulse per tick | 5.00 |
| acceleration, m/s2 | 62.7 |
| t(10 km), s | 25.2 |
| health sum | 2950 |
| hold, kg | 3700 |
| kit mass, kg | 820 |
| docking sections | 1 |
| turrets / torpedo bays / railguns / mining beams | 0 / 0 / 0 / 0 |
| PDC rounds in the kit | 0 |
| PDC rounds in the magazines | 0 |
| PDC rounds a second, every turret | 0.0 |
| fire seconds, kit and magazines | - |

| first board | value |
|---|---|
| rows that fit, mean of 3 | 3.00 |
| boards with 0 / 1 / 2 / 3 rows that fit | 0.0% / 0.0% / 0.0% / 100.0% |
| cells live when the board posts, min / median / max | 27 / 27 / 77 |
| seeds with a wreck the draw can name | 12.9% |
| wrecks when any, median; nearest wreck m, median | 2; 47700 |
| seeds with a hostile the draw can name | 16.7% |
| hostiles when any, median; nearest hostile m, median | 2; 49200 |
| seeds with a wreck in the cube, named by another board or not | 12.9% |
| seeds with a hostile in the cube, named by another board or not | 16.7% |

| kind share | value |
|---|---|
| drawn, 5 postings | Procure 24.7%, Recover 8.3%, Escort 8.4%, Distress 17.1%, Neutralize 0.0%, Supply 41.6% |
| posted, first board | Procure 26.8%, Recover 8.9%, Escort 0.0%, Distress 19.9%, Neutralize 0.0%, Supply 44.4% |
| posted, 5 postings | Procure 27.2%, Recover 9.0%, Escort 0.0%, Distress 18.9%, Neutralize 0.0%, Supply 44.9% |
| posted and fit, first board | Procure 26.8%, Recover 8.9%, Escort 0.0%, Distress 19.9%, Neutralize 0.0%, Supply 44.4% |
| posted and fit, 5 postings | Procure 27.2%, Recover 9.0%, Escort 0.0%, Distress 18.9%, Neutralize 0.0%, Supply 44.9% |

| draw outcome, 5 postings | value |
|---|---|
| Recover rows with a spawned target | 279 of 323 (86.4%) |
| Neutralize rows with a spawned target | 0 of 0 (-) |
| rows the starter redraw posted as another kind | 303 of 3600 (8.4%) |

Rows by posted kind, 5 postings. Reward, cost, net pay, paper seconds, leg and threat are min / median / max; cost, net pay and paper seconds over the rows that fit and can be done.

| kind | posted | fit | doable | pay | goal | reward cr | goods cost cr | net pay cr | paper s | leg m | threat |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Procure | 980 | 980 | 255 | 255 | 980 | 98 / 442 / 5000 | 0 / 0 / 424 | 110 / 466 / 826 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| Recover | 323 | 323 | 323 | 323 | 0 | 310 / 718 / 3329 | 0 / 0 / 0 | 310 / 718 / 3329 | 140 / 170 / 264 | 1500 / 10000 / 81400 | 0 / 0 / 0 |
| Escort | 0 | 0 | 0 | 0 | 0 | - | - | - | - | - | - |
| Distress | 682 | 682 | 682 | 682 | 0 | 430 / 700 / 953 | 0 / 0 / 0 | 430 / 700 / 953 | 159 / 175 / 184 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| Neutralize | 0 | 0 | 0 | 0 | 0 | - | - | - | - | - | - |
| Supply | 1615 | 1615 | 1202 | 1202 | 1615 | 460 / 918 / 2333 | 0 / 0 / 512 | 438 / 831 / 1441 | 159 / 175 / 184 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| all | 3600 | 3600 | 2462 | 2462 | 2595 | 98 / 759 / 5000 | 0 / 0 / 512 | 110 / 745 / 3329 | 0 / 170 / 264 | 0 / 8000 / 81400 | 0 / 0 / 0 |

| goods of the rows that fit, 5 postings | Procure | Supply |
|---|---|---|
| rows | 980 | 1615 |
| kit holds the stack | 23.3% | 41.5% |
| station stocks the stack | 4.5% | 56.4% |
| filled from the kit | 23.3% | 41.5% |
| bought at the station | 2.8% | 32.9% |
| cut with the hull's beam | 0.0% | 0.0% |
| not to be had | 74.0% | 25.6% |
| stack at the station's ask cr, min / median / max | 16 / 120 / 2240 | 4 / 124 / 1048 |

Recover rows that fit: 323; stack has room in the free hold: 100.0%; the stack at the station's bid, min / median / max: 4 / 154 / 4840 cr (kept by the player, not in net pay).

| first board, the rows that pay | value |
|---|---|
| seeds with a row that pays | 98.3% |
| best row by net pay per paper minute: net pay cr, median | 820 |
| best row: paper minutes, median | 2.9 |
| best row costs 0 s (a docked Procure) | 19.5% |
| best row kind | Procure 19.5%, Recover 5.9%, Escort 0.0%, Distress 19.5%, Neutralize 0.0%, Supply 55.1% |
| quickest row: paper seconds, median | 165 |
| quickest row: seconds to first payout with the start dock, median | 225 |
| quickest row: net pay cr, median | 680 |

| goal, rows that fit | value |
|---|---|
| the goal asks for | 3 |
| seeds whose first board has a goal row | 97.1% |
| seeds whose first board has a goal row that pays | 80.8% |
| goal rows that pay, of the goal rows that fit, 5 postings | 1457 of 2595 (56.1%) |
| seeds with no goal row in 5 postings | 0.0% |
| goal rows per 15 board rows, lower quartile / median / upper quartile | 10 / 11 / 12 |
| goal count with every fitting row of the 15 handed in, quartiles | 10 / 11 / 12 |

| greedy 15-minute model | value |
|---|---|
| credits at the start | 2200 |
| credits at 15 min, quartiles | 4666 / 5480 / 5930 |
| gain cr at 15 min, quartiles | 2466 / 3280 / 3730 |
| gain cr at 15 min, min / median / max | 0 / 3280 / 9028 |
| jobs handed in, quartiles | 3 / 4 / 5 |
| seeds with no job handed in | 1.7% |
| first payout, paper s, quartiles | 225 / 235 / 240 |
| last payout or stop, paper s, quartiles | 568 / 746 / 765 |
| goal count at 15 min of 3, quartiles | 1 / 2 / 3 |
| net job pay by kind | Procure 7.1%, Recover 14.4%, Escort 0.0%, Distress 24.2%, Neutralize 0.0%, Supply 54.3% |
| Escort and Neutralize share of net job pay | 0.0% |
| docked 0 s rows: share of jobs / of net job pay | 11.7% / 7.1% |
| sale of recovered stacks cr, quartiles | 0 / 0 / 130 |
| why the run stopped | goods not to be had or no hold room 47.9%, next row ends past the horizon 52.1% |

## bounty_hunter (Bounty Hunter)

Hull `interceptor`, job mix Bounty, goal "A kill list" ([(KillList { hulls: 3 }, 3000), (StationsOwned { stations: 1 }, 10000)]).

| start worth | cr |
|---|---|
| hull (every section's recipe at reference ask) | 6920 |
| kit stock at reference ask | 1180 |
| credits | 1800 |
| total | 9900 |

| hull | value |
|---|---|
| sections | 38 |
| mass, cells | 41.2 |
| thrust, impulse per tick | 5.00 |
| acceleration, m/s2 | 77.6 |
| t(10 km), s | 22.7 |
| health sum | 2570 |
| hold, kg | 2800 |
| kit mass, kg | 820 |
| docking sections | 1 |
| turrets / torpedo bays / railguns / mining beams | 2 / 0 / 0 / 0 |
| PDC rounds in the kit | 3000 |
| PDC rounds in the magazines | 1000 |
| PDC rounds a second, every turret | 200.0 |
| fire seconds, kit and magazines | 20 |

| first board | value |
|---|---|
| rows that fit, mean of 3 | 3.00 |
| boards with 0 / 1 / 2 / 3 rows that fit | 0.0% / 0.0% / 0.0% / 100.0% |
| cells live when the board posts, min / median / max | 27 / 27 / 77 |
| seeds with a wreck the draw can name | 12.9% |
| wrecks when any, median; nearest wreck m, median | 2; 47700 |
| seeds with a hostile the draw can name | 16.7% |
| hostiles when any, median; nearest hostile m, median | 2; 49200 |
| seeds with a wreck in the cube, named by another board or not | 12.9% |
| seeds with a hostile in the cube, named by another board or not | 16.7% |

| kind share | value |
|---|---|
| drawn, 5 postings | Procure 16.3%, Recover 16.6%, Escort 17.2%, Distress 16.4%, Neutralize 33.4%, Supply 0.0% |
| posted, first board | Procure 16.5%, Recover 16.1%, Escort 18.5%, Distress 15.7%, Neutralize 33.2%, Supply 0.0% |
| posted, 5 postings | Procure 16.3%, Recover 16.6%, Escort 17.2%, Distress 16.4%, Neutralize 33.4%, Supply 0.0% |
| posted and fit, first board | Procure 16.5%, Recover 16.1%, Escort 18.5%, Distress 15.7%, Neutralize 33.2%, Supply 0.0% |
| posted and fit, 5 postings | Procure 16.3%, Recover 16.6%, Escort 17.2%, Distress 16.4%, Neutralize 33.4%, Supply 0.0% |

| draw outcome, 5 postings | value |
|---|---|
| Recover rows with a spawned target | 529 of 598 (88.5%) |
| Neutralize rows with a spawned target | 1028 of 1204 (85.4%) |
| rows the starter redraw posted as another kind | 0 of 3600 (0.0%) |

Rows by posted kind, 5 postings. Reward, cost, net pay, paper seconds, leg and threat are min / median / max; cost, net pay and paper seconds over the rows that fit and can be done.

| kind | posted | fit | doable | pay | goal | reward cr | goods cost cr | net pay cr | paper s | leg m | threat |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Procure | 588 | 588 | 33 | 33 | 0 | 98 / 442 / 5000 | 0 / 40 / 424 | 110 / 320 / 680 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| Recover | 598 | 598 | 598 | 598 | 0 | 310 / 730 / 3379 | 0 / 0 / 0 | 310 / 730 / 3379 | 138 / 165 / 250 | 1500 / 10000 / 81400 | 0 / 0 / 0 |
| Escort | 618 | 618 | 618 | 618 | 0 | 946 / 1389 / 1840 | 0 / 0 / 0 | 946 / 1389 / 1840 | 243 / 287 / 309 | 10000 / 14000 / 16000 | 1 / 1 / 1 |
| Distress | 592 | 592 | 592 | 592 | 0 | 430 / 697 / 951 | 0 / 0 / 0 | 430 / 697 / 951 | 155 / 170 / 177 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| Neutralize | 1204 | 1204 | 1204 | 1204 | 1204 | 410 / 1136 / 3744 | 0 / 0 / 0 | 410 / 1136 / 3744 | 136 / 170 / 254 | 1200 / 12000 / 87100 | 0 / 1 / 2 |
| Supply | 0 | 0 | 0 | 0 | 0 | - | - | - | - | - | - |
| all | 3600 | 3600 | 3045 | 3045 | 1204 | 98 / 1000 / 5000 | 0 / 0 / 424 | 110 / 1070 / 3744 | 0 / 174 / 309 | 0 / 12000 / 87100 | 0 / 0 / 2 |

| goods of the rows that fit, 5 postings | Procure | Supply |
|---|---|---|
| rows | 588 | 0 |
| kit holds the stack | 2.4% | - |
| station stocks the stack | 4.4% | - |
| filled from the kit | 2.4% | - |
| bought at the station | 3.2% | - |
| cut with the hull's beam | 0.0% | - |
| not to be had | 94.4% | - |
| stack at the station's ask cr, min / median / max | 16 / 126 / 2240 | - |

Recover rows that fit: 598; stack has room in the free hold: 100.0%; the stack at the station's bid, min / median / max: 4 / 152 / 4840 cr (kept by the player, not in net pay).

| first board, the rows that pay | value |
|---|---|
| seeds with a row that pays | 99.2% |
| best row by net pay per paper minute: net pay cr, median | 1184 |
| best row: paper minutes, median | 2.9 |
| best row costs 0 s (a docked Procure) | 2.1% |
| best row kind | Procure 2.1%, Recover 10.5%, Escort 14.7%, Distress 8.8%, Neutralize 63.9%, Supply 0.0% |
| quickest row: paper seconds, median | 165 |
| quickest row: seconds to first payout with the start dock, median | 225 |
| quickest row: net pay cr, median | 882 |

| goal, rows that fit | value |
|---|---|
| the goal asks for | 3 |
| seeds whose first board has a goal row | 67.5% |
| seeds whose first board has a goal row that pays | 67.5% |
| goal rows that pay, of the goal rows that fit, 5 postings | 1204 of 1204 (100.0%) |
| seeds with no goal row in 5 postings | 0.0% |
| goal rows per 15 board rows, lower quartile / median / upper quartile | 4 / 5 / 6 |
| goal count with every fitting row of the 15 handed in, quartiles | 3 / 4 / 6 |

| greedy 15-minute model | value |
|---|---|
| credits at the start | 1800 |
| credits at 15 min, quartiles | 5816 / 6332 / 6880 |
| gain cr at 15 min, quartiles | 4016 / 4532 / 5080 |
| gain cr at 15 min, min / median / max | 0 / 4532 / 10166 |
| jobs handed in, quartiles | 4 / 4 / 4 |
| seeds with no job handed in | 0.8% |
| first payout, paper s, quartiles | 225 / 234 / 272 |
| last payout or stop, paper s, quartiles | 750 / 829 / 867 |
| goal count at 15 min of 3, quartiles | 1 / 2 / 3 |
| net job pay by kind | Procure 0.5%, Recover 13.6%, Escort 23.1%, Distress 9.4%, Neutralize 53.4%, Supply 0.0% |
| Escort and Neutralize share of net job pay | 76.5% |
| docked 0 s rows: share of jobs / of net job pay | 1.6% / 0.5% |
| sale of recovered stacks cr, quartiles | 0 / 0 / 237 |
| why the run stopped | goods not to be had or no hold room 8.3%, station cannot pay 0.4%, next row ends past the horizon 91.2% |

## military (Military)

Hull `patrol_picket`, job mix Patrol, goal "Patrol record" ([(Patrols { jobs: 3 }, 3000), (StationsOwned { stations: 1 }, 10000)]).

| start worth | cr |
|---|---|
| hull (every section's recipe at reference ask) | 6810 |
| kit stock at reference ask | 1980 |
| credits | 1800 |
| total | 10590 |

| hull | value |
|---|---|
| sections | 33 |
| mass, cells | 37.2 |
| thrust, impulse per tick | 3.00 |
| acceleration, m/s2 | 51.5 |
| t(10 km), s | 27.9 |
| health sum | 2320 |
| hold, kg | 2300 |
| kit mass, kg | 1120 |
| docking sections | 2 |
| turrets / torpedo bays / railguns / mining beams | 2 / 1 / 0 / 0 |
| PDC rounds in the kit | 3000 |
| PDC rounds in the magazines | 1000 |
| PDC rounds a second, every turret | 200.0 |
| fire seconds, kit and magazines | 20 |

| first board | value |
|---|---|
| rows that fit, mean of 3 | 3.00 |
| boards with 0 / 1 / 2 / 3 rows that fit | 0.0% / 0.0% / 0.0% / 100.0% |
| cells live when the board posts, min / median / max | 27 / 27 / 77 |
| seeds with a wreck the draw can name | 12.9% |
| wrecks when any, median; nearest wreck m, median | 2; 47700 |
| seeds with a hostile the draw can name | 16.7% |
| hostiles when any, median; nearest hostile m, median | 2; 49200 |
| seeds with a wreck in the cube, named by another board or not | 12.9% |
| seeds with a hostile in the cube, named by another board or not | 16.7% |

| kind share | value |
|---|---|
| drawn, 5 postings | Procure 16.3%, Recover 0.0%, Escort 33.8%, Distress 8.3%, Neutralize 24.6%, Supply 17.0% |
| posted, first board | Procure 16.5%, Recover 0.0%, Escort 34.6%, Distress 8.8%, Neutralize 23.3%, Supply 16.8% |
| posted, 5 postings | Procure 16.3%, Recover 0.0%, Escort 33.8%, Distress 8.3%, Neutralize 24.6%, Supply 17.0% |
| posted and fit, first board | Procure 16.5%, Recover 0.0%, Escort 34.6%, Distress 8.8%, Neutralize 23.3%, Supply 16.8% |
| posted and fit, 5 postings | Procure 16.3%, Recover 0.0%, Escort 33.8%, Distress 8.3%, Neutralize 24.6%, Supply 17.0% |

| draw outcome, 5 postings | value |
|---|---|
| Recover rows with a spawned target | 0 of 0 (-) |
| Neutralize rows with a spawned target | 753 of 885 (85.1%) |
| rows the starter redraw posted as another kind | 0 of 3600 (0.0%) |

Rows by posted kind, 5 postings. Reward, cost, net pay, paper seconds, leg and threat are min / median / max; cost, net pay and paper seconds over the rows that fit and can be done.

| kind | posted | fit | doable | pay | goal | reward cr | goods cost cr | net pay cr | paper s | leg m | threat |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Procure | 588 | 588 | 33 | 33 | 0 | 98 / 442 / 5000 | 0 / 40 / 424 | 110 / 320 / 680 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| Recover | 0 | 0 | 0 | 0 | 0 | - | - | - | - | - | - |
| Escort | 1216 | 1216 | 1216 | 1216 | 1216 | 945 / 1399 / 1840 | 0 / 0 / 0 | 945 / 1399 / 1840 | 248 / 293 / 315 | 10000 / 14000 / 16000 | 1 / 1 / 1 |
| Distress | 300 | 300 | 300 | 300 | 300 | 430 / 697 / 951 | 0 / 0 / 0 | 430 / 697 / 951 | 163 / 176 / 190 | 6000 / 10000 / 16000 | 0 / 0 / 0 |
| Neutralize | 885 | 885 | 885 | 885 | 0 | 413 / 1127 / 3644 | 0 / 0 / 0 | 413 / 1127 / 3644 | 139 / 181 / 284 | 1200 / 12000 / 87100 | 0 / 1 / 2 |
| Supply | 611 | 611 | 410 | 410 | 611 | 463 / 907 / 2297 | 0 / 32 / 496 | 442 / 840 / 1544 | 163 / 181 / 190 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| all | 3600 | 3600 | 2844 | 2844 | 2127 | 98 / 1132 / 5000 | 0 / 0 / 496 | 110 / 1187 / 3644 | 0 / 190 / 315 | 0 / 12000 / 87100 | 0 / 1 / 2 |

| goods of the rows that fit, 5 postings | Procure | Supply |
|---|---|---|
| rows | 588 | 611 |
| kit holds the stack | 2.4% | 26.7% |
| station stocks the stack | 4.4% | 56.1% |
| filled from the kit | 2.4% | 26.7% |
| bought at the station | 3.2% | 40.4% |
| cut with the hull's beam | 0.0% | 0.0% |
| not to be had | 94.4% | 32.9% |
| stack at the station's ask cr, min / median / max | 16 / 126 / 2240 | 4 / 124 / 1048 |

Recover rows that fit: 0; stack has room in the free hold: -; the stack at the station's bid, min / median / max: - cr (kept by the player, not in net pay).

| first board, the rows that pay | value |
|---|---|
| seeds with a row that pays | 98.8% |
| best row by net pay per paper minute: net pay cr, median | 1240 |
| best row: paper minutes, median | 3.2 |
| best row costs 0 s (a docked Procure) | 2.1% |
| best row kind | Procure 2.1%, Recover 0.0%, Escort 31.2%, Distress 4.2%, Neutralize 51.9%, Supply 10.5% |
| quickest row: paper seconds, median | 181 |
| quickest row: seconds to first payout with the start dock, median | 241 |
| quickest row: net pay cr, median | 999 |

| goal, rows that fit | value |
|---|---|
| the goal asks for | 3 |
| seeds whose first board has a goal row | 93.8% |
| seeds whose first board has a goal row that pays | 90.8% |
| goal rows that pay, of the goal rows that fit, 5 postings | 1926 of 2127 (90.6%) |
| seeds with no goal row in 5 postings | 0.0% |
| goal rows per 15 board rows, lower quartile / median / upper quartile | 7 / 9 / 10 |
| goal count with every fitting row of the 15 handed in, quartiles | 7 / 9 / 10 |

| greedy 15-minute model | value |
|---|---|
| credits at the start | 1800 |
| credits at 15 min, quartiles | 5419 / 5816 / 6257 |
| gain cr at 15 min, quartiles | 3619 / 4016 / 4457 |
| gain cr at 15 min, min / median / max | 0 / 4016 / 8222 |
| jobs handed in, quartiles | 3 / 3 / 4 |
| seeds with no job handed in | 1.2% |
| first payout, paper s, quartiles | 241 / 250 / 331 |
| last payout or stop, paper s, quartiles | 738 / 799 / 856 |
| goal count at 15 min of 3, quartiles | 1 / 2 / 2 |
| net job pay by kind | Procure 0.3%, Recover 0.0%, Escort 43.0%, Distress 4.4%, Neutralize 40.6%, Supply 11.8% |
| Escort and Neutralize share of net job pay | 83.5% |
| docked 0 s rows: share of jobs / of net job pay | 1.1% / 0.3% |
| sale of recovered stacks cr, quartiles | 0 / 0 / 0 |
| why the run stopped | goods not to be had or no hold room 12.5%, next row ends past the horizon 87.5% |

## pirate (Pirate)

Hull `raider`, job mix Raid, goal "An outlaw's name" ([(Outlaw { hulls: 3 }, 3000), (StationsOwned { stations: 1 }, 10000)]).

| start worth | cr |
|---|---|
| hull (every section's recipe at reference ask) | 6780 |
| kit stock at reference ask | 1180 |
| credits | 1800 |
| total | 9760 |

| hull | value |
|---|---|
| sections | 38 |
| mass, cells | 41.2 |
| thrust, impulse per tick | 4.00 |
| acceleration, m/s2 | 62.1 |
| t(10 km), s | 25.4 |
| health sum | 2560 |
| hold, kg | 2900 |
| kit mass, kg | 820 |
| docking sections | 1 |
| turrets / torpedo bays / railguns / mining beams | 2 / 0 / 0 / 0 |
| PDC rounds in the kit | 3000 |
| PDC rounds in the magazines | 1000 |
| PDC rounds a second, every turret | 200.0 |
| fire seconds, kit and magazines | 20 |

| first board | value |
|---|---|
| rows that fit, mean of 3 | 3.00 |
| boards with 0 / 1 / 2 / 3 rows that fit | 0.0% / 0.0% / 0.0% / 100.0% |
| cells live when the board posts, min / median / max | 27 / 27 / 77 |
| seeds with a wreck the draw can name | 12.9% |
| wrecks when any, median; nearest wreck m, median | 2; 47700 |
| seeds with a hostile the draw can name | 16.7% |
| hostiles when any, median; nearest hostile m, median | 2; 49200 |
| seeds with a wreck in the cube, named by another board or not | 12.9% |
| seeds with a hostile in the cube, named by another board or not | 16.7% |

| kind share | value |
|---|---|
| drawn, 5 postings | Procure 7.8%, Recover 33.6%, Escort 0.0%, Distress 8.8%, Neutralize 41.2%, Supply 8.7% |
| posted, first board | Procure 8.3%, Recover 33.2%, Escort 0.0%, Distress 9.6%, Neutralize 39.7%, Supply 9.2% |
| posted, 5 postings | Procure 7.8%, Recover 33.6%, Escort 0.0%, Distress 8.8%, Neutralize 41.2%, Supply 8.7% |
| posted and fit, first board | Procure 8.3%, Recover 33.2%, Escort 0.0%, Distress 9.6%, Neutralize 39.7%, Supply 9.2% |
| posted and fit, 5 postings | Procure 7.8%, Recover 33.6%, Escort 0.0%, Distress 8.8%, Neutralize 41.2%, Supply 8.7% |

| draw outcome, 5 postings | value |
|---|---|
| Recover rows with a spawned target | 1080 of 1210 (89.3%) |
| Neutralize rows with a spawned target | 1275 of 1484 (85.9%) |
| rows the starter redraw posted as another kind | 0 of 3600 (0.0%) |

Rows by posted kind, 5 postings. Reward, cost, net pay, paper seconds, leg and threat are min / median / max; cost, net pay and paper seconds over the rows that fit and can be done.

| kind | posted | fit | doable | pay | goal | reward cr | goods cost cr | net pay cr | paper s | leg m | threat |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Procure | 279 | 279 | 11 | 11 | 0 | 98 / 460 / 3800 | 0 / 0 / 364 | 158 / 330 / 500 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| Recover | 1210 | 1210 | 1210 | 1210 | 0 | 242 / 736 / 3379 | 0 / 0 / 0 | 242 / 736 / 3379 | 140 / 176 / 265 | 1500 / 12000 / 81400 | 0 / 0 / 0 |
| Escort | 0 | 0 | 0 | 0 | 0 | - | - | - | - | - | - |
| Distress | 315 | 315 | 315 | 315 | 0 | 430 / 707 / 953 | 0 / 0 / 0 | 430 / 707 / 953 | 159 / 176 / 184 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| Neutralize | 1484 | 1484 | 1484 | 1484 | 209 | 413 / 1130 / 3744 | 0 / 0 / 0 | 413 / 1130 / 3744 | 138 / 176 / 270 | 1200 / 12000 / 87100 | 0 / 1 / 2 |
| Supply | 312 | 312 | 214 | 214 | 0 | 463 / 902 / 2297 | 0 / 48 / 496 | 442 / 834 / 1441 | 159 / 176 / 184 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| all | 3600 | 3600 | 3234 | 3234 | 209 | 98 / 947 / 3800 | 0 / 0 / 496 | 158 / 960 / 3744 | 0 / 176 / 270 | 0 / 12000 / 87100 | 0 / 0 / 2 |

| goods of the rows that fit, 5 postings | Procure | Supply |
|---|---|---|
| rows | 279 | 312 |
| kit holds the stack | 2.5% | 22.1% |
| station stocks the stack | 2.9% | 55.4% |
| filled from the kit | 2.5% | 22.1% |
| bought at the station | 1.4% | 46.5% |
| cut with the hull's beam | 0.0% | 0.0% |
| not to be had | 96.1% | 31.4% |
| stack at the station's ask cr, min / median / max | 16 / 140 / 1602 | 9 / 124 / 1048 |

Recover rows that fit: 1210; stack has room in the free hold: 100.0%; the stack at the station's bid, min / median / max: 4 / 150 / 6578 cr (kept by the player, not in net pay).

| first board, the rows that pay | value |
|---|---|
| seeds with a row that pays | 100.0% |
| best row by net pay per paper minute: net pay cr, median | 1170 |
| best row: paper minutes, median | 3 |
| best row costs 0 s (a docked Procure) | 0.8% |
| best row kind | Procure 0.8%, Recover 17.9%, Escort 0.0%, Distress 3.8%, Neutralize 72.5%, Supply 5.0% |
| quickest row: paper seconds, median | 165 |
| quickest row: seconds to first payout with the start dock, median | 225 |
| quickest row: net pay cr, median | 843 |

| goal, rows that fit | value |
|---|---|
| the goal asks for | 3 |
| seeds whose first board has a goal row | 14.2% |
| seeds whose first board has a goal row that pays | 14.2% |
| goal rows that pay, of the goal rows that fit, 5 postings | 209 of 209 (100.0%) |
| seeds with no goal row in 5 postings | 83.3% |
| goal rows per 15 board rows, lower quartile / median / upper quartile | 0 / 0 / 0 |
| goal count with every fitting row of the 15 handed in, quartiles | 0 / 0 / 0 |

| greedy 15-minute model | value |
|---|---|
| credits at the start | 1800 |
| credits at 15 min, quartiles | 5932 / 6285 / 7043 |
| gain cr at 15 min, quartiles | 4132 / 4485 / 5243 |
| gain cr at 15 min, min / median / max | 763 / 4485 / 9544 |
| jobs handed in, quartiles | 4 / 4 / 4 |
| seeds with no job handed in | 0.0% |
| first payout, paper s, quartiles | 231 / 240 / 244 |
| last payout or stop, paper s, quartiles | 755 / 765 / 780 |
| goal count at 15 min of 3, quartiles | 0 / 0 / 0 |
| net job pay by kind | Procure 0.3%, Recover 27.5%, Escort 0.0%, Distress 4.8%, Neutralize 61.6%, Supply 5.8% |
| Escort and Neutralize share of net job pay | 61.6% |
| docked 0 s rows: share of jobs / of net job pay | 0.8% / 0.3% |
| sale of recovered stacks cr, quartiles | 0 / 178 / 444 |
| why the run stopped | goods not to be had or no hold room 3.8%, next row ends past the horizon 96.2% |

| hunt-only paper run | value |
|---|---|
| trading hulls in the window, quartiles | 5 / 8 / 12 |
| of those, hulls of the start station's civilization, quartiles | 3 / 6 / 8 |
| metres to the nearest, quartiles | 2205 / 2671 / 3046 |
| hulls taken inside 900 s, quartiles | 3 / 3 / 3 |
| of those, hulls of the start station's civilization, quartiles | 2 / 3 / 3 |
| seeds with no hull taken | 0.0% |
| paper s at the third hull, quartiles | 255 / 267 / 277 |
| seeds with fewer than three trading hulls in the window | 6.7% |
| hulls taken inside 900 s by a run that keeps its dock, quartiles | 3 / 3 / 3 |
| that run: seeds with 0 / 1 / 2 / 3 hulls taken | 0.0% / 1.2% / 5.4% / 93.3% |
| that run: paper s at the third hull, quartiles | 255 / 267 / 277 |
| that run: seeds with no third hull to take | 6.7% |
| split run, two hulls then the board: paper s back at the station, quartiles | 159 / 164 / 172 |
| split run: gain cr at 15 min, quartiles | 3147 / 3498 / 4175 |
| split run: jobs handed in, quartiles | 3 / 3 / 3 |
| split run: seeds with no second hull to take | 1.2% |
| board goal count at 15 min in the greedy run, quartiles | 0 / 0 / 0 |

## First boards of the shown seeds

To compare with the line `the station '<id>' posts 3 offer(s) over W wreck(s) and H hostile(s): ...` a live run logs for the same seed and role.

| seed | role | station | wrecks | hostiles | slot 0 | slot 1 | slot 2 |
|---|---|---|---|---|---|---|---|
| 10 | industrial | sector_0_6_2_cluster_0_4_1_station_0 | 0 | 0 | Bring 28 Water ice | Rearm a stranded ship with 1 Torpedo | Answer a distress call |
| 10 | scavenger | sector_0_6_2_cluster_0_4_1_station_0 | 0 | 0 | Recover 2 Water ice from Fenda derelict | Neutralize Fenda raider | Escort a hauler carrying 1,579 cr |
| 10 | transport | sector_0_6_2_cluster_0_4_1_station_0 | 0 | 0 | Recover 2 Water ice from Fenda derelict | Rearm a stranded ship with 1 Torpedo | Repair a stranded ship with 3 Repair kit |
| 10 | bounty_hunter | sector_0_6_2_cluster_0_4_1_station_0 | 0 | 0 | Recover 2 Water ice from Fenda derelict | Neutralize Fenda raider | Answer a distress call |
| 10 | military | sector_0_6_2_cluster_0_4_1_station_0 | 0 | 0 | Escort a hauler carrying 1,692 cr | Rearm a stranded ship with 1 Torpedo | Neutralize Fenda raider |
| 10 | pirate | sector_0_6_2_cluster_0_4_1_station_0 | 0 | 0 | Recover 2 Water ice from Fenda derelict | Neutralize Fenda raider | Neutralize Fenda raider |
| 17 | industrial | sector_0_0_0_cluster_0_0_0_station_0 | 0 | 0 | Recover 14 Rations from Wenelnor derelict | Bring 28 Water ice | Answer a distress call |
| 17 | scavenger | sector_0_0_0_cluster_0_0_0_station_0 | 0 | 0 | Recover 14 Rations from Wenelnor derelict | Recover 2 Salvaged parts from Wenelnor derelict | Recover 21 Rations from Wenelnor derelict |
| 17 | transport | sector_0_0_0_cluster_0_0_0_station_0 | 0 | 0 | Answer a distress call | Recover 2 Salvaged parts from Wenelnor derelict | Answer a distress call |
| 17 | bounty_hunter | sector_0_0_0_cluster_0_0_0_station_0 | 0 | 0 | Escort a hauler carrying 594 cr | Recover 2 Salvaged parts from Wenelnor derelict | Answer a distress call |
| 17 | military | sector_0_0_0_cluster_0_0_0_station_0 | 0 | 0 | Escort a hauler carrying 594 cr | Escort a hauler carrying 1,194 cr | Answer a distress call |
| 17 | pirate | sector_0_0_0_cluster_0_0_0_station_0 | 0 | 0 | Answer a distress call | Recover 2 Salvaged parts from Wenelnor derelict | Neutralize Wenelnor raider |

## Cross-role summary

| role | hull cr | kit stock cr | credits | accel m/s2 | hold kg | weapons | fire s | rows that fit (mean) | boards with 0 rows that fit | Recover rows with a spawned target | Neutralize rows with a spawned target | first board has a row that pays | first board has a goal row | first board has a goal row that pays | no goal row in 5 postings | first payout s (median) | greedy gain cr at 15 min (quartiles) | greedy jobs (median) | greedy seeds with no job | greedy goal at 15 min (median / need) | Escort and Neutralize share of greedy pay |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| industrial | 5060 | 1280 | 2200 | 58.2 | 2100 | 0 | - | 3.00 | 0.0% | 88.3% | - | 99.2% | 64.2% | 59.2% | 22.9% | 177 | 2894 / 3521 / 4010 | 5 | 0.8% | 1 / 3 | 0.0% |
| scavenger | 10560 | 1180 | 1500 | 46.3 | 5400 | 1 | 35 | 3.00 | 0.0% | 90.0% | 81.9% | 100.0% | 87.9% | 87.9% | 0.0% | 233 | 3622 / 4210 / 4958 | 4 | 0.0% | 2 / 3 | 26.1% |
| transport | 7580 | 1880 | 2200 | 62.7 | 3700 | 0 | - | 3.00 | 0.0% | 86.4% | - | 98.3% | 97.1% | 80.8% | 0.0% | 225 | 2466 / 3280 / 3730 | 4 | 1.7% | 2 / 3 | 0.0% |
| bounty_hunter | 6920 | 1180 | 1800 | 77.6 | 2800 | 2 | 20 | 3.00 | 0.0% | 88.5% | 85.4% | 99.2% | 67.5% | 67.5% | 0.0% | 225 | 4016 / 4532 / 5080 | 4 | 0.8% | 2 / 3 | 76.5% |
| military | 6810 | 1980 | 1800 | 51.5 | 2300 | 3 | 20 | 3.00 | 0.0% | - | 85.1% | 98.8% | 93.8% | 90.8% | 0.0% | 241 | 3619 / 4016 / 4457 | 3 | 1.2% | 2 / 3 | 83.5% |
| pirate | 6780 | 1180 | 1800 | 62.1 | 2900 | 2 | 20 | 3.00 | 0.0% | 89.3% | 85.9% | 100.0% | 14.2% | 14.2% | 83.3% | 225 | 4132 / 4485 / 5243 | 4 | 0.0% | 3 / 3 | 61.6% |

## Band check

A role median is in band between 0.75 and 1.33 times the median of the six role medians: gain 4113 cr, first payout 225 s. The goal count is in band at 1 or 2. At most 5% of seeds may hand in no job.

| role | median gain cr at 15 min | x median | | median first payout s | x median | | median goal count at 15 min | | seeds with no job | |
|---|---|---|---|---|---|---|---|---|---|---|
| industrial | 3521 | 0.86 | ok | 177 | 0.79 | ok | 1 / 3 | ok | 0.8% | ok |
| scavenger | 4210 | 1.02 | ok | 233 | 1.03 | ok | 2 / 3 | ok | 0.0% | ok |
| transport | 3280 | 0.80 | ok | 225 | 1.00 | ok | 2 / 3 | ok | 1.7% | ok |
| bounty_hunter | 4532 | 1.10 | ok | 225 | 1.00 | ok | 2 / 3 | ok | 0.8% | ok |
| military | 4016 | 0.98 | ok | 241 | 1.07 | ok | 2 / 3 | ok | 1.2% | ok |
| pirate | 4485 | 1.09 | ok | 225 | 1.00 | ok | 3 / 3 | OUT | 0.0% | ok |
