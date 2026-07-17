# RCS fine-adjustment movement: shift + mouse for small non-accelerating translations (docking)

- STATUS: OPEN
- PRIORITY: 5
- TAGS: v0.7.0,feature,input,flight

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
