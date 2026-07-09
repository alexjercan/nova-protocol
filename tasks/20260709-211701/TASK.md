# Turret lead ignores inherited shooter velocity (bullets miss under motion)

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.4.0,bug,turret,physics

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
