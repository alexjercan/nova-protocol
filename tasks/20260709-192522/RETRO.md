# Retro: Focus dwell + component fine-lock

- TASK: 20260709-192522
- BRANCH: feature/component-fine-lock (squash-merged as the focus-dwell
  commit on master)
- REVIEW ROUNDS: 2 (round 1 APPROVE with 1 NIT test gap, added; round 2
  APPROVE)

## What went well

- **Pure-core, thin-system held again.** ray_distance / snap_pick /
  cycle_order carry the tricky rules (behind-origin clamp, hysteresis
  identity case, stable ordering) as plain functions; the systems are
  plumbing. The whole review found only a missing test, no logic issues.
- **The dwell reset semantics fell out of change detection.** Resetting on
  the target-change frame without accruing makes FOCUS_TIME a true
  continuous hold; the test asserting 1.0 s after two ticks pinned this
  down before it could regress.
- **Feel knobs are named constants with spike references** (FOCUS_TIME,
  COMPONENT_PIN_WINDOW, SNAP_HYSTERESIS), so the playtest retune has one
  place to go.

## What went wrong

- **`Time::default()` ambiguity cost a compile round** - the generic clock
  needs `Time::<()>::default()` in plain-World tests. Same lesson family as
  the RunSystemOnce import: test-harness idioms differ from app code; check
  the last cycle's test module before writing new ones.
- **An input-path branch shipped untested (R1.1)** - the unfocused-cycle
  no-op gate. Root cause: tests were written from the state-machine
  perspective (focus, snap, pin) and the input observer's own gate was
  assumed covered by symmetry. Input entry points deserve their own
  negative test even when the gate is one line.

## What to improve next time

- Plain-World test setup: Time::<()>, RunSystemOnce - start from the
  previous cycle's test preamble instead of rediscovering it.
- For every new input observer, write the does-nothing-when-gated test
  first; it is one assert and it documents the gate.

## Action items

- None new; turret consumption of this state is next (20260709-173700).
