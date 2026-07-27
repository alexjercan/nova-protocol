# Retro: NOVA OS curved CRT edge rim + green grain

- TASK: 20260727-135204
- BRANCH: feature/nova-os-crt-frame
- REVIEW ROUNDS: 1 (APPROVE, no findings)

## What went well

- Recognized early that a UI border node can NEVER bow with the barrel warp, so
  the only correct home for the curved edge is the shader. That turned a fuzzy
  "make the border look 3D" into a concrete, testable choice (rim from the
  panel-edge distance in warped uv), recorded in DECISION.md before coding.
- Caught that the gray grain was gray *because* the scalar broadcast equally to
  RGB - so the fix was a one-line green multiply, not a new noise function.
- Validated the shader the honest way (ran the real app, confirmed clean
  AppExit + no naga panic) instead of trusting `cargo check`, which does not
  compile WGSL at all. The out-of-context reviewer independently reached the
  same "cargo check does not cover this" conclusion.
- Kept the phosphor-rim nodes (demoted, not deleted) so the existing rim test
  and the headless fallback both keep working - a smaller, safer diff than
  ripping them out.

## What went wrong

- Nothing blocked the task, but I initially reached for a Rust unit test as the
  DoD proof and only belatedly realized the CORE deliverable (the shader) has no
  cargo-check or unit-test coverage. A shader typo would have shipped a NOVA OS
  that panics on open. Root cause: reflex-reaching for the cheap headless test
  before asking "what actually exercises the thing I changed?".

## What to improve next time

- On any shader/asset change (runtime-loaded), run the rendering app/example as
  the FIRST validation step, not an afterthought - `cargo check` green means
  nothing for WGSL.

## Action items

- [x] Added ledger lesson `wgsl-not-covered-by-cargo-check` (x1) with the exact
  validation command.
- [x] Corrected the task's DoD filter off the bogus template `drawer` and added
  the shader-runtime DoD (screenshot_nova_os).
- [x] Filed follow-up tatr 20260727-143752 (p45) for the pre-existing
  `catalog_matches_disk` red test (screenshot_nova_os not smoke-listed),
  discovered while looking for a shader-validation harness.
