# Decouple portal/publish tests from specific mods

- STATUS: OPEN
- PRIORITY: 70
- TAGS: testing, refactor

## Goal

No core test names a specific mod. Mods must be renamable/removable
without touching core CI. Decision from audit task 20260716-141620
(FINDINGS.md section 2), user confirmed. Smaller than it looks: both
portal test files already have synthetic fixture builders; only the two
headline e2e tests and some borrowed fixture names use real mods.

## Steps

- [ ] `crates/nova_portal_gen/tests/generate.rs`: in
      `real_webmods_publish_and_hashes_verify` (:19), replace the
      gauntlet-specific assertion (:32) with the generic contract - every
      subdirectory of `webmods/` appears as an entry id in the generated
      catalog (and no extras). The synthetic-fixture tests (:110+ already
      exist) stay as-is.
- [ ] `crates/nova_assets/tests/portal_install.rs`: in
      `portal_fetch_install_enable_uninstall_over_the_wire` (:291), swap
      `gauntlet_files()` (:242, reads webmods/gauntlet) for a synthetic
      fixture mod built like `mock_files()` (:477) but including a
      scenario content file, so the enable step can still assert a
      scenario id registers in GameScenarios. Drop the gauntlet-specific
      assertions (meta name, bundle path, `gauntlet_run` registration) in
      favor of the fixture's own ids. `serve_portal_tree` (:259) serves
      whatever tree it is given - the fixture slots in.
- [ ] Rename borrowed fixture ids to neutral names:
      `crates/nova_mod_format/src/lib.rs` tests (:256-284) "gauntlet" ->
      e.g. "fixture_pack"; `crates/nova_menu/src/lib.rs` picker tests
      (:5180+) "gauntlet_run"/"Gauntlet Run" -> a neutral name that KEEPS
      the sort-order property the tests rely on (display name must sort
      before "Shakedown Run": :5249 asserts name-sorted default
      selection; e.g. "Practice Run" works).
- [ ] Gate check: `grep -rn gauntlet --include='*.rs' .` outside
      `webmods/` returns nothing (given 20260716-155830 landed first;
      historical tasks/ records exempt).
- [ ] Verify: `cargo check --workspace --all-targets`, `cargo fmt`, run
      generate.rs, portal_install.rs, nova_mod_format tests and the
      nova_menu picker tests (crate-solo lesson: run nova_menu tests with
      a unifying sibling or workspace-wide).

## Notes

- Real-webmods coverage after this: LOAD gate = webmods_validation.rs
  (generic), PUBLISH gate = the generic every-dir-publishes assertion.
- Depends on: 20260716-155830 (deletes gauntlet_race.rs; the grep gate
  assumes it).
