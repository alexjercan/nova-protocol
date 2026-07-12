# Nova typed-damage core: DamageType, resistance table, own-the-trigger application

- STATUS: OPEN
- PRIORITY: 46
- TAGS: v0.5.0,weapons,health,spike

## Goal

Foundation of the combat-depth typed-damage pass. Add a nova `DamageType` enum
(Kinetic/ArmorPiercing/Emp/Explosive), a `ProjectileDamage { amount, kind }`
component on projectiles, and a nova resistance table keyed by (section kind,
DamageType). Nova's weapon-hit callsites pre-scale (`amount x resistance`) and
THEN trigger bcs `HealthApplyDamage` - owning the trigger sidesteps Bevy 0.19's
arbitrary observer order, so no racing bcs's `on_damage` subtractor. Neutralize
bcs's emergent kinetic damage for the turret bullet, and route the torpedo blast
through the typed path as Explosive. Do NOT modify bcs.

"Done" = firing the turret at each section kind subtracts `authored_kinetic x
resistance` (with today's turret feel preserved because Kinetic is 1.0
everywhere), and a torpedo detonation subtracts `falloff x explosive_resistance`;
both proven in headless tests, with bcs's emergent kinetic contribution measured
to ~0 for the bullet.

## Steps

- [ ] Add a `damage` module in nova_gameplay (`crates/nova_gameplay/src/damage.rs`,
  registered in `lib.rs` + prelude). Define `DamageType { Kinetic, ArmorPiercing,
  Emp, Explosive }` (Component-free plain enum; `Copy`, `Reflect`) and
  `ProjectileDamage { amount: f32, kind: DamageType }` as a `Component`
  (`Copy`, `Reflect`).
- [ ] Add a section-kind discriminant readable at hit time. VERIFY FIRST which of
  the per-kind markers exist and are on the collider entity
  (`HullSectionMarker` hull_section.rs:27, `ThrusterSectionMarker`
  thruster_section.rs:84, `ControllerSectionMarker` controller_section.rs:87,
  `TurretSectionMarker` turret_section.rs:126, and the torpedo equivalent). The
  hit collider is the section entity itself (base_section.rs `base_section`
  attaches `Collider::cuboid` + `destructible_body` to that entity), so define a
  lightweight `SectionDamageClass { Hull, Thruster, Controller, Turret, Torpedo }`
  (`Component`, `Copy`, `Reflect`) and insert it wherever `base_section`/the
  kind-specific inserts run, so a single query resolves a hit entity's class
  without matching five markers.
- [ ] Add the resistance table as a `const fn resistance(class: SectionDamageClass,
  kind: DamageType) -> f32` in `damage.rs`, with EXACTLY the values from
  docs/spikes/20260712-160505 (Kinetic column all 1.0; AP Hull 1.5 / Thruster
  0.75 / Controller 1.0 / Turret 1.75 / Torpedo 1.0; EMP Hull 0.1 / Thruster 0.25
  / Controller 3.0 / Turret 1.5 / Torpedo 1.25; Explosive Hull 1.0 / Thruster 1.5
  / Controller 1.0 / Turret 0.5 / Torpedo 1.25). Document each row's intent in a
  comment referencing the spike.
- [ ] Add a nova helper `apply_typed_damage(commands, target_collider,
  source, class, damage: ProjectileDamage)` in `damage.rs` that computes
  `final = damage.amount * resistance(class, damage.kind)` and
  `commands.trigger(HealthApplyDamage { entity: target_collider, source, amount:
  final })`. Single application point so both weapons scale identically. (bcs
  `on_damage` health/mod.rs:110 then subtracts the pre-scaled amount; propagation
  and overkill-clamp are reused unchanged.)
- [ ] Turret: neutralize bcs emergent kinetic. bcs `on_impact_collision_deal_damage`
  (bcs integrity/plugin.rs:108) computes `effective_mass = m1*m2/(m1+m2)` then
  `damage <= f32::EPSILON` early-returns (plugin.rs:143-152). Set the turret
  bullet `Mass` (turret_section.rs:997) to a near-zero value (dedicated const,
  e.g. `1e-6`) so bcs's term rounds to ~0. Gravity is safe: `gravity_well_system`
  uses `forces.apply_linear_acceleration` (gravity.rs, mass-independent); Sensor
  bullets already impart no knockback. Keep `projectile_mass` config field OUT of
  bcs's kinetic role - author the typed amount instead (next step).
- [ ] Turret: author the Kinetic amount + own the trigger. Add a
  `ProjectileDamage { kind: Kinetic, amount }` to the bullet spawn bundle
  (turret_section.rs ~974). Add a `turret_bullet_damage` config field on
  `TurretSectionConfig`; author it to reproduce today's per-hit kinetic (measure
  in the headless rig - see tests) so DPS is unchanged. In `despawn_bullet_on_hit`
  (turret_section.rs:304, the nova-owned hit observer on the same `CollisionStart`)
  resolve the hit collider's `SectionDamageClass`, read the bullet's
  `ProjectileDamage`, and call `apply_typed_damage` before despawning the bullet.
  Keep the existing sensor/trigger-volume skip (do not damage pure volumes).
- [ ] Torpedo: type the blast Explosive via a nova-owned blast. Today
  `torpedo_detonate_system` (torpedo_section/projectile.rs:57) spawns a bcs
  `blast_damage(BlastDamageConfig{radius,max_damage})` sensor that bcs's
  `on_blast_collision_deal_damage` (bcs integrity/plugin.rs:178) applies UNTYPED.
  Replace that with a nova blast component carrying `{radius, max_damage, kind:
  Explosive, owner}` (NO bcs `BlastDamageMarker`, so bcs's blast observer stays
  dormant for it) and a nova observer on `CollisionStart` that ports bcs's
  `calculate_blast_damage` falloff (plugin.rs:229, `max_damage * (1 -
  distance/radius)`), then calls `apply_typed_damage` with the target's class.
  VERIFY FIRST that removing bcs's `BlastDamageMarker` from the torpedo path does
  not disturb the impact observer's `Without<BlastDamageMarker>` exclusion
  (plugin.rs:113) for other bodies.
- [ ] Tests (headless, `crates/nova_gameplay/src/damage.rs` + weapon modules):
  (1) `resistance()` returns the exact table values (a spot per column incl. the
  1.0 Kinetic invariant and the 3.0 EMP-vs-Controller peak). (2) FEEL-PRESERVING:
  a near-zero-mass bullet through bcs `on_impact` yields damage `<= f32::EPSILON`
  (would fail if mass left at 0.1 - A/B the const). (3) turret hit on a
  Turret-class vs Thruster-class target subtracts `amount x 1.0` for Kinetic
  (equal), and if a non-Kinetic test round is used, subtracts the scaled amount;
  assert the DELIVERY (health actually dropped) per delivery-guards lesson.
  (4) torpedo blast at distance d subtracts `max_damage*(1-d/radius)*explosive_res`
  against a known class, and bcs's blast observer did NOT also fire (no
  double-count). Use production spawn helpers / faithful rigs (production-faithful
  -rigs lesson); every assertion must be able to fail (would-it-fail-without-it).
- [ ] Docs: write `docs/<date>-typed-damage-core.md` (decision, the mass
  -neutralization measurement, the torpedo-blast-ownership choice, difficulties,
  self-reflection). Append a Fix record entry to BOTH spikes
  (20260712-133135 and 20260712-160505). Add a CHANGELOG Unreleased line.

## Notes

- Spikes: docs/spikes/20260712-133135 (architecture: own-the-trigger, why an
  observer would race) and docs/spikes/20260712-160505 (the four types,
  AmmoKind==DamageType decision, the resistance table + per-type intent).
- Do NOT modify bcs (git dep, rev a35b74c). Neutralize/route around it only.
- Relevant files: crates/nova_gameplay/src/sections/turret_section.rs
  (bullet spawn ~974, `despawn_bullet_on_hit` 304, `Mass` 997),
  sections/torpedo_section/projectile.rs (`torpedo_detonate_system` 57),
  sections/base_section.rs (`base_section`, `SectionMarker`),
  gravity.rs (`gravity_well_system`, mass-independent),
  integrity/mod.rs (nova adds bcs `IntegrityPlugin`). bcs:
  ~/.cargo/git/checkouts/bevy-common-systems-*/e0c115d/src/{integrity/plugin.rs,
  health/mod.rs}.
- Run `cargo check --workspace --all-targets` after adding config fields -
  `TurretSectionConfig` is constructed in nova_assets/sections.rs AND examples
  (check-all-targets-for-struct-field lesson); give new fields sensible defaults
  or update every initializer.
- AmmoKind vs DamageType stays 1:1 here; the superset question is phase-2's
  (task 20260712-133349).
- Blocks tasks 20260712-133349 (magazines/reload) and 20260712-133356 (alt-fire).
