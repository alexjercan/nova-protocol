# Turret lead ignores inherited shooter velocity (bullets miss under motion)

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.4.0, bug, turret, physics

Playtest report (20260709, component-lock arc): lock, focus and component
switching all work, but bullets do not hit the target.

Root cause (confirmed in code): `shoot_spawn_projectile`
(sections/turret_section.rs) gives every bullet the full muzzle point
velocity - `linear_velocity = muzzle_exit_velocity + inertia_vel`, where
inertia_vel = ship linear + angular swing at the muzzle
(rigid_body_point_velocity). But `lead_intercept_point` solves the intercept
as if the bullet flew at pure muzzle_speed in the WORLD frame, so the aim
point is wrong by the shooter's own velocity: any ship motion makes shots
drift off target by exactly that motion.

Fix: solve the intercept in the shooter's frame. `update_turret_aim_point`
subtracts the shooter's muzzle-point velocity (same
rigid_body_point_velocity math the spawn uses, same COM lift) from the
target velocity before calling lead_intercept_point. Aiming the barrel at
that shooter-frame intercept and adding the inherited velocity at fire time
lands the bullet on the true world-frame intercept (derivation: dir*s*t =
(target - muzzle) + (v_t - v_muzzle)*t). This fixes every feeder (player,
AI, 08_turret_range) in one place and keeps TurretSectionTargetVelocity's
semantic as the target's WORLD velocity.

Invariant test: shooter and target in formation (equal velocities) =>
relative velocity zero => aim point == target position (inherited velocity
alone carries the bullet). Behavioral test: a moving shooter with a static
target must aim BEHIND its own motion (retrograde offset).

## Resolution (20260709)

Shipped: `update_turret_aim_point` now solves the intercept in the shooter's
frame - it subtracts the muzzle point velocity the bullet will inherit on
launch (ship linear + angular swing, the exact rigid_body_point_velocity +
COM-lift math shoot_spawn_projectile uses) from the target velocity before
calling lead_intercept_point. Doc comments on the solver, the aim-point
component and the system record the frame contract and the derivation.
Physics-less shooters (test rigs, previews) inherit nothing and keep the old
world-frame solve.

3 new tests: formation flight aims at the target itself (the relative-zero
invariant that directly encodes the playtest bug), a strafing shooter aims
retrograde of its own motion with a bullet-meets-target consistency check
(barrel_dir * speed + inherited velocity lands on the target at the solved
flight time), and the marker-only fallback. 8 turret tests green; workspace
check clean; 12_hud_range full PASS (the pip stage reads the aim point, so
it is self-consistent through the change).

Fix location chosen over feeding relative velocity from the input side: the
solve-side fix corrects every feeder (player three-tier feed, AI, the
08_turret_range example) in one place and keeps TurretSectionTargetVelocity
meaning the target's world velocity. Skipped honestly per user instruction:
full local suite and clippy.
