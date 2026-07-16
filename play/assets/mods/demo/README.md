# Demo mod

A minimal example mod that demonstrates the bundle-overlay mechanism. A mod is
the same shape as the base game: a folder with a `*.bundle.ron` manifest listing
its `*.content.ron` files.

This mod:

- OVERRIDES the base `reinforced_hull_section` (same id) with more health and a
  renamed label - last-wins overlay by id, so the mod's version replaces the
  base's everywhere the section is used (editor palette, ships).
- ADDS a new scenario `demo_mod_arena` (a fresh id) alongside the base scenarios.

## How to enable

This mod is listed in `assets/mods.catalog.ron` (the installed-mods catalog), so it
is INSTALLED but disabled by default. Enable it from the main-menu **Mods** section -
the section palette then shows the buffed "Reinforced Hull Section (Demo Mod)" and
`demo_mod_arena` becomes available. Toggle it back off to return to the pristine base
game.

## How it loads

`mods.catalog.ron` -> `InstalledCatalog` (its dependencies are EVERY installed mod's
bundle) -> each `demo.bundle.ron` -> `BundleAsset` (deps = its content files) -> each
`mod.content.ron` -> `ContentAsset`. bevy gates on the recursive load state so all
installed bundles load. `register_bundles` (in `nova_assets`) then merges only the
ENABLED subset (`EnabledMods`) by id, base first - so enabling/disabling a mod is a
live re-merge, not a reload.

Naming note: manifests must be stemmed (`demo.bundle.ron`, not `bundle.ron`;
`mods.catalog.ron`, not `catalog.ron`) - see `web/src/wiki/dev/modding-ron.md`.
