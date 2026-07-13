# Review: Apply cubemap.png.meta in the real app via AssetMetaCheck::Paths

- TASK: 20260713-175416
- BRANCH: fix/cubemap-meta-check-paths

## Round 1

- VERDICT: APPROVE

No findings. Basis for the verdict (shared-session blind-spot rule:
load-bearing claims re-derived from source, not the implementer's summary):

- Re-derived `AssetMetaCheck::Paths` semantics in
  bevy_asset-0.19.0/src/server/mod.rs:1564: `read_meta` is
  `paths.contains(asset_path)` against the requested `AssetPath` - exact
  source+path+label containment. The green app-config test is the empirical
  proof the entry matches the collection's load of "textures/cubemap.png";
  no path-normalization gap.
- Verified the one production consumer of the changed image shape: the
  pinned bevy-common-systems rev a35b74c guards its fallback
  (src/camera/skybox.rs:120, `array_layer_count() == 1`), so an image born
  6-layer + Cube view (prepare_cubemap_view) makes the reinterpret a no-op
  and the attach proceeds - the July fix's intended design, now actually
  reached in the shipped app.
- Other assets are unaffected by construction: cubemap.png.meta is the
  repo's only meta file, and Paths reads metas for nothing else - so no new
  wasm HTTP requests or native FS probes beyond the one opted-in path.
- Test quality: asserts behavior (6 square layers through the app's exact
  config), differs from the pre-existing nova_assets test only in
  `..assets_plugin()` - precisely the broken bit - and was proven
  fail-first: with `Never` restored it fails "left: 1 / right: 6" at
  cubemap_meta_app_config.rs:54 (recorded in TASK.md). No tests weakened;
  the sibling test's doc comment now honestly scopes what it proves.
- Checks run in the worktree: `cargo check` green, `cargo fmt` clean,
  `cargo test -p nova_core --test cubemap_meta_app_config` and
  `cargo test -p nova_assets --test cubemap_meta` both 1/1 green. Full
  suite + clippy skipped per repo policy (CI owns them).
- Rider commit a235f8c (previous task's retro + ledger) is documentation
  only, forced onto this branch by the background session's checkout write
  guard; content matches the closed 20260713-175415 cycle.
- Residual risk, accepted: the wasm HTTP fetch of the .meta is exercised
  only on a deployed build (trunk copy-dir ships the file - verified in
  index.html); descriptor-level and config-level behavior is pinned here.
