# Fix: base bundle fails to load in-game (untyped load, single-dot bundle.ron extension unresolvable)

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: bug

Regression from the folder-bundle work (20260714-134119). In-game (web + native) the
base bundle never loads:

```
ERROR bevy_asset::server: Could not find an asset loader matching:
  Asset Type: None; Path: "base/bundle.ron";
```

The game keeps running but `register_bundles` inserts EMPTY `GameSections` /
`GameScenarios` (the bundle asset failed), so no sections/scenarios exist - the game
is effectively broken.

## Root cause (diagnosed, reproduced)

`bevy_asset_loader`'s collection kickoff loads EVERY `#[asset(path=...)]` field with
`asset_server.load_untyped(path)` for load-tracking (derive `assets.rs:490`), NOT the
typed `load::<T>`. An UNTYPED load resolves the loader by EXTENSION ONLY. Bevy's
`AssetPath::get_full_extension` returns the substring after the FIRST dot in the file
name, so:

- `base/bundle.ron` -> `"ron"`   (single dot) -> NO registered loader -> fails.
- `demo.content.ron` -> `"content.ron"` -> matches `ContentAssetLoader` -> fine.

`BundleAssetLoader` registered the extension `"bundle.ron"`, but a file literally named
`bundle.ron` never yields that full extension. The `demo_scenario` integration test
dodged the bug because it loads the bundle with an explicit `Handle<BundleAsset>`
annotation - a TYPED load, which falls back to the by-asset-type loader candidate and
succeeds. The game's untyped path has no such type to fall back on.

Reproduced: `DISPLAY=:0 RUST_LOG=bevy_asset=trace BCS_AUTOPILOT=1 cargo run --example
12_menu_newgame --features debug` logs the error right as bevy_asset_loader starts
loading the `GameAssets` collection.

## Fix

Give bundle manifests a STEM so the full extension is the registered `"bundle.ron"`
(the same convention `*.content.ron` already relies on). Convention:
`<packname>.bundle.ron`.

Steps:
- [x] 1. `git mv assets/base/bundle.ron assets/base/base.bundle.ron`.
- [x] 2. `GameAssets`: `#[asset(path = "base/base.bundle.ron")]`.
- [x] 3. Update the `demo_scenario` test path to `base/base.bundle.ron`.
- [x] 4. Docs: `nova_modding` module doc + `docs/modding-ron-format.md` - state the
  `<packname>.bundle.ron` naming rule and WHY (single-dot filenames resolve to the
  bare `ron` extension under bevy's untyped load, which bevy_asset_loader uses). The
  `BundleAssetLoader` extension stays `"bundle.ron"`.
- [x] 5. Regression test (nova_assets or nova_modding): load the base bundle via
  `asset_server.load_untyped("base/base.bundle.ron")` (the EXACT path the game uses,
  not a typed load), pump updates, and assert the recursive load state reaches
  `Loaded` (not `Failed`). This test FAILS under the old `bundle.ron` name (untyped ->
  extension `ron` -> unresolvable) and passes with the stem. Pins the exact failure
  mode a typed test cannot catch.
- [x] 6. Verify: reproduce with `12_menu_newgame` (+`09_editor`) headless - the error
  is GONE and sections/scenarios load; `cargo test --workspace --no-run`;
  nova_modding/nova_assets tests; parity green.

Note for 20260714-134127 (mods): the top-level `mods.ron` would hit the SAME bug (it
too is a bevy_asset_loader field, loaded untyped, single-dot -> `ron`). That task must
name its manifest with a stem (e.g. `*.mods.ron`) for the same reason - fold this
knowledge into its plan.
