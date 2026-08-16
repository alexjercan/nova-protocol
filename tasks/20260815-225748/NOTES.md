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
