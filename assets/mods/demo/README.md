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

Edit `assets/enabled.mods.ron` to list this bundle:

```ron
(mods: ["mods/demo/demo.bundle.ron"])
```

Then run the game - the section palette shows the buffed "Reinforced Hull Section
(Demo Mod)" and `demo_mod_arena` is available. Set it back to `(mods: [])` to
return to the pristine base game.

## How it loads

`enabled.mods.ron` -> `ModList` (its dependencies are the enabled mod bundles) ->
each `demo.bundle.ron` -> `BundleAsset` (its dependencies are the content files)
-> each `mod.content.ron` -> `ContentAsset`. bevy gates on the recursive load
state, then `register_bundles` (in `nova_assets`) merges base-then-mods by id.

Naming note: manifests must be stemmed (`demo.bundle.ron`, not `bundle.ron`;
`enabled.mods.ron`, not `mods.ron`) - see `docs/modding-ron-format.md`.
