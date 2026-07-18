# Review: Render-scale Low breaks UI clicks

- TASK: 20260718-132638
- BRANCH: fix/render-scale-clicks

## Round 1

- VERDICT: APPROVE

Focused fix (UI-camera routing + projection), reviewed against the bevy source
that defines the failure and the constraints.

Root cause independently confirmed in bevy source: `bevy_ui`'s `ui_focus_system`
(`focus.rs:193`) resolves a cursor only for `NormalizedRenderTarget::Window`
cameras, so an image-targeted UI camera gets no cursor and never registers a
click. The first cut put UI on the image-targeted scenario camera - hence the bug.

Fix verified on three axes:

- **Clicks:** UI is moved to the blit `Camera2d`, which targets the window
  (`Camera2d` default). A unit test pins the invariant (scenario camera is NOT
  `IsDefaultUiCamera`; the blit camera IS and targets a `Window`) - the exact
  precondition `ui_focus_system` needs. A crisp HUD in the screenshot is itself
  proof the UI renders on the window camera (a reduced-image HUD would be soft).
- **Projection stays aligned:** the scenario camera's image-target `scale_factor`
  is set to `image_physical / window_logical`, and bevy computes
  `RenderTargetInfo.scale_factor = image_target.scale_factor`
  (bevy_render `camera.rs:287-289`) with `logical = physical / scale_factor`
  (bevy_camera `camera.rs:434`), so `world_to_viewport` / `logical_viewport_size`
  (the only world->screen HUD projector, `hud/screen_indicator`) report
  window-space coords. The Low screenshot shows edge indicators at the same
  positions as High - no misalignment.
- **No regression:** RenderLayers isolation was dropped, which is safe (verified
  no world-space 2D `Sprite` entities exist, and a `Camera3d` never draws 2D
  sprites, so the blit `Camera2d` renders only its own sprite + the UI pass).
  Teardown restores the single-window-camera default. All 5 render_scale tests
  pass; the crate + workspace compile clean.

Not verifiable headlessly: an actual mouse click (no cursor in the Xvfb capture).
Mitigated by the window-target unit-test invariant + the crisp-HUD screenshot;
the user re-tests the live click.

No BLOCKER/MAJOR/MINOR findings. The change is contained and the failure mode is
now covered by a test.
