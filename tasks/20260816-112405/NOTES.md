# Skin debug dump: what was built, and what it measured

## What was built

A THIRD pass over a derived skin, `nova_ship::sections::skin_report`, and a
serialization of it in the state-snapshot capability. Not a second serializer:
it takes `derive_skin`'s plates and `read_plates`'s readings as arguments, so it
cannot disagree with the skin it describes.

- `skin_report(structure, plates, readings, style) -> SkinReport` - per clad
  cell, per BARE cell, and the statistics.
- `skin_summary(&report) -> String` - one line a human reads. Appended to the
  `spawn_ship_skin` debug log, beside the relief and decoration tallies.
- `ShellShape::is_coplanar`, `flat_area`, `tilt`, `is_stud` - the measurement of
  a plate top, on the shape itself where the next lane needs it.
- `is_diagonal_saddle`, `fallen_corners`, `relief_of`, `RELIEFS`,
  `PlateRelief::name` - made public; the saddle mask is now named rather than
  buried in a match arm.
- `DecorPlacement::reason` (`Rule` / `Patch`) - which of the two passes claimed
  a plate. New field, no back-compat shim.
- `section_cell` made public, so a reader can put a live plate back on the
  lattice the derivation used.

### Snapshot schema additions

`SNAPSHOT_SCHEMA` is NOT bumped: every key is new and no existing key changed
meaning.

- `ships[].skin` - null for a ship wearing none. `plates`, `shapes`, `relief`
  histogram, `coplanar`, `saddles`, `leaning`, `flat_area`, `bare_faces`,
  `bare[]` (`cell`, `reason`, `faces`), `decor` (`placed`, `off_flat`,
  `on_creased`, `relief`, `rules[]` as `id`/`taken`/`reach`), and `summary`.
- `...fixtures[].plate` on a skin plate - `cell`, `anchor`, `out`, `corners`,
  `midpoints`, `relief`, `fallen`, `saddle`, `coplanar`, `flat_area`, `tilt`,
  `along`, `fall`, `run`, `border`, `enclosure`, `height`, `depth`, `fitting`,
  and `decor[]` (`id`, `reason`, `turns`).
- `...fixtures[].stands_on` on a decoration - the plate under it: `cell`,
  `relief`, `coplanar`, `flat_area`, `tilt`.

The dump RE-DERIVES from the ship's live sections rather than reading the plates
back off the ship. It can, because the skin is a pure function of structure, and
it has to: a cell that carries NO plate has no entity to hang a record on, and
"why is this hull face bare" is the question the dump was asked for.

Determinism is kept by the same three rules the snapshot already had: fixed
insertion order in code, every float through `num()`, and no set iteration order
in the output (the bare cells are sorted by cell; the plates arrive sorted from
`derive_skin`).

## The owner's L - reproduced, explained, and his hypothesis refuted

Screenshot `20260816_132031.png`: three cubes in a run, two more up off the
right-hand end, arms of three. One section at the inside angle is unclad; every
plate on the hull reads as a pyramid.

Reproduced exactly as a five-cell structure, pinned in
`the_inside_angle_of_an_l_is_crowded_out_and_the_spikes_are_not_why`.

**The bare corner: `crowded`.** The inside-angle cell (1,1,0) can bolt to
structure two ways - down onto the run, and sideways onto the upright - but a
plate must also SHOW VACUUM through its out face, and with arms of three both of
those directions are already held by another plate. `cladding_cells` drops it,
and the two hull faces that look into the angle are then structure facing space.
This was `no_footing` in the first cut of the dump; it earned its own reason,
`Crowded`, because the fix is a different one from a cell with nothing to bolt
to.

**The owner's hypothesis - that the spikes cause it - is refuted, in both
directions:**

| Build | flat-topped | mean flat area | bare cells |
| --- | --- | --- | --- |
| elbow, arms of 2 | 1 of 13 (8%) | 0.308 | **0** |
| the owner's L, arms of 3 | 0 of 20 (0%) | 0.250 | **1** (crowded) |
| the same L, 3 cells thick | 10 of 40 (25%) | 0.438 | **3** (crowded) |

Arms of two are just as spiky with nothing bare; thickening the L takes spikes
OFF and leaves MORE bare cells, one per slice. They are two independent readings
of one property - the L has no interior. Structurally: `cladding_cells` never
reads a height. It runs to completion before `boundary_heights` is called at
all, so a shape cannot decide which cells are clad.

**The L's shape numbers** (20 plates, 4 shapes): spur x10, ridge x6, peak x4.
Zero flat, zero brink, zero step, zero bevel. Zero plates coplanar - every one
is a cone. Mean flat area 0.25, which is the floor of the metric. Zero saddles.
Every plate has all four corners on the floor (`fallen == 0b1111`): a one-cell
hull is all rim, so no corner anywhere has four cells carrying the surface on.

## The statistics, measured on a real run

`wfc_ships`, three hulls, default seed 20260815, industrial style, frame 35.
Real dump, read back with `jq`-equivalent. 526 plates.

| Relief | Plates | Share | Coplanar |
| --- | --- | --- | --- |
| brink | 174 | 33.1% | 174 (100%) |
| spur | 140 | 26.6% | 0 |
| step | 112 | 21.3% | 58 (52%) |
| flat | 76 | 14.4% | 76 (100%) |
| bevel | 18 | 3.4% | 0 |
| ridge | 6 | 1.1% | 0 |
| peak | 0 | 0% | - |

- **Creased plates: 218 of 526, 41.4%.** That is the size of the interior
  problem, and it is smaller than the relief classes suggest: the "broken set"
  (step + ridge + bevel + spur) is 276 plates / 52.5%, but 58 of the steps are
  clean ramps.
- **The relief class does not decide it.** `Flat` and `Brink` are surfaces every
  time, `Bevel`, `Ridge` and `Spur` are cones every time, and `Step` splits
  roughly in half - square-on to a raise it is a ramp, on the diagonal it is a
  cone. A case table keyed on relief alone would be wrong about a fifth of a
  hull. Pinned in `the_creased_classes_are_bevel_ridge_and_spur_and_half_the_steps`.
- **Diagonal saddles: ZERO of 526.** The mask `0b0101` occurs twice on the row,
  and both plates carry a sample at the whole cell, so `relief_of` classes them
  `Step` before the fallen mask is ever read. R1b's falsifier does not fire (it
  starves nothing) but neither does its premise: splitting the saddle out of
  `Spur` and refusing to decorate it would isolate NOTHING on a generated hull,
  and nothing on the owner's L either.
- **Mean flat area: 0.689 cell^2**, and it is BIMODAL - exactly 1.0 (308 plates)
  or exactly 0.25 (218), nothing between, on the generated hulls and on every
  hand-built structure tested. A plate is one whole surface, or a cone with one
  pair of facets usable. There is no middle for R8's seat size to tune against.
- **Decoration off flat: 388 of 456, 85.1%.** But only 169 (37.1%) stand on a
  CREASED top. The rest stand on `Brink` (161) and clean `Step` (58), which are
  flat surfaces that LEAN.
- **Leaning is the bigger placement defect than creasing.** 430 of 526 plates
  (81.7%) lean more than 15 degrees off their own out face, and only 68 of 456
  placed pieces stand on a level surface. Mean tilt under a piece: 0.452 rad,
  26 degrees. `decor_pose` lifts and yaws; it never tilts.
- **Bare hull faces: 142 across three hulls**, over 56 `fires_into` cells (by
  design - muzzles and nozzle mouths), 48 `no_socket` (fitting flanks), and
  **12 `crowded`** - the owner's inside-angle defect, twelve times, on a
  generated row nobody had looked at that closely.

Shape concentration, which bounds the work: six shapes cover 444 of 526 plates
(84%), and the single commonest creased shape is `shell_0002_0011` at 120 plates
(23% of the row) - one corner at half a cell with three on the floor.

## A real excerpt

```json
{"kind":"skin_plate","name":"Skin Plate shell_0002_0011","attached_to":"port_0_0_0",
 "shape":"shell_0002_0011","alive":true,
 "plate":{"cell":[-2,-2,-5],"anchor":[-1,-2,-5],"out":[-1,0,0],
   "corners":[0,0,0,2],"midpoints":[0,0,1,1],"relief":"spur","fallen":7,
   "saddle":false,"coplanar":false,"flat_area":0.25,"tilt":0.5097,
   "along":[0,0,1],"fall":[0,-1,-1],"run":1,"border":0,"enclosure":3,
   "height":1,"depth":2,"fitting":3,
   "decor":[{"id":"industrial_ribbing","reason":"rule","turns":1}]}}
```

and the ship-level half:

```json
"skin":{"plates":164,"shapes":9,"coplanar":118,"saddles":0,"leaning":126,
  "flat_area":0.7896,"bare_faces":32,
  "relief":{"bevel":6,"brink":76,"flat":38,"peak":0,"ridge":0,"spur":32,"step":12},
  "bare":[{"cell":[-3,-2,5],"faces":1,"reason":"no_socket"}, ...],
  "decor":{"placed":147,"off_flat":113,"on_creased":38,
    "rules":[{"id":"industrial_hazard_band","taken":54,"reach":54},
             {"id":"industrial_hatch","taken":14,"reach":48}, ...]},
  "summary":"164 plate(s): 118 flat-topped (72%), 0 saddle(s) (0%), 126 leaning; \
mean flat area 0.790 cell^2; 32 bare hull face(s) over 32 bare cell(s); \
147 piece(s), 113 off flat (77%), 38 on a creased top"}
```

## Determinism proof

`NOVA_PERF_SNAPSHOT_FRAMES=35,35,40,40,45,45` on a live `wfc_ships` run: three
pairs of captures of one frozen frame, each pair byte-identical (627 KB lines).
The skin half is identical across DIFFERENT frames too - only the poses drift -
which is the stronger claim, since the structure does not change.

## Two things the dump corrected while it was being written

Both were assumptions written into a test and then refuted by running it, which
is the loop the dump exists to close.

1. "The plates beside a proud fitting are raised." They are NOT. A fitting
   offers nothing to mate on its flanks, so `walls()` is false and the skin runs
   PAST it at its own height, level and flat. What raises the deck is structure
   that offers a socket in the skin's plane - a hull block standing proud - and
   even then only its four DIAGONAL neighbours come out creased; the edge
   neighbours climb it as one clean ramp.
2. "The muzzle cell is bare because nothing fires into it." A part may not carry
   a socket on the face it fires through, so a muzzle cell fails BOTH the socket
   test and the exit test. `cladding_cells` applies the socket test first, so
   the literal answer would be `no_socket` for every gun well on the ship. The
   dump reads the exit FIRST, deliberately, because that is the actionable half.

## What the numbers say about the shell-shape plan

- **R1b (split the saddle, refuse to decorate it) should be dropped or
  requeued.** Zero saddles on 526 generated plates and zero on the owner's L.
  It is the cheapest lever in the research document and it moves nothing.
- **R2/R3 (fix the tent, fix the saddle) are small.** Ridge is 6 plates of 526
  (1.1%) and the saddle is 0. They are worth doing because they are exact
  interpolants and free, not because of how much hull they cover.
- **The mass is `Spur` (140, 26.6%) and the creased half of `Step` (54,
  10.3%).** Those two are 89% of every creased plate on the row. Any case table
  that does not answer the spur tip has not touched the problem.
- **R5 (`decor_pose`) should be widened, not just kept.** The measured placement
  defect is not mainly the crease - 63% of placed pieces stand on a plate whose
  top IS one surface. It is the LEAN: 26 degrees mean under a piece, and only 15%
  of pieces on level ground. A piece could be oriented to its plate's own top
  normal today, before any interpolant changes.
- **R8 (seat size in the style schema) has nothing to grade.** Flat area is
  binary at 1.0 or 0.25. Until the interior changes, a seat-size knob is a
  coplanarity flag with extra steps.
- **`Crowded` is a defect the shape work will not fix.** Twelve on the generated
  row, one per slice on the owner's L, and it lives entirely in
  `cladding_cells`. It wants its own task: the inside angle of an L needs a
  tie-break that leaves one of the three competing cells clad.
