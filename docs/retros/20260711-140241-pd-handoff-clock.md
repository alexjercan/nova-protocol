# Retro: PD command handoff clock + the boundary bounce

- TASK: 20260711-140241
- BRANCH: fix/pd-command-handoff-clock (squash-merged as ee0bdba)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

The cycle where the "hygiene" task turned out to require a real control
fix - and the WARNING left by the previous cycle is what made that a
planned diagnosis instead of a shipped regression.

## What went well

- **Cross-cycle warnings work.** 140234's TASK.md warning (with measured
  numbers and an explicit falsification exit) turned what would have
  been a wobble regression shipped in good faith into a
  diagnose-first prerequisite. The mechanism (boundary bounce) was found
  with one frame-by-frame trace at the exact failing phase.
- **The trace corrected the previous cycle's mechanism label.**
  "Staleness is accidental dither breaking a re-aim limit cycle" was
  directionally right but named the wrong mechanism: the cycle was
  POSITIONAL (the finishing burn's spool tail burning through zero and
  exiting the standoff), not rotational. Root-cause fixes at the right
  layer (demand cutoff) made BOTH wirings quiet instead of preserving a
  load-bearing accident.
- **Regressions on the real plugins.** The staleness regression
  registers NovaFlightPlugin + ControllerSectionPlugin rather than
  hand-wiring the systems, so it pins the SHIPPED schedule - the exact
  divergence class this family kept tripping on.

## What went wrong

- **The first staleness bound was set below f32 quaternion precision**
  (1e-4 vs the ~1e-3 angle_between noise floor for near-identical
  rotations); the post-fix run "failed" at 0.00098 rad. Cheap to catch,
  but the same mistake in a flakier rig would have been a heisen-test.
  The committed bound documents the floor.

## What to improve next time

- When asserting angles between nearly-equal f32 quaternions, the floor
  is ~1e-3 rad (acos near dot=1); set bounds an order above it and say
  so in the test, or compare components instead.
- When a fix's effect differs across an environmental factor (wiring,
  frame rate), suspect a knife-edge in a POSITIONAL/stateful variable
  before a dynamics explanation - "which side of a threshold does the
  residual land on" explained everything the dither story hand-waved.

## Action items

- [x] Ledger: `quat-angle-noise-floor` seeded (x1); `diagnostic-first`
      bumped (the trace beat the dither theory);
      `cross-cycle-warning-with-numbers` seeded as a positive pattern.
- [ ] None outstanding; the spike family is fully closed (fix record).
