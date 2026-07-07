# Promote generic nova_gameplay helpers to bevy_common_systems

- STATUS: OPEN
- PRIORITY: 66
- TAGS: v0.4.0, refactor, crates

Follow-up from the nova_gameplay boundary audit (task 20260525-132936). These
modules are game-agnostic enough to belong in the external bevy_common_systems crate,
but promotion is a coordinated cross-repo change (add to the external repo, then
depend on it here and delete the local copy), so it is deferred out of v0.3.1.

Promotion candidates:
- hud/health.rs - text HUD over the generic Health component.
- hud/objectives.rs - generic id+message objectives text list.
- hud/velocity.rs - DirectionMagnitudeMaterial / DirectionSphereMaterial shader
  materials (must also move shaders/directional_*.wgsl).
- integrity/blast.rs plus calculate_blast_damage and on_impact_collision_deal_damage
  in integrity/mod.rs - radial blast volume + impulse/energy collision damage; only
  touch Avian physics and the generic Health/HealthApplyDamage. Needs extraction, not
  a plain file move, since they are entwined with the integrity observers.

Each promotion should only happen once the API is stable enough to be reused by
another game; until then they stay in nova_gameplay (tier 2 of the crate boundary
policy in docs/architecture.md).
