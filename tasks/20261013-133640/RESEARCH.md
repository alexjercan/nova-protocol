# Research: a clearer greeble placement language

Task: `tasks/20261013-133640/TASK.md`. Consumer: `tasks/20261013-133643/TASK.md`
(plating reshapes to fit the greeble). Research only. No code changed.
Diagrams: `RESEARCH.html` in this folder.

Labels: **[code]** read from the source at `7c2a4ce17`. **[log]** read from a
run log. **[doc]** read from a committed doc. **[web]** checked on the web
during this research. **[knowledge]** from memory, not checked. **[unverified]**
is a claim that nobody measured.

## 1. How placement works today

### Pipeline [code]

```
derive_skin(structure)            shell_skin.rs:263   plates (shape + rotation)
read_plates(structure, plates)    skin_reading.rs:300 one PlateReading per plate
scatter_decor(plates, readings)   skin_decor.rs:129   <= FixturePlacement acts here
  claims(): filter -> lattice -> hash share          skin_decor.rs:237-249
  fill_patches(): per-block floor + borrowing        skin_decor.rs:309-390
  turns_for(): quarter turns                         skin_decor.rs:456-479
decor_pose(plate, turns)          skin_decor.rs:530   bed on seat_normal, then yaw
spawn decor as ChildOf(plate)     shell_skin.rs:979-985
```

The plates are final before the scatter runs. Nothing flows back from a greeble
to its plate. Task 133643 must change this order.

An author writes three words. `StyleFixtureConfig::scatter` expands them into
a private `ScatterRule` (`skin_style.rs:291-305`) with ten fields: `relief`,
`whole_seat`, `facing`, `min_run`, `min_height`, `min_depth`, `near_fitting`,
`stride`, `chance` and `patch` (`skin_style.rs:465-505`).

### `region` [code]

`FixtureRegion::rule` (`skin_style.rs:370-422`) maps each word to a relief list
and gates. The relief is `relief_of(&plate.shape)` (`skin_reading.rs:492-511`):
`Flat` if no edge falls and no corner is capped; any other whole-cell plate is
`Step`; a half-cell plate is `Brink` (1 edge falls), `Ridge` (2 opposite),
`Peak` (4) or `Spur` (2 adjacent or 3).

| region | relief admitted | other gates |
|---|---|---|
| `Panel` | Flat | seat, `min_height` 1 |
| `Deck` | Flat, Step | seat, `min_height` 1, `min_depth` 2, ANY facing |
| `Flank` | Flat, Step | seat, `min_height` 1, `min_depth` 2, facing Side |
| `Edge` | Brink | seat |
| `HighGround` | Ridge, Peak, Spur, **Step** | NO seat gate, facing Up, `min_depth` 1 |
| `NearFitting` | any | seat, fitting distance <= 1 face step |
| `Anywhere` | any | seat |

Two more gates come from the collider, not from any word
(`skin_style.rs:319-339`): `min_run = ceil(max(x, z) * 2)`, capped at 3, and
`needs_seat = collider.y >= TRIM_HEIGHT (0.1)`. A piece under 0.1 cells tall is
"trim" and skips the seat gate in every region.

### `density` [code]

`FixtureDensity::rule` (`skin_style.rs:442-450`) is a (stride, share, patch)
triple:

| rung | stride | share | patch floor |
|---|---|---|---|
| `Rare` | 2 | 0.10 | 6 |
| `Sparse` | 2 | 0.15 | 6 |
| `Regular` | 2 | 0.30 | 5 |
| `Dense` | 2 | 0.50 | 4 |
| `Every` | 1 | 1.00 | 0 |

- Stride 2 keeps only cells with even coordinates on both in-plane axes
  (`on_lattice`, `skin_decor.rs:424-431`). So 3 of 4 plates are never on offer.
- The share is a hash of (cell, out face, fixture id) against the share
  (`skin_decor.rs:248`, `498-510`). No RNG. Spatially it is white noise.
- The patch floor puts one piece in each `patch`-cubed block that the share
  left empty (`skin_decor.rs:309-390`). If the block has no free plate, it may
  BORROW a plate from a lower-priority rule, under three conditions
  (`skin_decor.rs:297-308`, `397-400`).
- Priority: a plate takes at most one piece, and the first fixture in the list
  wins (`skin_decor.rs:125-143`). So the real count of a rule depends on every
  rule above it and on every rule it can borrow from.

### `orientation` [code]

`turns_for` (`skin_decor.rs:456-479`) picks 0 to 3 quarter turns about the
plate's out axis:

- `Free`: 0 turns. The piece keeps the PLATE's rotation. That rotation is the
  turn that makes the plate's shape canonical (`shell_skin.rs:643-661`), so it
  depends on the plate's own edge pattern, not on the ship.
- `Along`: +Z on `PlateReading::along`, the axis with the longer run of like
  plate; ties go to the lower axis (`skin_reading.rs:329-338`).
- `Outward`: +Z on `PlateReading::fall`. The fall is a cardinal only on a
  `Brink`; it is a diagonal on a corner and zero on Flat, Ridge and Peak
  (`skin_reading.rs:206-212`, `553-569`). Zero means 0 turns.

`decor_pose` then beds the piece onto `seat_normal` (`skin_decor.rs:530-535`),
so a piece on a raked plate already tilts with it. The owner's "turn with an
angled plate" is already true for tilt. It is not true for yaw.

### Real base content [code]

Generated RON, `assets/base/styles/base.content.ron`, from
`crates/nova_authoring/src/base_content/styles.rs`:

```ron
(   id: "industrial_stack",            // styles.rs:112
    collider: (0.18, 0.28, 0.18),
    placement: (region: HighGround, density: Regular, orientation: Free)),
(   id: "industrial_crane",            // styles.rs:129
    collider: (0.16, 0.45, 0.32),
    placement: (region: Edge, density: Rare, orientation: Outward)),
(   id: "industrial_hazard_band",      // styles.rs:145
    collider: (0.19, 0.03, 0.9),
    placement: (region: Edge, density: Every, orientation: Along)),
(   id: "industrial_radiator",         // styles.rs:237
    collider: (0.38, 0.27, 0.28),
    placement: (region: Panel, density: Dense, orientation: Along)),
(   id: "armoured_chaff",              // styles.rs:631
    collider: (0.3, 0.13, 0.18),
    placement: (region: Panel, density: Sparse, orientation: Outward)),
(   id: "armoured_ammo_stripes",       // styles.rs:663
    collider: (0.3, 0.02, 0.34),
    placement: (region: NearFitting, density: Every, orientation: Along)),
(   id: "civilian_windows",            // styles.rs:802
    placement: (region: Flank, density: Dense, orientation: Along)),
```

There are 56 fixtures in 5 styles. `styles.rs` has 27 comments with "measured"
and 13 with "starv". Most of them explain list order, not look. Example:
"the hatch measured `x0 of 12`" until it was moved above the corrugation
(`styles.rs:350-355`).

### Measured counts

What the hulls offer, per relief, on the landed skin [doc]
(`docs/sections.md`, table after line 690):

| hull | plates | flat | step | ridge | peak | brink | spur |
|---|---|---|---|---|---|---|---|
| `block_waystation` | 42 | 3 | 3 | 0 | 0 | 13 | 23 |
| `block_cutter` | 64 | 11 | 8 | 5 | 2 | 14 | 24 |
| `block_gunship` | 61 | 2 | 24 | 2 | 1 | 7 | 25 |
| `block_hauler` | 93 | 16 | 20 | 4 | 1 | 15 | 37 |
| `block_line_warship` | 171 | 23 | 54 | 21 | 0 | 44 | 29 |
| `ring_station` | 210 | 0 | 36 | 0 | 18 | 24 | 132 |
| `spire_station` | 134 | 10 | 16 | 4 | 0 | 48 | 56 |
| `yard_station` | 134 | 10 | 12 | 0 | 22 | 24 | 66 |

What the industrial kit took, on the landed skin [log] (the only post-landing
tally on disk: session scratchpad `133631/run-final-C.log`, scenario
`skin_survey`, ship entity `1334v0`, 139 plates, relief `step x34, ridge x13,
peak x3, brink x44, spur x45`, no flat plate). Format is TAKEN of REACH:

```
industrial_stack x2 of 6        industrial_radiator x0 of 0
industrial_crane x1 of 12       industrial_duct x0 of 0
industrial_hazard_band x40 of 40  industrial_winch x0 of 0
industrial_umbilical x1 of 1    industrial_floodlight x0 of 0
industrial_plate_rack x0 of 1   industrial_louvre x0 of 0
industrial_cells x0 of 1        industrial_hatch x9 of 34
industrial_stencil x8 of 34     industrial_ribbing x2 of 34
63 pieces, 63 off flat (100%), 8 on a creased top
```

7 of 14 rules placed nothing. The `Dense` filler (`ribbing`) took 2 and the
`Regular` hatch took 9, so the rung does not predict the count; list order
does. The task 133631 close also records `armoured_intake` at 0 of 1 on the
gunship [doc]. Earlier prototype logs in the same folder (`run-hybrid3-*`) use
a different relief split and are not valid for the landed code.

## 2. What is unclear to a modder

Each item is a place where the authored words do not predict where the piece
lands. All are [code] unless labelled.

1. **`HighGround` includes `Step`** (`skin_style.rs:403-408`). `Step` is "any
   other whole-cell plate: it climbs structure beside it"
   (`skin_reading.rs:94-96`). So `industrial_stack` can land at the FOOT of a
   raised block, which is the low ground beside a wall.
2. **`Panel` is `Flat` only, and `Flat` is now rare.** Since 18a4f9ae8 every
   plate top is a flat plateau inside chamfered rims (`docs/sections.md`,
   "Almost every plate still has a SEAT"). A Brink plate LOOKS like a flat
   panel with one bevelled side, but it is not `Panel`. The industrial radiator and duct,
   and the armoured sensor, intake, chaff and hatch, the civilian dish, livery,
   skylight, vent and fairing, and the salvage hose
   all ride `Panel`. On `ring_station` that is 0 plates; on the 139-plate survey
   ship it was 0 [log].
3. **`Deck` does not mean up.** Its facing is `Any` (`skin_style.rs:380-385`).
   `industrial_plate_rack` can lie on the belly or a flank. `Flank` is the same
   plates with facing Side, so it is a subset of `Deck`, and the two fight for
   the same plates (`styles.rs:172-183`).
4. **`NearFitting` reads narrow and is wide.** Measured, a pocket rule placed
   first took 45% of a ship [doc] (`web/src/create/styles.md`, "The seven
   regions"). Every base kit puts it last for that reason.
5. **`Edge` stops at corners.** A hull corner plate is `Spur`, not `Brink`, so
   a rim line has a gap at every corner. A whole-cell plate on an edge is
   `Step` (`skin_reading.rs:502-504`), so the line also stops where the plate
   climbs a wall.
6. **The collider changes the placement silently.** A decal 0.09 cells tall is
   trim and lands on a crease; at 0.10 it does not (`skin_style.rs:337-351`).
   A piece 0.45 long needs a run of 1; at 0.55 it needs a run of 2
   (`skin_style.rs:319-322`). No placement word says this.
7. **The rung word does not predict a count.** The count is stride x share x
   priority x patch floor x borrowing. A `Rare` rule landed nothing on every
   bench seed until borrowing was added (`skin_decor.rs:292-295`). Lattice
   parity "can zero a strided pocket rule outright" (`styles.rs`, comment on
   `armoured_ammo_stripes`, near line 676).
8. **The hash looks random.** The share is white noise over the lattice.
   Neighbouring plates of one run get unrelated answers. This is the owner's
   "it's pretty random", in code.
9. **`Outward` does nothing on most of its rules.** It turns only on a Brink.
   `armoured_chaff` is `Panel` (Flat only, no fall), so it never turns.
   `industrial_floodlight` is `NearFitting`, so it turns only where the plate
   beside the fitting is a Brink.
10. **`Free` is not "unturned".** It keeps the plate's canonical-shape turn
    (`shell_skin.rs:643-661`). On a symmetric Flat plate that is stable; on an
    asymmetric plate it depends on which edges fall. Two magazines on two
    neighbouring spurs can face different ways. [code; not seen in a frame]
11. **Priority is the real language.** Order decides starvation, and the
    builder comments are mostly about order. A modder cannot see starvation on
    screen; only the debug `x0 of N` tally shows it (`shell_skin.rs:990-1017`).

## 3. Prior art

1. **SpaceshipGenerator (Blender, a1studmuffin)** [web: source read via
   `gh api`]. Per face: skip faces with aspect ratio > 3, sort the face by
   normal (rear, front, top, bottom, side), then pick ONE detail type from a
   weighted table for that class (for example top: antenna 30%, grid 30%,
   cylinders 30%). `not engine_faces or ...` guarantees at least one engine.
   Lesson: classes come from geometry the reader can see, and one roll per
   face picks from a table. No priority list.
2. **Blender greeble add-ons (Plating Generator, Greeble Generator,
   GreebleMaster)** [web]. Plating Generator first cuts faces into panels, then
   scatters pieces on the panels. A piece "can either conform to the shape of
   the face or maintain [its] proportions". Greeble Generator aligns every part
   to the face normal with no manual rotation. Lesson: split the surface first,
   then fill; orientation is the surface frame, not an authored word.
3. **Houdini (Scatter, Copy to Points)** [web]. The surface carries attributes
   (normal, groups); points copy instances with +Z on `N`. Density is a point
   count or an attribute. Lesson: the frame comes from surface attributes.
   Density as a number works only with a human looking at the result.
4. **CGA shape grammar (Mueller et al., SIGGRAPH 2006; CityEngine)** [web for
   the paper; operators from knowledge]. Oriented scopes, `split`, `repeat`
   and `comp(f)`. `comp(f) { top: Roof | side: Facade }` names face classes;
   `repeat` places a module every N metres along an axis, so a facade gets a
   rhythm, not noise. Lesson: name the feature, then the pattern along it.
5. **Wave Function Collapse and Townscaper (Gumin; Stalberg)** [web]. Tiles
   are chosen from the values on their corners (dual grid, marching squares);
   WFC picks among compatible tiles with weights. Lesson: Nova's `ShellShape`
   is already a corner and edge keyed tile. A greeble could be chosen by the
   plate's key, or (task 133643) the plate key could be chosen to fit the
   greeble.
6. **Socket and anchor decoration** [knowledge]. Space Engineers blocks
   declare mount points per side; KSP parts declare attach nodes; Elite
   Dangerous ship kits have fixed slots per hull (bumper, spoiler, wings,
   tail; one piece per slot) [web for Elite]. No Man's Sky ships combine named
   descriptor parts with a chance per part [web]. Lesson: a slot is the
   clearest possible "where", but it must be authored per part. Nova hull
   cubes are uniform, so section sockets cannot say "edge". Nova already has
   sockets (`LinkPoint`, with `skin: Clad | Bare` from 53c5a186a); a fitting's
   exit face is the one place a socket adds facts.
7. **Starbase decals, Space Engineers greebling** [web]. Both are manual: the
   player places decals or decorative blocks. Builder advice is "mark off some
   broad areas with higher concentration, sprinkle small groupings between".
   Lesson: humans greeble by area and cluster, not by uniform share.

Common thread: the good systems name a FEATURE the reader can see, place by a
PATTERN or a table on that feature, and take orientation from the surface
frame. None of them has a hidden priority list.

## 4. Candidate languages

Shared by all three candidates:

- **`orientation` is deleted.** Every piece is posed in the FEATURE FRAME:
  +Y out of the plate, bedded on the tilt (`seat_normal`, as today); +Z along
  the run (or the long axis of a patch); +X toward outboard where the plate
  falls one way. The run axis has no sign, so the engine picks the sign of +Z
  that puts +X outboard. At a run end, +Z points into the run. So a crane jib
  is authored along +X once and always leans out. This keeps today's `Along`
  for every piece and turns `Outward` into a model convention. The 32 `Along`,
  18 `Free` and 6 `Outward` uses migrate by re-authoring at most the
  `Outward` models (rotate the model 90 degrees in its recipe). [unverified:
  per-model check needed for the 6]
- **Kinds are the relief classes, named as the log names them**: `Flat`,
  `Step`, `Ridge`, `Peak`, `Brink`, `Spur` (`skin_reading.rs:116-123`), plus
  `Mouth` = a plate one face step from a fitting exit. A plate is `Mouth`
  before its relief. The debug tally already prints these words, and the
  editor can tint them. Optional `facing: Up | Down | Side` reuses
  `PlateFacing` (`skin_reading.rs:148-159`).
- **Hidden gates die.** `min_depth`, `min_height`, the trim rule and the seat
  gate go away when 133643 gives every piece a pad. `min_run` becomes part of
  the pattern.
- **Saves are safe.** A frozen wreck stores each greeble's model, pose and
  health, not its placement (`skin_decor.rs:583-601`). A language change
  breaks style RON only. Mark it `**(breaking)**`.

### Candidate A: feature + pattern (CGA style)

**Model.** A placement is `on` (one or more kinds), an optional `facing`, and
a `pattern` over each CONNECTED FEATURE of that kind. A feature is a run (Brink,
Ridge, Step along a wall, Mouth ring) or a patch (connected Flat on one plane),
or a point (Peak, Spur).

| pattern | where it lands | replaces |
|---|---|---|
| `Line` | every plate of each run of 2 or more | `Every` + `min_run` |
| `Every(n)` | every n-th plate along the run, on the ship lattice (`cell mod n == 0`) | stride + share |
| `Ends` | the first and last plate of each run | (not possible today) |
| `Middle` | the middle plate of each run or patch | (not possible today) |
| `Once` | one per ship: the middle of the longest feature, ties to the lowest cell | `Rare` + patch floor |

On a point kind (Peak, Spur), `Line` means every point and `Every(n)` uses the
lattice. `Every(n)` is anchored to the ship lattice, not to the run start, so
adding a cell does not move the other pieces (the same stability as today).
`Ends` and `Middle` are run-relative by definition and may move when a run
grows.

List order stays, but only as a tie rule inside one feature: `Ends` before
`Line` on Brink gives caps and a line. Content lint can PROVE starvation: any
rule after a `Line` on the same kinds and facing can never land. That is an
error at lint.

**RON:**

```ron
(   id: "industrial_hazard_band",
    placement: (on: [Brink], pattern: Line)),
(   id: "industrial_stack",
    placement: (on: [Ridge, Peak, Spur], facing: Up, pattern: Every(2))),
(   id: "industrial_radiator",
    placement: (on: [Flat], pattern: Every(2))),
(   id: "armoured_ammo_stripes",
    placement: (on: [Mouth], pattern: Line)),
(   id: "industrial_crane",            // new look: a hoist capping each edge run
    placement: (on: [Brink], pattern: Ends)),
```

**Plates.** Kinds are `relief_of` plus a fitting distance. Runs reuse the walk
in `read_plates` (`skin_reading.rs:321-327`), which already measures like
runs; it needs the run's id, index and length kept instead of thrown away.

**Density and orientation.** Density becomes spacing (`Every(n)`) or count
(`Once`, `Middle`). Rarity is explicit: "one per ship", "one per run". No hash
share, no patch floor, no borrowing. Orientation: the feature frame above.

**Needs from 133643.** The placement pass must return, per placed piece, the
plate, the footprint (collider x and z), the feature frame, and for `Line` the
neighbours in the run. The plate then flattens a pad under the footprint. For
a `Line` the pads must meet across plate seams, so the reshape must keep the
level on the shared edge along the run. This is the order change:
derive -> read -> place -> reshape -> spawn.

**Main risk.** Regular patterns read as machined. `salvage` wants "unplanned"
(`web/src/create/styles.md`, "Reading as UNPLANNED"). Mitigation, if the owner
wants it: one explicit hash pattern `Scatter(n)` = "about one in n plates",
used only by salvage. That re-introduces a share, but as one honest number.

**Migration cost.**
- Base builders: rewrite 56 placements in `styles.rs`; delete most order
  comments; rewrite the ~20 builder tests that pin order and rungs
  (`styles.rs:1567-2145`). Regenerate base RON.
- Engine: replace `FixtureRegion`, `FixtureDensity`, `FixtureOrientation`,
  `ScatterRule`, `fill_patches`, `lends`, `turns_for` (`skin_style.rs`,
  `skin_decor.rs`); update `skin_report.rs` rule counts; editor
  `crates/nova_editor/src/skin.rs` (2 sites); `examples/shared/dev_fixtures/styles.rs`
  (6); `greeble_catalog.rs`, `block_bench.rs`, `shape_bench.rs`; 3 test
  fixtures in `frozen.rs` and `frozen_piece.rs`.
- Example mod (`assets/mods/example`): no style content. Cost 0.
- the-ledger webmod (`webmods/the-ledger`): no style content. Cost 0.
- Docs: `web/src/create/styles.md` placement section (about 150 lines),
  `docs/sections.md` style section (about 50 lines), `docs/concept-index.md`
  row, one `**(breaking)**` changelog entry.

### Candidate B: weighted table per plate kind (SpaceshipGenerator / WFC weights)

**Model.** Each plate of a kind rolls ONCE (the same cell hash as today)
against a table. The table holds every fixture whose `on` lists that kind
(after `facing`), each with its `weight`, plus the style's `bare` weight for
that kind. No list order and no lattice.

**RON:**

```ron
Style((
    id: "industrial",
    bare: {Flat: 1.0, Step: 6.0, Ridge: 2.0, Peak: 1.0, Brink: 0.0, Spur: 3.0, Mouth: 1.0},
    fixtures: [
        (   id: "industrial_hazard_band",
            placement: (on: [Brink], weight: 20.0)),
        (   id: "industrial_crane",           // 1 in 21 brink plates
            placement: (on: [Brink], weight: 1.0)),
        (   id: "industrial_stack",
            placement: (on: [Ridge, Peak, Spur], facing: Up, weight: 1.0)),
        (   id: "industrial_radiator",
            placement: (on: [Flat], weight: 1.0)),
    ],
)),
(   id: "armoured_ammo_stripes",
    placement: (on: [Mouth], weight: 1.0)),   // armoured bare: {Mouth: 0.0, ...}
```

**Plates.** Per plate, no runs needed. Kinds as above.

**Density and orientation.** Density becomes a weight against `bare`. The
share of a kind is exact: weight / (sum of weights + bare). Starvation is
impossible, because there is no order. Optional `at_least: 1` per ship covers
small hulls. Orientation: the feature frame.

**Needs from 133643.** Per placed piece: plate, footprint, frame. Pads are
per plate. A line (hazard band) is only a line when its weight wins every roll,
so pads along a run do not need to agree. Simplest for 133643.

**Main risk.** It is still white noise. The owner's complaint "pretty random"
stays: proportions become predictable, positions do not. A 1-in-21 crane on a
ship with 12 brink plates lands 0 or 1 by chance.

**Migration cost.** Same files as A. Smaller engine change: `fill_patches`,
`lends`, `on_lattice` and the ladder die; `claims` becomes one weighted pick.
Style RON gains a `bare` map. Example mod and the-ledger: 0. Docs: same pages.

### Candidate C: panel layout first, greebles fill panels (Plating Generator / CGA split)

**Model.** The engine first cuts each patch (connected same-kind, same-facing
plates on one plane) into PANELS: rectangles of whole plates, greedy
largest-first, on the ship lattice, from the style's panel sizes. Runs are cut
into strips. Each greeble declares the panel size it fills. Each panel takes
one greeble, by the hash among the fitting ones, or stays blank.

**RON:**

```ron
Style((
    id: "industrial",
    panels: [(2, 2), (2, 1), (1, 1)],
    fixtures: [
        (   id: "industrial_hazard_band",
            placement: (on: [Brink], panel: Strip)),
        (   id: "industrial_stack",
            placement: (on: [Ridge, Peak, Spur], facing: Up, panel: (1, 1))),
        (   id: "industrial_radiator",      // a 2-plate bank: needs a new model
            placement: (on: [Flat], panel: (2, 1))),
        (   id: "armoured_ammo_stripes",
            placement: (on: [Mouth], panel: Strip)),
    ],
)),
```

**Plates.** Needs patch and run segmentation plus a deterministic packer.

**Density and orientation.** Density becomes the panel mix and the blank
panels. Orientation: the panel's long axis. Multi-plate greebles become
possible; today "decoration cannot span cells" (`web/src/create/styles.md`).

**Needs from 133643.** The most: a panel is several plates that must reshape
as ONE pad at one level. The plate must learn its panel, not only its piece.

**Main risk.** Scope and stability. A one-cell edit re-packs a whole patch, so
the editor preview jumps on every build step. A piece over several plates
breaks "a greeble is a child of one plate" (`shell_skin.rs:985`): damage, shot
off, and frozen wrecks all assume one parent.

**Migration cost.** Highest. Everything in A, plus a packer, multi-plate decor
parents, frozen decor for spanning pieces, and new 2-plate models for the
kits that want them. Example mod and the-ledger: 0.

## 5. Recommendation

**Candidate A, feature + pattern**, with `Scatter(n)` held back unless the
salvage look needs it. A answers each of the owner's three points directly.
The place words become the relief classes the engine already logs and the
editor can tint, so `on: [Brink]` means exactly the plates a reader can see,
with no hidden union, depth, height or trim gate. Density stops being a
priority-coupled share and becomes a pattern a modder can predict from the
words alone ("every 2nd plate", "both ends", "one per ship"); the 27 "measured"
comments in `styles.rs` exist because today's rung does not predict a count.
Orientation dies into a feature frame that also gives `Outward` pieces a real
outboard axis on every falling plate, not only on a Brink. A also feeds 133643
the best inputs: a run and a footprint per piece, so the pads of a `Line` can
meet across seams. B is the cheaper fallback, but it keeps the white-noise look
the owner calls random. C is the right long-term direction for multi-plate
greebles, but its edit stability and multi-parent decor make it a later step
that can build on A's features.

Do you accept candidate A (`on` a relief kind, `pattern: Line | Every(n) | Ends | Middle | Once`, no density or orientation) as the language task 133643 implements?
