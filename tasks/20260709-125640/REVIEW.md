# Review: Residual roll after autopilot release: PD cannot damp fast roll (bcs)

- TASK: 20260709-125640
- BRANCH: fix/residual-roll-release

## Round 1

- VERDICT: REQUEST_CHANGES

Verified: the four Cargo.tomls pin a35b74c and Cargo.lock resolves to it
from the git source (no leftover path patch - the root Cargo.toml diff vs
master is empty); the diagnostic test is fully removed; the tightened
0.5 guard passes and the trace showed the release parking at 0.000 rad/s,
so the margin is honest; the flight module (55 tests) is green on the
bumped rev; TASK.md records the falsified theories instead of smoothing
them over.

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/input/ai.rs:2318 - the AI
  settle test's residual-spin bound carries the instruction "tighten
  toward ~0 when the bcs fix lands" and cites this very task; this branch
  is that landing, so the step is in-scope here, not follow-up work.
  Measure the post-fix residual in that rig and tighten the 0.5 bound
  (and rewrite the comment, which still says "the bcs PD cannot damp
  (open bug 20260709-125640)" - after this branch that claim is stale).
  - Response: measured the post-fix residual in the rig at ~5e-6 rad/s;
    tightened the bound to 0.05 (four orders of margin over solver noise,
    well under the pre-fix ~0.23 amplitude) and rewrote the comment to
    record the measurement instead of the stale open-bug claim. Fixed in
    the round-2 commit.

## Round 2

- VERDICT: APPROVE

Verified the R1.1 fix: the bound is 0.05 with a measured-value comment,
the stale open-bug claim is gone, and the ai module tests pass on the
bumped rev. No new findings.
