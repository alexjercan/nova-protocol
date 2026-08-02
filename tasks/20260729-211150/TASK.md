# Scenarios picker: pin pane widths across selections + indent campaign members

- PRIORITY: 52
- TAGS: v0.9.0, feedback, bug, ui, menu
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: -

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

- [x] Reproduce first: an App-driven nova_menu test that builds the Scenarios
      screen, selects scenarios whose details differ a lot (short vs very long
      description; with vs without thumbnail), runs UI layout, and asserts the
      `ComputedNode` width of the list pane AND the details pane is IDENTICAL
      across selections. Watch it fail for the right reason (record the two
      measured widths in this task) before touching layout.
- [x] Pin the split: give the list pane a fixed basis that cannot shrink
      (`flex_shrink: 0.0` plus its `percent(40)` width, or an explicit
      `flex_basis`), and let the details pane absorb all slack
      (`flex_grow: 1.0`, `min_width: px(0)`, wrapping text). The mods screen
      has the same two-pane shape - check it for the same defect in the same
      pass and fix it if it shares the bug (do not widen beyond the two
      two-pane screens).
- [x] Indent campaign members: honour `indent` in `spawn_scenario_row` with a
      left margin (and no indent for the uncampaigned tail), so a campaign's
      chapters read as grouped under their `[-]` header. Keep the row's
      selected/hover paint intact (the `list_row` reconciler still owns it).
- [x] Test the indent: a live-tree test asserting a campaign member row has a
      non-zero left margin and an uncampaigned row has none.
- [x] Verify by RUNNING the menu (Xvfb, `nix develop --command`), not just
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

## Implementation (2026-07-29)

Reproduced in the REAL app first. New example `examples/ui/menu_scenarios.rs`
(cataloged + smoked under `ui/`) opens the Scenarios picker, clicks every listed
row and logs each selection's laid-out pane widths, then a HELD/CHANGED verdict.
`BCS_AUTOPILOT=1 cargo run --example menu_scenarios --features debug`:

- BEFORE: `scenarios pane widths CHANGED across 13 selections: list spread
  190.0px` - the list pane measured 141.0 px on `final_tally` and 331.0 px on
  `asteroid_field`, purely from the selection.
- AFTER: `scenarios pane widths HELD across 13 selections (list=331.0
  details=481.0)`.

A headless unit rig cannot see this: with no font loaded every text node
measures zero-width and nothing overflows, so there is nothing to shrink. That
is why the rig is an example and the lib test pins the layout PROPERTY instead.

- Fix: the list pane in BOTH two-pane screens (`Scenarios List`, `Mods Left
  Pane`) gets `flex_shrink: 0.0` + `flex_grow: 0.0` beside its `width:
  percent(40)`. A flex row shrinks every shrinkable item, so the default
  `flex_shrink: 1.0` let the details pane's content bid width away from the
  list. The details pane keeps `flex_grow: 1.0` + `min_width: px(0)` and
  absorbs all slack. The panes' own `min_width: px(0)` on the pinned side is
  now moot and was dropped where it sat on the pinned pane.
- Indent: `spawn_scenario_row` honours its `indent` flag (it was
  `let _ = indent;`) by writing `margin.left = CAMPAIGN_MEMBER_INDENT_PX` (24
  px, sized to the header's `[-] ` marker) onto the spawned `list_row` Node via
  `entry::<Node>()`, so the row keeps one definition of its box.
- Eyeball: `BCS_REEL=1 NOVA_SHOT_DIR=... cargo run --example menu_scenarios
  --features debug` captures one PNG per selection. Reviewed: the campaign
  chapters sit inset under `[-] Nova Protocol`, and the split does not move
  between a one-line and a five-line description. (The first cut of the capture
  beat shot one selection AHEAD of its filename - the screenshot resolves at
  frame end, after the next click had already rebuilt the panes; the beat now
  waits for the PNG before clicking on.)

INHERITED RED fixed here as merge integration, not caused by this branch:
`catalog_matches_disk` was already failing on master - `widget_zoo` was added to
the `[[example]]` catalog (commit 8aa7f004) without joining a smoke list. It is
now in `NOT_SMOKED` with its reason (own `App`, no `GameStates`, so the
reach-Playing smoke has nothing to assert).

DoD status:
1. PASS - `cargo test -p nova_menu`: 75 passed, including the new
   `two_pane_list_panes_cannot_shrink` (failed first: `left: 1.0, right: 0.0`).
2. PASS - `campaign_member_rows_are_indented_under_their_header` (failed first:
   `left: Px(0.0), right: Px(24.0)`).
3. PASS - `cargo check --all-targets --features debug` green; `cargo test
   --test examples_smoke ui_` green (70.8s, 4 examples incl. the new one);
   `catalog_matches_disk` green.
4. PASS - rendered and eyeballed (see above).
5. PENDING owner.

## Review round 1 (REVIEW.md) - all six findings applied

- R1.1 (major, real defect, caught by the reviewer eyeballing MY captures): the
  indent was a shift, not an inset. `list_row` sets `width: percent(100)` and a
  margin sits OUTSIDE that box, so an indented row measured 100% + 24 px: it
  overhung the pane and crossed the details divider (header right border at
  x~428, member rows at x~452, divider at x~446), click target included. Fixed
  by also setting `width: Val::Auto` on the indented row so the column's stretch
  alignment sizes it to the pane MINUS the indent; the test now pins the width
  too, and the re-captured PNGs show the right borders aligned.
- R1.2: `ComputedNode::size` is physical - the rig divided by
  `inverse_scale_factor` where it should multiply. Only correct at scale 1 (as
  Xvfb runs), so the recorded numbers stand, but the rig now reads right at any
  DPI.
- R1.3: the rig's CHANGED verdict was only an `error!` log, and the smoke suite
  greps for reach-Playing, so the regression this example exists to catch would
  have passed CI. It now PANICS under `BCS_AUTOPILOT` on CHANGED and on "no
  measurements", making `ui_reach_playing_without_panic` the actual gate.
- R1.4: a fixed 30 s wall-clock window against a frame-counted walk that grows
  with the scenario set would slow-flake on a software-rendered CI GPU. The
  example now SELF-ENDS (`probe: script complete, exiting` + `HarnessCompletion`
  - the broadside pattern), the window is a 180 s safety bound, and
  `guard_run_completion` panics naming the stall if a run exits unfinished.
  Side effect: the smoke run is ~10 s instead of idling out 30 s.
- R1.5: the pin covered only half the invariant (the details pane could lose
  `flex_grow`/`min_width: 0` with the test still green). It now asserts both
  sides, for both two-pane screens.
- R1.6 (nit): CHANGELOG entry trimmed to one line without the measurement.

Re-verified after the fixes: `cargo test -p nova_menu` 75 passed; `cargo check
--all-targets --features debug` green; `cargo test --test examples_smoke ui_`
green; rig verdict HELD; PNGs re-eyeballed.

## Review round 2 - three findings applied

- R2.1: `guard_run_completion` was DEAD - it reads `AppExit` in `Last`, but the
  completion watcher writes that exit in `Last` too, ordered after it. The
  broadside pattern works because it also passes `.self_completing()`, which
  makes a stalled script abort from `PreUpdate`. Added; proven by cutting the
  runway to 5 s and watching the run exit 101 naming the stall (9 of 13 rows
  measured) instead of reporting a clean cycle.
- R2.2: the guard was registered outside the `BCS_AUTOPILOT` gate, so an
  interactive run would have panicked on window close. Moved inside.
- R2.3: the 180 s runway sat ABOVE the 120 s harness completion deadline, so it
  could never be what expired. Cut to 100 s, with the relationship documented at
  the constant.
- R2.4 (nit, not taken): pin the row's computed right edge after layout instead
  of its `Node` values. A headless rig has no text measure, which is why the
  example rig exists; the geometry is captured and eyeballed there. Recorded in
  REVIEW.md.

## Out of scope, filed not fixed

`cargo test --test examples_smoke` (the WHOLE suite) has one failure -
`screenshot_nova_os` exits ~1.5 s in without completing its cycle. Verified
INHERITED: it fails identically in a clean master checkout, and this branch
touches nothing it uses. Filed as task 20260729-222131. The `ui_` smoke this
task's DoD names is green.
