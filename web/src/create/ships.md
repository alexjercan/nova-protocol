# Ships for mods

A `Ship` is a whole DESIGN, authored once and spawned by id. It owns the
section layout, whether the hull is clad, how far it must be dismantled before
it collapses, and the voice it speaks to its pilot with. A
[Spaceship](../objects/#spaceship) spawn names it and adds everything that
differs per spawn: where it sits, who flies it, which side it is on, what it is
permitted to do.

Ships are content like sections, scenarios and styles. Create a new id to add a
design, or reuse a base id to rebuild every scenario's corvette at once.

This page is the field-by-field ship reference. For general RON spelling rules
such as double parentheses, `Some(...)`, and asset schemes, see
[RON spelling rules](../reference/#how-ron-content-is-written).

## Why a ship is content

A full loadout runs to hundreds of lines. Inlining one in every scenario that
spawns it means editing a corvette in eleven places and getting eleven slightly
different corvettes. A ship id is the fix: one definition, one edit, and a mod
can replace it for every scenario that names it.

## The Ship item

```ron
[
    Ship((
        id: "my_corvette",
        name: "My Corvette",
        design: (
            sections: [
                (
                    id: "fuselage",
                    position: (0.0, 0.0, 0.0),
                    rotation: (0.0, 0.0, 0.0, 1.0),
                    source: Prototype(id: "basic_controller_section"),
                ),
                // ... hull, thruster, and weapon sections ...
            ],
        ),
    )),
]
```

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | what a spawn names this design by. A mod reusing a base id REPLACES that design everywhere |
| `name` | string | required | the name a picker shows. Not used at spawn - a spawned ship is named by the scenario object that placed it |
| `design` | design | required | the design itself (below) |

### The design

| field | type | default | meaning |
|---|---|---|---|
| `sections` | list | `[]` | the hull/thruster/gun/controller layout: one entry per section, each with a ship-local `id`, a `position` in BUILD CELLS (one cell is 10 m) and a `rotation`, both relative to the ship root, and a `source` (`Prototype(id: "<section id>")`, with an optional `patch` over it, or `Inline((..))`) |
| `integrity` | integrity | all default | when the design comes apart (below) |
| `presentation` | presentation | all default | what it looks like and sounds like (below) |

`integrity` fields:

| field | type | default | meaning |
|---|---|---|---|
| `collapse_threshold` | `Option` number | `None` | structural collapse: the fraction of the health the ship was BUILT with below which whatever is left comes apart and the ship is destroyed. Strict RON `Some(0.1)`; omitted = the engine default `0.05`. Lower = the ship must be dismantled further (a capital), `Some(0.0)` = strip every last section. Clamped to `0..=1` |

`presentation` fields:

| field | type | default | meaning |
|---|---|---|---|
| `skin` | bool | `false` | clad the ship: the game DERIVES an outer skin from the sections at spawn - destructible plates, nothing authored, no id to reference (see [Cladding](../base-content/#cladding-not-a-prototype)). For hulls built out of the unit-cell sections; modelled parts are their own sizes and do not stand on that lattice |
| `style` | `Option` string | `None` | the LOOK the cladding wears, by [style](../styles/) id: plate materials plus the destructible decoration placed on them. Strict RON `Some("raider")`; omitted = built-in plate colours and no decoration. An id no loaded style authors fails at lint and stops the game at spawn |
| `collapse_sound` | `Option` asset ref | `None` | the sound the design makes when it COLLAPSES - the moment it stops being a ship and becomes wreckage, which is ONE event however many frames the sections then take to peel away. Strict RON `Some("dep://base/sounds/destroy_ship.wav")`. Authored on the design because a spine going is not a section failing loudly; omitted, the ship comes apart to the sound of its own sections |
| the cockpit voice | `Option` asset refs | `None` | `lock_on_sound`, `lock_off_sound`, `radar_deny_sound`, `radar_retarget_sound`, `safety_on_sound`, `warn_lock_sound`, `ammo_dry_sound`, `warn_hull_sound`, `kill_confirm_sound`, `torpedo_closing_sound`, `rcs_loop_sound` - the feedback the hull gives its pilot (`kill_confirm_sound` plays once per hull the player's torpedo blast defeated; `torpedo_closing_sound` ticks faster as the player's torpedo nears the locked hull, from 8 s out). Authored-or-silent: a sound nobody names plays nothing |
| `warn_hull_fraction` | number | `0.3` | the hull fraction the critical alarm fires at, once, on the way down. `0.0` never warns |

The voice is the DESIGN's, not its flight computer's: a hull that loses every
controller section still knows how to warn about its own structure.

Section entries are the same records a design carried when it was inlined - see
[section prototypes](../base-content/#section-prototypes) for the ids the base
game ships.

## Spawning one

```ron
SpawnScenarioObject((
    base: (
        id: "raider_1",
        name: "Raider",
        position: (0.0, 0.0, -3000.0),
        rotation: (0.0, 0.0, 0.0, 1.0),
    ),
    kind: Spaceship((
        controller: AI((engage_delay: Some(8.0))),
        design: Prototype(id: "my_corvette"),
        inventory: {},
        lootable: false,
        credits: 2000,
    )),
)),
```

## Changing one design for one spawn

A scenario that wants a harder flight computer or a fixed magazine does NOT need
its own ship. `section_patches` on the spawn aims the same typed deltas a
section reference carries at a section of the resolved design, and they are
applied after the design's own, so the spawn wins:

```ron
kind: Spaceship((
    controller: AI(()),
    design: Prototype(
        id: "my_corvette",
        section_patches: {
            "fuselage": (config: (health: Some(500.0))),
            "turret_port": (config: (kind: Some(Turret((
                ammunition: Some(Limited(900)),
                reload: Some(Disabled),
            ))))),
        },
    ),
    inventory: {},
    lootable: false,
    credits: 2000,
)),
```

A `section_patches` key the design does not carry does nothing at runtime, so
the content lint reports it as an error, and so does a patch that argues with
the section's kind. The full patch surface is in
[the sections list](../objects/#the-sections-list).

Reach for a second ship id instead when the difference is what the design IS -
the base game ships `light_freighter` and `freighter_derelict` as separate
hulls because a missing drive, transom and wing are a different ship to fly,
not a tweak. The Ledger does the same with its two hauler loads, `cargob`
and `cargob_lance`: which torpedo TYPE the pods carry decides whether a
defender's point defense can answer the salvo at all, and that is the ship, not
a tweak to it.

## One-off designs

A design nothing else will ever spawn can stay on the spawn:

```ron
kind: Spaceship((
    controller: None,
    design: Inline((
        sections: [
            (
                id: "bay",
                position: (0.0, 0.0, 0.0),
                rotation: (0.0, 0.0, 0.0, 1.0),
                source: Prototype(id: "torpedo_section"),
            ),
        ],
    )),
    inventory: {},
    lootable: false,
    credits: 2000,
)),
```

`Inline((..))` takes the same fields as a ship's `design`. Use it for a
scripted battery that is a single tube or a derelict that is five plates;
anything a second scenario would spawn belongs in the catalog. It takes no
`section_patches`: it is already this spawn's own.

## Overlay and lint

- A mod ship with a base id REPLACES the base design, so every scenario naming
  it flies the mod's - the same last-wins overlay sections and styles follow.
- The same id twice in ONE bundle is a conflict: the first wins and the
  duplicate is skipped.
- A design is linted where it is AUTHORED, not where it is spawned: its section
  prototypes must resolve, its own patches must apply, its sections must not
  interpenetrate, and its link-point graph must be connected. A spawn's
  `section_patches` are linted where the spawn is.
- A spawn naming a ship no bundle provides is a lint error, and at runtime it
  spawns an empty root and logs rather than crashing.

## Base ships

Every base design is BUILT: cells on a build grid, clad by the derived skin and
painted by a [style](../styles/). Nothing in the base game is a modelled mesh.

| id | what it is |
|---|---|
| `mining_tug` | Mining Tug - the industrial start, a pusher of light plate: a four-cell boom off the bow with a mining emitter at its tip (`mining_beam`), a three-wide body with a cargo intake on its fore deck (`cargo_intake`), a cab aft and three bell drives in a triangle |
| `salvage_hauler` | Salvage Hauler - the scavenger start: a forked bow of two prongs of light plate with a mining emitter in the crossbar between them (`mining_beam`), a work deck with a cargo intake on it, and a tall block aft with the bridge, one point-defense mount on its roof and five bell drives, one at each corner of the transom and one in the centre. The biggest hold of the start hulls |
| `light_freighter` | Light Freighter - the transport start: a one-cell neck of light plate with the cab on it, a three-wide box hold with a cargo intake on its roof, and a drive pod on each quarter. Five bell drives: two on each pod and one on the hold's transom. Unarmed |
| `freighter_derelict` | Light Freighter: Derelict - a yard-fitted freighter (reinforced plate, freight cubes, a tank pod, solar wings) after a raid took its stern: no drive, no transom, the aft tank cube and the starboard wing's outer panel gone. It is an older pattern than the `light_freighter`, a flat spine with a stack on it and a pod on the starboard flank, and its port collar stands where the freighter's does |
| `interceptor` | Interceptor - the bounty hunter start: a one-deck arrowhead of light plate that steps out from a one-cell tip to the wing tips, with a point-defense mount on each tip (`pdc_port`, `pdc_starboard`), a short bridge block and a cargo intake on the deck, and five bell drives: one on each wing tip, a pair on the tail and one raised behind the intake |
| `raider` | Raider - the pirate start: a hammerhead of light plate, a wide two-deck head forward with a point-defense mount on each end (`pdc_port`, `pdc_starboard`) and a cargo intake between them, a neck one cell wide, and four bell drives: one on the tail, two stacked on its port side and one on an outrigger pod to starboard (`drive_outrigger`) |
| `patrol_picket` | Patrol Picket - the military start: a low bow of light plate that widens aft, with a point-defense mount on the nose face, a torpedo bay under the nose (`torpedo_keel`), a cargo intake on the fore deck, a bridge tower aft of midships with the second mount on its roof, a docking collar off each flank, hatch outboard (`port_collar`, `starboard_collar`), a short strake behind each collar, and three bell drives in a triangle |
| `training_picket` | Training Picket - the tutorial range's hull, flown by the trainer and the drones: a slim dart of light plate with one point-defense mount on the nose face and two bell drives. Scenario-only, with no cargo intake |
| `line_warship` | Line Warship - the military goal: a long armoured body with six PDC mounts, a railgun on the bow, a torpedo bay on each shoulder, a cargo intake on the aft deck (`cargo_intake`) and a docking collar off each shoulder, hatch outboard (`port_collar`, `starboard_collar`) |
| `debris_plate` | Debris Plate - loose plating: the small pieces, and most of what a debris field is |

Every crewed base hull carries a docking collar on the port flank, `port_collar`.

A mod may ship MODELLED craft instead, and The Ledger does: its `racer`,
`cargoa`, `cargoa_raider`, `cargob` and `cargob_lance` are assembled from GLB
parts the mod carries under its own `gltf/parts/`.
