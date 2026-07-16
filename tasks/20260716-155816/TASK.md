# Remove the base demo scenario

- STATUS: CLOSED
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

- [x] Sweep first (sweep-then-delete): grep the whole repo (code, docs/,
      web/src/wiki, CHANGELOG, README) for references to the BASE demo
      scenario (id "demo", "Demo Scenario", demo.content.ron under base/)
      and list what needs updating; distinguish from demo-MOD mentions.
- [x] Delete `assets/base/scenarios/demo.content.ron`; drop
      `"scenarios/demo.content.ron"` from `assets/base/base.bundle.ron`.
- [x] Update `crates/nova_assets/tests/demo_scenario.rs`: drop/repoint
      the three base-"demo" assertions (catalog merge test, overlay test,
      merge_bundles test); keep all demo-MOD coverage.
- [x] Update the `content_ron_parity.rs` header comment: the
      "hand-migrated demo.content.ron is not guarded here" exception no
      longer exists.
- [x] Fix the doc/wiki mentions found in the sweep
      (web/src/wiki/dev/modding-ron.md worked-example pointer,
      nova_assets pretty_config doc, nova_mod_format content-path doc
      example).
- [x] Verify: `cargo check --workspace --all-targets`, `cargo fmt`, run
      the touched test files (demo_scenario; content_ron_parity) - full
      suite is CI's job per the standing skip-local-tests instruction.

## Notes

- Audit 141620 confirmed no production code references scenario id "demo".
- The demo scenario was not hidden, so the Scenarios picker loses one row;
  nothing selects it by default (default = first visible by name sort).
- `nova_modding` cfg(test) fixtures use "demo" as a self-contained id
  (lib.rs:348+) and `nova_mod_format` fixtures use the literal path
  string "scenarios/demo.content.ron" in inline RON (:179,:188); both
  parse in-memory data, never disk - left as-is.

## Close notes (2026-07-16)

What changed: deleted assets/base/scenarios/demo.content.ron, removed
its base.bundle.ron entry, repointed three demo_scenario.rs assertions
("base scenario present/remains" now pin shakedown_run; the built-in
list check gained broadside, which had landed on master but was missing
from the list), refreshed the content_ron_parity header, the
scenario_generation pretty_config doc, the BundleManifest content-path
doc example, and the modding-ron.md worked-example pointer (now the
demo MOD's mod.content.ron).

Difficulties: the plan's sweep listed two demo assertions in
demo_scenario.rs but the file had three - merge_bundles_overlays_
demo_over_base failed on first run (line 543, "a base scenario remains
after overlay") and was repointed the same way. The test run caught
what the grep summary missed because my earlier grep was head-truncated.

Also discovered: the audit FINDINGS.md says "four built-in scenarios";
broadside (builder-backed, f53fa5e8) makes it five - the audit read a
stale working file in the shared checkout. Doc counts in prose were
reworded to not state a number.

Verification: cargo check --workspace --all-targets green (cold 2m43s),
cargo fmt clean, demo_scenario 11/11, content_ron_parity 2/2. Full test
suite intentionally left to CI per the standing instruction.

Reflection: pipe grep sweeps through a file instead of head-truncating
them; the truncation hid the third assertion and cost one failed run.
