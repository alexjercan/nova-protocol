# Retro: Redundant closing-speed caption removal

- TASK: 20260711-125226
- BRANCH: fix/redundant-speed-caption (squash-merged as 506e895)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

## What went well

- Clean micro-cycle mirroring the orbit-ring-chip removal: sweep first
  (the telemetry field has planner consumers - kept), then the minimal
  caption change, one test string, done in minutes.

## What went wrong

- Nothing. Same process observation as the ring-chip retro applies: two
  readouts of one number in one screen area keep getting caught by
  playtests rather than at design time - the diegetic-HUD spike
  questionnaire now has that follow-up question; this instance predates
  it (the caption came from the maneuver instruments cycle).

## What to improve next time

- Nothing new; the existing "does the old element survive" spike question
  covers the class.

## Action items

- [x] None.
