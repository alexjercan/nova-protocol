# Skybox cubemap upload race (v0.4.0 release blocker)

## Symptom

The `examples_smoke` CI job failed on `03_scenario`. Two different failure
modes showed up on the way to the root cause:

1. On CI (first green-path run after the libxkbcommon fix): a panic inside
   `bevy_ui`'s layout system, in taffy 0.10.1
   (`resolve.rs:68: internal error: entered unreachable code`).
2. Locally under Xvfb + lavapipe (about 1 run in 5): the app quit before
   reaching Playing with:

   ```
   In Device::create_texture
     Dimension Y value 24576 exceeds the limit of 16384
   ```

## Root cause (the validation error)

`assets/textures/cubemap.png` is a vertically stacked skybox: 6 faces of
4096x4096, so the PNG is 4096x24576. The `SkyboxPlugin` in
`bevy_common_systems` reinterprets that stacked image into a 6 layer array
only when a `SkyboxConfig` is inserted on a camera.

But Bevy's renderer eagerly uploads every loaded `Image` asset
(`RenderAssetUsages::RENDER_WORLD`, the default). Asset events are published
in `Last` and the render world extracts at the end of the same frame, so no
main-world system can react to "the cubemap just loaded" before the upload
happens. If the PNG finishes loading on an earlier frame than the camera
spawn, the GPU sees the raw 24576 px tall 2D texture. Real GPUs typically
allow 32768 and don't care; llvmpipe (CI's software Vulkan) caps at 16384,
the `create_texture` call fails validation, and Bevy 0.19 quits the app on
render errors. Which side of the race you land on depends on the frame the
asset load completes, hence the flakiness.

## Fix

- `assets/textures/cubemap.png.meta` sets the image loader's
  `array_layout: Some(RowCount(rows: 6))`, so the asset is born as a 6 layer
  array inside the loader. The oversized 2D form never exists, which makes
  the eager upload harmless by construction. This is Bevy 0.19's
  `ImageLoaderSettings::array_layout`; no custom loader needed.
- Loader settings cannot express a texture view, so a new
  `prepare_cubemap_view` system in `GameAssetsStates::Processing` (after the
  collection loads, before anything spawns a camera) sets the `Cube` view
  dimension. `SkyboxPlugin`'s reinterpret is guarded by
  `array_layer_count() == 1`, so it now no-ops and just attaches `Skybox`.
- `crates/nova_assets/tests/cubemap_meta.rs` loads the real asset through a
  headless asset server and asserts the meta produces 6 square layers, so a
  deleted or broken `.meta` fails CI rather than reintroducing the flake.

Alternatives considered:

- Reacting to `AssetEvent<Image>` in `Update` and reinterpreting there: does
  not work; the renderer sees the event the same frame in `Last`/extract,
  before any main-world system of the next frame runs.
- `bevy_asset_loader`'s `image(array_texture_layers = 6)` attribute: runs
  when the loading state finishes, frames after the asset loaded - same race.
- Shipping a KTX2 cubemap: cleaner long term (loads directly as a cube), but
  needs toktx tooling and a compression decision; not required to fix the bug.
- Shrinking faces to 2048 (12288 < 16384): hides the bug behind a smaller
  limit instead of removing it.

## The taffy panic

The CI taffy panic is not fully explained. taffy packs a tag into the low
bits of each style value; a NaN or infinity cannot corrupt it (verified
empirically: a scratch taffy 0.10.1 tree with inf/NaN sizes and insets
computes layout without panicking). Hitting `unreachable!()` there requires a
corrupt tag, which safe code should never produce. The leading theory is
in-process memory corruption under llvmpipe (the JIT shares the heap with the
game), possibly triggered by the 400 MB oversized texture the race uploaded.
It did not reproduce locally (Mesa 26.1 llvmpipe, including pinned to 2
cores) in any of 10+ runs. If it recurs on CI after this fix, treat it as its
own investigation: suspect llvmpipe first, and try updating Mesa on the
runner (e.g. the kisak PPA) to rule the driver in or out.

## Reflection

- The first CI failure (missing `libxkbcommon-x11-0`) masked this one; the
  smoke test had never actually run green on CI, so "CI is red" did not mean
  "one bug".
- The decisive move was reproducing the *test harness's* invocation locally
  under Xvfb + forced lavapipe (`VK_DRIVER_FILES=...lvp_icd...`), not the
  example binary alone; only the full loop surfaced the 1-in-5 flake.
- Asset-shaped bugs hide behind driver limits: everything about the stacked
  cubemap worked on any desktop GPU. When a CI-only render failure mentions
  a dimension limit, check the assets before the code.
- Understand the frame timeline before writing an event-driven fix: the
  obvious "react to the asset event" system is provably too late here, and
  knowing that early (asset events publish in `Last`, extraction reads them
  the same frame) discarded two wrong designs cheaply.
