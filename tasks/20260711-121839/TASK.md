# Bullet renders one frame at its raw spawn pose before interpolation kicks in

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.5.0,rendering,turret

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

- [ ] Confirm the mechanism with bevy_transform_interpolation's actual
      first-frame behavior for a freshly spawned body (what previous state
      does it ease from on the spawn frame?).
- [ ] Fix direction to evaluate first: split the clocks at spawn - insert
      avian `Position`/`Rotation` explicitly with the raw physics pose
      (physics starts correct) while the spawn `Transform` carries the
      RENDER pose (eased muzzle, no lead offset), so the first rendered
      frame is already attached to the rendered barrel. Verify avian's
      transform_to_position does not overwrite the explicit Position
      (change-detection: a Position modified since the last physics tick
      is treated as user-set) and that the interpolation crate seeds its
      previous state from the render pose.
- [ ] If the split-clock spawn fights avian, fallback: keep one clock but
      seed the interpolation previous-state so the spawn frame renders at
      the eased muzzle.
- [ ] Extend the stream regression (or add a sibling) asserting the FIRST
      rendered Transform of a bullet sits within a small distance of the
      eased muzzle pose while its Position stays the raw spawn point -
      both clocks pinned, with the usual delivery guards.

## Notes

- Context: tasks/20260710-231930 (the spawn rewrite; documents the
  intentional render-vs-physics offset this task now polishes away),
  docs/spikes/20260711-103527-twitching-family-two-clocks.md.
- Cosmetic (one frame), hence P60; do not regress stream linearity or
  cadence (regressions must stay green).
