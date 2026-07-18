# Retro: render_mesh_transform on all section kinds

- TASK: 20260718-121205
- BRANCH: section-render-mesh-transform
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Front-loaded the architecture with an Explore agent before writing code. It
  surfaced two things that shaped the design cleanly: (1) all non-turret kinds
  spawn the render mesh as a CHILD (so the transform-on-child approach ports
  directly), and (2) torpedo reads its config directly while the other three use
  snapshot components - so I knew up front torpedo needed a different hook.
- The move of `RenderMeshTransform` to base_section.rs was the right call and
  cost little: turret's tests passing after the move was immediate proof the
  refactor was sound.
- The struct-field ripple went smoothly this time because I expected it: added
  the field, ran `--all-targets`, fixed each reported literal (Default impls,
  generator, test literals) in one pass. The lesson has fully internalized.
- Recognized that hull/thruster/controller share ONE mechanism, so one
  integration test (hull) plus the distinct torpedo test covers the real paths
  without four near-identical tests.

## What went wrong

- Minor only: no failed test runs or dead ends this cycle. The one thing to
  watch was the same asset-registration gotcha from last task
  (`init_asset::<WorldAsset>()`, schemeless path, `abs_diff_eq`) - I applied all
  three pre-emptively from the ledger, so the render tests passed first try.
  That is the ledger paying off.

## What to improve next time

- Consider whether thruster/controller deserve their own tests despite sharing
  the mechanism. I judged not (documented), but if these kinds ever diverge the
  shared-test assumption silently rots. A future divergence should add tests -
  noted in the REVIEW and docs so it is not forgotten.

## Action items

- [x] docs/design/section-render-mesh-transform.md written (design, shared-vs
  -per-kind split, ripple, test coverage rationale).
- [x] Ledger: bumped `register-assets-for-new-test-path` (its guidance
  pre-empted all render-test failures this time).
- No follow-up task filed: the feature now covers every section kind; further
  work (e.g. scale, or per-kind divergence) is user-driven.
