# Review: content lint core + bin + CI gate

- TASK: 20260716-191543
- BRANCH: feature/content-lint

## Round 1

- VERDICT: REQUEST_CHANGES

Verified: the gate A/B reproduced the original Ledger near-miss class
(unknown prototype failed CI naming ship/section/prototype; restored,
green); 7 unit tests each carry a failing case; the real tree lints
clean with exactly one accurate warning (the ch4 choice fork, correctly
downgraded to Warn by the same-handler/cross-handler split). The walk's
dependency-aware known-sections and all-bundles known-scenarios match
the spike.

- [x] R1.1 (MAJOR) crates/nova_scenario/src/lint.rs check_action - a
  `ScatterObjects` TEMPLATE is a full ScenarioObjectConfig and can be a
  Spaceship with Prototype sections, but only direct
  SpawnScenarioObject actions are inspected: a scattered ship with an
  unknown prototype lints clean (the exact bug class this task exists
  to catch, one wrapper deeper). Extract the spaceship-prototype check
  into a helper and run it on scatter templates too; add the failing
  unit test.
  - Response: fixed - check_object_prototypes helper runs on both direct
    spawns and scatter templates; the new
    unknown_prototype_in_a_scatter_template_is_an_error test fails
    without it (8/8 green).

## Round 2

- VERDICT: APPROVE

R1.1 verified: the helper covers scatter templates, the new unit test
pins it, the real tree still lints clean (one accurate warning). No new
findings.
