# Bundle manifest + loader + merge-by-kind router into id-keyed registries

- STATUS: CLOSED
- PRIORITY: 48
- TAGS: v0.6.0,modding,scenario

Spike: tasks/20260714-113418/SPIKE.md

Goal: the core bundle mechanism (wasm-safe). A bundle is a directory with a
`bundle.ron` manifest listing its content files (relative paths); a bundle loader
`load_context.load`s each -> its typed asset by extension (`*.sections.ron` /
`*.ship.ron` / `*.scenario.ron`). A `merge_bundle` step routes each loaded asset into
its id-keyed registry by kind (sections -> `GameSections`, ships -> `GameShips`,
scenarios -> `GameScenarios`); adding a kind = one new arm. Manifest, NOT
directory enumeration - `load_folder` is broken on wasm (see the spike). Gated on the
ship kind (20260714-134115) existing so all three kinds route. `spike` until planned.

## Re-based v2 (20260714, spike tasks/20260714-150410)

GATED ON the content model (20260714-150508), which already gives one `ContentLoader` +
`register_content` for single files. This task adds the FOLDER packaging on top: a
`bundle.ron` manifest listing content files (relative paths), a bundle loader that
`load_context.load`s each `Content` file and flattens the items, then merges by kind via
the existing `register_content` router with load-order overlay. The merge-by-KIND is
already the content router (data flag, not extension); this task is really "manifest +
directory-of-content-files + overlay", not per-extension routing. Old extension framing
superseded.

## Re-prioritized v2 (20260714): NOW NEXT in the bundle family

Bumped to lead the remaining family (user, during /flow 134115): the folder bundle is
the higher-value "real bundle" step and does NOT need the ship kind - the content router
(150508) already merges whatever kinds exist (Section + Scenario today). So this NO
LONGER gates on the ship kind (134115); drop that dependency. On the content-model
foundation this is: a `bundle.ron` manifest listing `Content` files (relative paths), a
bundle loader that `load_context.load`s each and flattens the items, merged via the
existing `register_content` router (by kind, not extension - the old "typed asset by
extension" framing above is superseded), with load-order overlay. Package the base
game's `.content.ron` files into an `assets/base/` folder + manifest. wasm-safe (manifest,
not load_folder). Next after this: 134123 (base-as-bundle) -> 134127 (mods+demo) -> then
134115 (ship kind, with a consumer).

## Plan (20260714) - folds base-as-bundle (134123) as the proof

The base game becoming a bundle IS the mechanism's end-to-end proof (like 150508 did
mechanism + migration together), so 134123 folds in here - no throwaway demo bundle.

Steps:
- [x] 1. nova_modding: `BundleManifest { content: Vec<String> }` (relative content-file
  paths) + `BundleAsset { content: Vec<Handle<ContentAsset>> }` + `BundleAssetLoader`
  (`bundle.ron`): parse the manifest, `load_context.load::<ContentAsset>(path)` each entry
  (resolved relative to the bundle file's dir - use the bevy-idiomatic relative path, or
  root-relative paths in the manifest if cleaner), store the handles. `BundleAsset`'s
  `VisitAssetDependencies` MUST visit its content handles (unlike ContentAsset, a bundle
  HAS dependencies) so the content loads with the bundle. Register in NovaModdingPlugin.
  Unit test: a bundle.ron manifest decodes; (integration) a bundle + its content load.
- [x] 2. Package the base as a bundle: move the six `*.content.ron` into `assets/base/`
  (`assets/base/sections/base.content.ron`, `assets/base/scenarios/*.content.ron`) and add
  `assets/base/bundle.ron` listing them. git-rename the files; update the content-parity
  test paths.
- [x] 3. nova_assets: `GameAssets` replaces the six `Handle<ContentAsset>` with ONE
  `Handle<BundleAsset>` (`assets/base/bundle.ron`). A `register_bundles` system (replaces/
  wraps `register_content`) reads a LIST of loaded bundles (just `[base]` for now), flattens
  every bundle's content items in order, and merges by kind into GameSections/GameScenarios
  with LOAD-ORDER overlay (later bundle's id wins). Written overlay-ready so 134127 (mods)
  just appends bundles. Keep `error!`+skip on an unloaded asset.
- [x] 4. Update the `demo_scenario` test + `_for_test` re-exports for the bundle path;
  assert GameScenarios (demo + 4 built-ins) + GameSections populated via the base bundle.
- [x] 5. Verify: `cargo test --workspace --no-run`; nova_modding/nova_assets tests;
  `12_menu_newgame` + `09_editor` under `DISPLAY=:0 BCS_AUTOPILOT=1 --features debug`;
  parity green. Behavior IDENTICAL (same content registered, now via the base bundle).

Follow-on: 134127 (mods = more bundles overlaid + demo mod); 134115 (ship kind, deferred).
