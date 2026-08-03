# Spike: decide the v0.10.0 example fleet roster

- PRIORITY: 82
- TAGS: v0.10.0, spike, examples, testing
- KIND: SPIKE
- ACTIVITY: PLANNING
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-115955

## Question

The example fleet is about to be rebuilt on the predicate autopilot
(`20260802-120025`). Before rewriting 22 examples, decide WHAT the fleet
should contain: which runs to keep, which to retire, and which new
purpose-built test scenarios to add.

The open tension: the mainline story scenarios (`broadside`, `lifeline`,
`final_tally`) are narrative one-shots. An autopilot pressing keys is unlikely
to WIN them, and driving them to an outcome would mean tuning the scenarios
for the robot. The owner's read: mainline runs only need to prove "the game
plays normally" and to collect perf data; deep coverage - reaching a real win
or lose state, exercising many objects and transitions - belongs to
purpose-built test scenarios that carry no story.

## Constraints

- Categories and their contracts are fixed by `20260802-120029`: `sections/`
  correctness, `gameplay/` correctness + profiling, `ui/` UI correctness,
  `screenshots/` capture-only, `perf/` frame-time baselines.
- New content is allowed. A test scenario does not need story, comms, or
  balance - it needs objects, transitions, and an observable outcome.
- Prefer deepening an existing run over adding a thin new one; the fleet gets
  deeper, not wider.
- Examples must be RUN under Xvfb, not just checked.

## What to produce

- A proposed fleet roster per category: keep / retire / rewrite / add, one
  line of why for each of the current 22 examples.
- For each proposed NEW test scenario: what it proves, roughly what it
  contains (objects, objectives, transitions), and whether it reaches a
  win/lose outcome.
- The profiling story: which runs fill a frame-time window, and how (loop
  point, scene size).
- The resulting task list, sized so `20260802-120029` can be executed per
  category.

## Notes

- The mainline-scenario bar is NOT settled by fiat: if the spike finds a
  cheap way to drive one story scenario to a real outcome, say so.
- Retiring a run that is the only evidence for a system is a regression;
  name what covers it afterwards.
