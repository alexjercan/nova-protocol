# Section catalog as data: assets/sections/*.ron loaded into GameSections via nova_modding

- STATUS: CLOSED
- PRIORITY: 58
- TAGS: v0.6.0, modding, scenario

Spike: tasks/20260714-110502/SPIKE.md

Goal (step 1 of the duplication direction): author the ~5 section prototypes
(`basic_controller_section`, `basic_hull_section`, turret, torpedo, thruster -
today built in `crates/nova_assets/src/sections.rs` `build_sections`) as
`assets/sections/*.ron` data, and load them into `GameSections` through a new
`nova_modding` catalog asset + loader (mirroring `ScenarioAsset`). Makes sections
moddable and is the reference target step 2 (20260714-113411) points at. Lowers to
the existing runtime `SectionConfig` - nothing downstream changes. `spike` until
planned.

## Plan (20260714)

Consumers to preserve: `GameSections` is read at runtime by the editor
(`nova_editor` build palette, `get_section("basic_controller_section")` etc.), so
the catalog must load the same section ids. `register_sections` runs
`OnEnter(GameAssetsStates::Processing)`. Mirror the `ScenarioAsset` pattern.

Steps:
- [x] 1. `nova_modding`: add `SectionCatalogAsset(pub Vec<SectionConfig>)` (bevy
  `Asset` with a no-op `VisitAssetDependencies`, like `ScenarioAsset`) +
  `SectionCatalogAssetLoader` for the `sections.ron` extension (ron decode into a
  `Vec<SectionConfig>`); register both in `NovaModdingPlugin`. Unit test: a minimal
  catalog RON decodes.
- [x] 2. Generate `assets/sections/base.sections.ron` by serializing
  `build_sections(&SectionMeshRefs::from_paths())` (path-based mesh refs), and add a
  `sections_ron_parity` test that guards it against builder drift (mirror
  `scenario_ron_parity`). Single catalog file for now; per-file split is the bundle
  spike's (113418) concern.
- [x] 3. `nova_assets`: add `section_catalog: Handle<SectionCatalogAsset>` to the
  `GameAssets` collection (`#[asset(path = "sections/base.sections.ron")]`); rewrite
  `register_sections` to insert `GameSections` from the loaded catalog asset
  (`Assets<SectionCatalogAsset>::get`, `error!`+skip on miss, no panic). Keep
  `build_sections`/`SectionMeshRefs` as the generator/parity source only.
- [x] 4. Verify: `cargo test --workspace --no-run` (test build - the check that
  caught the last merge break); nova_assets tests (catalog + parity); editor section
  ids intact; run `09_editor` and `12_menu_newgame` under `DISPLAY=:0 BCS_AUTOPILOT=1
  --features debug` to confirm the editor palette + boot still work off the data
  catalog.

Follow-on: step 2 of the family (20260714-113411) makes ship sections REFERENCE this
catalog by id instead of inlining - the big dedup.
