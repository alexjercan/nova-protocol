# Retro: Comms/story-beat action + HUD comms panel

- TASK: 20260716-183220
- BRANCH: feature/comms-story-panel (landed 9911382f)
- REVIEW ROUNDS: 1

## What went well

- Pattern-copying the objectives pipeline (event-world log -> write-on-
  diff sync -> change-gated HUD widget) meant every design question had
  a precedent answer, including the two traps it pre-solved: per-frame
  resource flagging and the teardown reset class.
- The missing-resource guard was designed in BEFORE any rig broke,
  because the messagereader-needs-resource-guard lesson predicted the
  failure class - a lesson preventing a bug instead of naming one.
- diagnostic-first on the dwell failure: a five-line probe test printing
  time deltas beat theorizing (the manual-time rig advances 0.25s per
  update, not the configured 0.5).

## What went wrong

- The first dwell test trusted TimeUpdateStrategy::ManualDuration at
  face value and cost two failed runs. measure-before-writing-the-number
  applies to CLOCKS in test rigs, not just docs.

## What to improve next time

- When a test asserts against elapsed virtual time, measure the rig's
  actual per-update advance first (one probe) and write the measured
  rate into a comment next to the assertion.

## Action items

- [x] Ledger: new manual-time-rig-measures-its-clock (x1).
