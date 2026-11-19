# Start-role census

Written by `crates/nova_authoring/tests/start_role_census.rs`. Do not edit the tables by hand: run the tool again.

Seeds 1 to 240: 240 measured, 0 refused by `starting_station`. 5 postings of each of the 3 slots, so 15 board rows a seed and role.

## What is real and what is modelled

Real, called and not copied:

- Base content from the Rust builders: items, sections, ship designs, start roles, the price table, trade profiles, and the base ship part packs.
- The world generator (`NovaLayeredWorld`) and the start pick (`starting_station`).
- The board pass `post_station_boards`, run as a system: station order, the 27-cell cube gate, the field rule (`FieldBodies`), the bodies other boards name, the draw (`station_job_offers`, `SlotDraw`), the fallback to Procure, the assured-fit rule of the starter board, rewards and renewals.
- The hull gate (`PlayerHull::fit`, `plan_job_hull_fit`), the accept rule (`plan_job_accept`), the hand-in (`plan_job_hand_in`), trades (`plan_item_trade`, `price`), and the goal count (`goal_progress`).
- Hull numbers: `resolve_ship_design`, `ShipMeasure`, `hull_price_cr`, `cargo_capacity_g`, item reference prices.

Modelled:

- No game runs. Each cell of the 125-cell window around the player's spawn is mirrored from its real manifest: a root, each station (services, market, dead or living) and each ship (hold, side, derelict or intact, live weapon sections). Rocks, planets and parked sale hulls are not mirrored; the board pass reads none of them.
- Streaming: one cell a frame, nearest to the spawn cell first. The game prepares cells on worker threads, so its order can differ between cells at equal distance.
- A row counts as shown when the hull gate passes. That is the starter-board rule of the Jobs pane (`board_listing` in `crates/nova_interface/src/jobs.rs`), stated here and not called: this crate does not depend on the interface crate.
- Postings 2 to 5 of a slot are window renewals: the play clock steps past the longer offer window (3601 s; Procure and the rest stand 1200 s, Escort 3600 s), no job taken, so every board of the window renews, the start board whole. Kit, credits and station stock stay as at the start for every posting.
- Visit postings, scenario postings, hails and encounters do not exist here. No stance changes. No restock.
- Goal: a row counts when `goal_progress` over that one job alone moves a line other than `Stations owned`. Stations owned is 0 and the outlaw facts are (0, 0).

## Paper time

`t(d) = 2 * sqrt(d / a)`: flip and burn over `d` metres from rest to rest, `a` the hull's acceleration. `a = thrust / (mass * tick) * 10 m`, thrust and mass from `ShipMeasure` over the start hull's sections, tick 0.015625 s. Assumptions: every drive pushes along the flight axis, cargo adds no mass, no speed limit, no turn time.

One dock costs 60 s (`DOCK_ALLOWANCE_S`, an assumption, not a measurement).

| kind | paper seconds from accept to payout |
|---|---|
| Procure | 0 when the kit holds the stack or the station sells it and the player can pay and stow it; else not doable (mining and trade elsewhere are not modelled) |
| Supply | 2 t(leg) + 2 docks, with the stack from the kit or the station; else not doable |
| Recover | 2 t(leg) + 2 docks; not doable when the stack does not fit the free hold |
| Distress | 2 t(leg) + 2 docks |
| Escort | leg / 100 m/s (the hauler's cruise, `HAULER_CRUISE`) + t(leg) + 1 dock |
| Neutralize | 2 t(leg) + 1 dock |

No combat time, no losses, no ammunition spent: an Escort and a Neutralize cost only their legs. Net pay is the reward less the goods bought at the station's ask; a stack from the kit costs 0. A recovered stack stays with the player and is not in net pay.

## Greedy 15-minute model

The player docks once at the start (60 s). Then, until 900 paper seconds: from the current board take the row with the best net pay per paper second among the rows that fit the hull, pass the accept rule, can be done, pay more than their goods, end inside the horizon and that the station has credits for. A row that costs 0 s goes before any timed row, the larger net pay first. The job is accepted, done, and handed in through the real plans; goods are bought through the real trade plan and leave the station's stock; the station's credits pay the reward. The handed-in slot renews on the next board pass, the other slots stand. A Recover empties its stack off the wreck and the player sells the stack to the station at its bid when the market takes it; that sale is listed apart. A Neutralize marks its hull neutralized. A run stops at 100 jobs. Escort and Neutralize income is listed apart because the model gives them no combat time and no losses.

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

Hull `mining_tug`, job mix Trade, goal "Ore to market" (OreDeliveries { jobs: 6 }).

| start worth | cr |
|---|---|
| hull (every section's recipe at reference ask) | 5060 |
| kit stock at reference ask | 1160 |
| credits | 2200 |
| total | 8420 |

| hull | value |
|---|---|
| sections | 28 |
| mass, cells | 33.0 |
| thrust, impulse per tick | 3.00 |
| acceleration, m/s2 | 58.2 |
| t(10 km), s | 26.2 |
| health sum | 1840 |
| hold, kg | 2100 |
| kit mass, kg | 260 |
| docking sections | 1 |
| turrets / torpedo bays / railguns / mining beams | 0 / 0 / 0 / 1 |
| PDC rounds in the kit | 0 |
| PDC rounds in the magazines | 0 |
| PDC rounds a second, every turret | 0.0 |
| fire seconds, kit and magazines | - |

| first board | value |
|---|---|
| rows shown, mean of 3 | 1.95 |
| boards with 0 / 1 / 2 / 3 rows shown | 0.0% / 27.9% / 49.6% / 22.5% |
| cells live when the board posts, min / median / max | 27 / 27 / 77 |
| seeds with a wreck the draw can name | 12.9% |
| wrecks when any, median; nearest wreck m, median | 2; 47700 |
| seeds with a hostile the draw can name | 16.7% |
| hostiles when any, median; nearest hostile m, median | 2; 49200 |
| seeds with a wreck in the cube, named by another board or not | 12.9% |
| seeds with a hostile in the cube, named by another board or not | 16.7% |

| kind share | value |
|---|---|
| drawn, 5 postings | Procure 32.9%, Recover 8.4%, Escort 34.1%, Distress 7.6%, Neutralize 8.3%, Supply 8.7% |
| posted, first board | Procure 47.5%, Recover 0.7%, Escort 33.5%, Distress 7.5%, Neutralize 1.7%, Supply 9.2% |
| posted, 5 postings | Procure 48.7%, Recover 0.9%, Escort 32.7%, Distress 7.6%, Neutralize 1.4%, Supply 8.7% |
| posted and shown, first board | Procure 73.2%, Recover 1.1%, Escort 0.0%, Distress 11.6%, Neutralize 0.0%, Supply 14.1% |
| posted and shown, 5 postings | Procure 74.0%, Recover 1.3%, Escort 0.0%, Distress 11.6%, Neutralize 0.0%, Supply 13.2% |

| draw outcome, 5 postings | value |
|---|---|
| Recover draws that fell back to Procure | 272 of 303 (89.8%) |
| Neutralize draws that fell back to Procure | 245 of 299 (81.9%) |
| rows the assured-fit rule replaced | 51 of 3600 (1.4%) |

Rows by posted kind, 5 postings. Reward, cost, net pay, paper seconds, leg and threat are min / median / max; cost, net pay and paper seconds over the rows that fit and can be done.

| kind | posted | fit | doable | pay | goal | reward cr | goods cost cr | net pay cr | paper s | leg m | threat |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Procure | 1754 | 1754 | 99 | 99 | 1352 | 62 / 262 / 5000 | 0 / 52 / 424 | 82 / 328 / 740 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| Recover | 31 | 31 | 31 | 31 | 0 | 242 / 1844 / 2937 | 0 / 0 / 0 | 242 / 1844 / 2937 | 145 / 236 / 266 | 2300 / 49200 / 77800 | 0 / 0 / 0 |
| Escort | 1178 | 0 | 0 | 0 | 0 | 952 / 1387 / 1840 | - | - | - | 10000 / 14000 / 16000 | 1 / 1 / 1 |
| Distress | 274 | 274 | 274 | 274 | 0 | 437 / 697 / 948 | 0 / 0 / 0 | 437 / 697 / 948 | 161 / 177 / 186 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| Neutralize | 51 | 0 | 0 | 0 | 0 | 434 / 2309 / 3744 | - | - | - | 1200 / 54700 / 87100 | 0 / 0 / 2 |
| Supply | 312 | 312 | 209 | 209 | 0 | 463 / 902 / 2297 | 0 / 0 / 496 | 466 / 838 / 1441 | 161 / 177 / 186 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| all | 3600 | 2371 | 613 | 613 | 1352 | 62 / 760 / 5000 | 0 / 0 / 496 | 82 / 719 / 2937 | 0 / 172 / 266 | 0 / 6000 / 87100 | 0 / 0 / 2 |

| goods of the rows that fit, 5 postings | Procure | Supply |
|---|---|---|
| rows | 1754 | 312 |
| kit holds the stack | 2.3% | 35.9% |
| station stocks the stack | 4.2% | 55.4% |
| filled from the kit | 2.3% | 35.9% |
| bought at the station | 3.3% | 31.1% |
| not to be had while docked | 94.4% | 33.0% |
| stack at the station's ask cr, min / median / max | 4 / 36 / 2240 | 9 / 124 / 1048 |

Recover rows that fit: 31; stack has room in the free hold: 100.0%; the stack at the station's bid, min / median / max: 6 / 180 / 513 cr (kept by the player, not in net pay).

| first board, the rows that pay | value |
|---|---|
| seeds with a row that pays | 41.2% |
| best row by net pay per paper minute: net pay cr, median | 730 |
| best row: paper minutes, median | 3 |
| best row costs 0 s (a docked Procure) | 18.2% |
| best row kind | Procure 18.2%, Recover 4.0%, Escort 0.0%, Distress 40.4%, Neutralize 0.0%, Supply 37.4% |
| quickest row: paper seconds, median | 172 |
| quickest row: seconds to first payout with the start dock, median | 232 |
| quickest row: net pay cr, median | 680 |

| goal, rows that fit | value |
|---|---|
| the goal asks for | 6 |
| seeds whose first board has a goal row | 67.9% |
| seeds whose first board has a goal row that pays | 0.0% |
| goal rows that pay, of the goal rows that fit, 5 postings | 0 of 1352 (0.0%) |
| seeds with no goal row in 5 postings | 22.9% |
| goal rows per 15 board rows, lower quartile / median / upper quartile | 4 / 6 / 8 |
| goal count with every fitting row of the 15 handed in, quartiles | 4 / 6 / 8 |

| greedy 15-minute model | value |
|---|---|
| credits at the start | 2200 |
| credits at 15 min, quartiles | 2200 / 2200 / 3021 |
| gain cr at 15 min, quartiles | 0 / 0 / 821 |
| gain cr at 15 min, min / median / max | 0 / 0 / 6897 |
| jobs handed in, quartiles | 0 / 0 / 1 |
| seeds with no job handed in | 58.8% |
| first payout, paper s, quartiles | 221 / 237 / 242 |
| last payout or stop, paper s, quartiles | 60 / 60 / 237 |
| goal count at 15 min of 6, quartiles | 0 / 0 / 0 |
| net job pay by kind | Procure 7.9%, Recover 14.7%, Escort 0.0%, Distress 36.8%, Neutralize 0.0%, Supply 40.7% |
| Escort and Neutralize share of net job pay | 0.0% |
| docked 0 s rows: share of jobs / of net job pay | 18.0% / 7.9% |
| sale of recovered stacks cr, quartiles | 0 / 0 / 0 |
| why the run stopped | goods not to be had or no hold room 99.2%, next row ends past the horizon 0.8% |

## scavenger (Scavenger)

Hull `salvage_hauler`, job mix Salvage, goal "A salvage run" (Recoveries { jobs: 6 }).

| start worth | cr |
|---|---|
| hull (every section's recipe at reference ask) | 10560 |
| kit stock at reference ask | 1060 |
| credits | 1800 |
| total | 13420 |

| hull | value |
|---|---|
| sections | 65 |
| mass, cells | 69.1 |
| thrust, impulse per tick | 5.00 |
| acceleration, m/s2 | 46.3 |
| t(10 km), s | 29.4 |
| health sum | 4190 |
| hold, kg | 5400 |
| kit mass, kg | 760 |
| docking sections | 1 |
| turrets / torpedo bays / railguns / mining beams | 1 / 0 / 0 / 1 |
| PDC rounds in the kit | 3000 |
| PDC rounds in the magazines | 500 |
| PDC rounds a second, every turret | 100.0 |
| fire seconds, kit and magazines | 35 |

| first board | value |
|---|---|
| rows shown, mean of 3 | 3.00 |
| boards with 0 / 1 / 2 / 3 rows shown | 0.0% / 0.0% / 0.0% / 100.0% |
| cells live when the board posts, min / median / max | 27 / 27 / 77 |
| seeds with a wreck the draw can name | 12.9% |
| wrecks when any, median; nearest wreck m, median | 2; 47700 |
| seeds with a hostile the draw can name | 16.7% |
| hostiles when any, median; nearest hostile m, median | 2; 49200 |
| seeds with a wreck in the cube, named by another board or not | 12.9% |
| seeds with a hostile in the cube, named by another board or not | 16.7% |

| kind share | value |
|---|---|
| drawn, 5 postings | Procure 7.8%, Recover 42.4%, Escort 8.3%, Distress 17.0%, Neutralize 7.6%, Supply 17.0% |
| posted, first board | Procure 53.8%, Recover 4.2%, Escort 8.8%, Distress 15.8%, Neutralize 0.7%, Supply 16.8% |
| posted, 5 postings | Procure 52.5%, Recover 4.2%, Escort 8.3%, Distress 17.0%, Neutralize 1.0%, Supply 17.0% |
| posted and shown, first board | Procure 53.8%, Recover 4.2%, Escort 8.8%, Distress 15.8%, Neutralize 0.7%, Supply 16.8% |
| posted and shown, 5 postings | Procure 52.5%, Recover 4.2%, Escort 8.3%, Distress 17.0%, Neutralize 1.0%, Supply 17.0% |

| draw outcome, 5 postings | value |
|---|---|
| Recover draws that fell back to Procure | 1374 of 1525 (90.1%) |
| Neutralize draws that fell back to Procure | 237 of 274 (86.5%) |
| rows the assured-fit rule replaced | 0 of 3600 (0.0%) |

Rows by posted kind, 5 postings. Reward, cost, net pay, paper seconds, leg and threat are min / median / max; cost, net pay and paper seconds over the rows that fit and can be done.

| kind | posted | fit | doable | pay | goal | reward cr | goods cost cr | net pay cr | paper s | leg m | threat |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Procure | 1890 | 1890 | 95 | 95 | 0 | 62 / 260 / 5050 | 0 / 52 / 424 | 59 / 311 / 740 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| Recover | 151 | 151 | 151 | 151 | 151 | 242 / 1844 / 3379 | 0 / 0 / 0 | 242 / 1844 / 3379 | 143 / 250 / 288 | 1500 / 49200 / 81400 | 0 / 0 / 0 |
| Escort | 300 | 300 | 300 | 300 | 0 | 952 / 1387 / 1810 | 0 / 0 / 0 | 952 / 1387 / 1810 | 189 / 212 / 257 | 10000 / 12000 / 16000 | 1 / 1 / 1 |
| Distress | 611 | 611 | 611 | 611 | 0 | 430 / 699 / 963 | 0 / 0 / 0 | 430 / 699 / 963 | 166 / 184 / 194 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| Neutralize | 37 | 37 | 37 | 37 | 0 | 413 / 2079 / 3385 | 0 / 0 / 0 | 413 / 2079 / 3385 | 80.4 / 191 / 230 | 1200 / 49400 / 84100 | 0 / 0 / 2 |
| Supply | 611 | 611 | 401 | 401 | 0 | 463 / 907 / 2297 | 0 / 54 / 496 | 442 / 821 / 1441 | 166 / 184 / 194 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| all | 3600 | 3600 | 1595 | 1595 | 151 | 62 / 589 / 5050 | 0 / 0 / 496 | 59 / 794 / 3385 | 0 / 184 / 288 | 0 / 0 / 84100 | 0 / 0 / 2 |

| goods of the rows that fit, 5 postings | Procure | Supply |
|---|---|---|
| rows | 1890 | 611 |
| kit holds the stack | 1.6% | 17.8% |
| station stocks the stack | 3.8% | 56.1% |
| filled from the kit | 1.6% | 17.8% |
| bought at the station | 3.4% | 47.8% |
| not to be had while docked | 95.0% | 34.4% |
| stack at the station's ask cr, min / median / max | 4 / 35 / 2240 | 4 / 124 / 1048 |

Recover rows that fit: 151; stack has room in the free hold: 100.0%; the stack at the station's bid, min / median / max: 6 / 119 / 1092 cr (kept by the player, not in net pay).

| first board, the rows that pay | value |
|---|---|
| seeds with a row that pays | 82.5% |
| best row by net pay per paper minute: net pay cr, median | 893 |
| best row: paper minutes, median | 3.2 |
| best row costs 0 s (a docked Procure) | 9.6% |
| best row kind | Procure 9.6%, Recover 10.1%, Escort 24.2%, Distress 30.3%, Neutralize 2.0%, Supply 23.7% |
| quickest row: paper seconds, median | 184 |
| quickest row: seconds to first payout with the start dock, median | 244 |
| quickest row: net pay cr, median | 749 |

| goal, rows that fit | value |
|---|---|
| the goal asks for | 6 |
| seeds whose first board has a goal row | 9.6% |
| seeds whose first board has a goal row that pays | 9.6% |
| goal rows that pay, of the goal rows that fit, 5 postings | 151 of 151 (100.0%) |
| seeds with no goal row in 5 postings | 87.1% |
| goal rows per 15 board rows, lower quartile / median / upper quartile | 0 / 0 / 0 |
| goal count with every fitting row of the 15 handed in, quartiles | 0 / 0 / 0 |

| greedy 15-minute model | value |
|---|---|
| credits at the start | 1800 |
| credits at 15 min, quartiles | 2461 / 3373 / 5032 |
| gain cr at 15 min, quartiles | 661 / 1573 / 3232 |
| gain cr at 15 min, min / median / max | 0 / 1573 / 8747 |
| jobs handed in, quartiles | 1 / 2 / 3 |
| seeds with no job handed in | 17.5% |
| first payout, paper s, quartiles | 239 / 250 / 254 |
| last payout or stop, paper s, quartiles | 233 / 410 / 701 |
| goal count at 15 min of 6, quartiles | 0 / 0 / 0 |
| net job pay by kind | Procure 3.0%, Recover 19.3%, Escort 25.3%, Distress 25.5%, Neutralize 4.0%, Supply 23.0% |
| Escort and Neutralize share of net job pay | 29.2% |
| docked 0 s rows: share of jobs / of net job pay | 8.9% / 3.0% |
| sale of recovered stacks cr, quartiles | 0 / 0 / 0 |
| why the run stopped | goods not to be had or no hold room 83.3%, next row ends past the horizon 16.7% |

## transport (Transport)

Hull `light_freighter`, job mix Trade, goal "A station network" (StationNetwork { stations: 4 }).

| start worth | cr |
|---|---|
| hull (every section's recipe at reference ask) | 7580 |
| kit stock at reference ask | 1360 |
| credits | 2000 |
| total | 10940 |

| hull | value |
|---|---|
| sections | 46 |
| mass, cells | 51.0 |
| thrust, impulse per tick | 5.00 |
| acceleration, m/s2 | 62.7 |
| t(10 km), s | 25.2 |
| health sum | 2950 |
| hold, kg | 3700 |
| kit mass, kg | 660 |
| docking sections | 1 |
| turrets / torpedo bays / railguns / mining beams | 0 / 0 / 0 / 0 |
| PDC rounds in the kit | 0 |
| PDC rounds in the magazines | 0 |
| PDC rounds a second, every turret | 0.0 |
| fire seconds, kit and magazines | - |

| first board | value |
|---|---|
| rows shown, mean of 3 | 1.95 |
| boards with 0 / 1 / 2 / 3 rows shown | 0.0% / 27.9% / 49.6% / 22.5% |
| cells live when the board posts, min / median / max | 27 / 27 / 77 |
| seeds with a wreck the draw can name | 12.9% |
| wrecks when any, median; nearest wreck m, median | 2; 47700 |
| seeds with a hostile the draw can name | 16.7% |
| hostiles when any, median; nearest hostile m, median | 2; 49200 |
| seeds with a wreck in the cube, named by another board or not | 12.9% |
| seeds with a hostile in the cube, named by another board or not | 16.7% |

| kind share | value |
|---|---|
| drawn, 5 postings | Procure 32.9%, Recover 8.4%, Escort 34.1%, Distress 7.6%, Neutralize 8.3%, Supply 8.7% |
| posted, first board | Procure 47.5%, Recover 0.7%, Escort 33.5%, Distress 7.5%, Neutralize 1.7%, Supply 9.2% |
| posted, 5 postings | Procure 48.7%, Recover 0.9%, Escort 32.7%, Distress 7.6%, Neutralize 1.4%, Supply 8.7% |
| posted and shown, first board | Procure 73.2%, Recover 1.1%, Escort 0.0%, Distress 11.6%, Neutralize 0.0%, Supply 14.1% |
| posted and shown, 5 postings | Procure 74.0%, Recover 1.3%, Escort 0.0%, Distress 11.6%, Neutralize 0.0%, Supply 13.2% |

| draw outcome, 5 postings | value |
|---|---|
| Recover draws that fell back to Procure | 272 of 303 (89.8%) |
| Neutralize draws that fell back to Procure | 245 of 299 (81.9%) |
| rows the assured-fit rule replaced | 51 of 3600 (1.4%) |

Rows by posted kind, 5 postings. Reward, cost, net pay, paper seconds, leg and threat are min / median / max; cost, net pay and paper seconds over the rows that fit and can be done.

| kind | posted | fit | doable | pay | goal | reward cr | goods cost cr | net pay cr | paper s | leg m | threat |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Procure | 1754 | 1754 | 423 | 423 | 1754 | 62 / 262 / 5000 | 0 / 0 / 424 | 82 / 250 / 740 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| Recover | 31 | 31 | 31 | 31 | 0 | 242 / 1844 / 2937 | 0 / 0 / 0 | 242 / 1844 / 2937 | 144 / 232 / 261 | 2300 / 49200 / 77800 | 0 / 0 / 0 |
| Escort | 1178 | 0 | 0 | 0 | 1178 | 952 / 1387 / 1840 | - | - | - | 10000 / 14000 / 16000 | 1 / 1 / 1 |
| Distress | 274 | 274 | 274 | 274 | 0 | 437 / 697 / 948 | 0 / 0 / 0 | 437 / 697 / 948 | 159 / 175 / 184 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| Neutralize | 51 | 0 | 0 | 0 | 0 | 434 / 2309 / 3744 | - | - | - | 1200 / 54700 / 87100 | 0 / 0 / 2 |
| Supply | 312 | 312 | 204 | 204 | 0 | 463 / 902 / 2297 | 0 / 20 / 496 | 466 / 797 / 1441 | 159 / 170 / 184 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| all | 3600 | 2371 | 932 | 932 | 2932 | 62 / 760 / 5000 | 0 / 0 / 496 | 82 / 546 / 2937 | 0 / 159 / 261 | 0 / 6000 / 87100 | 0 / 0 / 2 |

| goods of the rows that fit, 5 postings | Procure | Supply |
|---|---|---|
| rows | 1754 | 312 |
| kit holds the stack | 20.5% | 32.4% |
| station stocks the stack | 4.2% | 55.4% |
| filled from the kit | 20.5% | 32.4% |
| bought at the station | 3.6% | 33.0% |
| not to be had while docked | 75.9% | 34.6% |
| stack at the station's ask cr, min / median / max | 4 / 36 / 2240 | 9 / 124 / 1048 |

Recover rows that fit: 31; stack has room in the free hold: 100.0%; the stack at the station's bid, min / median / max: 6 / 180 / 513 cr (kept by the player, not in net pay).

| first board, the rows that pay | value |
|---|---|
| seeds with a row that pays | 60.4% |
| best row by net pay per paper minute: net pay cr, median | 376 |
| best row: paper minutes, median | 0 |
| best row costs 0 s (a docked Procure) | 51.7% |
| best row kind | Procure 51.7%, Recover 2.1%, Escort 0.0%, Distress 24.8%, Neutralize 0.0%, Supply 21.4% |
| quickest row: paper seconds, median | 0 |
| quickest row: seconds to first payout with the start dock, median | 60 |
| quickest row: net pay cr, median | 376 |

| goal, rows that fit | value |
|---|---|
| the goal asks for | 4 |
| seeds whose first board has a goal row | 88.8% |
| seeds whose first board has a goal row that pays | 31.2% |
| goal rows that pay, of the goal rows that fit, 5 postings | 423 of 1754 (24.1%) |
| seeds with no goal row in 5 postings | 0.0% |
| goal rows per 15 board rows, lower quartile / median / upper quartile | 6 / 7 / 9 |
| goal count with every fitting row of the 15 handed in, quartiles | 1 / 1 / 1 |

| greedy 15-minute model | value |
|---|---|
| credits at the start | 2000 |
| credits at 15 min, quartiles | 2000 / 2258 / 2870 |
| gain cr at 15 min, quartiles | 0 / 258 / 870 |
| gain cr at 15 min, min / median / max | 0 / 258 / 5681 |
| jobs handed in, quartiles | 0 / 1 / 1 |
| seeds with no job handed in | 39.6% |
| first payout, paper s, quartiles | 60 / 60 / 235 |
| last payout or stop, paper s, quartiles | 60 / 60 / 235 |
| goal count at 15 min of 4, quartiles | 0 / 0 / 1 |
| net job pay by kind | Procure 20.3%, Recover 10.3%, Escort 0.0%, Distress 36.1%, Neutralize 0.0%, Supply 33.3% |
| Escort and Neutralize share of net job pay | 0.0% |
| docked 0 s rows: share of jobs / of net job pay | 45.6% / 20.3% |
| sale of recovered stacks cr, quartiles | 0 / 0 / 0 |
| why the run stopped | goods not to be had or no hold room 99.6%, next row ends past the horizon 0.4% |

## bounty_hunter (Bounty Hunter)

Hull `interceptor`, job mix Combat, goal "A kill list" (KillList { hulls: 6 }).

| start worth | cr |
|---|---|
| hull (every section's recipe at reference ask) | 6640 |
| kit stock at reference ask | 1060 |
| credits | 1600 |
| total | 9300 |

| hull | value |
|---|---|
| sections | 38 |
| mass, cells | 41.2 |
| thrust, impulse per tick | 3.00 |
| acceleration, m/s2 | 46.5 |
| t(10 km), s | 29.3 |
| health sum | 2550 |
| hold, kg | 3000 |
| kit mass, kg | 760 |
| docking sections | 1 |
| turrets / torpedo bays / railguns / mining beams | 2 / 0 / 0 / 0 |
| PDC rounds in the kit | 3000 |
| PDC rounds in the magazines | 1000 |
| PDC rounds a second, every turret | 200.0 |
| fire seconds, kit and magazines | 20 |

| first board | value |
|---|---|
| rows shown, mean of 3 | 3.00 |
| boards with 0 / 1 / 2 / 3 rows shown | 0.0% / 0.0% / 0.0% / 100.0% |
| cells live when the board posts, min / median / max | 27 / 27 / 77 |
| seeds with a wreck the draw can name | 12.9% |
| wrecks when any, median; nearest wreck m, median | 2; 47700 |
| seeds with a hostile the draw can name | 16.7% |
| hostiles when any, median; nearest hostile m, median | 2; 49200 |
| seeds with a wreck in the cube, named by another board or not | 12.9% |
| seeds with a hostile in the cube, named by another board or not | 16.7% |

| kind share | value |
|---|---|
| drawn, 5 postings | Procure 7.2%, Recover 7.8%, Escort 7.6%, Distress 23.7%, Neutralize 30.6%, Supply 23.1% |
| posted, first board | Procure 39.2%, Recover 1.8%, Escort 7.1%, Distress 24.3%, Neutralize 4.7%, Supply 22.9% |
| posted, 5 postings | Procure 40.1%, Recover 1.1%, Escort 7.6%, Distress 23.7%, Neutralize 4.4%, Supply 23.1% |
| posted and shown, first board | Procure 39.2%, Recover 1.8%, Escort 7.1%, Distress 24.3%, Neutralize 4.7%, Supply 22.9% |
| posted and shown, 5 postings | Procure 40.1%, Recover 1.1%, Escort 7.6%, Distress 23.7%, Neutralize 4.4%, Supply 23.1% |

| draw outcome, 5 postings | value |
|---|---|
| Recover draws that fell back to Procure | 240 of 280 (85.7%) |
| Neutralize draws that fell back to Procure | 943 of 1103 (85.5%) |
| rows the assured-fit rule replaced | 0 of 3600 (0.0%) |

Rows by posted kind, 5 postings. Reward, cost, net pay, paper seconds, leg and threat are min / median / max; cost, net pay and paper seconds over the rows that fit and can be done.

| kind | posted | fit | doable | pay | goal | reward cr | goods cost cr | net pay cr | paper s | leg m | threat |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Procure | 1443 | 1443 | 73 | 73 | 0 | 62 / 258 / 5000 | 0 / 52 / 424 | 70 / 274 / 680 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| Recover | 40 | 40 | 40 | 40 | 0 | 310 / 1783 / 3329 | 0 / 0 / 0 | 310 / 1783 / 3329 | 143 / 245 / 287 | 1500 / 45500 / 81400 | 0 / 0 / 0 |
| Escort | 273 | 273 | 273 | 273 | 0 | 1004 / 1407 / 1783 | 0 / 0 / 0 | 1004 / 1407 / 1783 | 189 / 235 / 257 | 10000 / 14000 / 16000 | 1 / 1 / 1 |
| Distress | 853 | 853 | 853 | 853 | 0 | 430 / 683 / 963 | 0 / 0 / 0 | 430 / 683 / 963 | 165 / 179 / 194 | 6000 / 10000 / 16000 | 0 / 0 / 0 |
| Neutralize | 160 | 160 | 160 | 160 | 160 | 434 / 2104 / 3644 | 0 / 0 / 0 | 434 / 2104 / 3644 | 80.3 / 190 / 233 | 1200 / 49400 / 87100 | 0 / 0 / 2 |
| Supply | 831 | 831 | 535 | 535 | 0 | 463 / 914 / 2297 | 0 / 54 / 504 | 442 / 819 / 1441 | 165 / 184 / 194 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| all | 3600 | 3600 | 1934 | 1934 | 160 | 62 / 670 / 5000 | 0 / 0 / 504 | 70 / 782 / 3644 | 0 / 184 / 287 | 0 / 8000 / 87100 | 0 / 0 / 2 |

| goods of the rows that fit, 5 postings | Procure | Supply |
|---|---|---|
| rows | 1443 | 831 |
| kit holds the stack | 1.7% | 18.3% |
| station stocks the stack | 3.8% | 55.0% |
| filled from the kit | 1.7% | 18.3% |
| bought at the station | 3.4% | 46.1% |
| not to be had while docked | 94.9% | 35.6% |
| stack at the station's ask cr, min / median / max | 4 / 35 / 2240 | 4 / 124 / 1048 |

Recover rows that fit: 40; stack has room in the free hold: 100.0%; the stack at the station's bid, min / median / max: 6 / 76 / 1092 cr (kept by the player, not in net pay).

| first board, the rows that pay | value |
|---|---|
| seeds with a row that pays | 88.3% |
| best row by net pay per paper minute: net pay cr, median | 854 |
| best row: paper minutes, median | 3.2 |
| best row costs 0 s (a docked Procure) | 6.6% |
| best row kind | Procure 6.6%, Recover 4.7%, Escort 17.9%, Distress 31.6%, Neutralize 12.3%, Supply 26.9% |
| quickest row: paper seconds, median | 179 |
| quickest row: seconds to first payout with the start dock, median | 239 |
| quickest row: net pay cr, median | 704 |

| goal, rows that fit | value |
|---|---|
| the goal asks for | 6 |
| seeds whose first board has a goal row | 10.8% |
| seeds whose first board has a goal row that pays | 10.8% |
| goal rows that pay, of the goal rows that fit, 5 postings | 160 of 160 (100.0%) |
| seeds with no goal row in 5 postings | 83.3% |
| goal rows per 15 board rows, lower quartile / median / upper quartile | 0 / 0 / 0 |
| goal count with every fitting row of the 15 handed in, quartiles | 0 / 0 / 0 |

| greedy 15-minute model | value |
|---|---|
| credits at the start | 1600 |
| credits at 15 min, quartiles | 2810 / 4060 / 5249 |
| gain cr at 15 min, quartiles | 1210 / 2460 / 3649 |
| gain cr at 15 min, min / median / max | 0 / 2460 / 7395 |
| jobs handed in, quartiles | 1 / 3 / 4 |
| seeds with no job handed in | 11.7% |
| first payout, paper s, quartiles | 232 / 249 / 254 |
| last payout or stop, paper s, quartiles | 272 / 601 / 801 |
| goal count at 15 min of 6, quartiles | 0 / 0 / 0 |
| net job pay by kind | Procure 1.2%, Recover 5.9%, Escort 20.8%, Distress 31.3%, Neutralize 16.4%, Supply 24.4% |
| Escort and Neutralize share of net job pay | 37.2% |
| docked 0 s rows: share of jobs / of net job pay | 4.1% / 1.2% |
| sale of recovered stacks cr, quartiles | 0 / 0 / 0 |
| why the run stopped | goods not to be had or no hold room 72.1%, next row ends past the horizon 27.9% |

## military (Military)

Hull `patrol_picket`, job mix Combat, goal "Patrol record" (Patrols { jobs: 6 }).

| start worth | cr |
|---|---|
| hull (every section's recipe at reference ask) | 5950 |
| kit stock at reference ask | 1860 |
| credits | 1000 |
| total | 8810 |

| hull | value |
|---|---|
| sections | 27 |
| mass, cells | 31.2 |
| thrust, impulse per tick | 2.00 |
| acceleration, m/s2 | 41.0 |
| t(10 km), s | 31.2 |
| health sum | 1950 |
| hold, kg | 1800 |
| kit mass, kg | 1060 |
| docking sections | 2 |
| turrets / torpedo bays / railguns / mining beams | 2 / 1 / 0 / 0 |
| PDC rounds in the kit | 3000 |
| PDC rounds in the magazines | 1000 |
| PDC rounds a second, every turret | 200.0 |
| fire seconds, kit and magazines | 20 |

| first board | value |
|---|---|
| rows shown, mean of 3 | 3.00 |
| boards with 0 / 1 / 2 / 3 rows shown | 0.0% / 0.0% / 0.0% / 100.0% |
| cells live when the board posts, min / median / max | 27 / 27 / 77 |
| seeds with a wreck the draw can name | 12.9% |
| wrecks when any, median; nearest wreck m, median | 2; 47700 |
| seeds with a hostile the draw can name | 16.7% |
| hostiles when any, median; nearest hostile m, median | 2; 49200 |
| seeds with a wreck in the cube, named by another board or not | 12.9% |
| seeds with a hostile in the cube, named by another board or not | 16.7% |

| kind share | value |
|---|---|
| drawn, 5 postings | Procure 7.2%, Recover 7.8%, Escort 7.6%, Distress 23.7%, Neutralize 30.6%, Supply 23.1% |
| posted, first board | Procure 39.2%, Recover 1.8%, Escort 7.1%, Distress 24.3%, Neutralize 4.7%, Supply 22.9% |
| posted, 5 postings | Procure 40.1%, Recover 1.1%, Escort 7.6%, Distress 23.7%, Neutralize 4.4%, Supply 23.1% |
| posted and shown, first board | Procure 39.2%, Recover 1.8%, Escort 7.1%, Distress 24.3%, Neutralize 4.7%, Supply 22.9% |
| posted and shown, 5 postings | Procure 40.1%, Recover 1.1%, Escort 7.6%, Distress 23.7%, Neutralize 4.4%, Supply 23.1% |

| draw outcome, 5 postings | value |
|---|---|
| Recover draws that fell back to Procure | 240 of 280 (85.7%) |
| Neutralize draws that fell back to Procure | 943 of 1103 (85.5%) |
| rows the assured-fit rule replaced | 0 of 3600 (0.0%) |

Rows by posted kind, 5 postings. Reward, cost, net pay, paper seconds, leg and threat are min / median / max; cost, net pay and paper seconds over the rows that fit and can be done.

| kind | posted | fit | doable | pay | goal | reward cr | goods cost cr | net pay cr | paper s | leg m | threat |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Procure | 1443 | 1443 | 73 | 73 | 0 | 62 / 258 / 5000 | 0 / 52 / 424 | 70 / 274 / 680 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| Recover | 40 | 40 | 40 | 40 | 0 | 310 / 1783 / 3329 | 0 / 0 / 0 | 310 / 1783 / 3329 | 144 / 253 / 298 | 1500 / 45500 / 81400 | 0 / 0 / 0 |
| Escort | 273 | 273 | 273 | 273 | 273 | 1004 / 1407 / 1783 | 0 / 0 / 0 | 1004 / 1407 / 1783 | 191 / 237 / 260 | 10000 / 14000 / 16000 | 1 / 1 / 1 |
| Distress | 853 | 853 | 853 | 853 | 853 | 430 / 683 / 963 | 0 / 0 / 0 | 430 / 683 / 963 | 168 / 182 / 199 | 6000 / 10000 / 16000 | 0 / 0 / 0 |
| Neutralize | 160 | 160 | 160 | 160 | 0 | 434 / 2104 / 3644 | 0 / 0 / 0 | 434 / 2104 / 3644 | 81.7 / 199 / 244 | 1200 / 49400 / 87100 | 0 / 0 / 2 |
| Supply | 831 | 831 | 535 | 535 | 831 | 463 / 914 / 2297 | 0 / 36 / 504 | 442 / 846 / 1544 | 168 / 188 / 199 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| all | 3600 | 3600 | 1934 | 1934 | 1957 | 62 / 670 / 5000 | 0 / 0 / 504 | 70 / 784 / 3644 | 0 / 188 / 298 | 0 / 8000 / 87100 | 0 / 0 / 2 |

| goods of the rows that fit, 5 postings | Procure | Supply |
|---|---|---|
| rows | 1443 | 831 |
| kit holds the stack | 1.7% | 24.2% |
| station stocks the stack | 3.8% | 55.0% |
| filled from the kit | 1.7% | 24.2% |
| bought at the station | 3.4% | 40.2% |
| not to be had while docked | 94.9% | 35.6% |
| stack at the station's ask cr, min / median / max | 4 / 35 / 2240 | 4 / 124 / 1048 |

Recover rows that fit: 40; stack has room in the free hold: 100.0%; the stack at the station's bid, min / median / max: 6 / 76 / 1092 cr (kept by the player, not in net pay).

| first board, the rows that pay | value |
|---|---|
| seeds with a row that pays | 88.3% |
| best row by net pay per paper minute: net pay cr, median | 870 |
| best row: paper minutes, median | 3.2 |
| best row costs 0 s (a docked Procure) | 6.6% |
| best row kind | Procure 6.6%, Recover 4.7%, Escort 17.9%, Distress 32.1%, Neutralize 11.3%, Supply 27.4% |
| quickest row: paper seconds, median | 182 |
| quickest row: seconds to first payout with the start dock, median | 242 |
| quickest row: net pay cr, median | 707 |

| goal, rows that fit | value |
|---|---|
| the goal asks for | 6 |
| seeds whose first board has a goal row | 92.1% |
| seeds whose first board has a goal row that pays | 86.2% |
| goal rows that pay, of the goal rows that fit, 5 postings | 1661 of 1957 (84.9%) |
| seeds with no goal row in 5 postings | 0.0% |
| goal rows per 15 board rows, lower quartile / median / upper quartile | 7 / 8 / 10 |
| goal count with every fitting row of the 15 handed in, quartiles | 7 / 8 / 10 |

| greedy 15-minute model | value |
|---|---|
| credits at the start | 1000 |
| credits at 15 min, quartiles | 2210 / 3504 / 4666 |
| gain cr at 15 min, quartiles | 1210 / 2504 / 3666 |
| gain cr at 15 min, min / median / max | 0 / 2504 / 7830 |
| jobs handed in, quartiles | 1 / 3 / 4 |
| seeds with no job handed in | 11.7% |
| first payout, paper s, quartiles | 239 / 251 / 259 |
| last payout or stop, paper s, quartiles | 274 / 611 / 815 |
| goal count at 15 min of 6, quartiles | 1 / 2 / 3 |
| net job pay by kind | Procure 1.2%, Recover 5.9%, Escort 20.2%, Distress 31.2%, Neutralize 16.3%, Supply 25.1% |
| Escort and Neutralize share of net job pay | 36.5% |
| docked 0 s rows: share of jobs / of net job pay | 4.3% / 1.2% |
| sale of recovered stacks cr, quartiles | 0 / 0 / 0 |
| why the run stopped | goods not to be had or no hold room 72.1%, next row ends past the horizon 27.9% |

## pirate (Pirate)

Hull `raider`, job mix Combat, goal "An outlaw's name" (Outlaw { hulls: 6 }).

| start worth | cr |
|---|---|
| hull (every section's recipe at reference ask) | 6780 |
| kit stock at reference ask | 1060 |
| credits | 1800 |
| total | 9640 |

| hull | value |
|---|---|
| sections | 38 |
| mass, cells | 41.2 |
| thrust, impulse per tick | 4.00 |
| acceleration, m/s2 | 62.1 |
| t(10 km), s | 25.4 |
| health sum | 2560 |
| hold, kg | 2900 |
| kit mass, kg | 760 |
| docking sections | 1 |
| turrets / torpedo bays / railguns / mining beams | 2 / 0 / 0 / 0 |
| PDC rounds in the kit | 3000 |
| PDC rounds in the magazines | 1000 |
| PDC rounds a second, every turret | 200.0 |
| fire seconds, kit and magazines | 20 |

| first board | value |
|---|---|
| rows shown, mean of 3 | 3.00 |
| boards with 0 / 1 / 2 / 3 rows shown | 0.0% / 0.0% / 0.0% / 100.0% |
| cells live when the board posts, min / median / max | 27 / 27 / 77 |
| seeds with a wreck the draw can name | 12.9% |
| wrecks when any, median; nearest wreck m, median | 2; 47700 |
| seeds with a hostile the draw can name | 16.7% |
| hostiles when any, median; nearest hostile m, median | 2; 49200 |
| seeds with a wreck in the cube, named by another board or not | 12.9% |
| seeds with a hostile in the cube, named by another board or not | 16.7% |

| kind share | value |
|---|---|
| drawn, 5 postings | Procure 7.2%, Recover 7.8%, Escort 7.6%, Distress 23.7%, Neutralize 30.6%, Supply 23.1% |
| posted, first board | Procure 39.2%, Recover 1.8%, Escort 7.1%, Distress 24.3%, Neutralize 4.7%, Supply 22.9% |
| posted, 5 postings | Procure 40.1%, Recover 1.1%, Escort 7.6%, Distress 23.7%, Neutralize 4.4%, Supply 23.1% |
| posted and shown, first board | Procure 39.2%, Recover 1.8%, Escort 7.1%, Distress 24.3%, Neutralize 4.7%, Supply 22.9% |
| posted and shown, 5 postings | Procure 40.1%, Recover 1.1%, Escort 7.6%, Distress 23.7%, Neutralize 4.4%, Supply 23.1% |

| draw outcome, 5 postings | value |
|---|---|
| Recover draws that fell back to Procure | 240 of 280 (85.7%) |
| Neutralize draws that fell back to Procure | 943 of 1103 (85.5%) |
| rows the assured-fit rule replaced | 0 of 3600 (0.0%) |

Rows by posted kind, 5 postings. Reward, cost, net pay, paper seconds, leg and threat are min / median / max; cost, net pay and paper seconds over the rows that fit and can be done.

| kind | posted | fit | doable | pay | goal | reward cr | goods cost cr | net pay cr | paper s | leg m | threat |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Procure | 1443 | 1443 | 73 | 73 | 0 | 62 / 258 / 5000 | 0 / 52 / 424 | 70 / 274 / 680 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| Recover | 40 | 40 | 40 | 40 | 0 | 310 / 1783 / 3329 | 0 / 0 / 0 | 310 / 1783 / 3329 | 140 / 228 / 265 | 1500 / 45500 / 81400 | 0 / 0 / 0 |
| Escort | 273 | 273 | 273 | 273 | 0 | 1004 / 1407 / 1783 | 0 / 0 / 0 | 1004 / 1407 / 1783 | 185 / 230 / 252 | 10000 / 14000 / 16000 | 1 / 1 / 1 |
| Distress | 853 | 853 | 853 | 853 | 0 | 430 / 683 / 963 | 0 / 0 / 0 | 430 / 683 / 963 | 159 / 171 / 184 | 6000 / 10000 / 16000 | 0 / 0 / 0 |
| Neutralize | 160 | 160 | 160 | 160 | 0 | 434 / 2104 / 3644 | 0 / 0 / 0 | 434 / 2104 / 3644 | 77.6 / 173 / 210 | 1200 / 49400 / 87100 | 0 / 0 / 2 |
| Supply | 831 | 831 | 535 | 535 | 0 | 463 / 914 / 2297 | 0 / 54 / 504 | 442 / 819 / 1441 | 159 / 176 / 184 | 6000 / 12000 / 16000 | 0 / 0 / 0 |
| all | 3600 | 3600 | 1934 | 1934 | 0 | 62 / 670 / 5000 | 0 / 0 / 504 | 70 / 782 / 3644 | 0 / 176 / 265 | 0 / 8000 / 87100 | 0 / 0 / 2 |

| goods of the rows that fit, 5 postings | Procure | Supply |
|---|---|---|
| rows | 1443 | 831 |
| kit holds the stack | 1.7% | 18.3% |
| station stocks the stack | 3.8% | 55.0% |
| filled from the kit | 1.7% | 18.3% |
| bought at the station | 3.4% | 46.1% |
| not to be had while docked | 94.9% | 35.6% |
| stack at the station's ask cr, min / median / max | 4 / 35 / 2240 | 4 / 124 / 1048 |

Recover rows that fit: 40; stack has room in the free hold: 100.0%; the stack at the station's bid, min / median / max: 6 / 76 / 1092 cr (kept by the player, not in net pay).

| first board, the rows that pay | value |
|---|---|
| seeds with a row that pays | 88.3% |
| best row by net pay per paper minute: net pay cr, median | 854 |
| best row: paper minutes, median | 3 |
| best row costs 0 s (a docked Procure) | 6.6% |
| best row kind | Procure 6.6%, Recover 4.2%, Escort 17.9%, Distress 32.1%, Neutralize 12.3%, Supply 26.9% |
| quickest row: paper seconds, median | 171 |
| quickest row: seconds to first payout with the start dock, median | 231 |
| quickest row: net pay cr, median | 700 |

| goal, rows that fit | value |
|---|---|
| the goal asks for | 7 |
| seeds whose first board has a goal row | 0.0% |
| seeds whose first board has a goal row that pays | 0.0% |
| goal rows that pay, of the goal rows that fit, 5 postings | 0 of 0 (-) |
| seeds with no goal row in 5 postings | 100.0% |
| goal rows per 15 board rows, lower quartile / median / upper quartile | 0 / 0 / 0 |
| goal count with every fitting row of the 15 handed in, quartiles | 0 / 0 / 0 |

| greedy 15-minute model | value |
|---|---|
| credits at the start | 1800 |
| credits at 15 min, quartiles | 3010 / 4260 / 5457 |
| gain cr at 15 min, quartiles | 1210 / 2460 / 3657 |
| gain cr at 15 min, min / median / max | 0 / 2460 / 8661 |
| jobs handed in, quartiles | 1 / 3 / 4 |
| seeds with no job handed in | 11.7% |
| first payout, paper s, quartiles | 225 / 240 / 245 |
| last payout or stop, paper s, quartiles | 268 / 575 / 774 |
| goal count at 15 min of 7, quartiles | 0 / 0 / 0 |
| net job pay by kind | Procure 1.2%, Recover 5.9%, Escort 20.5%, Distress 31.1%, Neutralize 16.9%, Supply 24.4% |
| Escort and Neutralize share of net job pay | 37.4% |
| docked 0 s rows: share of jobs / of net job pay | 4.1% / 1.2% |
| sale of recovered stacks cr, quartiles | 0 / 0 / 0 |
| why the run stopped | goods not to be had or no hold room 73.3%, next row ends past the horizon 26.7% |

## First boards of the shown seeds

To compare with the line `the station '<id>' posts 3 offer(s) over W wreck(s) and H hostile(s): ...` a live run logs for the same seed and role.

| seed | role | station | wrecks | hostiles | slot 0 | slot 1 | slot 2 |
|---|---|---|---|---|---|---|---|
| 10 | industrial | sector_0_6_2_cluster_0_4_1_station_0 | 0 | 0 | Bring 7 Water ice | Bring 4 Iron ore | Escort a hauler carrying 1,579 cr |
| 10 | scavenger | sector_0_6_2_cluster_0_4_1_station_0 | 0 | 0 | Bring 7 Water ice | Rearm a stranded ship with 1 Torpedo | Answer a distress call |
| 10 | transport | sector_0_6_2_cluster_0_4_1_station_0 | 0 | 0 | Bring 7 Water ice | Bring 4 Iron ore | Escort a hauler carrying 1,579 cr |
| 10 | bounty_hunter | sector_0_6_2_cluster_0_4_1_station_0 | 0 | 0 | Answer a distress call | Rearm a stranded ship with 1 Torpedo | Bring 3 Carbon ore |
| 10 | military | sector_0_6_2_cluster_0_4_1_station_0 | 0 | 0 | Answer a distress call | Rearm a stranded ship with 1 Torpedo | Bring 3 Carbon ore |
| 10 | pirate | sector_0_6_2_cluster_0_4_1_station_0 | 0 | 0 | Answer a distress call | Rearm a stranded ship with 1 Torpedo | Bring 3 Carbon ore |
| 17 | industrial | sector_0_0_0_cluster_0_0_0_station_0 | 0 | 0 | Escort a hauler carrying 594 cr | Bring 7 Water ice | Escort a hauler carrying 328 cr |
| 17 | scavenger | sector_0_0_0_cluster_0_0_0_station_0 | 0 | 0 | Bring 6 Carbon ore | Bring 7 Iron ore | Escort a hauler carrying 328 cr |
| 17 | transport | sector_0_0_0_cluster_0_0_0_station_0 | 0 | 0 | Escort a hauler carrying 594 cr | Bring 7 Water ice | Escort a hauler carrying 328 cr |
| 17 | bounty_hunter | sector_0_0_0_cluster_0_0_0_station_0 | 0 | 0 | Answer a distress call | Answer a distress call | Bring 4 Water ice |
| 17 | military | sector_0_0_0_cluster_0_0_0_station_0 | 0 | 0 | Answer a distress call | Answer a distress call | Bring 4 Water ice |
| 17 | pirate | sector_0_0_0_cluster_0_0_0_station_0 | 0 | 0 | Answer a distress call | Answer a distress call | Bring 4 Water ice |

## Cross-role summary

| role | hull cr | kit stock cr | credits | accel m/s2 | hold kg | weapons | fire s | rows shown (mean) | boards with 0 rows | first board has a row that pays | first board has a goal row | first board has a goal row that pays | no goal row in 5 postings | first payout s (median) | greedy gain cr at 15 min (quartiles) | greedy jobs (median) | greedy seeds with no job | greedy goal at 15 min (median / need) | Escort and Neutralize share of greedy pay |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| industrial | 5060 | 1160 | 2200 | 58.2 | 2100 | 0 | - | 1.95 | 0.0% | 41.2% | 67.9% | 0.0% | 22.9% | 232 | 0 / 0 / 821 | 0 | 58.8% | 0 / 6 | 0.0% |
| scavenger | 10560 | 1060 | 1800 | 46.3 | 5400 | 1 | 35 | 3.00 | 0.0% | 82.5% | 9.6% | 9.6% | 87.1% | 244 | 661 / 1573 / 3232 | 2 | 17.5% | 0 / 6 | 29.2% |
| transport | 7580 | 1360 | 2000 | 62.7 | 3700 | 0 | - | 1.95 | 0.0% | 60.4% | 88.8% | 31.2% | 0.0% | 60 | 0 / 258 / 870 | 1 | 39.6% | 0 / 4 | 0.0% |
| bounty_hunter | 6640 | 1060 | 1600 | 46.5 | 3000 | 2 | 20 | 3.00 | 0.0% | 88.3% | 10.8% | 10.8% | 83.3% | 239 | 1210 / 2460 / 3649 | 3 | 11.7% | 0 / 6 | 37.2% |
| military | 5950 | 1860 | 1000 | 41.0 | 1800 | 3 | 20 | 3.00 | 0.0% | 88.3% | 92.1% | 86.2% | 0.0% | 242 | 1210 / 2504 / 3666 | 3 | 11.7% | 2 / 6 | 36.5% |
| pirate | 6780 | 1060 | 1800 | 62.1 | 2900 | 2 | 20 | 3.00 | 0.0% | 88.3% | 0.0% | 0.0% | 100.0% | 231 | 1210 / 2460 / 3657 | 3 | 11.7% | 0 / 7 | 37.4% |

## Run record

The sections from here down are written by hand for this run. The tool does not print them.

- Code and content: `master` at `6d0f0bc69`, with the visibility changes the census needs and no rule, content or balance change.
- Test time: 128.7 s, 129.0 s and 127.7 s over three runs on six worker threads; about 2 min 12 s wall time with the build already done. Two runs of the same code gave the same report and the same row dump, byte for byte.
- Row dump: set `NOVA_CENSUS_ROWS` to a path to get every judged board row (21,600 rows for this run) as tab-separated lines.

## Example run: `system_start_roles`

One run, under Xvfb and lavapipe, with a sandboxed profile and capture directory. It failed before it logged a number:

```text
thread 'main' panicked at examples/systems/system_start_roles.rs:662:13:
assertion `left == right` failed: start roles: 'industrial' section 'mining_beam' binding
  left: Some([Keyboard(KeyV), Gamepad(RightTrigger2)])
 right: Some([Keyboard(KeyV)])
```

The example expects a keyboard-only default binding and the game now adds a gamepad binding. The fault is in the example, not in this work. The panic is on the first role, so the log has no `starts <hull>: hull <n> cr, kit <n> cr` line and no `accepts N job(s)` line. There are no example numbers to copy.

In their place:

- Hull worth and kit worth in the tables above come from the functions the example logs with: `hull_price_cr` over the hull's section recipes, and `reference_price().buy_cr(count)` over the kit.
- Two live runs of other examples at the same commit log a first board that the census reproduces, title for title, with the same wreck and hostile count (0 and 0). `system_station_jobs`, seed 10, transport: `Bring 7 Water ice; Bring 4 Iron ore; Escort a hauler carrying 1,579 cr`. `system_encounters`, seed 17, bounty hunter: `Answer a distress call; Answer a distress call; Bring 4 Water ice`. See `First boards of the shown seeds`.
- Accepted starter jobs per role are not measured by a live run. The nearest census number is `rows shown, mean of 3`.

## What the numbers say

1. The first board shows the industrial and the transport hull 1.95 of 3 rows; 27.9% of their boards show 1 row. The other four hulls see 3.00. Escort is 32.7% of the rows posted to the Trade mix and neither hull has a weapon: 0 of 1178 Escort rows fit.
2. A Procure is 40.1% to 52.5% of the posted rows of every role over 5 postings. The kit or the station's stock can fill it while docked in 99 of 1754 rows (5.6%) for industrial, 95 of 1890 (5.0%) for scavenger, 73 of 1443 (5.1%) for the three Combat-mix roles, and 423 of 1754 (24.1%) for transport.
3. The Procure share is mostly fallback. A wreck the draw can name stands in 12.9% of seeds and a hostile in 16.7%. 1374 of 1525 Recover draws of the scavenger (90.1%) and 943 of 1103 Neutralize draws of the Combat mix (85.5%) posted as a Procure.
4. The first board has a row that fits, can be done under the model and pays in 41.2% of seeds for industrial, 60.4% for transport, 82.5% for scavenger and 88.3% for bounty hunter, military and pirate.
5. Greedy gain at 15 minutes, median: industrial 0 cr, transport 258 cr, scavenger 1573 cr, bounty hunter 2460 cr, pirate 2460 cr, military 2504 cr. Seeds with no job handed in: 58.8%, 39.6%, 17.5%, 11.7%, 11.7%, 11.7%.
6. Escort and Neutralize are 29.2% (scavenger) to 37.4% (pirate) of greedy net job pay for the four armed hulls, and the model charges them no combat time and no loss. The kit and magazines hold 20 s of fire for bounty hunter, military and pirate and 35 s for the scavenger.
7. The first board has a goal row that fits in 92.1% of seeds for military, 88.8% for transport, 67.9% for industrial, 10.8% for bounty hunter, 9.6% for scavenger and 0.0% for pirate. No goal row in 5 postings: pirate 100.0%, scavenger 87.1%, bounty hunter 83.3%, industrial 22.9%, transport 0.0%, military 0.0%.
8. Industrial goal rows are Procures of ore and ice. 1352 fit over 5 postings and 0 of them can be filled from the kit or the station's stock; the stack must be mined, which the model does not do. The military hull can do and be paid for 1661 of its 1957 goal rows (84.9%).
9. Greedy goal count at 15 minutes, median: military 2 of 6, every other role 0. The 125-cell window holds 1 station in all 240 seeds, and the transport goal counts distinct posting stations, so every fitting row of 15 handed in gives 1 of 4.
10. Hull speed is not what separates the roles on paper. Acceleration runs from 41.0 m/s2 (military) to 62.7 m/s2 (transport). The same 853 Distress rows of the Combat mix take a median 171 s for the pirate (62.1 m/s2) and 182 s for military (41.0 m/s2). Start worth runs from 8420 cr (industrial) to 13420 cr (scavenger); credits from 1000 (military) to 2200 (industrial).
