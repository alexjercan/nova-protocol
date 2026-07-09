# Thrust balancing: differential throttle through the center of mass

Task: `tasks/20260709-155920`. Closes the torque-aware allocation follow-up
recorded by the multi-thruster spike (`docs/spikes/20260709-121746`) and review
R1.2 of the flight-feel retune (`docs/2026-07-09-flight-feel-retune.md`).

## The problem

The thruster impulse system applies each engine's force at the engine's world
position (`apply_linear_impulse_at_point`), so an engine offset from the ship's
center of mass produces a real torque `(engine_pos - COM) x thrust`. Since the
retune cut the controller's `max_torque` 100 -> 40, the PD holds only about
`max_torque / 64` (~0.6) units of lateral lever arm per unit of thruster
magnitude. Past that break-even an asymmetric editor build, or a symmetric ship
whose COM has shifted under battle damage, pulls or pinwheels under burn: the
flight computer was fighting its own drive with the attitude controller instead
of aiming the drive.

## The model: torque-aware allocation (decided with the user)

Two candidates were weighed:

- **PD feed-forward** - predict the burn's torque and pre-command the controller
  to counter it. Rejected: feed-forward removes the controller's *lag*, not its
  *authority ceiling*. It still cannot produce more than `max_torque`, so a
  genuinely off-center drive still saturates and pulls - it just fights sooner.
  It does not solve the problem the task exists for.
- **Differential throttle (chosen)** - modulate each engine's throttle so the
  net thrust torque about the live COM is nulled at the source. No dependence on
  controller authority, no saturation ceiling; the cost is some net thrust. This
  is how real multi-engine craft balance (Falcon 9 differential throttle, KSP
  balancers), and it is the "full thrust allocation" the multi-thruster spike
  named as the endgame (its option B).

The user picked the general form (torque-aware allocation) over a pairwise
down-throttle heuristic, and asked for the more realistic option throughout.

### The allocation

`flight::balance_throttles` solves, over the firing set (the engines already
inside the alignment cone of the burn), a tiny convex QP:

```text
minimize   || sum_i torque_i * u_i ||^2         (net torque about the COM)
subject to sum_i forward_i * u_i = demand        (deliver the commanded thrust)
           0 <= u_i <= 1                          (throttle box)
```

- `forward_i` is the thrust engine `i` adds along the burn direction per unit
  input; `torque_i = (engine_pos - COM) x (thrust_dir_i * magnitude_i)` is its
  lever-arm torque per unit input.
- `demand` is exactly what the maneuver already asked for:
  `min(desired_impulse, firing_authority)` for the autopilot, `burn * sum
  forward_i` for the manual drive. **Force is a hard constraint, torque is the
  objective** - the computer delivers the thrust you commanded and routes the
  resultant as close to *through* the COM as the throttle headroom allows.

Why this shape:

- The uniform throttle `demand / sum(forward)` (the old shared-drive behavior)
  is always a feasible point, so a **symmetric drive returns unchanged** and a
  balanced ship is a strict no-op.
- **Headroom is the currency.** At partial throttle there is spare thrust to
  shift onto the better-placed engines and null the torque; at full throttle the
  force constraint pins every engine at 1.0 and there is nothing to trim with, so
  the ship pulls (held only by the PD) - which is physically honest: you cannot
  have maximum thrust *and* balance on an asymmetric ship.
- It **degrades gracefully**: a lone off-center engine has no redistribution
  freedom (throttle scales magnitude, not the line of action), so it just fires
  as commanded and the PD holds the residual - identical to the pre-balance
  behavior, no regression. The PD is the residual backstop, not the primary.

Respecting the demanded magnitude is why the force is a hard equality rather than
"maximize thrust subject to zero torque": the autopilot's arrival curve commands
a *specific* burn magnitude, and a balancer that always fired the maximum
balanced thrust would break arrival control. One model serves both the autopilot
and the manual drive.

### Solver

Projected gradient: gradient step on `||sum torque_i u_i||^2` (Lipschitz bound
`2 * sum||torque_i||^2`), then Euclidean projection back onto `{sum forward_i u_i
= demand} n [0,1]^n` via a bisection on the single capacity multiplier
(`project_onto_demand`). The firing set is a handful of engines and the objective
is convex, so ~40 iterations converge to f32 tolerance; the whole thing is a pure
function, unit-tested against hand-computed optima.

## Where it lives

Both input paths route through the same helper, at the point where thruster
inputs are chosen:

- `autopilot_system`: the firing set is collected with each engine's
  `(forward, torque)` about the world COM (`ComputedCenterOfMass` lifted with
  rotation + translation), the shared `demand` is allocated, and each engine
  spools toward its own balanced throttle instead of a shared one.
- `manual_burn_system`: the main-drive set is collected with its coefficients in
  the **ship-local** frame - the balance constraint (net torque = 0) is
  frame-invariant and `ComputedCenterOfMass` is already body-local, so no world
  lift is needed - and the analog burn is allocated the same way.

## Scope and boundaries (deliberate)

- **Only the firing set is used** - the engines that push toward the burn. The
  balancer does not recruit lateral/RCS thrusters to counter-torque, because
  firing a lateral would add a sideways force the maneuver did not ask for. This
  matches the user's "differential throttle" framing. Recruiting off-axis
  thrusters for pure counter-torque (which would let a *single* main drive be
  balanced against a shifted COM) is a further follow-up, not this task.
- **Full-throttle asymmetric burns still pull.** No headroom, no balance - held
  only by the PD within its cap, exactly as before. Easing off the stick frees
  the drive to fly straight. Diegetic and documented.
- **AI ships** (`input/ai.rs`) are a separate control path and are untouched, as
  with the rest of the flight-layer work.

## Verification

- Pure-helper unit tests (`balance_throttles_*`): the two-engine split to null
  torque (hand-computed `uA = 0.75, uB = 0.25`), a symmetric drive returned
  uniform, and the no-headroom fallbacks (lone engine, full-throttle demand,
  degenerate/empty inputs).
- Physics-level test `balanced_partial_burn_holds_an_off_center_twin_drive`: a
  twin drive at unequal lever arms (x = +4, -1; COM at x = 0.75) holds its
  heading within the centered-drive tolerance under a 40% burn, and pulls under a
  full-stick burn - the only difference being throttle headroom.
- `off_center_burn_pulls_but_a_centered_drive_is_held` (the R1.2 pin) is
  unchanged and still green: a lone off-center engine is the balancer's
  no-headroom floor, so its behavior is identical - now explained rather than
  merely accepted.
- All 31 `flight::` tests pass; workspace `cargo check` and `cargo fmt` clean.
</content>
</invoke>
