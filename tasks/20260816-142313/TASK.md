# Stand a decoration up straight on the plate it sits on

- STATUS: IN_PROGRESS
- PRIORITY: 67
- TAGS: v0.11.0,ship,render,art,skin

## The finding this rests on

Measured by the skin report over 526 plates (`tasks/20260816-112405/NOTES.md`):

- **82% of plates tilt past 15 degrees**, and only 68 of 456 placed pieces stand
  level. Mean tilt under a piece is 26 degrees.
- **63% of placed pieces sit on a plate that IS one flat surface.** Nothing is
  wrong with that geometry.
- Only 169 of 456 pieces (37%) are on a creased top.

So the dominant defect behind the owner's "the placement is meh, they get placed
on ridges and spikes and they look weird" is **LEAN, not crease**. A decoration
is a flat-bottomed model stood up as though its plate were level, on a plate that
is tilted.

## The work

1. **Orient every decoration to its plate's TOP NORMAL.** `decor_pose` currently
   lifts to `volume()` and does not rotate to the surface. That one change
   addresses the 63% for free.
2. **Gate placement on `is_coplanar`** - a BOOLEAN, not a graded seat size.
   Measured flat area is bimodal: exactly 1.0 (308 plates) or 0.25 (218), nothing
   between. There is nothing for a graded rule to grade, so the research's
   "expose flat seat size to the style schema" is dropped.
3. **Retire the relief-class placement rules** the predicate replaces. Relief
   class does not decide creasing anyway - 58 of 112 Steps are clean ramps - so
   rules keyed on relief are wrong about a fifth of a hull.

Keep an exception path. The owner: spikes and ridges are liked where they are
meant, and salvage whips and industrial stacks WANT a high point.

## Why this goes first

It touches NO geometry: no interpolant, no alphabet, no canonicalisation, no
`content gen` for the skin. It is the cheapest large win available, and it tells
us how much of the complaint was ever about shape. If it satisfies the owner,
the interior work (`20260816-112429`) shrinks or is not needed.

## Judge it on the L, not only the row

The owner's L - three cells flat, two more vertical off one end - is 0% coplanar
with every plate's four corners fallen, against 58.6% coplanar on the generated
row. Small hand-built hulls are the WORSE case and are what a player makes in the
editor. Render both.

## Definition of done

- decorations orient to the surface they stand on
- `off_flat` and mean tilt under a piece both measured down, reported as numbers
- before/after renders of the L and of the row
- pieces still on creased plates are either absent or explicitly justified
- the skin report's decor statistics reflect the change
