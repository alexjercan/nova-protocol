# Mod download/cache/install runtime: ehttp fetch, native data-dir + wasm IndexedDB storage, mods:// asset source, installed index

- STATUS: CLOSED
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
- [x] 1. nova_assets `mod_cache` module: `InstalledModRecord` (+ RON serde),
  the platform-split index + file primitives (native fs impl with pure
  `*_at(root, ...)` inner functions, tempfile-tested; wasm IDB wrapper behind
  cfg, statically reviewed), and `install_local` composing them. web-sys
  features grow Idb* types (target-gated dep already exists).
- [x] 2. Register the `mods://` source (cfg-split reader as above) BEFORE
  AssetPlugin; wasm keeps the shared `Dir` in a `ModsSourceDir` resource for
  the hydrator. Native root = `dirs::data_dir()/nova-protocol/mods`.
  ADAPTED from "nova_core assets_plugin": `assets_plugin()` returns the
  AssetPlugin VALUE for `.set()` and cannot register a source (that needs the
  `App`), so the registration helper `register_mods_source(app)` lives next to
  the cache in `nova_assets::mod_cache` and `AppBuilder::new` calls it before
  `DefaultPlugins`. Bonus: the integration tests build their rig on the exact
  production registration, and nova_core needs no new `dirs` dep (the root
  helpers stay in nova_assets, which already has it).
- [x] 3. nova_assets: `DownloadedMods` resource + systems - wasm: hydrate
  Dir from IDB in an IoTaskPool task at startup, then read the index; native:
  read the index directly; then `load` each record's bundle via `mods://` and
  hold handles. Extend `build_mod_catalog` to append downloaded rows (bundle
  meta once loaded, decl-only fallback) and `register_bundles` to merge
  enabled downloaded bundles after shipped ones, re-running on
  `DownloadedMods` change (plus `mark_downloaded_bundles_loaded`, which flags
  the resource when a bundle's async load lands - without it a mod that
  finishes loading after the last resource mutation would never merge).
- [x] 4. Tests (native, real asset server): (a) mod_cache round-trip under a
  temp data root (index write/read; store/remove/read files; corrupt index ->
  None-not-panic); (b) END-TO-END: `install_local` a copy of the REAL gauntlet
  mod (from webmods/) into a temp cache root, point the `mods://` source at
  it, drive the real load + merge - `gauntlet_run` appears in `GameScenarios`
  when enabled and disappears on uninstall (remove files + index + re-merge);
  (c) ModCatalog lists the downloaded mod's bundle meta. Rig: the source
  registration is test-overridable via the `NOVA_MOD_CACHE_ROOT` env var
  (read at registration time AND by every native path helper, so source and
  cache agree; tests serialize on a lock because env is process-global).
- [x] 5. Wasm compile gate: the nix toolchain ships the wasm32 sysroot, so the
  REAL `cargo check --target wasm32-unknown-unknown -p nova_assets -p
  nova_core` ran (green, no warnings) + static review of the IDB wrapper
  against web-sys 0.3.
- [x] 6. Docs: modding-ron-format.md gains the installed-index format +
  mods:// source section; docs/mod-portal.md cross-ref ("how installed mods
  are stored"); CHANGELOG (Added: local mod cache foundation).
- [x] 7. Verify: fmt; check --workspace --all-targets; `cargo test -p
  nova_assets` targets + new tests; the wasm gate from step 5. Full suite on
  CI. (content_ron_parity is red from a PRE-EXISTING master-side drift,
  verified unrelated - see close-out.)

## Notes

- Relevant files: crates/nova_assets/src/{lib.rs,mod_prefs.rs} (idioms to
  mirror), crates/nova_core/src/lib.rs:229 (assets_plugin),
  crates/nova_assets/tests/demo_scenario.rs (rig helpers),
  webmods/gauntlet (e2e subject), bevy_asset 0.19 io/{memory.rs,file}.
- The riskiest unknown (spike): the wasm IDB + memory-Dir path. It is
  compile-gated + statically reviewed here; its first RUNTIME exercise is
  163508's wasm testing / a manual web session - noted honestly.
- ehttp/PortalConfig/schema_version checks: NOT this task (163508).

## Close-out (20260715)

### What shipped

- `nova_assets::mod_cache` (new module): `InstalledModRecord { id, version,
  bundle }` (RON serde); native index at
  `<data_root>/installed.mods.ron` + files at `<data_root>/mods/<id>/<path>`
  with `<data_root>` = `dirs::data_dir()/nova-protocol`, overridable via
  `NOVA_MOD_CACHE_ROOT`; pure `*_at(root, ...)` inner fns under a private
  `backend` mod (the mod_prefs idiom), public wrappers adding root resolution;
  `install_local` composing store-files-then-upsert-index. Wasm half behind
  cfg: index in localStorage (`nova_protocol.installed_mods`), file bytes in
  IndexedDB (DB `nova-protocol`, store `mod-files`, key `<id>/<path>`) via a
  hand-rolled web-sys wrapper (open-with-upgrade, get, put, delete,
  getAllKeys as awaitable helpers bridging IDB events into a Promise via
  `Closure::once_into_js`). `register_mods_source(app)` registers the
  `mods://` source: native `FileAssetReader` over `<data_root>/mods` (empty
  in-memory root fallback when no data dir resolves, so `mods://` fails as
  not-found rather than unknown-source); wasm `MemoryAssetReader` over a
  shared `Dir` kept in a `ModsSourceDir` resource for the hydrator.
- `nova_core::AppBuilder::new` calls `register_mods_source` BEFORE
  `DefaultPlugins` (bevy builds sources at AssetPlugin insertion - verified in
  bevy_asset 0.19 src/lib.rs:563 register_asset_source, which errors if the
  AssetServer already exists).
- `nova_assets` integration: `DownloadedMods(Vec<DownloadedMod{record,
  bundle}>)`; native Startup `load_downloaded_mods` (read index, load each
  `mods://<id>/<bundle>`); wasm Startup `start_mod_cache_hydration`
  (IoTaskPool task: getAllKeys -> Dir::insert_asset each -> park index
  records in an Arc<Mutex> slot) + Update `poll_mod_cache_hydration` (consume
  slot, kick loads, drop marker); Update `mark_downloaded_bundles_loaded`
  (AssetEvent LoadedWithDependencies for a downloaded handle ->
  DownloadedMods.set_changed). `build_mod_catalog` appends downloaded rows
  (bundle meta once loaded, decl-only name=id fallback while in flight;
  re-run gated on resource_changed::<DownloadedMods>); `register_bundles`
  merges enabled downloaded bundles AFTER shipped ones in index order (its
  Update re-run now gated on `resource_changed::<EnabledMods>
  .or_else(resource_changed::<DownloadedMods>)`); an enabled-but-still-
  loading downloaded bundle is skipped with a warn and merges when the mark
  system fires. Downloaded mods install DISABLED (nothing touches
  EnabledMods) and carry no base/hidden flags.
- Docs: modding-ron-format.md "Downloaded mods: the local cache + the
  mods:// source" section (index format, storage locations, env override,
  runtime flow); mod-portal.md "How installed mods are stored (game side)"
  cross-ref (also corrected the "portal base URL is a config" task ref
  142906 -> 163508, the network half that owns it after the rescope);
  CHANGELOG Added entry.

### Decisions and why

- Source registration in nova_assets, not nova_core (plan step 2 adapted):
  `assets_plugin()` is a value factory for `.set()` and cannot register a
  source; registering needs `&mut App` before DefaultPlugins. Putting the
  helper next to the cache keeps root resolution in ONE crate and lets the
  test rig call the literal production function (production-faithful-rigs).
- Env-var override (`NOVA_MOD_CACHE_ROOT`) over a builder param: the root is
  read in two places (source registration + cache helpers) that share no
  constructor path; a process-level override keeps them agreeing by
  construction. Cost: env is process-global, so the e2e tests serialize on a
  static lock, each holding a fresh tempdir while locked.
- Hand-rolled IDB wrapper over rexie (per plan): four operations on one
  store, ~130 lines, no third-party wasm-bindgen pin. Requests bridge their
  success/error events into a Promise; the loser closure of each pair leaks a
  few bytes (documented) - acceptable at this call frequency.
- `write_index` is best-effort void (the mod_prefs idiom); the pure
  `write_index_at` returns io::Result for the 163508 installer's commit
  discipline (files first, index last - `install_local` already composes in
  that order).
- Path safety at the cache boundary: ids must be single normal components,
  file paths relative with no `..` (store/read/remove reject escapes) -
  downloaded index data is untrusted input even though the portal generator
  validates its own set (validate-membership-not-existence family).
- `mark_downloaded_bundles_loaded` exists because downloaded bundles sit
  OUTSIDE the GameAssets collection gate: a change-gated re-merge alone
  misses a load that completes after the last resource mutation (conditions
  are evaluated eagerly every frame, so pending "changed" ticks get consumed
  even while another run_if gates the system off - observed empirically in
  the rig, see difficulties).

### Evidence (all commands from the worktree root)

- `cargo test -p nova_assets --lib`: 30 passed (24 pre-existing + 6 new
  mod_cache unit tests: index round-trip / missing -> None / corrupt -> None /
  store-read-remove + dir prune / install_local upsert / escape rejection).
- `cargo test -p nova_assets --test mod_cache_install`: 4 passed (new file:
  install-enable-uninstall e2e, ModCatalog meta rows with deterministic
  decl-fallback-then-meta, enabled-before-load arrival, mark-system boundary
  pin).
- `cargo test -p nova_assets --test demo_scenario`: 11 passed (rig gained
  `init_resource::<DownloadedMods>()` - the systems' new Res param).
- `cargo test -p nova_assets --test cubemap_meta`: 1 passed;
  `--test webmods_validation`: 1 passed.
- `cargo fmt --check` clean; `cargo check --workspace --all-targets` green.
- Wasm gate: `cargo check --target wasm32-unknown-unknown -p nova_assets -p
  nova_core` green, no warnings (real check, not just static review - the nix
  toolchain ships the wasm sysroot; no rustup present).
- Would-it-fail sabotage runs (each applied, tested, reverted, re-verified
  green): (1) downloaded-merge arm of register_bundles deleted -> e2e enable
  step + arrival test FAIL; (2) mark system's set_changed no-op'd ->
  `loaded_event_flags_downloaded_mods_changed` FAILS; (3)
  `register_mods_source` no-op'd -> 3 tests FAIL with "Asset Source
  'AssetSourceId::Name(mods)' does not exist". Unit tests fail trivially with
  the backend fns gone (compile error).
- PRE-EXISTING failure, NOT this task: `cargo test -p nova_assets --test
  content_ron_parity` fails on `built_in_scenario_content_matches_committed_
  ron` - commit 713ac855 ("wider Shakedown spacing") changed the supply-crate
  positions in the shakedown BUILDER without regenerating the committed
  `assets/base/scenarios/shakedown_run.content.ron`. Verified unrelated by
  `git stash -u` + re-run on the clean branch head (same failure). Left for a
  master-side fix (regenerating base assets inside this feature branch would
  muddy the diff).

### Difficulties

- Bevy evaluates every `.run_if` condition each frame WITHOUT
  short-circuiting across the chain: the first e2e run panicked with
  "Parameter failed validation: Resource does not exist" because
  `resource_changed::<EnabledMods>` ran while EnabledMods was not yet
  inserted, even though the preceding `resource_exists::<GameAssets>` was
  false. Fix: the rig inits EnabledMods like production does. Corollary
  (documented in code): pending changed-ticks are consumed while a system is
  gated off, which is exactly why the mark system is load-bearing.
- Sabotage honesty: no-oping ONLY the mark system did NOT fail the
  first version of the arrival e2e - real load timing let a not-yet-consumed
  DownloadedMods tick re-fire after the load landed (`or_else` had
  short-circuited past it while EnabledMods was changed). Rather than keep a
  timing-lucky pin, the mark mechanism got a deterministic boundary test
  (hand-written AssetEvents, matched vs unrelated id) and the e2e docstring
  states what its sabotage actually showed.
- `AssetId::invalid()` and `Handle::default()` are DIFFERENT uuids in bevy
  0.19 (INVALID_UUID vs DEFAULT_UUID) but both are reserved; the boundary
  test uses `uuid_handle!` with its own uuid for the matched handle and
  `AssetId::invalid()` for the unmatched one.
- nova_assets had serde only transitively; `InstalledModRecord`'s derives
  needed it as a direct dep (serde 1, derive feature).

### Reflection

- Reading the bevy source first (register-before-AssetPlugin, Dir Arc
  sharing, FileAssetReader base-path join semantics, Message-based
  AssetEvent) meant zero engine surprises; the two real surprises were both
  ECS run-condition semantics, and both were caught by tests rather than
  reasoning - reinforces would-it-fail-without-it as a mandatory step, not a
  formality: one of three sabotages refuted the assumed mechanism.
- The plan's "one signature" cache API bent where async forces it (wasm file
  ops are async, native sync); mirroring mod_prefs for the index and
  documenting the split per-fn kept it honest instead of forcing a leaky
  abstraction.
- What to do differently: when a test asserts "X does not happen without Y",
  design the deterministic boundary pin FIRST and let the e2e cover the arc;
  the timing-lucky e2e pin cost a sabotage-diagnose-redesign loop.

## Review round R1 (20260715)

REQUEST_CHANGES addressed in a follow-up commit (1 MAJOR, 3 MINOR, 3 NIT):

- R1.1 (MAJOR, escape hardening): the mods:// load path trusted index records
  and bundle manifests. Both halves now guarded IN LAYERS: (a) `is_safe_id` /
  `is_safe_rel_path` hoisted out of the native backend to cfg-independent
  module level, applied in the PUBLIC cache API before the platform dispatch
  (wasm now validates identically, R1.3) and by `load_downloaded_mods`, which
  skips unsafe records with a warning; (b) the native source is wrapped in
  `SandboxedAssetReader`, rejecting any request with a non-Normal path
  component as not-found before the `FileAssetReader` raw-joins it.
  IMPORTANT empirical correction to the finding's premise: sabotaging ONLY the
  sandbox left the escaping-manifest e2e GREEN - bevy 0.19 already rejects
  escaping paths at load time (`AssetPlugin::unapproved_path_mode` defaults to
  `UnapprovedPathMode::Forbid`; `AssetServer` checks `is_unapproved()` at
  server/mod.rs:544, and `normalize_path` path.rs:692 preserves underflowing
  `..` per RFC 1808 exactly as the review said). So the claimed escape does
  NOT reproduce under default config; the sandbox is defense-in-depth so
  containment does not hinge on that default (Allow-configured apps,
  `load_override` callers, direct reader consumers). Tests: poisoned
  hand-written index -> only the safe record survives (sabotage-verified);
  escaping manifest + decoy outside the mods root -> load Failed, decoy
  scenario never registers (e2e, holds if EITHER guard holds - stated in its
  doc); the sandbox itself pinned at the reader layer by a unit test proving
  the RAW reader serves the escape and the sandboxed one refuses it
  (sabotage-verified: pass-through check -> FAILED).
- R1.2 (MINOR, id shadowing): a downloaded record whose id matches a SHIPPED
  catalog entry is skipped with a warning by BOTH `build_mod_catalog` (no row)
  and `register_bundles` (no merge) - "at load time" as the review suggested
  is not implementable natively (the shipped catalog asset is not loaded at
  Startup when the index is read), so the skip lives at the two consumers,
  which see the catalog. Documented in modding-ron-format.md; pinned by an
  e2e installing real gauntlet content under the shipped id "demo" (rows stay
  2 with shipped meta; shipped demo_mod_arena merges, downloaded gauntlet_run
  does not; sabotage-verified).
- R1.3 (MINOR): covered by R1.1(a) - validation before the cfg dispatch, and
  `is_safe_id` rejecting `/` excludes the `<id>/<path>` IDB key ambiguity
  (documented on the fn; unit test `shared_public_api_gate_rejects_...`).
- R1.4 (MINOR, IDB): open promise now wires `onblocked` to a rejection (a
  future DB_VERSION bump blocked by an old tab surfaces instead of wedging);
  every operation calls `db.close()` after settling (read_all_files no longer
  leaks N+1 connections to GC); `idb_put` documents that request-onsuccess is
  NOT transaction commit and 163508 must await transaction `complete` for its
  files-first-index-last discipline.
- R1.5 (NIT): the env override is absolutized (`std::path::absolute`) in
  `data_root()`, the single place it is read, so a relative
  NOVA_MOD_CACHE_ROOT cannot diverge between the FileAssetReader (exe-relative
  join) and the fs helpers (CWD join).
- R1.6 (NIT): the run condition is now one public system fn,
  `installed_set_changed` (EnabledMods-or-DownloadedMods is_changed), used by
  the plugin AND both test rigs. Bonus correctness: one reader consumes BOTH
  change ticks together, closing the `or_else` short-circuit artifact that had
  let a primed DownloadedMods tick re-fire on load timing - with it, the
  mark-system sabotage now deterministically fails the arrival e2e too (the
  docstring's earlier "does not fail" note was rewritten to match).
- R1.7 (NIT): documented in modding-ron-format.md - uninstall-while-enabled
  leaves the id in EnabledMods/prefs, so a reinstall comes back enabled;
  deliberate until 163508 decides pref-stripping.

Evidence after the round: `cargo test -p nova_assets --lib` 32 passed (8
mod_cache unit tests, 2 new); `--test mod_cache_install` 7 passed (3 new);
`--test demo_scenario` 11 passed; fmt clean; `cargo check --workspace
--all-targets` green; wasm gate re-run green (`cargo check --target
wasm32-unknown-unknown -p nova_assets -p nova_core`, no new warnings).
Sabotage runs (applied/reverted): record-filter removed -> poisoned-index
test FAILED; sandbox check pass-through -> reader unit test FAILED; merge
collision skip removed -> shadowing test FAILED; mark no-op -> arrival e2e
AND boundary test FAILED. content_ron_parity stays red from the pre-existing
master-side shakedown drift (unrelated, see the close-out above).
