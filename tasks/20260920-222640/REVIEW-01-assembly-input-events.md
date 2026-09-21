# Review 01: Assembly, input, and events

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Verdict: findings

## Findings

### MAJOR - `docs/architecture.md:224` - Headless plugin contract contradicts app assembly

The architecture document says `NovaHudPlugin` and `NovaOsUiPlugin` are render-gated and absent from headless runs. `crates/nova_core/src/lib.rs:425-433` adds both unconditionally. Its comment states that this is required to retain all input registry actions and NOVA OS behavior off-screen.

This gives probe and headless-run authors the wrong system and CPU-cost model. No API change is needed. Update the architecture description in a later fix.

Why not BLOCKER: runtime behavior is intentional and correct; the defect is in developer documentation.

### MINOR - `crates/nova_core/src/loading_screen.rs:353-356` - A retriggered scenario load keeps the previous destination note

When a scenario loading screen already exists, `spawn_scenario_load_screen` resets only `started` and returns. It does not recompute the note for the new `LoadScenario` ID. A second load request before dismissal can therefore show the first destination's note while loading the second destination.

This is source-only reasoning. No runtime capture reproduced it. No new API is needed.

Why not higher: loading copy is wrong, but scenario state and loading behavior remain correct.

### MINOR - `crates/nova_core/src/lib.rs:42-46` - A cross-crate import bypasses the exported `nova_ui` prelude

The status-bar names imported directly from `nova_ui::status_bar` are already exported by `nova_ui::prelude`. This violates the repository import rule. No behavior or API change is required.

### MINOR - `crates/nova_events/src/lib.rs:21` - The crate root bypasses `engine::prelude`

`use crate::engine::*` bypasses the module prelude, which already exports the required public names. This is an import-only craft defect.

### MINOR - `src/main.rs:90` - A bare lint allowance violates repository policy

`#[allow(unused_variables)]` has no reason and conflicts with the requirement to use reasoned `#[expect]` attributes. The cross-configuration use of `cli` needs a cfg-aware code shape or a documented lint treatment. No runtime behavior is affected.

## Adjudication

- Dropped the claimed per-frame heap allocation in `nova_events::queue_system`. `HashSet::new()` does not allocate until a once-handler is inserted, so the report did not establish the stated unconditional cost.
- Dropped the lexical complaint about the phrase "load-bearing." The adjacent comment gives the concrete reason and constraint.

## Coverage

Fully read every Rust file in root `src/`, `nova_core`, `nova_input`, `nova_events`, `nova_events_macros`, and `nova_info`, including their local tests and crate manifests.

Targeted cross-boundary reads covered `nova_ui` status-bar preludes, startup scenario publication, debug entry-point resolution, and architecture/performance documentation tied to these APIs.

Not checked: no Cargo command, game, probe, content lint, wasm build, workspace test, or Clippy run. Other crates were not reviewed beyond direct call or contract traces. The loading-screen finding was not reproduced in a rendered flow.
