# Retro: combat lock lets go of locked enemies

- TASK: 20260730-123009
- BRANCH: fix/combat-lock-decay
- ROUNDS: 2 (REQUEST_CHANGES -> APPROVE)

## What went well

- **The understanding phase found the real mechanism before any code was
  cut.** The task listed five candidate mechanisms; a workspace-wide grep for
  writers of `CombatDecay` found a sixth that none of them covered, and it was
  the one the owner had actually hit. Fifteen minutes of reading beat five
  planned experiments.
- **Instrumenting the branch instead of inferring it.** Making each drop name
  itself (`CombatLockDropped` + a `debug!`) meant the fail-first red read
  `reason: IdleDecay, idle_secs: 30.0` instead of "the lock is gone and the
  decay clock looks suspicious". The deviation from the plan's "test-only
  observer" wording was worth it and was recorded rather than quietly taken.
- **The fail-first discipline paid twice.** Once for the fix (disable it,
  watch the rig go red at step 29 with real numbers, re-enable) and once for
  the review response (restore the old cue formula, watch all three new tests
  fail, restore the fix). Both reds are quoted in NOTES.md and REVIEW.md.
- **Out-of-context review earned its keep.** R1.1 was a genuine defect in
  shipped-quality code that in-session review would very likely have waved
  through - the formula looked right, the tests were green, and the prose
  described what I intended rather than what the code did.

## What went wrong

- **I shipped an animation whose rate was wrong by construction (R1.1).**
  Root cause: I wrote `phase = elapsed_secs * hz` while `hz` itself varied.
  That is only a valid phase for a CONSTANT frequency; for a swept one the
  phase is the integral, so the effective rate became `hz + elapsed * dhz/dt`
  and grew with session uptime - 29 pulses over the window at t=0, 10 at
  t=60 s, 118 (aliasing) at t=300 s. I reached for the familiar
  `sin(t * rate)` shape without noticing the rate was no longer constant.
- **My tests were shaped to the code rather than to the player (R1.2).** The
  pure-function test sampled `elapsed_secs` in `[0, 1)` and the live-node test
  ran at `Time::default()` (elapsed 0, so the pulse term was identically
  zero). Both were sampling the exact regime where the bug is invisible. I
  tested "is there a pulse" rather than "what pulse does a player see 5
  minutes into a session".
- **I wrote the documentation for the cue I intended.** CHANGELOG, wiki and
  NOTES all said "pulses faster and faster" while the code produced a smooth
  slide or a flicker depending on uptime. The prose was written from the
  design, never re-checked against a measurement.
- **I overclaimed in a doc comment (R1.3).** "Answerable from a log" was true
  of the test rig, not of a shipped build, because nothing logged the message.
  The fix was to make the claim true, but the claim went in first.
- **The probe evidence was taken mid-cycle** and so covered a superseded
  commit; the reviewer caught it. Re-run on the final tree.

## What to improve next time

- When an animation's rate VARIES, derive the phase by integrating the rate,
  and sanity-check the closed form (`cycles(window)` against the mean-rate
  product) before writing any test.
- Test time-driven visuals across the SESSION-time range a player occupies,
  not just from t=0. "Same output at 0 s and 300 s uptime, at 60 and 600 fps"
  is a cheap assertion that would have caught this at the first attempt.
- Before believing a doc comment that says behaviour X ships, grep for the
  code that would have to write it. A comment citing a task id is a promise
  the task may not have kept - that is exactly how this bug survived.
- Run the DoD's probe proof AFTER the last code commit, not while review is
  still in flight.

## Lessons for the ledger

- `comment-citing-a-task-is-not-the-wiring` - three comments promised firing
  reset the decay "once 20260713-082337 lands"; it closed without the wiring
  and the gap read as a gameplay bug for weeks.
- `swept-rate-needs-an-integrated-phase` - `t * hz(t)` is not a chirp.
- `sample-the-regime-the-player-lives-in` - an animation test that only
  samples from t=0 tests the one regime the bug hides in.
