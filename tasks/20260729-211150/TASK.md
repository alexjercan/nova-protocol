# Scenarios picker: pin pane widths across selections + indent campaign members

- STATUS: OPEN
- PRIORITY: 52
- TAGS: v0.9.0,feedback,bug,ui,menu

## Story

Owner playtest (2026-07-29) of the reworked main menu: "the UI for the scenario
selection still changes sizes when selecting different scenarios, that should
not happen" and "the lists in the scenario selector should be indented such that
you can easily see which scenarios are part of a campaign".

Two defects on one screen (`nova_menu`'s Scenarios picker), one eyeball:

1. Width instability. `Scenarios Content` is a flex ROW holding the list pane
   (`width: percent(40)`, `min_width: px(0)`) and the details pane
   (`flex_grow: 1.0`, `min_width: px(0)`). Both panes keep the default
   `flex_shrink: 1.0`, so when the selected scenario's details content (long
   description, thumbnail) makes the details pane's content-based basis exceed
   the free space, BOTH panes shrink proportionally - the list pane's width
   therefore depends on which scenario is selected. Task 20260729-121847 added
   `min_width: px(0)` (which stops a hard overflow) but that is exactly what
   lets the panes shrink; nothing PINS the split.
2. No campaign indentation. `spawn_scenario_row` takes an `indent: bool` and
   throws it away (`let _ = indent;` - an explicit earlier decision that the
   `[-]` header alone carries the grouping). The owner wants the member rows
   visibly indented under their campaign header.

## Steps

- [ ] Reproduce first: an App-driven nova_menu test that builds the Scenarios
      screen, selects scenarios whose details differ a lot (short vs very long
      description; with vs without thumbnail), runs UI layout, and asserts the
      `ComputedNode` width of the list pane AND the details pane is IDENTICAL
      across selections. Watch it fail for the right reason (record the two
      measured widths in this task) before touching layout.
- [ ] Pin the split: give the list pane a fixed basis that cannot shrink
      (`flex_shrink: 0.0` plus its `percent(40)` width, or an explicit
      `flex_basis`), and let the details pane absorb all slack
      (`flex_grow: 1.0`, `min_width: px(0)`, wrapping text). The mods screen
      has the same two-pane shape - check it for the same defect in the same
      pass and fix it if it shares the bug (do not widen beyond the two
      two-pane screens).
- [ ] Indent campaign members: honour `indent` in `spawn_scenario_row` with a
      left margin (and no indent for the uncampaigned tail), so a campaign's
      chapters read as grouped under their `[-]` header. Keep the row's
      selected/hover paint intact (the `list_row` reconciler still owns it).
- [ ] Test the indent: a live-tree test asserting a campaign member row has a
      non-zero left margin and an uncampaigned row has none.
- [ ] Verify by RUNNING the menu (Xvfb, `nix develop --command`), not just
      checking: select a long-description and a short-description scenario and
      see the panes hold; see the campaign grouping.

## Definition of Done

1. test: `cargo test -p nova_menu` - the pane-width test passes (and failed
   first, with both measured widths recorded here).
2. test: `cargo test -p nova_menu` - the campaign-indent test passes.
3. cmd: `nix develop --command cargo check --all-targets` green.
4. render eyeball: the menu RUN in-engine shows stable pane widths across
   selections and indented campaign members.
5. manual: owner confirms both in-engine.

## Notes

- Follow-up to 20260729-121847 (menu polish - fixed panel widths), whose
  `min_width: px(0)` fix was necessary but not sufficient.
- Do not rewrite the campaign-header UI (20260723-095951's shape stands); this
  only adds the indent it deliberately skipped.

## Flow State

- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED
