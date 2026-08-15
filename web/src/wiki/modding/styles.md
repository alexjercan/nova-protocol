# Ship skin styles for mods

A `Style` is the LOOK a ship's derived cladding wears: what its plates are made
of, and the decoration scattered over them. A ship names one by id
(`style: Some("raider")` on a [Spaceship](../objects/#spaceship)) and wears it.

Styles are content like sections and scenarios. Create a new id to add a look,
or reuse a base id to restyle every ship that already names it. A style ships
its own decoration models the same way a section ships its render mesh - as
`.glb` files in your bundle, referenced with `self://`.

This page is the field-by-field style reference. For general RON spelling rules
such as double parentheses, `Some(...)`, and asset schemes, see
[RON spelling rules](../reference/#how-ron-content-is-written).

## What a style can and cannot change

A ship's SKIN is derived from the structure it wraps - see
[Cladding](../base-content/#cladding-not-a-prototype). The SHAPE of every plate
is a function of the hull, so a style cannot author one. What it authors is:

- the MATERIAL of each plate surface, and
- the DECORATION bolted on top - vents, ribbing, blisters, masts.

Decoration is destructible in the full sense: each piece carries its own health,
mass and collider, stops rounds, and comes off when it is shot out, leaving the
plate behind it bare. Nothing decorative does anything else - if losing a thing
should cost the ship an ability, that thing is a [section](../sections/), not
decoration.

## The Style item

```ron
[
    Style((
        id: "my_raider_look",
        name: "Raider",
        surfaces: [
            (surface: Top,  color: Srgba((red: 0.4, green: 0.3, blue: 0.25, alpha: 1.0)), roughness: 0.8, metallic: 0.1),
            (surface: Wall, color: Srgba((red: 0.2, green: 0.15, blue: 0.12, alpha: 1.0)), roughness: 0.9, metallic: 0.1),
        ],
        fixtures: [
            (
                id: "antenna",
                model: "self://gltf/greebles/antenna.glb#Scene0",
                health: 8.0,
                density: 0.05,
                collider: (0.12, 0.38, 0.12),
                scatter: (
                    relief: [Ridge, Peak],
                    facing: Up,
                    min_depth: 2,
                    chance: 0.3,
                ),
            ),
        ],
    )),
]
```

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | what a ship names this style by. A base id REPLACES that look |
| `name` | string | required | display name |
| `surfaces` | list | `[]` | one entry per plate surface to dress. A surface left out keeps the built-in colour |
| `fixtures` | list | `[]` | the decoration, in PRIORITY order - see [Priority](#priority-one-piece-per-plate) |

### `surfaces`

| field | type | meaning |
|---|---|---|
| `surface` | `Top` / `Wall` / `Floor` | which face of a plate. `Top` faces space; `Wall` is the side a plate drops away at; `Floor` is against the hull and never seen |
| `color` | color | base colour, tagged: `Srgba((red: .., green: .., blue: .., alpha: ..))` |
| `roughness` | float | 0 (mirror) to 1 (matte) |
| `metallic` | float | 0 (dielectric) to 1 (metal) |

### `fixtures`

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | names the piece within its style, and SALTS its scatter - two pieces sharing one rule do not claim the same plates |
| `model` | asset ref | required | the `.glb` scene, schemed. See [The frame a greeble is authored in](#the-frame-a-greeble-is-authored-in) |
| `health` | float | required | what the piece takes before it comes off |
| `density` | float | required | mass per unit of collider volume. A greeble is light - the base placeholders run 0.05 to 0.2, against a plate's 0.25 |
| `collider` | `(x, y, z)` | required | the box a round stops on, in cells, standing on the mounting face. Not the model: a hull of one would cost more than it is worth |
| `scatter` | rule | every plate | where the piece may stand |

## The scatter rule

Every field is a filter over the plate's NEIGHBOURHOOD, except the last three.
An empty rule matches every plate.

| field | type | default | meaning |
|---|---|---|---|
| `relief` | list | any | the shapes of plate the piece may stand on: `Flat`, `Step`, `Ridge`, `Peak`, `Rim` |
| `facing` | `Any`/`Up`/`Down`/`Side` | `Any` | which way the plate faces in the SHIP's own frame |
| `min_run` | int | 0 | the shortest run of LIKE plate the piece will stand on |
| `min_height` | int | 0 | how much of its cell the plate must fill, in quarter cells (0-4) |
| `min_border` | int | 0 | how far in from the end of that run the plate must be |
| `max_border` | `Some(int)` | none | how far in it may be at most. `Some(0)` is TRIM: only ever at the end of a run |
| `min_depth` | int | 0 | how many cells of ship must stand under the plate |
| `min_enclosure` | int | 0 | how many of the eight surrounding cells the surface carries on into (0-8) |
| `near_fitting` | `Some(int)` | none | how close the mouth of a fitting - a drive bay, a gun well - must be, across the surface |
| `stride` | int | 1 | the LATTICE the piece claims cells on. `2` is every other cell on both in-plane axes |
| `chance` | float | 1.0 | the share of the plates that pass everything above which take the piece |
| `align` | bool | false | yaw the piece so its own `+Z` points down the run |

### The five reliefs

What the top of a plate is shaped like, read off the derivation:

| relief | what it is |
|---|---|
| `Flat` | a flat panel - every boundary sample at the same height. Where a big piece fits |
| `Step` | the plate climbs structure standing proud beside it. A hard edge |
| `Ridge` | a crest across the plate: the tent a run of skin one cell wide comes out as |
| `Peak` | every sample on the floor, so the middle rides half a cell: a lone clad cell |
| `Rim` | the edge of the skin, tapering away |

Measured on the generator's own hulls, about four fifths of every ship comes out
`Rim`, a seventh `Step`, and under a seventh `Flat`, with `Ridge` rare and `Peak`
absent. **A rule written for flat panels alone lands on almost nothing.** Start
from what a hull actually offers.

### Priority: one piece per plate

A plate takes AT MOST ONE piece, and the FIRST fixture in the list whose rule
accepts it wins. So the order is a priority order: put the rare, specific pieces
first and the common filler last.

Watch out for a rule that reads as specific but is not. `near_fitting: Some(1)`
sounds narrow; on a hull dense with drives and bays it is nearly everywhere, and
first in the list it will carpet the ship and starve everything below it.

### Alignment, not noise

`stride` and `align` exist because alignment is what makes decoration read as
bolted on rather than as confetti. A piece claims cells on a lattice and turns to
the direction the surface runs, so a row of vents lines up with itself and with
the hull. There is no random jitter and no rotation freedom, deliberately.

## The scatter is deterministic

There is no RNG. Whether a plate takes a piece is decided by hashing the CELL it
would stand in together with the fixture's id, so:

- the same ship always wears the same decoration, saved or not;
- the build view can show it live while a hull is dragged, without flicker;
- two ships built the same way are decorated the same way.

`chance` therefore does not mean "roll a die"; it means "the share of eligible
cells whose hash falls below this". Lowering it REMOVES pieces rather than moving
them.

## The frame a greeble is authored in

A hull plate is one cell - the unit cube, out along `+Y`. A decoration model uses
that same frame:

- `+Y` is out of the plate and `y = 0` is the mounting face. Nothing sits behind
  it.
- The footprint is centred on the origin and stays inside half a cell, so a piece
  cannot spill across a plate seam onto its neighbour. A tall piece (a mast)
  reaches further up.
- `+Z` is the piece's own long axis, which `align: true` points down the run.
- Flat-shaded, low-poly, untextured, one primitive per flat colour - and under
  200 triangles, because decoration is scattered many times over a hull.

The base game generates its own from committed JSON recipes
(`scripts/gen-greebles.py`, `scripts/greeble-recipes/`). A mod can ship `.glb`
files made any way it likes and reference them the same way.

## Using a style

A ship wears a style by naming it, alongside the `skin` flag that asks for
cladding in the first place:

```ron
kind: Spaceship((
    controller: AI(...),
    skin: true,
    style: Some("my_raider_look"),
    sections: [ ... ],
)),
```

Both fields are per SHIP, so one scenario can put a raider look on its enemies
and a clean one on the civilians. A style id nothing authored leaves the ship
clad and BARE rather than falling back to another look - a missing mod is
visible, not silently substituted.

The base game ships `placeholder`, which is scaffolding for the authored kits
rather than a look to build on.
