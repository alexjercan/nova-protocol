# Notes

Phase A: the plate vocabulary, styles as content, and the deterministic scatter.
The art half (`scripts/gen-greebles.py`, four placeholder `.glb`) landed
separately in `f48dcaee` and is not touched here.

Landed as `13d0b83c` on `skin-vocabulary`, off `c763ee28`.

## What was exposed, and what was dropped

`read_plates` (`crates/nova_ship/src/sections/skin_reading.rs`) is a SECOND PASS
over the finished plates, never a second derivation. The plates are the whole
input: their cells are the clad set, `cell - anchor` is the face each shows to
space, and their shapes are the relief - so a reading cannot drift from the skin
it describes.

| reading | what it answers |
| --- | --- |
| `relief` | `Flat` / `Step` / `Ridge` / `Peak` / `Rim` |
| `out` | which way the plate faces, in ship cells |
| `along` | the in-plane direction the run through it points |
| `run` | how long that run of LIKE plate is |
| `border` | how far from the end of that run |
| `enclosure` | how many of the 8 in-plane cells the surface carries on into |
| `height` | how much of its cell the plate fills, in quarter cells |
| `depth` | how many cells of ship stand under it |
| `pocket` | how far the mouth of the nearest fitting is |

Three beyond the task's list, and why:

- `depth` is FREE. `plate_for` already measures it to choose which way a plate
  faces, then throws it away. It separates the skin over the BODY of a ship from
  the skin over a one-cell spar.
- `along` is the alignment axis, and it is the single most load-bearing field.
  The research says grid claiming rather than blue noise BECAUSE alignment is
  what makes a greeble read as bolted on; without an axis to turn to, a rule can
  only make confetti.
- `height` splits the rim, which nothing else could. See the measurement below.

Nothing was dropped. One thing was REDEFINED after measuring: `run` and `border`
started out as "the contiguous FLAT run", the task's words. That reads as silence
on four fifths of every real ship, so they are now measured over LIKE plate - a
flat panel's run is its flat neighbours, and a rim's run is the edge of the ship
it lies on. The flat case is unchanged.

`pocket` is the research's "weight decoration toward link points", answered with
the only socket fact that discriminates: every hull cube offers a socket on every
face, so socket proximity is constant, and it is the BLIND faces - the fittings -
that mark where a ship gets interesting.

## The style schema

`ShipStyleConfig` (`skin_style.rs`), routed as `Content::Style` into `GameStyles`
with the same last-wins-by-id overlay every other content kind gets.

```
Style(( id, name,
  surfaces: [( surface: Top|Wall|Floor, color, roughness, metallic )],
  fixtures: [( id, model, health, density, collider, scatter: (
      relief, facing, min_run, min_height, min_border, max_border,
      min_depth, min_enclosure, near_fitting, stride, chance, align ) )] ))
```

A style cannot author a plate's SHAPE - that is a function of the hull - only
what it is made of and what is bolted to it. `fixtures` is a PRIORITY list: a
plate takes at most one piece and the first accepting rule wins.

A ship names one with `style: Some("<id>")` beside `skin: true`. An id nothing
authored leaves the ship clad and BARE, never falling back to another look.

The editor has no picker yet, so the build view falls back to the FIRST authored
style rather than a hard-coded id - a mod that ships one look is what it shows.

## How the scatter stays deterministic

No RNG anywhere, and there must never be one. A plate's claim is
`share(fnv1a(cell, out, fixture.id)) < chance`.

- Hand-written FNV-1a, not `DefaultHasher`: std's hasher is not promised stable
  across releases, and a ship that comes back wearing different antennae after a
  toolchain bump breaks the promise the derived skin exists to keep.
- The OUT FACE is in the hash as well as the cell. A corner cell can be clad from
  two directions on two ships, and those are different places.
- `scatter_decor` takes the READINGS, not the structure, so it cannot reach past
  the vocabulary into the derivation.
- `chance` removes claims, it never moves them - pinned by a test.

## Measured

`--lib` test profile, synthetic 8x8x8 block, 384 plates:

| | cost |
| --- | --- |
| `derive_skin` | 1.57 ms |
| `read_plates` | 0.57 ms |
| `scatter_decor` | 0.02 ms |

The vocabulary is +36% on the derive; the scatter itself is free. Editor reflows
on a real build measured 0.15-0.28 ms including both, against 0.11-0.23 ms before
this work.

COLLIDERS, on the default 3-ship `wfc_ships` row: 438 plate colliders and 121
decoration colliders, so decoration adds 28% on top of the skin's own. No frame
time was isolated - the skin work already showed plate colliders sat inside the
run-to-run spread, and this is a quarter of that number again.

WHAT A HULL OFFERS, per ship on the row - the number that mattered most:

| | flat | step | ridge | peak | rim | plates |
| --- | --- | --- | --- | --- | --- | --- |
| seed 20260815 | 6 | 22 | 4 | 0 | 100 | 132 |
| seed 20260816 | 12 | 22 | 0 | 0 | 110 | 144 |
| seed 20260817 | 22 | 18 | 2 | 0 | 120 | 162 |

`spawn_ship_skin` logs that histogram and the per-fixture tally at debug, so the
Phase B agents tune against a measurement instead of a screenshot.

## Two findings the first placeholder cut produced

Both were found by rendering, and both are traps the four candidate looks will
walk into otherwise.

1. A RULE THAT READS AS SPECIFIC IS NOT. `near_fitting: Some(1)` sounds narrow.
   On a hull as full of drives and bays as the generator draws, nearly every
   plate has a pocket within one cell: first in the priority list it claimed 45%
   of every ship and the other three rules never got a plate. It is last now.
2. A RULE WRITTEN FOR FLAT PANELS LANDS NOWHERE. The first cut gated the vent
   and the trim on `Flat`; with 6-22 flat plates a ship, minus a stride and a
   share, the trim landed 1 piece on a 20-plate editor build. Trim now reads the
   BORDER of any relief, and dropped its stride - a lattice is for making a ROW,
   which is the vent's job; the end of a run is already sparse and already
   structured.

## The checkpoint render

`skin-style-row.png` (the row), `skin-style-row-detail.png` (one ship, cropped
and upscaled 2.5x so the greebles are legible), `skin-style-editor.png` (a
5-section editor build). `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 --features dev`, Xvfb
1920x1080.

Honest reading, on the row: the mechanism WORKS and the alignment is visible.
The ribbed vents on the flat roof panels all run the same way, the trim blocks
sit square at the ends of the runs, the blisters cluster round the fittings, and
every piece sits flush on the surface it stands on with none floating or sunk.
The difference between the first render (every piece at a free position, reading
as a sprinkle of specks) and this one is entirely the alignment and the lattice,
which is the research's claim reproduced.

Honest reading, on the editor build: 3 pieces on 20 plates, and two are on the
far side. A small build is nearly all one-cell rims and studs at quarter height,
so the vocabulary has very little to say about it. That is the resolution limit
the skin work already recorded, not a new defect, but it means a candidate look
must be judged on a GENERATED hull and not on a hand-built one.

## Verdict: can the vocabulary carry a good look?

Yes for placement, with one honest caveat.

What it can express today: pieces on the flat panels of a hull, aligned to the
panel; trim at the end of any run, aligned to the run; a field of pieces on a
lattice; something rare on the high ground; something clustered round the
fittings. That is enough to build four distinguishable looks, and it is enough
to graft one look's trim onto another's vents, because both speak in the same
readings.

What it cannot express, and what the next agent to touch this should know:

- `Rim` is one bucket over four fifths of every ship. `height`, `run`, `border`
  and `pocket` cut it four ways, and that is what makes it workable - but a look
  that wants to treat the OUTER silhouette differently from an inner edge cannot
  say so. The cheapest addition would be the in-plane direction the plate falls
  away toward (the gradient of the eight samples), which the derivation already
  has: it would let a fairing point off the ship and a rim strip know which side
  is outboard.
- There is no density normalisation. A rule tuned for a 150-plate hull starves a
  20-plate one, because every knob is per plate. A per-patch "at least one"
  would still be deterministic (a function of the whole structure) but it is not
  built.
- Decoration cannot span cells. Every piece is one plate, so a long radiator or
  a continuous rib run has to be faked by a row of aligned pieces on a stride.
  That may be enough - it is what the render shows - but it is a real ceiling.

None of those blocks Phase B. All three are worth knowing before four agents
spend their budget.

# Phase A widening: splitting the falling plate, and a density that scales

Landed on `skin-vocabulary-widen`, off `cf61373d`. Two of the three ceilings
above are closed; the third (a piece cannot span cells) is untouched and still
real.

## Ceiling 1: `Rim` was 80% of every ship

The suggestion above - the direction the plate falls away toward - turned out to
be TWO answers, and both are worth having.

First, the SPLIT. A corner sample dies to the cell floor for exactly one reason:
`ends_against` is false there, so open space stands at that corner. Structure
holds a corner up, and so does a fitting's pocket. So counting the dead corners
counts the directions a plate has vacuum in, and the old `Rim` divides three ways
with no new computation at all:

| relief | corners on the floor | what it is |
| --- | --- | --- |
| `Bevel` | 1 | a panel with a corner taken off - nearly a `Flat` |
| `Brink` | 2, adjacent | the surface falls along one whole side: the straight edge of a hull |
| `Spur` | 2 opposite, 3, or 4 | falls two ways or more: a tip, an outer corner, a saddle |

Second, the DIRECTION. Summing those dead-corner directions in the shape's own
frame and turning them by the plate's own rotation gives `PlateReading::fall`,
which is OUTBOARD by construction. The sum is what makes it worth having: a
`Brink`'s two adjacent corners add to the cardinal between them, and a saddle, a
`Ridge` and a `Peak` all cancel to zero rather than picking a side. `ScatterAlign`
grew from a bool to `Free` / `Run` / `Outward`, so a fairing can lean out over the
edge it stands on instead of only lying down it.

### The measurement

Three ships of the `wfc_ships` row, at the fixed seeds:

| plates | flat | step | ridge | peak | bevel | brink | spur | (was rim) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 132 | 6 | 22 | 4 | 0 | 10 | 48 | 42 | 100 |
| 144 | 12 | 22 | 0 | 0 | 14 | 62 | 34 | 110 |
| 162 | 22 | 18 | 2 | 0 | 12 | 66 | 42 | 120 |

The largest bucket went from 76-83% of a ship to 36-41%. The split is 45-55%
`Brink`, 35-42% `Spur`, 9-13% `Bevel` - not the 95/5 that would have made it
cosmetic.

VERDICT: it buys room, and the demonstration is the placeholder mast. It reads
the high ground, and the high ground of a generated hull is its tips and outer
corners - which under one `Rim` were indistinguishable from the middle of a
flank. `relief: [Ridge, Peak, Step, Spur]` is a sentence the old vocabulary could
not say.

What it did NOT buy: the task framed this as telling the outer silhouette from
an INNER EDGE around a fitting pocket, and that distinction does not exist in the
derivation. A pocket holds a corner UP (`ends_against` counts it), so the plates
round a well come out `Flat` and a rim always falls toward open space. That
question was already answered by `pocket`, and the answer is a distance, not a
relief. The split is about SHAPE - how many ways a plate falls - and it is
honestly that.

## Ceiling 2: density normalisation

`ScatterRule::patch`. Set it to `N` and the rule is guaranteed a piece in every
block of N cubed cells, keyed by the out face, that it can stand on at all.

- A FLOOR, never a cap. It only adds, and never onto a plate another rule
  claimed, so priority still means what it says.
- It drops the SHARE only. The filter and the lattice still hold, so a floor
  piece stands on the same grid the rest of the rule does - the lattice is what
  makes a row read as a row, and a normalisation that broke it would undo the
  checkpoint's own finding.
- `chance: 0.0` with a `patch` is the pure form: no share, exactly one piece per
  block. A density stated in cells of ship.

WHEN A HULL GROWS BY ONE CELL: every piece outside the block that cell lands in
is untouched, because the blocks are `div_euclid` of the ship's own cells and
nothing shifts. Inside that block, the floor's own pick can move - and only if
the new plate hashes lower than the incumbent. A piece the SHARE placed never
moves at all, anywhere, which is the property the old scatter had and this
keeps. So the claim is now a pure function of a BLOCK rather than of a cell,
bounded to `patch` cubed cells, and that is the whole of what was given up.

### The measurement, and the thing the floor cannot fix

The editor build has its own histogram now, and it is the most useful number
this lane produced:

| subject | plates | flat | step | ridge | peak | bevel | brink | spur |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| generated row | 132-162 | 6-22 | 18-22 | 0-4 | 0 | 10-14 | 48-66 | 34-42 |
| editor build | 19-27 | 0 | 2-3 | 3-8 | 1-3 | 0 | 0 | 9-17 |

A hand-built ship has NO flat plate, NO bevel and NO brink. It is one cell wide
nearly everywhere, so it is spurs, ridges and studs. The vent rule (which wants
flat plate) therefore cannot land on it at any density, and no floor rescues
that: a floor over an empty eligible set is still empty. The floor normalises
DENSITY; it does not widen a FILTER. That is worth knowing before a Phase B agent
spends a budget tuning `patch` on a rule that was never going to fire.

With the placeholder retuned around this, a hand-built ship went from 1-3 pieces
to 2-6 on 19-27 plates, and the row went from ~40 to ~50 pieces a ship.

## The two authoring traps

Both are now VISIBLE rather than documented-and-forgotten, which is the fix that
survives a rule being written at 2 a.m.

- Every fixture is logged as `taken of REACH`, where reach is everything the
  filter and the lattice admit BEFORE the share and before priority. `x0 of 78`
  is a starved rule; `x0 of 0` is an impossible one. They look identical on
  screen and have opposite fixes. `sync_editor_skin` logs the same line, so the
  build view - where the owner actually builds - is measured too.
- `near_fitting` counts FACE STEPS instead of rings, so `Some(1)` is the four
  cells beside a nozzle rather than the eight around it. That is what the field
  reads as, and it halves the carpet the old measure made.
- The trap generalises past `near_fitting`, and the log is what showed it:
  `max_border: Some(0)` - "only at the end of a run" - admitted 126 of 132
  plates, because on a broken-up hull nearly every plate is the end of its own
  one-cell run. Pairing it with `min_run: 2` took the placeholder's trim from 94
  eligible plates back to a rule that means what it says. Documented in
  `modding/styles.md` beside the `near_fitting` case.

## Rendered

`skin-vocabulary-row.png`, `skin-vocabulary-row-detail.png` (the same crop and
2.5x upscale as the checkpoint's, so the two are comparable),
`skin-vocabulary-editor.png`. Same harness as the checkpoint: `NOVA_AUTOPILOT=1
NOVA_CAPTURE=1 --features dev`, Xvfb 1920x1080.

Honest reading against `skin-style-row-detail.png`: the mechanism is unchanged
and still works - flush, aligned, on a lattice. What changed is visible but
small, because the placeholder kit is four garish magenta primitives: the trim
now covers the hull evenly instead of clumping (it is a density, not a share),
and there are thin masts standing on the outer corners of the silhouette, which
is the placement the split bought. Nothing regressed.

Honest reading on the editor build: still a faceted crystal, and still the
resolution limit the skin work recorded. 2-6 pieces instead of 1-3 is a real
improvement and is not a fix - a 5-part ship has almost nothing for a rule to
say about it, and the histogram above is why. A candidate look must still be
judged on a generated hull.

The editor shot is NOT strictly comparable to the checkpoint's: the `editor`
example's tower beat is a known pre-existing flake (recorded in
`20260815-190741`), so two runs photograph slightly different builds.

## Still open

Decoration cannot span cells. Untouched, and still a real ceiling: a long
radiator is a strided row of pieces. It is a mesh and placement change, not a
vocabulary one.

# Phase B candidate: ARMOURED

Landed on `look-armoured`, off `d1e82c39`. One of four rival looks; nothing here
touches the `placeholder` style, which stays as scaffolding.

## The point of view, in one sentence

**An armoured hull's feature is its EDGES**, and everything else stays flush. So
the kit spends nearly all of itself on one piece - a belt down every straight
edge the hull has - and the other three exist so that belt is not the only thing
on the ship.

## The kit: four pieces

| piece | what it is | where it lands | why |
| --- | --- | --- | --- |
| `armoured_strake` | two slabs: a narrow light rail on a wide dark base, 0.98 cells LONG | every `Brink` in a run of 2+, at full share, turned down the `Run` | the hero. The straight edge of a hull is its largest single bucket |
| `armoured_cap` | a 4-sided truncated pyramid on a dark lip | every other `Spur` (`stride: 2`) | the outer corners and spar tips, reinforced |
| `armoured_sensor` | octagonal ring, faceted hood, glossy optic | `Flat`/`Bevel`, one per 6-cell block (`chance: 0.0`, `patch: 6`) | the only piece allowed to break the plane |
| `armoured_hatch` | a light plate raised inside a dark surround, 0.042 cells tall | `Flat`/`Bevel`/`Step` on a stride | scale on a bare panel without a target on it |

One repeated idea holds the kit together: **every piece is a light body standing
on a dark base lip.** That is what makes a piece read as bolted on rather than
as a lump of the plate under it, and it is the same sentence at four sizes.

## A piece CAN span cells

`NOTES.md` above records "decoration cannot span cells" as an open ceiling. It is
payable with the only currency there is: a piece that fills its OWN cell.

`decor_pose` centres a greeble on its plate with no jitter, so a recipe that
raises its footprint budget to a whole cell meets its neighbour in the run. The
strake is 0.98 and not 1.0 - a hair of clearance so two neighbours never share a
coincident face - and a run of them comes out as ONE strake with visible section
joints rather than as a row of blocks. Verified at 5x zoom on the render.

It needs `chance: 1.0` and `stride: 1` to work: a share turns a line back into
dashes. That is the one rule in this style allowed to be a carpet, and it is the
whole look.

## What the palette can and cannot do - MEASURED

A diagnostic run painted `Wall` magenta and photographed the row
(`/tmp` only, not committed; reproduce by editing the generated RON, running the
example and regenerating).

- **A `Wall` is NOT the seam between two plates.** Two plates in a run press
  their walls together and neither is ever seen. What came back magenta was the
  OUTER RIM of the skin, plus the exposed side of a plate that climbs past a
  lower neighbour. About a twentieth of what a camera sees.
- So a big top-to-wall ratio does not draw a panel line on every step. It is
  still worth carrying: at 6:1 the rim sits at 30% against a 45% top on screen,
  and a hard dark rim is what makes the plate above it read thick.
- **The VALUE lever is weak and the HUE lever is strong.** Cutting the top's
  albedo 2.2x (linear 0.100 -> 0.045) moved a lit flank from 55% to 46% on
  screen, not the 45% the albedo says: the rig's ambient and the tonemapper put
  a floor under everything. Going darker again buys almost nothing.
- What the palette bought outright: the HUE (off the built-in blue slate onto
  desaturated matte gunmetal - a warship wants no specular highlight), and the
  SEPARATION between plate and greeble. The kit runs 3x the hull's albedo, so a
  belt is a light rail on a dark hull instead of a lump of the same stuff.

Honest split: materials are maybe a third of this look, not half. The belt is
the rest.

## `taken of reach`, on the three fixed seeds

| ship | plates | sensor | cap | strake | hatch |
| --- | --- | --- | --- | --- | --- |
| 162 plates | 22 flat / 66 brink / 42 spur | 6 of 24 | 8 of 8 | 46 of 46 | 11 of 15 |
| 144 plates | 12 flat / 62 brink / 34 spur | 9 of 20 | 8 of 8 | 54 of 54 | 8 of 11 |
| 132 plates | 6 flat / 48 brink / 42 spur | 6 of 12 | 9 of 9 | 44 of 44 | 5 of 7 |

71-79 pieces a ship, and 44-54 of them are the belt. No rule logs `x0`.

Two rules were fixed BY the log and could not have been fixed by a screenshot:

- `armoured_cap` first read `Spur` + `facing: Up` + `min_depth: 2` +
  `min_height: 1` + `stride: 2`, and logged `x0 of 0` and `x2 of 2`. A spur is
  the TIP of something, so there is rarely two cells of ship under it and it
  fills almost none of its own cell. Three filters that each read as mild,
  multiplied, made an impossible rule out of a bucket 42 plates deep. The
  facing came off last, because a corner is a corner from whatever side it is
  shot at and reinforcing only the top left the flanks bare.
- `armoured_sensor` ran `patch: 4` and put 7-12 blisters on a hull, which is a
  rash of the one piece that is supposed to be rare. 6 at `patch: 6`.

## Honest read of the render

`look-armoured-row.png`, `look-armoured-detail.png` (the same 560x420+1160+340
crop at 2.5x as `skin-vocabulary-row-detail.png`, so all four candidates compare
like for like).

**What works.** The belt does exactly what it was designed to do: long unbroken
light strakes trace the ship's straight edges and describe the silhouette, and
at 5x they are flush, with section joints that read as bolted armour rather than
as a defect. The hull is a cool gunmetal that reads as a warship rather than as
the placeholder's blue-white. The sensor blisters are legible as armoured plugs
and there are the right number of them - three or four in shot.

**What does not.** The corner bosses and the hatches are nearly invisible at row
distance; they are texture, not features, and a merge could drop the hatch
without losing anything. The bottom third of the flank carries nothing at all -
that is deliberate (a warship's flank IS plain armour) but it is exactly where
this brief tips into "bare hull", and a viewer who does not look at the edges
will see a grey lump. The look is carried by ONE idea. If the owner does not
buy the belt, there is nothing behind it.

**The specific failure mode of this brief - has restraint tipped into absence?**
Close, but no. The belt is unmistakably an authored decision and it is one no
other candidate can produce without the footprint trick above. But it is a thin
margin, and it is the honest answer.
# Phase B candidate: CIVILIAN

The racer's look, on `look-civilian` off `d1e82c39`. Style id `civilian`, first
in `style_catalog` because the producers take the FIRST authored style rather
than naming an id. The placeholder style is untouched.

## The look, in one sentence

A pale painted airframe with ONE continuous cobalt livery rail down every
straight edge of the hull, lit cabin windows, and everything mechanical hidden
under a smooth fairing.

## The kit: five pieces

| piece | what it is | where it lands | tris |
| --- | --- | --- | --- |
| `civilian_stripe` | the livery rail: pale fillet, cobalt face, white pinstripe | `Brink`, `min_run: 3`, no stride, no share, `Run` | 36 |
| `civilian_windows` | pale frame, dark glazing channel, three lit ports | `Flat`/`Bevel`/`Step`, `Side`, stride 2, `Run` | 120 |
| `civilian_fin` | a raked blade with a cobalt tip | `Spur`/`Ridge`/`Peak`, `Up`, `Outward` | 36 |
| `civilian_fairing` | a smooth 12-sided dome on a cobalt ring | `Flat`/`Bevel`/`Step`, stride 2, patch 3 | 144 |
| `civilian_beacon` | pale foot, dark collar, amber lens | `near_fitting: Some(1)`, LAST | 96 |

Five and not six: `civilian_cap` was authored, rendered, and cut. See the band
below.

## The palette, and how much it carries

Nearly all of it. LINEAR values.

| | colour | where |
| --- | --- | --- |
| hull | 0.420 0.420 0.400, rough 0.35, metallic 0.0 | plate `Top`, and four of five pieces |
| accent | 0.010 0.100 0.380 | the livery, the fin tip, the fairing ring |
| lamp | 0.890 0.342 0.027 | window ports, beacon lens |
| shadow | 0.012 0.013 0.016 | glazing channel, beacon collar |
| trim | 0.720 0.700 0.640 | the pinstripe down the middle of the band |
| wall | 0.058 0.060 0.068, rough 0.50, metallic 0.05 | plate `Wall` |

The top is ~0.68 sRGB against the built-in 0.36, at metallic 0. That single
change is what makes this look impossible to mistake for the unstyled hull:
PAINT rather than plate. The kit then spends exactly two accents - cobalt for
anything painted, amber for anything lit - and leaves the other four pieces
hull-coloured, so five pieces read as one livery instead of five props.

## The band, which is the whole look, and the trap that nearly killed it

Authored first as TWO rules over `Brink`, split by border: a rounded terminal on
`max_border: Some(0)` and the band on `min_border: 1`. The diagnostic answered
in one run:

```text
civilian_cap x38 of 38, civilian_stripe x0 of 0
```

`x0 of 0` - an impossible filter. `border` is `alike.iter().min()` over ALL FOUR
in-plane walks, and a hull edge always has open space on one side and unlike
plate on the other, so **a `Brink` reads border 0 always**. Only the middle of a
wide flat field can read more. So `min_border` is a rule for `Flat` and nothing
else, and the terminal rule silently took the entire edge: a row of 0.58-cell
pills with a 0.42-cell gap between each, which photographs as a dashed line.
That is a fifth authoring trap alongside the three already recorded, and it has
the same shape as the other two - a field that reads as narrow and is not.

The fix is the whole of the look: ONE rule, `Brink` + `min_run: 3`, no stride,
no share, `align: Run`, and a model whose footprint budget is raised to a WHOLE
cell (0.99 along `+Z`). A piece always stands at its plate's centre, so a
full-cell piece cannot spill onto a neighbour - and consecutive plates of a run
butt their bands together into one unbroken rail. 24-40 pieces a ship, and they
photograph as two or three lines rather than as 40 objects.

The rail is PROUD (0.082 cells) rather than paint-thin, deliberately: the plate
under it falls away across the band's width, so a decal-thin strip would bury
itself on the up side. Measured skew across 0.30 cells of fall is ~0.05, under
the rail's own height, so it never sinks.

## What a hull offers, and what each rule took

Fixed subject, default seeds, `taken of REACH`:

| ship | plates | windows | fin | stripe | fairing | beacon |
| --- | --- | --- | --- | --- | --- | --- |
| 162 | 162 | 4 of 4 | 4 of 8 | 38 of 38 | 8 of 15 | 6 of 19 |
| 144 | 144 | 4 of 6 | 4 of 4 | 40 of 40 | 7 of 11 | 6 of 12 |
| 132 | 132 | 4 of 4 | 6 of 8 | 24 of 24 | 2 of 7 | 5 of 10 |

No rule is starved and none is impossible. 60-62 pieces a ship against the
placeholder's ~50, so decoration colliders are up about a quarter - the band is
most of that, and it is the cheapest geometry in the kit at 36 triangles.

Probe run (temporary zero-chance fixtures, reach only), which is how the band
was sized:

| | 2 | 3 | 4 | 5 | 6 | 7 |
| --- | --- | --- | --- | --- | --- | --- |
| `Brink` by `min_run` | 44-54 | 24-40 | 0-32 | 0-12 | 0-12 | 0 |
| `Brink` by `min_enclosure` | 44-54 | 44-54 | 44-54 | 44-54 | 10-20 | 2-4 |

`min_enclosure` is the only lever that cuts a hull edge at all, and it cuts out
the MIDDLE, not the ends. There is no way to say "the end of an edge".

## Measured: what the `Wall` surface is worth

Painted `Wall` pure red and shot the row. It came back only as thin wedges at
the skin's OUTER RIM and on the lip around a fitting - roughly a twentieth of
what the camera sees. Two plates in a run press their walls together and neither
is ever seen.

So a dark wall under a pale top does NOT draw a panel line at every plate
boundary. It makes the silhouette read thick, like plate with depth, which is
worth having on a pale hull that would otherwise bleed into space - but it is a
SILHOUETTE effect, not a panelling one. Recorded in `modding/styles.md` so the
next author does not budget for it.

## Honest reading of the render

`look-civilian-row.png`, `look-civilian-detail.png` (the same 560x420+1160+340
crop at 2.5x as `skin-vocabulary-row-detail.png`, so all four candidates compare
like for like).

**Is it distinguishable from an unstyled hull? Yes, at a glance, and not
narrowly.** The unstyled derivation is a cool dark slate; this is a cream
airframe with two cobalt lines down it. Nobody would confuse the two.

What works: the band. On the two right-hand ships it runs unbroken the whole
length of the flank, and where two edges run parallel it comes out as a twin
pinstripe that looks authored rather than scattered. The cabin windows read
exactly as intended at row distance - a dark slot with three warm ports. The
fins read as small raked blades with a blue tip and give the top deck a
silhouette without greebling it.

What does not: **the leftmost ship wears no visible accent from this camera.**
It took 38 band pieces, all of them on edges facing away. A look whose whole
identity is one rule is at the mercy of which way that rule's relief happens to
face, and that is the honest cost of the discipline. A grafted version could put
a second, quieter accent on `Spur` (34-42 plates a ship, currently used only by
the rare fin) to cover that case - I authored exactly that as `civilian_cap` on
`Spur` + `min_enclosure: 4` and cut it, because at 16-22 pieces a ship it read
as blue confetti in the same colour as the band and diluted it. That is a taste
call and it is the first thing to revisit if the owner wants coverage over
discipline.

Also honest: the hull is BRIGHT, close to the top of what the tonemap will take.
It reads as clean, and it will read as washed out to anyone who wanted grit.
That is the brief, but it is worth naming.

The lower flanks are largely bare. That is deliberate - "smooth surfaces,
minimal greebles" - and it is the single thing most likely to be read as timid.

# Phase C: all four ship, and all four are reachable

The owner took every candidate, so the merge is not a graft - it is four kits
side by side in one catalog, on `styles-merge` off `44704438`. Four merge
commits keep each author's work and its render attached to its own commit; the
integration is one commit on top.

## What actually conflicted

`style_catalog` was the expected fight and it was the smallest one: every
candidate put itself FIRST, because `wfc_ships` and the editor take
`styles.first()`. Beyond it:

- `BaseContentAssets`, twice each (the fields and the constructor). Two kits
  named their fields `greeble_<kit>_<piece>` and two `<kit>_<piece>`; all are
  `greeble_` now, which is what groups them among the sounds and textures.
- `base.bundle.ron`, the greeble block. Sorted flat by prefix, one comment.
- `gltf/greebles/README.md` and `modding/styles.md`, both at the "what ships"
  paragraph, rewritten for four kits rather than stacked.
- `NOTES.md`, where two candidates had appended a section and two had not.
- `assets/base/styles/base.content.ron` on every merge. Never resolved by hand;
  `content gen` after the builders were merged, and `content lint` at 0.
- `CHANGELOG.md` did NOT conflict, but only because three of four skipped it.
  The armoured entry is replaced by one covering all four.

The tests merged the same way: every candidate's own assertions kept, the two
rival "my look leads the catalog" tests folded into one, and one new test that
no two styles and no two fixtures within a style share an id - an invariant that
did not exist while only one authored look shipped.

## The default order, and why

`industrial, armoured, civilian, salvage, placeholder`.

The head of the list is the FALLBACK - what a ship that named no style wears,
what the editor's build view shows before anything is picked, and what
`wfc_ships` photographs. So it should be the least opinionated thing in the
catalog. The other three are cast parts: the warship, the racer, the raider. A
ship nobody dressed is none of those; a working hull with its services on the
outside is what an arbitrary generated hull reads as. `industrial` is also the
widest kit - seven pieces over six zones of the vocabulary - so it has the most
to say about a hull nobody authored for it.

Scaffolding goes last for the reason every candidate gave: a placeholder at the
head photographs the test pattern.

**The honest counter-argument**, since the measurement below cuts against the
choice: `industrial` is now by far the densest look and the one that drifted
most on the new skin. If the owner wants a quieter default, `armoured` is the
swap and it is one line in `style_catalog`.

## The drift onto the new skin

All four were authored against `d1e82c39`. `44704438` stopped fittings punching
holes in the cladding, and the subject moved under every one of them.

| seed | plates then | plates now | flat | step | ridge | bevel | brink | spur |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 20260815 | 132 | 164 | 6 -> 38 | 22 -> 12 | 4 -> 0 | 10 -> 6 | 48 -> 76 | 42 -> 32 |
| 20260816 | 144 | 158 | 12 -> 20 | 22 -> 52 | 0 -> 2 | 14 -> 4 | 62 -> 48 | 34 -> 32 |
| 20260817 | 162 | 204 | 22 -> 18 | 18 -> 48 | 2 -> 4 | 12 -> 8 | 66 -> 50 | 42 -> 76 |

438 plates across the row became 526, which is +20%. The BUCKETS moved far more
than that and not in one direction: `flat` went 6 -> 38 on one seed and 22 -> 18
on another, `step` more than doubled on two, and `brink` and `spur` traded 26
plates each way. So no style's densities moved by a single factor, and no rule
can be corrected by one.

### `taken of reach`, per style, against what its author reported

Nothing below is retuned. This is what the four kits do on the merged tree.

INDUSTRIAL. Reported "60-87 pieces a ship". Now 141-168, roughly DOUBLE, on a
row only 20% bigger.

| seed | stack | hazard_band | radiator | duct | louvre | hatch | ribbing | total |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 20260815 | 2 of 6 | 54 of 54 | 6 of 6 | 31 of 36 | 1 of 3 | 14 of 48 | 39 of 164 | 147 |
| 20260816 | 1 of 6 | 22 of 22 | 6 of 6 | 56 of 60 | 3 of 7 | 16 of 41 | 37 of 158 | 141 |
| 20260817 | 7 of 14 | 36 of 36 | 5 of 5 | 37 of 40 | 2 of 3 | 17 of 53 | 64 of 204 | 168 |

The duct is the drift: `relief: [Step, Flat]` at full share, and `step` more than
doubled on two seeds. The band is second - it takes every `Brink` run it can
reach, and reach is now up to 54. Nothing is starved and nothing is impossible.

ARMOURED. Reported 71-79 a ship. Now 74-107, and the shape of the look holds.

| seed | sensor | cap | strake | hatch | total (was) |
| --- | --- | --- | --- | --- | --- |
| 20260815 | 14 of 40 (was 6 of 12) | 8 of 8 (was 9 of 9) | 74 of 74 (was 44) | 11 of 13 (was 5 of 7) | 107 (64) |
| 20260816 | 10 of 22 (was 9 of 20) | 13 of 13 (was 8 of 8) | 42 of 42 (was 54) | 9 of 13 (was 8 of 11) | 74 (79) |
| 20260817 | 8 of 22 (was 6 of 24) | 21 of 21 (was 8 of 8) | 46 of 46 (was 46) | 12 of 18 (was 11 of 15) | 87 (71) |

The belt tracks `brink` exactly, so it went both ways: +68% on one seed and -22%
on another. The corner boss nearly tripled on the biggest hull (8 -> 21). This is
the look that survived the change best.

CIVILIAN. Reported 60-62 a ship. Now 58-75.

| seed | windows | fin | stripe | fairing | beacon |
| --- | --- | --- | --- | --- | --- |
| 20260815 | 4 of 9 (was 4 of 4) | 4 of 6 (was 6 of 8) | 54 of 54 (was 24) | 9 of 13 (was 2 of 7) | 1 of 3 (was 5 of 10) |
| 20260816 | 5 of 6 (was 4 of 6) | 5 of 6 (was 4 of 4) | 22 of 22 (was 40) | 6 of 13 (was 7 of 11) | 6 of 7 (was 6 of 12) |
| 20260817 | 5 of 5 (was 4 of 4) | 5 of 10 (was 4 of 8) | 36 of 36 (was 38) | 11 of 18 (was 8 of 15) | 2 of 3 (was 6 of 19) |

SALVAGE. The author published no per-fixture table, so there is nothing to
compare against; this is the baseline for next time.

| seed | whip | drum | hook | weld_seam | patch_strip | patch_plate | patch_scab |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 20260815 | 3 of 6 | 11 of 48 | 0 of 3 | 54 of 54 | 2 of 10 | 26 of 132 | 20 of 164 |
| 20260816 | 3 of 6 | 17 of 72 | 1 of 7 | 22 of 22 | 10 of 26 | 32 of 124 | 22 of 158 |
| 20260817 | 5 of 14 | 18 of 60 | 1 of 3 | 36 of 36 | 3 of 10 | 28 of 124 | 23 of 192 |

### The one rule that now lands somewhere absurd

**`near_fitting` lost most of its reach**, and it is the same cause in three
kits. `44704438` narrowed a "fitting" from every outward face of a part to the
one face it FIRES through, so the pocket distance is measured to far fewer
faces. Every rule written as `near_fitting: Some(1)` reads narrower than its
author measured it:

| rule | reach then | reach now | taken now |
| --- | --- | --- | --- |
| `civilian_beacon` | 10-19 | 3-7 | 1-6 |
| `salvage_hook` | not reported | 3-7 | 0-1 |

`salvage_hook x0 of 3` on seed 20260815 is a rule that is now starved rather
than impossible, and the tow cleat is effectively absent from the salvage look
on a third of the row. The civilian beacon is down to one piece on the same
ship. Neither is fixed here: the fix is a wider `near_fitting` or a second
reading, and it is the owner's call whether that is a retune or a vocabulary
change.

Note also that `near_fitting` is now much LESS of a trap than the docs warn -
the "sounds narrow, carpets a ship" case that put the placeholder blister last
in its list no longer holds on this skin.

## Reachability

`wfc_ships` binds `L` (not `S`: the free-fly camera owns that) to step the
active style over the merged catalog on the SAME seeds, and turns the cladding
back on with it, since a look is invisible on a bare hull. The readout names the
current style. `--style <id>` picks one for a capture run and PANICS on an id
nothing authored, because a typo would otherwise photograph the first look and
read as the one that was asked for. The index is taken modulo the catalog, so a
mod's fifth look joins the rotation with nothing in the example changing.

The editor got a LIST, not a cycle, which is what the owner asked for
mid-flight: every merged style as a row under the cladding toggle, shown only
while the skin is on, carrying the shared `Selected` mark. Both the rail and the
build view spell the same fallback - a ship that has picked no style wears the
first - so the marked row is what is on screen.

The rows are 22px against a tool button's 34, and that was found by rendering
rather than by taste: at tool height five looks push Play off the bottom of a
1024x768 window. Confirmed on `merged-editor-rail.png`.

## Rendered

`merged-industrial-row.png`, `merged-armoured-row.png`, `merged-civilian-row.png`
and `merged-salvage-row.png` - the same three seeds, the same pose, the same rig,
one run each with `--style`, and the style NAMED in the title line of every
frame. `merged-editor-rail.png` is the build view with the look list up.

Honest reading:

- INDUSTRIAL is unmistakably a working hull and unmistakably the busiest. The
  yellow runs every straight edge and every ridge line, and on the upper decks
  it is close to a carpet - at double the density its author measured, the
  restraint that made the paint an accent has thinned. Still legible, and the
  alignment holds, but it is the look most changed by the new skin.
- ARMOURED reads exactly as its author described: light strakes down the edges
  of a dark gunmetal hull, small bosses at the corners, flush hatches, one
  blister. The belt is continuous with visible section joints, not a dashed row.
  The lower flank is bare, which is the brief and is still the thin margin.
- CIVILIAN is the cleanest and the brightest - close to the top of the tonemap,
  as its author warned. The cobalt rails read as thin bright lines at row
  distance rather than as blue; the amber beacons are nearly invisible, and the
  beacon starvation above is why.
- SALVAGE reads as repaired: cool grey patches over a dark warm hull, weld beads
  down the long edges, orange drums lashed to the flanks, whips on the high
  ground. It is the look whose IDENTITY depends least on the rules that drifted.

All four are clearly four different ships at row distance, which is the thing
that had to be true.

## Not done

No scenario or campaign assigns a style to a ship yet. The cast mapping the task
sketched - raiders in `salvage`, the racer in `civilian` - is content authoring
and is untouched here.
