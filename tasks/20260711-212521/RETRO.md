# Retro: AI orbit directive (config, passive state, autopilot wiring)

- TASK: 20260711-212521
- BRANCH: feat/ai-orbit-directive (landed 24a161b)
- REVIEW ROUNDS: 2 (R1 REQUEST_CHANGES: 1 MAJOR, 1 MINOR, 3 NIT; R2 APPROVE)

## What went well

- Mirroring an existing pattern end to end (AIPatrolRoute -> AIOrbitDirective)
  meant every design question had a precedent answer: where the component
  lives, how config maps, how the passive arm engages the autopilot, and
  where the tests go. Implementation surfaced zero surprises because the
  spike had pre-verified the load-bearing seams (ORBIT self-plans, never
  self-completes, engages() covers all combat consumers).
- The out-of-context reviewer (sixth catch) went beyond the diff: it traced
  flight.rs completion semantics to RULE OUT a suspected stuck-state (the
  auto-park ORBIT path is player-only), turning a vague worry into a
  verified non-issue - and found the one real MAJOR.

## What went wrong

- R1.1 (MAJOR): the new no-churn test was vacuous - a re-engage produces a
  bit-identical component when autopilot_system is not in the pipeline, so
  the assert could not fail. Root cause: I copied the neighboring
  a_mid_leg_maneuver_is_left_alone test's pattern without asking
  would-it-fail-without-it; the copied pattern was itself vacuous. Copying
  an existing test shape transfers its blind spots along with its
  conventions.
- R1.2: I documented "retargeting is a non-goal" instead of noticing the
  failure mode was silent-and-permanent (ORBIT never completes, so a
  retargeted directive would be ignored forever). Declaring a non-goal is
  fine; a non-goal whose violation is undetectable is a trap. The fix (a
  leg_changed analogue) was 6 lines.

## What to improve next time

- Apply would-it-fail-without-it to COPIED tests too, not just newly
  invented ones - stale patterns propagate. When a no-churn/left-alone
  claim is tested without the mutating system in the loop, a sentinel
  mutation between runs is the standard fix.
- When declaring a runtime mutation "unsupported", check what actually
  happens if someone does it anyway: silent-forever beats loud-failure
  only when documented on the component itself, and a cheap
  make-it-work-instead is usually available in an ECS.

## Action items

- [x] Ledger: bump would-it-fail-without-it (x4, new vacuous-by-copying
      variant), out-of-context-review-pass (x6, ruled-out-a-non-issue as a
      new value mode).
