# Review: Integrity physics-level tests

- TASK: 20260707-170001
- BRANCH: test/physics-integrity-tests

## Round 1

- VERDICT: APPROVE

Diff adds a headless-avian test harness (`integrity/test_support.rs`) and six physics-level
tests co-located with the code under test (4 in `plugin.rs`, 2 in `glue.rs`). No production
code changed - this is a pure test addition.

Verified:

- Spec. The task asked for three things and the diff delivers all three: a collision applies
  the expected damage (`an_impact_applies_damage_from_relative_velocity_and_mass`), a blast
  sensor overlap applies falloff damage (`a_blast_sensor_overlap_applies_falloff_damage`), and
  `build_integrity_relations` produces the right neighbor lists + `IntegrityRoot` for a ship
  (`a_ship_builds_adjacency_from_section_positions`) vs. a lone asteroid
  (`a_lone_body_becomes_an_empty_leaf_root`). Two negative tests round it out.
- Tests assert behavior, not execution. The impact test recomputes the expected damage from
  the real `ComputedMass` and the observer's own private constants, so it verifies the wiring +
  real physics state rather than a magic number, and asserts damage lands on collider1 only
  (not the source collider2). The graph tests assert exact neighbor sets.
- Determinism is sound and the tradeoffs are justified in-code. Injecting `CollisionStart` for
  the impact case (because the solver zeroes the read velocity) and using a real overlap for
  the blast case (a sensor has no solver response) is the right split, and the earlier
  double-count bug (real overlap + injected event) was caught and removed. Mass is read after
  `settle()` because `ComputedMass` is `NaN` on the first step - documented.
- Harness matches avian's own `create_app`, including the `MeshPlugin` needed by the
  `collider-from-mesh` feature; the "Message not initialized" hazard is explained in a comment.
- Full suite green: `cargo test --workspace` (48 nova_gameplay incl. 6 new, examples_smoke
  under Xvfb 46s), `cargo clippy --workspace --all-targets` clean. The remaining `struct
  update` warning is the pre-existing `hull_section.rs` one (already filed in the 133008 retro),
  outside this diff.

No BLOCKER/MAJOR/MINOR findings. Coverage boundary is honest and the harness is reusable for
future physics-level integrity tests.
