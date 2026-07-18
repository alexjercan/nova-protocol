# Review: live LOW<->HIGH switching renders at stale resolution

- TASK: 20260718-140903
- BRANCH: fix/render-scale-switch

## Round 1

- VERDICT: APPROVE

Root cause pinned in bevy source (bevy_render `camera.rs`, `camera_system`): the
recompute condition is `normalized_target.is_changed(windows, images) ||
camera.is_added() || camera_projection.is_changed()`. It reacts to the target
CONTENT changing or the camera being added, but NOT to the `RenderTarget`
component being swapped in place - so a runtime target switch leaves
`computed.target_info` (physical size + scale factor) stale, and the camera
renders with the previous target's dimensions. This exactly explains the observed
inversion (switch to Low did nothing; switch back to High shrank the window) and
why fresh-start worked (the change coincided with `is_added()`).

Fix verified:

- **Reproduced first:** the `NOVA_SWITCH_QUALITY` example mode flips the preset
  mid-run; the pre-fix shots (`shots/switch-*.png`) show the inversion (Low stays
  crisp, High goes soft), and the post-fix shots (`shots/fixed-*.png`) show the
  correct result both ways (Low soft world + crisp HUD, High fully crisp).
- **Mechanism:** `projection.set_changed()` on every target switch (into the
  image and back to the window) forces the re-derive. A unit test
  (`every_target_switch_marks_the_camera_projection_changed`) asserts the
  projection change tick is bumped on each switch and NOT on a steady frame (so
  no per-frame recompute churn).
- **No regression:** all 6 render_scale tests pass; the crate compiles clean. The
  teardown target reset moved from a deferred `commands.insert` back to an
  immediate `&mut` write (needed so the reset and the projection touch land in the
  same frame); the only cost is the blit's stale sprite showing for at most one
  frame on the way out - a cosmetic 1-frame artifact, far better than the broken
  switch.

Not headlessly verifiable: the physical feel of dragging the settings slider; the
screenshots capture the post-switch frame, which is the thing that was wrong.

No BLOCKER/MAJOR/MINOR findings.
