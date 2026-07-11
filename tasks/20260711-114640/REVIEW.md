# Review: Torpedo launch samples the eased pose in Update

- TASK: 20260711-114640
- BRANCH: fix/torpedo-spawn-raw-clock

## Round 1

- VERDICT: APPROVE

No blocking findings. Verification performed beyond reading the diff:

- Independently re-derived the exit-direction equivalence: the old
  `spawner_transform.up()` is `global_rotation * Vec3::Y`, and the new
  `(ship_raw_rotation * bay_local_rotation) * Vec3::Y` is the same
  composition on the raw clock (no scale in the chain) - the change is
  exactly the clock, not the geometry. The arming origin moves by the
  deleted 0.01 s nudge (0.3 u at exit 30), immaterial against
  arm_distance.
- Re-checked the two-ticks-along bound in the regression: exit travel is
  0.469 u/tick at the rig's 30 u/s, and a launch observed after a
  double-tick frame sits at most 0.94 u ahead - the 0.9875 bound is
  tight but correct, and the damping-zeroed config note explains why it
  can be.
- Schedule audit verified against the code: every system left in Update
  writes control inputs (steering, rotation command, throttle level) or
  gameplay thresholds; the force/spawn writers are all FixedUpdate. The
  fuze's one-frame staleness is honestly recorded as observed-and-left.
- Helper promotion checked for behavior drift: `local_pose_in_root`
  moved verbatim to sections/mod.rs; the turret module suite (13) stays
  green against the shared copy, and no other call sites exist.
- A/B honesty: reproduced the pre-fix failure (1.72 u cross-offset at
  150 u/s, velocity-proportional as the task predicted) and the
  post-fix pass; the allegiance test's rig update adds the components
  the new query shape requires without weakening its assertion.
- Suites run in the worktree: torpedo 61, turret 13, flight 60, ai 86,
  fmt/check clean. Full workspace suite deferred to CI per the user's
  standing instruction.

Note (non-blocking): the regression's section-level rotation is identity
- the non-trivial rotation lives on the spawner. The composition depth
is otherwise covered by the turret rigs that already exercise the shared
helper through three-level chains, so this is noted for completeness,
not requested as a change.
