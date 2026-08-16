# A shape bench for judging the skin

- STATUS: IN_PROGRESS
- PRIORITY: 65
- TAGS: v0.11.0,ship,render,harness,skin

## Goal

The owner's design, in their words: an example that builds MANY manual shapes -
"different hulls in different positions + thrusters + PDCs and bays" - clads
them, takes `--style` so a look swaps without a rebuild, and lets the expected
look be compared against what the debug dump says.

## Why it is needed

Today there are two ways to look at the skin and neither is right for judging a
shape rule:

- `wfc_ships` shows GENERATED hulls - whatever the seed gives, not the case you
  want to examine
- the `editor` example needs every shape hand-built, and carries a known ~50%
  flake on the "raise a tower" beat

Neither lets a person flip through twenty deliberate cases.

And the case that matters most is the hand-built one. The owner's L measured 0%
coplanar with every plate's four corners fallen, against 58.6% coplanar on the
generated row - so small deliberate shapes are the WORSE subject, and they are
what a player builds. Judging shape work on the generated row understates it.

## Contents

- a fixed roster of hand-placed structures: the owner's L, straight runs of
  varying length and thickness, a T, a cross, a lone cell, a plate ending against
  open space, an inside corner, a hull with fittings on several faces
- each clad, laid out in a row with a name, at a fixed pose with `freeze_bodies`
  so runs are comparable
- `--style <id>` and a key to step looks, matching what `wfc_ships` already does
- the skin report printed per subject, so the picture and the numbers come out of
  one run

## Why it comes second

`20260816-112429` (the plate interior) cannot be judged honestly without it.
Every A/B render judged in the first half of the skin work was invalid because
`freeze_bodies` did not exist and subjects rotated between runs at a rate that
depended on machine load. Comparability has to be built in, not assumed.

## Inherited questions (from 20260816-112429, closed into this bench)

Task 20260816-112429 closed with two open questions. The bench answers both;
neither needs new machinery, only that the report and renders surface them.

1. **Do creased plates read as a defect?** The interior is still a fan off one
   centre vertex, so `Spur` and uneven `Step` carry a soft crease (89% of all
   measured creasing). The owner looked at the current wfc row and likes it.
   The bench must make the call checkable on the WORST subjects - the small
   hand-built shapes where every corner falls. Report per subject: count of
   creased plates by relief class, so a render can be judged next to its
   numbers. No fix is planned unless the bench renders show one is needed.
2. **Three measured style regressions, unjudged.** Against the pre-styles skin:
   `near_fitting` lost most of its reach, `salvage_hook` is absent from a third
   of a row, industrial density roughly doubled. The bench's fitting-bearing
   subject plus `--style` stepping is the instrument. Report per subject and
   style: decoration count and `near_fitting` hit count. Judgment stays with
   the owner; the bench only has to make the comparison possible in one run.

## Definition of done

- one command renders the whole roster, clad, named, at a repeatable pose
- a style swaps without a rebuild
- the skin report for each subject is emitted with the render
- the owner's L is in the roster as a named case
- the report carries the two inherited counts above (creases by relief class,
  decoration/`near_fitting` counts per style)

## Lane

shape-bench
