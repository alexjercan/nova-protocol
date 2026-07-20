# Review: base bundle fails to load in-game (untyped extension)

- TASK: 20260714-163342
- BRANCH: fix/bundle-untyped-extension

## Round 1

- VERDICT: APPROVE

Self-authored branch; per the review skill I independently re-derived the load-bearing
claims rather than trusting the summary:

- Root cause re-derived from bevy 0.19 source: `bevy_asset_loader_derive` `assets.rs:490`
  emits `asset_server.load_untyped(path).untyped()` for every basic collection field;
  `AssetServer::load_untyped` -> `load_internal(None, ...)` -> `get_meta_loader_and_reader`
  with `asset_type_id = None` -> `loaders.find(None, path)` resolves by extension only;
  `AssetPath::get_full_extension` returns the substring after the FIRST dot, so
  `bundle.ron` -> `"ron"` (unregistered) and `base.bundle.ron` -> `"bundle.ron"`
  (registered). The typed `demo_scenario` test dodged it via the by-asset-type fallback
  in `loaders.find`.
- Reproduced the exact failure on master in-game (`12_menu_newgame`, `RUST_LOG=bevy_asset=trace`):
  the error fires the instant bevy_asset_loader starts loading the `GameAssets` collection.

Findings:

- [x] R1.1 (NIT) [verification, not a defect] The new guard must fail without the fix.
  Sabotage-verified: copied the manifest to the un-stemmed `bundle.ron`, pointed the
  untyped test at it -> it panicked with the exact in-game error
  ("Could not find an asset loader matching: Asset Type: None; Path: base/bundle.ron");
  reverted. The typed `base_bundle_loads_into_game_registries` still passes either way,
  confirming a typed test cannot catch this.

Correctness / spec:
- The fix (rename to `base.bundle.ron`, path update, stem convention documented) delivers
  the Goal. In-game re-verified: `12_menu_newgame` and `09_editor` (the section-palette
  consumer that this bug zeroed out) both run headless with 0 loader errors and no panic.
- No stray references to the old name anywhere (grepped rs/toml/html/md/ron/sh/yaml);
  `BundleAssetLoader`'s registered extension `"bundle.ron"` is correct - it is the full
  extension of any `*.bundle.ron` file.
- Tests: nova_assets (18 unit + demo_scenario 2) and nova_modding pass;
  `cargo test --workspace --no-run` green; fmt clean; parity green.

Docs: the naming rule + the untyped-vs-typed rationale are captured in the
`BundleAssetLoader` doc and `docs/modding-ron-format.md`, and a forward note warns
task 134127 that `mods.ron` needs the same stem treatment.

No BLOCKER/MAJOR/MINOR. Ships.
