# Round 2: hull plating shape and greeble placement

Scoped market research for the shell-shape work. Four questions, asked in
`TASK.md` under "Round 2". Round 1 is `RESEARCH.md` and is NOT repeated here -
where this round touches it, it says so and says whether it confirms or
contradicts.

## The headline, before the sources

The complaint is an INTERIOR problem, not a boundary problem, and the two can be
separated cleanly:

- CONTINUITY is decided inside the plate. Every plate top is a fan off one apex
  at the mean of the eight boundary samples. Where the boundary is COPLANAR that
  mean is the plane and the fan is already exactly right. Where it is not, the
  middle of the plate is pulled to the mean and the plate reads as a cone, a dish
  or a saddle.
- The DELIBERATE spikes live in the boundary. The all-floor stud and the
  tapering rim are boundary votes. Nothing proposed here touches a vote, so
  nothing proposed here can flatten them.

That split is what makes the work cheap. It is also the thing the literature
agrees on hardest, and the paper that states it most directly is Cubical
Marching Squares (section 1.2).

The fan is right or wrong per shape, and the test is EXACT in integers
(section 4.1). Computed, DERIVED, in cell units:

| Relief | Example samples | Coplanar? | Apex | What the fan gives |
| --- | --- | --- | --- | --- |
| `Flat` | `c 2222 / m 2222` | yes | 0.5 | the plane. Correct. |
| planar ramp | `c 0044 / m 0242` | yes | 0.5 | the plane. Correct. |
| `Brink` | `c 0022 / m 0121` | yes | 0.25 | the plane. Correct. |
| `Bevel` | `c 0222 / m 1221` | no | 0.375 | DISHED 0.125 below the running plane |
| `Step` | `c 2242 / m 2332` | no | 0.625 | DISHED 0.125 below the running plane |
| `Spur`, 3 fallen | `c 0002 / m 0011` | no | 0.125 | a low cone |
| `Spur`, diagonal | `c 0202 / m 1111` | no | 0.25 | the bilinear SADDLE - the owner's "0101" |
| `Ridge` (tent) | `c 0000 / m 2020` | no | 0.125 | crest 0.5, middle SAGS to 0.125 |
| `Peak` (stud) | `c 0000 / m 0000` | yes | 0.5 | a cone, DELIBERATELY - `volume()` overrides to half a cell |

Two of the wrong ones have an EXACT alternative interpolant of the same eight
samples - the tent, and the saddle, which has two (gable or valley). Those cost
nothing in fidelity and nothing in the neighbour contract. That is the single
most actionable finding in this round.

And one source challenges the whole premise: the only writeup found that
addresses "chunky detail on lumps looks wrong" head-on says the answer is to
RECESS chunky detail into wells and trenches, and to let only flat detail sit on
top of anything. If that holds, a flat seat is the wrong thing to build.
Section 3.6, and R9 is the cheap test.

CORRECTION to my own first reading, recorded because it changes the ranking: it
is NOT true that most of the hull is cones. `Flat` and `Brink` are already
coplanar and already correct, and `Brink` is the one a style comment calls "THE
SIGNATURE". The defect is confined to `Bevel`, `Step`, `Spur` and `Ridge`. I
could not measure their combined share - see the closing section.

## How to read this

Round 1's rules apply. Every source carries its licence. Share-alike (GPL,
CC-BY-SA) and no-licence repositories are flagged loudly as UNUSABLE for code,
readable for ideas. Nothing is committed beside this file.

Two extra labels are used, because the brief asked for them:

- DEMONSTRATED - the source shows the thing working, with an image, a
  measurement or shipped code.
- CLAIMED - the source asserts it.

Arithmetic in section 4 is DERIVED here and marked so. It was not taken from a
source and the implementer should re-derive it rather than trust it.

## 0. The code reading this round rests on

Verified in the worktree, not taken from the brief. Line numbers will drift.

- `shell_shape.rs:311` `ShellShape::surfaces` builds the top as eight triangles
  fanned off `Vec3::new(0.0, floor + self.centre_height(), 0.0)`. ONE apex.
- `shell_shape.rs:273` `centre_height` IS `volume`, and `volume` is the mean of
  the eight boundary samples. The two are deliberately one function.
- The module note gives the reason, and it is a good one: the mean is "the one
  choice that leaves a ramp flat". Section 4 confirms it - the mean of the eight
  samples is the least-squares plane through them, evaluated at the cell centre,
  so on a coplanar boundary the fan reproduces the plane exactly.
- `shell_skin.rs:395` `boundary_heights` votes corners FULL / HALF / 0 and takes
  midpoints as the exact mean of their corners, except on a dead edge.
- `shell_shape.rs:279` states the contract in the source: the boundary polyline
  "is the whole contract with the neighbours".

Two facts the brief did not have, and they change the ranking:

1. **The decoration sits on the apex.** `skin_decor.rs:411` `decor_pose` lifts
   every piece to `-REACH + plate.shape.volume()` and puts it at the plate
   CENTRE with a quantised yaw. So today a greeble is a flat-bottomed model
   balanced on the tip of a cone. That is the direct mechanical cause of
   "decorations get placed on ridges and spikes and they look weird" - it is not
   a scatter-rule problem at all.
2. **The vocabulary already names the saddle.** `skin_reading.rs:462`
   `relief_of` classes the diagonal fallen mask `0b0101` / `0b1010` as `Spur`,
   with the comment "the diagonals, are saddles and fall two ways". The owner's
   "0101" is that mask exactly.

And the measurement that partly sizes the prize, from `skin_reading.rs`'s module
note: the generator's hulls come out "four fifths FALLING PLATE and a seventh
`Step`, with `Flat` under a seventh and `Peak` absent". Falling plate is
`Bevel` + `Brink` + `Spur` together, and the note does not split them. Since
`Brink` is coplanar and already correct, the affected share is somewhere between
"a seventh" and "four fifths" and **this lane could not narrow it**. The styles
in `crates/nova_authoring/src/base_content/styles.rs` ask for
`relief: vec![Flat, Bevel]` for their largest pieces, and `Bevel` is one of the
broken ones.

## 1. Plate top geometry

### 1.0 The general answer: nobody picks a better centre point. They declare a REGION.

Three independent sources converge, and this is the answer to question 1. Every
system that solves "fixed boundary, free interior" **partitions the tile into a
constrained band plus a free region, and forbids the interior rule from touching
the band.**

- **Lagae and Dutre 2006, section 7.1** - the interior half of the corner-tile
  paper round 1 already banked, which round 1 did not cover. Verbatim: "The
  Poisson disk radius divides a tile into corner regions, edge regions, and an
  interior region ... **the corner regions are slightly enlarged in order to make
  the distance between edge regions 2r. Hence, points in edge regions only affect
  points in corner regions.**" Build order is corners -> edges -> interior, and
  edge construction may not touch corner data.
  <https://doi.org/10.1145/1183287.1183296>
- **Neyret and Cani 1999** - round 1 banked the zero-gradient-at-the-vertex rule;
  here is its REASON and its band. Reason: "As soon as a mesh node on a curved
  surface can be shared by an arbitrary (small) number of neighboring patches ...
  no global orientation can be defined ... this enforces a zero texture gradient
  at those points". Band: "this operation has to be done within **two ranks of
  cells** surrounding the corner ... since the noise values in the second rank of
  cells may influence the texture gradient there." Interior: "the noise values of
  the **inner unconstrained region** of each triangle are chosen at random."
  <https://doi.org/10.1145/311535.311561>. Their "no noticeable visual artifacts"
  line is a CLAIM, not a measurement.
- **omega-tiles** (Ng, Wen, Tan, Zhang, Kim, CGI 2005) - the closest published
  analogue of "keep the boundary, rebuild the middle". The cutting curve passes
  "through the middle points of the four sides" and "is also restricted to lie
  inside a (pink) circle with the same center as I and having the diameter equal
  to the width of I". Free region = the inscribed circle; fixed region = the
  corner-bearing annulus. <https://doi.org/10.1109/CGI.2005.1500411>. OUT as
  machinery - graph cut plus Poisson blending, offline, image domain.

**The rule worth taking, and it removes a magic number from R4:** in the good
sources the band width is never guessed. It is **the reach of whatever could
perturb the boundary** - Lagae and Dutre size it by the Poisson radius, Neyret
and Cani by how far a noise value can influence a gradient. Nova's analogue is
how far inboard a vertex can move before it changes a triangle that touches a
boundary sample. Derivable, not a taste parameter.

Supporting, from the marching-squares side: "Marching Squares always creates
straight lines on the interior of any cell"
(<https://www.boristhebrave.com/2018/04/15/dual-contouring-tutorial/>). A flat
plateau with the fan pushed out to a rim is INSIDE the algorithm's contract, not
a hack.

### 1.0a The closest documented analogue, with Nova's complaint as its motivation

Catlike Coding's Unity Hex Map builds each cell as a **flat solid inner core** -
a "solid factor" of 0.75 of the outer radius - plus an **outer ring** carrying
all of the neighbour transition. The motivation is the owner's complaint one
dimension down: "Blending across the entire surface of a hexagon leads to a
blurry mess. You can no longer clearly see the individual cells."
<https://catlikecoding.com/unity/tutorials/hex-map/part-2/>

Part 3 adds terraces - a stepped chamfer in the ring - and states a gotcha that
is Nova's cone in miniature: "the Y coordinate must only change on odd steps, not
even steps. **Otherwise we wouldn't get flat terraces.**"
<https://catlikecoding.com/unity/tutorials/hex-map/part-3/>

DEMONSTRATED, with shipped code and images. **Licence split, and it matters:**
code and assets are **MIT-0** and vendorable; the tutorial **text, screenshots
and diagrams are CC BY-NC-SA 4.0** - do not paste prose or figures.
<https://catlikecoding.com/license/>

Best precedent found for R4, and it supplies a starting inset: a solid core at
0.75 of the radius, the remaining quarter doing all the matching.

### 1.1 The apex is not always a point

The closest prior art to Nova's exact construction is **Extended Marching Cubes**
(Kobbelt, Botsch, Schwanecke, Seidel, SIGGRAPH 2001) and **Cubical Marching
Squares** (Ho, Wu, Chen, Chuang, Ouhyoung, Eurographics 2005). Both build a cell
surface as a TRIANGLE FAN off a single interior point, exactly as Nova does. What
they do differently is CHOOSE that point.

CMS, section 3.4, verbatim from the paper text:

> If there is a sharp feature p in a component consisting of the vertices
> v1,...,vn ..., we use p as the center to create a triangle fan with triangles
> pv1v2, pv2v3, ..., pvnv1. If there is no component sharp feature, we calculate
> the average point of all sample points on this component and use it as the
> center to generate the triangle fan.

Nova does the second branch unconditionally. **Fanning off the average is the
literature's NO-FEATURE fallback**, and Nova applies it to every plate including
the ones that have a feature. That is the defect stated in the field's own terms.

The feature point itself comes from EMC section 4.3: place the new sample at the
intersection of the tangent elements, i.e. solve

```
[..., n_i, ...]^T p = [..., n_i^T s_i, ...]
```

by the pseudo-inverse from the singular value decomposition. DEMONSTRATED - the
paper's Figure 7 comparison shows MC, EMC, DC and CMS on a tetrahedron and CMS
recovers the sharp edges the others round off, and Table 2 gives lower geometric
error for CMS on every marching-cubes case.

- Kobbelt et al. 2001, "Feature Sensitive Surface Extraction from Volume Data":
  <https://www.graphics.rwth-aachen.de/media/papers/feature1.pdf>. ACM
  copyright. READ ONLY - no text or figure reuse, ideas are free.
- Ho et al. 2005, "Cubical Marching Squares: Adaptive Feature Preserving Surface
  Extraction from Volume Data": <http://graphics.csie.ntu.edu.tw/CMS/> (the
  project page's TLS certificate has expired; the PDF at
  `http://graphics.csie.ntu.edu.tw/CMS/download/cms-eg2005.pdf` fetches).
  Eurographics / Blackwell copyright. READ ONLY.

### 1.2 CMS is Nova's architecture, and says the interior is free

CMS unfolds a cube into its six faces, solves 2D marching squares on each face
independently, folds them back and traces the segments into components. The
paper's own words on the interior, section 3.2:

> The triangulation can be chosen arbitrarily as long as it is consistent.

And on why sharing the face is what buys crack-freedom, section 3.4:

> all edges on the transition faces are generated from segments and every
> segment is exactly shared by two components from two neighboring cells. Hence,
> the resulting mesh is guaranteed crack free.

That is `shell_shape.rs:279` restated by a peer-reviewed paper: **the shared face
data is the contract, the interior is unconstrained**. Nova arrived at the same
architecture independently. This CONFIRMS finding 3 of the brief with an outside
authority, and it means an inset plateau, a ridge, or any other interior is
legal by construction and needs no seam geometry and no test beyond "the
boundary ring is still `boundary()`".

CMS also names the failure Nova would hit if it tried to make a feature run
ACROSS plates. EMC has to flip an edge to connect two cells' features, which puts
triangles outside their own cell - the paper calls this **inter-cell
dependency**, and says DC has "a similar drawback". CMS removes it by sampling a
sharp feature ON THE SHARED FACE. Read into Nova: a ridge whose ends are
BOUNDARY SAMPLES is automatically consistent in position, because both plates
compute those samples; a ridge whose ends are anywhere else is not. Any interior
feature must terminate on `boundary()` points.

### 1.3 Open-source CMS implementations, with licences

Found by GitHub repository search. None is a drop-in - they are volume
extractors, not tile builders - but they are legally readable specs for the
fan-off-a-solved-feature construction.

| Repo | Licence | Verdict |
| --- | --- | --- |
| `ZachHembree/GreedyCubicalMarchingSquares` | **MIT** | Usable. Readable and adaptable. |
| `metalisai/Aviz.Cms` | **Apache-2.0** | Usable. NOTICE requirement. |
| `sidit77/CMS` | **MIT** | Usable, tiny, author calls it rough. |
| `TheCyberBrick/Unity-Cubical-Marching-Squares-Prototype` | NOASSERTION | **UNUSABLE.** GitHub could not identify a licence - treat as all rights reserved. |
| `TheWiseLion/CubicalMarchingSquares` | none | **UNUSABLE.** No licence file. |

### 1.4 Do NOT go dual contouring. Two reasons, both fatal.

**1. It breaks Nova's independence guarantee outright.** "Unlike Marching Cubes,
we cannot evaluate cells independently. We must consider adjacent ones to 'join
the dots'." The whole reason Nova needs no seam geometry is that a plate is a
pure function of its own neighbourhood; DC gives that up.

**2. It does not deliver flatness anyway.** On planar input the QEF is
rank-deficient, and Garland and Heckbert say it in the source paper for the
metric: "**parallel planes (e.g., around a planar surface region) will produce
level surfaces which are two parallel planes**", with the matrix invertible only
"as long as the level surfaces are non-degenerate ellipsoids"
(<https://www.cs.cmu.edu/~garland/Papers/quadrics.pdf>). Boris the Brave adds
"there's no actual guarantee that the resulting point is inside the cell". So
planar input gives an underdetermined vertex and an arbitrary tie-break -
exactly the wrong behaviour for a system whose commonest case is flat plate.

CMS names the resulting artefact directly: without feature preservation "**a flat
surface might become wavy**".

One STRUCTURAL caveat worth recording honestly, because it limits how much of
this literature transfers: DC, Surface Nets and CMS all key off a SIGN CHANGE. A
cell with equal corner samples emits no geometry at all. So the literal question
"does any of them guarantee a flat interior when the corners agree" has no answer
there - their guarantee is a BOUNDARY guarantee, not an interior-flatness one.
Nova is asking a question the isosurface literature does not ask.

### 1.4a The centre-bias trick, which does transfer

DC is out as an ARCHITECTURE, but one of its implementation details transfers to
whatever solves a Nova interior point. Dual contouring is Ju, Losasso, Schaefer
and Warren, SIGGRAPH 2002. **The primary PDF could NOT be reached** -
`cs.wustl.edu` did not resolve and two mirrors 404'd - so the following is from a
practitioner secondary source and is labelled as such.

Boris the Brave, "Dual Contouring Tutorial"
(<https://www.boristhebrave.com/2018/04/15/dual-contouring-tutorial/>, blog text
copyright, analysis only). CLAIMED, with worked code:

- the per-cell vertex minimises a quadratic error function over the boundary
  Hermite data - "the point that is most consistent with the normals";
- the QEF "doesn't actually work very well" when the normals are near colinear,
  which happens **on large flat surfaces**, and the minimiser then falls outside
  the cell;
- the fixes are a CONSTRAINED solve that forces the answer inside the cell, plus
  a BIAS toward the cell centre;
- failure modes named: self-intersection (dismissed as "completely ignorable")
  and non-manifold output where two surfaces share a cell.

EMC independently gives the same centre-bias trick, section 4.3: they translate
the samples so their centre of gravity is at the origin before solving, "in
order to guarantee that this point lies in a reasonable configuration to the
samples".

Read into Nova: a solved interior point must be CLAMPED to the plate and biased
to the cell centre, or a plate with a strong gradient gets an apex somewhere
silly. Nova's fixed footprint makes the analogous plane fit well conditioned
(section 4.1), so this is a guard rather than a live risk - but it is the guard
the field says to write.

### 1.5 The strongest warning found, and it contradicts the obvious implementation

EMC section 4.3, "Remarks", on classifying a cell as flat / edge / corner:

> It is tempting to try to read off the feature classification of the local
> configuration directly from the magnitude of the singular values. However it
> turns out that this is a very unreliable criterion since the singular values
> not only depend on the angles between the normals but also on their
> distribution.

They give the counterexample: `[n0,n0,n0,n0,n0,n1]` and `[n0,n0,n0,n1,n1,n1]`
have very different singular value distributions and the same feature.

The obvious Nova implementation is "fit a plane to the eight samples, threshold
the residual, and if it is large do something else". **EMC says that specific
move is unreliable and to classify STRUCTURALLY first, then solve.** In Nova's
terms: classify by the boundary's equality and fallen-corner pattern - the data
`relief_of` already computes - and only then compute geometry. This AGREES with
round 1's banked decoration finding, "canonicalise by EQUALITY PATTERN, not by
value", arrived at from a completely different direction.

CONFIDENCE: high. Falsifier: an implementation that thresholds a residual and
produces stable, correct classes over the 12720-shape test spread would show the
warning does not bite at Nova's tiny, exact, integer alphabet - which is a fair
argument, since EMC's caution is about noisy scanned gradients.

### 1.6 Three shipped tile engines: the saddle is legal TERRAIN and illegal GROUND

This is the best-supported finding in the round, and it converges from three
independent codebases. All three are share-alike: **read the idea, never vendor
the code.**

**OpenTTD, GPL-2.0.** <https://github.com/OpenTTD/OpenTTD>

- `src/slope_type.h` enumerates the slopes: `SLOPE_FLAT`, four one-corner, four
  two-adjacent-corner, `SLOPE_EW` and `SLOPE_NS` (the two OPPOSITE-corner cases),
  four three-corner, the steep variants, and `SLOPE_HALFTILE_*` commented "one
  halftile is leveled (non continuous slope)".
- The alternating-corner case IS a legal terrain slope. But it is not BUILDABLE:
  `src/rail_cmd.cpp`'s `_valid_tracks_without_foundation` allows all track on
  flat, only three specific tracks on a single raised corner, and **nothing on
  `SLOPE_EW` / `SLOPE_NS` without a foundation**. `CheckRailSlope` computes the
  foundation required and rejects what no foundation can fix.
- `src/slope_func.h`'s `FlatteningFoundation()` returns "either
  `Foundation::None` if the tile was already flat, or `Foundation::Leveled`",
  and `Foundation::Leveled` is commented "The tile is leveled up to a flat
  slope."
- Bonus technique: `src/terraform_cmd.cpp` holds the height invariant ("Is the
  height difference to the neighboured corner greater than 1?") by **bounded
  recursive propagation over the edit**, not by a global relaxation over the
  map. That is the shape of a legal answer for a system with Nova's constraint.

**Simutrans, Artistic License 1.0.** This is the one that settles it.
`src/simutrans/dataobj/ribi.h` stores corners as BASE-3 digits (weights 1, 3, 9,
27), so 81 representable slopes, and `ribi.cc` carries an explicit
`const int slope_t::flags[81]` table. Of the 81, only 15 carry `way_ns` or
`way_ew`: flat, all-up-1, all-up-2, and the uniform inclines.

> **Slope 10 (`ne1, sw1`) and slope 30 (`nw1, se1`) - the two-opposite-corner
> saddles - have flags `0`. Not buildable in either direction. The single-corner
> slopes 1, 3, 9 and 27 have flags `0` too.**

So Simutrans excludes BOTH the saddle AND the lone raised corner from the
buildable set, keeping only flat and uniformly inclined tiles. It has the same
foundation concept (`src/simutrans/ground/fundament.h`).

**OpenRCT2, GPL-3.0-or-later.**
`src/openrct2/world/tile_element/Slope.h` gives the alternating case a NAME:
`kTileSlopeWEValley = E|W` and `kTileSlopeNSValley = N|S`. A first-class shape,
not an accident.

**CORRECTION to my own first reading.** My initial pass had OpenTTD alone as a
NEGATIVE result - "nobody forbids the saddle". That was wrong, and it was wrong
because I only read the slope enum. The field's actual answer is a two-level
one, and it is better than either extreme:

> The awkward slope is allowed to EXIST as terrain. It is forbidden from
> CARRYING anything. When you must build there, you insert a levelled foundation
> and build on that.

Read into Nova, that is three separate levers, and Nova already has two of them:

1. refuse to decorate the ugly plate - `PlateRelief` filtering, which the style
   system already does;
2. give it a levelled seat and decorate that - the plateau, R4;
3. leave the plate shape alone either way - which is what the boundary contract
   forces anyway.

CONFIDENCE: high on the facts, medium-high on the analogy. Falsifier: a Nova
plateau that reads as a separate object bolted onto the plate rather than as the
plate's own top - which is exactly what an OpenTTD foundation deliberately DOES
read as, and Nova does not want.

**COUNTER-EVIDENCE, recorded because it cuts against R3.** OpenRCT2 calls the
alternating case a VALLEY, which is the resolution that joins the LOW corners -
the opposite of the gable the owner asked for. One engine's naming is weak
evidence, but it is evidence that the other reading is defensible, and it is the
cheapest thing to A/B: both are exact interpolants (section 4.3), so rendering
gable and valley side by side costs one extra branch.

### 1.7 What breaks, for Nova specifically

Enumerated from the code, not from a source. Ranked by how likely it is to bite.

1. **`volume() == centre_height()` stops being true.** They are one function on
   purpose (`shell_shape.rs:249-275`). Change the interior and the mean of the
   eight samples is no longer the height of the middle, and it is no longer the
   solid's true volume either. DERIVED: a true gable's mean height over the cell
   is `2H/3` where the mean of the eight samples is `H/2`, so the sample mean
   under-reads by 25%; for the tent it under-reads by 50% (0.125 against 0.25).
   Three callers care - the collider height, the health scale, and
   `PlateReading::height`.
2. **`decor_pose` puts the greeble in the wrong place, or inside the plate.**
   It lifts to `volume()`. On a corrected tent the surface at the centre is at
   0.5 and the piece would sit at 0.125 - BURIED. This must change in the same
   commit or the fix makes decoration worse, not better.
3. **Silhouette changes on the tent.** Round 1's inviolable rule is "model
   whatever changes the outline". A corrected tent raises the middle of a
   one-cell spine by 0.375 cells, which IS an outline change. It is the intended
   one, but it must be looked at in a matched-pose render rather than assumed.
4. **Non-planar skirt quads, and the diagonal choice is a real engine-level
   knob.** A ring quad between an inset plateau and the boundary has four points
   that are generally not coplanar, so which way it splits is a visible decision
   under flat shading. Prior art exists and one piece of it is vendorable:
   - **Bullet** exposes the whole policy set in one line - fixed diagonal,
     checkerboard by `x+z` parity (`m_useDiamondSubdivision`), or alternate by
     row parity (`m_useZigzagSubdivision`), in
     `btHeightfieldTerrainShape.cpp`. **Licence zlib, vendorable.**
     <https://github.com/bulletphysics/bullet3>
   - **PhysX 5** makes it a per-sample flag: "The flag and materials refer to the
     cell below and to the right of the sample point, and indicate along which
     diagonal to split it into triangles."
   - **Blender Triangulate** names the canonical policies: Beauty, Fixed, Fixed
     Alternate, Shortest Diagonal, Longest Diagonal.
   - The only STATED reason for zigzag anywhere is interop - Bullet's comment
     says it "could help compatibility with Ogre heightfields", i.e. a mismatched
     diagonal convention between renderer and physics is a real shipped bug. That
     is worth knowing here, because Nova's plate mesh and its collider are built
     separately.
   - **NOT documented anywhere reachable:** any authoritative source naming
     "directional ridging" or "herringbone" as the artefact, or showing a
     before/after. "Shortest diagonal is Delaunay" and "the saddle becomes a
     ridge or a valley depending on the split" are derivations, not citations.
5. **The waffle risk, and it is the one that would sink the whole idea.** An
   inset plateau plus a chamfer on EVERY plate would make the cell grid visible
   over the whole hull, which is worse for continuity than the cones - it would
   answer the owner's complaint with a different one. The case table in R4
   avoids it by construction: the coplanar row leaves `Flat`, `Brink` and every
   planar ramp geometrically IDENTICAL to today, so no chamfer appears anywhere
   the surface is already continuous.
6. **Not a risk, checked:** z-fighting (no coplanar overlap is introduced -
   and this was searched for, with zero results on graphics.SE for every
   phrasing; mechanically a plateau and its chamfer are not coplanar, so
   z-fighting is not the expected failure), normal seams (`MeshFaces` already
   emits one vertex per corner per facet, so every facet keeps its own normal),
   T-junctions (new vertices are strictly inside the boundary, and the floor fan
   already uses the same eight footprint points), sockets (the socket rule reads
   midpoint samples, which do not move), triangle count (top goes from 8
   triangles to about 24 - at ~150 plates a ship that is +2400 triangles, which
   round 1 already calls nothing).
7. **What people who have actually shipped an inset ring report.** These are real
   bug reports and shipped mitigations, and they replace what was a gap:
   - **Self-intersection when the inset is large relative to the space
     available.** The existence proof is Blender Bevel's **Clamp Overlap**:
     "Limits the width of each beveled edge so that edges cannot cause
     overlapping intersections with other geometry." Engine-level answer: clamp
     the inset per edge against the local budget, do not use a constant.
     <https://docs.blender.org/manual/en/latest/modeling/modifiers/generate/bevel.html>
   - **The plateau/chamfer shading seam is a named problem with a named fix that
     costs per-vertex data.** Blender **Harden Normals**: "the per-vertex face
     normals of the bevel faces are adjusted to match the surrounding faces ...
     For this effect to work, a mesh must have a custom split normals
     attribute." Nova is flat-shaded, so it gets a hard crease and needs none of
     this - but that is a decision, not an accident.
   - **Corners where three or more beveled edges meet need dedicated handling.**
     Blender's Intersections offers Grid Fill and Cutoff, the latter existing
     "when the new intersection is too complex for a smooth grid fill". Nova's
     plate corners are exactly those junctions: the inset is cheap along an edge
     and expensive at a corner.
   - **Ring geometry breaks picking, raycasts and colliders.** gazebo-classic
     #2315: "the terrain skirts are actually not part of the terrain surface and
     so the ogre terrain's ray intersection function we use doesn't return any
     results." Nova shoots at plates, so this one is live.
   - **Ring geometry casts wrong shadows.** CesiumGS/cesium #11459: "the skirts
     contribute in casting shadows causing shadows artifacts. This is better
     visible when the light source is tangent to surface" - and disabling them
     gives "worse tiles edges connection".
   - **The ring becomes visible from the wrong side.** Cesium ships
     `Globe.showSkirts` and documents that "Skirts are always hidden when the
     camera is underground or translucency is enabled".
   - **NOT found:** any primary source quantifying triangle-count blowup,
     collider cost or silhouette change for an inset ring.
8. **Canonicalisation.** Any interior rule must be a pure function of the
   CANONICAL shape or the per-shape mesh cache breaks. A shape with C2 symmetry
   has two canonical turns; they must agree about which diagonal a ridge runs
   along. For `[0,h,0,h]` and the tent they do - a half turn maps each diagonal
   to itself - but that is an argument, and it wants a test.

### 1.8 Stalberg's tiles: hand-authored, and the combinatorial lesson

The gap I had flagged as this round's likeliest miss is partly closed. There is
no reachable Stalberg talk, but Boris the Brave's deconstruction of **Planet** is
semi-primary - he states "I got most of the details from twitter threads and
discussions with Oskar himself".
<https://www.boristhebrave.com/2022/12/18/how-does-planet-work/>

- "In his earlier project **Brick Block**, the selection process that takes data
  stored on base vertices ... **is simply Marching Cubes**."
- "**there is no procedural mesh generation in the game, everything is made by
  sticking together pre-authored modules**."
- **The lesson that lands hardest on Nova's five-value ladder**, verbatim on why
  they did NOT author one mesh per case: "There's 8 possible heights for the
  terrain and 4 different terrains ... for each of 3 corners ... You'd need to
  construct (8x4)^3 different tiles ... **this is too many tiles to manually
  author, or even for the game to load.**" Their fix was to decompose into
  independent BINARY layers.
- They also cut the case count by changing the CELL: "triangle prisms ... Having
  only 6 corners instead of 8 means much fewer combinations."
- Useful escape hatch for decoration, and it is a different answer from R8:
  "vertex meshes" hung off the LATTICE rather than off a tile - "Doing this helps
  'break the grid'". It costs overlap.

**Do NOT credit this source with a flatness rule.** The article says nothing
about keeping faces flat. What it supports is Nova's existing decision to BUILD
meshes rather than author them: at five heights and eight samples the authored
route is arithmetically impossible, which is the same conclusion
`20260815-190741/NOTES.md` reached by deleting 41 authored `.glb` tiles.

### 1.9 Author on a canonical cell then warp - and the trap in it

Sylves (**MIT**, vendorable) formulates Nova's contract cleanly: "Each
deformation is a continuous map, which maps a cell from its canonical shape ...
via linear/bilinear or trilinear interpolation".
<https://github.com/BorisTheBrave/sylves>. On a shared edge the bilinear map
collapses to linear in the two shared corners, so both cells agree - which is the
neighbour guarantee stated as a map rather than as a sample set.

**TRAP, and it is the round's neatest convergence.** A bilinear warp does NOT
preserve flatness: a flat interior authored on the canonical square becomes a
hyperbolic-paraboloid patch whenever the target corners are non-coplanar. That is
section 4.2's saddle, arrived at from the opposite direction. So "author one flat
tile and warp it to the corner heights" is a dead end for exactly the reason the
current fan is - and this is an INFERENCE, flagged as such by the lane that
found it, not a sourced claim.

### 1.10 Winged tiles: found, and they are the opposite strategy

Round 1 banked "winged tiles (Carlson)" as prior art worth knowing. Having now
read it: **it does not apply, and it should be struck from the shortlist.**

Christopher Carlson, "Multi-Scale Truchet Patterns", Bridges 2018
(<https://archive.bridgesmathart.org/2018/bridges2018-39.pdf>): "A winged tile
consists of the content of the tile ... plus 'wings' that complete the motif
outside of the boundary ... Winged tiles are assembled along their square content
boundaries with the wings **overlapping** adjacent tiles."

It hides joins under OVERLAP instead of matching boundaries. OUT twice over: it
needs a draw order ("smaller tiles are always placed on top of the larger"), and
overlap does not work in watertight 3D. It is 2D generative graphic design, has
its own documented failure at region boundaries ("ugly unevenness"), and 3D
appears only in a future-work sentence. The Wolfram package has **no stated
licence**.

### 1.11 Flat-region preservation: all the classic tools are OUT except one lever

| Technique | RNG | Global pass | Verdict |
| --- | --- | --- | --- |
| Garland-Heckbert QEM | no | yes, greedy queue | OUT |
| meshoptimizer `simplify` | no | yes, whole-mesh | OUT unless run PER PLATE with `LockBorder` |
| Blender Limited Dissolve | no | no, local angle threshold | compatible in principle; GPL code |
| Variational Shape Approximation (Cohen-Steiner 2004) | seed-dependent | yes, Lloyd iteration | OUT |

- QEM demonstrably dissolves flat regions - "the initial error estimate for each
  vertex is 0, since each vertex lies in the planes of all its incident
  triangles" - but the collapse order is global and greedy.
- **The one usable lever**: `meshopt_SimplifyLockBorder` "restricts the
  simplifier from collapsing edges that are on the border of the mesh", which
  lets Nova simplify a plate while keeping its boundary polyline byte-identical.
  meshoptimizer is **MIT and vendorable**. Two cautions from its own README: it
  does NOT claim flat-region preservation (flatness falls out of the quadric
  metric), `meshopt_simplifySloppy` "doesn't preserve attribute seams or borders"
  and is unusable here, and the simplifier can get "stuck" on faceted meshes
  unless "identical vertices are 'welded' together" - which Nova's per-facet
  vertex duplication guarantees they are not.
- Blender Limited Dissolve produces N-GONS, which puts you straight back into the
  diagonal-choice problem of section 1.7 item 4.

## 2. Continuity over spikes

### 2.1 The Nova-specific answer, and it is the same change

DERIVED, not sourced: on the four broken classes the apex is pulled off the
surface the boundary describes, so every one of those cells carries a bump or a
dimple in its middle. `Bevel` and `Step` are dished by an eighth of a cell,
`Spur` at a spar tip is a low cone, the diagonal `Spur` is a saddle and the tent
sags by three eighths. A field of those, at one per cell, is a texture of noise
laid over the plating - which is what "spikes all over the place" describes.
Give each of them the surface its own boundary already implies and a run of them
becomes continuous, with a fold at the cell boundary instead of a lump in the
middle of it.

The deliberate spikes are untouched because they are BOUNDARY facts:

- the all-floor stud is `volume()`'s half-cell fallback plus an all-zero
  boundary;
- the tapering rim is `ends_against` voting a corner to the floor.

Neither is an interior rule. CONFIDENCE: medium-high on the diagnosis, medium on
"this alone satisfies the owner". Falsifier: a matched-pose A/B render over the
default row - with `freeze_bodies`, per bug 15 in `20260815-190741/NOTES.md` -
that still reads as spiky after the interior change. That render is the single
cheapest experiment available and it should be run before anything else in this
document is built.

### 2.2 The blob tileset's legality rule, and why Nova mostly has it already

The canonical statement of "which neighbourhood configurations are legal" is
cr31's blob tileset - a 47-tile subset of the 256 two-edge two-corner Wang
tiles. Original site dead; Boris the Brave hosts the mirror at
<https://www.boristhebrave.com/permanent/24/06/cr31/stagecast/wang/blob.html>.
(Mirror of a dead site, provenance unverifiable - treat as secondary. Boris the
Brave's own articles are blog text, copyright, analysis only.)

The constraint, verbatim: **"no tile has a blue edge between two yellow
corners"** - an edge may be unfilled only if both corners either side are
unfilled. The construction is purely local and deterministic: an EDGE becomes 1
only if both tiles sharing it are 1; a CORNER becomes 1 only if all four
surrounding edges are 1.

Two readings, and the second matters more:

- CONFIRMS the architecture. The reason a blob run reads as continuous is not
  smoothing, it is that two adjacent filled cells can never select tiles that
  disagree on their shared edge. That is Nova's shared-sample property, arrived
  at from tile art rather than from isosurfaces. And the ISOLATED case needs no
  special handling - bitmask 0 is simply one of the 47, drawn as its own thing.
  Nova's stud is the same idea.
- **But do NOT "adopt" the rule as if it were new.** Nova's corner vote already
  requires all four incident cells to `ends_against` before a corner leaves the
  floor, which is the blob's corner clause in Nova's own terms - it is why a
  diagonal-only touch produces no raised corner and no blade. The one real
  structural difference is DIRECTION: the blob derives corners FROM edges, Nova
  derives midpoints FROM corners. Reversing that is a change to the vote, and
  this round found no evidence it would improve anything.

CONFIDENCE: medium. Falsifier: a worked case where Nova's derivation produces a
configuration the blob rule would forbid. I did not find one, but I did not
enumerate exhaustively either.

### 2.3 The clearest shipped "isolated versus run" rule: Minecraft walls

<https://minecraft.wiki/w/Wall> - **secondary source**, a community wiki, not
Mojang documentation; behaviour is observable but I did not check decompiled
code, and the wiki's text licence was not verified, so this is analysis only.

A wall block has `up` plus `north/south/east/west` in `{none, low, tall}`, and:

> "A wall block has a center post ... unless connecting to only two opposite
> sides or all four sides."

That is exactly the behaviour the owner asked for, expressed as a pure
four-neighbour predicate:

- part of a straight RUN, or a full crossing -> no post, reads continuous;
- isolated, an L-corner, or a T-junction -> a post, reads as a deliberate
  pillar.

Minecraft stairs do the same for corners: a `shape` of
`{straight, inner_left, inner_right, outer_left, outer_right}` chosen by what the
neighbouring stairs present, and "right side up stairs do not join with upside-
down stairs".

Read into Nova: the deliberate spike is not a special case bolted on, it is the
zero-connection member of the same local rule that makes runs continuous. Nova's
stud already is that. The lesson is that no extra suppression mechanism is
needed - the vocabulary carries it.

### 2.4 Morphological opening, and the one form of it that is legal here

The named technique for "remove small protrusions, keep large features" is
morphological OPENING - erode then dilate. It is anti-extensive, increasing and
idempotent (<https://en.wikipedia.org/wiki/Opening_(morphology)>, CC-BY-SA -
facts free, text not reusable). The median filter is the cheap despeckle with
the same character, and unlike a blur it preserves edges.

Named in a permissive volumetric library, which makes it readable:

- **OpenVDB, Apache-2.0** (<https://github.com/AcademySoftwareFoundation/openvdb>,
  NOTICE requirement). `openvdb/tools/LevelSetFilter.h`'s header names
  "morphological operations (e.g., morphological opening)" alongside Laplacian
  flow and mean-value filtering. `openvdb/tools/Morphology.h` has
  `dilateActiveValues` / `erodeActiveValues` with `NN_FACE` (6), `NN_FACE_EDGE`
  (18) and `NN_FACE_EDGE_VERTEX` (26) neighbourhoods, and the design note "the
  morphological operations only change the state of voxels, not their values".
- scikit-image's `skimage.morphology` has `isotropic_opening`,
  `isotropic_closing`, `remove_small_objects`, `remove_small_holes`. Licence
  BSD-3-Clause by convention, NOT verified this session.

**The constraint test, and it is the useful part.** Opening at radius r is a
pure function of the r+1 neighbourhood. So it is legal here IF it is folded into
what each sample already reads - you widen the neighbourhood, you do NOT add a
pass. Anything that needs a global sweep is out, and that rules out the obvious
neighbours:

- connected-component size thresholds (`remove_small_objects`, and Sebastian
  Lague's `GetRegions` flood fill in Procedural-Cave-Generation, **MIT**,
  <https://github.com/SebLague/Procedural-Cave-Generation>) - unbounded flood
  fill, **OUT**. His `SmoothMap` 8-neighbour majority rule is local and IS legal.
- Voxel Farm's grid relaxation (Miguel Cepero,
  <https://procworld.blogspot.com/2010/11/just-relax.html>, ~7 iterations of
  corner averaging) - iterative global pass, **OUT**, and it is exactly what the
  brief already rules out. Worth recording that a sweep of his archive found no
  rule for suppressing single-voxel noise at all; his whole answer to "voxels
  look like voxels" is relaxation plus dual contouring.
- Wave Function Collapse (<https://github.com/mxgmn/WaveFunctionCollapse>,
  **MIT**) - RNG plus global propagation, **OUT**. Round 1 already said so.

NEGATIVE RESULT worth recording: no shipping game, postmortem or engine was found
that NAMES morphological opening applied to a voxel hull or to game terrain. The
name lives in image processing and volumetric libraries only.

### 2.5 An existing, deterministic knob for "isolated versus deliberate"

`PlateReading::enclosure` counts how many of the eight in-plane neighbours the
surface carries on into: 8 is the middle of a field of plate, 0 is a lone stud.
It is already computed, already deterministic, already a pure function of the
structure. Any rule of the form "flatten where the surface continues, keep the
point where it does not" can be written against it with no new derivation and no
relaxation pass.

That is morphological OPENING expressed per plate. The example already runs
`erode_studs`, so the vocabulary exists on both sides.

## 3. Greeble placement

### 3.1 The finding that reframes the question

Nova does not have a scatter problem. `skin_decor.rs` places at most one piece
per plate, at the plate CENTRE, at `volume()` height, with a quantised yaw. The
plate IS the scatter cell, and round 1's grid-occupancy-claiming recommendation
is therefore already satisfied in the strongest possible form - the claim grid is
the ship's own lattice.

CONFIRMS round 1 recommendation 2 (grid-occupancy claiming over blue noise):
Nova is already past it, and nothing found in this round argues for going back
to a sampler.

CONFIRMS round 1 recommendation 3 (weight toward borders and link points):
`PlateReading::border` and `PlateReading::fitting` exist and the styles use them.
One correction worth recording, from the code rather than from a source: the
tree already found that "beside a nozzle" measured as a RING of 1 carpeted 45%
of a hull, and moved to four FACE steps. That is a real measurement against
round 1's advice and it refines it - "weight toward fittings" needs a distance
metric chosen by measurement, not a neighbourhood test.

**The missing key is not a new statistic. It is a flat SEAT of a known size at
the anchor point.** Today `decor_pose` has no seat to offer, so a style's only
lever is the `relief` filter, and the filter is being used as a proxy for "does
this plate have anywhere flat" - which is why the styles ask for
`relief: vec![Flat, Bevel]` and why `Flat` being under a seventh of a hull
starves them.

### 3.2 What this implies for the style schema

If the plateau lands, the plate can expose the ONE number a style actually
needs: the radius or the side length of the flat area at the anchor. Then a
style says "this piece needs 0.4 cells of flat" instead of listing reliefs, and
the filter stops being a proxy. That is a schema change, so it belongs to the
implementation task rather than here, but it is the natural consequence and it
should be decided at the same time as the geometry.

CONFIDENCE: medium. Falsifier: if the plateau's size turns out to be nearly
constant across reliefs, then a size field carries no information the relief
class did not, and the existing filter is fine.

### 3.3 Edges, corners and high points

The brief asked which decorations are worth putting on edges, corners and high
points, and how those are made to read as intentional rather than stuck on.

**NEGATIVE RESULT, searched for properly and not found.** No artist source names
corner caps, edge trim rails, panel-line terminators or masts-on-high-points as a
deliberate convention. The Polycount wiki has no Greeble, Hard Surface, Detail
Placement or Panel Line page - all 500 page titles were enumerated through the
MediaWiki API. Nothing verifiable came back from any of the hard-surface artists
whose names come up for this, so **nothing in this section may be attributed to
them.** That mapping does not appear to be written down anywhere reachable.

What follows is reasoning from the tree and from round 1, labelled as such. Read
section 3.6 first - one source argues the question is posed wrongly.

The one defensible rule, and it is round 1's, is that a decoration must be
JUSTIFIED by the form it sits on. Nova already has the vocabulary to say that,
and the geometry work would sharpen it:

- an EDGE piece wants a straight edge to lie along, which is `Brink` and its
  `fall` direction - the only relief with a single outward direction. That is
  already how the styles use it, and `Brink` is coplanar, so an edge strip
  already has a straight seat. Nothing here needs fixing.
- a CORNER piece wants a corner, which is `Bevel`'s fallen corner and the
  diagonal `Spur`'s two. Today neither is locatable - the reading records the
  relief class but not WHICH corner fell, and `decor_pose` can only anchor at the
  plate centre. If corner caps are ever wanted, the fallen-corner mask has to
  reach the placement, and a decoration needs an anchor other than the centre.
  That is a bigger change than R2-R4 and it is not recommended yet.
- a HIGH POINT is the tent's crest and the stud. Both are things the owner says
  he likes. R2 makes the crest a real line rather than a sag, which gives a
  spine-top piece something straight to sit on - and `PlateReading::along`
  already names its direction.

CONFIDENCE: low, because it is unsourced reasoning. Falsifier: any of it, cheaply,
by rendering one piece on one plate of each class.

### 3.4 What shipped tools actually key scatter on - and the trap in it

The brief asked exactly this. The answer, DEMONSTRATED by shipped node lists
rather than claimed: **slope, curvature and enclosure are three DIFFERENT keys
and every serious tool ships them separately.** Nobody rolls them into one
"flatness" number.

- Houdini **HeightField Mask by Feature** is the cleanest single piece of
  evidence - one node, five separate masks:
  `Mask by Slope`, `Mask by Height`, `Mask by Curvature` (tab labelled "Peaks and
  Valleys"), `Mask by Direction`, `Mask by Occlusion` ("Create a mask based on
  nearby obstructed terrain areas").
  <https://www.sidefx.com/docs/houdini/nodes/sop/heightfield_maskbyfeature.html>
- **World Creator** ships `slope`, `steepness`, `curvature`, `cavity`,
  `occlusion`, `flow`, `height-gradient`, `roughness` as separate distributions.
  `Cavity` "selects an area based on the cavity of the terrain ... either use a
  convex or concave cavity". <https://docs.world-creator.com/>
- **Gaea 2** splits Derive/Aspect (`Angle`, `Curvature`, `Height`, `Normals`,
  `Peaks`, `Slope`) from Derive/Generative (`FlowMap`, `Occlusion`, `RockMap`).
  `Curvature` "creates a mask where convex areas (protrusions, sharp edges, etc.)
  are selected". Docs repo is **MIT**
  (<https://github.com/QuadSpinner/Gaea2-Docs>); the tool is proprietary.

**THE TRAP, and it lands squarely on question 4.** Houdini's Measure SOP
documents that MEAN curvature is zero "when the surface is flat **or it curves as
much outwards in one principle direction as it does inwards in the other**".
A mean-curvature flatness test therefore reports Nova's saddle as FLAT. The
scalar that does not lie is `Curvedness`, "the square root of the average of
squares of the two principal curvatures ... without distinguishing between them
or considering their signs".
<https://www.sidefx.com/docs/houdini/nodes/sop/measure.html>

This is worth having even though Nova will not compute curvature: it is a second,
independent warning of the same shape as EMC's in section 1.5 - **a single
averaged scalar cannot classify a saddle.** Two sources from unrelated fields say
the same thing, and section 4.2 shows Nova already fell into exactly this hole,
since the mean of the eight samples is precisely the averaging that cancels.

One more, and it is directly implementable. Blender's **Edge Angle** geometry
node ships `Signed Angle`, documented as **"Concave angles are positive and
convex angles are negative."** That is a shipped, deterministic, texture-free,
UV-free convex/concave discriminator computed from mesh topology alone. Nova's
plate lattice could answer "is this fold a ridge or a trench" the same way.
Manual is **CC-BY-SA 4.0** - readable, not vendorable.

Two cautions from the same family, both documented: Blender's `Pointiness` is
per-vertex and therefore mesh-resolution dependent, and the viewport `Cavity`
screen mode "does not take the size of the ridges and valleys into account", i.e.
it is scale-blind. Any cheap cavity approximation inherits one of those.

### 3.5 Nobody ships a flat-AREA gate. Two exact primitives build one.

**The strongest negative result of the round, and it answers the brief's
question directly: no reachable shipped scatter tool gates placement on a minimum
contiguous flat AREA.** Every tool gives per-sample curvature or slope. The
footprint test is a hole in production tooling, not a Nova oversight.

Unreal Engine 5's PCG is the sharpest evidence, because it is the most recent and
most likely to have it:

- **`Normal To Density` IS the whole slope filter**, and it is one dot product
  per point. From `PCGNormalToDensity.cpp`:
  `Density = pow(clamp(Up.Dot(Normal) + Offset, 0, 1), 1/Strength)`. It reads one
  point - no neighbourhood, no radius, no window.
- Full-text search of `Engine/Plugins/PCG`: `planarity` -> **0 hits**;
  `curvature` -> 8 hits, all SPLINE curvature; `slope` -> 2 incidental hits in
  remap code. A 197-node index contains no node mentioning flat, planar, area,
  region, contiguous or connected.
- Unreal Landscape Grass has **no slope filter at all** - `FGrassVariety` has no
  slope, angle, normal-threshold or curvature property. Slope is entirely the
  artist's job in the grass-map material.
- All under the Unreal EULA. **Readable, never vendorable.** Also OUT on
  architecture: `Surface Sampler` jitters by seed, `Self Pruning` randomises by
  default and sweeps a global octree, `Blur` and `Collapse Points` are explicit
  relaxation loops.

Two exact, deterministic, RNG-free primitives compose into the gate nobody ships,
and **one of them is native to a voxel lattice**:

1. **Greedy meshing** (Mikola Lysenko,
   <https://0fps.net/2012/06/30/meshing-in-a-minecraft-game/>) merges coplanar
   voxel faces into maximal rectangles, with a lexicographic total order on quad
   position then dimensions - so it is fully deterministic for identical input.
   **Nova's plates ARE cell faces of a voxel lattice.** Greedy-merging the
   coplanar ones yields a maximal flat RECTANGLE whose width and height are
   literally the available flat footprint. One decoration per merged rectangle,
   sized to it, seated on it. The article names a JS reference implementation but
   states NO LICENCE - treat as read-only.
2. **The exact distance transform** (Felzenszwalb and Huttenlocher, "Distance
   Transforms of Sampled Functions",
   <https://cs.brown.edu/people/pfelzens/papers/dt-final.pdf>) is exact,
   deterministic and linear time per dimension. Over the plate grid, compute the
   distance to the nearest non-flat cell; **the value at a cell IS the maximum
   decoration radius that fits there**, and local maxima are the centres of the
   largest inscribed flat regions. It is equivalent to morphological erosion, so
   it is the section 2.4 operator in its useful form. No RNG, no relaxation, no
   per-instance state, no textures.

Convergent evidence that this is the right shape of answer: PCG's only
footprint-ish route is also a distance transform (`Distance` -> `Attribute
Filter`). Nobody ships the area test; everyone who approximates it reaches for
distance.

CONFIDENCE: high that these are exact and legal here. Medium that Nova needs
them - greedy merging spans MULTIPLE plates, and Nova's decoration is currently
per-plate, so taking this seriously means letting one decoration span plates.
Falsifier: a per-plate plateau (R4) turns out to give a big enough seat, in which
case the cross-plate machinery buys nothing.

### 3.6 The finding that contradicts the whole framing: RECESS the chunky detail

The one primary-ish source reached that speaks DIRECTLY to "chunky detail on
lumps looks wrong" says the answer is not to hunt for flat tops at all.

RC Sci Fi, "Model Kit Part Detailing Guide"
(<https://rcscifi.blogspot.com/p/model-kit-part-detailing-guidelines.html>),
an anonymous physical model maker's rules derived from studying 1960s-80s film
miniatures. Craft opinion, no licence stated - quotable, not vendorable, and
CLAIMED rather than DEMONSTRATED. Verbatim:

> Wherever possible chunky detail always looks the best when below the surface or
> recessed. It can be in a trench or a hole or where a panel has been removed.
> The other possibility is to build up around an area so it appears recessed.
> **Avoid just gluing big lumpy detail parts on top of the surface.** Flat details
> such as panels, piping and ducting are OK.

That is a two-class rule Nova could implement immediately, and it cuts across
this whole document: chunky 3D pieces want a WELL, not a plateau; flat pieces -
panels, piping, ducting - can go on anything, including a cone. Note round 1
already recorded that bulkhead "dropped recessed fittings because faces-with-holes
were not representable, using height variation between plates to supply recessed
channels instead", and that Nova's flat-shaded plates have the same constraint -
so a Nova recess has to be a height difference between neighbouring plates, not a
hole in one.

Two more rules from the same source, both adding to round 1:

- CLUMPING, which the author calls the most important: detail "should clump
  together ... **The heights of the chunky detail should also be clumped like a
  grove of trees, taller in the centre and smaller as the clump radiates out.**"
  Round 1 banked "cluster, do not evenly sprinkle" for PRESENCE. This says
  cluster the HEIGHT too, which is new and which Nova's per-plate `share` could
  express.
- RANDOM names the failure mode precisely: "randomly place detail in terms of
  location across the surface, but each location is about the same distance away
  from each other." **That is a description of blue noise, called out as a
  mistake by an art source** - independent corroboration of round 1's ruling
  against Poisson-disk from a completely different direction.

CONFIDENCE: medium. One craft source, no images verified, and it is about physical
miniatures rather than flat-shaded low-poly. Falsifier: render one chunky piece
recessed into a plate and one standing on a plateau, side by side. That is a
cheap experiment and it should be run alongside R1.

## 4. Saddle avoidance

### 4.1 Coplanarity is an EXACT integer test, and it is the whole taxonomy

DERIVED. Put the face on `[-0.5, 0.5]^2` in cell units. The eight footprint
points are the four corners at `(+-0.5, +-0.5)` and the four edge midpoints at
`(+-0.5, 0)` and `(0, +-0.5)`. A plane over a square hits its four corners iff
the bilinear cross term vanishes, so with samples as INTEGERS in quarter cells:

```
the four corners are coplanar          iff  c0 + c2 == c1 + c3
a midpoint lies on that plane          iff  2 * m_i == c_i + c_(i+1)
the eight samples are coplanar         iff  both, for all i
```

No epsilon, no residual, no threshold. This satisfies EMC's warning in section
1.5 by construction: the classification is STRUCTURAL. And the second clause has
a plain reading in Nova's own terms - `boundary_heights` already sets every live
edge's midpoint to the mean of its corners, so the only thing that can break it
is a DEAD edge casting its own vote. **The taxonomy is therefore exactly two
causes: non-planar corners, or a dead-edge vote.**

Their centroid is the origin and the design is orthogonal, so the least-squares
plane `h = a*x + b*z + c` through the eight samples has

```
c = mean(h_i)
a = sum(x_i * h_i) / sum(x_i^2)      sum(x_i^2) = 1.5
b = sum(z_i * h_i) / sum(z_i^2)      sum(z_i^2) = 1.5
```

`c` is exactly `centre_height()`. **So today's apex is already the least-squares
plane evaluated at the cell centre**, which is why the module note is right that
the mean "leaves a ramp flat". Worked, DERIVED: the ramp `corners [0,0,4,4]`,
`midpoints [0,2,4,2]` is the plane `h = 0.5 - z` at every one of the eight
samples, residuals zero.

That gives the invariant the implementation should assert: **on any plate whose
eight boundary samples are coplanar, the new mesh must still be the plane.**
Cheap over the existing 12720-shape spread, and it kills the waffle risk in
section 1.7 outright.

### 4.1a The least-squares plane is the WRONG generic plateau. Do not use it.

DERIVED, and it kills the tidiest version of the brief's hypothesis. Fit the LS
plane to a `Bevel`, `corners [0,2,2,2]` / `midpoints [1,2,2,1]`:

```
a = -0.25   b = -0.25   c = 0.375   max |residual| = 0.125
```

The plateau comes out TILTED, because one fallen corner drags the whole fit.
Two `Bevel` plates side by side with their fallen corners on opposite sides
would tilt opposite ways, and a run of them would read worse than the dish it
replaced. The plate's own doc comment says what it actually is - "a panel with
a corner taken off" - and the right interior is a HORIZONTAL plateau at the
running height with the corner chamfered off, not a tilted plane.

Nor does any other single formula work. The modal sample height is right for
`Bevel` (5 of 8 samples at the running height) and right for `Step`, but it is
catastrophically wrong for `Ridge`: six of the tent's eight samples are on the
floor, so a modal plateau would flatten the ridge the owner explicitly likes.

**Conclusion: the interior must be a small CASE TABLE keyed on the structural
class, with today's fan kept as the fallback.** That is marching cubes' own
architecture - a case table - and it is what EMC and CMS do (classify, then
construct). It is also incremental: every unclassified shape keeps the mesh it
has today, so nothing can regress on a case nobody looked at.

CONFIDENCE: high. Falsifier: a single formula that is measurably better than the
fan on all of `Bevel`, `Step`, `Spur` and `Ridge` at once, which would make the
table unnecessary.

### 4.2 The saddle is the bilinear answer, not a vote artefact

DERIVED. Take `corners [0, h, 0, h]`, so `midpoints` are all `h/2` by the
mean-of-corners rule. Then

```
mean of the eight samples = (0 + h + 0 + h + 4*(h/2)) / 8 = h/2
bilinear patch at the centre of corners (0, h, 0, h) = (0 + h + 0 + h)/4 = h/2
```

They are the same number. The plate's middle sits at the height the bilinear
interpolant of its corners puts it, and the bilinear patch over alternating
corners is a hyperbolic paraboloid - **a saddle by definition**. This CORRECTS
the framing in the brief: the saddle is not forced by `boundary_heights` voting
three ways, it is what the INTERIOR interpolant is. The vote does not need to
change for the saddle to go away.

### 4.3 The gable and the valley are both EXACT interpolants

DERIVED, and this is the finding to act on. With `corners [c0,c1,c2,c3] =
[0,h,0,h]` at `FACE_CORNERS` positions `(0.5,0.5), (-0.5,0.5), (-0.5,-0.5),
(0.5,-0.5)`, the high corners are `c1` and `c3`, on the anti-diagonal `x + z = 0`.

GABLE - join the high corners with a ridge at height `h`:

```
u = x + z          h(u) = h * (1 - |u|)
corners: |u| = 1 -> 0 at c0 and c2, and u = 0 -> h at c1 and c3
midpoints: |u| = 0.5 -> h/2, at all four
```

Every one of the eight boundary samples is hit exactly.

VALLEY - join the LOW corners instead:

```
v = x - z          h(v) = h * |v|
corners: v = 0 -> 0 at c0 and c2, |v| = 1 -> h at c1 and c3
midpoints: |v| = 0.5 -> h/2, at all four
```

Also exact.

So the interior admits at least three exact interpolants of the same boundary:
the bilinear saddle (shipped), the gable, and the valley. All three honour the
contract. **Choosing between them is free and is purely an art decision.** The
owner has already made it: "instead of using a full ridge (roof style) it uses a
0101 with lowered center ... and it's kind of ugly". Join the HIGH corners.

CONFIDENCE: high - this is arithmetic, and the implementer can re-derive it in
five minutes. Falsifier: a render where the gable reads worse than the saddle,
which is possible if diagonal ridges across a hull read as a herringbone.

### 4.4 The tent sags, and nobody had noticed

DERIVED, and it is a second bug of the same family. The one-cell spine's shape is
`corners [0,0,0,0]`, `midpoints [2,0,2,0]` - crest points at `(0, +-0.5)` at
height 0.5, everything else on the floor.

```
mean of the eight samples = (0.5 + 0 + 0.5 + 0 + 0 + 0 + 0 + 0) / 8 = 0.125
```

So the plate's middle rides at 0.125 while its own crest points stand at 0.5. A
one-cell spine's "tent" SAGS by 0.375 cells - three eighths of a cell - in the
middle of every plate along its length. The true tent `h = 0.5 * (1 - 2|x|)` hits
all eight samples exactly, as above.

`20260815-190741/NOTES.md` records "a one-cell spine still reads as a ridge" from
the row render, which is true - it does read as a ridge. Nobody measured that it
is a sagging one. This is the clearest single piece of evidence that the fan is
the defect: the feature the owner explicitly LIKES is being drawn wrong by the
same mechanism that draws the feature he dislikes.

CONFIDENCE: high. Falsifier: a close render of a spine that shows no sag - which
would mean the crest midpoints are not where this reading assumes.

### 4.5 The asymptotic decider, and why it does NOT apply

Nielson and Hamann 1991, "The asymptotic decider: resolving the ambiguity in
marching cubes", Proc. Visualization '91, pp. 83-91, is the canonical answer to
the alternating-corner face. Confirmed via
<https://en.wikipedia.org/wiki/Asymptotic_decider> (CC-BY-SA 4.0 - facts free,
text not reusable). The rule: the bilinear interpolant is written
`f(a,b) = g*(a - a0)*(b - b0) + d`, and `d` is the value at the saddle; the
saddle "ought to belong to the section which contains two corners", so if `d` is
above the isovalue the positive corners are joined and the negative pair is
separated, and vice versa.

**NOT APPLICABLE to Nova, and the implementer should not reach for it.** The
asymptotic decider settles a TOPOLOGY question - are two contour components
joined or separate. Nova's plate top is a single-valued height field over a fixed
footprint; there is no isovalue, no sign, and no topology to decide. What
transfers is only the SHAPE of the argument: the alternating-corner case has two
defensible readings, the choice is genuinely free, and a deterministic rule must
pick one. CMS's own remark that it resolves the same ambiguity by feature overlap
"although the results are possibly different from the results obtained using
asymptotic deciders" says the same thing - the ambiguity is real and more than
one resolution is defensible.

The closed form IS available after all, from Boris the Brave, "Resolving
Ambiguities in Marching Squares"
(<https://www.boristhebrave.com/2022/01/03/resolving-ambiguities-in-marching-squares/>,
blog text copyright, analysis only). He states the ambiguous configuration as
"top_left and bottom_right both positive, bottom_left and top_right negative, or
vice versa", and gives the decider as

```
Q = top_left * bottom_right - bottom_left * top_right
```

with the sign of `Q` choosing the configuration. He also names the limitation
that matters here: **the decider needs numeric field values**, and a pure boolean
cell set has none, so it degenerates to a fixed convention.

Nova sits in an odd middle position - it HAS numeric heights, but they are not a
signed field about an isovalue, so `Q` carries no meaning. Worked, DERIVED: for
`corners [0,h,0,h]`, `Q = 0*0 - h*h = -h^2`, negative for every `h`. The decider
would return the same answer on every saddle Nova can build, so a fixed
convention is all it could ever be. **Choose the convention on taste and render
both.** That is what R3 says, and section 1.6's OpenRCT2 "valley" naming is the
reason to actually render the other one.

One supporting line from the marching-cubes history
(<https://en.wikipedia.org/wiki/Marching_cubes>, CC-BY-SA - facts free, text
not reusable): the 256 configurations reduce to 15 base cases by symmetry
(Lorensen and Cline 1987), face ambiguity "occurs when its face vertices have
alternating signs", and the resolution was Chernyaev's Marching Cubes 33 (1995),
which ENLARGED and disambiguated the case table to 33 cases rather than smoothing
the output. Same lesson as section 4.1a: the table was not wrong about geometry,
it was wrong about which configurations it bothered to distinguish.

## Ranked recommendations

What the implementation task should try, in order.

**R1. Render the A/B first, before building anything.** Matched pose, matched
seeds, `freeze_bodies` on. The interior change is the hypothesis; a photograph is
the only thing that has ever settled a skin rule on this project, and
`20260815-190741/NOTES.md` records two rules adopted from reasoning and later
disproved by rendering. Cost: an hour.

**R1b. Split the diagonal saddle out of `Spur`, and refuse to decorate it.**
The cheapest lever in this document, available before any geometry changes at
all. `relief_of` already isolates the mask; give it its own `PlateRelief` and the
styles can stop putting pieces on it. This is precisely what OpenTTD and
Simutrans do with the same slope - it stays as terrain, it stops carrying built
geometry (section 1.6). It does not answer the "it looks ugly" half of the
complaint, only the "decorations look weird on it" half, but it is an afternoon.
CONFIDENCE: high. Falsifier: the saddle is common enough that refusing it starves
a style, which `relief_tally` can answer in one run.
One cost, checked: `PlateRelief` variants are serialised BY NAME into
`assets/base/styles/base.content.ron`, so a new variant is a `content gen`
regenerate, and any webmod naming a relief sees a vocabulary change.

**R2. Fix the tent.** `corners [0,0,0,0]` with two opposite midpoints up gets a
true ridge between its crest points instead of a fan to a sagging apex. Smallest
possible change, exact interpolant, no classification needed beyond the `Ridge`
class `relief_of` already computes, and it improves a feature the owner says he
LIKES. CONFIDENCE: high. Falsifier: the render shows the spine now reads as a
blade rather than a hull.

**R3. Fix the saddle - gable, joining the HIGH corners.** The diagonal fallen
mask `0b0101` / `0b1010`, which `relief_of` already isolates inside `Spur`. Exact
interpolant, free under the contract, and it is the change the owner asked for in
words. Split it out of `Spur` into its own relief while you are there - a saddle
and a spar tip are not the same place to stand a decoration.
Render the VALLEY at the same time - it is the other exact interpolant, it is one
extra branch, and OpenRCT2 names its equivalent slope a "valley" rather than a
roof (section 1.6), which is the only outside evidence found either way.
CONFIDENCE: high on correctness, medium on taste. Falsifier: diagonal ridges read
as herringbone across a flank, or the valley reads better than the gable.

**R4. Add a CASE TABLE for the interior, with today's fan as the fallback.**
This is the brief's hypothesis, refined in two ways that section 4.1a shows are
not optional: the plateau must not be a least-squares plane, and no single
formula covers all four broken classes. The table:

| Class | Test | Interior |
| --- | --- | --- |
| coplanar | `c0+c2 == c1+c3` and `2*m_i == c_i + c_(i+1)` | the plane. Byte-identical to today. |
| tent | corners all 0, two opposite midpoints up | ridge between the crest points (R2) |
| diagonal saddle | fallen mask `0b0101` / `0b1010` | gable joining the HIGH corners (R3) |
| `Bevel`, `Step` | one sample off a dominant plane | horizontal plateau at the running height, chamfer to the odd sample |
| anything else | - | today's fan, unchanged |

Ranked below R2 and R3 because it is bigger, and because R2 and R3 may already
satisfy the complaint. The fallback row is what makes it safe to land in pieces.

Two things section 1.0 supplies that the brief did not have. **Do not guess the
inset width** - the good sources derive the band from the reach of whatever could
perturb the boundary, so Nova's is "how far inboard can a vertex move before it
changes a triangle touching a boundary sample". And there is a shipped precedent
to start from: Catlike Coding's hex map uses a solid core at 0.75 of the radius
with the outer quarter carrying all the matching, for verbatim Nova's reason
(section 1.0a).
CONFIDENCE: medium-high. Falsifier: the assert "coplanar boundary implies the
plane" fails over the 12720-shape spread, or the render shows a visible cell grid
on flat hull.

**R5. Fix `decor_pose` in the same commit as any of R2-R4.** It lifts to
`volume()`. Once the interior is not the mean, that number is neither the surface
height at the anchor nor the solid's volume. Leaving it puts greebles inside the
plating. This is not optional and it is not separable.

**R6. Recompute `volume()` per interior primitive.** The mean of the eight
samples under-reads a gable by 25% and the tent by 50%. Collider height, health
scale and `PlateReading::height` all read it.

**R7. Classify structurally, then solve. Do not threshold a residual.** EMC's
"Remarks" says the tempting move is unreliable. Use the fallen-corner mask and
the equality pattern `relief_of` already builds. This also keeps the interior a
pure function of the canonical shape, which the mesh cache needs.

**R8. Expose the flat seat size to the style schema.** Only after R4. Lets a
style say "this piece needs 0.4 cells of flat" instead of listing reliefs. If a
per-plate seat proves too small, the exact primitive for a cross-plate answer is
the distance transform in section 3.5 - the value at a cell IS the largest
decoration radius that fits there, and it is exact, linear and RNG-free.

**R9. Render one chunky piece RECESSED, beside one on a plateau.** Cheap, and it
tests the one source that contradicts this whole document (section 3.6): chunky
detail may want a well rather than a flat top, with only flat pieces - panels,
piping, ducting - going on the surface. If it holds, the style schema needs a
"wants a recess" flag more than it needs a seat size, and a Nova recess has to be
a height difference between neighbouring plates rather than a hole in one.
CONFIDENCE: medium, one craft source about physical miniatures. Falsifier: the
render.

## Licence positions for everything cited in this round

Nova is MIT. Share-alike and unlicensed code is UNUSABLE for copying; the ideas
are free everywhere.

| Source | Licence | Status |
| --- | --- | --- |
| OpenVDB (morphology, level-set filter) | Apache-2.0 | **Usable**, NOTICE requirement |
| `ZachHembree/GreedyCubicalMarchingSquares` | MIT | **Usable** |
| `metalisai/Aviz.Cms` | Apache-2.0 | **Usable**, NOTICE requirement |
| `sidit77/CMS` | MIT | **Usable** |
| `SebLague/Procedural-Cave-Generation` | MIT | **Usable** (but the region pruning is OUT on architecture) |
| `mxgmn/WaveFunctionCollapse` | MIT | Usable, but the algorithm is OUT - round 1 already said so |
| jess-hammer dual-grid (Godot, Unity), `pablogila/TileMapDual`, `skner-dev/skner.DualGrid` | MIT | **Usable** |
| Excalibur.js | BSD-2-Clause | **Usable** |
| scikit-image | BSD-3-Clause by convention, **NOT verified this session** | verify before relying |
| OpenTTD | **GPL-2.0** | **READ ONLY.** Never copy. |
| Freeciv | **GPL-2.0** | **READ ONLY.** |
| OpenRCT2 | **GPL-3.0-or-later** | **READ ONLY.** |
| Continuity (Minecraft CTM) | **LGPL-3.0** | **READ ONLY.** |
| Simutrans | **Artistic License 1.0** | **READ ONLY.** |
| `TheCyberBrick/Unity-Cubical-Marching-Squares-Prototype` | NOASSERTION | **UNUSABLE** - treat as all rights reserved |
| `TheWiseLion/CubicalMarchingSquares` | none | **UNUSABLE** - no licence file |
| EMC 2001, CMS 2005 papers | ACM / Eurographics copyright | Read only. Ideas free, no text or figure reuse. |
| Wikipedia (marching cubes, asymptotic decider, morphology) | CC-BY-SA 4.0 | Facts free, **text not reusable** |
| Boris the Brave articles, cr31 mirror | blog text, copyright | Analysis and links only |
| minecraft.wiki, OptiFine docs | not verified | Secondary. Analysis only. |
| Gaea 2 docs repo (`QuadSpinner/Gaea2-Docs`) | MIT | **Usable** (docs only; the tool is proprietary) |
| `a1studmuffin/SpaceshipGenerator` | MIT - GitHub shows NOASSERTION only because the LICENSE file has a preamble | **Usable** for the algorithm. Generated OUTPUT is separately claimed CC-BY-3.0. |
| Unreal Engine source and PCG plugin | **Unreal EULA** | **READ ONLY.** Never vendor. |
| Blender manual, `add_mesh_discombobulator` | **CC-BY-SA 4.0 / GPL** | **READ ONLY.** |
| SideFX Houdini, World Creator, World Machine docs | proprietary | Read only. Node names are facts. |
| Neil Blevins art lessons | all rights reserved | Quotable, not vendorable |
| RC Sci Fi detailing guide | no licence stated -> all rights reserved | Quotable, not vendorable |
| 80.lv, Gnomon interviews | editorial copyright | Analysis only |
| Felzenszwalb and Huttenlocher distance transform paper | academic copyright | Algorithm free to implement |
| 0fps greedy-meshing reference implementation | **none stated** | **UNUSABLE** - algorithm only |
| meshoptimizer (`meshopt_SimplifyLockBorder`) | MIT | **Usable and vendorable** |
| Bullet `btHeightfieldTerrainShape` diagonal policies | zlib | **Usable and vendorable** |
| Transvoxel tables | MIT | **Usable** |
| Sylves, DeBroglie | MIT | **Usable** |
| `BorisTheBrave/mc-dc` | CC0 per README, no LICENSE file | Usable, but the missing file is a real gap |
| Ulrich chunklod notes | public domain | Usable. **Wording unverified** - the PDF has no ToUnicode map. |
| CesiumJS | Apache-2.0 | **Usable**. The quantized-mesh SPEC repo has **no detected licence**. |
| Catlike Coding hex map | code and assets **MIT-0**; text, screenshots, diagrams **CC BY-NC-SA 4.0** | Code vendorable, **prose and figures are not** |
| Carlson Truchet Wolfram package | none stated | **UNUSABLE** |
| Space Engineers source | custom EULA, "not allowed to use our source code in an application other than Space Engineers" | **DO NOT TOUCH** |
| Tessera (Unity asset) | proprietary | **DO NOT TOUCH** |

Nothing was committed beside this file.

## What CONTRADICTS or corrects earlier rounds

1. **The brief's finding 2 is half right.** "Saddles are forced" by
   `boundary_heights` is true of the SAMPLES but not of the LOOK. The saddle read
   comes from the interior fan, which reproduces the bilinear patch (section
   4.2). A gable IS expressible today without touching a single vote, because the
   diagonal ridge is interior geometry. The brief says "a gable roof cannot be
   expressed because nothing asks for one" - it can be, and nothing has to ask.
2. **The brief's leading hypothesis - "an inset flat plateau chamfered out to
   the same boundary" - does not survive contact with the arithmetic in its
   simple form.** A HORIZONTAL plateau staircases every planar ramp, which the
   module note in `shell_shape.rs` explicitly designed the mean to avoid; and a
   least-squares plateau TILTS on a `Bevel` and reads worse than the dish it
   replaces. The answer is a case table, not a plateau. Sections 4.1 and 4.1a.
   This is the round's main correction to its own brief.
3. **"Fan off the average" is a known fallback, not a design.** CMS uses it only
   when a component has NO sharp feature. Nova uses it unconditionally. That
   reframes the change from "invent an interior" to "add the branch the
   literature already has".
4. **"Forbid the saddle in the alphabet" is half right, and I got it wrong
   first.** My first pass read only OpenTTD's slope enum, found `SLOPE_EW` and
   `SLOPE_NS` present, and recorded a negative result. Reading further and
   reading Simutrans reverses it: three shipped engines let the saddle EXIST as
   terrain and forbid it from CARRYING anything, and Simutrans' `flags[81]`
   table forbids the lone raised corner too. The lever is not the alphabet, it is
   what may stand on a shape. Section 1.6, and R1b.
5. **Round 1 recommendation 2 (grid-occupancy claiming over blue noise) is
   CONFIRMED and already exceeded.** Nova claims on the ship's own lattice, one
   piece per plate. Nothing in this round argues for a sampler.
6. **Round 1 recommendation 3 (weight toward borders and fittings) is CONFIRMED
   and refined by a measurement in the tree**: "beside a fitting" as a ring of 1
   carpeted 45% of a hull; four face steps is what "beside" means. The advice was
   right, the metric had to be measured.
7. **The task's framing may be wrong, and one source says so.** The whole brief
   assumes the fix is to give decoration a flat top. RC Sci Fi's rule is that
   chunky detail belongs BELOW the surface - in a trench, a hole, or a removed
   panel - and that only flat detail (panels, piping, ducting) should sit on top
   of anything. Section 3.6, and R9. One craft source, so weak, but it is the
   only source found that addresses the complaint head-on.
8. **Round 1's "cluster, do not sprinkle" extends to HEIGHT.** The same source
   says chunky detail heights should clump "like a grove of trees, taller in the
   centre and smaller as the clump radiates out". Round 1 banked clustering for
   presence only.
9. **Round 1's ruling against blue noise gets independent corroboration from an
   ART source.** RC Sci Fi's RANDOM rule describes "each location about the same
   distance away from each other" as a mistake - which is a description of
   Poisson-disk sampling. Round 1 reached the same conclusion from alignment
   arguments about machined hardware.
10. **Round 1's scale-ruler recommendation needs one amendment.** Blevins,
   quoting Pascal Blanche's "Theory of the Lego Block", says detail size stays
   roughly world-constant as an object grows - but also "if you have two bolts,
   one that is an inch tall and one that's a foot tall, don't use the same design
   for both, make a different bulkier looking design for the big one". So the
   rule is a world-constant size PER FAMILY, plus a separate bulkier family for
   large parts - not one family never rescaled.
   <http://www.neilblevins.com/art_lessons/composition_details_make_big/composition_details_make_big.htm>
11. **Strike "winged tiles" from round 1's shortlist.** Round 1 banked Carlson's
   winged tiles as prior art worth knowing. Read properly, they are the OPPOSITE
   strategy - they hide joins under overlap and need a draw order, which does not
   work in watertight 3D. Section 1.10.
12. **Round 1 banked Lagae and Dutre and Neyret and Cani for their BOUNDARY
   rules; both papers also answer the INTERIOR question and were not mined for
   it.** Both partition the tile into a constrained band plus a free region, and
   both DERIVE the band width rather than choosing it. Section 1.0. That is the
   single most useful thing this round adds to question 1, and it was sitting in
   sources round 1 already cited.
13. **Dual contouring should be struck as a candidate, not merely cautioned.** My
   own first draft called it "a guard rather than a live risk". It is worse than
   that: DC cannot evaluate cells independently, which is the property Nova's
   whole design rests on. Section 1.4.
14. **Round 1's authored-versus-generated decision gets outside support.**
   Stalberg's Planet abandoned per-case authored tiles on a pure combinatorial
   argument - "(8x4)^3 different tiles ... too many tiles to manually author, or
   even for the game to load". Nova reached the same conclusion by deleting 41
   authored `.glb` tiles. Section 1.8.
15. **Round 1's "silhouette is inviolable" rule bites here.** R2 raises the middle
   of every one-cell spine by 0.375 cells, which changes the outline. Intended,
   but it must be looked at rather than assumed.

## What could NOT be found out

Stated plainly rather than filled in.

- **The primary Dual Contouring PDF.** `cs.wustl.edu` did not resolve from this
  environment and two mirrors returned 404. Section 1.4 rests on a practitioner
  blog and on EMC's independent agreement, and is labelled CLAIMED.
- CLOSED, not a gap: the asymptotic decider's closed form was found after all
  (section 4.5), and it turns out not to apply.
- **Whether the alternating-corner slope arises from OpenTTD's map GENERATOR or
  only from player terraforming.** The slope enum and the terraform invariant
  were read; the map generator was not. It matters for how strong the "it is
  legal terrain" half of section 1.6 is.
- **A published rule of the literal form "keep a feature only if it belongs to a
  run of length >= N".** Every instance found is either a fixed-radius
  neighbourhood predicate (local, usable) or a connected-component size threshold
  (global, unusable). No system was found doing run-length semantics as a pure
  local function on a heightfield.
- **Anything citable on Astroneer or Deep Rock Galactic terrain.** Both were
  searched for and neither yielded a source.
- **The OptiFine 47-tile index-to-neighbour-bitmask mapping.** It ships as a PNG
  template; neither OptiFine's `ctm.properties` nor Continuity's wiki spells out
  the indices.
- **METHOD CAVEAT, and it is the biggest limit on this round.** The session's
  web search budget (200 calls) was already exhausted when this round began, so
  nothing here came from a search engine. Everything is a direct fetch of a URL
  reasoned about in advance, a GitHub API query, a PDF pulled and extracted
  locally, or a reading of the tree. Most alternative engines returned captchas,
  403 or 429. Breadth is therefore worse than round 1's, and the gaps below are
  gaps in COVERAGE, not evidence of absence.
- All three parallel breadth sweeps eventually returned. Sections 1.6 and 2.2-2.4
  come from the continuity sweep, 3.4-3.6 from the greeble sweep, and 1.0, 1.0a,
  1.4 and 1.7-1.11 from the flat-top sweep.
- **No primary Hardspace: Shipbreaker developer talk was reached**, and no
  Stalberg talk quotes - YouTube captions were blocked. Section 1.8 is Boris the
  Brave's deconstruction, which is semi-primary at best (he states he got the
  details from Twitter threads and conversations with Stalberg). **No modular-kit
  connector-plane talk was reached.**
- **Hand-authored marching-cubes tilesets essentially do not exist as open
  source.** Exhaustive `gh search repos` and `gh search code` over the obvious
  phrasings returned only algorithmic isosurface extractors, and the GDC
  transcript corpus has four marching-cubes mentions, all algorithmic. Treat as
  "not findable this session", not "provably nonexistent".
- **No authoritative source names the diagonal-choice artefact** or shows a
  before/after. StackExchange returned zero on every phrasing. The claims that
  "shortest diagonal is Delaunay" and that the split decides ridge-versus-valley
  are derivations, not citations.
- **No primary source quantifies triangle-count blowup, collider cost or
  silhouette change for an inset ring.** Section 1.7's failure list is real bug
  reports for the qualitative failures and reasoning from Nova's code for the
  quantities.
- **Manifold Dual Contouring** (Schaefer, Ju, Warren 2007) text could not be
  obtained - IEEE TVCG, no open access. Deliberately not summarised from memory.
- **Ares Lagae's thesis full text** could not be reached; `ares.lagae.be` no
  longer resolves and the KU Leuven handle redirects into a single-page app. The
  section 7.1 material came from the SIGGRAPH paper.
- **Ulrich's skirt taxonomy wording is UNVERIFIED.** That PDF has no ToUnicode
  map and was decoded by substitution. The cleanest quotable primary on skirts is
  the Cesium quantized-mesh spec instead - and note it has **no skirtHeight
  field** and the repo reports no detected licence. No first-party Godot, Unity
  or Unreal documentation of terrain skirts exists at all; skirts are convention,
  not a documented engine feature.
- **Hard-blocked hosts**, which is why the digital-texturing and artist-forum
  corner of question 3 is under-covered: polycount.com (Cloudflare),
  web.archive.org, Adobe Substance docs (timeout / redirect to a generic home),
  reddit, and Semantic Scholar (429 on every attempt). Bing, DuckDuckGo, Mojeek
  and Marginalia all returned captchas or junk.
- **Substance Designer's Flood Fill family** - the one texture-side tool that
  measures contiguous regions - could not be verified at all. Adobe's docs were
  unreachable. Do not rely on any claim about it.
- **World Machine's selector device names.** Only `Select Wetness` is documented
  in the current knowledge base. That community macros exist named "Convexity
  Selector Enhanced" and "e2's cavitymapper" SUGGESTS the base tool lacks them,
  but that is inference, not a documented fact.
- **Enclosure as a POSITIVE key for installing hard-surface machinery.** Every
  digital tool reached uses cavity, curvature and occlusion for weathering, dirt,
  sediment and wear - things that SETTLE, never things that get INSTALLED.
  Convex-positive is documented (Gaea `Curvature` and `Peaks` select
  protrusions); concave-positive is not, except in RC Sci Fi's craft rule
  (section 3.6). Treat that negative as weak - the source class most likely to
  contain it was exactly the blocked one.
- **Any source arguing decoration should run ALONG feature lines** rather than
  across them. Not found. The reachable equivalent is the clump-and-connect pair
  in section 3.6, which is a routing rule rather than an alignment rule. The
  mathematical version does ship - Houdini Measure SOP `Principal`/`Direction`
  gives a principal-curvature axis, though "reported as vectors up to a sign
  change", so it fixes yaw only modulo 180 degrees.
- **Wong, Zongker and Salesin, "Computer-Generated Floral Ornament" (SIGGRAPH
  1998)**, the classic on ornament adapting to its region, is only PARTIALLY
  verified - the abstract confirms, the PDF 404s, and its enumerated design
  principles were not read.
- **No open-source project was found that builds hull plating from a per-cell
  height field.** GitHub repository search for the obvious phrasings returned
  nothing. That is a weak negative - the search vocabulary may simply be wrong -
  but it is consistent with round 1's finding that Nova's derivation has no close
  public relative.
- **No measurement of how much flat area a plateau would actually yield.** It
  needs the derivation to run, and this lane writes no Rust. At an inset of 0.15
  cells the plateau is roughly 0.7 by 0.7, about half the cell face, but that is
  arithmetic on an assumed inset and not a measurement.
