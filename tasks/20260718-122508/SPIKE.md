# Spike: RCS fine-adjustment base mechanics

- DATE: 20260718-122508
- STATUS: RECOMMENDED
- TAGS: spike, feature, input, flight
- SEEDS FROM: tasks/20260717-105406 (RCS fine-adjustment movement)

## Question

What are the base mechanics for an RCS (reaction control system) fine-adjust
mode - a hold-SHIFT translation mode for the last few meters of a docking
approach that nudges the ship along its local axes without letting the player
accelerate off into the void? Concretely, the spike must pin down four forks
before the feature can be planned:

1. How the "no free acceleration" constraint is enforced (the speed cap).
2. What produces the translational force (virtual impulse at COM vs physical
   thrusters).
3. How SHIFT + mouse/scroll map to the six translation directions, and what
   the control model is (held-direction vs discrete nudge).
4. Where RCS lives in the architecture so the autopilot can later reuse it
   (verb capability + a shared intent primitive).

A good answer names the recommended mechanic for each fork, is honest about
the runners-up, and leaves a planner able to write steps without re-deciding.

## Context

Grounding facts from the current code (Bevy 0.19, Avian3D 0.7,
`bevy_enhanced_input` 0.26):

- **Verb capability model.** `FlightVerb` (`sections/controller_section.rs:175`)
  enumerates the computer-provided maneuvers a controller grants: `Stop`,
  `Goto`, `Orbit`, `Lock`. `WithheldVerbs` (a `HashSet<FlightVerb>` component,
  `:197`) tracks which are withheld; a ship grants a verb only if it has a live
  controller section that does not withhold it. `SetControllerVerb` (scenario
  action) and the `DisableVerb` section modification flip these. This is
  exactly the "RCS is a verb on the controller so not all controllers can do
  RCS" hook the user asked for: add `FlightVerb::Rcs`.

- **The speed cap the user described already exists for the main burn.**
  `FlightSpeedCap(f32)` (`flight.rs:112`) is a soft cap on the manual main-drive
  burn. `manual_burn_system` (`flight.rs:1899-1911`) tapers the commanded burn
  to zero as the velocity component ALONG the burn direction approaches the cap:

  ```rust
  let burn_direction = rotation.0.mul_vec3(Vec3::NEG_Z);
  let along = velocity.dot(burn_direction);              // u/s along burn axis
  let taper_band = (**cap * SPEED_CAP_TAPER_FRACTION).max(1.0); // last 20%
  burn *= ((**cap - along) / taper_band).clamp(0.0, 1.0);
  ```

  This is precisely the user's rule ("moving 5u/s forward -> RCS forward does
  nothing, RCS backward works down to -5u/s"), generalized: RCS gates each
  commanded axis on `sign * v_axis < cap`. Only the along-burn component counts,
  so braking/turning are never blocked - the same property RCS wants.

- **Force application.** Thrusters apply `force.apply_linear_impulse_at_point`
  at the mount (`thruster_section.rs:291-329`), which produces both force and
  torque via the lever arm. `manual_burn_system` routes the analog burn through
  a torque-nulling allocator (`balance_throttles`) over the live engine set,
  because the drive can be off-center. `LinearVelocity` and `Rotation` (Avian,
  world frame) give ship-local velocity via `velocity.dot(rotation.0.mul_vec3(axis))`.

- **Autopilot authority.** `Autopilot { action, phase }` (`flight.rs:120`) is a
  presence-gated component on the ship root: present = engaged, absent = manual.
  Any flight input removes it. While present, `update_controller_target_rotation_torque`
  (`input/player.rs:311`, gated `Without<Autopilot>`) stops feeding the mouse to
  the helm, so the mouse becomes camera-only free-look and the heading holds.

- **Input flow.** Mouse -> chase-camera rig -> `PointRotationOutput` (a Quat) ->
  slewed into `ControllerSectionRotationInput` at the ship's turn rate. Actions
  are `bevy_enhanced_input` `InputAction`s bound in the flight rig
  (`input/player.rs:574+`); autopilot verbs are `bool` actions (X/G/O/Z) whose
  observers check `ship_grants_verb(...)` then insert `Autopilot`. Scroll is
  already bound (mouse wheel, `Clamp::pos()`) for lock stepping.

- **Diegetic sphere HUD.** `hud/velocity.rs` renders a sphere+cone orbiting the
  ship: `VelocityHudSource::Velocity` reads `LinearVelocity` (magnitude ->
  shader tint + cone scale), `Gravity` reads gravity pull. Palette already
  switches on autopilot presence (white/blue manual -> cyan engaged). Adding an
  RCS-active tint is the same mechanism.

## Options considered

### Fork 1 - enforcing "no free acceleration"

- **A. Per-axis capped force (RECOMMENDED).** Reuse the `manual_burn_system`
  taper, generalized to a commanded 3-axis direction: for each ship-local axis,
  apply RCS force toward the commanded sign only while `sign * v_axis < rcs_cap`,
  tapering over the last band. Velocity settles at `+/-rcs_cap` per axis and
  cannot be pushed past it. Matches the user's mental model verbatim, reuses a
  proven mechanic, is frame-rate independent, and is Newtonian (no teleport).
  Residual drift within the cap persists until counter-nudged (see Fork 3/Q2).
- **B. Direct positional steps.** Each input event translates `Transform` by a
  fixed epsilon. Trivially "non-accelerating" but not physical: fights the
  solver, ignores collisions/contacts (bad at a docking port), and reads as
  teleport. Rejected.
- **C. Velocity clamp post-integration.** Let RCS add velocity freely, then
  clamp total speed each tick. Blunt: clamps ALL motion not just the RCS axis,
  interferes with the main drive and gravity. Rejected.

### Fork 2 - what produces the force

- **A. Virtual capped impulse at COM (RECOMMENDED for the base spike).** RCS
  applies a pure linear impulse at the ship's center of mass on the ship root,
  no torque, no dependence on physical thruster geometry. The `FlightVerb::Rcs`
  capability is the fiction ("this flight computer has cold-gas RCS quads"); any
  ship granted the verb can strafe/lift regardless of whether it physically
  mounts lateral/vertical thrusters. Simplest, geometry-independent, and never
  induces unwanted rotation - ideal for fine adjust. Downside: less diegetic
  (no visible nozzle fires).
- **B. Route through physical thrusters via the balancer.** Reuse
  `balance_throttles` to allocate RCS demand over real lateral/retro engines.
  Fully diegetic and consistent with the main burn. But most ships lack
  side/vertical thrusters, so RCS would silently do nothing on them, and the
  allocator would fight lever-arm torque for a maneuver that specifically wants
  zero rotation. Higher effort, geometry-dependent. Good candidate for a later
  diegetic-polish task, not the base mechanic.

### Fork 3 - input mapping and control model

Only two mouse axes exist; the user proposed mouse XZ (ship forward/back +
strafe) and scroll for Y (up/down). Two sub-questions:

- **Axis mapping (user-specified, adopted):** mouse Y -> ship local +/-Z
  (forward/back), mouse X -> ship local +/-X (strafe), scroll -> ship local
  +/-Y (up/down). While SHIFT is held the mouse is repurposed from aiming to
  translation, so RCS should take rotation authority the same way the autopilot
  does (freeze the helm at its current heading, gate
  `update_controller_target_rotation_torque` on "not RCS-active" too).
- **Control model - the real fork:**
  - **Held-direction / virtual joystick (RECOMMENDED).** Mouse OFFSET from a
    recentered origin (or held displacement) is a sustained direction command:
    push and hold right -> ship strafes right, building to `+rcs_cap`, then the
    cap holds it. Release -> command goes to zero. Best fit for a sustained
    docking approach ("hold toward the port"). Pairs naturally with the per-axis
    cap: the cap is the terminal speed of a held push.
  - **Discrete delta nudges.** Each frame's mouse delta is an impulse pulse; to
    keep moving you keep moving the mouse. Precise for single taps but awkward to
    sustain over meters, and delta-as-impulse is frame-rate sensitive unless
    integrated. Runner-up.

### Fork 4 - architecture so the autopilot can reuse RCS

- **A. RCS as a shared low-level intent primitive (RECOMMENDED).** Introduce an
  `RcsIntent` component on the ship root (a ship-local desired-direction Vec3,
  or per-axis command) plus an `rcs_burn_system` (FixedUpdate, sibling of
  `manual_burn_system`) that reads `RcsIntent` + `LinearVelocity` + `Rotation` +
  an `RcsSpeedCap` and applies the capped impulse. The PLAYER input path writes
  `RcsIntent` from SHIFT+mouse/scroll (gated on `FlightVerb::Rcs`); LATER the
  autopilot writes the same `RcsIntent` internally for ORBIT station-keeping.
  RCS is a primitive both drivers share, not a maneuver. Clean separation, and
  the follow-up "integrate RCS into autopilot" becomes "have the autopilot write
  RcsIntent".
- **B. RCS as an `AutopilotAction::Rcs { direction }`.** Model RCS as a
  fire-and-forget autopilot maneuver. Rejected as the base: RCS is a live,
  continuously-driven manual mode (mouse every frame), not a planned maneuver
  with align/burn/hold phases; shoehorning it into the autopilot state machine
  fights the presence-gated "any input disengages" contract and muddles the
  clean primitive the autopilot itself should call.

## Recommendation

Build RCS as a **shared translational primitive**, driven for now by player
input:

1. **Capability.** Add `FlightVerb::Rcs` to the enum and its grant checks;
   thread it through `WithheldVerbs`, `SetControllerVerb`, and `DisableVerb` so
   scenarios can withhold it. Only ships whose live controller grants `Rcs` can
   fine-adjust.
2. **Primitive.** Add `RcsIntent` (ship-local desired direction) on the ship
   root and `RcsSpeedCap(f32)` (small, e.g. 1-3 u/s). Add `rcs_burn_system` in
   FixedUpdate that, per ship-local axis, applies a pure linear impulse at COM
   toward the commanded sign only while `sign * v_axis < cap`, tapering over the
   last band - the `manual_burn_system` cap math generalized to three signed
   axes. No torque, geometry-independent (Fork 1A + Fork 2A + Fork 4A).
3. **Input.** While SHIFT is held on an RCS-capable ship: take rotation
   authority (freeze the helm like the autopilot does), map mouse Y -> local Z,
   mouse X -> local X, scroll -> local Y into `RcsIntent` using the
   held-direction model (Fork 3, virtual joystick). Entering RCS disengages any
   engaged autopilot (consistent with "any flight input"). On release, clear
   `RcsIntent` and restore mouse-to-helm.
4. **HUD.** Give the velocity sphere an RCS-active state: a distinct palette
   (and optionally render the `rcs_cap` as a bounding ring so the pilot sees the
   ceiling their nudges settle at), reusing the existing autopilot-presence
   palette switch in `hud/velocity.rs`.
5. **Follow-up (not this spike).** Have the autopilot write `RcsIntent` during
   ORBIT station-keeping and terminal GOTO arrival instead of coarse main-burn
   micro-pulses. Falls out for free once the primitive exists.

This beats the runners-up because every piece reuses an existing, proven
mechanism (the burn taper, the verb model, the autopilot rotation-authority
gate, the HUD palette switch), keeps RCS decoupled from physical thruster
geometry, and lands the primitive in the one place the autopilot can later
share.

## Open questions

All four design forks were put to the user and RESOLVED (2026-07-18); every
recommendation above was confirmed:

- **Q1 (control model) - RESOLVED: held-direction virtual joystick.**
  Push-and-hold toward the cap; release = zero command.
- **Q2 (rest behaviour) - RESOLVED: persist (Newtonian).** Residual drift within
  the cap continues; the pilot counter-nudges the opposite direction to stop.
  Matches the "5u/s" example. Auto-null station-keeping is explicitly NOT part
  of the base mechanic (possible later toggle).
- **Q3 (force source) - RESOLVED: virtual capped impulse at COM.** Pure linear
  impulse, no torque, geometry-independent; the `Rcs` verb is the fiction.
  Physical-thruster routing is deferred as later diegetic polish.
- **Q4 (rotation authority) - RESOLVED: freeze heading.** RCS takes rotation
  authority like the autopilot while SHIFT is held; mouse is fully repurposed to
  translation.

Remaining genuinely-open items (for the planner / follow-up, not blocking):

- Exact `rcs_cap` value and taper band (tune during implementation; start ~1-3
  u/s). Whether the cap is a fixed `FlightSettings` constant or an authorable
  `RcsSpeedCap` component per controller.
- Whether the held-direction origin is a recentered mouse offset or accumulated
  displacement, and the input curve (deadzone, linear vs eased) - an input-feel
  detail to settle when building the input task.

## Next steps

Direction-level tasks seeded from this spike (Q1-Q4 confirmed), each needing a
/plan pass before implementation:

- tatr 20260718-122906 (p5): RCS core primitive - `FlightVerb::Rcs` +
  `RcsIntent` + `RcsSpeedCap` + `rcs_burn_system` (per-axis capped impulse at
  COM), verb threaded through `WithheldVerbs` / `SetControllerVerb` /
  `DisableVerb`.
- tatr 20260718-122912 (p4): RCS player input - SHIFT-held mode, mouse XZ +
  scroll Y -> `RcsIntent` (held-direction), rotation-authority freeze, verb
  gating, autopilot disengage.
- tatr 20260718-122923 (p3): RCS HUD indication on the velocity sphere (active
  palette + optional cap ring).
- tatr 20260718-122932 (p2): follow-up - integrate RCS into the autopilot
  (ORBIT station-keep, GOTO terminal arrival) by writing `RcsIntent`.

Parent request: tasks/20260717-105406.

## Fix record

- 20260718 tatr 20260718-122906 (core primitive): landed `FlightVerb::Rcs`,
  `RcsIntent`/`RcsSpeedCap` components, `FlightSettings::rcs_speed_cap` (2.0
  u/s) + `rcs_accel` (1.5 u/s^2), and `rcs_burn_system` - the manual-burn taper
  generalized to three signed ship-local axes, applied as one mass-scaled
  `apply_linear_impulse` at the COM (no torque, geometry-independent, verb-gated,
  not autopilot-gated). Newtonian residual (Q2). 5 tests green (4 flight, 1
  scenario). Detail: tasks/20260718-122906/NOTES.md. Remaining: player input
  (-122912), HUD (-122923), autopilot integration (-122932).
