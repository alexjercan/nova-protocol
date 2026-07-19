# Review: depth markers (probe-all T3)

- TASK: 20260719-210450
- BRANCH: feature/probe-depth-markers (stacked on T2)
- ROUND: 1

## What I tried to break

- **Marker overreach** (the goldens failure mode): every addition sits at
  an EXISTING assertion or stage site - nothing new is promised, the
  markers only record what the harness already enforces. The monotonic
  audit rejected every candidate (broadside's `act` resets on Retry; the
  ranges have no growing variables) - recorded as decisions, not
  omissions.
- **Emission correctness**: all once-guarded (flag-flip observation with
  `*_marked` fields; broadside buffers in `advance` because state is out
  of the world there, flushing at the single insert point). Live
  validation shows exactly one marker per outcome and exactly 11 stage
  markers - no per-frame spam, no duplicates.
- **Assertion interference**: markers fire AFTER their asserts pass (an
  outcome marker cannot mask a failing range); the borrow shapes compute
  flags first, drop the resource borrow, then mark - no runtime panics
  across 8 live runs.
- **Stacked-base staleness**: the branch merged master post-T2-landing
  before validation, so the validation ran against exactly what will
  land (including the perf_baseline exit fix).

## Findings

- R1.1 (NIT, accepted): hull's partial-hit marker records `health_after`
  but not the expected value (it is in the assert message); good enough
  for the timeline's purpose.
- R1.2 (NIT, recorded): broadside stage markers carry the note text from
  `advance` - if stage notes are ever reworded, timeline comparisons
  across versions will differ on `note` while `stage N` names stay
  stable. Compare by name, not note.

## Verdict

APPROVE - land per the stacked-flow authorization.
