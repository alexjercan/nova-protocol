# Open world: nova_world_base owns the feature field

- STATUS: CLOSED
- PRIORITY: 72
- TAGS: v0.15.0, architecture, open-world

Fifth PR in the stack: #58 -> #59 (`nova-world-foundation`) -> #61
(`nova-world-generator-interface`) -> #62 (`nova-open-world-new-game`) -> this
branch (`nova-world-base-owns-features`). Parent spike: `20260824-125938`.

## User facts

- `nova_world` is the generic engine. Its manifest is body-only: asteroids,
  planets and ships. It names no feature, noise, layer or strength.
- `nova_world_base` owns all feature-field policy: layers, spheres, thinning,
  halo, sector strengths, the placement inset and the clearance margin.
- Examples own their own generators. `UniformAsteroids` keeps its own inset
  and margin. `world_features` computes the spheres around each root itself.
- Generated bodies, ids and visuals do not change.

## Decisions

- Owner, 2026-09-23: approved with four corrections.
  - New generic `SectorFault::Generation { id, field, value }` for
    non-finite field readings and invalid generator-internal owner or
    strength values. `InvalidGeometry` stays for non-finite geometry and
    `DuplicateId` for duplicate ids. `Noise`, `Feature` and
    `DuplicateFeature` are gone.
  - Focused `nova_world_base` feature tests: repeat and order independence,
    seam agreement, same-layer thinning, cross-layer overlap, off-lattice and
    halo refusal. The `system_world_sectors` claims stay.
  - A pinned body-manifest digest test (see Verification for its derivation).
  - The 128 km edge ceiling documents both reasons: feature-halo completeness
    and the measured body budget.
- Also approved: public `sector_strengths`, base-owned public
  `PLACEMENT_INSET` (0.7) and `CLEARANCE_MARGIN` (500 m),
  `WorldGeometry::require_owning_edge(inset, clearance, body)` refusing an
  inset outside `[0, 1)`, `bodies_clear(.., margin)` with a zero margin in the
  core overlap check, example-local `RootFeatures`, no generic feature
  streaming components.

## Done when

- The stacked PR is open against `nova-open-world-new-game` with the focused
  checks green. Not merged; Copilot review not requested yet.

## Verification

- Body identity: canonical output for 750 cells (seeds 20260922 and
  20260923; 32 km windows around the origin and (2,1,2); a 128 km window
  around the origin) from `19b348e43`, with the feature and strength lines
  and the planet and ship feature columns stripped, is byte-identical to the
  new body-only output.
- Digest `0x1384_6dca_698c_e11a`: FNV-1a 64 over the stripped pre-move
  canonical text of the 125 cells around (2,1,2), seed 20260922, 32 km edge.
  The window holds 22 asteroids, 1 planet and 3 ships. The new test
  reproduces it.
- Unit: `nova_world` 20 + 1 doctest, `nova_world_base` 16.
- Clippy `-D warnings` on both crates' tests and on the six world examples
  with `debug`. Rustdoc `-D warnings` on both crates. fmt and diff check.
- Probe `--correctness-only` on an RTX 3060 Ti: system_world_sectors,
  world_sectors, world_features, world_field_slices, world_field_clouds,
  system_open_world all OK. system_world_sectors recorded all 15 outcomes
  with the pre-change figures: 250 manifests identical over three walks,
  6 spheres (2/1/3), 26 objects with 23 same-cell pairs, 1 planetoid and
  3 inert derelicts.
- Hardware captures before and after: all five slice and cloud frames are
  pixel-identical. The three world_features frames differ only in the fps
  counter, the build hash and sparse single-pixel shading speckle. The
  readouts, census lines and the shot target line match exactly.
- Skipped: workspace tests and workspace clippy (not affected).

## Review fixes

An independent read-only review of #63 found six defects. All six are fixed.

- Claim 14 was vacuous: `(0, -2, 2)` and `(-4, -2, 2)` lie outside the
  window around `(2, 1, 2)`, so ordinary stale discards took the handed-in
  work. Under seed 20260923 no window cell holds bodies under both seeds.
  Owner decision, 2026-09-23: `REPLACEMENT_SEED` 20260925, job cell
  `(0, 0, 0)` (1 planetoid -> 4 rocks), ready cell `(0, 0, 4)` (1 rock ->
  2 rocks). The beat waits for the whole new window. The report asserts that
  each cell's live children are the new manifest's ids, and that the job stats
  moved by exactly +126 requested, +125 completed, +125 materialized and
  +2 discarded. Both cells are desired and the observer stays still, so only
  `clear_sector_work` can take the work.
- The digest test builds its 32 km, radius-2 inputs itself. The digest is
  unchanged. It is not widened: no stripped pre-change dump was kept, and
  building `19b348e43` to make one would need a second checkout.
- Claim 10 says only what the runtime observes: manifest-named bodies parented
  under their root. A `nova_world_base` test ties each planetoid to an owned
  planet sphere centre, and each derelict to an owned derelict sphere.
- A `nova_world` test accepts two rocks 100 m apart at their clearance
  spheres. It fails when the core check is given a 500 m margin.
- `docs/architecture.md` credits the 128 km ceiling to the body budget only.
  The halo bound (about 447 km) is listed separately.
- The `index_slug` doc says cell index. `node_slug` stays private in
  `nova_world_base`. A public cross-crate helper would widen the core
  interface only to share generator naming.

Fix verification:

- Unit: `nova_world` 21 + 1 doctest, `nova_world_base` 17. The digest is
  unchanged.
- Clippy `-D warnings` on both crates' tests and on the six world examples
  with `debug`. Rustdoc `-D warnings` on both crates. fmt and diff check.
- Probe `--correctness-only` on an RTX 3060 Ti: system_world_sectors OK, with
  all 15 outcomes and the figures above. Claim 14 reads retired 125,
  discarded 2, rebuilt 125.
- Mutation (reverted): if `clear_sector_work` keeps `ReadySectors`, claim 14
  fails on the stale `sector_0_0_4_body_0`. If the core check uses a 500 m
  margin, the zero-margin unit test fails.
- Not rerun: the other five world example probes. Their code did not change
  and clippy covers them.

## Architecture artifact

- [world-architecture.html](world-architecture.html): how `nova_world` and
  `nova_world_base` stream, validate, prepare and materialize sectors, from
  the New Game seed to ECS entities. Generated from `d1c583213`.
