# Editor skybox may miss its Cube view: direct SkyboxConfig insert bypasses the swap applier

- STATUS: CLOSED
- PRIORITY: 30
- TAGS: v0.7.0,editor,rendering,bug

## Resolution (2026-07-17): FALSIFIED - the editor is already covered

The suspected bug does not exist. The editor's direct `SkyboxConfig` insert is
safe because `game_assets.cubemap` already has its Cube texture view by the time
the editor camera spawns:

- `nova_assets::prepare_cubemap_view` (crates/nova_assets/src/lib.rs:1057) runs
  in `OnEnter(GameAssetsStates::Processing)` and sets the Cube view on
  `game_assets.cubemap` (the base `cubemap.png`) when it is 6-layer. Its own doc
  says the point is to prepare the view "before anything spawns a camera" so
  `SkyboxPlugin` "just attaches the `Skybox` component".
- State ordering is strict: `Loading -> Processing (prepare runs) -> Loaded`,
  then `GameStates::Playing -> ExampleStates::Editor -> setup_editor_scene`
  (nova_editor/src/lib.rs:108). So the view is set before the editor's
  `SkyboxConfig` insert.
- The editor uses the SAME handle (`game_assets.cubemap`) that
  `prepare_cubemap_view` prepared, so the bcs observer sees a ready 6-layer +
  Cube image and skips its (view-setting) single-layer branch harmlessly.

Review R1 of 20260717-111558 flagged this as a possible gap because it reasoned
only from the bcs observer + `apply_pending_skybox_swaps`, missing the startup
`prepare_cubemap_view` that covers the direct-insert path. No code fix needed.

Delivered instead (evidence rig + non-behavior pin):

- `prepare_cubemap_view_sets_cube_view_on_the_game_assets_cubemap`
  (crates/nova_assets/src/lib.rs tests): pins that an arrayed cubemap gets its
  Cube view and a single-layer one is left for the fallback - so the editor's
  coverage cannot silently regress.
- A comment at the editor's `SkyboxConfig` insert (nova_editor/src/ui/mod.rs)
  explaining why the direct insert is safe, pointing at `prepare_cubemap_view`,
  so this is not re-filed.

## Context (spun out of review R1 on 20260717-111558, 2026-07-17)

`nova_editor/src/ui/mod.rs:110` inserts a `SkyboxConfig` DIRECTLY on the WASD
camera (not through `nova_scenario::apply_pending_skybox_swaps`). The bcs
`SkyboxPlugin` observer only sets the `Cube` texture view on its single-layer
fallback branch; a cubemap that arrives ALREADY 6-layer (its `.meta`
`array_layout` applied - the base cubemap has been meta-applied since the
`Paths` set of task 20260717-013440, and now universally under
`AssetMetaCheck::Always`, task 20260717-111558) SKIPS that branch. Only the swap
applier sets the view for the 6-layer case, and the editor does not go through
it.

So the editor's skybox may be silently missing its `Cube` view - bevy's
`sanity_check_skybox_image_and_warn` would `warn_once` and withhold the bind
group, dropping the sky. This is UNCONFIRMED (needs a GPU run of the editor);
it is pre-existing and was NOT introduced by the `Always` switch.

## Steps

- [ ] Reproduce: run the editor (`ExampleStates::Editor`) and confirm whether
      the skybox renders; check the log for the bevy skybox sanity `warn_once`.
- [ ] If broken: set the `Cube` `texture_view_descriptor` for the editor's
      cubemap too. Prefer a shared helper over duplicating the applier logic -
      e.g. factor the "6-layer arrival needs a Cube view" write out of
      `apply_pending_skybox_swaps` and reuse it, or route the editor through the
      same pending-swap path.
- [ ] Add a regression test if practical (headless view-descriptor assertion in
      the style of `apply_pending_skybox_swaps`' tests).

## Notes

- Evidence trace: tasks/20260717-111558/REVIEW.md (Round 1, Out of scope).
