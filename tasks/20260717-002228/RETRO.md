# Retro: Let sections declare sounds + move base sounds under assets/base/

- TASK: 20260717-002228
- BRANCH: task-20260717-002228-section-sounds (squash-landed c67a8989)
- REVIEW ROUNDS: 2 (APPROVE at round 2)

See TASK.md for what/why and REVIEW.md for the findings; this is process only.

## What went well

- Auditing before planning paid off. The audit surfaced that audio is a mature,
  heavily-tested global `SoundBank<NovaSfx>` enum with observer-driven playback -
  not a per-section handle like `render_mesh`. That reframed "sections declare
  sounds" as a real design decision (pick an exemplar, prefer-with-fallback,
  don't gut the tested cue system) instead of a mechanical field copy.
- The independent out-of-context reviewer earned its keep. Implementer and
  reviewer shared one session, and the blind spot was exactly where the skill
  warns: TEST SHAPE. The two playback tests manually inserted the
  `TurretSectionFireSound` component, so the real declaration path
  (`config.fire_sound` -> `insert_turret_section` -> component) was untested. A
  correctness-only self-review would have missed it; the fresh-eyes pass named it.
- Self-review against the sibling precedent caught the design flaw before it
  shipped. `muzzle_effect` (same file) stores the UNRESOLVED `AssetRef` and
  resolves later; comparing my code to it exposed that I had put resolution in the
  wrong place (see below) rather than only finding it via a test failure.

## What went wrong

- R1.1 (resolution in the wrong observer). My first cut resolved `fire_sound` in
  `insert_turret_section`, adding `Res<AssetServer>` to an observer that
  `TurretSectionPlugin` registers UNCONDITIONALLY - so every headless section rig
  that spawns a turret would now need an `AssetServer` it never needed before.
  Root cause: I resolved at the "obvious" spot (the build observer, next to the
  render-mesh snapshots) without checking WHERE the analogous `AssetRef` field
  (`muzzle_effect`) actually resolves - it snapshots unresolved at build and
  resolves in a render-time observer. The convention was one function away.
- A piped build masked a real compile error - twice. `cargo build ... | tail -25`
  reported "exit 0" (the notification shows the PIPELINE's exit = tail's), while
  cargo had actually failed with E0593 (`unwrap_or_else(|| ..)` on a `Result`
  needs `|_|`). I only noticed when the next `cargo run` printed the error text.
  The same masking recurred on `cargo test -p nova_scenario | tail` (a
  feature-gated `Serialize` compile error read as "exit 0"). Root cause: trusting
  the harness "exit code 0" of a piped command instead of reading the output for
  `error[`/`could not compile`/`Finished`.
- R1.2 (test shape, above) is the same root as a general habit: I wrote the
  cheapest test that exercised the new code (manual component) rather than the one
  that proves the SEAM the feature adds (config -> component -> playback).

## What to improve next time

- When adding an `AssetRef<_>` (or any resource-resolving) field, find the nearest
  SIBLING field of the same kind and mirror its resolve SITE, not just its
  declaration attributes. Resolution site determines which systems gain a resource
  dependency.
- Never trust a piped cargo command's exit code. Either drop the pipe, add
  `set -o pipefail`, or grep the output for `error[`/`could not compile` AND
  `Finished` - "exit 0" from `| tail`/`| grep` is the pipe's, not cargo's.
- For a new authorable field, write the test that crosses the whole new seam
  (declaration -> resolution -> effect) first; a test that only touches the tail
  of the seam (manual component -> playback) hides a broken head.

## Action items

- [x] Both lessons appended to docs/LESSONS.md (`mirror-sibling-resolve-site`,
  `piped-cargo-masks-exit-code`).
- No follow-up code task: torpedo/thruster/damage sounds were deliberately scoped
  out (documented in TASK.md + design doc) and are a clean future extension, not a
  defect - filed only if a later task wants parity beyond the turret exemplar.
