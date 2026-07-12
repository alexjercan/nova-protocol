# Variable damage by section type

- STATUS: CLOSED
- PRIORITY: 48
- TAGS: v0.5.0, health

Thrusters take more, turrets take less, etc. Legacy #122.

Direction (from the user, 2026-07-12): implement this as per-section-type
HEALTH differences, not a damage-interception system. With today's single
(kinetic) damage model, "a thruster takes more damage than a turret" is exactly
"a thruster has less health than a turret", and it is order-independent and
needs no touch of bevy_common_systems. A real per-damage-type `DamageResistance`
(AP/EMP vs section resistances) belongs to the next pass (task 20260708-162005),
implemented NOVA-side as its own health/damage-type system - NOT in bcs.

Why not intercept incoming damage now: both damage sources (kinetic impact,
torpedo blast) converge on bcs's `HealthApplyDamage`, and Bevy 0.19 makes
observer execution order arbitrary (bevy_ecs observer docs), so a nova observer
cannot reliably scale the amount before bcs's `on_damage` subtracts it. Health
tuning sidesteps that entirely.

## Steps

- [x] Give section TYPE a legible durability, expressed as named baseline
      constants in nova_assets/sections.rs with rationale (thrusters exposed ->
      fragile / take more; turrets armored mounts -> tough / take less;
      controller + torpedo bay at a mid baseline; hull structural). Direction
      follows the task title; it is a playtest knob and trivially flipped.
- [x] Differentiate the currently-identical 100-hp sections (basic_thruster,
      basic_controller, better_turret, torpedo) by type using those baselines.
      Keep the deliberately-tuned hull variants (reinforced 200, light 60) and
      the scavenger light_turret (60, documented) so the shakedown pirate fight
      is not silently rebalanced; comment where a per-section variant departs
      from its type baseline on purpose.
- [x] Lock the design intent as a regression: a test asserting the type-
      durability ORDERING holds (thruster baseline < controller baseline <
      turret baseline), so "variable by type" is a checked invariant, not loose
      magic numbers that can drift back to uniform.
- [x] Verify: cargo check + fmt; run the new test.
