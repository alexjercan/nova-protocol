# Retro: Shakedown beat sheet v2

- TASK: 20260713-140929
- BRANCH: feat/beat-sheet-v2 (landed a112263)
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- The verify-first geometry step did its job LOUDLY: the spike's mental
  map ("beacon 4 outside the SOI") was wrong against the code (beacon 3
  was already deep INSIDE it), and the plan-time check forced the honest
  reshuffle (beacon 3 out, beacon 4 in) plus surfaced the waypoint-leg
  range gap (800u leg vs 600u default beacon lock range) BEFORE any
  handler was written. Two of the design's three hard problems were found
  by arithmetic, not playtest.
- The break-away [Z] beat cost zero vocabulary: the coast ring's OnExit
  was already there once the ring existed - a reminder to inventory the
  event grammar for free beats before adding content.
- Spawning gated content WITH its beat (ring, beacon 4, derelict) made
  the already-inside/consumed-one-shot traps structurally impossible
  instead of carefully avoided; the two remaining lock-ordering hazards
  are covered by the bridge echo and pinned in the walk.

## What went wrong

- The spike's beat sheet said "beats 1-11" but the geometry reshuffle
  split the radar lesson differently and the final count is 12; the spike
  numbering and the shipped numbering drifted mid-implementation, caught
  while renumbering handlers. Beat sheets in spikes should number by NAME
  (burn/look/lock...) and let the plan assign integers.

## What to improve next time

- When a spike hand-sketches world positions, run the existing geometry
  pins against the sketch AT SPIKE TIME (the test was sitting right
  there); the reshuffle was cheap at plan time but would have been free
  at spike time.

## Action items

- [x] Playtest notes recorded in REVIEW.md (coast drift duration, the
  manual return leg) for the user's next session.
