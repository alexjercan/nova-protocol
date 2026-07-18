# Gauntlet time-trial: visible run timer + clean-run bonus

- STATUS: OPEN
- PRIORITY: 42
- TAGS: scenario,content,modding,hud,v0.8.0

## Goal

Turn Gauntlet Run into a real time-trial: a visible run timer counting from
START to FINISH, and a "clean run" bonus for finishing without crashing.
Gauntlet 2.0 (20260716-124722) ships the course, hazards and outcome frames but
NOT a clock, because the scenario vocabulary has no timer readout - OnUpdate +
a variable can accumulate elapsed time, but nothing puts it on the HUD. This
task adds that missing modding-surface piece and wires the gauntlet to it.

## Notes

- Spike: tasks/20260716-174631/SPIKE.md (open question "visible timer /
  time-trial").
- Blocked-ish on a small engine/modding-surface addition: a scenario-driven
  numeric HUD readout bound to a variable (the timer), or a dedicated timer
  widget. Decide the surface (generic "show variable X on HUD" action vs. a
  purpose-built timer) when picked up - the generic one is more reusable.
- Clean-run bonus: a crash counter variable (increment on player-ship
  hazard-zone OnEnter or on a damage signal) gates a Victory message variant.
- Depends on: 20260716-124722 (Gauntlet 2.0 must land first; this decorates it).
