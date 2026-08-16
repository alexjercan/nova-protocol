# Ships for mods

A `Ship` is a whole HULL, authored once and spawned by id. It owns the section
layout, whether the hull is clad, and how far it must be dismantled before it
collapses. A [Spaceship](../objects/#spaceship) spawn names it and adds
everything that differs per spawn: where it sits, who flies it, which side it is
on.

Ships are content like sections, scenarios and styles. Create a new id to add a
hull, or reuse a base id to rebuild every scenario's corvette at once.

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
        hull: (
            sections: [
                (
                    id: "fuselage",
                    position: (0.0, 0.0, 0.0),
                    rotation: (0.0, 0.0, 0.0, 1.0),
                    source: Prototype("cargoa_fuselage"),
                ),
                // ... hull, thruster, and weapon sections ...
            ],
        ),
    )),
]
```

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | what a spawn names this hull by. A mod reusing a base id REPLACES that hull everywhere |
| `name` | string | required | the name a picker shows. Not used at spawn - a spawned ship is named by the scenario object that placed it |
| `hull` | hull | required | the hull itself (below) |

### The hull

| field | type | default | meaning |
|---|---|---|---|
| `sections` | list | `[]` | the hull/thruster/gun/controller layout: one entry per section, each with a ship-local `id`, a `position` and `rotation` relative to the ship root, a `source` (`Prototype("<section id>")` or `Inline((..))`), and optional `modifications` |
| `collapse_threshold` | `Option` number | `None` | structural collapse: the fraction of the health the ship was BUILT with below which whatever is left comes apart and the ship is destroyed. Strict RON `Some(0.1)`; omitted = the engine default `0.25`. Lower = the ship must be dismantled further (a capital), `Some(0.0)` = strip every last section. Clamped to `0..=1` |
| `skin` | bool | `false` | clad the ship: the game DERIVES an outer skin from the sections at spawn - destructible plates, nothing authored, no id to reference (see [Cladding](../base-content/#cladding-not-a-prototype)). For hulls built out of the unit-cell sections; the modelled semantic parts do not stand on that lattice |
| `style` | `Option` string | `None` | the LOOK the cladding wears, by [style](../styles/) id: plate materials plus the destructible decoration scattered over them. Strict RON `Some("raider")`; omitted = built-in plate colours and no decoration. An id nothing authored leaves the ship clad and bare rather than falling back to another look |

Section entries are the same records a hull carried when it was inlined - see
[section prototypes](../base-content/#section-prototypes) for the ids the base
game ships.

## Spawning one

```ron
SpawnScenarioObject((
    base: (
        id: "raider_1",
        name: "Raider",
        position: (0.0, 0.0, -300.0),
        rotation: (0.0, 0.0, 0.0, 1.0),
    ),
    kind: Spaceship((
        controller: AI((engage_delay: Some(8.0))),
        hull: Prototype("my_corvette"),
    )),
)),
```

## Changing one hull for one spawn

A scenario that wants a harder flight computer or a fixed magazine does NOT need
its own ship. `modifications` on the spawn aims the same data-only deltas a
section carries at a section of the resolved hull, and they are applied after
the hull's own, so the spawn wins:

```ron
kind: Spaceship((
    controller: AI(()),
    hull: Prototype("my_corvette"),
    modifications: [
        (section: "fuselage", modifications: [SetHealth(500.0)]),
        (section: "turret_port", modifications: [SetAmmo(900)]),
    ],
)),
```

A `section` id the hull does not carry does nothing at runtime, so the content
lint reports it as an error.

Reach for a second ship id instead when the difference is what the hull IS - the
base game ships `cargoa` and `cargoa_raider` as separate hulls because thinner
plating and scavenger-grade guns are a different ship to fight, not a tweak.

## One-off hulls

A hull nothing else will ever spawn can stay on the spawn:

```ron
kind: Spaceship((
    controller: None,
    hull: Inline((
        sections: [
            (
                id: "bay",
                position: (0.0, 0.0, 0.0),
                rotation: (0.0, 0.0, 0.0, 1.0),
                source: Prototype("torpedo_section"),
            ),
        ],
    )),
)),
```

`Inline((..))` takes the same fields as a ship's `hull`. Use it for a scripted
battery that is a single tube or a derelict that is five plates; anything a
second scenario would spawn belongs in the catalog.

## Overlay and lint

- A mod ship with a base id REPLACES the base hull, so every scenario naming it
  flies the mod's - the same last-wins overlay sections and styles follow.
- The same id twice in ONE bundle is a conflict: the first wins and the
  duplicate is skipped.
- A hull is linted where it is AUTHORED, not where it is spawned: its section
  prototypes must resolve, its sections must not interpenetrate, and its
  link-point graph must be connected.
- A spawn naming a ship no bundle provides is a lint error, and at runtime it
  spawns an empty root and logs rather than crashing.

## Base ships

| id | what it is |
|---|---|
| `racer` | the unarmed Racer yacht - fast, expensive, the civilian hull the campaign protects |
| `cargob` | the CargoB hauler - two torpedo pods, two PDC mounts, the capital silhouette |
| `cargoa` | the CargoA corvette at player grade - the armed hauler the player flies |
| `cargoa_raider` | the same corvette at scavenger grade: thinner plating, light turrets, a softer flight computer |
