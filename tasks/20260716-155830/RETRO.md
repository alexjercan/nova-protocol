# Retro: Remove deep mod-content behavior tests from core CI

- TASK: 20260716-155830
- BRANCH: refactor/drop-mod-content-tests (landed bfd2a9ef)
- REVIEW ROUNDS: 1

## What went well

- The coverage audit reframed the task before any deletion: the real gap
  was that filters.rs owned the filter/action semantics with ZERO tests -
  the per-mod content tests had been silently standing in for engine
  coverage. Deleting first and back-filling later would have shipped a
  hole; the audit-then-delete order is what made this a net win
  (-196 lines AND stronger engine pins).
- The review round ran its own mutation (fails-open flip) instead of
  trusting the close notes; the pin went red exactly as claimed.
- The sweep caught a comment in broadside_assault.rs that cited the
  deleted file as its division-of-labor referent - prose references to
  TEST files rot too, not just to mechanisms.

## What went wrong

- One verify run used `cargo test -p nova_scenario` solo although the
  ledger already records that it does not compile (feature unification).
  Cost: one confusing red run mid-verify. The lesson existed; it was not
  re-checked before typing the command.

## What to improve next time

- When a task deletes tests over shipped data, treat it as a coverage
  REPAIR task first: list which mechanism assertions the doomed tests
  uniquely carry, re-pin those at the owning crate's boundary, only then
  delete.
- Before running crate-scoped cargo test, grep the ledger for the crate
  name; known-broken invocations are recorded there.

## Action items

- [x] Ledger: bumped crate-solo-tests-miss-unified-features (x2), added
      deleted-content-tests-carry-engine-coverage (x1).
