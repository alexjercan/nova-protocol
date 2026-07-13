# Review: Fix WebGL2 fatal crash: inset render target view_formats

- TASK: 20260713-175415
- BRANCH: fix/webgl2-inset-view-formats

## Round 1

- VERDICT: APPROVE

No findings. Basis for the verdict, since implementer and reviewer share a
session (out-of-context blind-spot rule: re-derive the load-bearing claims
instead of trusting the diff):

- Re-derived the crash mechanism in bevy_render-0.19.0 source
  (src/texture/gpu_image.rs prepare_asset): image upload is
  `create_texture_with_data(&image.texture_descriptor)` (create_texture +
  write_texture) followed by `create_view` - the browser log's three
  validation errors in exact order. `view_formats` flows verbatim from the
  descriptor into the failing `create_texture`, and
  bevy_image-0.19.0/src/image.rs:1266 fills it iff the `view_format` arg is
  `Some`. The fix (`Rgba8UnormSrgb`, `None`) leaves it empty.
- Coherence of the new state: with `texture_view_descriptor: None`, GpuImage
  falls back to `TextureViewDescriptor::default()`, so the view format is
  the texture's own Rgba8UnormSrgb; before, the same sRGB format was reached
  via the view override. Render attachment, UI sampling, and every consumer
  of `GpuImage.texture_descriptor.format` now agree on one format - strictly
  simpler than the old mixed state, no native behavior change.
- Swept the workspace and docs/ for `Rgba8Unorm`, `new_target_texture`, and
  `view_formats`: target_inset.rs is the only site; no living doc references
  the old pattern. The kill cam (`TargetInsetLastFramed`) freezes target +
  camera pose only - no pixel readback that could care about the storage
  format.
- Regression test asserts behavior (descriptor invariants), not mere
  execution, and was proven fail-first: constructor reverted to
  `(Rgba8Unorm, Some(Rgba8UnormSrgb))` fails the format assertion at
  target_inset.rs:813; restored fix passes. Evidence recorded in TASK.md.
- Checks run in the worktree: `cargo check` green, `cargo fmt --check`
  clean, `cargo test -p nova_gameplay --lib hud::target_inset` 15/15 green.
  Full suite + clippy skipped per repo policy (CI owns them).
- Residual risk, accepted and recorded honestly in TASK.md: no WebGL2
  context exists in this environment, so the on-device confirmation belongs
  to the user on the deployed build. The sibling cubemap task
  (20260713-175416) covers the remaining warning from the same play session.
