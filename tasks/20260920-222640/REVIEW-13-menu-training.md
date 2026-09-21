# Review 13: Menu and training

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Verdict: minor findings

## Findings

### MINOR - `crates/nova_menu/src/scenarios.rs:580-593` and `training.rs:1204-1230` - Media load/rejection transitions lack a real asset proof

Menu tests do not create an `AssetServer`, so scenario thumbnail and lesson media polling return at their first guard. No test exercises still-loading to loaded re-arming, non-2D image rejection, or uneven loop-grid rejection. These branches protect against media never appearing and invalid texture bindings.

The implementation appears consistent by inspection. This is a proof gap for named stable asset-lifecycle behavior, not a reproduced runtime defect.

### MINOR - `crates/nova_menu/src/tests/support.rs:407-418,493-503` - Shared test support has duplicate helper paths

`find_named` and `entity_by_name` are identical and both used. `all_text` and `all_texts` are also identical and both used. These dual test-support interfaces can drift without providing different semantics.

## Adjudication

Checked and cleared pause/NOVA OS/outcome arbitration, clock-freeze ownership, loading restart, settings debounce/flush, training persistence, rebind conflict handling, portal action state, scenario/campaign roles, lesson IDs, and field-note rotation. No concrete unordered `NextState` conflict survived review.

## Coverage

Fully read every source and test file in `nova_menu` and `nova_training`, including all menu subsystem tests and training inline tests.

Not checked: no Cargo command, game, probe, rendered UI run, GPU measurement, wasm build, workspace test, or Clippy run. Media branches were not run with real assets. Cross-crate scenario outcome, gameplay freeze, UI widget, and asset-storage internals were reviewed only at their menu call sites.
