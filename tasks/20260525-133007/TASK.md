# Unit tests for section graph (DFS/BFS)

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.4.0, test

Validate that the graph builds and updates correctly. [new]

## Resolution (closed as superseded)

Superseded, not implemented. The "DFS/BFS" premise is obsolete: the section graph
was refactored (task 20260706-162911) from a central `IntegrityGraph(HashMap)` built
by traversal to per-node `ConnectedTo` relations, built by `build_integrity_relations`
and updated by `on_destroyed` + `derive_integrity_leaves`. There is no DFS/BFS to test.

The task's two goals are already handled:
- "graph updates correctly" - covered by the destruction-pipeline tests (task
  20260525-133008, PR #35): `leaves_are_derived_from_the_connection_count` (leaf
  re-derivation when a node's connections change) and
  `destruction_chains_through_a_connected_structure` (pruning propagates through the
  graph as nodes are destroyed).
- "graph builds correctly" - `build_integrity_relations` (neighbor lists from section
  grid positions; asteroid -> empty; mark `IntegrityRoot`) needs an avian world and is
  tracked by the follow-up task 20260707-170001 (physics-level integrity tests).

Writing further section-graph tests here would only duplicate 133008. Closing.
