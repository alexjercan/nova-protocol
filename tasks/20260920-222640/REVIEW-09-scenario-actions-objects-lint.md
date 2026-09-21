# Review 09: Scenario actions, objects, and lint

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Specialist: content IDs and shader contracts
- Verdict: minor findings

## Findings

### MINOR - `crates/nova_scenario/src/objects/asteroid_kind.rs:13,240` - Documentation names removed `AsteroidConfig::material`

The authored field is `AsteroidConfig::kind`, and both lint and runtime use `kind`. The module retains a broken intra-doc link to `material` and prose that says lint checks an authored material. No live content, builder, editor label, or creator document retains that old field name.

This is documentation-only. The live lint/load contract is consistent and fails loudly on unknown kinds.

### MINOR - `crates/nova_scenario/src/objects/asteroid.rs:31,587-804` - Obsolete planet-height terrain code remains public

`PlanetHeight` and `PlanetHeightNoise` have no workspace consumer outside their own unit test, but remain exported from the asteroid prelude. Their documentation falsely says `asteroid_scenario_object` uses them. Production asteroid meshing now uses `RockHeight` through `pristine_rock_mesh`.

This is dead internal/public surface from a replaced generation path. No runtime behavior currently depends on it.

## Specialist verification

The content specialist confirmed:

- all shipped base content, example mods, bench fixtures, editor choices, and web widgets use the same five asteroid kind IDs;
- asteroid and planet Rust uniform field order matches the corresponding WGSL structs and array limits;
- `PlanetHeight` has no content, tool, or runtime consumer;
- no live content or documentation retains asteroid `material` as the kind field.

## Coverage

Across the pair, fully read all remaining scenario actions, objects, lint, names, variables, syntax, render-scale, test-support, integration-test, and benchmark files. One reviewer sampled the large `lint/scenario.rs` test body by test name, while the other reported a full read. Together with review 08, every Rust file in `nova_scenario` has a static pass.

Not checked: no Cargo command, content lint execution, content generation, game, probe, wasm build, workspace test, or Clippy run. Shader agreement was verified by source layout comparison, not GPU execution. General balance and narrative content were not judged.
