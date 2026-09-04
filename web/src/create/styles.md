# Ship skin styles for mods

A `Style` is the LOOK a ship's derived cladding wears: what its plates are made
of, and the decoration scattered over them. A [ship](../ships/) names one by id
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
                    relief: [Ridge, Peak, Spur],
                    facing: Up,
                    min_depth: 2,
                    chance: 0.3,
                    align: Outward,
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
| `color` | color | base colour, tagged: `Srgba((red: .., green: .., blue: .., alpha: ..))` or `LinearRgba((..))` - both spellings of the engine colour type parse; the shipped kits author `LinearRgba` |
| `roughness` | float | 0 (mirror) to 1 (matte) |
| `metallic` | float | 0 (dielectric) to 1 (metal) |

**`Top` is nearly the whole of what a camera sees.** Measured, by painting
`Wall` a colour nothing else uses and shooting the `wfc_ships` row: two plates
in a run press their walls together and neither is ever seen, so `Wall` comes
back only at the skin's OUTER RIM and on the side of a plate climbing past a
lower neighbour - roughly a twentieth of the hull. A dark wall under a pale top
therefore does NOT draw a panel line at every plate boundary; it makes the
silhouette read thick, like plate with depth. Panel lines are geometry, or they
are the livery. Budget your effort accordingly.

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
| `seat` | `Whole`/`Any` | `Whole` | what the plate's TOP must be: one unbroken surface, or anything at all - see [The seat](#the-seat-one-surface-or-a-crease) |
| `relief` | list | any | which ZONE of the hull the piece may stand in - see [The seven reliefs](#the-seven-reliefs) |
| `facing` | `Any`/`Up`/`Down`/`Side` | `Any` | which way the plate faces in the SHIP's own frame |
| `min_run` | int | 0 | the shortest run of LIKE plate the piece will stand on |
| `min_height` | int | 0 | how much of its cell the plate must fill, in quarter cells (0-4) |
| `min_border` | int | 0 | how far in from the end of that run the plate must be |
| `max_border` | `Some(int)` | none | how far in it may be at most. `Some(0)` is TRIM: only ever at the end of a run |
| `min_depth` | int | 0 | how many cells of ship must stand under the plate |
| `min_enclosure` | int | 0 | how many of the eight surrounding cells the surface carries on into (0-8) |
| `near_fitting` | `Some(int)` | none | how many steps across the surface the nearest fitting - a drive, a bay, a gun mount - may be at most |
| `stride` | int | 1 | the LATTICE the piece claims cells on. `2` is every other cell on both in-plane axes |
| `chance` | float | 1.0 | the share of the plates that pass everything above which take the piece |
| `patch` | int | 0 | at least one piece per block of this many cells - see [Density](#density-one-piece-per-block-of-hull) |
| `align` | `Free`/`Run`/`Outward` | `Free` | which way the piece is turned - see [Alignment](#alignment-not-noise) |

### The seat: one surface, or a crease

A plate's top is either ONE PLANE or a CONE, and nothing in between. `seat` is
the filter for it, and it is the one filter that is ON by default:

| `seat` | what it takes |
|---|---|
| `Whole` | plates whose top is one unbroken surface. TILTED COUNTS - a ramp is a surface, and a piece is bedded onto it |
| `Any` | any plate at all. The exception, for a piece that WANTS the high ground: a crest, a spar tip or a stud |

A piece is stood up on the plate it lands on: on a surface it is turned onto that
surface's own normal, so it lies flush however far the plate is raked; on a cone
it stands up the cell, because a cone has no plane to lie on and the middle of
the plate is its apex.

**Do not use `relief` to mean this.** Measured over 526 generated plates: `Flat`
and `Brink` are surfaces every time, `Bevel`, `Ridge`, `Peak` and `Spur` are cones
every time, and `Step` splits about in half - square-on to a raise it is a clean
ramp, on its diagonal it is a cone. A relief list written to mean "somewhere flat
to lie" is therefore wrong about a fifth of a hull in both directions. Name the
zone with `relief` and the seat with `seat`.

**A hand-built ship has no seat anywhere on it.** A small build is one cell thick
nearly everywhere, so every plate on it is a cone: the owner's five-cube L reads
0% seated against 58.6% on a generated hull. A kit whose every rule is `Whole`
therefore decorates a generated ship and leaves an editor build BARE. Give the
pieces that belong on the high ground `seat: Any` - every base kit does, for its
mast, whip, fin, stack or corner boss - and they carry the small builds.

### The seven reliefs

What the top of a plate is shaped like, read off the derivation. Four fifths of a
ship FALLS AWAY somewhere, and the last three say how many ways:

| relief | what it is |
|---|---|
| `Flat` | a flat panel - every boundary sample at the same height. Where a big piece fits |
| `Step` | the plate climbs structure standing proud beside it. A hard edge |
| `Ridge` | a crest across the plate: the tent a run of skin one cell wide comes out as |
| `Peak` | every sample on the floor, so the middle rides half a cell: a lone clad cell |
| `Bevel` | falls at ONE CORNER only: a panel with a corner taken off, and nearly a `Flat` |
| `Brink` | falls along ONE WHOLE SIDE: the straight edge of a hull. The only relief with an outward direction to turn to |
| `Spur` | falls TWO WAYS OR MORE: an outer corner, the tip of a spar, a saddle |

### Start from what a hull offers

A GENERATED hull and a HAND-BUILT one offer almost opposite things. Measured, per
ship, on the three fixed seeds of the `wfc_ships` row and on a 7-9 part editor
build:

| subject | plates | flat | step | ridge | peak | bevel | brink | spur |
|---|---|---|---|---|---|---|---|---|
| generated (the `wfc_ships` row) | 158-204 | 18-38 | 12-52 | 0-4 | 0 | 4-8 | 48-76 | 32-76 |
| hand-built (a 7-9 part editor ship) | 22-27 | 0 | 2-3 | 0-5 | 1-4 | 0 | 0 | 14-20 |

**A rule written for flat panels lands on almost nothing, and on a hand-built
ship it lands on NOTHING AT ALL.** A small build is one cell wide nearly
everywhere, so it has no flat plate, no bevel and no brink - only spurs, ridges
and studs, none of which is a [seat](#the-seat-one-surface-or-a-crease). No
amount of `patch` rescues that: a floor over an empty set is still empty. Widen
the `relief` list, and give the piece `seat: Any` if it belongs on the high
ground.

**Read the SPREAD, not the middle.** Every bucket above swings by two or three
times across three seeds of one generator, so a rule sized against one hull is
not sized against the next. Tune against the logged tally below, on more than
one subject.

### Priority: one piece per plate

A plate takes AT MOST ONE piece, and the FIRST fixture in the list whose rule
accepts it wins. So the order is a priority order: put the rare, specific pieces
first and the common filler last.

Watch out for a rule that READS AS SPECIFIC AND IS NOT. Two measured examples,
both of which carpeted a ship and starved every rule below them:

- `near_fitting: Some(1)` sounds narrow. On a hull dense with drives and bays,
  something is beside a fitting nearly everywhere.
- `max_border: Some(0)` - "only at the end of a run" - admitted 126 of 132
  plates, because on a broken-up hull almost every plate is the end of its own
  one-cell run. Pair it with `min_run: 2` so a run has to be a run.

Both are visible rather than guessable. `spawn_ship_skin` and the build view log
each rule as `taken of reach` at debug, where reach is everything the filter and
the lattice admit before the share and before priority:

```text
decoration mast x3 of 8, vent x5 of 7, block x29 of 94, blister x12 of 19
```

`x0 of 78` is a rule that was starved or thinned away; `x0 of 0` is a filter that
matches nothing this hull has. They look identical on screen and have opposite
fixes.

### Density: one piece per block of hull

Every field above is per plate, and they MULTIPLY. A stride of 2 is a quarter of
the surface, a share of 0.5 is half of that, and a relief filter is another
fraction again - which reads as a field of pieces on a 150-plate generated hull
and as one piece on a 20-plate hand-built one.

`patch` states the density in cells of ship instead. Set it to `N` and the rule is
guaranteed a piece in every block of `N` x `N` x `N` cells, per face, that it can
stand on at all: where the share already put one, nothing happens; where it did
not, the block's lowest-hashing eligible plate takes one.

- It is a FLOOR, never a cap. It only ever adds, and it never takes a plate
  another rule already claimed, so priority still means what it says.
- It drops the SHARE only. The filter and the lattice still hold, so a floor
  piece lands on the same grid the rest of the rule does. Keep `patch` at or
  above `stride`.
- `chance: 0.0` with a `patch` set is the pure form: no share at all, exactly one
  piece per block. That is a density that reads the same at any hull size.
- On a BIG hull a small `patch` outvotes the share - one per 3 cells over a
  fragmented hull is a lot of pieces. Size it against the ship, then check the
  logged tally.

### Alignment, not noise

`stride` and `align` exist because alignment is what makes decoration read as
bolted on rather than as confetti. There is no random jitter and no rotation
freedom, deliberately - a piece is turned in quarter turns about the plate's own
outward axis, or not at all.

| `align` | what the piece's own `+Z` points down |
|---|---|
| `Free` | nothing. Right for anything with no long axis - a blister, a stud, a hatch |
| `Run` | the direction the surface RUNS, so a rib strip follows the spine it is on and a row of vents lines up with itself |
| `Outward` | the direction the surface FALLS, which is off the ship - a fairing leans out over the edge it stands on instead of lying along it |

The piece is turned ACROSS THE SURFACE it lies on, and then bedded onto it, so on
a raked plate its `+Z` comes out raked too: a fairing on a hull edge noses down
the slope rather than standing square out of it.

The two are square to each other. `Outward` is a rule for the falling plate:
`Brink` has a single outward cardinal, an outer corner leans on the diagonal
between its two sides, and a plate that does not fall one way is left unturned.

## The scatter is deterministic

There is no RNG. Whether a plate takes a piece is decided by hashing the CELL it
would stand in together with the fixture's id, so:

- the same ship always wears the same decoration, saved or not;
- the build view can show it live while a hull is dragged, without flicker;
- two ships built the same way are decorated the same way.

`chance` therefore does not mean "roll a die"; it means "the share of eligible
cells whose hash falls below this". Lowering it REMOVES pieces rather than moving
them.

`patch` is the one thing decided by a BLOCK of hull rather than by a single cell,
and the blocks are a fixed division of the ship's own cells. So growing a hull by
one cell leaves every piece outside the block that cell lands in exactly where it
was; inside that block the floor's own pick can move, if the new plate hashes
lower. Nothing shuffles across the ship.

## The frame a greeble is authored in

A hull plate is one cell - the unit cube, out along `+Y`. A decoration model uses
that same frame:

- `+Y` is out of the plate and `y = 0` is the mounting face. Nothing sits behind
  it.
- The footprint is centred on the origin and stays inside half a cell, so a piece
  cannot spill across a plate seam onto its neighbour. A tall piece (a mast)
  reaches further up.
- `+Z` is the piece's own long axis, which `align: Run` points down the run and
  `align: Outward` points off the ship.
- Flat-shaded, low-poly, untextured, one primitive per flat colour - and under
  200 triangles, because decoration is scattered many times over a hull.

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

The base game ships four authored looks and one piece of scaffolding:

| id | what it is |
|---|---|
| `industrial` | a working hull: exposed services, corrugation, radiators, safety-yellow paint on its edges |
| `armoured` | flat plate, a belt down every straight edge, sensor blisters |
| `civilian` | a private yacht's: pale satin paint, a cobalt livery rail, lit cabin windows, smooth fairings |
| `salvage` | the raider's: mismatched patches, weld beads, a lashed drum, a whip antenna |
| `placeholder` | scaffolding for the authored kits rather than a look to build on |

A ship that names no style flies undressed: built-in plate colours, no
decoration. The EDITOR's build view instead previews the first style the merged
content offers while none is picked, so the authored looks are listed before
the scaffolding. The editor lists every merged
style under its cladding toggle - a mod's look appears there beside the base
ones with nothing to register - and the `wfc_ships` example cycles the same list
with `L`, or takes `--style <id>`.

### Drawing a CONTINUOUS line

Decoration cannot span cells: every piece stands on one plate. A band that
looks continuous is therefore a row of pieces, and it needs three things at
once - `civilian_stripe` is the worked example.

- The MODEL fills its cell along `+Z` (a raised budget in the recipe), so
  neighbours butt together instead of leaving a gap. A piece is always centred
  on its plate, so a full-cell piece still cannot spill onto a neighbour.
- The RULE takes every eligible plate: `stride: 1` and `chance: 1.0`. A band
  with every other cell missing is a dashed line.
- `align: Run` turns each piece down the run, and `relief: [Brink]` is the run
  worth following - the straight edge of a hull.

**Do not try to cap the run's ends with `min_border`.** `border` is the smallest
of the four in-plane walks, and a `Brink` has open space on one side, so a hull
edge reads border 0 along its whole length: a body rule on `min_border: 1`
matches nothing (`x0 of 0`) and the terminal takes the entire edge. `min_border`
is a rule for `Flat` and for nothing else.

### Reading as UNPLANNED with no randomness

The scatter has no RNG in it, deliberately. `salvage` is the worked example of
what that makes hardest: decoration that reads as unplanned. Four devices carry
it, and none of them is jitter:

- three patch pieces in three materials, each gated on a DIFFERENT structural
  reading (`near_fitting`, `min_depth`, a `patch` floor), because a hash is
  spatially incoherent and splitting one rule three ways by `chance` alone gives
  an even speckle rather than regions;
- two of those pieces authored long on OPPOSITE in-plane axes with both aligned
  to the run, so neighbouring repairs cross at right angles while every piece
  stays square to the grid;
- every model authored OFF-CENTRE in its own footprint, since the scatter offers
  no jitter and a centred piece repeated is a tile;
- a weld bead built from four lumps of different size rather than from the
  `ribs` primitive, because even ribbing reads as machined.
