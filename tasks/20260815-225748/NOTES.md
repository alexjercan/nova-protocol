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
