# Spike: Diegetic autopilot - the computer flies the ship through its real actuators

- DATE: 20260709-103324
- STATUS: RECOMMENDED
- TAGS: spike, handling, autopilot, v0.4.0

## Question

User feedback on the shipped velocity-servo flight assist (52b582d, unpushed):
wrong model. Instead of an always-on servo applying an invisible RCS impulse at
the center of mass, the flight computer should *fly the ship the way a pilot
would* - figure out how to use the existing controller module (PD torque) and
thrusters to achieve a goal. Concretely: press `X` and the ship physically
turns around to face retrograde and burns its main drive to stop. Beyond that,
an engageable **autopilot mode** with actions ("stop", "fly to object") where
the computer takes rotation authority (the mouse must stop steering the hull),
flies the maneuver, and hands control back the moment the player wants it.
What should v1 build, what survives from 52b582d, and how does the autopilot
integrate with the camera/input rig?

Design calls settled with the user (2026-07-09):

1. **Salvage substrate, replace the brain.** Keep the unpushed commits as
   history; remove the velocity-hold servo, the RCS-at-COM impulse,
   `rcs_magnitude`, and the strafe keys. Reuse the spool ramp, the HUD status
   line, capability-from-live-sections, and the input/observer plumbing.
2. **v1 actions: STOP + GOTO the locked object.** `X` engages STOP
   (flip retrograde, burn to zero). `G` engages GOTO on the current aim-assist
   lock: accelerate, flip, decelerate, arrive stopped at a standoff. One
   maneuver machine, two goals. FOLLOW/velocity-match is deferred.
3. **Breakout: any flight input disengages** (thrust, brake/engage keys);
   mouse movement does NOT - while the autopilot flies, the mouse is
   camera-only free-look, so watching the maneuver never cancels it.
4. **Minimal readouts this cycle** (autopilot state line, projected
   destination marker, speed/ETA numbers); the real diegetic instrument/panel
   design is its own future task.

## Context

- **The AI already flies this way.** `input/ai.rs::ai_desired_direction` is a
  crude autopilot: point retrograde when too fast, point at the target
  otherwise, thrust only when aligned (dot > 0.95). The player autopilot is
  the same brain grown up: a maneuver state machine plus a real deceleration
  plan instead of a linear speed gain.
- **Rotation authority already has a seam.** The ship turns because
  `update_controller_target_rotation_torque` copies the camera rig's
  `PointRotationOutput` into `ControllerSectionRotationInput`, and the
  controller section's PD torques the hull. The autopilot writes the same
  input; engaging = gating the camera-copy system off, disengaging = gating
  it back on. Nothing new touches the physics.
- **The camera rig already supports mouse-without-ship.** FreeLook mode
  (hold Alt) activates a separate `PointRotation` rig for the camera while
  the ship keeps its last command - exactly the behavior autopilot needs,
  minus the "hold a key" part. `SpaceshipCameraControlMode` +
  `SpaceshipRotationInputActiveMarker` switching in `camera_controller.rs`
  is the integration point (an `Autopilot` mode variant that activates the
  free-look rig and restores `Normal` on disengage, reusing the existing
  `initial_rotation` handoff so nothing snaps).
- **Deceleration math is cheap and honest.** Main-drive authority (sum of
  live forward thruster magnitudes, from 52b582d's capability scan) plus
  `ComputedMass` gives the ship's braking acceleration `a`. The classic
  arrival rule `v_allowed(d) = sqrt(2 * a * margin * d)` says how fast the
  ship may still be going `d` units from the goal; when actual speed exceeds
  it, it is time to flip and burn. A margin (~0.85) absorbs spool lag and PD
  settling. Stopping distance is the same formula inverted - it can go
  straight onto the HUD readout.
- **The lock is the destination picker.** `SpaceshipPlayerTorpedoTargetEntity`
  already resolves "the thing I am looking at" with a forgiving cone; GOTO
  consumes the same lock, so no new selection UI is needed for v1.
- What 52b582d leaves behind after the brain swap: `flight.rs` as the module
  home, the spool ramp (both manual and autopilot burns stay eased), the
  status HUD file, the physics-test harness reuse, and analog forward burn on
  W/Space. `FlightCommand`, `FlightRcsImpulse`, `apply_flight_rcs`,
  `ControllerSectionRcsMagnitude` (component, config field, torpedo/sections
  entries) and the ADQE strafe bindings are removed - with one thruster and
  no RCS quads, lateral intent was fiction anyway.

## Options considered

- **A. Maneuver-state autopilot through the real actuators (recommended).**
  An `Autopilot` component on the ship root holding the engaged action
  (`Stop`, `Goto { target }`) and phase (`Align`, `Burn`, `Done`). Each
  physics tick it computes a desired facing and burn from the goal, the
  arrival rule, and live capability; writes `ControllerSectionRotationInput`
  (facing) and the spooled `ThrusterSectionInput` (burn, gated on alignment
  like the AI). Manual rotation copy and manual burn are gated off while
  engaged; any flight input disengages. Pros: fully diegetic (the hull
  actually swings, the plume is the brake light), no invisible forces, one
  brain reusable by the AI later, degrades honestly (controller dead = no
  autopilot; engines dead = it aligns but cannot burn). Cons: maneuvers take
  real time and can be beaten by moving targets - accepted, that is the
  fantasy.
- **B. Keep the velocity servo as a hidden trim layer under the autopilot.**
  Rejected with the user: invisible COM forces are exactly what felt wrong,
  and the complexity double-pays (servo + maneuver logic).
- **C. Full trajectory planner (waypoints, collision avoidance, moving-target
  intercept).** The "go there safely" end state, but v1 straight-line
  maneuvers with an arrival curve deliver the feel; avoidance is a separate
  hard problem. Deferred - v1 GOTO replans toward the target's current
  position every tick, so slow drift is handled and fast targets simply take
  longer; it does not dodge asteroids on the way (noted on the HUD task list
  as a known limitation).

### Resolved sub-decisions (within A)

1. **Phases, not scripts.** `Align` (PD swings the nose to the goal facing;
   no burn until aligned within ~0.95 dot, like the AI), `Burn` (spooled
   main drive; for GOTO the goal facing flips retrograde once
   `v > v_allowed(d)`), `Done` (speed under threshold / inside standoff ->
   disengage to manual with the command handed back cleanly). STOP is GOTO
   with only the braking half.
2. **Standoff before blast radius.** Arrive stopped at ~50u from the target
   (outside the 30u torpedo blast radius) - a `FlightSettings` tunable.
3. **Disengage = re-point, not snap.** On breakout the normal camera rig's
   `PointRotation` is re-seeded from the current free-look rotation (the
   existing mode-switch machinery), so the ship does not lurch toward a
   stale command.
4. **Manual mode is what shipped before this saga** plus the spool: mouse
   points the hull, W/Space burns (now analog + eased). X and G are autopilot
   engagements, Z (or engaging key again) disengages.
5. **HUD v1:** the status line grows autopilot states (`AP STOP - ALIGN`,
   `AP GOTO - BURN 320m`), and the destination gets a projected marker
   (reuse the torpedo-target HUD projection pattern). Anything richer waits
   for the diegetic-instruments task.

## Recommendation

Build **A** as a rework task on the existing `flight.rs` (remove the servo
pieces, add the maneuver machine), with the same testing shape that worked
last cycle: pure helpers for the arrival rule / facing / phase transitions,
physics-level integration tests through the real PD + thruster systems
("engage STOP while coasting -> ship flips and speed reaches ~0 without any
external force", "GOTO arrives within standoff and disengages", "any flight
input mid-maneuver disengages and restores manual authority"). Keep the AI
untouched this cycle but put the shared math in pure helpers it can adopt.

## Open questions

- Collision avoidance during GOTO (v1 flies through whatever is in the way) -
  future planner work, revisit after the vertical-slice scenario exists.
- FOLLOW / velocity-match action - deferred by user choice; the maneuver
  machine's goal enum leaves room for it.
- The diegetic instrument design (in-world panel showing the maneuver plan,
  flip point, ETA) - seeded as its own direction-level task; needs the HUD
  projection substrate from the weapons-HUD spike.

## Next steps

- tatr 20260709-103434: rework flight assist into the diegetic autopilot
  (brain swap + STOP/GOTO + input authority handover + minimal readouts).
- tatr 20260709-095043 (re-scoped): manual-feel polish (rotation slew, camera
  smoothing/burn push) + the playtest retune, now covering autopilot
  constants too.
- tatr 20260709-103454 (direction-level, parked v0.5.0): diegetic flight
  instruments - design the in-world autopilot/maneuver UI on the HUD
  projection substrate.
