# Retro: The beat-sheet pass

- TASK: 20260717-163058
- BRANCH: content/beat-sheet-pass (landed 488483d8)
- REVIEW ROUNDS: 1 (APPROVE; 1 MINOR + 1 NIT, both addressed post-verdict)

## What went well

- Arms before folds: writing the two lint arms FIRST turned the content
  pass into a mechanical burn-down (9 enumerated violations -> 0) and
  gave the reviewer a mutation check for free (restore master's ch1,
  watch the arm fire). First round-1 APPROVE of this flow.
- The convention doc wrote itself from the finished content instead of
  the other way around, so the wiki section cites only semantics that
  exist (dwell clamp, grace, delay, auto_advance).

## What went wrong

- A red tree got committed. The verify chain gated on
  `cargo test ... | grep "test result"`, and grep succeeds when the
  line says FAILED - the commit ran anyway. Caught one command later,
  fixed forward. Root cause: a display-grep used as a gate.
- The new one-line-per-beat arm fired on the pre-existing dwell test
  fixture (3 StoryMessages in one handler). Second time in this flow a
  new arm tripped a neighbor fixture (the swallow arm did it in
  20260717-163050). The sweep of shipped CONTENT was planned; the sweep
  of the lint module's own TEST FIXTURES was not.
- NOTES.md claimed "the writing survives" while three clauses were
  deliberately trimmed - prose written from intent, not from the diff
  (review R1.1). Second occurrence of `prose-from-diff-not-intent`,
  this time in task notes rather than the CHANGELOG.
- Flow-level: the spike's fix record was still empty at flow finish -
  none of the four landing cycles appended its entry; all back-filled
  now.

## What to improve next time

- A verification inside an &&-chain must exit non-zero on red: grep for
  "test result: ok" (or run the test bare) - never grep the result line
  just to display it, with a commit downstream.
- Adding a lint arm: grep the lint test module for fixture shapes the
  arm matches BEFORE the first run, and isolate them per arm.
- Landing a task that cites a multi-task spike includes appending its
  fix-record line - put it next to the TASK.md close, not at flow end.

## Action items

- [x] ledger: bump prose-from-diff-not-intent (x2, sharpened to cover
  task notes); new lint-arm-sweeps-own-fixtures (x2),
  chain-gates-must-fail-on-red (x1), spike-fix-record-appends-on-land (x1)
- [x] spike fix record back-filled for all four cycles
