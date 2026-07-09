# Torpedo takes no contact damage from its own ship at launch: unify projectile owner collision filter

- STATUS: OPEN
- PRIORITY: 95
- TAGS: v0.4.0,bug,torpedo

Reported in play: torpedoes fired from the torpedo bay section immediately take
damage or are destroyed at spawn. With the combat-juice work the damage is now
visible as a hit flash right at the bay the moment a torpedo fires.

Root cause (verified in code): the torpedo root spawns essentially at the bay
spawner transform (`projectile_position + spawner_exit_velocity * 0.01`) with
`RigidBody::Dynamic` and two child sections (controller, thruster) that each get a
1x1x1 cuboid collider, 1.0 health and `CollisionEventsEnabled` (auto-added to any
collider with Health by bevy_common_systems). The torpedo leaves the bay at muzzle
speed relative to the ship, so when its child colliders overlap the firing ship's
section colliders at spawn, avian raises `CollisionStart` and
`on_impact_collision_deal_damage` (bevy_common_systems integrity plugin) applies
impulse/energy damage from the relative velocity. With 1.0 health per torpedo
section, the torpedo dies instantly; the ship's own bay/hull section can eat the
same contact damage. The arming gate (task 20260707-100003) only gates
detonation, not incoming contact damage - that task explicitly deferred hull
clipping ("revisit if torpedoes are seen clipping the hull").

Turret bullets already solve exactly this with avian collision hooks:
`TurretProjectileHooks::filter_pairs` skips any pair where one collider is a
bullet whose `TurretBulletProjectileOwner` equals the other collider's
`ColliderOf.body`, enabled per-entity via `ActiveCollisionHooks::FILTER_PAIRS`.
Torpedoes have no such filter. avian registers exactly ONE `CollisionHooks` type
per app (`plugin.rs:36`), so the fix is to generalize the existing hook, not add
a second one.

Expected: firing a torpedo never contact-damages the torpedo or the firing ship
at launch; the torpedo flies out, arms, and detonates normally. Contact
collisions with every other body (targets, asteroids, enemy PDC fire) keep
working.

## Steps

- [ ] Add a shared `ProjectileOwner(pub Entity)` component and a `ProjectileHooks`
      `CollisionHooks` impl in a new `crates/nova_gameplay/src/sections/projectile_hooks.rs`,
      exported through the sections prelude. `filter_pairs` resolves each collider's
      owner by looking for `ProjectileOwner` on the collider entity itself OR on its
      `ColliderOf.body` (torpedo colliders are children of the owning root), and
      returns false when that owner equals the other collider's `ColliderOf.body`
      (check both orientations of the pair).
- [ ] Replace `TurretBulletProjectileOwner` (private, turret_section.rs) and
      `TorpedoProjectileOwner` (pub, torpedo_section/mod.rs:149, used by
      input/player.rs:267 and its tests) with `ProjectileOwner`. Keep the marker
      components (`TurretBulletProjectileMarker`, `TorpedoProjectileMarker`) as the
      type discriminators in queries.
- [ ] Register the generalized hook in `crates/nova_gameplay/src/plugin.rs`:
      `with_collision_hooks::<ProjectileHooks>()` replacing `TurretProjectileHooks`.
- [ ] In `shoot_spawn_projectile` (torpedo_section/mod.rs), add
      `ActiveCollisionHooks::FILTER_PAIRS` to both torpedo child section entities
      (the entities that carry the colliders - the flag on the collider-less root
      does nothing).
- [ ] Physics-level integration tests (avian stepping, cf.
      `integrity/glue.rs::physics_tests` and `integrity/test_support.rs`; the test
      app must register `PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>()`):
      (1) a torpedo spawned overlapping its owner ship takes no damage and the ship
      sections take none; (2) the same torpedo overlapping a NON-owner body still
      collides (damage or contact reported); (3) a turret bullet still ignores its
      owner (regression for the renamed hook).
- [ ] Verify end to end with the torpedo range: headless
      `BCS_AUTOPILOT=1` run of `examples/06_torpedo_range.rs` under Xvfb still
      reports 3 fired / 3 armed / 3 detonated with no torpedo dying at spawn and no
      spawn-time hit flash/damage on the player ship.
- [ ] Document the decision (owner filter is permanent, turret parity; why a
      state-dependent/arming-gated filter was rejected) in the task resolution and
      retro.

## Notes

- Relevant files:
  - `crates/nova_gameplay/src/sections/torpedo_section/mod.rs` (spawn:
    `shoot_spawn_projectile` ~348-521; `TorpedoProjectileOwner` at 149)
  - `crates/nova_gameplay/src/sections/turret_section.rs` (hook 266-294, bullet
    spawn 822-838)
  - `crates/nova_gameplay/src/plugin.rs:36` (single hook registration)
  - `crates/nova_gameplay/src/input/player.rs:267` (torpedo owner query)
  - bevy_common_systems `src/integrity/plugin.rs` (`on_impact_collision_deal_damage`,
    `on_collider_of_spawn_insert_collision_events`) - external crate, read-only here.
- Decision: the owner filter is permanent for the projectile's lifetime (same
  semantics turret bullets already have). A torpedo that loops back passes through
  its own ship instead of contact-damaging it; blast damage is a separate sensor
  path and still hurts anything in radius including the owner. An arming-gated
  filter was considered and rejected: avian evaluates `filter_pairs` when a
  broad-phase pair is created, so a filter that changes answer mid-overlap is not
  re-evaluated reliably.
- Deliberately NOT filtered: torpedo-vs-torpedo pairs from the same ship (owner
  check compares projectile owner to the other collider's body, and a salvo
  sibling's body is the sibling torpedo, not the ship). Fire-rate spacing makes
  this a non-issue today; revisit only if salvo self-collisions show up.
- One avian `CollisionHooks` type per app: this is why the turret hook is
  generalized instead of adding `TorpedoProjectileHooks` alongside it.
- Verify while implementing: whether avian adds `ColliderOf` when collider and
  body share one entity (turret bullet root). The hook's "collider entity itself
  OR ColliderOf.body" lookup covers both cases regardless.
