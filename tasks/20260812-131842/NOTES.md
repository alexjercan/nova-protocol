# Notes

## Accepted design - 2026-08-13

### Migration scope

Replace all three shipped cube ships: Racer, CargoB, and CargoA. Add a CargoA
semantic-part recipe. Delete every shipped cube GLB and cube prototype. No
compatibility aliases remain.

### Ship composition

Use semantic body parts plus mounted functional modules.

- Racer: seven recipe body parts plus two turret modules.
- CargoB: seven recipe body parts. The side pods own torpedo behavior. Add two
  mounted turret modules.
- CargoA: semantic body parts only. Engines own thrust; the central body owns
  control.

Mounted modules use semantic section IDs. They are not hidden cube remnants.

### Collider and origin policy

Center each section entity on its tight primitive collider. Preserve exact
visual assembly with `render_mesh_transform`. Author link points in the same
centered section-local frame. Do not add collider offsets and do not inflate
symmetric colliders around recipe anchors.

### IDs and compatibility

Use a clean semantic break. Prototype and placed IDs describe parts, for
example `racer_fuselage`, `engine_port`, and `turret_starboard`. Migrate all
base content, bundled mods, examples, tests, input bindings, and section
modifications explicitly. Bump each changed bundled mod. Keep no old aliases.

### Link points

Hand-author final sockets in the Rust content builders. Recipes continue to own
mesh partitioning; builders own gameplay attachment intent. Pin every assembled
ship as one valid link-point graph. Generic recipe socket generation remains in
`20260812-131005`.

### Delivery order

1. Add and render-review the CargoA semantic-part recipe.
2. Generate Racer, CargoB, and CargoA into `assets/base/gltf/parts/`.
3. Add centered-collider part prototypes with explicit link points.
4. Replace ship builders with semantic IDs and mounted weapon modules.
5. Regenerate base content and migrate all bundled consumers.
6. Delete cube assets, prototypes, coordinate tables, and helpers.
7. Run content lint and focused graph/player-path checks.
8. Render-review ships, refresh invalidated screenshots, update routed docs,
   and run the probe sweep.

Balance drift is accepted and recorded, not tuned in this task.

## Balance drift

- Racer moves from 18 cube sections to seven body parts plus two turret
  modules. Body health is grouped by semantic footprint; total health and mass
  distribution therefore change.
- CargoB moves from 43 sections to seven body parts plus two turret modules.
  Each full side pod now owns one torpedo bay and a larger health pool.
- CargoA moves from 53 sections to seven body parts. Large pods carry most of
  its structural health.
- Fewer, larger colliders reduce section count and make damage chunkier. No
  handling, weapon, or encounter retune was attempted.

### Overlap lint decision - 2026-08-13

Permit collider AABB overlap only between sections that have a direct authored
link-point mate. Semantic meshes can interlock while their primitive colliders
remain tight. Unmated overlap remains an error. Remove the obsolete unit-grid
mount-base adjacency lint; the authoritative link-point graph now validates
mount structure.

## Delivery record

- Added `craft_cargoa.json`. Its seven-part cut conserves source area, has zero
  open cut edges, and reopens every GLB. Assembled and exploded viewer captures
  were inspected.
- Generated 21 shipped body meshes under `assets/base/gltf/parts/`. Deleted
  all Racer, CargoA, and CargoB cube mesh libraries and prototypes.
- Migrated base scenarios, Gauntlet, The Ledger, screenshot fixtures, tests,
  section ids, input bindings, and modifications. Gauntlet is 1.8.0; The
  Ledger is 1.21.0.
- Refreshed affected flight and combat web captures at 1920x1080 and inspected
  representative ship, chase, combat, and torpedo frames.
- The first probe sweep found two unrelated fixture assumptions made visible by
  the stricter runtime graph and destruction lifecycle: `scenario_grammar` had
  a detached turret placement, and `hud_range` used generic despawn while
  asserting a destruction-confirmed kill cam. Both fixtures now exercise their
  real seams. Their reruns pass.

## Verification

- `python3 scripts/cut-obj-into-parts.py --self-test`
- `nix develop --command cargo check`
- `nix develop --command cargo test --lib -p nova_authoring` - 48 passed
- Focused scenario lint tests - 8 passed
- Gauntlet integration - 12 passed
- Ledger chapter 2 - 16 passed; chapter 3 - 18 passed; chapter 4 - 12 passed;
  chapter 5 - 13 passed after its version pin was updated
- Webmod recursive validation - 2 passed
- Clean-profile player path completed two rounds
- `nix develop --command cargo run content -- lint` - 0 errors, 0 warnings
- Probe coverage: all 27 cataloged examples. The initial sweep timed out after
  15 entries; completed entries passed except `scenario_grammar` and
  `hud_range`. Both fixed reruns pass, and the remaining 13 examples pass in
  `probe-runs/parts-migration-remaining/be3a9c3d/`.
- `cd web && npm run ci`
- `nix develop --command cargo fmt --check`
- `git diff --check`
