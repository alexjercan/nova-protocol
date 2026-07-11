# Retro: Ship twitch at high velocity - re-test against the impulse fix

- TASK: 20260710-231931
- BRANCH: fix/ship-twitch-retest (squash-merged as f76a06b)
- REVIEW ROUNDS: 2 (R1: 1 MAJOR; R2 APPROVE)

Verification-only spoke of the twitching family; close-out and evidence in
the task file, family status in the spike doc's fix record.

## What went well

- **The plan was corrected before implementation, not after.** The task's
  original "straight-line burn" regression encoded the spike's
  pre-diagnostic model and would have been vacuously green (parallel
  offsets produce no torque; coasting produces none at all). Rewriting the
  Steps to the cross-velocity regime BEFORE writing the test (the work
  skill's update-steps-first rule) is the only reason the regression means
  anything.
- **A/B against the reverted fix, cheaply.** Temporarily restoring the old
  impulse body in the worktree turned "should be fixed" into 4.26 -> ~0
  rad/s in one compile cycle - the residual-roll retro's path-patch
  discipline, applied in-repo.
- **Honest deferral instead of a fake verdict.** The "re-test visually"
  step was closed as DEFERRED to the umbrella's user playtest with the
  reason written down, rather than claiming a feel verdict a headless run
  cannot produce.

## What went wrong

- **R1.1 (MAJOR): the regression asserted only that nothing happens.** A
  steady hull and a silent engine were indistinguishable - if the
  FlightIntent -> manual burn -> throttle seam ever changed, the test
  would keep passing while guarding nothing. Root cause: the test was
  designed around the failure mode (spin) without asking what proves the
  STIMULUS was delivered; the neighboring off_center test self-proves its
  engine fired and the pattern was not copied.

## What to improve next time

- Any "X must stay zero / nothing must happen" regression needs a paired
  delivery assertion proving the provoking stimulus actually fired.
  Second occurrence of a vacuous-test lesson in this family's cycles
  (camera-twitch R1.2 was presence-vs-behavior); if a third shows up,
  promote a "no null-assertion without a delivery guard" note to the
  review skill's checklist.

## Action items

- [x] None new; family continues with 20260710-231930 (bullets), then the
      HUD pair, then the umbrella's combined verification + user playtest.
