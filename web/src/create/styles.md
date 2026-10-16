# Ship skin styles for mods

A `Style` is the LOOK a ship's derived cladding wears: what its plates are made
of, and the decoration placed on them. A [ship](../ships/) names one by id
(`style: Some("raider")` on its hull) and wears it.

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

Decoration is destructible in the full sense: each piece carries its own health
and collider, stops rounds, and comes off when it is shot out, leaving the
plate behind it bare. Nothing decorative does anything else - if losing a thing
should cost the ship an ability, that thing is a [section](../sections/), not
decoration.

<figure class="figure">
    <!-- Capture: assets/greeble-catalog-industrial.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot</span
        >
        <span class="figure__placeholder-name"
            >assets/greeble-catalog-industrial.png</span
        >
        <span class="figure__placeholder-note"
            >The base styles laid out a row per style, each
            piece on its own pedestal under its id, its box
            in meters and its health - the industrial row
            read close, with armoured and civilian behind
            it.</span
        >
    </div>
    <figcaption class="figure__caption">
        Every fixture the base styles place, one row per
        style. A mod's own pieces sit in the same catalog
        once the style declares them.
    </figcaption>
</figure>

## The Style item

```ron
[
    Style((
        id: "my_raider_look",
        name: "Raider",
        palette: (
            top: (
                color: Srgba((red: 0.4, green: 0.3, blue: 0.25, alpha: 1.0)),
                roughness: 0.8,
                metallic: 0.1,
            ),
            wall: (
                color: Srgba((red: 0.2, green: 0.15, blue: 0.12, alpha: 1.0)),
                roughness: 0.9,
                metallic: 0.1,
            ),
        ),
        fixtures: [
            (
                id: "antenna",
                model: "self://gltf/greebles/antenna.glb#Scene0",
                health: 8.0,
                collider: (0.12, 0.38, 0.12),
            ),
        ],
        greeble_rules: (
            link_types: [
                (id: "any", connects: ["any", "empty"]),
                (id: "empty", connects: ["any", "empty", "spaced"]),
                (id: "spaced", connects: ["empty"]),
            ],
            greebles: [
                (
                    id: "empty",
                    fixture: None,
                    weight: 6.0,
                    rotate: r#None,
                    link_points: (up: "empty", right: "empty", down: "empty", left: "empty"),
                ),
                (
                    id: "antenna",
                    fixture: Some("antenna"),
                    weight: 0.3,
                    rotate: r#None,
                    link_points: (up: "spaced", right: "spaced", down: "spaced", left: "spaced"),
                    near: [(greebles: ["antenna"], radius: 4, factor: 0.0)],
                    max_count: Some(2),
                ),
            ],
        ),
    )),
]
```

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | what a ship names this style by. A base id REPLACES that look |
| `name` | string | required | display name |
| `palette` | palette | required | what the plates are made of |
| `fixtures` | list | `[]` | the decoration pieces: model, health and collider |
| `greeble_rules` | rules | required | where the pieces go - see [Greeble rules](#greeble-rules) |

### `palette`

Two surfaces, both required:

| field | meaning |
|---|---|
| `top` | the shell face exposed to space, and normally the only one lit |
| `wall` | the shell side exposed at slopes, gaps and edges |

The third surface - the cell floor, against the section the plate clads - is
not authored. It is never in shot, so it keeps the engine's own finish.

Each of the two is a finish:

| field | type | meaning |
|---|---|---|
| `color` | color | base colour, tagged: `Srgba((red: .., green: .., blue: .., alpha: ..))` or `LinearRgba((..))` - both spellings of the engine colour type parse; the shipped kits author `LinearRgba` |
| `roughness` | float | 0 (mirror) to 1 (matte) |
| `metallic` | float | 0 (dielectric) to 1 (metal) |

**`top` is nearly the whole of what a camera sees.** Measured, by painting
`wall` a colour nothing else uses and shooting the `wfc_ships` row: two plates
in a run press their walls together and neither is ever seen, so `wall` comes
back only at the skin's OUTER RIM and on the side of a plate climbing past a
lower neighbour - roughly a twentieth of the hull. A dark wall under a pale top
therefore does NOT draw a panel line at every plate boundary; it makes the
silhouette read thick, like plate with depth. Panel lines are geometry, or they
are the livery. Budget your effort accordingly.

Both are required because the relationship BETWEEN them is most of a look. A
style that set only one would be a style whose main move was decided by
whichever default it happened to keep.

### `fixtures`

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | names the piece within its style. A greeble places it by this id |
| `model` | asset ref | required | the `.glb` scene, schemed. See [The frame a greeble is authored in](#the-frame-a-greeble-is-authored-in) |
| `health` | float | required | what the piece takes before it comes off |
| `collider` | `(x, y, z)` | required | the box a round stops on, in cells, standing on the mounting face. Not the model: a hull of one would cost more than it is worth |

There is no authored physics density. Every decoration stands at the engine's
`DECOR_DENSITY` of 0.25, the same as the skin's own cladding, and avian derives
the real mass from that and the collider's volume. Nothing a style says about
its look can make one ship heavier than another.

`health` and `collider` stay authored, and for different reasons. `health`
decides when a piece is shot OFF, and two greebles of a size are not equally
worth shooting off. `collider` is authored because a headless simulation never
loads the model its bounds could otherwise be read from.

## Greeble rules

The engine reads a ship's skin as a grid of CELLS: one cell per plate, each with
four edges, and across each edge the plate that continues the surface. The
solve puts exactly one GREEBLE on every cell. A greeble is either a fixture or
empty space, and it shows a LINK TYPE on each of its four edges. Two cells that
share an edge must show link types that connect. The solve does not read the
shape of a plate: every plate is one cell.

| field | type | default | meaning |
|---|---|---|---|
| `link_types` | list | required | every link type the greebles use, at most 64 |
| `greebles` | list | required | every greeble, with at least one `fixture: None`. An empty list places no decoration |

### `link_types`

| field | type | meaning |
|---|---|---|
| `id` | string | the name greebles use for it |
| `connects` | list of ids | the link types it may share an edge with. Must be symmetric: if `a` lists `b`, `b` lists `a` |

A link type that does not connect to any link type of an empty greeble FORCES
its neighbour: the cell across it cannot stay empty. A line link type that
connects only to itself is the usual case.

### `greebles`

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | names the greeble in `near` terms |
| `fixture` | `Option` string | required | the fixture it places, by id, or `None` for empty space |
| `weight` | float | required | relative chance against the other greebles a cell still allows. Above 0. Shared by all the turns |
| `rotate` | `All`, `Half` or `r#None` | required | the quarter turns the solve may use: all four, none and the half turn, or none. Spell the last `r#None`, because a bare `None` reads as an empty `Option` |
| `link_points` | `(up, right, down, left)` | required | the link type on each edge |
| `near` | list | `[]` | range terms, below |
| `max_count` | `Option` integer | `None` | the most cells this greeble may take on one ship |

### `near`

| field | type | meaning |
|---|---|---|
| `greebles` | list of ids | the greebles the term looks for |
| `radius` | integer | how far the term reaches, in cells along the grid. At least 1 |
| `factor` | float | multiplies this greeble's weight where one of `greebles` is in range. 0 is a hard ban, in both directions |

### What fails

Every rule set is checked by `content lint` and again at load. A rule set that
fails is an error, and the style is dropped: every ship that names it flies
clad and bare. Each of these fails:

- a link type or a greeble declared twice, or more than 64 link types;
- a `connects`, a link point or a `near` term that names an unknown id;
- `connects` that is not symmetric;
- a `weight` that is not a finite number above 0, a `factor` that is not a
  finite number of at least 0, or a `radius` of 0;
- no greeble with `fixture: None`;
- a greeble whose `fixture` names no fixture in the style;
- a `factor: 0` term on a greeble that a forcing link type can force, because
  the ban could empty a cell that must be filled;
- a `factor: 0` term or a `max_count` on an empty greeble, because empty space
  must stay available everywhere.

### When a hull contradicts the rules

A hull is built by a player, so some rule sets cannot be met on every hull: a
line that must continue has nowhere to end, or a cap runs out. The solve then
starts again with a different hash, up to eight times. If every attempt fails it
solves once more WITHOUT the greebles that carry a forcing link type, logs a
warning, and leaves any cell that still contradicts empty. The ship is never
refused and never crashes; it is dressed with less.

## The solve is deterministic

There is no RNG. The seed is a hash of the style id, and every draw also hashes
the cell it is for, so:

- the same ship always wears the same decoration, saved or not;
- the build view can show it live while a hull is dragged, without the whole
  hull changing;
- two ships built the same way are decorated the same way.

The structure is not in the seed. An edit changes the draws only where it
changes the cells and their edges, so most plates far from the edit keep
their pieces. Not all of them: the pick order is global, so a few plates
anywhere on the hull can change.

When a ship is RE-CLAD after damage, the plates that stayed keep their pieces
and only the new plates are solved. A link across the edge of the repair is not
checked against the piece that stayed.

`spawn_ship_skin` and the build view log, at debug, how many plates each fixture
took:

```text
decoration mast 2, vent 7, block 11, blister 9
```

A `0` on a big hull is a fixture its weight, its caps or its link types starve.

## The frame a greeble is authored in

A hull plate is one cell - the unit cube, out along `+Y`. A decoration model uses
that same frame:

- `+Y` is out of the plate and `y = 0` is the mounting face. Nothing sits behind
  it.
- The footprint is centred on the origin and stays inside half a cell, so a piece
  cannot spill across a plate seam onto its neighbour. A tall piece (a mast)
  reaches further up.
- `+Z` points at the edge that holds the greeble's `up` link point. A line
  greeble with `line` on `up` and `down` therefore lies with `+Z` down the
  line.
- Flat-shaded, low-poly, untextured, one primitive per flat colour - and under
  200 triangles, because decoration is placed many times over a hull.

The base game generates its own from committed JSON recipes
(`scripts/gen-greebles.py`, `scripts/greeble-recipes/`). A mod can ship `.glb`
files made any way it likes and reference them the same way.

## Using a style

A ship wears a style by naming it, alongside the `skin` flag that asks for
cladding in the first place:

```ron
Ship((
    id: "my_raider",
    name: "My Raider",
    hull: (
        skin: true,
        style: Some("my_raider_look"),
        sections: [ ... ],
    ),
)),
```

Both fields are per HULL, so a raider hull and a civilian hull wear different
looks and every scenario spawning them gets the right one. A style id nothing authored leaves the ship
clad and BARE rather than falling back to another look - a missing mod is
visible, not silently substituted.

The base game ships five looks:

| id | what it is |
|---|---|
| `industrial` | a working hull: exposed services, cranes and stacks, radiator clusters, safety-yellow hazard bands and ducts as lines |
| `armoured` | flat plate, applique clusters, strake lines, sensor blisters and masts |
| `civilian` | a private yacht's: pale satin paint, cobalt stripe and cabin window lines, dishes, fins and fairings |
| `salvage` | mismatched patch clusters, weld seam, chain and hose lines, a lashed drum, a whip antenna |
| `station` | station plate: antennae, beacons, radiator clusters, hatches, markings and window lines |

A ship that names no style flies undressed: built-in plate colours, no
decoration. The EDITOR's build view instead previews the first style the merged
content offers while none is picked. The editor lists every merged style under
its cladding toggle - a mod's look appears there beside the base ones with
nothing to register - and the `wfc_ships` example cycles the same list with `L`,
or takes `--style <id>`.

## Patterns the base kits use

Every base kit is built from the same four shapes of greeble. Read them in
`crates/nova_authoring/src/base_content/styles.rs`.

- **Empty space.** One `fixture: None` greeble on `empty` link points, with
  the largest weight in the kit. It sets how bare the hull is: raise its weight
  and every fixture thins out together.
- **A filler.** `any` on all four link points and `rotate: r#None`. It can
  stand next to anything except a spaced piece. Hatches, vents and stencils.
- **A spaced piece.** `spaced` on all four link points, which connects only to
  `empty`, so nothing stands beside it. Add a `near` ban on itself and a
  `max_count` so a ship carries one or two, apart. Masts, dishes, cranes.
- **A cluster.** A filler with a `near` term on itself at radius 1 and a factor
  above 1, so one piece makes the next one beside it more likely. Radiators,
  applique, salvage patches.
- **A line.** A line link type that connects only to itself, a body greeble
  with it on `up` and `down` and `any` on the sides, `rotate: Half`, and an END
  greeble with it on `up` only and `rotate: All`. Both place the same fixture.
  Because the line link type forces its neighbour, a started line runs until an
  end greeble closes it, so a line never stops at a gap. Use a second line link
  type (`line_2`) for a second line fixture, so the two never join.

```ron
link_types: [
    (id: "any", connects: ["any", "empty"]),
    (id: "empty", connects: ["any", "empty", "spaced"]),
    (id: "spaced", connects: ["empty"]),
    (id: "line", connects: ["line"]),
],
greebles: [
    // ... the empty greeble ...
    (
        id: "stripe",
        fixture: Some("stripe"),
        weight: 1.2,
        rotate: Half,
        link_points: (up: "line", right: "any", down: "line", left: "any"),
    ),
    (
        id: "stripe_end",
        fixture: Some("stripe"),
        weight: 0.3,
        rotate: All,
        link_points: (up: "line", right: "any", down: "any", left: "any"),
    ),
],
```

The model of a line piece fills its cell along `+Z`, so neighbours butt
together. A piece is always centred on its plate, so a full-cell piece still
cannot spill onto a neighbour.

**Reading as unplanned with no randomness.** `salvage` is the worked example.
Its patches are clusters in three materials, so they gather in regions instead
of an even speckle; its weld seam, chain and hose are three separate lines; and
every model is authored off-centre in its own footprint, so a piece repeated is
not a tile.
