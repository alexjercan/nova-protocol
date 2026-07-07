# Integrity: physics-level tests for collision damage + graph construction

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.4.0,test,integrity

Follow-up from the destruction-pipeline tests (task 20260525-133008). Those cover the
avian-free core of the integrity pipeline (disable -> destroy -> chain, aggregation,
meshless despawn) by driving `ConnectedTo` / `HealthApplyDamage` directly. The
physics-driven *inputs* to the pipeline are still untested because they need an avian
world:

- `on_impact_collision_deal_damage` - impulse/energy damage on `CollisionStart`.
- `on_blast_collision_deal_damage` - radial blast damage on collision with a blast
  sensor (also carries the FIXME in `20260706-162912` about inconsistent firing).
- `build_integrity_relations` - derives each node's `ConnectedTo` from `ColliderOf`
  and section grid positions, and marks the body `IntegrityRoot`.

## Steps

- [ ] Add integration tests that run `avian3d::PhysicsPlugins` (headless) and set up a
      small body with colliders/sections, then assert: a collision applies the expected
      damage; a blast sensor overlap applies falloff damage; and `build_integrity_relations`
      produces the right neighbor lists + `IntegrityRoot` for a ship vs. a lone asteroid.
- [ ] Reuse the pattern from the unit tests where possible; keep them headless.

## Notes

Source: `crates/nova_gameplay/src/integrity/plugin.rs` (collision observers),
`crates/nova_gameplay/src/integrity/glue.rs` (`build_integrity_relations`). Pairs with
the FIXME in `20260706-162912` (OnDestroyed / blast-collision inconsistency).
