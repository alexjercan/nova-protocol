# Giving a generated ship a front

Design note, 2026-08-16. Written against `examples/screenshots/wfc_ships.rs` on
branch `wfc-layout`.

The complaint it answers: looking at any render of the `wfc_ships` row, drives
point in several directions on the same hull. The generator places parts by
mating rules alone. Nothing knows a thruster belongs at the back, a bridge on
top, or guns where they can bear.

One of the recommendations below is already implemented and rendered, because it
turned out to be twelve lines. The rest is a proposal.

## 1. The actual cause, which is narrower than "no sense of layout"

Two facts about the generator, together, produce the whole symptom.

**A drive's direction is not a choice the generator makes.** The thruster
section carries ONE socket, on its forward end, and its exit normal is `+Z` in
its own frame. So the face it is bolted to IS the direction it exhausts. A drive
bolted to the roof fires up; one bolted to the transom fires aft. Nothing
"chooses badly" - the collapse never chooses at all, it just finds a hull face.

**The mating rule is BINARY and cannot hold a fact about a whole ship.**
`compatible(tiles, here, face, there)` reads one cell and its neighbour. "Aft" is
not a property of any pair of cells; it is a property of the grid. A binary
constraint cannot express it, ever, no matter how the tileset is authored.

So the symptom is not a missing heuristic. It is a category error: the file was
asking a local rule to carry a global fact. Everything below is a way of putting
that fact somewhere a local rule can see it.

Worth saying plainly what is NOT broken. Guns already point somewhere sensible,
and for a reason: exit clearance forbids a muzzle whose lane is fouled, so a
mount can only survive standing proud of the hull with space in front of it.
Clearance is doing the "where they can bear" job already. Bays likewise - a
broadside tube is a real warship, and a bay firing into its own hull is already
illegal. The drive was the one part whose direction was both free and wrong.

## 2. What the grid already gives us to hang this on

More than expected. The generator is not orientation-blind - it is
orientation-blind *about parts* while being quite opinionated about *shape*.

| Fact | Where | What it already means |
| --- | --- | --- |
| `LENGTH = 11` on `z` | `HULL_GRID` | the ship's long axis |
| `HEIGHT = 5` on `y`, `KEEL_ROW = 2` | `HULL_GRID`, `KEEL_ROW` | up, and a middle |
| `x = 0` is the mirror plane | `HULL_GRID.origin.x = 0.5` | port/starboard |
| `VACUUM_BOW_TAPER = 24.0` | `hull_vacuum_weight` | **`z = 0` is the BOW** |
| `VACUUM_STERN = 9.0` | `hull_vacuum_weight` | the last row is sparse |
| `seed_keel` | collapses the spine by hand | a connected structure to grow on |
| `keel_prototype(LENGTH / 3)` | the bridge, forward of centre | a front, already |

The answer to "does `keel_component` / the mating structure give a natural axis
to hang this on" is yes, and it is not the mating structure - it is the KEEL.
`seed_keel` already hand-collapses eleven cells before the generator gets a say,
and `keel_prototype` already puts a bridge forward of centre. The precedent for
"decide the big thing up front, let the collapse fill in around it" is in the
file, working, with a doc comment explaining why. Everything in this note is that
same move applied one more time.

What the grid does NOT give: any asymmetry in `y`. `off_keel` is
`x + |y - KEEL_ROW|`, symmetric about the keel row, so a hull's deck and its
belly are weighted identically. A ship generated here has a front and a back but
genuinely has no top.

## 3. What comparable generators do

Read rather than re-derived. Sources at the end.

**Nobody makes a local constraint carry a global fact.** The consistent answer
across practitioner writeups is: decide the coarse thing FIRST, by other means,
then let the constraint solve fill in detail. Boris the Brave calls this driven
WFC and states the cost plainly - the algorithm already starts with a set of
possible tiles per cell, so filtering that set before the solve is free. His
worked example is Townscaper, where the player's painted solidity decides which
tiles may go in each cell and WFC never sees a global decision at all.

**Per-cell BANS, not per-cell weights.** This is the distinction that matters and
it is easy to get wrong. Weights bias; they do not forbid. A low-weight drive at
the bow still lands at the bow eventually, and worse, tile weights in the classic
formulation are global frequency hints - vary them per cell and the entropy
heuristic has to move from a count to a proper Shannon entropy over the surviving
weights or the cell ordering stops meaning anything. A unary domain filter has
none of that cost. This is standard constraint-solving practice, not a trick:
unary constraints are handled by adjusting variable domains before the solve
starts.

**Buckets by normal direction, after the fact.** `a1studmuffin/SpaceshipGenerator`
(MIT), the best-known procedural spaceship, builds a hull by extruding front and
rear faces and then categorises every face by its normal and rolls detail per
category - engines on rear faces, antennae on front and top, weapons on the
sides. Two things to take: detail follows orientation, and there is a GUARANTEE
GUARD - the rear-face rule fires an engine if the roll passes *or the engine list
is still empty*, because a pure probability table produces engineless ships.

**Regions, if bans start causing contradictions.** Merrell's model synthesis
"modifying in blocks" - solve overlapping blocks with their borders constrained
to what is already placed, and restart the offending block rather than the whole
output - is the standard answer, and Caves of Qud ships a version of it, running
WFC with different settings on subsets of the map. This is the escape hatch, not
the starting point. It is worth knowing that our collapse has NO backtracking and
cannot get one cheaply, so anything that raises the contradiction rate is
expensive here in a way it is not elsewhere.

**Grammars are not automatically the semantics fix.** Shape and graph grammars
express "door on the front facade" naturally, because a grammar's scope is an
oriented box. But Merrell's own graph-grammar work targets LOCAL SIMILARITY, the
same goal as WFC, and does not claim to encode front/back/top. Swapping solvers
does not buy the fact; putting the fact in the right place does.

## 4. Ranked recommendations

Cost against effect. The first is done; the rest are not.

### 1. A per-part AIM, as a unary constraint. DONE, RENDERED

`Part` gains `aim: Option<usize>` - the only face this part may fire through -
and `hull_domains` strikes any tile that disagrees. The thruster gets
`Some(AFT)`; everything else keeps `None`.

- **Cost:** one struct field, a four-line predicate, one line in `hull_domains`.
  Cannot empty a domain, because `VACUUM` is compatible with everything and is
  never struck, so the no-backtracking collapse is not put at risk.
- **Effect:** every nozzle on every ship points aft. This is the whole reported
  complaint.
- **Price, measured over 12 seeds:** drives 258 -> 74. That is not a rounding
  error, it is 70% of the engines, and the reason is structural rather than a
  tuning miss: a drive has one socket and five blind faces, so it can only stand
  where EXACTLY ONE neighbour is solid, and fixing that neighbour to be the
  forward one means it can only stand on an aft-facing proud surface. On a hull
  whose only aft-facing surface is the transom, there are very few such cells.
  Raising the weight alone does not buy them back: at ten times the weight and no
  seed, the row still came back with 40 drives over 12 ships. **Supply, not
  price, is the binding constraint** - which is exactly why the next item is not
  optional.

### 2. Seed the drive deck, the way the keel is seeded. DONE, RENDERED

`seed_stern` hand-collapses two cells before the roll: a hull block beside the
last keel cell, and a nozzle bolted to its aft face. The mirror makes that a pair
either side of the centreline. `seed_keel` now stops one cell short of the
transom so the seam cell beside the drive is free - a keel cube there would press
a socket into the drive's blind flank.

- **Cost:** fifteen lines, in the shape of the function above it.
- **Effect:** every ship has an engine at the back of it, guaranteed, and the
  seeded pair MAKES the aft-facing surface the roll then fills the transom
  around. Item 1 says where a drive may not go; it cannot conjure the place where
  it may. The two are one change.
- **Measured, 12 seeds, with the drive weight taken 3.2 -> 6.4 to pay for the
  rule:** hull 1774 -> 2000, drives 258 -> 102, bays 96 -> 74, mounts 92 -> 62,
  bridges 46 -> 52. Per ship that is 21.5 scattered drives becoming 8.5 in one
  stern bank. Weapons are down about a third, which is a weight question and
  the note under item 6.
- **Rendered** at a matched camera over the default row. The ships gained a
  readable stern: a bank of nozzles all firing the same way, with the hull
  running forward from it. This is the single biggest change to how the row reads
  of anything in this note.

### 3. Give the hull a deck and a belly. MEASURED, NOT LANDED

`off_keel` is symmetric in `y`, so a ship has no top. Weight the two directions
differently - `|y - KEEL_ROW|` times `DECK_TAPER` above and `BELLY_TAPER` below -
and the hull fills out above the keel and tapers under it.

- **Cost:** one line and two constants.
- **Effect, measured over 12 seeds at 0.6 / 1.6:** hull 1774 -> 1968, drives
  258 -> 212, bays 96 -> 78, mounts 92 -> 90. **Rendered, and I did not land
  it.** It makes the ships denser and blockier rather than more oriented: one of
  the three read distinctly better, one read as a brick, and the owner has said
  on the record that the pointy, faceted vocabulary is wanted. A silhouette
  change is a taste call for the owner, not a bug fix, and this one is not
  clearly an improvement. Left here with its numbers so the next person does not
  have to measure it again.
- If it IS wanted, the honest version is probably not this: it is a `y`-dependent
  keel row, or a superstructure seeded on top of the keel the way item 2 seeds
  the stern. Which is item 4.

### 4. Seed a superstructure, not just a bridge cell

`keel_prototype` puts the controller at `y = KEEL_ROW`, mid-height, buried inside
the hull. A bridge that reads is a bridge you can SEE: on top, forward of centre,
standing proud.

- **Cost:** a second seeded run of cells, in `seed_stern`'s shape - two or three
  hull cells at `y = HEIGHT - 1` around `z = LENGTH / 3`, with the controller on
  top of them.
- **Effect:** a ship gains a recognisable island, which with the derived skin
  becomes a plated superstructure rather than a lump. It also gives the decoration
  scatter's `PlateFacing::Up` rules somewhere meaningful to fire.
- **Risk, and it is real:** a seeded tower is a stud, and `erode_studs` exists to
  take those off. It would need the same treatment the keel gets - either enough
  seeded neighbours to pass `SPIKE_SUPPORT`, or an exemption. Budget a couple of
  hours, not ten minutes.

### 5. Zone the grid, and give each zone its own part list

The general form of items 1-4: split `z` into bow / midships / engineering bands
(the vacuum taper already computes the signal) and ban part families per band -
no drives forward of the engineering band, no bays in it, bridges only in
midships-top. Same mechanism as item 1, a table instead of a formula.

- **Cost:** a band function and a table. Maybe forty lines.
- **Effect:** the ships get an internal LAYOUT rather than a uniform texture,
  which is the difference between "a hull with parts on it" and "a hull with a
  bow, a waist and an engineering section".
- **Do this only if items 1-4 are not enough.** Every band is more domain
  filtering, and filtering shrinks the solution space while local propagation
  still cannot see a dead end coming. This collapse has no backtracking. If bands
  start producing contradictions the answer is modifying-in-blocks per band, and
  that is a real piece of work.

### 6. Re-tune the weights, once, after all of the above

The record already says weights have to be tuned together and read together,
because the parts compete with each other rather than only with vacuum. Items 1
and 2 changed the competition: drives now bid for a small set of aft cells
instead of every exposed face, so the surface freed up went to hull. Bays and
mounts came down a third and nobody asked them to.

- **Cost:** a sweep, and the harness for it exists (`--ships 12` prints the
  histogram).
- **Do it LAST.** Tuning weights against a layout that is about to change is
  wasted work, and it is how the previous round of numbers got measured twice.

## 5. What I would NOT do

**Do not put the fact in the tileset.** It is tempting to author two thruster
prototypes, an "aft drive" and a "manoeuvring thruster", and let mating sort them
out. It cannot: mating is binary and the distinction is global, so both would
still land anywhere. It would also put a generator concern into shipped catalog
content that the editor and every scenario have to carry.

**Do not add a scoring-and-rejection pass.** Generate N ships, score each for
"engines at the back, bridge on top", keep the best. It is the obvious answer and
it is a trap: the cost is N collapses per ship, the failure mode is that the
SCORER quietly becomes the real designer while the generator's own rules stop
being the thing that produces the design, and any weighting inside it is invisible
in a way a per-part field is not. The one practitioner example I found of scoring
generated output optimises the generator's PARAMETERS rather than picking among
outputs, which is a different and much cheaper shape. If we ever want this, that
is the version to want.

**Do not rewrite the collapse as a grammar or a graph rewrite.** It would
express front/back/top naturally, and it would throw away the thing this
generator is actually for: the adjacency rules ARE the catalog's link points, so
a hull the generator draws is a hull a player could have built. That property is
the whole point of the example and no amount of layout sense is worth it.

**Do not make the exit-clearance rule directional.** It is correct as it stands,
it is shared with the editor, and a version of it that knows about "aft" would
put a generator's taste inside a rule a player's ship is judged by. Aim belongs
in `Part`, where it is one file's opinion; clearance belongs in `nova_ship`,
where it is physics.

**Do not reach for backtracking.** Every item above is a unary domain filter and
none of them can empty a domain, because vacuum is compatible with everything.
That property is worth defending: a collapse that cannot fail needs no restart
loop, no retry counter and no failure budget. Any proposal that costs it should
have to argue for itself.

## Sources

Repo-local, read first:

- `git show master:tasks/20260815-231945/RESEARCH.md` - section 7, the
  face-normal dispatcher and the guarantee guard.
- `tasks/20260815-190741/NOTES.md` - the constraint-solving and decoration
  findings, and bug 18 on how the parts compete with each other for weight.

External, verified from the primary source:

- Boris the Brave, *Driven WaveFunctionCollapse* -
  <https://www.boristhebrave.com/2021/06/06/driven-wavefunctioncollapse/>
- Boris the Brave, *WFC Tips and Tricks* (fixed tiles, biome filtering) -
  <https://www.boristhebrave.com/2020/02/08/wave-function-collapse-tips-and-tricks/>
- Boris the Brave, *Arc Consistency Explained* (unary constraints are domain
  filters) - <https://www.boristhebrave.com/2021/08/30/arc-consistency-explained/>
- Boris the Brave, *Constraint-Based Tile Generators* (hard rules versus the
  soft heuristic) -
  <https://www.boristhebrave.com/2021/10/31/constraint-based-tile-generators/>
- Boris the Brave, *Model Synthesis and Modifying in Blocks* -
  <https://www.boristhebrave.com/2021/10/26/model-synthesis-and-modifying-in-blocks/>
- Paul Merrell, model synthesis and graph grammars -
  <https://paulmerrell.org/model-synthesis/>, <https://paulmerrell.org/grammar/>
- gridbugs, *Wave Function Collapse* (the weighted-entropy formula) -
  <https://www.gridbugs.org/wave-function-collapse/>
- mxgmn/WaveFunctionCollapse (README: "the algorithm doesn't know about global
  structure") - <https://github.com/mxgmn/WaveFunctionCollapse>
- a1studmuffin/SpaceshipGenerator, MIT -
  <https://github.com/a1studmuffin/SpaceshipGenerator>
- Oskar Stalberg, *Wave Function Collapse in Bad North*, EPC2018 -
  <https://www.youtube.com/watch?v=0bcZb-SsnrA>
