# RCS fine-adjustment movement: shift + mouse for small non-accelerating translations (docking)

- STATUS: CLOSED
- PRIORITY: 5
- TAGS: v0.7.0, feature, input, flight

Add an RCS (reaction control system) fine-adjustment mode to the ship
controller for precise maneuvering, e.g. when docking.

Behavior:

- Holding shift + moving the mouse translates the ship in small adjustments
  along the ship-local axes: up/down, left/right, and forward/backward.
- RCS input does NOT accelerate the ship: it must not add to the ship's
  velocity in a way that lets you gain extra speed by spamming it. It is
  purely for fine positional nudges, not a propulsion mechanic.
- Intended use case is the last few meters of a docking approach, where the
  main thrusters are too coarse.

Open questions for planning:

- How mouse axes map to the six translation directions (e.g. mouse X/Y for
  lateral/vertical, scroll or extra key for forward/backward), given only two
  mouse axes are available.
- Whether RCS applies a capped low-speed velocity that decays, or direct small
  positional steps; either way the no-extra-speed constraint must hold.
- Interaction with the existing flight model and shift's current binding (if
  any), plus HUD indication that RCS mode is active.

Will also need a /spike to improve the planning.
Needs a /plan pass to break into steps before implementation.

## Delivered (2026-07-18)

Spiked (tasks/20260718-122508/SPIKE.md) and delivered as a family:

- Core primitive (20260718-122906): `FlightVerb::Rcs`, `RcsIntent`/`RcsSpeedCap`,
  `rcs_burn_system` - a per-axis speed-capped, torque-free COM impulse; SHIFT-held
  held-direction control, Newtonian residual, no runaway (the "non-accelerating"
  constraint = a small absolute cap).
- Player input (20260718-122912): SHIFT + mouse (XZ) + scroll (Y) into `RcsIntent`,
  helm + camera frozen while held.
- HUD (20260718-122923): a violet "RCS active" palette on the velocity sphere.
- Autopilot (20260718-122932): the STOP/GOTO terminal settle uses RCS when the
  ship grants it.
- Keybind + mainline-disable (20260718-175502): a `[SHIFT] RCS` hint shown only
  when granted; RCS is a normal verb, DISABLED in the mainline campaign pending
  rework (per-scenario `DisableVerb(Rcs)`).

Deliberately deferred (seeded): cap ring on the sphere (20260718-144939) and an
error-relative RCS mode for ORBIT station-keep + tightening the terminal creep
(20260718-151102). RCS is off in the mainline until that rework.
