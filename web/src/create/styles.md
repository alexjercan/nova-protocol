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
        Every fixture the base styles scatter, one row per
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
                placement: (
                    region: HighGround,
                    density: Sparse,
                    orientation: Outward,
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
| `palette` | palette | required | what the plates are made of |
| `fixtures` | list | `[]` | the decoration, in PRIORITY order - see [Priority](#priority-one-piece-per-plate) |

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
| `id` | string | required | names the piece within its style, and SALTS its scatter - two pieces sharing one placement do not claim the same plates |
| `model` | asset ref | required | the `.glb` scene, schemed. See [The frame a greeble is authored in](#the-frame-a-greeble-is-authored-in) |
| `health` | float | required | what the piece takes before it comes off |
| `collider` | `(x, y, z)` | required | the box a round stops on, in cells, standing on the mounting face. Not the model: a hull of one would cost more than it is worth |
| `placement` | placement | `(region: Anywhere, density: Regular, orientation: Free)` | where the piece belongs |

There is no authored physics density. Every decoration stands at the engine's
`DECOR_DENSITY` of 0.25, the same as the skin's own cladding, and avian derives
the real mass from that and the collider's volume. Nothing a style says about
its look can make one ship heavier than another.

`health` and `collider` stay authored, and for different reasons. `health`
decides when a piece is shot OFF, and two greebles of a size are not equally
worth shooting off. `collider` is authored because a headless simulation never
loads the model its bounds could otherwise be read from - and it is
load-bearing for the LOOK as well, see [What the collider
decides](#what-the-collider-decides).

## The placement

Three words. They say what a piece is FOR, not how the skin is derived:

| field | type | default | meaning |
|---|---|---|---|
| `region` | region | `Anywhere` | the part of the hull the piece belongs on |
| `density` | density | `Regular` | how much of that region it covers |
| `orientation` | orientation | `Free` | which way it is turned on the plate it lands on |

Each expands internally into the neighbourhood filter, the lattice, the share
and the patch floor the scatter actually runs. That expansion is engine policy:
it is tuned against real hulls and it changes when the plate derivation
changes. A style that had spelled it out would go quietly wrong the next time
it did. You pick the high ground; the engine decides what the high ground
measures on this hull.

### The seven regions

| region | what it is |
|---|---|
| `Panel` | unbroken panel: the flat face of a hull, and the panel with one corner taken off that still reads as one. Where a sign, a hatch or a decal goes |
| `Deck` | working deck: flat plate with real ship under it. The laydown area - a crate, a rack, a tank, anything that would need a floor to stand on |
| `Flank` | the ship's SIDES: deck plate that faces out rather than up. Windows, doors and anything read from another ship |
| `Edge` | the straight edge of a hull, where the skin falls away along one whole side. Rim strips, hazard paint, fairings - the pieces that draw a silhouette |
| `HighGround` | the pointiest thing on the hull: crests, spar tips and studs. Masts, stacks and aerials |
| `NearFitting` | right beside the mouth of a fitting - a nozzle, a gun well, a bay. The service kit: grilles, lighting, machinery |
| `Anywhere` | anything with a seat on it. The FILLER, and the default: a style's last rule, which takes whatever the rules above it left |

**`NearFitting` reads as narrow and is not.** On a hull dense with drives and
bays, something is beside a fitting nearly everywhere: measured, a pocket rule
placed first took 45% of a ship and starved every rule below it. Every base kit
puts its pocket rule last.

**`HighGround` is the one region that accepts a creased plate.** A crest, a
spar tip and a stud are cones every time, so a region that demanded an unbroken
surface would refuse every plate the high ground is for - and would strip a
ship of its silhouette to fix a bedding defect. Everywhere else, a piece is
bedded onto one flat surface, and the plates that crease are refused unless the
piece is trim (see below).

### Start from what a hull offers

A GENERATED hull and a HAND-BUILT one offer almost opposite things. The engine
reads every plate into one of seven shapes and logs the tally beside the
decoration tally (see [Priority](#priority-one-piece-per-plate)). Measured per
ship, on the three fixed seeds of the `wfc_ships` row and on a 7-9 part editor
build:

| subject | plates | flat | step | ridge | peak | bevel | brink | spur |
|---|---|---|---|---|---|---|---|---|
| generated (the `wfc_ships` row) | 158-204 | 18-38 | 12-52 | 0-4 | 0 | 4-8 | 48-76 | 32-76 |
| hand-built (a 7-9 part editor ship) | 22-27 | 0 | 2-3 | 0-5 | 1-4 | 0 | 0 | 14-20 |

Only `flat` and `step` are unbroken surfaces; `brink` is an outer rim, and
`ridge`, `peak` and `spur` are the creases and studs of a skinny hull. A small
build is one cell wide nearly everywhere, so it has no flat plate, no bevel and
no brink at all. **`Panel`, `Deck`, `Flank` and `Edge` therefore land on
NOTHING on a hand-built hull.** A kit that uses only those decorates a
generated ship and leaves an editor build bare. Give a style at least one
`HighGround` piece and at least one `Anywhere` piece flat enough to count as
trim; every base kit does.

**Read the SPREAD, not the middle.** Every bucket above swings by two or three
times across three seeds of one generator, so a rule sized against one hull is
not sized against the next. Tune against the logged tally below, on more than
one subject.

### The five densities

A ladder rather than a number, because the dials underneath it interact and a
style that set them apart gets a hull carpeted at one size and bare at another.

| density | what it is for |
|---|---|
| `Rare` | the thinnest rung there is: a piece or two on a whole ship. The share is nearly off, so what actually places it is the floor below |
| `Sparse` | scattered thinly, with a floor so a big hull does not clump it |
| `Regular` | the workhorse rung: a piece every few cells of the region |
| `Dense` | heavy cover, still on the lattice |
| `Every` | every plate the region admits, off the lattice entirely. For the pieces that ARE the surface - a rim strip down a whole edge, cladding |

Every rung but `Every` carries a patch floor: a guarantee of at least one piece
per block of hull the rule can stand on at all. That is what keeps a rule
reading the same on a 150-plate generated hull and a 20-plate hand-built one.

The floor takes free plate first. When there is none left - a thin rule whose
whole region was swept by the broad rules under it - it may BORROW one, and
only under three conditions: the lender is thicker than the borrower, it has a
floor of its own (so `Every` never lends), and it still keeps a piece somewhere
on the ship. That is why a rare piece on a busy region lands at all, and it is
bounded enough that the rule underneath never disappears.

### Orientation

Quarter turns about the plate's own outward axis, and nothing finer. There is
no random jitter and no free yaw, deliberately - alignment is what makes
decoration read as bolted on rather than as confetti.

| `orientation` | what the piece's own `+Z` points down |
|---|---|
| `Free` | nothing. Right for anything with no long axis - a blister, a stud, a hatch |
| `Along` | the direction the surface RUNS, so a rib strip follows the spine it is on and a row of vents lines up with itself |
| `Outward` | the direction the surface FALLS, which is off the ship - a fairing leans out over the edge it stands on instead of lying along it |

The piece is turned ACROSS THE SURFACE it lies on, and then bedded onto it, so
on a raked plate its `+Z` comes out raked too: a fairing on a hull edge noses
down the slope rather than standing square out of it.

The two are square to each other. `Outward` is a rule for the falling plate,
and `Edge` above all: a plate that does not fall one way is left unturned.

### What the collider decides

Two things are read off the piece's own size rather than authored, because the
size already answers them and a second authored word could disagree with it:

- **How long a run of like plate it needs under it.** One cell of run per half
  cell the piece spans across the plate (the two in-plane axes; `y` stands off
  the plate and says nothing). So a strip most of a cell long asks for a
  neighbour either side, and a stud asks for nothing. Shorten a model and its
  gate relaxes with it.
- **Whether it needs an unbroken seat at all.** A piece standing less than 0.1
  cells - one metre - off the plate is TRIM: a stencil, a livery patch or a
  hazard stripe, where a crease underneath is a fold in a decal rather than a
  gap you can see. Anything that stands proud needs the seat, because on a
  crease it touches along one line and floats at both ends.

Trim is also what reaches a hand-built hull, which has no unbroken plate on it
anywhere. A kit whose every filler stands proud leaves exactly the shapes
players build bare.

### Priority: one piece per plate

A plate takes AT MOST ONE piece, and the FIRST fixture in the list whose
placement accepts it wins. So the order is a priority order: put the rare,
specific pieces first and the common filler last.

Watch out for a placement that READS AS SPECIFIC AND IS NOT. `NearFitting` is
the measured example - see above. The general failure is a rare rule sampled
after a broad one: it is starved out of the leavings and lands nothing, which
looks identical on screen to a rule that matched nothing at all.

Both are visible rather than guessable. `spawn_ship_skin` and the build view
log each rule as `taken of reach` at debug, where reach is everything the
region and the lattice admit before the share and before priority:

```text
decoration mast x3 of 8, vent x5 of 7, block x6 of 94, blister x12 of 19
```

`x0 of 78` is a rule that was starved or thinned away; `x0 of 0` is a region
that matches nothing this hull has. They look identical on screen and have
opposite fixes.

## The scatter is deterministic

There is no RNG. Whether a plate takes a piece is decided by hashing the CELL it
would stand in together with the fixture's id, so:

- the same ship always wears the same decoration, saved or not;
- the build view can show it live while a hull is dragged, without flicker;
- two ships built the same way are decorated the same way.

A density rung therefore does not mean "roll a die"; its share is "the fraction
of eligible cells whose hash falls below this". Dropping a rung REMOVES pieces
rather than moving them.

The patch floor is the one thing decided by a BLOCK of hull rather than by a
single cell, and the blocks are a fixed division of the ship's own cells. So
growing a hull by one cell leaves every piece outside the block that cell lands
in exactly where it was; inside that block the floor's own pick can move, if the
new plate hashes lower. Nothing shuffles across the ship.

## The frame a greeble is authored in

A hull plate is one cell - the unit cube, out along `+Y`. A decoration model uses
that same frame:

- `+Y` is out of the plate and `y = 0` is the mounting face. Nothing sits behind
  it.
- The footprint is centred on the origin and stays inside half a cell, so a piece
  cannot spill across a plate seam onto its neighbour. A tall piece (a mast)
  reaches further up.
- `+Z` is the piece's own long axis, which `orientation: Along` points down the
  surface's run and `orientation: Outward` points off the ship.
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
- The RULE takes every eligible plate: `density: Every`, which is the one rung
  off the lattice. A band with every other cell missing is a dashed line.
- `region: Edge` is the run worth following - the straight edge of a hull - and
  `orientation: Along` turns each piece down it.

So a continuous band is `(region: Edge, density: Every, orientation: Along)` and
a model that fills its cell. Every base kit's rim piece is spelled exactly that
way; what differs between them is only the model.

**A band cannot be capped with a different piece at its ends.** There is no
authored way to ask for "the end of a run": ends are not a region. Draw the
whole edge with one piece and let the silhouette end where the hull does.

### Reading as UNPLANNED with no randomness

The scatter has no RNG in it, deliberately. `salvage` is the worked example of
what that makes hardest: decoration that reads as unplanned. Four devices carry
it, and none of them is jitter:

- three patch pieces in three materials, spread across regions and rungs
  (`salvage_patch_strip` on `NearFitting`/`Regular`, `salvage_patch_plate` on
  `Anywhere`/`Regular`, `salvage_patch_scab` on `Anywhere`/`Sparse` below it),
  because a hash is spatially incoherent: one rule split three ways gives an
  even speckle, while a rule gated on the ship's own service gear gives
  regions;
- two of those pieces authored long on OPPOSITE in-plane axes with both on
  `orientation: Along`, so neighbouring repairs cross at right angles while
  every piece stays square to the grid;
- every model authored OFF-CENTRE in its own footprint, since the scatter offers
  no jitter and a centred piece repeated is a tile;
- a weld bead built from four lumps of different size rather than from the
  `ribs` primitive, because even ribbing reads as machined.
