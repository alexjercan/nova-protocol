# Typed-damage core: DamageType, resistance table, own-the-trigger application

Task: tasks/20260712-133343. Spikes: docs/spikes/20260712-133135 (architecture),
docs/spikes/20260712-160505 (taxonomy + table).

## What shipped

A nova typed-damage layer, phase 1 of the combat-depth pass, in a new
`crates/nova_gameplay/src/damage.rs` module:

- `DamageType { Kinetic, ArmorPiercing, Emp, Explosive }` and a
  `ProjectileDamage { amount, kind }` component on projectiles. Weapon damage is
  now AUTHORED, not emergent from bullet mass x velocity.
- `SectionDamageClass { Hull, Thruster, Controller, Turret, Torpedo }` - a
  discriminant-only mirror of `SectionKind`, inserted alongside each section's
  kind marker in the five `*_section` bundle fns, so a hit resolves its class in
  one query.
- `const fn resistance(class, kind) -> f32` - the taxonomy spike's table
  verbatim (Kinetic 1.0 everywhere; EMP 3.0 vs Controller / 0.1 vs Hull; AP 1.75
  vs Turret / 0.75 vs Thruster; Explosive 1.5 vs Thruster / 0.5 vs Turret; ...).
- `apply_typed_damage(commands, target, source, class, damage)` - the single
  application point: it scales `amount x resistance` and OWNS the
  `HealthApplyDamage` trigger, so bcs's `on_damage` just subtracts the pre-scaled
  amount and the whole integrity/destruction pipeline is reused unchanged. This
  is what sidesteps Bevy 0.19's arbitrary observer order (rejected option C).
- Turret bullets: spawned at a near-zero `NEUTRALIZED_BULLET_MASS` so bcs's
  emergent kinetic term rounds to ~0, plus an authored Kinetic `ProjectileDamage`.
  `despawn_bullet_on_hit` (nova already owned this observer) now applies the typed
  damage before despawning. The `projectile_mass` config knob was replaced with an
  authored `bullet_damage`, set via `representative_kinetic_damage(mass, speed)` so
  the two catalog turrets reproduce their old per-hit at the representative
  engagement speed - Kinetic-at-1.0 makes this exactly feel-preserving.
- Torpedoes: a nova `NovaBlast { radius, max_damage, kind }` typed blast replaces
  bcs's untyped `blast_damage`. A nova observer ports bcs's linear falloff, scales
  by the Explosive column, and triggers the typed damage. NovaBlast carries no bcs
  `BlastDamageMarker`, so bcs's blast observer stays dormant and damage is not
  double-counted.

## Decisions and alternatives

- **Own the trigger vs a scaling observer.** Forced by the architecture spike: a
  second observer on `HealthApplyDamage` races bcs's subtractor (no observer
  ordering in Bevy 0.19). Scaling before the trigger, in code nova owns, is the
  only race-free option that also does not modify bcs.
- **Neutralize bcs kinetic via near-zero mass** (not by dropping bcs's impact
  observer). The observer stays on for genuine kinetic collisions (a ship ramming
  an asteroid); only the bullet's contribution is zeroed, by mass. Verified safe
  for gravity: `gravity_well_system` applies a mass-INDEPENDENT acceleration
  (`forces.apply_linear_acceleration`), and sensor bullets take no contact forces.
- **Fixed authored damage vs velocity-dependent.** The old turret damage scaled
  with impact speed; the authored amount is fixed. This is the intended design
  (the spike calls it out) - "AP does X" cannot come from one velocity formula.
  Feel is preserved at the representative speed, not bit-for-bit per shot.
- **AmmoKind == DamageType** for now; the superset question is phase 2's.
- **SectionDamageClass as a component** vs matching five kind markers at hit time.
  A single discriminant component is one query and one source of truth; unknown
  targets (asteroids) simply default to a 1.0 multiplier.

## Difficulties and how they were diagnosed

- **Bundle arity.** Adding `ProjectileDamage` as a 17th top-level element of the
  bullet spawn tuple broke `impl Bundle` (max 16). Fixed by nesting
  `(Mass, ProjectileDamage)` - the same trick the file already used for the
  collider triple.
- **Test rigs needed the production entity shape.** First-pass integration tests
  spawned a target as a single entity with `destructible_body` + `Collider` and
  no `RigidBody`; bcs's impact/blast observers read `body1/body2` (the RigidBody
  entities), which were `None`, so no damage landed (drop 0). Fixed with a
  `spawn_target` helper that mirrors production: a RigidBody parent with a child
  collider holding the Health (and class). This is the production-faithful-rigs
  lesson - a clean trace on a non-faithful rig is not evidence.
- **Consumer sweep caught silent VFX loss.** Grepping `BlastDamageMarker` before
  finishing found the real blast-radius visual and particle-effect observers keyed
  on `Add<BlastDamageMarker>`, plus two example loggers and two "no blast" test
  assertions. Switching the torpedo to `NovaBlast` would have silently killed the
  detonation VFX/SFX. All were retargeted to `NovaBlast`; the two test assertions
  were fixed from a now-vacuous `With<BlastDamageMarker>` (nothing spawns it
  anymore) to `With<NovaBlast>`.

## Verification

New tests, all green (`cargo check --workspace --all-targets` clean, fmt clean):

- Resistance table spot checks incl. the Kinetic-1.0 invariant and the EMP/AP/
  Explosive extremes.
- `representative_kinetic_damage` pins the two authored turret values (20.25,
  3.825).
- Neutralization is proven by driving the REAL bcs impact observer: a
  neutralized-mass bullet deals < 0.01, and an A/B at the old 0.1 mass deals > 15
  (so the test can fail - would-it-fail-without-it).
- bcs subtracts exactly the pre-scaled amount nova triggers (own-the-trigger
  end to end).
- A real sensor overlap fires the nova blast once for the typed falloff x
  resistance, with no bcs double-count.
- Production-faithful turret hit: a near-zero-mass bullet with `ProjectileDamage`
  through `despawn_bullet_on_hit` deals Kinetic 1.0 / AP 1.75 (Turret) / AP 0.75
  (Thruster).

Per the repo convention the full suite runs in CI; locally only the new tests and
the directly-affected modules (damage, turret_section, torpedo_section, integrity)
were run.

## Self-reflection

- The consumer grep for `BlastDamageMarker` should have been the FIRST step of the
  torpedo change, not a near-final safety pass - it is exactly the sweep-then-edit
  discipline, and the blast VFX regression would have been a shipped, silent bug
  (no test covers particle spawning) had the grep been skipped.
- Authoring the turret damage from `representative_kinetic_damage(old_mass,
  old_speed)` rather than a magic number kept the "why 20.25" legible and testable;
  worth repeating whenever a value replaces an emergent one.
- Follow-ups this unblocks: multi-type magazines + reload (20260712-133349) and
  alt-fire (20260712-133356); HUD surfacing of type/ammo folds into 20260712-131348.
  Actual resistance numbers are a playtest knob - the intent is the durable part.
