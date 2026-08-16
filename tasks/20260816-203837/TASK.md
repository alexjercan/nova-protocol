# Bench building blocks: ship-worthy shapes for judging styles

- STATUS: CLOSED
- PRIORITY: 59
- TAGS: v0.11.0,example,skin,harness

## Goal

Follow-up 2 of the greeble spike (tasks/20260816-194637/GREEBLES.md section 7,
approved by the owner). An example just like shape_bench: a BUILDING BLOCKS
roster - larger hand-placed shapes that pass the owner's bar of "well this
thing looks nice, it can be used for an actual ship" - clad, named, held
still, idle orbit, --style swap, skin report per subject.

Runs on the SAME sprout as 20260816-203812 (rule repair), sequentially after
it, so the blocks are first judged with repaired rules.

## Done when

- the doc's section 7 definition of done
- renders of the blocks set per style land in this folder for owner review;
  this is the evidence for the block-ships-as-mainline-cast decision and the
  gate before the vocabulary batches (follow-up 4)

## Closure

Landed as 75cec156 (2026-08-16), lane greeble-flow commit c1ac1223.
`cargo run --example block_bench --features debug`. Eight shapes: wedge_8,
spine_freighter, outrigger, tower_ship, carrier_deck, trench_hull,
owners_l_2x, asym_gunship - all lint clean, exits unblocked. Per-style
renders in this folder are the owner review evidence for the vocabulary
batches and the block-ships-as-mainline-cast decision.

Lane's honest read: trench_hull, carrier_deck, wedge_8 pass comfortably;
spine_freighter, asym_gunship, owners_l_2x pass; outrigger plain;
tower_ship weakest (all-cone spire).

Facts for batch A: asym_gunship's gun pocket reads 0 for every seat-gated
near_fitting rule (the plates beside a boom gun are cones) - armoured ammo
stripes MUST be cone-friendly or they never sit beside that well.
carrier_deck's tucked drive zeroes strided pocket rules via lattice parity.
