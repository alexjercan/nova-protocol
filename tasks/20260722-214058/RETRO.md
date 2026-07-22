# Retro: Ledger beat-sheet pacing pass ch1/ch2/ch2b (20260722-214058)

## What went well

- Authoring against the diagnostic brief (20260722-214053 NOTES) paid off: the
  per-chapter target rhythm was already written, so the work was "apply the
  idiom" not "rediscover the gaps". The Shakedown opening cascade
  (`open_step` + `scenario_elapsed` gates, final step lazy-posts objective 1)
  transplanted cleanly into hand-authored RON.
- The additive-only discipline held: ch2/ch2b spawn geometry, counts, loadouts
  and `engage_delay: 8.0` telegraphs are byte-identical old-vs-new (the reviewer
  grepped every geometry token to confirm). The pacing layer is pure
  variables + OnUpdate handlers + StoryMessage/Objective timing, so the fairness
  rig's contract is untouched.
- The deferred-Victory change was handled correctly: a StoryMessage cannot sit
  beside an Outcome (lint arm), so the win comms line fires first and the
  Victory + lingering NextScenario defer a beat behind `win_gate`. The test
  gained a `pump_clock` helper to advance `scenario_elapsed` past the gate -
  exactly the clock-pump lesson from the Shakedown pacing pass (20260721-211506):
  a time-gated change needs a clock pump or event-driven walk tests silently
  stall.

## What went wrong / was tricky

- Review flagged (LOW) that `deaths_after_the_win_declare_nothing` might stay
  inert for the wrong reason (undefined-var-fails-closed vs the real guard) -
  the `review-rig-can-false-green` risk. Investigated it against the actual
  handlers rather than taking the note at face value: the win handler sets
  `act = 2` SYNCHRONOUSLY with the `kills > 1` detection, and the Defeat handlers
  gate `act < 2`. So the Defeat window closes the instant the win is detected,
  BEFORE the deferred Victory resolves - there is no breather-window race
  (`defer-opens-a-consumer-race` does NOT bite here because `act` is the latch,
  not the deferred Outcome). The test seeds `act = 2`, the true post-win state,
  and proves the act-gate - the real mechanism. No fix warranted; the note was a
  mild mischaracterization (inertness is act-gating, not `win_said`).

## Lessons / what to do differently

- When deferring an Outcome behind a clock gate, keep the ACT LATCH synchronous
  with the trigger detection - bump `act` in the same handler that detects the
  win, and let only the player-facing Outcome/overlay defer. That closes the
  Defeat/consumer window immediately and sidesteps `defer-opens-a-consumer-race`.
  (This is why the ch2 deferring is safe; ch4's ending rework must do the same -
  advance `act` before any death window.)
- A review note is a hypothesis: trace a flagged rig-faithfulness concern
  against the real handlers before "fixing" the rig - applying the suggested
  seed here would have added a variable the tested path never reads, implying a
  guard that isn't the one doing the work.

## Follow-ups

- None blocking. Owner playtest question 3 (opener length: ch1 ~29s vs ch2/ch2b
  ~11s) stays batched for Finish. The version bump stays with the close-out task
  (20260722-214119), per the shared-checkout single-owner convention.
- Carry the "synchronous act latch, deferred overlay" rule into the ch4 ending
  rework (20260722-214110).
