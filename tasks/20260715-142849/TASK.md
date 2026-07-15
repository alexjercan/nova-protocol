# Bundle meta block: mod metadata moves into bundle.ron, catalogs become thin pointers

- STATUS: OPEN
- PRIORITY: 18
- TAGS: modding

Spike: tasks/20260714-202515/SPIKE.md (option A)
Depends on: nothing (foundation for the portal + UI tasks).

Goal: make `*.bundle.ron` the single source of truth for a mod's metadata - the
Factorio info.json analog. Grow `BundleManifest` with an optional
`meta: ( name, description, author, version, dependencies: [ids], icon,
screenshots: [paths] )` block (serde defaults so existing bundles stay valid).
`ModCatalog` (the menu's view) builds name/description/etc from the LOADED
bundles' metas instead of catalog entries; the shipped `mods.catalog.ron`
shrinks to a thin ordered pointer list (id, bundle path, base, hidden -
deployment flags stay catalog-level). Author metas for base, demo and
screenshot-reel. `dependencies` is schema-only here (resolution is its own
task). Update docs/modding-ron-format.md.

## Plan (20260715)

Schema decided from the code (all consumers grepped):

- `ModMeta` (nova_modding, all fields `serde(default)`, derives Default):
  `name: String`, `description: String`, `author: String`, `version: String`,
  `dependencies: Vec<String>` (ids; schema-only until 142931), `icon:
  Option<String>`, `screenshots: Vec<String>` (paths relative to the bundle dir;
  consumed by the portal 142900 and details panel 142911, reserved here).
- `BundleManifest` gains `#[serde(default)] pub meta: ModMeta`
  (crates/nova_modding/src/lib.rs:91); `BundleAsset` gains `pub meta: ModMeta`
  (lib.rs:107), copied over by `BundleAssetLoader`. Existing meta-less bundles
  keep decoding (back-compat pin in the manifest test).
- `ModEntry` (the CATALOG entry, lib.rs:258) thins to `{ id, bundle, base,
  hidden }` - name/description move to bundle meta. `CatalogEntry.meta` renames
  to `decl` (it is the catalog DECLARATION - identity + deployment flags - not
  metadata anymore; consumers at nova_assets lib.rs:148-149, :176-179, :236 +
  tests).
- Menu-facing view: `ModInfo { id: String, base: bool, meta: ModMeta }` in
  nova_assets, built by a pure `ModInfo::new(decl: &ModEntry, meta:
  Option<&ModMeta>)` that falls back `name = id` when the meta name is empty
  (a mod without meta still renders a usable row). `ModCatalog` becomes
  `Vec<ModInfo>`; `build_mod_catalog` (nova_assets lib.rs:136) gains
  `Res<Assets<BundleAsset>>` and composes decl + the loaded bundle's meta
  (missing/unloaded bundle: `error!` + decl-only fallback row, never a panic -
  matches the file's existing error style). Menu consumers: nova_menu
  lib.rs:689 (`m.name`) and :698 (`m.description`) become `m.meta.*`;
  `ModToggle {id, base}` unchanged; two test literals update to `ModInfo`.
- Authored metas: base = name "Base Game", the catalog's old description,
  author "Nova Protocol", version left empty (base is versioned by the game
  itself; convention documented); demo + screenshot-reel = their old
  name/description, version "1.0.0". `dependencies` stay empty everywhere -
  base is an IMPLICIT dependency for now (documented; 142931 revisits).
- No third-party mods exist yet, so the catalog/bundle format break is free
  (CHANGELOG still notes it).

Steps:
- [ ] 1. nova_modding: add `ModMeta` (fields above, Default + serde defaults) +
  `meta` on `BundleManifest` and `BundleAsset`; `BundleAssetLoader` carries it
  through. Extend `bundle_manifest_ron_decodes`: a manifest WITH a meta block
  decodes every field; the existing meta-less body still decodes to
  `ModMeta::default()` (back-compat pin).
- [ ] 2. nova_modding: thin `ModEntry` to `{ id, bundle, base, hidden }`;
  rename `CatalogEntry.meta` -> `decl`. Update `catalog_manifest_ron_decodes`
  (drop name/description, keep base/hidden default assertions). Update the
  module doc + prelude (export `ModMeta`).
- [ ] 3. assets: author `meta` blocks in `base/base.bundle.ron`,
  `mods/demo/demo.bundle.ron`, `mods/screenshot-reel/screenshot-reel.bundle.ron`
  (values above, moved from the catalog); thin `assets/mods.catalog.ron` to
  pointers (id, bundle, base, hidden) and update its header comment.
- [ ] 4. nova_assets: `ModInfo` + pure `ModInfo::new` with the name-fallback;
  `ModCatalog(Vec<ModInfo>)`; `build_mod_catalog` composes catalog decls with
  loaded bundle metas (+ `Res<Assets<BundleAsset>>` param, decl-only fallback on
  a missing bundle). Rename the `e.meta.*` consumer sites to `e.decl.*`
  (`seed_enabled_mods`, `register_bundles`). Re-export `ModInfo`/`ModMeta` from
  the prelude; drop the now-unused `ModEntry` re-export if nothing outside
  nova_assets needs it (grep first).
- [ ] 5. nova_menu: rows read `m.meta.name` / `m.meta.description`; update the
  two `ModCatalog` test literals to `ModInfo` (crates/nova_menu/src/lib.rs:1415,
  :1423).
- [ ] 6. Tests (nova_assets): unit-test `ModInfo::new` (empty meta -> name
  falls back to id; authored meta passes through). Integration
  (tests/demo_scenario.rs): `mod_catalog_lists_installed_mods_metadata` now
  asserts demo's name/description are the strings AUTHORED IN
  demo.bundle.ron's meta (proving the plumbing reads the bundle, not the
  catalog) and still filters hidden; keep all 9 tests green.
- [ ] 7. Docs: modding-ron-format.md - bundle manifest section gains the meta
  block (fields + conventions: version is a plain semver-ish string, base dep
  implicit, icon/screenshots reserved for portal/details panel); catalog section
  updated to the thin pointer shape. CHANGELOG: Changed entry (mod metadata
  lives in bundle.ron; catalog slimmed; format break noted).
- [ ] 8. Verify: `cargo fmt --check`; `cargo check --workspace --all-targets`
  (ModEntry/ModCatalog literal breaks surface here); `cargo test -p
  nova_modding`, `-p nova_assets --test demo_scenario`, `-p nova_menu`. Full
  suite stays on CI.

## Notes

- Relevant files: crates/nova_modding/src/lib.rs (BundleManifest:91,
  BundleAsset:107, loaders:161-248, ModEntry:258, CatalogEntry/InstalledCatalog,
  decode tests at end), crates/nova_assets/src/lib.rs (ModCatalog:127,
  build_mod_catalog:136, seed:166, register_bundles:214+),
  crates/nova_menu/src/lib.rs (:689, :698, test literals :1415/:1423),
  assets/mods.catalog.ron, assets/**/[*.bundle.ron x3],
  docs/modding-ron-format.md:99+.
- `ModEntry {` literals outside nova_modding: only the two nova_menu test sites
  (become ModInfo). `.name`/`.description` consumers: only nova_menu :689/:698.
- Loader path/extension untouched - the untyped-load guard and
  stemmed-compound-extension rules are unaffected.
- Assumption: version stays an opaque string this task (exact-compare update
  badges come with 142916; semver ordering deferred).

