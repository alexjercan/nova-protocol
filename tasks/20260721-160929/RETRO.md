# Retro: base chain voice pass

- TASK: 20260721-160929
- BRANCH: content/base-voice-pass (landed 770bde4f)
- REVIEW ROUNDS: 1 (APPROVE; one MINOR fixed in-round)

## What went well

- The v0.7.0 authoring stack absorbed the whole pass with zero engine
  changes; the beat-sheet lint arms acted as design rails, not just checks -
  the one plan-vs-lint collision (shakedown epilogue line) resolved INTO a
  better design (the banner promises the call, Broadside's open speaks it).
- Driving both banner variants through the act machine in one test
  (victory_banner_reflects_the_haulers_fate) made the conditional-flavor
  mechanism verifiable end to end, not just structurally.
- The reviewer's R1.1 (pin the first-kill line's mutual exclusion) was the
  exact class of subtle content invariant that rots silently; the config
  pin now holds it.

## What went wrong

- Two older tests needed a hauler_lost seed retrofit discovered by compile
  failure rather than foresight: adding a filter to an EXISTING gated
  handler changes every rig that drives that handler. Cheap this time;
  worth a habit.

## What to improve next time

- When adding a filter to an existing handler, grep the test suite for
  rigs seeding that handler's variables BEFORE running the suite - the
  failure list is knowable from the diff.

## Action items

- none beyond the in-round fix; ch3 tasks inherit the cast + conventions
  via NOTES.md.
