# Content model + generic kind-router: one Content enum (kind-in-RON) + ContentLoader + register_content; refactor per-kind loaders onto it

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.6.0, modding, scenario

Spike: tasks/20260714-150410/SPIKE.md

Goal: the GENERIC foundation of the bundle family, built FIRST so every content kind
is just a variant (no bespoke catalog to later fold). Define a `Content` enum with the
kind flag IN the data - `Content = Section(SectionConfig) | Scenario(ScenarioConfig)`
(Ship added by 20260714-134115) - authored as a RON `Vec<Content>`. Add ONE
`ContentAsset(Vec<Content>)` + one `ContentLoader`, and a generic `register_content`
router that dispatches each item by variant into its id-keyed registry (GameSections /
GameScenarios / ...). REFACTOR the existing per-kind loaders (`SectionCatalogAsset`
`*.sections.ron`, `ScenarioAsset` `*.scenario.ron`) + the base game's register_* + the
committed RON onto the Content model - the shipped section catalog becomes
`[Section((..)), ...]`, a scenario file becomes `[Scenario((..))]`. Behavior-preserving;
carry the parity/demo tests over (re-generated in the Content shape). Consider
normalizing `GameSections` (a Vec today) to an id-keyed map for clean overlay. This is
the markup-language AST groundwork. Everything else in the family gates on this.

## Plan (20260714)

Behavior-preserving. Keep `GameSections` a `Vec` for now (defer map-normalization to the
overlay task 134119 - minimizes churn; the editor reads it unchanged). One `register_content`
replaces `register_sections`+`register_scenario`. Content files use the `.content.ron`
extension.

Steps:
- [x] 1. nova_modding: `Content` enum `{ Section(SectionConfig), Scenario(ScenarioConfig) }`
  (kind flag in data) + `ContentAsset(pub Vec<Content>)` (Asset + no-op
  VisitAssetDependencies, like ScenarioAsset) + `ContentLoader` (extension `content.ron`,
  ron-decodes `Vec<Content>`, reuses `ModdingLoaderError`). REMOVE `ScenarioAsset` +
  `SectionCatalogAsset` + their loaders; register `ContentAsset`+loader in `NovaModdingPlugin`.
  Unit test: a content RON mixing a `Section((..))` and a `Scenario((..))` decodes.
- [x] 2. nova_assets: a `register_content` system (replaces register_sections+register_scenario
  in the OnEnter(Processing) chain) reading the loaded `ContentAsset`s from `GameAssets` and
  routing each item by variant: `Section` -> `GameSections` (collect into the Vec),
  `Scenario` -> `GameScenarios` (insert by id). error+skip a missing/empty asset, no panic.
  `GameAssets` content-file fields become `Handle<ContentAsset>`. Update the `_for_test`
  re-exports (register_content_for_test).
- [x] 3. Migrate + regenerate RON: rename the content files to `.content.ron`
  (`sections/base.content.ron` = `[Section((..)), ...]`; each `scenarios/<name>.content.ron`
  = `[Scenario((..))]`; `demo.content.ron` hand-migrated). Generators emit `Vec<Content>`
  (build_scenarios -> each `Content::Scenario`; build_section_catalog -> each `Content::Section`).
  Fold the two parity tests into a content parity guard; regenerate. Do it by
  serializing (never hand-author) - the parity test is the safety net.
- [x] 4. Update `demo_scenario` test (load `ContentAsset`s; assert `GameScenarios` has the
  built-ins + demo AND `GameSections` populated) + `GameAssets` paths. Editor reads
  `GameSections` unchanged (still a Vec).
- [x] 5. Verify: `cargo test --workspace --no-run`; nova_modding/nova_scenario/nova_assets
  tests; `12_menu_newgame` + `09_editor` under `DISPLAY=:0 BCS_AUTOPILOT=1 --features debug`;
  parity green. Behavior IDENTICAL to pre-refactor (same sections/scenarios registered).
