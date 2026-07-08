# Add collider section as separate child entity

- STATUS: CLOSED
- PRIORITY: 25
- TAGS: v0.4.0, refactor

Stores collider-related components, spawned per hull/section. Legacy #80.

## Investigation (deferred - needs runtime verification)

Left OPEN deliberately. This is a deep physics/integrity change that cannot be trusted
from a compile check alone; it must be validated by running the game.

Why it is coupled: the integrity system keys its graph and collision handling on
`Collider` and `Health` living on the SAME entity:
- integrity/plugin.rs `on_collider_of_spawn_insert_collision_events`:
  `Query<Entity, (With<ColliderOf>, With<Health>)>`
- `on_collider_graph_create`: `Query<(Entity, &ChildOf), With<ColliderOf>>` -> builds
  IntegrityGraph on the rigidbody
- damage handlers resolve `ColliderOf.body` and apply damage to the collider entity's
  Health.

Today `base_section()` puts `Collider` + `Health` on the section entity (a child of the
ship-root RigidBody), so avian's `ColliderOf` links them and the queries match. Moving the
Collider to a separate child entity (the task's goal) splits Collider from Health, so
`(With<ColliderOf>, With<Health>)` no longer matches and the graph/damage silently break.

Recommended approach (for a runtime-capable session):
1. Introduce a `collider_section(...)` bundle that carries the collider-related
   components (Collider, ColliderDensity, CollisionEventsEnabled, and the Health +
   integrity markers), spawned as a child of each hull/section entity.
2. Move Health + SectionMarker + ExplodableEntity onto that collider child (or teach the
   integrity queries to look at the collider child) so `ColliderOf + Health` stay together.
3. Re-check every integrity observer that queries SectionMarker/Health/ColliderOf/ChildOf
   for the new hierarchy depth (section -> collider child).
4. Runtime-verify: collisions still deal damage, sections still disable/destroy, the
   chain-reaction still propagates, and asteroids still explode.

Compile-only verification is insufficient here, so it was not landed on the cleanup branch.

## Attempt 2026-07-06 (reverted) - deferred to land with 20260706-162911

Tried the "collider on a child entity" split during v0.3.1 and reverted it: it repeatedly
destabilized ship physics for what is a preparatory refactor with no near-term gameplay
payoff. Findings, so the next attempt starts informed:

- Damage still works with the collider one level down, because `HealthApplyDamage` is
  `#[entity_event(propagate, auto_propagate)]` and bubbles up `ChildOf` from the collider
  child through the section to the ship root. So the damage handlers need NO change - the
  event reaches whichever ancestor has `Health`. (The self-or-parent resolve I first feared
  is unnecessary.)
- `on_collider_of_spawn_insert_collision_events` must enable events on a collider whose
  Health lives on its parent (self-or-parent Health check).
- The integrity graph builder must key on `ColliderOf.body` (the ship root) instead of the
  collider's `ChildOf` (which becomes the section).
- The blocker was mass/inertia: avian DOES support a collider nested two levels under a
  RigidBody (it recursively propagates `ColliderTransform` down through
  `AncestorMarker<ColliderMarker>` intermediates - see avian
  `collision/collider/collider_transform/plugin.rs`), and the collider child MUST carry a
  `Transform` (avian derives its mass contribution from `GlobalTransform`, which Bevy only
  maintains for entities with a `Transform`). Even after adding the `Transform`, ship mass
  aggregation stayed broken (thrust and bullet impulse produced no motion, and avian logged
  "Dynamic rigid body ... has no mass or inertia"). The likely remaining cause is spawning
  the collider child from a deferred observer (`On<Add, SectionColliderSpec>` +
  `with_children`) not re-triggering avian's `AncestorMarker`/mass-update pipeline in the
  right order. A `children!` inline in `base_section` is NOT an option: kind bundles like the
  thruster already contribute their own `children!` (exhaust), and two `children!` in one
  bundle both insert `Children` and panic ("duplicate components: Children").

Recommendation: do this together with 20260706-162911 (integrity graph via relations), where
the whole collider/health/section-node model is reconsidered at once, rather than bolting a
hierarchy change onto the current same-entity assumptions. Retagged to v0.4.0 to travel with
that rework.

## Resolution (CLOSED - won't do, 2026-07-08)

Closed as no value. Decided not to split Collider onto a child entity:

- No gameplay payoff. Purely an organizational refactor ("store collider components on a
  child"), as the 2026-07-06 notes admit.
- Reverted twice (v0.3.1 and 2026-07-06), both times destabilizing ship physics: avian's
  mass/inertia aggregation broke ("Dynamic rigid body ... has no mass or inertia"), thrust
  and bullet impulse stopped moving the ship. An unresolved deferred-spawn / AncestorMarker
  ordering blocker remains.
- The rework it was retagged to travel with, 20260706-162911 (integrity graph via
  relations), is now CLOSED and landed keeping Collider + Health on the SAME section entity.
  It works cleanly, proving the split is not a prerequisite for anything.
- The split's only load-bearing requirement is teaching bcs's generic
  `on_collider_of_spawn_insert_collision_events` to find Health on an ancestor (self-or-parent
  check), because splitting moves Collider to the child while Health stays on the section.
  We do not want to bloat the generic integrity core with ancestor-walking to support a
  nova-only hierarchy that has no payoff.

Keeping the current same-entity design (Collider + Health on the section entity) is simpler:
exact-entity ColliderOf+Health match in bcs, HealthApplyDamage already auto-propagates up
ChildOf, no ancestor-walking anywhere.
