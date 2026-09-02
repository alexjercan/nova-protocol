# Spend the lance's wasted power on width: the rake blast

- STATUS: CLOSED
- PRIORITY: 62
- TAGS: v0.13.0,ship,weapon,balance

Split out of the review of `20260824-125947` on 2026-09-02, at owner
direction. The lance shipped and reads WEAK beside PDC spam. It is not
weak. It is aimed through a budget no shipped ship is thick enough to
spend.

## The measurement

`slug_power: 1800`, and a Pierce round pays `max_health /
pierce_power_multiplier` to cross a layer. A 1500 u/s slug pins that
multiplier at its `PIERCE_POWER_CEILING` of 3.0, so a 200 hp reinforced
cell costs 66.7. The budget therefore buys **27 layers**. The lance also
carries `layers: u32::MAX`, so nothing else bounds it.

No shipped craft is more than about six cells deep along a line of fire.
A four-cell corvette line spends 267 of 1800: **85 percent of every shot
leaves through the far side.**

What that costs, per 13.5 s cycle (1.5 s charge + 12 s reload), against
four cells of reinforced hull:

| weapon | reach | sustained | intercept |
|---|---|---|---|
| PDC kinetic | 200 u | 267 dps | none possible |
| Railgun lance | 1800 u | 59 dps | none possible |
| Torpedo (Serpent) | 3200 u | 75 dps | ~370 PDC rounds each |

The gun is 4.5 times worse than the weapon it is meant to outrange, and
it costs a 1.5 s unabortable commit to fire.

## What this task wants

Convert the surplus DEPTH into WIDTH. Owner's framing: a slug that
punches a needle should leave a hole you can see. A lance should excavate
a narrow, continuous corridor through a deep hull, with total destruction
increasing as it crosses more structure.

The first-round design is a Pierce rake, not a blast:

- The original narrow slug tip must hit a body directly before the rake is
  armed for that body. A near miss stays a miss, and each separate body
  requires its own direct hit.
- Once armed, a sphere trails the tip. Its centre is offset backward by its
  radius along the direction of travel, so its front is tangent to the tip
  and cannot damage untouched sections ahead.
- Sweep the trailing sphere as the slug advances. The swept volume is a
  cylinder with a rounded rear cap, so it catches angled and irregular
  lateral sections without the gaps made by discrete perpendicular disks.
- Every section in that volume takes the normal flat Pierce `slug_damage`
  once and spends the same shared `slug_power` budget. There is no blast
  damage, falloff, transmission rule, or second damage amount.
- Body arming persists for the slug's lifetime and through internal gaps.
  After the tip leaves the far side, the trailing sphere finishes passing
  through the body and opens a same-width exit. The first version has no
  special exit flare.
- Expose the rake radius as one optional railgun authoring field. Omitted
  means exactly today's narrow-round behavior, so only the base catalog
  changes. Choose the base value from the probe rather than assuming the
  old blast seed of 4.0 is suitable.

The width consumes the existing power rather than adding an independent
lethality pool. A dense, wide hull spends the slug sooner; a fighter on the
centreline still presents little material and receives little extra damage.

## Explore while proving the shape

- Measure which rake radius catches immediate lateral neighbours without
  making the result read as a whole-ship delete. The exact value is not yet
  chosen.
- Define body identity from the collider ownership already used by rounds and
  ships. Lateral candidates must match a body the narrow tip armed; one ship
  must not grant a wide hit against another.
- Resolve candidates deterministically. Nearer travel depth should win, and
  candidates at the same depth should be paid from the axis outward, so an
  exhausted budget leaves a centred hole rather than an arbitrary half-cut.
- Inspect where lateral `SurfaceImpact` and carve marks land. The health result
  is not enough: placing every mark on the central axis can make the visible
  corridor and exit wrong even when the correct sections die.
- Prove that the trailing sweep continues far enough beyond the last direct
  hit to complete the exit, including across internal gaps.
- Rocks have no `Health` and stop the existing Pierce round. Keep that contract
  unless the probe demonstrates a deliberate rake rule for them;
  `mark_radius(200)` is already 2.29 u, so the lance does not look narrow on
  asteroids today.

## Done when

- The base lance leaves a visibly wider cylindrical corridor than its bore,
  including a same-width opening on the far side, and a probe run shows the
  widened bite without damage ahead of the tip.
- A separate body that the tip misses takes no rake damage.
- Every raked section takes Pierce damage once and spends the shared power;
  internal gaps neither duplicate damage nor disable the armed body's rake.
- Effective dps against a four-cell line is recorded before and after,
  in the same rig, and sits in a defensible place against the PDC's 267.
- The optional rake-radius field is documented in the creator reference's
  Railgun chapter, which now exists.
- If the rake lands, the reload is NOT also shortened. It roughly
  triples effective per-shot damage on its own.

## Landed (2026-09-02)

One optional field, `rake_radius`, on `RailgunSectionConfig`. Omitted, the
slug spawns with no `RoundRake` at all and cuts exactly the column its bore
crossed, so every mod authored before the field existed is untouched. The
base lance authors **1.0**.

### Why 1.0 and not the 4.0 blast seed

Measured, in `system_railgun_lance`'s stand bank, against 4-deep blocks of
200 hp reinforced cells on a unit lattice:

| stand | rake | cells | corridor profile | removed | dps |
|---|---|---|---|---|---|
| narrow_line | none | 4 | `[1,1,1,1]` | 800 | 59.3 |
| raked_line | 1.0 | 12 | `[3,3,3,3]` | 2400 | **177.8** |
| narrow_wall | none | 4 | `[1,1,1,1]` | 800 | 59.3 |
| raked_wall | 1.0 | 28 | `[9,9,9,1]` | 5600 | 414.8 |
| wide_wall | 4.0 | 28 | `[25,3,0,0]` | 5600 | 414.8 |

Against the four-cell line the task specified: **59.3 dps before, 177.8
after**, over the same unshortened 13.5 s cycle. That is 67 percent of the
PDC's 267 at nine times the reach, and three times the lance's own old
number - which is what the task asked the width to be worth.

Wider is NOT more. Both walls spend the identical 1800 power and remove the
identical 5600, because against dense material the budget binds either way.
What the radius chooses is the SHAPE: 1.0 bores three cells wide through all
four layers and out the far side, 4.0 strips the entry face and stops one
layer in. The rendered pair says the same thing - the authored corridor is a
hole punched clean through the block, the 4.0 seed leaves a block you still
cannot see through.

1.0 also has a geometric argument, which is why it was chosen over the other
values that measure well. It is the smallest radius that reaches the
immediate lateral neighbour on every shipped hull: the cargoa's pods stand
0.81 off its spine, the cargob's 0.61. On the unit lattice hulls are built
on it takes the face (0.5) and diagonal (0.707) neighbours of the cell it
bores and stops short of the second ring at 1.5.

### A sweep defect found on the way

`raked_wall` first measured `[5,9,9,5]`, not the `[9,9,9,1]` the
depth-then-offset ordering rule requires: the four diagonal cells of the
ENTRY layer were never charged while the identical four one layer deeper
were.

The cause is not the rake. At 1500 u/s a fixed step is 23 units of travel,
and handing parry a 23-unit capsule to intersection-test against a 1-unit
cell is badly enough conditioned that shallow overlaps come back as misses -
swept down a 5x5x4 lattice it reported 31 of the 36 cells it covers, and the
five it lost overlap it by 0.29, not by a rounding error.
`SpatialQuery::shape_intersections` runs the same test, so the broad phase
inherits it.

The sweep now resolves the capsule ANALYTICALLY instead, from the corridor
measurement it already takes for ordering and for placing the carve: a cell
is inside when its nearest point to the flight axis lies within the radius
of the sphere's centre track. One measurement now drives inclusion, order
and contact point, and there is no shape query in the rake at all. Pinned by
`a_raked_slug_crossing_in_one_step_still_takes_the_shallow_corners`.

`corridor_contact` lost its `project_point_predicate` for the same reason -
a whole-world closest-point traversal, filtered down to one known collider,
is now a direct projection onto that collider.

### What the rake is bounded by

The armed body's own colliders are the whole candidate set, so there is no
world query to cap and no layer count: `layers: u32::MAX` stands, and the
authored power is the only bound. The wall spends 1866.67 of 1800 - 28 cells
at 66.67 - because the bite that empties the budget still lands.

### Rendered

Corridor, entry and exit inspected at 3x cell scale under Xvfb. The bore is
a square hole through the block, three cells wide at the entry face and one
at the exit, which is the budget running out and not a taper rule. Lateral
impacts and carve marks land on the faces the corridor opened, not on the
central axis. The 4.0 seed's block shows no opening from any angle.

### One flake fixed on the way

The range tapped the commit for exactly one frame. The walk runs on the render
clock and the gun on the fixed one, so that tap could land entirely between two
of the gun's ticks - and then the range sat out its whole run waiting for a shot
nothing had asked for. It failed roughly one run in five. The trigger is now
held until the charge is actually running and released then, which is the same
tap and the same reading for invariant 1.

### Verified

`nova_gameplay --lib rounds::` (27, including the ten this task asked for),
`nova_ship --lib railgun` (13), `nova_scenario --lib lint::ship` (13),
`content lint` (0 errors, 0 warnings), `cargo fmt --all`, and
`probe run system_railgun_lance` OK on all nine invariants, four runs in a
row.

Not run: the workspace test suite and Clippy.

### Deliberately not done

- **The reload is unchanged.** The rake roughly triples effective per-shot
  damage on its own; both together would be a different weapon.
- **No exit flare, no falloff, no second damage amount.** Every raked
  section takes the flat Pierce `slug_damage` once and spends the shared
  budget, which is the whole rule.
- **Rocks keep their contract.** They have no `Health` and stop the round,
  as they did before. Nothing in the bank argued for a rake rule for them.

### Docs pass (2026-09-02)

The wiki's Railgun page leads with a corridor scope (`lance-corridor` in
`web/src/widgets.ts`): the stand bank's block, the tip and the trailing sphere
replayed in scope time, the entry face counting what each column lost. Its
model walks the budget in f32 and `web/tests/widgets.test.ts` pins it to the
measured stands - `[9,9,9,1]`, `[25,3,0,0]`, `[3,3,3,3]`, `[1,1,1,1]`. The
combat page gains the lance as a third family and an engagement ladder
(`weapon-reach`); the HUD page gains the bore sight; the glossary, keybinds,
creator chapter, dev book (a `sweep_raking` flowchart) and concept index are
synced. Four capture slots and one loop slot are placeholders with no producer
yet, and `catalog-railgun-lance-section.png` aliases the page's hero; the
railgun icon is generated.

One correction to the measurement above: the PDC's 267 dps assumed the
200-round batch is spent in no time. The catalog's own sustained formula
(`sections/mod.rs sustained_per_second`, amount over delay plus batch fire
time) gives 40 rounds/s, which is **160 hp/s** kinetic at 100 u/s closing and
320 at the head-on ceiling. The wiki cites 160. The verdict does not move: the
corridor's 178 against the corvette line sits above it at nine times the reach.

## Not this task

`AI_STANDOFF_RANGE` is 100 u while the lance reaches 1800 u, so every
fight collapses to inside PDC range and the lance never gets its window.
That is the OTHER half of the balance problem and it is fight geometry,
not a weapon stat. It wants its own task.
