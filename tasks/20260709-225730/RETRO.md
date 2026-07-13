# Retro: AI patrol and idle flight states

- TASK: 20260709-225730
- BRANCH: feature/ai-patrol-idle (local branch by user request, merged)
- REVIEW ROUNDS: 1 (APPROVE, one MINOR fixed in-cycle, two NITs documented)

A clean one-round cycle. The task's core bet - reuse the GOTO autopilot
instead of writing a parallel AI steering path - paid off everywhere.

## What went well

- **Reuse-first paid off.** Adding `AutopilotAction::GotoPos` (one enum
  variant plus a goal-resolution refactor in `autopilot_system`) bought
  patrol legs, arrival curves, braking, engine spooling and the disengage
  lifecycle for free. Exhaustive matches surfaced every consumer to update
  (HUD status line, destination anchor), and the screen indicator already
  had a `Point` anchor variant, so even the HUD support was honest rather
  than stubbed.
- **Actuator ownership designed up front.** The contention between the
  autopilot's spooled thruster writes and the AI brain's per-frame zeroing
  was identified before coding (not debugged after): the thrust system
  yields while an autopilot is engaged, engaging states drop the autopilot,
  and Bevy's auto sync points make the handoff same-frame. No contention
  bug ever surfaced.
- **Checkpoint habit held.** Work was committed before the long test run
  (the lesson from the standoff retro, applied deliberately).
- **Edge cases got dedicated design attention**: single-waypoint routes
  (station keeping without engage/complete churn), empty routes, and the
  reviewer round added self-healing for inspector-shrunk waypoint lists.

## What went wrong

- **One test failed on first run, and the root cause is worth keeping:**
  the far-hostile test spawned the hostile before the ship's first
  transition tick. Ships spawn in the default `Engage` state, and combat
  states deliberately hold on ANY acquired target - only passive states
  gate on detection range - so the ship never reached Patrol. The test was
  written from the pure transition table's point of view and forgot the
  stateful pipeline's starting state.

## What to improve next time

- When testing a state machine with sticky states, establish the starting
  state explicitly (run the pipeline to settle first), instead of assuming
  the state the scenario narrative implies.

## Action items

- [x] tatr 20260710 (below): consolidated CHANGELOG entry for the v0.4.0
  AI combat-behavior wave - none of the wave's tasks (225726..225733) have
  logged to CHANGELOG.md yet, and the repo convention is that shipped
  features get an entry.
- Playtest knobs noted in code, not tasks: AI_ENGAGE_RANGE (800 m),
  AI_WAYPOINT_SLACK (25 m), AI_IDLE_DRIFT_SPEED (1 u/s).
