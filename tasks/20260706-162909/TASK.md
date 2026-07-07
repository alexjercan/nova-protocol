# Use inertia tensor for projectile muzzle velocity

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.4.0, physics

From the TODO sweep (task 20260525-132954). Projectile muzzle velocity currently uses
only the spaceship's linear velocity as inertia; it should account for angular velocity
(inertia tensor) so projectiles fired from a rotating ship inherit the correct motion.

Source FIXMEs:
- crates/nova_gameplay/src/sections/torpedo_section.rs
- crates/nova_gameplay/src/sections/turret_section.rs

## Root cause / correctness note

Both spawn sites already sketched `ang_vel.cross(radius_vector) + lin_vel` but discarded it
(`let _inertia_vel = ...; let inertia_vel = **lin_vel;`). The sketch also had a frame bug:
`radius_vector = projectile_position - **center`, subtracting avian's *body-local*
`ComputedCenterOfMass` from a *world* muzzle position. So enabling it as-written would have used
a wrong lever arm. The correct radius is `muzzle_world - world_center_of_mass`, where the world
COM is the local COM lifted through the ship's global transform.

## Fix

- Added `rigid_body_point_velocity(linear, angular, center_of_mass, point)` in `game_object.rs`
  - the standard rigid-body relation `v = v_lin + omega x (p - com)` - as a shared, pure,
  unit-tested helper (both sections need it).
- Both `shoot_spawn_projectile` sites (torpedo `mod.rs`, turret `turret_section.rs`) now compute
  the ship's global transform, lift the local `ComputedCenterOfMass` to world space with it, and
  call the helper. A shot from a rotating ship now inherits the muzzle's tangential swing, not
  just the ship's linear velocity.

## Steps

- [x] Add + unit-test the pure `rigid_body_point_velocity` helper (translation-only, on-COM,
      pure-rotation, combined).
- [x] Wire both spawn sites through it with the world-space COM conversion; remove the FIXMEs
      and the mis-framed `radius_vector`.
- [x] Full check suite green: `cargo test --workspace` (54 nova_gameplay, incl. 4 new;
      examples_smoke under Xvfb), `cargo clippy --workspace --all-targets`.

## Notes

Kept as a pure helper rather than inlining the cross product twice so the kinematics are tested
once and the two call sites stay readable. The world-frame COM conversion (`transform_point`) is
the subtle part and is called out in comments at both sites.
