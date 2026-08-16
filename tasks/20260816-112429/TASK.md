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

## What the code actually does

Read on master at `8cc8332e`:

1. **Every plate top is a cone, not a surface.** `ShellShape::surfaces`
   (shell_shape.rs:350) fans the top off ONE centre vertex at `centre_height()`,
   the mean of the eight boundary samples, into eight triangles. Unless all eight
   samples are equal a plate has NO flat area at all. That is the decoration
   problem: there is nowhere flat for anything to sit.

2. **The saddle is forced.** `boundary_heights` (shell_skin.rs:412) votes each
   corner three ways only - FULL, HALF or 0. Midpoints are then the mean of their
   two corners (:420). Corners `[0,4,0,4]` therefore give midpoints all `2` and an
   apex at the mean: a saddle, deterministically. A gable roof is not in the
   vocabulary because nothing can ask for one.

3. **The quarter cells bought no shape variety.** They were added to fix edge
   stepping, and the code states why (:389): "Quarter cells are in the alphabet so
   that mean always lands on a real sample." Corners never take them. The corner
   alphabet is still ternary.

4. **Spikes have two named causes**, both deliberate and both revertible:
   the `volume()` all-floor exception (shell_shape.rs:258) makes a lone clad cell
   read as half a cell so it comes out a STUD rather than invisible; and the rim
   taper makes a rim facing open space fall away, pinned by
   `a_rim_that_faces_open_space_tapers_away`.

## The cheap fix to try first

The code states the invariant (:279): the boundary polyline "is the whole
contract with the neighbours". Everything INSIDE the boundary is free - it never
affects seam matching, canonicalisation, or the alphabet.

So replace the single-apex fan with an inset flat PLATEAU chamfered out to the
same boundary - the trapezoid the owner described, without adding samples. Pure
mesh generation. Seams stay exact, shape ids and plate counts are unchanged, and
every plate gains a flat top.

Test rather than assume: an alternating-corner plate becomes a mesa with two
chamfers up and two down, which may read better or worse; the plateau height must
stay a deterministic function of the boundary samples; and the chamfer eats area,
so a small plate may keep little flat. Scale is the real risk.

## Placement, the second lever

Greebles should state a required FLAT AREA, not a relief class. Today they key
off Flat/Step/Ridge/Peak/Bevel/Brink/Spur, which describes shape, not usable real
estate. Contiguous-run length is already computed. Keep an exception path -
salvage whips and industrial stacks WANT a high point.

Placement needs a pass regardless: the styles merge measured `near_fitting`
losing most of its reach, `salvage_hook` absent from a third of the row, and
industrial roughly doubling in density against the post-`44704438` skin.

## The test bench (owner's design)

An example that builds MANY manual shapes - hulls in different positions plus
thrusters, PDCs and bays - clads them, and takes `--style` so a look can be
swapped without a rebuild. Then compare how the shell is EXPECTED to look against
what the debug dump says it is.

## Order, and why

Market research first, so the technique question is not re-derived. Then the skin
debug dump, then this.

The dump is the instrument. Today "the shapes are meh" is a picture; with the
dump it is: how many plates come out saddles, mean flat area per plate, what
fraction of greebles landed on non-flat relief. That matters here specifically
because TWICE this sprint a skin rule was adopted from a render and later
disproved - the corner-softening rule, and the top:wall panel-line claim.

## Depends on

Skin debug dump.

## Lane

Not started. Queued behind the skin debug dump (task 20260816-112405) and the
shape research (task 20260816-112446).
