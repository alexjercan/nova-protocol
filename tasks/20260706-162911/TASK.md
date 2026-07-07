# Refactor integrity plugin: graph via relations, split glue systems

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.3.1, refactor

From the TODO sweep (task 20260525-132954). The integrity plugin carries an
IntegrityGraph component that the author would rather express as Bevy relations, and
several systems are glue that should move to a glue.rs so integrity stays focused.

Source TODOs (crates/nova_gameplay/src/integrity/plugin.rs):
- IntegrityGraph component -> use relations instead
- move glue systems out to glue.rs (x2)

Note: the generic blast/impact-damage systems in the same file are tracked separately by
task 20260706-151804 (promote to bevy_common_systems).

## Resolution (CLOSED)

Done in v0.3.1. Two parts:

1. Graph via relations. Replaced the central `IntegrityGraph(HashMap<Entity, Vec<Entity>>)`
   component on the body with a per-node model, matching the code TODO ("children as nodes,
   a ConnectedTo Vec<Entity> on the child node"):
   - `ConnectedTo(pub Vec<Entity>)` on each integrity node (a ship section, or an asteroid's
     collider node) lists its adjacent nodes.
   - `IntegrityRoot` marks the owning body (ship root / asteroid), replacing `With<IntegrityGraph>`
     in `handle_parent_destroy` and `aggregate_ship_health`.
   - `build_integrity_relations` (On<Add, ColliderOf>) writes each node's `ConnectedTo` and
     marks the body `IntegrityRoot` (replaces `on_collider_build_integrity_graph`).
   - `on_destroyed` now prunes the destroyed node from its neighbors' `ConnectedTo` (mutating
     a neighbor marks it Changed) instead of editing a central map.
   - `derive_integrity_leaves` (Changed<ConnectedTo>) re-derives leaf markers - a node with
     <= 1 neighbors is a leaf (replaces `on_changed_graph`). Same cascade semantics.

2. Glue split. Moved the section-coupled systems (`build_integrity_relations`,
   `on_section_disable`, `aggregate_ship_health`) into a new `integrity/glue.rs`
   (`IntegrityGluePlugin`), so the integrity core in `plugin.rs` no longer references
   `SectionMarker`/`SectionInactiveMarker`. The generic core (destroy chain, leaf derivation,
   damage, collision-events) stays in `plugin.rs`.

Difficulty / bug found: firing torpedoes panicked in `aggregate_ship_health` -
`commands.entity(root).insert(Health)` on a root that was despawned the same frame. A torpedo
warhead is a short-lived mini-ship (TempEntity) that is itself an `IntegrityRoot`, so the
per-frame aggregate raced its despawn. Fixed with `try_insert`. This race existed in the
original aggregate too (it used a plain `insert`); it only surfaced now because testing
exercised torpedoes harder.

Self-reflection: this landed cleanly because it changed only the graph *representation* and
file layout, not the destruction/death *semantics* (aggregate + damage bubbling +
handle_parent_destroy), which were left byte-for-byte equivalent. The contrast with the
reverted collider-as-child attempt (20260525-132949) is the lesson: representation/organization
refactors are low-risk; moving components across the ECS hierarchy fights avian's mass/collider
assumptions and is not.
