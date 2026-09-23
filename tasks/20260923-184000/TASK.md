# Open world: nova_world_base owns the feature field

- STATUS: OPEN
- PRIORITY: 72
- TAGS: v0.15.0,architecture,open-world

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
- The first probe run caught claim 14 comparing only the home cell, which
  holds no body under either seed. Its precondition now compares the whole
  window (49 of 214 shared cells differ).
- Hardware captures before and after: all five slice and cloud frames are
  pixel-identical. The three world_features frames differ only in the fps
  counter, the build hash and sparse single-pixel shading speckle. The
  readouts, census lines and the shot target line match exactly.
- Skipped: workspace tests and workspace clippy (not affected).
