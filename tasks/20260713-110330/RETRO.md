# Retro: Live radar lock (threshold latch + live writes)

- TASK: 20260713-110330
- BRANCH: feat/live-radar-lock (landed 0c99f92)
- REVIEW ROUNDS: 1 (APPROVE; one doc nit folded in)

## What went well

- Moving the writes from observers into the per-frame search FORCED the
  gesture e2e rig to become production-faithful: the old rig hand-stuffed
  `RadarState.candidate`; the new one has to feed real bodies through the
  real picker off the real split-camera ray. A design simplification paid a
  test-fidelity dividend unprompted.
- All ten rewritten gesture e2e tests passed on the FIRST run - the
  adversarial round had already surfaced every semantic shift (boundary
  re-pin, pause rephrase, keep-last guard), so the tests were written
  against decided semantics, not discovered ones.
- The questionnaire-before-plan flow meant zero mid-implementation
  decisions: Q1a/Q2a were already answers, not debates.

## What went wrong

- Nothing broke, but one semantic change was IMPLIED rather than planned:
  the old "release during pause drops the commit" pin has no commit to
  drop under live-lock. The plan said "verify pause gating"; it should
  have said "the pause CONTRACT changes, decide the new pin" - caught
  while rewriting the test, recorded in the Outcome.

## What to improve next time

- When a model change moves a side effect earlier in time (commit ->
  threshold), sweep the OLD model's cancellation/abort paths explicitly at
  plan time: every "X drops the pending effect" pin either dies or needs a
  new meaning.

## Action items

- (none - 110311 consumes RadarState.engaged and RadarLockAcquired next)
