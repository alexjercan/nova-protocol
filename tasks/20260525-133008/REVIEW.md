# Review: Unit tests for health + destruction pipeline

- TASK: 20260525-133008
- BRANCH: test/destruction-pipeline

## Round 1

- VERDICT: APPROVE

Turns a completely untested pipeline into 13 co-located integration tests that
exercise the real observer/system cascade, not mocks. The design choice is the
right one: the core integrity observers are avian-free and bcs `HealthPlugin` is
observer-only, so the tests run in a bare `App` with no physics or asset setup -
fast, deterministic, and still end-to-end through real `HealthApplyDamage`.

Coverage maps onto the task's "damage -> disable -> explode" sequence and more:
blast-damage falloff; leaf derivation by connection count; disabled-leaf ->
destroy; disabled-non-leaf -> NOT destroyed (the important negative); chain trigger
(becoming a leaf while disabled); whole-body destroy on a disabled root; the full
damage -> health-zero -> disabled -> destroyed path; a genuine chain reaction
through an A-B-C line; ship health = sum of sections and -> zero when gone; section
deactivate vs. destroy; meshless despawn.

Verified independently in the worktree:

- `cargo test -p nova_gameplay`: 42/42 pass (13 new + 29 existing).
- `cargo clippy -p nova_gameplay --tests`: the integrity additions are clean.

The tests assert behavior (specific markers appear / do not appear, health values,
entity despawn), not mere execution, and the negatives (`a_disabled_non_leaf_is_not_destroyed`,
`a_disabled_leaf_section_is_not_deactivated`) guard the branch conditions that make
the cascade correct rather than trigger-happy.

No BLOCKER/MAJOR. Two NITs.

- [ ] R1.1 (NIT) The physics-driven inputs to the pipeline are out of coverage: the
  collision-damage observers (`on_impact_collision_deal_damage`,
  `on_blast_collision_deal_damage`) and the graph builder
  (`build_integrity_relations`, which derives `ConnectedTo` from `ColliderOf` +
  section positions). These need an avian world, so they are a natural follow-up
  (an integrity test that runs `PhysicsPlugins`), not this unit-level task. The
  logic downstream of graph construction is covered by driving `ConnectedTo`
  directly. Worth a follow-up task if physics-level integrity coverage is wanted.
  - Response:
- [ ] R1.2 (NIT) `clippy --tests` surfaces a pre-existing `needless_update`
  (`..default()`) in `hull_section.rs:112`, unrelated to this change and left
  untouched to keep the diff focused. Trivial one-line fix for whoever next touches
  that file.
  - Response:
