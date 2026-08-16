# Greeble rule repair: give the signatures reach

- STATUS: IN_PROGRESS
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
