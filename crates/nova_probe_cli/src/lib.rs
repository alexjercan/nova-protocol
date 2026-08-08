//! nova_probe_cli: the HOST half of the run-harness. It spawns a cataloged
//! example as a child process, collects whatever artifacts that run wrote,
//! grades them against the check roster and renders the reports.
//!
//! The other half is `nova_probe`, which links into the game and writes those
//! artifacts. The two never share a process: the filesystem and the
//! `NOVA_PERF_*` / autopilot env vars are the IPC, which is why this crate is
//! host-only with no wasm build at all - the split IS the `cfg`.
//!
//! The pipeline reads left to right, and so does the module tree:
//!
//! `collect evidence` (nova_probe, in the child) -> [`evaluation`] ->
//! [`report`].
//!
//! - [`native`] - the driver: command line, spec resolution, paths, child-run
//!   environments and supervision, the single run, the web pass, the sweep.
//! - [`evaluation`] - what the run left behind and what it is worth: the
//!   catalog it was resolved from, its manifest, its artifacts, the chrome
//!   trace, and the checks that grade them.
//! - [`report`] - the HTML and JSON that evaluation turns into: the per-run
//!   report and the multi-run status index.
//! - [`profile_sandbox`] - the profile state a child run is given, so a run
//!   never reads or writes the operator's own mods and settings.
#![warn(missing_docs)]

pub mod evaluation;
pub mod native;
pub mod profile_sandbox;
pub mod report;

pub use evaluation::{
    aggregate_system_costs, categories, evaluate_checks, load_example_catalog,
    parse_example_catalog, render_top_table, CatalogExample, Check, CheckStatus, RunArtifacts,
    SystemCost,
};
pub use report::{
    index_json, overall_verdict as aggregate_verdict, render_index, AllManifest, AllRow,
};
