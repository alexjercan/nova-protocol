# Spike: Multi-thruster autopilot - every engine is an actuator, not just the nose

- DATE: 20260709-121746
- STATUS: RECOMMENDED
- TAGS: spike, handling, autopilot, v0.4.0

## Question

Playtest report: a ship with multiple thrusters (retro, laterals - built in
the editor, each with its own keybind for manual convenience) flies its
autopilot maneuvers as if only the nose-mounted main drive existed. The
autopilot's model is literally "the main drive points out of the rigid body's
forward": `main_authority` sums only thrusters within ~25 degrees of the
root's `-Z`, and every maneuver plans a nose swing. How should the flight
computer model per-engine directions and choose the *fastest path* - rotate
to bring the big drive to bear, or fire the engine already pointing the right
way? (Keybinds are irrelevant here: in autopilot mode every engine is the
computer's, as settled last cycle.)

Design calls settled with the user (2026-07-09):

1. **All aligned engines fire.** The planner picks a best group to decide how
   far to rotate, but at burn time every live thruster whose thrust direction
   sits inside the alignment cone of the needed burn fires - free authority,
   and all the right bells light up.
2. **Torque is ignored in v1.** Off-center engines fire regardless of lever
   arm; the controller section's PD counter-steers within its `max_torque`.
   Honest physics - a badly balanced ship flies badly, which is diegetic and
   rewards ship design. Torque-aware allocation (this is where section
   *positions* around the center of mass genuinely matter) is the recorded
   follow-up.
3. **Time-optimal with a rotation-bias knob.** Group score =
   `rotation_time * rotation_bias + burn_time`, `rotation_bias` a reflected
   `FlightSettings` knob (default 1.5, mildly rotation-averse): small trims
   use the engines you built for them, big burns still flip to the main
   drive, and the retune can shift the personality.

## Context

- Each thruster already knows its own direction - no section graph needed for
  this. A section's bevy `Transform.rotation` is its (static) attitude
  relative to the ship root, so `local_dir = transform.rotation * -Z` and
  `world_dir = root_rotation * local_dir` (rigid body; engines do not
  gimbal). The user's instinct that the graph is involved is right one step
  later: torque-aware allocation needs each engine's *position* and lever arm
  about the computed center of mass - deferred with design call 2.
- The autopilot's control law already reduces every maneuver to "produce this
  velocity error"; the only nose-specific parts are the authority scan
  (forward-aligned only), the facing command (always rotates the *nose* onto
  the error), and the burn gate (nose alignment). All three generalize to
  "group" versions without touching the maneuver logic (arrival curve,
  deadband, settle, disengage semantics all stay).
- The arrival plan (GOTO) currently assumes braking means flipping the main
  drive (`flip_lead_time` = 1.5s of un-braked travel). With a retro group the
  correct plan brakes *without* a flip: both the braking authority and the
  lead time must come from the group the planner would actually brake with.
- Manual mode is untouched: W remains "main drive forward" (bound thrusters
  keep their own keys - that is what the binds are for).

## Options considered

- **A. Direction groups + time-scored choice (recommended).** Cluster live
  thrusters by local direction (greedy, reusing the existing ~25 degree
  alignment cone); each group has a direction and an authority sum. Per tick,
  score each group against the velocity error: rotation time (angle between
  the group's world direction and the error, over an estimated turn rate) *
  bias + burn time (`error * mass / authority`); command the attitude that
  brings the *winning group* (not the nose) onto the error; fire every
  engine currently inside the cone (call 1). Pros: one clean generalization,
  pure and testable, flip-less retro braking falls out, degrades exactly like
  today when the ship has one rear thruster. Cons: the turn-rate estimate is
  a constant knob, not derived from inertia - good enough for feel, recorded
  for the retune.
- **B. Full thrust allocation (solve for per-engine throttle vector).** The
  real solver: minimize time subject to per-engine limits and (later) torque
  balance. Correct endgame, overkill now - the group model covers the ships
  the editor can build today, and the solver slots in behind the same seam
  when torque-awareness arrives. Deferred, not rejected.
- **C. Keep nose-only, require builders to point the main drive backward.**
  Rejected: it wastes the sections system's whole point and contradicts the
  user's report directly.

### Resolved sub-decisions (within A)

1. **Grouping is per-tick and greedy** over live engines (few thrusters, no
   caching): first engine seeds a group, later ones join the first group
   whose direction is within the cone, else seed their own. Weighted mean
   direction, summed authority.
2. **The fine deadband generalizes.** Within `attitude_deadband` the computer
   still never rotates, but "finish axially" becomes "fire any engine already
   on the error" - a lateral thruster now kills a lateral crumb without a
   pirouette, so multi-thruster ships stop *more* precisely, not less.
3. **Arrival lead becomes dynamic**: `lead = brake_rotation_angle /
   est_turn_rate + spool pad`, where the brake group is chosen by the same
   scorer against the retrograde direction; `flip_lead_time` is replaced by
   the pad + rate knobs. A retro-equipped ship brakes late and flat; a
   main-drive-only ship still budgets its 180.
4. **Per-engine burn gate with the existing hysteresis** (lit engines keep a
   looser cone via their own spooled input), and the settle/cool-on-release
   rules apply to every driven engine.
5. **New knobs** (reflected, retune-owned): `rotation_bias` (1.5),
   `est_turn_rate_deg` (~90 deg/s, the same capital-feel number the polish
   task already planned to tune), `spool pad` for the arrival lead.

## Recommendation

Build **A** as a rework of `autopilot_system` plus pure helpers
(`cluster_thrusters`, `score_group`, generalized gates), keeping every
maneuver-level rule (arrival curve with floor, deadband, settle,
cool-on-release, breakout) exactly as shipped. Physics-level tests drive it,
as both previous flight bugs were caught that way: a retro-equipped ship must
brake a small overspeed *without* the hull flipping, flip anyway for a large
burn (the bias math), kill a lateral crumb with a side thruster, and lose a
group's authority when its engines are destroyed.

## Open questions

- Torque-aware allocation (positions/COM lever arms - the section-graph
  follow-up), and deriving the turn-rate estimate from PD frequency +
  computed inertia instead of a knob: both recorded for later; the knob is
  retune-owned until then.
- Whether the AI brain should adopt the group planner once shared - existing
  deferral stands.

## Next steps

- tatr 20260709-121842 (created with this spike): implement the group
  planner in `flight.rs` per the calls above.
- 20260709-095043 (retune) additionally owns `rotation_bias` and
  `est_turn_rate_deg`.
