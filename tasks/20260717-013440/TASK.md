# cubemap_alt.png.meta not in meta_check Paths (broadside skybox may miss its cube layout)

- STATUS: OPEN
- PRIORITY: 25
- TAGS: v0.7.0,assets,bug

## Context (surfaced during the base-art migration review, 2026-07-17)

`crates/nova_core/src/lib.rs` `assets_plugin()` uses `AssetMetaCheck::Paths(...)`
listing ONLY `base/textures/cubemap.png` (was `textures/cubemap.png` before the
Option A move, task 20260717-002105). But `base/textures/cubemap_alt.png` ALSO
has a `.meta` sidecar with `array_layout: Some(RowCount(rows: 6))` - the 6-face
cube-skybox reinterpret - and cubemap_alt IS a live skybox (broadside scenario,
and the gauntlet/ledger mods via `dep://base/textures/cubemap_alt.png`). Under
`AssetMetaCheck::Paths`, a path NOT in the set has its `.meta` ignored, so
cubemap_alt would load as a flat 2D image, not a cube.

This is PRE-EXISTING (master listed only `cubemap.png` before the move too), not a
migration regression - but the migration is when it surfaced, and cubemap_alt now
has more consumers (the mods that dep://base it).

## Goal

Decide whether cubemap_alt needs its `.meta` honored (does the broadside/gauntlet
skybox currently render as a cube or a flat image?), and if so add
`base/textures/cubemap_alt.png` to the `meta_check` Paths set. Verify visually
(the skybox actually reads as a cube) - no test currently covers skybox visual
correctness, so a manual run or a screenshot check is needed.

## Notes

- File: crates/nova_core/src/lib.rs (`assets_plugin`, the `AssetMetaCheck::Paths`
  set).
- If cubemap_alt has been rendering flat all along, this is a latent visual bug;
  if the skybox system applies the cube layout another way, this may be a no-op -
  investigate before changing (changing meta_check alters rendering).
