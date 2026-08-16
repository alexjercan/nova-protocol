# Shell shape and decoration placement: flat tops, continuous skin

- STATUS: OPEN
- PRIORITY: 66
- TAGS: v0.11.0,ship,render,art,skin

## The complaint

Owner, on the four landed looks: the styles "look good, that's cool, but the
placement is meh, they get placed on ridges and spikes and they look weird, most
shell decorations would look good only on flat surfaces". The salvage kit is the
clearest case - big plates and scrap-metal doors that only read on flat.

And on the shapes themselves: "instead of using a full ridge (roof style) it uses
a 0101 with lowered center, so it looks like a saddle kind of, and it's kind of
ugly".

Design preference stated: **prefer a CONTINUOUS skin over spikes all over the
place.** Ridges and spikes are liked, but they should not be the default texture
of a hull.

## Scope: the skin is for BLOCK ships

Owner: "I basically want to use skin only for the 'block' ships, the ones that we
are using from kenney doesn't really make sense because they are already
modeled".

So the subject is the generated cube-built hulls - what `wfc_ships` and the
editor produce - not the modelled cast (racer, cargoa, cargob). This matches
what actually ships: `skin: true` occurs exactly once in the tree, in
`crates/nova_editor/src/placement.rs`. No scenario is clad.

Two consequences. Judge every render on a block hull, not on a cast ship. And the
style cast mapping - raiders in salvage, racer in civilian - is NOT part of this
work and has no effect while no scenario clads anything.

## What the code does

Read on master. The research round confirmed all of this from the source.

1. **Every plate top is a cone, not a surface.** `ShellShape::surfaces`
   (shell_shape.rs:350) fans the top off ONE centre vertex at `centre_height()`,
   the mean of the eight boundary samples, into eight triangles. Unless all eight
   samples are equal a plate has NO flat area at all.

2. **The saddle comes from the INTERIOR INTERPOLANT, not the corner vote.** The
   eight-sample mean is exactly the bilinear centre, so an alternating corner
   pattern fans to a sagging apex. (An earlier reading blamed the three-way corner
   vote in `boundary_heights`. That was wrong - the vote sets the corners, the fan
   makes the saddle.)

3. **The boundary is the only contract.** shell_shape.rs:279: the boundary
   polyline "is the whole contract with the neighbours". The plate INTERIOR is
   free - it never affects seam matching, canonicalisation, or the alphabet.

4. **Only four relief classes are actually broken.** Coplanarity is an exact
   integer test - `c0+c2 == c1+c3` and `2*m_i == c_i + c_(i+1)`. `Flat` and
   `Brink` already pass it and are correct today. `Bevel`, `Step`, `Spur` and
   `Ridge` are the broken set.

5. **`decor_pose` lifts every decoration to `volume()` at the plate centre.** So a
   greeble is a flat-bottomed model balanced on a cone tip. That is the placement
   complaint, in one line of code.

6. **Spikes have two named causes**, both deliberate and both revertible: the
   `volume()` all-floor exception (shell_shape.rs:258) makes a lone clad cell read
   as half a cell so it comes out a STUD rather than invisible; and the rim taper
   makes a rim facing open space fall away, pinned by
   `a_rim_that_faces_open_space_tapers_away`.

## The brief's original hypothesis is REFUTED - do not build it

An earlier version of this task proposed replacing the fan with an inset flat
plateau. Round 2 of the market research disproves it as stated:

- a horizontal plateau **staircases a ramp**
- a least-squares plateau **tilts on a `Bevel`**
- the modal rule that fixes one class flattens another

There is no single formula. It needs a case table.

## What the research says to do instead

Full findings: `tasks/20260815-231945/PLATING-AND-GREEBLES.md`. Read the ranked
recommendations section first, then section 1.0 and section 4.1a.

The technique nobody in the literature skips: **declare a REGION** - a
constrained band round the boundary plus a free interior - rather than picking a
better centre vertex. Lagae and Dutre section 7.1, Neyret and Cani, and
omega-tiles all do it. **The band width is never guessed**: it is the reach of
whatever could perturb the boundary, which for Nova is "how far inboard can a
vertex move before it changes a triangle touching a boundary sample". Catlike
Coding's hex map is a shipped precedent - solid core at 0.75 of radius, outer
quarter carries all the matching.

And two shapes are FREE: the gable and the true tent are exact interpolants of
the same eight samples the skin already carries. The roof the owner asked for
costs nothing - it is simply not the interpolant currently chosen.

## Order of work

Investigation first. Owner: "let's first use our knowledge to investigate this
thing a bit more."

- **R1. Render the A/B before building anything.** Matched pose, matched seeds,
  `freeze_bodies` on. Two rules were adopted from reasoning on this project and
  later disproved by rendering; this is the third chance to skip that step and it
  should not be taken. Also measure the relief-class distribution over a real
  hull - the research could not, because it wrote no Rust, and nobody yet knows
  what share of a hull the four broken classes make up.
- **R1b. Split the diagonal saddle out of `Spur` and refuse to decorate it.**
  Cheapest lever available, needs no geometry change - `relief_of` already
  isolates the mask. OpenTTD and Simutrans do exactly this with the same slope:
  it stays as terrain, it stops carrying built geometry. Cost to check:
  `PlateRelief` variants serialise BY NAME into `assets/base/styles/base.content.ron`,
  so a new variant means a `content gen` regenerate.
- **R2. Fix the tent.** Corners all 0 with two opposite midpoints up gets a true
  ridge between crest points instead of a fan to a sagging apex. Exact
  interpolant, and it improves a feature the owner LIKES.
- **R3. Fix the saddle - a gable joining the HIGH corners.** Render the VALLEY at
  the same time; it is the other exact interpolant and one extra branch.
- **R4. A case table for the interior, with today's fan as the fallback.** The
  fallback row is what makes it landable in pieces.
- **R5. Fix `decor_pose` in the same commit as any of R2-R4.** Not optional and
  not separable: once the interior is not the mean, `volume()` is neither the
  surface height at the anchor nor the solid's volume, and greebles end up inside
  the plating.
- **R6. Recompute `volume()` per interior primitive.** The eight-sample mean
  under-reads a gable by 25% and a tent by 50%, and collider height, health scale
  and `PlateReading::height` all read it.
- **R7. Classify structurally, then solve.** Do not threshold a residual - use
  the fallen-corner mask and equality pattern `relief_of` already builds. Keeps
  the interior a pure function of the canonical shape, which the mesh cache needs.
- **R8. Expose flat SEAT SIZE to the style schema**, after R4. A style says "this
  piece needs 0.4 cells of flat" instead of listing relief classes.
- **R9. Render one chunky piece RECESSED beside one on a plateau.** Tests the one
  source that contradicts the whole document: chunky detail may want a well rather
  than a flat top, with only flat pieces going on the surface.

Each carries a falsifier in the research file. Honour them.

## Placement needs a pass regardless

The styles merge measured `near_fitting` losing most of its reach,
`salvage_hook` absent from a third of the row, and industrial roughly doubling in
density against the post-`44704438` skin.

## The test bench (owner's design)

An example that builds MANY manual shapes - hulls in different positions plus
thrusters, PDCs and bays - clads them, and takes `--style` so a look can be
swapped without a rebuild. Then compare how the shell is EXPECTED to look against
what the debug dump says it is.

## Depends on

Skin debug dump (task 20260816-112405) for the measurements. R1 and R1b do not
need it and can run first.

## Lane

Not started.
