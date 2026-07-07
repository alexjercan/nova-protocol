# Review: Turret aim lead (intercept aim)

- TASK: 20260707-150001
- BRANCH: feature/turret-aim-lead

## Round 1

- VERDICT: APPROVE

Delivers the Goal, and the range proves it. The turret now aims at the lead
intercept instead of the target's current position: `lead_intercept_point` (pure,
tested) solves the projectile-intercept quadratic; `update_turret_aim_point`
resolves it once into a public `TurretSectionAimPoint`; the yaw/pitch systems
steer to that. Target velocity is carried on `TurretSectionTargetVelocity`, set by
whoever aims - the crosshair leaves it zero (fixed aim point, no lead, unchanged),
the range feeds the moving gate's `LinearVelocity`.

Verified independently in the worktree:

- `cargo test -p nova_gameplay`: 29/29 pass, including the 3 lead tests - notably
  `lead_intercepts_a_crossing_target`, which checks the intercept is *consistent*
  (a bullet at `muzzle_speed` and the target arrive at the same point at the same
  time), not just "leads a bit".
- `cargo clippy -p nova_gameplay`: clean.
- `cargo build --example 08_turret_range` (no debug): green - harness cfg's out.
- Headless (Xvfb): aim error against the sweeping gate drops from the old 7-20 deg
  oscillation to a steady **0.1-0.7 deg** after the initial slew; bullets fire
  throughout; cycle complete, no panic.

Design is clean: single source of truth for the aim point (yaw, pitch, and the
example's gizmos/telemetry all read `TurretSectionAimPoint`), no duplicated lead
math, and the crosshair path is untouched. The aim-point system is correctly
chained before yaw/pitch in PostUpdate after transform propagation.

No BLOCKER/MAJOR. Two NITs, both benign.

- [ ] R1.1 (NIT) The intercept assumes constant target velocity, so against an
  accelerating / reversing target (the range's sinusoidal sweeper) it is slightly
  off at the reversal points. In practice the range still holds sub-degree, so this
  is not worth an iterative predictor. No change.
  - Response:
- [ ] R1.2 (NIT) The intercept is computed from the muzzle position at the current
  frame, which moves a little as the turret rotates - a one-frame approximation.
  Negligible at these speeds. No change.
  - Response:
