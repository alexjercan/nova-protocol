# Mod-shipped skybox cubemaps bypass load-time meta: fallback reinterpret keeps the teardown upload race

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.7.0,assets,modding,bug

## Decision (2026-07-17): global `AssetMetaCheck::Always`

Picked the "Idea" direction (global `Always`) over the per-source / bcs
options, after verifying the load-bearing objection against the pinned bevy
source rather than the old doc comment's assertion:

- `Always` sets `read_meta = true` for every asset
  (`bevy_asset-0.19.0/src/server/mod.rs:1564`); on wasm the reader `fetch()`es
  `<path>.meta` (`io/wasm.rs:122`), a missing sidecar returns HTTP 404
  (`io/wasm.rs:100`), and bevy falls back to `default_meta()`
  (`server/mod.rs:1616`). So the 404 the old comment warned about is REAL but
  NON-FATAL: extra web requests + console noise, negligible on native. User
  chose to accept that cost for closing the class with zero per-path
  bookkeeping.
- `Paths` cannot list dynamic `self://`/`mods://` paths; bevy has no per-source
  `meta_check` and no predicate variant; a nova-side reinterpret would leave the
  teardown race open for any cubemap reaching `Assets` single-layer. `Always`
  makes the cube arrive 6-layer from the loader, so the single-layer form never
  exists - the base fix's mechanism, now for every source.

Implementation + verification: see `docs/design/mod-skybox-meta-always.md`.
Pre-existing out-of-scope note captured there: `nova_editor` inserts
`SkyboxConfig` directly and may miss its `Cube` view (not regressed by this
change).

## Context (split out of 20260717-013440, 2026-07-17)

Task 20260717-013440 fixed the BASE alt cubemap by adding its path to
`assets_plugin()`'s `AssetMetaCheck::Paths` set and setting the Cube view in
`apply_pending_skybox_swaps`. That per-path opt-in cannot cover MOD skyboxes:

- A mod's own skybox (`self://textures/nebula.png` -> `mods/example/...` for
  shipped mods, `mods://<id>/...` for downloaded ones) is a dynamic path; the
  static Paths set never lists it, so its shipped `.meta` sidecar (which
  `web/src/wiki/dev/guide-make-a-mod.md` tells authors to include, and
  `mod_binary_resources.rs` asserts ships) is silently IGNORED.
- Those cubemaps therefore load as raw stacked single-layer images and rely on
  the bcs `SkyboxPlugin` observer's fallback reinterpret (bevy-common-systems
  v0.19.0 `src/camera/skybox.rs`). Normal path is safe (reinterpret runs the
  same frame the image lands, before extraction), but a scenario teardown
  during the PNG decode leaves the stacked image to upload as-is - fatal wgpu
  validation error on GPUs with max_texture_dimension_2d = 16384 (WebGL2-class,
  llvmpipe). Exactly the class 20260717-013440 closed for base.

## Goal

Close the class for mod cubemaps instead of per-path opt-ins. Candidate
directions (weigh in the task, pick one):

- Upstream bevy-common-systems fix: the `setup_skybox_camera` observer also
  needs `texture_view_descriptor` set when the image arrives ALREADY 6-layer
  (meta applied) - today that branch is skipped, and bevy's skybox sanity
  check refuses the resulting non-Cube view (warn_once) and skips rendering:
  the sky silently disappears (bevy_core_pipeline 0.19 skybox/mod.rs:261).
  Needs a bcs release + tag bump across the workspace. (An earlier version of
  this task also flagged the observer's unconditional `images.get_mut()` as a
  per-insert re-upload; review R1.2 of 20260717-013440 refuted that - the
  `AssetMut` guard queues `AssetEvent::Modified` only on an actual write, and
  the observer's skip path only reads. No churn work item there.)
- Or a nova-side meta strategy: bevy's `AssetMetaCheck` has no predicate
  variant, so honoring dynamic mod paths means either registering mod paths
  into the set at mod-install time (the set is fixed at App build - not
  possible today), switching to `Always` for the mods source only (per-source
  meta_check does not exist upstream), or teaching the mod cache to apply
  `array_layout` itself when it installs a cubemap.
- Either way: make the guide-make-a-mod.md wording truthful about when the
  sidecar is honored.

## Notes

- The teardown race window is real but narrow; severity is web-weighted
  (WebGL2 16384 limit) and only hits mods that ship their OWN skybox
  (gauntlet/ledger use `dep://base/...` and are covered by the base fix).
- Evidence and mechanism trace: tasks/20260717-013440/TASK.md (Investigation
  section).

## Idea

- is using `Always` a bad idea? If not then we can just set it like that and
  the fix from 20260717-013440 is no longer needed, right? Like we would set
  `Always` for all assets, not only for mods; that way bevy checks for `.meta`
  files by itself. Validate that this would work;
