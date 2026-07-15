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
