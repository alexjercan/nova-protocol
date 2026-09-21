# Review 19: WFC and example infrastructure

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Verdict: minor findings; example-body coverage incomplete

## Findings

### MINOR - `examples/playable/shape_bench.rs` and `block_bench.rs` - Bench scaffolding is duplicated almost wholesale

The two examples share most of their loader, subject, refusal, readout, label, skin-report, framing, idle-orbit, and scripted-harness code. Their intended difference is the subject roster. This is a large dual maintenance path with no current behavior failure.

### MINOR - Seven playable/screenshot examples duplicate idle-orbit camera logic and have drifted

`greeble_catalog`, `wfc_ships`, `wfc_arena`, `shape_bench`, `block_bench`, `screenshot_thruster_gallery`, and `screenshot_section_gallery` each carry a local idle-orbit implementation. The `greeble_catalog` copy re-arms on active camera aim, while the other playable copies do not. No current failure was demonstrated, but the drift confirms the maintenance risk.

### MINOR - `examples/playable/wfc_ships.rs:34`, `wfc_arena.rs:4`, and `Cargo.toml:84` point to deleted `shared/wfc.rs`

WFC moved into the `nova_wfc` crate. These comments still direct readers to the removed shared example module, while another comment in `wfc_arena.rs` correctly names `nova_wfc`.

### MINOR - `examples/playable/wfc_arena.rs:354-357` bypasses exported prototype ID constants

The example hardcodes the torpedo, kinetic PDC, and pierce PDC IDs although `nova_ship` exports checked constants and the same block already uses the railgun constant. Current literals match. A future rename would not produce a compiler error and could silently break armament binding.

## Verification

`nix develop --command cargo test -p nova_wfc --lib` passed 17 tests. `cargo test --example wfc_arena --features debug` passed 15 tests. `cargo test -p nova_probe_cli --test catalog_drift` passed both catalog and invariant-roster tests.

No correctness defect was found in WFC collapse, constraints, refusal paths, or the two WFC proof examples.

## Coverage

Fully read every `nova_wfc` file and test, every playable example, and the complete WFC arena module tree. Read representative lesson, loop, screenshot, and shared screenshot infrastructure. Catalog- and pattern-scanned the remaining screenshot examples.

The correctness reviewer did not follow the request to read every `examples/systems` file; those 65 files received catalog and trap-pattern scans only. Roughly 86 screenshot example files also received structural scans rather than full reads. These gaps require additional paired batches before the final coverage audit.

Not checked: no game, probe, GPU measurement, content lint/generation, workspace test, or Clippy run. Most screenshot bodies and all system-example bodies remain unreviewed line by line.
