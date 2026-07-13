# Apply cubemap.png.meta in the real app via AssetMetaCheck::Paths

- STATUS: CLOSED
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

- [x] In `crates/nova_core/src/lib.rs`, change `assets_plugin()` to
      `meta_check: AssetMetaCheck::Paths` containing exactly
      `"textures/cubemap.png"`, with a comment explaining both directions:
      `Never` silently defeated the cubemap loader-settings fix, and
      per-path opt-in avoids the wasm HTTP 404 for every other asset's
      missing .meta (the original reason for `Never`).
- [x] Make `assets_plugin()` `pub` so the regression test exercises the
      app's real asset configuration instead of a hand-rolled one (the gap
      that let this ship: crates/nova_assets/tests/cubemap_meta.rs builds
      its own AssetPlugin with default meta_check).
- [x] Add a regression test in `crates/nova_core/tests/` that loads
      `textures/cubemap.png` through `AssetPlugin { file_path:
      "../../assets".into(), ..assets_plugin() }` headlessly (mirror
      cubemap_meta.rs) and asserts 6 square layers. Prove it fail-first
      against the current `Never` config and record the failing output.
- [x] Verify the existing `cubemap_meta.rs` test still passes and update its
      doc comment to note the app-config test in nova_core is the one that
      guards the real app.
- [x] `cargo check` + `cargo fmt`; run the new/affected tests only; skip the
      full local suite per repo policy and say so.
- [x] CHANGELOG.md entry: skybox cubemap meta now actually applies in the
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

## Record

What changed: `assets_plugin()` (crates/nova_core/src/lib.rs) now sets
`AssetMetaCheck::Paths` with exactly `textures/cubemap.png` and is `pub`
(commit 7b5c4d9). New regression test
`crates/nova_core/tests/cubemap_meta_app_config.rs` loads the real cubemap
through `..assets_plugin()` and asserts 6 square layers; the nova_assets
sibling test's doc comment now states it proves the meta file, not the app,
and points here. CHANGELOG Unreleased/Fixed entry added.

Alternatives considered: `AssetMetaCheck::Always` (per-asset HTTP 404s on
wasm - the original reason for Never); bevy_asset_loader's
`image(array_texture_layers = 6)` derive attribute (applies frames after
load, same eager-upload race the 20260710 retro discarded);
`load_with_settings` outside the collection (bypasses metas but forks the
GameAssets loading path for one asset).

Fail-first A/B (fix committed first, then sabotage): with `meta_check`
reverted to `Never`, the new test fails at
cubemap_meta_app_config.rs:54 - "assertion `left == right` failed: the
app's meta_check must apply cubemap.png.meta's array_layout / left: 1 /
right: 6". Restored via `git checkout -- crates/nova_core/src/lib.rs`;
test passes (1 passed, 0.31s). Sibling test `-p nova_assets --test
cubemap_meta` also green (1 passed).

Verification: `cargo check` green, `cargo fmt` clean, both cubemap tests
run individually. Full suite and clippy skipped per repo policy - CI is
the source of truth. Web-side confirmation (meta fetched over HTTP from
the trunk bundle) belongs to the user's next deployed build; trunk's
copy-dir of assets/ was verified in index.html.

Reflection: the fix itself is three lines; the entire value was in the
verification design - the new test differs from the existing one only in
`..assets_plugin()`, which is precisely the bit that was broken. When a
"verified" fix ships broken, the test's config diverging from the app's
config should be the first suspect.
