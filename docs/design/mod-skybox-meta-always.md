# Mod-shipped skybox cubemaps: `AssetMetaCheck::Always` (task 20260717-111558)

## What changed

`nova_core::assets_plugin()` now sets `meta_check: AssetMetaCheck::Always`
instead of the two-path `AssetMetaCheck::Paths` set it carried since task
20260717-013440. The app now reads a `.meta` sidecar for every asset, from
every source, so a mod that ships its own skybox cubemap + `.meta` gets the
`array_layout: RowCount(6)` reinterpret applied at load time exactly like the
base cubemaps.

Everything else about the skybox path is unchanged: `apply_pending_skybox_swaps`
still sets the `Cube` texture view for a cubemap that arrives already 6-layer
(the meta-applied case skips the bcs SkyboxPlugin fallback that would otherwise
set it), and that logic now covers base and mod cubemaps uniformly.

## Why (and the decision the task asked to weigh)

The bug: a mod's own skybox is a dynamic `self://`/`mods://` path. The old
`Paths` set is fixed at App build and can only list static base paths, so a mod
cubemap's shipped sidecar was silently ignored. It then loaded as a raw
single-layer 4096x24576 stacked image and relied on the bcs observer's fallback
reinterpret. The normal path is safe (the observer reinterprets the same frame
the image lands, before extraction), but a scenario teardown during the PNG
decode can leave the stacked image to upload as-is - a fatal wgpu validation
error on GPUs with `max_texture_dimension_2d = 16384` (WebGL2-class, llvmpipe).
This is the exact class task 20260717-013440 closed for the base cubemaps.

The task floated using `Always` globally and asked whether that is a bad idea.
The old doc comment claimed `Always` "would fire an HTTP request per asset on
wasm just to 404". We verified that claim against the pinned bevy source rather
than trusting it:

- `bevy_asset-0.19.0/src/server/mod.rs:1564` - `Always` sets `read_meta = true`
  for every asset (vs. `paths.contains(asset_path)` for `Paths`).
- `read_meta = true` calls `asset_reader.read_meta(path)`; the wasm reader
  (`src/io/wasm.rs:122`) `fetch()`es `<path>.meta`.
- A missing sidecar returns HTTP 404, mapped to `NotFound`
  (`src/io/wasm.rs:100`).
- bevy handles `NotFound` by falling back to `loader.default_meta()`
  (`src/server/mod.rs:1616`) - graceful, no error.

So the 404 is real but non-fatal: on web, one extra `.meta` request per asset,
404 for the (many) assets without a sidecar, plus the console noise those
404s produce. On native it is a cheap filesystem stat per asset, no network.

We chose `Always` anyway (user decision), trading that web cost for closing the
skybox-crash class without any per-path bookkeeping. The alternatives were all
worse or unavailable: bevy has no per-source `meta_check` and no predicate
variant; the `Paths` set cannot be extended at mod-install time; and a
nova-side reinterpret in the swap applier would still leave the teardown race
open for any cubemap that reaches `Assets` single-layer. `Always` makes the
cubemap arrive 6-layer from the loader, so the single-layer stacked form never
exists - the same reason the base fix worked, now for every source.

## Verification

- `trunk build` (web target) compiles cleanly with `Always` (the switch is not
  a wasm compile problem; the cost is purely the runtime 404s described above).
- `cargo check` + `cargo fmt` clean on the affected crates.
- `crates/nova_core/tests/cubemap_meta_app_config.rs` gained
  `app_asset_config_loads_mod_cubemap_as_six_layer_array`, which loads the
  example mod's `mods/example/textures/nebula.png` through the real
  `assets_plugin()` and asserts a 6-layer array. It fails under the old `Paths`
  config (nebula was never listed) and passes under `Always` - the regression
  pin for the switch.

## Docs swept in the same task

- `CHANGELOG.md` [Unreleased] Fixes: added the mod-skybox entry beside the
  existing base-cubemap one.
- `web/src/wiki/dev/guide-make-a-mod.md`: the skybox-sidecar bullet now states
  the game reads the sidecar at load time on every platform/source, and warns
  that omitting it risks the WebGL2 crash.
- `crates/nova_assets/tests/cubemap_meta.rs` and
  `crates/nova_scenario/src/actions.rs` doc comments: updated the stale "app
  reads metas per-path" wording to `Always`.

## Reflection

The task listed several candidate directions (an upstream bcs fix, several
nova-side meta strategies) and explicitly asked to weigh `Always`. The right
first move was to verify the load-bearing claim (the 404 cost) against source
before designing around it - the old comment stated it as fact but it turned
out to overstate the severity (real cost, but non-fatal, and web-only). Reading
the three bevy source hops took minutes and turned a "known" tradeoff into a
measured one, which is what let the global-`Always` option be chosen with
confidence. Lesson for next time: when a doc comment asserts a cost as the
reason a simpler design was rejected, check the assertion before inheriting it.

One pre-existing observation surfaced and was deliberately left out of scope:
`nova_editor` inserts `SkyboxConfig` directly (not via
`apply_pending_skybox_swaps`) with the base cubemap, which already arrives
6-layer today - so the editor's skybox may already miss its `Cube` view. This
change does not regress it (that cubemap was already meta-applied under `Paths`),
so it is noted here for a future task rather than folded in.
