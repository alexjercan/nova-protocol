# Review: Nova typed-damage core

- TASK: 20260712-133343
- BRANCH: feature/typed-damage-core

## Round 1

- VERDICT: APPROVE

Out-of-context (fresh-context agent) review. Independently re-derived the
neutralization (bcs residual ~2e-4 at 1e-6 mass) and the authored damage numbers
(better 20.25, light 3.825), confirmed no double-scale/double-count, exact
resistance-table match, full SectionDamageClass spawn-path coverage, and a clean
`BlastDamageMarker` sweep (zero remaining uses / zero `blast_damage(` calls).
Check suite: `cargo check --workspace --all-targets` clean; damage 7/7,
turret_section 18/18, torpedo_section 26/26; `cargo fmt --check` clean.

Only two NITs, both about test clarity (not correctness):

- [x] R1.1 (NIT) damage.rs:544 - `nova_blast_deals_typed_falloff_once` manually
  `add_observer(on_nova_blast_collision)` because `integrity_physics_app()` does
  NOT include `NovaDamagePlugin`; that single registration is load-bearing (a
  second registration would double the damage and mask a real double-count
  regression). Add a comment making the deliberate non-inclusion explicit.
  - Response: fixed - added a comment at the manual add_observer noting the rig
    deliberately omits NovaDamagePlugin so this is the sole registration.
- [x] R1.2 (NIT) turret_section.rs:2203 - `sensor_bullets_damage_without_knockback`
  still spawns a `Mass(0.1)` bullet with no `ProjectileDamage`, exercising bcs's
  emergent kinetic (correct for a knockback/no-tunnel test, but non-representative
  of the production bullet). Note that it intentionally uses the old emergent path
  to isolate the physics-contact behavior.
  - Response: fixed - added a note that this test intentionally uses the old
    emergent-kinetic path (Mass 0.1, no ProjectileDamage) to isolate knockback;
    the typed path has its own test (typed_bullet_applies_resistance_scaled_damage).
