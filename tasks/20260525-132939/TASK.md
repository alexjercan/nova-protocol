# When switching scenes, remove all objects

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: v0.3.1, bug

Full cleanup on scene transition; no leftover entities between scenarios. Legacy #102.

## Steps

- [x] Enumerate every `commands.spawn` reachable during an active scenario.
- [x] Confirm each is torn down on a scene switch (scoped / auto-scoped / child / temp /
      remove-observer).
- [x] Document the cleanup contract so it does not regress.

## Resolution (CLOSED - verified complete)

Audited every entity-spawn site reachable while a scenario is active (loader, scenario
actions, all ship sections, HUDs, integrity/explode). Every one is cleaned up on a
scene transition, via one of five mechanisms:

1. Explicit ScenarioScopedMarker: scenario camera, directional light, input context,
   and all SpawnScenarioObject/CreateScenarioArea objects (base_scenario_object carries
   the marker). Both UnloadScenario and on_load_scenario despawn all scoped entities
   (recursively, so children go too) and clear NovaEventWorld before the next scenario.
2. Auto-scoped via on_add_entity_with observers in loader.rs, which retro-tag new
   entities carrying MeshFragmentMarker / TurretBulletProjectileMarker /
   TorpedoProjectileMarker while a scenario is loaded (explosion fragments, turret
   bullets, torpedoes).
3. Children of scoped entities: turret part meshes and muzzle effects are children of a
   ship section, removed by recursive despawn.
4. TempEntity self-despawn: torpedo blast effects (0.1s / 2.0s) expire on their own.
5. Player-ship-tied: HUDs spawn on Add<PlayerSpaceshipMarker> and despawn on
   Remove<PlayerSpaceshipMarker>, which fires when the scoped player ship is despawned.

Conclusion: the leftover-entities bug (legacy #102) is already fully resolved by the
ScenarioScopedMarker + on_add_entity_with architecture that was built out; there are no
un-cleaned spawn sites. No code change was needed. To keep it from regressing, I
documented the cleanup contract (the five buckets + the rule for new spawns) in
docs/scenario-system.md.

Self-reflection: the honest result of a "bug" task can be "already fixed, here's the
proof and the invariant that keeps it fixed". The exhaustive spawn-site sweep was the
right way to be sure rather than assuming; documenting the contract is what turns a
verification into durable value.
