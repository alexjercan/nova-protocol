# Catalog-driven mod loading: mods.catalog.ron + InstalledCatalog asset + EnabledMods (base default-enabled) + enabled-subset merge

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: modding, menu

Spike: tasks/20260714-174000/SPIKE.md

Goal: replace the `base_bundle` + `mod_list` `GameAssets` fields with a single
wasm-safe INSTALLED-mods catalog. `assets/mods.catalog.ron` lists every installed mod
as `{ id, name, description, bundle: "<path>.bundle.ron", base: bool }`; `base` is a
catalog entry (`base: true`). An `InstalledCatalog` asset visits (loads) EVERY cataloged
bundle at startup (recursive-gated, like `ModList` does). A runtime `EnabledMods` set of
mod ids (base enabled by default) selects which cataloged bundles `register_bundles`
MERGES, in catalog order (base first). Re-run the merge when `EnabledMods` changes so a
toggle applies live. Startup behaviour IDENTICAL: only base enabled by default, the demo
mod loaded-but-not-merged. The demo bundle + `merge_bundles` are reused as-is; the
`enabled.mods.ron`/`ModList` enable-list-as-asset is REPLACED (enabled state moves to the
runtime resource). Foundation for the Mods menu (174126) and persistence (174131).

## Plan (20260714)

Data model: `mods.catalog.ron` = `(mods: [ (id, name, description, bundle, base), ... ])`.
Stemmed name -> bevy full extension `catalog.ron` (untyped-load-safe, per 163342). `base`
is the first entry (`base: true`); `demo` follows. Both bundles LOAD at startup; only the
ENABLED ones MERGE.

Naming/lessons applied: `stemmed-compound-extension` (mods.catalog.ron -> `catalog.ron`
loader ext), `test-the-production-load-path` (untyped-load guard + drive the real system
not just the pure fn), `registered-system-for-change-detection` (re-merge test uses a real
App, not run_system_once).

Steps:
- [x] 1. nova_modding: add `ModEntry { id, name, description, bundle: String, base: bool }`
  + `CatalogManifest { mods: Vec<ModEntry> }` + `InstalledCatalog { entries: Vec<CatalogEntry> }`
  where `CatalogEntry { meta: ModEntry, bundle: Handle<BundleAsset> }`, + `CatalogLoader`
  (extension `catalog.ron`) that parses the manifest and `load_context.load::<BundleAsset>`s
  each entry's (asset-root-relative) bundle path. `InstalledCatalog`'s
  `VisitAssetDependencies` visits EVERY entry's bundle handle (so all installed bundles
  load, recursive-gated). Register in `NovaModdingPlugin`. REMOVE `ModList`/`ModListLoader`/
  `ModListManifest` (superseded). Update prelude. Unit test: a `catalog.ron` body decodes
  into `CatalogManifest` (incl. `base: true` and default `base` omitted -> serde default
  false).
- [x] 2. Author `assets/mods.catalog.ron` listing base (`base: true`, bundle
  `base/base.bundle.ron`) then demo (`base: false`, bundle `mods/demo/demo.bundle.ron`),
  each with name+description. DELETE `assets/enabled.mods.ron` (replaced).
- [x] 3. nova_assets: `GameAssets` - REPLACE `base_bundle` + `mod_list` with one
  `#[asset(path = "mods.catalog.ron")] pub catalog: Handle<InstalledCatalog>`. Add a
  `#[derive(Resource, Default)] EnabledMods(pub HashSet<String>)` (mod ids).
- [x] 4. nova_assets: at `OnEnter(Processing)`, BEFORE the merge, init `EnabledMods` from
  the catalog - default-enable every `base: true` entry (idempotent: only seed if empty, so
  a later persistence load (174131) or menu toggle isn't clobbered). Then `register_bundles`
  reads the catalog + `EnabledMods`: build the ordered bundle list = catalog entries in
  order whose id is in `EnabledMods`, map to their bundle handles, `merge_bundles`. base is
  enabled+first, so startup is identical (base only; demo loaded, not merged). Keep
  `error!`+skip on a missing/unloaded asset.
- [x] 5. nova_assets: re-merge on change - run `register_bundles` again whenever
  `EnabledMods` changes (run condition `resource_changed::<EnabledMods>`, in `Update`
  during/after Loaded), so a menu toggle (174126) re-populates GameSections/GameScenarios
  live. Guard against the initial-insert double-run (or accept it - idempotent).
- [x] 6. Migrate tests (crates/nova_assets/tests/demo_scenario.rs): update the `GameAssets`
  literals (catalog field, drop base_bundle/mod_list); replace the ModList-based
  `register_bundles_applies_enabled_mods` with a catalog+`EnabledMods` version; replace the
  `enabled.mods.ron` untyped guard with a `mods.catalog.ron` untyped guard; keep the base
  parity test. Add: (a) unit - `EnabledMods={base}` merges base only (no `demo_mod_arena`),
  `EnabledMods={base,demo}` merges both (hull override + `demo_mod_arena`); (b) an App-level
  re-merge test - toggling `EnabledMods` re-runs the system and updates `GameScenarios`
  (real App + `resource_changed` run condition, per the change-detection lesson).
- [x] 7. Docs: update `docs/modding-ron-format.md` (catalog replaces the enable-list; the
  `catalog.ron` stem rule) and the nova_modding module doc.
- [x] 8. Verify: `cargo test --workspace --no-run`; nova_modding/nova_assets tests; parity;
  `12_menu_newgame` + `09_editor` headless (behaviour IDENTICAL - only base enabled). Also
  a manual check: temporarily seed `EnabledMods` with `demo` and confirm `demo_mod_arena`
  registers (proving the live path before the menu exists).
