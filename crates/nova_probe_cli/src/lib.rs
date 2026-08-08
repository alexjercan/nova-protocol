//! nova_probe_cli: the HOST half of the run-harness. It spawns a cataloged
//! example as a child process, collects whatever artifacts that run wrote,
//! grades them against the check roster and renders the reports.
//!
//! The other half is `nova_probe`, which links into the game and writes those
//! artifacts. The two never share a process: the filesystem and the
//! `NOVA_PERF_*` / autopilot env vars are the IPC, which is why this crate is
//! host-only with no wasm build at all - the split IS the `cfg`.
//!
//! `collect evidence` (nova_probe, in the child) -> `run evaluation`
//! ([`run_report`], here) -> `generate report` ([`report`], [`aggregate`]).
//!
//! - [`native`] - the driver: command line, spec resolution, paths, child-run
//!   environments and supervision, the single run, the web pass, the sweep.
//! - [`catalog`] - the Cargo.toml `[[example]]` catalog runs resolve against.
//! - [`profile`] - chrome-trace post-processing into a costliest-systems table.
//! - [`profile_sandbox`] - the profile-state sandbox child runs are given.
//! - [`run_report`] - artifacts in, checks + `report.html` + `checks.json` out.
//! - [`report`] - the shared HTML pieces both reports compose.
//! - [`aggregate`] - the multi-run status index.
#![warn(missing_docs)]

pub mod aggregate;
pub mod catalog;
pub mod native;
pub mod profile;
pub mod profile_sandbox;
pub mod report;
pub mod run_report;

pub use aggregate::{
    index_json, overall_verdict as aggregate_verdict, render_index, AllManifest, AllRow,
};
pub use catalog::{categories, load_example_catalog, parse_example_catalog, CatalogExample};
pub use profile::{aggregate_system_costs, render_top_table, SystemCost};
pub use run_report::{evaluate_checks, Check, CheckStatus, RunArtifacts};
