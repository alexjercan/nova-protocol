# Base Content Catalog (Nova Protocol)

Raw data artifact: every id and asset a mod can reference from the shipped
base game ("Base Game" bundle, id `base`).

Sources of truth:

- Rust builders: `crates/nova_authoring/src/sections.rs` (section catalog),
  `crates/nova_authoring/src/scenario/craft.rs` (cut-cube ship prototypes),
  `crates/nova_authoring/src/scenario*.rs` (scenarios),
  `crates/nova_authoring/src/scenario_generation.rs` (campaigns + file map).
- Generated tree: `assets/base/**/*.content.ron` (written by
  `cargo run -p nova_authoring --bin content -- gen`; parity test
  `content_ron_parity` pins builders == files). Never hand-edit the RON.
- Manifest: `assets/base/base.bundle.ron` (content file list + the declared
  `resources` that gate `dep://base/...`).
- Catalog: `assets/mods.catalog.ron` (entry `id: "base"`, `base: true`).

Counts: 123 sections, 10 scenarios, 1 campaign, 140 declared resource files
(117 glb, 13 wav, 3 textures, 6 thumbnails, 1 banner).

Content kinds that exist in base content files: `Section`, `Scenario`,
`Campaign` (the full `Content` enum,
`crates/nova_modding/src/lib.rs:68-80`). No ships/factions as content items:
ships are `Spaceship` scenario OBJECTS built from section-prototype ids
inside scenario events, not standalone content.

---

## 1. Section prototypes (123)

All defined by `build_sections`
(`crates/nova_authoring/src/sections.rs:183-439`), serialized into
`assets/base/sections/base.content.ron`. Registry key: `SectionConfig.base.id`.
Mass is 1.0 for every section except where noted (only
`example` MOD content differs; all 123 base sections are mass 1.0).

Editor palette visibility: the 7 core sections have `hide_in_editor: false`;
ALL 116 cut-cube prototypes are `hide_in_editor: true`
(`crates/nova_authoring/src/scenario/craft.rs:205-209`), so the editor
sandbox palette shows only the 7 core sections. Mods can still reference
every id.

### 1.1 Core sections (7)

Builder: `crates/nova_authoring/src/sections.rs` (line = id definition).

| id | kind | display name | health | key stats | builder |
|---|---|---|---|---|---|
| `reinforced_hull_section` | Hull | Reinforced Hull Section | 200 | mesh `self://gltf/hull-01.glb#Scene0` | sections.rs:185-201 |
| `basic_thruster_section` | Thruster | Basic Thruster Section | 70 | magnitude 1.0, loop sound `thruster_loop.wav` | sections.rs:202-223 |
| `basic_controller_section` | Controller | Basic Controller Section | 100 | freq 4.0, damping 4.0, max_torque 40.0, full radar/lock/RCS sound set | sections.rs:224-263 |
| `better_turret_section` | Turret | Better Turret Section | 130 | Kinetic 4.0/hit @ 100 rps, muzzle 100 u/s, ammo 500, reload 3.0s dump-refill (player PDC) | sections.rs:264-312 |
| `light_hull_section` | Hull | Light Hull Section | 60 | scavenger-grade hull, same hull-01 mesh | sections.rs:313-331 |
| `light_turret_section` | Turret | Light Turret Section | 60 | Kinetic 3.825/hit @ 25 rps, muzzle 60 u/s, ammo 150, reload 2.5s (scavenger gun) | sections.rs:332-383 |
| `torpedo_section` | Torpedo | Torpedo Bay Section | 100 | blast 100 dmg / 30 u radius, ammo 6, rearm 1 per 4.0s, max_speed 35, nav_constant 3.0, mesh `torpedo-bay-01.glb` | sections.rs:384-431 |

All 7 share `impact.wav` / `explosion.wav` as impact/destroy sounds. Turrets
share the joint tree `turret_joint_tree` (sections.rs:49-109): base ->
yaw(Y) -> pitch(X, -30..90 deg) -> barrel -> muzzle, meshes
`turret-yaw-01/turret-pitch-01/turret-barrel-01.glb`.

### 1.2 Racer prototypes (20)

Builder: `racer_prototypes`,
`crates/nova_authoring/src/scenario/craft.rs:394-468`; cube table
`RACER_CUBES` craft.rs:21-40; id stem helper `stem`/`enc` craft.rs:171-182.
One prototype per cut cube of the Kenney craft_racer; meshes
`self://gltf/racer/cube_i{I}_j{J}_k{K}.glb#Scene0`. Player-grade HP: hull 60,
thruster 70, controller 100, turret 130, light turret 60 (craft.rs:150-154).

| id | kind | display name | health |
|---|---|---|---|
| `racer_cube_i0_j0_k0` | Hull | Racer Cube (0,0,0) | 60 |
| `racer_cube_i0_j0_k1` | Hull | Racer Cube (0,0,1) | 60 |
| `racer_cube_i0_j0_k2` | Hull | Racer Cube (0,0,2) | 60 |
| `racer_cube_i0_j0_km1` | Hull | Racer Cube (0,0,-1) | 60 |
| `racer_cube_i0_j0_km2` | Hull | Racer Cube (0,0,-2) | 60 |
| `racer_cube_i0_j1_k0` | Controller | Racer Controller | 100 |
| `racer_cube_i0_j1_k1` | Hull | Racer Cube (0,1,1) | 60 |
| `racer_cube_i0_j1_k2` | Hull | Racer Cube (0,1,2) | 60 |
| `racer_cube_i0_j1_km1` | Hull | Racer Cube (0,1,-1) | 60 |
| `racer_cube_i0_j1_km2` | Hull | Racer Cube (0,1,-2) | 60 |
| `racer_cube_i1_j0_k0` | Hull | Racer Cube (1,0,0) | 60 |
| `racer_cube_i1_j0_k1` | Hull | Racer Cube (1,0,1) | 60 |
| `racer_cube_i1_j0_k2` | Thruster | Racer Thruster (1,0,2) | 70 |
| `racer_cube_i1_j0_km1` | Turret | Racer Turret (1,0,-1) | 130 |
| `racer_light_cube_i1_j0_km1` | Turret | Racer Light Turret (1,0,-1) | 60 |
| `racer_cube_im1_j0_k0` | Hull | Racer Cube (-1,0,0) | 60 |
| `racer_cube_im1_j0_k1` | Hull | Racer Cube (-1,0,1) | 60 |
| `racer_cube_im1_j0_k2` | Thruster | Racer Thruster (-1,0,2) | 70 |
| `racer_cube_im1_j0_km1` | Turret | Racer Turret (-1,0,-1) | 130 |
| `racer_light_cube_im1_j0_km1` | Turret | Racer Light Turret (-1,0,-1) | 60 |

Notes:

- `racer_cube_i0_j1_k0` = "Racer Controller" (craft.rs:443-453): the searchable
  display name in the editor; max_torque 800.0, freq 4.0, damping 4.0.
- Racer turrets = player PDC stats (4.0/hit @ 100 rps, ammo 500); the
  `racer_light_*` pair = scavenger stats (3.825/hit @ 25 rps, ammo 150),
  used by AI enemies (craft.rs:249-295).
- Racer thrusters carry a rect exhaust (0.3 x 0.5, craft.rs:431-441).
- On-ship section-instance ids (and input-mapping keys) are the bare stems,
  e.g. the player fire binding targets `cube_i1_j0_km1` / `cube_im1_j0_km1`
  (`RACER_TURRET_IDS`, craft.rs:146).

### 1.3 Cargob prototypes (43) - the Rust Tally boss hull

Builder: `cargob_prototypes`, craft.rs:472-554; cube table `CARGOB_CUBES`
craft.rs:41-84. Meshes `self://gltf/cargob/...`. Hull HP 70 (craft.rs:155).

| id | kind | display name | health |
|---|---|---|---|
| `cargob_cube_i0_j0_k0` | Hull | Cargo Cube (0,0,0) | 70 |
| `cargob_cube_i0_j0_k1` | Hull | Cargo Cube (0,0,1) | 70 |
| `cargob_cube_i0_j0_k2` | Hull | Cargo Cube (0,0,2) | 70 |
| `cargob_cube_i0_j0_km1` | Hull | Cargo Cube (0,0,-1) | 70 |
| `cargob_cube_i0_j1_k2` | Hull | Cargo Cube (0,1,2) | 70 |
| `cargob_cube_i0_j1_km1` | Hull | Cargo Cube (0,1,-1) | 70 |
| `cargob_cube_i0_j1_km2` | Hull | Cargo Cube (0,1,-2) | 70 |
| `cargob_cube_i0_j2_k0` | Hull | Cargo Cube (0,2,0) | 70 |
| `cargob_cube_i0_j2_k1` | Hull | Cargo Cube (0,2,1) | 70 |
| `cargob_cube_i0_j2_k2` | Hull | Cargo Cube (0,2,2) | 70 |
| `cargob_cube_i0_j2_km1` | Hull | Cargo Cube (0,2,-1) | 70 |
| `cargob_cube_i0_j2_km2` | Hull | Cargo Cube (0,2,-2) | 70 |
| `cargob_cube_i1_j0_k0` | Hull | Cargo Cube (1,0,0) | 70 |
| `cargob_cube_i1_j0_k1` | Hull | Cargo Cube (1,0,1) | 70 |
| `cargob_cube_i1_j0_k2` | Hull | Cargo Cube (1,0,2) | 70 |
| `cargob_cube_i1_j0_km1` | Hull | Cargo Cube (1,0,-1) | 70 |
| `cargob_cube_i1_j0_km2` | Hull | Cargo Cube (1,0,-2) | 70 |
| `cargob_cube_i1_j1_k0` | Hull | Cargo Cube (1,1,0) | 70 |
| `cargob_cube_i1_j1_k1` | Hull | Cargo Cube (1,1,1) | 70 |
| `cargob_cube_i1_j1_k2` | Thruster | Cargo Thruster (1,1,2) | 70 |
| `cargob_cube_i1_j1_km1` | Hull | Cargo Cube (1,1,-1) | 70 |
| `cargob_cube_i1_j1_km2` | Torpedo | Cargo Torpedo Bay (1,1,-2) | 100 |
| `cargob_cube_i1_j2_k0` | Turret | Cargo Turret (1,2,0) | 130 |
| `cargob_cube_i1_j2_k1` | Hull | Cargo Cube (1,2,1) | 70 |
| `cargob_cube_i1_j2_k2` | Hull | Cargo Cube (1,2,2) | 70 |
| `cargob_cube_i1_j2_km1` | Hull | Cargo Cube (1,2,-1) | 70 |
| `cargob_cube_i1_j2_km2` | Hull | Cargo Cube (1,2,-2) | 70 (collider Cuboid 0.8) |
| `cargob_cube_im1_j0_k0` | Hull | Cargo Cube (-1,0,0) | 70 |
| `cargob_cube_im1_j0_k1` | Hull | Cargo Cube (-1,0,1) | 70 |
| `cargob_cube_im1_j0_k2` | Hull | Cargo Cube (-1,0,2) | 70 |
| `cargob_cube_im1_j0_km1` | Hull | Cargo Cube (-1,0,-1) | 70 |
| `cargob_cube_im1_j0_km2` | Hull | Cargo Cube (-1,0,-2) | 70 |
| `cargob_cube_im1_j1_k0` | Hull | Cargo Cube (-1,1,0) | 70 |
| `cargob_cube_im1_j1_k1` | Hull | Cargo Cube (-1,1,1) | 70 |
| `cargob_cube_im1_j1_k2` | Thruster | Cargo Thruster (-1,1,2) | 70 |
| `cargob_cube_im1_j1_km1` | Hull | Cargo Cube (-1,1,-1) | 70 |
| `cargob_cube_im1_j1_km2` | Torpedo | Cargo Torpedo Bay (-1,1,-2) | 100 |
| `cargob_cube_im1_j2_k0` | Turret | Cargo Turret (-1,2,0) | 130 |
| `cargob_cube_im1_j2_k1` | Hull | Cargo Cube (-1,2,1) | 70 |
| `cargob_cube_im1_j2_k2` | Hull | Cargo Cube (-1,2,2) | 70 |
| `cargob_cube_im1_j2_km1` | Hull | Cargo Cube (-1,2,-1) | 70 |
| `cargob_cube_im1_j2_km2` | Hull | Cargo Cube (-1,2,-2) | 70 (collider Cuboid 0.8) |
| `cargob_core_controller` | Controller | Core Controller | 100 (no mesh, max_torque 800) |

Cargob turrets = player-grade PDC stats; torpedo bays = same numbers as
`torpedo_section` (blast 100/30 u, ammo 6, 1 per 4s) with the cube as
render mesh (craft.rs:342-370). Core controller: craft.rs:544-552.

### 1.4 Cargoa prototypes (53) - the unarmed hauler

Builder: `cargoa_prototypes`, craft.rs:560-603; cube table `CARGOA_CUBES`
craft.rs:89-142. Meshes `self://gltf/cargoa/...`. Unarmed: hull cubes + two
rear thrusters + hollow-core controller only. Hull HP 70 (craft.rs:156).

| id | kind | display name | health |
|---|---|---|---|
| `cargoa_cube_i0_j0_k0` | Hull | Cargo Cube (0,0,0) | 70 |
| `cargoa_cube_i0_j0_k1` | Hull | Cargo Cube (0,0,1) | 70 |
| `cargoa_cube_i0_j0_k2` | Hull | Cargo Cube (0,0,2) | 70 |
| `cargoa_cube_i0_j0_km1` | Hull | Cargo Cube (0,0,-1) | 70 |
| `cargoa_cube_i0_j1_k1` | Hull | Cargo Cube (0,1,1) | 70 |
| `cargoa_cube_i0_j1_k2` | Hull | Cargo Cube (0,1,2) | 70 |
| `cargoa_cube_i0_j1_km1` | Hull | Cargo Cube (0,1,-1) | 70 |
| `cargoa_cube_i0_j1_km2` | Hull | Cargo Cube (0,1,-2) | 70 |
| `cargoa_cube_i0_j2_k0` | Hull | Cargo Cube (0,2,0) | 70 |
| `cargoa_cube_i0_j2_k1` | Hull | Cargo Cube (0,2,1) | 70 |
| `cargoa_cube_i0_j2_km1` | Hull | Cargo Cube (0,2,-1) | 70 |
| `cargoa_cube_i0_j2_km2` | Hull | Cargo Cube (0,2,-2) | 70 |
| `cargoa_cube_i1_j0_k0` | Hull | Cargo Cube (1,0,0) | 70 |
| `cargoa_cube_i1_j0_k1` | Hull | Cargo Cube (1,0,1) | 70 |
| `cargoa_cube_i1_j0_k2` | Hull | Cargo Cube (1,0,2) | 70 |
| `cargoa_cube_i1_j0_km1` | Hull | Cargo Cube (1,0,-1) | 70 |
| `cargoa_cube_i1_j1_k0` | Hull | Cargo Cube (1,1,0) | 70 |
| `cargoa_cube_i1_j1_k1` | Hull | Cargo Cube (1,1,1) | 70 |
| `cargoa_cube_i1_j1_k2` | Thruster | Cargo Thruster (1,1,2) | 70 |
| `cargoa_cube_i1_j1_km1` | Hull | Cargo Cube (1,1,-1) | 70 |
| `cargoa_cube_i1_j1_km2` | Hull | Cargo Cube (1,1,-2) | 70 |
| `cargoa_cube_i1_j2_k0` | Hull | Cargo Cube (1,2,0) | 70 |
| `cargoa_cube_i1_j2_k1` | Hull | Cargo Cube (1,2,1) | 70 |
| `cargoa_cube_i1_j2_km1` | Hull | Cargo Cube (1,2,-1) | 70 |
| `cargoa_cube_i1_j2_km2` | Hull | Cargo Cube (1,2,-2) | 70 |
| `cargoa_cube_i2_j0_k0` | Hull | Cargo Cube (2,0,0) | 70 |
| `cargoa_cube_i2_j0_k1` | Hull | Cargo Cube (2,0,1) | 70 |
| `cargoa_cube_i2_j0_km1` | Hull | Cargo Cube (2,0,-1) | 70 |
| `cargoa_cube_i2_j1_k0` | Hull | Cargo Cube (2,1,0) | 70 |
| `cargoa_cube_i2_j1_k1` | Hull | Cargo Cube (2,1,1) | 70 |
| `cargoa_cube_i2_j1_k2` | Hull | Cargo Cube (2,1,2) | 70 |
| `cargoa_cube_i2_j1_km1` | Hull | Cargo Cube (2,1,-1) | 70 |
| `cargoa_cube_im1_j0_k0` | Hull | Cargo Cube (-1,0,0) | 70 |
| `cargoa_cube_im1_j0_k1` | Hull | Cargo Cube (-1,0,1) | 70 |
| `cargoa_cube_im1_j0_k2` | Hull | Cargo Cube (-1,0,2) | 70 |
| `cargoa_cube_im1_j0_km1` | Hull | Cargo Cube (-1,0,-1) | 70 |
| `cargoa_cube_im1_j1_k0` | Hull | Cargo Cube (-1,1,0) | 70 |
| `cargoa_cube_im1_j1_k1` | Hull | Cargo Cube (-1,1,1) | 70 |
| `cargoa_cube_im1_j1_k2` | Thruster | Cargo Thruster (-1,1,2) | 70 |
| `cargoa_cube_im1_j1_km1` | Hull | Cargo Cube (-1,1,-1) | 70 |
| `cargoa_cube_im1_j1_km2` | Hull | Cargo Cube (-1,1,-2) | 70 |
| `cargoa_cube_im1_j2_k0` | Hull | Cargo Cube (-1,2,0) | 70 |
| `cargoa_cube_im1_j2_k1` | Hull | Cargo Cube (-1,2,1) | 70 |
| `cargoa_cube_im1_j2_km1` | Hull | Cargo Cube (-1,2,-1) | 70 |
| `cargoa_cube_im1_j2_km2` | Hull | Cargo Cube (-1,2,-2) | 70 |
| `cargoa_cube_im2_j0_k0` | Hull | Cargo Cube (-2,0,0) | 70 |
| `cargoa_cube_im2_j0_k1` | Hull | Cargo Cube (-2,0,1) | 70 |
| `cargoa_cube_im2_j0_km1` | Hull | Cargo Cube (-2,0,-1) | 70 |
| `cargoa_cube_im2_j1_k0` | Hull | Cargo Cube (-2,1,0) | 70 |
| `cargoa_cube_im2_j1_k1` | Hull | Cargo Cube (-2,1,1) | 70 |
| `cargoa_cube_im2_j1_k2` | Hull | Cargo Cube (-2,1,2) | 70 |
| `cargoa_cube_im2_j1_km1` | Hull | Cargo Cube (-2,1,-1) | 70 |
| `cargoa_core_controller` | Controller | Core Controller | 100 (no mesh, max_torque 800) |

Display-name collision note: cargob and cargoa cubes share the "Cargo Cube
(i,j,k)" / "Cargo Thruster (i,j,k)" display names, and both core controllers
are named "Core Controller". Ids are always unique; search by id when the
name is ambiguous.

---

## 2. Scenarios (10)

Registry key: `ScenarioConfig.id`. Generated to
`assets/base/scenarios/<id>.content.ron`. Build order:
`build_scenarios` (`crates/nova_authoring/src/scenario_generation.rs:52-80`).
`hidden: true` scenarios do not appear in the Scenarios picker but ARE
launchable by id (NextScenario chains, campaign membership, menu backdrops).

| id | display name | hidden | builder | contents (one line) |
|---|---|---|---|---|
| `asteroid_field` | Asteroid Field | no | scenario.rs:31-445 | Sandbox: player + dummy ship (core sections), 20 seeded scatter rocks, one invulnerable gravity rock, destroy-5-then-reach-zone objectives. |
| `asteroid_next` | Asteroid Field - Next | yes | scenario.rs:447-473 | Pure relay: OnStart NextScenario cut back into `asteroid_field`. |
| `menu_ambience` | Menu Ambience | yes | scenario/menu.rs:20 | Main-menu backdrop: planetoid with gravity well, rock scatter, one AI ship flying an orbit. No player. |
| `menu_waystation` | Waystation Traffic | yes | scenario/menu.rs:287 | Menu backdrop: two haulers ("Hauler Biscuit" etc.) in convoy orbit, dock beacons, cargo-rock lane. |
| `menu_scrapyard` | Scrapyard Drift | yes | scenario/menu.rs:390 | Menu backdrop: salvage yard - lazy tug "Tug Pebble", tumbling crates, wreck rocks, one beacon. |
| `shakedown_run` | Shakedown Run | no | scenario/shakedown/mod.rs:726 | New Game start (chapter 1): five tutorial beats - beacons, freelook, crate salvage, autopilot GOTO/ORBIT, one gentle pirate (enemy-grade racer). |
| `broadside` | Broadside | no | scenario/broadside.rs:309 | Chapter 2 part 1: answer a hauler distress call, break a two-corvette scavenger ambush; Victory chains into `broadside_gunship`. |
| `broadside_gunship` | Broadside: Rust Tally | yes | scenario/broadside.rs:608 | Chapter 2 part 2: the cargob gunship boss - screen torpedoes with the PDC, break it section by section; chains into `lifeline`. |
| `lifeline` | Lifeline | no | scenario/lifeline.rs:416 | Chapter 3 part 1: convoy defense - screen loitering unarmed haulers against raider waves until a relief countdown expires. |
| `final_tally` | Final Tally | yes | scenario/final_tally.rs:339 | Chapter 3 part 2 (finale): gravity-well anchorage, survey-by-lock, orbital picket, flagship + escort fight, epilogue. |

Scenario id constants: `SHAKEDOWN_SCENARIO_ID` (shakedown/mod.rs:32),
`BROADSIDE_SCENARIO_ID` / `BROADSIDE_GUNSHIP_SCENARIO_ID`
(broadside.rs:60-61), `LIFELINE_SCENARIO_ID` (lifeline.rs:48),
`FINAL_TALLY_SCENARIO_ID` (final_tally.rs:45).

New Game start: `new_game_scenario: Some("shakedown_run")` in
`assets/base/base.bundle.ron:187`. Honored ONLY from the base bundle
(`crates/nova_assets/src/merge.rs:242-278`); a mod declaring it is warned
and ignored.

Picker thumbnails exist for the 6 non-menu, non-relay scenarios
(`self://thumbnails/<id>.png`).

---

## 3. Other content kinds

### Campaign (1)

Builder: `build_campaigns`
(`crates/nova_authoring/src/scenario_generation.rs:88-100`); file
`assets/base/campaigns/nova_protocol.content.ron`.

| id | display name | member scenario ids (in play order) |
|---|---|---|
| `nova_protocol` | Nova Protocol | `shakedown_run`, `broadside`, `broadside_gunship`, `lifeline`, `final_tally` |

There are no other content kinds. `Content` =
Section | Scenario | Campaign (`crates/nova_modding/src/lib.rs:68-80`).
Registries: `GameSections` (Vec, palette order), `GameScenarios` (map),
`GameCampaigns` (map).

---

## 4. Assets reachable via dep://base/...

### Resolution (code-confirmed)

- Schemes: `self://` and `dep://<id>/<path>`
  (`crates/nova_assets/src/mod_refs.rs:36-39`), rewritten at bundle-merge
  time, never seen by the AssetServer.
- `base` is the IMPLICIT universal dependency: `dep://base/<path>` is always
  allowed without declaring `base` in `meta.dependencies`
  (mod_refs.rs:93-106, merge.rs:176-187).
- Rewrite: `dep://base/<path>` -> `<base resource_base>/<path>` =
  `base/<path>` (the base bundle lives at `assets/base/base.bundle.ron`, so
  its `resource_base` is `base`; `join_base` mod_refs.rs:241-247). So
  `dep://base/sounds/turret_fire.wav` loads `assets/base/sounds/turret_fire.wav`.
- Membership gate: the `<path>` MUST appear in the base bundle's declared
  `resources` list (`assets/base/base.bundle.ron:24-179`); an undeclared ref
  is a lint/merge Error and is left literal so it fails loudly
  (mod_refs.rs:112-152, merge.rs:218-232). `#Scene0`-style labels are
  stripped before the membership check (mod_refs.rs:251-253).
- Every base ref below is declared, and the declared list exactly matches
  the 140 binary files on disk. Undeclared files under `assets/base/`
  (NOT referenceable): `sounds/README.md`, `textures/cubemap.png.meta`,
  `textures/cubemap_alt.png.meta` (the .meta sidecars ride along with their
  png automatically).

### Meshes - 117 glb files under `dep://base/gltf/`

Top level (5):

- `gltf/hull-01.glb` (hull render mesh, label `#Scene0`)
- `gltf/turret-yaw-01.glb`, `gltf/turret-pitch-01.glb`,
  `gltf/turret-barrel-01.glb` (turret joint meshes)
- `gltf/torpedo-bay-01.glb`

Cut hull cubes (naming `cube_i{I}_j{J}_k{K}.glb`, `m` = minus):

- `gltf/racer/` - 18 files: `cube_i0_j0_k0`, `cube_i0_j0_k1`,
  `cube_i0_j0_k2`, `cube_i0_j0_km1`, `cube_i0_j0_km2`, `cube_i0_j1_k0`,
  `cube_i0_j1_k1`, `cube_i0_j1_k2`, `cube_i0_j1_km1`, `cube_i0_j1_km2`,
  `cube_i1_j0_k0`, `cube_i1_j0_k1`, `cube_i1_j0_k2`, `cube_i1_j0_km1`,
  `cube_im1_j0_k0`, `cube_im1_j0_k1`, `cube_im1_j0_k2`, `cube_im1_j0_km1`.
- `gltf/cargob/` - 42 files: `i0_j0_{k0,k1,k2,km1}`,
  `i0_j1_{k2,km1,km2}`, `i0_j2_{k0,k1,k2,km1,km2}`,
  `i1_j0_{k0,k1,k2,km1,km2}`, `i1_j1_{k0,k1,k2,km1,km2}`,
  `i1_j2_{k0,k1,k2,km1,km2}`, `im1_j0_{k0,k1,k2,km1,km2}`,
  `im1_j1_{k0,k1,k2,km1,km2}`, `im1_j2_{k0,k1,k2,km1,km2}`.
- `gltf/cargoa/` - 52 files: `i0_j0_{k0,k1,k2,km1}`,
  `i0_j1_{k1,k2,km1,km2}`, `i0_j2_{k0,k1,km1,km2}`,
  `i1_j0_{k0,k1,k2,km1}`, `i1_j1_{k0,k1,k2,km1,km2}`,
  `i1_j2_{k0,k1,km1,km2}`, `i2_j0_{k0,k1,km1}`, `i2_j1_{k0,k1,k2,km1}`,
  `im1_j0_{k0,k1,k2,km1}`, `im1_j1_{k0,k1,k2,km1,km2}`,
  `im1_j2_{k0,k1,km1,km2}`, `im2_j0_{k0,k1,km1}`, `im2_j1_{k0,k1,k2,km1}`.

Mesh refs in content carry the gltf scene label: `...glb#Scene0`.

### Sounds - 13 wav files under `dep://base/sounds/`

`dry_fire.wav`, `explosion.wav`, `impact.wav`, `lock_off.wav`,
`lock_on.wav`, `radar_deny.wav`, `radar_retarget.wav`, `rcs_loop.wav`,
`safety_on.wav`, `salvage_pickup.wav`, `thruster_loop.wav`,
`torpedo_launch.wav`, `turret_fire.wav`.

(UI sounds - menu clicks, objective chimes - are engine chrome at the asset
root, NOT part of the base bundle and NOT dep-referenceable.)

### Images - 10 files

- Textures: `textures/cubemap.png` (skybox), `textures/cubemap_alt.png`
  (chapter 2/3 skybox), `textures/asteroid.png`.
- Thumbnails: `thumbnails/asteroid_field.png`, `thumbnails/shakedown_run.png`,
  `thumbnails/broadside.png`, `thumbnails/broadside_gunship.png`,
  `thumbnails/lifeline.png`, `thumbnails/final_tally.png`.
- `banner.png` (the base mod's banner).

Total declared: 117 + 13 + 10 = 140 resources (checked against both the
manifest and the on-disk tree).

---

## 5. Overlay rule: how a mod references/replaces a base item

Implementation: `merge_bundles` + `merge_content_item`
(`crates/nova_assets/src/merge.rs:371-460`), driven by `register_bundles`
(merge.rs:48-340).

- Matching key: the item's id string per kind - `SectionConfig.base.id`,
  `ScenarioConfig.id`, `CampaignConfig.id`. Nothing else (no name, no file
  path) participates in matching.
- Merge order: enabled bundles in catalog order (base first,
  `assets/mods.catalog.ron`), then downloaded mods, topologically re-sorted
  so a dependency merges BEFORE its dependents (merge.rs:105-142).
- CROSS-bundle, same id: last-wins WHOLE-ITEM REPLACE. The later (mod) item
  replaces the earlier (base) item entirely - it is not a field-level patch,
  so an overlay must restate every field it wants to keep
  (merge.rs:357-370, 442-460). Sections replace IN PLACE in the Vec so the
  editor palette order is preserved; scenarios/campaigns are map inserts.
- Same id twice WITHIN one bundle: authoring error - first occurrence kept,
  duplicate skipped, conflict logged (merge.rs:381-425).
- New id: appended/added alongside base content.
- Asset refs inside a mod item are rewritten in the OWNING mod's scope:
  `self://X` -> the mod's own folder (`mods/<id>/X` shipped,
  `mods://<id>/X` downloaded), `dep://base/X` -> `base/X`
  (mod_refs.rs:86-107). So an overlay of a base section can keep base
  meshes/sounds via `dep://base/...` or ship its own via `self://...`.
- Live example: `assets/mods/example/example.content.ron` overlays
  `reinforced_hull_section` (same id -> replaces it everywhere, editor
  palette included) and adds `example_plated_hull_section`,
  `example_arena`, `example_menu`.
- Guard rails: mods cannot redirect New Game (base-only
  `new_game_scenario`), and a downloaded mod whose id shadows a SHIPPED
  catalog id is skipped (merge.rs:73-103).

---

## 6. Id naming conventions

- Character set: lowercase ASCII snake_case throughout; digits allowed;
  the only separator is `_`.
- Core sections: `<variant>_<kind>_section` -
  `reinforced_hull_section`, `basic_thruster_section`,
  `basic_controller_section`, `better_turret_section`,
  `light_hull_section`, `light_turret_section`, `torpedo_section`.
- Cut-cube prototypes: `<ship>_cube_i<I>_j<J>_k<K>` where `<ship>` is
  `racer` | `cargob` | `cargoa` and negative grid coordinates are encoded
  with an `m` prefix (`m1` = -1, `m2` = -2): `racer_cube_im1_j0_km1`.
  Encoder: `enc`/`stem`, craft.rs:171-182.
- Weak AI-turret variants: `racer_light_cube_i<I>_j<J>_k<K>` (only the two
  turret cubes have light variants).
- Hollow-core controllers: `<ship>_core_controller` (`cargob_`, `cargoa_`).
- On-ship section-instance ids (SpaceshipSectionConfig.id, also the
  input-mapping key): the bare cube stem `cube_i<I>_j<J>_k<K>`, or a role
  word (`controller`, `hull_front`, `turret`, `core_controller`) on the
  hand-built ships.
- Scenarios: short snake_case slug; menu backdrops carry the `menu_`
  prefix; a chained continuation suffixes its parent
  (`broadside_gunship`, `asteroid_next`).
- Campaigns: snake_case (`nova_protocol`).
- Scatter-spawned object ids: `id_prefix` + index, e.g. the field rocks use
  `asteroid_` (scenario.rs:39-45); scenario object/area/objective ids are
  short snake_case (`player_spaceship`, `asteroid_zone`, `destroy_asteroids`).
- Content files: one file for ALL sections
  (`base/sections/base.content.ron`), one file per scenario
  (`base/scenarios/<scenario_id>.content.ron`), one per campaign
  (`base/campaigns/<campaign_id>.content.ron`) - map in `content_files`,
  scenario_generation.rs:158-176.
- Mod-facing convention: a mod's own new ids should carry the mod's own
  prefix (the example mod uses `example_*`); reusing a base id means
  "replace that base item".
