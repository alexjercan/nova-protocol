# A crowded cell refuses cladding it could carry

- STATUS: IN_PROGRESS
- PRIORITY: 63
- TAGS: v0.11.0, ship, render, bug, skin

## The bug

The owner built an L in the editor - three hull cubes in a flat run, two more
vertically off one end - and the section at the INNER CORNER came out bare while
every other section was clad.

Screenshot: `/home/alex/Pictures/Screenshots/20260816_132031.png`.

## Diagnosed, not guessed

The skin report reproduces it from a 5-cell structure. The inner corner is
`Crowded`: it can bolt down two ways, but both outward directions are already
held by other plates, so `cladding_cells` drops it.

It is not rare and not confined to hand-built shapes - the generated row carries
**12 crowded faces**, alongside 56 `fires_into` (by design) and 48 `no_socket`
(fitting flanks).

## The owner's hypothesis is refuted, both ways

He suspected the inside-face spikes caused the bare corner. They do not:

- the same L with arms of TWO is just as spiky and has ZERO bare cells
- the same L three cells thick is LESS spiky and has THREE bare cells

`cladding_cells` never reads a height. The spikes and the bare corner are
independent.

## Why this is its own task

It lives in `cladding_cells`, which decides WHICH cells are clad. The shape work
(`20260816-112429`) changes what a clad plate LOOKS like. No amount of
interpolant work will touch this, and no amount of this will touch the spikes.

## The question to answer

Is refusing a crowded cell correct? A cell that can bolt down two ways but whose
outward directions are taken may genuinely have nowhere to sit - or the rule may
be over-strict and one of the two mounts is fine. Decide with evidence, not by
loosening it until the corner fills.

If the cell genuinely cannot carry a plate in the current vocabulary, say so and
report what shape it WOULD need. That is a finding for the shape task, not a
failure here.

## Definition of done

- the L's inner corner is clad, or there is a stated reason it cannot be
- the 12 crowded faces on the generated row are resolved or justified
- a test pins the L, since it is a 5-cell structure and the skin is a pure
  function of structure
- no regression in `fires_into` or `no_socket` bare faces - those are correct
