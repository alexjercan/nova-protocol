# Review: Migrate base art under assets/base/, reference via self://+dep://base

- TASK: 20260717-002105
- BRANCH: feat/base-art-migration

Independent out-of-context completeness review (fresh eyes on the diff, hunting
for runtime 404s a moved-but-not-repointed reference would cause), plus a self
re-verification of the base `resources` list. The full test suite is green, but
the review specifically targeted the failure mode tests can miss: a reference
outside the crates/ test trees.

## Round 1

- VERDICT: REQUEST_CHANGES

- [x] R1.1 (BLOCKER) examples/data/reel.content.ron:17,39,66 - three bare base-art
  refs (`textures/cubemap.png`, `textures/asteroid.png` x2) were NOT repointed and
  404 at runtime. This file is EXAMPLE-OWNED data loaded by
  `examples/13_screenshot_reel.rs` via `include_str!` + `ron::de::from_str` +
  `LoadScenario` - directly through the default asset source, NOT the mod merge -
  so the `self://`/`dep://` content sweep (which scanned `assets/` + `webmods/`)
  and the `.rs` load-site grep both missed it. It is a real consumer:
  `scripts/gen-web-screenshots.py` runs the reel in the web-screenshot pipeline.
  Fix: direct `base/textures/...` paths (the reel is not a mod, so not
  `self://`/`dep://` - same default-source idiom as GameAssets' kept-root icons).
  - Response: Fixed - repointed the 3 refs to `base/textures/cubemap.png` /
    `base/textures/asteroid.png`. Confirmed `13_screenshot_reel.rs` triggers
    `LoadScenario` directly with no rewrite, so direct base-prefixed paths are
    correct. Verified `examples/data/reel.content.ron` is the ONLY content file
    under `examples/`, and every other bare-ref grep hit is `#[cfg(test)]`
    synthetic data.
- [ ] R1.2 (MINOR) [PRE-EXISTING, deferred] crates/nova_core/src/lib.rs:245 - the
  `meta_check` `Paths` set lists only `cubemap.png` (now `base/textures/cubemap.png`),
  not `cubemap_alt.png`, though `cubemap_alt.png.meta` carries the cube
  `RowCount(6)` layout and cubemap_alt IS a live skybox (broadside). This was
  already true on master (not a migration regression); flagged because the
  migration is when these paths get audited. Deferred to a follow-up, not changed
  here (adding it alters skybox rendering behavior and wants a visual check).
- [ ] R1.3 (MINOR) [out-of-scope, deferred] the author wiki
  (guide-author-scenario.md, guide-make-a-mod.md, modding-ron.md,
  guide-author-section.md) still teaches the OLD bare-ref pattern that this
  migration + the bare-ref lint (task 20260717-002133) retire. Prose only, no
  runtime impact. Owned by the docs task 20260717-002203.

## Round 2

- VERDICT: APPROVE

R1.1 (the only BLOCKER) is resolved - the reel now points at the moved files.
R1.2 and R1.3 are pre-existing / other-task concerns, correctly deferred (they
are not migration regressions).

Verified beyond the fix:
- **Content**: no bare base-art ref remains in `assets/base/**`, `assets/mods/**`,
  `webmods/**`, or `examples/data/`. Base uses `self://`, mods use `dep://base/`.
- **Base resources**: the 9 `self://` files base content references map 1:1 to the
  `resources` list, and all 9 exist on disk (no content-gate / portal error).
- **meta**: both `.meta` sidecars moved; meta_check repointed to `base/...`.
- **Deployment**: native packaging `cp -r assets/` and the web game serve the whole
  tree, so the intra-tree move is transparent; the website's own banner is separate.
- **icons/sounds**: no content references them; icons load only via kept-root
  GameAssets paths. Nothing broke.

Suites green: nova_assets (lib + all integration incl content_lint_gate,
content_ron_parity, webmods_validation, example_scenario, gauntlet_course,
broadside_assault, cubemap_meta, portal_install, mod_cache_install),
nova_portal_gen, skybox_swap_e2e, cubemap_meta_app_config.
