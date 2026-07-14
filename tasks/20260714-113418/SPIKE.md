# Spike: typed multi-file content bundles (sections/ships/scenarios merged by kind)

- DATE: 20260714-113418
- STATUS: SUPERSEDED-IN-PART
- TAGS: spike, modding, scenario, bundle

> SUPERSEDED IN PART by `tasks/20260714-150410/SPIKE.md` (v2, 20260714). This doc's
> "merge by kind + manifest + wasm-no-load_folder + overlay" conclusions still hold,
> but its recommendation that a file declares its kind by EXTENSION (option A) is
> OVERRIDDEN: v2 puts the kind flag IN the RON (a `Content` enum), one generic loader +
> router, and builds that generic foundation FIRST so no kind needs a bespoke catalog.
> Follow v2's task ordering (foundation 150508 first). Trust v2 over the "how kind is
> declared" / task-ordering parts below.

## Question

How should Nova Protocol load a MOD - a pile of typed content files (a ship, a
section, a scenario, ...) that each declare their kind and get merged into the right
registries (Wesnoth's units/scenarios/maps model), such that "the base game" and "a
mod" are the same shape? The real uncertainties: how a file self-identifies its
kind, how a bundle's files are discovered (wasm-safely), and how ids namespace /
overlay across bundles. A good answer names a concrete loader + router + overlay
design a planner can build, and folds in the ship-prototype mechanism (from the
closed 113414).

## Context

The RON modding format shipped (docs/modding-ron-format.md). Content is loaded per
kind by extension: `*.scenario.ron` -> `ScenarioAsset` -> `GameScenarios`;
`*.sections.ron` -> `SectionCatalogAsset` -> `GameSections` (task 113408). Section
PROTOTYPE references + component modifications shipped (113411): a scenario's ship
sections reference catalog sections by id, resolved at spawn.

Two facts constrain the answer:

1. **The base game loads content by HARDCODED paths.** `crates/nova_assets/src/lib.rs`
   `GameAssets` is a `bevy_asset_loader` collection with `#[asset(path = "...")]` for
   each scenario/catalog file, loaded at `Loading` -> `Processing`, then
   `register_sections`/`register_scenario` populate the id-keyed registries. There is
   no directory enumeration today - which is why it is wasm-safe, but also why it
   can't discover files a modder drops in.
2. **wasm cannot list directories.** Bevy's `AssetServer::load_folder` / directory
   reading fails on the web ("Reading directories is not supported with the
   HttpWasmAssetReader"; bevy #9591/#10459/#5827). The only wasm-safe way to know a
   bundle's files is an explicit list - i.e. a MANIFEST. This is decisive: the game
   ships wasm and modding must stay wasm-safe.

Ship prototypes (113414, folded here): no built-in ship is reused, so ships want to
be a CONTENT KIND in this model rather than a bespoke standalone catalog.

## Options considered

### How a file declares its kind

- **A. By extension (recommended).** `*.sections.ron`, `*.ship.ron`, `*.scenario.ron`
  - reuses the pattern already shipped. A manifest entry `load_context.load(path)`
  yields the typed asset (its concrete `Asset` type identifies the kind). New kind =
  new extension + loader + registry arm. Pro: zero new machinery, consistent. Con:
  kind is in the filename, not the content.
- **B. Content wrapper enum.** Every content file is one `BundleEntry { Section(..),
  Ship(..), Scenario(..) }` behind a single loader/extension. Files self-identify by
  content. Pro: a loose `.ron` needs no special extension. Con: one giant asset type,
  loses per-kind loaders/registries, and a big enum to grow - fights the clean
  per-kind assets already built.
- **C. Manifest declares each file's kind.** The manifest maps `path -> Kind`. Pro:
  explicit. Con: redundant with the extension; more manifest to maintain.

### How a bundle's files are discovered

- **D. Manifest (recommended, wasm-safe).** A bundle is a directory with a
  `bundle.ron` manifest listing its content files (relative paths). A bundle loader
  reads it and `load_context.load`s each part as a dependency. Works identically
  native + wasm.
- **E. `load_folder` directory enumeration.** Convenient natively; BROKEN on wasm
  (see context 2). Rejected - violates the wasm constraint.
- **F. Build-time generated asset index.** A build step emits the file list. Extra
  toolchain; a manifest is simpler and modder-authorable.

### id namespacing / overlay across bundles

- **G. id-keyed registries + load-order overlay (recommended).** Registries stay
  `HashMap<Id, T>` (`GameSections`, `GameShips`, `GameScenarios`). Bundles load in
  order (base first, then enabled mods); a later bundle's id overrides an earlier one
  (mod-overrides-base), and an intra-bundle duplicate id is a hard error. Simple,
  matches "a mod overlays the base."
- **H. Namespaced ids (`modname:id`).** No collisions, but every reference must
  qualify, and cross-mod references get awkward. Heavier; defer unless collisions
  bite.

## Recommendation

**Manifest-driven, extension-typed content bundles merged by kind into id-keyed
registries (A + D + G), with ship prototypes as a first content kind.**

Concretely:

1. **Ship-prototype content kind (folds 113414, can land first).** Add `GameShips`
   (`HashMap<ShipId, SpaceshipConfig>`), a `*.ship.ron` loader (a `SpaceshipConfig`
   prototype, whose sections already use section-prototype refs), and a
   `ShipSource = Inline(SpaceshipConfig) | Prototype(ShipId)` on the scenario's
   `ScenarioObjectKind::Spaceship`, resolved at spawn against `GameShips` - exactly
   the section model (113411) one level up. Ship-level modifications reuse the
   component-observer model (a `ShipModification` analogue on the ship root, inert
   where N/A). This is independently shippable and is the ship "kind" the bundle
   loads.
2. **Bundle: manifest + loader + kind-router.** A `bundle.ron` manifest lists content
   files (relative paths). A bundle loader `load_context.load`s each -> its typed
   asset (`SectionCatalogAsset` / `ShipAsset` / `ScenarioAsset`) by extension. A
   `merge_bundle` step routes each loaded asset into its registry by kind (sections ->
   `GameSections`, ships -> `GameShips`, scenarios -> `GameScenarios`). Adding a kind
   is one new arm. wasm-safe (no dir enumeration).
3. **Base game IS a bundle.** Replace the hardcoded `GameAssets` content entries with
   `assets/base/bundle.ron` listing the base sections/ships/scenarios; the base is
   loaded through the same bundle path. (Raw texture/gltf assets can stay in
   `GameAssets` or move to bundle asset-refs - already `AssetRef` paths.)
4. **Mods = more bundles, overlaid.** A wasm-safe top-level `mods.ron` (or a setting)
   lists enabled mod-bundle manifests; each is loaded after the base and merged by
   kind with load-order overlay (later id wins; intra-bundle dup = error). Native may
   optionally enumerate a `mods/` dir for convenience, but `mods.ron` stays the
   wasm-safe source of truth. Ship a demo mod that overrides one section and adds one
   scenario to prove overlay end-to-end.

Why this beats the runners-up: extension-typing (A) reuses the shipped per-kind
assets/loaders instead of a monolithic `BundleEntry` (B); the manifest (D) is the
only wasm-safe discovery mechanism (E is broken on web); load-order overlay (G) is
the simplest model that delivers "mod overrides base" without qualifying every id
(H). It realizes the user's "base game and a mod are the same shape" and generalizes
prototype+catalog to every content kind rather than one bespoke catalog each.

## Open questions

- **Manifest schema.** Just a `Vec<PathBuf>` (kind by extension), or `{ name, version,
  depends_on, files }`? Start minimal (files list); add metadata when mod-deps appear.
- **When does merge run / hot-reload.** Merge at load (like `register_*` today) is
  enough; live mod reload is a later nicety.
- **Non-content assets in a bundle** (a mod's own textures/gltf): resolved by
  `AssetRef` path relative to the assets root already; confirm mod-relative paths.
- **Removal/patch semantics** beyond override (a mod that DISABLES a base scenario) -
  defer until asked.
- **Ship-modification starter set** - resolve when planning task 1.

## Next steps

Direction-level tasks (for `/plan`). Task 1 is independently shippable; 2 is the core;
3-4 build on 2.

- tatr 20260714-134115: ship-prototype content kind (GameShips + `*.ship.ron` +
  ShipSource + ship-modifications) - folds 113414.
- tatr 20260714-134119: bundle manifest + loader + merge-by-kind router into the
  id-keyed registries.
- tatr 20260714-134123: base game as a bundle (convert the hardcoded GameAssets
  content loading to the base bundle manifest).
- tatr 20260714-134127: mod loading + load-order overlay + a demo mod (override a
  section, add a scenario) proving base+mod merge.

## Fix record

(Appended by each implementing task as it lands.)
