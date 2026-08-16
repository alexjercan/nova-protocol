# Standing a decoration up on the plate it stands on

## What was built

1. **`ShellShape::top_normal`** - the unit normal of the largest flat piece of a
   plate's top, which is the same piece `flat_area` measures. `tilt` is now
   `acos(top_normal . Y)` rather than a second walk over the facets.
2. **`ShellShape::seat_normal`** - what a DECORATION is stood up on: the top's
   own plane where the top is one plane, and the cell's out axis where it is a
   cone. The split is the whole of what a flat-bottomed model can be told about
   a plate, and it is argued below.
3. **`decor_pose`** beds the piece onto that normal:
   `from_rotation_arc(Y, seat_normal) * from_rotation_y(quarter(turns))`. The
   yaw stays INSIDE the bedding, because the quarter turn was chosen against the
   plate's in-plane axes and turning first keeps that choice meaning what it
   said.
4. **`ScatterSeat` / `ScatterRule::seat`** - `Whole` (the default) or `Any`.
   `Whole` takes only a plate whose top is one surface; `Any` is the exception.
5. **`PlateReading::coplanar`** - the seat, in the vocabulary, so a rule asks
   `read_plates` for it like it asks for the relief.
6. **`SkinReport::decor_tilt`** and the snapshot's `decor.tilt` - the mean tilt
   of the plates carrying a piece, in radians, in the summary line and in the
   dump.
7. The base kits: five rules opt out with `seat: Any`, and every seated rule
   dropped the reliefs that can never carry a seat. Pinned by
   `only_the_high_ground_opts_out_of_the_seat` and
   `no_seated_rule_names_a_relief_that_is_always_a_cone`.

Nothing here touches geometry. The plate interior, the interpolant,
`ShellShape::surfaces`, the sample alphabet and the canonicalisation are exactly
as they were, and the measurement proves it: 526 plates and 308 coplanar before
and after, to the plate.

## The numbers

`wfc_ships`, three hulls, default seed 20260815, industrial, frame 35, from
`NOVA_PERF_SNAPSHOT`. The last row is not in the dump - it is computed off the
per-plate records, because it is the one number that changed meaning.

| the row, 526 plates | before | after |
| --- | --- | --- |
| plates / coplanar | 526 / 308 (58.6%) | 526 / 308 (58.6%) |
| mean plate tilt | 0.4501 rad | 0.4501 rad |
| pieces placed | 456 | **302** |
| `off_flat` | 388 (85.1%) | **234 (77.5%)** |
| `on_creased` | 169 (37.1%) | **10 (3.3%)** |
| mean tilt under a piece | 0.4515 rad (25.9 deg) | **0.4032 rad (23.1 deg)** |
| mean LEAN of a piece off its own footing | 0.4515 rad (25.9 deg) | **0.0188 rad (1.1 deg)** |

The last two rows are the honest pair, and the fourth one on its own would be
misleading. **Mean tilt under a piece barely moves**, and it cannot: the seat
gate does not prefer LEVEL ground, it prefers UNBROKEN ground, and the biggest
seated bucket on a hull is `Brink` - the straight edge, one flat surface raked
27 degrees. What collapses is the piece's own lean off the surface it stands on,
from 25.9 degrees to 1.1, because every piece on a surface is now flush with it
and only the ten high-ground pieces stand up a cone.

Per rule, summed over the three hulls (`taken`, and `reach` for the filler):

| rule | before | after |
| --- | --- | --- |
| `industrial_stack` (`seat: Any`) | 10 | 10 |
| `industrial_hazard_band` (`Brink`) | 112 | 112 |
| `industrial_radiator` (`Flat`, was `Flat`+`Bevel`) | 17 | 14 |
| `industrial_duct` (`Step`+`Flat`) | 124 | 96 |
| `industrial_louvre` (pocket) | 6 | 3 |
| `industrial_hatch` (filler) | 47 | 18 |
| `industrial_ribbing` (filler, reach 526 -> 308) | 140 | 49 |

The two rules that lose nothing are the two that were already asking for a
surface without knowing it - the band on `Brink` and the stack on the high
ground. **The cost is carried by the FILLERS**, and it is a third of the
decoration on the row. That is the trade the gate makes, and it is visible in
`decor-row-*-detail.png`: the raked flanks that used to carry a ribbing panel
across a crease are bare now.

## The L, which is the interesting case

The owner's five-cube L, spawned through `wfc_ships` (a scratch hull swap, run
and reverted - it is not in the commit):

| the L, 20 plates | before | after |
| --- | --- | --- |
| coplanar | 0 (0%) | 0 (0%) |
| pieces | 4 | **1** |
| on a creased top | 4 (100%) | 1 (100%) |
| mean tilt under a piece | 0.4549 rad | 0.7854 rad |

`decor-l-before.png` against `decor-l-after.png`. Before: three
`industrial_ribbing` panels - flat plates 0.46 x 0.42 of a cell - standing
upright on three of the L's spikes and jutting off the hull sideways, each
touching its plate at one point. That IS the owner's "they look weird",
photographed. After: they are gone, and the L keeps the one piece that was ever
meant to be there, the stack on the top peak.

**What a coplanar gate does when nothing is coplanar: it takes three quarters of
the decoration off, and all three of the pieces it takes were the defect.** The
L is not made bare BY the gate - it was already down to four pieces, because
`min_depth: 2` keeps the fillers off almost all of a hull one cell thick. So the
gate is not the argument against itself I was told to look for here. The
argument it does make is the next task's: a hand-built hull has NO seat anywhere
on it, so no amount of rule authoring puts a well-bedded piece on one. Only the
interior work can create one.

## Why a cone is left standing up its own cell

The obvious reading of "orient every decoration to its plate's top normal" is to
bed everything onto `top_normal`, cone included, since the widest facet pair is
where a piece has the most material under it. That is refused, and the refusal
is `seat_normal`:

- a cone's middle is its APEX, and the piece stands at the middle. Leaning it
  onto one facet pair tips it 45 degrees off the tip of a spar and reads as a
  mast blown over - worse than the upright it replaces;
- a cone is symmetric about the out axis, so standing up it is the one answer
  that does not pick a side;
- and the pieces that stand on cones are, by the gate, exactly the five that
  ASKED for the high ground. A stack on a spur tip wants to stand up the tip.

So the bedding is applied where it is exact and nowhere else, and the crease is
answered by refusing it rather than by leaning into it.

## The exception path, and what it is spent on

Five rules carry `seat: Any`, one per kit plus the placeholder:
`industrial_stack`, `armoured_cap`, `civilian_fin`, `salvage_whip`,
`placeholder_mast`. Each is a piece whose whole purpose is the pointiest thing
on the hull, and every one of them would otherwise land NOTHING:
`armoured_cap` reads `relief: [Spur]`, and a spur falls two ways by definition,
so it is a cone every time. A blanket "surfaces only" would have deleted the
corner boss, the fin, the whip and the mast from the game and taken the
silhouette off every ship to fix a bedding defect.

`ScatterSeat` has two variants and not three. A `Crease` ("only the high
ground") has no user: every piece that wants a cone already names the reliefs
that are cones, so a third variant would be machinery with nothing behind it.

## Retiring the relief rules the predicate replaces

The rule applied, and now pinned by a test: **a relief list names a ZONE of the
hull; the seat says whether there is something to lie on.** A relief that can
never be coplanar (`Bevel`, `Ridge`, `Peak`, `Spur`) is dead weight in a seated
rule, so it comes off:

- `industrial_radiator`, `armoured_sensor`, `placeholder_vent`: `Flat`+`Bevel`
  -> `Flat`;
- `armoured_hatch`, `civilian_windows`, `civilian_fairing`, `salvage_drum`:
  `Flat`+`Bevel`+`Step` -> `Flat`+`Step`;
- `salvage_patch_plate`: the whole list goes. `Flat`+`Bevel`+`Brink`+`Step` was
  the predicate spelled badly - an enumeration of "the reliefs with a broad top"
  written before there was a way to ask - and the seat says it exactly and says
  it NARROWER: 308 of 526 plates against the 380 that list admitted.

The `Brink` rules (band, strake, stripe, seam) and the high-ground lists keep
their reliefs: those name a place, not a seat.

`R8` from the research (a graded seat size in the style schema) stays dropped.
Flat area is bimodal at exactly 1.0 or exactly 0.25 on every hull measured, so a
graded knob would be this boolean with extra steps.

## What surprised me

- **`decor_off_flat` is a weak instrument.** It counts pieces whose relief is
  not `Flat`, and 77.5% of the pieces are still "off flat" after every one of
  them is bedded on a real surface - because `Brink` is not `Flat` and is a
  perfectly good seat. The number that reads true is `on_creased`, 169 -> 10.
- **The measured lean and the reported tilt point in opposite directions on the
  L**: mean tilt under a piece went UP, 0.4549 -> 0.7854 rad, because the one
  surviving piece stands on the sharpest thing on the ship on purpose. A mean
  over one deliberate piece is not a defect reading.
- The filler's `reach` is now exactly the hull's coplanar count (164/158/204 ->
  118/100/90). That is a nice property to have by accident: an unfiltered rule's
  reach IS the seated hull.

## What I would do next

- The gate costs a third of the decoration on a generated hull, all of it
  filler. If the row reads too bare to the owner, the lever is the fillers'
  `chance`, not the gate - `industrial_ribbing` at 0.45 over a hull that is now
  58.6% eligible instead of 100% is the same rule asking for half as much.
- The hand-built hull is now measurably a GEOMETRY problem and nothing else:
  0 seats on 20 plates. `20260816-112429` owns it.
