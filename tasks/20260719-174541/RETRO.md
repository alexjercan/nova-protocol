# Retro: probe hardening

- TASK: 20260719-174541
- BRANCH: fix/probe-hardening (squash-landed as ae506128)
- REVIEW ROUNDS: 1 (APPROVE; R1.1 label pin fixed in-round)

## What went well

- The fix-the-findings shape worked cleanly: the fresh-eyes review's
  numbered findings became the task's Story verbatim, each fix verified
  against its finding at review time, and the three e2e proofs each
  invert a specific live repro (misdirecting skip details, the
  abort-without-report timeout, the OK-over-nothing verdict).
- One design refinement was argued, recorded and re-encoded in the steps
  BEFORE implementation (OK-with-coverage vs NO_DATA) instead of being
  discovered as an inconsistency at test time.
- The manifest unified three findings (stale identity, dropped exit
  status, missing run metadata) into one artifact - cheaper than three
  point fixes and it hands the consolidation task its report gate.

## What went wrong

- The landing itself: the squash-land was chained onto a command that had
  cd'd into the WORKTREE, merging the branch into itself (a no-op,
  caught by the "nothing to squash" output). The landing-no-cd lesson
  (x3, promoted) exists precisely for this; the sync step and the land
  step belong in SEPARATE commands, the land alone from the main
  checkout.

## What to improve next time

- The land is its own command, never appended to a chain that changed
  directory - re-read the promoted lesson before landing, not after the
  no-op.

## Action items

- [x] Lesson bumped: landing-no-cd (x4) - the chain variant.
- [ ] Consolidation task 20260719-174603 is next (same branch lineage:
      manifest gate ready).
