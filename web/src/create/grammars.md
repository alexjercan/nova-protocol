# Ship grammars for mods

A `Grammar` is the table a PROCEDURAL hull is drawn from: the parts a generator
may reach for, how often it reaches for each, how big the block it works in is,
and how much of that block it is allowed to leave empty.

It is not the rule about what may sit next to what. That is already in your
[sections](../sections/): the generator reads the link points off each prototype
and works out the joins by itself, so a mod that ships a section has already
changed what a hull can be built from. What a section cannot say is TASTE - that
a drive belongs at the back, that a turret should be rare, that this line of ship
is long and narrow with a broad tail. That is what a grammar is for.

This page is the field-by-field grammar reference. For general RON spelling rules
such as double parentheses, `Some(...)`, and asset schemes, see
[RON spelling rules](../reference/#how-ron-content-is-written).

## The Grammar item

```ron
[
    Grammar((
        id: "my_mod_gunship",
        name: "Gunship",
        grid: (
            half_width: 3,
            height: 5,
            length: 13,
        ),
        vacuum: (
            base: 0.22,
            taper: 1.4,
            stern: 9.0,
            bow_taper: 24.0,
        ),
        keel: (
            hull: "reinforced_hull_section",
            bridge: "basic_controller_section",
            stern_deck: "reinforced_hull_section",
            stern_drive: "basic_thruster_section",
            bow_gun: Some("railgun_section"),
        ),
        parts: [
            (prototype: "reinforced_hull_section", weight: 6.0),
            (prototype: "basic_controller_section", weight: 0.15),
            (prototype: "basic_thruster_section", weight: 6.4, aim: Some(Aft)),
            (prototype: "torpedo_section", weight: 1.4, zone: Some(Flank)),
            (prototype: "pdc_kinetic_turret_section", weight: 1.0),
        ],
    )),
]
```

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | Stable key, and the overlay key. Re-declare a base id to replace that grammar; a new id is a new line of ship. Prefix new ids with your mod id. |
| `name` | string | required | What a picker shows. |
| `grid` | grid | required | The block of cells the hull is built in - see [below](#the-grid). |
| `vacuum` | vacuum | required | How emptiness is priced against the parts - see [below](#vacuum-the-silhouette). |
| `keel` | keel | required | The spine laid down before the generator gets a say - see [below](#the-keel). |
| `parts` | list of parts | required | What may be drawn, and how often - see [below](#parts-and-weights). |

Nothing here is optional with a silent default. A grammar names every role it
seeds and every part it draws, and a section id it names that your catalog does
not hold is a `content lint` error and a load refusal - not a hull with a gap in
it.

## The grid

```ron
grid: (half_width: 4, height: 5, length: 11),
```

| field | type | meaning |
|---|---|---|
| `half_width` | number | Cells across the STARBOARD half. The hull is this wide either side of the centreline. |
| `height` | number | Cells tall. |
| `length` | number | Cells nose to tail. |

These are counts of build-grid cells, not meters. One cell is one world unit,
which is 10 m, so the shipped `standard_hull` at `4 x 5 x 11` is a hull roughly
80 m across, 50 m tall and 110 m long.

The floor on all three is set by the SKIN, not by the structure. A plate is drawn
as a flat run only where it has neighbours on all four sides, so a surface needs
to be at least three cells across before it has any interior at all. Go below
that in any axis and the hull comes out as all rim: a heap of ramps rather than a
ship. Length noticeably more than width is most of why a result reads as a craft
instead of as a box.

## Vacuum: the silhouette

```ron
vacuum: (base: 0.22, taper: 1.4, stern: 9.0, bow_taper: 24.0),
```

Vacuum is drawn against the parts like anything else, and these four numbers are
its weight. They decide WHERE the hull may be sparse and never what may sit next
to what - this is the shape dial, and it is pure taste.

| field | type | meaning |
|---|---|---|
| `base` | number | Weight of vacuum everywhere. Keep it low: a porous hull is a lattice, and skin on a lattice is one plate per strut - noise, not a surface. |
| `taper` | number | How much likelier vacuum gets per cell away from the keel. This is how the hull thins toward its edges. |
| `stern` | number | How much likelier vacuum gets in the LAST row. A drive carries one socket and needs its other five faces clear, so on a hull built solid it has nowhere to stand at all. |
| `bow_taper` | number | How much likelier vacuum gets at the bow than at the stern. This is the whole silhouette: a nose and a broad tail rather than a brick. |

If your hulls come out as slabs, `bow_taper` is the first number to raise. If the
drives are missing, raise `stern`. If the result looks like scaffolding, lower
`base`.

## The keel

```ron
keel: (
    hull: "reinforced_hull_section",
    bridge: "basic_controller_section",
    stern_deck: "reinforced_hull_section",
    stern_drive: "basic_thruster_section",
    bow_gun: Some("railgun_section"),
),
```

The keel is placed before the generator starts, and it buys one guarantee: a
seeded spine is a single connected structure to grow on, so a hull never comes
out as a cloud of islands.

| field | type | default | meaning |
|---|---|---|---|
| `hull` | string | required | What the keel is made of, and what a one-cell dent is packed with. Must carry a socket on every face it can be met on. |
| `bridge` | string | required | The flight computer, laid a third of the way back where a bridge reads. |
| `stern_deck` | string | required | The block beside the last keel cell that the seeded drive bolts to. |
| `stern_drive` | string | required | The drive on that block's aft face. Mirrored like everything else, so a hull comes out with a PAIR either side of the centreline. |
| `bow_gun` | `Option` string | `None` | The spinal gun on the bow end of the keel. `None` = a hull plan that seats no spinal gun - not a default. |

A spinal gun is seeded rather than drawn for the same reason the stern drive is:
a gun the whole SHIP aims fires down its own axis, and a lane that long is only
ever clear at an END of the hull. On the keel line it mirrors into a tight pair
and keeps the recoil on the ship's axis.

## Parts and weights

```ron
parts: [
    (prototype: "basic_thruster_section", weight: 6.4, aim: Some(Aft)),
    (prototype: "torpedo_section", weight: 1.4, zone: Some(Flank)),
],
```

| field | type | default | meaning |
|---|---|---|---|
| `prototype` | string | required | A [section](../sections/) id from the visible catalog - yours, or one from a mod you depend on. |
| `weight` | number | required | How OFTEN the part is offered, spent across whatever orientations of it are legal in a cell. Taste, not a rule: it never decides where a part may go. |
| `aim` | `Option` face | `None` | The only face this part may fire, launch or exhaust through: `Aft`, `Bow`, `Starboard`, `Port`, `Dorsal`, `Ventral`. `None` = the mating rule alone decides, which is the answer for a traversing turret and for a broadside tube. |
| `zone` | `Option` region | `None` | The only region of the hull this part may stand in: `Bow`, `Amidships`, `Stern` (thirds nose to tail), `Dorsal`, `Ventral` (above and below the keel row), `Flank` (the outboard half, clear of the centreline). `None` = anywhere the mating rule allows. |

`parts` is the DRAW, not the rule. A prototype left off this list is never
offered; one on it may still go nowhere, because the catalog's link points refuse
the join.

`aim` and `zone` answer different questions - an aim says which way a part
POINTS, a zone says where it STANDS - and both are taste with teeth: neither can
make an illegal placement legal, only forbid a legal one. A zone holds a part
only if EVERY cell of it is inside, so a three-cell lance zoned `Bow` has to lie
wholly in the forward third rather than merely start there.

Expect fitting weights to sit far above what a headcount would suggest. A fitting
is not competing with hull for a cell; it is competing for a cell WITH ROOM
AROUND IT, and most of the ones drawn are taken back off again when their exit
lane turns out to be blocked.

## Base grammars

One ships: `standard_hull` ("Standard Hull"), the keeled, mirrored warship the
editor's generator draws from unless told otherwise. Read
`assets/base/grammars/base.content.ron` for its exact numbers - it is the tuned
starting point to copy and push around, and every value in it was arrived at by
looking at hulls rather than derived from anything.
