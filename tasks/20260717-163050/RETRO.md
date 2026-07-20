# Retro: Transition pacing

- TASK: 20260717-163050
- BRANCH: feature/transition-pacing (landed on master)
- REVIEW ROUNDS: 2 (REQUEST_CHANGES -> APPROVE; 1 MAJOR, 2 MINOR, 3 NIT)

## What went well

- Reading the system's TAIL before committing caught the command-flush
  starvation an early return would have caused - and the reviewer's
  matching mutation (M5) then demanded the test that pins it, closing
  the loop from read-catch to mechanical guard.
- The clock-choice analysis (virtual for the cut, real for the banner)
  was done up front and survived the reviewer's bevy-source re-derivation
  verbatim.
- The user's directive shipped as authorable vocabulary with the traps
  linted, not just a feature flag.

## What went wrong

- R1.1: two new duration fields reached Timer::from_seconds unclamped -
  authored 1e30 would panic at runtime. The sibling dwell field got the
  clamp+lint treatment ONE TASK earlier in the same initiative; the
  pattern did not transfer because it lived in another crate's file.
  When a schema gains a DURATION, the clamp+finite+lint trio is part of
  the field, not an optional extra.
- The first lint test missed that the swallow-trap arm would also fire
  on its fixture (4 warns, not 3) - fixture shapes must be trap-free
  when testing a different rule.

## What to improve next time

- New authored duration/magnitude fields get finite-check + cap at every
  construction site and a lint arm IN THE SAME EDIT as the field - copy
  the dwell treatment as the template.

## Action items

- [x] LESSONS.md: new lesson authored-durations-clamp-trio (x1).
