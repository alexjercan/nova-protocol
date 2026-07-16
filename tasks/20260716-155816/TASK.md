# Remove the base demo scenario

- STATUS: OPEN
- PRIORITY: 100
- TAGS: refactor, content

## Goal

Delete the base "demo" scenario: it is useless as shipped content and the
only hand-written base content (no .rs builder, not parity-guarded).
After this, all base content is builder-generated. Decision from audit
task 20260716-141620 (FINDINGS.md section 1), user confirmed.

The demo MOD is NOT touched: `assets/mods/demo/` (scenario
`demo_mod_arena`) and its `mods.catalog.ron` entry stay.

## Steps

- [ ] Sweep first (sweep-then-delete): grep the whole repo (code, docs/,
      web/src/wiki, CHANGELOG, README) for references to the BASE demo
      scenario (id "demo", "Demo Scenario", demo.content.ron under base/)
      and list what needs updating; distinguish from demo-MOD mentions.
- [ ] Delete `assets/base/scenarios/demo.content.ron`; drop
      `"scenarios/demo.content.ron"` from `assets/base/base.bundle.ron`.
- [ ] Update `crates/nova_assets/tests/demo_scenario.rs`:
      `catalog_loads_and_base_only_merges_by_default` (:343) asserts
      scenario "demo" registers from base and lists registered ids at
      :364-371 - drop the "demo" expectations. Keep all demo-MOD coverage
      (`enabling_demo_overrides_a_section_and_adds_a_scenario`,
      `merge_bundles_overlays_demo_over_base`, hidden-fixture tests).
- [ ] Update the `content_ron_parity.rs` header comment: the
      "hand-migrated demo.content.ron is not guarded here" exception no
      longer exists.
- [ ] Fix the doc/wiki mentions found in the sweep.
- [ ] Verify: `cargo check --workspace --all-targets`, `cargo fmt`, run
      the touched test files (demo_scenario; content_ron_parity) - full
      suite is CI's job per the standing skip-local-tests instruction.

## Notes

- Audit 141620 confirmed no production code references scenario id "demo".
- The demo scenario is not hidden, so the Scenarios picker loses one row;
  nothing selects it by default (default = first visible by name sort).
- `nova_modding` cfg(test) fixtures use "demo" as a self-contained id
  (lib.rs:348+); those parse inline strings, leave them.
