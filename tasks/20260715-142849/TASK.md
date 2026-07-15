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
shrinks toward a thin ordered pointer list (id, bundle path, base, hidden -
deployment flags stay catalog-level). Author metas for base, demo and
screenshot-reel. `dependencies` is schema-only here (resolution is its own
task). Update docs/modding-ron-format.md.

