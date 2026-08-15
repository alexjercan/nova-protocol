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

Ids are lowercase snake_case. Core editor parts use the
`<variant>_<kind>_section` form. Shipped semantic ship parts use
`<ship>_<part>`, for example `racer_fuselage` and `cargoa_engine_port`.
Section kinds are `Hull`, `Thruster`, `Controller`, `Turret`, and `Torpedo`.

### Core sections (the editor palette)

| id | kind | display name | health | notes |
|---|---|---|---|---|
| `reinforced_hull_section` | Hull | Reinforced Hull Section | 200 | the armor workhorse |
| `basic_thruster_section` | Thruster | Basic Thruster Section | 70 | magnitude 1.0 |
| `basic_controller_section` | Controller | Basic Controller Section | 100 | freq 4.0, damping 4.0, max torque 40 |
| `better_turret_section` | Turret | Better Turret Section | 130 | Kinetic 4.0/hit at 100 rps, ammo 500 |
| `light_hull_section` | Hull | Light Hull Section | 60 | scavenger-grade hull |
| `light_turret_section` | Turret | Light Turret Section | 60 | Kinetic 3.825/hit at 25 rps, ammo 150 |
| `pdc_kinetic_turret_section` | Turret | PDC Turret (Kinetic) | 130 | the better turret's gun on a 0.5 mount box; fits any hull face. Kinetic 4.0/hit at 100 rps, ammo 500 |
| `pdc_pierce_turret_section` | Turret | PDC Turret (Pierce) | 130 | the same gun loading penetrators: Pierce 2.0/hit, dealt to every section it rakes through |
| `torpedo_section` | Torpedo | Torpedo Bay Section | 100 | blast 750 dmg / 30 u, ordnance 10 hp, ammo 6 |
| `heavy_torpedo_section` | Torpedo | Siege Torpedo Bay Section | 100 | blast 2000 dmg / 45 u, armored ordnance (5000 hp), unlimited ammo; scene dressing, hidden in the editor |

### Semantic ship parts

These prototypes are in the editor palette, and mods can reference them. Their
tight primitive colliders and authored link points are part of each prototype -
and those link points are what places one: a part attaches only where its own
sockets meet another's. Ships use the suffix as the instance id: prototype
`racer_engine_port` normally becomes instance `engine_port`.

The `turret_*` suffixes are the exception: they carry no mesh of their own, so
all ten of them are the same PDC on the same joint tree. They stay in the
catalog for ships and mods, but the editor hides them and offers the two
`pdc_*_turret_section` mounts instead - one gun that fits any hull face, in a
Kinetic and a Pierce loadout.

| family | prototype suffix | kind | health |
|---|---|---|---|
| Racer | `engine_port`, `engine_starboard` | Thruster | 70 |
| Racer | `wing_port`, `wing_starboard` | Hull | 180 |
| Racer | `nose`, `tail` | Hull | 120 |
| Racer | `fuselage` | Controller | 240 |
| Racer | `turret_port`, `turret_starboard` | Turret | 130 |
| Racer | `turret_port_light`, `turret_starboard_light` | Turret | 60 |
| CargoB | `engine_port`, `engine_starboard` | Thruster | 70 |
| CargoB | `pod_port`, `pod_starboard` | Torpedo | 350 |
| CargoB | `nose` | Hull | 180 |
| CargoB | `tail` | Hull | 150 |
| CargoB | `fuselage` | Controller | 300 |
| CargoB | `turret_port`, `turret_starboard` | Turret | 130 |
| CargoA | `engine_port`, `engine_starboard` | Thruster | 70 |
| CargoA | `pod_port`, `pod_starboard` | Hull | 350 |
| CargoA | `nose` | Hull | 180 |
| CargoA | `tail` | Hull | 150 |
| CargoA | `fuselage` | Controller | 350 |
| CargoA | `turret_port`, `turret_starboard` | Turret | 130 |
| CargoA | `turret_port_light`, `turret_starboard_light` | Turret | 60 |

Prefix each suffix with `racer_`, `cargob_`, or `cargoa_`. Input mappings use
the instance id, such as `"turret_port"` or `"pod_starboard"`. The old
coordinate-named cube prototypes do not exist.

The shipped assemblies cast the hulls by role: the cargoa is the campaign's
armed corvette (turrets on the pod shoulders), the cargob its torpedo-and-PDC
gunship, and the racer an unarmed civilian (the yacht the story protects).
The `racer_turret_*` prototypes stay in the catalog for mods that arm the
racer themselves (the ledger campaign does), but no shipped racer mounts
them.

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
| `menu_waystation` | Waystation Traffic | yes | menu backdrop: hauler convoy (carousel: hands off to the gauntlet) |
| `menu_gauntlet` | Torpedo Gauntlet | yes | menu backdrop: a doomed point-defense stand (hands off to the weave) |
| `menu_weave` | Asteroid Weave | yes | menu backdrop: waypoint run through a dense rock band (hands off to the duel) |
| `menu_duel` | Duel Cycle | yes | menu backdrop: a duel ended by a siege torpedo (hands off to the waystation) |

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

### Meshes (26 glb)

Named meshes:

- `gltf/hull-01.glb` - the core hull (use label `#Scene0`)
- `gltf/turret-yaw-01.glb`, `gltf/turret-pitch-01.glb`,
  `gltf/turret-barrel-01.glb` - the turret joint meshes
- `gltf/torpedo-bay-01.glb` - the torpedo bay

Semantic ship meshes use `#Scene0` and live under `gltf/parts/`:

- `gltf/parts/racer/` - `engine_port`, `engine_starboard`, `wing_port`,
  `wing_starboard`, `nose`, `tail`, and `fuselage`.
- `gltf/parts/cargob/` - `engine_port`, `engine_starboard`, `pod_port`,
  `pod_starboard`, `nose`, `tail`, and `fuselage`.
- `gltf/parts/cargoa/` - the same seven CargoB mesh names.

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

The builders behind this page live under
`crates/nova_authoring/src/base_content/`: `sections/standard.rs` owns generic
section prototypes, `ships/` owns semantic parts and complete craft,
`scenarios/` groups mainline, main-menu, and sandbox scenarios, and
`campaigns.rs` owns campaign membership. If this page and the generated RON
ever disagree, the RON is the
truth and this page has a bug - the `content_ron_parity` test pins the RON to
the builders.
