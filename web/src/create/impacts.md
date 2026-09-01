# The impact table for mods

What a hit SOUNDS like has two halves: the round that arrived, and the thing it
arrived on. A slug on ship plate and the same slug on stone are different
noises, and so are a slug and a penetrator on the same plate.

An `Impact` item is one row of that pairing: a damage type, a material, and the
sound. Sections and asteroids name only what they are MADE of; the table does
the rest.

This page is the field-by-field impact reference. For general RON spelling rules
such as double parentheses, `Some(...)`, and asset schemes, see
[RON spelling rules](../reference/#how-ron-content-is-written).

## The Impact item

```ron
[
    Impact((
        id: "my_mod_pierce_ceramic",
        damage: Pierce,
        material: Some("my_mod_ceramic"),
        sound: "self://sounds/pierce_ceramic.wav",
    )),
    Impact((
        id: "my_mod_kinetic_ceramic",
        damage: Kinetic,
        material: Some("my_mod_ceramic"),
        sound: "self://sounds/kinetic_ceramic.wav",
    )),
]
```

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | Stable row key, and the overlay key. Re-declare a base id to replace that row; a new id adds one. Prefix new ids with your mod id. |
| `damage` | damage type | required | `Kinetic` (the punch), `Pierce` (the rake), or `Explosive` (a warhead's pressure). |
| `material` | `Option` string | `None` | The material this row is for. `None` makes it the DEFAULT row for its damage type. |
| `sound` | asset ref | required | The voice. `self://sounds/...` from your bundle, or `dep://base/sounds/...`. |

## How a hit finds its row

1. The round's damage type and the struck body's `material` are looked up as a
   pair.
2. If no row names that pair, the damage type's default row (`material: None`)
   is used.
3. If there is no default row either, the hit is SILENT.

That is the whole rule, and it falls back exactly once. A material with no
`Pierce` row does not borrow its own `Kinetic` row - it takes the `Pierce`
default, because the round is what the player is listening to.

## Naming a material

Materials are open strings. A thing is made of one by saying so:

- a [section](../sections/) sets `base.material`; omitted means `"hull"`.
- an [asteroid](../objects/#asteroid) sets `material`; omitted means `"rock"`.

Adding a material to the game is naming it on your objects and authoring the
rows to hear it. Nothing else changes.

## Base rows

The base game ships four rows - three defaults and one material:

| id | damage | material | sound |
|---|---|---|---|
| `impact_kinetic` | `Kinetic` | (default) | `dep://base/sounds/impact.wav` |
| `impact_kinetic_rock` | `Kinetic` | `"rock"` | `dep://base/sounds/impact_rock.wav` |
| `impact_pierce` | `Pierce` | (default) | `dep://base/sounds/impact_pierce.wav` |
| `impact_explosive` | `Explosive` | (default) | `dep://base/sounds/impact_explosive.wav` |

Re-declaring one of those four ids re-voices it for the whole game, base content
included. Everything the table does not name falls to its damage type's default,
so a mod that adds ten materials and no rows is still audible - it just sounds
like ship plate.

## Destruction is not here

A hit is the round meeting a surface. DESTRUCTION is one event with one sound
whatever caused it, so it stays a per-target field: `base.destroy_sound` on a
[section](../sections/), `destroy_sound` on an
[asteroid](../objects/#asteroid). Omitted means silent.
