# Playable examples

Hand-played examples. Each one starts already staged through the game's own
code paths, shows a one-line hint, and stays open until you close it. No
deadline or scripted beat runs while `NOVA_AUTOPILOT` is unset. With
`NOVA_AUTOPILOT=1` (and the `debug` feature) the example runs a short smoke
check: it checks the staging, writes one screenshot with `NOVA_CAPTURE=1`,
and exits 0.

Run every example the same way:

```text
nix develop --command cargo run --example <name> --features debug [-- <args>]
```

## Diagnostics layer

`nova_debug::diagnostics` is the overlay these examples draw on. Every debug
build has it, and it boots off. An example turns it on at start with
`DiagnosticsHintPlugin::new(intro)`, whose intro and toggle keys fade a few
seconds into play. `F3`, or the Diagnostics checkbox in the Nova Debug window
(`F11`), shows and hides it. It has four parts:

- gizmos in the world: rings, spheres, arrows and lines in the palette
  colours (cyan velocity, yellow target, violet zone, green OK, red alert,
  orange push, grey muted);
- labels over world points, which carry the numbers next to the thing they
  measure;
- a short panel at the top left;
- a row of key hints above the HUD's key chips, each shown only while its
  action is possible.

`Shift+F3`, or the Diagnostics detail checkbox, adds the full value list to
the panel. While a surface is up (the TAB interface, the command modal, the
pause menu or the Nova Debug window) the panel moves to the bottom left, the
hints drop to the bottom edge, and the world labels hide. These examples need
`--features debug`.

## Features and examples

| Feature | Example | Command | What to try | Diagnostics |
|---|---|---|---|---|
| New Game start roles: hull, kit, bound weapons | `start_role_sandbox` | `nix develop --command cargo run --example start_role_sandbox --features debug -- --role pirate` | Start as each role (`industrial`, `scavenger`, `transport`, `bounty_hunter`, `military`, `pirate`). Fire with LMB, `R`, `F`, mine with `V`. Open TAB > Inventory to see the kit. Jettison and take back: TAB, Inventory, click a hold row, Jettison, quantity, Confirm; then fly the intake door onto the canister. | Panel: role, hull, credits, hold load. Labels: each intake's face (READY when a pickup is ready), each drifting canister's contents, each generated hull's span. Gizmos: the intake's pickup box, detection ring and normal; a sphere round each generated hull. Detail: kit, hull value, live sections, cell, intake counts, last jettison and pickup. |
| Advancement curve: larger hulls far out | `start_role_sandbox` | `nix develop --command cargo run --example start_role_sandbox --features debug -- --start-km 2500` | Start at 1,000, 2,500 and 3,500 km. Compare the spans of the generated ships you meet. | Detail: km from origin and the longest generated hull spans. Sphere and span label on each generated hull. |
| Escorts: hire, follow on own drive, hold for a dock | `escort_sandbox` | `nix develop --command cargo run --example escort_sandbox --features debug` | Burn (`W`), turn and stop (`X`): each escort flies to its slot. Stop: inside 60 m the escort trims on RCS. Hold `Ctrl` on an escort inside its dock-hold radius, then fly port to port and dock. | Formation slots and the line to each, 60 m trim zone, escort and leader velocity arrows, dock-hold radius, hull radii (red where two overlap). Labels: distance to slot, LOCKED, overlap. Detail: closing speed, speed, drive on or off, RCS share. |
| RCS gas puffs | `escort_sandbox` | as above | Stop near the escorts and watch them trim. `Shift`+mouse for your own RCS. | Orange arrow per RCS push, puff count in the detail. |
| Neutral retaliation threshold (30 HP, drain) and the 10 m/s free contact | `neutral_contact_sandbox` | `nix develop --command cargo run --example neutral_contact_sandbox --features debug` | Drift into the raider under 10 m/s, then ram it faster. Fire a short PDC burst, then a long one. Do the same to the station 1.5 km off. Destroy the raider to see its hold drop as canisters. | Closing-velocity arrow (green free, red ram) with its speed label, ring round each Neutral (red while it answers) with your tally bar against 30 HP, line to its target. Panel: last contact peak against 10 m/s. Detail: tally per attacker with the drain, who it answers, damage each hull took. |
| Neutral station fires back only; rams move a station | `neutral_contact_sandbox` | as above | Fire on the station until it answers. Ram it and watch it move. | As above, for the station. |
| Ship pane refit: Catalogue, place, Q, Delete, rebind, rebuild | `station_docked_sandbox` | `nix develop --command cargo run --example station_docked_sandbox --features debug` | Ship tab: open the Catalogue (the parts gallery) and place a part; the view eases onto it. `Q` picks a part off the ship and puts it away over empty space. Delete lists the yield under Salvage, or is disabled with the reason there. `B` rebinds. Station tab: buy a hull, hire an escort. | Ship view: free sockets, target socket, part bounds, roll arrow, station collision volume. Detail: what a delete gives back or why it is refused. |
| Jobs pane: read, accept, Show on Map, do the work, hand in | `job_board_sandbox` | `nix develop --command cargo run --example job_board_sandbox --features debug` (`NOVA_SANDBOX_ROLE=bounty_hunter` for Combat boards, `scavenger` for recoveries) | TAB > Jobs: click a row, read the inspector, Accept. TAB, undock, TAB > Jobs: the held row names the hand-in station and its distance; Show on Map opens the chart on `JOB-1`. A procurement: dock again, TAB > Jobs > Hand in. A located job: fly to the mark, recover the stack, escort the hauler, answer the call or neutralize the hull, then dock again and Hand in. With `NOVA_AUTOPILOT=1` the walk first reads the starter board at the starting station: only what the hull can do is listed. | Panel: jobs held and completed, balance, jobs posted. Detail: each posted job with its reward, state and task, each held job and its target. |
| Computed station prices | `market_sandbox` | `nix develop --command cargo run --example market_sandbox --features debug` (`NOVA_SANDBOX_MARKET=planet-heavy` for the planet-heavy station) | TAB > Inventory: read the price and `vs ref` percent on each row, click Iron ore on your side and read Reference, Pays, Asks, Scarcity, Cluster, Civilization and Total, then the MARKET strip under the columns. Sell it, or Sell all. | Panel: the wanted cluster type, the station's market and leading role. Detail: the iron ore quote, each Sell checked. |
| Buy a station | `station_own_sandbox` | `nix develop --command cargo run --example station_own_sandbox --features debug` | TAB > Station > Buy station. Check it is yours, then undock and dock again. | Station label: name, side, price or OWNED. Panel: balance. Detail: the station's civilization warmed to Ally by merits before the buy, chart entry, admission answers, trade prices. |
| Station storage and double-click quick moves | `storage_quick_loop` | `nix develop --command cargo run --example storage_quick_loop --features debug` | Double-click an item to move its stack, Shift for one. Store, sell and move to your other ship. | Line to each own ship offered as storage, with a label. Panel: last quick move. Detail: hold loads. |
| Whole-ship strip and Take all | `recycle_sandbox` | `nix develop --command cargo run --example recycle_sandbox --features debug` | TAB > Inventory > Strip. The port comes off last and the wreck is gone. Take all in the wreck's column takes what fits and names what stayed. | Strip progress, next section and time, parts yielded, hold, wreck state. |
| Shipyard: hulls docked at every port, Board, Info | `shipyard_sandbox` | `nix develop --command cargo run --example shipyard_sandbox --features debug` | TAB > Station: each card turns the hull's own model. Buy at the full price, or Board your old ship for free. Info opens the hull read-only in the Ship view. | Each port coloured by its holder, a line to each docked hull, price rings. Detail: holders, prices, escorts. |
| Docking approach and the player's own RCS | `docking_approach` | `nix develop --command cargo run --example docking_approach --features debug` | Hold `Ctrl` on the spar to lock it, fly the sight, press the dock key. | None of the shared layer; the docking sight is the instrument. |
| Mining, intake, sell; refused mining pulse sound | `mine_and_sell` | `nix develop --command cargo run --example mine_and_sell --features debug` | Mine a rock, take a canister, sell its ore. Hold `V` with no rock locked to hear a refused pulse. | None of the shared layer. |
| Trade through the Inventory pane | `trade_loop` | `nix develop --command cargo run --example trade_loop --features debug` | Buy, sell, and get refused. | None of the shared layer. |
| New hulls and the rebuilt hull skin, `Bare` link points | `world_ships`, `first_shift_ships`, `wfc_ships` | `nix develop --command cargo run --example world_ships --features debug` | Fly round the hulls. Look at plate rims, folds, skirts, and the clean truss ends of `mining_tug`. | Own labels. `world_ships` starts bare, `C` or `-- --skin` clads it. In `wfc_ships`, `R` rerolls, `C` clads, `L` cycles the style. |
| Generated ships and stations by advancement: role, archetype, wreck, seed, skin, style | `generator_gallery` | `nix develop --command cargo run --example generator_gallery --features debug` | Change a control in the Generator window and release it: the rows are generated again, linted and fitted in view. Drag to orbit, scroll to zoom. Click a hull to orbit it alone; a new row keeps the focus on the same slot. `F` fits all the rows again. | Ships above stations. Label per hull: advancement, clearance radius, sections, ports, turrets, computers; a ring of the clearance radius. Panel: the rows. Refused requests in red in the window. |
| Hull skin rules at structure scale | `shape_bench`, `block_bench` | `nix develop --command cargo run --example shape_bench --features debug` | Compare clad shapes. `C` toggles cladding, `L` cycles the style. | Own name and report labels. |
| Station style greebles | `greeble_catalog` | `nix develop --command cargo run --example greeble_catalog --features debug` | Find the `station` row. | Own row labels. |
| Greeble placement by link-point solve | `greeble_solve` | `nix develop --command cargo run --example greeble_solve --features debug -- --ship raider` | `N` steps through the base fleet. Match each fixture to its labelled cell, and follow a line greeble from end piece to end piece. | One sphere per cell: green for a fixture (labelled with its greeble id), grey for empty space. A tick per link point, coloured by link type, and a line per link: green where the link types connect, red where they do not; a red tick is an edge with no neighbour. Panel: hull, style, links that do not connect, fallback; a "no fit" label on each such link. Detail: cells, links, border edges, restarts, cells per greeble. |
| Encounter rules on generated hulls | `world_encounters` | `nix develop --command cargo run --example world_encounters --features debug` | Fly the staged cast. Your magazines are empty, so use `neutral_contact_sandbox` for retaliation by fire. | Own labels. |
| Spare magazines in a fight | `wfc_arena` | `nix develop --command cargo run --example wfc_arena --features debug` | Watch a long fight: each fighter carries up to two spare magazines per mounted ammo in its hold. | Own labels. |
| Chart view, generated stations, credits cheats | `start_role_sandbox` | as above | TAB, `M` to Map, `C` for the chart. `:` opens the shell: `cheats enable`, then `credits give player 1000`. | As above. |
| Proposed job board, chart, market, load and new game screens in the game's pane chrome (owner feedback 2026-10-17), mock data | `ui_gallery` | `nix develop --command cargo run --example ui_gallery --features debug` | Top bar: Jobs, Chart, Market, Load, New Game, Icons; Docked/Undocked; theme. Jobs: Starter/Core board, Accept, Hand in (hover a greyed one for the refusal), click a row for the inspector, Show on Map. Chart: 3D sectors, right-drag turns, wheel zooms, WASD pans, Q/E R/F T, click a sector. Market: category chips, click a row, quantity field, All, slider, Buy/Sell; a Storage row offers Store/Take. Load: pick a row, 3D preview, fleet and open objectives. New Game: the menu with its modal, name, seed, Randomize, seed preview, Next, role list with a spinning hull, Back, Create. Icons: every mask at 16/24/48 px. `S` flips the theme. | None of the shared layer. `NOVA_AUTOPILOT=1` clicks every page and prints `gallery:` verdicts; `NOVA_CAPTURE=1` writes one PNG per page and one per New Game step. |

## Features with no playable example

| Feature | Why, and where to try it |
|---|---|
| Sandbox ship Station box | Editor flow: main menu Sandbox, tick Station. Only the autopilot range `system_sandbox_station` exists. |
| Save, Load, DEFEAT and Load last save | Main menu flow: `nix develop --command cargo run`, New Game, then Load. Saves go under `$NOVA_CONFIG_ROOT/worlds`. |
| Hold events and `example_prospect` | Enable the example mod under Mods, then Scenarios, or `cargo run -- --scenario example_prospect`. |
| GOTO path for a moving target | `screenshot_goto_moving_target` stays open: lock the target, press `G`. |
| Pause when unfocused | Settings > Graphics. Harness runs force it off. |
| Generated world `bounds` and `OnExit` | Needs an authored scenario with `bounds`; no example stages one. |
