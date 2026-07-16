# Review: Docs - canonical scheme model / base as a normal mod

- TASK: 20260717-002203
- BRANCH: docs/canonical-scheme-model

Docs-only task; review is accuracy + completeness (no build). Verified each claim
against the landed behavior and swept for stale teaching/paths.

## Round 1

- VERDICT: APPROVE

Accuracy (each claim matches landed code):
- "dep://base is implicit, never declared" - matches task 20260717-000416.
- "base art lives under assets/base/" - matches 20260717-002105.
- "a bare asset ref is an error at author/publish time; the hard guarantee is
  structural (404)" - matches 20260717-002133 (static lint + portal; no runtime
  gate).
- All example refs (`dep://base/gltf/hull-01.glb#Scene0`, `dep://base/textures/
  cubemap.png`, `dep://base/banner.png`) name real base resources (in base's
  `resources` list), so they are valid canonical examples.

Completeness (swept):
- No contradictory bare-ref teaching remains in the five docs (the one "bare path
  == base game" mention left is inside the Option A section describing what was
  RETIRED).
- No bare base-art example refs remain in the wiki.
- Stale moved-path instruction fixed: guide-make-a-mod.md now points at
  `assets/base/textures/cubemap.png.meta` (the `.meta` moved with the migration).
  The only other `assets/textures/...` mention is inside a SUPERSEDED historical
  note in the design doc (correct - it describes the old root path).

Coverage: guide-make-a-mod, modding-ron, guide-author-scenario, guide-author-section,
mod-binary-resources design doc, and CHANGELOG (breaking entry) all updated. No
findings.
