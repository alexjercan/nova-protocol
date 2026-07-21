# Retro: Harness-prove ally allegiance + orbit-directive combat guards

- TASK: 20260721-160906
- BRANCH: test/ch3-mechanisms-rig (landed 16509993)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

## What went well

- The fail-fast task did its job cheaply: the whole ch3 design risk
  (unshipped ally mechanism) is now retired with three small rigs, and both
  content tasks got their variant decision in writing before authoring.
- Reading the existing test inventory before writing rigs shrank the task:
  two of three planned rigs were already covered (orbit guard) or better
  answered at the source (event emission); the amendments were recorded in
  the step texts instead of padding duplicate tests.
- The nearest-draw rig turned the spike's "positioning controls who draws
  fire" claim from prose into a pin the convoy tuning can rely on.

## What went wrong

- Nothing costly. The plan's rig list was written from the spike's model of
  what would need proving, before the test inventory was read - the
  execution-time amendment was the fix (verify-first-plan-steps, again, in
  its mild form).

## What to improve next time

- When a plan step says "write rig for X", the first act is grepping the
  module's existing `mod *_tests` inventory - this repo pins mechanisms
  close to their source, so coverage often already exists.

## Action items

- [x] LESSONS.md: bumped `verify-first-plan-steps` x9 -> x10 with this id.
