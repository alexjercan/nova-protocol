# Review: split fps pass + scene looping (S2)

- TASK: 20260720-000616
- BRANCH: feature/fps-pass-loop (+ bcs v0.19.4)
- ROUND: 1

## What I tried to break

- **Comparability**: reload intervals are excluded plus the boundary
  frame after the gate closes (its delta spans the reload); the report
  line carries count/mean/max so nothing is hidden. Two looped runs'
  distributions are like-for-like scene frames.
- **Contamination**: the clean pass can no longer arm the capture at
  all - the split is structural, not conditional; sweep cells were
  already capture-only (precedent honored, not duplicated).
- **Loop-cycle hazards**: three found by the e2e, three fixed at their
  honest sites (Option params, the reloading gate, seed-waiting) - each
  a first-cycle protection the loop stripped; the close-out records the
  class for the ledger.
- **Unenrolled behavior**: broadside proves the no-loop path still
  completes via the S1 wait (idle tail, documented); the smoke suite's
  sentinel greps are untouched.
- **Published-tag truth**: final 80/80 against the pushed v0.19.4, lock
  source verified; all five temp path deps restored.

## Findings

- R1.1 (NIT, accepted): playable never looped on this host (its window
  fits); its loop path is exercised only by scenario here. A slower host
  or bigger window will exercise it; the mechanism is shared code.
- R1.2 (recorded): 233732 (partial-emit) is now purely the deadline
  net + skip diagnostics; its body already says so.

## Verdict

APPROVE - land.
