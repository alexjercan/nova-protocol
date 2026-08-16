# A shape bench for judging the skin

- STATUS: CLOSED
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

shape-bench, landed as a1d65470 (2026-08-16). Reproduce everything below with:
`cargo run --example shape_bench --features debug` (add `-- --style <id>`;
L steps the look, C strips cladding; report prints for all five styles per run).

## Findings delivered (the inherited questions, answered with numbers)

Question 1 - creased plates. The split is THICKNESS, not shape: every
one-cell-thick subject is 100% creased (owners_l 21/21, lone_cell 6/6,
run_2 10/10, run_5 22/22, tee 32/32, cross 34/34 - all ridge/peak/spur).
Flat and brink surfaces only appear at thickness >= 2 (run_5_thick 24/48
creased, plate_open 24/48, inside_corner 30/43, fitted_hull 24/39 - the
non-creased share is flat/brink plate). Render confirms it: thin subjects
read as faceted rock, thick subjects read as plated hulls.

Question 2 - style regressions. near_fitting hits exist only on the fitted
hull (industrial 2/3, civilian 3/3, salvage 4/12, placeholder 3/3, armoured
has no near_fitting rules). The sharper finding: SALVAGE PLACES NOTHING on
small hull-only subjects (0 pieces on owners_l, tee, cross, run_2, run_5,
lone_cell; salvage_hook 0 everywhere except x2 of 3 on fitted_hull).
Industrial is similarly absent on thin shapes (0 on tee, cross, run_5).
Armoured is the only style that dresses thin shapes (10 on tee, 18 on cross).

Judgment stays with the owner. If either number set reads as a defect on the
render, open a NEW task with the bench subject as the repro.

## Noted in passing (unfixed)

- fitted_hull reports 6 bare hull faces: 4 are muzzle mouths (FiresInto, on
  purpose), 2 presumably NoSocket against fitting flanks.
- wfc_ships and shape_bench used capturing() outside a debug cfg while its
  import sat inside one, so neither built without --features debug - which is
  what CI's default-features `--all-targets` gate runs. Introduced by 00fcdf04,
  copied by the bench. FIXED in-session right after landing: both examples now
  import `nova_debug::prelude::capturing` unconditionally (nova_debug is an
  ungated root dependency; only the `nova_protocol::nova_debug` PATH is
  feature-gated). Verified in both feature states.
