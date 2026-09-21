# Review 22: System examples B

- Baseline: `b7a56f0586`
- Scope: `system_helm_orders.rs` through `system_wreck_lock.rs`
- Lanes: craft, performance, correctness, contracts
- Verdict: minor diagnostic-text findings

## Finding

### MINOR - Several proof failure messages contain accidental whitespace

Long space runs appear inside assertion messages at:

- `examples/systems/system_hull_scaling.rs:347,355`
- `examples/systems/system_scenario_picker.rs:304`
- `examples/systems/system_ship_editor.rs:1554,1575`

The assertions and measured conditions are correct. Only diagnostics shown when a proof fails are degraded.

## Adjudication

Dropped the mirrored sever-physics constants as a craft finding. The example uses an independent oracle rather than importing production values into its proof, and the mirror is explicitly documented. Importing the implementation constants could make the test vacuous.

No weak outcome, timing assertion, fixture-fidelity, warm-up, change-detection, runtime-ID, content, ECS, or portability defect survived. A proof specialist follow-up was not needed.

## Coverage

Both reviewers fully read all 31 top-level files in the assigned range and every included shared/nested module, including the 4,274-line ship-editor proof and turret slider module. Outcome counts and representative texts were checked against the catalog roster.

Together with review 21, every `examples/systems` source file has a full static review.

Not checked: no Cargo command, game, probe, GPU run, workspace test, or Clippy run. Catalog tests were read but not executed.
