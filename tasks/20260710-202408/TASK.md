# GOTO standoff must be surface-relative: account for target size

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.5.0, autopilot, bug, ux

## Goal

Playtest finding (user, 2026-07-10): `FlightSettings::arrival_standoff`
(50u) measures from the target's CENTER, so a GOTO at a big object stops
too close to - or inside - its surface. The arrival rule must budget the
target's size: stop `arrival_standoff` from the SURFACE, i.e. effective
standoff = `arrival_standoff + target_radius`.

Direction: the arrival leg (arrival_desired in autopilot_system,
goto_desired_velocity) needs a target radius. Candidate sources, to weigh
at plan time: `GravityWell::body_radius` (well bodies - the case that
hurts today), `LockSignature` (already radius-authored on asteroids, and
the lock is the GOTO designator, but it is a scanner magnitude, not
strictly geometry), or a dedicated `BodyRadius`-style component the
scenario authors once and both systems read. Ships as GOTO targets
already work at 50u (their extent is small); zero-radius default keeps
current behavior for everything unsized.

## Notes

- Filed 2026-07-10, queued after 20260710-201514 (gravity indicator).
- Touches the same code region as 20260710-193500 (gravity-aware arrival)
  and 20260710-195954 (GOTO parks into ORBIT): the ORBIT handoff wants
  the arrival radius inside the stable band, and the band math already
  uses body_radius - whichever of the three lands last should re-read the
  other two. Consider whether one small "arrival geometry" pass should
  implement 202408 + 195954 together.
- ManeuverTelemetry (distance/ETA/flip) and the destination readout
  should report the surface-relative numbers too, or the chip reads "50m"
  while hovering over a mountain.
