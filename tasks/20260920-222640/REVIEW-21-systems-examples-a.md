# Review 21: System examples A

- Baseline: `b7a56f0586`
- Scope: `bug_carve_apply.rs` through `system_headless_replay.rs`
- Lanes: craft, performance, correctness, contracts
- Verdict: minor findings

## Findings

### MINOR - `crates/nova_probe_cli/tests/catalog_drift.rs:127-129` - Comment says headless ranges have no markers

All six `system_headless_*` ranges now contain outcome markers and populated roster entries. The mechanical test is correct; only its explanatory comment describes the old unmarked state.

### MINOR - `examples/systems/stress_hull_collapse.rs:1257,1264` and `system_field_controls.rs:206` - Assertion messages contain garbled whitespace

The failure messages contain long accidental space runs at line-wrap joins. Assertions remain correct, but a failed proof would print degraded diagnostics.

### MINOR - `examples/screenshots/shared/kit.rs:29-32` - Shared fixture uses an `allow` contrary to repository lint policy

The module uses `#![allow(dead_code, reason = ...)]`. Its multi-target inclusion gives a credible reason why one `#[expect]` may be fulfilled in some binaries and not others, but the repository rule has no documented exception. Either the code shape or the policy needs one owned resolution.

## Adjudication

The shared `examples/systems/shared/` policy contradiction duplicates review 17's stale README finding and is not counted again.

No weak outcome, fixture-fidelity, warm-up, change-detection, timing-assertion, ID, or portability defect survived. A proof specialist follow-up was not needed.

## Coverage

Both reviewers fully read all 33 top-level example files in the assigned range and every included shared/nested module. Outcome markers were checked against the compiled catalog roster. No grep-only substitution remained.

Not checked: no Cargo command, game, probe, GPU run, workspace test, or Clippy run. The per-target lint behavior of the shared kit was not compiled.
