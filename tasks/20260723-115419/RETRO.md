# Retro: Fix final_tally_claim survey->picket tests

- TASK: 20260723-115419
- BRANCH: fix/final-tally-survey-picket
- REVIEW ROUNDS: 1 (APPROVE, out-of-context, no findings)

## What went well

- INSTRUMENT before committing to a fix. My first hypothesis (the objectives
  are breathe-gated, so just advance the clock) was INCOMPLETE - it left the
  tests red. A one-line debug dump (`surveyed / picket_gate / picket_posted /
  elapsed / objs`) showed `picket_posted=None`, which pointed straight at the
  real cause (an unseeded variable) in a single step. Guessing twice would have
  cost two more compile cycles.
- Fixed the ROOT cause (a drifted rig), not the symptom: `seed_live_claim`
  claimed to "seed the whole OnStart block" but had silently fallen behind the
  content. Completing it to mirror OnStart exactly is the faithful fix
  (`production-faithful-rigs`), and it keeps every other test in the file
  honest too.
- Confirmed it was a stale test, not a content regression, with evidence
  (git show 0ae5c7f9 touched content but not the test; OnStart genuinely seeds
  the 5 vars) before changing anything - so the fix didn't paper over a real
  bug.
- The payoff landed: with this + the earlier content_lint_gate fix, the full
  `cargo test -p nova_assets` is green end to end for the first time this
  session.

## What went wrong

- Wasted one iteration on the incomplete "just advance the clock" hypothesis
  before instrumenting. Root cause: I reasoned from the content's gating
  mechanism (breathe delay) and stopped at the first plausible cause instead of
  dumping the actual variable state first. The dump should have been step one.

## What to improve next time

- For a scenario/variable-driven test failure, DUMP the relevant variables +
  objectives at the failure point BEFORE forming a fix hypothesis. The state is
  cheap to print and usually names the cause outright.

## Action items

- Added a ledger lesson `seed-helper-drifts-from-source`: a hand-maintained
  "seed/mirror the whole <source> block" test helper rots silently every time
  the source grows a field; pin the helper's key set against the source (or
  generate it) so the drift fails loudly. (A sharper, actionable form of the
  already-promoted `production-faithful-rigs`.)
