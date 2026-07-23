# Retro: Fix stale content_lint_gate test (ledger ch4 mutually-exclusive warn gone)

- TASK: 20260723-103523
- BRANCH: fix/content-lint-gate-ledger-ch4
- REVIEW ROUNDS: 1 (APPROVE, out-of-context, no findings)

## What went well

- Diagnosed with REAL data before touching the test: ran
  `content -- lint --target the-ledger` and read the actual output (0/0/1-acked)
  plus the ack reason text, which spelled out that the dual spawn was removed on
  purpose. That turned "is the test stale or is the content broken?" into a
  documented answer, not a guess.
- Re-pinned on a DURABLE signal instead of deleting the assertion: switched to
  `collect_target` to reach the acked auditor exception (a recorded, playtested
  design decision) rather than an incidental warn. The test still means
  something - it fails if attribution breaks, an error regresses, or the ack
  vanishes - which is exactly the `would-it-fail-without-it` / pin-durable-
  intents bar.
- merge-red / check-source-first discipline caught TWO false alarms: both
  content_lint_gate and the newly-surfaced final_tally_claim failures looked
  like "my change broke the suite", but a master re-run proved both inherited.
  One cheap check each, correct attribution, no mis-blame.
- Kept scope tight: the final_tally_claim failures (different subsystem) went to
  their own task (115419) instead of widening this branch.

## What went wrong

- Nothing in the fix. The only friction: the full `cargo test -p nova_assets`
  carries MULTIPLE pre-existing inherited failures (content_lint_gate AND
  final_tally_claim), so "the suite is green" is not a usable gate on this repo
  right now - the local-merge-skips-CI reality means master ships red tests.
  Root cause is upstream (the 20260722 rework landed with stale tests); this
  task clears one, 115419 will clear the other.

## What to improve next time

- When a task's DoD says "full suite green" on a repo that lands via local
  squash + advisory CI, sanity-check the suite's CURRENT baseline on master
  first - there may be pre-existing reds that make "full green" the wrong gate.
  Scope the DoD to the specific guard the task owns (here: the one test file)
  and file the rest.

## Action items

- [x] Filed 20260723-115419: the inherited final_tally_claim survey->picket
  failures (committed to master).
- No new ledger lesson: the applied lessons (merge-red/check-source-first,
  would-it-fail-without-it/pin-durable-intents, local-merge-skips-the-guarding-ci)
  are all already promoted.
