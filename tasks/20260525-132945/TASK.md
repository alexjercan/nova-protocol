# Rework projectile and spawner plugin

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.3.1, refactor

Remove generics, simplify API. Legacy #94.

## Resolution (CLOSED - already resolved)

There is no longer a generic projectile/spawner plugin. Projectile spawning has been
reworked into concrete, per-section implementations:
- Turret bullets: TurretBulletProjectileMarker / TurretBulletProjectileOwner /
  TurretProjectileHooks (turret_section.rs).
- Torpedoes: TorpedoProjectileMarker / TorpedoProjectileOwner and the
  TorpedoSectionSpawner* state (torpedo_section.rs).

No generic `ProjectilePlugin<T>` / `SpawnerPlugin<T>` and no shared generic projectile
module remain. The only generics left are lifetime parameters on TurretProjectileHooks,
which avian's `CollisionHooks` SystemParam requires - not removable "API generics".

So the "remove generics" deliverable is met. The remaining simplification aspiration
(the torpedo spawner/targeting code is still large and inline) is tracked separately by
ticket 20260706-162913 (extract torpedo into its own module/plugin). Closed as
already-resolved.
