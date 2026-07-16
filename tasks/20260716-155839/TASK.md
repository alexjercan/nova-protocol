# Decouple portal/publish tests from specific mods

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: v0.7.0,testing,refactor

## Goal

No core test names a specific mod. Mods must be renamable/removable
without touching core CI. Decision from audit task 20260716-141620
(FINDINGS.md section 2), user confirmed. Smaller than it looks: both
portal test files already have synthetic fixture builders; only the two
headline e2e tests and some borrowed fixture names use real mods.

## Steps

- [x] `crates/nova_portal_gen/tests/generate.rs`: in
      `real_webmods_publish_and_hashes_verify` (:19), replace the
      gauntlet-specific assertion (:32) with the generic contract - every
      subdirectory of `webmods/` appears as an entry id in the generated
      catalog (and no extras). The synthetic-fixture tests (:110+ already
      exist) stay as-is.
- [x] `crates/nova_assets/tests/portal_install.rs`: in
      `portal_fetch_install_enable_uninstall_over_the_wire` (:291), swap
      `gauntlet_files()` (:242, reads webmods/gauntlet) for a synthetic
      fixture mod built like `mock_files()` (:477) but including a
      scenario content file, so the enable step can still assert a
      scenario id registers in GameScenarios. Drop the gauntlet-specific
      assertions (meta name, bundle path, `gauntlet_run` registration) in
      favor of the fixture's own ids. `serve_portal_tree` (:259) serves
      whatever tree it is given - the fixture slots in.
- [x] Rename borrowed fixture ids to neutral names:
      `crates/nova_mod_format/src/lib.rs` tests (:256-284) "gauntlet" ->
      e.g. "fixture_pack"; `crates/nova_menu/src/lib.rs` picker tests
      (:5180+) "gauntlet_run"/"Gauntlet Run" -> a neutral name that KEEPS
      the sort-order property the tests rely on (display name must sort
      before "Shakedown Run": :5249 asserts name-sorted default
      selection; e.g. "Practice Run" works).
- [x] DISCOVERED during work: `crates/nova_assets/tests/mod_cache_install.rs`
      ALSO read `webmods/gauntlet` (its own gauntlet_files at :56) - the
      plan's sweep was head-truncated (same lesson as 155816). Swapped to
      the same in-memory fixture mod (no disk source needed;
      `install_local` takes bytes directly), all ids/meta swapped with
      count-asserted scripted edits.
- [x] Also: neutral fixture names in `crates/nova_assets/src/mod_cache.rs`
      (doc example `my_mod.bundle.ron`, unit fixture `pack_a`).
- [x] Gate check: `grep -rni gauntlet --include='*.rs' .` outside
      `webmods/` returns nothing (given 20260716-155830 landed first;
      historical tasks/ records exempt).
- [x] Verify: `cargo check --workspace --all-targets`, `cargo fmt`, run
      generate.rs, portal_install.rs, nova_mod_format tests and the
      nova_menu picker tests (crate-solo lesson: run nova_menu tests with
      a unifying sibling or workspace-wide).

## Notes

- Real-webmods coverage after this: LOAD gate = webmods_validation.rs
  (generic), PUBLISH gate = the generic every-dir-publishes assertion.
- Depends on: 20260716-155830 (deletes gauntlet_race.rs; the grep gate
  assumes it).

## Close notes (2026-07-16)

What changed: the generate.rs real-webmods test now asserts the generic
contract (every webmods/ subdirectory publishes, nothing else - by
directory listing, no mod named, with a non-empty delivery guard); the
portal_install e2e and ALL of mod_cache_install run on a synthetic
fixture mod ("fixture-slalom", in-memory bytes; portal_install writes it
to a temp source dir because the generator scans a directory); fixture
ids in nova_mod_format and nova_menu picker tests renamed (fixture_pack;
practice_run/"Practice Run", which keeps the name-sort property the
picker tests lean on); neutral examples in mod_cache.rs. Gate:
`grep -ri gauntlet` over crates/src/examples is EMPTY outside webmods/.

Discoveries:

1. mod_cache_install.rs also read webmods/gauntlet - missed by the
   audit's head-truncated grep (second instance of
   truncated-sweep-is-not-a-sweep, this one survived into a plan).
2. INHERITED MASTER RED: mod_cache_install.rs carried two
   `contains_key("demo")` base-merge guards asserting the base demo
   scenario, which task 20260716-155816 removed - so this test has been
   failing on master since 564ff12d landed. 155816's sweep grep was
   ALSO head-truncated; the same truncation hid both. Fixed here
   (guards now pin shakedown_run); a repo-wide
   `grep -rn 'contains_key("demo")'` confirms zero remain.
3. The portal generator's id gate rejects underscores in MOD ids
   (lowercase/digits/'-' only; scenario ids are a different namespace) -
   the fixture is "fixture-slalom" with scenario "fixture_slalom_run".

Verification: portal_install 9/9, mod_cache_install 7/7,
nova_portal_gen 12/12, nova_mod_format 9/9, nova_menu 46/46 (full),
check --all-targets zero errors/unused-warnings, fmt clean. Full suite
is CI's job per the standing instruction.

Reflection: two of three discoveries trace to the SAME root cause
(head-truncated sweeps in the audit/plan phase) - the lesson from 155816
was written mid-flow but earlier truncations had already poisoned two
downstream plans. When a lesson lands mid-flow, re-audit the REMAINING
queue against it, not just future work.
