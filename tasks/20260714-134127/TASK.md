# Mod loading + load-order overlay + a demo mod (override a section, add a scenario)

- STATUS: CLOSED
- PRIORITY: 36
- TAGS: v0.6.0, modding, scenario

Spike: tasks/20260714-113418/SPIKE.md

Goal: the payoff - a mod is another bundle merged on top of the base. A wasm-safe
top-level `mods.ron` lists enabled mod-bundle manifests; each loads after the base
and merges by kind with LOAD-ORDER overlay (later id wins = mod overrides base;
intra-bundle duplicate id = hard error). Native may optionally enumerate a `mods/`
dir, but `mods.ron` stays the wasm-safe source of truth. Ship a DEMO mod that
overrides one base section and adds one scenario, with a test proving the base+mod
merge + overlay end-to-end. Gated on the base-as-bundle (20260714-134123). `spike`
until planned.

## Re-based v2 (20260714)

Re-based on the content-model bundle design (spike tasks/20260714-150410): "sections/
ships/scenarios" are all `Content` items (kind-in-data); the base/mod bundle is a folder
of `Content` files + a `bundle.ron` manifest, merged by kind via `register_content`.
Otherwise unchanged. Gated on the folder-bundle mechanism (20260714-134119).

## Plan (20260714) - on the landed folder-bundle mechanism (134119)

Design: a mod is just another bundle merged AFTER the base. The enabled mods are
declared in a wasm-safe top-level enable-list manifest (never directory enumeration).
This mirrors the `*.bundle.ron -> BundleAsset` mechanism exactly, one level up:
`enabled.mods.ron -> ModList` (whose dependencies are the enabled mod bundles), so
bevy's recursive-dependency load-state gate pulls in every mod bundle + its content
before `register_bundles` runs. The router already overlays cross-bundle last-wins by
id (134119); this task adds the mod source and the intra-bundle duplicate-id guard.

Key decisions:
- STEMMED filename, `enabled.mods.ron` NOT `mods.ron`. Like the base bundle, the
  enable-list is a `GameAssets` field and bevy_asset_loader loads it UNTYPED, which
  resolves the loader by the file's FULL extension only (everything after the first
  dot). A bare `mods.ron` -> `ron` (no loader) fails in-game - the exact bug fixed in
  20260714-163342. `enabled.mods.ron` -> full extension `mods.ron`, which the
  `ModListLoader` registers. Mod bundle manifests follow `<pack>.bundle.ron` too.
- Default `assets/enabled.mods.ron` ships EMPTY (`(mods: [])`) so the base game stays
  pristine and the 12_menu_newgame / 09_editor behavior gate is preserved (every
  prior family task was behavior-identical). The demo mod ships enable-able but OFF by
  default; enabling it is a one-line edit. Flag this to the user at close - they may
  want it on for the modding showcase.
- Intra-bundle duplicate id: surfaced as a loud `error!` + skip (keep first),
  NOT a panic. A panic in an asset-processing system would crash the whole app on
  bad mod data - worse for a modding system than logging and continuing. This
  still satisfies "not silently accepted" (the point of the spike's "hard error").
  Cross-bundle same id stays silent overlay (mod-overrides-base, by design).

Steps:
- [x] 1. nova_modding: `ModListManifest { mods: Vec<String> }` + `ModList { bundles:
  Vec<Handle<BundleAsset>> }` asset + `ModListLoader` (extension `mods.ron`): parse
  the manifest, `load_context.load::<BundleAsset>(path)` each entry (asset-root paths
  like `mods/demo/demo.bundle.ron`; resolve relative to the manifest dir which is the
  root), store the handles. `ModList`'s `VisitAssetDependencies` visits its bundle
  handles (like `BundleAsset` visits its content) so mods load with the list. Register
  in `NovaModdingPlugin`. Unit test: a `*.mods.ron` body decodes into `ModListManifest`.
- [x] 2. nova_assets: refactor the merge into a pure, testable core.
  `merge_bundles(bundles)` takes an ORDERED list of bundles, each an ordered list of
  its `&Content` items (flattened across the bundle's content files), and returns
  `{ sections: Vec<SectionConfig>, scenarios: GameScenarios, conflicts: Vec<String> }`.
  Cross-bundle: last-wins overlay by id (reuse `merge_content_item`). Intra-bundle:
  track a per-bundle seen-id set per kind; a repeat id within the SAME bundle is a
  conflict (recorded, item skipped, first kept). `register_bundles` builds the
  ordered `[base] ++ mod_list.bundles` list, flattens each bundle's content from
  `Assets`, calls `merge_bundles`, `error!`s each conflict, and inserts the
  resources. Keep `error!`+skip on an unloaded asset.
- [x] 3. nova_assets `GameAssets`: add `#[asset(path = "enabled.mods.ron")] pub
  mod_list: Handle<ModList>`. Ship `assets/enabled.mods.ron` = `(mods: [])`.
  `register_bundles` reads `Res<Assets<ModList>>` for the ordered mod bundles after the
  base. NOTE: add an UNTYPED-load regression guard for `enabled.mods.ron` (mirroring
  the base bundle's `bundle_untyped_load_resolves_the_loader`) so the stem rule can't
  silently regress.
- [x] 4. Ship the demo mod under `assets/mods/demo/`: `demo.bundle.ron` +
  content file(s) that (a) OVERRIDE one existing base section by id (e.g. re-stat or
  rename a hull) and (b) ADD one new scenario id. Author by hand (small), in the same
  RON style as the base content. A short `assets/mods/demo/README.md` (or header
  comment) says what it demonstrates and how to enable it.
- [x] 5. Tests:
  - unit: `merge_bundles` overlay (later bundle's section+scenario win by id) and
    intra-bundle dup (recorded as a conflict, first kept) - pin both.
  - integration (nova_assets): a test App loads the base bundle AND the demo mod
    bundle via the real loaders (`load::<BundleAsset>`), waits for recursive load,
    runs `merge_bundles([base, demo])`, and asserts the overridden section took the
    mod's value + the added scenario id is present. This exercises loader ->
    manifest -> content -> Content -> overlay end to end.
  - `demo_scenario` still passes with the empty shipped `enabled.mods.ron` (base
    unchanged).
- [x] 6. Verify: `cargo test --workspace --no-run`; nova_modding/nova_assets tests;
  `12_menu_newgame` + `09_editor` under `DISPLAY=:0 BCS_AUTOPILOT=1 --features debug`
  (must be behavior-IDENTICAL - enable-list empty); parity green.

Follow-on: 134115 (ship kind, deferred until a real consumer - the demo mod could
become that consumer later). Native `mods/` dir auto-enumeration stays out of scope
(wasm-unsafe); the `enabled.mods.ron` enable-list is the source of truth.
