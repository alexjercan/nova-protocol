# Review: Thrust balancing - compensate off-center engine torque under burn

- TASK: 20260709-155920
- BRANCH: thrust-balancing

## Round 1

- VERDICT: APPROVE

Reviewed the diff against master (`flight.rs` +the design doc +TASK.md), ran
the `flight::` suite (31 pass) and `cargo check`/`cargo fmt` (clean, no
warnings). The load-bearing physics claims were recomputed against sources
rather than trusted:

- **QP correctness.** `balance_throttles` minimizes `||sum torque_i u_i||^2`
  s.t. `sum forward_i u_i = demand`, `u_i in [0,1]` by projected gradient with a
  Euclidean capacity projection (`project_onto_demand`). Hand-traced the
  reviewer's own two-engine case (T_A=+0.5, T_B=-1.5, w=1, demand=1): PG
  converges to `uA=0.75, uB=0.25` in ~3-4 iterations, matching the unit test.
  The projection's bracket both terminates (lo maps to 0 <= demand, hi expands
  to total_forward >= demand) and is monotone, so the bisection is well-posed.
  The Lipschitz bound `2*sum||T_i||^2 >= 2*lambda_max(M^T M)` is conservative
  (correct direction). Uniform `demand/total_forward` is always the feasible
  seed, so a symmetric drive is a strict no-op - confirmed by
  `balance_throttles_keeps_a_symmetric_drive_uniform`.

- **Frame-invariance (manual path).** Net torque = 0 is invariant under the ship
  rotation (R*0 = 0), and the forward constraint is built consistently in the
  body-local frame (b = local -Z, `forward_i = mag * local_dir.dot(-Z)`), so
  computing the allocation in the local frame is valid. `ComputedCenterOfMass`
  is already body-local, so no lift is needed there - correct and simpler.

- **World-COM lift + lever arm (autopilot path).** Verified sections are direct
  children of the root (`nova_scenario/.../spaceship.rs:87`, `with_children`
  with `Transform::from_translation(section.position)`), so
  `position.0 + rotation.mul_vec3(transform.translation)` reconstructs exactly
  the world point the impulse system applies force at
  (`GlobalTransform.translation()`), and `com_world = rotation*com_local +
  position.0` matches the repo's COM convention (`live_structure_anchor`,
  turret/torpedo). avian `Position` is the origin (not the COM): if it were the
  COM the lift would double-count and the "balanced" throttles would be computed
  about the wrong center and fail to null the physical torque - the balanced
  test would drift. It holds, which end-to-end validates the lever-arm math.

- **Test non-vacuity.** `balanced_partial_burn_holds_an_off_center_twin_drive`
  contrasts partial (0.4, holds < 0.15) vs full (1.0, pulls > 0.4) burn on the
  *same* off-center geometry, so the only variable is throttle headroom and the
  full-burn pull proves the ship is genuinely off-center. The R1.2 pin
  `off_center_burn_pulls_but_a_centered_drive_is_held` is unchanged (a lone
  engine is the no-headroom floor) and still green - no regression.

Design is sound: force is a hard constraint (respects the autopilot's commanded
burn magnitude, which "maximize thrust s.t. zero torque" would have broken) and
torque is the objective; the PD remains the residual backstop. Scope
(firing-set-only, no lateral recruitment) is deliberate, matches the user's
differential-throttle framing, and is documented. No BLOCKER/MAJOR/MINOR
findings.

- [x] R1.1 (NIT) flight.rs:balance_throttles - for a firing set of one engine
  the 40-iter PG + projection is dead work (a lone engine can't be balanced;
  the projection just pins it to `demand/forward`). An early return when
  `engines.len() <= 1` would save the loop. Micro-optimization only, on a path
  that runs for ~one ship; take it or leave it.
  - Response: fixed - added `if n == 1 { return u; }` right after the uniform
    seed (which for n=1 is exactly `demand/forward`). Verified: `flight::` suite
    still 31/31 green.
- [x] R1.2 (NIT) flight.rs autopilot firing pass - the direct-child-of-root
  assumption behind the world-point lift is load-bearing but implicit (as it
  already was for the pre-existing `dir` computation). A one-line comment noting
  "sections are direct children of the root" would help a future reader who
  mounts a nested section. Optional.
  - Response: already covered - the firing-pass comment reads "World point of
    the engine (direct child of the root), the same point the impulse system
    pushes from". Redundant NIT on my part; no change needed.
</content>
