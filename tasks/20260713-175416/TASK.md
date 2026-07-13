# Apply cubemap.png.meta in the real app via AssetMetaCheck::Paths

- STATUS: OPEN
- PRIORITY: 95
- TAGS: v0.5.0,bug,web

## Goal

The skybox cubemap fix from tasks/20260710 (retro
docs/retros/20260710-skybox-cubemap-upload-race.md) relies on
`assets/textures/cubemap.png.meta` reinterpreting the stacked 4096x24576 PNG
into a 6 layer array inside the image loader. That meta is silently ignored
in the shipped app on every platform: `assets_plugin()` in
`crates/nova_core/src/lib.rs:216` sets `meta_check: AssetMetaCheck::Never`
(present since 2025-10, long before the meta fix landed 2026-07-10). The
v0.5.0 web build logs the canary warning
"prepare_cubemap_view: cubemap loaded as a single layer image" and the
oversized 2D upload race from the retro is back for every GPU with a 16384
texture limit. Make the real app read exactly the cubemap's meta.

## Steps

- [ ] In `crates/nova_core/src/lib.rs`, change `assets_plugin()` to
      `meta_check: AssetMetaCheck::Paths` containing exactly
      `"textures/cubemap.png"`, with a comment explaining both directions:
      `Never` silently defeated the cubemap loader-settings fix, and
      per-path opt-in avoids the wasm HTTP 404 for every other asset's
      missing .meta (the original reason for `Never`).
- [ ] Make `assets_plugin()` `pub` so the regression test exercises the
      app's real asset configuration instead of a hand-rolled one (the gap
      that let this ship: crates/nova_assets/tests/cubemap_meta.rs builds
      its own AssetPlugin with default meta_check).
- [ ] Add a regression test in `crates/nova_core/tests/` that loads
      `textures/cubemap.png` through `AssetPlugin { file_path:
      "../../assets".into(), ..assets_plugin() }` headlessly (mirror
      cubemap_meta.rs) and asserts 6 square layers. Prove it fail-first
      against the current `Never` config and record the failing output.
- [ ] Verify the existing `cubemap_meta.rs` test still passes and update its
      doc comment to note the app-config test in nova_core is the one that
      guards the real app.
- [ ] `cargo check` + `cargo fmt`; run the new/affected tests only; skip the
      full local suite per repo policy and say so.
- [ ] CHANGELOG.md entry: skybox cubemap meta now actually applies in the
      app; web/native single-layer warning gone.

## Notes

- Trunk ships the whole assets dir (`<link data-trunk rel="copy-dir"
  href="assets"/>` in index.html), so `cubemap.png.meta` is present in the
  web bundle; `AssetMetaCheck::Paths` will fetch it over HTTP only for the
  cubemap.
- `AssetMetaCheck::Paths(HashSet<AssetPath>)` exists in Bevy 0.19
  (bevy_asset-0.19.0/src/lib.rs:319).
- assets/textures/cubemap.png.meta is the only .meta in the repo.
- bevy_asset_loader's `image(array_texture_layers = 6)` derive option is NOT
  a fix: it reinterprets when the loading state finishes, frames after the
  load - the same eager-upload race the retro documents.
- The GameAssets collection loads the path as "textures/cubemap.png"
  (crates/nova_assets/src/lib.rs:62), matching the Paths entry.
- Why the July fix looked verified: its test passes (own AssetPlugin,
  default meta_check Always) while the app config never reads metas -
  lesson for the retro: regression tests must go through the app's config.
