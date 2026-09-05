# Base content catalog

Every id and asset the base game ships, so your mod can reference them
instead of guessing. Five registries matter to a mod author: **section
prototype ids** (what `source: Prototype("<id>")` can name), **ship ids**
(what `hull: Prototype("<id>")` can name - tabled in
[Ships](../ships/#base-ships)), **style ids** (what a hull's `style:` can
name), **scenario ids** (what `NextScenario` and campaigns can target), and
**base asset paths** (what `dep://base/<path>` can reach). All of it is generated from the Rust builders
by `cargo run content gen` into
`assets/base/**/*.content.ron` - the generated RON is the shipped truth, and
`content -- lint` checks your references against it.

`base` is the IMPLICIT universal dependency: every mod may reference base
content and `dep://base/` assets without listing `base` in
`meta.dependencies`. Declaring it explicitly is allowed and also resolves -
the shipped portal mods do - but it is never required.

## Section prototypes

Lengths are in METERS and speeds in meters per second: the number in the file is
the number in the world, and there is nothing to convert. World units survive
in one place a creator ever touches - the BUILD GRID a ship is assembled on,
whose cell is 10 m on a side. A section's `collider`, its `link_points` and its
`position` in a ship's `sections` list are counted in cells, and so is a
thruster's exhaust cone; everything else on this page is metric.

A prototype id is used two ways: a ship's section list references it
compactly (`source: Prototype("<id>")` resolves the whole config, meshes and
sounds included), and a mod `Section` item that reuses the id REPLACES that
part everywhere. See [Ship sections for mods](../sections/) for
the `Section` grammar and the overlay flow.

Ids are lowercase snake_case, in the `<variant>_<kind>_section` form. Every
base prototype is GENERIC - a hull cell, a drive, a mount, a bay - so the same
part serves every hull. Section kinds are `Hull`, `Thruster`, `Controller`,
`Turret`, `Torpedo`, and `Railgun`.

### Core sections (the editor palette)

| id | kind | display name | health | notes |
|---|---|---|---|---|
| `reinforced_hull_section` | Hull | Reinforced Hull Section | 200 | the armor workhorse |
| `basic_thruster_section` | Thruster | Basic Thruster Section | 70 | magnitude 1.0; authored 1x1 shell bell; one socket on the `-Z` mounting face; cone plume starts at `Z = 0.51` with radius 0.24 and exits `+Z` |
| `vector_thruster_section` | Thruster | Vector Thruster Section | 480 | 3x3x2 collider and footprint; magnitude 9; nine `-Z` mounting sockets; one vectoring bell and plume |
| `capital_thruster_section` | Thruster | Capital Thruster Section | 1250 | 5x5x3 collider and footprint; magnitude 25; 25 `-Z` mounting sockets; one capital bell and plume |
| `basic_controller_section` | Controller | Basic Controller Section | 100 | steering lag 0.5 s, max torque 1501.0; the hull derives its own turn ceiling from that torque, its inertia and its length |
| `light_hull_section` | Hull | Light Hull Section | 60 | scavenger-grade hull |
| `cargo_hull_section` | Hull | Cargo Hull Section | 200 | the reinforced cell's stats under caged freight; a visual choice until real hull types arrive |
| `tank_hull_section` | Hull | Tank Hull Section | 200 | the reinforced cell's stats around a pressure vessel in open frame rails |
| `pdc_kinetic_turret_section` | Turret | PDC Turret (Kinetic) | 130 | the gatling every craft mounts, on a 0.5 mount box; fits any hull face. Kinetic 4.0/hit at 100 rps, ammo 500, +200 after 3 s idle |
| `pdc_pierce_turret_section` | Turret | PDC Turret (Pierce) | 130 | the same 500-round, +200 after 3 s idle gatling loading penetrators: Pierce 2.0/hit, dealt to every section it rakes through |
| `pdc_twin_kinetic_turret_section` | Turret | Twin PDC Turret (Kinetic) | 130 | the same slugs from the two-barrel mount: each tube at half the gatling's cadence, so two offset streams at the same total rate and magazine drain |
| `pdc_twin_pierce_turret_section` | Turret | Twin PDC Turret (Pierce) | 130 | penetrators from the two-barrel mount - half per-hit damage through every layer, split across two offset streams |
| `torpedo_section` | Torpedo | Torpedo Bay (Serpent) | 100 | blast 750 dmg over a radius of 300 m, ordnance 10 hp, ammo 6 restoring +1 after 10 s idle; loads the WEAVING Serpent - cruise 320 m/s, ~390 PDC rounds an intercept, killed ~400 m out |
| `lance_torpedo_section` | Torpedo | Torpedo Bay (Lance) | 100 | the same six-round, +1 after 10 s idle bay and warhead loading the straight-running Lance: no weave, cruise 350 m/s, ~116 PDC rounds an intercept, killed ~1.14 km out |
| `railgun_lance_section` | Railgun | Railgun Lance | 180 | the spinal lance: no traverse, so the HULL aims it. A 1.5 s charge you can see walk the bore, then Pierce 300 to every layer it rakes until 1800 slug power runs out; a slug at 15,000 m/s for 1.2 s, rake radius 10 m, recoil 45 at the muzzle, one round reloading over 12 s |
| `siege_railgun_lance_section` | Railgun | Siege Railgun Lance | 180 | the same lance at capital grade, mounted only by the campaign's stolen warship: Pierce 500 to every layer it rakes until 360,000 slug power runs out, rake radius 30 m. Same 1.5 s charge, same 15,000 m/s slug for 1.2 s, same recoil 45 and one round over 12 s. Deliberately overpowered siege ordnance, not a balanced duel |
| `heavy_torpedo_section` | Torpedo | Siege Torpedo Bay Section | 100 | experimental and deliberately overpowered balancing kit, not intended for normal play; blast 2000 dmg over a radius of 450 m, armored ordnance (5000 hp), ammo 6 restoring +1 after 10 s idle; loads the crimson siege Breaker (cruise 700 m/s, with a shallow weave) |

Every shipped prototype authors its [damage
effects](../sections/#damage-effects) by kind, and the whole catalog follows one
rule: Hull wears `([Cracks])` (the default, so it is omitted), Controller,
Turret, Torpedo and Railgun wear `([Cracks, Sparks])`, and Thruster wears
`([Cracks, Sparks, Plume])`.
No shipped mod authors the field at all, so base is the worked example.

### Cladding (not a prototype)

A ship's outer SKIN is DERIVED from the structure it wraps, as a pure function
of it: nothing places a plate, no id names one, and none is saved. Build the
structure and the skin follows. There is nothing here for a mod to reference.

A ship asks for it with one field - `skin: true` on the
[ship](../ships/)'s hull - and gets destructible cladding: each plate
carries its own health and mass, comes off when it is shot out, and leaves the
hull behind it bare. Build the hull out of the unit-cell sections above;
modelled parts a mod brings are shapes of their own sizes and are not on the
lattice the derivation reads.

What the cladding LOOKS like is a [style](../styles/), which IS content and does
have an id.

### Skin styles

| id | what it is |
|---|---|
| `industrial` | a working hull: exposed services, corrugation, radiators, safety-yellow paint on its edges |
| `armoured` | flat plate, a belt down every straight edge, sensor blisters |
| `civilian` | a private yacht's: pale satin paint, a cobalt livery rail, lit cabin windows |
| `salvage` | the raider's: mismatched patches, weld beads, a lashed drum, a whip antenna |
| `placeholder` | scaffolding, in deliberately garish magenta: four placeholder greebles wired to four rules that exercise the whole plate vocabulary. It makes no art decision |

A ship names one with `style: Some("<id>")` beside `skin: true` on its
[hull](../ships/#the-hull). A mod declaring
a `Style` with the same id replaces that look everywhere; a new id is a new look.
See [Ship skin styles](../styles/).

### Modelled ship parts (not base content)

Base ships NO parts cut for one craft. Every base hull is built out of the
generic prototypes above, on the build grid, and clad by the derived skin.

A mod that brings modelled craft brings their part prototypes and their meshes
with it. [The Ledger](../publish-a-mod/) is the worked example: its `racer`,
`cargoa`, `cargoa_raider`, `cargob` and `cargob_lance` are assembled from
`racer_*`, `cargoa_*` and `cargob_*` prototypes it declares itself, over GLBs it
carries under its own `gltf/parts/`. Those ids resolve only where The Ledger is
installed; nothing in base references them.

Turret MOUNTS are not prototypes either, in base or in a mod. A hull names where
a gun goes; the gun is `pdc_kinetic_turret_section`, seated on the face it stands
on, and a scavenger-grade craft flies the SAME gun with a `SetHealth(60.0)`
modification on the mount.

## Impact rows

The rows of the [impact table](../impacts/), re-declarable by id to re-voice a
hit for the whole game: `impact_kinetic`, `impact_pierce` and
`impact_explosive` are the three damage-type defaults, and
`impact_kinetic_rock`, `impact_kinetic_metal`, `impact_kinetic_ice`,
`impact_kinetic_carbon` and `impact_kinetic_plain` are the stone rows, one per
asteroid kind.

The materials the base game names are `"hull"` (every section) and the five
asteroid kinds, which are the same ids an asteroid's `material` field takes.

## Scenario ids

What `NextScenario((scenario_id: ...))`, a `Campaign` member list, and the
Scenarios picker can target. A `hidden` scenario stays off the picker but is
launchable by id.

| id | display name | hidden | what it is |
|---|---|---|---|
| `first_shift` | First Shift | no | the New Game opening: a salvage shift, and what ends it |
| `second_shift` | Second Shift | no | chapter 2: search the wreck, leave before the cleanup group finds you |
| `menu_waystation` | Waystation Traffic | yes | menu backdrop: hauler convoy (carousel: hands off to the gauntlet) |
| `menu_gauntlet` | Torpedo Gauntlet | yes | menu backdrop: a doomed point-defense stand (hands off to the weave) |
| `menu_weave` | Asteroid Weave | yes | menu backdrop: waypoint run through a dense rock band (hands off to the duel) |
| `menu_duel` | Duel Cycle | yes | menu backdrop: a duel ended by a siege torpedo (hands off to the waystation) |

One campaign ships: `nova_protocol` ("Nova Protocol"), members
`first_shift`, `second_shift` in play order. There are no other content kinds - a content file holds
`Section`, `Scenario`, `Campaign`, `Ship`, and `Style` items only; factions
are not content. The base ship ids are tabled in
[Ships](../ships/#base-ships), the style ids [above](#skin-styles).

New Game is base-owned: `new_game_scenario: Some("first_shift")` in
`assets/base/base.bundle.ron` is honored only from the base bundle; a mod
declaring it is warned and ignored.

## Assets: what dep://base/ can reach

`dep://base/<path>` resolves against the base bundle's own folder
(`assets/base/<path>`), gated on `<path>` being in the base bundle's declared
`resources` list - an undeclared path is a lint/merge Error, not a silent
404. A `#Scene0`-style gltf label is stripped before the membership check,
so `dep://base/gltf/hull-01.glb#Scene0` is fine.

The editor reads this same list. A row holding an image, a sound or a model
opens a picker of the files the installed bundles declare, writes the `dep://`
ref for the one you pick (with the `#Scene0` label where a mesh needs it), and
marks a `dep://` path no bundle ships - so you never type one of these paths
from memory.

The declared list, complete:

### Meshes (41 glb)

Named meshes (all use label `#Scene0`). First the thruster bells:

- `gltf/shell_bell.glb`, `gltf/shell_vector.glb`, `gltf/shell_capital.glb` -
  the three thruster bells

Then the generated section parts the catalog renders
(`scripts/gen-section-parts.py`, committed under `gltf/`):

- `gltf/hull_personnel.glb`, `gltf/hull_cargo.glb`, `gltf/hull_tank.glb` -
  the three hull cell looks (reinforced, cargo, tank)
- `gltf/core_wires.glb` - the controller core
- `gltf/pdc_gatling_yaw.glb`, `gltf/pdc_gatling_pitch.glb`,
  `gltf/pdc_gatling_barrel.glb` - the gatling PDC joint meshes
- `gltf/pdc_twin_yaw.glb`, `gltf/pdc_twin_pitch.glb`,
  `gltf/pdc_twin_barrel.glb` - the twin PDC joint meshes
- `gltf/pdc_housing.glb` - the stow housing both PDC mounts sink into,
  with its animatable `stow_lid_*` nodes
- `gltf/bay_tube.glb` - the two-cell torpedo bay, with its animatable
  `door_petal_*` iris nodes

The retired first-pass art stays in the bundle so old refs resolve, but no
base section renders it: `gltf/hull-01.glb`, `gltf/turret-yaw-01.glb`,
`gltf/turret-pitch-01.glb`, `gltf/turret-barrel-01.glb`,
`gltf/torpedo-bay-01.glb`.

Base ships no per-craft part meshes. A mod that brings modelled craft carries
them itself, under its own `gltf/parts/`.

Cladding ships no meshes and never will: a ship's skin is derived from its
structure and built at run time - see [Cladding](#cladding-not-a-prototype).

### Greebles (54 glb)

The decoration models the base [styles](../styles/) scatter, under
`dep://base/gltf/greebles/`, named `<kit>_<piece>.glb`: the four authored
kits - `armoured_*` (10), `civilian_*` (12), `industrial_*` (14),
`salvage_*` (14) - plus the four garish magenta `placeholder_*` pieces that
prove the pipeline.
All are generated from committed JSON recipes (`scripts/gen-greebles.py`,
`scripts/greeble-recipes/`), and a mod can ship its own `.glb` the same way.

### Sounds (31 wav)

All under `dep://base/sounds/`: `ammo_dry.wav`, `bay_door.wav`,
`destroy_rock.wav`, `destroy_ship.wav`, `dry_fire.wav`, `explosion.wav`,
`impact.wav`, `impact_explosive.wav`, `impact_pierce.wav`, `impact_rock.wav`,
`lock_off.wav`, `lock_on.wav`, `pdc_stow_close.wav`, `pdc_stow_open.wav`,
`pdc_twin_fire.wav`, `radar_deny.wav`, `radar_retarget.wav`,
`railgun_charge.wav`, `railgun_fire.wav`, `railgun_reload.wav`, `rcs_loop.wav`,
`safety_on.wav`, `salvage_pickup.wav`, `thruster_capital_loop.wav`,
`thruster_loop.wav`, `thruster_vector_loop.wav`, `torpedo_detonate.wav`,
`torpedo_launch.wav`, `turret_fire.wav`, `warn_hull.wav`, `warn_lock.wav`.

(UI chrome sounds - menu clicks, objective chimes - are engine assets, not
base bundle resources, and are NOT dep-referenceable.)

### Images (6)

- `textures/cubemap.png` - the stock skybox, and the sky both campaign chapters
  are flown under
- `textures/cubemap_alt.png` - the alternate skybox: a `SetSkybox` swap target
  and what the editor sandbox loads
- `textures/asteroid.png` - the asteroid surface texture
- `thumbnails/first_shift.png`, `thumbnails/second_shift.png` - the picker plates
- `banner.png` - the base mod's banner

Skybox `.png.meta` sidecars (the cube reinterpret) ride along with their
image automatically and are never listed or referenced directly.

## The overlay rule

How a mod item interacts with this catalog (implemented in
`crates/nova_assets/src/merge.rs`):

- The matching key is the id string per kind - `Section` matches on
  `base.id`; `Scenario`, `Campaign`, `Ship`, and `Style` on `id`. Names and
  file paths never participate.
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
section prototypes, `sections/ordnance.rs` the torpedo types, `styles.rs` the
skin styles, `ships/` owns the block hulls,
`scenarios/` groups mainline and main-menu scenarios, and
`campaigns.rs` owns campaign membership. If this page and the generated RON
ever disagree, the RON is the
truth and this page has a bug - the `content_ron_parity` test pins the RON to
the builders.
