# Review: cubemap_alt.png.meta not in meta_check Paths

- TASK: 20260717-013440
- BRANCH: meta-check-cubemap-alt

## Round 1

- VERDICT: REQUEST_CHANGES

Reviewed as a diff against master (7f1da64c) plus an out-of-context review
pass (fresh-context subagent re-deriving the load-bearing claims from bevy
0.19 / bevy-common-systems / bevy_asset source). The CODE is correct,
necessary, well-tested, and delivers the Goal - the coverage sweep found no
SkyboxConfig insert site broken by the meta change, the tests are non-vacuous
(mutation-traced: each new assertion fails with the fix deleted, matching the
recorded sabotage A/B), the CHANGELOG line is accurate, and the diff is
ASCII-clean. The findings are all in the documentation/narrative layer, but
R1.1 is a repeated false failure-mode claim that would mislead future
sessions, so it blocks.

- [x] R1.1 (MAJOR) crates/nova_scenario/src/actions.rs:349 (fn doc), the doc
  above `skybox_swap_sets_cube_view_on_a_preinterpreted_cubemap`,
  crates/nova_scenario/tests/skybox_swap_e2e.rs:190,
  tasks/20260717-013440/TASK.md (Investigation "hard render error", close-out
  "fatal wgpu validation error", visual-verify "would have crashed"),
  tasks/20260717-111558/TASK.md ("a D2Array view would hit the Cube binding") -
  the claimed failure mode for a missing Cube view is WRONG. bevy 0.19's
  `prepare_skybox_bind_groups` gates on `sanity_check_skybox_image_and_warn`
  (bevy_core_pipeline-0.19.0/src/skybox/mod.rs:239,261-280), which requires
  `texture_view_descriptor.dimension == Some(Cube)` and otherwise `warn_once!`s
  and removes the SkyboxBindGroup - "we ignore the skybox so as not to break
  rendering". No wgpu validation error, no crash: the sky silently disappears.
  The fix stays necessary (an invisible sky is a real regression), but reword
  all five sites to the warn-and-skip failure mode, and correct the TASK.md
  visual-verify inference ("zero wgpu errors proves the view fix" is invalid -
  the log would only show a warn_once; the rendered-cube screenshot plus the
  e2e view assertion are the actual evidence; state whether the warn_once line
  is absent from the example-19 log).
  - Response: fixed in b5c782d7 - all five sites reworded to the warn_once-and-skip failure mode; TASK.md's visual-verify inference replaced with the real evidence (the warn_once line 'must be TextureViewDimension::Cube' greps to 0 matches in the example-19 log, and the brightened screenshot shows the sky rendering, which the sanity check would have removed entirely); Investigation/close-out/difficulties updated and marked as R1.1 corrections.
- [x] R1.2 (MINOR) crates/nova_scenario/src/actions.rs:352 (fn doc "get_mut
  emits AssetEvent::Modified") and tasks/20260717-111558/TASK.md (the bcs
  "unconditional get_mut re-uploads on every insert" item) - bevy_asset 0.19's
  `Assets::get_mut` returns an `AssetMut` guard that queues Modified only on
  actual mutable deref (assets.rs:627,668-690). The bcs observer's skip path
  for already-arrayed images only READS through the guard, so it queues no
  Modified and re-uploads nothing: drop that item from the follow-up task and
  fix the actions.rs wording (the write emits Modified, not the get_mut call).
  Also soften the TASK.md claim that the no-churn test "guards the
  get_mut-only-when-needed shape" - it pins "no unconditional descriptor
  overwrite", which is the part that matters.
  - Response: fixed in b5c782d7 - actions.rs doc now attributes Modified to the write through the AssetMut guard (kept the explicit look-then-borrow shape); the follow-up task drops the re-upload item with the refutation recorded inline; TASK.md now says the pin covers 'no unconditional descriptor overwrite'.
- [x] R1.3 (MINOR) crates/nova_scenario/src/actions.rs
  (`skybox_swap_does_not_remodify_an_already_cubed_image`) - the test asserts
  only the ABSENCE of Modified; if a future rig/schedule change stopped
  AssetEvents reaching `Messages<AssetEvent<Image>>`, it would go vacuously
  green. Add a delivery guard: assert the drained events CONTAIN
  `AssetEvent::Added` for the cubemap id (the `.add()` must have produced it).
  - Response: fixed in b5c782d7 - the churn test now asserts the drained buffer CONTAINS Added for the cubemap id before asserting no Modified; 3/3 lib skybox tests green after the change.
- [x] R1.4 (NIT) crates/nova_core/src/lib.rs:235 - the doc comment (reworked
  by this branch) still points at `docs/retros/20260710-skybox-cubemap-upload-race.md`,
  which does not exist; the record lives at `tasks/20260710-143138/NOTES.md`
  (its line 16 carries the historical "Dimension Y value 24576 exceeds the
  limit of 16384" error). Fix the pointer while touching this comment.
  - Response: fixed in b5c782d7 - pointer now tasks/20260710-143138/NOTES.md (verified: its line 16 carries the historical 24576-vs-16384 error text).

Verified and standing (no action needed): the bcs observer couples
reinterpret+view-set in its single-layer-only branch (bevy-common-systems
f292222 src/camera/skybox.rs:117-130); without Fix 2 the swapped sky would
not render (GpuImage falls back to a D2Array default view,
bevy_render-0.19.0/src/texture/gpu_image.rs:149-164, which the sanity check
refuses); `AssetMetaCheck` defaults to Always so the e2e's meta assumption
holds (bevy_asset-0.19.0/src/lib.rs:318); the Paths entry matches both the
tests' and the game's runtime path (`self://` rewrite, nova_modding
src/lib.rs:114); the applier registration (Update, run_if(scenario_is_live))
covers every production swap path including menu backdrops (empirically: the
example-19 run installed the menu skybox at stage 0); the editor path uses the
preloaded cubemap prepared at Processing and is unaffected; the app-config
test exercises the exact shipped config and recorded its fail-first.

## Round 2

- VERDICT: APPROVE

All four Round 1 findings verified resolved in b5c782d7:

- R1.1: grep over crates/ + both task files finds no crash claim for the
  missing-view case; every remaining "fatal wgpu validation error" is the
  UPLOAD-limit mechanism (24576 px vs max_texture_dimension_2d 16384), which
  is genuine (matches the historical error text in
  tasks/20260710-143138/NOTES.md:16). The TASK.md visual-verify step now
  cites the absent warn_once line + the rendered sky + the e2e assertion as
  the view evidence. Ticked.
- R1.2: the applier doc attributes Modified to the write through the AssetMut
  guard; the follow-up task records the refutation inline and keeps only the
  real upstream item. Ticked.
- R1.3: the Added-presence delivery guard is in the churn test
  (actions.rs:1410-1415); 3/3 lib skybox tests green after the change.
  Ticked.
- R1.4: the pointer resolves to a file that exists and carries the record.
  Ticked.

No new findings: the round's diff is doc/test-comment text plus the one
delivery-guard assertion, fmt-clean, ASCII-clean, and the affected tests were
re-run green. The branch delivers the Goal; approved.
