# Review: entity-filter id/other_id docs + scenario load/test rework

- TASK: 20260715-212600
- BRANCH: docs/scenario-filters-loadtest

## Round 1

- VERDICT: APPROVE

Docs-only change; the risk is factual accuracy of the per-event table and the
load/test claims. Re-verified the load-bearing claims directly against the code:

- OnEnter direction: `crates/nova_scenario/src/objects/area.rs:60-64` fires
  `OnEnterEventInfo { id: area_id, other_id: <entrant>, ... }` - confirms `id` is
  the area and `other_id` the body that entered (the doc's central claim).
- Filter-only nuance: `ScenarioObjectConfig::action(&self, world, _info)` in
  actions.rs takes the event info as `_info` (underscored = unused) - confirms
  actions do not consume id/other_id; it is filter-only. Verified.
- "No scenario picker yet": task 20260715-200828 is OPEN and the menu only wires
  New Game -> `NEW_GAME_SCENARIO_ID` (`shakedown_run`) in nova_menu/src/lib.rs -
  the section-7 workaround advice is accurate.
- `08_scenario.rs` builds its `ScenarioConfig` in the `showcase()` fn (Rust, not
  RON) - so cutting it as an "author test" is correct.
- Base bundle is always-enabled mod content (`assets/base/base.bundle.ron` +
  `mods.catalog.ron` `base: true`) - the "add to the base bundle" loop is valid.

`npm run ci` green; headless render confirms the per-event Entity table and the
reworked section 7 display correctly.

- [x] R1.1 (NIT, fixed) The subject column header "`id` / `type_name`" over-implied
  that both are always populated; OnEnter/OnExit carry the area's `id` but not its
  `type_name`. Added a sentence after the table noting not every event fills every
  field and a filter on an unfilled field never matches.
