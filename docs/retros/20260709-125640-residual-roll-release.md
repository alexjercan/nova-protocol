# Retro: residual roll after autopilot release

- TASK: 20260709-125640 (nova) + 20260711-091519, 20260711-094942 (bcs)
- BRANCH: fix/residual-roll-release (squashed to master as f43e550);
  bcs branches fix/pd-fast-spin-damping (13e33e5) and
  fix/pd-backward-euler-gains (d9e13e1)
- REVIEW ROUNDS: nova 2 (R1: 1 MAJOR; R2 APPROVE); bcs cycles reviewed in
  their own repo (see bcs docs/retros/)

## What went well

- The one-line dependency fix was carried by diagnostic discipline: an
  ignored `#[test]` tick trace (spin, command error, PD output, per-tick
  around the release) turned "it corkscrews forever" into a precise
  terminal-state description in one run, and the cargo `[patch]`
  path-override A/B against the fixed dependency settled root cause in
  minutes.
- Cross-repo flow held its shape: two bcs tasks ran their own full
  plan-work-review-retro cycles in the bcs repo, nova's task tracked them
  as dependencies, and each repo's trail is self-contained.
- Falsified theories were recorded as outcomes, not buried: the task file
  now reads as "starving theory -> wrong; limit-cycle theory -> right
  description, wrong cause; frame order -> actual cause", which is what
  the next physics bug hunt should expect of itself.
- The reviewer sweep for OTHER guards referencing the task (grep for the
  task ID / "cannot damp") caught the second, stale bound in ai.rs that
  the implementer forgot (nova R1.1) - the fix landed with both guards
  tightened (2.0 -> 0.5 release spin; 0.5 -> 0.05 AI settle roll).

## What went wrong

- A whole task (bcs backward-Euler conditioning) was planned from a trace
  gathered against the UNFIXED dependency. The limit-cycle reading was a
  faithful description of the terminal state, and the plan encoded it as
  the mechanism. Root cause: fix #2 was designed before A/B-ing fix #1;
  the path patch that falsified it took five minutes and was available
  from the moment the frame fix landed.
- The original 2026-07-09 evidence note "command parked ON its attitude
  keeps spinning forever" was trusted as a pure-damper statement, but the
  harness never behaved that way at the crate boundary - reproducing the
  claim in isolation (bcs task 1) is what redirected the hunt. Evidence
  notes should record the exact rig (which systems ran, which command
  path) or they mislead the next session.

## What to improve next time

- After landing a dependency fix, the FIRST downstream experiment is
  re-running the original symptom against it (cargo `[patch]` +
  path), before interpreting old traces or planning further fixes.
- Degenerate inertia is a trap worth remembering: avian's eigen sort
  hands even an axis-aligned symmetric ship a cyclic-permutation
  local frame, so "my body is a plain box, the local frame is identity"
  is false in general. Any frame-composition code must be tested with
  both-frames-non-identity cases; single-frame cases pass under either
  order.
- Keep using the ignored-diagnostic-test pattern for physics bugs; it
  beats theorizing every time it has been tried. Delete it in the same
  branch once the regression guard is tight.

## Action items

- [x] Both bcs follow-up tasks CLOSED (one fixed, one falsified) with
      retros in the bcs repo.
- [x] Guards tightened in nova (flight release spin, AI settle roll).
- [ ] None outstanding; pushing nova master is the user's call (bcs
      master was pushed as a hard prerequisite of the rev bump).
