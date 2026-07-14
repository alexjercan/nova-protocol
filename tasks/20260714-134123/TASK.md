# Base game as a bundle: convert hardcoded GameAssets content loading to a base bundle manifest

- STATUS: OPEN
- PRIORITY: 36
- TAGS: v0.6.0,modding,scenario,spike

Spike: tasks/20260714-113418/SPIKE.md

Goal: make the base game itself a bundle - "base game and a mod are the same shape."
Replace the hardcoded content entries in `GameAssets`
(`crates/nova_assets/src/lib.rs`: the `*.scenario.ron` / `base.sections.ron` handles)
with an `assets/base/bundle.ron` manifest listing the base sections/ships/scenarios,
loaded through the bundle loader (20260714-134119) into the registries. Raw
texture/gltf assets can stay in `GameAssets` or move to bundle asset-refs (already
`AssetRef` paths). Keep the smoke suite green. Gated on 20260714-134119. `spike`
until planned.
