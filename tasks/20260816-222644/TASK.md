# Greeble batch: armoured, the military

- STATUS: IN_PROGRESS
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
