# Review: Bullet renders one frame at its raw spawn pose before interpolation kicks in

- TASK: 20260711-121839
- BRANCH: fix/bullet-spawn-render-pose

## Round 1

- VERDICT: APPROVE

No findings. Verification performed beyond reading the diff:

- Independently re-verified the load-bearing teleport-guard claim against
  bevy_transform_interpolation 0.5.0 source:
  `reset_easing_states_on_transform_change` resets only when the Transform
  differs from BOTH `start` and `end`; `end` is copied FROM the Transform
  at FixedLast and nothing writes the Transform between FixedLast and the
  guard, so the equality holds bitwise and the seed survives. The scale
  easing state (required via ScaleInterpolation) is also safe: its `end`
  equals the unchanged spawn scale.
- Re-verified the type-identity risk of the new direct dependency:
  `cargo tree` shows a single unified bevy_transform_interpolation v0.5.0
  instance, so the seeded components are the same types avian's systems
  read. A future avian bump that splits the versions would make the seed
  inert - and the new regression fails loudly in that case, which is the
  right guard.
- Re-derived the expected first-frame geometry: seeded start = tick-start
  muzzle, end = same-tick integrated raw pose, so first render minus
  eased muzzle = alpha * v_exit * (dt - lead) - purely along-stream. The
  test's measured cross-stream of exactly 0 empirically confirms avian's
  same-tick writeback (an un-integrated `end` would leak ship-motion
  terms into cross).
- A/B honesty: reproduced the pre-fix failure (2.09 u cross-stream) and
  the post-fix pass; the two pre-existing stream regressions (linearity,
  cadence) are untouched by the diff and green.
- Test quality: null-style assertion ("sits ON the stream line") carries
  delivery guards (>= 10 first frames sampled, easing alpha swept below
  0.5), so a dead turret or a beat that never misaligns cannot
  greenwash it.

Out-of-scope note (not a finding): the torpedo spawn will want the same
easing-seed pattern when task 20260711-114640 moves it to FixedUpdate;
noted here so that cycle picks it up.
