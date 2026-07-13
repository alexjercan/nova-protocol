# Retro: Show-don't-tell lock HUD

- TASK: 20260713-110311
- BRANCH: feat/show-dont-tell-hud (landed 95eb7bb)
- REVIEW ROUNDS: 1 (APPROVE; reader-drain bug + stale module doc folded in)

## What went well

- The state-coverage TRACE as the review spine (every player-visible
  state x what renders) caught nothing missing - because the adversarial
  round had already enumerated the holes (F4 raised-manual, F5 beacon
  flicker, F7 silent deny) before a line was written. Finding holes at
  spike time is an order of magnitude cheaper than at review time.
- The placeholder-sound generator made "needs a sound asset" a
  four-line-dict change: LockOn/LockOff/SafetyOn/RadarDeny landed in the
  same task as their triggers, and the 082337-deferred safety blip
  finally shipped.
- A live user note ("panel overlaps the status bar") folded in mid-task
  for the cost of one constant, because the panel geometry was already
  under edit - batching feel fixes with the surface that owns them beats
  filing a task.

## What went wrong

- `run_system_once` + MessageReader bit AGAIN (deny-flash test) despite
  being a ledger lesson - it was recognized in seconds, but writing the
  test with `register_system` FROM THE START is the actual lesson, not
  fixing it fast.
- The `play_lock_cues` collapse comment lied about the code (`.next()`
  does not drain); caught only at review's cold read. A comment that
  states a guarantee is a test obligation: either pin it or do not claim
  it.
- One plan step contradicted the questionnaire answer it cited (Q6a
  distance scope) because the step was drafted before re-reading the
  answer text. The questionnaire is the contract; steps should quote it,
  not paraphrase it.

## What to improve next time

- When a task consumes questionnaire answers, paste the answer text
  verbatim into the step it governs - paraphrase drift caused the one
  deviation this task had to explain.
- Message-reading test systems: reach for `register_system` by default,
  not after the first false failure.

## Action items

- [x] LESSONS.md: bump registered-system-for-change-detection with the
  MessageReader variant (this retro).
