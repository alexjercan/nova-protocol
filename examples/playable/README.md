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
| New Game start roles: hull, kit, bound weapons | `start_role_sandbox` | `nix develop --command cargo run --example start_role_sandbox --features debug -- --role pirate` | Start as each role (`industrial`, `scavenger`, `transport`, `bounty_hunter`, `military`, `pirate`). Fire with LMB, `R`, `F`, mine with `V`. Open TAB > Inventory to see the kit. Jettison and take back: TAB, Inventory, click a hold row, Jettison, quantity, Confirm; then fly the intake door onto the canister. | Panel: role, hull, kit, credits, hold load, hull value at ask and trade-in, live sections, cell, `intake: yes/no` with its face, pickups ready, last jettison and last pickup (item, count). Spheres round generated hulls in the window, labelled with their span. Each intake shows its face, pickup box, detection ring and normal; each drifting canister is ringed and labelled with its contents. |
| Advancement curve: larger hulls far out | `start_role_sandbox` | `nix develop --command cargo run --example start_role_sandbox --features debug -- --start-km 2500` | Start at 1,000, 2,500 and 3,500 km. Compare the spans of the generated ships you meet. | Panel: km from origin and the longest generated hull spans. Sphere and span label on each generated hull. |
| Escorts: hire, follow on own drive, hold for a dock | `escort_sandbox` | `nix develop --command cargo run --example escort_sandbox --features debug` | Burn (`W`), turn and stop (`X`): each escort flies to its slot. Stop: inside 60 m the escort trims on RCS. Hold `Ctrl` on an escort inside its dock-hold radius, then fly port to port and dock. | Formation slots and the line to each, 60 m trim zone, escort and leader velocity arrows, dock-hold radius, hull radii (red where two overlap). Panel: distance to slot, closing speed, speed, drive on or off, RCS share. |
| RCS gas puffs | `escort_sandbox` | as above | Stop near the escorts and watch them trim. `Shift`+mouse for your own RCS. | Orange arrow per RCS push, puff count in the panel. |
| Neutral retaliation threshold (30 HP, drain) and the 10 m/s free contact | `neutral_contact_sandbox` | `nix develop --command cargo run --example neutral_contact_sandbox --features debug` | Drift into the raider under 10 m/s, then ram it faster. Fire a short PDC burst, then a long one. Do the same to the station 1.5 km off. Destroy the raider to see its hold drop as canisters. | Panel: each Neutral's tally per attacker against 30 HP with the drain applied, who it answers, closing speed and last contact peak against 10 m/s, damage each hull took. Closing-velocity arrow (green free, red ram), ring round each Neutral (red while it answers), line to its target. |
| Neutral station fires back only; rams move a station | `neutral_contact_sandbox` | as above | Fire on the station until it answers. Ram it and watch it move. | As above, for the station. |
| Station tab, refit swap and add, rebuild, hull buy, rebind | `station_docked_sandbox` | `nix develop --command cargo run --example station_docked_sandbox --features debug` | Being changed by the Ship view and shipyard workers; see the example's header. | Ship view: free sockets, target socket, part bounds, roll arrow. |
| Buy a station | `station_own_sandbox` | `nix develop --command cargo run --example station_own_sandbox --features debug` | TAB > Station > Buy station. Check it is yours. Station allegiance is still being changed by its worker. | Panel: station name, side, civilization, chart entry, admission answers, price, balance. |
| Station storage and double-click quick moves | `storage_quick_loop` | `nix develop --command cargo run --example storage_quick_loop --features debug` | Still being changed by the storage worker. Double-click an item to move its stack, Shift for one. | Line to each own ship offered as storage, hold loads, last quick action. |
| Recycle tab: whole-ship strip and loot | `recycle_sandbox` | `nix develop --command cargo run --example recycle_sandbox --features debug` | Being added by the Recycle worker. TAB > Recycle > Strip, Take. | Strip progress, next section and time, parts yielded, hold. |
| Multi-dock, shipyard, Ship view additions, cargo intake door | none yet | - | Being added by the feature's worker. | - |
| Docking approach and the player's own RCS | `docking_approach` | `nix develop --command cargo run --example docking_approach --features debug` | Hold `Ctrl` on the spar to lock it, fly the sight, press the dock key. | None of the shared layer; the docking sight is the instrument. |
| Mining, intake, sell; refused mining pulse sound | `mine_and_sell` | `nix develop --command cargo run --example mine_and_sell --features debug` | Mine a rock, take a canister, sell its ore. Hold `V` with no rock locked to hear a refused pulse. | None of the shared layer. |
| Trade through the Inventory pane | `trade_loop` | `nix develop --command cargo run --example trade_loop --features debug` | Buy, sell, and get refused. | None of the shared layer. |
| New hulls and the rebuilt hull skin, `Bare` link points | `world_ships`, `first_shift_ships`, `wfc_ships` | `nix develop --command cargo run --example world_ships --features debug` | Fly round the hulls. Look at plate rims, folds, skirts, and the clean truss ends of `mining_tug`. | Own labels. `world_ships` starts bare, `C` or `-- --skin` clads it. In `wfc_ships`, `R` rerolls, `C` clads, `L` cycles the style. |
| Hull skin rules at structure scale | `shape_bench`, `block_bench` | `nix develop --command cargo run --example shape_bench --features debug` | Compare clad shapes. `C` toggles cladding, `L` cycles the style. | Own name and report labels. |
| Station style greebles | `greeble_catalog` | `nix develop --command cargo run --example greeble_catalog --features debug` | Find the `station` row. | Own row labels. |
| Greeble placement by link-point solve | `greeble_solve` | `nix develop --command cargo run --example greeble_solve --features debug -- --ship raider` | `N` steps through the base fleet. Match each fixture to its labelled cell, and follow a line greeble from end piece to end piece. | One sphere per cell: green for a fixture (labelled with its greeble id), grey for empty space. A tick per link point, coloured by link type, and a line per link: green where the link types connect, red where they do not; a red tick is an edge with no neighbour. Panel: hull, style, cells, links, border edges, restarts, fallback, cells per greeble. |
| Encounter rules on generated hulls | `world_encounters` | `nix develop --command cargo run --example world_encounters --features debug` | Fly the staged cast. Your magazines are empty, so use `neutral_contact_sandbox` for retaliation by fire. | Own labels. |
| Spare magazines in a fight | `wfc_arena` | `nix develop --command cargo run --example wfc_arena --features debug` | Watch a long fight: each fighter carries up to two spare magazines per mounted ammo in its hold. | Own labels. |
| Chart view, generated stations, credits cheats | `start_role_sandbox` | as above | TAB, `M` to Map, `C` for the chart. `:` opens the shell: `cheats enable`, then `credits give player 1000`. | As above. |

## Features with no playable example

| Feature | Why, and where to try it |
|---|---|
| Station design viewer (`ring_station`, `spire_station`, `yard_station`) | No playable viewer. `screenshot_station_showcase` stays open but is not framed for flying. |
| Sandbox ship Station box | Editor flow: main menu Sandbox, tick Station. Only the autopilot range `system_sandbox_station` exists. |
| Save, Load, DEFEAT and Load last save | Main menu flow: `nix develop --command cargo run`, New Game, then Load. Saves go under `$NOVA_CONFIG_ROOT/worlds`. |
| Hold events and `example_prospect` | Enable the example mod under Mods, then Scenarios, or `cargo run -- --scenario example_prospect`. |
| GOTO path for a moving target | `screenshot_goto_moving_target` stays open: lock the target, press `G`. |
| Pause when unfocused | Settings > Graphics. Harness runs force it off. |
| Generated world `bounds` and `OnExit` | Needs an authored scenario with `bounds`; no example stages one. |
