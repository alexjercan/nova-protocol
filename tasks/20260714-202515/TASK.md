# Spike: remote mod catalog + download/cache (replace hand-maintained mods.catalog.ron; powers Explore)

- STATUS: OPEN
- PRIORITY: 18
- TAGS: spike, v0.6.0, modding, menu, wasm

Related: spike tasks/20260714-174000 (the mod-manager family); the "Explore online
(coming soon)" placeholder in the Mods menu (task 20260714-174126) is the UI this
would make real. Builds on the catalog model from 20260714-174120.

## Why (user request, 20260714)

Today `assets/mods.catalog.ron` is a HAND-MAINTAINED manifest: every installed mod
must be registered there by editing the file. That friction should go away. The
vision:

- The catalog (or an "available mods" catalog) could be a STATIC file hosted on a
  server, DOWNLOADED when the user hits "Explore" in the Mods menu - so the game
  discovers available mods online instead of shipping a fixed list.
- When a user DOWNLOADS a mod, it (and the catalog entry) gets CACHED locally, and
  enable/disable then operates on the cached/installed set. So a downloaded mod
  becomes "installed" without hand-editing `mods.catalog.ron`.
- This lands together with the actual "Explore" feature (turning the coming-soon
  placeholder into a working remote-browse + install flow).

## Question the spike must answer

How should mod discovery + acquisition work end to end, distinguishing:
1. INSTALLED mods (local, cached, what you can enable/disable now) - and how the local
   installed set is discovered WITHOUT a hand-maintained manifest and WITHOUT directory
   enumeration (wasm can't list dirs), and how it is cached/persisted (native FS vs
   wasm - localStorage is small; IndexedDB / OPFS / Cache API may fit bundle content).
2. AVAILABLE-REMOTE mods (the "Explore" catalog) - a static file fetched from a server
   (URL/CDN), its schema, versioning, and how the game fetches it (native + wasm
   `fetch`), plus trust/validation of downloaded content.
3. DOWNLOAD + INSTALL - fetching a mod's bundle + all its content files (the bundle is a
   folder of `*.content.ron`; how do we enumerate/fetch them - a per-mod manifest the
   server serves?), writing them into the local cache, and registering the mod into the
   installed set so `register_bundles` can merge it. All wasm-safe.

## Context / constraints

- Current model (tasks 150508 / 134119 / 134127 / 174120-174131): a mod is a folder
  BUNDLE (`*.bundle.ron` manifest -> `*.content.ron` files); `mods.catalog.ron` is the
  installed catalog (`InstalledCatalog` asset, loads every listed bundle); `EnabledMods`
  (persisted via `nova_assets::mod_prefs`) selects which merge; the Mods menu toggles it.
- WASM is a hard constraint throughout: no directory listing; assets are read-only and
  bundled; runtime downloads must use `fetch`/`web-sys`; writable storage is
  localStorage (tiny) / IndexedDB / OPFS / the Cache API. Native has the filesystem.
- The bevy `AssetServer` loads by path from the configured source; a downloaded mod that
  lives OUTSIDE the bundled `assets/` needs a way to be loaded (a custom asset source /
  reader, an in-memory source, or writing into a runtime-served dir) - a real unknown to
  resolve in the spike.

## Explore (spike, do not build)

- Local installed discovery: keep a small WRITABLE `installed.mods.ron`-style index
  (native file / wasm storage) that download-install appends to, replacing the shipped
  read-only `mods.catalog.ron` for the installed set. (Auto-scan is wasm-hostile.)
- Remote catalog: fetch a static `catalog.ron`/JSON from a configurable base URL; cache
  it; the Explore panel lists it, marking already-installed entries.
- Download: fetch a per-mod manifest + its content files into the local cache; register a
  custom bevy asset source so `BundleAssetLoader`/`ContentAssetLoader` can load cached
  bundles by a virtual path. Prototype the wasm storage path (IndexedDB/OPFS/Cache API)
  since it is the riskiest unknown.
- Trust/versioning: mod ids, versions, integrity (hash?), and what happens on a catalog
  or mod update. Note but likely defer.

## Deliverable

A SPIKE.md with a recommended architecture (installed vs remote catalog split; the
wasm-safe download/cache mechanism; how a cached mod loads through the asset system) and
the seeded implementation tasks - which LAND together with the Explore feature (the
coming-soon button becomes real). May conclude "phase it" (local-index-first, remote
second). `spike` until planned; backlog until the Explore feature is scheduled.
