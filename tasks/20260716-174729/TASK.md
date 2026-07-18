# Gauntlet time-trial: visible run timer + clean-run bonus

- STATUS: OPEN
- PRIORITY: 42
- TAGS: scenario,content,modding,hud,v0.8.0

## Story

As a player flying Gauntlet Run, I want a visible clock from START to FINISH
and recognition for a clean run, so that the course has a score to chase and a
reason to re-fly it.

Gauntlet 2.0 (20260716-124722, landed - the shipped bundle is at 1.2.0) ships
the course, hazards and outcome frames but NOT a clock, because the scenario
vocabulary has no timer readout. Since v0.7.0 the engine maintains
`scenario_elapsed` (live, pause-frozen, retry-reset scenario seconds readable
from expression filters), so the timekeeping half exists; what is missing is
the display half - nothing puts a scenario variable on the HUD. This task adds
that missing modding-surface piece and wires the gauntlet to it.

## Steps

- [ ] Decide the HUD surface: a generic "show variable X on the HUD" scenario
      action (more reusable, benefits every mod) vs a purpose-built timer
      widget. Record the decision and why; the spike
      (tasks/20260716-174631/SPIKE.md) leans generic.
- [ ] Implement the readout as scenario vocabulary (action to show/hide, bound
      to a variable such as `scenario_elapsed`, formatted mm:ss.s for time),
      respecting HUD visibility tiers and the pause/outcome freeze.
- [ ] Wire Gauntlet: timer visible from the START gate, stopped and shown in
      the Victory banner text at FINISH.
- [ ] Clean-run bonus: a crash counter variable (increment on player-ship
      hazard-zone OnEnter or on a damage signal) gates a Victory message
      variant ("CLEAN RUN" + time vs time only).
- [ ] Extend `tests/gauntlet_course.rs` to pin the timer wiring (visible after
      START, stops at FINISH, clean-run variant gating).
- [ ] Bump the gauntlet bundle version (minor - content rework) and re-publish;
      update the test's version assertion deliberately.
- [ ] Docs in the same task: the new action goes into the scenario action
      reference (coordinate with 20260718-231555, which documents Gauntlet's
      whole vocabulary), CHANGELOG entry, gauntlet README.

## Definition of Done

- A scenario-authorable HUD readout exists, documented, usable by any mod.
- Gauntlet shows a running clock, reports the final time on Victory, and
  distinguishes a clean run; retry resets the clock via `scenario_elapsed`
  semantics.
- Tests pin the behavior; the re-published bundle installs and updates cleanly
  from the portal.

## Notes

- Spike: tasks/20260716-174631/SPIKE.md (open question "visible timer /
  time-trial").
- This is the one v0.8.0 content task that needs a small engine/modding-surface
  addition (the readout) - accepted in the v0.8.0 plan as a modding-surface
  piece, not a gameplay feature. Keep the addition minimal.
- Dependency status: 20260716-124722 (Gauntlet 2.0) HAS landed; this decorates
  the shipped course.
