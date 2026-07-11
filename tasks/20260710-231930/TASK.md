# Bullets twitch badly at high spaceship velocity

- STATUS: OPEN
- PRIORITY: 85
- TAGS: v0.5.0, rendering, physics, bug

## Goal

Playtest bug (user, 2026-07-10): bullets look funky - they twitch really
badly and "spew out" non-linearly at high spaceship velocity. Root cause
(docs/spikes/20260711-103527-twitching-family-two-clocks.md): bullets spawn
in Update from the EASED muzzle pose with RAW inherited velocity, the fire
timer quantizes shots to render frames, the only compensation is a static
`muzzle_exit_velocity * 0.01`, and a mid-frame-spawned bullet freezes until
the next physics tick. Every term errs by ~V * tick with per-shot phase, so
streams scatter at high ship velocity.

## Steps

- [ ] Move fire timing to FixedUpdate: tick `TurretSectionBarrelFireState`
      with fixed dt (`update_barrel_fire_state`, turret_section.rs:237) and
      expose the timer overshoot (time since the shot was due) to the
      spawner. Preserve multi-shot-per-tick for fire intervals shorter
      than a tick.
- [ ] Move `shoot_spawn_projectile` (turret_section.rs:752) to FixedUpdate
      and compute the muzzle pose from raw physics state: root raw
      `Position`/`Rotation` composed with the local chain
      (section -> rotators -> muzzle local Transforms; these are game-written
      locals, safe to reuse). No `TransformHelper`/`GlobalTransform` - both
      are stale eased state in FixedUpdate.
- [ ] Source every velocity term from the same raw state: keep
      `muzzle_exit_velocity + rigid_body_point_velocity(...)` but lift the
      COM with the raw pose instead of the ship GlobalTransform
      (turret_section.rs:826).
- [ ] Replace the `+ muzzle_exit_velocity * 0.01` fudge (turret_section.rs:834)
      with overshoot compensation: advance the spawn position by the
      bullet's FULL velocity times the timer overshoot, so spacing stays
      linear at any fire rate/frame rate.
- [ ] Regression test: ship at high V (e.g. 100 u/s), fire a burst across
      several ticks; rewind each bullet by `t_i * velocity_i` to its spawn
      time and assert the muzzle-frame spawn points are collinear and
      uniformly spaced within epsilon (the "linear stream" property the
      playtest missed).
- [ ] Verify the muzzle flash observer still fires at the rendered (eased)
      muzzle so visuals stay attached to the barrel; note the intentional
      render-vs-physics offset (bullet origin up to one tick ahead of the
      eased muzzle) here and in the docs.
- [ ] cargo check + fmt + new tests; extend the spike doc fix record.

## Notes

- Evidence: turret_section.rs:240 (spawn system in Update), :803 (eased
  muzzle via TransformHelper), :783 (raw velocities), :826 (eased COM
  lift), :834 (static fudge), :237 (render-rate fire timer).
- Bullets keep `TransformInterpolation` (turret_section.rs:854); spawning
  on the tick boundary makes their first rendered frames well-defined.
- Same investigation umbrella as 20260710-231928/229/231; spike covered
  all four - do not re-spike.
