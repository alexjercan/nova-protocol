# Retro: Picker - group + order scenarios by campaign, inline position prefix

- TASK: 20260723-095930
- BRANCH: feature/picker-campaign-grouping
- REVIEW ROUNDS: 1 (APPROVE, out-of-context, 1 NIT fixed)

## What went well

- Genuine test-first on the ordering: wrote
  `listed_scenarios_groups_campaign_members_in_order` against the OLD
  alphabetical sort and WATCHED it fail (with the exact left/right vectors)
  before implementing the grouped sort. That red proved the test distinguishes
  grouped from alphabetical - the reviewer independently re-derived the same.
- Chose the right eyeball altitude for a text-list "layout":
  `picker_rows_render_campaign_grouped_and_prefixed` reads the ACTUAL spawned
  row Text in child (display) order through the real refresh->spawn path. That
  is seeing the rendered widget content deterministically and headlessly - it
  would catch a spawn path that dropped the label or the order, which a pure-fn
  test would not.
- Reused one `scenario_row_label` helper for both the list row and the
  details-pane title, so the campaign prefix reads consistently and there is a
  single testable seam.
- The `check-all-targets-for-struct-field`-style discipline paid off in
  reverse: the fallback fixtures are all uncampaigned, so I could reason (and
  the full 67-test suite confirmed) that first-listed identity was unchanged -
  no fixture churn needed.

## What went wrong

- The pixel-screenshot eyeball was a time sink. I added a temporary capture
  beat to `screenshot_ui`, but the capture-path settle frames overrun the fixed
  autopilot window on this software GPU - a limit the rig's OWN comments already
  document (llvmpipe, task 20260716). Two long debug recompiles later it still
  produced no PNG. Root cause: I did not read those comments before attempting
  the capture; they predicted exactly this.
- Minor: a test-count miscount in the close-out ("66 pre-existing + 3 new"
  vs the true 67 total) - the reviewer caught it. Root cause: I quoted an
  earlier suite run (before the 3rd test existed) from memory instead of the
  final number.

## What to improve next time

- For a render eyeball, reach for the widget-level assertion (read the spawned
  Text/Node tree) FIRST; treat a pixel screenshot as optional polish, and
  before attempting one, read the capture rig's constraints (window/settle
  budget, GPU) so a known-unfittable capture is not attempted.
- Write prose numbers from the FINAL run, not memory (this is the promoted
  "write prose from the diff/final state" rule applied to test counts).

## Action items

- No new ledger lesson: the applicable lessons (`render-output-eyeball`,
  write-from-final-state) are already promoted. The screenshot-window limit is
  already documented in the rig itself.
- Filed earlier this run: 20260723-103523 (inherited content_lint_gate
  failure) and 20260723-095951 (deferred richer campaign-header UI).
