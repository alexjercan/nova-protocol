# Base game as a bundle: convert hardcoded GameAssets content loading to a base bundle manifest

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: v0.6.0,modding,scenario,folded

Spike: tasks/20260714-113418/SPIKE.md

Goal: make the base game itself a bundle - "base game and a mod are the same shape."
Replace the hardcoded content entries in `GameAssets`
(`crates/nova_assets/src/lib.rs`: the `*.scenario.ron` / `base.sections.ron` handles)
with an `assets/base/bundle.ron` manifest listing the base sections/ships/scenarios,
loaded through the bundle loader (20260714-134119) into the registries. Raw
texture/gltf assets can stay in `GameAssets` or move to bundle asset-refs (already
`AssetRef` paths). Keep the smoke suite green. Gated on 20260714-134119. `spike`
until planned.

## Re-based v2 (20260714)

Re-based on the content-model bundle design (spike tasks/20260714-150410): "sections/
ships/scenarios" are all `Content` items (kind-in-data); the base/mod bundle is a folder
of `Content` files + a `bundle.ron` manifest, merged by kind via `register_content`.
Otherwise unchanged. Gated on the folder-bundle mechanism (20260714-134119).

## FOLDED into 134119 (20260714)

The base game becoming a bundle IS the bundle mechanism (134119) end-to-end proof, so
it is done there (like 150508 combined mechanism + migration), not as a separate step.
Nothing shipped here; the work moved to 134119.
