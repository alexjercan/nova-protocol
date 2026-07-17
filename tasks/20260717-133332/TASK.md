# Editor skybox may miss its Cube view: direct SkyboxConfig insert bypasses the swap applier

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.7.0,editor,rendering,bug

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
