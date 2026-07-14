# Review: section catalog as data

- TASK: 20260714-113408
- BRANCH: modding/section-catalog

Reviewed out-of-context (one fresh-eyes agent) plus implementer re-verification of
the load-bearing claim (all 4 editor-referenced section ids are present in the
committed `base.sections.ron`; corroborated by a live `09_editor` run). A faithful
mirror of the already-landed `ScenarioAsset` pattern.

## Round 1

- VERDICT: APPROVE

Clean. No BLOCKER/MAJOR/MINOR findings.

Verified:
- Correctness - `register_sections` reads the catalog at `OnEnter(Processing)`,
  which `load_collection` gates on the whole `GameAssets` collection (incl. the
  catalog handle) finishing, so `Assets::get` resolves; the `error!`+`default()`
  else-arm is non-panicking and cannot fire in practice (a missing/corrupt file
  stalls `Loading`, never reaching the system). `from_game_assets` removal leaves no
  dangling refs. No ordering issue with `register_scenario` (it reads
  `Assets<ScenarioAsset>`, not `GameSections`).
- Tests - `sections_ron_parity` is a genuine (non-circular) drift guard: the file is
  committed, so runs take the compare branch, not the write-on-missing branch. The
  `demo_scenario` `GameSections` assertion is meaningful - it fails if the catalog
  does not load (else-arm inserts an empty registry). No existing test weakened.
- Design - `ScenarioLoaderError -> ModdingLoaderError` shared by both loaders is a
  clean generalization; single-catalog-file is correctly scoped (per-file split
  deferred to spike 113418); `SectionCatalogAsset`'s hand-impl'd `Asset`/no-op
  `VisitAssetDependencies` matches `ScenarioAsset` (lazy `AssetRef` resolution).
- Spec - delivered: sections are data, loaded into `GameSections`, runtime
  `SectionConfig` unchanged; TASK.md honest, all steps match the diff.

- [ ] R1.1 (NIT) tasks/20260714-113408/TASK.md - the narrative says "~5 section
  prototypes" but `build_sections` (the single source) emits 7
  (`basic_controller_section`, `basic_thruster_section`, `better_turret_section`,
  `light_hull_section`, `light_turret_section`, `reinforced_hull_section`,
  `torpedo_section`). Cosmetic; correct as generated. No action required.
  - Response: acknowledged; left as-is (the count is authored by the code source,
    not this task).
