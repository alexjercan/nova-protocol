# Use inertia tensor for projectile muzzle velocity

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.4.0, physics

From the TODO sweep (task 20260525-132954). Projectile muzzle velocity currently uses
only the spaceship's linear velocity as inertia; it should account for angular velocity
(inertia tensor) so projectiles fired from a rotating ship inherit the correct motion.

Source FIXMEs:
- crates/nova_gameplay/src/sections/torpedo_section.rs
- crates/nova_gameplay/src/sections/turret_section.rs
