# Retro: RCS error-relative mode for autopilot ORBIT station-keep

- TASK: 20260718-151102
- BRANCH: feature/rcs-error-relative
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- The whole change hinges on one design identity: an optional `RcsReference`
  that DEFAULTS TO ZERO, and `v - 0 == v`. That made "does not regress the
  player/STOP modes" a STRUCTURAL guarantee (every existing path leaves the
  reference unset) rather than something to hope the tests catch. 60+ existing
  flight tests passed unmodified; only the one test whose contract was
  intentionally reversed had to change.
- Front-loading a precise code map (an Explore agent quoting rcs_burn_system,
  the autopilot RCS branch, the ORBIT dispatch, the chain registration, and
  every RCS test by name with line numbers) turned a "needs a spike" task into
  a mechanical edit. The design uncertainty was genuinely narrow - only the
  cap's reference frame - so folding the spike into a NOTES.md design record
  instead of spawning a separate spike task was proportionate.
- The pre-existing `orbit_engages_from_near_rest_and_holds_the_ring_for_a_lap`
  became a free integration test of the new trim path: because RCS is
  granted-by-default, that test now exercises the trim during its hold phase,
  so its passing proves the trim is stable, not just present. Good leverage
  from an existing test.
- Same-session review re-derived the two load-bearing claims from scratch (the
  zero-reference identity and the trim's convergent sign), per the review
  skill's blind-spot rule, rather than just re-reading the diff.

## What went wrong

- Nothing structural; one-round APPROVE. Minor friction only: a `*desired`
  deref slip (desired is already a Vec3, not a Deref guard) caught immediately
  by cargo check. Cheap.

## What to improve next time

- Keep reaching for the "identity default" shape when extending a primitive
  that existing code depends on: give the new parameter a default that
  reproduces the old behavior exactly, so the no-regression argument is
  algebraic, not empirical. Worth stating as a general lesson (added below).

## Action items

- [x] Bumped `changed-shared-observer-run-the-module-suites` to x4 in
  LESSONS.md (modified two shared systems + a shared observer; ran the whole
  flight:: suite to catch regressions).
- [x] Added `identity-default-makes-no-regression-structural` to LESSONS.md.
- No follow-up code task. The RCS family's remaining item (the cap ring) was
  closed won't-do (20260718-144939). The trim handoff hysteresis and the STOP
  terminal-creep are documented known-limitations, revived only on a playtest
  signal.
