# Greeble batch: armoured, the military

- STATUS: CLOSED
- PRIORITY: 56
- TAGS: v0.11.0,art,skin,content

## Goal

Vocabulary batch, owner-approved: armoured - THE MILITARY. Fiction:
everything sealed, everything numbered. Art direction: SUPPRESSION
(GREEBLES.md section 2) - low, flush, bolted, matte; detail at corners and
edges, never mid-panel; gunmetal plus ONE stencil white; smallest kit of the
four BY DOCTRINE - restraint is the identity.

## Pieces (6 new; recipes + rules + models)

From the approved matrix (GREEBLES.md section 3):
- armoured_mast: stub sensor spike, shortest of the four, raked, one matte
  radome - a warship hides its silhouette
- armoured_intake: flush shuttered slit, armoured louvres angled shut
- armoured_magazine: low bolted box, one white stencil
- armoured_ammo_stripes: white rounds-count stripes beside gun wells. HARD
  CONSTRAINT (bench-proven, task 20260816-203837 closure): the plates beside
  a boom-mounted gun are CONES - this rule MUST be cone-friendly (seat Any,
  no min_depth) or it never places on asym_gunship. Verify on that subject.

Owner-approved additions (batch C):
- armoured_applique: flat reactive-armour tile grid - a second thin-shape
  carrier that reads as doctrine
- armoured_chaff: low bolted decoy cylinder - the one piece hinting at its
  combat life

## Kit cap

4 -> 10 (stays the smallest). Update ONLY your own style's cap pin; do NOT
touch the shared cap-ordering assertion - the coordinator re-pins it after
all batches land.

## Done when

- greeble_catalog shows all 6 with correct labels and materials
- block_bench armoured render: ammo stripes sit beside asym_gunship's gun
  well; the row reads MORE dressed but still suppressed
- style tests pass with the new cap

## Closure

Landed as c9286507 (2026-08-16), lane batch-military. All six pieces shipped
as recipes + generated models + rules; kit is now 10, smallest by doctrine.
The ammo-stripes hard constraint held: cone-friendly rule pinned by test
`the_armoured_ammo_stripes_read_the_gun_pocket_off_any_seat`, x1 of 2 on
asym_gunship's gun pocket, x3 of 3 on carrier_deck, visually confirmed.
Honest read: the previously flattest row is now clearly dressed and still
suppressed - one grey family, silhouettes clean.

For the tuning pass (GREEBLES.md follow-up 6):
- magazine fires on only 2 of 8 subjects (higher-priority rules claim its
  blocks); doctrine-low but worth a look
- near_fitting cannot tell a gun well from a drive pocket, so tallies also
  appear beside drives (same limitation as louvre/beacon)
