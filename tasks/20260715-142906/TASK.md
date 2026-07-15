# Mod download/cache/install runtime: ehttp fetch, native data-dir + wasm IndexedDB storage, mods:// asset source, installed index

- STATUS: OPEN
- PRIORITY: 16
- TAGS: modding,wasm

Spike: tasks/20260714-202515/SPIKE.md (options M, P, T)
Depends on: 20260715-142900 (portal - something real to download).

Goal: the game can download, cache, and load a portal mod on native AND wasm.
Pieces: an `ehttp`-based fetch layer driven from IoTaskPool tasks + a channel
resource, with a `PortalConfig` base URL (native default = the Pages portal
URL, wasm default derived from `window.location`, dev override via env var /
query param); STAGED installs (fetch all files to memory, verify sha256 via
`sha2`, then commit, index entry last); storage backends - native
`dirs::data_dir()/nova-protocol/mods/<id>/` + `installed.mods.ron` index, wasm
IndexedDB for file bytes (wrapper crate vs hand-rolled web-sys: decide in
/plan) + localStorage for the small index; a `mods://` asset source registered
before AssetPlugin (native FileAssetReader rooted at the cache dir, wasm
MemoryAssetReader over a shared Dir hydrated async from IndexedDB - verified
against bevy_asset 0.19); downloaded mods join the installed set (shipped
catalog + downloaded index) and fold in via the existing live re-merge, plus
uninstall (remove files + index entry + disable + re-merge). The riskiest
unknown is the wasm IndexedDB path - prototype it first. No UI here beyond
what testing needs; the Explore tab wires it up (20260715-142916).

