# Review 11: Authoring and base content

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Specialist: content proof and runtime-ID follow-up
- Verdict: major finding

## Finding

### MAJOR - `webmods/gauntlet/README.md:31-34` and `docs/scenario-system.md:670-673` - Gauntlet geometry is documented as automatically proved, but no proof exists

The Gauntlet README says `crates/nova_assets/tests/gauntlet_course.rs` loads the real content and proves gate non-overlap and racing-line clearance. That test target was deleted. Its replacement, `scenario_gate_course.rs`, uses a synthetic gate fixture with no asteroids and proves only generic sequencing.

The scenario-system document instead says `content lint` checks these per-bundle geometry invariants. Full inspection of `nova_authoring::lint_walk` and `nova_scenario::lint::scenario` found no scenario-area overlap check and no racing-line clearance check against worst-case asteroid geometry. Header comments in `webmods/gauntlet/gauntlet.content.ron:15-25` still tell authors to keep this nonexistent proof green.

Concrete failure:

- the documented `cargo test -p nova_assets --test gauntlet_course` target does not exist;
- overlapping gate areas or an authored rock obstructing the racing line produce no named geometry finding from content lint.

Why not BLOCKER: current content was not shown invalid and runtime loading still works. The defect is a false authoring-safety contract around future edits.

## Adjudication

No code defect was found in generation ownership, parity-test structure, lint/report mechanics, builder IDs, balance acknowledgments, lesson links, or base scenario composition.

The content specialist independently confirmed the missing Gauntlet proof, fully checked the real Ledger campaign test and IDs, and verified that generation and parity use the same `content_files()` map. A cold focused parity-test build timed out after two minutes, so byte parity was not established by execution in this review.

## Coverage

Fully read all `nova_authoring` source files and eight integration tests, including all Rust base-content builders. Traced section, ship, scenario, campaign, lesson, input, asset, and style IDs across builders and consumers. Read the Gauntlet content and README, the Ledger bundle and its real-content integration test, the example mod, generated-file map, and targeted generated RON samples.

Not checked: no successful Cargo test, content lint execution, content generation, game, probe, wasm build, workspace test, or Clippy run. Not every generated RON file was manually diffed against serialized builder output. Binary assets, balance quality, and narrative quality were not judged.
