# Retro: HUD visibility levels (grave/tilde cycle)

- TASK: 20260711-180501
- BRANCH: feature/hud-visibility-levels (squashed to master a67e0fc)
- REVIEW ROUNDS: 2 (round 1: 1 MAJOR, 3 MINOR, 2 NIT)

## What went well

- Two ledger lessons were applied BEFORE they could bite, for the first
  time in this task family: the plan's verify-first step on the
  screen-indicator projection (producer/consumer schedule) shaped the
  PostUpdate placement up front, and when the naive static hint row broke
  the keybind cluster's "no rig, no keys, no hints" invariant, the
  module's own regression test - written because of the 20260708-165705
  lesson - caught it within minutes, in-session, not in review.
- The tier model (HudTier on roots + ancestor resolution for reconciled
  indicator nodes) covered all 12 HUD modules, the status bar, and the
  ephemeral holos without touching any module's internal logic.
- The e2e throwaway harness pattern (third cycle running) proved the full
  cycle against real widgets in the real app in one run.

## What went wrong

- R1.1 (MAJOR): the enforcement system was ordered after its producer but
  had NO upper bound against Bevy's visibility propagation in the same
  schedule - correct by tie-break luck, not by construction. Root cause:
  the two-clocks discipline asks "where is my input written?" but not the
  dual question "who consumes my OUTPUT later this schedule?". Ordering
  needs bounding on both sides when a system sits between a producer and
  a downstream reader.
- The ordering contract was untestable as first written (the test
  simulated the widget between updates instead of inside the schedule);
  review had to demand the in-schedule stand-in driver. Same shape as the
  previous cycle's uncommitted-harness finding: behavioral evidence kept
  living outside the committed suite.
- Two python edit scripts died on stale anchors (a global replace had
  already rewritten text the second script expected; a line-wrap mismatch
  killed a response fill). Cheap but recurring friction: anchor
  multi-edit scripts on unique, freshly verified text, and prefer one
  script per file state.

## What to improve next time

- When registering a system whose writes feed a same-schedule downstream
  stage (propagation, layout, projection), bound it on BOTH sides at the
  registration site, and make the contract executable by putting a
  stand-in producer in the real SystemSet inside the test app.

## Action items

- [x] LESSONS.md: new `bound-scheduling-both-sides`; bumped the
  `two-clocks` family (downstream-consumer variant), bumped
  `out-of-context-review-pass` to x3, noted the in-schedule stand-in test
  pattern under it.
