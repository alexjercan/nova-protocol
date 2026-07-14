# Catalog-driven mod loading: mods.catalog.ron + InstalledCatalog asset + EnabledMods (base default-enabled) + enabled-subset merge

- STATUS: OPEN
- PRIORITY: 60
- TAGS: modding, spike

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
runtime resource). Foundation for the Mods menu (174126) and persistence (174131). `spike`
until planned.
