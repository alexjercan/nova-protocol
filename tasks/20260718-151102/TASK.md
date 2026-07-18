# RCS error-relative mode for autopilot ORBIT station-keep (needs primitive redesign)

- STATUS: OPEN
- PRIORITY: 1
- TAGS: v0.7.0,feature,flight,spike

## Goal

Let the autopilot use RCS for ORBIT station-keeping (and any correction while the
ship moves faster than the fine-adjust cap). Split out of the autopilot-RCS task
(20260718-122932), which delivered only the GOTO/STOP terminal settle because:

- `rcs_burn_system` caps ABSOLUTE along-axis speed at `rcs_speed_cap` (2 u/s):
  the gate is `along = velocity.dot(world_axis)`, absolute velocity.
- ORBIT moves at `circular_orbit_speed = sqrt(mu/r)` ~= 2.5-6 u/s (>2 u/s cap),
  so a prograde RCS push gates to zero and a retrograde one BRAKES the orbit.
  RCS as-built cannot express "add a small correction while already moving at
  orbital speed".

The fix needs a PRIMITIVE change, not just an autopilot hookup, so it is its own
task (and probably its own spike):

- A target-relative / error-relative RCS mode: cap the CORRECTION relative to a
  supplied desired velocity (`|v - desired| < cap`), not the absolute speed - so
  the same primitive can trim a fast-moving orbit by a sub-cap delta.
- Must NOT regress the player-facing absolute-cap mode (the SHIFT+mouse
  fine-adjust feel, task 20260718-122912). Likely a second mode/flag on
  `rcs_burn_system` or a sibling system, chosen deliberately.

## Notes

Spike: tasks/20260718-122508/SPIKE.md (Fork 4). Parent: 20260718-122932 (GOTO/STOP
terminal RCS landed). The incompatibility is documented there and in
tasks/20260718-122932/NOTES.md. Start with a /spike on the error-relative mode
(it touches the landed player primitive). Needs a /plan pass after the spike.