# Retro: Configurable section collider shape and size

- TASK: 20260718-102022
- BRANCH: section-collider-config
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Verified avian's collider semantics from the crate source
  (`cuboid` halves its extents, `capsule` endpoints at +/- length/2) BEFORE
  writing `aabb_half_extents`, so the half-extent math was right the first time
  and the same read served as the review's independent re-derivation.
- Confirmed spawn-path coverage by reading each `Collider::cuboid(1,1,1)` site
  rather than trusting the grep list: four of the six were `#[cfg(test)]` bodies,
  so wiring only `base_section`/`preview_section` was complete, not a miss.
- `content_lint_gate` turned out to be the ideal end-to-end check - it both
  deserializes the new cargob RON and runs the updated overlap lint over the
  whole shipped tree, so one existing test validated the risky parts together.

## What went wrong

- The required-field ripple: adding `collider` (not filled by
  `..Default::default()`) broke the seven fully-explicit `BaseSectionConfig`
  literals in `nova_assets/src/sections.rs`. Root cause: my first compile check
  was scoped to `-p nova_gameplay -p nova_scenario`, which does not touch
  nova_assets, so the break only surfaced two test-runs later when I built
  `content_lint_gate`. A workspace `--all-targets` check right after the struct
  change would have caught all of it at once.
- Lost a test-run to the serde-feature gotcha: `cargo test -p nova_scenario`
  fails to compile because its serde derives are feature-gated and only the
  workspace build unifies the feature in via the app crate. Had to re-run with
  `--features serde`.

## What to improve next time

- When adding a field to a widely-constructed struct, immediately grep every
  `<Struct> {` literal and run `cargo check --workspace --all-targets` - do not
  scope the first check to the crate you edited.
- For crate-scoped tests in this repo, reach for `--features serde` (or run the
  whole workspace) up front rather than discovering the gate on the first fail.

## Action items

- [x] Documented the density-vs-mass decision and both difficulties in
  docs/design/section-collider-config.md.
- [x] Ledger updated (`struct-field-ripple`, `crate-scoped-test-features`).
- No follow-up code task filed: the prototype-collider lint fallback (REVIEW
  R1.1) is a conscious, documented limitation and no shipped prototype needs it
  yet.
