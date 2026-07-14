# Content model + generic kind-router: one Content enum (kind-in-RON) + ContentLoader + register_content; refactor per-kind loaders onto it

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.6.0,modding,scenario,spike

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
the markup-language AST groundwork. `spike` until planned. Everything else in the family
gates on this.
