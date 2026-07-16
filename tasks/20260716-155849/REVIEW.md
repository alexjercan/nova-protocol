# Review: Data-driven menu scenario roles

- TASK: 20260716-155849
- BRANCH: feature/menu-scenario-roles

## Round 1

- VERDICT: APPROVE

Verified independently:

- The base-only trust rule is real and its pin can fail: this review
  sabotaged the `entry.decl.base` gate in register_bundles and
  `new_game_declaration_is_honored_only_from_base` went red with
  "hijacked_start" winning; restored, 13/13. The rule the user asked for
  (mods cannot set the New Game scenario) is enforced at the merge and
  proven at the merge.
- Production names zero scenario ids: the grep gate over crates/*/src
  shows only content builders defining their own ids, test modules, and
  doc examples.
- The shipped end-to-end path is pinned: real base.bundle.ron ->
  loader -> merge -> NewGameStart == Some("shakedown_run").
- The seeded rotation test proves the draw stays in the flagged set AND
  reaches both backdrops (deterministic seed, cannot flake); the
  candidates are id-sorted before the draw, so HashMap iteration order
  cannot leak into the sequence.
- Failure paths all covered by tests: unregistered pick, unregistered
  declaration, no declaration, empty registry (loads nothing, no
  panic), no backdrop flagged (bare camera spawns).
- fmt clean, check --all-targets clean, nova_menu 49/49,
  demo_scenario 13/13, nova_mod_format 9/9, content_ron_parity 2/2.

Remark (non-blocking): the "menu UI renders through the scenario
camera, so a missing camera bricks the menu" rationale rests on the
pre-existing blanking logic's own comments rather than a dedicated
test - the fallback camera is strictly an improvement either way, and
a headless pin of UI-camera coupling is not practical in this suite.

No findings.
