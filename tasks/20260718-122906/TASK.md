# RCS core primitive: Rcs verb + RcsIntent + capped impulse burn system

- STATUS: OPEN
- PRIORITY: 5
- TAGS: v0.7.0,feature,flight,spike

## Goal

The base RCS mechanic as a shared translational primitive, independent of any
player input. Deliver:

- `FlightVerb::Rcs` added to the verb enum, threaded through the grant checks,
  `WithheldVerbs`, the `SetControllerVerb` scenario action, and the `DisableVerb`
  section modification, so scenarios can grant/withhold RCS per controller. Only
  a ship whose live controller grants `Rcs` can fine-adjust.
- `RcsIntent` component on the ship root: a ship-local desired-direction command
  (per-axis or Vec3), written by whoever drives RCS (player input now, autopilot
  later). Absent/zero = no RCS.
- `RcsSpeedCap` for the small fine-adjust ceiling (start ~1-3 u/s; decide fixed
  `FlightSettings` constant vs authorable component during planning).
- `rcs_burn_system` in FixedUpdate (sibling of `manual_burn_system`): for each
  ship-local axis, apply a pure linear impulse at the center of mass toward the
  commanded sign ONLY while `sign * v_axis < cap`, tapering over the last band -
  the `manual_burn_system` speed-cap math (flight.rs:1899-1911) generalized to
  three signed axes. No torque, no dependence on physical thruster geometry.
  Residual velocity within the cap persists (Newtonian, no auto-null).

## Notes

Spike: tasks/20260718-122508/SPIKE.md (RECOMMENDED; forks Q1-Q4 resolved).
Reference points: `FlightVerb`/`WithheldVerbs` in
sections/controller_section.rs:175; `FlightSpeedCap` + taper in flight.rs:112,1899;
thruster impulse in sections/thruster_section.rs:291. Needs a /plan pass to
break into steps.
