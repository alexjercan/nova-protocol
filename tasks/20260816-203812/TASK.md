# Greeble rule repair: give the signatures reach

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.11.0,art,skin,content

## Goal

Follow-up 1 of the greeble spike (tasks/20260816-194637/GREEBLES.md section 8,
approved by the owner). Rules only, no new art.

Fix the three zero-reach signature pieces (civilian_windows, armoured_sensor,
industrial_radiator - their Flat+min_run+min_depth filters describe generated
hulls, not player builds) and give civilian + salvage one thin-shape carrier
each by loosening one existing filler per style (the armoured cap's filter
shape is the pattern: cone-friendly, seat Any, no min_depth 2).

## Done when (the doc's bar, measured on the shape_bench report)

- every style places >= 1 piece on every bench subject except lone_cell
- the three signatures each reach > 0 plates on at least three subjects

## Notes

- Rules live in crates/nova_authoring/src/base_content/styles.rs; regenerate
  with `content -- gen`. Never hand-edit the generated .content.ron.
- Existing tests pin kit caps and rule doctrine; update pins deliberately,
  with the reasoning in the assertion message.

## Closure

Landed as 75cec156 (2026-08-16), lane greeble-flow commit f1971908. All
filter changes and the measured bar are in the lane report; headline:
- civilian_windows Flat/Step+Side+min_run+min_depth -> Flat/Step/Brink+Side
- armoured_sensor Flat+min_run+min_depth -> Flat/Step
- industrial_radiator stride 2 chance 1.0 -> stride 1 chance 0.5
- armoured_cap Spur -> Spur/Ridge/Peak (stride parity accident on the L)
- thin carriers: industrial_stack, civilian_fairing (+Spur, seat Any),
  salvage_patch_scab (seat Any)
Bar met: every authored style >= 1 piece on every bench subject except
lone_cell; windows/sensor/radiator each reach 3 subjects. Seat opt-out pin
renamed and extended with reasoning.

Known lever for the tuning pass: fill_patches keys its floor on
(origin-octant block, face), so a centred hull can draw ~24 floor pieces
from one patch:10 rule - the scab's thick-hull counts are floor-driven.
