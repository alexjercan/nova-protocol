# Unship screenshot-reel: embed the reel scenario in the example, drop it from assets/ and the catalog

- STATUS: OPEN
- PRIORITY: 18
- TAGS: modding,examples

User request (20260715, mid-flow on 142849): the reel mod should not live in the
mods folder or ship at all - bake it into the examples. The `hidden` catalog flag
STAYS as a feature ("just in case"), it just loses its only shipped user.

Goal: `examples/13_screenshot_reel.rs` stops using the mod pipeline. Move
`assets/mods/screenshot-reel/reel.content.ron` to an examples-owned location
(e.g. `examples/data/reel.content.ron`, versioned but never shipped - Trunk
copy-dirs only `assets/`); the example `include_str!`s it, parses
`Vec<Content>` (the same `nova_modding::Content` type the loader uses; ADD
`ron` to the root crate's dev-deps if not already available to examples), pulls
out the `ScenarioConfig`, and fires `LoadScenario` directly once assets are
Loaded - no catalog entry, no `EnabledMods`, no re-merge wait. Delete
`assets/mods/screenshot-reel/` (bundle.ron included - the reel is not a mod
anymore) and its catalog entry.

Consequences to handle:
- The hidden-flag tests (added by 142844/142849) use the real reel entry as
  their subject: `hidden_mod_still_merges_when_enabled_by_id`,
  `seed_enabled_mods_strips_restored_hidden_ids`, and the catalog test's
  "reel filtered" assertion. Re-point them at a test FIXTURE catalog + tiny
  bundle under `tests/fixtures/` (a second headless app helper with
  `AssetPlugin.file_path` at the fixtures root; not under assets/, ships
  nowhere). The hidden machinery stays fully pinned without a shipped user.
- The example currently exercises the LIVE re-merge path; that coverage
  remains in `toggling_enabled_mods_remerges_live` (note in close-out).
- `relocation-leaves-ignored-siblings` lesson: after landing, check the main
  checkout for leftover untracked files under assets/mods/screenshot-reel/.
- Sweep docs: modding-ron-format.md's hidden bullet cites screenshot-reel as
  the example; README under the reel folder; docs/development.md BCS_REEL
  line; CHANGELOG entry.

Rejected alternative: bevy `embedded://` asset source + catalog-external
bundle load - more machinery for zero benefit (the reel is one scenario, needs
no overlay semantics, and AssetRef paths resolve at spawn against the normal
asset server either way).

Depends on: 20260715-142849 (bundle meta - in flight; lands first so this task
rebases on the final shape).

## Plan (20260715)

Verified facts: the reel folder holds exactly reel.content.ron +
screenshot-reel.bundle.ron (no README); root Cargo.toml has NO `ron` dep
(nova_modding uses ron 0.12 - add `ron = "0.12"` to root [dev-dependencies];
examples compile against dev-deps; STAGE Cargo.lock with it); the example's
final act is already `commands.trigger(LoadScenario(scenario.clone()))`
(examples/13_screenshot_reel.rs:99), so bypassing the mod pipeline only changes
WHERE the ScenarioConfig comes from; reel content references assets by AssetRef
PATHS resolved at spawn, so a parsed-from-string config loads textures fine
under `cargo run --example`.

Test strategy for the orphaned hidden-flag tests: SYNTHETIC catalog, no fixture
file tree. All `InstalledCatalog`/`CatalogEntry`/`BundleAsset` fields are pub,
so tests build an `InstalledCatalog` asset in memory: real base+demo entries
from the loaded real catalog PLUS a synthetic `hidden: true` decl whose bundle
handle REUSES the loaded demo bundle - real loaders, real content, zero new
files. The flag's decode path stays pinned by nova_modding's RON unit test;
flag SEMANTICS (filter/strip/merge-when-enabled) are IO-independent.

Steps:
- [ ] 1. `git mv assets/mods/screenshot-reel/reel.content.ron
  examples/data/reel.content.ron`; delete
  assets/mods/screenshot-reel/screenshot-reel.bundle.ron (the reel is not a mod
  anymore) and the folder; drop the screenshot-reel entry from
  assets/mods.catalog.ron.
- [ ] 2. Root Cargo.toml: add `ron = "0.12"` to [dev-dependencies] (+ lockfile).
- [ ] 3. examples/13_screenshot_reel.rs: replace enable_reel_mod +
  the GameScenarios poll with: `include_str!("data/reel.content.ron")` parsed
  once via `ron::de::from_str::<Vec<nova_modding::prelude::Content>>` (expect
  with a clear message - a broken reel file should fail loud in this dev tool),
  extract the `Content::Scenario` config, and trigger
  `LoadScenario(config)` at `OnEnter(GameAssetsStates::Loaded)` (keep the
  ReelLoaded once-guard). Drop MOD_ID/EnabledMods usage; update the module doc
  (the reel is example-owned data, not a mod; the mod pipeline's live re-merge
  coverage lives in `toggling_enabled_mods_remerges_live`).
- [ ] 4. crates/nova_assets/tests/demo_scenario.rs: rework the three
  reel-dependent tests to the synthetic-catalog rig - a helper that loads the
  real catalog, then builds a modified `InstalledCatalog` asset (base + demo +
  synthetic `hidden-fixture` decl reusing the DEMO bundle handle) and points
  `GameAssets.catalog` at it. (a) build_mod_catalog filters the hidden decl
  (list = base + demo, no "hidden-fixture"); (b) seed strips a restored
  "hidden-fixture" id while keeping visible choices + base; (c) register_bundles
  with enabled = {base, hidden-fixture} merges the demo content
  (demo_mod_arena registers - hidden != disabled through the production merge).
  Real-catalog test drops its reel assertions (catalog = base + demo, len 2
  unchanged).
- [ ] 5. Sweep docs + changelog: modding-ron-format.md hidden bullet (reel no
  longer the shipped example - make it generic "dev/tooling mods");
  CHANGELOG: amend the 142844 Unreleased entry (reel is no longer the shipped
  first user) and add an entry (the reel capture set no longer ships in game
  assets at all; it is example-embedded). docs/scenario-system.md:100 and
  docs/development.md:130 stay valid (the example still exists and drives
  framed shots).
- [ ] 6. Verify: `cargo fmt --check`; `cargo check --workspace --all-targets`;
  `cargo test -p nova_modding`, `-p nova_assets --test demo_scenario`,
  `-p nova_menu`; then prove the example still boots its scene: timeout-run
  `cargo run --example 13_screenshot_reel` (no debug feature; per its doc a
  plain run boots the scene) and grep stderr for the "loading scenario" log
  (run-example-via-cargo-run-for-assets lesson: from the crate root, 2>&1).
- [ ] 7. After landing (main checkout): check for leftover untracked files
  under assets/mods/screenshot-reel/ (relocation-leaves-ignored-siblings) -
  expected none (folder had only tracked files).

## Notes (plan)

- Relevant files: examples/13_screenshot_reel.rs (:60-102 rework zone),
  assets/mods.catalog.ron, assets/mods/screenshot-reel/*,
  crates/nova_assets/tests/demo_scenario.rs (reel-dependent tests + helpers),
  Cargo.toml (+lock), CHANGELOG.md, docs/modding-ron-format.md:129.
- The 15-18 screenshot examples do not reference the reel mod (grepped:
  MOD_ID/screenshot-reel only in example 13).
- Reel meta authored in 142849 dies with the bundle file - expected, noted in
  that task's close-out.
