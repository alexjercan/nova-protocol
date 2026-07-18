# Integrate RCS into autopilot (ORBIT station-keep, GOTO terminal arrival write RcsIntent)

- STATUS: OPEN
- PRIORITY: 2
- TAGS: v0.7.0,feature,flight,spike

## Goal

Follow-up (the user flagged it as such): once the RCS primitive exists, have the
autopilot drive it instead of coarse main-burn micro-pulses for the maneuvers
that need sub-cap precision:

- ORBIT station-keeping: hold the ring with `RcsIntent` micro-nudges rather than
  main-drive pulses.
- GOTO terminal arrival: the last-few-meters settle at `arrival_standoff` via
  RCS.
- The autopilot writes the same `RcsIntent` the player input writes, so no new
  force path is needed - this is the payoff of building RCS as a shared
  primitive (spike Fork 4A). Requires the RCS verb to be granted for the ship.

## Notes

Spike: tasks/20260718-122508/SPIKE.md (Fork 4: RCS as shared primitive).
Depends on the RCS core primitive (task 20260718-122906). Autopilot state
machine in flight.rs:120 (Autopilot/AutopilotAction/AutopilotPhase). Lower
priority than the player-facing base mechanic. Needs a /plan pass.
