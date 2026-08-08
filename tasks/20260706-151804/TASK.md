# Promote generic nova_gameplay helpers to bevy_common_systems

- STATUS: CLOSED
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

## Progress

Spike: tasks/20260708-110317/SPIKE.md. The generic code was
added to bevy_common_systems (PRs #2 and #3 there, master 34b3f0a).

Migrated here (this branch): bumped the bcs git rev to 34b3f0a in all four crates and
deleted the local copies, now consuming the promoted symbols:
- hud/health.rs -> bcs ui/health_display (HealthDisplay*).
- hud/objectives.rs -> bcs ui/objectives (GameObjectives/Objective/objectives_panel). The
  scenario-action ObjectiveActionConfig stays nova-local (it impls the foreign EventAction
  trait; orphan rule), moved into nova_scenario and backed by the bcs Objective.
- integrity/blast.rs + integrity/plugin.rs core (collision/blast damage, leaf/chain/destroy,
  calculate_blast_damage) + integrity/components.rs -> bcs integrity. Nova keeps glue.rs +
  explode.rs, now bundled by NovaIntegrityPlugin (bcs IntegrityPlugin + glue + explode).
- game_object.rs (rigid_body_point_velocity, destructible_body) -> bcs physics/rigid_body.
  bcs destructible_body omits ExplodableEntity, so section/asteroid spawns add it explicitly.

Still local (deferred, Tier C in the spike): hud/velocity.rs
(DirectionMagnitudeMaterial / DirectionSphereMaterial + shaders/directional_*.wgsl) - needs
the wgsl vendored into bcs first.

Decision (2026-07-08): hud/velocity stays in nova and will NOT be promoted. Promoting the
DirectionMagnitude / DirectionSphere materials requires vendoring their wgsl into
bevy_common_systems, which would compile the shaders into the crate (embedded_asset) and make
them awkward to tweak or hot-reload from a game. We prefer shaders to remain game-side,
editable assets, so the materials + shaders stay in hud/velocity. (bcs PR #4 attempted the
promotion and was closed as not approved.) With this, everything promotable from this task is
done; closing.
