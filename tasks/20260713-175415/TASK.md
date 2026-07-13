# Fix WebGL2 fatal crash: inset render target view_formats

- STATUS: CLOSED
- PRIORITY: 100
- TAGS: v0.5.0,bug,web

## Goal

The v0.5.0 web (WASM/WebGL2) build quits with a fatal render validation error
the moment gameplay starts (New Game or editor Play): `Device::create_texture`
fails with "Downlevel flags DownlevelFlags(VIEW_FORMATS) are required but not
supported on the device", then the invalid texture cascades into
`Queue::write_texture` / `Texture::create_view` errors and Bevy quits the app.
Make the target inset render target WebGL2-safe so the web build is playable.

## Root cause

`create_render_target` in `crates/nova_gameplay/src/hud/target_inset.rs:231`
calls `Image::new_target_texture(px, px, TextureFormat::Rgba8Unorm,
Some(TextureFormat::Rgba8UnormSrgb))`. With a `Some` view format, Bevy 0.19
fills `texture_descriptor.view_formats` with `format.srgb_view_formats()`
(bevy_image-0.19.0/src/image.rs:1266). A non-empty `view_formats` list
requires the wgpu `DownlevelFlags::VIEW_FORMATS` capability, which the
WebGL2/GLES backend does not have - the browser log states this verbatim.
The image is created when the player HUD spawns on gameplay start, which is
exactly when the crash happens. The GpuImage upload path (create_texture +
write_texture, then create_view) matches the reported error sequence one to
one. The pattern was copied from Bevy's own `render_to_texture` example,
which carries the same WebGL2 incompatibility.

## Steps

- [x] In `crates/nova_gameplay/src/hud/target_inset.rs`, change
      `create_render_target` to
      `Image::new_target_texture(INSET_TEXTURE_PX, INSET_TEXTURE_PX,
      TextureFormat::Rgba8UnormSrgb, None)`. With `None`, `view_formats`
      stays empty and no `texture_view_descriptor` is set; the default view
      has the sRGB format, so rendering and UI sampling behave exactly as
      before on native.
- [x] Rewrite the doc comment on `create_render_target`: it cites the Bevy
      0.19 RTT example convention (Rgba8Unorm storage + sRGB view); it must
      now explain that the example's pattern needs
      `DownlevelFlags::VIEW_FORMATS`, which WebGL2 lacks, making it a fatal
      validation error there, and that a plain sRGB-format target is
      equivalent end to end.
- [x] Add a regression test in the `tests` module of `target_inset.rs`
      asserting the WebGL2-safe invariant of the created image:
      `texture_descriptor.view_formats` is empty,
      `texture_view_descriptor` is `None`, and the format is
      `Rgba8UnormSrgb`. Prove it fail-first against the pre-fix
      constructor call and record the failing output here.
- [x] `cargo check` + `cargo fmt`; run the new test only
      (`cargo test -p nova_gameplay <filter>`); per repo policy skip the
      full local suite (CI runs it) and say so.
- [x] Add a CHANGELOG.md entry noting the web build fatal crash fix.

## Notes

- Browser crash sequence: menu is fine; on Play, a WARN about a despawned
  entity (tracked separately in 20260713-175352), then the three validation
  errors, then "Quitting the application due to Validation RenderError".
- `Image::new_target_texture` source: bevy_image-0.19.0/src/image.rs:1232;
  `view_formats: match view_format { Some(_) => format.srgb_view_formats(),
  None => &[] }`, `texture_view_descriptor` set only for `Some`.
- Verified: target_inset.rs is the workspace's only `new_target_texture` /
  render-to-texture site; no other `view_formats` use exists.
- Visual equivalence: the current code renders into and samples through an
  Rgba8UnormSrgb *view* of an Rgba8Unorm texture; the fix makes the texture
  itself Rgba8UnormSrgb with the default view - same view format end to end,
  so no color shift on native.
- A real WebGL2 context cannot run in this environment; the descriptor-level
  invariant test is the practical regression pin.
- Sibling fix for the same play session: 20260713-175416 (cubemap .meta
  ignored because AssetMetaCheck::Never).

## Record

What changed: `create_render_target` now builds the inset target as
`Rgba8UnormSrgb` with `view_format: None` (commit da89ff6), so the texture
descriptor carries no `view_formats` and no view override - the WebGL2-safe
shape. Doc comment rewritten to warn that Bevy's own render_to_texture
example pattern (Rgba8Unorm + Some(sRGB view)) is a fatal validation error
on WebGL2. Regression test `render_target_is_webgl2_safe` pins format,
empty `view_formats`, and absent `texture_view_descriptor`. CHANGELOG
Unreleased/Fixed entry added.

Alternatives considered: keeping Unorm storage and gating the view format
by `cfg(target_arch = "wasm32")` (two code paths, native keeps a pattern
that is broken on one platform for no benefit); querying
`RenderDevice::features`/downlevel caps at runtime (heavier, and the sRGB
target is equivalent everywhere anyway).

Fail-first A/B (fix committed first, then sabotage, per lesson): with the
constructor reverted to `(Rgba8Unorm, Some(Rgba8UnormSrgb))`, the test
fails at the first assertion (target_inset.rs:813, format Rgba8Unorm !=
Rgba8UnormSrgb); restored via `git checkout -- <file>`, test passes
(1 passed, 476 filtered out).

Verification: `cargo check` green, `cargo fmt` clean, new test run in
isolation. Full test suite and clippy deliberately skipped per repo
policy - CI (.github/workflows/ci.yaml) is the source of truth for those.
A real WebGL2 context cannot run here; descriptor-level invariants are the
practical pin, and the user should confirm on the deployed build.

Reflection: the root cause was found by matching the browser log's error
sequence (create_texture -> write_texture -> create_view) to Bevy's
GpuImage upload path, then grepping for the one RTT site - faster and more
reliable than theorizing about renderer internals. What could have gone
better: the original inset work copied the upstream example without
checking its downlevel requirements; when copying a rendering pattern,
check what wgpu capabilities it implies for the web target (this repo
ships wasm as a first-class platform).
