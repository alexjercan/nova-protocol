# Spike: main-menu mod manager - installed catalog, base as a default mod, enable/disable + persistence

- DATE: 20260714-174000
- STATUS: RECOMMENDED
- TAGS: spike, modding, menu, ui, wasm

## Question

How do we add a main-menu "Mods" section (Wesnoth add-ons / Factorio style) that
LISTS installed mods with enable/disable toggles and an "Explore online" (coming
soon) placeholder, where "base" is itself a mod that is enabled by default? A good
answer names the data model (how mods are discovered wasm-safely and how metadata
reaches the list), how "enabled" is represented and persisted, how a toggle takes
effect, and the menu wiring - concretely enough to plan without re-litigating.
Success: the `demo` mod appears in the list and can be enabled from the menu.

## Context

The modding pipeline already exists (tasks 150508 / 134119 / 134127):
- A mod is a folder BUNDLE: `*.bundle.ron` manifest -> `BundleAsset` (deps = its
  `*.content.ron` files -> `ContentAsset` -> `Content` items: sections, scenarios).
- `nova_assets::register_bundles` merges an ORDERED list of bundles into
  `GameSections` / `GameScenarios` via the pure `merge_bundles` (cross-bundle
  last-wins overlay by id; intra-bundle dup = logged conflict).
- Today `GameAssets` has TWO fields: `base_bundle` (hardcoded `base/base.bundle.ron`)
  and `mod_list` (`enabled.mods.ron` -> `ModList`, deps = enabled mod bundles). The
  demo mod ships under `assets/mods/demo/` but is disabled (`enabled.mods.ron` empty).

Hard constraints (from exploration):
- WASM cannot list directories (`load_folder` broken) -> discovery MUST be a manifest.
- WASM/native both load the `GameAssets` collection ONCE at startup; there is no hot
  reload. bevy_asset_loader loads each field UNTYPED, so any new manifest field needs
  a STEMMED name (`<name>.<ext>.ron`, e.g. `mods.catalog.ron`) or it fails in-game
  (see 163342).
- There is NO persistence anywhere: no file I/O, no `bevy_pkv`, no localStorage, no
  settings/save system. `assets/` is read-only (can't write the enable-list there).
- The menu (`nova_menu`) is native bevy_ui: one panel + modal overlays. Settings is a
  hidden panel toggled by `Visibility`. There is a reusable scroll-list pattern
  (`EditorScrollPanel` + `Overflow::scroll_y()` + a wheel system) and a data-driven
  list idiom (the editor palette iterates `GameSections` into `button(name)` rows).

The list must show INSTALLED mods, not just enabled ones - so the enable-list asset
(which only names enabled mods, and has no metadata) is insufficient. We need a
discoverable catalog of installed mods WITH metadata (name/description), and the
enabled state must move OUT of the read-only asset into a runtime (and persisted)
store.

## Options considered

### Discovery + metadata (how the list is populated)

- **A. Installed-mods CATALOG manifest (recommended).** A wasm-safe
  `assets/mods.catalog.ron` lists every INSTALLED mod as an entry with metadata:
  `{ id, name, description, bundle: "<path>.bundle.ron", base: bool }`. The menu reads
  the catalog to render the list WITHOUT loading any mod content. `base` is a catalog
  entry (`base: true`). Pros: one lightweight index, wasm-safe, metadata present
  up-front, base unified in. Cons: a second file to keep in sync with what is actually
  shipped (acceptable - it IS the manifest, same as every bundle manifest).
- **B. Metadata inside each `*.bundle.ron`.** Add a `meta: { name, description }`
  header to bundle manifests; the menu loads every installed bundle to read it. Cons:
  to LIST mods you must LOAD them all (loading ~= enabling), and you still need a
  wasm-safe list of installed bundle paths -> you end up needing (A) anyway. Rejected
  as the primary mechanism; a bundle self-meta header is a nice-to-have layered on top
  later.
- **C. Directory scan.** Rejected - broken on wasm.

### Which bundles load, and what "enabled" means

- **D. Catalog asset loads ALL installed bundles; merge only the ENABLED subset
  (recommended).** Make the catalog an ASSET (`InstalledCatalog`) whose
  `VisitAssetDependencies` visits EVERY cataloged bundle handle, so all installed mods'
  content loads at startup (gated by the recursive load state, exactly like `ModList`
  does for enabled mods today). A runtime `EnabledMods` set (of mod ids) decides which
  cataloged bundles `register_bundles` actually MERGES, in catalog order (base first).
  Pros: toggling is just a re-MERGE (no asset reload), so enabling a mod can take
  effect LIVE in-session - the goal is demoable without a restart; base unifies in as a
  default-enabled entry. Cons: disabled mods' assets still load (wasted for large mod
  counts). Fine at today's scale (base + demo); selective/lazy loading is a future
  optimization, noted, not silently dropped.
- **E. Load only ENABLED bundles (dynamic asset collection).** More faithful to a real
  mod manager (disabled mods cost nothing), but requires dynamic asset loading and a
  RESTART/reload to apply a toggle. Heavier and worse UX for the goal. Deferred - can
  become the optimization behind (D) later.

### Persisting the enabled set (survives restart, native + wasm)

- **F. Small hand-rolled cross-platform config store (recommended).** A tiny module
  (in `nova_core` or a new `nova_persist`): native writes a RON file under
  `dirs::config_dir()/nova-protocol/`, wasm reads/writes `window.localStorage` via
  `web-sys`. Store the enabled set (a `Vec<String>`/`HashSet<String>` of mod ids). Load
  at startup (default: base enabled), save on toggle. Pros: full control, no bet on a
  third-party crate supporting the bleeding-edge Bevy 0.19; small surface. Cons: we own
  the cfg-branching (two short impls).
- **G. `bevy_pkv` (or bevy-persistent).** Purpose-built cross-platform KV (native file,
  wasm localStorage). Pros: less code. Cons: Bevy 0.19 is brand-new; these crates
  likely lag its version, and a modding UI should not block on an external upgrade.
  Reconsider if one already tracks 0.19.
- **H. RAM-only (no persistence).** Enable state lost on restart. Meets the LITERAL
  goal in-session but not the spirit of a mod manager. Acceptable ONLY as the first
  increment while (F) lands right after.

### Menu wiring

- **I. Modal "Mods" panel (recommended).** Mirror the existing Settings panel: a
  `ModsPanel` marker, `Visibility::Hidden`, toggled by a new "Mods" main-menu button.
  Inside: a scrollable list (`EditorScrollPanel` pattern) of catalog entries, each a row
  with the mod name + an enable/disable toggle button whose label/color reflects
  `EnabledMods` (base row shown enabled + locked/greyed), a disabled "Explore online
  (coming soon)" button, and a Back button. A toggle observer flips `EnabledMods`,
  persists it, and triggers a re-merge. Fits the codebase's proven patterns exactly.

## Recommendation

**A catalog-driven mod manager: A + D + F + I.**

Data model: `assets/mods.catalog.ron` lists every installed mod with metadata and its
bundle path; `base` is an entry with `base: true`. An `InstalledCatalog` asset loads
all cataloged bundles at startup (recursive-gated). A runtime `EnabledMods` set (base
default-enabled) selects which cataloged bundles `register_bundles` merges, in catalog
order (base first, so mods overlay it). Toggling a mod in a modal "Mods" menu panel
updates `EnabledMods`, persists it via a small native-file/wasm-localStorage store, and
re-runs the merge live. This replaces today's `base_bundle` + `mod_list` GameAssets
fields with the single catalog, and makes "base is a mod" fall out of the model rather
than being special-cased.

Why it beats the runners-up: the catalog (A) is the only wasm-safe way to LIST installed
mods with metadata (B/C don't); load-all-merge-enabled (D) makes the goal demoable LIVE
and unifies base in, where dynamic loading (E) would force a restart; the hand-rolled
store (F) avoids betting the feature on a third-party crate tracking Bevy 0.19. It reuses
the existing `merge_bundles` overlay and the proven Settings-panel + editor-list UI
idioms, so each piece is a known pattern, not new machinery.

Sequencing keeps every step shippable and behaviour-preserving: the catalog+merge
refactor (task 1) leaves startup behaviour IDENTICAL (only base enabled by default, demo
loaded-but-not-merged); the menu (task 2) is where the goal is MET (see demo, toggle it,
it merges live); persistence (task 3) hardens it across restarts.

## Open questions

- Live re-merge vs "restart to apply": recommendation is LIVE (re-run register_bundles on
  `EnabledMods` change). It affects FUTURE game loads / editor palette, not the
  already-running menu ambience scene - confirm that is acceptable UX (it should be; a
  toggled mod shows up when you start a New Game). If a mod ever needs to change the live
  menu scene, revisit.
- Is `base` disableable? Recommendation: show it enabled + LOCKED (no toggle) - disabling
  the base game is a footgun. Revisit if total-conversion mods want to replace base
  wholesale (then base becomes a normal, disableable entry).
- Does the catalog REPLACE `enabled.mods.ron`/`ModList`, or coexist? Recommendation:
  REPLACE (enabled state moves to the runtime `EnabledMods` + persisted store; the asset
  becomes the INSTALLED catalog). The demo bundle and `merge_bundles` are reused as-is.
- Persistence location/format details (config dir name, localStorage key, RON schema) -
  settle in task 3's plan.

## Next steps

Direction-level tasks this spike seeded, for `/plan` to break into steps:

- tatr 20260714-174120: catalog-driven loading - `mods.catalog.ron` + `InstalledCatalog`
  asset (loads all cataloged bundles) + `EnabledMods` resource (base default-enabled) +
  `register_bundles` merges the enabled subset in catalog order + re-merge on change.
  Replaces the `base_bundle` + `mod_list` GameAssets fields. Startup behaviour identical.
- tatr 20260714-174126: the "Mods" main-menu panel - a modal `ModsPanel` (Settings-panel
  pattern) listing catalog entries with enable/disable toggles bound to `EnabledMods`
  (base shown enabled + locked), an "Explore online (coming soon)" disabled placeholder,
  and Back. Toggling re-merges live. THE GOAL is met here (see demo, enable it).
- tatr 20260714-174131: persist `EnabledMods` cross-platform - a small native-file /
  wasm-localStorage store; load at startup (default base-enabled), save on toggle.

## Fix record

- 20260714, catalog-driven loading (174120) landed on master (`6c4f455`): replaced the
  `base_bundle` + `mod_list` `GameAssets` fields with one `mods.catalog.ron` ->
  `InstalledCatalog` asset (nova_modding: `ModEntry`/`CatalogManifest`/`CatalogEntry` +
  `CatalogLoader`, `catalog.ron` ext). The catalog visits every installed bundle so all
  load at startup (recursive-gated); a runtime `EnabledMods` set (seeded from `base:true`
  entries, idempotent) selects which merge, in catalog order (base first), re-merging live
  on change (`resource_changed`). `ModList`/`enabled.mods.ron` removed. "base is a mod"
  is now data; startup behaviour identical (base only). Reviewed APPROVE (out-of-context,
  no defects). Next: 174126 (Mods menu - meets the goal), then 174131 (persistence). See
  tasks/20260714-174120/{TASK,REVIEW,RETRO}.md.
- 20260714, Mods menu section (174126) landed on master (`efa2523`): THE GOAL is met -
  the demo mod is in the main-menu Mods list and enableable. Added a `ModCatalog` resource
  (nova_assets, metadata built from the catalog at Processing) + re-exported `ModEntry`; a
  modal `ModsPanel` in nova_menu (Settings-panel pattern) lists installed mods with
  enable/disable toggles bound to `EnabledMods` (base shown locked), a coming-soon "Explore
  online" placeholder, and a scrollable list. Toggling flips `EnabledMods` -> 174120's
  re-merge applies it live (updates GameScenarios for the next New Game). Reviewed APPROVE
  (out-of-context, one pre-existing modal-overlap UX nit deferred). Remaining: 174131
  (persist the enabled set across restarts). See tasks/20260714-174126/{TASK,REVIEW,RETRO}.md.
- 20260714, EnabledMods persistence (174131) landed on master (`f6742fa`): the LAST task -
  the mod manager is complete. Added `nova_assets::mod_prefs` (native RON file under
  `dirs::config_dir()/nova-protocol/`, wasm `localStorage`) + `load_enabled_mods`
  (start of Processing) / `save_enabled_mods` (on change); `seed_enabled_mods` now unions
  base ids in. Verified in-game with a temp XDG_CONFIG_HOME (writes [base], honors saved
  [base,demo]); wasm checked against the web-sys 0.3 API (not built by automated CI).
  Reviewed APPROVE (out-of-context, no defects; a comment-accuracy MINOR + a test NIT
  fixed). FAMILY COMPLETE: enabling the demo mod from the menu now persists across
  restarts on native and web. See tasks/20260714-174131/{TASK,REVIEW,RETRO}.md.
