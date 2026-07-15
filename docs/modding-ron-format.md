# RON scenario/mod format

The declarative modding language for Nova Protocol: scenarios authored as
`*.scenario.ron` data files instead of Rust. Implemented on branch
`modding-language` (v0.6.0). Spikes: `tasks/20260714-081636`,
`tasks/20260714-083224` (detailed design), `tasks/20260714-091336`
(crate boundary).

## What shipped

Scenarios are now serializable. A `*.scenario.ron` file deserializes into the
same `nova_scenario::ScenarioConfig` the runtime already used, and loads through
a Bevy `AssetLoader` into the `GameScenarios` resource.

- `nova_scenario` and `nova_gameplay` gained off-by-default `serde` features that
  `cfg_attr`-derive `Serialize`/`Deserialize` on the whole config tree (events,
  filters, the variables AST, actions, ship/section/object configs). Each engine
  crate is serde-free in isolation (`cargo build -p nova_scenario`). Note the
  SHIPPED game binary is not: `nova_modding` enables `nova_scenario/serde`
  unconditionally and the game depends on it (via `nova_assets`), so Cargo feature
  unification turns `bevy/serialize` on for the real build - the loader is always
  present and genuinely needs it. The cost (extra serialize/`Reflect` registrations)
  is small, but it is paid at runtime, not only in tests.
- `nova_gameplay::asset_ref::AssetRef<A>` is the authorable asset reference. In a
  data file an asset is a path string (`"textures/asteroid.png"`); `AssetRef`
  deserializes that as `Path`, and `resolve(&AssetServer)` turns it into a
  `Handle<A>` lazily at spawn. Resolution is non-mutating and idempotent, so a ref
  keeps its path and re-serializes (for editor save). Code-built configs use
  `AssetRef::from(handle)`. It replaced the 13 section-config handle fields plus
  the scenario cubemap and asteroid texture.
- `nova_modding` (new crate) owns the format: `ScenarioAsset`, the
  `ScenarioAssetLoader` for the `scenario.ron` extension, and `NovaModdingPlugin`.
  It depends on `nova_scenario` with its `serde` feature.
- `nova_assets` registers the plugin, loads `assets/scenarios/*.ron` as part of the
  `GameAssets` collection, and merges them into `GameScenarios`. See
  `assets/scenarios/demo.scenario.ron` for a worked, ship-less example.

## Architecture decisions

- **New crate + optional serde on the engine (not a parallel authoring tree).**
  The engine crates carry the serde derives behind a feature; `nova_modding` owns
  the loader and (future) authoring niceties. A full duplicate authoring type tree
  was rejected as too much drift-prone duplication. See `tasks/20260714-091336`.
- **`AssetRef` field type, not authoring wrappers.** One representation end to end.
  The only impedance was asset handles; wrapping just those in `AssetRef` (rather
  than mirroring every config struct) keeps duplication proportional to the actual
  mismatch. Resolution happens at the render-build observers, where an
  `AssetServer` is already in hand - behavior is identical to before.
- **Lazy resolution at spawn, not at load.** The loader is a pure RON decode; it
  does not walk the tree resolving handles. `AssetServer.load(path)` at spawn
  returns the same handle `GameAssets` holds for a matching path, so shared asset
  processing (e.g. the cubemap cube-view) is preserved.
- **Bindings via a `serde(with)` helper, runtime type unchanged.**
  `bevy_enhanced_input::Binding` has no serde impl. Rather than change
  `PlayerControllerConfig.input_mapping`'s runtime type (which would ripple into
  spawn and the editor), a small `BindingInput` authoring enum
  (`Keyboard`/`Mouse`/`Gamepad`) plus a `binding_map_serde` module (de)serialize
  the field through `BindingInput`, rejecting non-simple bindings (mod keys, mouse
  motion/wheel, axes) with an error.

## Difficulties and how they were resolved

- **The blocker set was bigger than "two handles".** Beyond the cubemap/texture
  handles, the section tree carried 13 asset handles, and three foreign non-serde
  types blocked the ship subtree: `FlightVerb`/`SectionConfig` (nova_gameplay,
  Reflect-only) and `Binding` (external). Resolved by adding serde to nova_gameplay
  and the `BindingInput` helper. This split the work into two tiers (logic/objects
  vs ships).
- **`AssetRef` generic-trait bounds.** Deriving `Clone`/`Debug`/`PartialEq` on
  `AssetRef<A>` would add an `A: Trait` bound and exclude `EffectAsset` (not
  `Debug`). The standard traits are hand-implemented without the bound.
- **`Asset` derive walks fields.** `#[derive(Asset)]` on `ScenarioAsset` would try
  to visit the wrapped `ScenarioConfig` for handle dependencies; since scenario
  refs are lazy `AssetRef` paths, `VisitAssetDependencies` is hand-implemented as a
  no-op and `Asset` implemented manually.
- **`AssetLoader` requires `TypePath`** on the loader struct in Bevy 0.19 - added.

## RON syntax notes (gotchas)

Authored shapes that are easy to get wrong by hand (generate them with
`ron::ser::to_string_pretty` on a code-built config if unsure):

- Asset ref: a bare string - `texture: "textures/asteroid.png"`.
- `Color`: externally-tagged - `color: Srgba((red: .., green: .., blue: .., alpha: ..))`.
- `Quat`: a bare 4-tuple - `rotation: (0.0, 0.0, 0.0, 1.0)`.
- Enum action/kind variants use RON newtype form - `DebugMessage((message: ..))`,
  `kind: Asteroid((..))`.

## Built-ins ported (task 20260525-133028, done)

All four built-ins are now data files under `assets/scenarios/` and load through
`nova_modding`; `register_scenario` builds none in code. The files are generated by
serializing the code configs with path-based `AssetRef`s (`SectionMeshRefs::from_paths`
+ the scenario builders taking asset refs), and a `scenario_ron_parity` test rebuilds
each and asserts it matches the committed file, so the data cannot silently drift from
the intended config. `menu_ambience`/`asteroid_field` use the seeded `ScatterObjects`
action instead of runtime RNG. Verified by the `12_menu_newgame` boot example.

## Mods: catalog + bundles + enabled set

The modding data model (tasks 150508 / 134119 / 134127 / 174120):

- A MOD is a folder BUNDLE: a `*.bundle.ron` manifest listing its `*.content.ron`
  files (`Content` items: sections, scenarios). The BASE game is just a mod
  (`assets/base/`).
- `assets/mods.catalog.ron` is the INSTALLED-mods CATALOG - a wasm-safe manifest (never
  a directory scan) listing every installed mod with metadata (`id`, `name`,
  `description`, `bundle`, `base`, `hidden`), base first. It loads as an
  `InstalledCatalog` asset whose dependencies are EVERY installed mod's bundle, so all
  installed content loads at startup regardless of what is enabled.
- `nova_assets::EnabledMods` (a runtime resource, not an asset) is the set of enabled
  mod ids. `register_bundles` merges only the enabled cataloged bundles, in catalog
  order (base first, so mods overlay it by id). Toggling it (from the main-menu Mods
  section) re-merges live. Base is enabled by default (`base: true`).
- `hidden: true` marks a DEV/TOOLING mod (e.g. `screenshot-reel`, the capture set for
  the website screenshots): `build_mod_catalog` filters it out of the player-facing
  `ModCatalog`, so it never appears in the Mods menu - but it stays installed, its
  bundle loads, and it merges like any other mod when its id is enabled (examples
  insert the id into `EnabledMods` directly, task 20260715-142844). A hidden mod's
  enablement is SESSION-ONLY: `seed_enabled_mods` strips hidden (non-base) ids from
  the restored prefs at startup, so an example run can never leave a hidden mod
  stuck-enabled with no menu row to disable it - examples re-enable by id each run,
  after that chain.

## File naming (bundles, content, catalog) - load-bearing

A bundle manifest MUST be named `<pack>.bundle.ron` (e.g. `assets/base/base.bundle.ron`),
content files `<name>.content.ron`, and the catalog `<name>.catalog.ron` (e.g.
`mods.catalog.ron`) - always a STEM before the compound extension, never a bare
`bundle.ron` / `catalog.ron`.

Why: `bevy_asset_loader` kicks off every collection field with an UNTYPED
`asset_server.load_untyped(path)`, which resolves the loader by the file's FULL
extension only. Bevy's full extension is everything after the FIRST dot in the file
name, so `bundle.ron` resolves to the bare `ron` extension (no loader) and the load
fails in-game with "Could not find an asset loader"; `base.bundle.ron` resolves to
`bundle.ron` (and `mods.catalog.ron` to `catalog.ron`), which the loader registers. A
TYPED load (`asset_server.load::<T>`) would fall back to the by-asset-type loader and
mask the problem - so tests must exercise the untyped path (see the
`catalog_untyped_load_resolves_the_loader` guard). Regression: task
`tasks/20260714-163342`.

## Known limitation: authoring verbosity

The generated files are large and repetitive - `shakedown_run.scenario.ron` is
~1480 lines because each ship inlines its whole section catalog, restating
`name`/`description`/mass/health/meshes per section. Faithful, but a poor
hand-authoring surface. Reducing this (a prototype+modifications model, sections as
their own RON, scenarios as multi-file bundles) is spiked in
`tasks/20260714-110502`. `ScatterObjects` is a first example of a declarative
primitive that collapses duplication.

## Still to do

- The editor scenario-builder (`tasks/20260714-081703`) can now serialize the
  authoring form for save/load.
- The RON-duplication spike (`tasks/20260714-110502`).

## Self-reflection

- Sequencing bottom-up (leaf serde -> AssetRef -> container serde -> loader ->
  wiring) with a green, committed increment at each step kept the workspace always
  buildable and made the big cross-cutting `AssetRef` change reviewable in
  isolation. Worth repeating for wide refactors.
- The single most useful de-risking move was generating RON via serde rather than
  hand-writing it; every hand-authoring gotcha above was avoided that way. Future
  content ports should lean on a small "dump this config to RON" helper.
- The scope discovery (foreign non-serde types) came only once serde was attempted;
  a quick `grep` for non-derive types in the config tree during the spike would
  have surfaced the two-tier split earlier.
