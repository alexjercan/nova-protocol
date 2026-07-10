# Gravity wells: bounded one-way gravity with sphere of influence (physics substrate)

- STATUS: OPEN
- PRIORITY: 100
- TAGS: v0.5.0, physics, gravity, spike

Spike: docs/spikes/20260709-193147-gravity-wells-orbital-mechanics.md

Goal: designated bodies (asteroids above a radius threshold, per-scenario
override) carry a `GravityWell`; ships and torpedoes opt in via
`GravityAffected` and feel `a = mu / r^2` toward the center - clamped at the
surface, smoothstep-faded to zero at the SOI edge, zero outside, one dominant
well with hysteresis when SOIs overlap. Wells never pull wells; sources stay
static, so the world cannot clump. Strength is authored (surface_gravity,
radius-derived defaults, capped well below main-drive acceleration), not
mass-derived. New `gravity.rs` module beside flight.rs, one FixedUpdate force
system, tunables in one reflected settings tree, pure-math helpers +
physics-level orbit-stability tests. Direction-level: /plan owns the steps.
Prerequisite for the ORBIT verb (20260709-193339).