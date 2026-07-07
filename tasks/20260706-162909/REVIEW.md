# Review: Use inertia tensor for projectile muzzle velocity

- TASK: 20260706-162909
- BRANCH: feat/muzzle-inertia-tensor

## Round 1

- VERDICT: APPROVE

Diff adds a pure `rigid_body_point_velocity` helper (game_object.rs) with four unit tests, and
rewrites the muzzle-velocity computation at both `shoot_spawn_projectile` sites (torpedo mod.rs,
turret_section.rs) to use it with a world-space center of mass. The two FIXMEs are removed.

Verified:

- Physics is correct. `v = v_lin + omega x (p - com)` is the rigid-body point-velocity relation;
  a muzzle offset from the COM of a spinning ship gains the tangential term. The unit tests pin
  the meaningful cases (pure translation ignores offset, a point on the COM ignores rotation,
  pure rotation gives the right tangential direction/magnitude, and the two add).
- The frame bug in the original sketch is fixed, not carried forward. avian's
  `ComputedCenterOfMass` is body-local (confirmed against the avian docs), and both sites now
  lift it to world space via `ship_transform.transform_point(**center)` before differencing
  against the world-space muzzle position. The old `projectile_position - **center` mixed frames
  and would have used a wrong lever arm.
- No behavioral regression on the linear-only path: with zero angular velocity the helper
  returns exactly `lin_vel`, so non-rotating ships fire exactly as before.
- Error handling matches the surrounding code: the added `compute_global_transform(*spaceship)`
  failure path logs and `continue`s like the existing muzzle/spawner transform lookups.
- Reuse over duplication: one tested helper rather than two inline cross products; both call
  sites read cleanly and share the tricky world-COM conversion, which is commented at each.
- Full suite green: `cargo test --workspace` (54 nova_gameplay incl. 4 new, examples_smoke under
  Xvfb), `cargo clippy --workspace --all-targets` clean (only the pre-existing `hull_section.rs`
  `struct update` warning, outside this diff).

No BLOCKER/MAJOR/MINOR findings. The subtle part (local vs world COM) is handled correctly and
documented.
