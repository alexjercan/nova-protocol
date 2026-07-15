# Hidden dev mods: catalog hidden flag keeps screenshot-reel out of the Mods menu

- STATUS: CLOSED
- PRIORITY: 20
- TAGS: modding,menu

Spike: tasks/20260714-202515/SPIKE.md (option W)

Goal: dev/tooling mods must not appear in the player-facing Mods menu. Add a
serde-default `hidden: bool` field to the shipped catalog's `ModEntry`
(nova_modding), set `hidden: true` on the `screenshot-reel` entry in
`assets/mods.catalog.ron`, and filter hidden entries out of `ModCatalog` (the
menu's view), so nova_menu needs no change. The mod still ships, still loads at
startup, and `examples/13_screenshot_reel.rs` keeps enabling it by id via
`EnabledMods` unchanged. Update the modding docs to mention the flag.

## Plan (20260715)

Filter location decided: `build_mod_catalog` (crates/nova_assets/src/lib.rs:132) -
`ModCatalog` is documented as the menu-facing view, so hidden entries simply never
reach the menu; nova_menu code stays untouched. Hidden is NOT disabled: the entry
stays in `InstalledCatalog`, its bundle loads, and `register_bundles` merges it
whenever its id is in `EnabledMods` (the example's path, verified:
examples/13_screenshot_reel.rs `enable_reel_mod` mutates `EnabledMods` directly).

CONTEXT: master CI is RED right now - `mod_catalog_lists_installed_mods_metadata`
(crates/nova_assets/tests/demo_scenario.rs:130) asserts 2 catalog entries but
92aaf8da shipped a third (screenshot-reel) without updating it (CI run on 92aaf8da:
left: 3, right: 2). This task makes the assertion's intent true again (ModCatalog =
base + demo) and turns master green.

Steps:
- [x] 1. nova_modding (crates/nova_modding/src/lib.rs:258 `ModEntry`): add
  `#[serde(default)] pub hidden: bool` with a doc comment (dev/tooling mods the
  player-facing list omits; still installed, loadable, enableable by id). Extend the
  `catalog_manifest_ron_decodes` unit test (lib.rs:463): a third entry with
  `hidden: true` decodes, and `hidden` defaults to false when omitted.
- [x] 2. assets/mods.catalog.ron: `hidden: true` on the screenshot-reel entry; add
  the flag to the header comment documenting entry fields.
- [x] 3. nova_assets `build_mod_catalog` (crates/nova_assets/src/lib.rs:132): keep
  only `!e.meta.hidden` entries; update its doc comment + the `ModCatalog` doc
  (player-facing view, hidden entries filtered).
- [x] 4. Tests (crates/nova_assets/tests/demo_scenario.rs): fix
  `mod_catalog_lists_installed_mods_metadata` against the real 3-entry catalog -
  ModCatalog is exactly [base, demo] and asserts "screenshot-reel" is ABSENT (the
  filter's boundary pin; fails on pre-change code). Add a merge-path assertion that
  hidden != disabled: `EnabledMods = {base, screenshot-reel}` merges the reel
  scenario (`screenshot_reel` in GameScenarios), pinning the example's contract
  through the production register_bundles path.
- [x] 5. Fix `ModEntry` literal initializers broken by the new field: nova_menu
  tests (crates/nova_menu/src/lib.rs:1415, :1422). Catch stragglers with
  `cargo check --workspace --all-targets` (examples/tests included, per
  check-all-targets-for-struct-field).
- [x] 6. Docs: docs/modding-ron-format.md catalog section (line ~106) gains the
  `hidden` field; CHANGELOG if the repo's per-task practice applies.
- [x] 7. Verify: `cargo fmt --check`; `cargo check --workspace --all-targets`;
  run the touched test targets only (`cargo test -p nova_modding`,
  `cargo test -p nova_assets --test demo_scenario`, `cargo test -p nova_menu`) -
  full suite stays on CI per standing instruction.

## Notes

- Relevant files: crates/nova_modding/src/lib.rs (ModEntry:258, decode test:463),
  crates/nova_assets/src/lib.rs (ModCatalog:128, build_mod_catalog:132),
  crates/nova_assets/tests/demo_scenario.rs:108-139, assets/mods.catalog.ron,
  examples/13_screenshot_reel.rs (contract: enables by id, no menu),
  docs/modding-ron-format.md:99-115.
- ModEntry has no Default derive; the only literal constructors outside
  nova_modding are the two nova_menu test sites (grepped).
- Pre-existing red CI on master (92aaf8da) is fixed by this task's step 4.

## Close-out (20260715)

What changed: `ModEntry` gained a serde-default `hidden: bool` (nova_modding);
the screenshot-reel entry in `assets/mods.catalog.ron` is `hidden: true`;
`build_mod_catalog` filters hidden entries out of the player-facing `ModCatalog`,
so nova_menu needed no production change (only its two test `ModEntry` literals
gained the field). Docs: modding-ron-format.md documents the flag; CHANGELOG entry
under Unreleased. Alternatives (separate dev catalog, cfg-gating, example
sideloading) were weighed in the spike (option W won: one field, zero example
churn).

Filter location rationale: `ModCatalog` is documented as the menu's view, so
filtering at its construction keeps the policy in one place and out of the UI;
`register_bundles` reads the full `InstalledCatalog`, so hidden stays orthogonal
to enabled.

Evidence: fmt clean; `cargo check --workspace --all-targets` clean;
`cargo test -p nova_modding` 3 passed; `-p nova_assets --test demo_scenario`
8 passed (incl. the new `hidden_mod_still_merges_when_enabled_by_id` and the
fixed `mod_catalog_lists_installed_mods_metadata`); `-p nova_menu` 13 passed.
Full suite deliberately left to CI (standing instruction). The boundary pin's
fail-first proof is master's own CI on 92aaf8da: the same assertion fails
`left: 3, right: 2` on pre-change code - no local sabotage needed.

Also fixes red master CI (stale 2-entry assertion after 92aaf8da shipped the
third catalog entry without updating the test).

Review round 1 (out-of-context pass): APPROVE with one MINOR (R1.1) - example
runs persist `screenshot-reel` into the prefs store, and post-hide there is no
menu row to disable a restored hidden mod. Fixed on-branch: `seed_enabled_mods`
now reconciles (unions base in, strips hidden non-base ids out), making hidden
enablement session-only; prefs self-heal on the next save; the example is
unaffected (re-enables at `OnEnter(Loaded)`, after the chain). New rig case
`seed_enabled_mods_strips_restored_hidden_ids` pins it (9 demo_scenario tests
green).

Reflection: the plan survived contact unchanged; the one discovery worth keeping
is that the fail-first evidence for a test fix can come from CI history instead
of a local sabotage run when master is already red on the exact assertion. The
persistence interaction (R1.1) was findable at plan time by asking "who else
writes EnabledMods" - the out-of-context pass earned its keep again.

