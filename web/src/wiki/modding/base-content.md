# Base content catalog

Every id and asset the base game ships, so your mod can reference them
instead of guessing. Three registries matter to a mod author: **section
prototype ids** (what `source: Prototype("<id>")` can name), **scenario ids**
(what `NextScenario` and campaigns can target), and **base asset paths** (what
`dep://base/<path>` can reach). All of it is generated from the Rust builders
by `cargo run content gen` into
`assets/base/**/*.content.ron` - the generated RON is the shipped truth, and
`content -- lint` checks your references against it.

`base` is the IMPLICIT universal dependency: every mod may reference base
content and `dep://base/` assets without listing `base` in
`meta.dependencies`. Declaring it explicitly is allowed and also resolves -
the shipped portal mods do - but it is never required.

## Section prototypes

A prototype id is used two ways: a ship's section list references it
compactly (`source: Prototype("<id>")` resolves the whole config, meshes and
sounds included), and a mod `Section` item that reuses the id REPLACES that
part everywhere. See [Ship sections for mods](../sections/) for
the `Section` grammar and the overlay flow.

Ids are lowercase snake_case. The core parts follow
`<variant>_<kind>_section`; the ship-frame cubes follow
`<ship>_cube_i<I>_j<J>_k<K>` where a negative grid coordinate gets an `m`
prefix (`km1` = k -1). Section KINDS are `Hull`, `Thruster`, `Controller`,
`Turret`, `Torpedo` - the kind decides what the slot does, so this table is
how you tell a structure cube from a gun before you swap one out.

### Core sections (the editor palette)

The seven parts visible in the editor sandbox palette. Health and behavior
numbers are the shipped balance (mass is 1.0 across the whole base catalog).

| id | kind | display name | health | notes |
|---|---|---|---|---|
| `reinforced_hull_section` | Hull | Reinforced Hull Section | 200 | the armor workhorse |
| `basic_thruster_section` | Thruster | Basic Thruster Section | 70 | magnitude 1.0 |
| `basic_controller_section` | Controller | Basic Controller Section | 100 | freq 4.0, damping 4.0, max torque 40 |
| `better_turret_section` | Turret | Better Turret Section | 130 | Kinetic 4.0/hit at 100 rps, ammo 500 |
| `light_hull_section` | Hull | Light Hull Section | 60 | scavenger-grade hull |
| `light_turret_section` | Turret | Light Turret Section | 60 | Kinetic 3.825/hit at 25 rps, ammo 150 |
| `torpedo_section` | Torpedo | Torpedo Bay Section | 100 | blast 100 dmg / 30 u, ammo 6 |

### Racer prototypes (the player ship)

The mainline player ship is built from these 20 cut-cube prototypes (the
Kenney craft_racer hull, one prototype per cube). They are
`hide_in_editor: true` - not in the palette, but fully referenceable. This is
the map of which slot is structure, which is thrust, and which is a gun:

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

So the racer's two `i1/im1_j0_km1` flank cubes are its guns, its two
`i1/im1_j0_k2` tail cubes are its engines, `i0_j1_k0` is the controller, and
everything else is structure. The `racer_light_*` pair are the same two turret slots at
scavenger-grade stats - the shipped way to build a weaker enemy racer is to
swap those two prototype references, not to author new guns.

Two id spaces, easy to conflate: the PROTOTYPE id (`racer_cube_i1_j0_km1`,
this catalog's key) and the on-ship SECTION-INSTANCE id, which the shipped
ships write as the bare stem (`cube_i1_j0_km1`). A ship's `input_mapping`
binds to the INSTANCE id, so you can swap a bound slot's `source:
Prototype(...)` for another prototype (or your own section) without touching
the binding, as long as the instance id stays.

### Cargo hauler prototypes

Two cargo frames ship. `cargob_*` (43 prototypes) is the armed gunship frame
(the Broadside boss); `cargoa_*` (53 prototypes) is the unarmed hauler (the
Lifeline convoy). Display names are formulaic - `Cargo Cube (i,j,k)` /
`Cargo Thruster (i,j,k)` / `Core Controller` - and COLLIDE between the two
families, so search by id, not by name.

Every cube not listed below is a `Hull`, display name `Cargo Cube (i,j,k)`,
health 70. The non-hull slots:

| id | kind | health |
|---|---|---|
| `cargob_cube_i1_j1_k2` | Thruster | 70 |
| `cargob_cube_im1_j1_k2` | Thruster | 70 |
| `cargob_cube_i1_j1_km2` | Torpedo | 100 |
| `cargob_cube_im1_j1_km2` | Torpedo | 100 |
| `cargob_cube_i1_j2_k0` | Turret | 130 |
| `cargob_cube_im1_j2_k0` | Turret | 130 |
| `cargob_core_controller` | Controller | 100 |
| `cargoa_cube_i1_j1_k2` | Thruster | 70 |
| `cargoa_cube_im1_j1_k2` | Thruster | 70 |
| `cargoa_core_controller` | Controller | 100 |

The full hull-cube id sets (all `Hull`, health 70):

- `cargob_cube_` + `i0_j0_{k0,k1,k2,km1}`, `i0_j1_{k2,km1,km2}`,
  `i0_j2_{k0,k1,k2,km1,km2}`, `i1_j0_{k0,k1,k2,km1,km2}`,
  `i1_j1_{k0,k1,km1}`, `i1_j2_{k1,k2,km1,km2}`,
  `im1_j0_{k0,k1,k2,km1,km2}`, `im1_j1_{k0,k1,km1}`,
  `im1_j2_{k1,k2,km1,km2}`.
- `cargoa_cube_` + `i0_j0_{k0,k1,k2,km1}`, `i0_j1_{k1,k2,km1,km2}`,
  `i0_j2_{k0,k1,km1,km2}`, `i1_j0_{k0,k1,k2,km1}`,
  `i1_j1_{k0,k1,km1,km2}`, `i1_j2_{k0,k1,km1,km2}`, `i2_j0_{k0,k1,km1}`,
  `i2_j1_{k0,k1,k2,km1}`, `im1_j0_{k0,k1,k2,km1}`,
  `im1_j1_{k0,k1,km1,km2}`, `im1_j2_{k0,k1,km1,km2}`,
  `im2_j0_{k0,k1,km1}`, `im2_j1_{k0,k1,k2,km1}`.

(Read `i1_j0_{k0,k1}` as `i1_j0_k0`, `i1_j0_k1`. The braces are this page's
shorthand, not RON.)

## Scenario ids

What `NextScenario((scenario_id: ...))`, a `Campaign` member list, and the
Scenarios picker can target. A `hidden` scenario stays off the picker but is
launchable by id.

| id | display name | hidden | what it is |
|---|---|---|---|
| `shakedown_run` | Shakedown Run | no | the New Game tutorial (chapter 1) |
| `broadside` | Broadside | no | chapter 2 part 1: the corvette ambush |
| `broadside_gunship` | Broadside: Rust Tally | yes | chapter 2 part 2: the gunship boss |
| `lifeline` | Lifeline | no | chapter 3 part 1: convoy defense |
| `final_tally` | Final Tally | yes | chapter 3 finale: the anchorage |
| `asteroid_field` | Asteroid Field | no | the combat and gravity sandbox |
| `asteroid_next` | Asteroid Field - Next | yes | a relay that loops the sandbox |
| `menu_ambience` | Menu Ambience | yes | menu backdrop: planetoid + orbiting ship |
| `menu_waystation` | Waystation Traffic | yes | menu backdrop: hauler convoy |
| `menu_scrapyard` | Scrapyard Drift | yes | menu backdrop: drifting salvage yard |

One campaign ships: `nova_protocol` ("Nova Protocol"), members
`shakedown_run`, `broadside`, `broadside_gunship`, `lifeline`, `final_tally`
in play order. There are no other content kinds - a content file holds
`Section`, `Scenario` and `Campaign` items only; ships and factions are not
content items (ships are `Spaceship` scenario objects built from prototype
ids).

New Game is base-owned: `new_game_scenario: Some("shakedown_run")` in
`assets/base/base.bundle.ron` is honored only from the base bundle; a mod
declaring it is warned and ignored.

## Assets: what dep://base/ can reach

`dep://base/<path>` resolves against the base bundle's own folder
(`assets/base/<path>`), gated on `<path>` being in the base bundle's declared
`resources` list - an undeclared path is a lint/merge Error, not a silent
404. A `#Scene0`-style gltf label is stripped before the membership check,
so `dep://base/gltf/hull-01.glb#Scene0` is fine. The declared list, complete:

### Meshes (117 glb)

Named meshes:

- `gltf/hull-01.glb` - the core hull (use label `#Scene0`)
- `gltf/turret-yaw-01.glb`, `gltf/turret-pitch-01.glb`,
  `gltf/turret-barrel-01.glb` - the turret joint meshes
- `gltf/torpedo-bay-01.glb` - the torpedo bay

Cut hull cubes, named `cube_i<I>_j<J>_k<K>.glb` (`m` = minus, matching the
prototype ids above):

- `gltf/racer/` - 18 cubes: `i0_j0_{k0,k1,k2,km1,km2}`,
  `i0_j1_{k0,k1,k2,km1,km2}`, `i1_j0_{k0,k1,k2,km1}`,
  `im1_j0_{k0,k1,k2,km1}`.
- `gltf/cargob/` - 42 cubes: `i0_j0_{k0,k1,k2,km1}`, `i0_j1_{k2,km1,km2}`,
  `i0_j2_{k0,k1,k2,km1,km2}`, and all five `k0,k1,k2,km1,km2` for each of
  `i1_j0`, `i1_j1`, `i1_j2`, `im1_j0`, `im1_j1`, `im1_j2`.
- `gltf/cargoa/` - 52 cubes: `i0_j0_{k0,k1,k2,km1}`,
  `i0_j1_{k1,k2,km1,km2}`, `i0_j2_{k0,k1,km1,km2}`, `i1_j0_{k0,k1,k2,km1}`,
  `i1_j1_{k0,k1,k2,km1,km2}`, `i1_j2_{k0,k1,km1,km2}`, `i2_j0_{k0,k1,km1}`,
  `i2_j1_{k0,k1,k2,km1}`, `im1_j0_{k0,k1,k2,km1}`,
  `im1_j1_{k0,k1,k2,km1,km2}`, `im1_j2_{k0,k1,km1,km2}`,
  `im2_j0_{k0,k1,km1}`, `im2_j1_{k0,k1,k2,km1}`.

### Sounds (13 wav)

All under `dep://base/sounds/`: `dry_fire.wav`, `explosion.wav`,
`impact.wav`, `lock_off.wav`, `lock_on.wav`, `radar_deny.wav`,
`radar_retarget.wav`, `rcs_loop.wav`, `safety_on.wav`,
`salvage_pickup.wav`, `thruster_loop.wav`, `torpedo_launch.wav`,
`turret_fire.wav`.

(UI chrome sounds - menu clicks, objective chimes - are engine assets, not
base bundle resources, and are NOT dep-referenceable.)

### Images (10)

- `textures/cubemap.png` - the stock skybox (chapter 1 / sandbox)
- `textures/cubemap_alt.png` - the alternate skybox (chapters 2-3)
- `textures/asteroid.png` - the asteroid surface texture
- `thumbnails/asteroid_field.png`, `thumbnails/shakedown_run.png`,
  `thumbnails/broadside.png`, `thumbnails/broadside_gunship.png`,
  `thumbnails/lifeline.png`, `thumbnails/final_tally.png` - the picker plates
- `banner.png` - the base mod's banner

Skybox `.png.meta` sidecars (the cube reinterpret) ride along with their
image automatically and are never listed or referenced directly.

## The overlay rule

How a mod item interacts with this catalog (implemented in
`crates/nova_assets/src/merge.rs`):

- The matching key is the id string per kind - `Section` matches on
  `base.id`, `Scenario` and `Campaign` on `id`. Names and file paths never
  participate.
- Same id as base (or an earlier bundle) = REPLACE, whole item. It is not a
  field-level patch: an overlay must restate every field it wants to keep.
  Sections replace in place, so the editor palette order is preserved.
- New id = ADD alongside the catalog. Prefix your own ids with your mod's
  name (`example_*` in the shipped example mod) so they cannot collide.
- Same id twice within ONE bundle = a conflict: the first item is kept, the
  duplicate skipped and logged.
- Merge order is catalog order (base first), then downloaded mods, then a
  topological pass so a dependency merges before its dependents - so a mod
  overlays base and everything it depends on, and the last independent mod
  wins.

## See the source

The builders behind this page: `crates/nova_authoring/src/sections.rs` (core
sections), `crates/nova_authoring/src/scenario/craft.rs` (the cut-cube
prototypes), `crates/nova_authoring/src/scenario*.rs` (scenarios and the
campaign). If this page and the generated RON ever disagree, the RON is the
truth and this page has a bug - the `content_ron_parity` test pins the RON to
the builders.
