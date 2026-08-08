# Torpedo launch samples the eased pose in Update (raw-clock follow-up)

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.5.0, physics, torpedo

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

- [x] `update_spawner_fire_state` + `shoot_spawn_projectile` moved to
      FixedUpdate (chained; the fire timing gates a physics spawn, so it
      ticks with physics). The spawn composes the bay from the ship's
      raw `Position`/`Rotation` via `local_pose_in_root`, which is
      PROMOTED to sections/mod.rs and shared with the turret (duplicate
      deleted). The old static `+ exit * 0.01` nudge is dropped: launch
      timing stays tick-quantized on purpose (single guided launch, no
      stream invariant, PN absorbs the sub-tick residual - documented at
      the spawn site, in contrast to the turret's exact sub-tick lead).
      The spawn also seeds the interpolation easing states (the
      20260711-121839 pattern, as that cycle's review requested) so the
      first RENDERED frame sits at the rendered bay - no launch pop.
- [x] Update-schedule readers audited: torpedo_pn_guidance (steering,
      control input), torpedo_sync_system (steering -> rotation command,
      consumed next tick by the FixedUpdate copy from 20260711-140241),
      torpedo_thrust_system (writes ThrusterSectionInput, a control
      level; the impulse itself is applied by the FixedUpdate
      thruster_impulse_system from raw pose), update_torpedo_arming /
      torpedo_detonate_system (gameplay thresholds, no force writes).
      All legitimately stay on the render clock; the schedule comment in
      the plugin records the reasoning. Observed-and-left: the proximity
      fuze reads a render-clock position, so its trigger distance
      carries up to one frame of closing-speed staleness - a gameplay
      tolerance question, not a physics-clock bug; file separately if
      playtests ever notice.
- [x] Regression `torpedo_launches_from_the_bay_on_both_clocks_at_speed`
      (real TorpedoSectionPlugin, ship at 150 u/s perpendicular to the
      launch direction, tilted spawner rotation, damping zeroed):
      every launch's raw Position must sit ON the raw bay's launch line
      (cross-offset < 0.05, along within two ticks of exit travel) and
      its first rendered Transform on the RENDERED bay's launch line.
      A/B: the old Update/eased-pose spawn fails at 1.72 u cross-offset.
      Delivery guards: repeated launches, ship still at speed. The
      allegiance test's bare-World rig updated to the new query shape
      (ship raw pose components, spawner in the mount chain).

## Notes

- Context: tasks/20260711-103527/SPIKE.md
  (the two-clocks rule of thumb) and the turret fix in
  tasks/20260710-231930.
- Not part of the twitching-family umbrella's playtest symptoms; do not
  block 20260711-094915 on it.

## Resolution

What changed: the torpedo launch chain (fire timing + spawn) moved to
FixedUpdate on the raw root pose with the shared `local_pose_in_root`
helper; the spawn drops the static nudge, seeds the interpolation easing
states, and inherits bay point-velocity from the same raw pose. Suites:
torpedo 61, turret 13, flight 60, ai 86 all green; fmt+check clean; full
workspace suite deferred to CI per user instruction.

Evidence rig (record-the-rig rule): unfinished_integrity_physics_app_with
(ProjectileHooks) + TorpedoSectionPlugin{render:false}, ship
(RigidBody + TransformInterpolation, unit cuboid) at LinearVelocity
X*150, torpedo section child at (0,1,0) with spawn_offset (0,0,-1),
spawn_rotation rot_x(0.3), spawner_speed 30, fire_rate 2/s, damping 0;
launches sampled on their first observed frame against the bay composed
from the ship's raw pose (Position/Rotation) and rendered pose
(Transform) of the same frame.

Self-reflection: a clean, small cycle - the pattern was already proven
twice (turret spawn, bullet easing seed) and the task file carried exact
pointers. The one non-mechanical judgment was keeping tick-quantized
launch timing instead of porting the sub-tick lead; writing the reason
at the spawn site (no stream invariant to protect) is what makes the
asymmetry with the turret legible to the next reader.
