# Thrust balancing: compensate off-center engine torque under burn

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.4.0,handling,physics

From review R1.2 of the flight-feel retune (20260709-095043). Since the torque
budget cut (max_torque 100 -> 40) the flight computer can only hold roughly
`max_torque / 64` units of lateral lever arm per unit thruster magnitude
(~0.6 units): an asymmetric editor build, or a damage-shifted COM, pulls or
pinwheels under burn. Documented as diegetic in
docs/retros/20260709-flight-feel-retune.md and pinned by the
`off_center_burn_pulls_but_a_centered_drive_is_held` test - but a real flight
computer would balance thrust, not fight it with RCS.

## Steps

- [x] Decide the model with the user: differential throttle. Settled on
      *torque-aware allocation* (the differential-throttle general form): the
      flight computer solves per-engine throttles in [0,1] that deliver the
      demanded thrust along the burn direction while minimizing net torque
      about the live COM. Chosen over PD feed-forward because feed-forward
      cannot exceed the controller's `max_torque` (the very cap this task
      exists to stop fighting) - it removes lag, not the authority ceiling -
      whereas differential throttle balances at the source. The PD stays as a
      residual backstop for what throttle headroom cannot null (a lone
      off-center engine, or a full-throttle demand with no spare thrust).
- [x] Implement in the flight layer (manual_burn_system and the autopilot's
      spool loop both set thruster inputs; balancing belongs where the inputs
      are chosen, using each engine's lever arm about the live COM).
- [x] Extend the off-axis physics test: balanced burn tracks the command
      within the centered-drive tolerance.

## Notes

- Lever math: torque_i = (engine_pos - COM) x (world_dir * magnitude * input).
- Related: 20260709-155922 (disabled controller torque), the multi-thruster
  spike's deferred torque-aware allocation (docs/spikes/20260709-121746).

## Resolution

Implemented torque-aware allocation (differential throttle) in `flight.rs`.
Design and rationale: `docs/retros/20260709-thrust-balancing.md`.

- New pure helper `balance_throttles` (+ `project_onto_demand`,
  `BalanceEngine`): a tiny convex QP that splits the commanded thrust demand
  across the firing set to minimize net torque about the live COM
  (`min ||sum torque_i u_i||^2` s.t. `sum forward_i u_i = demand`, `u_i in
  [0,1]`), solved by projected gradient. Force is a hard constraint (deliver
  the commanded burn), torque is the objective; the uniform throttle is always
  feasible, so a symmetric drive is a no-op and headroom is what buys balance.
- `autopilot_system` collects each firing engine's `(forward, torque)` about
  the world COM (`ComputedCenterOfMass` lifted with rotation + translation) and
  spools each toward its own balanced throttle instead of a shared one.
- `manual_burn_system` does the same for the main-drive set in the body-local
  frame (the constraint is frame-invariant and the COM is already body-local).
- The PD stays as the residual backstop for what headroom cannot null.

Tests: `balance_throttles_*` unit tests (hand-computed split, symmetric no-op,
no-headroom fallbacks, degenerate inputs) and the physics test
`balanced_partial_burn_holds_an_off_center_twin_drive` (a twin drive at unequal
lever arms holds heading under a partial burn, pulls at full stick). The R1.2
pin `off_center_burn_pulls_but_a_centered_drive_is_held` is unchanged and still
green - a lone off-center engine is the balancer's no-headroom floor. All 31
`flight::` tests pass; workspace `cargo check`/`cargo fmt` clean.

### What went well / difficulties / reflection

- The decisive design argument (feed-forward cannot exceed `max_torque`, so it
  cannot solve a task that exists *because* `max_torque` was cut) came from
  reading the retune doc and the existing `off_center` test before writing
  anything - past-session breadcrumbs paying off again.
- Choosing "force hard, torque objective" over "maximize thrust s.t. zero
  torque" mattered: the latter ignores the autopilot's commanded burn magnitude
  and would break arrival control, and it strands a lone engine (torque=0 forces
  it off). The chosen shape degrades to the exact pre-balance behavior for a
  single engine, so no regression and the R1.2 test stayed valid untouched.
- Arithmetic before assertions: the two-engine split (`uA=0.75, uB=0.25`) and
  the twin-drive COM (x=0.75, arms 3.25/1.75) were hand-computed first, and the
  physics test contrasts partial vs full burn on the *same* geometry so the only
  variable is throttle headroom (non-vacuous by construction).
- Next time / follow-up: recruiting off-axis thrusters for pure counter-torque
  (to balance a single main drive against a shifted COM) is left as a further
  task - the firing-set-only scope matches the user's differential-throttle
  framing and avoids adding maneuver-unrequested sideways force.
