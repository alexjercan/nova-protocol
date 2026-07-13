# Bullet renders one frame at its raw spawn pose before interpolation kicks in

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.5.0, rendering, turret

## Goal

Playtest (user, 2026-07-11, after the raw-clock spawn fix 20260710-231930
landed): bullets look good in flight, but "when they get spawned the
position is off by an amount, then on the next frame they get repositioned
to the right place". Mechanism: the bullet spawns in FixedUpdate with its
PHYSICS pose (raw clock, including the sub-tick lead offset behind the
muzzle), and that Transform is what the first rendered frame shows;
starting with the next frame, TransformInterpolation renders the eased
pose aligned with the rest of the world. One frame of visible pop at the
muzzle, proportional to ship speed plus up to one tick of muzzle-exit
travel.

## Steps

- [x] Confirm the mechanism with bevy_transform_interpolation's actual
      first-frame behavior for a freshly spawned body. CONFIRMED against
      bevy_transform_interpolation 0.5.0 source: easing lerps only when
      BOTH `start` and `end` are Some; `start` is written at FixedFirst
      (before the body exists on its spawn tick) and `end` at FixedLast.
      A mid-tick spawn therefore has start=None on its first frame - the
      ease is skipped and the frame shows the raw avian writeback pose,
      while every pre-existing body renders eased. Exactly one frame of
      pop; from the next tick's FixedFirst the states are complete.
- [x] Split-clock spawn evaluated and REJECTED with the derivation
      instead of code: avian's writeback overwrites the spawn Transform
      with the raw pose in the same tick's FixedPostUpdate, so a
      render-pose Transform at spawn cannot survive to the first render
      regardless of Position change-detection - and an explicit raw
      `Position` insert is byte-identical to what avian already derives
      from the raw spawn Transform. The split-clock variant degenerates
      to the fallback plus risk; the fallback is the whole fix.
- [x] Fallback implemented as the fix: seed `TranslationEasingState`/
      `RotationEasingState` `start` in the spawn bundle with the
      tick-start muzzle pose (no lead offset) and rotation; `end` fills
      at FixedLast as usual. First frame then renders
      lerp(muzzle_tick_start, raw_end, alpha) - the same interpolation
      clock as the ship, so the bullet is attached to the rendered
      barrel (cross-stream offset exactly 0) and only ever AHEAD of it
      along the stream by alpha * muzzle_speed * (dt - lead), bounded by
      one tick of muzzle travel. The teleport-reset guard
      (reset_easing_states_on_transform_change) keeps the seed because
      the written Transform equals `end` bitwise. Requires a direct
      bevy_transform_interpolation dep (avian re-exports the markers,
      not the easing states) pinned to the version avian resolves.
- [x] Regression `first_rendered_frame_attaches_the_bullet_to_the_eased_muzzle`
      (turret_section.rs): every bullet of a 24 rounds/s stream at
      150 u/s cross-travel is sampled on its FIRST rendered frame
      against the muzzle composed from the ship's eased Transform;
      asserts cross-stream < 0.02 u, along-stream in (-0.05, one tick of
      muzzle travel], with delivery guards (>= 10 bullets, easing alpha
      actually swept below 0.5). A/B: pre-fix fails at 2.09 u
      cross-stream; post-fix measures exactly 0. The raw-clock side
      stays pinned by the untouched stream linearity + cadence
      regressions (both green).

## Notes

- Context: tasks/20260710-231930 (the spawn rewrite; documents the
  intentional render-vs-physics offset this task now polishes away),
  tasks/20260711-103527/SPIKE.md.
- Cosmetic (one frame), hence P60; do not regress stream linearity or
  cadence (regressions must stay green).

## Resolution

What changed: the turret projectile spawn bundle seeds its interpolation
easing states (translation start = tick-start muzzle pose without the
sub-tick lead offset, rotation start = spawn rotation), plus a direct
`bevy_transform_interpolation = 0.5.0` dependency for the state types.
Physics is untouched: Position/velocity math and the sub-tick lead are
exactly as 20260710-231930 landed them.

Evidence rig (record-the-rig rule): unfinished_integrity_physics_app_with
(60 fps manual frames, 64 Hz avian), spawn_stream_rig (ship with
TransformInterpolation, turret at (0,1,0), muzzle at (0,0,-0.5) yawed
0.3, muzzle speed 200), ship LinearVelocity X*150, fire rate 24/s so the
64-vs-60 beat sweeps easing alpha; first rendered Transform of each new
bullet compared against the muzzle recomposed from the ship's eased
Transform of the same frame.

Difficulties: first version of the regression composed the expected
muzzle by rotating the muzzle's mount translation with the muzzle's own
rotation - a constant 0.148 u phantom cross-stream offset that looked
like a residual bug in the fix; the constant-across-alpha signature
identified it as a test-side frame error (a child's rotation aims its
frame, it does not displace its mount point).

Self-reflection: the task's primary direction (split-clock spawn) was
worth rejecting on paper rather than in code - reading the dependency's
writeback and easing schedules settled in minutes what a prototype would
have taken a cycle to discover. The constant-offset debugging pattern
(offset invariant across alpha implies rig math, not timing) is worth
remembering for interpolation tests.
