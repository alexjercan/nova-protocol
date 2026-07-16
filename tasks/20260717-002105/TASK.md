# Migrate base art under assets/base/ and repoint every reference (Option A)

- STATUS: OPEN
- PRIORITY: 49
- TAGS: v0.7.0,modding,base,assets,feature

## Goal

Physically move base game art UNDER `assets/base/` and repoint every reference so
base uses `self://` and mods use `dep://base/<path>` - the migration half of
Option A. This is ONE ATOMIC task: moving the art breaks every bare ref at once,
so the move and all repoints must land together and keep the game green.

## Scope of the move (user decision 2026-07-16)

- MOVE under `assets/base/`: `gltf/` (5 meshes), `textures/` (incl `.meta`
  sidecars), `banner.png` (used as a scenario thumbnail).
- KEEP at root: `icons/` (game UI, loaded by GameAssets, not mod content),
  `sounds/` (no mod-sound support yet - see follow-up task), `shaders/` (engine),
  `mods/`, `mods.catalog.ron`.

## Steps

- [x] `git mv` `assets/gltf/` -> `assets/base/gltf/`, `assets/textures/` ->
      `assets/base/textures/` (with `.meta` sidecars), `assets/banner.png` ->
      `assets/base/banner.png`. `rm -rf` any emptied root dirs (LESSONS:
      git-mv-leaves-empty-parent).
- [x] Update the ~9 hardcoded `GameAssets` `#[asset(path=...)]` in
      crates/nova_assets/src/lib.rs to `base/...` (these load via the default
      source, not the merge).
- [x] Update the `meta_check` path list in `assets_plugin()` (nova_core) for the
      moved `.meta` (cubemap etc.).
- [x] gen_content emits `self://`: update `SectionMeshRefs::from_paths`
      (sections.rs), the `scenario_generation` path constants (lib.rs), the
      broadside thumbnail (`banner.png`) and any other base builder to emit
      `self://gltf/...` / `self://textures/...` / `self://banner.png`, then
      regenerate every `assets/base/**/*.content.ron`.
- [x] `assets/base/base.bundle.ron` declares a `resources` list of the moved art
      (emitted by gen_content so it cannot drift from the builders).
- [x] Migrate every mod's bare base-art ref to `dep://base/<path>`:
      assets/mods/example, webmods/gauntlet, webmods/the-ledger (4 chapters).
- [x] Verify: the game loads; base scenarios resolve art from `base/`; a mod
      scenario resolves base art via `dep://base`; the `content_ron_parity` and
      `content_lint_gate` tests pass; check nothing else at root is referenced by
      content bare (e.g. base sound refs - if any content references `sounds/`,
      it stays root/bare for now and must be exempt until the sounds follow-up).
- [x] Sweep installed/asset-count and path assertions across the test suite
      (full untruncated sweep first - LESSONS: truncated-sweep-is-not-a-sweep).

## Notes

- Depends on task 20260717-000416 (dep://base mechanism).
- Spike: tasks/20260716-235458/SPIKE.md (Option A).
- After this, bare asset refs in content resolve to nothing (fail to load); the
  author-time lint that rejects them is task 20260717-002133.
