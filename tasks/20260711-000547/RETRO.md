# Retro: Remove the redundant ORBIT ring chip

- TASK: 20260711-000547
- BRANCH: fix/remove-orbit-ring-chip (squashed to master as d1abe38)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

## What went well

- Smooth micro-cycle: playtest feedback to merged removal in one pass,
  because the sweep-then-delete order (grep for consumers BEFORE deleting
  a symbol) found that orbit_ring_point was chip-only and
  circular_orbit_speed was not - the diff was exactly as big as it needed
  to be.
- The user's feedback arrived mid-flow and was queued as its own task
  instead of widening the tint task's branch; the flow discipline held.

## What went wrong

- Nothing in execution. Process observation: the spike explicitly kept
  the ring chip "as-is", and one playtest overturned that within hours of
  the spoke landing. The redundancy (two `r` readouts in the same screen
  area) was foreseeable at spike time; the questionnaire asked spoke OR
  ring-chip-extension but never asked what happens to the ring chip when
  the spoke wins.

## What to improve next time

- In replacement-flavored spikes, when an option adds an element that
  overlaps an existing one, add the explicit follow-up question "does the
  old element survive?" instead of defaulting to keep. Cheap to ask in
  the same questionnaire.

## Action items

- [x] None beyond the lesson above; no follow-up code work.
