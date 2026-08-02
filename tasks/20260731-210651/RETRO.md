# Retro: Make the local test suite runnable: cap link jobs and drop test-binary DWARF

- TASK: 20260731-210651
- BRANCH: master (already landed)
- REVIEW ROUNDS: 1

## What went well

- Measured isolated linker RSS, embedded DWARF, and whole-suite RSS instead of
  trusting the existing mitigation comment.
- Reversed the initial `split-debuginfo = "off"` plan when measurements showed
  it would push dependency DWARF back into the linker.
- The jobs cap and profile changes reduced the full-suite peak to 8.19 GiB on
  the 31 GiB development box.

## What went wrong

- Existing comments asserted unmeasured effects. Two replacement numbers were
  also initially asserted and required the follow-up correction `ac70dba8`.
- The first full-suite proof was obscured by an unrelated shakedown failure,
  leaving this completed task stuck in REVIEWING after landing.

## What to improve next time

- Record the command beside every measured build number and re-run it after the
  final edit.
- Separate resource-exhaustion success from unrelated test correctness when a
  suite exercises both.

## Action items

- Submitted central lesson `measure-link-memory-knobs-and-concurrency` with
  project and task provenance.
- No follow-up implementation required. The unrelated shakedown test is green
  on the current tree.
