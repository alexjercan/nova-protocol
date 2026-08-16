# Greeble batch: industrial, the builders

- STATUS: CLOSED
- PRIORITY: 57
- TAGS: v0.11.0,art,skin,content

## Goal

Vocabulary batch, owner-approved: industrial - THE BUILDERS. Fiction: a
working shipyard that never left the ship. Art direction: EXPOSURE
(GREEBLES.md section 2) - everything serviceable outside with a part number;
yellow keeps its three-use discipline; a piece must be something a fitter
would unbolt.

## Pieces (7 new; recipes + rules + models)

From the approved matrix (GREEBLES.md section 3):
- industrial_cells: open battery rack, yellow terminal collar
- industrial_stencil: unit number + hazard diamond panel - FLAT decal
  geometry, a thin-shape carrier (cone-friendly filter, no min_depth)
- industrial_winch: winch drum with cog flank, near fittings - deck machinery

Owner-approved additions (batch C):
- industrial_crane: folded jib arm on a pedestal - builders hoist things
- industrial_plate_rack: stacked spare hull plates lashed flat - they carry
  their materials
- industrial_floodlight: work-light cluster aimed at fittings
- industrial_umbilical: a row of capped sockets - ships get built plugged in

## Kit cap

7 -> 14. Update ONLY your own style's cap pin; do NOT touch the shared
cap-ordering assertion - the coordinator re-pins it after all batches land.

## Done when

- greeble_catalog shows all 7 with correct labels and materials
- block_bench per-style render shows them placed sanely (no confetti)
- style tests pass with the new cap

## Closure

Landed 2026-08-16, lane batch-industrial. All seven pieces shipped; kit is
14, pinned. Honest read: EXPOSURE, not confetti - bands trace edges, cranes
lean overboard from the Brink stubs the band refuses, racks and conduits
cluster on decks, sockets row on flanks, stencils dress the thin shapes
(7-13 each).

For the tuning pass:
- block_bench exercises NO industrial pocket rule (near_fitting 0 of 0
  bench-wide at stride 2); winch and floodlight are proven as objects in the
  catalog but never placed by this roster - the civilian lane's stride-1
  fix is the known cure
- the styles.rs header prose "Seven rules opt out with ScatterSeat::Any" is
  stale across the batches; coordinator refreshes it with the ordering
  re-pin
