# MAP: how a start hull gets its look

State of `master` at `515c0c935`, read 2026-11-19. Findings only. No design
decision is made here. Paths are relative to the repository root. A claim with
no `path:line` is labelled.

Short version:

- The build vocabulary is cubes. No slope, wedge, corner or plate part exists.
- The faceted look is a derived skin: a pure function of the filled cells.
- A hull picks one style id. A style is two colours, a greeble kit and solver
  rules. Nothing is set per cell or per hull.
- The paper numbers (mass, HP, hold, worth, acceleration) are linear in section
  counts. The same cells in a new arrangement keep all of them.
- The six start hulls are 5 to 10 cells long. A one-cell chamfered lump is 10 to
  20 percent of the hull, so every hull reads as a pile of the same lumps.

## A. Section vocabulary at advancement 0

Grid: one cell is one engine unit and 10 m. `-Z` is the bow, `-X` is port
(`crates/nova_authoring/src/base_content/ships/block.rs:1-30`). A start hull may
carry advancement-0 parts only (`block.rs:9-11`,
`crates/nova_scenario/src/lint/start_role.rs:151-157`).

Files below are under `crates/nova_authoring/src/base_content/sections/`.
Mass is collider volume at density 1 (`crates/nova_ship/src/sections/ship_measure.rs:30`).
Cost is the recipe in `recipes.rs`, plus its parts.

| Part (prototype id) | Size, cells | Mass | HP | Recipe | Stat | Sockets and facing | Evidence |
|---|---|---|---|---|---|---|---|
| `light_hull_section` | 1x1x1 | 1.0 | 60 | 80 cr, 2 section panels | 100 kg hold | every face | `hull.rs:78-85`, `recipes.rs:55` |
| `basic_controller_section` | 1x1x1 | 1.0 | 100 | 100 cr, flight core, 2 panels | 1 per 50 cells of mass | every face | `controller.rs:17,82-83`, `recipes.rs:93` |
| `basic_thruster_section` | 1x1x1 | 1.0 | 70 | 60 cr, drive bell, 2 panels | thrust 1.0 | one socket, on `-Z`. Bell opens `+Z` | `thruster.rs:17,131-153`, `recipes.rs:70` |
| `pdc_kinetic_turret_section` | 0.5 cube | 0.125 | 130 | 100 cr, body, barrels, actuator, 2 panels | weapon | one socket, on its `-Y` base | `turret.rs:19,79,481`, `recipes.rs:103` |
| `pdc_twin_kinetic_turret_section` | 0.5 cube | 0.125 | 130 | `pdc_turret(barrels)` | weapon | as above | `turret.rs:510` |
| `torpedo_section` (Serpent) | 1x1x2 | 2.0 | 100 | 150 cr, tube, actuator, 4 panels, spar | weapon | back `+Z` and flanks. None on the `-Z` muzzle | `torpedo_bay.rs:57,73,362`, `recipes.rs:117` |
| `lance_torpedo_section` | 1x1x2 | 2.0 | 100 | `torpedo_bay()` | weapon | as above | `torpedo_bay.rs:61,381` |
| `docking_port_section` | 1x1x1 | 1.0 | 90 | 80 cr, collar, actuator, 2 panels | docking (job gate) | every face but the `-Z` hatch | `docking_port.rs:20,130-131`, `recipes.rs:150` |
| `cargo_intake_section` | 3x2x1 | 6.0 | 90 | 60 cr, 2 actuators, 4 panels, spar | intake (lint gate) | none on the `-Z` door | `cargo_intake.rs:24,28,160-161`, `recipes.rs:164` |
| `mining_beam_section` | 1x1x1 | 1.0 | 90 | 100 cr, capacitor, actuator, 2 panels | mining, 100 m reach | none on the `-Z` emitter | `mining_beam.rs:23,146-147`, `recipes.rs:178` |

The masses and HP above reproduce all six census rows exactly (section B).

Not available at advancement 0: reinforced, cargo and tank cubes (0.3,
`hull.rs:61-62,111-112,134-135`), truss (0.2, `hull.rs:159-160`), solar panel
(0.5, `hull.rs:182-183`), vector thruster (0.25, `thruster.rs:178-179`), capital
thruster (0.5), armoured controller (0.3, `controller.rs:106-107`), pierce
turrets (0.25), Dart bay (0.25, `torpedo_bay.rs:398`), railguns (0.5, 0.6),
wide docking port (0.3), wide intake (0.3), long mining beam (0.3).

Placement rules:

- One socket graph. Every section must mate through facing sockets
  (`crates/nova_ship/src/sections/design_sections.rs:529`, fault `Disconnected`
  at `:411`; test `ships/mod.rs:359`).
- Clear exit lanes. The cells a bell, muzzle, bay mouth or emitter points into
  must hold no structure, and no neighbour may offer a socket into them. The
  lane runs to the edge of the hull's box
  (`crates/nova_ship/src/sections/clearance.rs:1-30,154-158`). So no cowl,
  shroud or fin may stand aft of or beside a bell's lane.
- Thrust floor: thrust >= 0.06 per cell of mass (`ship_measure.rs:23`).
- Controllers: one per 50 cells of mass, rounded up (`ship_measure.rs:26,109`).
- A collar stands one cell outboard of a hull cell, hatch outboard, with three
  clear cells ahead of the hatch (`block.rs:806-820`, test `block.rs:1033-1072`).
  Every crewed hull must have `port_collar` (`block.rs:1041-1046`).
- The intake must not reach a docking port's approach
  (`start_role.rs:124-131`).
- Symmetry is not a rule. Nothing checks it.
- A special placed exactly on a cell replaces that cell's plate. Plates are
  named `plate_N` in cell order (`block.rs:852-881`).

Exists but no start hull uses it:

- Slopes, wedges, corners, plates as parts: none exist. See section C.
- `pdc_twin_kinetic_turret_section` and `lance_torpedo_section`: advancement 0,
  not on any start hull.
- `under_turret` (a turret hung below the hull, `block.rs:790`): line warship
  only (`block.rs:694-695`).
- `carve` (`block.rs:902`) and zones (`block.rs:262`): derelict only.
- A keel deck is rare: only the hauler body, the freighter transom cell and the
  picket's torpedo bay stand below `y=0`.

## B. The six hulls as authored

All in `crates/nova_authoring/src/base_content/ships/block.rs`. Every plain cell
is `light_hull_section` (`plate: LIGHT_HULL_SECTION_ID`, for example `:176`).
`block_hull` turns the skin on and sets the style
(`ships/mod.rs:270-274`).

Where the numbers come from:

- Sections, mass, thrust: `ShipMeasure::of` (`ship_measure.rs:75`).
- Acceleration: `a = thrust / (mass * tick) * 10 m`, tick 0.015625 s
  (`tasks/20261116-132143/CENSUS-AFTER.md:149`).
- HP: sum of prototype health (table in A).
- Hold: 100 kg per hull section
  (`crates/nova_scenario/src/objects/ship_design.rs:413,422`).
- Worth: every section's recipe at reference ask
  (`crates/nova_authoring/tests/start_role_census.rs:2154`).
- Printed rows: worth, acceleration and hold per role at
  `tasks/20261116-132143/CENSUS-AFTER.md:935-942`. Sections and health sum per
  hull in each role block (tug `:205,210`). Armed hulls at `:75`.
- Mass is not printed there. The mass column is the sum of the volumes in A.
  It matches the web figures for the tug (33.00) and the interceptor (41.25)
  (`web/src/wiki/sections/thruster.md:24`).

| Hull | Builder | Style | Box W x H x L | Sections | Light hull | Mass | Drives | m/s2 | HP | Hold kg | Worth cr | Weapons |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `mining_tug` | `:136` | industrial | 4 x 2 x 10 | 28 | 21 | 33.0 | 3 | 58.2 | 1840 | 2100 | 5060 | none, 1 beam |
| `salvage_hauler` | `:190` | salvage | 4 x 4 x 9 | 65 | 54 | 69.1 | 5 | 46.3 | 4190 | 5400 | 10560 | 1 PDC, 1 beam |
| `light_freighter` | `:333` | civilian | 5 x 4 x 8 | 46 | 37 | 51.0 | 5 | 62.7 | 2950 | 3700 | 7580 | none |
| `interceptor` | `:391` | armoured | 5 x 3 x 5 | 38 | 28 | 41.2 | 5 | 77.6 | 2570 | 2800 | 6920 | 2 PDC |
| `raider` | `:448` | salvage | 5 x 3 x 7 | 38 | 29 | 41.2 | 4 | 62.1 | 2560 | 2900 | 6780 | 2 PDC |
| `patrol_picket` | `:504` | armoured | 5 x 4 x 8 | 33 | 23 | 37.2 | 3 | 51.5 | 2320 | 2300 | 6810 | 2 PDC, 1 bay |

Plans. Rows run bow to stern. Columns run port to starboard. `#` light hull,
`B` controller, `M` mining beam, `E` drive, `D` docking collar, `T` turret
(drawn in the cell above its seat), `i` intake (3 x 2, door up, drawn on the
deck it lies on), `p` torpedo bay. The plans were generated from the builder
calls by
`/tmp/claude-1000/-home-alex-personal-nova-protocol/3f34df46-822e-4bc8-b585-f58cc90d779d/scratchpad/looks-map/plans.py`.
Its mass, HP, hold and acceleration match the census for all six hulls.

```
mining_tug  x -2..1, z -6..3
 y=+1    y=0
 ....    ..M.   z=-6
 ....    ..#.
 ....    ..#.
 ....    .###   z=-3
 ..B.    .###
 ..#.    D###
 ....    .###   z=0
 .iii    .###
 .iii    .###
 ....    .EEE   z=+3

salvage_hauler  x -2..1, z -5..3
 y=+2    y=+1    y=0     y=-1
 ....    ....    ..M.    ....   z=-5
 ....    ..B.    ..#.    ....
 ....    .###    .###    .###   z=-3
 ..T.    .###    D###    .###
 ....    .###    .###    .###
 .iii    .###    .###    .###   z=0
 .iii    .###    .###    .###
 ....    .###    .#B#    .###
 ....    ..E.    .EEE    ..E.   z=+3

light_freighter  x -2..2, z -3..4
 y=+2     y=+1     y=0      y=-1
 .....    ..B..    .###.    .....   z=-3
 .....    .###.    D###.    .....
 .....    .###.    .###.    .....
 .iii.    .###.    .####    .....   z=0
 .iii.    .###.    .#B##    .....
 .....    .....    .####    .....
 .....    ..#..    .###.    ..#..   z=+3
 .....    ..E..    .EEE.    ..E..   z=+4

interceptor  x -2..2, z -2..2
 y=+2     y=+1     y=0
 .....    .###.    .###.   z=-2
 .T.T.    .#B#.    .###.
 .....    .###.    D###.   z=0
 .iii.    .###.    #####
 .iii.    .###.    EEEEE   z=+2

raider  x -2..2, z -3..3
 y=+2     y=+1     y=0
 .....    .....    ..#..   z=-3
 .....    .#B#.    .###.
 ..T..    .####    .####
 ....T    .####    D####   z=0
 .....    .iii.    .####
 .....    .iii.    .###E
 .....    .....    .EEE.   z=+3

patrol_picket  x -2..2, z -4..3
 y=+2     y=+1     y=0      y=-1
 .....    .....    ..T..    .....   z=-4  (nose PDC, turned to face the bow)
 .....    .....    ..#..    ..p..
 ..T..    ..#..    .###.    ..p..
 .....    ..B..    .###.    .....
 .....    .....    D###D    .....   z=0
 .....    .iii.    #####    .....
 .....    .iii.    #####    .....
 .....    .....    #EEE#    .....   z=+3
```

Length to body width: tug 3.3, hauler 3.0, freighter 2.7, raider 2.3, picket
2.7 (three-cell body), interceptor 1.7. The generator holds its hulls to 2.0 to
3.0 stations per cell of width before any waist
(`crates/nova_world_base/src/ship_layout.rs:1176-1192`).

## C. The skin

There is no cell mesh to bevel. The look is the derived skin
(`crates/nova_ship/src/sections/shell_skin.rs:1-33`):

- It is a pure function of the filled cells and their socket faces. Nothing
  places or saves a plate (`shell_skin.rs:3-7`).
- Every empty cell that touches the hull gets one plate
  (`cladding_cells`, `shell_skin.rs:297`). A cell a part fires into is not
  clad, and a face with no socket is not clad (`shell_skin.rs:17-19`).
- A plate's shape comes from its neighbourhood only: a level (half cell or full
  cell, `shell_shape.rs:55,59`), four edges and four corner heights
  (`boundary_heights`, `shell_skin.rs:388`).
- Edge types: Plain, Fold, Skirt, Ramp, RampEnd, Wall. Chamfer slope 2, ramp
  slope 1, wall slope 8 (`crates/nova_ship/src/sections/shell_shape.rs:81,89,94`).
- A diagonal staircase of cubes is clad as one slope-1 ramp
  (`docs/sections.md:1165-1228`,
  `examples/screenshots/screenshot_diagonal_skin.rs:4-9`). This is the only way
  to author a slope.
- Materials are one flat `StandardMaterial` per style and per role, Top or
  Wall. No texture, no panel line, no per-plate tint
  (`shell_skin.rs:1366-1425`, `skin_style.rs:155-160`).
- A plate is a fixture. It has 80 HP per full cell and density 0.25. It is not
  in the socket graph and not in the ship's health
  (`shell_skin.rs:84,98,814-824`).
- Skin mass is real in flight. `ShipMeasure` does not count it
  (`web/src/wiki/sections/thruster.md:21,24`, which cites
  `crates/nova_ship/src/flight/authority.rs:153`).

A style (`crates/nova_ship/src/sections/skin_style.rs:99-117`):

- `id`, `name`.
- `palette`: `top` and `wall`, each colour, roughness, metallic
  (`skin_style.rs:54-60,85-89`).
- `fixtures`: the greeble kit, each a model, a health and a collider
  (`skin_style.rs:128-149`).
- `greeble_rules`: section D.

A hull names one style id (`ships/mod.rs:271-274`). Palettes, linear RGB, in
`crates/nova_authoring/src/base_content/styles.rs`:

| Style | Top | Wall | Hulls | Lines |
|---|---|---|---|---|
| industrial | 0.200, 0.190, 0.168 | 0.030, 0.028, 0.024 | tug | `:74,79` |
| armoured | 0.045, 0.048, 0.055 | 0.007, 0.0075, 0.009 | interceptor, picket | `:236,241` |
| civilian | 0.420, 0.420, 0.400 | 0.058, 0.060, 0.068 | freighter | `:341,346` |
| salvage | 0.024, 0.0175, 0.0125 | 0.0038, 0.0028, 0.0020 | hauler, raider | `:466,471` |
| station | 0.42, 0.43, 0.45 | 0.06, 0.062, 0.07 | none | `:586,591` |

What a builder can vary:

- Per hull: the cell layout, the specials, one style id. Nothing else.
- Per cell: nothing. No colour, no plate shape, no trim, no decal.
- Per style (global to every hull that wears it): two finishes, the kit, the
  rules.
- Not present anywhere: trim colour, accent colour, decals as a layer, emissive
  surfaces, hull lights. Ships carry no running lights (grep: `AmbientLight`
  and hull `PointLight` exist only in preview rigs, station port beacons
  `crates/nova_ship/src/sections/docking_section/beacon.rs`, and weapon
  effects). The floodlight greeble is mesh only:
  "the pipeline has no emissive"
  (`scripts/greeble-recipes/industrial_floodlight.json:2`).

## D. Greebles

Pipeline:

- Recipes: `scripts/greeble-recipes/*.json`, 60 files: 14 industrial, 14
  salvage, 12 civilian, 10 armoured, 6 station, 4 placeholder.
- Generator: `scripts/gen-greebles.py`. Primitives box, cylinder, taper, ribs,
  disc. Budgets: footprint 0.5 cell, height 0.5 cell, 200 triangles
  (`scripts/gen-greebles.py:64,65,70`). A recipe may override footprint and
  height (`:449-450`).
- Output: `assets/base/gltf/greebles/*.glb`, declared in
  `assets/base/base.bundle.ron:100-165`.
- A style names its kit in `fixtures` and its rules in `greeble_rules`
  (`styles.rs:62,206,331,436`).

Placement rule in code:

- The skin is read as a surface graph: one cell per plate, four edges
  (`crates/nova_ship/src/sections/skin_surface.rs:35`).
- A seeded, deterministic WFC-style solve puts one greeble, or the empty
  greeble, on every cell
  (`crates/nova_ship/src/sections/greeble_solver.rs:678`).
- The seed is a hash of the style id alone. Each draw also hashes the plate's
  own cell and face (`greeble_solver.rs:665-676`). So two hulls in one style
  share a seed, and a hull has no seed of its own.
- Rule helpers in content: `empty(weight)`, `filler`, `alone(fixture, weight,
  radius, max_count)`, `line(fixture, line, weight, end_weight)`, `clustered`,
  `near` (`styles.rs:656-720`).
- Density is the `empty` weight against the kit weights: 6.0 for industrial,
  armoured, civilian and station, 5.0 for salvage
  (`styles.rs:729,754,779,820,848`). It is per style, not per hull.
- The plate under a piece is reshaped so the piece sits flat
  (`docs/sections.md:1259-1377`). Test: no piece overhangs its plateau on any
  styled hull (`crates/nova_authoring/tests/base_hull_greebles.rs:35`).
- Kit sizes are pinned by a test (`styles.rs:996`).

History of the placement language (`tasks/20261013-133640/TASK.md:17-27`):

- Round 1, 2026-10-14, three candidates: A "`on` a plate kind plus `pattern:
  Line | Every(n) | Ends | Middle | Once`", B "a weighted table per plate kind
  with a bare weight", C "panels first".
- Owner, 2026-10-14: "all three candidates are REJECTED, and so is the in-game
  framing of the language ("Kinds a reader can see")."
- Direction given: "Greebles get link points, like sections do." "A
  deterministic, seeded WFC-style solver places them." "The language is a
  technical solver configuration. It uses no game-fiction words; the code is
  the code."
- Round 2, same day: "candidate C's structure is accepted, but its plate
  filters and socket keywords are not."
- Built under `tasks/20261013-133643`. "Owner accepted the look 2026-10-16 ...
  The raider nose rim jog is accepted as is." (`tasks/20261013-133643/TASK.md:122-123`).

What each start hull shows now. Counts are from the `spawn_ship_skin` debug
line of one `screenshot_base_fleet` run. The hull for each line was matched by
style and spawn order, not by a printed id.

| Hull | Plates | Flat-topped | Pieces | Pieces per plate | Never placed |
|---|---|---|---|---|---|
| tug | 62 | 28 (45%) | 30 | 0.48 | stack, crane, hazard band, winch |
| hauler | 84 | 57 (68%) | 52 | 0.62 | whip |
| freighter | 73 | 31 (42%) | 30 | 0.41 | none |
| interceptor | 55 | 28 (51%) | 17 | 0.31 | intake, magazine |
| raider | 56 | 28 (50%) | 30 | 0.54 | whip, chain, kills |
| picket | 60 | 17 (28%) | 18 | 0.30 | none |

The tug's most frequent piece is the duct, 12 of 30. The two armoured hulls
carry the fewest pieces.

## E. Constraints a redesign must keep

Gates in code:

- Post-build checks and a docking port on every crewed hull
  (`ships/mod.rs:311`, checks in `design_sections.rs:529`).
- One connected socket graph (`ships/mod.rs:359`).
- Unique ids, no shared cell, collar rule, a `bridge` on every hull
  (`block.rs:953,969,1033,1087`). The picket nose gun keeps the id `pdc`
  (`block.rs:79,1004`).
- Start role lint (`crates/nova_scenario/src/lint/start_role.rs`): known hull
  (`:49-54`), a cargo intake (`:104-111`), intake clear of the dock approach
  (`:124-131`), advancement 0 (`:151-157`), kit weight under the hold
  (`crates/nova_scenario/src/lint/ship.rs:165-171`).
- No lint rule asks for a docking port. `ships/mod.rs:311` does.
- Job gate reads the live hull: hold capacity, a docking section, a weapon
  section (`crates/nova_gameplay/src/jobs.rs:3106-3189`,
  `crates/nova_world_base/src/jobs.rs:221-239`).

Balance (`tasks/20261116-132143/TASK.md:81-90`,
`tasks/20261116-132143/CENSUS-AFTER.md:12,75,935-946`):

- Stated orderings: the interceptor is "the fastest armed start". The picket
  "stays the lightest armed start". Armed starts by acceleration: interceptor
  77.6, raider 62.1, picket 51.5, hauler 46.3. The scavenger hull "is worth
  twice the tug".
- Bands: a role median must stay between 0.75 and 1.33 times the six-role
  median for gain (4,113 cr) and first payout (225 s). Hold and acceleration
  feed both.
- No tolerance is written for a hull number. The safe reading is: section
  counts by kind do not move. Then mass, thrust, HP, hold and worth do not
  move.
- Per light hull cell added: mass +1.0, HP +60, hold +100 kg, worth +80 cr plus
  two panels, acceleration down.
- `start_role_census.rs` is an ignored printer, not a gate (`:2841`). Nothing
  fails when a hull number moves. The census must be rerun by hand.
- Not in the paper numbers: skin mass (more outer surface, more plates, heavier
  in flight), inertia and lever arm, turret arcs, where a collar sits.

Pins on exact numbers or cells:

- `web/src/widgets.ts:2723` `INTERCEPTOR_CELLS`: every interceptor cell, by
  `plate_N` id and coordinate. `web/src/widgets.ts:201` `INTERCEPTOR_ARM_U =
  3.513`.
- `web/tests/widgets.test.ts:235-242`: interceptor is 38 sections and mass
  41.25. More asserts on the arm and the sponsons follow in that file.
- Web prose: `web/src/wiki/sections/thruster.md:21,24` (tug 33.00, 58 m/s2,
  interceptor 41.25, 78 m/s2), `web/src/wiki/sections/controller.md:35`
  (35.1 m arm), `web/src/create/ships.md:190-201` (shape prose, `port_collar`).
  `thruster.md:21` still lists the tug as 19 reinforced plus 2 truss: stale
  today.
- `CHANGELOG.md:51`: five drives on the interceptor, third drive and two aft
  strakes on the picket.
- `assets/base/**/*.content.ron` is generated and pinned byte for byte by
  `crates/nova_authoring/tests/content_ron_parity.rs`. Regenerate, never edit.
- Saves: repair jobs are keyed by section id
  (`crates/nova_world_base/src/save/mod.rs`, reported by a search agent, lines
  not rechecked). A reshaped hull changes `plate_N` ids.

Other users of the six hull ids (reported by a search agent; the items with a
line were spot-checked only where section E cites them):

- Roles: `crates/nova_authoring/src/base_content/start_roles.rs:46,68,89,112,133,155`.
- Shared geometry: `light_freighter` shares `freighter_bow` and
  `freighter_fittings` with `freighter_derelict` (`block.rs:250,277,363`).
- Menu scenes: `main_menu/duel.rs` (interceptor, inline raider, patches
  `pdc_port`, `pdc_starboard`, `pdc_dorsal`, `pdc_boom`, `bridge`),
  `main_menu/weave.rs` (tug), `main_menu/shared.rs` (freighter or tug). The
  inline raider is `ships/mod.rs:117`.
- Season one stage: tug, patches `bridge` and `port_collar`.
- Examples keyed to cells: `examples/screenshots/loop_damage_sequence.rs`
  (interceptor cells and `pdc_starboard`), `loop_spine_cut.rs` (interceptor
  cell `(-1,1,-1)`), `screenshot_mining_beam.rs` (emitter face `(0,0,-6.5)`),
  `lesson_mining.rs`, `examples/playable/mine_and_sell.rs` (tug collar cell),
  `loop_docked_helm.rs` and `lesson_cargo.rs` (tug and freighter collars).
- Hand-written content: `assets/mods/example/example.content.ron`,
  `webmods/gauntlet/gauntlet.content.ron`,
  `crates/nova_bench/scenarios/arsenal.content.ron` and `range.content.ron`.
- Frozen news images: `web/src/assets/news-0160-start-hulls.png`,
  `news-0130-block-fleet.png`. Living wiki images come from hollow, ring and
  drydock scenes that fly the interceptor or the tug.
- Not found: a start hull in raider bands or encounters. Escort raiders are
  generated.

## F. Render rigs

| Rig | What it frames | Light and backdrop | Run |
|---|---|---|---|
| `examples/screenshots/screenshot_base_fleet.rs` (default) | nine catalog hulls in a row, one still each from above the port bow, fixed eye offset times a hand factor per hull | `ThreePointRig`, game skybox | `cargo run --features debug --example screenshot_base_fleet` |
| same, `-- --looks` (added for this task) | six start hulls, four views each, camera sized from the hull's cells | same rig, grey backdrop, ambient 250, 20 degree lens | section I |
| `examples/screenshots/news_0160_start_hulls.rs` | six role hulls in one grid, bow left, rolled 0.55 rad to the lens, one still | `ThreePointRig`, skybox | `--example news_0160_start_hulls` |
| `examples/screenshots/screenshot_hero_ship.rs` | interceptor close-up in the drydock scene | scene lights | `--example screenshot_hero_ship` |
| `examples/playable/greeble_solve.rs` | one catalog hull with the greeble solve drawn over it | game | `--example greeble_solve` |
| `examples/playable/first_shift_ships.rs` | eight hulls side by side, free fly | game | `--example first_shift_ships` |
| `examples/screenshots/screenshot_generated_ships.rs` | four generated hulls, one per role, and a wreck | rig | `--example screenshot_generated_ships` |
| `examples/screenshots/screenshot_diagonal_skin.rs`, `screenshot_solar_wing.rs` | hand-built cells, skin cases | rig | by name |
| `crates/nova_hud/src/preview/hull.rs` | the in-game turntable: New Game role step, sale and hire cards | own key and rim, ambient 220, theme surface colour (`:59,171,179-180`) | in game |
| `crates/nova_editor/src/ui/thumbnail.rs` | editor asset tiles, 112 px | ambient 300 (`:273`) | in editor |

Notes:

- `ThreePointRig`: key from `(-60, 50, 60)` at 11000 lux, warm, the only shadow
  caster. Rim from `(30, 40, -80)` at 16000 lux, cold. Fill from `(70, -20,
  40)` at 2600 lux (`crates/nova_scenario/src/objects/light.rs:113-141`).
- Light problem: a game scene has no ambient light. A face that neither key nor
  fill reaches is black. On the near-black skybox the salvage and armoured
  walls (0.004 to 0.007 linear) merge with the sky. The default fleet stills
  show this. The tug and the freighter read; the raider and the hauler lose
  their shaded side.
- The default fleet mode also catches the next hull in the frame corner and
  fills about a third of the frame width.
- Clearest frame under lavapipe: the `--looks` mode. All 24 frames are
  readable, none is clipped, and the hull fills 50 to 89 percent of the width.
- The first-contact view in the game is the `nova_hud` turntable, not the
  flight camera. It has its own lights.

## G. Prior art

Hulls that read better, and what they do that the start hulls do not:

- `line_warship` (`block.rs:621-712`): 15 cells long on a 3-wide body. A
  one-cell nose step, a keel, two 12-cell shoulders, a tower. At that length a
  cell lump is small against the hull, so the chamfers read as detail and the
  shoulders read as lines. Greebles repeat along the shoulder as a belt.
- Generated hulls (`crates/nova_world_base/src/ship_layout.rs:4-11,462`): a
  mirrored spine. The nose starts at one cell and widens linearly
  (`:1253`, `:2005-2006`). A seeded waist, shoulder, taper or flare changes the
  section along the length (`:2010-2012`). A minimum length per cell of width
  keeps every hull long (`:1176-1192`). Drives are a centred single or mirrored
  pairs (`:1555-1586`).
- Editor WFC hull (`crates/nova_wfc/src/plan.rs:230-275`): "Below three a hull
  is all rim and comes out a heap of ramps" (`:238-240`). Four cells to a side
  gives every face an interior for a flat plate.
- `freighter_derelict` (`block.rs:363-385`): a solar wing and cargo zones give
  it parts of different kinds. Both need advancement above 0.
- Modelled craft (Racer, CargoA, CargoB) left the base game. They ship only in
  `webmods/the-ledger` (reported by a search agent, not rechecked).

Against these, the start hulls: one- and two-cell runs everywhere (the nose,
the cab, the wings, the pod, the strakes), so most plates are rim plates; no
mirrored taper; interceptor and raider are as wide as they are long.

Owner statements on looks:

- `tasks/20261116-132144/TASK.md:11` (2026-11-16): "ships look a bit meh and
  could be done nicer".
- `tasks/20261013-133631/TASK.md:8` (2026-10-13): "I don't like that it forms
  "spikes" would like to not have that happen anymore". Same file `:18`: the
  hybrid "looks really good".
- `tasks/20260816-112429/TASK.md:9-11` (2026-08-16): the styles "look good,
  that's cool, but the placement is meh, they get placed on ridges and spikes
  and they look weird, most shell decorations would look good only on flat
  surfaces". `:14-16`: a saddle plate is "kind of ugly". `:24-26`: "I basically
  want to use skin only for the 'block' ships".
- `tasks/20260816-194637/TASK.md:15-16`: armoured and civilian are "pretty low
  kind of equal (they have not that much variety)". `:26-27`: "combat and
  civilian have to look clean and official, but scrap can go ham and add all
  sorts of things". `:29-31`: block ships could become "the flagship for ships
  instead of the kenney models". `:34-35`: a building block is "well this thing
  looks nice, it can be used for an actual ship". `:87-88`: "the new greeble
  kit looks good... I still think salvage > industrial, but now armoured and
  civilian are closer to them."
- `tasks/20260816-200255/TASK.md:23-24`: "let the style shell actually make it
  look good" (on the plain thruster model).
- Not found: an owner statement on silhouette, proportion, colour or livery of
  the start hulls. The "meh" remark comes from an owner review of a claude.ai
  artifact (`tasks/20261116-132144/TASK.md:7-8`). That page is not in the
  repository and was not read.

## H. Levers, ranked by effect per effort

Each line: what it changes, the cost, the balance cost.

1. Rearrange the same cells. Longer body, mirrored taper, a stepped nose, runs
   of three or more cells so faces get flat plates, diagonal stairs where a
   slope is wanted. Cost: builder edits in `block.rs`, regenerate content,
   update the pins in section E (interceptor web table first). Balance: none
   on paper if counts by kind stay. Real: skin mass, inertia, turret arcs and
   the collar position move. Rerun the census.
2. Move the specials. Drive pattern (row, cross, mirrored pairs), turret seats,
   the intake's deck, the collar's station. The bells, the intake door and the
   collar ring are the three largest readable features on every hull. Cost and
   balance: as 1. Rules: exit lanes, intake against dock approach, collar
   clearance.
3. Style palette. Two finishes per style. Cost: four numbers in `styles.rs`.
   Balance: none. Limit: it changes every hull in that style, the generated
   ones too. The style comments record that value is a weak lever and hue a
   strong one.
4. A style per start hull. Interceptor and picket share armoured; hauler and
   raider share salvage. A new style id gives a hull its own colours, kit
   weights, density and seed. Cost: a style builder, its lint, docs in
   `web/src/create/styles.md`. Balance: none. Kit reuse is free; new greeble
   models cost a recipe each.
5. Greeble rules. `empty` weight for density, `line` for belts, `alone` caps.
   Armoured places 0.30 pieces per plate against 0.62 on the hauler. Cost:
   weights in `styles.rs`, the fit test. Balance: none (a piece has health and
   a small mass). Limit: per style.
6. New greeble recipes. Larger pieces need a per-recipe budget above 0.5 cell.
   Cost: a JSON recipe, `gen-greebles.py`, a bundle line, a kit line. Balance:
   none.
7. Section art. The thruster bell, the turret pan, the collar and the intake
   are the same model on all six hulls. Cost: `scripts/gen-section-parts.py`
   work. Balance: none. Effect is global.
8. Add or remove cells. The widest freedom, and the only lever with a direct
   balance cost: +1 mass, +60 HP, +100 kg, worth, acceleration per cell. Moves
   the stated orderings if not paired with a drive or kit change.

Not possible with the current vocabulary:

- A true slope, wedge, wing or fin part. A slope exists only as skin over a
  staircase of full cubes, each with mass, HP and hold.
- A thin feature. The thinnest structure is one cube, and its cladding makes
  it two cells thick (the tug boom).
- Anything aft of or beside a drive lane: no cowl, no nacelle shroud.
- Per-cell or per-hull colour, a stripe, a decal layer, a trim colour.
- Emissive panels, windows that glow, running lights.
- Per-hull greeble seed or density inside a shared style.
- A second plating material at advancement 0. Reinforced, cargo and tank cubes
  need 0.3.
- Half-cell structure. Only the turret is smaller than a cell.

## I. Baseline frames and the rig

Frames: `tasks/20261116-132144/before/<hull>-bow.png`, `-stern.png`,
`-profile.png`, `-top.png` for the six hulls, 1280 x 720. Sheets:
`before/sheet.png` (six rows; bow, stern, profile) and `before/sheet-top.png`.

Rig: `examples/screenshots/screenshot_base_fleet.rs`, new `--looks` flag. The
default mode is unchanged (checked: nine stills; the compared still differs
from the run before the edit by 9 pixels).

- The hull turns, the camera stands still on one bearing, so every view of
  every hull takes the same light from the same side.
- Views, as the viewer's place in the hull frame: bow `(-1, 0.62, -0.9)`, stern
  `(1, 0.62, 0.9)`, profile `(-1, 0, 0)` (port flank, bow left), top `(0, 1,
  0)` (bow left, starboard up).
- Camera distance is solved from the design's own cells, padded 0.9 cell, to
  fit 94 percent of the frame.
- Rig-only changes, none in the game: grey backdrop (linear 0.55) in place of
  the skybox, ambient light 250, a 20 degree lens in place of the 45 degree
  one. With the game lens the near corner of the hull set the distance and
  the hull filled well under half of the width in the quarter views.
- Hull width in the frame, measured with `magick -fuzz 3% -trim`: 643 px
  (interceptor stern) to 1134 px (freighter profile).

Commands (fish). The capture run wipes its capture directory.

```fish
set S /tmp/nova-looks
mkdir -p $S/cap $S/xdg-config $S/xdg-data
cd ~/personal/nova-protocol
env XDG_CONFIG_HOME=$S/xdg-config XDG_DATA_HOME=$S/xdg-data \
    NOVA_CAPTURE_DIR=$S/cap NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
    NOVA_AUTOPILOT_DEADLINE=3000 NOVA_CAPTURE_SIZE=1280x720 \
    ALSA_CONFIG_PATH=/dev/null \
    VK_DRIVER_FILES=/nix/store/vi9s3w1viw56plx4c8wbi52zgmklcv5i-graphics-drivers/share/vulkan/icd.d/lvp_icd.x86_64.json \
    nix develop --command xvfb-run -a -s "-screen 0 1920x1080x24" \
    cargo run --features debug --example screenshot_base_fleet -- --looks

set B tasks/20261116-132144/after
mkdir -p $B
cp $S/cap/*.png $B/
cd $B
set hulls mining_tug salvage_hauler light_freighter interceptor raider patrol_picket
set args
for h in $hulls
    for v in bow stern profile
        set -a args -label "$h  $v" $h-$v.png
    end
end
montage -font DejaVu-Sans -pointsize 22 -fill white -background '#202020' \
    $args -tile 3x6 -geometry 640x360+6+6 -depth 8 sheet.png
set args
for h in $hulls
    set -a args -label "$h  top" $h-top.png
end
montage -font DejaVu-Sans -pointsize 22 -fill white -background '#202020' \
    $args -tile 3x2 -geometry 640x360+6+6 -depth 8 sheet-top.png
```

The capture takes about two minutes after the build. The montage lines were run
in fish and gave a byte-identical `sheet.png`.

Observations, from the frames:

All six:

- Same three features on every hull at the same size: the 3 x 2 intake door,
  the collar ring, the bell. They are the brightest parts of every frame.
- Every hull is built from one-cell chamfered lumps of one size. No long line
  runs the length of any hull.
- One colour per hull. No accent, no stripe that reads, no light but the bells
  and the intake strip.

`mining_tug`:

- A long flat slab, one deck and a two-cell cab. No taper. The stern is a flat
  wall with three bells in a row at keel level.
- The two-cell boom does not read as a boom. Cladding makes it a box two cells
  wide, joined to the body by a short neck. The beam is a recessed panel on a
  blunt face.
- The flanks repeat the same trapezoid lump four times a side.
- Warm beige all over. Greebles are small dark grilles and ducts. None of the
  large industrial pieces (stack, crane, hazard band) was placed.
- The intake door is the largest feature and takes the aft third of the deck.

`salvage_hauler`:

- A brick: 3 x 3 x 6 with a snout. Nearly as tall as it is wide.
- The flank is one flat wall three cells high. All its interest is greebles:
  patch plates, strips, a drum, green doors. The density reads right.
- Five bells in a cross, set in a notch in the transom. The clearest stern of
  the six.
- Dark brown on dark brown. Top and wall are close in value. The grey patch
  plates are the only contrast.
- The one turret is small in a pale pan and is hard to find in both quarter
  views.

`light_freighter`:

- The most varied massing: cab, two-deck stack, starboard pod, transom cross.
  From the bow quarter it reads as a pile of rounded lumps with no long axis.
- Lopsided in plan: pod to starboard, collar to port.
- Five bells on a cross, the dorsal and ventral ones on one-cell stalks. A
  busy stern that reads as engines.
- Off-white all over. Civilian pieces are few and small. The port flank is a
  blank face about four cells by two with one orange dot.
- The livery and stripe pieces do not read at this distance.

`interceptor`:

- The stubbiest hull: 5 x 5 in plan, two decks. It reads as a box. The bow is
  a flat wall three cells by two.
- The wings are one cell at keel level at the stern. They read as side blocks.
- Five bells in one line under a blank transom wall. From the stern quarter
  they read as a row of wheels.
- The intake door covers about 40 percent of the plan. The two turrets sit in
  pale pans on the fore deck.
- Dark grey all over, 17 small pieces. Large blank faces.

`raider`:

- From the port bow it is close to the interceptor: a two-deck box with the
  intake aft. Same mass, 38 sections each.
- The outrigger shows only from the stern and from above. From the port bow
  the hull looks symmetric.
- The bow is a one-cell snout under a bluff two-deck wall.
- Four bells in a row at keel level, the outrigger bell one cell forward.
- The salvage pieces (patch plates, strips, cog, green panel) give it more
  life than the interceptor. Dark brown.

`patrol_picket`:

- The most ship-like plan: a narrow nose that widens to the strakes aft.
- Nose-heavy in profile. The nose turret box, the spine and the keel bay make
  the bow four cells high; the stern is one cell high.
- The strakes are plain one-cell boxes either side of the drives. They hide
  half of the three bells from the stern quarter.
- The two turret boxes are pale grey on a dark hull and stand out as bare
  cubes.
- Same dark grey and same pieces as the interceptor. The two armoured hulls
  differ in massing only.
