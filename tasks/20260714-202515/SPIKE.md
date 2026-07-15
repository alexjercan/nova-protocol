# Spike: remote mod catalog + download/cache + mods UI rework

- DATE: 20260714-202515
- STATUS: RECOMMENDED
- TAGS: spike, v0.6.0, modding, menu, wasm

## Question

How should mod discovery + acquisition work end to end, replacing the
hand-maintained `mods.catalog.ron` and making the "Explore online (coming soon)"
button real? Four sub-questions:

1. SERVING, FOR NOW: how are remote mods hosted with zero server budget, in a way
   that survives the later jump to a real server (Wesnoth add-ons-server style)?
2. FETCH + CACHE + LOAD: how does the game download a mod and load it on BOTH
   native and wasm (no directory listing, read-only bundled assets, no filesystem
   on the web)?
3. UI: what does the Mods menu become (Factorio/Wesnoth-style: installed list with
   quiet enable toggles, a details side panel, an online browse tab, room for
   dependencies)?
4. DEV-ONLY MODS (added mid-spike): how do tooling mods like `screenshot-reel`
   (used only by `examples/13_screenshot_reel.rs` to film website shots) stay OUT
   of the player-facing list without losing them for the examples?

A good answer names concrete mechanisms (schema, storage, asset-system wiring, UI
layout) so `/plan` can expand each seeded task without re-litigating.

## Context

What exists (spike 174000 family, all landed):

- A mod is a folder BUNDLE: `*.bundle.ron` (`BundleManifest { content: Vec<String> }`,
  paths relative to the manifest) -> `*.content.ron` files -> `Content` items
  (Section/Scenario). Loaders in `crates/nova_modding/src/lib.rs` key on compound
  extensions (`bundle.ron`, `content.ron`, `catalog.ron`) and are source-agnostic.
- `assets/mods.catalog.ron` (`CatalogManifest` -> `InstalledCatalog` asset) is the
  hand-maintained installed index: `{ id, name, description, bundle, base }` per
  entry. ALL cataloged bundles load at startup (recursive-gated); the runtime
  `EnabledMods` set picks which ones `register_bundles` MERGES (last-wins overlay
  by id, catalog order, base first). Toggles re-merge LIVE via
  `resource_changed::<EnabledMods>` (`crates/nova_assets/src/lib.rs:189`).
- `EnabledMods` persists via `nova_assets::mod_prefs`: native RON file under
  `dirs::config_dir()/nova-protocol/`, wasm `localStorage` (`web-sys`).
- The Mods menu (`crates/nova_menu/src/lib.rs:552`) is a modal panel: scroll list
  of catalog entries with toggle buttons, base locked, plus the inert "Explore
  online (coming soon)" button this spike replaces.
- Web deploy (`.github/workflows/deploy-page.yaml`): webpack builds `web/` to the
  site root; Trunk builds the game to `/play/` and `copy-dir`s the whole `assets/`
  folder verbatim. `public_url = "./"` so the game is subpath-independent. On wasm
  bevy's default asset source is `HttpWasmAssetReader` - every asset is ALREADY
  fetched over HTTP at load time; assets are not embedded in the wasm binary.
- No HTTP client crate anywhere yet; `web-sys` is used only for localStorage.

Verified against bevy_asset 0.19 (vendored source):

- `AssetApp::register_asset_source` exists (`bevy_asset-0.19.0/src/lib.rs:563`);
  sources must be registered BEFORE `AssetPlugin` is added.
- `bevy::asset::io::memory::{Dir, MemoryAssetReader}` exists; `Dir` is
  `Arc<RwLock<..>>` with `insert_asset(path, bytes)` - it can be registered empty
  at app build and hydrated later from async storage, and the reader sees the
  writes. This resolves the task's "how does a cached mod load" unknown.
- `examples/13_screenshot_reel.rs` enables its mod by mutating `EnabledMods`
  directly (`enable_reel_mod`), not through the menu - so hiding the mod from the
  menu cannot break the example.

Constraints: wasm cannot list directories; `assets/` is read-only; wasm writable
storage is localStorage (tiny) / IndexedDB / OPFS / Cache API; mods are data-only
RON (no scripting), so downloaded content's blast radius is bad game data, not
code execution.

## Options considered

### 1. Where mod metadata lives (name, description, author, version, deps, images)

- **A. `meta` block in `*.bundle.ron` (recommended).** The bundle manifest grows an
  optional `meta: ( name, description, author, version, dependencies: [ids],
  icon/screenshots: [paths] )` - the Factorio `info.json` analog. ONE per-mod
  source of truth: the shipped menu list reads metas from loaded bundles (all
  cataloged bundles already load), the portal generator reads the same meta to
  build the remote catalog, the details panel renders it. Catalogs shrink to thin
  ordered pointer lists. Deployment-level flags (`base`, `hidden`) stay in the
  catalog - they describe the INSTALL, not the mod.
- **B. Keep metadata in catalog entries (today's shape).** Works for the shipped
  list, but the portal catalog would need the same fields duplicated per mod, and
  a downloaded mod's details would have no authoritative in-mod source. Two hands
  to keep in sync - exactly the friction this spike exists to remove.
- **C. Separate `info.ron` per mod folder.** A third file per mod with no loader
  benefit; the bundle manifest is already the mod's front door. Rejected.

### 2. Serving remote mods, FOR NOW

- **D. Generated static portal on the existing GitHub Pages site (recommended).**
  Remote mod sources live in a repo-root `webmods/` folder (same bundle shape as
  `assets/mods/*`, NOT under `assets/` so they don't ship inside the game). A
  small workspace bin (reusing `nova_modding` types, so it validates by parsing
  the real formats) scans `webmods/`, verifies ids/deps/parses, computes sizes +
  sha256 per file, and emits `site/mods/catalog.json` plus the mod files under
  `site/mods/<id>/<version>/...`. One new deploy-workflow step. NOTHING is
  hand-maintained: the catalog is generated from bundle metas. Versioned paths
  make updates immutable-cache-friendly and leave room for keeping old versions.
- **E. Hand-maintained static JSON + files in `web/` or `dist/`.** The user's
  minimal sketch. Works, but reintroduces the hand-edited-manifest friction
  (sizes, hashes, file lists by hand) and drifts from the bundle metas. The
  generator (D) is a few hundred lines and kills that permanently. Rejected in
  favor of D, which is the same hosting with the manifest automated.
- **F. GitHub raw / release-asset URLs as the portal.** No deploy integration
  needed, but CORS and rate limits are less predictable, URLs are uglier, and it
  still needs a generated index somewhere. Rejected.
- **G. Real server now (API + DB).** Nothing today needs auth, uploads by third
  parties, or search at scale. Premature; see "the real server later" below -
  the static portal is designed to BE the v1 API contract.

### 3. Wire format for the remote catalog

- **H. JSON (recommended).** `serde_json` in-game (serde types shared with the
  generator); trivially consumed later by the TypeScript website (a web mods page
  can render the SAME catalog.json) and by a future REST API, which would return
  this exact shape. Matches the user's instinct.
- **I. RON.** Consistent with game assets, but hostile to web tooling and to any
  future non-Rust consumer. The catalog is a WIRE format, not a game asset.
  Rejected for the wire; mod content itself stays RON.

### 4. Mod transfer format (how a mod's files get to the client)

- **J. Per-file fetch from a generated file list (recommended).** Each catalog
  entry carries `files: [ { path, size, sha256 } ]` (the generator enumerates the
  real folder natively - the wasm can't-list-dirs problem evaporates because
  listing happens at PUBLISH time). Install = fetch each file, verify hash, store.
  Zero new packaging deps, works identically native/wasm, cache layout mirrors the
  mod folder, and binary files (icons) are just bytes. Mods today are 1-3 small
  RON files; request count is a non-issue.
- **K. Zip archive per mod (Factorio/Wesnoth style).** One URL, one hash, atomic,
  nicer for third-party uploads later. Costs a zip dependency working on wasm and
  a pack/unpack step now, for no present benefit. Deferred: the catalog schema
  gets an optional `package` field later; a real server can serve archives while
  clients that only know per-file still work off `files`.
- **L. Single packed RON blob (inline all content in one file).** No binary asset
  support (icons/screenshots), invents a format nobody else reads. Rejected.

### 5. HTTP client

- **M. `ehttp` (recommended).** Tiny, maintained (emilk), one API over native
  (ureq) and wasm (fetch), callback-based so it drives cleanly from a bevy
  IoTaskPool task + crossbeam channel resource. No bevy version coupling, so the
  spike-174000 "don't bet on crates tracking bleeding-edge Bevy" rule doesn't
  apply.
- **N. Hand-rolled cfg-split (ureq native + web-sys fetch wasm).** Full control,
  two implementations to own; ~200 lines for what M gives for free. Fallback if
  ehttp disappoints in `/plan`.
- **O. Abuse the asset system (register an HTTP source pointed at the portal).**
  bevy's wasm reader IS an HTTP fetcher, but native has none, there is no
  progress/error UI hook, and no persistence - it re-downloads every run.
  Rejected as the mechanism (noted as a possible wasm-only "preview before
  install" trick someday).

### 6. Wasm cache storage (the task's riskiest unknown)

- **P. IndexedDB for file bytes + localStorage for the installed index
  (recommended).** IndexedDB: universal browser support, large quota, binary
  blobs, async (fine - hydration is async anyway). Keys `mod-files/<id>/<path>`.
  The installed INDEX (small: id, version, bundle path per mod) goes in
  localStorage next to the existing `mod_prefs` key - same "small prefs sync,
  bulk bytes async" split the codebase already has. Access via a thin wrapper
  crate (`rexie` or `indexed_db_futures`; wasm-bindgen-level, no bevy coupling)
  or hand-rolled web-sys if we would rather own it - decide in `/plan`.
  Install is STAGED: fetch everything to memory, verify hashes, then write files,
  then the index entry last - a mid-install failure leaves only orphan bytes,
  swept on next start.
- **Q. OPFS.** File semantics match native nicely, but Safari's main-thread write
  support (`createWritable`) has been patchy; sync handles are worker-only.
  Avoidable risk. Rejected for now.
- **R. Cache API.** URL-keyed HTTP-response semantics fit poorly with
  install/uninstall bookkeeping, and entries are evictable. Rejected.
- **S. localStorage for everything.** ~5MB string-only quota; dies on the first
  mod with an icon. Rejected for content (kept for the tiny index).

Native storage needs no debate: files under `dirs::data_dir()/nova-protocol/mods/<id>/`,
index at `dirs::data_dir()/nova-protocol/installed.mods.ron` (data_dir for
content, config_dir stays for prefs).

### 7. Loading a cached mod through the asset system

- **T. Custom `mods://` asset source (recommended).** Register at app build (must
  precede `AssetPlugin`): native = `FileAssetReader` rooted at the mods data dir;
  wasm = `MemoryAssetReader` over a shared `Dir`, hydrated from IndexedDB by an
  async startup task (verified: `Dir` is Arc-shared, register-empty-fill-later
  works). Downloaded bundles then load as `mods://<id>/<bundle>` through the
  EXISTING loaders untouched (they key on extensions, not sources). The installed
  set becomes shipped-catalog entries + downloaded-index entries; downloaded mods
  fold in AFTER Processing via the existing live re-merge (extended to watch a
  `DownloadedMods` resource), so startup never blocks on storage hydration.
- **U. Write downloaded files into `assets/`.** Read-only on wasm and on most
  installed native layouts. Impossible.
- **V. Bypass the asset system (parse downloaded RON directly into registries).**
  Duplicates the loader/merge pipeline and diverges the two mod populations
  (shipped vs downloaded) forever. Rejected.

### 8. Dev-only mods (screenshot-reel)

- **W. `hidden: true` catalog flag (recommended).** One serde-default field on the
  shipped catalog entry; the menu's `ModCatalog` filters hidden entries out (or
  carries the flag and the UI skips them). The mod still ships, still loads, and
  the example still enables it by id through `EnabledMods` - zero example
  changes, players never see it. Cost: a few KB of RON ships to players; if a
  player hand-edits prefs to enable it, they get a harmless showcase scenario.
- **X. Separate dev catalog file (`mods.dev.catalog.ron`).** A second catalog
  asset loaded only in dev builds. More moving parts (two catalogs, build-flavor
  divergence in the load path) for the same result. Rejected.
- **Y. cfg(debug_assertions) gating of catalog entries.** Makes the shipped
  catalog parse differently per build profile; release-built examples silently
  lose the mod. Rejected.
- **Z. Examples sideload their bundle outside the catalog.** Cleanest separation
  but needs new one-off loading machinery only examples use. Rejected; W is one
  field.

### 9. Mods UI

- **AA. Two-pane Factorio-style Mods screen (recommended).** Replace the modal
  list with a larger panel: LEFT = tab bar (Installed | Explore online) + search
  box + scrollable rows (name, version, author; a quiet per-row enable checkbox
  on Installed; an installed/update-available badge on Explore). RIGHT = details
  panel for the selected mod from its bundle meta: title, author, version,
  description, dependencies, icon/screenshots (stretch), and the actions -
  Enable/Disable (installed, base locked), Install/Uninstall/Update (remote).
  Explore fetches the catalog on tab open (spinner; on failure an error + retry
  plus the last cached catalog with a "stale" note). Built from the proven
  idioms: `button()`/`observe()`, scroll panel, `theme::*`.
- **AB. Keep extending the current modal list.** Cheaper, but there is nowhere
  for details/images/actions to live, and Explore would be a second modal on a
  modal (the known overlap nit). Rejected - the rework IS part of the ask.

### 10. Dependencies

- **AC. Schema now, resolution next (recommended).** `dependencies: [ids]` (no
  version constraints yet) lives in bundle meta from day one so published mods
  declare them; the generator validates they exist. RESOLUTION ships as its own
  task: install pulls missing deps from the same catalog; enabling auto-enables
  deps (Factorio behavior); merge order becomes catalog-order-respecting
  topological. Version constraints (semver ranges) deferred until real demand.
- **AD. Full version-constraint solving now.** Factorio-grade; nothing in the
  ecosystem (three first-party mods) needs it. Rejected as premature.

## Recommendation

**A generated static portal on the Pages site + a `mods://` cache source in the
game + a two-pane mods screen - phased, with the dev-only `hidden` flag first.**

The architecture in one pass: mod metadata moves INTO each `*.bundle.ron` as a
`meta` block (A) - one source of truth for the menu, the details panel, and the
portal. Remote mods live in repo-root `webmods/`; a workspace generator bin
parses them with the real `nova_modding` types, and emits `catalog.json` (JSON
wire format, H; versioned schema; entries carry id/version/meta plus a generated
`files: [{path, size, sha256}]` list, J) and the files under
`site/mods/<id>/<version>/` on the existing GitHub Pages deploy (D). The demo mod
MOVES to the portal (removed from the shipped catalog, source stays in-repo as
the modding example) so Explore has a real mod on day one and every install
dogfoods the full path. In-game, `ehttp` (M) fetches catalog + files off the
IoTaskPool; installs are staged-then-committed into native `data_dir` files or
wasm IndexedDB + a small localStorage index (P); a `mods://` asset source (T;
native file reader, wasm memory `Dir` hydrated async from IndexedDB) lets the
existing loaders and the existing live re-merge treat downloaded mods exactly
like shipped ones. The Mods menu becomes a two-pane Installed|Explore screen
(AA) with quiet enable checkboxes and a details side panel rendered from bundle
meta. `screenshot-reel` gets `hidden: true` in the shipped catalog (W) and
disappears from players' menus while the examples keep working unchanged.
Dependencies get a schema field now and a resolution task later (AC).

Why this beats the runners-up: the generator (D over E) removes hand-maintenance
permanently rather than relocating it to a JSON file; per-file transfer (J over
K) needs zero new packaging machinery on wasm while the catalog schema keeps an
upgrade path to archives; IndexedDB (P over Q/R) is the only universally-safe
big-enough browser store; the `mods://` source (T) reuses every loader and merge
line already reviewed instead of forking the pipeline; and the `hidden` flag (W)
is a one-field fix verified against how the example actually enables the mod.

The jump to a real server later (Wesnoth-style add-ons server, done right): the
static portal IS the v1 API contract. A future service (e.g. axum + sqlite)
serves the SAME catalog JSON at the same relative endpoints - generated from a DB
instead of a folder scan - and adds what static hosting cannot: third-party
upload/publish (token auth; a `.pbl`-style publish manifest or in-game publish),
server-side validation (the generator's checks become the upload gate), download
counts, search/pagination past the one-file catalog, and mirrors. The client's
portal base URL is already a config (`PortalConfig`: native default = the Pages
URL, wasm default = derived from `window.location` so same-origin serving keeps
working, override via env var / query param for local dev), so pointing at the
real server - or at MULTIPLE catalog sources - is a config change, not a rework.
Plain HTTPS throughout; Wesnoth's custom-TCP campaignd protocol is the
cautionary tale, not the blueprint.

Trust/safety for now: sha256 verification of every downloaded file (`sha2`),
size caps from the catalog's declared sizes, id-collision rejection against the
shipped set, and the fact that mods are data-only RON (parse errors are already
logged-and-skipped; no code execution). Signing and publisher identity are
real-server-phase concerns, noted in the schema (`schema_version`) so old
clients can detect new catalogs.

Phasing (each step shippable): hidden flag -> bundle meta refactor -> portal
generator + deploy -> download/cache/install runtime -> mods screen rework ->
Explore tab (the coming-soon button dies here) -> dependency resolution.

## Open questions

- IndexedDB wrapper crate (`rexie` / `indexed_db_futures`) vs hand-rolled
  web-sys: decide in the runtime task's `/plan` after a look at current
  maintenance state. The wrapper is not bevy-coupled, so the usual
  version-tracking objection is weak.
- Icons/screenshots in the details panel: paths exist in bundle meta from the
  meta task, and installed mods can load them through their source; whether the
  Explore details panel fetches remote screenshots (ehttp -> `Image`) in v1 or
  ships text-only first is a UI-task scope call.
- Should Explore auto-refresh the catalog (TTL) or fetch only on tab open?
  Recommendation: on tab open + manual refresh button; revisit with a real
  server.
- Update UX when an installed mod's version differs from the catalog: v1 shows
  an Update button (exact string compare). Semver ordering and changelogs are
  future.
- Whether the shipped `mods.catalog.ron` should ALSO be generated (build.rs
  scanning `assets/mods/`): once demo moves online the shipped catalog is
  base + hidden dev mods and nearly never changes - not worth automating yet.

## Next steps

Direction-level tasks this spike seeded, for `/plan` to break into steps:

- tatr 20260715-142844: `hidden` catalog flag - screenshot-reel out of the
  player-facing Mods list, examples unchanged.
- tatr 20260715-142849: bundle `meta` block + thin catalogs - metadata moves
  into `*.bundle.ron`; `ModCatalog` builds from loaded bundle metas;
  `dependencies` field exists (unresolved).
- tatr 20260715-142900: static mod portal - `webmods/` source dir, generator
  bin (catalog.json + hashed file lists + versioned copies), deploy-workflow
  step, demo mod moves online.
- tatr 20260715-142906: download/cache/install runtime - ehttp fetch layer,
  native data-dir + wasm IndexedDB/localStorage storage, staged installs,
  installed index, `mods://` asset source, merge integration.
- tatr 20260715-142911: mods screen rework - two-pane Installed|Explore layout,
  search, quiet enable checkboxes, details side panel from bundle meta.
- tatr 20260715-142916: Explore online tab - fetch the portal catalog,
  install/uninstall/update actions, offline/stale handling; the coming-soon
  placeholder becomes real here.
- tatr 20260715-142931: dependency resolution - auto-install/auto-enable deps,
  topological merge order (schema landed with the meta task).

## Fix record

- 20260715, bundle meta (142849) landed on master (`60958111`): `ModMeta`
  (name/description/author/version/dependencies/icon/screenshots) authored in
  each `*.bundle.ron`, carried onto `BundleAsset`; the catalog thinned to
  declarations (id/bundle/base/hidden, `CatalogEntry.decl`); menu-facing
  `ModCatalog` is now `Vec<ModInfo>` (decl + bundle meta, id fallback name).
  One source of truth per mod - the portal generator (142900) and details panel
  (142911) read the same meta. Review APPROVE round 1 (two NITs fixed: icon
  `Some()` doc, menu render pin). NOTE mid-cycle user request became task
  20260715-151551: unship screenshot-reel from assets/ entirely (embed in the
  example; the hidden flag stays as a feature) - runs next, before 142900. See
  tasks/20260715-142849/{TASK,REVIEW,RETRO}.md.
- 20260715, hidden dev mods (142844) landed on master (`4a6d2615`): `ModEntry`
  gained `hidden: bool`; screenshot-reel is hidden from the player-facing
  `ModCatalog` (filtered in `build_mod_catalog`) while staying installed and
  enableable by id (the example's path, test-pinned). Review APPROVE round 1;
  its one MINOR added session-only semantics: `seed_enabled_mods` strips
  restored hidden ids so prefs pollution from example runs self-heals. Also
  fixed the red master CI (stale 2-entry catalog assertion). Next: 142849
  (bundle meta). See tasks/20260715-142844/{TASK,REVIEW,RETRO}.md.
