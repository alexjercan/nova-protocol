# Mod download/cache/install runtime: ehttp fetch, native data-dir + wasm IndexedDB storage, mods:// asset source, installed index

- STATUS: OPEN
- PRIORITY: 16
- TAGS: modding,wasm

Spike: tasks/20260714-202515/SPIKE.md (options P, T)
Depends on: 20260715-142900 (portal - the formats this cache stores).

RESCOPED at plan time: the network half (ehttp fetch, staged download,
install/uninstall events) split to 20260715-163508 - this task is the LOCAL
foundation it commits into: the mod cache (native FS + wasm IndexedDB), the
`mods://` asset source, the installed index, and the merge integration. After
this task, a mod whose files sit in the cache loads and merges like a shipped
one - no network code anywhere yet.

## Plan (20260715)

Design (from the code + the spike's verified facts):

- MODULE LAYOUT: a new `nova_assets::mod_cache` module owns storage + index;
  `nova_core::assets_plugin` grows the `mods://` source registration (must
  precede AssetPlugin - verified: `AssetApp::register_asset_source`,
  bevy_asset-0.19.0/src/lib.rs:563).
- INDEX (small, sync): `InstalledModRecord { id, version, bundle }` list.
  Native: RON file `dirs::data_dir()/nova-protocol/installed.mods.ron`. Wasm:
  localStorage key `nova_protocol.installed_mods` (same split as mod_prefs:
  small prefs sync, bulk bytes async). Records are DOWNLOADED mods only; the
  shipped catalog stays the other half of the installed set.
- FILE BYTES: native `dirs::data_dir()/nova-protocol/mods/<id>/<files>`
  (data_dir for content, config_dir stays prefs-only). Wasm: IndexedDB, DB
  `nova-protocol`, object store `mod-files`, key `<id>/<path>`, value the raw
  bytes - via a thin hand-rolled `web-sys` wrapper (IdbFactory open +
  onupgradeneeded creating the store, get/put/delete/getAllKeys as
  wasm-bindgen-futures-awaitable helpers; ~150 lines behind
  `#[cfg(target_arch = "wasm32")]`). Decision vs the plan-question: hand-rolled
  over `rexie` - the repo owns its two short storage impls already
  (mod_prefs), the needed surface is four operations on one store, and it
  avoids pinning a third-party wasm-bindgen version. Revisit if the surface
  grows.
- CACHE API (platform-split like mod_prefs, one signature): `read_index()`,
  `write_index(records)`, `store_mod_files(id, files: &[(path, bytes)])`,
  `remove_mod_files(id, paths)`, `read_mod_file(id, path)` (native sync fs;
  wasm async IDB). The COMMIT discipline (files first, index last) lives in
  the caller (163508); this task provides the primitives + a native-side
  `install_local(id, version, bundle, files)` helper used by tests and the
  future installer.
- ASSET SOURCE `mods://`: native = `FileAssetReader::new(<data_dir>/mods)`;
  wasm = `MemoryAssetReader { root: Dir }` with the `Dir` handle ALSO stored
  in a resource; a startup async task hydrates it from IDB (read index ->
  read each mod's files -> `Dir::insert_asset`) then flips a
  `ModCacheHydrated` state/resource (verified: `Dir` is Arc-shared,
  register-empty-fill-later works, bevy_asset-0.19.0/src/io/memory.rs).
  Native hydration is a no-op (the FS reader reads live).
- INSTALLED-SET INTEGRATION: a `DownloadedMods` resource (records + a
  `Handle<BundleAsset>` per record, loaded via
  `asset_server.load("mods://<id>/<bundle>")` once hydrated); `ModCatalog`
  rows extend with downloaded mods (ModInfo from bundle meta, id fallback);
  `register_bundles` merges shipped-enabled THEN downloaded-enabled (index
  order), re-running when `DownloadedMods` changes (same resource_changed
  idiom as EnabledMods; combine with `.or()`). Downloaded mods install
  DISABLED (enable is the existing EnabledMods toggle; hidden-strip and base
  semantics untouched - downloaded records have no base/hidden flags).
- WASM COMPILE GATE: `rustup target list --installed` includes
  wasm32-unknown-unknown? if yes `cargo check --target wasm32-unknown-unknown
  -p nova_assets -p nova_core`; else the trunk build. (233438 ran the real
  trunk build locally; CI does not compile wasm on PRs.)

Steps:
- [ ] 1. nova_assets `mod_cache` module: `InstalledModRecord` (+ RON serde),
  the platform-split index + file primitives (native fs impl with pure
  `*_at(root, ...)` inner functions, tempfile-tested; wasm IDB wrapper behind
  cfg, statically reviewed), and `install_local` composing them. web-sys
  features grow Idb* types (target-gated dep already exists).
- [ ] 2. nova_core `assets_plugin`: register the `mods://` source (cfg-split
  reader as above) BEFORE AssetPlugin; wasm keeps the shared `Dir` in a
  `ModsSourceDir` resource for the hydrator. Native root =
  `dirs::data_dir()/nova-protocol/mods` (dirs dep moves/copies to nova_core's
  native deps as needed).
- [ ] 3. nova_assets: `DownloadedMods` resource + systems - wasm: hydrate
  Dir from IDB in an IoTaskPool task at startup, then read the index; native:
  read the index directly; then `load` each record's bundle via `mods://` and
  hold handles. Extend `build_mod_catalog` to append downloaded rows (bundle
  meta once loaded, decl-only fallback) and `register_bundles` to merge
  enabled downloaded bundles after shipped ones, re-running on
  `DownloadedMods` change.
- [ ] 4. Tests (native, real asset server): (a) mod_cache round-trip under a
  temp data root (index write/read; store/remove/read files; corrupt index ->
  None-not-panic); (b) END-TO-END: `install_local` a copy of the REAL gauntlet
  mod (from webmods/) into a temp cache root, point the `mods://` source at
  it, drive the real load + merge - `gauntlet_run` appears in `GameScenarios`
  when enabled and disappears on uninstall (remove files + index + re-merge);
  (c) ModCatalog lists the downloaded mod's bundle meta. Rig: the source
  registration must be test-overridable - root path injection (env var or
  builder param) decided in-code; cite the chosen mechanism in the close-out.
- [ ] 5. Wasm compile gate: wasm32 check (or trunk build) proving the cfg
  halves compile; static review of the IDB wrapper against web-sys 0.3 docs.
- [ ] 6. Docs: modding-ron-format.md gains the installed-index format +
  mods:// source paragraph; docs/mod-portal.md cross-ref ("how installed mods
  are stored"); CHANGELOG (Added: local mod cache foundation).
- [ ] 7. Verify: fmt; check --workspace --all-targets; `cargo test -p
  nova_assets` targets + new tests; the wasm gate from step 5. Full suite on
  CI.

## Notes

- Relevant files: crates/nova_assets/src/{lib.rs,mod_prefs.rs} (idioms to
  mirror), crates/nova_core/src/lib.rs:229 (assets_plugin),
  crates/nova_assets/tests/demo_scenario.rs (rig helpers),
  webmods/gauntlet (e2e subject), bevy_asset 0.19 io/{memory.rs,file}.
- The riskiest unknown (spike): the wasm IDB + memory-Dir path. It is
  compile-gated + statically reviewed here; its first RUNTIME exercise is
  163508's wasm testing / a manual web session - noted honestly.
- ehttp/PortalConfig/schema_version checks: NOT this task (163508).

