# Add collider section as separate child entity

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.3.1,refactor


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
