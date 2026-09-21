# Review 15: Editor model and I/O

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Verdict: minor findings

## Findings

### MINOR - `crates/nova_editor/src/bundle.rs:458-464` - Ordinal parsing duplicates the editor's canonical ID rule

`resume_ordinal` parses the final underscore suffix directly, while `node.rs::split_ordinal` and `id_order` own the documented canonical rule, including `_section_` handling. Current minted IDs produce the same result, so no collision was reproduced. The second parser can drift and affect ordinal resume after open or generation.

No new interface is needed; the existing canonical rule should own both paths in a later fix.

### MINOR - `crates/nova_editor/src/lib.rs:10-33` - The crate structure map omits live modules

The crate-level structure list omits `asset_index` and `palette`, although both are declared and active. This is documentation-only.

## Adjudication

Checked and cleared document/view ownership, ID minting, save/load atomicity, native/wasm file handling, lint-before-generation commit, nested retry retargeting, inspector declaration/write agreement, quantity units, reference resolution, expression round trips, editor bundle format, and reserved editor bundle IDs.

## Coverage

Fully read node, inspect and tests, scenario, event and tests, bundle and tests, generation and tests, asset index and tests, templates and tests, file-window UI and tests, and editor plugin wiring.

Not checked: no Cargo command, content lint, content generation, game, probe, wasm build, workspace test, or Clippy run. Interactive UI, placement, stage, gizmo, skin, gallery, highlight, and remaining visual/editor support modules remain for review 16. Saved artifacts were not exercised on disk.
