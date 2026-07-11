# Torpedo launch samples the eased pose in Update (raw-clock follow-up)

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.5.0,physics,torpedo

## Goal

Follow-up filed from the 20260710-231930 review (R1.1): the torpedo
section's own `shoot_spawn_projectile`
(crates/nova_gameplay/src/sections/torpedo_section/mod.rs:305-308,443)
still spawns in Update from `TransformHelper` (eased render pose) with
raw velocities - the same two-clocks mix fixed for turret bullets, at
lower severity: single guided launches, PN guidance absorbs the initial
offset, and there is no stream whose spacing can visibly scatter. Fix it
for consistency when touching the torpedo section next.

## Steps

- [ ] Move the torpedo `shoot_spawn_projectile` (and
      `update_spawner_fire_state` if its timing feeds physics) to
      FixedUpdate on the raw root pose, reusing the turret section's
      `local_pose_in_root` pattern (consider promoting that helper to a
      shared location instead of duplicating it).
- [ ] Check the torpedo arming/fuze/guidance Update-schedule readers while
      there: guidance steering on the render clock is a control input
      (acceptable); anything that writes forces or spawn states should be
      raw (see the FixedUpdate audit table in tasks/20260711-103527).
- [ ] Regression in the spirit of the turret stream test if a measurable
      invariant exists (launch pose offset vs ship velocity), else a
      schedule/pose-source assertion with a delivery guard.

## Notes

- Context: docs/spikes/20260711-103527-twitching-family-two-clocks.md
  (the two-clocks rule of thumb) and the turret fix in
  tasks/20260710-231930.
- Not part of the twitching-family umbrella's playtest symptoms; do not
  block 20260711-094915 on it.
